//! Vertex GoogleVertexChatApi adapter assertions.

use super::fixtures::*;
use super::*;
use opi_ai::api::contract::{SimpleOptions, SimpleToolChoice};
use opi_ai::api::gemini::{GoogleApiThinkingLevel, GoogleThinkingConfig, GoogleToolChoice};
use opi_ai::api::vertex::{GoogleVertexChatApi, GoogleVertexOptions, VertexBodyBuilder};
use opi_ai::model_info::{ModelCapabilities, ModelInfo, ThinkingLevel};
use opi_ai::models::{ChatModelRecord, Models};
use opi_ai::providers::gemini::GeminiProvider;
use opi_ai::stream::StopReason;
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

fn body_41(provider: &impl VertexBodyBuilder, request: &Request) -> serde_json::Value {
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
fn vertex_native_disabled_thinking_uses_the_lowest_supported_level() {
    use opi_ai::model_info::{ThinkingLevelMap, ThinkingLevelMapping};

    let id = "gemini-4-argon";
    let model = model_41(id).with_thinking_level_map(
        ThinkingLevelMap::reasoning_default()
            .with_mapping(ThinkingLevel::None, ThinkingLevelMapping::Unsupported)
            .with_mapping(ThinkingLevel::Minimal, ThinkingLevelMapping::Unsupported)
            .with_mapping(
                ThinkingLevel::Low,
                ThinkingLevelMapping::Mapped("medium".into()),
            ),
    );
    let provider = vertex_provider()
        .with_models(vec![model])
        .with_native_options(native_41(
            Some(GoogleThinkingConfig {
                enabled: false,
                budget_tokens: None,
                level: None,
            }),
            None,
            None,
            None,
        ));
    let mut request = plain_request_41(ThinkingConfig::default());
    request.model = format!("vertex:{id}");
    let body = body_41(&provider, &request);
    assert_eq!(
        thinking_config_of(&body),
        &serde_json::json!({"thinkingLevel": "MEDIUM"})
    );
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
        VertexProvider::express(Some(server.uri())).with_models(vec![model_41("gemini-2.5-flash")]),
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
    let error = GoogleVertexChatApi::new(provider, wrong).expect_err("wrong wire must fail typed");
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

#[cfg(feature = "telemetry-tracing")]
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
            .any(
                |part| part.get("functionCall").is_some() && part["thoughtSignature"] == "dmVydGV4"
            )
    );
}
