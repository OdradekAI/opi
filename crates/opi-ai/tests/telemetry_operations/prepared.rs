//! Frozen telemetry across prepared Anthropic requests and admitted retries.

use opi_ai::auth::{AuthProvenanceSource, AuthScheme, StaticAuthResolver};
use opi_ai::callbacks::RequestCallbacks;
use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
use opi_ai::http::HttpClientBuilder;
use opi_ai::message::ToolDef;
use opi_ai::model_info::{AnthropicMessagesCompat, WireCompat};
use opi_ai::models::Models;
use opi_ai::provider::{ProviderError, Request, ThinkingConfig};
use opi_ai::providers::anthropic::AnthropicProvider;
use opi_ai::retry::AcquisitionPolicy;
use opi_ai::stream::ChatStream;
use opi_ai::telemetry::TelemetryContext;
use opi_ai::transcript::{Context, normalize_context};
use opi_ai::{ModelCapabilities, ModelInfo, WireApi};
use tracing::Level;
use tracing_core::dispatcher;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::fixtures::{
    PROMPT_CANARY, SECRET_CANARY, assert_all_parented, assert_no_canaries, message_text,
    probe_spans, recorder,
};

const ANTHROPIC_SSE_OK: &str = r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_abc","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":25,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":15}}

event: message_stop
data: {"type":"message_stop"}

"#;

const STRICT_LIMITS_400: &str = r#"{"type":"error","error":{"type":"invalid_request_error","message":"Too many strict tools (65). Reduce the number of strict tools or convert some to non-strict."}}"#;

async fn anthropic_models(base_url: String) -> Models {
    let model = ModelInfo::new(
        "strict-model",
        "Strict Model",
        WireApi::AnthropicMessages,
        ModelCapabilities::new(1000, 100),
    )
    .with_compat(WireCompat::AnthropicMessages(AnthropicMessagesCompat {
        supports_eager_tool_input_streaming: false,
        force_adaptive_thinking: false,
        supports_temperature: true,
        supports_strict_tools: true,
        supports_mid_convo_system_messages: false,
        supports_mid_convo_tool_changes: false,
        ..Default::default()
    }))
    .expect("valid compat");
    let provider = AnthropicProvider::for_route(
        "test".into(),
        vec![model],
        Some(base_url),
        Default::default(),
        std::sync::Arc::new(HttpClientBuilder::new().build().expect("client build")),
        false,
    );
    let mut models = Models::new();
    models
        .register_route(
            Box::new(provider),
            std::sync::Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                secrecy::SecretString::from(SECRET_CANARY),
            )),
            AuthProvenanceSource::Static,
            opi_ai::models::CompatMetadata {
                openai_compatible: false,
                profile: None,
            },
        )
        .expect("register anthropic route");
    models
}

fn strict_tool(name: &str, preference: StrictPreference) -> ToolDef {
    ToolDef {
        name: name.into(),
        description: "telemetry conformance tool".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {"path": {"type": "string"}},
            "required": ["path"]
        }),
        constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict: preference }),
    }
}

fn strict_request(context: Option<TelemetryContext>, callbacks: RequestCallbacks) -> Request {
    Request {
        projection: Default::default(),
        model: "test:strict-model".into(),
        context: normalize_context(Context {
            system_prompt: None,
            messages: vec![opi_ai::message::Message::User(
                opi_ai::message::UserMessage {
                    content: vec![opi_ai::message::InputContent::Text {
                        text: PROMPT_CANARY.into(),
                    }],
                    timestamp_ms: 0,
                },
            )],
            tools: vec![strict_tool("prefer-tool", StrictPreference::Prefer)],
        })
        .expect("valid context declarations"),
        max_tokens: Some(64),
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: Default::default(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: context,
        callbacks,
        transport: Default::default(),
    }
}

async fn wait_retry(admission: opi_ai::retry::RetryAdmission) {
    tokio::time::sleep(std::time::Duration::from_millis(admission.delay_ms.max(1))).await;
}

#[tokio::test]
async fn advanced_prepare_retries_and_strict_variants_keep_the_frozen_context() {
    let (a_recording, a_dispatch) = recorder();
    let (b_recording, b_dispatch) = recorder();

    let server = MockServer::start().await;
    // Variant 1 (prefer-strict): one source-qualified strict-limit 400.
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(400).set_body_string(STRICT_LIMITS_400))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    // Variant 2 (rebuilt, prefer removed): one retryable 429, then success.
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(429))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(ANTHROPIC_SSE_OK),
        )
        .mount(&server)
        .await;
    let models = anthropic_models(server.uri()).await;

    let parent =
        dispatcher::with_default(&a_dispatch, || tracing::span!(Level::INFO, "caller_parent"));
    let parent_id = parent.id().unwrap().into_u64();
    let context = dispatcher::with_default(&b_dispatch, || TelemetryContext::from_span(parent));

    // Payload and response callbacks run inside the instrumented producer;
    // they are the host's observation probes for each attempt.
    let callbacks = RequestCallbacks::new()
        .with_on_payload(std::sync::Arc::new(|_body| {
            tracing::span!(Level::INFO, "attempt_payload_marker").in_scope(|| ());
            Ok(None)
        }))
        .with_on_response(std::sync::Arc::new(|_status, _headers| {
            tracing::span!(Level::INFO, "attempt_response_marker").in_scope(|| ());
            Ok(())
        }));

    let mut policy = AcquisitionPolicy::with_max_retries(1);
    policy.jitter_seed = Some(0);
    let call = models
        .prepare_call_with_policy(
            "test:strict-model",
            strict_request(Some(context), callbacks),
            policy,
        )
        .await
        .expect("prepare call");

    // Variant 1: the strict-limit 400 fails the first variant.
    let first = ChatStream::new(call.start_attempt().expect("v1 send 1"))
        .result()
        .await
        .expect_err("strict-limit 400 fails the first variant");
    assert!(matches!(
        first.cause(),
        ProviderError::StrictToolLimits { .. }
    ));
    assert!(call.retry_admission().is_none());
    assert!(call.strict_variant_admission().is_some());

    // Variant 2: one ordinary retry, then success.
    let second = ChatStream::new(call.start_attempt().expect("v2 send 1"))
        .result()
        .await
        .expect_err("429 on the rebuilt variant");
    assert!(matches!(second.cause(), ProviderError::RateLimited { .. }));
    wait_retry(call.retry_admission().expect("ordinary retry admission")).await;
    let message = ChatStream::new(call.start_attempt().expect("v2 send 2"))
        .result()
        .await
        .expect("variant 2 completes");
    assert_eq!(message_text(&message), "Hello");

    // Every attempt's producer sections observed the frozen parent on A.
    // Payload callbacks freeze once per payload variant (prefer-strict and
    // the rebuilt one); the response callback fires only on the acquired
    // final response — together they cover variant 1, the rebuilt variant's
    // first send, and its post-retry success.
    let spans = probe_spans(&a_recording);
    let payloads = spans
        .iter()
        .filter(|(name, _)| *name == "attempt_payload_marker")
        .count();
    let responses = spans
        .iter()
        .filter(|(name, _)| *name == "attempt_response_marker")
        .count();
    assert_eq!(
        payloads, 2,
        "payload callbacks ran once per payload variant"
    );
    assert_eq!(
        responses, 1,
        "response callback fired on the acquired response"
    );
    assert_all_parented(&spans, parent_id);
    assert!(
        b_recording.lock().unwrap().spans.is_empty(),
        "ambient B must never observe the retried operation"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
    assert_no_canaries(&a_recording);

    // Invariance twin: identical wire sequence without any context produces
    // the same sends and the same final message, and adds nothing to A.
    let spans_before_twin = probe_spans(&a_recording).len();
    let twin = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(400).set_body_string(STRICT_LIMITS_400))
        .up_to_n_times(1)
        .mount(&twin)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(429))
        .up_to_n_times(1)
        .mount(&twin)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(ANTHROPIC_SSE_OK),
        )
        .mount(&twin)
        .await;
    let models = anthropic_models(twin.uri()).await;
    let mut policy = AcquisitionPolicy::with_max_retries(1);
    policy.jitter_seed = Some(0);
    let call = models
        .prepare_call_with_policy(
            "test:strict-model",
            strict_request(None, RequestCallbacks::default()),
            policy,
        )
        .await
        .expect("prepare twin");
    assert!(
        ChatStream::new(call.start_attempt().expect("twin v1"))
            .result()
            .await
            .is_err()
    );
    assert!(
        ChatStream::new(call.start_attempt().expect("twin v2 send 1"))
            .result()
            .await
            .is_err()
    );
    wait_retry(call.retry_admission().expect("twin retry admission")).await;
    let twin_message = ChatStream::new(call.start_attempt().expect("twin v2 send 2"))
        .result()
        .await
        .expect("twin completes");
    assert_eq!(message_text(&twin_message), "Hello");
    assert_eq!(twin.received_requests().await.unwrap().len(), 3);
    assert_eq!(
        probe_spans(&a_recording).len(),
        spans_before_twin,
        "the context-free twin must not add SDK observation"
    );
}
