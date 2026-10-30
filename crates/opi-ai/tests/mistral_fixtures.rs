//! Mistral provider profile fixture tests (task 2.5).
//!
//! Verifies: model resolution, routing through OpenAI-compatible adapter,
//! request body construction, SSE text/tool-call streaming, error handling,
//! usage tracking, and Mistral-specific model list.

use futures_util::StreamExt;
use opi_ai::message::{InputContent, Message, ToolDef, UserMessage};
use opi_ai::openai_chat::{CompatConfig, OpenAiChatProvider};
use opi_ai::provider::{
    CacheRetention, EventStream, Provider, ProviderError, Request, ThinkingConfig,
};
use opi_ai::registry::ProviderRegistry;
use opi_ai::stream::AssistantStreamEvent;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: create a Mistral-configured provider.
fn mistral_provider() -> OpenAiChatProvider {
    opi_ai::mistral::mistral_provider(None)
}

/// Helper: collect stream events asynchronously.
async fn collect_stream(stream: EventStream) -> Vec<AssistantStreamEvent> {
    stream.filter_map(|r| async move { r.ok() }).collect().await
}

async fn write_chunk(socket: &mut tokio::net::TcpStream, body: &str) -> std::io::Result<()> {
    let chunk = format!("{:X}\r\n{}\r\n", body.len(), body);
    tokio::io::AsyncWriteExt::write_all(socket, chunk.as_bytes()).await
}

async fn spawn_stalled_mistral_server() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled Mistral server");
    let addr = listener.local_addr().expect("stalled server addr");

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept stalled stream");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];

        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read request");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        let request_text = String::from_utf8_lossy(&request);
        assert!(
            request_text.starts_with("POST /v1/chat/completions "),
            "unexpected request line: {request_text}"
        );

        tokio::io::AsyncWriteExt::write_all(
            &mut socket,
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
        )
        .await
        .expect("write response headers");
        write_chunk(&mut socket, stalled_mistral_start_chunk())
            .await
            .expect("write initial SSE chunk");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let _ = write_chunk(&mut socket, stalled_mistral_terminal_chunk()).await;
        let _ = tokio::io::AsyncWriteExt::write_all(&mut socket, b"0\r\n\r\n").await;
    });

    format!("http://{addr}")
}

fn stalled_mistral_start_chunk() -> &'static str {
    "data: {\"id\":\"chatcmpl-slow\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"mistral-small-latest\",\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n"
}

fn stalled_mistral_terminal_chunk() -> &'static str {
    concat!(
        "data: {\"id\":\"chatcmpl-slow\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"mistral-small-latest\",\"choices\":[{\"delta\":{\"content\":\"late\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-slow\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"mistral-small-latest\",\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n",
        "data: [DONE]\n\n",
    )
}

// ---------------------------------------------------------------------------
// Provider identity
// ---------------------------------------------------------------------------

#[test]
fn mistral_provider_id_is_mistral() {
    let provider = mistral_provider();
    assert_eq!(provider.id(), "mistral");
}

// ---------------------------------------------------------------------------
// Model resolution via registry
// ---------------------------------------------------------------------------

#[test]
fn mistral_resolves_model_in_registry() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(Box::new(mistral_provider()))
        .unwrap();
    let (provider, model) = registry.resolve("mistral:mistral-large-latest").unwrap();
    assert_eq!(provider.id(), "mistral");
    assert_eq!(model.id, "mistral-large-latest");
}

#[test]
fn mistral_registry_lists_provider_id() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(Box::new(mistral_provider()))
        .unwrap();
    let ids = registry.provider_ids();
    assert!(ids.contains(&"mistral"));
}

#[test]
fn mistral_unknown_model_returns_error() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(Box::new(mistral_provider()))
        .unwrap();
    let result = registry.resolve("mistral:nonexistent-model");
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Request body — prefix stripping
// ---------------------------------------------------------------------------

#[test]
fn mistral_request_body_strips_provider_prefix() {
    let provider = mistral_provider();
    let request = Request {
        transport: Default::default(),
        model: "mistral:mistral-small-latest".into(),
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
    assert_eq!(body["model"], "mistral-small-latest");
}

// ---------------------------------------------------------------------------
// SSE text streaming through OpenAI adapter
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mistral_text_streaming_produces_start_delta_done() {
    let provider = mistral_provider();
    let sse = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":null}}],\"model\":\"mistral-large-latest\"}\n\n\
               data: {\"choices\":[{\"delta\":{\"content\":\"Hi there\"}}]}\n\n\
               data: {\"choices\":[{\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n\
               data: [DONE]\n\n";
    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

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
async fn mistral_done_event_has_mistral_provider() {
    let provider = mistral_provider();
    let sse = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":null}}]}\n\n\
               data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"}}]}\n\n\
               data: {\"choices\":[{\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":2}}\n\n\
               data: [DONE]\n\n";
    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let done = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::Done { message, .. } => Some(message.provider.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(done, "mistral");
}

// ---------------------------------------------------------------------------
// SSE tool call streaming
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mistral_tool_call_streaming_works() {
    let provider = mistral_provider();
    let sse = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":null}}]}\n\n\
               data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"read_file\",\"arguments\":\"\"}}]}}]}\n\n\
               data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{\\\"path\\\":\\\"foo.rs\\\"}\"}}]}}]}\n\n\
               data: {\"choices\":[{\"finish_reason\":\"tool_calls\"}],\"usage\":{\"prompt_tokens\":20,\"completion_tokens\":10}}\n\n\
               data: [DONE]\n\n";
    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

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
async fn mistral_error_event_routing() {
    let provider = mistral_provider();
    let sse = "data: {\"error\":{\"message\":\"Model not found\"}}\n\n\
               data: [DONE]\n\n";
    let items: Vec<Result<AssistantStreamEvent, ProviderError>> = provider
        .stream_from_sse(sse, CancellationToken::new())
        .collect()
        .await;

    // Current producers stop the dual terminal format: the error frame routes
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
async fn mistral_usage_in_done_event() {
    let provider = mistral_provider();
    let sse = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":null}}]}\n\n\
               data: {\"choices\":[{\"delta\":{\"content\":\"test\"}}]}\n\n\
               data: {\"choices\":[{\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":42,\"completion_tokens\":13}}\n\n\
               data: [DONE]\n\n";
    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

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
// Model list
// ---------------------------------------------------------------------------

#[test]
fn mistral_has_model_list() {
    let provider = mistral_provider();
    let models = provider.models();
    assert!(
        !models.is_empty(),
        "Mistral provider should have at least one model"
    );
    assert!(
        models.iter().any(|m| m.id == "mistral-large-latest"),
        "should have mistral-large-latest model"
    );
    assert!(
        models.iter().any(|m| m.id == "mistral-small-latest"),
        "should have mistral-small-latest model"
    );
    assert!(
        models.iter().any(|m| m.id == "codestral-latest"),
        "should have codestral-latest model"
    );
}

// ---------------------------------------------------------------------------
// Custom base URL
// ---------------------------------------------------------------------------

#[test]
fn mistral_custom_base_url() {
    let provider = opi_ai::mistral::mistral_provider(Some("https://custom.proxy".into()));
    assert_eq!(provider.id(), "mistral");
}

// ---------------------------------------------------------------------------
// Multiple text deltas
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mistral_multiple_text_deltas() {
    let provider = mistral_provider();
    let sse = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":null}}]}\n\n\
               data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n\
               data: {\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\n\n\
               data: {\"choices\":[{\"delta\":{\"content\":\"!\"}}]}\n\n\
               data: {\"choices\":[{\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n\
               data: [DONE]\n\n";
    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let deltas: Vec<&AssistantStreamEvent> = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::TextDelta { .. }))
        .collect();
    assert_eq!(deltas.len(), 3, "should have three TextDelta events");

    let done_text = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::Done { message, .. } => {
                let text: String = message
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        opi_ai::message::AssistantContent::Text { text, .. } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect();
                Some(text)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(done_text, "Hello world!");
}

// ---------------------------------------------------------------------------
// Production request contract through Provider::stream (Phase 12.1)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_sends_text_request_body_and_auth_through_http() {
    let sse = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n\
               data: {\"choices\":[{\"delta\":{\"content\":\"Hi there\"},\"finish_reason\":null}]}\n\n\
               data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n\
               data: [DONE]\n\n";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .and(body_partial_json(serde_json::json!({
            "model": "mistral-small-latest",
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello"}
            ],
            "max_completion_tokens": 1024
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = opi_ai::mistral::mistral_provider(Some(server.uri()));
    let request = Request {
        transport: Default::default(),
        model: "mistral:mistral-small-latest".into(),
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

    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    let mut saw_done = false;
    while let Some(result) = stream.next().await {
        match result.expect("matched text request must stream successfully") {
            AssistantStreamEvent::Done { reason, .. } => {
                assert_eq!(reason, opi_ai::stream::StopReason::Stop);
                saw_done = true;
            }
            AssistantStreamEvent::Error { .. } => {
                panic!("text fixture must complete with Done");
            }
            _ => {}
        }
    }
    assert!(saw_done, "text fixture must produce a terminal Done event");

    // verify() confirms the production request carried the Mistral chat body
    // (prefix-stripped model + system/user messages + max_tokens), the Bearer
    // auth header, and the /v1/chat/completions path.
    server.verify().await;
}

// ---------------------------------------------------------------------------
// Phase 12 task 12.3 — Mistral inherits the shared compat profile path
// ---------------------------------------------------------------------------

#[test]
fn mistral_profile_inherits_shared_compat_flags() {
    // Mistral is a config-driven OpenAI-compatible profile built on the shared
    // OpenAI Chat adapter. With a developer-role + strict-tools compat + the
    // max_completion_tokens field, its request body reflects all flags through
    // the shared serializer (DoD), not a parallel Mistral-specific serializer.
    let provider = OpenAiChatProvider::new_for_profile(
        "https://mistral.example.com".into(),
        "mistral".into(),
        CompatConfig {
            instruction_role: Some(opi_ai::model_info::ChatInstructionRole::Developer),
            max_tokens_field: Some("max_completion_tokens".into()),
            strict_tool_schema: true,
            ..Default::default()
        },
        vec![],
        vec![],
    );
    let request = Request {
        transport: Default::default(),
        model: "mistral:mistral-large-latest".into(),
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
                input_schema: serde_json::json!({"type":"object","properties":{}}),
                constrained_sampling: None,
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
    let body = provider.build_request_body(&request).unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(
        messages[0]["role"], "developer",
        "Mistral inherits developer-role override from the shared compat path"
    );
    assert!(
        body.get("max_completion_tokens").is_some(),
        "Mistral inherits max_completion_tokens field from the shared compat path"
    );
    let tools = body["tools"].as_array().expect("tools present");
    assert!(
        tools[0]["function"]["strict"] == true,
        "Mistral inherits strict-tool-schema from the shared compat path"
    );
}

// ---------------------------------------------------------------------------
// Provider stream cancellation (Phase 12 task 12.7 DoD clause 6)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_cancellation_aborts_before_completion() {
    // The CancellationToken is threaded into the Mistral adapter's HTTP
    // body-stream loop (inherited from the shared OpenAI-compat path; see
    // openai_chat.rs `cancel.cancelled()` select arm). Cancelling while the
    // stream is open must terminate it before the delayed terminal chunk
    // arrives, proving the inherited adapter path observes cancellation.
    let server = spawn_stalled_mistral_server().await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::mistral::mistral_provider(Some(server));
    let request = Request {
        transport: Default::default(),
        model: "mistral:mistral-small-latest".into(),
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

// ---------------------------------------------------------------------------
// Task 20.51: the Mistral profile carries callbacks through the Chat adapter
// ---------------------------------------------------------------------------

use opi_ai::callbacks::RequestCallbacks;

#[tokio::test]
async fn mistral_profile_carries_callbacks_through_chat_adapter() {
    let order: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let raw_recorder = order.clone();
    let response_recorder = order.clone();
    let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wiremock::matchers::body_partial_json(
            serde_json::json!({"mistral_marker": "replaced"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = opi_ai::mistral::mistral_provider(Some(server.uri()));
    let request = Request {
        transport: Default::default(),
        model: "mistral:mistral-small-latest".into(),
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
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: RequestCallbacks::new()
            .with_on_payload(std::sync::Arc::new(|body| {
                let mut replacement = body.clone();
                replacement["mistral_marker"] = serde_json::json!("replaced");
                Ok(Some(replacement))
            }))
            .with_on_response(std::sync::Arc::new(move |status, _headers| {
                response_recorder
                    .lock()
                    .unwrap()
                    .push(format!("response:{status}"));
                Ok(())
            }))
            .with_on_raw(std::sync::Arc::new(move |event| {
                raw_recorder
                    .lock()
                    .unwrap()
                    .push(format!("raw:{}", event["object"].as_str().unwrap_or("?")));
                Ok(())
            })),
    };

    let events = collect_stream(
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()),
    )
    .await;
    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::Done { .. }))
    );
    let order = order.lock().unwrap().clone();
    assert_eq!(order.first().map(String::as_str), Some("response:200"));
    assert!(
        order.iter().any(|m| m.starts_with("raw:")),
        "profile providers share the Chat adapter's callback seams"
    );
    server.verify().await;
}

// ---------------------------------------------------------------------------
// Task 20.43 — native mistral-conversations adapter (DEL-11)
//
// The OpenAI-compat profile tests above stay untouched; this module pins the
// native wire per pi packages/ai/src/api/mistral-conversations.ts: the actual
// POST {base}/v1/chat/completions endpoint, typed MistralOptions, the
// snake_case wire projection, prompt-cache affinity, the headers-only
// timeout, the 9-char tool-call id normalizer and the SSE consumption
// contract (chunked text/thinking, index-keyed tool accumulation, cached
// usage split, stop map incl. retryable in-stream error).
// ---------------------------------------------------------------------------

mod mistral_43 {
    use super::*;
    use opi_ai::WireApi;
    use opi_ai::message::{AssistantContent, MediaType, OutputContent, ToolCall};
    use opi_ai::mistral::{
        MistralConversationsChatApi, MistralConversationsProvider, MistralOptions,
        MistralPromptMode, MistralReasoningEffort, MistralToolChoice,
    };
    use opi_ai::model_info::{ThinkingLevel, ThinkingLevelMap, ThinkingLevelMapping};
    use opi_ai::registry::ModelCapabilities;
    use opi_ai::stream::StopReason;
    use secrecy::SecretString;
    use std::time::{Duration, Instant};

    const SMALL_43: &str = "mistral-small-latest";

    fn model_43(id: &str, thinking: bool, images: bool) -> opi_ai::provider::ModelInfo {
        let mut caps = ModelCapabilities::new(32_000, 8_192).with_streaming(true);
        if thinking {
            caps = caps.with_thinking(true);
        }
        if images {
            caps = caps.with_images(true);
        }
        opi_ai::provider::ModelInfo::new(id, id, WireApi::MistralConversations, caps)
    }

    fn effort_model_43(id: &str, map: ThinkingLevelMap) -> opi_ai::provider::ModelInfo {
        let mut model = model_43(id, true, false);
        model.thinking_level_map = map;
        model
    }

    fn map_43(pairs: &[(ThinkingLevel, &str)]) -> ThinkingLevelMap {
        let mut map = ThinkingLevelMap::disabled();
        for (level, value) in pairs {
            map = map.with_mapping(*level, ThinkingLevelMapping::Mapped((*value).into()));
        }
        map
    }

    fn provider_43(
        server_uri: Option<&str>,
        model: opi_ai::provider::ModelInfo,
        options: MistralOptions,
    ) -> MistralConversationsProvider {
        MistralConversationsProvider::new(server_uri.map(str::to_owned))
            .with_models(vec![model])
            .with_native_options(options)
    }

    fn req_43(user: &str) -> Request {
        let mut request = base_req_43();
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

    fn base_req_43() -> Request {
        Request {
            transport: Default::default(),
            model: format!("mistral:{SMALL_43}"),
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

    fn sse_ok_43() -> ResponseTemplate {
        ResponseTemplate::new(200)
            .set_body_string(concat!(
                "data: {\"id\":\"chatcmpl-1\",\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"\"}}]}\n\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"}}]}\n\n",
                "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,\"total_tokens\":15}}\n\n",
                "data: [DONE]\n\n",
            ))
            .insert_header("content-type", "text/event-stream")
    }

    async fn mount_43(server: &MockServer, response: ResponseTemplate) {
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(response)
            .expect(1)
            .mount(server)
            .await;
    }

    async fn body_43(server: &MockServer) -> serde_json::Value {
        let requests = server.received_requests().await.expect("captured requests");
        serde_json::from_slice(&requests[0].body).expect("request body is JSON")
    }

    async fn run_43(
        provider: &MistralConversationsProvider,
        request: Request,
    ) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
        let stream =
            provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
        stream.collect().await
    }

    async fn done_message_43(
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

    async fn first_error_43(
        events: &[Result<AssistantStreamEvent, ProviderError>],
    ) -> ProviderError {
        events
            .iter()
            .find_map(|item| match item {
                Err(error) => Some(error.clone()),
                _ => None,
            })
            .expect("stream fails typed")
    }

    // -- wire projection ----------------------------------------------------

    #[tokio::test]
    async fn tool_choice_wire_modes() {
        let cases = [
            (Some(MistralToolChoice::Auto), serde_json::json!("auto")),
            (Some(MistralToolChoice::None_), serde_json::json!("none")),
            (Some(MistralToolChoice::Any), serde_json::json!("any")),
            (
                Some(MistralToolChoice::Required),
                serde_json::json!("required"),
            ),
            (
                Some(MistralToolChoice::Named("read_file".into())),
                serde_json::json!({"type": "function", "function": {"name": "read_file"}}),
            ),
            (None, serde_json::Value::Null),
        ];
        for (choice, expected) in cases {
            let server = MockServer::start().await;
            mount_43(&server, sse_ok_43()).await;
            let options = MistralOptions::native(choice, None, None);
            let provider = provider_43(
                Some(&server.uri()),
                model_43(SMALL_43, false, false),
                options,
            );
            let events = run_43(&provider, req_43("go")).await;
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e, Ok(AssistantStreamEvent::Done { .. }))),
                "mode {expected} completes"
            );
            let body = body_43(&server).await;
            match expected {
                serde_json::Value::Null => assert!(
                    body.get("tool_choice").is_none(),
                    "omitted tool choice must stay off the wire: {body}"
                ),
                value => assert_eq!(body["tool_choice"], value),
            }
            server.verify().await;
        }
    }

    #[tokio::test]
    async fn native_reasoning_effort_and_prompt_mode_ride_the_wire() {
        let server = MockServer::start().await;
        mount_43(&server, sse_ok_43()).await;
        let options = MistralOptions::native(
            None,
            Some(MistralReasoningEffort::Max),
            Some(MistralPromptMode::Reasoning),
        );
        let provider = provider_43(
            Some(&server.uri()),
            model_43(SMALL_43, false, false),
            options,
        );
        run_43(&provider, req_43("go")).await;
        let body = body_43(&server).await;
        assert_eq!(body["reasoning_effort"], serde_json::json!("max"));
        assert_eq!(body["prompt_mode"], serde_json::json!("reasoning"));
    }

    #[tokio::test]
    async fn simple_reasoning_uses_effort_map_on_mapped_models() {
        // A reasoning model whose level map carries explicit entries sends
        // `reasoning_effort` from the map; an unmapped level falls back to
        // pi's "high" and the off entry projects when reasoning is disabled.
        // Requested levels outside the map clamp to the nearest supported
        // level (pi clampThinkingLevel): Max clamps down to medium.
        let cases = [
            (ThinkingLevel::Medium, serde_json::json!("medium")),
            (ThinkingLevel::Max, serde_json::json!("medium")),
        ];
        for (level, expected) in cases {
            let server = MockServer::start().await;
            mount_43(&server, sse_ok_43()).await;
            let model = effort_model_43(
                "magistral-medium-latest",
                map_43(&[
                    (ThinkingLevel::Low, "low"),
                    (ThinkingLevel::Medium, "medium"),
                ]),
            );
            let provider =
                MistralConversationsProvider::new(Some(server.uri())).with_models(vec![model]);
            let mut request = req_43("go");
            request.model = "mistral:magistral-medium-latest".into();
            request.thinking = ThinkingConfig {
                enabled: true,
                level,
                budget_tokens: None,
            };
            let stream =
                provider.stream_simple(request, opi_ai::test_support::resolved_bearer_auth());
            let events: Vec<_> = stream.collect().await;
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e, Ok(AssistantStreamEvent::Done { .. }))),
                "level {level:?} completes"
            );
            let body = body_43(&server).await;
            assert_eq!(body["reasoning_effort"], expected, "level {level:?}");
            assert!(
                body.get("prompt_mode").is_none(),
                "mapped models never send prompt_mode: {body}"
            );
        }
    }

    #[tokio::test]
    async fn simple_reasoning_off_uses_effort_map_off_entry() {
        let server = MockServer::start().await;
        mount_43(&server, sse_ok_43()).await;
        let model = effort_model_43(
            "magistral-medium-latest",
            map_43(&[(ThinkingLevel::Low, "low"), (ThinkingLevel::None, "low")]),
        );
        let provider =
            MistralConversationsProvider::new(Some(server.uri())).with_models(vec![model]);
        let mut request = req_43("go");
        request.model = "mistral:magistral-medium-latest".into();
        request.thinking = ThinkingConfig {
            enabled: false,
            level: ThinkingLevel::None,
            budget_tokens: None,
        };
        let stream = provider.stream_simple(request, opi_ai::test_support::resolved_bearer_auth());
        let events: Vec<_> = stream.collect().await;
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Ok(AssistantStreamEvent::Done { .. })))
        );
        let body = body_43(&server).await;
        assert_eq!(body["reasoning_effort"], serde_json::json!("low"));
    }

    #[tokio::test]
    async fn simple_reasoning_without_map_uses_prompt_mode() {
        // A reasoning model without explicit map entries (opi's synthetic
        // identity default) rides pi's prompt_mode=reasoning branch instead.
        let server = MockServer::start().await;
        mount_43(&server, sse_ok_43()).await;
        let model = model_43("magistral-small-latest", true, false);
        let provider =
            MistralConversationsProvider::new(Some(server.uri())).with_models(vec![model]);
        let mut request = req_43("go");
        request.model = "mistral:magistral-small-latest".into();
        request.thinking = ThinkingConfig {
            enabled: true,
            level: ThinkingLevel::Low,
            budget_tokens: None,
        };
        let stream = provider.stream_simple(request, opi_ai::test_support::resolved_bearer_auth());
        let events: Vec<_> = stream.collect().await;
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Ok(AssistantStreamEvent::Done { .. })))
        );
        let body = body_43(&server).await;
        assert_eq!(body["prompt_mode"], serde_json::json!("reasoning"));
        assert!(
            body.get("reasoning_effort").is_none(),
            "unmapped models never send reasoning_effort: {body}"
        );
    }

    #[tokio::test]
    async fn simple_non_reasoning_models_send_neither_reasoning_member() {
        let server = MockServer::start().await;
        mount_43(&server, sse_ok_43()).await;
        let provider = MistralConversationsProvider::new(Some(server.uri()))
            .with_models(vec![model_43(SMALL_43, false, false)]);
        let mut request = req_43("go");
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
                .any(|e| matches!(e, Ok(AssistantStreamEvent::Done { .. })))
        );
        let body = body_43(&server).await;
        assert!(body.get("reasoning_effort").is_none(), "{body}");
        assert!(body.get("prompt_mode").is_none(), "{body}");
    }

    #[tokio::test]
    async fn sampling_projects_temperature_and_max_tokens_only() {
        // pi's Mistral payload builder sets only temperature and maxTokens;
        // stop sequences, top_p and the penalty family stay off this wire.
        let server = MockServer::start().await;
        mount_43(&server, sse_ok_43()).await;
        let provider = provider_43(
            Some(&server.uri()),
            model_43(SMALL_43, false, false),
            MistralOptions::default(),
        );
        let mut request = req_43("go");
        request.temperature = Some(0.7);
        request.max_tokens = Some(512);
        request.stop_sequences = vec!["END".into()];
        run_43(&provider, request).await;
        let body = body_43(&server).await;
        assert_eq!(body["temperature"], serde_json::json!(0.7));
        assert_eq!(body["max_tokens"], serde_json::json!(512));
        for absent in [
            "stop",
            "top_p",
            "presence_penalty",
            "frequency_penalty",
            "random_seed",
            "safe_prompt",
        ] {
            assert!(
                body.get(absent).is_none(),
                "member {absent} must stay off the wire: {body}"
            );
        }
        server.verify().await;
    }

    #[tokio::test]
    async fn tools_project_strict_default_true() {
        let server = MockServer::start().await;
        mount_43(&server, sse_ok_43()).await;
        let provider = provider_43(
            Some(&server.uri()),
            model_43(SMALL_43, false, false),
            MistralOptions::default(),
        );
        let mut request = req_43("go");
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text { text: "go".into() }],
                timestamp_ms: 0,
            })],
            tools: vec![ToolDef {
                name: "read_file".into(),
                description: "Read a file".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {"path": {"type": "string"}},
                    "required": ["path"],
                    "additionalProperties": false
                }),
                constrained_sampling: None,
            }],
        })
        .expect("valid context declarations");
        run_43(&provider, request).await;
        let body = body_43(&server).await;
        let function = &body["tools"][0]["function"];
        assert_eq!(function["name"], serde_json::json!("read_file"));
        assert_eq!(function["strict"], serde_json::json!(true));
        assert_eq!(
            function["parameters"]["properties"]["path"]["type"],
            serde_json::json!("string")
        );
    }

    // -- prompt caching affinity --------------------------------------------

    #[tokio::test]
    async fn prompt_cache_key_and_x_affinity_gate_together() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .and(header("x-affinity", "session-1"))
            .and(body_partial_json(
                serde_json::json!({"prompt_cache_key": "session-1"}),
            ))
            .respond_with(sse_ok_43())
            .expect(1)
            .mount(&server)
            .await;
        let provider = provider_43(
            Some(&server.uri()),
            model_43(SMALL_43, false, false),
            MistralOptions::default(),
        );
        let mut request = req_43("go");
        request.session_id = Some("session-1".into());
        request.cache_retention = CacheRetention::Short;
        run_43(&provider, request).await;
        server.verify().await;
    }

    #[tokio::test]
    async fn explicit_affinity_header_wins_and_disabled_retention_suppresses() {
        // Explicit x-affinity override is kept verbatim (the adapter does not
        // rewrite it); a Disabled retention drops both affinity signals.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .and(header("x-affinity", "pinned"))
            .respond_with(sse_ok_43())
            .expect(1)
            .mount(&server)
            .await;
        let provider = provider_43(
            Some(&server.uri()),
            model_43(SMALL_43, false, false),
            MistralOptions::default(),
        );
        let mut request = req_43("go");
        request.session_id = Some("session-1".into());
        request.cache_retention = CacheRetention::Short;
        request.extra_headers = vec![("x-affinity".into(), "pinned".into())];
        run_43(&provider, request).await;
        let body = body_43(&server).await;
        // pi: the explicit header only suppresses the auto affinity header;
        // prompt_cache_key follows the caching gate, not the header.
        assert_eq!(body["prompt_cache_key"], serde_json::json!("session-1"));
        server.verify().await;

        let server2 = MockServer::start().await;
        mount_43(&server2, sse_ok_43()).await;
        let provider2 = provider_43(
            Some(&server2.uri()),
            model_43(SMALL_43, false, false),
            MistralOptions::default(),
        );
        let mut request2 = req_43("go");
        request2.session_id = Some("session-1".into());
        request2.cache_retention = CacheRetention::Disabled;
        run_43(&provider2, request2).await;
        let body2 = body_43(&server2).await;
        assert!(body2.get("prompt_cache_key").is_none(), "{body2}");
    }

    #[tokio::test]
    async fn sse_headers_and_trailing_slash_base_url() {
        // The API keeps the mistral-conversations identity but posts the
        // chat/completions endpoint regardless of a trailing-slash base URL.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("accept", "text/event-stream"))
            .and(header("content-type", "application/json"))
            .respond_with(sse_ok_43())
            .expect(1)
            .mount(&server)
            .await;
        let provider = MistralConversationsProvider::new(Some(format!("{}/", server.uri())))
            .with_models(vec![model_43(SMALL_43, false, false)]);
        run_43(&provider, req_43("go")).await;
        server.verify().await;
    }

    // -- SSE consumption -----------------------------------------------------

    #[tokio::test]
    async fn chunked_content_streams_thinking_then_text_blocks() {
        let sse = concat!(
            "data: {\"id\":\"chatcmpl-9\",\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":[{\"type\":\"thinking\",\"thinking\":[{\"text\":\"ponder\"}]},{\"type\":\"text\",\"text\":\"answer\"}]}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":4,\"total_tokens\":7}}\n\n",
            "data: [DONE]\n\n",
        );
        let server = MockServer::start().await;
        mount_43(
            &server,
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .await;
        let provider = provider_43(
            Some(&server.uri()),
            model_43(SMALL_43, false, false),
            MistralOptions::default(),
        );
        let events = run_43(&provider, req_43("go")).await;
        let message = done_message_43(&events).await;
        assert_eq!(message.response_id.as_deref(), Some("chatcmpl-9"));
        let mut kinds = Vec::new();
        for content in &message.content {
            kinds.push(match content {
                AssistantContent::Thinking { .. } => "thinking",
                AssistantContent::Text { .. } => "text",
                other => panic!("unexpected content block: {other:?}"),
            });
        }
        assert_eq!(kinds, vec!["thinking", "text"]);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Ok(AssistantStreamEvent::ThinkingStart { .. })))
        );
    }

    #[tokio::test]
    async fn empty_content_deltas_do_not_split_blocks() {
        // GLM-style empty deltas around thinking must not open new blocks
        // (a split replays as multiple thinking chunks, which Mistral
        // rejects on replay).
        let sse = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":[{\"type\":\"thinking\",\"thinking\":[{\"text\":\"a\"}]}]}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":[{\"type\":\"thinking\",\"thinking\":[{\"text\":\"b\"}]}]}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n",
        );
        let server = MockServer::start().await;
        mount_43(
            &server,
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .await;
        let provider = provider_43(
            Some(&server.uri()),
            model_43(SMALL_43, false, false),
            MistralOptions::default(),
        );
        let events = run_43(&provider, req_43("go")).await;
        let message = done_message_43(&events).await;
        let thinking = message
            .content
            .iter()
            .filter(|content| matches!(content, AssistantContent::Thinking { .. }))
            .count();
        assert_eq!(
            thinking, 1,
            "empty deltas must not split the thinking block"
        );
        match &message.content[0] {
            AssistantContent::Thinking { thinking, .. } => assert_eq!(thinking, "ab"),
            other => panic!("expected one thinking block, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn tool_calls_accumulate_by_index_and_derive_ids() {
        // The first chunk's id is the literal "null" placeholder: the id is
        // derived from the tool index; argument fragments join by index.
        let sse = concat!(
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"null\",\"function\":{\"name\":\"read_file\",\"arguments\":\"\"}}]}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{\\\"path\\\":\\\"foo.rs\\\"}\"}}]}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\n",
            "data: [DONE]\n\n",
        );
        let server = MockServer::start().await;
        mount_43(
            &server,
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .await;
        let provider = provider_43(
            Some(&server.uri()),
            model_43(SMALL_43, false, false),
            MistralOptions::default(),
        );
        let events = run_43(&provider, req_43("go")).await;
        let message = done_message_43(&events).await;
        assert_eq!(message.stop_reason, StopReason::ToolUse);
        match &message.content[0] {
            AssistantContent::ToolCall { tool_call } => {
                assert_eq!(tool_call.name, "read_file");
                assert!(
                    tool_call.arguments.contains("\"path\":\"foo.rs\"")
                        || serde_json::from_str::<serde_json::Value>(&tool_call.arguments)
                            .map(|v| v["path"] == serde_json::json!("foo.rs"))
                            .unwrap_or(false),
                    "arguments parse with path: {}",
                    tool_call.arguments
                );
                assert_eq!(
                    tool_call.id.len(),
                    9,
                    "derived id must be 9 chars: {}",
                    tool_call.id
                );
                assert!(
                    tool_call.id.chars().all(|c| c.is_ascii_alphanumeric()),
                    "derived id must be alphanumeric: {}",
                    tool_call.id
                );
            }
            other => panic!("expected tool call, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn usage_splits_cached_prompt_tokens() {
        let sse = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":100,\"completion_tokens\":20,\"prompt_tokens_details\":{\"cached_tokens\":30}}}\n\n",
            "data: [DONE]\n\n",
        );
        let server = MockServer::start().await;
        mount_43(
            &server,
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .await;
        let provider = provider_43(
            Some(&server.uri()),
            model_43(SMALL_43, false, false),
            MistralOptions::default(),
        );
        let events = run_43(&provider, req_43("go")).await;
        let message = done_message_43(&events).await;
        let usage = &message.usage;
        assert_eq!(usage.input_tokens, 70, "input = prompt - cached");
        assert_eq!(usage.output_tokens, 20);
        assert_eq!(usage.cache_read_tokens, 30);
        assert_eq!(
            usage.cache_write_tokens, 0,
            "mistral reports no cache writes"
        );
        assert_eq!(usage.total_tokens, Some(120), "total falls back to the sum");
    }

    #[tokio::test]
    async fn stop_reason_map_covers_mistral_names() {
        let cases = [
            ("stop", StopReason::Stop),
            ("length", StopReason::Length),
            ("model_length", StopReason::Length),
        ];
        for (raw, expected) in cases {
            let sse = format!(
                "data: {{\"choices\":[{{\"delta\":{{\"content\":\"x\"}}}}]}}\n\n\
                 data: {{\"choices\":[{{\"delta\":{{}},\"finish_reason\":\"{raw}\"}}]}}\n\n\
                 data: [DONE]\n\n"
            );
            let server = MockServer::start().await;
            mount_43(
                &server,
                ResponseTemplate::new(200)
                    .set_body_string(sse)
                    .insert_header("content-type", "text/event-stream"),
            )
            .await;
            let provider = provider_43(
                Some(&server.uri()),
                model_43(SMALL_43, false, false),
                MistralOptions::default(),
            );
            let events = run_43(&provider, req_43("go")).await;
            let message = done_message_43(&events).await;
            assert_eq!(message.stop_reason, expected, "raw {raw}");
        }
    }

    #[tokio::test]
    async fn in_stream_error_stop_is_typed_failure() {
        for (raw, needle) in [
            ("error", "Provider stopped with: error (server error)"),
            ("something_else", "Provider stopped with: something_else"),
        ] {
            let sse = format!(
                "data: {{\"choices\":[{{\"delta\":{{\"content\":\"x\"}}}}]}}\n\n\
                 data: {{\"choices\":[{{\"delta\":{{}},\"finish_reason\":\"{raw}\"}}]}}\n\n\
                 data: [DONE]\n\n"
            );
            let server = MockServer::start().await;
            mount_43(
                &server,
                ResponseTemplate::new(200)
                    .set_body_string(sse)
                    .insert_header("content-type", "text/event-stream"),
            )
            .await;
            let provider = provider_43(
                Some(&server.uri()),
                model_43(SMALL_43, false, false),
                MistralOptions::default(),
            );
            let events = run_43(&provider, req_43("go")).await;
            let error = first_error_43(&events).await;
            assert!(
                error.to_string().contains(needle),
                "stop {raw} must fail typed with {needle}, got {error}"
            );
        }
    }

    #[tokio::test]
    async fn missing_finish_reason_fails_typed() {
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"x\"}}]}\n\ndata: [DONE]\n\n";
        let server = MockServer::start().await;
        mount_43(
            &server,
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .await;
        let provider = provider_43(
            Some(&server.uri()),
            model_43(SMALL_43, false, false),
            MistralOptions::default(),
        );
        let events = run_43(&provider, req_43("go")).await;
        let error = first_error_43(&events).await;
        assert!(
            error
                .to_string()
                .contains("Mistral stream ended without a finish reason"),
            "got {error}"
        );
    }

    #[tokio::test]
    async fn invalid_streaming_event_fails_typed() {
        let sse = "data: {\"id\":\"x\"}\n\ndata: [DONE]\n\n";
        let server = MockServer::start().await;
        mount_43(
            &server,
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .await;
        let provider = provider_43(
            Some(&server.uri()),
            model_43(SMALL_43, false, false),
            MistralOptions::default(),
        );
        let events = run_43(&provider, req_43("go")).await;
        let error = first_error_43(&events).await;
        assert!(
            error
                .to_string()
                .contains("Invalid Mistral streaming event"),
            "got {error}"
        );
    }

    // -- HTTP failures --------------------------------------------------------

    #[tokio::test]
    async fn http_errors_format_status_and_truncated_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal boom"))
            .expect(1)
            .mount(&server)
            .await;
        let provider = provider_43(
            Some(&server.uri()),
            model_43(SMALL_43, false, false),
            MistralOptions::default(),
        );
        let events = run_43(&provider, req_43("go")).await;
        let error = first_error_43(&events).await;
        assert!(
            error
                .to_string()
                .contains("Mistral API error (500): internal boom"),
            "got {error}"
        );

        // 429 keeps its retryable identity (the body detail does not
        // displace the rate-limit family).
        let server2 = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(429).set_body_string("slow down"))
            .expect(1)
            .mount(&server2)
            .await;
        let provider2 = provider_43(
            Some(&server2.uri()),
            model_43(SMALL_43, false, false),
            MistralOptions::default(),
        );
        let events2 = run_43(&provider2, req_43("go")).await;
        let error2 = first_error_43(&events2).await;
        assert!(
            matches!(error2, ProviderError::RateLimited { .. }),
            "got {error2}"
        );

        // Long bodies truncate at pi's 4000-char boundary; the crate-wide
        // sanitized-summary cap then bounds the observable text, so neither
        // the full body nor its tail survives into diagnostics.
        let long = "x".repeat(5000);
        let server3 = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(503).set_body_string(long))
            .expect(1)
            .mount(&server3)
            .await;
        let provider3 = provider_43(
            Some(&server3.uri()),
            model_43(SMALL_43, false, false),
            MistralOptions::default(),
        );
        let events3 = run_43(&provider3, req_43("go")).await;
        let error3 = first_error_43(&events3).await;
        let text = error3.to_string();
        assert!(text.contains("Mistral API error (503)"), "got {text}");
        let xs = text.matches('x').count();
        assert!(
            xs < 5000,
            "the raw 5000-char body must not ride whole into diagnostics ({xs} x chars)"
        );
        assert!(
            text.chars().count() < 5000,
            "the sanitized summary caps the formatted error text"
        );
    }

    // -- headers-only timeout boundary ----------------------------------------

    async fn spawn_raw_43(delay_headers: Duration, delay_body: Duration) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut request = Vec::new();
            let mut buf = [0u8; 2048];
            loop {
                let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                    .await
                    .unwrap();
                if read == 0 {
                    return;
                }
                request.extend_from_slice(&buf[..read]);
                if request.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            // Drain the request body before responding.
            let header_end = request.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
            let declared = String::from_utf8_lossy(&request[..header_end])
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|v| v.trim().parse::<usize>().ok())
                .unwrap_or(0);
            while request.len() < header_end + 4 + declared {
                let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                    .await
                    .unwrap();
                if read == 0 {
                    return;
                }
                request.extend_from_slice(&buf[..read]);
            }

            tokio::time::sleep(delay_headers).await;
            tokio::io::AsyncWriteExt::write_all(
                &mut socket,
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
            write_chunk(
                &mut socket,
                "data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\n\n",
            )
            .await
            .unwrap();
            tokio::time::sleep(delay_body).await;
            write_chunk(
                &mut socket,
                concat!(
                    "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
                    "data: [DONE]\n\n",
                ),
            )
            .await
            .unwrap();
            let _ = tokio::io::AsyncWriteExt::write_all(&mut socket, b"0\r\n\r\n").await;
        });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn header_wait_deadline_times_out_before_headers() {
        let uri = spawn_raw_43(Duration::from_millis(900), Duration::from_millis(50)).await;
        let provider = MistralConversationsProvider::new(Some(uri))
            .with_models(vec![model_43(SMALL_43, false, false)]);
        let mut request = req_43("go");
        request.timeout = Some(Duration::from_millis(250));
        let started = Instant::now();
        let events = run_43(&provider, request).await;
        let error = first_error_43(&events).await;
        assert!(matches!(error, ProviderError::Timeout), "got {error}");
        assert!(
            started.elapsed() < Duration::from_millis(800),
            "header deadline must fire fast"
        );
    }

    #[tokio::test]
    async fn stalled_body_after_headers_is_not_under_the_header_deadline() {
        // The default deadline covers only the wait for response headers: a
        // body stall past the header budget must still complete.
        let uri = spawn_raw_43(Duration::from_millis(50), Duration::from_millis(900)).await;
        let provider = MistralConversationsProvider::new(Some(uri))
            .with_models(vec![model_43(SMALL_43, false, false)]);
        let mut request = req_43("go");
        request.timeout = Some(Duration::from_millis(250));
        let events = run_43(&provider, request).await;
        let message = done_message_43(&events).await;
        assert_eq!(message.stop_reason, StopReason::Stop);
    }

    // -- message conversion ----------------------------------------------------

    #[tokio::test]
    async fn message_conversion_projects_mistral_shapes() {
        let server = MockServer::start().await;
        mount_43(&server, sse_ok_43()).await;
        let provider = provider_43(
            Some(&server.uri()),
            model_43(SMALL_43, false, true),
            MistralOptions::default(),
        );
        let mut assistant = opi_ai::message::AssistantMessage {
            content: vec![
                AssistantContent::Thinking {
                    thinking: "step".into(),
                    signature: None,
                },
                AssistantContent::Text {
                    text: "  ".into(),
                    signature: None,
                },
                AssistantContent::ToolCall {
                    tool_call: ToolCall {
                        id: "call_abcdefgh1234567".into(),
                        name: "read_file".into(),
                        arguments: serde_json::json!({"path": "x.rs"}).to_string(),
                        arguments_error: None,
                        signature: None,
                    },
                },
            ],
            api: opi_ai::ApiKind::OpenAi,
            provider: "mistral".into(),
            model: SMALL_43.into(),
            response_model: None,
            wire: Some(WireApi::MistralConversations),
            response_id: None,
            usage: Default::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp_ms: 0,
            diagnostics: vec![],
            duration_ms: None,
            provider_thinking_level: None,
            deferred: None,
        };
        assistant.content[1] = AssistantContent::Text {
            text: "  ".into(),
            signature: None,
        };
        let mut request = base_req_43();
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("Be brief.".into()),
            messages: vec![
                Message::User(UserMessage {
                    content: vec![
                        InputContent::Text {
                            text: "look".into(),
                        },
                        InputContent::Image {
                            source: opi_ai::message::ImageSource::Bytes {
                                data: vec![0xff, 0xd8, 0xff],
                            },
                            media_type: MediaType::Jpeg,
                        },
                    ],
                    timestamp_ms: 0,
                }),
                Message::Assistant(assistant),
                Message::ToolResult(opi_ai::message::ToolResultMessage {
                    not_executed: None,
                    tool_call_id: "call_abcdefgh1234567".into(),
                    tool_name: "read_file".into(),
                    content: vec![
                        OutputContent::Text {
                            text: "  ok  ".into(),
                        },
                        OutputContent::Image {
                            source: opi_ai::message::ImageSource::Bytes {
                                data: vec![0xff, 0xd8, 0xff],
                            },
                            media_type: MediaType::Jpeg,
                        },
                    ],
                    details: None,
                    is_error: false,
                    truncated: false,
                    timestamp_ms: 0,
                }),
                Message::ToolResult(opi_ai::message::ToolResultMessage {
                    not_executed: None,
                    tool_call_id: "call_abcdefgh1234567".into(),
                    tool_name: "read_file".into(),
                    content: vec![],
                    details: None,
                    is_error: true,
                    truncated: false,
                    timestamp_ms: 0,
                }),
            ],
            tools: vec![],
        })
        .expect("valid context declarations");
        run_43(&provider, request).await;
        let body = body_43(&server).await;
        let messages = body["messages"].as_array().expect("messages array");
        assert_eq!(
            messages[0]["role"],
            serde_json::json!("system"),
            "the normalized leading system prompt stays first"
        );

        let user = &messages[1];
        assert_eq!(user["role"], serde_json::json!("user"));
        assert_eq!(user["content"][0]["type"], serde_json::json!("text"));
        assert_eq!(
            user["content"][1]["image_url"],
            serde_json::json!("data:image/jpeg;base64,/9j/")
        );

        let assistant_msg = &messages[2];
        assert_eq!(assistant_msg["role"], serde_json::json!("assistant"));
        assert_eq!(assistant_msg["prefix"], serde_json::json!(false));
        // Blank text drops; thinking replays as a chunked thinking entry.
        assert_eq!(
            assistant_msg["content"][0]["type"],
            serde_json::json!("thinking")
        );
        assert_eq!(
            assistant_msg["content"][0]["thinking"][0]["text"],
            serde_json::json!("step")
        );
        // Long tool-call ids normalize to 9 alnum chars, mirrored into the
        // tool result's tool_call_id so the pairing stays consistent.
        let wire_id = assistant_msg["tool_calls"][0]["id"]
            .as_str()
            .expect("tool call id");
        assert_eq!(wire_id.len(), 9, "id normalizes to 9 chars: {wire_id}");
        assert!(wire_id.chars().all(|c| c.is_ascii_alphanumeric()));
        assert_eq!(
            assistant_msg["tool_calls"][0]["function"]["arguments"],
            serde_json::json!("{\"path\":\"x.rs\"}")
        );

        let tool_ok = &messages[3];
        assert_eq!(tool_ok["role"], serde_json::json!("tool"));
        assert_eq!(tool_ok["tool_call_id"], serde_json::json!(wire_id));
        assert_eq!(tool_ok["name"], serde_json::json!("read_file"));
        assert_eq!(tool_ok["content"][0]["text"], serde_json::json!("ok"));
        assert_eq!(
            tool_ok["content"][1]["type"],
            serde_json::json!("image_url")
        );

        let tool_err = &messages[4];
        assert_eq!(
            tool_err["content"][0]["text"],
            serde_json::json!("[tool error] (no tool output)")
        );
    }

    #[tokio::test]
    async fn non_image_models_get_placeholder_projections() {
        let server = MockServer::start().await;
        mount_43(&server, sse_ok_43()).await;
        let provider = provider_43(
            Some(&server.uri()),
            model_43(SMALL_43, false, false),
            MistralOptions::default(),
        );
        let mut request = base_req_43();
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Image {
                    source: opi_ai::message::ImageSource::Bytes {
                        data: vec![0xff, 0xd8, 0xff],
                    },
                    media_type: MediaType::Jpeg,
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations");
        run_43(&provider, request).await;
        let body = body_43(&server).await;
        assert_eq!(
            body["messages"][0]["content"],
            serde_json::json!("(image omitted: model does not support images)")
        );
    }

    #[tokio::test]
    async fn mid_conversation_system_messages_collapse() {
        let server = MockServer::start().await;
        mount_43(&server, sse_ok_43()).await;
        let provider = provider_43(
            Some(&server.uri()),
            model_43(SMALL_43, false, false),
            MistralOptions::default(),
        );
        let mut request = base_req_43();
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("root system".into()),
            messages: vec![
                Message::User(UserMessage {
                    content: vec![InputContent::Text { text: "one".into() }],
                    timestamp_ms: 0,
                }),
                Message::System(opi_ai::transcript::SystemMessage {
                    content: "mid system".into(),
                    sections: vec![],
                    tools_added: vec![],
                    tools_removed: vec![],
                    timestamp_ms: 0,
                }),
                Message::User(UserMessage {
                    content: vec![InputContent::Text { text: "two".into() }],
                    timestamp_ms: 0,
                }),
            ],
            tools: vec![],
        })
        .expect("valid context declarations");
        run_43(&provider, request).await;
        let body = body_43(&server).await;
        let messages = body["messages"].as_array().expect("messages array");
        let roles: Vec<_> = messages
            .iter()
            .map(|message| message["role"].as_str().unwrap_or_default())
            .collect();
        // Mistral takes no mid-conversation system role: collapse replays
        // the merged current system state as the leading message and drops
        // the in-place update (pi resolveTranscript + opi collapse policy).
        assert_eq!(roles, vec!["system", "user", "user"]);
        assert_eq!(
            messages[0]["content"],
            serde_json::json!("root system\n\nmid system")
        );
        assert_eq!(messages[1]["content"], serde_json::json!("one"));
    }

    // -- lifecycle -------------------------------------------------------------

    #[test]
    fn wrong_wire_record_is_rejected_typed() {
        let provider = MistralConversationsProvider::new(None);
        let openai_model = opi_ai::provider::ModelInfo::new(
            SMALL_43,
            SMALL_43,
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(32_000, 8_192),
        );
        let error = MistralConversationsChatApi::new(provider, openai_model)
            .expect_err("wrong wire must be rejected");
        assert!(matches!(error, ProviderError::Config(_)), "got {error}");
    }

    #[tokio::test]
    async fn native_lifecycle_through_models_complete() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer mistral-key"))
            .and(body_partial_json(serde_json::json!({
                "model": "magistral-medium-latest",
                "reasoning_effort": "medium"
            })))
            .respond_with(sse_ok_43())
            .expect(1)
            .mount(&server)
            .await;

        let model = effort_model_43(
            "magistral-medium-latest",
            map_43(&[
                (ThinkingLevel::Low, "low"),
                (ThinkingLevel::Medium, "medium"),
            ]),
        );
        let api = MistralConversationsChatApi::new(
            MistralConversationsProvider::new(Some(server.uri())).with_models(vec![model.clone()]),
            model,
        )
        .expect("valid mistral record");
        let mut models = opi_ai::models::Models::new();
        let handle = models
            .register_chat_api(
                std::sync::Arc::new(api),
                std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::Bearer,
                    SecretString::from("mistral-key"),
                )),
                opi_ai::AuthProvenanceSource::Static,
            )
            .expect("register");

        let mut request = req_43("go");
        request.model = "mistral:magistral-medium-latest".into();
        request.thinking = ThinkingConfig {
            enabled: true,
            level: ThinkingLevel::Medium,
            budget_tokens: None,
        };
        let options = MistralOptions::native(None, Some(MistralReasoningEffort::Medium), None);
        let message = models
            .complete(
                opi_ai::models::ChatModelRecord::new(
                    "mistral",
                    effort_model_43(
                        "magistral-medium-latest",
                        map_43(&[
                            (ThinkingLevel::Low, "low"),
                            (ThinkingLevel::Medium, "medium"),
                        ]),
                    ),
                ),
                request,
                handle.options(options),
            )
            .await
            .expect("native lifecycle completes");
        assert_eq!(message.stop_reason, StopReason::Stop);
        server.verify().await;
    }

    #[tokio::test]
    async fn node5_models_mistral_simple_tool_choice_reaches_wire() {
        for typed in [false, true] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/v1/chat/completions"))
                .respond_with(sse_ok_43())
                .expect(2)
                .mount(&server)
                .await;
            let model = model_43(SMALL_43, false, false).with_base_url(server.uri());
            let provider = MistralConversationsProvider::new(Some(server.uri()))
                .with_models(vec![model.clone()]);
            let auth = std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::AuthScheme::Bearer,
                "fixture-key".into(),
            ));
            let mut models = opi_ai::Models::new();
            if typed {
                models
                    .register_chat_api(
                        std::sync::Arc::new(
                            MistralConversationsChatApi::new(provider, model.clone()).unwrap(),
                        ),
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
            for choice in [
                opi_ai::SimpleToolChoice::Auto,
                opi_ai::SimpleToolChoice::None,
            ] {
                models
                    .complete_simple(
                        opi_ai::ChatModelRecord::new("mistral", model.clone()),
                        req_43("go"),
                        opi_ai::SimpleOptions {
                            tool_choice: Some(choice),
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();
            }
            let sent = server.received_requests().await.unwrap();
            for (request, expected) in sent.iter().zip(["auto", "none"]) {
                let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
                assert_eq!(body["tool_choice"], expected);
            }
        }
    }

    #[tokio::test]
    async fn node5_mistral_producer_capture_reload_preserves_content_and_native_effort() {
        let server = MockServer::start().await;
        let sse = concat!(
            "data: {\"id\":\"capture-1\",\"choices\":[{\"delta\":{\"content\":[{\"type\":\"thinking\",\"thinking\":[{\"text\":\"consider\"}]},{\"type\":\"text\",\"text\":\"answer\"}]}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":4,\"total_tokens\":7}}\n\n",
            "data: [DONE]\n\n",
        );
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(sse)
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(2)
            .mount(&server)
            .await;
        let model = model_43(SMALL_43, true, false).with_base_url(server.uri());
        let api = MistralConversationsChatApi::new(
            MistralConversationsProvider::new(Some(server.uri())),
            model.clone(),
        )
        .unwrap();
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
            handle.options(MistralOptions::native(
                None,
                Some(MistralReasoningEffort::Medium),
                None,
            ))
        };
        let first = models
            .complete(
                opi_ai::ChatModelRecord::new("mistral", model.clone()),
                req_43("first"),
                options(),
            )
            .await
            .unwrap();
        assert_eq!(first.provider, "mistral");
        assert_eq!(first.wire, Some(WireApi::MistralConversations));
        assert_eq!(first.response_id.as_deref(), Some("capture-1"));
        // The pinned Mistral response protocol does not supply an effort stamp.
        assert!(first.provider_thinking_level.is_none());
        let reload: opi_ai::message::AssistantMessage =
            serde_json::from_slice(&serde_json::to_vec(&first).unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(&reload).unwrap(),
            serde_json::to_value(&first).unwrap()
        );
        let mut request = req_43("second");
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            tools: vec![],
            messages: vec![
                Message::Assistant(reload),
                Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "continue".into(),
                    }],
                    timestamp_ms: 0,
                }),
            ],
        })
        .unwrap();
        models
            .complete(
                opi_ai::ChatModelRecord::new("mistral", model),
                request,
                options(),
            )
            .await
            .unwrap();
        let sent = server.received_requests().await.unwrap();
        for request in &sent {
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            assert_eq!(body["reasoning_effort"], "medium");
        }
        let replay: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
        let assistant = replay["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|message| message["role"] == "assistant")
            .unwrap();
        assert_eq!(
            assistant["content"],
            serde_json::json!([
                {"type":"thinking","thinking":[{"type":"text","text":"consider"}]},
                {"type":"text","text":"answer"},
            ])
        );
    }

    #[tokio::test]
    async fn producer_runs_under_the_frozen_telemetry_context() {
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
        let parent = dispatcher::with_default(&source, || tracing::info_span!("source_parent_43"));
        let context = opi_ai::telemetry::TelemetryContext::from_span(parent);

        let server = MockServer::start().await;
        mount_43(&server, sse_ok_43()).await;
        let provider = provider_43(
            Some(&server.uri()),
            model_43(SMALL_43, false, false),
            MistralOptions::default(),
        );

        let seen = std::sync::Arc::new(AtomicUsize::new(0));
        let counter = seen.clone();
        let mut request = req_43("go");
        request.telemetry_context = Some(context);
        request.callbacks =
            opi_ai::callbacks::RequestCallbacks::new().with_async_on_payload(move |_| {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    tracing::info_span!("payload_probe_43").in_scope(|| ());
                    Ok(None)
                }
            });

        let events = run_43(&provider, request).await;
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Ok(AssistantStreamEvent::Done { .. })))
        );
        assert_eq!(seen.load(Ordering::SeqCst), 1);
        let state = recorder.lock().unwrap();
        assert!(
            state
                .spans
                .iter()
                .any(|(name, _)| *name == "payload_probe_43"),
            "mistral producer telemetry flows to the frozen carrier: {:?}",
            state.spans
        );
    }

    async fn node5_mistral_complete(
        base: &str,
        request: Request,
    ) -> Result<opi_ai::message::AssistantMessage, opi_ai::stream::CallFailure> {
        let model = model_43(SMALL_43, false, false);
        let api = MistralConversationsChatApi::new(
            MistralConversationsProvider::new(Some(base.into())).with_models(vec![model.clone()]),
            model.clone(),
        )
        .unwrap();
        let mut models = opi_ai::models::Models::new();
        let handle = models
            .register_chat_api(
                std::sync::Arc::new(api),
                std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::Bearer,
                    SecretString::from("fixture"),
                )),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        models
            .complete(
                opi_ai::models::ChatModelRecord::new("mistral", model),
                request,
                handle.options(MistralOptions::native(None, None, None)),
            )
            .await
    }

    #[tokio::test]
    async fn node5_mistral_parser_failure_keeps_live_partial_and_usage() {
        for delta in [
            serde_json::json!({"content":"visible"}),
            serde_json::json!({"content":[{"type":"thinking","thinking":[{"text":"reason"}]}]}),
            serde_json::json!({"tool_calls":[{"index":0,"id":"abcdefghi","function":{"name":"lookup","arguments":"{\"q\":\"live\"}"}}]}),
            serde_json::json!({}),
        ] {
            let server = MockServer::start().await;
            let raw = serde_json::json!({"id":"partial-id","choices":[{"delta":delta}],"usage":{"prompt_tokens":11,"completion_tokens":3}});
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string(format!("data: {raw}\n\ndata: {{broken\n\n"))
                        .insert_header("content-type", "text/event-stream"),
                )
                .expect(1)
                .mount(&server)
                .await;
            let failure = node5_mistral_complete(&server.uri(), req_43("probe"))
                .await
                .expect_err("malformed stream");
            assert_eq!(failure.message().usage.input_tokens, 11);
            assert_eq!(failure.message().usage.output_tokens, 3);
            assert_eq!(failure.message().response_id.as_deref(), Some("partial-id"));
            if let Some(content) = delta.get("content") {
                if content.is_string() {
                    assert!(
                        matches!(&failure.message().content[0],AssistantContent::Text{text,..} if text=="visible")
                    );
                } else {
                    assert!(
                        matches!(&failure.message().content[0],AssistantContent::Thinking{thinking,..} if thinking=="reason")
                    );
                }
            }
            if delta.get("tool_calls").is_some() {
                assert!(
                    matches!(&failure.message().content[0],AssistantContent::ToolCall{tool_call} if tool_call.arguments.contains("live"))
                );
            }
            server.verify().await;
        }
    }

    #[tokio::test]
    async fn node5_mistral_direct_delta_contains_current_text() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(sse_ok_43())
            .mount(&server)
            .await;
        let provider = MistralConversationsProvider::new(Some(server.uri()))
            .with_models(vec![model_43(SMALL_43, false, false)]);
        let events: Vec<_> = provider
            .stream_prepared(
                req_43("probe"),
                opi_ai::test_support::resolved_bearer_auth(),
            )
            .collect()
            .await;
        assert!(events.iter().any(|event|matches!(event,Ok(AssistantStreamEvent::TextDelta{delta,partial,..}) if delta=="Hi"&&matches!(&partial.content[0],AssistantContent::Text{text,..} if text=="Hi"))));
    }

    #[tokio::test]
    async fn node5_mistral_raw_and_header_callbacks_reach_real_transport() {
        for callback_failure in [false, true] {
            let server = MockServer::start().await;
            let first = serde_json::json!({"wire_only":"first","choices":[{"delta":{"content":"visible"}}]});
            let last = serde_json::json!({"wire_only":"last","choices":[{"delta":{},"finish_reason":"stop"}]});
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string(format!(
                            "data: {first}\n\ndata: {last}\n\ndata: [DONE]\n\n"
                        ))
                        .insert_header("content-type", "text/event-stream"),
                )
                .expect(1)
                .mount(&server)
                .await;
            let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let observed = seen.clone();
            let mut request = req_43("probe");
            request.callbacks = opi_ai::callbacks::RequestCallbacks::new()
                .with_async_on_raw(move |value| {
                    let observed = observed.clone();
                    async move {
                        observed.lock().unwrap().push(value.clone());
                        if callback_failure && value["wire_only"] == "last" {
                            Err(opi_ai::callbacks::CallbackError::new("fixture raw failure"))
                        } else {
                            Ok(())
                        }
                    }
                })
                .with_async_header_transform(|mut headers| async move {
                    headers.push(("x-fixture-transform".into(), "applied".into()));
                    Ok(headers)
                });
            let result = node5_mistral_complete(&server.uri(), request).await;
            assert_eq!(seen.lock().unwrap().as_slice(), &[first, last]);
            let sent = server.received_requests().await.unwrap();
            assert_eq!(
                sent[0].headers.get("x-fixture-transform").unwrap(),
                "applied"
            );
            if callback_failure {
                let failure = result.expect_err("callback failure");
                assert!(
                    matches!(&failure.message().content[0],AssistantContent::Text{text,..} if text=="visible")
                );
            } else {
                assert!(result.is_ok());
            }
            server.verify().await;
        }
    }

    #[tokio::test]
    async fn node5_mistral_cache_fields_follow_source_order() {
        for (extra, expected) in [
            (serde_json::json!({"num_cached_tokens":30}), 30),
            (
                serde_json::json!({"numCachedTokens":40,"num_cached_tokens":30}),
                40,
            ),
            (
                serde_json::json!({"promptTokensDetails":{"cachedTokens":0},"num_cached_tokens":30}),
                0,
            ),
            (
                serde_json::json!({"prompt_tokens_details":{"cached_tokens":20},"num_cached_tokens":30}),
                20,
            ),
        ] {
            let server = MockServer::start().await;
            let mut usage = extra;
            usage["prompt_tokens"] = 100.into();
            usage["completion_tokens"] = 20.into();
            let raw = serde_json::json!({"choices":[{"delta":{"content":"ok"},"finish_reason":"stop"}],"usage":usage});
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string(format!("data: {raw}\n\ndata: [DONE]\n\n"))
                        .insert_header("content-type", "text/event-stream"),
                )
                .mount(&server)
                .await;
            let message = node5_mistral_complete(&server.uri(), req_43("probe"))
                .await
                .unwrap();
            assert_eq!(message.usage.cache_read_tokens, expected);
            assert_eq!(message.usage.input_tokens, 100 - expected);
        }
    }
}
