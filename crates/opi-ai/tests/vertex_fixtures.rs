//! Google Vertex AI provider fixture tests (task 3.3).
//!
//! Verifies: Vertex-specific URL formatting, OAuth token auth, secret redaction,
//! SSE event mapping (reuses Gemini wire format), model resolution, and
//! request body construction. No live Google Cloud calls.

use futures_util::StreamExt;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{
    CacheRetention, EventStream, Provider, ProviderError, Request, ThinkingConfig,
};
use opi_ai::registry::ProviderRegistry;
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::vertex::VertexProvider;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn direct_cause(error: &ProviderError) -> &ProviderError {
    match error {
        ProviderError::StreamFailure(failure) => failure.cause(),
        cause => cause,
    }
}

fn accepted_failure_cause<'a>(error: &'a ProviderError, request: &Request) -> &'a ProviderError {
    use sha2::Digest as _;
    let ProviderError::StreamFailure(failure) = error else {
        panic!("accepted projection must retain its failure carrier: {error:?}");
    };
    let facts = failure
        .message()
        .projection
        .as_ref()
        .expect("accepted projection is known");
    let bytes = serde_json::to_vec(request.context.messages()).unwrap();
    assert_eq!(
        facts.source_prefix.message_count,
        request.context.messages().len()
    );
    assert_eq!(
        facts.source_prefix.sha256,
        format!("{:x}", sha2::Sha256::digest(bytes))
    );
    assert_eq!(
        facts.temporal.mode,
        opi_ai::transcript::ProjectionMode::Lossless
    );
    assert!(facts.temporal.affected_ranges.is_empty());
    assert!(facts.replay.losses.is_empty());
    failure.cause()
}

fn vertex_provider() -> VertexProvider {
    VertexProvider::new("my-project".into(), "us-central1".into(), None)
}

#[tokio::test]
async fn node5_five_native_wires_reject_known_image_input_before_auth() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct Auth(Arc<AtomicUsize>);
    impl opi_ai::AuthResolver for Auth {
        fn resolve(
            &self,
        ) -> opi_ai::credential::BoxAuthFuture<'_, Result<opi_ai::ResolvedAuth, ProviderError>>
        {
            Box::pin(async move {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(opi_ai::ResolvedAuth {
                    env: Default::default(),
                    scheme: opi_ai::AuthScheme::ApiKey,
                    secret: "fixture-key".into(),
                    base_url: None,
                    account_id: None,
                    provenance: Default::default(),
                })
            })
        }
    }
    async fn check<O: Send + 'static>(
        api: Arc<dyn opi_ai::models::ChatApi<O>>,
        model: opi_ai::provider::ModelInfo,
        options: O,
    ) {
        let provider = api.provider_id().to_owned();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut models = opi_ai::Models::new();
        let handle = models
            .register_chat_api(
                api,
                Arc::new(Auth(calls.clone())),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        let mut request = lifecycle_text_request();
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            tools: vec![],
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Image {
                    source: opi_ai::message::ImageSource::Base64 {
                        data: "AA==".into(),
                    },
                    media_type: opi_ai::message::MediaType::Png,
                }],
                timestamp_ms: 0,
            })],
        })
        .unwrap();
        let call = models.complete(
            opi_ai::ChatModelRecord::new(provider, model),
            request,
            handle.options(options),
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let error = call.await.unwrap_err();
        assert!(
            matches!(error.cause(), ProviderError::UnsupportedCapability(_)),
            "{error:?}"
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "known input rejection precedes auth"
        );
    }
    let server = MockServer::start().await;
    let model = |wire| {
        opi_ai::provider::ModelInfo::new(
            "native-preauth",
            "native-preauth",
            wire,
            opi_ai::registry::ModelCapabilities::new(100_000, 1000),
        )
        .with_base_url(server.uri())
    };
    let google = model(opi_ai::WireApi::GoogleGenerativeAi);
    check(
        Arc::new(
            opi_ai::gemini::GoogleGenerativeAiChatApi::new(
                Arc::new(opi_ai::gemini::GeminiProvider::new(Some(server.uri()))),
                google.clone(),
            )
            .unwrap(),
        ),
        google,
        opi_ai::gemini::GoogleGenerativeAiOptions::default(),
    )
    .await;
    let vertex = model(opi_ai::WireApi::GoogleVertex);
    check(
        Arc::new(
            opi_ai::vertex::GoogleVertexChatApi::new(
                Arc::new(VertexProvider::new(
                    "fixture-project".into(),
                    "fixture-location".into(),
                    Some(server.uri()),
                )),
                vertex.clone(),
            )
            .unwrap(),
        ),
        vertex,
        opi_ai::vertex::GoogleVertexOptions::default(),
    )
    .await;
    let bedrock = model(opi_ai::WireApi::BedrockConverseStream);
    check(
        Arc::new(
            opi_ai::bedrock::BedrockConverseStreamChatApi::new(
                Arc::new(opi_ai::bedrock::BedrockProvider::new(
                    Some(server.uri()),
                    Arc::new(opi_ai::http::HttpClient::new()),
                )),
                bedrock.clone(),
            )
            .unwrap(),
        ),
        bedrock,
        opi_ai::bedrock::BedrockOptions::default(),
    )
    .await;
    let mistral = model(opi_ai::WireApi::MistralConversations);
    check(
        Arc::new(
            opi_ai::mistral::MistralConversationsChatApi::new(
                opi_ai::mistral::MistralConversationsProvider::new(Some(server.uri())),
                mistral.clone(),
            )
            .unwrap(),
        ),
        mistral,
        opi_ai::mistral::MistralOptions::default(),
    )
    .await;
    let pi = model(opi_ai::WireApi::PiMessages);
    check(
        Arc::new(
            opi_ai::pi_messages::PiMessagesChatApi::new(
                opi_ai::pi_messages::PiMessagesProvider::new(Some(server.uri())),
                pi.clone(),
            )
            .unwrap(),
        ),
        pi,
        opi_ai::pi_messages::PiMessagesOptions::native(None, None, false),
    )
    .await;
    assert!(server.received_requests().await.unwrap().is_empty());
}

mod builtin_routes {
    use super::*;
    use opi_ai::auth::{AuthContext, AuthContextError};
    use opi_ai::message::{AssistantContent, ToolResultMessage};
    use opi_ai::provider_collection::{BuiltinOverrides, register_builtin};
    use opi_ai::{ChatModelRecord, Models};
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Sources {
        env: BTreeMap<String, String>,
        reads: Mutex<Vec<String>>,
    }

    impl AuthContext for Sources {
        fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
            self.reads.lock().unwrap().push(name.into());
            Ok(self.env.get(name).cloned())
        }

        fn file_exists(&self, path: &str) -> Result<bool, AuthContextError> {
            self.reads.lock().unwrap().push(format!("file:{path}"));
            Ok(std::path::Path::new(path).is_file())
        }
    }

    fn fixture_response() -> ResponseTemplate {
        let raw = serde_json::json!({"candidates":[{"content":{"parts":[{"functionCall":{"id":"call-1","name":"lookup","args":{"q":"fixture"}},"thoughtSignature":"c2lnbmF0dXJl"}]},"finishReason":"STOP"}]});
        ResponseTemplate::new(200)
            .set_body_string(format!("data: {raw}\n\n"))
            .insert_header("content-type", "text/event-stream")
    }

    #[tokio::test]
    async fn node5_pinned_backend_thinking_config_is_nested_on_both_google_wires() {
        for (provider, key) in [
            ("google", "GEMINI_API_KEY"),
            ("google-vertex", "GOOGLE_CLOUD_API_KEY"),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(fixture_response())
                .expect(1)
                .mount(&server)
                .await;
            let context = Arc::new(Sources {
                env: BTreeMap::from([(key.into(), "fixture-key".into())]),
                ..Default::default()
            });
            let mut overrides = BuiltinOverrides::default();
            overrides.base_urls.insert(
                provider.into(),
                if provider == "google" {
                    format!("{}/v1beta", server.uri())
                } else {
                    server.uri()
                },
            );
            let mut models = Models::new();
            register_builtin(
                &mut models,
                context,
                Arc::new(opi_ai::http::HttpClient::new()),
                &overrides,
            )
            .unwrap();
            let model = models.get_model(provider, "gemini-2.5-flash").unwrap();
            let mut request = lifecycle_text_request();
            request.thinking = ThinkingConfig {
                enabled: true,
                level: opi_ai::ThinkingLevel::High,
                budget_tokens: Some(2048),
            };
            request.max_tokens = Some(8192);
            models
                .complete_simple(
                    ChatModelRecord::new(provider, model),
                    request,
                    Default::default(),
                )
                .await
                .unwrap();
            let sent = server.received_requests().await.unwrap();
            let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
            assert_eq!(
                body["generationConfig"]["thinkingConfig"],
                serde_json::json!({"includeThoughts":true,"thinkingBudget":2048}),
                "{provider}: {body}"
            );
            assert!(body.get("thinkingConfig").is_none());
        }
    }

    #[tokio::test]
    async fn node5_pinned_backend_vertex_complete_base_has_collection_scope() {
        #[derive(Clone)]
        struct FixedAuth(opi_ai::ResolvedAuth);
        impl opi_ai::AuthResolver for FixedAuth {
            fn resolve(
                &self,
            ) -> opi_ai::credential::BoxAuthFuture<'_, Result<opi_ai::ResolvedAuth, ProviderError>>
            {
                Box::pin(async move { Ok(self.0.clone()) })
            }
        }
        for scheme in [opi_ai::AuthScheme::ApiKey, opi_ai::AuthScheme::Bearer] {
            for (suffix, version) in [
                ("", "/v1"),
                ("/proxy", "/v1"),
                ("/v1beta1", ""),
                ("/edge/v2/nested", ""),
            ] {
                let selected = MockServer::start().await;
                let fallback = MockServer::start().await;
                for server in [&selected, &fallback] {
                    Mock::given(method("POST"))
                        .respond_with(fixture_response())
                        .mount(server)
                        .await;
                }
                let mut model = opi_ai::provider::ModelInfo::new(
                    "gemini-2.5-flash",
                    "fixture",
                    opi_ai::WireApi::GoogleVertex,
                    opi_ai::registry::ModelCapabilities::new(100_000, 1000),
                );
                model.base_url = Some(format!("{}{suffix}", selected.uri()));
                let provider = VertexProvider::new(
                    "fixture-project".into(),
                    "fixture-location".into(),
                    Some(fallback.uri()),
                )
                .with_models(vec![model.clone()]);
                let auth = opi_ai::ResolvedAuth {
                    scheme: scheme.clone(),
                    secret: "fixture-key".into(),
                    env: Default::default(),
                    base_url: Some(format!(
                        "{}/frozen/publishers/google/models",
                        fallback.uri()
                    )),
                    account_id: None,
                    provenance: Default::default(),
                };
                let mut models = Models::new();
                let handle = models
                    .register_chat_api(
                        Arc::new(
                            opi_ai::vertex::GoogleVertexChatApi::new(
                                Arc::new(provider.clone()),
                                model.clone(),
                            )
                            .unwrap(),
                        ),
                        Arc::new(FixedAuth(auth.clone())),
                        opi_ai::AuthProvenanceSource::Static,
                    )
                    .unwrap();
                models
                    .complete(
                        ChatModelRecord::new("vertex", model.clone()),
                        lifecycle_text_request(),
                        handle.options(opi_ai::vertex::GoogleVertexOptions::default()),
                    )
                    .await
                    .unwrap();
                let mut simple = Models::new();
                simple
                    .register_route(
                        Box::new(provider.clone()),
                        Arc::new(FixedAuth(auth.clone())),
                        opi_ai::AuthProvenanceSource::Static,
                        Default::default(),
                    )
                    .unwrap();
                simple
                    .complete_simple(
                        ChatModelRecord::new("vertex", model.clone()),
                        lifecycle_text_request(),
                        Default::default(),
                    )
                    .await
                    .unwrap();
                let mut request = lifecycle_text_request();
                request.model = "vertex:gemini-2.5-flash".into();
                opi_ai::stream::ChatStream::new(provider.stream_prepared(request, auth))
                    .result()
                    .await
                    .unwrap();
                let sent = selected.received_requests().await.unwrap();
                assert_eq!(
                    sent.len(),
                    3,
                    "complete selected base must reach all public/direct paths"
                );
                assert!(
                    fallback.received_requests().await.unwrap().is_empty(),
                    "auth-derived route cannot replace explicit model base"
                );
                for request in sent {
                    assert_eq!(
                        request.url.path(),
                        format!(
                            "{suffix}{version}/publishers/google/models/gemini-2.5-flash:streamGenerateContent"
                        )
                    );
                    assert!(!request.url.path().contains("projects/"));
                    match scheme {
                        opi_ai::AuthScheme::ApiKey => {
                            assert_eq!(request.headers["x-goog-api-key"], "fixture-key")
                        }
                        opi_ai::AuthScheme::Bearer => {
                            assert_eq!(request.headers["authorization"], "Bearer fixture-key")
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn node5_builtin_google_and_vertex_simple_tool_choice_reaches_wire() {
        for (provider, key) in [
            ("google", "GEMINI_API_KEY"),
            ("google-vertex", "GOOGLE_CLOUD_API_KEY"),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(fixture_response())
                .expect(2)
                .mount(&server)
                .await;
            let context = Arc::new(Sources {
                env: BTreeMap::from([(key.into(), "isolated-key".into())]),
                ..Default::default()
            });
            let mut overrides = BuiltinOverrides::default();
            overrides.base_urls.insert(
                provider.into(),
                if provider == "google" {
                    format!("{}/v1beta", server.uri())
                } else {
                    server.uri()
                },
            );
            let mut models = Models::new();
            register_builtin(
                &mut models,
                context,
                Arc::new(opi_ai::http::HttpClient::new()),
                &overrides,
            )
            .unwrap();
            let model = models.get_model(provider, "gemini-2.5-flash").unwrap();
            for choice in [
                opi_ai::SimpleToolChoice::Auto,
                opi_ai::SimpleToolChoice::None,
            ] {
                let mut request = lifecycle_text_request();
                request.context =
                    opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                        system_prompt: None,
                        messages: request.context.messages().to_vec(),
                        tools: vec![opi_ai::message::ToolDef {
                            name: "lookup".into(),
                            description: "fixture".into(),
                            input_schema: serde_json::json!({"type":"object","properties":{}}),
                            constrained_sampling: None,
                        }],
                    })
                    .unwrap();
                models
                    .complete_simple(
                        ChatModelRecord::new(provider, model.clone()),
                        request,
                        opi_ai::SimpleOptions {
                            tool_choice: Some(choice),
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();
            }
            let sent = server.received_requests().await.unwrap();
            for (request, expected) in sent.iter().zip(["AUTO", "NONE"]) {
                let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
                assert_eq!(
                    body["toolConfig"]["functionCallingConfig"]["mode"],
                    expected
                );
                assert_eq!(request.headers["x-goog-api-key"], "isolated-key");
            }
        }
    }

    fn replay_request(provider: &str, first: opi_ai::message::AssistantMessage) -> Request {
        let mut request = lifecycle_text_request();
        request.model = format!("{provider}:gemini-2.5-flash");
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            tools: vec![],
            messages: vec![
                Message::User(UserMessage {
                    content: vec![InputContent::Text { text: "go".into() }],
                    timestamp_ms: 0,
                }),
                Message::Assistant(first),
                Message::ToolResult(ToolResultMessage {
                    tool_call_id: "call-1".into(),
                    tool_name: "lookup".into(),
                    content: vec![],
                    details: None,
                    is_error: false,
                    not_executed: None,
                    truncated: false,
                    timestamp_ms: 0,
                }),
            ],
        })
        .unwrap();
        request
    }

    #[tokio::test]
    async fn node5_canonical_google_and_vertex_capture_reload_replay() {
        for (provider, key) in [
            ("google", "GEMINI_API_KEY"),
            ("google-vertex", "GOOGLE_CLOUD_API_KEY"),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(fixture_response())
                .expect(3)
                .mount(&server)
                .await;
            let context = Arc::new(Sources {
                env: BTreeMap::from([(key.into(), "isolated-key".into())]),
                ..Default::default()
            });
            let mut overrides = BuiltinOverrides::default();
            overrides.base_urls.insert(
                provider.into(),
                if provider == "google" {
                    format!("{}/v1beta", server.uri())
                } else {
                    server.uri()
                },
            );
            let mut models = Models::new();
            register_builtin(
                &mut models,
                context.clone(),
                Arc::new(opi_ai::http::HttpClient::new()),
                &overrides,
            )
            .unwrap();
            let mut model = models
                .get_model(provider, "gemini-2.5-flash")
                .expect("real canonical catalog record");
            model.headers = vec![
                ("x-model-proof".into(), Some("selected-record".into())),
                ("x-priority".into(), Some("model".into())),
            ];
            assert!(
                context.reads.lock().unwrap().is_empty(),
                "register and lookup are inert"
            );
            let mut request = lifecycle_text_request();
            request.model = format!("{provider}:{}", model.id);
            request
                .extra_headers
                .push(("x-priority".into(), "request".into()));
            let call = models.complete_simple(
                ChatModelRecord::new(provider, model.clone()),
                request,
                Default::default(),
            );
            assert!(
                context.reads.lock().unwrap().is_empty(),
                "creating call is inert"
            );
            let first = call.await.unwrap();
            assert_eq!(first.provider, provider);
            let AssistantContent::ToolCall { tool_call } = &first.content[0] else {
                panic!("tool capture")
            };
            assert_eq!(tool_call.signature.as_deref(), Some("c2lnbmF0dXJl"));
            let reload = serde_json::from_slice(&serde_json::to_vec(&first).unwrap()).unwrap();
            let replay = replay_request(provider, reload);
            models
                .complete_simple(
                    ChatModelRecord::new(provider, model),
                    replay.clone(),
                    Default::default(),
                )
                .await
                .unwrap();
            let other = models.get_model(provider, "gemini-2.5-pro").unwrap();
            models
                .complete_simple(
                    ChatModelRecord::new(provider, other),
                    replay,
                    Default::default(),
                )
                .await
                .unwrap();
            let sent = server.received_requests().await.unwrap();
            let body: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
            assert!(
                body["contents"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .flat_map(|content| content["parts"].as_array().unwrap())
                    .any(|part| part.get("functionCall").is_some()
                        && part["thoughtSignature"] == "c2lnbmF0dXJl"),
                "canonical same identity preserves captured signature: {body}"
            );
            let foreign: serde_json::Value = serde_json::from_slice(&sent[2].body).unwrap();
            assert!(
                !foreign.to_string().contains("c2lnbmF0dXJl"),
                "different model cannot reuse signature"
            );
            assert_eq!(sent[0].headers["x-goog-api-key"], "isolated-key");
            assert_eq!(sent[0].headers["x-model-proof"], "selected-record");
            assert_eq!(sent[0].headers["x-priority"], "request");
            assert_eq!(
                sent[0].url.path(),
                if provider == "google" {
                    "/v1beta/models/gemini-2.5-flash:streamGenerateContent"
                } else {
                    "/v1/publishers/google/models/gemini-2.5-flash:streamGenerateContent"
                }
            );
        }
    }

    async fn capture_signed_google_parts(
        provider: &str,
        expected_requests: u64,
    ) -> (
        MockServer,
        Models,
        opi_ai::ModelInfo,
        opi_ai::message::AssistantMessage,
    ) {
        let server = MockServer::start().await;
        let raw = serde_json::json!({"candidates":[{"content":{"parts":[
            {"text":"","thoughtSignature":"dGV4dA=="},
            {"text":"","thought":true,"thoughtSignature":"ZW1wdHk="},
            {"functionCall":{"id":"call-1","name":"lookup","args":{"q":"fixture"}},"thoughtSignature":"dG9vbA=="},
            {"text":"answer","thoughtSignature":"YW5zd2Vy"}
        ]},"finishReason":"STOP"}]});
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!("data: {raw}\n\n"))
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(expected_requests)
            .mount(&server)
            .await;
        let context = Arc::new(Sources {
            env: BTreeMap::from([
                ("GEMINI_API_KEY".into(), "google-fixture-key".into()),
                ("GOOGLE_CLOUD_API_KEY".into(), "vertex-fixture-key".into()),
            ]),
            ..Default::default()
        });
        let mut overrides = BuiltinOverrides::default();
        overrides
            .base_urls
            .insert("google".into(), format!("{}/v1beta", server.uri()));
        overrides
            .base_urls
            .insert("google-vertex".into(), server.uri());
        let mut models = Models::new();
        register_builtin(
            &mut models,
            context,
            Arc::new(opi_ai::http::HttpClient::new()),
            &overrides,
        )
        .unwrap();
        let model = models.get_model(provider, "gemini-2.5-flash").unwrap();
        let first = models
            .complete_simple(
                ChatModelRecord::new(provider, model.clone()),
                lifecycle_text_request(),
                Default::default(),
            )
            .await
            .unwrap();
        assert_eq!(first.provider, provider);
        assert_eq!(first.wire, Some(model.wire_api));
        assert_eq!(first.model, model.id);
        assert_eq!(first.raw_stop_reason.as_deref(), Some("STOP"));
        assert_eq!(first.content.len(), 4, "producer preserves block binding");
        assert!(
            matches!(&first.content[0], AssistantContent::Text {text, signature} if text.is_empty() && signature.as_deref() == Some("dGV4dA=="))
        );
        assert!(
            matches!(&first.content[1], AssistantContent::Thinking {thinking, signature} if thinking.is_empty() && signature.as_deref() == Some("ZW1wdHk="))
        );
        assert!(
            matches!(&first.content[2], AssistantContent::ToolCall {tool_call} if tool_call.id == "call-1" && tool_call.signature.as_deref() == Some("dG9vbA=="))
        );
        assert!(
            matches!(&first.content[3], AssistantContent::Text {text, signature} if text == "answer" && signature.as_deref() == Some("YW5zd2Vy"))
        );
        let bytes = serde_json::to_vec(&first).unwrap();
        let reloaded = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(serde_json::to_vec(&reloaded).unwrap(), bytes);
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(
            sent[0].url.path(),
            if provider == "google" {
                "/v1beta/models/gemini-2.5-flash:streamGenerateContent"
            } else {
                "/v1/publishers/google/models/gemini-2.5-flash:streamGenerateContent"
            }
        );
        assert_eq!(
            sent[0].headers["x-goog-api-key"],
            if provider == "google" {
                "google-fixture-key"
            } else {
                "vertex-fixture-key"
            }
        );
        (server, models, model, reloaded)
    }

    fn replayed_model_parts(body: &serde_json::Value) -> &serde_json::Value {
        &body["contents"]
            .as_array()
            .unwrap()
            .iter()
            .find(|content| content["role"] == "model")
            .unwrap()["parts"]
    }

    fn signed_google_parts() -> serde_json::Value {
        serde_json::json!([
            {"text":"","thoughtSignature":"dGV4dA=="},
            {"text":"","thought":true,"thoughtSignature":"ZW1wdHk="},
            {"functionCall":{"name":"lookup","args":{"q":"fixture"}},"thoughtSignature":"dG9vbA=="},
            {"text":"answer","thoughtSignature":"YW5zd2Vy"}
        ])
    }

    fn projected_google_parts() -> serde_json::Value {
        serde_json::json!([
            {"functionCall":{"name":"lookup","args":{"q":"fixture"}}},
            {"text":"answer"}
        ])
    }

    #[tokio::test]
    async fn google_and_vertex_signed_empty_capture_reloads_into_same_and_foreign_requests() {
        for provider in ["google", "google-vertex"] {
            let (server, models, model, reloaded) = capture_signed_google_parts(provider, 4).await;
            let canonical = serde_json::to_vec(&reloaded).unwrap();
            models
                .complete_simple(
                    ChatModelRecord::new(provider, model),
                    replay_request(provider, reloaded.clone()),
                    Default::default(),
                )
                .await
                .unwrap();
            let foreign_provider = if provider == "google" {
                "google-vertex"
            } else {
                "google"
            };
            let foreign_model = models
                .get_model(foreign_provider, "gemini-2.5-flash")
                .unwrap();
            models
                .complete_simple(
                    ChatModelRecord::new(foreign_provider, foreign_model),
                    replay_request(foreign_provider, reloaded.clone()),
                    Default::default(),
                )
                .await
                .unwrap();
            let mut changed_model = models.get_model(provider, "gemini-2.5-flash").unwrap();
            changed_model.id = "gemini-3-pro".into();
            models
                .complete_simple(
                    ChatModelRecord::new(provider, changed_model),
                    replay_request(provider, reloaded.clone()),
                    Default::default(),
                )
                .await
                .unwrap();
            let sent = server.received_requests().await.unwrap();
            assert_eq!(sent.len(), 4);
            let same: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
            let foreign: serde_json::Value = serde_json::from_slice(&sent[2].body).unwrap();
            let changed: serde_json::Value = serde_json::from_slice(&sent[3].body).unwrap();
            assert_eq!(replayed_model_parts(&same), &signed_google_parts());
            assert_eq!(replayed_model_parts(&foreign), &projected_google_parts());
            let mut changed_parts = projected_google_parts();
            changed_parts[0]["functionCall"]["id"] = serde_json::json!("call-1");
            assert_eq!(replayed_model_parts(&changed), &changed_parts);
            assert!(
                sent[3]
                    .url
                    .path()
                    .ends_with("/models/gemini-3-pro:streamGenerateContent")
            );
            assert_eq!(
                sent[2].url.path(),
                if foreign_provider == "google" {
                    "/v1beta/models/gemini-2.5-flash:streamGenerateContent"
                } else {
                    "/v1/publishers/google/models/gemini-2.5-flash:streamGenerateContent"
                }
            );
            assert_eq!(
                sent[2].headers["x-goog-api-key"],
                if foreign_provider == "google" {
                    "google-fixture-key"
                } else {
                    "vertex-fixture-key"
                }
            );
            assert_eq!(serde_json::to_vec(&reloaded).unwrap(), canonical);
        }
    }

    #[tokio::test]
    async fn google_and_vertex_capability_change_preserves_valid_history_without_new_thinking() {
        for provider in ["google", "google-vertex"] {
            let (server, models, mut model, reloaded) =
                capture_signed_google_parts(provider, 2).await;
            let canonical = serde_json::to_vec(&reloaded).unwrap();
            assert!(model.capabilities.supports_thinking);
            model.capabilities.supports_thinking = false;
            models
                .complete_simple(
                    ChatModelRecord::new(provider, model),
                    replay_request(provider, reloaded.clone()),
                    Default::default(),
                )
                .await
                .unwrap();
            let sent = server.received_requests().await.unwrap();
            let first: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
            let replay: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
            assert_eq!(
                first["generationConfig"]["thinkingConfig"],
                serde_json::json!({"thinkingBudget":0})
            );
            assert!(replay["generationConfig"].get("thinkingConfig").is_none());
            assert_eq!(replayed_model_parts(&replay), &signed_google_parts());
            assert_eq!(serde_json::to_vec(&reloaded).unwrap(), canonical);
        }
    }

    #[tokio::test]
    async fn google_and_vertex_invalid_signatures_project_after_real_capture_and_reload() {
        for provider in ["google", "google-vertex"] {
            let (server, models, model, reloaded) = capture_signed_google_parts(provider, 7).await;
            let signatures = ["bad!", "====", "A===", "=AAA", "AA=A", "AB=="];
            for incoming in signatures {
                let mut history = reloaded.clone();
                for block in &mut history.content {
                    match block {
                        AssistantContent::Text { signature, .. }
                        | AssistantContent::Thinking { signature, .. } => {
                            *signature = Some(incoming.into());
                        }
                        AssistantContent::ToolCall { tool_call } => {
                            tool_call.signature = Some(incoming.into());
                        }
                        _ => unreachable!("capture contains only text, thinking and tool calls"),
                    }
                }
                let canonical = serde_json::to_vec(&history).unwrap();
                models
                    .complete_simple(
                        ChatModelRecord::new(provider, model.clone()),
                        replay_request(provider, history.clone()),
                        Default::default(),
                    )
                    .await
                    .unwrap();
                assert_eq!(serde_json::to_vec(&history).unwrap(), canonical);
            }
            let sent = server.received_requests().await.unwrap();
            assert_eq!(sent.len(), 7);
            for (request, signature) in sent[1..].iter().zip(signatures) {
                let replay: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
                let expected = if signature == "AB==" {
                    // The source regex accepts nonzero unused bits. A strict
                    // decoder would introduce an unsupported rejection here.
                    let mut parts = signed_google_parts();
                    for part in parts.as_array_mut().unwrap() {
                        part["thoughtSignature"] = serde_json::json!(signature);
                    }
                    parts
                } else {
                    projected_google_parts()
                };
                assert_eq!(
                    replayed_model_parts(&replay),
                    &expected,
                    "signature syntax {signature:?} must follow the source regex: {provider}"
                );
            }
        }
    }

    #[tokio::test]
    async fn google_and_vertex_unknown_wire_cannot_reuse_captured_text_or_tool_signatures() {
        for provider in ["google", "google-vertex"] {
            let (server, models, model, reloaded) = capture_signed_google_parts(provider, 3).await;
            let changed_wire = match model.wire_api {
                opi_ai::WireApi::GoogleGenerativeAi => opi_ai::WireApi::GoogleVertex,
                opi_ai::WireApi::GoogleVertex => opi_ai::WireApi::GoogleGenerativeAi,
                _ => unreachable!(),
            };
            for wire in [None, Some(changed_wire)] {
                let mut history = reloaded.clone();
                history.wire = wire;
                let canonical = serde_json::to_vec(&history).unwrap();
                models
                    .complete_simple(
                        ChatModelRecord::new(provider, model.clone()),
                        replay_request(provider, history.clone()),
                        Default::default(),
                    )
                    .await
                    .unwrap();
                assert_eq!(serde_json::to_vec(&history).unwrap(), canonical);
            }
            let sent = server.received_requests().await.unwrap();
            for request in &sent[1..] {
                let replay: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
                assert_eq!(
                    replayed_model_parts(&replay),
                    &projected_google_parts(),
                    "unknown or changed API cannot reuse captured signatures: {provider}: {replay}"
                );
            }
        }
    }

    #[tokio::test]
    async fn node5_builtin_adc_configuration_token_and_generation_are_distinct() {
        use opi_ai::auth::adc::native::{
            AdcAuthSelection, AdcNativeOwner, ReqwestAdcTransport, SystemAdcClock,
        };
        use opi_ai::auth::builtin_vertex::BuiltinVertexConfig;
        use wiremock::matchers::path;
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let credentials = dir.path().join("adc.json");
        std::fs::write(&credentials, r#"{"type":"authorized_user","client_id":"fixture-client","client_secret":"fixture-secret","refresh_token":"fixture-refresh","quota_project_id":"fixture-quota"}"#).unwrap();
        Mock::given(method("POST")).and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"access_token":"fixture-adc-token","expires_in":3600,"token_type":"Bearer"})))
            .expect(1).mount(&server).await;
        Mock::given(method("POST"))
            .and(path("/v1/projects/fixture-project/locations/europe-west1/publishers/google/models/gemini-2.5-flash:streamGenerateContent"))
            .and(header("authorization", "Bearer fixture-adc-token"))
            .and(header("x-goog-user-project", "fixture-quota"))
            .respond_with(fixture_response()).expect(2).mount(&server).await;
        let context = Arc::new(Sources {
            env: BTreeMap::from([
                (
                    "GOOGLE_APPLICATION_CREDENTIALS".into(),
                    credentials.to_string_lossy().into_owned(),
                ),
                ("GCLOUD_PROJECT".into(), "fixture-project".into()),
                ("GOOGLE_CLOUD_LOCATION".into(), "europe-west1".into()),
            ]),
            ..Default::default()
        });
        let overrides = BuiltinOverrides {
            vertex_auth: BuiltinVertexConfig {
                base_url: Some(server.uri()),
                adc: Some(
                    AdcNativeOwner::new(
                        AdcAuthSelection::Auto,
                        Arc::new(ReqwestAdcTransport::new()),
                        Arc::new(SystemAdcClock),
                    )
                    .with_token_endpoint(Some(format!("{}/token", server.uri()))),
                ),
                ..Default::default()
            },
            ..Default::default()
        };
        let mut models = Models::new();
        register_builtin(
            &mut models,
            context.clone(),
            Arc::new(opi_ai::http::HttpClient::new()),
            &overrides,
        )
        .unwrap();
        assert!(context.reads.lock().unwrap().is_empty());
        assert!(models.check_auth("google-vertex").await.unwrap().is_some());
        assert!(
            server.received_requests().await.unwrap().is_empty(),
            "availability must not acquire token"
        );
        let model = models
            .get_model("google-vertex", "gemini-2.5-flash")
            .unwrap();
        for _ in 0..2 {
            let mut request = lifecycle_text_request();
            request.model = "google-vertex:gemini-2.5-flash".into();
            let first = models
                .complete_simple(
                    ChatModelRecord::new("google-vertex", model.clone()),
                    request,
                    Default::default(),
                )
                .await
                .unwrap();
            assert_eq!(first.provider, "google-vertex");
        }
        let sent = server.received_requests().await.unwrap();
        assert_eq!(
            sent.len(),
            3,
            "one auth exchange and two actual generations"
        );
    }

    #[tokio::test]
    async fn node5_builtin_vertex_stored_key_never_reads_adc_sources() {
        use opi_ai::credential::{Credential, CredentialStore};
        use opi_ai::mutation::InMemoryCredentialStore;
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(header("x-goog-api-key", "stored-fixture-key"))
            .respond_with(fixture_response())
            .expect(1)
            .mount(&server)
            .await;
        let context = Arc::new(Sources::default());
        let store = Arc::new(InMemoryCredentialStore::new());
        store
            .write(
                "google-vertex",
                &Credential::ApiKey("stored-fixture-key".into()),
            )
            .await
            .unwrap();
        let mut models = Models::new();
        models.set_credential_store(store);
        let mut overrides = BuiltinOverrides::default();
        overrides
            .base_urls
            .insert("google-vertex".into(), server.uri());
        register_builtin(
            &mut models,
            context.clone(),
            Arc::new(opi_ai::http::HttpClient::new()),
            &overrides,
        )
        .unwrap();
        let model = models
            .get_model("google-vertex", "gemini-2.5-flash")
            .unwrap();
        let mut request = lifecycle_text_request();
        request.model = "google-vertex:gemini-2.5-flash".into();
        models
            .complete_simple(
                ChatModelRecord::new("google-vertex", model),
                request,
                Default::default(),
            )
            .await
            .unwrap();
        assert!(
            context.reads.lock().unwrap().is_empty(),
            "selected stored key suppresses all env/ADC reads"
        );
        assert_eq!(
            server.received_requests().await.unwrap()[0].url.path(),
            "/v1/publishers/google/models/gemini-2.5-flash:streamGenerateContent"
        );
    }
}

async fn collect_stream(stream: EventStream) -> Vec<AssistantStreamEvent> {
    stream.filter_map(|r| async move { r.ok() }).collect().await
}

// ---------------------------------------------------------------------------
// Provider identity
// ---------------------------------------------------------------------------

#[test]
fn vertex_provider_id_is_vertex() {
    let provider = vertex_provider();
    assert_eq!(provider.id(), "vertex");
}

// ---------------------------------------------------------------------------
// URL construction
// ---------------------------------------------------------------------------

#[test]
fn vertex_url_contains_project_and_location() {
    let provider = vertex_provider();
    let url = provider.build_vertex_url("gemini-2.5-flash");
    assert!(
        url.contains("my-project"),
        "URL should contain project: {url}"
    );
    assert!(
        url.contains("us-central1"),
        "URL should contain location: {url}"
    );
    assert!(
        url.contains("publishers/google/models/gemini-2.5-flash"),
        "URL should contain model in path: {url}"
    );
    assert!(
        url.contains("streamGenerateContent"),
        "URL should contain streamGenerateContent: {url}"
    );
}

#[test]
fn vertex_url_has_alt_sse_param() {
    let provider = vertex_provider();
    let url = provider.build_vertex_url("gemini-2.5-flash");
    assert!(
        url.contains("alt=sse"),
        "URL should have alt=sse query param: {url}"
    );
}

#[test]
fn vertex_url_uses_aiplatform_domain() {
    let provider = vertex_provider();
    let url = provider.build_vertex_url("gemini-2.5-flash");
    assert!(
        url.starts_with("https://us-central1-aiplatform.googleapis.com"),
        "URL should use Vertex AI domain: {url}"
    );
}

#[test]
fn vertex_url_with_custom_base() {
    let provider = VertexProvider::new(
        "proj".into(),
        "europe-west1".into(),
        Some("https://custom.vertex.proxy".into()),
    );
    let url = provider.build_vertex_url("gemini-2.5-pro");
    assert!(
        url.contains("europe-west1"),
        "custom base should still inject location: {url}"
    );
}

// ---------------------------------------------------------------------------
// Secret redaction
// ---------------------------------------------------------------------------

#[test]
fn vertex_access_token_not_in_debug() {
    // Phase 17.5: the access token moved out of VertexProvider construction
    // into ResolvedAuth (passed via stream_prepared). The provider no longer
    // stores the credential, so it cannot leak through its Debug output.
    // ResolvedAuth's own Debug redaction is pinned in auth_contracts.rs.
    let provider = VertexProvider::new("proj".into(), "us-central1".into(), None);
    let debug = format!("{provider:?}");
    assert!(
        !debug.contains("super-secret-oauth-token-12345"),
        "access token leaked in Debug: {debug}"
    );
}

#[test]
fn vertex_project_visible_in_debug() {
    let provider = vertex_provider();
    let debug = format!("{provider:?}");
    assert!(debug.contains("my-project"));
    assert!(debug.contains("us-central1"));
}

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

#[test]
fn vertex_default_models() {
    let provider = vertex_provider();
    let models = provider.models();
    assert!(!models.is_empty(), "Vertex should have default model list");
    assert!(
        models.iter().any(|m| m.id == "gemini-2.5-flash"),
        "should include gemini-2.5-flash"
    );
}

#[test]
fn vertex_custom_models_from_config() {
    let provider = VertexProvider::from_config(
        "proj".into(),
        "europe-west4".into(),
        vec!["my-custom-model".into(), "other-model".into()],
        None,
    );
    let models = provider.models();
    assert_eq!(models.len(), 2);
    assert_eq!(models[0].id, "my-custom-model");
    assert_eq!(models[1].id, "other-model");
}

// ---------------------------------------------------------------------------
// Model resolution via registry
// ---------------------------------------------------------------------------

#[test]
fn vertex_resolves_model_in_registry() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(Box::new(vertex_provider()))
        .unwrap();
    let (provider, model) = registry.resolve("vertex:gemini-2.5-flash").unwrap();
    assert_eq!(provider.id(), "vertex");
    assert_eq!(model.id, "gemini-2.5-flash");
}

// ---------------------------------------------------------------------------
// SSE text streaming (reuses Gemini wire format)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn vertex_text_streaming_produces_start_delta_done() {
    let provider = vertex_provider();
    let sse = text_sse_fixture();

    let events = collect_stream(provider.stream_from_sse(&sse, CancellationToken::new())).await;

    let starts = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::Start { .. }))
        .count();
    let deltas = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::TextDelta { .. }))
        .count();
    let dones = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .count();

    assert_eq!(starts, 1, "should have exactly one Start");
    assert_eq!(deltas, 1, "should have exactly one TextDelta");
    assert_eq!(dones, 1, "should have exactly one Done");
}

#[tokio::test]
async fn vertex_malformed_frame_does_not_expose_upstream_content() {
    let canary = "vertex-malformed-secret-canary";
    let sse = format!("data: {{not-json-{canary}}}\n\n");
    let events = vertex_provider()
        .stream_from_sse(&sse, CancellationToken::new())
        .collect::<Vec<_>>()
        .await;
    let rendered = format!("{events:?}");
    assert!(events.iter().any(Result::is_err));
    assert!(
        !rendered.contains(canary),
        "malformed Vertex frame leaked upstream content: {rendered}"
    );
}

#[tokio::test]
async fn vertex_done_event_has_vertex_provider() {
    let provider = vertex_provider();
    let sse = text_sse_fixture();

    let events = collect_stream(provider.stream_from_sse(&sse, CancellationToken::new())).await;

    let done_provider = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::Done { message, .. } => Some(message.provider.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(done_provider, "vertex");
}

// ---------------------------------------------------------------------------
// SSE tool call streaming
// ---------------------------------------------------------------------------

#[tokio::test]
async fn vertex_tool_call_streaming_works() {
    let provider = vertex_provider();
    let sse = tool_call_sse_fixture();

    let events = collect_stream(provider.stream_from_sse(&sse, CancellationToken::new())).await;

    let tool_starts = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::ToolCallStart { .. }))
        .count();
    let tool_ends = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::ToolCallEnd { .. }))
        .count();

    assert_eq!(tool_starts, 1, "should have one ToolCallStart");
    assert_eq!(tool_ends, 1, "should have one ToolCallEnd");
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

#[tokio::test]
async fn vertex_error_event_routing() {
    let provider = vertex_provider();
    let sse = "data: {\"error\":{\"code\":404,\"message\":\"Model not found\",\"status\":\"NOT_FOUND\"}}\n\n";

    let items: Vec<Result<AssistantStreamEvent, ProviderError>> = provider
        .stream_from_sse(sse, CancellationToken::new())
        .collect()
        .await;

    // Current producers stop the dual terminal format: the error event routes
    // to a typed stream failure, not an Error event.
    assert!(
        items.iter().any(|e| e.as_ref().is_err_and(
            |cause| cause.category() == opi_ai::provider::ProviderErrorCategory::Stream
        )),
        "should have a typed stream failure"
    );
    assert!(
        !items
            .iter()
            .any(|e| matches!(e, Ok(AssistantStreamEvent::Error { .. })))
    );
}

// ---------------------------------------------------------------------------
// Usage tracking
// ---------------------------------------------------------------------------

#[tokio::test]
async fn vertex_usage_in_done_event() {
    let provider = vertex_provider();
    let sse = text_sse_fixture();

    let events = collect_stream(provider.stream_from_sse(&sse, CancellationToken::new())).await;

    let usage = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::Done { message, .. } => Some(message.usage.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(usage.input_tokens, 42);
    assert_eq!(usage.output_tokens, 13);
}

// ---------------------------------------------------------------------------
// Request body construction (delegates to Gemini format)
// ---------------------------------------------------------------------------

#[test]
fn vertex_request_body_uses_gemini_format() {
    let provider = vertex_provider();
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "vertex:gemini-2.5-flash".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("You are helpful.".into()),
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
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };
    let body = provider.build_request_body(&request).unwrap();
    assert!(
        body.get("contents").is_some(),
        "should use Gemini contents format"
    );
    assert!(
        body.get("systemInstruction").is_some(),
        "should have systemInstruction"
    );
    assert!(
        body.get("model").is_none(),
        "model should NOT be in body (goes in URL)"
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn text_sse_fixture() -> String {
    concat!(
        "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hello\"}]},\"index\":0}]}\n\n",
        "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":42,\"candidatesTokenCount\":13,\"totalTokenCount\":55}}\n\n",
    ).into()
}

fn tool_call_sse_fixture() -> String {
    let data = serde_json::json!({
        "candidates": [{
            "content": {
                "role": "model",
                "parts": [{
                    "functionCall": {
                        "name": "read_file",
                        "args": {"path": "foo.rs"}
                    }
                }]
            },
            "index": 0
        }],
        "usageMetadata": {
            "promptTokenCount": 20,
            "candidatesTokenCount": 10,
            "totalTokenCount": 30
        }
    });
    format!("data: {data}\n\n")
}

// ---------------------------------------------------------------------------
// Production Provider::stream lifecycle through a real HTTP exchange
// ---------------------------------------------------------------------------
//
// The offline `stream_from_sse` tests above exercise the Gemini SSE parser and
// mapper but bypass the production HTTP transport (`Provider::stream` ->
// `stream_vertex_http`), so they cannot prove the Vertex AI URL, OAuth bearer
// auth header, Gemini request body, or end-to-end draining through the adapter
// path. These wiremock tests cover that contract using local mock HTTP only.

fn lifecycle_text_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "vertex:gemini-2.5-flash".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("You are helpful.".into()),
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
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

#[tokio::test]
async fn stream_drains_text_lifecycle_through_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(query_param("alt", "sse"))
        .and(header("authorization", "Bearer test-access-token"))
        .and(body_partial_json(serde_json::json!({
            "contents": [{"role": "user", "parts": [{"text": "Hello"}]}],
            "systemInstruction": {"parts": [{"text": "You are helpful."}]},
            "generationConfig": {"maxOutputTokens": 1024}
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = VertexProvider::new(
        "my-project".into(),
        "us-central1".into(),
        Some(server.uri()),
    );

    // The OAuth bearer token is supplied via ResolvedAuth (Phase 17.5); the
    // production transport derives `Authorization: Bearer <secret>` from
    // `auth.secret` in stream_prepared.
    let auth = opi_ai::auth::ResolvedAuth {
        env: Default::default(),
        scheme: opi_ai::auth::AuthScheme::Bearer,
        secret: secrecy::SecretString::from("test-access-token"),
        base_url: None,
        account_id: None,
        provenance: opi_ai::AuthProvenance::default(),
    };
    let mut stream = provider.stream_prepared(lifecycle_text_request(), auth);
    let mut events = Vec::new();
    while let Some(result) = stream.next().await {
        let event = result.expect("matched Vertex request must stream successfully");
        assert!(!matches!(event, AssistantStreamEvent::Error { .. }));
        events.push(event);
    }

    // Lifecycle: Start -> TextDelta -> Done.
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Start { .. })),
        "should emit Start through the HTTP adapter path"
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            AssistantStreamEvent::TextDelta { delta, .. } if delta == "Hello"
        )),
        "should emit TextDelta carrying the streamed text"
    );
    let done = events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("stream should produce a terminal Done event");
    match done {
        AssistantStreamEvent::Done { reason, message } => {
            // Gemini `STOP` finish reason must map to the shared StopReason::Stop.
            assert_eq!(*reason, opi_ai::stream::StopReason::Stop);
            assert_eq!(message.provider, "vertex");
            // Shared Google mapping preserves independent totals and absent subsets.
            assert_eq!(
                message.usage,
                opi_ai::stream::Usage::reported(42, 13, 0, 0, None, None)
                    .with_total_tokens(Some(55))
            );
        }
        other => panic!("expected Done, got {other:?}"),
    }

    // verify() confirms the OAuth bearer header, alt=sse query, and Gemini
    // body all matched the production request. Independently assert the Vertex
    // URL path carries project/location/model/resource so the structural path
    // is pinned regardless of colon encoding in `:streamGenerateContent`.
    server.verify().await;
    let received = server
        .received_requests()
        .await
        .expect("should have recorded the provider request");
    assert_eq!(received.len(), 1);
    let url = received[0].url.as_str();
    assert!(url.contains("projects/my-project"), "url: {url}");
    assert!(url.contains("locations/us-central1"), "url: {url}");
    assert!(
        url.contains("publishers/google/models/gemini-2.5-flash"),
        "url: {url}"
    );
    assert!(url.contains("streamGenerateContent"), "url: {url}");
}

#[tokio::test]
async fn auth_error_bodies_are_absent_from_public_errors() {
    let canaries = [
        "vertex-access-canary-with-no-known-token-shape",
        "vertex-secret-canary-with-no-known-token-shape",
        "vertex-session-canary-with-no-known-token-shape",
        "vertex-token-canary-with-no-known-token-shape",
    ];
    let body = canaries.join(" ");

    for status in [401, 403] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(query_param("alt", "sse"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body.clone()))
            .mount(&server)
            .await;

        let provider = VertexProvider::new(
            "my-project".into(),
            "us-central1".into(),
            Some(server.uri()),
        );

        let mut stream = provider.stream_prepared(
            lifecycle_text_request(),
            opi_ai::test_support::resolved_bearer_auth(),
        );
        let error = stream
            .next()
            .await
            .expect("auth failure should produce an event")
            .expect_err("auth failure should produce ProviderError");
        assert!(matches!(
            direct_cause(&error),
            opi_ai::provider::ProviderError::AuthFailed(_)
        ));
        let rendered = format!("{error} {error:?}");
        for canary in canaries {
            assert!(
                !rendered.contains(canary),
                "Vertex HTTP {status} body leaked through ProviderError: {rendered}"
            );
        }
    }
}

#[tokio::test]
async fn direct_auth_statuses_do_not_wait_for_stalled_bodies() {
    for status in [401, 403] {
        let (server, headers_flushed) = spawn_stalled_vertex_error_body_server(status).await;
        let provider = VertexProvider::new("my-project".into(), "us-central1".into(), Some(server));
        let mut stream = provider.stream_prepared(
            lifecycle_text_request(),
            opi_ai::test_support::resolved_bearer_auth(),
        );

        assert!(futures_util::poll!(stream.next()).is_pending());
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            headers_flushed.notified(),
        )
        .await
        .expect("Vertex auth-error headers must be flushed before the body stalls");

        let result = tokio::time::timeout(std::time::Duration::from_millis(500), stream.next())
            .await
            .expect("Vertex direct auth status must not wait for its body")
            .expect("Vertex direct auth status must produce a stream item");

        let error = result.expect_err("Vertex direct auth status must fail");
        assert!(matches!(
            direct_cause(&error),
            opi_ai::provider::ProviderError::AuthFailed(_)
        ));
        assert_eq!(
            error.to_string(),
            "authentication failed: provider rejected credentials"
        );
    }
}

#[tokio::test]
async fn oversized_embedded_auth_body_is_not_read_past_the_classification_cap() {
    let server = MockServer::start().await;
    let body = format!(
        r#"{{"error":{{"code":401}},"padding":"{}"}}"#,
        "x".repeat(64 * 1024)
    );
    Mock::given(method("POST"))
        .and(query_param("alt", "sse"))
        .respond_with(ResponseTemplate::new(400).set_body_string(body))
        .mount(&server)
        .await;
    let provider = VertexProvider::new(
        "my-project".into(),
        "us-central1".into(),
        Some(server.uri()),
    );
    let mut stream = provider.stream_prepared(
        lifecycle_text_request(),
        opi_ai::test_support::resolved_bearer_auth(),
    );

    let error = stream
        .next()
        .await
        .expect("oversized Vertex error body must produce a stream item")
        .expect_err("oversized Vertex error body must fail");

    assert!(matches!(
        direct_cause(&error),
        opi_ai::provider::ProviderError::ProviderSide(_)
    ));
}

#[tokio::test]
async fn request_enrichment_reaches_vertex_http_boundary() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer test-access-token"))
        .and(header("content-type", "application/json"))
        .and(header("x-opi-request", "vertex"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = VertexProvider::new(
        "my-project".into(),
        "us-central1".into(),
        Some(server.uri()),
    );
    let auth = opi_ai::auth::ResolvedAuth {
        env: Default::default(),
        scheme: opi_ai::auth::AuthScheme::Bearer,
        secret: secrecy::SecretString::from("test-access-token"),
        base_url: None,
        account_id: None,
        provenance: opi_ai::AuthProvenance::default(),
    };
    let mut request = lifecycle_text_request();
    request.extra_headers = vec![("X-Opi-Request".into(), "vertex".into())];

    let events = collect_stream(provider.stream_prepared(request, auth.clone())).await;

    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::Done { .. }))
    );
    server.verify().await;
}

#[tokio::test]
async fn request_timeout_maps_to_typed_timeout_at_vertex_boundary() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(200)))
        .mount(&server)
        .await;
    let provider = VertexProvider::new(
        "my-project".into(),
        "us-central1".into(),
        Some(server.uri()),
    );
    let mut request = lifecycle_text_request();
    request.timeout = Some(std::time::Duration::from_millis(20));
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());

    let result = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("Vertex request timeout must resolve promptly")
        .expect("Vertex timeout must produce a stream item");

    assert!(matches!(
        result.as_ref().err().map(direct_cause),
        Some(opi_ai::provider::ProviderError::Timeout)
    ));
}

#[tokio::test]
async fn stalled_embedded_auth_body_respects_a_stricter_request_timeout() {
    let (server, headers_flushed) = spawn_stalled_vertex_error_body_server(400).await;
    let provider = VertexProvider::new("my-project".into(), "us-central1".into(), Some(server));
    let mut request = lifecycle_text_request();
    request.timeout = Some(std::time::Duration::from_millis(50));
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());

    assert!(futures_util::poll!(stream.next()).is_pending());
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        headers_flushed.notified(),
    )
    .await
    .expect("Vertex error headers must be flushed before the body stalls");

    let result = tokio::time::timeout(std::time::Duration::from_millis(500), stream.next())
        .await
        .expect("Vertex error-body timeout must resolve promptly")
        .expect("Vertex error-body timeout must produce a stream item");

    assert!(matches!(
        result.as_ref().err().map(direct_cause),
        Some(opi_ai::provider::ProviderError::Timeout)
    ));
}

async fn spawn_stalled_vertex_error_body_server(
    status: u16,
) -> (String, std::sync::Arc<tokio::sync::Notify>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled Vertex error server");
    let addr = listener.local_addr().expect("stalled error server addr");
    let headers_flushed = std::sync::Arc::new(tokio::sync::Notify::new());
    let server_headers_flushed = headers_flushed.clone();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept Vertex request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read Vertex request");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let reason = match status {
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            _ => "Error",
        };
        let headers = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Length: 32\r\nConnection: close\r\n\r\n"
        );
        tokio::io::AsyncWriteExt::write_all(&mut socket, headers.as_bytes())
            .await
            .expect("write Vertex error headers");
        tokio::io::AsyncWriteExt::flush(&mut socket)
            .await
            .expect("flush Vertex error headers");
        server_headers_flushed.notify_one();
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    });

    (format!("http://{addr}"), headers_flushed)
}

#[derive(Clone, Copy)]
enum VertexStallPoint {
    BeforeHeaders,
    ResponseBody,
    ErrorBody,
}

async fn spawn_stalled_vertex_server(
    stall_point: VertexStallPoint,
) -> (String, std::sync::Arc<tokio::sync::Notify>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled Vertex server");
    let addr = listener.local_addr().expect("stalled Vertex server addr");
    let stalled = std::sync::Arc::new(tokio::sync::Notify::new());
    let server_stalled = stalled.clone();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept Vertex request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read Vertex request");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        if !matches!(stall_point, VertexStallPoint::BeforeHeaders) {
            let status = if matches!(stall_point, VertexStallPoint::ErrorBody) {
                "400 Bad Request"
            } else {
                "200 OK"
            };
            tokio::io::AsyncWriteExt::write_all(
                &mut socket,
                format!(
                    "HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nContent-Length: 1024\r\nConnection: close\r\n\r\n"
                )
                .as_bytes(),
            )
            .await
            .expect("write Vertex response headers");
            tokio::io::AsyncWriteExt::flush(&mut socket)
                .await
                .expect("flush Vertex response headers");
        }
        server_stalled.notify_one();
        std::future::pending::<()>().await;
    });

    (format!("http://{addr}"), stalled)
}

#[tokio::test]
async fn request_header_cannot_override_vertex_auth_routing() {
    let server = MockServer::start().await;
    let provider = VertexProvider::new(
        "my-project".into(),
        "us-central1".into(),
        Some(server.uri()),
    );
    let mut request = lifecycle_text_request();
    request.extra_headers = vec![("authorization".into(), "override".into())];
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());

    assert!(matches!(
        stream
            .next()
            .await
            .as_ref()
            .and_then(|item| item.as_ref().err())
            .map(direct_cause),
        Some(opi_ai::provider::ProviderError::RequestFailed(_))
    ));
    assert!(
        server
            .received_requests()
            .await
            .expect("recorded requests")
            .is_empty(),
        "reserved Vertex request headers must fail before dispatch"
    );
}

// ---------------------------------------------------------------------------
// Provider stream cancellation (Phase 12 task 12.7 DoD clause 6)
// ---------------------------------------------------------------------------

async fn assert_vertex_cancelled(stall_point: VertexStallPoint) {
    let (server, stalled) = spawn_stalled_vertex_server(stall_point).await;
    let cancel = CancellationToken::new();
    let provider = VertexProvider::new("my-project".into(), "us-central1".into(), Some(server));
    let mut request = lifecycle_text_request();
    request.cancel = cancel.clone();
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());

    assert!(futures_util::poll!(stream.next()).is_pending());
    tokio::time::timeout(std::time::Duration::from_secs(1), stalled.notified())
        .await
        .expect("Vertex server must reach the selected stall point");
    cancel.cancel();

    let remaining = tokio::time::timeout(std::time::Duration::from_secs(1), async move {
        let mut remaining = Vec::new();
        while let Some(item) = stream.next().await {
            remaining.push(item);
        }
        remaining
    })
    .await
    .expect("Vertex cancellation must terminate without waiting for HTTP");
    assert!(
        matches!(
            remaining.as_slice(),
            [Err(cause)] if cause.category() == opi_ai::provider::ProviderErrorCategory::Cancelled
        ),
        "Vertex cancellation must yield exactly one typed error, got {remaining:?}"
    );
}

#[tokio::test]
async fn cancellation_before_response_headers_is_typed_and_prompt() {
    assert_vertex_cancelled(VertexStallPoint::BeforeHeaders).await;
}

#[tokio::test]
async fn cancellation_during_response_body_is_typed_and_prompt() {
    assert_vertex_cancelled(VertexStallPoint::ResponseBody).await;
}

#[tokio::test]
async fn cancellation_during_error_body_is_typed_and_prompt() {
    assert_vertex_cancelled(VertexStallPoint::ErrorBody).await;
}

// ---------------------------------------------------------------------------
// ADC native sources through the real Vertex adapter (task 20.21.1)
// ---------------------------------------------------------------------------

mod adc_vertex {
    use futures_util::StreamExt;
    use opi_ai::auth::adc::native::{AdcAuthSelection, AdcNativeOwner, ReqwestAdcTransport};
    use opi_ai::auth::{AdcCredentialSource, AuthContext, AuthContextError, AuthResolver};
    use opi_ai::models::CompatMetadata;
    use opi_ai::registry::ProviderRegistry;
    use opi_ai::stream::AssistantStreamEvent;
    use opi_ai::vertex::VertexProvider;
    use opi_ai::{AuthProvenance, AuthProvenanceSource, BoxAuthFuture, Models, ResolvedAuth};
    use secrecy::ExposeSecret;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Instant;
    use tokio_util::sync::CancellationToken;
    use wiremock::matchers::{body_string_contains, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Non-secret synthetic admission key (RDF-23 sealed corpus; copy of the
    /// `pub(crate)` crypto fixture so the integration test can sign through
    /// the public owner).
    const SERVICE_ACCOUNT_PEM: &str = r###"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCgvXz9l8JQ6kC1
E5/ZBuDBqumYD03J6pq9twlaSZ4cDCSOhadPVn8y9yayRNxEDZ25eWObVr5SrQvJ
QnlnKmN4bIXKThvkFxzGWTX96ESZbBBg2Fbv3EjkaRaE+H8WBUtc/Hfn5av8CfKj
x9GOwSzW3VkBPW4LhBSCbXLB07f4hgQ6tMyTaejx5nNBzYtD8SsTW9LnpW4GE2GX
z+U1axSU8WFLfgUzVIXZMSrHY0rwOSwCcSicgbNGd8uIEIv0juOyakaRrPlIk5Ce
s+BYC2dT/SEDJd7VqH98qAw9KxOMmGw9rD7odHafXIB8VmQsTsm53N0KdrsufhJO
32YWIsWFAgMBAAECggEACR6ES+Z99kq6mztO3zMXcbgBUtM9ccuO9gfqGSv3iHZS
OvQPXajV2b2b/ahP9HFu+xHitNz7OmQJuOQ9+ycfEmWFF/ON/Ab+ptcdVndvWuKU
BvaHGh+90pg5WEJj2As6rxQTYRM5NQ5DFzJz1lzpxGfgOo8yYidfECuRmOMAJd0s
3k8FZ/y6nshQgfQ0Lw96aFGfCu8Ycu+3K7p8mh+wH1Bq5oVeXgExlcdO6DLXjtQO
vq0H52mBEHKTG7uiJRadPwoYL8gNhyMhg8hDMUX0w6cUIUchJkLygEaa6NElisMZ
jnSo+gcvHDl2LGQrknineuwgJT12SQ1mtnhltp//eQKBgQDQR5ssNInh/sMOxD8o
ZbuT1iRYMQYeNZqLZeJjstMtQHy3Ul1NH2REImStR1Y5dJRO3d1vRiGIZZx++SEp
44QXdfKin0IfW9w89+2KrZrxfk6eraWHPbYX6FRCob2cfm84RgI+P9YQ7jNLoiix
264MjWJ59Qoyl+ZxTFKMh6637QKBgQDFkYJQSeOApt2OUqtbozJ6NuiyzqpTJVDd
tx4/gGGwKtNm5c1FhYBF6n3WEq261l3drvDC6ucnoN2f7eItcS7Owv5Vylzx1oAM
8KPr59WM96phpnugAWBrOYa4UyJYUxn1eVyjx9l986IeaSIXdhXa32bVfd220Ouu
CC7Lv/1g+QKBgQCugA/mDW6oRcYvDQTNV868oeF5cjlp/gcpW4c9eImtlCXpgbxh
7TcYTqiC9Q2svZu0J/ZqUBuwJ1Hin9/FNLVq4osS7NN9an0vJ5mBYso6IlTEZUAV
+wIhLanK2a/++X89aopelSVIsgmQCKCHw0IjiiFDhKWylcmgiw1uaA9eUQKBgAaE
gFoJtfmeh5yUWZbSUW36tb6AVtobVwmZauQJZi+bmhHA7v6mIs8rpy8Suqx5OeuA
Q+0KNsn14pnODTIWk8a6ZA2MeXh47QyAO8gig8b8vKAwD8/9e6LN6QXXqEBlrWuN
r2Kvyxr+mJCMQ1WmzZ3E1O2iE1hemAfLuU97a+5xAoGBAKoGwyrOChBroJmIcMc7
RYpC88yq0FXkN4rXD1o/3n0CUzeeb1pad2wlt0efNCWRU5E2dM13aBxdQvg0kGNq
f1JdzC/2b2W3wiseeVHZn7MSsny0JpfbveYtQ9xbrmmRY2amtuMzS/jbnrIKQ+wv
7VCl8xqVmjOIpkF7O/B26nBb
-----END PRIVATE KEY-----"###;

    fn service_account_json() -> String {
        let escaped = SERVICE_ACCOUNT_PEM.replace('\n', "\\n");
        format!(
            r#"{{"type":"service_account","client_email":"sa@proj.iam.gserviceaccount.com","private_key":"{escaped}"}}"#
        )
    }

    fn authorized_user_json() -> String {
        r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1","quota_project_id":"quota-proj"}"#.to_owned()
    }

    #[derive(Default)]
    struct MapContext {
        env: HashMap<String, String>,
    }

    impl MapContext {
        fn with_env(mut self, name: &str, value: &str) -> Self {
            self.env.insert(name.to_owned(), value.to_owned());
            self
        }
    }

    impl AuthContext for MapContext {
        fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
            Ok(self.env.get(name).cloned())
        }
        fn file_exists(&self, _path: &str) -> Result<bool, AuthContextError> {
            Ok(false)
        }
    }

    /// Public-path resolver: the ADC owner resolves, provenance carries the
    /// selected source identity, and the Vertex adapter consumes the Bearer
    /// secret at its wire boundary.
    struct AdcRouteResolver {
        owner: Arc<AdcNativeOwner>,
    }

    impl AuthResolver for AdcRouteResolver {
        fn resolve<'a>(
            &'a self,
        ) -> BoxAuthFuture<'a, Result<ResolvedAuth, opi_ai::provider::ProviderError>> {
            Box::pin(async move {
                let token = self.owner.resolve_token().await.map_err(map_adc_error)?;
                Ok(ResolvedAuth {
                    env: Default::default(),
                    scheme: opi_ai::auth::AuthScheme::Bearer,
                    secret: token.access_token,
                    base_url: None,
                    account_id: None,
                    provenance: AuthProvenance {
                        source: AuthProvenanceSource::Adc {
                            source: token.source,
                        },
                        fallback: opi_ai::auth::AuthFallback::NotAttempted,
                    },
                })
            })
        }
    }

    fn map_adc_error(
        error: opi_ai::auth::adc::native::AdcResolutionError,
    ) -> opi_ai::provider::ProviderError {
        use opi_ai::auth::adc::native::AdcResolutionError;
        use opi_ai::provider::{ProviderError, ProviderErrorSummary};
        match error {
            AdcResolutionError::NotConfigured => ProviderError::CredentialNeeded {
                provider_id: "vertex".to_owned(),
            },
            other => {
                ProviderError::Config(ProviderErrorSummary::from_untrusted(other.safe_summary()))
            }
        }
    }

    fn owner_for(
        selection: AdcAuthSelection,
        ambient: Arc<dyn AuthContext>,
        token_endpoint: Option<String>,
    ) -> Arc<AdcNativeOwner> {
        Arc::new(
            AdcNativeOwner::new(
                selection,
                Arc::new(ReqwestAdcTransport::new()),
                Arc::new(opi_ai::auth::adc::native::SystemAdcClock),
            )
            .with_ambient(ambient)
            .with_token_endpoint(token_endpoint),
        )
    }

    /// Models with the mock-backed provider as the streaming route and a
    /// default-catalog instance for listing. The route provider is the one
    /// that streams; it must carry the mock base URL.
    fn vertex_models(provider: VertexProvider, resolver: AdcRouteResolver) -> Models {
        let mut registry = ProviderRegistry::new();
        registry
            .register_provider(Box::new(VertexProvider::new(
                "p1".into(),
                "us-central1".into(),
                None,
            )))
            .unwrap();
        let mut models = Models::from_registry(registry);
        models
            .register_route(
                Box::new(provider),
                Arc::new(resolver),
                AuthProvenanceSource::Adc {
                    source: AdcCredentialSource::ServiceAccount,
                },
                CompatMetadata::default(),
            )
            .unwrap();
        models
    }

    fn sse_ok() -> ResponseTemplate {
        ResponseTemplate::new(200)
            .set_body_string(super::text_sse_fixture())
            .insert_header("content-type", "text/event-stream")
    }

    async fn drain_to_done(models: &Models) -> AssistantStreamEvent {
        let mut request = super::lifecycle_text_request();
        request.model = "vertex:gemini-2.5-flash".into();
        request.cancel = CancellationToken::new();
        let prepared = models
            .prepare_call("vertex:gemini-2.5-flash", request)
            .await
            .expect("prepare call");
        let mut attempt = prepared.start_attempt().expect("attempt starts");
        let mut done = None;
        while let Some(item) = attempt.next().await {
            let event = item.expect("stream item");
            if matches!(event, AssistantStreamEvent::Done { .. }) {
                done = Some(event);
                break;
            }
        }
        done.expect("stream reaches Done")
    }

    #[tokio::test]
    async fn express_api_key_uses_key_header_and_projectless_url() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(query_param("alt", "sse"))
            .and(header("x-goog-api-key", "express-key"))
            .respond_with(sse_ok())
            .expect(1)
            .mount(&server)
            .await;

        // The static API-key resolver drives the Express route.
        let mut registry = ProviderRegistry::new();
        registry
            .register_provider(Box::new(VertexProvider::new(
                "p1".into(),
                "us-central1".into(),
                None,
            )))
            .unwrap();
        let mut models = Models::from_registry(registry);
        models
            .register_route(
                Box::new(VertexProvider::express(Some(server.uri()))),
                Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::ApiKey,
                    secrecy::SecretString::from("express-key"),
                )),
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .unwrap();
        drain_to_done(&models).await;
    }

    #[tokio::test]
    async fn adc_authorized_user_end_to_end_bearer_and_quota_header() {
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("adc.json");
        std::fs::write(&creds, authorized_user_json()).unwrap();

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("client_id=cid"))
            .and(body_string_contains("client_secret=csec"))
            .and(body_string_contains("refresh_token=rt1"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"vtok","expires_in":3600}"#),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer vtok"))
            .and(header("x-goog-user-project", "quota-proj"))
            .respond_with(sse_ok())
            .expect(1)
            .mount(&server)
            .await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/token", server.uri())),
                ),
            },
        );

        // Source identity is captured on the public resolution path.
        let resolved = models
            .get_auth("vertex", Default::default())
            .await
            .expect("configured")
            .expect("resolves");
        assert_eq!(
            resolved.provenance.source,
            AuthProvenanceSource::Adc {
                source: AdcCredentialSource::AuthorizedUser {
                    quota_project: Some("quota-proj".to_owned())
                }
            }
        );
        assert_eq!(resolved.secret.expose_secret(), "vtok");

        drain_to_done(&models).await;
    }

    #[tokio::test]
    async fn adc_source_failure_fails_before_generation() {
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("bad.json");
        std::fs::write(&creds, r#"{"type":"authorized_user"}"#).unwrap();

        let token_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(
                ResponseTemplate::new(400).set_body_string(r#"{"error":"invalid_request"}"#),
            )
            .mount(&token_server)
            .await;
        let generation = MockServer::start().await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/token", token_server.uri())),
                ),
            },
        );
        let error = models
            .get_auth("vertex", Default::default())
            .await
            .expect_err("a selected source error fails the resolution");
        assert!(matches!(
            error,
            opi_ai::models::ModelsError::AuthResolution { .. }
        ));
        // Fail before generation: zero requests to the model endpoint.
        let requests = generation.received_requests().await.unwrap();
        assert!(requests.is_empty(), "no generation send may happen");
    }

    #[tokio::test]
    async fn adc_service_account_loopback_exchange_shape_and_single_resolution() {
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("sa.json");
        std::fs::write(&creds, service_account_json()).unwrap();

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains(
                "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"satok","expires_in":3600}"#),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer satok"))
            .respond_with(sse_ok())
            .expect(2)
            .mount(&server)
            .await;

        let endpoint = format!("{}/token", server.uri());
        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(endpoint.clone()),
                ),
            },
        );

        drain_to_done(&models).await;
        drain_to_done(&models).await;

        // The JWT assertion: canonical grant type, iss/aud from the file and
        // the actually used endpoint, exp = iat + 3600.
        let requests = server.received_requests().await.unwrap();
        let token_post = requests
            .iter()
            .find(|r| r.url.path() == "/token")
            .expect("token exchange observed");
        let body = String::from_utf8(token_post.body.clone()).unwrap();
        let assertion = body
            .strip_prefix(
                "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion=",
            )
            .expect("jwt-bearer form");
        let parts: Vec<&str> = assertion.split('.').collect();
        assert_eq!(parts.len(), 3);
        let header = b64url_decode(parts[0]);
        assert_eq!(header, b"{\"alg\":\"RS256\"}");
        let claims: serde_json::Value = serde_json::from_slice(&b64url_decode(parts[1])).unwrap();
        assert_eq!(claims["iss"], "sa@proj.iam.gserviceaccount.com");
        assert_eq!(claims["aud"], endpoint);
        assert_eq!(
            claims["exp"].as_i64().unwrap() - claims["iat"].as_i64().unwrap(),
            3600
        );

        // One exchange serves both logical calls within the eager window.
        let token_posts = requests.iter().filter(|r| r.url.path() == "/token").count();
        assert_eq!(token_posts, 1);
    }

    #[tokio::test]
    async fn token_503_then_success_retries_with_the_first_fixed_delay() {
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("creds.json");
        std::fs::write(&creds, authorized_user_json()).unwrap();

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"rt","expires_in":3600}"#),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .respond_with(sse_ok())
            .mount(&server)
            .await;

        let started = Instant::now();
        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/token", server.uri())),
                ),
            },
        );
        drain_to_done(&models).await;
        assert!(
            started.elapsed() >= std::time::Duration::from_millis(100),
            "the first retry waits the fixed 100ms gaxios delay"
        );
        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests.iter().filter(|r| r.url.path() == "/token").count(),
            2
        );
    }

    #[tokio::test]
    async fn token_redirect_is_terminal_and_generation_is_never_contacted() {
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("creds.json");
        std::fs::write(&creds, authorized_user_json()).unwrap();

        let token_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(302).insert_header("location", "/elsewhere"))
            .mount(&token_server)
            .await;
        let generation = MockServer::start().await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/token", token_server.uri())),
                ),
            },
        );
        assert!(
            models.get_auth("vertex", Default::default()).await.is_err(),
            "a redirect on a token exchange is terminal"
        );
        let requests = generation.received_requests().await.unwrap();
        assert!(requests.is_empty());
    }

    #[tokio::test]
    async fn direct_metadata_selected_explicitly_streams_bearer() {
        let metadata = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(
                "/computeMetadata/v1/instance/service-accounts/default/token",
            ))
            .and(query_param(
                "scopes",
                "https://www.googleapis.com/auth/cloud-platform",
            ))
            .and(header("Metadata-Flavor", "Google"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"mdtok","expires_in":3600}"#)
                    .insert_header("Metadata-Flavor", "Google"),
            )
            .expect(1)
            .mount(&metadata)
            .await;

        let generation = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer mdtok"))
            .respond_with(sse_ok())
            .expect(1)
            .mount(&generation)
            .await;

        let host = metadata.uri().trim_start_matches("http://").to_owned();
        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::DirectMetadata {
                        residency: Some(false),
                    },
                    Arc::new(MapContext::default().with_env("GCE_METADATA_HOST", &host)),
                    None,
                ),
            },
        );
        let resolved = models
            .get_auth("vertex", Default::default())
            .await
            .expect("metadata resolves")
            .expect("configured");
        assert_eq!(
            resolved.provenance.source,
            AuthProvenanceSource::Adc {
                source: AdcCredentialSource::DirectMetadata
            }
        );
        drain_to_done(&models).await;
    }

    fn b64url_decode(input: &str) -> Vec<u8> {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut out = Vec::new();
        let mut buffer: u32 = 0;
        let mut bits = 0;
        for &byte in input.as_bytes() {
            if byte == b'=' {
                continue;
            }
            let value = ALPHABET
                .iter()
                .position(|&a| a == byte)
                .expect("valid base64url byte") as u32;
            buffer = (buffer << 6) | value;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((buffer >> bits) as u8);
            }
        }
        out
    }
}

/// Task 20.21.2: external-account federation through the public path.
mod adc_external {
    use futures_util::StreamExt;
    use opi_ai::auth::adc::native::{
        AdcAuthSelection, AdcNativeOwner, ReqwestAdcTransport, SystemAdcClock,
    };
    use opi_ai::auth::{
        AdcCredentialSource, AuthContext, AuthContextError, AuthResolver, ExternalAccountSupplier,
    };
    use opi_ai::models::CompatMetadata;
    use opi_ai::registry::ProviderRegistry;
    use opi_ai::vertex::VertexProvider;
    use opi_ai::{AuthProvenance, AuthProvenanceSource, BoxAuthFuture, Models, ResolvedAuth};
    use secrecy::ExposeSecret;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio_util::sync::CancellationToken;
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const AUDIENCE: &str =
        "//iam.googleapis.com/projects/123/locations/global/workloadIdentityPools/p/providers/o";
    const SUBJECT_TOKEN_TYPE: &str = "urn:ietf:params:oauth:token-type:jwt";

    struct MapContext {
        env: HashMap<String, String>,
        process_dir: tempfile::TempDir,
    }

    impl Default for MapContext {
        fn default() -> Self {
            Self {
                env: HashMap::new(),
                process_dir: tempfile::tempdir().unwrap(),
            }
        }
    }

    impl MapContext {
        fn with_env(mut self, name: &str, value: &str) -> Self {
            self.env.insert(name.to_owned(), value.to_owned());
            self
        }
    }

    impl AuthContext for MapContext {
        fn process_snapshot(&self) -> Result<opi_ai::auth::AuthProcessSnapshot, AuthContextError> {
            Ok(opi_ai::test_support::portable_auth_process_snapshot(
                self.process_dir.path(),
            ))
        }
        fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
            Ok(self.env.get(name).cloned())
        }
        fn file_exists(&self, _path: &str) -> Result<bool, AuthContextError> {
            Ok(false)
        }
    }

    struct AdcRouteResolver {
        owner: Arc<AdcNativeOwner>,
    }

    impl AuthResolver for AdcRouteResolver {
        fn resolve<'a>(
            &'a self,
        ) -> BoxAuthFuture<'a, Result<ResolvedAuth, opi_ai::provider::ProviderError>> {
            Box::pin(async move {
                let token = self.owner.resolve_token().await.map_err(map_adc_error)?;
                Ok(ResolvedAuth {
                    env: Default::default(),
                    scheme: opi_ai::auth::AuthScheme::Bearer,
                    secret: token.access_token,
                    base_url: None,
                    account_id: None,
                    provenance: AuthProvenance {
                        source: AuthProvenanceSource::Adc {
                            source: token.source,
                        },
                        fallback: opi_ai::auth::AuthFallback::NotAttempted,
                    },
                })
            })
        }
    }

    fn map_adc_error(
        error: opi_ai::auth::adc::native::AdcResolutionError,
    ) -> opi_ai::provider::ProviderError {
        use opi_ai::auth::adc::native::AdcResolutionError;
        use opi_ai::provider::{ProviderError, ProviderErrorSummary};
        match error {
            AdcResolutionError::NotConfigured => ProviderError::CredentialNeeded {
                provider_id: "vertex".to_owned(),
            },
            other => {
                ProviderError::Config(ProviderErrorSummary::from_untrusted(other.safe_summary()))
            }
        }
    }

    fn owner_for(
        selection: AdcAuthSelection,
        ambient: Arc<dyn AuthContext>,
        token_endpoint: Option<String>,
    ) -> Arc<AdcNativeOwner> {
        Arc::new(
            AdcNativeOwner::new(
                selection,
                Arc::new(ReqwestAdcTransport::new()),
                Arc::new(SystemAdcClock),
            )
            .with_ambient(ambient)
            .with_token_endpoint(token_endpoint),
        )
    }

    fn vertex_models(provider: VertexProvider, resolver: AdcRouteResolver) -> Models {
        let mut registry = ProviderRegistry::new();
        registry
            .register_provider(Box::new(VertexProvider::new(
                "p1".into(),
                "us-central1".into(),
                None,
            )))
            .unwrap();
        let mut models = Models::from_registry(registry);
        models
            .register_route(
                Box::new(provider),
                Arc::new(resolver),
                AuthProvenanceSource::Adc {
                    source: AdcCredentialSource::ServiceAccount,
                },
                CompatMetadata::default(),
            )
            .unwrap();
        models
    }

    fn sse_ok() -> ResponseTemplate {
        ResponseTemplate::new(200)
            .set_body_string(super::text_sse_fixture())
            .insert_header("content-type", "text/event-stream")
    }

    fn external_json(credential_source: &str) -> String {
        format!(
            r#"{{"type":"external_account","audience":"{AUDIENCE}","subject_token_type":"{SUBJECT_TOKEN_TYPE}","credential_source":{credential_source}}}"#
        )
    }

    async fn drain_to_done(models: &Models) {
        let mut request = super::lifecycle_text_request();
        request.model = "vertex:gemini-2.5-flash".into();
        request.cancel = CancellationToken::new();
        let prepared = models
            .prepare_call("vertex:gemini-2.5-flash", request)
            .await
            .expect("prepare call");
        let mut attempt = prepared.start_attempt().expect("attempt starts");
        while let Some(item) = attempt.next().await {
            let event = item.expect("stream item");
            if matches!(
                event,
                opi_ai::stream::AssistantStreamEvent::Done { .. }
                    | opi_ai::stream::AssistantStreamEvent::Error { .. }
            ) {
                assert!(!matches!(
                    event,
                    opi_ai::stream::AssistantStreamEvent::Error { .. }
                ));
                break;
            }
        }
    }

    #[tokio::test]
    async fn external_file_supplier_streams_bearer() {
        let dir = tempfile::tempdir().unwrap();
        let subject_file = dir.path().join("subject.txt");
        std::fs::write(&subject_file, "k8s-projected-token").unwrap();
        let creds = dir.path().join("external.json");
        std::fs::write(
            &creds,
            external_json(&format!(
                r#"{{"file":{}}}"#,
                serde_json::to_string(&subject_file).unwrap()
            )),
        )
        .unwrap();

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/token"))
            .and(body_string_contains(
                "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Atoken-exchange",
            ))
            .and(body_string_contains("audience=%2F%2Fiam.googleapis.com"))
            .and(body_string_contains("subject_token=k8s-projected-token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"fedtok","expires_in":3600}"#),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer fedtok"))
            .respond_with(sse_ok())
            .expect(1)
            .mount(&server)
            .await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/v1/token", server.uri())),
                ),
            },
        );

        let resolved = models
            .get_auth("vertex", Default::default())
            .await
            .expect("configured")
            .expect("resolves");
        assert_eq!(
            resolved.provenance.source,
            AuthProvenanceSource::Adc {
                source: AdcCredentialSource::ExternalAccount {
                    supplier: ExternalAccountSupplier::File,
                    workforce_user_project: None,
                }
            }
        );
        assert_eq!(resolved.secret.expose_secret(), "fedtok");

        drain_to_done(&models).await;
    }

    #[tokio::test]
    async fn external_sts_failure_sends_zero_generation() {
        let dir = tempfile::tempdir().unwrap();
        let subject_file = dir.path().join("subject.txt");
        std::fs::write(&subject_file, "s").unwrap();
        let creds = dir.path().join("external.json");
        std::fs::write(
            &creds,
            external_json(&format!(
                r#"{{"file":{}}}"#,
                serde_json::to_string(&subject_file).unwrap()
            )),
        )
        .unwrap();

        let sts = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/token"))
            .respond_with(
                ResponseTemplate::new(400).set_body_string(r#"{"error":"invalid_grant"}"#),
            )
            .mount(&sts)
            .await;
        let generation = MockServer::start().await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/v1/token", sts.uri())),
                ),
            },
        );

        let error = models
            .get_auth("vertex", Default::default())
            .await
            .expect_err("failed supplier/STS fails the resolution");
        assert!(matches!(
            error,
            opi_ai::models::ModelsError::AuthResolution { .. }
        ));
        assert!(generation.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn external_executable_streams_bearer() {
        let dir = tempfile::tempdir().unwrap();
        let expires = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600;
        let response = format!(
            r#"{{"version":1,"success":true,"token_type":"{SUBJECT_TOKEN_TYPE}","id_token":"pluggable-token","expiration_time":{expires}}}"#
        );
        // Platform bodies stay plain strings; the executable command is a
        // direct (quote-aware) argv, never a shell re-parse.
        let unix = format!("printf '%s' '{}'", response);
        let windows = format!("Write-Host -NoNewline '{}'", response);
        let command =
            opi_ai::test_support::portable_executable_script(dir.path(), "helper", &unix, &windows);
        let creds = dir.path().join("external.json");
        std::fs::write(
            &creds,
            external_json(&format!(
                r#"{{"executable":{{"command":"{}"}}}}"#,
                command.replace('\\', "\\\\").replace('"', "\\\"")
            )),
        )
        .unwrap();

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/token"))
            .and(body_string_contains("subject_token=pluggable-token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"exetok","expires_in":3600}"#),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer exetok"))
            .respond_with(sse_ok())
            .expect(1)
            .mount(&server)
            .await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(
                        MapContext::default()
                            .with_env("GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES", "1"),
                    ),
                    Some(format!("{}/v1/token", server.uri())),
                ),
            },
        );

        let resolved = models
            .get_auth("vertex", Default::default())
            .await
            .expect("configured")
            .expect("resolves");
        assert_eq!(
            resolved.provenance.source,
            AuthProvenanceSource::Adc {
                source: AdcCredentialSource::ExternalAccount {
                    supplier: ExternalAccountSupplier::Executable,
                    workforce_user_project: None,
                }
            }
        );
        drain_to_done(&models).await;
    }

    #[tokio::test]
    async fn external_aws_env_streams_bearer() {
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("external.json");
        std::fs::write(
            &creds,
            external_json(
                r#"{"environment_id":"aws1","regional_cred_verification_url":"https://sts.{region}.example.dev?Action=GetCallerIdentity&Version=2011-06-15"}"#,
            ),
        )
        .unwrap();

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/token"))
            .and(body_string_contains("GetCallerIdentity"))
            .and(body_string_contains(urlencoded(AUDIENCE)))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"awstok","expires_in":3600}"#),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer awstok"))
            .respond_with(sse_ok())
            .expect(1)
            .mount(&server)
            .await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(
                        MapContext::default()
                            .with_env("AWS_REGION", "us-east-1")
                            .with_env("AWS_ACCESS_KEY_ID", "AKIDEXAMPLE")
                            .with_env("AWS_SECRET_ACCESS_KEY", "wJalrXUtnFEMI")
                            .with_env("AWS_SESSION_TOKEN", "session-token"),
                    ),
                    Some(format!("{}/v1/token", server.uri())),
                ),
            },
        );

        let resolved = models
            .get_auth("vertex", Default::default())
            .await
            .expect("configured")
            .expect("resolves");
        assert_eq!(
            resolved.provenance.source,
            AuthProvenanceSource::Adc {
                source: AdcCredentialSource::ExternalAccount {
                    supplier: ExternalAccountSupplier::Aws,
                    workforce_user_project: None,
                }
            }
        );
        drain_to_done(&models).await;
    }

    fn urlencoded(value: &str) -> String {
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("", value)
            .finish()
            .trim_start_matches('=')
            .to_owned()
    }
}

// ---------------------------------------------------------------------------
// Task 20.21.3 — impersonation, recursive composition, keyed cache (e2e)
// ---------------------------------------------------------------------------

mod adc_impersonation {
    use futures_util::StreamExt;
    use opi_ai::auth::adc::native::{
        AdcAuthSelection, AdcNativeOwner, ReqwestAdcTransport, SystemAdcClock,
    };
    use opi_ai::auth::{
        AdcCredentialSource, AuthContext, AuthContextError, AuthResolver, ExternalAccountSupplier,
    };
    use opi_ai::models::CompatMetadata;
    use opi_ai::registry::ProviderRegistry;
    use opi_ai::vertex::VertexProvider;
    use opi_ai::{AuthProvenance, AuthProvenanceSource, BoxAuthFuture, Models, ResolvedAuth};
    use secrecy::ExposeSecret;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio_util::sync::CancellationToken;
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const TARGET: &str = "target@proj.iam.gserviceaccount.com";

    #[derive(Default)]
    struct MapContext(HashMap<String, String>);

    impl AuthContext for MapContext {
        fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
            Ok(self.0.get(name).cloned())
        }
        fn file_exists(&self, _path: &str) -> Result<bool, AuthContextError> {
            Ok(false)
        }
    }

    struct AdcRouteResolver {
        owner: Arc<AdcNativeOwner>,
    }

    impl AuthResolver for AdcRouteResolver {
        fn resolve<'a>(
            &'a self,
        ) -> BoxAuthFuture<'a, Result<ResolvedAuth, opi_ai::provider::ProviderError>> {
            Box::pin(async move {
                let token = self.owner.resolve_token().await.map_err(map_adc_error)?;
                Ok(ResolvedAuth {
                    env: Default::default(),
                    scheme: opi_ai::auth::AuthScheme::Bearer,
                    secret: token.access_token,
                    base_url: None,
                    account_id: None,
                    provenance: AuthProvenance {
                        source: AuthProvenanceSource::Adc {
                            source: token.source,
                        },
                        fallback: opi_ai::auth::AuthFallback::NotAttempted,
                    },
                })
            })
        }
    }

    fn map_adc_error(
        error: opi_ai::auth::adc::native::AdcResolutionError,
    ) -> opi_ai::provider::ProviderError {
        use opi_ai::auth::adc::native::AdcResolutionError;
        use opi_ai::provider::{ProviderError, ProviderErrorSummary};
        match error {
            AdcResolutionError::NotConfigured => ProviderError::CredentialNeeded {
                provider_id: "vertex".to_owned(),
            },
            other => {
                ProviderError::Config(ProviderErrorSummary::from_untrusted(other.safe_summary()))
            }
        }
    }

    fn owner_for(
        selection: AdcAuthSelection,
        ambient: Arc<dyn AuthContext>,
        token_endpoint: Option<String>,
        impersonation_endpoint: Option<String>,
    ) -> Arc<AdcNativeOwner> {
        Arc::new(
            AdcNativeOwner::new(
                selection,
                Arc::new(ReqwestAdcTransport::new()),
                Arc::new(SystemAdcClock),
            )
            .with_ambient(ambient)
            .with_token_endpoint(token_endpoint)
            .with_impersonation_endpoint(impersonation_endpoint),
        )
    }

    fn vertex_models(provider: VertexProvider, resolver: AdcRouteResolver) -> Models {
        let mut registry = ProviderRegistry::new();
        registry
            .register_provider(Box::new(VertexProvider::new(
                "p1".into(),
                "us-central1".into(),
                None,
            )))
            .unwrap();
        let mut models = Models::from_registry(registry);
        models
            .register_route(
                Box::new(provider),
                Arc::new(resolver),
                AuthProvenanceSource::Adc {
                    source: AdcCredentialSource::ServiceAccount,
                },
                CompatMetadata::default(),
            )
            .unwrap();
        models
    }

    fn sse_ok() -> ResponseTemplate {
        ResponseTemplate::new(200)
            .set_body_string(super::text_sse_fixture())
            .insert_header("content-type", "text/event-stream")
    }

    async fn drain_to_done(models: &Models) {
        let mut request = super::lifecycle_text_request();
        request.model = "vertex:gemini-2.5-flash".into();
        request.cancel = CancellationToken::new();
        let prepared = models
            .prepare_call("vertex:gemini-2.5-flash", request)
            .await
            .expect("prepare call");
        let mut attempt = prepared.start_attempt().expect("attempt starts");
        while let Some(item) = attempt.next().await {
            let event = item.expect("stream item");
            if matches!(
                event,
                opi_ai::stream::AssistantStreamEvent::Done { .. }
                    | opi_ai::stream::AssistantStreamEvent::Error { .. }
            ) {
                assert!(!matches!(
                    event,
                    opi_ai::stream::AssistantStreamEvent::Error { .. }
                ));
                break;
            }
        }
    }

    fn impersonated_creds(path: &std::path::Path, source_json: &str, quota: Option<&str>) {
        let quota_field = quota
            .map(|q| format!(r#","quota_project_id":"{q}""#))
            .unwrap_or_default();
        std::fs::write(
            path,
            format!(
                r#"{{"type":"impersonated_service_account","source_credentials":{source_json},"service_account_impersonation_url":"https://iamcredentials.example.com/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"{quota_field}}}"#
            ),
        )
        .unwrap();
    }

    #[tokio::test]
    async fn impersonated_authorized_user_streams_bearer_with_quota_and_provenance() {
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("imp.json");
        impersonated_creds(
            &creds,
            r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#,
            Some("quota-proj"),
        );

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/source/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"sourcetok","expires_in":3600}"#),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"
            )))
            .and(header("authorization", "Bearer sourcetok"))
            .and(body_string_contains(
                "\"https://www.googleapis.com/auth/cloud-platform\"",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(
                    r#"{"accessToken":"imptok","expireTime":"2035-01-01T00:00:00Z"}"#,
                ),
            )
            .expect(1)
            .mount(&server)
            .await;
        let generation = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer imptok"))
            .and(header("x-goog-user-project", "quota-proj"))
            .respond_with(sse_ok())
            .expect(1)
            .mount(&generation)
            .await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/source/token", server.uri())),
                    Some(format!(
                        "{}/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken",
                        server.uri()
                    )),
                ),
            },
        );

        let resolved = models
            .get_auth("vertex", Default::default())
            .await
            .expect("configured")
            .expect("resolves");
        assert_eq!(
            resolved.provenance.source,
            AuthProvenanceSource::Adc {
                source: AdcCredentialSource::Impersonated {
                    source: Box::new(AdcCredentialSource::AuthorizedUser {
                        quota_project: None,
                    }),
                    target_principal: TARGET.to_owned(),
                    delegates: 0,
                    quota_project: Some("quota-proj".to_owned()),
                }
            }
        );
        assert_eq!(resolved.secret.expose_secret(), "imptok");

        drain_to_done(&models).await;
    }

    #[tokio::test]
    async fn owner_caches_the_impersonated_chain_across_streams() {
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("imp.json");
        impersonated_creds(
            &creds,
            r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#,
            None,
        );

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/source/token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"sourcetok","expires_in":3600}"#),
            )
            // Two streams, one acquisition chain: the keyed cache serves the
            // second call without a new source refresh or impersonation.
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"
            )))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(
                    r#"{"accessToken":"imptok","expireTime":"2035-01-01T00:00:00Z"}"#,
                ),
            )
            .expect(1)
            .mount(&server)
            .await;
        let generation = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .respond_with(sse_ok())
            .expect(2)
            .mount(&generation)
            .await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/source/token", server.uri())),
                    Some(format!(
                        "{}/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken",
                        server.uri()
                    )),
                ),
            },
        );

        drain_to_done(&models).await;
        drain_to_done(&models).await;
    }

    #[tokio::test]
    async fn external_federation_then_impersonation_streams_bearer() {
        let dir = tempfile::tempdir().unwrap();
        let subject_file = dir.path().join("subject.txt");
        std::fs::write(&subject_file, "k8s-projected-token").unwrap();
        let creds = dir.path().join("external.json");
        std::fs::write(
            &creds,
            format!(
                r#"{{"type":"external_account","audience":"//iam.googleapis.com/projects/123/locations/global/workloadIdentityPools/p/providers/o","subject_token_type":"urn:ietf:params:oauth:token-type:jwt","service_account_impersonation_url":"https://iamcredentials.example.com/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken","service_account_impersonation":{{"token_lifetime_seconds":2700}},"credential_source":{{"file":{}}}}}"#,
                serde_json::to_string(&subject_file).unwrap()
            ),
        )
        .unwrap();

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/token"))
            .and(body_string_contains("subject_token=k8s-projected-token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"fedtok","expires_in":3600}"#),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"
            )))
            .and(header("authorization", "Bearer fedtok"))
            .and(body_string_contains("\"lifetime\":\"2700s\""))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(
                    r#"{"accessToken":"imptok","expireTime":"2035-01-01T00:00:00Z"}"#,
                ),
            )
            .expect(1)
            .mount(&server)
            .await;
        let generation = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer imptok"))
            .respond_with(sse_ok())
            .expect(1)
            .mount(&generation)
            .await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/v1/token", server.uri())),
                    Some(format!(
                        "{}/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken",
                        server.uri()
                    )),
                ),
            },
        );

        let resolved = models
            .get_auth("vertex", Default::default())
            .await
            .expect("configured")
            .expect("resolves");
        assert_eq!(
            resolved.provenance.source,
            AuthProvenanceSource::Adc {
                source: AdcCredentialSource::Impersonated {
                    source: Box::new(AdcCredentialSource::ExternalAccount {
                        supplier: ExternalAccountSupplier::File,
                        workforce_user_project: None,
                    }),
                    target_principal: TARGET.to_owned(),
                    delegates: 0,
                    quota_project: None,
                }
            }
        );
        assert_eq!(resolved.secret.expose_secret(), "imptok");

        drain_to_done(&models).await;
    }

    #[tokio::test]
    async fn impersonation_failure_sends_zero_generation() {
        let dir = tempfile::tempdir().unwrap();
        let creds = dir.path().join("imp.json");
        impersonated_creds(
            &creds,
            r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#,
            None,
        );

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/source/token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"access_token":"sourcetok","expires_in":3600}"#),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"
            )))
            .respond_with(
                ResponseTemplate::new(403)
                    .set_body_string(r#"{"error":{"status":"PERMISSION_DENIED"}}"#),
            )
            .mount(&server)
            .await;
        let generation = MockServer::start().await;

        let models = vertex_models(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
            AdcRouteResolver {
                owner: owner_for(
                    AdcAuthSelection::CredentialsFile(creds),
                    Arc::new(MapContext::default()),
                    Some(format!("{}/source/token", server.uri())),
                    Some(format!(
                        "{}/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken",
                        server.uri()
                    )),
                ),
            },
        );

        let resolved = models.get_auth("vertex", Default::default()).await;
        assert!(matches!(
            resolved,
            Err(opi_ai::models::ModelsError::AuthResolution { .. })
        ));
        assert!(generation.received_requests().await.unwrap().is_empty());
    }
}

// ---------------------------------------------------------------------------
// Functional Vertex callbacks: payload/raw supported, response unsupported.
// ---------------------------------------------------------------------------

use opi_ai::callbacks::RequestCallbacks;

#[tokio::test]
async fn vertex_response_observer_unsupported_but_payload_callback_flows() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::body_partial_json(
            serde_json::json!({"vertex_marker": "replaced"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let observer_server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&observer_server)
        .await;

    let provider = VertexProvider::new(
        "my-project".into(),
        "us-central1".into(),
        Some(server.uri()),
    );
    let auth = opi_ai::auth::ResolvedAuth {
        env: Default::default(),
        scheme: opi_ai::auth::AuthScheme::Bearer,
        secret: secrecy::SecretString::from("test-access-token"),
        base_url: None,
        account_id: None,
        provenance: opi_ai::AuthProvenance::default(),
    };

    // Payload-only callbacks flow through the shared Google wire projection.
    let mut request = lifecycle_text_request();
    request.callbacks = RequestCallbacks::new().with_on_payload(std::sync::Arc::new(|body| {
        let mut replacement = body.clone();
        replacement["vertex_marker"] = serde_json::json!("replaced");
        Ok(Some(replacement))
    }));
    let events = collect_stream(provider.stream_prepared(request, auth.clone())).await;
    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::Done { .. }))
    );
    server.verify().await;

    // HTTP response observers are explicitly unsupported on Google wires.
    let observer_provider = VertexProvider::new(
        "my-project".into(),
        "us-central1".into(),
        Some(observer_server.uri()),
    );
    let mut observer_request = lifecycle_text_request();
    observer_request.callbacks =
        RequestCallbacks::new().with_on_response(std::sync::Arc::new(|_status, _headers| Ok(())));
    let mut stream = observer_provider.stream_prepared(observer_request.clone(), auth);
    let error = stream
        .next()
        .await
        .expect("one refusal")
        .expect_err("vertex rejects response observers before dispatch");
    let ProviderError::UnsupportedCapability(summary) =
        accepted_failure_cause(&error, &observer_request)
    else {
        panic!("expected exact response-observer cause: {error:?}");
    };
    assert_eq!(
        summary.as_str(),
        "Google wires do not support response callbacks"
    );
    assert!(stream.next().await.is_none(), "refusal must be terminal");
    observer_server.verify().await;
}

#[tokio::test]
async fn vertex_async_raw_observer_sees_wire_fields_before_normalization() {
    let raw = serde_json::json!({"wire_only":"retained","candidates":[{"content":{"parts":[{"text":"Hi"}]},"finishReason":"STOP"}]});
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!("data: {raw}\n\n")),
        )
        .mount(&server)
        .await;
    let seen = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = seen.clone();
    let mut request = lifecycle_text_request();
    request.callbacks = RequestCallbacks::new().with_async_on_raw(move |value| {
        let count = count.clone();
        async move {
            tokio::task::yield_now().await;
            assert_eq!(value["wire_only"], "retained");
            count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    });
    let provider = VertexProvider::new(
        "my-project".into(),
        "us-central1".into(),
        Some(server.uri()),
    );
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    let mut done = false;
    while let Some(item) = stream.next().await {
        let event = item.expect("Vertex raw observer is supported");
        assert_eq!(seen.load(std::sync::atomic::Ordering::SeqCst), 1);
        done |= matches!(event, AssistantStreamEvent::Done { .. });
    }
    assert!(done);
}

#[tokio::test]
async fn vertex_transform_cannot_introduce_google_or_host_identity() {
    for name in ["Host", "X-Goog-Api-Key", "X-Goog-User-Project"] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
        let mut request = lifecycle_text_request();
        request.callbacks =
            RequestCallbacks::new().with_header_transform(std::sync::Arc::new(move |_| {
                Ok(vec![(name.into(), "changed".into())])
            }));
        let provider = VertexProvider::new(
            "my-project".into(),
            "us-central1".into(),
            Some(server.uri()),
        );
        let failure = opi_ai::stream::ChatStream::new(
            provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()),
        )
        .result()
        .await
        .unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::CallbackPhase(_)));
        server.verify().await;
    }
}

mod constrained_guard_tests {
    use super::*;
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    use opi_ai::message::ToolDef;
    use opi_ai::provider::ProviderError;

    fn constrained_tool(strict: StrictPreference) -> ToolDef {
        ToolDef {
            name: "structured".into(),
            description: "d".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"]
            }),
            constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict }),
        }
    }

    fn guarded_request(tools: Vec<ToolDef>) -> Request {
        Request {
            projection: Default::default(),
            transport: Default::default(),
            model: "vertex:gemini-2.5-flash".into(),
            context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                system_prompt: None,
                messages: vec![Message::User(UserMessage {
                    content: vec![InputContent::Text { text: "go".into() }],
                    timestamp_ms: 0,
                })],
                tools,
            })
            .expect("valid context declarations"),
            max_tokens: Some(64),
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

    #[test]
    fn require_constrained_sampling_fails_typed_before_dispatch() {
        let provider = vertex_provider();
        let error = provider
            .build_request_body(&guarded_request(vec![constrained_tool(
                StrictPreference::Require,
            )]))
            .expect_err("require never silently drops on this wire");
        assert!(
            matches!(error, ProviderError::Config(_)),
            "typed pre-dispatch failure, got {error:?}"
        );
    }

    #[test]
    fn prefer_constrained_sampling_projects_non_strict() {
        let provider = vertex_provider();
        let body = provider
            .build_request_body(&guarded_request(vec![constrained_tool(
                StrictPreference::Prefer,
            )]))
            .expect("prefer projects non-strict");
        let serialized = body.to_string();
        assert!(
            !serialized.contains("\"strict\""),
            "no strict flag is invented on this wire: {serialized}"
        );
    }
}

// ---------------------------------------------------------------------------
// Task 20.41: complete google-vertex chat adapter (DEL-11)
// ---------------------------------------------------------------------------
//
// The typed public surface `GoogleVertexChatApi` (api id `google-vertex`)
// serves the native `GoogleVertexOptions` inventory through the four `Models`
// entries. Wire rules are inherited from the shared 20.40 engine; the
// Vertex-own contracts pinned here are the two routing modes (API-key Express
// vs ADC Bearer project/location), the per-wire neutral budget table (pi
// `getGoogleBudget` has no flash-lite entry on Vertex), and the producer
// telemetry carrier.

mod google_41 {
    use super::*;
    use opi_ai::gemini::{
        GeminiProvider, GoogleApiThinkingLevel, GoogleThinkingConfig, GoogleToolChoice,
    };
    use opi_ai::model_info::{ModelCapabilities, ModelInfo, ThinkingLevel};
    use opi_ai::models::{ChatModelRecord, Models, SimpleOptions, SimpleToolChoice};
    use opi_ai::stream::StopReason;
    use opi_ai::vertex::{GoogleVertexChatApi, GoogleVertexOptions};
    use secrecy::SecretString;
    use std::sync::Arc;
    use wiremock::matchers::{body_partial_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn model_41(id: &str) -> ModelInfo {
        ModelInfo::new(
            id,
            id,
            opi_ai::WireApi::GoogleVertex,
            ModelCapabilities::new(1_000_000, 65_536)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        )
    }

    fn plain_request_41(thinking: ThinkingConfig) -> Request {
        Request {
            projection: Default::default(),
            transport: Default::default(),
            model: "vertex:gemini-2.5-flash".into(),
            context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                system_prompt: None,
                messages: vec![Message::User(UserMessage {
                    content: vec![InputContent::Text { text: "go".into() }],
                    timestamp_ms: 0,
                })],
                tools: vec![],
            })
            .expect("valid context"),
            max_tokens: Some(64),
            temperature: None,
            sampling_params: Default::default(),
            thinking,
            stop_sequences: vec![],
            metadata: None,
            cancel: tokio_util::sync::CancellationToken::new(),
            timeout: None,
            extra_headers: vec![],
            cache_retention: CacheRetention::None,
            session_id: None,
            telemetry_context: None,
            callbacks: Default::default(),
        }
    }

    fn native_41(
        thinking: Option<GoogleThinkingConfig>,
        tool_choice: Option<GoogleToolChoice>,
        project: Option<&str>,
        location: Option<&str>,
    ) -> GoogleVertexOptions {
        GoogleVertexOptions::native(
            thinking,
            tool_choice,
            project.map(str::to_owned),
            location.map(str::to_owned),
        )
    }

    fn body_41(provider: &VertexProvider, request: &Request) -> serde_json::Value {
        provider
            .build_request_body(request)
            .expect("vertex body compiles")
    }

    fn thinking_config_of(body: &serde_json::Value) -> &serde_json::Value {
        body["generationConfig"]
            .get("thinkingConfig")
            .expect("thinkingConfig present")
    }

    fn tooled_request_41(thinking: ThinkingConfig) -> Request {
        let mut request = plain_request_41(thinking);
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text { text: "go".into() }],
                timestamp_ms: 0,
            })],
            tools: vec![opi_ai::message::ToolDef {
                name: "structured".into(),
                description: "d".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {"path": {"type": "string"}},
                    "required": ["path"]
                }),
                constrained_sampling: None,
            }],
        })
        .expect("valid context declarations");
        request
    }

    fn sse_ok_41() -> ResponseTemplate {
        ResponseTemplate::new(200)
            .set_body_string(text_sse_fixture())
            .insert_header("content-type", "text/event-stream")
    }

    // -- options inventory ------------------------------------------------

    #[test]
    fn vertex_from_simple_maps_tool_choice_only() {
        let simple = SimpleOptions {
            tool_choice: Some(SimpleToolChoice::Auto),
            ..Default::default()
        };
        let options = GoogleVertexOptions::from_simple(&simple);
        assert!(options.is_simple());
        assert!(matches!(options.tool_choice, Some(GoogleToolChoice::Auto)));
        assert!(options.thinking.is_none());
        assert!(options.project.is_none() && options.location.is_none());
        assert!(!GoogleVertexOptions::native(None, None, None, None).is_simple());
    }

    #[test]
    fn vertex_native_thinking_shapes_are_passed_through() {
        let provider = vertex_provider();
        // Level wins over a simultaneous budget; -1 and 0 stay untouched.
        let level = native_41(
            Some(GoogleThinkingConfig {
                enabled: true,
                budget_tokens: Some(-1),
                level: Some(GoogleApiThinkingLevel::High),
            }),
            None,
            None,
            None,
        );
        let body = body_41(
            &provider.clone().with_native_options(level),
            &plain_request_41(ThinkingConfig::default()),
        );
        assert_eq!(
            thinking_config_of(&body).get("thinkingLevel"),
            Some(&serde_json::json!("HIGH"))
        );

        let zero = native_41(
            Some(GoogleThinkingConfig {
                enabled: true,
                budget_tokens: Some(0),
                level: None,
            }),
            None,
            None,
            None,
        );
        let body = body_41(
            &provider.clone().with_native_options(zero),
            &plain_request_41(ThinkingConfig::default()),
        );
        assert_eq!(
            thinking_config_of(&body).get("thinkingBudget"),
            Some(&serde_json::json!(0))
        );
    }

    #[test]
    fn vertex_native_omission_sends_no_thinking_config() {
        let provider = vertex_provider();
        let options = native_41(None, Some(GoogleToolChoice::Auto), None, None);
        let body = body_41(
            &provider.with_native_options(options),
            &plain_request_41(ThinkingConfig::default()),
        );
        assert!(body["generationConfig"].get("thinkingConfig").is_none());
    }

    #[test]
    fn vertex_native_tool_choice_projects_wire_modes() {
        // Pi emits a functionCallingConfig mode only when tools ride the
        // request; the choice modes project directly.
        let provider = vertex_provider();
        for (choice, mode) in [
            (GoogleToolChoice::Any, "ANY"),
            (GoogleToolChoice::None_, "NONE"),
            (GoogleToolChoice::Auto, "AUTO"),
        ] {
            let options = native_41(None, Some(choice), None, None);
            let body = body_41(
                &provider.clone().with_native_options(options),
                &tooled_request_41(ThinkingConfig::default()),
            );
            assert_eq!(
                body["toolConfig"]["functionCallingConfig"]["mode"],
                serde_json::json!(mode),
                "{mode}"
            );
        }
        // Without tools the wire carries no toolConfig regardless of intent
        // (pi resolveGoogleFunctionCallingMode is tools-gated).
        let options = native_41(None, Some(GoogleToolChoice::Any), None, None);
        let body = body_41(
            &provider.with_native_options(options),
            &plain_request_41(ThinkingConfig::default()),
        );
        assert!(body.get("toolConfig").is_none());
    }

    // -- per-wire neutral budget table ------------------------------------

    #[test]
    fn vertex_neutral_budget_table_has_no_flash_lite_entry() {
        let vertex_models: Vec<ModelInfo> = [
            "gemini-2.5-pro",
            "gemini-2.5-flash",
            "gemini-2.5-flash-lite",
            "gemini-2.0-flash",
        ]
        .iter()
        .map(|id| model_41(id))
        .collect();
        let vertex =
            VertexProvider::new("p1".into(), "us-central1".into(), None).with_models(vertex_models);
        for (id, level, expected) in [
            ("gemini-2.5-pro", ThinkingLevel::Medium, 8192i64),
            ("gemini-2.5-pro", ThinkingLevel::High, 32768),
            ("gemini-2.5-flash", ThinkingLevel::Medium, 8192),
            // Pi vertex `getGoogleBudget` has no dedicated flash-lite table:
            // the `2.5-flash` substring match routes flash-lite through the
            // flash table (minimal 128), unlike the generative-ai wire (512).
            ("gemini-2.5-flash-lite", ThinkingLevel::Minimal, 128),
            ("gemini-2.0-flash", ThinkingLevel::Medium, -1),
        ] {
            let mut request = plain_request_41(ThinkingConfig {
                enabled: true,
                budget_tokens: None,
                level,
            });
            request.model = format!("vertex:{id}");
            let body = body_41(&vertex, &request);
            assert_eq!(
                thinking_config_of(&body).get("thinkingBudget"),
                Some(&serde_json::json!(expected)),
                "{id} {level:?}"
            );
        }

        // The generative-ai wire keeps the flash-lite table for the same id.
        let gemini = GeminiProvider::new(None).with_models(vec![ModelInfo::new(
            "gemini-2.5-flash-lite",
            "flash-lite",
            opi_ai::WireApi::GoogleGenerativeAi,
            ModelCapabilities::new(1_000_000, 65_536).with_thinking(true),
        )]);
        let mut request = plain_request_41(ThinkingConfig {
            enabled: true,
            budget_tokens: None,
            level: ThinkingLevel::Minimal,
        });
        request.model = "gemini:gemini-2.5-flash-lite".into();
        let body = gemini
            .build_request_body(&request)
            .expect("gemini body compiles");
        assert_eq!(
            body["generationConfig"]
                .get("thinkingConfig")
                .and_then(|c| c.get("thinkingBudget")),
            Some(&serde_json::json!(512))
        );
    }

    // -- routing: Express vs ADC, project/location ownership --------------

    #[tokio::test]
    async fn vertex_adc_missing_project_or_location_is_typed() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
        for (project, location, expected) in [
            (
                "",
                "us-central1",
                "Vertex AI requires a project ID for ADC routing; pass one in the native options or configure the provider",
            ),
            (
                "p1",
                "",
                "Vertex AI requires a location for ADC routing; pass one in the native options or configure the provider",
            ),
        ] {
            let provider = VertexProvider::new(project.into(), location.into(), Some(server.uri()));
            let request = plain_request_41(ThinkingConfig::default());
            let mut stream = provider.stream_prepared(request.clone(), bearer_auth_41());
            let error = stream
                .next()
                .await
                .expect("one refusal")
                .expect_err("missing ADC routing must fail closed");
            let ProviderError::Config(summary) = accepted_failure_cause(&error, &request) else {
                panic!("expected exact missing-routing cause: {error:?}");
            };
            assert_eq!(summary.as_str(), expected);
            assert!(stream.next().await.is_none(), "refusal must be terminal");
        }
        assert!(server.received_requests().await.unwrap().is_empty());
        server.verify().await;
    }

    #[tokio::test]
    async fn vertex_native_options_override_project_and_location() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p2/locations/europe-west4/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(query_param("alt", "sse"))
            .respond_with(sse_ok_41())
            .expect(1)
            .mount(&server)
            .await;

        let provider = Arc::new(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri()))
                .with_models(vec![model_41("gemini-2.5-flash")]),
        );
        let api = GoogleVertexChatApi::new(provider, model_41("gemini-2.5-flash"))
            .expect("valid vertex record");
        let mut models = Models::new();
        let handle = models
            .register_chat_api(
                Arc::new(api),
                Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::Bearer,
                    SecretString::from("adc-token"),
                )),
                opi_ai::AuthProvenanceSource::Adc {
                    source: opi_ai::auth::AdcCredentialSource::AuthorizedUser {
                        quota_project: None,
                    },
                },
            )
            .expect("register");

        let options = native_41(None, None, Some("p2"), Some("europe-west4"));
        let message = models
            .complete(
                ChatModelRecord::new("vertex", model_41("gemini-2.5-flash")),
                plain_request_41(ThinkingConfig::default()),
                handle.options(options),
            )
            .await
            .expect("override route completes");
        assert_eq!(message.stop_reason, StopReason::Stop);
    }

    #[tokio::test]
    async fn vertex_express_ignores_project_location_options() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(query_param("alt", "sse"))
            .and(header("x-goog-api-key", "express-key"))
            .respond_with(sse_ok_41())
            .expect(1)
            .mount(&server)
            .await;

        let provider = Arc::new(
            VertexProvider::express(Some(server.uri()))
                .with_models(vec![model_41("gemini-2.5-flash")]),
        );
        let api = GoogleVertexChatApi::new(provider, model_41("gemini-2.5-flash"))
            .expect("valid vertex record");
        let mut models = Models::new();
        let handle = models
            .register_chat_api(
                Arc::new(api),
                Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::ApiKey,
                    SecretString::from("express-key"),
                )),
                opi_ai::AuthProvenanceSource::Static,
            )
            .expect("register");

        // Inert under Express (pi: the API-key client carries no
        // project/location).
        let options = native_41(
            Some(GoogleThinkingConfig {
                enabled: true,
                budget_tokens: Some(2048),
                level: None,
            }),
            None,
            Some("p2"),
            Some("europe-west4"),
        );
        let message = models
            .complete(
                ChatModelRecord::new("vertex", model_41("gemini-2.5-flash")),
                plain_request_41(ThinkingConfig::default()),
                handle.options(options),
            )
            .await
            .expect("express route completes");
        assert_eq!(message.stop_reason, StopReason::Stop);
    }

    #[tokio::test]
    async fn vertex_endpoint_override_keeps_project_and_location() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p9/locations/us-east1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(query_param("alt", "sse"))
            .respond_with(sse_ok_41())
            .expect(1)
            .mount(&server)
            .await;

        // A custom base still rebuilds the project/location prefix.
        let provider = VertexProvider::new("p9".into(), "us-east1".into(), Some(server.uri()));
        let url = provider.build_vertex_url("gemini-2.5-flash");
        assert!(url.starts_with(&server.uri()), "{url}");
        assert!(url.contains("projects/p9/locations/us-east1"), "{url}");

        let stream = provider.stream_prepared(
            plain_request_41(ThinkingConfig::default()),
            bearer_auth_41(),
        );
        let mut stream = Box::pin(stream);
        while let Some(item) = stream.next().await {
            if let AssistantStreamEvent::Done { .. } = item.expect("stream item") {
                break;
            }
        }
    }

    // -- direct adapter identity ------------------------------------------

    #[test]
    fn vertex_direct_adapter_rejects_wrong_wire_records() {
        let wrong = ModelInfo::new(
            "gemini-2.5-flash",
            "Gemini (generative-ai wire)",
            opi_ai::WireApi::GoogleGenerativeAi,
            ModelCapabilities::new(1000, 100),
        );
        let provider = Arc::new(vertex_provider());
        let error =
            GoogleVertexChatApi::new(provider, wrong).expect_err("wrong wire must fail typed");
        assert!(matches!(error, ProviderError::Config(_)));
    }

    #[tokio::test]
    async fn vertex_native_lifecycle_through_models_complete() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(query_param("alt", "sse"))
            .and(body_partial_json(serde_json::json!({
                "generationConfig": {"thinkingConfig": {"includeThoughts": true, "thinkingBudget": 2048}}
            })))
            .respond_with(sse_ok_41())
            .expect(1)
            .mount(&server)
            .await;

        let provider = Arc::new(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri()))
                .with_models(vec![model_41("gemini-2.5-flash")]),
        );
        let api = GoogleVertexChatApi::new(provider, model_41("gemini-2.5-flash"))
            .expect("valid vertex record");
        let mut models = Models::new();
        let handle = models
            .register_chat_api(
                Arc::new(api),
                Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::Bearer,
                    SecretString::from("adc-token"),
                )),
                opi_ai::AuthProvenanceSource::Adc {
                    source: opi_ai::auth::AdcCredentialSource::AuthorizedUser {
                        quota_project: Some("quota-proj".into()),
                    },
                },
            )
            .expect("register");

        let options = native_41(
            Some(GoogleThinkingConfig {
                enabled: true,
                budget_tokens: Some(2048),
                level: None,
            }),
            Some(GoogleToolChoice::Any),
            None,
            None,
        );
        let message = models
            .complete(
                ChatModelRecord::new("vertex", model_41("gemini-2.5-flash")),
                plain_request_41(ThinkingConfig::default()),
                handle.options(options),
            )
            .await
            .expect("native lifecycle completes");
        assert_eq!(message.stop_reason, StopReason::Stop);
        assert_eq!(message.provider, "vertex");
    }

    #[tokio::test]
    async fn vertex_telemetry_carrier_instruments_the_producer() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tracing_core::span::{Attributes, Id, Record};
        use tracing_core::{Dispatch, Event, Metadata, Subscriber, dispatcher};

        #[derive(Default)]
        struct Recording {
            next: u64,
            spans: Vec<(&'static str, Option<u64>)>,
        }
        struct Recorder(std::sync::Arc<std::sync::Mutex<Recording>>);

        impl Subscriber for Recorder {
            fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
                true
            }
            fn new_span(&self, attributes: &Attributes<'_>) -> Id {
                let mut state = self.0.lock().unwrap();
                state.next += 1;
                let id = Id::from_u64(state.next);
                let parent = attributes
                    .parent()
                    .cloned()
                    .or_else(|| attributes.is_contextual().then(|| Id::from_u64(1)))
                    .map(|id| id.into_u64());
                state.spans.push((attributes.metadata().name(), parent));
                id
            }
            fn record(&self, _id: &Id, _values: &Record<'_>) {}
            fn record_follows_from(&self, _sub: &Id, _follows: &Id) {}
            fn event(&self, _event: &Event<'_>) {}
            fn enter(&self, _id: &Id) {}
            fn exit(&self, _id: &Id) {}
            fn try_close(&self, _id: Id) -> bool {
                false
            }
        }

        let recorder = std::sync::Arc::new(std::sync::Mutex::new(Recording::default()));
        let source = Dispatch::new(Recorder(recorder.clone()));
        let parent = dispatcher::with_default(&source, || tracing::info_span!("source_parent_41"));
        let context = opi_ai::telemetry::TelemetryContext::from_span(parent);

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(sse_ok_41())
            .expect(1)
            .mount(&server)
            .await;
        let provider = VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri()));

        let seen = Arc::new(AtomicUsize::new(0));
        let counter = seen.clone();
        let mut request = plain_request_41(ThinkingConfig::default());
        request.telemetry_context = Some(context);
        request.callbacks =
            opi_ai::callbacks::RequestCallbacks::new().with_async_on_payload(move |_| {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    tracing::info_span!("payload_probe_41").in_scope(|| ());
                    Ok(None)
                }
            });

        let stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
        let mut stream = Box::pin(stream);
        let mut done = false;
        while let Some(item) = stream.next().await {
            if let AssistantStreamEvent::Done { .. } = item.expect("stream item") {
                done = true;
                break;
            }
        }
        assert!(done, "stream reaches Done");
        assert_eq!(seen.load(Ordering::SeqCst), 1);
        let state = recorder.lock().unwrap();
        assert!(
            state
                .spans
                .iter()
                .any(|(name, _)| *name == "payload_probe_41"),
            "vertex producer telemetry flows to the frozen carrier: {:?}",
            state.spans
        );
    }

    fn bearer_auth_41() -> opi_ai::auth::ResolvedAuth {
        opi_ai::auth::ResolvedAuth {
            env: Default::default(),
            scheme: opi_ai::auth::AuthScheme::Bearer,
            secret: SecretString::from("adc-token"),
            base_url: None,
            account_id: None,
            provenance: Default::default(),
        }
    }

    #[tokio::test]
    async fn node5_vertex_tool_signature_capture_reload_and_real_replay() {
        use opi_ai::message::{AssistantContent, ToolResultMessage};
        let server = MockServer::start().await;
        let raw = serde_json::json!({"candidates":[{"content":{"parts":[{"functionCall":{"id":"call-1","name":"lookup","args":{}},"thoughtSignature":"dmVydGV4"}]},"finishReason":"STOP"}]});
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!("data: {raw}\n\n"))
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(2)
            .mount(&server)
            .await;
        let model = model_41("gemini-3-pro");
        let provider = Arc::new(
            VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri()))
                .with_models(vec![model.clone()]),
        );
        let api = GoogleVertexChatApi::new(provider, model.clone()).unwrap();
        let mut models = Models::new();
        let handle = models
            .register_chat_api(
                Arc::new(api),
                Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::Bearer,
                    SecretString::from("fixture"),
                )),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        let mut request = plain_request_41(Default::default());
        request.model = "vertex:gemini-3-pro".into();
        let first = models
            .complete(
                ChatModelRecord::new("vertex", model.clone()),
                request,
                handle.options(native_41(None, None, None, None)),
            )
            .await
            .unwrap();
        let AssistantContent::ToolCall { tool_call } = &first.content[0] else {
            panic!("captured tool")
        };
        assert_eq!(tool_call.signature.as_deref(), Some("dmVydGV4"));
        let reload = serde_json::from_slice(&serde_json::to_vec(&first).unwrap()).unwrap();
        let mut request = plain_request_41(Default::default());
        request.model = "vertex:gemini-3-pro".into();
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            tools: vec![],
            messages: vec![
                Message::User(UserMessage {
                    content: vec![InputContent::Text { text: "go".into() }],
                    timestamp_ms: 0,
                }),
                Message::Assistant(reload),
                Message::ToolResult(ToolResultMessage {
                    tool_call_id: "call-1".into(),
                    tool_name: "lookup".into(),
                    content: vec![],
                    details: None,
                    is_error: false,
                    not_executed: None,
                    truncated: false,
                    timestamp_ms: 0,
                }),
            ],
        })
        .unwrap();
        models
            .complete_simple(
                ChatModelRecord::new("vertex", model),
                request,
                Default::default(),
            )
            .await
            .unwrap();
        let sent = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
        assert!(
            body["contents"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|content| content["parts"].as_array().unwrap())
                .any(|part| part.get("functionCall").is_some()
                    && part["thoughtSignature"] == "dmVydGV4")
        );
    }
}
