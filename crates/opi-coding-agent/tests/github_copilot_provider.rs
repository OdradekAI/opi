use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::credential::{Credential, CredentialStore};
use opi_ai::message::{
    ImageSource, InputContent, MediaType, Message, OutputContent, ToolResultMessage, UserMessage,
};
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::{ThinkingLevel, WireApi, WireCompat};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::credential_store::{
    CredentialResolver, FakeKeyringBackend, KeychainCredentialStore,
};
use opi_coding_agent::github_copilot::{
    GITHUB_COPILOT_DEFAULT_BASE_URL, github_copilot_catalog, github_copilot_static_headers,
};
use opi_coding_agent::oauth::OAuthProviderRegistry;
use opi_coding_agent::provider_factory::build_provider_with_oauth;
use secrecy::SecretString;
use tempfile::TempDir;
use time::OffsetDateTime;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "fixtures/pi-0.80.6/github-copilot.models.json"
    ))
    .expect("valid normalized fixture")
}

#[test]
fn github_copilot_tests_do_not_reference_ignored_repo() {
    let test_source = include_str!("github_copilot_provider.rs");
    let ignored_repo_reference = [".", "repo"].concat();
    let filesystem_read = ["std::fs", "::read"].concat();
    assert!(!test_source.contains(&ignored_repo_reference));
    assert!(!test_source.contains(&filesystem_read));
}

#[test]
fn github_copilot_catalog_matches_pi_0806_fixture() {
    let fixture = fixture();
    assert_eq!(fixture["pi_version"], "0.80.6");
    assert_eq!(
        fixture["source_path"],
        "packages/ai/src/providers/github-copilot.models.ts"
    );
    assert_eq!(fixture["provider_id"], "github-copilot");
    assert_eq!(fixture["default_base_url"], GITHUB_COPILOT_DEFAULT_BASE_URL);
    assert_eq!(
        fixture["source_sha256"],
        "6FE91A9895552B56F882428F124466DFBB08CE27F4D4CE0ED0C5F23168517EFA"
    );

    let fixture_models = fixture["models"].as_array().unwrap();
    let catalog = github_copilot_catalog();
    assert_eq!(fixture_models.len(), 25);
    assert_eq!(catalog.len(), 25);
    let expected_headers: BTreeMap<_, _> = github_copilot_static_headers().into_iter().collect();
    for (actual, expected) in catalog.iter().zip(fixture_models) {
        assert_eq!(actual.id, expected["id"]);
        assert_eq!(actual.display_name, expected["display_name"]);
        assert_eq!(actual.wire_api.as_str(), expected["wire"]);
        assert_eq!(actual.base_url.as_deref(), expected["base_url"].as_str());
        let capabilities = &expected["capabilities"];
        assert_eq!(
            actual.capabilities.context_window,
            capabilities["context_window"].as_u64().unwrap()
        );
        assert_eq!(
            actual.capabilities.max_output_tokens,
            capabilities["max_output_tokens"].as_u64().unwrap()
        );
        assert_eq!(
            actual.capabilities.supports_images,
            capabilities["supports_images"].as_bool().unwrap()
        );
        assert_eq!(
            actual.capabilities.supports_streaming,
            capabilities["supports_streaming"].as_bool().unwrap()
        );
        assert_eq!(
            actual.capabilities.supports_thinking,
            capabilities["supports_thinking"].as_bool().unwrap()
        );
        assert_eq!(
            actual.capabilities.supports_cache_control,
            capabilities["supports_cache_control"].as_bool().unwrap()
        );
        assert_eq!(
            actual.capabilities.supports_long_cache_retention,
            capabilities["supports_long_cache_retention"]
                .as_bool()
                .unwrap()
        );
        assert_eq!(
            expected["input_modes"].as_array().unwrap().len(),
            if actual.capabilities.supports_images {
                2
            } else {
                1
            }
        );
        for (level, mapped) in expected["thinking_map"].as_object().unwrap() {
            let level = ThinkingLevel::from_str(level).unwrap();
            match mapped.as_str() {
                Some(value) => assert_eq!(
                    actual.thinking_level_map.resolve(level).unwrap().as_deref(),
                    Some(value)
                ),
                None => assert!(actual.thinking_level_map.resolve(level).is_err()),
            }
        }
        match (&actual.compat, actual.wire_api) {
            (WireCompat::AnthropicMessages(compat), WireApi::AnthropicMessages) => {
                let source = expected["compat"].as_object().unwrap();
                assert_eq!(
                    compat.supports_eager_tool_input_streaming,
                    source
                        .get("supportsEagerToolInputStreaming")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(true)
                );
                assert_eq!(
                    compat.force_adaptive_thinking,
                    source
                        .get("forceAdaptiveThinking")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false)
                );
                assert_eq!(
                    compat.supports_temperature,
                    source
                        .get("supportsTemperature")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(true)
                );
            }
            (WireCompat::OpenAiCompletions(compat), WireApi::OpenAiCompletions) => {
                assert!(!compat.supports_store);
                assert!(!compat.supports_developer_role);
                assert!(!compat.supports_reasoning_effort);
                assert_eq!(compat.chat_completions_path, "/chat/completions");
            }
            (WireCompat::OpenAiResponses(compat), WireApi::OpenAiResponses) => {
                assert_eq!(compat.responses_path, "/responses");
            }
            other => panic!("fixture wire/compat mismatch: {other:?}"),
        }
        let headers: BTreeMap<_, _> = expected["headers"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, value)| (name.clone(), value.as_str().unwrap().to_owned()))
            .collect();
        assert_eq!(headers, expected_headers);
        let pricing = actual.pricing.as_ref().expect("embedded pricing").base;
        assert_eq!(
            pricing.input_cost_per_mtok,
            expected["pricing"]["input"].as_f64().unwrap()
        );
        assert_eq!(
            pricing.output_cost_per_mtok,
            expected["pricing"]["output"].as_f64().unwrap()
        );
        assert_eq!(
            pricing.cache_read_cost_per_mtok,
            expected["pricing"]["cache_read"].as_f64().unwrap()
        );
        assert_eq!(
            pricing.cache_write_cost_per_mtok,
            expected["pricing"]["cache_write"].as_f64().unwrap()
        );
    }
}

#[test]
fn github_copilot_catalog_has_25_models_and_three_wires() {
    let catalog = github_copilot_catalog();
    let mut by_wire: BTreeMap<WireApi, Vec<&str>> = BTreeMap::new();
    for model in &catalog {
        by_wire.entry(model.wire_api).or_default().push(&model.id);
    }
    assert_eq!(catalog.len(), 25);
    assert_eq!(
        by_wire[&WireApi::OpenAiCompletions],
        [
            "claude-fable-5",
            "gemini-2.5-pro",
            "gemini-3-flash-preview",
            "gemini-3.1-pro-preview",
            "gemini-3.5-flash",
            "gpt-4.1",
            "kimi-k2.7-code",
            "mai-code-1-flash-picker",
        ]
    );
    assert_eq!(
        by_wire[&WireApi::AnthropicMessages],
        [
            "claude-haiku-4.5",
            "claude-opus-4.5",
            "claude-opus-4.6",
            "claude-opus-4.7",
            "claude-opus-4.8",
            "claude-sonnet-4",
            "claude-sonnet-4.5",
            "claude-sonnet-4.6",
            "claude-sonnet-5",
        ]
    );
    assert_eq!(
        by_wire[&WireApi::OpenAiResponses],
        [
            "gpt-5-mini",
            "gpt-5.2",
            "gpt-5.2-codex",
            "gpt-5.3-codex",
            "gpt-5.4",
            "gpt-5.4-mini",
            "gpt-5.4-nano",
            "gpt-5.5",
        ]
    );
}

#[test]
fn github_copilot_model_listing_is_static_and_secret_free() {
    let canary = "copilot-secret-DO-NOT-LEAK";
    let rendered = format!(
        "{:?}\n{}",
        github_copilot_catalog(),
        include_str!("fixtures/pi-0.80.6/github-copilot.models.json")
    );
    assert!(!rendered.contains(canary));
    assert_eq!(github_copilot_catalog().len(), 25);
}

fn oauth_credential(access: &str, base_url: String) -> Credential {
    Credential::OAuthToken {
        access: SecretString::new(access.to_owned().into_boxed_str()),
        refresh: SecretString::new("offline-refresh".to_owned().into_boxed_str()),
        expires_at: Some(OffsetDateTime::now_utc() + std::time::Duration::from_secs(3600)),
        base_url: Some(base_url),
        account_id: None,
    }
}

async fn factory_provider(
    model: &str,
    access: &str,
    base_url: String,
) -> (TempDir, Arc<KeychainCredentialStore>, Box<dyn Provider>) {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(KeychainCredentialStore::new(
        Box::new(FakeKeyringBackend::new()),
        dir.path().to_path_buf(),
    ));
    store
        .write("github-copilot", &oauth_credential(access, base_url))
        .await
        .unwrap();
    let resolver = CredentialResolver::new(store.clone(), Arc::new(|_: &str| None));
    let mut config = OpiConfig::default();
    config.defaults.model = format!("github-copilot:{model}");
    let provider = build_provider_with_oauth(
        &config,
        &resolver,
        &OAuthProviderRegistry::registry_with_builtins(),
    )
    .await
    .expect("offline Copilot provider");
    (dir, store, provider)
}

fn request(model: &str) -> Request {
    Request {
        model: format!("github-copilot:{model}"),
        system: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "hello".into(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

async fn mount_stream(server: &MockServer, request_path: &str, status: u16) {
    Mock::given(method("POST"))
        .and(path(request_path))
        .respond_with(
            ResponseTemplate::new(status)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(server)
        .await;
}

async fn drain(provider: &dyn Provider, request: Request) -> Option<ProviderError> {
    let mut stream = provider.stream(request);
    let mut error = None;
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) if event.is_terminal() => break,
            Ok(_) => {}
            Err(found) => {
                error = Some(found);
                break;
            }
        }
    }
    error
}

async fn assert_route(model: &str, request_path: &str) {
    let server = MockServer::start().await;
    mount_stream(&server, request_path, 200).await;
    let (_dir, _store, provider) =
        factory_provider(model, "copilot-route-token", server.uri()).await;
    let _ = drain(&*provider, request(model)).await;
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), request_path);
    assert_eq!(
        requests[0]
            .headers
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap(),
        "Bearer copilot-route-token"
    );
    assert!(requests[0].headers.get("x-api-key").is_none());
}

#[tokio::test]
async fn github_copilot_anthropic_model_posts_v1_messages_with_bearer() {
    assert_route("claude-sonnet-4.5", "/v1/messages").await;
}

#[tokio::test]
async fn github_copilot_chat_model_posts_chat_completions() {
    assert_route("gpt-4.1", "/chat/completions").await;
}

#[tokio::test]
async fn github_copilot_responses_model_posts_responses() {
    assert_route("gpt-5.4", "/responses").await;
}

#[tokio::test]
async fn github_copilot_headers_match_reviewed_static_contract() {
    for (model, request_path) in [
        ("claude-sonnet-4.5", "/v1/messages"),
        ("gpt-4.1", "/chat/completions"),
        ("gpt-5.4", "/responses"),
    ] {
        let server = MockServer::start().await;
        mount_stream(&server, request_path, 200).await;
        let (_dir, _store, provider) =
            factory_provider(model, "copilot-header-token", server.uri()).await;
        let _ = drain(&*provider, request(model)).await;
        let requests = server.received_requests().await.unwrap();
        let headers = &requests[0].headers;
        for (name, expected) in github_copilot_static_headers() {
            assert_eq!(
                headers.get(&name).unwrap().to_str().unwrap(),
                expected,
                "{model} {name}"
            );
        }
        assert_eq!(
            headers.get("Openai-Intent").unwrap().to_str().unwrap(),
            "conversation-edits"
        );
    }

    let server = MockServer::start().await;
    mount_stream(&server, "/chat/completions", 200).await;
    let (_dir, _store, provider) =
        factory_provider("gpt-4.1", "copilot-header-token", server.uri()).await;
    for name in [
        "User-Agent",
        "Editor-Version",
        "Editor-Plugin-Version",
        "Copilot-Integration-Id",
        "X-Initiator",
        "Openai-Intent",
        "Copilot-Vision-Request",
    ] {
        let mut overridden = request("gpt-4.1");
        overridden
            .extra_headers
            .push((name.into(), "malicious-override".into()));
        assert!(
            matches!(
                drain(&*provider, overridden).await,
                Some(ProviderError::RequestFailed(_))
            ),
            "{name} must remain provider-managed"
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn github_copilot_initiator_tracks_last_user_or_agent_message() {
    let server = MockServer::start().await;
    for request_path in ["/v1/messages", "/chat/completions", "/responses"] {
        mount_stream(&server, request_path, 200).await;
    }
    let (_dir, _store, provider) =
        factory_provider("gpt-4.1", "copilot-initiator-token", server.uri()).await;
    let _ = drain(&*provider, request("claude-sonnet-4.5")).await;

    let mut assistant = request("gpt-4.1");
    assistant
        .messages
        .push(Message::Assistant(opi_ai::test_support::base_assistant()));
    let _ = drain(&*provider, assistant).await;

    let mut tool = request("gpt-5.4");
    tool.messages.push(Message::ToolResult(ToolResultMessage {
        tool_call_id: "call-1".into(),
        tool_name: "read".into(),
        content: vec![OutputContent::Text {
            text: "done".into(),
        }],
        details: None,
        is_error: false,
        truncated: false,
        timestamp_ms: 0,
    }));
    let _ = drain(&*provider, tool).await;

    let requests = server.received_requests().await.unwrap();
    let initiators: BTreeMap<_, _> = requests
        .iter()
        .map(|request| {
            (
                request.url.path().to_owned(),
                request
                    .headers
                    .get("X-Initiator")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned(),
            )
        })
        .collect();
    assert_eq!(initiators["/v1/messages"], "user");
    assert_eq!(initiators["/chat/completions"], "agent");
    assert_eq!(initiators["/responses"], "agent");
}

#[tokio::test]
async fn github_copilot_vision_header_covers_user_and_tool_result_images() {
    let server = MockServer::start().await;
    for request_path in ["/v1/messages", "/chat/completions", "/responses"] {
        mount_stream(&server, request_path, 200).await;
    }
    let (_dir, _store, provider) =
        factory_provider("gpt-4.1", "copilot-vision-token", server.uri()).await;

    let _ = drain(&*provider, request("claude-sonnet-4.5")).await;
    let mut user_image = request("gpt-4.1");
    user_image.messages = vec![Message::User(UserMessage {
        content: vec![InputContent::Image {
            source: ImageSource::Base64 {
                data: "AA==".into(),
            },
            media_type: MediaType::Png,
        }],
        timestamp_ms: 0,
    })];
    let _ = drain(&*provider, user_image).await;
    let mut tool_image = request("gpt-5.4");
    tool_image
        .messages
        .push(Message::ToolResult(ToolResultMessage {
            tool_call_id: "call-image".into(),
            tool_name: "screenshot".into(),
            content: vec![OutputContent::Image {
                source: ImageSource::Base64 {
                    data: "AA==".into(),
                },
                media_type: MediaType::Png,
            }],
            details: None,
            is_error: false,
            truncated: false,
            timestamp_ms: 0,
        }));
    let _ = drain(&*provider, tool_image).await;

    let requests = server.received_requests().await.unwrap();
    let by_path: BTreeMap<_, _> = requests
        .iter()
        .map(|request| (request.url.path().to_owned(), &request.headers))
        .collect();
    assert!(
        by_path["/v1/messages"]
            .get("Copilot-Vision-Request")
            .is_none()
    );
    assert_eq!(
        by_path["/chat/completions"]
            .get("Copilot-Vision-Request")
            .unwrap()
            .to_str()
            .unwrap(),
        "true"
    );
    assert_eq!(
        by_path["/responses"]
            .get("Copilot-Vision-Request")
            .unwrap()
            .to_str()
            .unwrap(),
        "true"
    );

    let anthropic_server = MockServer::start().await;
    mount_stream(&anthropic_server, "/v1/messages", 200).await;
    let (_dir, _store, anthropic_provider) = factory_provider(
        "claude-sonnet-4.5",
        "copilot-vision-token",
        anthropic_server.uri(),
    )
    .await;
    let mut anthropic_user_image = request("claude-sonnet-4.5");
    anthropic_user_image.messages = vec![Message::User(UserMessage {
        content: vec![InputContent::Image {
            source: ImageSource::Base64 {
                data: "AA==".into(),
            },
            media_type: MediaType::Png,
        }],
        timestamp_ms: 0,
    })];
    let _ = drain(&*anthropic_provider, anthropic_user_image).await;
    let anthropic_requests = anthropic_server.received_requests().await.unwrap();
    assert_eq!(anthropic_requests.len(), 1);
    assert_eq!(
        anthropic_requests[0]
            .headers
            .get("Copilot-Vision-Request")
            .unwrap()
            .to_str()
            .unwrap(),
        "true"
    );
}

#[tokio::test]
async fn github_copilot_next_stream_observes_changed_token_and_enterprise_base_url() {
    let first_server = MockServer::start().await;
    let second_server = MockServer::start().await;
    mount_stream(&first_server, "/chat/completions", 200).await;
    mount_stream(&second_server, "/chat/completions", 200).await;
    let (_dir, store, provider) =
        factory_provider("gpt-4.1", "old-token", first_server.uri()).await;
    let _ = drain(&*provider, request("gpt-4.1")).await;
    store
        .write(
            "github-copilot",
            &oauth_credential("new-token", second_server.uri()),
        )
        .await
        .unwrap();
    let _ = drain(&*provider, request("gpt-4.1")).await;

    let first = first_server.received_requests().await.unwrap();
    let second = second_server.received_requests().await.unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert_eq!(
        first[0]
            .headers
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap(),
        "Bearer old-token"
    );
    assert_eq!(
        second[0]
            .headers
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap(),
        "Bearer new-token"
    );
}

#[tokio::test]
async fn github_copilot_401_and_403_are_revoked_on_every_wire() {
    for status in [401, 403] {
        for (model, request_path) in [
            ("claude-sonnet-4.5", "/v1/messages"),
            ("gpt-4.1", "/chat/completions"),
            ("gpt-5.4", "/responses"),
        ] {
            let server = MockServer::start().await;
            mount_stream(&server, request_path, status).await;
            let (_dir, _store, provider) =
                factory_provider(model, "revoked-token", server.uri()).await;
            let error = drain(&*provider, request(model))
                .await
                .expect("auth-invalid response");
            assert!(
                matches!(
                    error,
                    ProviderError::CredentialRevoked { ref provider_id }
                        if provider_id == "github-copilot"
                ),
                "{model} status {status}: {error:?}"
            );
            assert!(!error.is_retryable());
            assert_eq!(
                server.received_requests().await.unwrap().len(),
                1,
                "{model} status {status}: no follow-up request should fire"
            );
        }
    }
}
