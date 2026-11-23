//! Vertex and cross-provider builtin route assertions.

use super::fixtures::*;

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
        api: Arc<dyn opi_ai::api::contract::ChatApi<O>>,
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
            opi_ai::api::gemini::GoogleGenerativeAiChatApi::new(
                Arc::new(opi_ai::providers::gemini::GeminiProvider::new(Some(
                    server.uri(),
                ))),
                google.clone(),
            )
            .unwrap(),
        ),
        google,
        opi_ai::api::gemini::GoogleGenerativeAiOptions::default(),
    )
    .await;
    let vertex = model(opi_ai::WireApi::GoogleVertex);
    check(
        Arc::new(
            opi_ai::api::vertex::GoogleVertexChatApi::new(
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
        opi_ai::api::vertex::GoogleVertexOptions::default(),
    )
    .await;
    let bedrock = model(opi_ai::WireApi::BedrockConverseStream);
    check(
        Arc::new(
            opi_ai::api::bedrock::BedrockConverseStreamChatApi::new(
                Arc::new(opi_ai::providers::bedrock::BedrockProvider::new(
                    Some(server.uri()),
                    Arc::new(opi_ai::http::HttpClient::new()),
                )),
                bedrock.clone(),
            )
            .unwrap(),
        ),
        bedrock,
        opi_ai::api::bedrock::BedrockOptions::default(),
    )
    .await;
    let mistral = model(opi_ai::WireApi::MistralConversations);
    check(
        Arc::new(
            opi_ai::api::mistral::MistralConversationsChatApi::new(
                opi_ai::providers::mistral::MistralConversationsProvider::new(Some(server.uri())),
                mistral.clone(),
            )
            .unwrap(),
        ),
        mistral,
        opi_ai::api::mistral::MistralOptions::default(),
    )
    .await;
    let pi = model(opi_ai::WireApi::PiMessages);
    check(
        Arc::new(
            opi_ai::api::pi_messages::PiMessagesChatApi::new(
                opi_ai::providers::pi_messages::PiMessagesProvider::new(Some(server.uri())),
                pi.clone(),
            )
            .unwrap(),
        ),
        pi,
        opi_ai::api::pi_messages::PiMessagesOptions::native(None, None, false),
    )
    .await;
    assert!(server.received_requests().await.unwrap().is_empty());
}

use super::*;
use opi_ai::auth::{AuthContext, AuthContextError};
use opi_ai::message::{AssistantContent, ToolResultMessage};
use opi_ai::providers::collection::{BuiltinOverrides, register_builtin};
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
                        opi_ai::api::vertex::GoogleVertexChatApi::new(
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
                    handle.options(opi_ai::api::vertex::GoogleVertexOptions::default()),
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
            request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
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
        let (server, models, mut model, reloaded) = capture_signed_google_parts(provider, 2).await;
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
    overrides.base_urls.insert(
        "google-vertex".into(),
        format!("{}/proxy/v1beta1", server.uri()),
    );
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
        "/proxy/v1beta1/publishers/google/models/gemini-2.5-flash:streamGenerateContent"
    );
}
