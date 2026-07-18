//! Task 14.11: factory-built Anthropic cache-marker wire evidence.

use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use opi_ai::ApiKind;
use opi_ai::credential::{Credential, CredentialStore};
use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, Message, ToolDef, UserMessage,
};
use opi_ai::provider::{CacheRetention, ModelInfo, Request, ThinkingConfig};
use opi_ai::registry::{ModelCapabilities, ProviderRegistry};
use opi_ai::stream::{StopReason, Usage};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::credential_store::{
    CredentialResolver, FakeKeyringBackend, KeychainCredentialStore,
};
use opi_coding_agent::oauth::OAuthProviderRegistry;
use opi_coding_agent::provider_factory::build_provider_with_oauth;
use secrecy::SecretString;
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const BUILTIN_MODEL: &str = "anthropic:claude-sonnet-4-5-20250514";
const CUSTOM_MODEL: &str = "anthropic:custom-claude";
const UNKNOWN_MODEL: &str = "anthropic:unknown-claude";
const TERMINAL_SSE: &str = r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_cache","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":1,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"ok"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":1}}

event: message_stop
data: {"type":"message_stop"}

"#;

fn secret(value: &str) -> SecretString {
    SecretString::new(value.to_owned().into_boxed_str())
}

fn test_store() -> (TempDir, Arc<KeychainCredentialStore>) {
    let dir = TempDir::new().expect("temp credential directory");
    let store = Arc::new(KeychainCredentialStore::with_lock_timeout(
        Box::new(FakeKeyringBackend::new()),
        dir.path().to_path_buf(),
        Duration::from_secs(2),
    ));
    (dir, store)
}

fn assistant(texts: &[&str]) -> Message {
    Message::Assistant(AssistantMessage {
        content: texts
            .iter()
            .map(|text| AssistantContent::Text {
                text: (*text).to_owned(),
            })
            .collect(),
        api: ApiKind::Anthropic,
        provider: "anthropic".into(),
        model: "claude-sonnet-4-5-20250514".into(),
        response_model: None,
        response_id: None,
        usage: Usage::unknown(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms: 0,
    })
}

fn user(texts: &[&str]) -> Message {
    Message::User(UserMessage {
        content: texts
            .iter()
            .map(|text| InputContent::Text {
                text: (*text).to_owned(),
            })
            .collect(),
        timestamp_ms: 0,
    })
}

fn request(model: &str, cache_retention: CacheRetention) -> Request {
    Request {
        model: model.into(),
        system: Some("system prompt".into()),
        messages: vec![
            user(&["old user"]),
            assistant(&["old assistant"]),
            user(&["final user prefix", "final user text"]),
            assistant(&["final assistant prefix", "final assistant text"]),
        ],
        tools: vec![
            ToolDef {
                name: "first_tool".into(),
                description: "must remain unmarked".into(),
                input_schema: json!({"type": "object"}),
            },
            ToolDef {
                name: "last_tool".into(),
                description: "must receive the final tool marker".into(),
                input_schema: json!({"type": "object"}),
            },
        ],
        max_tokens: Some(64),
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: Some(Duration::from_secs(2)),
        extra_headers: vec![],
        cache_retention,
        session_id: None,
    }
}

async fn send(registry: &ProviderRegistry, model: &str, retention: CacheRetention) {
    let provider = registry
        .get_provider("anthropic")
        .expect("factory-built Anthropic provider registered");
    let mut stream = provider.stream(request(model, retention));
    while let Some(event) = stream.next().await {
        event.expect("mock Anthropic stream must not fail");
    }
}

fn collect_cache_controls<'a>(value: &'a Value, found: &mut Vec<&'a Value>) {
    match value {
        Value::Object(map) => {
            if let Some(marker) = map.get("cache_control") {
                found.push(marker);
            }
            for child in map.values() {
                collect_cache_controls(child, found);
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_cache_controls(child, found);
            }
        }
        _ => {}
    }
}

fn assert_exact_markers(body: &Value, marker: &Value) {
    let mut found = Vec::new();
    collect_cache_controls(body, &mut found);
    assert_eq!(found, vec![marker, marker, marker, marker]);

    assert_eq!(&body["system"][0]["cache_control"], marker);
    assert_eq!(&body["messages"][2]["content"][1]["cache_control"], marker);
    assert_eq!(&body["messages"][3]["content"][1]["cache_control"], marker);
    assert_eq!(&body["tools"][1]["cache_control"], marker);

    assert!(body["messages"][0]["content"][0]["cache_control"].is_null());
    assert!(body["messages"][1]["content"][0]["cache_control"].is_null());
    assert!(body["messages"][2]["content"][0]["cache_control"].is_null());
    assert!(body["messages"][3]["content"][0]["cache_control"].is_null());
    assert!(body["tools"][0]["cache_control"].is_null());
}

fn assert_no_markers(body: &Value) {
    let mut found = Vec::new();
    collect_cache_controls(body, &mut found);
    assert!(found.is_empty(), "unexpected cache markers: {found:?}");
    assert!(body["system"].is_string());
}

#[tokio::test]
async fn factory_built_anthropic_cache_markers_follow_final_capabilities() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(TERMINAL_SSE)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let (_dir, store) = test_store();
    store
        .write("anthropic", &Credential::ApiKey(secret("factory-test-key")))
        .await
        .expect("seed Anthropic API key");
    let resolver = CredentialResolver::new(store, Arc::new(|_: &str| None));
    let oauth_registry = OAuthProviderRegistry::registry_with_builtins();
    let mut config = OpiConfig::default();
    config.defaults.model = BUILTIN_MODEL.into();
    config.providers.anthropic.base_url = Some(server.uri());

    let provider = build_provider_with_oauth(&config, &resolver, &oauth_registry)
        .await
        .expect("build Anthropic through the production provider factory");
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(provider)
        .expect("register provider");

    let builtin = registry
        .capabilities(BUILTIN_MODEL)
        .expect("resolve final built-in ModelInfo capabilities");
    assert!(builtin.supports_cache_control);
    assert!(builtin.supports_long_cache_retention);

    registry
        .register_model(
            "anthropic",
            ModelInfo::new(
                "custom-claude",
                "Custom Claude",
                opi_ai::WireApi::AnthropicMessages,
                ModelCapabilities::new(100_000, 4_096),
            ),
        )
        .expect("register custom model with default-off cache capabilities");
    let custom = registry
        .capabilities(CUSTOM_MODEL)
        .expect("resolve custom model capabilities");
    assert!(!custom.supports_cache_control);
    assert!(!custom.supports_long_cache_retention);
    assert!(registry.capabilities(UNKNOWN_MODEL).is_err());

    send(&registry, BUILTIN_MODEL, CacheRetention::Long).await;
    send(&registry, BUILTIN_MODEL, CacheRetention::Short).await;
    send(&registry, BUILTIN_MODEL, CacheRetention::Disabled).await;
    send(&registry, CUSTOM_MODEL, CacheRetention::Long).await;
    send(&registry, UNKNOWN_MODEL, CacheRetention::Long).await;

    let requests = server.received_requests().await.expect("captured requests");
    assert_eq!(requests.len(), 5);
    for request in &requests {
        assert_eq!(
            request
                .headers
                .get("x-api-key")
                .and_then(|value| value.to_str().ok()),
            Some("factory-test-key")
        );
    }

    let bodies: Vec<Value> = requests
        .iter()
        .map(|request| serde_json::from_slice(&request.body).expect("Anthropic JSON request"))
        .collect();
    for (case, body) in ["long", "short", "disabled", "custom", "unknown"]
        .into_iter()
        .zip(&bodies)
    {
        println!("{}", json!({"case": case, "request_body": body}));
    }
    assert_exact_markers(&bodies[0], &json!({"type": "ephemeral", "ttl": "1h"}));
    assert_exact_markers(&bodies[1], &json!({"type": "ephemeral"}));
    assert_no_markers(&bodies[2]);
    assert_no_markers(&bodies[3]);
    assert_no_markers(&bodies[4]);
}
