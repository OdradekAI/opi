//! OpenRouter Chat lifecycle and registration assertions.

use super::*;
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

use opi_ai::AuthProvenanceSource;
use opi_ai::auth::AuthScheme;
use opi_ai::http::HttpClient;
use opi_ai::models::Models;

// ---------------------------------------------------------------------------
// Phase 12 task 12.3 — OpenRouter inherits the shared compat profile path
// ---------------------------------------------------------------------------

#[test]
fn openrouter_profile_inherits_shared_compat_flags() {
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};

    // The profile's strict capability enables explicit per-tool intent;
    // unconstrained tools retain the shared Chat default.
    let input_schema = serde_json::json!({
        "type": "object",
        "properties": {"path": {"type": "string"}}
    });
    let build_provider = |strict_tool_schema| {
        OpenAiChatProvider::new_for_profile(
            "https://openrouter.example.com".into(),
            "openrouter".into(),
            CompatConfig {
                instruction_role: Some(opi_ai::model_info::ChatInstructionRole::Developer),
                strict_tool_schema,
                ..Default::default()
            },
            vec![
                ("HTTP-Referer".into(), "https://myapp.example.com".into()),
                ("X-Title".into(), "my-app".into()),
            ],
            vec![],
        )
    };
    let build_request = |preference: Option<StrictPreference>| Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openrouter:openai/gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("You are helpful.".into()),
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "list files".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![ToolDef {
                name: "list_dir".into(),
                description: "list a directory".into(),
                input_schema: input_schema.clone(),
                constrained_sampling: preference
                    .map(|strict| ConstrainedSampling::JsonSchema { strict }),
            }],
        })
        .expect("valid context declarations"),
        max_tokens: Some(1024),
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
    };
    for strict_tool_schema in [false, true] {
        for preference in [
            None,
            Some(StrictPreference::Prefer),
            Some(StrictPreference::Require),
        ] {
            let provider = build_provider(strict_tool_schema);
            let request = build_request(preference);
            let original_context = request.context.clone();
            let body = provider.build_request_body(&request);
            assert_eq!(request.context, original_context);
            if !strict_tool_schema && preference == Some(StrictPreference::Require) {
                assert!(matches!(body, Err(ProviderError::Config(_))));
                continue;
            }
            let body = body.unwrap();
            let messages = body["messages"].as_array().unwrap();
            assert_eq!(
                messages[0]["role"], "developer",
                "OpenRouter inherits developer-role override from the shared compat path"
            );
            let function = &body["tools"][0]["function"];
            let expected_strict =
                strict_tool_schema.then(|| serde_json::json!(preference.is_some()));
            assert_eq!(function.get("strict"), expected_strict.as_ref());
            if strict_tool_schema && preference.is_some() {
                let parameters = &function["parameters"];
                assert_eq!(parameters["additionalProperties"], false);
                assert_eq!(parameters["required"], serde_json::json!(["path"]));
                assert_eq!(
                    parameters["properties"]["path"]["anyOf"],
                    serde_json::json!([{"type": "string"}, {"type": "null"}])
                );
            } else {
                assert_eq!(function["parameters"], input_schema);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Provider stream cancellation (Phase 12 task 12.7 DoD clause 6)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_cancellation_aborts_before_completion() {
    // The CancellationToken is threaded into the OpenRouter adapter's HTTP
    // body-stream loop (inherited from the shared OpenAI-compat path; see
    // api/openai_completions.rs `cancel.cancelled()` select arm). Cancelling while the
    // stream is open must terminate it before the delayed terminal chunk
    // arrives, proving the inherited adapter path observes cancellation.
    let server = spawn_stalled_openrouter_server().await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::providers::openrouter::openrouter_provider(Some(server));
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openrouter:openai/gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(1024),
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: cancel.clone(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());

    let first = stream
        .next()
        .await
        .expect("stream should produce at least one event")
        .expect("first event should be valid");
    assert!(matches!(first, AssistantStreamEvent::Start { .. }));
    cancel.cancel();

    let next = tokio::time::timeout(std::time::Duration::from_millis(200), stream.next())
        .await
        .expect("stream must surface cancellation before the delayed terminal fixture completes");
    match next {
        Some(Err(cause))
            if cause.category() == opi_ai::provider::ProviderErrorCategory::Cancelled =>
        { /* typed cancellation, as contracted */ }
        other => panic!("expected Err(ProviderError::Cancelled) on cancellation, got: {other:?}"),
    }
}

#[tokio::test]
async fn custom_chat_replacement_keeps_its_resolver_after_store_attachment() {
    use opi_ai::credential::{
        BoxAuthFuture, Credential, CredentialSource, CredentialStore, CredentialStoreError,
    };
    use std::sync::Arc;

    struct NoAmbient;
    impl opi_ai::auth::AuthContext for NoAmbient {
        fn env(&self, _: &str) -> Result<Option<String>, opi_ai::auth::AuthContextError> {
            Ok(None)
        }
        fn file_exists(&self, _: &str) -> Result<bool, opi_ai::auth::AuthContextError> {
            Ok(false)
        }
    }
    struct Store(AtomicU32);
    impl CredentialStore for Store {
        fn read<'a>(
            &'a self,
            _: &'a str,
        ) -> BoxAuthFuture<'a, Result<Option<Credential>, CredentialStoreError>> {
            self.0.fetch_add(1, AtomicOrdering::SeqCst);
            Box::pin(async {
                Ok(Some(Credential::ApiKey(
                    secrecy::SecretString::from("builtin-store-key").into(),
                )))
            })
        }
        fn write<'a>(
            &'a self,
            _: &'a str,
            _: &'a Credential,
        ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
            panic!("fixture never writes")
        }
        fn delete<'a>(&'a self, _: &'a str) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
            panic!("fixture never deletes")
        }
        fn probe<'a>(&'a self, _: &'a str) -> BoxAuthFuture<'a, CredentialSource> {
            Box::pin(async { CredentialSource::Absent })
        }
    }

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string("data: {\"id\":\"fixture\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n"))
        .expect(1).mount(&server).await;
    let mut models = Models::new();
    let context: Arc<dyn opi_ai::auth::AuthContext> = Arc::new(NoAmbient);
    models.set_auth_context(context.clone());
    opi_ai::providers::collection::register_builtin(
        &mut models,
        context,
        Arc::new(HttpClient::new()),
        &Default::default(),
    )
    .unwrap();
    let info = opi_ai::ModelInfo::new(
        "custom",
        "Custom",
        opi_ai::WireApi::OpenAiCompletions,
        opi_ai::ModelCapabilities::new(8192, 1024).with_streaming(true),
    )
    .with_base_url(server.uri());
    let api = opi_ai::api::openai_completions::OpenAiCompletionsChatApi::new(
        Arc::new(opi_ai::providers::openrouter::openrouter_provider(Some(
            server.uri(),
        ))),
        info.clone(),
    )
    .unwrap();
    let handle = models
        .register_chat_api(
            Arc::new(api),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from("custom-route-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let store = Arc::new(Store(AtomicU32::new(0)));
    models.set_credential_store(store.clone());
    let request = Request {
        projection: Default::default(),
        model: "openrouter:custom".into(),
        context: opi_ai::transcript::normalize_context(Default::default()).unwrap(),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        callbacks: Default::default(),
        telemetry_context: None,
        transport: Default::default(),
    };
    models
        .complete(
            opi_ai::ChatModelRecord::new("openrouter", info),
            request,
            handle.options(Default::default()),
        )
        .await
        .unwrap();
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests[0].headers.get("authorization").unwrap(),
        "Bearer custom-route-key"
    );
    assert_eq!(
        store.0.load(AtomicOrdering::SeqCst),
        0,
        "custom route must not read replaced builtin sources"
    );
}
