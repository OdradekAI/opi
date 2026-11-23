use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_util::StreamExt;
use opi_ai::catalog::{Catalog, CatalogOperation};
use opi_ai::credential::{Credential, CredentialStore};
use opi_ai::message::{
    AssistantContent, ImageSource, InputContent, MediaType, Message, OutputContent, ToolCall,
    ToolResultMessage, UserMessage,
};
use opi_ai::provider::{CacheRetention, ProviderError, Request, ThinkingConfig};
use opi_ai::{AuthProvenanceSource, CompatMetadata, Models, ThinkingLevel, WireApi};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::credential_store::{
    AuthSource, CredentialResolver, FakeKeyringBackend, KeychainCredentialStore,
};
use opi_coding_agent::github_copilot::{github_copilot_catalog, github_copilot_static_headers};
use opi_coding_agent::oauth::OAuthProviderRegistry;
use opi_coding_agent::provider_factory::{
    build_collection_for_listing_with_store, build_provider_with_oauth,
};
use secrecy::SecretString;
use tempfile::TempDir;
use time::OffsetDateTime;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PROVIDER: &str = "github-copilot";
const RETIRED_MODELS: [&str; 20] = [
    "claude-opus-4.5",
    "claude-opus-4.6",
    "claude-opus-4.7",
    "claude-sonnet-4",
    "claude-sonnet-4.5",
    "gemini-2.5-pro",
    "gemini-3-flash-preview",
    "gemini-3.1-pro-preview",
    "gemini-3.5-flash",
    "gemini-3.6-flash",
    "gemini-3.7-flash",
    "gpt-4.1",
    "gpt-5-mini",
    "gpt-5.2",
    "gpt-5.2-codex",
    "gpt-5.4",
    "gpt-5.4-mini",
    "gpt-5.5",
    "kimi-k2.7-code",
    "mai-code-1-flash-picker",
];

#[tokio::test]
async fn github_copilot_catalog_and_factory_match_the_published_chat_records() {
    let catalog = Catalog::builtin().unwrap();
    let reviewed: Vec<_> = catalog
        .models("github-copilot", CatalogOperation::Chat)
        .unwrap()
        .map(|record| record.to_chat_model_info().unwrap())
        .collect();
    let listed = github_copilot_catalog();
    assert_eq!(listed.len(), 27, "CLI Copilot catalog count");
    assert_eq!(reviewed.len(), 27, "SDK Copilot catalog count");
    assert_eq!(listed, reviewed);

    let mut by_wire = BTreeMap::new();
    for model in &reviewed {
        *by_wire.entry(model.wire_api).or_insert(0) += 1;
    }
    assert_eq!(
        by_wire,
        BTreeMap::from([
            (WireApi::OpenAiResponses, 14),
            (WireApi::AnthropicMessages, 10),
            (WireApi::OpenAiCompletions, 3),
        ]),
    );

    let server = MockServer::start().await;
    let (_dir, _store, runtime) =
        factory_provider("gpt-6.1-terra", "fixture-token", server.uri()).await;
    let builtin = sdk_models(server.uri(), None).await;
    assert_eq!(runtime.get_models(Some(PROVIDER)), reviewed);
    assert_eq!(builtin.get_models(Some(PROVIDER)), reviewed);
    for model in reviewed {
        assert_eq!(
            runtime.get_model(PROVIDER, &model.id).as_ref(),
            Some(&model)
        );
        assert_eq!(
            builtin.get_model(PROVIDER, &model.id).as_ref(),
            Some(&model)
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

fn oauth_credential(
    access: &str,
    base_url: String,
    available_model_ids: Option<Vec<String>>,
) -> Credential {
    Credential::OAuthToken {
        access: SecretString::new(access.to_owned().into_boxed_str()),
        refresh: SecretString::new("offline-refresh".to_owned().into_boxed_str()),
        expires_at: Some(OffsetDateTime::now_utc() + std::time::Duration::from_secs(3600)),
        base_url: Some(base_url),
        account_id: None,
        client_id: None,
        enterprise_domain: None,
        available_model_ids,
        scopes: None,
    }
}

async fn sdk_models(base_url: String, available_ids: Option<Vec<String>>) -> Models {
    use opi_ai::auth::{AuthContext, AuthContextError};
    use opi_ai::mutation::InMemoryCredentialStore;
    use opi_ai::providers::collection::{BuiltinOverrides, register_builtin};

    struct EmptySources;
    impl AuthContext for EmptySources {
        fn env(&self, _: &str) -> Result<Option<String>, AuthContextError> {
            Ok(None)
        }

        fn file_exists(&self, _: &str) -> Result<bool, AuthContextError> {
            Ok(false)
        }
    }

    let store = Arc::new(InMemoryCredentialStore::new());
    store
        .write(
            PROVIDER,
            &oauth_credential("sdk-copilot-token", base_url, available_ids),
        )
        .await
        .unwrap();
    let mut models = Models::new();
    models.set_credential_store(store);
    register_builtin(
        &mut models,
        Arc::new(EmptySources),
        Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    models
}

async fn factory_provider(
    model: &str,
    access: &str,
    base_url: String,
) -> (TempDir, Arc<KeychainCredentialStore>, Models) {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(KeychainCredentialStore::new(
        Box::new(FakeKeyringBackend::new()),
        dir.path().to_path_buf(),
    ));
    store
        .write(PROVIDER, &oauth_credential(access, base_url, None))
        .await
        .unwrap();
    let resolver = CredentialResolver::new(store.clone(), Arc::new(|_: &str| None));
    let oauth_registry = OAuthProviderRegistry::registry_with_builtins();
    let mut config = OpiConfig::default();
    config.defaults.model = format!("github-copilot:{model}");
    let provider = build_provider_with_oauth(&config, &resolver, &oauth_registry)
        .await
        .expect("offline Copilot provider");
    // Phase 17.5: auth moved off the provider object onto the collection route.
    // Register the store-backed OAuth resolver so prepare_call resolves the live
    // token + enterprise base_url from the keychain on each logical call.
    let auth_resolver: Arc<dyn opi_ai::AuthResolver> = Arc::new(AuthSource::Store {
        resolver: Arc::new(resolver),
        provider_id: "github-copilot".into(),
        oauth: oauth_registry
            .lookup("github-copilot")
            .expect("github-copilot OAuth provider registered"),
    });
    let mut collection = Models::new();
    collection
        .register_route(
            provider,
            auth_resolver,
            AuthProvenanceSource::OAuth {
                kind: "github-copilot".into(),
            },
            CompatMetadata::default(),
        )
        .expect("register copilot route");
    (dir, store, collection)
}

fn request(model: &str) -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: format!("github-copilot:{model}"),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

async fn mount_stream(server: &MockServer, request_path: &str, status: u16) {
    let body = if status != 200 {
        ""
    } else if request_path.ends_with("/chat/completions") {
        concat!(
            "data: {\"id\":\"x\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hi\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"x\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n"
        )
    } else if request_path.ends_with("/responses") {
        concat!(
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hi\"}\n\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"r\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1,\"total_tokens\":2}}}\n\n"
        )
    } else {
        assert!(request_path.ends_with("/v1/messages"));
        concat!(
            "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"model\":\"fixture\",\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n",
            "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hi\"}}\n\n",
            "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":1}}\n\n",
            "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n"
        )
    };
    Mock::given(method("POST"))
        .and(path(request_path))
        .respond_with(
            ResponseTemplate::new(status)
                .set_body_string(body)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(server)
        .await;
}

#[tokio::test]
async fn github_copilot_published_models_drive_requests_through_both_factories() {
    let sdk = MockServer::start().await;
    let cli = MockServer::start().await;
    let builtin = sdk_models(format!("{}/enterprise", sdk.uri()), None).await;
    let (_dir, _store, runtime) = factory_provider(
        "gpt-6.1-terra",
        "cli-copilot-token",
        format!("{}/enterprise", cli.uri()),
    )
    .await;

    let cases = [
        (
            "gpt-6.1-terra",
            "/responses",
            Some(ThinkingLevel::Max),
            Some("max"),
        ),
        (
            "gpt-5.4-nano",
            "/responses",
            Some(ThinkingLevel::Minimal),
            Some("low"),
        ),
        (
            "gpt-5.4-nano",
            "/responses",
            Some(ThinkingLevel::Max),
            Some("xhigh"),
        ),
        ("claude-sonnet-4.6", "/v1/messages", None, None),
        ("gemini-4-argon", "/chat/completions", None, None),
    ];
    for (server, models, authorization) in [
        (&sdk, &builtin, "Bearer sdk-copilot-token"),
        (&cli, &runtime, "Bearer cli-copilot-token"),
    ] {
        for suffix in ["/responses", "/v1/messages", "/chat/completions"] {
            mount_stream(server, &format!("/enterprise{suffix}"), 200).await;
        }

        for (model, _, thinking, _) in cases {
            let mut call = request(model);
            call.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                messages: vec![Message::User(UserMessage {
                    content: vec![InputContent::Image {
                        source: ImageSource::Base64 {
                            data: "AA==".into(),
                        },
                        media_type: MediaType::Png,
                    }],
                    timestamp_ms: 0,
                })],
                ..Default::default()
            })
            .unwrap();
            if let Some(level) = thinking {
                call.thinking = ThinkingConfig {
                    enabled: true,
                    budget_tokens: None,
                    level,
                };
            }
            let record = models.get_model(PROVIDER, model).unwrap();
            let result = models
                .complete_simple(
                    opi_ai::ChatModelRecord::new(PROVIDER, record),
                    call,
                    Default::default(),
                )
                .await
                .unwrap_or_else(|error| panic!("{authorization} {model}: {error:?}"));
            assert_eq!(result.stop_reason, opi_ai::stream::StopReason::Stop);
        }

        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), cases.len());
        for (request, (model, suffix, _, effort)) in sent.iter().zip(cases) {
            assert_eq!(request.url.path(), format!("/enterprise{suffix}"));
            assert_eq!(request.headers["authorization"], authorization);
            assert!(request.headers.get("x-api-key").is_none());
            assert_eq!(
                request
                    .headers
                    .get("x-initiator")
                    .and_then(|value| value.to_str().ok()),
                Some("user"),
                "{authorization} {model}"
            );
            assert_eq!(request.headers["openai-intent"], "conversation-edits");
            assert_eq!(request.headers["copilot-vision-request"], "true");
            for (name, value) in github_copilot_static_headers() {
                assert_eq!(request.headers[name.as_str()], value);
            }
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            assert_eq!(body["model"], model);
            assert_eq!(body["reasoning"]["effort"].as_str(), effort, "{model}");
        }
    }
}

#[tokio::test]
async fn github_copilot_retired_models_are_rejected_and_cannot_be_restored_by_account_metadata() {
    let server = MockServer::start().await;
    let (_dir, store, runtime) =
        factory_provider("gpt-6.1-terra", "cli-copilot-token", server.uri()).await;
    let listing = build_collection_for_listing_with_store(&OpiConfig::default(), store.as_ref())
        .await
        .unwrap();

    let allowed = ["gpt-6.1-terra"].into_iter().chain(RETIRED_MODELS);
    let builtin = sdk_models(server.uri(), Some(allowed.map(str::to_owned).collect())).await;
    let available = builtin.get_available(Some(PROVIDER)).await.unwrap();
    assert_eq!(available.len(), 1);
    assert_eq!(available[0].0, PROVIDER);
    assert_eq!(
        available[0].1,
        builtin.get_model(PROVIDER, "gpt-6.1-terra").unwrap()
    );
    assert_eq!(builtin.get_models(Some(PROVIDER)).len(), 27);

    let catalog = Catalog::builtin().unwrap();
    for id in RETIRED_MODELS {
        assert!(
            catalog
                .model(PROVIDER, CatalogOperation::Chat, id)
                .is_none(),
            "{id}"
        );
        assert!(listing.get_model(PROVIDER, id).is_none(), "{id}");
        for models in [&builtin, &runtime] {
            let call = request(id);
            let spec = call.model.clone();
            assert!(models.get_model(PROVIDER, id).is_none(), "{id}");
            assert!(
                matches!(
                    models.prepare_call(&spec, call).await,
                    Err(opi_ai::ModelsError::Registry(opi_ai::RegistryError::UnknownModel {
                        provider, model,
                    })) if provider == PROVIDER && model == id
                ),
                "{spec} must fail during registry lookup"
            );
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

async fn drain(collection: &Models, request: Request) -> Option<ProviderError> {
    let spec = request.model.clone();
    let prepared = collection
        .prepare_call(&spec, request)
        .await
        .expect("prepare_call resolves the copilot route");
    let mut stream = prepared.start_attempt().expect("start_attempt");
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
    let (_dir, _store, collection) =
        factory_provider(model, "copilot-route-token", server.uri()).await;
    let _ = drain(&collection, request(model)).await;
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
    assert_route("claude-sonnet-4.6", "/v1/messages").await;
}

#[tokio::test]
async fn factory_built_copilot_anthropic_models_apply_catalog_compatibility() {
    for model in ["claude-opus-5", "claude-opus-4.8"] {
        let server = MockServer::start().await;
        mount_stream(&server, "/v1/messages", 200).await;
        let (_dir, _store, collection) =
            factory_provider(model, "copilot-compat-token", server.uri()).await;
        let mut captured = request(model);
        captured.temperature = Some(0.3);
        captured.thinking = ThinkingConfig {
            enabled: true,
            budget_tokens: Some(4096),
            level: ThinkingLevel::Medium,
        };
        let _ = drain(&collection, captured).await;

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1, "{model}");
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        // Task 20.39 replacement proof: adaptive thinking carries the
        // display default (summarized) per pi buildParams.
        assert_eq!(
            body["thinking"],
            serde_json::json!({"type": "adaptive", "display": "summarized"})
        );
        assert!(
            body.get("temperature").is_none(),
            "{model} must omit unsupported temperature"
        );
    }

    let server = MockServer::start().await;
    mount_stream(&server, "/v1/messages", 200).await;
    let model = "claude-haiku-4.5";
    let (_dir, _store, collection) =
        factory_provider(model, "copilot-compat-token", server.uri()).await;
    let mut captured = request(model);
    captured.temperature = Some(0.3);
    captured.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: Some(4096),
        level: ThinkingLevel::Medium,
    };
    let _ = drain(&collection, captured).await;

    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["thinking"]["type"], "enabled");
    assert_eq!(body["thinking"]["budget_tokens"], 4096);
    assert!(body.get("temperature").is_none());

    let mut without_thinking = request(model);
    without_thinking.temperature = Some(0.3);
    let _ = drain(&collection, without_thinking).await;
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    let body: serde_json::Value = serde_json::from_slice(&requests[1].body).unwrap();
    assert_eq!(body["temperature"], 0.3);
}

#[tokio::test]
async fn github_copilot_chat_model_posts_chat_completions() {
    assert_route("gemini-4-argon", "/chat/completions").await;
}

#[tokio::test]
async fn github_copilot_responses_model_posts_responses() {
    assert_route("gpt-6.1-terra", "/responses").await;
}

#[tokio::test]
async fn github_copilot_headers_match_reviewed_static_contract() {
    for (model, request_path) in [
        ("claude-sonnet-4.6", "/v1/messages"),
        ("gemini-4-argon", "/chat/completions"),
        ("gpt-6.1-terra", "/responses"),
    ] {
        let server = MockServer::start().await;
        mount_stream(&server, request_path, 200).await;
        let (_dir, _store, collection) =
            factory_provider(model, "copilot-header-token", server.uri()).await;
        let _ = drain(&collection, request(model)).await;
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
}

#[tokio::test]
async fn github_copilot_managed_headers_are_rejected_before_callbacks_and_dispatch() {
    for (model, request_path) in [
        ("claude-sonnet-4.6", "/v1/messages"),
        ("gemini-4-argon", "/chat/completions"),
        ("gpt-6.1-terra", "/responses"),
    ] {
        let server = MockServer::start().await;
        mount_stream(&server, request_path, 200).await;
        let (_dir, _store, collection) =
            factory_provider(model, "copilot-header-token", server.uri()).await;
        for name in [
            "User-Agent",
            "Editor-Version",
            "Editor-Plugin-Version",
            "Copilot-Integration-Id",
            "X-Initiator",
            "Openai-Intent",
            "Copilot-Vision-Request",
        ] {
            let transforms = Arc::new(AtomicUsize::new(0));
            let observed = transforms.clone();
            let mut overridden = request(model);
            overridden
                .extra_headers
                .push((name.into(), "override".into()));
            overridden.callbacks =
                overridden
                    .callbacks
                    .with_header_transform(Arc::new(move |headers| {
                        observed.fetch_add(1, Ordering::Relaxed);
                        Ok(headers.to_vec())
                    }));

            let error = drain(&collection, overridden)
                .await
                .expect("managed header refusal");
            let ProviderError::RequestFailed(summary) = accepted_projection_cause(&error) else {
                panic!("{model} {name}: expected a request failure, got {error:?}");
            };
            assert_eq!(
                summary.to_string(),
                format!("request header '{name}' is reserved for GitHub Copilot"),
                "{model}"
            );
            assert_eq!(
                transforms.load(Ordering::Relaxed),
                0,
                "{model} {name} must fail before the header transform"
            );
        }
        assert!(
            server.received_requests().await.unwrap().is_empty(),
            "{model}"
        );
    }
}

fn call_assistant(id: &str, name: &str) -> Message {
    Message::Assistant(opi_ai::message::AssistantMessage {
        projection: None,
        content: vec![AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: id.into(),
                name: name.into(),
                arguments: "{}".into(),
                arguments_error: None,
                signature: None,
            },
        }],
        api: opi_ai::ApiKind::OpenAi,
        provider: "github-copilot".into(),
        model: "gemini-4-argon".into(),
        response_model: None,
        wire: None,
        response_id: None,
        usage: Default::default(),
        stop_reason: opi_ai::stream::StopReason::ToolUse,
        error_message: None,
        timestamp_ms: 0,
        diagnostics: Vec::new(),
        duration_ms: None,
        raw_stop_reason: None,
        provider_thinking_level: None,

        deferred: None,
    })
}

#[tokio::test]
async fn github_copilot_initiator_tracks_last_user_or_agent_message() {
    let server = MockServer::start().await;
    for request_path in ["/v1/messages", "/chat/completions", "/responses"] {
        mount_stream(&server, request_path, 200).await;
    }
    let (_dir, _store, collection) =
        factory_provider("gemini-4-argon", "copilot-initiator-token", server.uri()).await;
    let _ = drain(&collection, request("claude-sonnet-4.6")).await;

    let mut assistant = request("gemini-4-argon");
    let mut assistant_messages = assistant.context.into_messages();
    assistant_messages.push(Message::Assistant(opi_ai::test_support::base_assistant()));
    assistant.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        messages: assistant_messages,
        ..Default::default()
    })
    .expect("valid context declarations");
    let _ = drain(&collection, assistant).await;

    let mut tool = request("gpt-6.1-terra");
    let mut tool_messages = tool.context.into_messages();
    tool_messages.push(call_assistant("call-1", "read"));
    tool_messages.push(Message::ToolResult(ToolResultMessage {
        not_executed: None,
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
    tool.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        messages: tool_messages,
        ..Default::default()
    })
    .expect("valid context declarations");
    let _ = drain(&collection, tool).await;

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
    let (_dir, _store, collection) =
        factory_provider("gemini-4-argon", "copilot-vision-token", server.uri()).await;

    let _ = drain(&collection, request("claude-sonnet-4.6")).await;
    let mut user_image = request("gemini-4-argon");
    user_image.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Image {
                source: ImageSource::Base64 {
                    data: "AA==".into(),
                },
                media_type: MediaType::Png,
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
    })
    .expect("valid context declarations");
    let _ = drain(&collection, user_image).await;
    let mut tool_image = request("gpt-6.1-terra");
    let mut tool_image_messages = tool_image.context.into_messages();
    tool_image_messages.push(call_assistant("call-image", "screenshot"));
    tool_image_messages.push(Message::ToolResult(ToolResultMessage {
        not_executed: None,
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
    tool_image.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        messages: tool_image_messages,
        ..Default::default()
    })
    .expect("valid context declarations");
    let _ = drain(&collection, tool_image).await;

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
    let (_dir, _store, anthropic_collection) = factory_provider(
        "claude-sonnet-4.6",
        "copilot-vision-token",
        anthropic_server.uri(),
    )
    .await;
    let mut anthropic_user_image = request("claude-sonnet-4.6");
    anthropic_user_image.context =
        opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Image {
                    source: ImageSource::Base64 {
                        data: "AA==".into(),
                    },
                    media_type: MediaType::Png,
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations");
    let _ = drain(&anthropic_collection, anthropic_user_image).await;
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
    let (_dir, store, collection) =
        factory_provider("gemini-4-argon", "old-token", first_server.uri()).await;
    let _ = drain(&collection, request("gemini-4-argon")).await;
    store
        .write(
            "github-copilot",
            &oauth_credential("new-token", second_server.uri(), None),
        )
        .await
        .unwrap();
    let _ = drain(&collection, request("gemini-4-argon")).await;

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
            ("claude-sonnet-4.6", "/v1/messages"),
            ("gemini-4-argon", "/chat/completions"),
            ("gpt-6.1-terra", "/responses"),
        ] {
            let server = MockServer::start().await;
            mount_stream(&server, request_path, status).await;
            let (_dir, _store, collection) =
                factory_provider(model, "revoked-token", server.uri()).await;
            let error = drain(&collection, request(model))
                .await
                .expect("auth-invalid response");
            assert!(
                matches!(
                    accepted_projection_cause(&error),
                    ProviderError::CredentialRevoked { provider_id }
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

fn accepted_projection_cause(
    error: &opi_ai::provider::ProviderError,
) -> &opi_ai::provider::ProviderError {
    let opi_ai::provider::ProviderError::StreamFailure(failure) = error else {
        panic!("accepted projection retains its cause and facts: {error:?}");
    };
    assert!(failure.message().projection.is_some());
    failure.cause()
}
