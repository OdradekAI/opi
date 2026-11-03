//! pi-messages adapter fixture tests (task 20.44, DEL-11).
//!
//! The pi-messages wire is pi's own message protocol spoken by the Radius
//! gateway: one POST of `{ model, context, options }` to `<baseUrl>/messages`
//! answered by an SSE stream of serialized assistant-message events plus a
//! terminal done/error event. These tests pin the pinned-source contracts
//! (`.repo/pi-1.3.0/packages/ai/src/api/pi-messages.ts`): the exact six-field
//! options whitelist with omission-preserving semantics, the simple bypass
//! (no invented clamp, no default maxTokens, no context clamp), the canonical
//! chronology projection into pi's camelCase wire JSON, typed event
//! validation (sparse indices, unknown shapes, missing terminal), the
//! rewrite-impact diagnostic, HTTP error formatting and the lawful
//! send/callback ordering.

use futures_util::StreamExt;
use opi_ai::callbacks::RequestCallbacks;
use opi_ai::message::{InputContent, Message, ToolDef, UserMessage};
use opi_ai::provider::{Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::stream::AssistantStreamEvent;
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use opi_ai::WireApi;
use opi_ai::model_info::{ThinkingLevel, ThinkingLevelMap, ThinkingLevelMapping};
use opi_ai::pi_messages::{
    PiMessagesChatApi, PiMessagesOptions, PiMessagesProvider, PiMessagesToolChoice,
    resolve_wire_cache_retention,
};
use opi_ai::provider::CacheRetention;
use opi_ai::registry::ModelCapabilities;
use opi_ai::stream::StopReason;

const MODEL_44: &str = "claude-sonnet-4-5";

fn model_44(id: &str) -> opi_ai::provider::ModelInfo {
    opi_ai::provider::ModelInfo::new(
        id,
        id,
        WireApi::PiMessages,
        ModelCapabilities::new(200_000, 16_384).with_streaming(true),
    )
}

fn thinking_model_44() -> opi_ai::provider::ModelInfo {
    let mut model = model_44(MODEL_44);
    model.capabilities = model.capabilities.with_thinking(true);
    model.thinking_level_map = model.thinking_level_map.with_mapping(
        ThinkingLevel::High,
        ThinkingLevelMapping::Mapped("high".into()),
    );
    model
}

fn clamped_model_44() -> opi_ai::provider::ModelInfo {
    // An explicit map that would clamp High to "max" if any clamp ran; the
    // pi-messages simple bypass must ignore it entirely.
    let mut map = ThinkingLevelMap::disabled();
    map = map.with_mapping(
        ThinkingLevel::High,
        ThinkingLevelMapping::Mapped("max".into()),
    );
    let mut model = model_44(MODEL_44);
    model.thinking_level_map = map;
    model
}

fn req_44(user: &str) -> Request {
    let mut request = base_req_44();
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text { text: user.into() }],
            timestamp_ms: 0,
        })],
        tools: vec![],
    })
    .expect("valid context declarations");
    request
}

fn base_req_44() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: format!("radius:{MODEL_44}"),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
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

fn sse_ok_44() -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_string(concat!(
            "data: {\"type\":\"start\"}\n\n",
            "data: {\"type\":\"text_start\",\"contentIndex\":0}\n\n",
            "data: {\"type\":\"text_delta\",\"contentIndex\":0,\"delta\":\"Hel\"}\n\n",
            "data: {\"type\":\"text_end\",\"contentIndex\":0,\"content\":\"Hello\"}\n\n",
            "data: {\"type\":\"done\",\"reason\":\"stop\",\"usage\":{\"input\":10,\"output\":5,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":15,\"cost\":{\"input\":0.1,\"output\":0.2,\"cacheRead\":0,\"cacheWrite\":0,\"total\":0.3}},\"responseId\":\"resp-1\"}\n\n",
            "data: [DONE]\n\n",
        ))
        .insert_header("content-type", "text/event-stream")
}

async fn mount_44(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(response)
        .expect(1)
        .mount(server)
        .await;
}

async fn body_44(server: &MockServer) -> serde_json::Value {
    let requests = server.received_requests().await.expect("captured requests");
    serde_json::from_slice(&requests[0].body).expect("request body is JSON")
}

async fn run_44(
    provider: &PiMessagesProvider,
    request: Request,
) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
    let stream = provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    stream.collect().await
}

async fn done_message_44(
    events: &[Result<AssistantStreamEvent, ProviderError>],
) -> opi_ai::message::AssistantMessage {
    events
        .iter()
        .find_map(|item| match item {
            Ok(AssistantStreamEvent::Done { message, .. }) => Some(message.clone()),
            _ => None,
        })
        .expect("stream reaches Done")
}

async fn first_error_44(events: &[Result<AssistantStreamEvent, ProviderError>]) -> ProviderError {
    events
        .iter()
        .find_map(|item| match item {
            Err(ProviderError::StreamFailure(failure)) => Some(failure.cause().clone()),
            Err(error) => Some(error.clone()),
            _ => None,
        })
        .expect("stream fails typed")
}

// ---------------------------------------------------------------------------
// identity and endpoint
// ---------------------------------------------------------------------------

#[test]
fn provider_identity_and_curated_catalog() {
    let provider = PiMessagesProvider::new(None);
    assert_eq!(provider.id(), "radius");
    let models = provider.models();
    assert!(
        !models.is_empty(),
        "curated baseline covers pre-catalog time"
    );
    assert!(
        models
            .iter()
            .all(|model| model.wire_api == WireApi::PiMessages)
    );
}

#[tokio::test]
async fn endpoint_is_messages_path_and_trailing_slashes_strip() {
    let server = MockServer::start().await;
    mount_44(&server, sse_ok_44()).await;
    let provider = PiMessagesProvider::new(Some(format!("{}/", server.uri())))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    assert!(
        events
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
    );
    server.verify().await;
}

// ---------------------------------------------------------------------------
// wire body: exact whitelist, omissions, debug
// ---------------------------------------------------------------------------

#[tokio::test]
async fn wire_body_is_exact_whitelist() {
    let server = MockServer::start().await;
    mount_44(&server, sse_ok_44()).await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(
            Some(ThinkingLevel::High),
            Some(PiMessagesToolChoice::Function {
                name: "lookup".into(),
            }),
            false,
        ));
    let mut request = req_44("hi");
    request.temperature = Some(0.7);
    request.max_tokens = Some(1200);
    request.session_id = Some("sess-44".into());
    request.cache_retention = CacheRetention::Long;
    run_44(&provider, request).await;

    let body = body_44(&server).await;
    let mut top: Vec<&str> = body
        .as_object()
        .expect("object body")
        .keys()
        .map(String::as_str)
        .collect();
    top.sort_unstable();
    assert_eq!(top, vec!["context", "model", "options"]);
    assert_eq!(body["model"], serde_json::json!(MODEL_44));

    let options = &body["options"];
    let mut keys: Vec<&str> = options
        .as_object()
        .expect("options object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "cacheRetention",
            "maxTokens",
            "reasoning",
            "sessionId",
            "temperature",
            "toolChoice"
        ],
        "the pi-messages whitelist is exactly these six members"
    );
    assert_eq!(options["temperature"], serde_json::json!(0.7));
    assert_eq!(options["maxTokens"], serde_json::json!(1200));
    assert_eq!(options["reasoning"], serde_json::json!("high"));
    assert_eq!(options["cacheRetention"], serde_json::json!("long"));
    assert_eq!(options["sessionId"], serde_json::json!("sess-44"));
    assert_eq!(
        options["toolChoice"],
        serde_json::json!({"type": "function", "function": {"name": "lookup"}})
    );
    server.verify().await;
}

#[tokio::test]
async fn omissions_stay_omitted_without_defaults() {
    // PI_CACHE_RETENTION is process-global; hold the env lock so the
    // legacy-env test cannot race this omission assertion.
    let _guard = env_lock_44().lock().await;
    let server = MockServer::start().await;
    mount_44(&server, sse_ok_44()).await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    run_44(&provider, req_44("hi")).await;

    let body = body_44(&server).await;
    assert_eq!(
        body["options"],
        serde_json::json!({}),
        "no temperature/maxTokens/reasoning/cacheRetention/sessionId/toolChoice is invented"
    );
    server.verify().await;
}

#[tokio::test]
async fn debug_flag_adds_query_parameter() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(query_param("debug", "1"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(sse_ok_44())
        .expect(1)
        .mount(&server)
        .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, true));
    let events = run_44(&provider, req_44("hi")).await;
    assert!(
        events
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
    );
    server.verify().await;
}

#[tokio::test]
async fn reasoning_wire_values_including_off() {
    for (level, wire) in [
        (ThinkingLevel::None, "off"),
        (ThinkingLevel::Minimal, "minimal"),
        (ThinkingLevel::Low, "low"),
        (ThinkingLevel::Medium, "medium"),
        (ThinkingLevel::High, "high"),
        (ThinkingLevel::XHigh, "xhigh"),
        (ThinkingLevel::Max, "max"),
    ] {
        let server = MockServer::start().await;
        mount_44(&server, sse_ok_44()).await;
        let provider = PiMessagesProvider::new(Some(server.uri()))
            .with_models(vec![model_44(MODEL_44)])
            .with_native_options(PiMessagesOptions::native(Some(level), None, false));
        run_44(&provider, req_44("hi")).await;
        let body = body_44(&server).await;
        assert_eq!(body["options"]["reasoning"], serde_json::json!(wire));
        server.verify().await;
    }
}

#[tokio::test]
async fn tool_choice_wire_modes() {
    let cases = [
        (PiMessagesToolChoice::Auto, serde_json::json!("auto")),
        (PiMessagesToolChoice::None_, serde_json::json!("none")),
        (
            PiMessagesToolChoice::Required,
            serde_json::json!("required"),
        ),
        (
            PiMessagesToolChoice::Function {
                name: "search".into(),
            },
            serde_json::json!({"type": "function", "function": {"name": "search"}}),
        ),
    ];
    for (choice, wire) in cases {
        let server = MockServer::start().await;
        mount_44(&server, sse_ok_44()).await;
        let provider = PiMessagesProvider::new(Some(server.uri()))
            .with_models(vec![model_44(MODEL_44)])
            .with_native_options(PiMessagesOptions::native(None, Some(choice), false));
        run_44(&provider, req_44("hi")).await;
        let body = body_44(&server).await;
        assert_eq!(body["options"]["toolChoice"], wire);
        server.verify().await;
    }
}

// ---------------------------------------------------------------------------
// cache retention resolution (explicit request value, legacy env opt-in)
// ---------------------------------------------------------------------------

#[test]
fn cache_retention_resolution_matrix() {
    // Unset keeps the backend default: only the legacy explicit "long"
    // opt-in maps without an explicit request value.
    assert_eq!(
        resolve_wire_cache_retention(CacheRetention::None, None),
        None
    );
    assert_eq!(
        resolve_wire_cache_retention(CacheRetention::None, Some("long")),
        Some("long")
    );
    assert_eq!(
        resolve_wire_cache_retention(CacheRetention::None, Some("short")),
        None
    );
    assert_eq!(
        resolve_wire_cache_retention(CacheRetention::None, Some("none")),
        None
    );
    // Explicit request values win and map one-to-one onto pi's wire values.
    assert_eq!(
        resolve_wire_cache_retention(CacheRetention::Disabled, Some("long")),
        Some("none")
    );
    assert_eq!(
        resolve_wire_cache_retention(CacheRetention::Short, None),
        Some("short")
    );
    assert_eq!(
        resolve_wire_cache_retention(CacheRetention::Long, None),
        Some("long")
    );
}

static ENV_LOCK_44: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
fn env_lock_44() -> &'static tokio::sync::Mutex<()> {
    ENV_LOCK_44.get_or_init(|| tokio::sync::Mutex::new(()))
}

#[tokio::test]
async fn legacy_env_long_maps_when_request_unset() {
    let guard = env_lock_44().lock().await;
    unsafe { std::env::set_var("PI_CACHE_RETENTION", "long") };
    let server = MockServer::start().await;
    mount_44(&server, sse_ok_44()).await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    run_44(&provider, req_44("hi")).await;
    unsafe { std::env::remove_var("PI_CACHE_RETENTION") };
    drop(guard);

    let body = body_44(&server).await;
    assert_eq!(body["options"]["cacheRetention"], serde_json::json!("long"));
    server.verify().await;
}

// ---------------------------------------------------------------------------
// canonical chronology projection (pi camelCase wire JSON)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn context_projects_pi_wire_json() {
    let server = MockServer::start().await;
    mount_44(&server, sse_ok_44()).await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));

    let context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: Some("Be brief.".into()),
        tools: vec![ToolDef {
            name: "lookup".into(),
            description: "Look things up.".into(),
            input_schema: serde_json::json!({"type": "object"}),
            constrained_sampling: None,
        }],
        messages: vec![
            Message::User(UserMessage {
                content: vec![
                    InputContent::Text { text: "see".into() },
                    InputContent::Image {
                        source: opi_ai::message::ImageSource::Base64 {
                            data: "aW1n".into(),
                        },
                        media_type: opi_ai::message::MediaType::Png,
                    },
                ],
                timestamp_ms: 1_000,
            }),
            serde_json::from_value::<opi_ai::message::Message>(serde_json::json!({
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "Found it.", "signature": "sig-t"},
                    {"type": "thinking", "thinking": "hmm", "signature": "sig-h"},
                    {"type": "redacted_thinking", "data": "enc-1"},
                    {"type": "tool_call", "tool_call": {
                        "id": "call-1", "name": "lookup", "arguments": "{\"q\":\"x\"}"
                    }}
                ],
                "api": "Anthropic",
                "provider": "radius",
                "model": "claude-sonnet-4-5",
                "wire": "anthropic-messages",
                "response_id": "resp-9",
                "provider_thinking_level": "high",
                "usage": {
                    "input_tokens": 4, "output_tokens": 2, "cache_read_tokens": 1,
                    "total_tokens": 6
                },
                "stop_reason": "tool_use",
                "raw_stop_reason": "tool_use",
                "error_message": null,
                "timestamp_ms": 2_000
            }))
            .expect("assistant message"),
            serde_json::from_value::<opi_ai::message::Message>(serde_json::json!({
                "role": "tool_result",
                "tool_call_id": "call-1",
                "tool_name": "lookup",
                "content": [
                    {"type": "text", "text": "result"},
                    {"type": "image", "source": {"type": "base64", "data": "aW1n"},
                     "media_type": "image/png"}
                ],
                "details": {"rows": 3},
                "is_error": false,
                "not_executed": null,
                "truncated": false,
                "timestamp_ms": 3_000
            }))
            .expect("tool result message"),
        ],
    })
    .expect("valid context");
    let mut request = base_req_44();
    request.context = context;
    run_44(&provider, request).await;

    let body = body_44(&server).await;
    let messages = body["context"]["messages"]
        .as_array()
        .expect("context messages");
    assert_eq!(messages.len(), 4);

    let system = &messages[0];
    assert_eq!(system["role"], serde_json::json!("system"));
    assert_eq!(system["content"], serde_json::json!("Be brief."));
    assert_eq!(
        system["toolsAdded"][0],
        serde_json::json!({
            "name": "lookup",
            "description": "Look things up.",
            "parameters": {"type": "object"}
        })
    );
    assert_eq!(system["timestamp"], serde_json::json!(0));

    let user = &messages[1];
    assert_eq!(user["role"], serde_json::json!("user"));
    assert_eq!(
        user["content"],
        serde_json::json!([
            {"type": "text", "text": "see"},
            {"type": "image", "data": "aW1n", "mimeType": "image/png"}
        ])
    );
    assert_eq!(user["timestamp"], serde_json::json!(1_000));

    let assistant = &messages[2];
    assert_eq!(assistant["role"], serde_json::json!("assistant"));
    assert_eq!(assistant["api"], serde_json::json!("anthropic-messages"));
    assert_eq!(assistant["provider"], serde_json::json!("radius"));
    assert_eq!(assistant["model"], serde_json::json!("claude-sonnet-4-5"));
    assert_eq!(assistant["responseId"], serde_json::json!("resp-9"));
    assert_eq!(
        assistant["providerThinkingLevel"],
        serde_json::json!("high")
    );
    assert_eq!(assistant["stopReason"], serde_json::json!("toolUse"));
    assert_eq!(assistant["rawStopReason"], serde_json::json!("tool_use"));
    assert_eq!(
        assistant["content"][0],
        serde_json::json!({"type": "text", "text": "Found it.", "textSignature": "sig-t"})
    );
    assert_eq!(
        assistant["content"][1],
        serde_json::json!({
            "type": "thinking", "thinking": "hmm", "thinkingSignature": "sig-h"
        })
    );
    assert_eq!(
        assistant["content"][2],
        serde_json::json!({
            "type": "thinking", "thinking": "", "thinkingSignature": "enc-1", "redacted": true
        })
    );
    assert_eq!(
        assistant["content"][3],
        serde_json::json!({
            "type": "toolCall", "id": "call-1", "name": "lookup", "arguments": {"q": "x"}
        })
    );
    assert_eq!(
        assistant["usage"],
        serde_json::json!({
            "input": 4, "output": 2, "cacheRead": 1, "cacheWrite": 0, "totalTokens": 6
        })
    );

    let tool_result = &messages[3];
    assert_eq!(tool_result["role"], serde_json::json!("toolResult"));
    assert_eq!(tool_result["toolCallId"], serde_json::json!("call-1"));
    assert_eq!(tool_result["toolName"], serde_json::json!("lookup"));
    assert_eq!(
        tool_result["content"],
        serde_json::json!([
            {"type": "text", "text": "result"},
            {"type": "image", "data": "aW1n", "mimeType": "image/png"}
        ])
    );
    assert_eq!(tool_result["details"], serde_json::json!({"rows": 3}));
    assert_eq!(tool_result["isError"], serde_json::json!(false));
    assert_eq!(tool_result["timestamp"], serde_json::json!(3_000));
    server.verify().await;
}

#[tokio::test]
async fn mid_conversation_system_messages_stay_native() {
    let server = MockServer::start().await;
    mount_44(&server, sse_ok_44()).await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));

    let mid_system: Message = serde_json::from_value(
        serde_json::json!({"role": "system", "content": "more", "timestamp_ms": 5}),
    )
    .expect("system message");
    let context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: Some("base".into()),
        tools: vec![],
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text { text: "one".into() }],
                timestamp_ms: 1,
            }),
            mid_system,
        ],
    })
    .expect("valid context");
    let mut request = base_req_44();
    request.context = context;
    run_44(&provider, request).await;

    let body = body_44(&server).await;
    let messages = body["context"]["messages"]
        .as_array()
        .expect("context messages");
    assert_eq!(messages.len(), 3, "chronology stays native, no collapse");
    assert_eq!(messages[2]["role"], serde_json::json!("system"));
    assert_eq!(messages[2]["content"], serde_json::json!("more"));
    server.verify().await;
}

// ---------------------------------------------------------------------------
// SSE consumption: blocks, sparse indices, terminals
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sse_text_thinking_tool_with_sparse_indices() {
    let server = MockServer::start().await;
    mount_44(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(concat!(
                "data: {\"type\":\"start\"}\n\n",
                "data: {\"type\":\"text_start\",\"contentIndex\":0}\n\n",
                "data: {\"type\":\"text_delta\",\"contentIndex\":0,\"delta\":\"Hel\"}\n\n",
                "data: {\"type\":\"text_end\",\"contentIndex\":0,\"content\":\"Hello\",\"contentSignature\":\"sig-t\"}\n\n",
                "data: {\"type\":\"toolcall_start\",\"contentIndex\":2,\"id\":\"call-9\",\"toolName\":\"lookup\"}\n\n",
                "data: {\"type\":\"toolcall_delta\",\"contentIndex\":2,\"delta\":\"{\\\"q\\\":\"}\n\n",
                "data: {\"type\":\"toolcall_delta\",\"contentIndex\":2,\"delta\":\"\\\"x\\\"}\"}\n\n",
                "data: {\"type\":\"toolcall_end\",\"contentIndex\":2,\"toolCall\":{\"id\":\"call-9\",\"name\":\"lookup\",\"arguments\":{\"q\":\"x\"}}}\n\n",
                "data: {\"type\":\"done\",\"reason\":\"toolUse\",\"usage\":{\"input\":1,\"output\":1,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":2},\"responseId\":\"resp-2\"}\n\n",
            ))
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;

    assert!(
        events
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Start { .. })))
    );
    let end = events.iter().find_map(|item| match item {
        Ok(AssistantStreamEvent::TextEnd { content, .. }) => Some(content.clone()),
        _ => None,
    });
    assert_eq!(end.as_deref(), Some("Hello"));
    let tool_end = events.iter().find_map(|item| match item {
        Ok(AssistantStreamEvent::ToolCallEnd { tool_call, .. }) => Some(tool_call.clone()),
        _ => None,
    });
    let tool_call = tool_end.expect("tool call completes");
    assert_eq!(tool_call.id, "call-9");
    assert_eq!(tool_call.name, "lookup");
    assert_eq!(tool_call.arguments, "{\"q\":\"x\"}");

    let message = done_message_44(&events).await;
    assert_eq!(message.stop_reason, StopReason::ToolUse);
    assert_eq!(message.response_id.as_deref(), Some("resp-2"));
    let content = &message.content;
    assert_eq!(
        content.len(),
        2,
        "the unstarted hole at index 1 stays empty"
    );
    assert!(matches!(
        &content[0],
        opi_ai::message::AssistantContent::Text { text, signature }
            if text == "Hello" && signature.as_deref() == Some("sig-t")
    ));
    assert!(matches!(
        &content[1],
        opi_ai::message::AssistantContent::ToolCall { tool_call }
            if tool_call.id == "call-9" && tool_call.arguments == "{\"q\":\"x\"}"
    ));
    server.verify().await;
}

#[tokio::test]
async fn thinking_blocks_carry_signature_and_redacted() {
    let server = MockServer::start().await;
    mount_44(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(concat!(
                "data: {\"type\":\"thinking_start\",\"contentIndex\":0}\n\n",
                "data: {\"type\":\"thinking_delta\",\"contentIndex\":0,\"delta\":\"deep\"}\n\n",
                "data: {\"type\":\"thinking_end\",\"contentIndex\":0,\"content\":\"deep thought\",\"contentSignature\":\"sig-h\"}\n\n",
                "data: {\"type\":\"thinking_start\",\"contentIndex\":1}\n\n",
                "data: {\"type\":\"thinking_end\",\"contentIndex\":1,\"content\":\"\",\"contentSignature\":\"enc\",\"redacted\":true}\n\n",
                "data: {\"type\":\"done\",\"reason\":\"stop\",\"usage\":{\"input\":0,\"output\":0,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":0}}\n\n",
            ))
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let message = done_message_44(&events).await;
    assert_eq!(message.content.len(), 2);
    assert!(matches!(
        &message.content[0],
        opi_ai::message::AssistantContent::Thinking { thinking, signature }
            if thinking == "deep thought" && signature.as_deref() == Some("sig-h")
    ));
    assert!(matches!(
        &message.content[1],
        opi_ai::message::AssistantContent::RedactedThinking { data }
            if data == "enc"
    ));
    server.verify().await;
}

#[tokio::test]
async fn done_maps_usage_levels_rewrite_and_stops() {
    for (reason, expected) in [("stop", StopReason::Stop), ("length", StopReason::Length)] {
        let server = MockServer::start().await;
        mount_44(
            &server,
            ResponseTemplate::new(200)
                .set_body_string(format!(
                    concat!(
                        "data: {{\"type\":\"done\",\"reason\":\"{}\",",
                        "\"usage\":{{\"input\":10,\"output\":5,\"cacheRead\":3,\"cacheWrite\":2,",
                        "\"cacheWrite1h\":1,\"reasoning\":4,\"totalTokens\":20,",
                        "\"cost\":{{\"input\":0.1,\"output\":0.2,\"cacheRead\":0.01,\"cacheWrite\":0.02,\"total\":0.33}}}},",
                        "\"responseId\":\"resp-3\",\"providerThinkingLevel\":\"high\",",
                        "\"rewrite\":{{\"policyId\":\"pol-1\",\"policyVersion\":2,\"changed\":true,",
                        "\"tokenCountChange\":-5,\"messageCountChange\":0,\"systemPromptChanged\":false}}}}\n\n",
                    ),
                    reason
                ))
                .insert_header("content-type", "text/event-stream"),
        )
        .await;
        let provider = PiMessagesProvider::new(Some(server.uri()))
            .with_models(vec![model_44(MODEL_44)])
            .with_native_options(PiMessagesOptions::native(None, None, false));
        let events = run_44(&provider, req_44("hi")).await;
        let message = done_message_44(&events).await;

        assert_eq!(message.stop_reason, expected);
        assert_eq!(message.response_id.as_deref(), Some("resp-3"));
        assert_eq!(message.provider_thinking_level.as_deref(), Some("high"));
        assert_eq!(message.usage.input_tokens, 10);
        assert_eq!(message.usage.output_tokens, 5);
        assert_eq!(message.usage.cache_read_tokens, 3);
        assert_eq!(message.usage.cache_write_tokens, 2);
        assert_eq!(message.usage.cache_write_1h_tokens, Some(1));
        assert_eq!(message.usage.reasoning_tokens, Some(4));
        assert_eq!(message.usage.total_tokens, Some(20));
        assert_eq!(message.diagnostics.len(), 1);
        assert_eq!(message.diagnostics[0].kind, "pi_messages_rewrite");
        server.verify().await;
    }
}

#[tokio::test]
async fn error_event_fails_typed_with_message() {
    let server = MockServer::start().await;
    mount_44(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(concat!(
                "data: {\"type\":\"text_start\",\"contentIndex\":0}\n\n",
                "data: {\"type\":\"text_delta\",\"contentIndex\":0,\"delta\":\"partial\"}\n\n",
                "data: {\"type\":\"error\",\"reason\":\"error\",\"usage\":{\"input\":1,\"output\":1,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":2},\"errorMessage\":\"upstream exploded\"}\n\n",
            ))
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let error = first_error_44(&events).await;
    match error {
        ProviderError::StreamError(summary) => {
            let text = summary.to_string();
            assert!(text.contains("upstream exploded"), "got {text}");
        }
        other => panic!("expected StreamError, got {other}"),
    }
    server.verify().await;
}

#[tokio::test]
async fn aborted_event_maps_cancelled() {
    let server = MockServer::start().await;
    mount_44(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(concat!(
                "data: {\"type\":\"error\",\"reason\":\"aborted\",",
                "\"usage\":{\"input\":0,\"output\":0,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":0}}\n\n",
            ))
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let error = first_error_44(&events).await;
    assert!(matches!(error, ProviderError::Cancelled), "got {error}");
    server.verify().await;
}

#[tokio::test]
async fn missing_terminal_event_fails() {
    let server = MockServer::start().await;
    mount_44(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(concat!(
                "data: {\"type\":\"text_start\",\"contentIndex\":0}\n\n",
                "data: {\"type\":\"text_delta\",\"contentIndex\":0,\"delta\":\"orphan\"}\n\n",
            ))
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let error = first_error_44(&events).await;
    match error {
        ProviderError::StreamError(summary) => {
            let text = summary.to_string();
            assert!(text.contains("terminal"), "got {text}");
        }
        other => panic!("expected StreamError, got {other}"),
    }
    server.verify().await;
}

#[tokio::test]
async fn done_marker_crlf_and_trailing_frame_parse() {
    let server = MockServer::start().await;
    mount_44(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(concat!(
                "data: {\"type\":\"start\"}\r\n\r\n",
                "data: [DONE]\r\n\r\n",
                "data: {\"type\":\"done\",\"reason\":\"stop\",\"usage\":{\"input\":2,\"output\":1,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":3}}",
            ))
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let message = done_message_44(&events).await;
    assert_eq!(message.usage.total_tokens, Some(3));
    server.verify().await;
}

#[tokio::test]
async fn invalid_events_fail_typed() {
    for bad in [
        "data: {not json}\n\n",
        "data: {\"type\":\"mystery\"}\n\n",
        "data: 42\n\n",
    ] {
        let server = MockServer::start().await;
        mount_44(
            &server,
            ResponseTemplate::new(200)
                .set_body_string(format!(
                    "{bad}data: {{\"type\":\"done\",\"reason\":\"stop\",\"usage\":{{\"input\":0,\"output\":0,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":0}}}}\n\n"
                ))
                .insert_header("content-type", "text/event-stream"),
        )
        .await;
        let provider = PiMessagesProvider::new(Some(server.uri()))
            .with_models(vec![model_44(MODEL_44)])
            .with_native_options(PiMessagesOptions::native(None, None, false));
        let events = run_44(&provider, req_44("hi")).await;
        let error = first_error_44(&events).await;
        match error {
            ProviderError::StreamError(summary) => {
                let text = summary.to_string();
                assert!(
                    text.contains("Invalid pi-messages streaming event"),
                    "got {text}"
                );
            }
            other => panic!("expected StreamError, got {other}"),
        }
        server.verify().await;
    }
}

#[tokio::test]
async fn delta_without_start_fails_closed() {
    let server = MockServer::start().await;
    mount_44(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(
                "data: {\"type\":\"text_delta\",\"contentIndex\":0,\"delta\":\"ghost\"}\n\n",
            )
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let error = first_error_44(&events).await;
    assert!(
        matches!(error, ProviderError::StreamError(_)),
        "got {error}"
    );
    server.verify().await;
}

// ---------------------------------------------------------------------------
// HTTP failures
// ---------------------------------------------------------------------------

#[tokio::test]
async fn http_errors_format_status_message_code() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(503).set_body_string(
            "{\"error\":{\"message\":\"upstream down\",\"code\":\"rate_limited\"}}",
        ))
        .expect(1)
        .mount(&server)
        .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let error = first_error_44(&events).await;
    match error {
        ProviderError::ServiceFault { detail, .. } => {
            let text = detail.to_string();
            assert!(
                text.contains("503 Service Unavailable: upstream down (rate_limited)"),
                "got {text}"
            );
        }
        other => panic!("expected ServiceFault, got {other}"),
    }
    server.verify().await;
}

#[tokio::test]
async fn http_error_falls_back_to_raw_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(500).set_body_string("plain boom"))
        .expect(1)
        .mount(&server)
        .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let error = first_error_44(&events).await;
    match error {
        ProviderError::ServiceFault { detail, .. } => {
            let text = detail.to_string();
            assert!(
                text.contains("500 Internal Server Error: plain boom"),
                "got {text}"
            );
        }
        other => panic!("expected ServiceFault, got {other}"),
    }
    server.verify().await;
}

#[tokio::test]
async fn rate_limit_keeps_retryable_identity() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(429).set_body_string("{\"error\":{\"message\":\"slow\"}}"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let error = first_error_44(&events).await;
    assert!(
        matches!(error, ProviderError::RateLimited { .. }),
        "got {error}"
    );
    server.verify().await;
}

#[tokio::test]
async fn empty_body_response_fails_typed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let error = first_error_44(&events).await;
    match error {
        ProviderError::StreamError(summary) => {
            let text = summary.to_string();
            assert!(text.contains("no body"), "got {text}");
        }
        other => panic!("expected StreamError, got {other}"),
    }
    server.verify().await;
}

// ---------------------------------------------------------------------------
// callbacks, cancel, auth, models
// ---------------------------------------------------------------------------

#[tokio::test]
async fn callbacks_observe_payload_response_and_raw_events() {
    let server = MockServer::start().await;
    mount_44(&server, sse_ok_44()).await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));

    let order: std::sync::Arc<std::sync::Mutex<Vec<String>>> = Default::default();
    let payload_order = order.clone();
    let response_order = order.clone();
    let raw_order = order.clone();
    let mut request = req_44("hi");
    request.callbacks = RequestCallbacks::new()
        .with_on_payload(std::sync::Arc::new(move |body| {
            payload_order.lock().unwrap().push(format!(
                "payload:{}",
                body.get("model").and_then(|m| m.as_str()).unwrap_or("?")
            ));
            Ok(None)
        }))
        .with_on_response(std::sync::Arc::new(move |status, _headers| {
            response_order
                .lock()
                .unwrap()
                .push(format!("response:{status}"));
            Ok(())
        }))
        .with_on_raw(std::sync::Arc::new(move |event| {
            if let Some(kind) = event.get("type").and_then(|t| t.as_str()) {
                raw_order.lock().unwrap().push(format!("raw:{kind}"));
            }
            Ok(())
        }));

    let events = run_44(&provider, request).await;
    assert!(
        events
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
    );
    let order = order.lock().unwrap().clone();
    assert_eq!(
        order.first().map(String::as_str),
        Some("payload:claude-sonnet-4-5")
    );
    let response_at = order
        .iter()
        .position(|m| m == "response:200")
        .expect("response observer fires");
    let raw_at = order
        .iter()
        .position(|m| m == "raw:start")
        .expect("raw observer sees parsed events");
    assert!(response_at < raw_at, "response precedes raw events");
    assert!(
        order.iter().any(|m| m == "raw:done"),
        "terminal events reach the raw observer"
    );
    server.verify().await;
}

#[tokio::test]
async fn cancel_mid_stream_fails_cancelled() {
    let server = MockServer::start().await;
    mount_44(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(concat!(
                "data: {\"type\":\"start\"}\n\n",
                "data: {\"type\":\"text_start\",\"contentIndex\":0}\n\n",
                "data: {\"type\":\"text_delta\",\"contentIndex\":0,\"delta\":\"chunk\"}\n\n",
            ))
            .insert_header("content-type", "text/event-stream")
            .set_delay(std::time::Duration::from_secs(30)),
    )
    .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let request = req_44("hi");
    let cancel = request.cancel.clone();
    let stream = provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    let handle = tokio::spawn(async move {
        let events: Vec<_> = stream.collect().await;
        events
    });
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    cancel.cancel();
    let events = handle.await.expect("join");
    assert_eq!(first_error_44(&events).await, ProviderError::Cancelled);
}

#[tokio::test]
async fn non_bearer_auth_is_rejected_typed() {
    let server = MockServer::start().await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let auth = opi_ai::auth::ResolvedAuth {
        env: Default::default(),
        scheme: opi_ai::auth::AuthScheme::ApiKey,
        secret: SecretString::from("key"),
        base_url: None,
        account_id: None,
        provenance: Default::default(),
    };
    let request = req_44("hi");
    let mut stream = provider.stream_prepared(request.clone(), auth);
    let error = stream
        .next()
        .await
        .expect("one refusal")
        .expect_err("wrong auth scheme");
    let ProviderError::StreamFailure(failure) = error else {
        panic!("accepted projection must retain its failure carrier")
    };
    let ProviderError::Config(summary) = failure.cause() else {
        panic!("wrong underlying auth refusal: {:?}", failure.cause())
    };
    assert_eq!(
        summary.as_str(),
        "pi-messages accepts only Bearer authentication"
    );
    assert_direct_projection_prefix(failure.message(), &request);
    assert!(stream.next().await.is_none());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn unknown_model_fails_typed() {
    let provider = PiMessagesProvider::new(None)
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let mut request = req_44("hi");
    request.model = "radius:nonexistent".into();
    let events = run_44(&provider, request).await;
    let error = first_error_44(&events).await;
    match error {
        ProviderError::Config(summary) => {
            let text = summary.to_string();
            assert!(text.contains("nonexistent"), "got {text}");
        }
        other => panic!("expected Config, got {other}"),
    }
}

// ---------------------------------------------------------------------------
// simple bypass and lifecycle
// ---------------------------------------------------------------------------

#[tokio::test]
async fn simple_passthrough_is_unclamped_and_omission_preserving() {
    let _guard = env_lock_44().lock().await;
    let server = MockServer::start().await;
    mount_44(&server, sse_ok_44()).await;
    let provider =
        PiMessagesProvider::new(Some(server.uri())).with_models(vec![clamped_model_44()]);
    let mut request = req_44("hi");
    request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::High,
        budget_tokens: None,
    };
    let stream = provider.stream_simple(request, opi_ai::test_support::resolved_bearer_auth());
    let events: Vec<_> = stream.collect().await;
    assert!(
        events
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
    );

    let body = body_44(&server).await;
    assert_eq!(
        body["options"]["reasoning"],
        serde_json::json!("high"),
        "the simple bypass forwards the requested level verbatim; the record's map never clamps"
    );
    assert!(
        body["options"].get("maxTokens").is_none(),
        "omitted maxTokens stays omitted — no default injection"
    );
    server.verify().await;
}

#[test]
fn wrong_wire_record_is_rejected_typed() {
    let provider = PiMessagesProvider::new(None);
    let openai_model = opi_ai::provider::ModelInfo::new(
        MODEL_44,
        MODEL_44,
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(32_000, 8_192),
    );
    let error =
        PiMessagesChatApi::new(provider, openai_model).expect_err("wrong wire must be rejected");
    assert!(matches!(error, ProviderError::Config(_)), "got {error}");
}

#[tokio::test]
async fn native_lifecycle_through_models_complete() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(header("authorization", "Bearer radius-key"))
        .and(body_partial_json(serde_json::json!({
            "model": MODEL_44,
            "options": {"reasoning": "high"}
        })))
        .respond_with(sse_ok_44())
        .expect(1)
        .mount(&server)
        .await;

    let model = thinking_model_44();
    let api = PiMessagesChatApi::new(
        PiMessagesProvider::new(Some(server.uri())).with_models(vec![model.clone()]),
        model.clone(),
    )
    .expect("valid pi-messages record");
    let mut models = opi_ai::models::Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::Bearer,
                SecretString::from("radius-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register");

    let mut request = req_44("go");
    request.model = format!("radius:{MODEL_44}");
    request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::High,
        budget_tokens: None,
    };
    let options = PiMessagesOptions::native(Some(ThinkingLevel::High), None, false);
    let message = models
        .complete(
            opi_ai::models::ChatModelRecord::new("radius", model.clone()),
            request,
            handle.options(options),
        )
        .await
        .expect("complete succeeds");
    assert_eq!(message.stop_reason, StopReason::Stop);
    assert_eq!(message.response_id.as_deref(), Some("resp-1"));
    server.verify().await;
}

fn node5_pi_sse(events: &[serde_json::Value]) -> String {
    events
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect()
}

#[tokio::test]
async fn node5_models_pi_simple_bypasses_budget_and_effort_mapping_with_tool_choice() {
    for typed in [false, true] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(sse_ok_44())
            .expect(2)
            .mount(&server)
            .await;
        let mut model = clamped_model_44();
        model.base_url = Some(server.uri());
        let provider = PiMessagesProvider::new(Some(server.uri())).with_models(vec![model.clone()]);
        let mut models = opi_ai::Models::new();
        let auth = std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
            opi_ai::auth::AuthScheme::Bearer,
            SecretString::from("fixture-key"),
        ));
        if typed {
            models
                .register_chat_api(
                    std::sync::Arc::new(PiMessagesChatApi::new(provider, model.clone()).unwrap()),
                    auth,
                    opi_ai::AuthProvenanceSource::Static,
                )
                .unwrap();
        } else {
            models
                .register_route(
                    Box::new(provider),
                    auth,
                    opi_ai::AuthProvenanceSource::Static,
                    Default::default(),
                )
                .unwrap();
        }
        for (max_tokens, choice) in [
            (None, opi_ai::SimpleToolChoice::Auto),
            (Some(999_999_999), opi_ai::SimpleToolChoice::None),
        ] {
            let mut request = req_44("go");
            request.max_tokens = max_tokens;
            request.thinking = ThinkingConfig {
                enabled: true,
                level: ThinkingLevel::XHigh,
                budget_tokens: None,
            };
            models
                .complete_simple(
                    opi_ai::ChatModelRecord::new("radius", model.clone()),
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
        for (index, choice) in ["auto", "none"].iter().enumerate() {
            let body: serde_json::Value = serde_json::from_slice(&sent[index].body).unwrap();
            assert_eq!(body["options"]["reasoning"], "xhigh");
            assert_eq!(body["options"]["toolChoice"], *choice);
            if index == 0 {
                assert!(body["options"].get("maxTokens").is_none());
            } else {
                assert_eq!(body["options"]["maxTokens"], 999_999_999_u64);
            }
        }
    }
}

fn node5_pi_terminal(kind: &str) -> serde_json::Value {
    serde_json::json!({"type":kind,"reason":if kind=="error"{"error"}else{"toolUse"},
        "responseId":"terminal-id","errorMessage":"fixture failure",
        "usage":{"input":13,"output":7,"cacheRead":0,"cacheWrite":0,"totalTokens":20,
            "cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"total":0}}})
}

async fn node5_pi_complete(
    base: &str,
) -> Result<opi_ai::message::AssistantMessage, opi_ai::stream::CallFailure> {
    let model = model_44(MODEL_44);
    let api = PiMessagesChatApi::new(
        PiMessagesProvider::new(Some(base.into())).with_models(vec![model.clone()]),
        model.clone(),
    )
    .unwrap();
    let mut models = opi_ai::models::Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::Bearer,
                SecretString::from("test-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .unwrap();
    models
        .complete(
            opi_ai::models::ChatModelRecord::new("radius", model),
            req_44("probe"),
            handle.options(PiMessagesOptions::native(None, None, false)),
        )
        .await
}

#[tokio::test]
async fn node5_pi_terminal_error_preserves_final_accounting_and_context_envelope() {
    let server = MockServer::start().await;
    let body = node5_pi_sse(&[
        serde_json::json!({"type":"start"}),
        serde_json::json!({"type":"text_start","contentIndex":0}),
        serde_json::json!({"type":"text_delta","contentIndex":0,"delta":"partial"}),
        node5_pi_terminal("error"),
    ]);
    mount_44(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(body)
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let failure = node5_pi_complete(&server.uri())
        .await
        .expect_err("error terminal");
    assert!(matches!(failure.cause(), ProviderError::StreamError(_)));
    assert_eq!(failure.message().usage.input_tokens, 13);
    assert_eq!(failure.message().usage.output_tokens, 7);
    assert_eq!(
        failure.message().response_id.as_deref(),
        Some("terminal-id")
    );
    assert!(
        matches!(&failure.message().content[0],opi_ai::message::AssistantContent::Text{text,..} if text=="partial")
    );
    let body = body_44(&server).await;
    assert!(body["context"].is_object());
    assert!(body["context"]["messages"].is_array());
    server.verify().await;
}

#[tokio::test]
async fn node5_pi_toolcall_end_overrides_delta_arguments() {
    for delta in [None, Some("{\"q\":\"draft\"}")] {
        let server = MockServer::start().await;
        let mut events = vec![
            serde_json::json!({"type":"start"}),
            serde_json::json!({"type":"toolcall_start","contentIndex":0,"id":"call-1","toolName":"lookup"}),
        ];
        if let Some(delta) = delta {
            events
                .push(serde_json::json!({"type":"toolcall_delta","contentIndex":0,"delta":delta}));
        }
        events.extend([serde_json::json!({"type":"toolcall_end","contentIndex":0,"toolCall":{"type":"toolCall","id":"call-1","name":"lookup","arguments":{"q":"final"},"thoughtSignature":"opaque"}}),node5_pi_terminal("done")]);
        mount_44(
            &server,
            ResponseTemplate::new(200)
                .set_body_string(node5_pi_sse(&events))
                .insert_header("content-type", "text/event-stream"),
        )
        .await;
        let provider =
            PiMessagesProvider::new(Some(server.uri())).with_models(vec![model_44(MODEL_44)]);
        let events = run_44(&provider, req_44("probe")).await;
        let message = done_message_44(&events).await;
        let opi_ai::message::AssistantContent::ToolCall { tool_call } = &message.content[0] else {
            panic!("tool call")
        };
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&tool_call.arguments).unwrap(),
            serde_json::json!({"q":"final"})
        );
        assert_eq!(tool_call.signature.as_deref(), Some("opaque"));
    }
}

#[tokio::test]
async fn node5_pi_producer_capture_reload_preserves_authoritative_tools_and_effort() {
    let server = MockServer::start().await;
    let mut terminal = node5_pi_terminal("done");
    terminal["providerThinkingLevel"] = serde_json::json!("high");
    let body = node5_pi_sse(&[
        serde_json::json!({"type":"start"}),
        serde_json::json!({"type":"thinking_start","contentIndex":0}),
        serde_json::json!({"type":"thinking_end","contentIndex":0,"content":"","contentSignature":"pi-signed-empty"}),
        serde_json::json!({"type":"thinking_start","contentIndex":1}),
        serde_json::json!({"type":"thinking_end","contentIndex":1,"content":"","contentSignature":"pi-redacted","redacted":true}),
        serde_json::json!({"type":"thinking_start","contentIndex":2}),
        serde_json::json!({"type":"thinking_delta","contentIndex":2,"delta":"pi-private"}),
        serde_json::json!({"type":"thinking_end","contentIndex":2,"content":"pi-private","contentSignature":"pi-thinking-signature"}),
        serde_json::json!({"type":"text_start","contentIndex":3}),
        serde_json::json!({"type":"text_delta","contentIndex":3,"delta":"draft"}),
        serde_json::json!({"type":"text_end","contentIndex":3,"content":"pi-answer","contentSignature":"pi-text-signature"}),
        serde_json::json!({"type":"toolcall_start","contentIndex":4,"id":"call-1","toolName":"lookup"}),
        serde_json::json!({"type":"toolcall_delta","contentIndex":4,"delta":"{\"q\":\"draft\"}"}),
        serde_json::json!({"type":"toolcall_end","contentIndex":4,"toolCall":{"type":"toolCall","id":"call-1","name":"lookup","arguments":{"q":"final"},"thoughtSignature":"opaque-signature"}}),
        terminal,
    ]);
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(body)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(3)
        .mount(&server)
        .await;
    let model = thinking_model_44().with_base_url(server.uri());
    let api =
        PiMessagesChatApi::new(PiMessagesProvider::new(Some(server.uri())), model.clone()).unwrap();
    let mut models = opi_ai::Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::AuthScheme::Bearer,
                "fixture-key".into(),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .unwrap();
    let options = || {
        handle.options(PiMessagesOptions::native(
            Some(ThinkingLevel::High),
            None,
            false,
        ))
    };
    let first = models
        .complete(
            opi_ai::ChatModelRecord::new("radius", model.clone()),
            req_44("first"),
            options(),
        )
        .await
        .unwrap();
    assert_eq!(first.provider_thinking_level.as_deref(), Some("high"));
    assert_eq!(first.response_id.as_deref(), Some("terminal-id"));
    let encoded = serde_json::to_vec(&first).unwrap();
    let reload: opi_ai::message::AssistantMessage = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
        serde_json::to_value(&reload).unwrap(),
        serde_json::to_value(&first).unwrap()
    );
    let mut request = req_44("second");
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        tools: vec![],
        messages: vec![
            Message::Assistant(reload.clone()),
            Message::ToolResult(opi_ai::message::ToolResultMessage {
                tool_call_id: "call-1".into(),
                tool_name: "lookup".into(),
                content: vec![opi_ai::message::OutputContent::Text {
                    text: "lookup-result".into(),
                }],
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
        .complete(
            opi_ai::ChatModelRecord::new("radius", model.clone()),
            request.clone(),
            options(),
        )
        .await
        .unwrap();
    let mut changed_model = model;
    changed_model.id = "different-pi-model".into();
    let changed = models
        .complete(
            opi_ai::ChatModelRecord::new("radius", changed_model.clone()),
            request.clone(),
            options(),
        )
        .await
        .unwrap();
    assert_eq!(changed.model, changed_model.id);
    let sent = server.received_requests().await.unwrap();
    for (sent, target_model) in sent[1..].iter().zip([MODEL_44, changed_model.id.as_str()]) {
        assert_eq!(sent.url.path(), "/messages");
        assert_eq!(sent.headers["authorization"], "Bearer fixture-key");
        let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
        assert_eq!(body["model"], target_model);
        let replay = body["context"]["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|message| message["role"] == "assistant")
            .unwrap();
        assert_eq!(replay["provider"], "radius");
        assert_eq!(replay["api"], "pi-messages");
        assert_eq!(replay["model"], MODEL_44);
        assert_eq!(replay["providerThinkingLevel"], "high");
        assert_eq!(
            replay["content"],
            serde_json::json!([
                {"type":"thinking","thinking":"","thinkingSignature":"pi-signed-empty"},
                {"type":"thinking","thinking":"","thinkingSignature":"pi-redacted","redacted":true},
                {"type":"thinking","thinking":"pi-private","thinkingSignature":"pi-thinking-signature"},
                {"type":"text","text":"pi-answer","textSignature":"pi-text-signature"},
                {"type":"toolCall","id":"call-1","name":"lookup","arguments":{"q":"final"},"thoughtSignature":"opaque-signature"}
            ])
        );
        assert_eq!(body["options"]["reasoning"], "high");
    }

    // The gateway carries canonical history. A native inference wire owns
    // the source-identity projection when it consumes that same history.
    let destination = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(concat!(
                    "data: {\"choices\":[{\"delta\":{\"content\":\"target-answer\"},\"finish_reason\":\"stop\"}]}\n\n",
                    "data: [DONE]\n\n",
                )),
        )
        .expect(1)
        .mount(&destination)
        .await;
    let target_model = opi_ai::provider::ModelInfo::new(
        "mistral-replay-target",
        "mistral-replay-target",
        WireApi::MistralConversations,
        ModelCapabilities::new(200_000, 16_384).with_streaming(true),
    )
    .with_base_url(destination.uri());
    let target = models
        .register_chat_api(
            std::sync::Arc::new(
                opi_ai::mistral::MistralConversationsChatApi::new(
                    opi_ai::mistral::MistralConversationsProvider::new(Some(destination.uri())),
                    target_model.clone(),
                )
                .unwrap(),
            ),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::AuthScheme::Bearer,
                "destination-key".into(),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .unwrap();
    let result = models
        .complete(
            opi_ai::ChatModelRecord::new("mistral", target_model),
            request,
            target.options(opi_ai::mistral::MistralOptions::default()),
        )
        .await
        .unwrap();
    assert_eq!(result.provider, "mistral");
    assert_eq!(result.wire, Some(WireApi::MistralConversations));
    let projected = destination.received_requests().await.unwrap();
    assert_eq!(
        projected[0].headers["authorization"],
        "Bearer destination-key"
    );
    let projected: serde_json::Value = serde_json::from_slice(&projected[0].body).unwrap();
    let messages = projected["messages"].as_array().unwrap();
    let assistant = messages
        .iter()
        .find(|message| message["role"] == "assistant")
        .unwrap();
    assert_eq!(
        assistant["content"],
        serde_json::json!([{"type":"text","text":"pi-answer"}])
    );
    let call = &assistant["tool_calls"][0];
    assert_eq!(call["function"]["name"], "lookup");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(call["function"]["arguments"].as_str().unwrap())
            .unwrap(),
        serde_json::json!({"q":"final"})
    );
    let tool_result = messages
        .iter()
        .find(|message| message["role"] == "tool")
        .unwrap();
    assert_eq!(tool_result["tool_call_id"], call["id"]);
    assert_eq!(
        tool_result["content"],
        serde_json::json!([{"type":"text","text":"lookup-result"}])
    );
    for secret in [
        "pi-private",
        "pi-signed-empty",
        "pi-redacted",
        "pi-thinking-signature",
        "pi-text-signature",
        "opaque-signature",
    ] {
        assert!(!projected.to_string().contains(secret));
    }
    assert_eq!(serde_json::to_vec(&reload).unwrap(), encoded);
}

#[tokio::test]
async fn node5_pi_split_utf8_preserves_delta_and_final_text() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut byte = [0; 1];
            socket.read_exact(&mut byte).await.unwrap();
            head.push(byte[0]);
        }
        let content_length = std::str::from_utf8(&head)
            .unwrap()
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .map(|(_, value)| value.trim().parse::<usize>().unwrap())
            .unwrap();
        let mut input = vec![0; content_length];
        socket.read_exact(&mut input).await.unwrap();
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").await.unwrap();
        let body = node5_pi_sse(&[
            serde_json::json!({"type":"start"}),
            serde_json::json!({"type":"text_start","contentIndex":0}),
            serde_json::json!({"type":"text_delta","contentIndex":0,"delta":"中"}),
            serde_json::json!({"type":"text_end","contentIndex":0,"content":"中"}),
            node5_pi_terminal("done"),
        ])
        .into_bytes();
        let mut bounds = vec![0];
        bounds.extend(
            body.iter()
                .enumerate()
                .filter_map(|(i, b)| (*b == 0xe4).then_some(i + 1)),
        );
        bounds.push(body.len());
        for range in bounds.windows(2) {
            let bytes = &body[range[0]..range[1]];
            socket
                .write_all(format!("{:x}\r\n", bytes.len()).as_bytes())
                .await
                .unwrap();
            socket.write_all(bytes).await.unwrap();
            socket.write_all(b"\r\n").await.unwrap();
            socket.flush().await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(40)).await;
        }
        socket.write_all(b"0\r\n\r\n").await.unwrap();
    });
    let provider = PiMessagesProvider::new(Some(format!("http://{address}")))
        .with_models(vec![model_44(MODEL_44)]);
    let events = run_44(&provider, req_44("probe")).await;
    let deltas: String = events
        .iter()
        .filter_map(|event| match event {
            Ok(AssistantStreamEvent::TextDelta { delta, .. }) => Some(delta.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(deltas, "中");
    let message = done_message_44(&events).await;
    assert!(
        matches!(&message.content[0],opi_ai::message::AssistantContent::Text{text,..} if text=="中")
    );
    task.await.unwrap();
}

fn assert_direct_projection_prefix(
    message: &opi_ai::message::AssistantMessage,
    request: &opi_ai::provider::Request,
) {
    use sha2::Digest as _;
    let facts = message
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
}
