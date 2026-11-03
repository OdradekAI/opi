//! OpenAI Responses API provider fixture tests (task 2.3).
//!
//! Verifies: SSE event mapping, text streaming, tool call streaming,
//! usage tracking, error handling, model resolution, and request body
//! construction for the OpenAI Responses API (`/v1/responses`).

use futures_util::StreamExt;
use opi_ai::message::{InputContent, Message, ToolDef, UserMessage};
use opi_ai::openai_responses::{OpenAiResponsesProvider, ResponsesConfig};
use opi_ai::provider::{
    CacheRetention, EventStream, Provider, ProviderError, Request, ThinkingConfig,
};
use opi_ai::registry::ProviderRegistry;
use opi_ai::stream::AssistantStreamEvent;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: create an OpenAI Responses provider.
fn responses_provider() -> OpenAiResponsesProvider {
    OpenAiResponsesProvider::new(None)
}

/// Helper: collect stream events asynchronously.
async fn collect_stream(stream: EventStream) -> Vec<AssistantStreamEvent> {
    stream.filter_map(|r| async move { r.ok() }).collect().await
}

#[tokio::test]
async fn raw_terminal_reason_survives_response_outcome_and_storage() {
    use serde_json::json;
    for (event, status, details, expected) in [
        (
            "response.completed",
            Some("completed"),
            None,
            Some("completed"),
        ),
        (
            "response.incomplete",
            Some("incomplete"),
            Some(json!({"reason":"max_output_tokens"})),
            Some("incomplete.max_output_tokens"),
        ),
        ("response.failed", Some("failed"), None, Some("failed")),
        ("response.completed", None, None, None),
    ] {
        let server = MockServer::start().await;
        let mut terminal = json!({"type":event,"response":{"output":[]}});
        if let Some(status) = status {
            terminal["response"]["status"] = json!(status);
        }
        if let Some(details) = details {
            terminal["response"]["incomplete_details"] = details;
        }
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(format!("data: {terminal}\n\n")),
            )
            .expect(1)
            .mount(&server)
            .await;
        let provider = OpenAiResponsesProvider::new(Some(server.uri()));
        let result = opi_ai::stream::ChatStream::new(provider.stream_prepared(
            responses_text_request(),
            opi_ai::test_support::resolved_bearer_auth(),
        ))
        .result()
        .await;
        let message = if event == "response.failed" {
            result
                .expect_err("failed response remains a typed failure")
                .message()
                .clone()
        } else {
            result.expect("successful terminal fixture must reach the adapter")
        };
        assert_eq!(message.raw_stop_reason.as_deref(), expected, "{event}");
        let restored: opi_ai::message::AssistantMessage =
            serde_json::from_slice(&serde_json::to_vec(&message).unwrap()).unwrap();
        assert_eq!(restored, message);
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn successful_headers_seal_acquisition_before_a_body_timeout() {
    use std::io::{Read, Write};
    use std::sync::Arc;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let peer = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            socket.read_exact(&mut byte).unwrap();
            head.push(byte[0]);
        }
        socket
            .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\n")
            .unwrap();
        socket.flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(250));
    });
    let provider = OpenAiResponsesProvider::new(Some(format!("http://127.0.0.1:{port}")));
    let model_id = provider.models()[0].id.clone();
    let mut models = opi_ai::Models::new();
    models
        .register_route(
            Box::new(provider),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::Bearer,
                secrecy::SecretString::from("fake-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let mut request = responses_text_request();
    request.model = format!("openai-responses:{model_id}");
    request.timeout = Some(std::time::Duration::from_millis(50));
    let call = models
        .prepare_call_with_policy(
            &request.model.clone(),
            request,
            opi_ai::retry::AcquisitionPolicy::with_max_retries(1),
        )
        .await
        .unwrap();
    let error = opi_ai::stream::ChatStream::new(call.start_attempt().unwrap())
        .result()
        .await
        .expect_err("successful headers precede the body timeout");
    assert!(matches!(error.cause(), ProviderError::Timeout));
    assert!(call.retry_admission().is_none(), "no post-handoff resend");
    assert_eq!(
        call.acquisition_seal(),
        Some(opi_ai::retry::AcquisitionSeal::PostHandoff)
    );
    peer.join().unwrap();
}

// ---------------------------------------------------------------------------
// Provider identity
// ---------------------------------------------------------------------------

#[test]
fn responses_provider_id_is_openai_responses() {
    let provider = responses_provider();
    assert_eq!(provider.id(), "openai-responses");
}

// ---------------------------------------------------------------------------
// Model resolution via registry
// ---------------------------------------------------------------------------

#[test]
fn responses_resolves_model_in_registry() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(Box::new(responses_provider()))
        .unwrap();
    let (provider, model) = registry.resolve("openai-responses:gpt-4o").unwrap();
    assert_eq!(provider.id(), "openai-responses");
    assert_eq!(model.id, "gpt-4o");
}

#[test]
fn responses_registry_lists_provider_id() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(Box::new(responses_provider()))
        .unwrap();
    let ids = registry.provider_ids();
    assert!(ids.contains(&"openai-responses"));
}

#[test]
fn responses_unknown_model_returns_error() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(Box::new(responses_provider()))
        .unwrap();
    let result = registry.resolve("openai-responses:nonexistent-model");
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Request body construction
// ---------------------------------------------------------------------------

#[test]
fn responses_request_body_uses_input_field() {
    let provider = responses_provider();
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai-responses:gpt-4o".into(),
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
    // Responses API uses "input" not "messages"
    assert!(body.get("input").is_some(), "should have 'input' field");
    assert!(
        body.get("messages").is_none(),
        "should NOT have 'messages' field"
    );
    // max_output_tokens not max_tokens
    assert_eq!(body["max_output_tokens"], 1024);
    assert!(
        body.get("max_tokens").is_none(),
        "should NOT have 'max_tokens' field"
    );
    // Model should be stripped of prefix
    assert_eq!(body["model"], "gpt-4o");
    assert!(body.get("instructions").is_none());
    assert_eq!(
        body["input"][0],
        serde_json::json!({"role":"system","content":"You are helpful."})
    );
}

#[test]
fn responses_request_body_strips_provider_prefix() {
    let provider = responses_provider();
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai-responses:o3".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
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
    };
    let body = provider.build_request_body(&request).unwrap();
    assert_eq!(body["model"], "o3");
}

// ---------------------------------------------------------------------------
// Phase 12 task 12.3 — native Responses request semantics
//
// DoD: OpenAI Responses fixture tests assert native Responses request semantics
// for store, strict JSON schema/tool schema, previous_response_id, and
// reasoning.effort where supported by opi's existing request model, or 12.9
// documents the unsupported bits as deferred provider correctness work.
// ---------------------------------------------------------------------------

fn responses_tool_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai-responses:gpt-4o".into(),
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
    }
}

fn responses_tool_context() -> opi_ai::transcript::Context {
    opi_ai::transcript::Context {
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
    }
}

#[test]
fn responses_config_represents_store_reasoning_strict() {
    let config = ResponsesConfig {
        store: Some(true),
        reasoning_effort: Some("high".into()),
        source: opi_ai::model_info::OpenAiResponsesSourceCompat {
            supports_strict_mode: Some(true),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(config.store, Some(true));
    assert_eq!(config.reasoning_effort.as_deref(), Some("high"));
    assert_eq!(config.source.supports_strict_mode, Some(true));
}

#[test]
fn responses_request_body_store_emitted_when_configured() {
    let provider = OpenAiResponsesProvider::new_with_config(
        None,
        ResponsesConfig {
            store: Some(false),
            ..Default::default()
        },
    );
    let body = provider
        .build_request_body(&responses_tool_request())
        .unwrap();
    assert_eq!(
        body["store"], false,
        "store must be emitted when configured"
    );
}

#[test]
fn responses_static_reasoning_effort_does_not_drive_wire_output() {
    let provider = OpenAiResponsesProvider::new_with_config(
        None,
        ResponsesConfig {
            reasoning_effort: Some("high".into()),
            ..Default::default()
        },
    );
    let body = provider
        .build_request_body(&responses_tool_request())
        .unwrap();
    assert!(body.get("reasoning").is_none());
}

#[test]
fn responses_request_body_reasoning_effort_absent_by_default() {
    let provider = responses_provider();
    let body = provider
        .build_request_body(&responses_tool_request())
        .unwrap();
    assert!(
        body.get("reasoning").is_none(),
        "reasoning must be absent by default"
    );
}

#[test]
fn responses_reasoning_effort_uses_request_thinking_and_model_map() {
    use std::sync::Arc;

    use opi_ai::http::HttpClient;
    use opi_ai::{
        ModelCapabilities, ModelInfo, ThinkingLevel, ThinkingLevelMap, ThinkingLevelMapping,
        WireApi,
    };

    let provider = responses_provider();
    let mut identity = responses_tool_request();
    identity.model = "openai-responses:o3".into();
    identity.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: None,
        level: ThinkingLevel::High,
    };
    assert_eq!(
        provider.build_request_body(&identity).unwrap()["reasoning"]["effort"],
        "high"
    );

    let mapped_model = ModelInfo::new(
        "mapped",
        "Mapped",
        WireApi::OpenAiResponses,
        ModelCapabilities::new(128_000, 16_384)
            .with_streaming(true)
            .with_thinking(true),
    )
    .with_thinking_level_map(ThinkingLevelMap::reasoning_default().with_mapping(
        ThinkingLevel::High,
        ThinkingLevelMapping::Mapped("provider-high".into()),
    ));
    let mapped_provider = OpenAiResponsesProvider::for_route(
        Some("https://example.test".into()),
        "mapped".into(),
        Default::default(),
        vec![mapped_model],
        Arc::new(HttpClient::new()),
    );
    let mut remapped = identity;
    remapped.model = "mapped:mapped".into();
    assert_eq!(
        mapped_provider.build_request_body(&remapped).unwrap()["reasoning"]["effort"],
        "provider-high"
    );

    let mut disabled = remapped;
    disabled.thinking.enabled = false;
    assert_eq!(
        mapped_provider.build_request_body(&disabled).unwrap()["reasoning"],
        serde_json::json!({"effort": "none"}),
        "reasoning models send the off mapping when no effort is requested (source: off ?? none)"
    );

    let mut off = disabled;
    off.thinking.enabled = true;
    off.thinking.level = ThinkingLevel::None;
    assert_eq!(
        mapped_provider.build_request_body(&off).unwrap()["reasoning"],
        serde_json::json!({"effort": "none"})
    );

    let unsupported_model = ModelInfo::new(
        "unsupported",
        "Unsupported",
        WireApi::OpenAiResponses,
        ModelCapabilities::new(128_000, 16_384)
            .with_streaming(true)
            .with_thinking(true),
    )
    .with_thinking_level_map(ThinkingLevelMap::disabled());
    let unsupported_provider = OpenAiResponsesProvider::for_route(
        Some("https://example.test".into()),
        "unsupported".into(),
        Default::default(),
        vec![unsupported_model],
        Arc::new(HttpClient::new()),
    );
    let mut unsupported = off;
    unsupported.model = "unsupported:unsupported".into();
    unsupported.thinking.level = ThinkingLevel::High;
    assert!(
        unsupported_provider
            .build_request_body(&unsupported)
            .unwrap()
            .get("reasoning")
            .is_none()
    );
}

#[test]
fn responses_request_body_strict_tools_emitted_when_configured() {
    let provider = OpenAiResponsesProvider::new_with_config(
        None,
        ResponsesConfig {
            source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                supports_strict_mode: Some(true),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let body = provider
        .build_request_body(&responses_tool_request())
        .unwrap();
    let tools = body["tools"].as_array().expect("tools array present");
    assert!(
        tools[0]["strict"] == false,
        "strict capability keeps unconstrained function tools explicitly non-strict: {tools:?}"
    );
}

#[test]
fn responses_request_body_strict_tools_absent_by_default() {
    let provider = responses_provider();
    let body = provider
        .build_request_body(&responses_tool_request())
        .unwrap();
    let tools = body["tools"].as_array().expect("tools array present");
    assert!(
        tools[0].get("strict").is_none(),
        "strict must be absent by default: {tools:?}"
    );
}

#[test]
fn responses_request_body_previous_response_id_is_deferred() {
    // DoD escape hatch: "where supported by opi's existing request model, or
    // 12.9 must explicitly document any unsupported native Responses semantics
    // as deferred." opi's Request model carries no prior-response state (the
    // agent runtime reconstructs context from the message history, not from a
    // server-side response chain), so previous_response_id is deferred to 12.9.
    // This test pins that the shared Responses adapter does not synthesize a
    // previous_response_id field today; 12.9 documents the deferral in docs.
    let provider = responses_provider();
    let body = provider
        .build_request_body(&responses_tool_request())
        .unwrap();
    assert!(
        body.get("previous_response_id").is_none(),
        "previous_response_id is deferred (no response-loop state in opi's Request model)"
    );
}

// ---------------------------------------------------------------------------
// SSE text streaming
// ---------------------------------------------------------------------------

#[tokio::test]
async fn responses_text_streaming_produces_start_delta_done() {
    let provider = responses_provider();
    let sse = "event: response.created\n\
               data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n\
               event: response.output_item.added\n\
               data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"in_progress\",\"role\":\"assistant\",\"content\":[]}}\n\n\
               event: response.content_part.added\n\
               data: {\"type\":\"response.content_part.added\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"output_text\",\"text\":\"\"}}\n\n\
               event: response.output_text.delta\n\
               data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\"Hi there\"}\n\n\
               event: response.output_text.done\n\
               data: {\"type\":\"response.output_text.done\",\"output_index\":0,\"content_index\":0,\"text\":\"Hi there\"}\n\n\
               event: response.output_item.done\n\
               data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"completed\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hi there\"}]}}\n\n\
               event: response.completed\n\
               data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hi there\"}]}],\"usage\":{\"input_tokens\":10,\"output_tokens\":5}}}\n\n";

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
async fn responses_done_event_has_provider_id() {
    let provider = responses_provider();
    let sse = "event: response.created\n\
               data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n\
               event: response.output_item.added\n\
               data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"in_progress\",\"role\":\"assistant\",\"content\":[]}}\n\n\
               event: response.content_part.added\n\
               data: {\"type\":\"response.content_part.added\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"output_text\",\"text\":\"\"}}\n\n\
               event: response.output_text.delta\n\
               data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\"Hello\"}\n\n\
               event: response.output_text.done\n\
               data: {\"type\":\"response.output_text.done\",\"output_index\":0,\"content_index\":0,\"text\":\"Hello\"}\n\n\
               event: response.output_item.done\n\
               data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"completed\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hello\"}]}}\n\n\
               event: response.completed\n\
               data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hello\"}]}],\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}}\n\n";

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let done_provider = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::Done { message, .. } => Some(message.provider.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(done_provider, "openai-responses");
}

// ---------------------------------------------------------------------------
// SSE tool call streaming
// ---------------------------------------------------------------------------

#[tokio::test]
async fn responses_tool_call_streaming_works() {
    let provider = responses_provider();
    let sse = "event: response.created\n\
               data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n\
               event: response.output_item.added\n\
               data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"read_file\",\"arguments\":\"\"}}\n\n\
               event: response.function_call_arguments.delta\n\
               data: {\"type\":\"response.function_call_arguments.delta\",\"output_index\":0,\"item_id\":\"fc_1\",\"call_id\":\"call_1\",\"delta\":\"{\\\"path\\\":\\\"foo.rs\\\"}\"}\n\n\
               event: response.output_item.done\n\
               data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"read_file\",\"arguments\":\"{\\\"path\\\":\\\"foo.rs\\\"}\"}}\n\n\
               event: response.completed\n\
               data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"read_file\",\"arguments\":\"{\\\"path\\\":\\\"foo.rs\\\"}\"}],\"usage\":{\"input_tokens\":20,\"output_tokens\":10}}}\n\n";

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
async fn responses_error_event_routing() {
    let provider = responses_provider();
    let sse = "event: error\n\
               data: {\"type\":\"error\",\"message\":\"Model not found\"}\n\n";

    let items: Vec<Result<AssistantStreamEvent, ProviderError>> = provider
        .stream_from_sse(sse, CancellationToken::new())
        .collect()
        .await;

    // Current producers stop the dual terminal format: the error event
    // routes to a typed stream failure, not an Error event.
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
async fn responses_usage_in_done_event() {
    let provider = responses_provider();
    let sse = "event: response.created\n\
               data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n\
               event: response.output_item.added\n\
               data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"in_progress\",\"role\":\"assistant\",\"content\":[]}}\n\n\
               event: response.content_part.added\n\
               data: {\"type\":\"response.content_part.added\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"output_text\",\"text\":\"\"}}\n\n\
               event: response.output_text.delta\n\
               data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\"test\"}\n\n\
               event: response.output_text.done\n\
               data: {\"type\":\"response.output_text.done\",\"output_index\":0,\"content_index\":0,\"text\":\"test\"}\n\n\
               event: response.output_item.done\n\
               data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"completed\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"test\"}]}}\n\n\
               event: response.completed\n\
               data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"test\"}]}],\"usage\":{\"input_tokens\":42,\"output_tokens\":13}}}\n\n";

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let usage = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::Done { message, .. } => Some(message.usage.clone()),
            _ => None,
        })
        .unwrap();
    assert!(usage.is_reported());
    assert_eq!(usage.input_tokens, 42);
    assert_eq!(usage.output_tokens, 13);
}

// ---------------------------------------------------------------------------
// Response ID propagation (Phase 12 task 12.6, DoD clause 7)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn responses_response_id_round_trips_into_done_message() {
    let provider = responses_provider();
    let sse = "event: response.created\n\
               data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n\
               event: response.output_item.added\n\
               data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"in_progress\",\"role\":\"assistant\",\"content\":[]}}\n\n\
               event: response.content_part.added\n\
               data: {\"type\":\"response.content_part.added\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"output_text\",\"text\":\"\"}}\n\n\
               event: response.output_text.delta\n\
               data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\"test\"}\n\n\
               event: response.output_text.done\n\
               data: {\"type\":\"response.output_text.done\",\"output_index\":0,\"content_index\":0,\"text\":\"test\"}\n\n\
               event: response.output_item.done\n\
               data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"completed\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"test\"}]}}\n\n\
               event: response.completed\n\
               data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"test\"}]}],\"usage\":{\"input_tokens\":42,\"output_tokens\":13}}}\n\n";

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let done = events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("stream must produce a Done event");
    let response_id = match done {
        AssistantStreamEvent::Done { message, .. } => &message.response_id,
        _ => unreachable!(),
    };
    assert_eq!(
        response_id,
        &Some("resp_1".to_string()),
        "OpenAI Responses response id must round-trip into AssistantMessage::response_id instead of being dropped"
    );
}

// ---------------------------------------------------------------------------
// Cache token fields (Phase 12 task 12.6, DoD clause 6)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn responses_cache_tokens_in_done_event() {
    let provider = responses_provider();
    let sse = "event: response.created\n\
               data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_cache\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n\
               event: response.output_item.added\n\
               data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"in_progress\",\"role\":\"assistant\",\"content\":[]}}\n\n\
               event: response.content_part.added\n\
               data: {\"type\":\"response.content_part.added\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"output_text\",\"text\":\"\"}}\n\n\
               event: response.output_text.delta\n\
               data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\"cached\"}\n\n\
               event: response.output_text.done\n\
               data: {\"type\":\"response.output_text.done\",\"output_index\":0,\"content_index\":0,\"text\":\"cached\"}\n\n\
               event: response.output_item.done\n\
               data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"completed\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"cached\"}]}}\n\n\
               event: response.completed\n\
               data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_cache\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"cached\"}]}],\"usage\":{\"input_tokens\":100,\"output_tokens\":20,\"input_tokens_details\":{\"cached_tokens\":400}}}}\n\n";

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let done = events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("expected Done event");
    if let AssistantStreamEvent::Done { message, .. } = done {
        assert_eq!(
            message.usage.cache_read_tokens, 400,
            "input_tokens_details.cached_tokens must map to cache_read_tokens"
        );
    }
}

// --- Reasoning tokens (Phase 14 task 14.5) ---

fn responses_reasoning_fixture(reasoning_tokens: Option<u64>, output_tokens: u32) -> String {
    let mut usage = serde_json::json!({
        "input_tokens": 100,
        "output_tokens": output_tokens,
    });
    if let Some(tokens) = reasoning_tokens {
        usage["output_tokens_details"] = serde_json::json!({"reasoning_tokens": tokens});
    }
    let payload = serde_json::json!({
        "type": "response.completed",
        "response": {
            "id": "resp_reason",
            "status": "completed",
            "model": "o3",
            "output": [],
            "usage": usage,
        },
    });
    format!("event: response.completed\ndata: {payload}\n\n")
}

#[tokio::test]
async fn responses_reasoning_tokens_in_done_event() {
    let provider = responses_provider();
    let sse = "event: response.created\n\
               data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_reason\",\"status\":\"in_progress\",\"model\":\"o3\",\"output\":[]}}\n\n\
               event: response.output_item.added\n\
               data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"in_progress\",\"role\":\"assistant\",\"content\":[]}}\n\n\
               event: response.content_part.added\n\
               data: {\"type\":\"response.content_part.added\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"output_text\",\"text\":\"\"}}\n\n\
               event: response.output_text.delta\n\
               data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\"ok\"}\n\n\
               event: response.output_text.done\n\
               data: {\"type\":\"response.output_text.done\",\"output_index\":0,\"content_index\":0,\"text\":\"ok\"}\n\n\
               event: response.output_item.done\n\
               data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"completed\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"ok\"}]}}\n\n\
               event: response.completed\n\
               data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_reason\",\"status\":\"completed\",\"model\":\"o3\",\"output\":[],\"usage\":{\"input_tokens\":100,\"output_tokens\":500,\"input_tokens_details\":{\"cached_tokens\":0},\"output_tokens_details\":{\"reasoning_tokens\":300}}}}\n\n";

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;
    let done = events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("expected Done event");
    if let AssistantStreamEvent::Done { message, .. } = done {
        assert_eq!(message.usage.output_tokens, 500);
        assert_eq!(message.usage.reasoning_tokens, Some(300));
        assert!(message.usage.reasoning_tokens.unwrap() <= u64::from(message.usage.output_tokens));
    }
}

#[tokio::test]
async fn responses_reasoning_absent_zero_and_equality_are_preserved() {
    let provider = responses_provider();
    for (reasoning, expected) in [(None, None), (Some(0), Some(0)), (Some(500), Some(500))] {
        let sse = responses_reasoning_fixture(reasoning, 500);
        let events = collect_stream(provider.stream_from_sse(&sse, CancellationToken::new())).await;
        let usage = events
            .iter()
            .find_map(|event| match event {
                AssistantStreamEvent::Done { message, .. } => Some(&message.usage),
                _ => None,
            })
            .expect("expected Done event");
        assert_eq!(usage.reasoning_tokens, expected);
    }
}

#[tokio::test]
async fn responses_reasoning_malformed_subset_stops_production_stream_with_non_retryable_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_reasoning_fixture(Some(800), 500))
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = OpenAiResponsesProvider::new(Some(server.uri()));
    let results: Vec<_> = provider
        .stream_prepared(
            responses_tool_request(),
            opi_ai::test_support::resolved_bearer_auth(),
        )
        .collect()
        .await;
    let errors: Vec<_> = results
        .iter()
        .filter_map(|result| result.as_ref().err())
        .collect();
    assert_eq!(errors.len(), 1, "invalid usage must emit one error");
    assert!(errors[0].category() == opi_ai::provider::ProviderErrorCategory::Stream);
    assert_eq!(
        errors[0].to_string(),
        "stream error: reasoning_tokens (800) exceeds output_tokens (500)"
    );
    assert!(!errors[0].is_retryable());
    assert!(matches!(
        results.last(),
        Some(Err(cause)) if cause.category() == opi_ai::provider::ProviderErrorCategory::Stream
    ));
    assert!(
        !results.iter().any(|result| matches!(
            result,
            Ok(AssistantStreamEvent::Done { .. } | AssistantStreamEvent::Error { .. })
        )),
        "no completion event may follow malformed usage"
    );
}

// ---------------------------------------------------------------------------
// Model list
// ---------------------------------------------------------------------------

#[test]
fn responses_has_model_list() {
    let provider = responses_provider();
    let models = provider.models();
    assert!(
        !models.is_empty(),
        "Responses provider should have at least one model"
    );
    // Should include gpt-4o and o-series models
    assert!(
        models.iter().any(|m| m.id == "gpt-4o"),
        "should have gpt-4o model"
    );
    assert!(models.iter().any(|m| m.id == "o3"), "should have o3 model");
}

// ---------------------------------------------------------------------------
// Multi-delta text streaming
// ---------------------------------------------------------------------------

#[tokio::test]
async fn responses_multiple_text_deltas() {
    let provider = responses_provider();
    let sse = "event: response.created\n\
               data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n\
               event: response.output_item.added\n\
               data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"in_progress\",\"role\":\"assistant\",\"content\":[]}}\n\n\
               event: response.content_part.added\n\
               data: {\"type\":\"response.content_part.added\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"output_text\",\"text\":\"\"}}\n\n\
               event: response.output_text.delta\n\
               data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\"Hello\"}\n\n\
               event: response.output_text.delta\n\
               data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\" world\"}\n\n\
               event: response.output_text.delta\n\
               data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\"!\"}\n\n\
               event: response.output_text.done\n\
               data: {\"type\":\"response.output_text.done\",\"output_index\":0,\"content_index\":0,\"text\":\"Hello world!\"}\n\n\
               event: response.output_item.done\n\
               data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"completed\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hello world!\"}]}}\n\n\
               event: response.completed\n\
               data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hello world!\"}]}],\"usage\":{\"input_tokens\":10,\"output_tokens\":5}}}\n\n";

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let deltas: Vec<&AssistantStreamEvent> = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::TextDelta { .. }))
        .collect();
    assert_eq!(deltas.len(), 3, "should have three TextDelta events");

    // Verify accumulated text in the Done message
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
// Tool call with multiple argument deltas
// ---------------------------------------------------------------------------

#[tokio::test]
async fn responses_tool_call_multiple_arg_deltas() {
    let provider = responses_provider();
    let sse = "event: response.created\n\
               data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n\
               event: response.output_item.added\n\
               data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"edit_file\",\"arguments\":\"\"}}\n\n\
               event: response.function_call_arguments.delta\n\
               data: {\"type\":\"response.function_call_arguments.delta\",\"output_index\":0,\"item_id\":\"fc_1\",\"call_id\":\"call_1\",\"delta\":\"{\\\"path\\\":\"}\n\n\
               event: response.function_call_arguments.delta\n\
               data: {\"type\":\"response.function_call_arguments.delta\",\"output_index\":0,\"item_id\":\"fc_1\",\"call_id\":\"call_1\",\"delta\":\"\\\"main.rs\\\",\\\"old\\\":\\\"fn main()\\\"}\"}\n\n\
               event: response.output_item.done\n\
               data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"edit_file\",\"arguments\":\"{\\\"path\\\":\\\"main.rs\\\",\\\"old\\\":\\\"fn main()\\\"}\"}}\n\n\
               event: response.completed\n\
               data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"edit_file\",\"arguments\":\"{\\\"path\\\":\\\"main.rs\\\",\\\"old\\\":\\\"fn main()\\\"}\"}],\"usage\":{\"input_tokens\":20,\"output_tokens\":15}}}\n\n";

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let arg_deltas: Vec<&AssistantStreamEvent> = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::ToolCallDelta { .. }))
        .collect();
    assert_eq!(arg_deltas.len(), 2, "should have two ToolCallDelta events");
}

// ---------------------------------------------------------------------------
// Custom base URL
// ---------------------------------------------------------------------------

#[test]
fn responses_custom_base_url() {
    let provider =
        opi_ai::openai_responses::OpenAiResponsesProvider::new(Some("https://custom.proxy".into()));
    assert_eq!(provider.id(), "openai-responses");
}

// ---------------------------------------------------------------------------
// Phase 12 task 12.4 — tool-call conversion breadth (scenarios 3/5/6)
//
// DoD: multiple tool calls, malformed JSON arguments, and provider tool-call
// IDs. The Responses mapper uses `call_id` as ToolCall.id (the value
// function_call_output must echo back) and accumulates the raw argument string
// without parsing, so malformed JSON is preserved for the agent loop.

#[tokio::test]
async fn responses_multi_tool_call_produces_two_calls() {
    let provider = responses_provider();
    let sse = "event: response.created\n\
               data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_multi\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n\
               event: response.output_item.added\n\
               data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_a\",\"call_id\":\"call_a\",\"name\":\"read_file\",\"arguments\":\"\"}}\n\n\
               event: response.function_call_arguments.delta\n\
               data: {\"type\":\"response.function_call_arguments.delta\",\"output_index\":0,\"item_id\":\"fc_a\",\"call_id\":\"call_a\",\"delta\":\"{\\\"path\\\":\\\"a.rs\\\"}\"}\n\n\
               event: response.output_item.done\n\
               data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_a\",\"call_id\":\"call_a\",\"name\":\"read_file\",\"arguments\":\"{\\\"path\\\":\\\"a.rs\\\"}\"}}\n\n\
               event: response.output_item.added\n\
               data: {\"type\":\"response.output_item.added\",\"output_index\":1,\"item\":{\"type\":\"function_call\",\"id\":\"fc_b\",\"call_id\":\"call_b\",\"name\":\"bash\",\"arguments\":\"\"}}\n\n\
               event: response.function_call_arguments.delta\n\
               data: {\"type\":\"response.function_call_arguments.delta\",\"output_index\":1,\"item_id\":\"fc_b\",\"call_id\":\"call_b\",\"delta\":\"{\\\"cmd\\\":\\\"ls\\\"}\"}\n\n\
               event: response.output_item.done\n\
               data: {\"type\":\"response.output_item.done\",\"output_index\":1,\"item\":{\"type\":\"function_call\",\"id\":\"fc_b\",\"call_id\":\"call_b\",\"name\":\"bash\",\"arguments\":\"{\\\"cmd\\\":\\\"ls\\\"}\"}}\n\n\
               event: response.completed\n\
               data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_multi\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"function_call\",\"id\":\"fc_a\",\"call_id\":\"call_a\",\"name\":\"read_file\",\"arguments\":\"{\\\"path\\\":\\\"a.rs\\\"}\"},{\"type\":\"function_call\",\"id\":\"fc_b\",\"call_id\":\"call_b\",\"name\":\"bash\",\"arguments\":\"{\\\"cmd\\\":\\\"ls\\\"}\"}],\"usage\":{\"input_tokens\":30,\"output_tokens\":20}}}\n\n";

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let starts = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::ToolCallStart { .. }))
        .count();
    let ends = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::ToolCallEnd { .. }))
        .count();
    assert_eq!(starts, 2, "two ToolCallStart events");
    assert_eq!(ends, 2, "two ToolCallEnd events");

    let content_len = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::Done { message, .. } => Some(message.content.len()),
            _ => None,
        })
        .expect("Done event");
    assert_eq!(content_len, 2, "Done message carries both function calls");
}

#[tokio::test]
async fn responses_interleaved_tool_deltas_route_by_output_index() {
    let provider = responses_provider();
    let sse = r#"event: response.created
data: {"type":"response.created","response":{"id":"resp_interleave","model":"gpt-4o"}}

event: response.output_item.added
data: {"type":"response.output_item.added","output_index":0,"item":{"type":"function_call","id":"fc_a","call_id":"call_a","name":"read_file","arguments":""}}

event: response.output_item.added
data: {"type":"response.output_item.added","output_index":1,"item":{"type":"function_call","id":"fc_b","call_id":"call_b","name":"bash","arguments":""}}

event: response.function_call_arguments.delta
data: {"type":"response.function_call_arguments.delta","output_index":0,"item_id":"fc_a","call_id":"call_a","delta":"{\"path\":"}

event: response.function_call_arguments.delta
data: {"type":"response.function_call_arguments.delta","output_index":1,"item_id":"fc_b","call_id":"call_b","delta":"{\"cmd\":"}

event: response.function_call_arguments.delta
data: {"type":"response.function_call_arguments.delta","output_index":0,"item_id":"fc_a","call_id":"call_a","delta":"\"a.rs\"}"}

event: response.function_call_arguments.delta
data: {"type":"response.function_call_arguments.delta","output_index":1,"item_id":"fc_b","call_id":"call_b","delta":"\"ls\"}"}

event: response.output_item.done
data: {"type":"response.output_item.done","output_index":0,"item":{"type":"function_call","id":"fc_a","call_id":"call_a","name":"read_file","arguments":"{\"path\":\"a.rs\"}"}}

event: response.output_item.done
data: {"type":"response.output_item.done","output_index":1,"item":{"type":"function_call","id":"fc_b","call_id":"call_b","name":"bash","arguments":"{\"cmd\":\"ls\"}"}}

event: response.completed
data: {"type":"response.completed","response":{"id":"resp_interleave","model":"gpt-4o","usage":{"input_tokens":1,"output_tokens":2}}}

"#;

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;
    let ended: Vec<_> = events
        .into_iter()
        .filter_map(|event| match event {
            AssistantStreamEvent::ToolCallEnd { tool_call, .. } => Some(tool_call),
            _ => None,
        })
        .collect();

    assert_eq!(ended.len(), 2);
    // Dual-ID packing (V-07): the canonical id carries `call_id|fc_item_id`
    // so same-identity replay can restore both fields.
    assert_eq!(ended[0].id, "call_a|fc_a");
    assert_eq!(ended[0].arguments, "{\"path\":\"a.rs\"}");
    assert_eq!(ended[1].id, "call_b|fc_b");
    assert_eq!(ended[1].arguments, "{\"cmd\":\"ls\"}");
}

#[tokio::test]
async fn responses_message_output_item_done_does_not_duplicate_tool_end() {
    let provider = responses_provider();
    let sse = r#"event: response.created
data: {"type":"response.created","response":{"id":"resp_mixed","model":"gpt-4o"}}

event: response.output_item.added
data: {"type":"response.output_item.added","output_index":0,"item":{"type":"function_call","id":"fc_1","call_id":"call_1","name":"read_file","arguments":""}}

event: response.function_call_arguments.delta
data: {"type":"response.function_call_arguments.delta","output_index":0,"item_id":"fc_1","call_id":"call_1","delta":"{\"path\":\"a.rs\"}"}

event: response.output_item.done
data: {"type":"response.output_item.done","output_index":0,"item":{"type":"function_call","id":"fc_1","call_id":"call_1","name":"read_file","arguments":"{\"path\":\"a.rs\"}"}}

event: response.output_item.added
data: {"type":"response.output_item.added","output_index":1,"item":{"type":"message","status":"in_progress","role":"assistant","content":[]}}

event: response.output_text.delta
data: {"type":"response.output_text.delta","output_index":1,"content_index":0,"delta":"ok"}

event: response.output_item.done
data: {"type":"response.output_item.done","output_index":1,"item":{"type":"message","status":"completed","role":"assistant","content":[{"type":"output_text","text":"ok"}]}}

event: response.completed
data: {"type":"response.completed","response":{"id":"resp_mixed","model":"gpt-4o","usage":{"input_tokens":1,"output_tokens":2}}}

"#;

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;
    let tool_ends = events
        .iter()
        .filter(|event| matches!(event, AssistantStreamEvent::ToolCallEnd { .. }))
        .count();

    assert_eq!(tool_ends, 1);
}

#[tokio::test]
async fn responses_text_after_tool_reports_actual_content_index() {
    let provider = responses_provider();
    let sse = r#"event: response.created
data: {"type":"response.created","response":{"id":"resp_mixed_index","model":"gpt-4o"}}

event: response.output_item.added
data: {"type":"response.output_item.added","output_index":0,"item":{"type":"function_call","id":"fc_1","call_id":"call_1","name":"read_file","arguments":""}}

event: response.function_call_arguments.delta
data: {"type":"response.function_call_arguments.delta","output_index":0,"item_id":"fc_1","call_id":"call_1","delta":"{\"path\":\"a.rs\"}"}

event: response.output_item.done
data: {"type":"response.output_item.done","output_index":0,"item":{"type":"function_call","id":"fc_1","call_id":"call_1","name":"read_file","arguments":"{\"path\":\"a.rs\"}"}}

event: response.output_item.added
data: {"type":"response.output_item.added","output_index":1,"item":{"type":"message","status":"in_progress","role":"assistant","content":[]}}

event: response.output_text.delta
data: {"type":"response.output_text.delta","output_index":1,"content_index":0,"delta":"ok"}

event: response.output_item.done
data: {"type":"response.output_item.done","output_index":1,"item":{"type":"message","status":"completed","role":"assistant","content":[{"type":"output_text","text":"ok"}]}}

event: response.completed
data: {"type":"response.completed","response":{"id":"resp_mixed_index","model":"gpt-4o","usage":{"input_tokens":1,"output_tokens":2}}}

"#;

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;
    let text_indexes: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            AssistantStreamEvent::TextStart { content_index, .. }
            | AssistantStreamEvent::TextDelta { content_index, .. }
            | AssistantStreamEvent::TextEnd { content_index, .. } => Some(*content_index),
            _ => None,
        })
        .collect();
    let done_content_len = events
        .iter()
        .find_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => Some(message.content.len()),
            _ => None,
        })
        .expect("Done event");

    assert_eq!(
        text_indexes,
        vec![1, 1, 1],
        "text lifecycle events must point at the text block after the preceding tool call"
    );
    assert_eq!(done_content_len, 2);
}

#[tokio::test]
async fn responses_completed_closes_unfinished_tool_call_before_done() {
    let provider = responses_provider();
    let sse = r#"event: response.created
data: {"type":"response.created","response":{"id":"resp_fallback","model":"gpt-4o"}}

event: response.output_item.added
data: {"type":"response.output_item.added","output_index":0,"item":{"type":"function_call","id":"fc_1","call_id":"call_1","name":"read_file","arguments":""}}

event: response.completed
data: {"type":"response.completed","response":{"id":"resp_fallback","model":"gpt-4o","output":[{"type":"function_call","id":"fc_1","call_id":"call_1","name":"read_file","arguments":"{\"path\":\"a.rs\"}"}],"usage":{"input_tokens":1,"output_tokens":2}}}

"#;

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;
    let ended: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            AssistantStreamEvent::ToolCallEnd { tool_call, .. } => Some(tool_call.clone()),
            _ => None,
        })
        .collect();
    let tool_end_index = events
        .iter()
        .position(|event| matches!(event, AssistantStreamEvent::ToolCallEnd { .. }))
        .expect("ToolCallEnd emitted before Done");
    let done_index = events
        .iter()
        .position(|event| matches!(event, AssistantStreamEvent::Done { .. }))
        .expect("Done emitted");
    let tool_ends = events
        .iter()
        .filter(|event| matches!(event, AssistantStreamEvent::ToolCallEnd { .. }))
        .count();

    assert_eq!(tool_ends, 1, "one ToolCallEnd from completion fallback");
    assert_eq!(ended[0].id, "call_1|fc_1");
    assert_eq!(ended[0].name, "read_file");
    assert_eq!(ended[0].arguments, "{\"path\":\"a.rs\"}");
    assert!(
        tool_end_index < done_index,
        "ToolCallEnd should be emitted before Done"
    );
}

#[tokio::test]
async fn responses_tool_call_id_is_the_call_id() {
    // Scenario 6: provider tool-call ID round-trip. The Responses API gives
    // function_call items both an `id` (fc_1) and a `call_id` (call_1); opi
    // MUST surface `call_id` as ToolCall.id because that is the value a
    // subsequent function_call_output must echo back.
    let provider = responses_provider();
    let sse = "event: response.created\n\
               data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_id\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n\
               event: response.output_item.added\n\
               data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_roundtrip\",\"name\":\"read_file\",\"arguments\":\"{}\"}}\n\n\
               event: response.output_item.done\n\
               data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_roundtrip\",\"name\":\"read_file\",\"arguments\":\"{}\"}}\n\n\
               event: response.completed\n\
               data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_id\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_roundtrip\",\"name\":\"read_file\",\"arguments\":\"{}\"}],\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}}\n\n";

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let end = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::ToolCallEnd { tool_call, .. } => Some(tool_call.clone()),
            _ => None,
        })
        .expect("ToolCallEnd emitted");
    assert_eq!(
        end.id, "call_roundtrip|fc_1",
        "ToolCall.id packs `call_id|fc_item_id` so both round-trip (V-07)"
    );
    assert_eq!(
        end.arguments, "{}",
        "Responses tool-call arguments carried on output_item.added/done must be preserved even without argument deltas"
    );
}

#[tokio::test]
async fn responses_malformed_tool_args_pass_raw_string_without_panic() {
    let provider = responses_provider();
    let sse = "event: response.created\n\
               data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_bad\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n\
               event: response.output_item.added\n\
               data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_bad\",\"call_id\":\"call_bad\",\"name\":\"read_file\",\"arguments\":\"\"}}\n\n\
               event: response.function_call_arguments.delta\n\
               data: {\"type\":\"response.function_call_arguments.delta\",\"output_index\":0,\"item_id\":\"fc_bad\",\"call_id\":\"call_bad\",\"delta\":\"{not-json\"}\n\n\
               event: response.output_item.done\n\
               data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_bad\",\"call_id\":\"call_bad\",\"name\":\"read_file\",\"arguments\":\"{not-json\"}}\n\n\
               event: response.completed\n\
               data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_bad\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"function_call\",\"id\":\"fc_bad\",\"call_id\":\"call_bad\",\"name\":\"read_file\",\"arguments\":\"{not-json\"}],\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}}\n\n";

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let end = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::ToolCallEnd { tool_call, .. } => Some(tool_call.clone()),
            _ => None,
        })
        .expect("ToolCallEnd emitted despite malformed argument JSON");
    assert_eq!(end.arguments, "{not-json");
    assert_eq!(end.id, "call_bad|fc_bad");
    assert_eq!(end.name, "read_file");
}

// ---------------------------------------------------------------------------
// Production request contract through Provider::stream (Phase 12.1)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_sends_text_request_body_and_auth_through_http() {
    let sse = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hi\"}]}],\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}}\n\n",
    );
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(header("authorization", "Bearer test-key"))
        .and(body_partial_json(serde_json::json!({
            "model": "gpt-4o",
            "max_output_tokens": 1024,
            "input": [{"role":"system","content":"You are helpful."}]
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenAiResponsesProvider::new(Some(server.uri()));
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai-responses:gpt-4o".into(),
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

    // verify() confirms the production request carried the Responses body
    // (model + max_output_tokens + instruction input), the Bearer auth header, and
    // the /v1/responses path.
    server.verify().await;
}

// ---------------------------------------------------------------------------
// Provider stream cancellation (Phase 12 task 12.7 DoD clause 6)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_cancellation_drains_without_hang_after_cancel() {
    // The CancellationToken is threaded into the OpenAI Responses adapter's
    // HTTP body-stream loop (openai_responses.rs `cancel.cancelled()` select
    // arm). This wiremock fixture is fully buffered, so it only proves the
    // adapter drains promptly after cancellation; it does not prove
    // cancellation wins a race against delayed terminal SSE data.
    let sse = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hi\"}]}],\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}}\n\n",
    );
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = OpenAiResponsesProvider::new(Some(server.uri()));
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai-responses:gpt-4o".into(),
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

    let _ = stream
        .next()
        .await
        .expect("stream should produce at least one event");
    cancel.cancel();

    let drain = async {
        while let Some(result) = stream.next().await {
            match result {
                Ok(event) if event.is_terminal() => break,
                Err(_) => break,
                _ => {}
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(2), drain)
        .await
        .expect("stream must drain promptly after cancellation (no hang/panic)");
}

// ---------------------------------------------------------------------------
// Task 20.51: functional callbacks at the Responses wire (P20-S51)
// ---------------------------------------------------------------------------

use opi_ai::callbacks::RequestCallbacks;

fn responses_text_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai-responses:gpt-4o".into(),
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

#[tokio::test]
async fn responses_payload_callback_sees_projected_input_body() {
    let sse = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hi\"}]}],\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}}\n\n",
    );
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let seen: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let provider = OpenAiResponsesProvider::new(Some(server.uri()));
    let mut request = responses_text_request();
    request.callbacks = RequestCallbacks::new().with_on_payload(std::sync::Arc::new(move |body| {
        recorder.lock().unwrap().push(body.clone());
        Ok(None)
    }));

    collect_stream(provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()))
        .await;

    let bodies = seen.lock().unwrap().clone();
    assert_eq!(bodies.len(), 1);
    assert!(
        bodies[0]["input"].is_array(),
        "payload callback sees the projected `input` field, not raw messages"
    );
    let sent = server.received_requests().await.unwrap()[0].clone();
    let wire_body: serde_json::Value =
        serde_json::from_slice(&sent.body).expect("captured body is JSON");
    assert_eq!(bodies[0], wire_body);
}

#[tokio::test]
async fn responses_async_response_and_raw_callbacks_follow_source_order() {
    let order: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let raw_recorder = order.clone();
    let response_recorder = order.clone();
    let sse = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hi\"}]}],\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}}\n\n",
    );
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = OpenAiResponsesProvider::new(Some(server.uri()));
    let mut request = responses_text_request();
    request.callbacks = RequestCallbacks::new()
        .with_async_header_transform(|headers| async move {
            tokio::task::yield_now().await;
            Ok(headers)
        })
        .with_async_on_payload(|_| async {
            tokio::task::yield_now().await;
            Ok(None)
        })
        .with_async_on_response(move |status, _headers| {
            let response_recorder = response_recorder.clone();
            async move {
                tokio::task::yield_now().await;
                response_recorder
                    .lock()
                    .unwrap()
                    .push(format!("response:{status}"));
                Ok(())
            }
        })
        .with_async_on_raw(move |event| {
            let raw_recorder = raw_recorder.clone();
            async move {
                tokio::task::yield_now().await;
                raw_recorder
                    .lock()
                    .unwrap()
                    .push(format!("raw:{}", event["type"].as_str().unwrap_or("?")));
                Ok(())
            }
        });

    collect_stream(provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()))
        .await;

    let order = order.lock().unwrap();
    assert_eq!(
        order.iter().map(String::as_str).collect::<Vec<_>>(),
        vec![
            "response:200",
            "raw:response.created",
            "raw:response.completed"
        ],
        "response first, then parsed wire events in stream order"
    );
}

// --- task 20.11: Responses constrained sampling (grammar + strict) ---

mod responses_constrained_tests {
    use super::*;
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    use opi_ai::openai_responses::ResponsesConfig;

    fn grammar_tool(lark: Option<&str>, regex: Option<&str>) -> ToolDef {
        ToolDef {
            name: "grammar-tool".into(),
            description: "grammar constrained".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"input": {"type": "string"}},
                "required": ["input"]
            }),
            constrained_sampling: Some(ConstrainedSampling::Grammar {
                openai_lark: lark.map(String::from),
                openai_regex: regex.map(String::from),
            }),
        }
    }

    fn json_schema_tool(strict: StrictPreference) -> ToolDef {
        ToolDef {
            name: "structured".into(),
            description: "d".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"path": {"type": "string"}, "limit": {"type": "number"}},
                "required": ["path"]
            }),
            constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict }),
        }
    }

    fn request_with(tools: Vec<ToolDef>) -> Request {
        let mut request = responses_tool_request();
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("You are helpful.".into()),
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text { text: "go".into() }],
                timestamp_ms: 0,
            })],
            tools,
        })
        .expect("valid context declarations");
        request
    }

    #[tokio::test]
    async fn models_complete_record_enables_explicit_grammar_support() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_string(responses_sse_ok()))
            .expect(1)
            .mount(&server)
            .await;
        let mut model = o3_responses_model();
        if let opi_ai::model_info::WireCompat::OpenAiResponses(compat) = &mut model.compat {
            compat.source.supports_grammar_tools = Some(true);
        } else {
            panic!("Responses compat");
        }
        // The registered default denies grammar; the complete selected record
        // opts in for this call, without changing the registered adapter.
        let (models, handle) =
            registered_responses_models(server.uri(), o3_responses_model()).await;
        let mut request = request_with(vec![grammar_tool(Some("start: WORD"), None)]);
        request.model = "openai-responses:o3".into();
        models
            .complete(
                ChatModelRecord::new("openai-responses", model),
                request,
                handle.options(OpenAiResponsesOptions::default()),
            )
            .await
            .unwrap();
        let sent = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
        assert_eq!(body["tools"][0]["type"], "custom");
        assert_eq!(body["tools"][0]["format"]["syntax"], "lark");
    }

    #[test]
    fn grammar_tool_projects_custom_tool_preferring_lark() {
        let provider = OpenAiResponsesProvider::new_with_config(
            None,
            ResponsesConfig {
                source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                    supports_grammar_tools: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let body = provider
            .build_request_body(&request_with(vec![grammar_tool(
                Some("start: WORD"),
                Some("^a+$"),
            )]))
            .unwrap();
        assert_eq!(body["tools"][0]["type"], serde_json::json!("custom"));
        assert_eq!(
            body["tools"][0]["format"],
            serde_json::json!({"type": "grammar", "syntax": "lark", "definition": "start: WORD"}),
            "lark is preferred over regex when both variants are provided"
        );
    }

    #[test]
    fn grammar_unsupported_target_projects_plain_function_with_recorded_loss() {
        let provider = responses_provider();
        let body = provider
            .build_request_body(&request_with(vec![grammar_tool(Some("start: WORD"), None)]))
            .unwrap();
        assert_eq!(body["tools"][0]["type"], serde_json::json!("function"));
        assert!(
            body["tools"][0].get("format").is_none(),
            "no grammar format is sent to an unsupported target"
        );
    }

    #[test]
    fn grammar_supported_target_without_variant_fails_before_dispatch() {
        let provider = OpenAiResponsesProvider::new_with_config(
            None,
            ResponsesConfig {
                source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                    supports_grammar_tools: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let error = provider
            .build_request_body(&request_with(vec![grammar_tool(None, Some("   "))]))
            .unwrap_err();
        assert!(
            matches!(error, ProviderError::Config(_)),
            "typed pre-dispatch failure, got {error:?}"
        );
    }

    #[test]
    fn strict_json_schema_resolves_per_tool_over_the_wire_default() {
        // Default strict (config strict_tools = false): prefer stays
        // non-strict; require fails typed because this wire lacks strict
        // support by default.
        let provider = responses_provider();
        let body = provider
            .build_request_body(&request_with(vec![json_schema_tool(
                StrictPreference::Prefer,
            )]))
            .unwrap();
        assert_eq!(body["tools"][0]["type"], serde_json::json!("function"));
        assert!(
            body["tools"][0].get("strict").is_none(),
            "no strict flag without strict support"
        );
        let error = provider
            .build_request_body(&request_with(vec![json_schema_tool(
                StrictPreference::Require,
            )]))
            .unwrap_err();
        assert!(matches!(error, ProviderError::Config(_)));

        // With strict support: prefer resolves strict with the converted
        // schema (required-all + additionalProperties false).
        let provider = OpenAiResponsesProvider::new_with_config(
            None,
            ResponsesConfig {
                source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                    supports_strict_mode: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let body = provider
            .build_request_body(&request_with(vec![json_schema_tool(
                StrictPreference::Prefer,
            )]))
            .unwrap();
        assert_eq!(body["tools"][0]["strict"], serde_json::json!(true));
        assert_eq!(
            body["tools"][0]["parameters"]["additionalProperties"],
            serde_json::json!(false)
        );
        assert_eq!(
            body["tools"][0]["parameters"]["properties"]["path"],
            serde_json::json!({"type": "string"}),
            "required properties stay unchanged"
        );
        assert_eq!(
            body["tools"][0]["parameters"]["properties"]["limit"],
            serde_json::json!({"anyOf": [{"type": "number"}, {"type": "null"}]}),
            "optional properties wrap nullable"
        );
    }

    #[test]
    fn malformed_required_is_typed_and_preserves_the_trusted_schema() {
        let provider = OpenAiResponsesProvider::new_with_config(
            None,
            ResponsesConfig {
                source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                    supports_strict_mode: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        for required in [
            serde_json::Value::Null,
            serde_json::json!("path"),
            serde_json::json!(1),
            serde_json::json!(false),
            serde_json::json!({"path": true}),
            serde_json::json!([1]),
            serde_json::json!(["unknown"]),
        ] {
            for properties in [None, Some(serde_json::json!({"path": {"type": "string"}}))] {
                let mut tool = json_schema_tool(StrictPreference::Require);
                tool.input_schema = serde_json::json!({"type": "object", "required": required});
                if let Some(properties) = properties {
                    tool.input_schema["properties"] = properties;
                }
                let request = request_with(vec![tool]);
                let canonical = request.context.clone();
                let error = provider.build_request_body(&request).unwrap_err();
                assert_eq!(request.context, canonical);
                let ProviderError::Config(summary) = error else {
                    panic!("required admission must fail typed: {error:?}")
                };
                assert!(summary.as_str().contains("required"));
            }
        }
    }

    #[tokio::test]
    async fn malformed_required_fails_before_native_authentication_and_dispatch() {
        use opi_ai::model_info::{
            ModelCapabilities, ModelInfo, OpenAiResponsesCompat, OpenAiResponsesSourceCompat,
            WireApi, WireCompat,
        };
        use opi_ai::models::{ChatModelRecord, Models};
        use opi_ai::openai_responses::OpenAiResponsesChatApi;
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };

        struct CountingAuth(Arc<AtomicUsize>);
        impl opi_ai::auth::AuthResolver for CountingAuth {
            fn resolve(
                &self,
            ) -> opi_ai::BoxAuthFuture<'_, Result<opi_ai::auth::ResolvedAuth, ProviderError>>
            {
                self.0.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Ok(opi_ai::test_support::resolved_bearer_auth()) })
            }
        }
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(responses_sse_ok()),
            )
            .mount(&server)
            .await;
        let model = ModelInfo::new(
            "required-probe",
            "Required probe",
            WireApi::OpenAiResponses,
            ModelCapabilities::new(100_000, 4_096),
        )
        .with_base_url(server.uri())
        .with_compat(WireCompat::OpenAiResponses(OpenAiResponsesCompat {
            source: OpenAiResponsesSourceCompat {
                supports_strict_mode: Some(true),
                ..Default::default()
            },
            ..Default::default()
        }))
        .unwrap();
        let api = OpenAiResponsesChatApi::new(
            Arc::new(OpenAiResponsesProvider::new(Some(server.uri()))),
            model.clone(),
        )
        .unwrap();
        let mut models = Models::new();
        let count = Arc::new(AtomicUsize::new(0));
        let handle = models
            .register_chat_api(
                Arc::new(api),
                Arc::new(CountingAuth(count.clone())),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        let record = ChatModelRecord::new("openai-responses", model);
        models
            .complete(
                record.clone(),
                request_with(vec![json_schema_tool(StrictPreference::Require)]),
                handle.options(Default::default()),
            )
            .await
            .expect("valid strict control must authenticate and dispatch");
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
        for required in [
            serde_json::Value::Null,
            serde_json::json!("path"),
            serde_json::json!(1),
        ] {
            for properties in [None, Some(serde_json::json!({"path": {"type": "string"}}))] {
                let mut tool = json_schema_tool(StrictPreference::Require);
                tool.input_schema = serde_json::json!({"type": "object", "required": required});
                if let Some(properties) = properties {
                    tool.input_schema["properties"] = properties;
                }
                let failure = models
                    .complete(
                        record.clone(),
                        request_with(vec![tool]),
                        handle.options(Default::default()),
                    )
                    .await
                    .expect_err("malformed required must be rejected before auth");
                let cause = match failure.cause() {
                    ProviderError::StreamFailure(frame) => frame.cause(),
                    cause => cause,
                };
                let ProviderError::Config(summary) = cause else {
                    panic!("typed required failure: {cause:?}")
                };
                assert!(summary.as_str().contains("required"));
                assert_eq!(count.load(Ordering::SeqCst), 1);
                assert_eq!(server.received_requests().await.unwrap().len(), 1);
            }
        }
    }

    fn custom_tool_call_sse() -> String {
        [
            r#"data: {"type":"response.output_item.added","output_index":1,"item":{"type":"custom_tool_call","id":"ctc_1","call_id":"call_1","name":"grammar-tool","input":""}}"#,
            "",
            r#"data: {"type":"response.custom_tool_call_input.delta","output_index":1,"delta":"hel"}"#,
            "",
            r#"data: {"type":"response.custom_tool_call_input.delta","output_index":1,"delta":"lo"}"#,
            "",
            r#"data: {"type":"response.custom_tool_call_input.done","output_index":1,"input":"hello"}"#,
            "",
            r#"data: {"type":"response.output_item.done","output_index":1,"item":{"type":"custom_tool_call","id":"ctc_1","call_id":"call_1","name":"grammar-tool","input":"hello"}}"#,
            "",
            r#"data: {"type":"response.completed","response":{"id":"resp_1","model":"gpt-4o","usage":{"input_tokens":3,"output_tokens":2}}}"#,
            "",
            "",
        ]
        .join("\n")
    }

    #[tokio::test]
    async fn custom_tool_call_input_wraps_incrementally_and_finalizes_raw() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(custom_tool_call_sse()),
            )
            .mount(&server)
            .await;
        let provider = OpenAiResponsesProvider::new_with_config(
            Some(server.uri()),
            ResponsesConfig {
                source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                    supports_grammar_tools: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let events: Vec<_> = provider
            .stream_prepared(
                request_with(vec![grammar_tool(Some("start: WORD"), None)]),
                opi_ai::test_support::resolved_bearer_auth(),
            )
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .map(|event| event.expect("valid custom input deltas must not emit a failure"))
            .collect();

        // The streaming deltas carry the wrapped JSON object text.
        let deltas: Vec<&str> = events
            .iter()
            .filter_map(|event| match event {
                AssistantStreamEvent::ToolCallDelta { delta, .. } => Some(delta.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            deltas.join(""),
            r#"{"input":"hello"}"#,
            "the raw wire input wraps into the incremental JSON object text"
        );
        // The final tool call keeps the raw input under its property.
        let final_call = events
            .iter()
            .find_map(|event| match event {
                AssistantStreamEvent::ToolCallEnd { tool_call, .. } => Some(tool_call.clone()),
                _ => None,
            })
            .expect("custom tool call finalizes");
        assert_eq!(final_call.name, "grammar-tool");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&final_call.arguments).unwrap(),
            serde_json::json!({"input": "hello"})
        );
        assert!(final_call.arguments_error.is_none());
    }

    #[tokio::test]
    async fn custom_tool_initial_input_is_included_in_the_first_wrapped_delta() {
        let server = MockServer::start().await;
        let sse = custom_tool_call_sse()
            .replace(r#""input":"""#, r#""input":"he""#)
            .replace(r#""delta":"hel""#, r#""delta":"l""#);
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(sse),
            )
            .mount(&server)
            .await;
        let provider = OpenAiResponsesProvider::new_with_config(
            Some(server.uri()),
            ResponsesConfig {
                source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                    supports_grammar_tools: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let events: Vec<_> = provider
            .stream_prepared(
                request_with(vec![grammar_tool(Some("start: WORD"), None)]),
                opi_ai::test_support::resolved_bearer_auth(),
            )
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .map(|event| event.expect("initial input plus wire increments is valid"))
            .collect();
        let delta = events
            .iter()
            .filter_map(|event| match event {
                AssistantStreamEvent::ToolCallDelta { delta, .. } => Some(delta.as_str()),
                _ => None,
            })
            .collect::<String>();
        assert_eq!(delta, r#"{"input":"hello"}"#);
    }

    #[tokio::test]
    async fn custom_tool_terminal_item_preserves_input_without_input_done() {
        for added in [true, false] {
            let server = MockServer::start().await;
            let mut wire = Vec::new();
            if added {
                wire.push(serde_json::json!({
                    "type": "response.output_item.added", "output_index": 0,
                    "item": {"type": "custom_tool_call", "id": "ctc_terminal",
                        "call_id": "call_terminal", "name": "grammar-tool", "input": ""}
                }));
            }
            wire.push(serde_json::json!({
                "type": "response.output_item.done", "output_index": 0,
                "item": {"type": "custom_tool_call", "id": "ctc_terminal",
                    "call_id": "call_terminal", "name": "grammar-tool", "input": "hello"}
            }));
            wire.push(serde_json::json!({
                "type": "response.completed", "response": {"id": "resp_terminal",
                    "model": "gpt-4o", "output": []}
            }));
            let sse = wire
                .into_iter()
                .map(|value| format!("data: {value}\n\n"))
                .collect::<String>();
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(sse),
                )
                .mount(&server)
                .await;
            let provider = OpenAiResponsesProvider::new_with_config(
                Some(server.uri()),
                ResponsesConfig {
                    source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                        supports_grammar_tools: Some(true),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );
            let message = opi_ai::stream::ChatStream::new(provider.stream_prepared(
                request_with(vec![grammar_tool(Some("start: WORD"), None)]),
                opi_ai::test_support::resolved_bearer_auth(),
            ))
            .result()
            .await
            .expect("terminal custom input is a complete valid call");
            let tool = message
                .content
                .iter()
                .find_map(|content| match content {
                    opi_ai::message::AssistantContent::ToolCall { tool_call } => Some(tool_call),
                    _ => None,
                })
                .expect("terminal item creates and finishes its tool slot");
            assert_eq!(tool.id, "call_terminal|ctc_terminal");
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&tool.arguments).unwrap(),
                serde_json::json!({"input": "hello"}),
                "terminal input is authoritative whether the added event was present"
            );
            assert!(tool.arguments_error.is_none());
        }
    }

    #[tokio::test]
    async fn custom_tool_call_closed_input_violation_is_observable() {
        let server = MockServer::start().await;
        // The done event's input contradicts the accumulated deltas.
        let broken = custom_tool_call_sse().replace(
            r#""response.custom_tool_call_input.done","output_index":1,"input":"hello""#,
            r#""response.custom_tool_call_input.done","output_index":1,"input":"hexlo""#,
        );
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(broken),
            )
            .mount(&server)
            .await;
        let provider = OpenAiResponsesProvider::new_with_config(
            Some(server.uri()),
            ResponsesConfig {
                source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                    supports_grammar_tools: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        use futures_util::StreamExt;
        let results = provider
            .stream_prepared(
                request_with(vec![grammar_tool(Some("start: WORD"), None)]),
                opi_ai::test_support::resolved_bearer_auth(),
            )
            .collect::<Vec<_>>()
            .await;
        let failure_index = results.iter().position(Result::is_err).expect("failure");
        assert!(
            results[failure_index + 1..].is_empty(),
            "a grammar failure seals the mapper instead of later finalizing successfully"
        );
        let failure = results
            .into_iter()
            .find_map(Result::err)
            .expect("the non-monotonic close fails typed");
        let is_typed_stream_failure = match &failure {
            ProviderError::StreamError(_) => true,
            ProviderError::StreamFailure(boxed) => {
                matches!(boxed.cause(), ProviderError::StreamError(_))
            }
            _ => false,
        };
        assert!(
            is_typed_stream_failure,
            "closed-input violation surfaces as a typed stream failure, got {failure:?}"
        );
    }

    #[tokio::test]
    async fn custom_tool_completed_input_cannot_rewrite_the_streamed_prefix() {
        let server = MockServer::start().await;
        let wire = [
            serde_json::json!({
                "type": "response.output_item.added", "output_index": 0,
                "item": {"type": "custom_tool_call", "id": "ctc_terminal",
                    "call_id": "call_terminal", "name": "grammar-tool", "input": ""}
            }),
            serde_json::json!({
                "type": "response.custom_tool_call_input.delta", "output_index": 0,
                "delta": "hel"
            }),
            serde_json::json!({
                "type": "response.completed", "response": {"id": "resp_terminal",
                    "model": "gpt-4o", "output": [{"type": "custom_tool_call",
                        "id": "ctc_terminal", "call_id": "call_terminal",
                        "name": "grammar-tool", "input": "hexlo"}]}
            }),
        ];
        let sse = wire
            .into_iter()
            .map(|value| format!("data: {value}\n\n"))
            .collect::<String>();
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(sse),
            )
            .mount(&server)
            .await;
        let provider = OpenAiResponsesProvider::new_with_config(
            Some(server.uri()),
            ResponsesConfig {
                source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                    supports_grammar_tools: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let results = provider
            .stream_prepared(
                request_with(vec![grammar_tool(Some("start: WORD"), None)]),
                opi_ai::test_support::resolved_bearer_auth(),
            )
            .collect::<Vec<_>>()
            .await;
        assert!(results.last().is_some_and(Result::is_err));
        assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
        assert!(
            !results
                .iter()
                .any(|result| matches!(result, Ok(AssistantStreamEvent::Done { .. })))
        );
    }
}

// ---------------------------------------------------------------------------
// Task 20.37 (P20-S37 / ONB-03-20.37): typed native options, encrypted
// reasoning/replay, service tier, cache retention, mid-conversation effort
// stamps, duration, and the public Models entries.
// ---------------------------------------------------------------------------

use opi_ai::models::{ChatModelRecord, Models};
use opi_ai::openai_responses::{
    OpenAiResponsesChatApi, OpenAiResponsesOptions, ResponsesReasoningSummary,
    ResponsesServiceTier, ResponsesToolChoice,
};
use opi_ai::{ModelCapabilities, ModelInfo, ThinkingLevel, WireApi};

fn o3_responses_model() -> ModelInfo {
    ModelInfo::new(
        "o3",
        "o3",
        WireApi::OpenAiResponses,
        ModelCapabilities::new(200_000, 100_000)
            .with_images(true)
            .with_streaming(true)
            .with_thinking(true),
    )
}

fn responses_public_request(model: &str, thinking: Option<ThinkingLevel>) -> Request {
    let mut request = responses_tool_request();
    request.model = format!("openai-responses:{model}");
    request.thinking = match thinking {
        Some(level) => ThinkingConfig {
            enabled: true,
            budget_tokens: None,
            level,
        },
        None => ThinkingConfig::default(),
    };
    request
}

async fn registered_responses_models(
    server_uri: String,
    model: ModelInfo,
) -> (
    Models,
    opi_ai::models::ChatApiHandle<OpenAiResponsesOptions>,
) {
    let provider = std::sync::Arc::new(OpenAiResponsesProvider::new(Some(server_uri)));
    let api = OpenAiResponsesChatApi::new(provider, model).expect("valid responses record");
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::Bearer,
                secrecy::SecretString::from("offline-responses-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register responses api");
    (models, handle)
}

fn responses_record() -> ChatModelRecord {
    ChatModelRecord::new("openai-responses", o3_responses_model())
}

fn responses_sse_ok() -> String {
    concat!(
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"status\":\"in_progress\",\"model\":\"o3\",\"output\":[]}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"o3\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hi\"}]}],\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}}\n\n",
    )
    .to_owned()
}

#[tokio::test]
async fn responses_request_body_store_false_by_default() {
    let provider = responses_provider();
    let body = provider
        .build_request_body(&responses_tool_request())
        .unwrap();
    assert_eq!(
        body["store"], false,
        "source semantics: store:false is sent unconditionally by default"
    );
}

#[test]
fn responses_disabled_and_off_thinking_send_none_effort_for_reasoning_models() {
    let provider = responses_provider();
    let mut disabled = responses_tool_request();
    disabled.model = "openai-responses:o3".into();
    assert_eq!(
        provider.build_request_body(&disabled).unwrap()["reasoning"],
        serde_json::json!({"effort": "none"}),
        "reasoning models send the off mapping (source: off ?? none)"
    );

    let mut off = disabled.clone();
    off.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: None,
        level: ThinkingLevel::None,
    };
    assert_eq!(
        provider.build_request_body(&off).unwrap()["reasoning"],
        serde_json::json!({"effort": "none"})
    );
}

#[test]
fn responses_copilot_route_skips_off_effort() {
    use std::sync::Arc as StdArc;

    use opi_ai::http::HttpClient;

    let mut provider = OpenAiResponsesProvider::with_auth_extra(
        None,
        ResponsesConfig::default(),
        "github-copilot".into(),
        Vec::new(),
        StdArc::new(HttpClient::new()),
    );
    provider
        .replace_model_catalog(vec![o3_responses_model()])
        .expect("replace catalog");
    let mut request = responses_tool_request();
    request.model = "github-copilot:o3".into();
    assert!(
        provider
            .build_request_body(&request)
            .unwrap()
            .get("reasoning")
            .is_none(),
        "Sign-in-with-ChatGPT/copilot routes skip the off effort (source)"
    );
}

#[test]
fn responses_xai_route_always_includes_encrypted_content() {
    use std::sync::Arc as StdArc;

    use opi_ai::http::HttpClient;

    let mut provider = OpenAiResponsesProvider::with_auth_extra(
        None,
        ResponsesConfig::default(),
        "xai".into(),
        Vec::new(),
        StdArc::new(HttpClient::new()),
    );
    provider
        .replace_model_catalog(vec![o3_responses_model()])
        .expect("replace catalog");
    let mut request = responses_tool_request();
    request.model = "xai:o3".into();
    let body = provider.build_request_body(&request).unwrap();
    assert_eq!(
        body["include"],
        serde_json::json!(["reasoning.encrypted_content"]),
        "xai reasoning models always request encrypted reasoning"
    );
}

#[test]
fn responses_off_effort_omits_summary_and_include() {
    let provider = responses_provider();
    let mut request = responses_tool_request();
    request.model = "openai-responses:o3".into();
    let body = provider.build_request_body(&request).unwrap();
    assert!(
        body["reasoning"].get("summary").is_none(),
        "the off branch sends effort only (source)"
    );
    assert!(
        body.get("include").is_none(),
        "encrypted reasoning is requested only when an effort runs"
    );
}

#[tokio::test]
async fn models_native_options_reach_the_responses_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (models, handle) = registered_responses_models(server.uri(), o3_responses_model()).await;
    let mut value = OpenAiResponsesOptions::default();
    value.tool_choice = Some(ResponsesToolChoice::Named("list_dir".into()));
    value.reasoning_effort = Some(ThinkingLevel::High);
    value.reasoning_summary = Some(ResponsesReasoningSummary::Detailed);
    value.service_tier = Some(ResponsesServiceTier::Flex);
    let options = handle.options(value);

    let request = responses_public_request("o3", Some(ThinkingLevel::Minimal));
    let message = models
        .complete(responses_record(), request, options)
        .await
        .expect("native responses call completes");
    assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Stop);

    let sent = &server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
    assert_eq!(
        body["tool_choice"],
        serde_json::json!({"type": "function", "name": "list_dir"})
    );
    assert_eq!(
        body["reasoning"],
        serde_json::json!({"effort": "high", "summary": "detailed"})
    );
    assert_eq!(
        body["include"],
        serde_json::json!(["reasoning.encrypted_content"])
    );
    assert_eq!(body["service_tier"], serde_json::json!("flex"));
    assert_eq!(body["store"], serde_json::json!(false));
    server.verify().await;
}

#[tokio::test]
async fn native_sampling_uses_native_effort_summary_default_and_request_overrides() {
    for (effort, summary, request_temperature, expected, wire_effort) in [
        (Some(ThinkingLevel::High), None, None, 0.9, "high"),
        (
            None,
            Some(ResponsesReasoningSummary::Detailed),
            None,
            0.6,
            "medium",
        ),
        (None, None, None, 0.2, "none"),
        (Some(ThinkingLevel::High), None, Some(0.4), 0.4, "high"),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_string(responses_sse_ok()))
            .expect(1)
            .mount(&server)
            .await;
        let model = o3_responses_model()
            .with_sampling_params(
                [("temperature".into(), serde_json::json!(0.1))]
                    .into_iter()
                    .collect(),
            )
            .with_sampling_level_params(
                ThinkingLevel::None,
                [("temperature".into(), serde_json::json!(0.2))]
                    .into_iter()
                    .collect(),
            )
            .with_sampling_level_params(
                ThinkingLevel::Medium,
                [("temperature".into(), serde_json::json!(0.6))]
                    .into_iter()
                    .collect(),
            )
            .with_sampling_level_params(
                ThinkingLevel::High,
                [("temperature".into(), serde_json::json!(0.9))]
                    .into_iter()
                    .collect(),
            );
        let (models, handle) = registered_responses_models(server.uri(), model.clone()).await;
        let mut request = responses_public_request("o3", Some(ThinkingLevel::Low));
        if let Some(value) = request_temperature {
            request
                .sampling_params
                .insert("temperature".into(), serde_json::json!(value));
        }
        let mut options = OpenAiResponsesOptions::default();
        options.reasoning_effort = effort;
        options.reasoning_summary = summary;
        models
            .complete(
                ChatModelRecord::new("openai-responses", model),
                request,
                handle.options(options),
            )
            .await
            .expect("native call completes");
        let sent = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
        assert_eq!(body["reasoning"]["effort"], wire_effort);
        assert_eq!(body["temperature"], serde_json::json!(expected));
        server.verify().await;
    }
}

#[tokio::test]
async fn models_native_unsupported_effort_fails_typed_before_auth() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(0)
        .mount(&server)
        .await;

    let (models, handle) = registered_responses_models(server.uri(), o3_responses_model()).await;
    let mut value = OpenAiResponsesOptions::default();
    value.reasoning_effort = Some(ThinkingLevel::XHigh);
    let options = handle.options(value);
    let failure = models
        .complete(
            responses_record(),
            responses_public_request("o3", None),
            options,
        )
        .await
        .expect_err("unsupported native effort fails typed preflight");
    assert!(
        matches!(failure.cause(), opi_ai::provider::ProviderError::Config(_)),
        "typed preflight failure, got {:?}",
        failure.cause()
    );
    server.verify().await;
}

#[tokio::test]
async fn models_simple_neutral_effort_reaches_responses_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let mut models = Models::new();
    let provider = std::sync::Arc::new(OpenAiResponsesProvider::new(Some(server.uri())));
    let api = OpenAiResponsesChatApi::new(provider, o3_responses_model()).expect("record");
    models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::Bearer,
                secrecy::SecretString::from("offline-responses-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register");

    let request = responses_public_request("o3", Some(ThinkingLevel::XHigh));
    let message = models
        .complete_simple(responses_record(), request, Default::default())
        .await
        .expect("simple responses call completes");
    assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Stop);

    let sent = &server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
    assert_eq!(
        body["reasoning"],
        serde_json::json!({"effort": "high", "summary": "auto"}),
        "neutral entry clamps to the nearest supported level and maps summary default auto"
    );
    assert_eq!(
        body["include"],
        serde_json::json!(["reasoning.encrypted_content"])
    );
    server.verify().await;
}

#[tokio::test]
async fn models_direct_adapter_uses_injected_auth_and_returns_result() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(header("authorization", "Bearer injected-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (models, handle) = registered_responses_models(server.uri(), o3_responses_model()).await;
    let options = handle.options(OpenAiResponsesOptions::default());
    let message = models
        .complete_direct(
            responses_record(),
            responses_public_request("o3", None),
            options,
            opi_ai::auth::ResolvedAuth {
                env: Default::default(),
                scheme: opi_ai::auth::AuthScheme::Bearer,
                secret: secrecy::SecretString::from("injected-key"),
                base_url: None,
                account_id: None,
                provenance: Default::default(),
            },
        )
        .await
        .expect("direct responses call completes");
    assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Stop);
    server.verify().await;
}

#[tokio::test]
async fn models_direct_adapter_rejects_wrong_wire_records() {
    let chat_model = ModelInfo::new(
        "o3",
        "o3",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 100_000).with_streaming(true),
    );
    let provider = std::sync::Arc::new(OpenAiResponsesProvider::new(None));
    let failure = OpenAiResponsesChatApi::new(provider, chat_model)
        .expect_err("wrong-wire records fail before authentication");
    assert!(
        matches!(failure, opi_ai::provider::ProviderError::Config(_)),
        "typed rejection, got {failure:?}"
    );
}

#[tokio::test]
async fn responses_long_cache_retention_sends_prompt_cache_retention() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (models, handle) = registered_responses_models(server.uri(), o3_responses_model()).await;
    let mut request = responses_public_request("o3", None);
    request.cache_retention = CacheRetention::Long;
    request.session_id = Some("sess-123".into());
    let options = handle.options(OpenAiResponsesOptions::default());
    models
        .complete(responses_record(), request, options)
        .await
        .expect("long retention call completes");

    let sent = &server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
    assert_eq!(body["prompt_cache_key"], serde_json::json!("sess-123"));
    assert_eq!(
        body["prompt_cache_retention"],
        serde_json::json!("24h"),
        "long retention maps to the source 24h retention window"
    );
    server.verify().await;
}

fn reasoning_sse() -> String {
    concat!(
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_r\",\"status\":\"in_progress\",\"model\":\"o3\",\"output\":[]}}\n\n",
        "data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"reasoning\",\"id\":\"rs_1\"}}\n\n",
        "data: {\"type\":\"response.reasoning_summary_text.delta\",\"output_index\":0,\"summary_index\":0,\"delta\":\"Think\"}\n\n",
        "data: {\"type\":\"response.reasoning_summary_text.delta\",\"output_index\":0,\"summary_index\":0,\"delta\":\"ing\"}\n\n",
        "data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"reasoning\",\"id\":\"rs_1\",\"summary\":[{\"type\":\"summary_text\",\"text\":\"Thinking\"}]}}\n\n",
        "data: {\"type\":\"response.output_item.added\",\"output_index\":1,\"item\":{\"type\":\"message\",\"status\":\"in_progress\",\"role\":\"assistant\",\"content\":[]}}\n\n",
        "data: {\"type\":\"response.output_text.delta\",\"output_index\":1,\"content_index\":0,\"delta\":\"Answer\"}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_r\",\"status\":\"completed\",\"model\":\"o3\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"Answer\"}]}],\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}}\n\n",
    )
    .to_owned()
}

#[tokio::test]
async fn responses_reasoning_summary_deltas_stream_thinking_blocks() {
    let provider = responses_provider();
    let events: Vec<AssistantStreamEvent> = provider
        .stream_from_sse(&reasoning_sse(), CancellationToken::new())
        .filter_map(|event| async move { event.ok() })
        .collect()
        .await;
    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::ThinkingStart { .. }))
    );
    assert!(events
        .iter()
        .any(|event| matches!(event, AssistantStreamEvent::ThinkingDelta { delta, .. } if delta == "Think")));
    let done = events
        .iter()
        .find_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => Some(message.clone()),
            _ => None,
        })
        .expect("stream completes");
    let thinking = message_thinking(&done).expect("thinking block persisted");
    assert_eq!(thinking, "Thinking");
}

#[tokio::test]
async fn responses_reasoning_signature_persists_encrypted_content() {
    let sse = concat!(
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_e\",\"status\":\"in_progress\",\"model\":\"o3\",\"output\":[]}}\n\n",
        "data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"reasoning\",\"id\":\"rs_9\"}}\n\n",
        "data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"reasoning\",\"id\":\"rs_9\",\"summary\":[{\"type\":\"summary_text\",\"text\":\"Plan\"}],\"encrypted_content\":\"ENC\"}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_e\",\"status\":\"completed\",\"model\":\"o3\",\"output\":[],\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}}\n\n",
    );
    let provider = responses_provider();
    let events: Vec<AssistantStreamEvent> = provider
        .stream_from_sse(sse, CancellationToken::new())
        .filter_map(|event| async move { event.ok() })
        .collect()
        .await;
    let done = events
        .iter()
        .find_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => Some(message.clone()),
            _ => None,
        })
        .expect("stream completes");
    let signature = done
        .content
        .iter()
        .find_map(|content| match content {
            opi_ai::message::AssistantContent::Thinking { signature, .. } => signature.clone(),
            _ => None,
        })
        .expect("reasoning item persisted a signature");
    let item: serde_json::Value = serde_json::from_str(&signature).expect("signature is JSON");
    assert_eq!(item["id"], serde_json::json!("rs_9"));
    assert_eq!(item["encrypted_content"], serde_json::json!("ENC"));
}

#[tokio::test]
async fn responses_completed_backfills_encrypted_signature_from_terminal_output() {
    let sse = concat!(
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_b\",\"status\":\"in_progress\",\"model\":\"o3\",\"output\":[]}}\n\n",
        "data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"reasoning\",\"id\":\"rs_b\"}}\n\n",
        "data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"reasoning\",\"id\":\"rs_b\",\"summary\":[{\"type\":\"summary_text\",\"text\":\"Ponder\"}]}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_b\",\"status\":\"completed\",\"model\":\"o3\",\"output\":[{\"type\":\"reasoning\",\"id\":\"rs_b\",\"summary\":[{\"type\":\"summary_text\",\"text\":\"Ponder\"}],\"encrypted_content\":\"BACKFILL\"}],\"usage\":{\"input_tokens\":5,\"output_tokens\":2}}}\n\n",
    );
    let provider = responses_provider();
    let events: Vec<AssistantStreamEvent> = provider
        .stream_from_sse(sse, CancellationToken::new())
        .filter_map(|event| async move { event.ok() })
        .collect()
        .await;
    let done = events
        .iter()
        .find_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => Some(message.clone()),
            _ => None,
        })
        .expect("stream completes");
    let signature = done
        .content
        .iter()
        .find_map(|content| match content {
            opi_ai::message::AssistantContent::Thinking { signature, .. } => signature.clone(),
            _ => None,
        })
        .expect("reasoning signature persisted");
    let item: serde_json::Value = serde_json::from_str(&signature).expect("signature is JSON");
    assert_eq!(
        item["encrypted_content"],
        serde_json::json!("BACKFILL"),
        "terminal response output backfills the omitted encrypted content"
    );
}

fn message_thinking(message: &opi_ai::message::AssistantMessage) -> Option<String> {
    message.content.iter().find_map(|content| match content {
        opi_ai::message::AssistantContent::Thinking { thinking, .. } => Some(thinking.clone()),
        _ => None,
    })
}

#[test]
fn responses_encrypted_reasoning_replay_pushes_signed_items_back_into_input() {
    let provider = responses_provider();
    let mut request = responses_tool_request();
    request.model = "openai-responses:o3".into();
    let mut context = responses_tool_context();
    context.messages.insert(
        0,
        Message::Assistant(history_assistant_message(
            vec![
                opi_ai::message::AssistantContent::Thinking {
                    thinking: "Plan".into(),
                    signature: Some(
                        r#"{"id":"rs_old","type":"reasoning","encrypted_content":"OLD"}"#.into(),
                    ),
                },
                opi_ai::message::AssistantContent::Text {
                    text: "Earlier answer".into(),
                    signature: None,
                },
            ],
            None,
        )),
    );
    request.context = opi_ai::transcript::normalize_context(context).expect("valid context");
    let body = provider.build_request_body(&request).unwrap();
    let input = &body["input"].as_array().unwrap();
    let reasoning = input
        .iter()
        .find(|item| item.get("type").and_then(|t| t.as_str()) == Some("reasoning"))
        .expect("signed reasoning item replays into the input list");
    assert_eq!(reasoning["id"], serde_json::json!("rs_old"));
    assert_eq!(reasoning["encrypted_content"], serde_json::json!("OLD"));
}

fn history_assistant_message(
    content: Vec<opi_ai::message::AssistantContent>,
    provider_thinking_level: Option<String>,
) -> opi_ai::message::AssistantMessage {
    opi_ai::message::AssistantMessage {
        projection: None,
        content,
        api: opi_ai::ApiKind::OpenAi,
        provider: "openai-responses".into(),
        deferred: None,
        model: "o3".into(),
        response_model: None,
        wire: Some(WireApi::OpenAiResponses),
        response_id: None,
        usage: Default::default(),
        stop_reason: opi_ai::stream::StopReason::Stop,
        error_message: None,
        timestamp_ms: 0,
        diagnostics: Vec::new(),
        duration_ms: None,
        raw_stop_reason: None,
        provider_thinking_level,
    }
}

#[tokio::test]
async fn responses_mid_convo_effort_stamps_configuration_updates() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    use opi_ai::http::HttpClient;
    use opi_ai::model_info::{OpenAiResponsesCompat, WireCompat};
    let model = o3_responses_model()
        .with_compat(WireCompat::OpenAiResponses(OpenAiResponsesCompat {
            source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                supports_mid_convo_effort: Some(true),
                ..Default::default()
            },
            ..Default::default()
        }))
        .expect("compat");
    let provider = std::sync::Arc::new(OpenAiResponsesProvider::for_route(
        Some(server.uri()),
        "openai-responses".into(),
        Default::default(),
        vec![model.clone()],
        std::sync::Arc::new(HttpClient::new()),
    ));
    let api = OpenAiResponsesChatApi::new(provider, model.clone()).expect("record");
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::Bearer,
                secrecy::SecretString::from("offline-responses-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register");

    let mut request = responses_public_request("o3", None);
    let mut context = responses_tool_context();
    context.messages.insert(
        0,
        Message::Assistant(history_assistant_message(
            vec![opi_ai::message::AssistantContent::Text {
                text: "Earlier".into(),
                signature: None,
            }],
            Some("low".into()),
        )),
    );
    request.context = opi_ai::transcript::normalize_context(context).expect("valid context");
    let mid_convo_record = ChatModelRecord::new("openai-responses", model.clone());
    let mut value = OpenAiResponsesOptions::default();
    value.reasoning_effort = Some(ThinkingLevel::High);
    let options = handle.options(value);
    let message = models
        .complete(mid_convo_record, request, options)
        .await
        .expect("mid-convo effort call completes");

    let sent = &server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
    let input = body["input"].as_array().unwrap();
    let updates: Vec<&serde_json::Value> = input
        .iter()
        .filter(|item| item.get("type").and_then(|t| t.as_str()) == Some("configuration_update"))
        .collect();
    assert_eq!(
        updates.len(),
        2,
        "one stamp before the earlier same-model reply, one at the end: {input:?}"
    );
    assert_eq!(updates[0]["reasoning"]["effort"], serde_json::json!("low"));
    assert_eq!(updates[1]["reasoning"]["effort"], serde_json::json!("high"));
    assert_eq!(
        body["reasoning"]["effort"],
        serde_json::json!("medium"),
        "the request effort stays fixed while configuration items carry the level"
    );
    assert_eq!(
        message.provider_thinking_level.as_deref(),
        Some("high"),
        "the reply records its provider thinking level for the next stamp"
    );
    server.verify().await;
}

#[tokio::test]
async fn responses_done_message_carries_measured_response_duration() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (models, handle) = registered_responses_models(server.uri(), o3_responses_model()).await;
    let options = handle.options(OpenAiResponsesOptions::default());
    let message = models
        .complete(
            responses_record(),
            responses_public_request("o3", None),
            options,
        )
        .await
        .expect("duration call completes");
    assert!(
        message.duration_ms.is_some(),
        "terminal messages carry the measured response duration"
    );
}

#[tokio::test]
async fn responses_setup_failure_before_activation_has_no_duration() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenAiResponsesProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        responses_public_request("o3", None),
        opi_ai::test_support::resolved_bearer_auth(),
    );
    let mut saw_done = false;
    while let Some(item) = stream.next().await {
        match item {
            Ok(AssistantStreamEvent::Done { .. }) => saw_done = true,
            Ok(_) => {}
            Err(cause) => {
                assert!(
                    !cause.is_retryable(),
                    "unauthorized is a terminal setup failure: {cause}"
                );
            }
        }
    }
    assert!(!saw_done, "setup failure never reaches a terminal Done");
    server.verify().await;
}

#[tokio::test]
async fn native_known_invalid_input_and_endpoint_fail_before_authentication() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct CountingAuth(Arc<AtomicUsize>);
    impl opi_ai::auth::AuthResolver for CountingAuth {
        fn resolve(
            &self,
        ) -> opi_ai::credential::BoxAuthFuture<'_, Result<opi_ai::auth::ResolvedAuth, ProviderError>>
        {
            Box::pin(async move {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(opi_ai::auth::ResolvedAuth {
                    env: Default::default(),
                    scheme: opi_ai::auth::AuthScheme::Bearer,
                    secret: "fixture-key".into(),
                    base_url: None,
                    account_id: None,
                    provenance: Default::default(),
                })
            })
        }
    }
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(responses_sse_ok()),
        )
        .expect(1)
        .mount(&server)
        .await;
    let model = ModelInfo::new(
        "probe",
        "Probe",
        WireApi::OpenAiResponses,
        ModelCapabilities::new(100000, 1000),
    )
    .with_base_url(server.uri());
    let api = OpenAiResponsesChatApi::new(
        Arc::new(OpenAiResponsesProvider::new(Some(server.uri()))),
        model.clone(),
    )
    .unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            Arc::new(api),
            Arc::new(CountingAuth(count.clone())),
            opi_ai::AuthProvenanceSource::Static,
        )
        .unwrap();
    let mut request = responses_public_request("probe", None);
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        tools: vec![],
        messages: vec![Message::User(opi_ai::message::UserMessage {
            content: vec![opi_ai::message::InputContent::Image {
                source: opi_ai::message::ImageSource::Base64 {
                    data: "AA==".into(),
                },
                media_type: opi_ai::message::MediaType::Png,
            }],
            timestamp_ms: 0,
        })],
    })
    .unwrap();
    assert!(
        models
            .complete(
                ChatModelRecord::new("openai-responses", model.clone()),
                request,
                handle.options(Default::default())
            )
            .await
            .is_err()
    );
    assert_eq!(count.load(Ordering::SeqCst), 0);
    for base in ["not-a-valid-url", "file:///tmp/not-an-http-endpoint"] {
        let mut invalid = model.clone();
        invalid.base_url = Some(base.into());
        let failure = models
            .complete(
                ChatModelRecord::new("openai-responses", invalid),
                responses_public_request("probe", None),
                handle.options(Default::default()),
            )
            .await
            .unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::Config(_)));
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }
    assert!(server.received_requests().await.unwrap().is_empty());
    models
        .complete(
            ChatModelRecord::new("openai-responses", model),
            responses_public_request("probe", None),
            handle.options(Default::default()),
        )
        .await
        .unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn multiline_sse_line_endings_survive_http_chunk_splits() {
    async fn run(body: String, split: Option<usize>) -> Vec<AssistantStreamEvent> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 65536];
            let received = socket.read(&mut request).await.unwrap();
            assert!(
                received > 0,
                "client sent request bytes before the response"
            );
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes()).await.unwrap();
            if let Some(split) = split {
                socket.write_all(&body.as_bytes()[..split]).await.unwrap();
                socket.flush().await.unwrap();
                tokio::time::sleep(std::time::Duration::from_millis(40)).await;
                socket.write_all(&body.as_bytes()[split..]).await.unwrap();
            } else {
                socket.write_all(body.as_bytes()).await.unwrap();
            }
            socket.shutdown().await.unwrap();
        });
        let provider = OpenAiResponsesProvider::new(Some(format!("http://{address}")));
        let results: Vec<_> = provider
            .stream_prepared(
                responses_text_request(),
                opi_ai::test_support::resolved_bearer_auth(),
            )
            .collect()
            .await;
        peer.await.unwrap();
        results
            .into_iter()
            .map(|event| event.expect("legal complete multiline event"))
            .collect()
    }

    let body = [
        serde_json::json!({"type":"response.created","response":{"id":"resp_1","status":"in_progress","model":"gpt-4o","output":[]}}),
        serde_json::json!({"type":"response.output_item.added","output_index":0,"item":{"type":"message","status":"in_progress","role":"assistant","content":[]}}),
        serde_json::json!({"type":"response.content_part.added","output_index":0,"content_index":0,"part":{"type":"output_text","text":""}}),
        serde_json::json!({"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":"hello"}),
        serde_json::json!({"type":"response.output_text.done","output_index":0,"content_index":0,"text":"hello"}),
        serde_json::json!({"type":"response.output_item.done","output_index":0,"item":{"type":"message","status":"completed","role":"assistant","content":[{"type":"output_text","text":"hello"}]}}),
        serde_json::json!({"type":"response.completed","response":{"id":"resp_1","status":"completed","model":"gpt-4o","output":[{"type":"message","content":[{"type":"output_text","text":"hello"}]}],"usage":{"input_tokens":10,"output_tokens":5}}}),
    ].iter().map(|value| format!("data: {value}\n\n")).collect::<String>();
    for newline in ["\n", "\r\n", "\r"] {
        let multiline = body
            .lines()
            .map(|line| {
                if line.starts_with("data: {") {
                    line.replacen(',', &format!(",{newline}data: "), 1)
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join(newline)
            + newline;
        let split = multiline.find(&format!("{newline}data: ")).unwrap() + 1;
        let whole = run(multiline.clone(), None).await;
        let split = run(multiline, Some(split)).await;
        let kinds = |events: &[AssistantStreamEvent]| {
            events
                .iter()
                .map(|event| serde_json::to_value(event).unwrap()["type"].clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(kinds(&split), kinds(&whole), "newline={newline:?}");
        let final_message = |events: Vec<AssistantStreamEvent>| {
            events
                .into_iter()
                .find_map(|event| match event {
                    AssistantStreamEvent::Done { message, .. } => Some(message),
                    _ => None,
                })
                .expect("terminal outcome")
        };
        let whole = final_message(whole);
        let split = final_message(split);
        assert_eq!(split.content, whole.content);
        assert_eq!(split.stop_reason, whole.stop_reason);
        assert_eq!(split.usage, whole.usage);
        assert!(!split.content.is_empty());
    }
}

mod source_compat_controls {
    use super::*;
    use opi_ai::model_info::{
        OpenAiResponsesCompat, OpenAiResponsesSourceCompat, SessionAffinityFormat, WireCompat,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    fn source_model(base: Option<String>, source: OpenAiResponsesSourceCompat) -> ModelInfo {
        let mut model = o3_responses_model()
            .with_compat(WireCompat::OpenAiResponses(OpenAiResponsesCompat {
                source,
                ..Default::default()
            }))
            .unwrap();
        model.base_url = base;
        model
    }

    fn provider(base: String, provider_id: &str, model: ModelInfo) -> OpenAiResponsesProvider {
        OpenAiResponsesProvider::for_route(
            Some(base),
            provider_id.into(),
            Default::default(),
            vec![model],
            Arc::new(opi_ai::http::HttpClient::new()),
        )
        .with_affinity_policy(opi_ai::openai_responses::ResponsesAffinityPolicy::Direct)
    }

    fn request(provider_id: &str, retention: CacheRetention) -> Request {
        let mut request = responses_public_request("o3", Some(ThinkingLevel::High));
        request.model = format!("{provider_id}:o3");
        let mut context = responses_tool_context();
        context
            .messages
            .push(Message::System(opi_ai::transcript::SystemMessage {
                content: "later instruction".into(),
                sections: vec![],
                tools_added: vec![],
                tools_removed: vec![],
                timestamp_ms: 1,
            }));
        request.context = opi_ai::transcript::normalize_context(context).unwrap();
        request.max_tokens = Some(7);
        request.temperature = Some(0.25);
        request.session_id = Some("raw-session-id".into());
        request.cache_retention = retention;
        request
    }

    #[test]
    fn responses_initial_instruction_uses_the_selected_input_role() {
        for flag in [None, Some(false), Some(true)] {
            for reasoning in [false, true] {
                let source = OpenAiResponsesSourceCompat {
                    supports_developer_role: flag,
                    supports_mid_convo_system_messages: Some(true),
                    ..Default::default()
                };
                let mut model = source_model(None, source);
                model.capabilities.supports_thinking = reasoning;
                let provider = provider("http://127.0.0.1:1".into(), "openai-responses", model);
                let request = request("openai-responses", CacheRetention::Short);
                let canonical = request.context.clone();
                let body = provider.build_request_body(&request).unwrap();
                assert_eq!(
                    body["input"][0]["role"],
                    if reasoning && flag != Some(false) {
                        "developer"
                    } else {
                        "system"
                    }
                );
                assert_eq!(body["input"][0]["content"], "You are helpful.");
                assert!(
                    body.get("instructions").is_none(),
                    "ordinary Responses carries the initial instruction once in input"
                );
                assert_eq!(request.context, canonical);
            }
        }
    }

    #[tokio::test]
    async fn responses_source_defaults_false_and_true_reach_actual_http() {
        for flag in [None, Some(false), Some(true)] {
            for retention in [
                CacheRetention::Disabled,
                CacheRetention::Short,
                CacheRetention::Long,
            ] {
                let server = MockServer::start().await;
                Mock::given(method("POST"))
                    .respond_with(
                        ResponseTemplate::new(200)
                            .insert_header("content-type", "text/event-stream")
                            .set_body_string(responses_sse_ok()),
                    )
                    .expect(1)
                    .mount(&server)
                    .await;
                let source = OpenAiResponsesSourceCompat {
                    supports_developer_role: flag,
                    supports_max_output_tokens: flag,
                    supports_long_cache_retention: flag,
                    supports_explicit_prompt_cache_mode: flag,
                    supports_strict_mode: flag,
                    supports_mid_convo_system_messages: Some(true),
                    ..Default::default()
                };
                let provider =
                    provider(server.uri(), "openai-responses", source_model(None, source));
                let request = request("openai-responses", retention);
                let canonical = request.context.clone();
                let message = opi_ai::stream::ChatStream::new(provider.stream_prepared(
                    request.clone(),
                    opi_ai::test_support::resolved_bearer_auth(),
                ))
                .result()
                .await
                .unwrap();
                assert!(message.projection.is_some());
                assert_eq!(request.context, canonical);
                let sent = server.received_requests().await.unwrap();
                assert_eq!(sent.len(), 1);
                let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
                let instruction = body["input"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|item| item["content"] == "later instruction")
                    .unwrap();
                assert_eq!(
                    instruction["role"],
                    if flag == Some(false) {
                        "system"
                    } else {
                        "developer"
                    }
                );
                assert_eq!(
                    body.get("max_output_tokens").cloned(),
                    (flag != Some(false)).then(|| serde_json::json!(16))
                );
                let strict = body["tools"][0].get("strict");
                assert_eq!(
                    strict,
                    (flag == Some(true)).then_some(&serde_json::Value::Bool(false))
                );
                let expected_options = if flag == Some(true) {
                    match retention {
                        CacheRetention::Disabled => Some(serde_json::json!({"mode":"explicit"})),
                        CacheRetention::Long => Some(serde_json::json!({"ttl":"30m"})),
                        _ => None,
                    }
                } else {
                    None
                };
                assert_eq!(body.get("prompt_cache_options"), expected_options.as_ref());
                let legacy = retention == CacheRetention::Long && flag.is_none();
                assert_eq!(
                    body.get("prompt_cache_retention")
                        .and_then(serde_json::Value::as_str),
                    legacy.then_some("24h")
                );
                assert_eq!(
                    body.get("prompt_cache_key").is_some(),
                    retention != CacheRetention::Disabled
                );
                assert_eq!(sent[0].headers["session_id"], "raw-session-id");
                assert_eq!(sent[0].headers["x-client-request-id"], "raw-session-id");
                server.verify().await;
            }
        }
    }

    #[tokio::test]
    async fn responses_strict_capability_preserves_unconstrained_initial_and_inline_schemas() {
        use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
        for preference in [
            None,
            Some(StrictPreference::Prefer),
            Some(StrictPreference::Require),
        ] {
            for native in [false, true] {
                let server = MockServer::start().await;
                Mock::given(method("POST"))
                    .respond_with(
                        ResponseTemplate::new(200)
                            .insert_header("content-type", "text/event-stream")
                            .set_body_string(responses_sse_ok()),
                    )
                    .expect(1)
                    .mount(&server)
                    .await;
                let schema =
                    serde_json::json!({"type":"object","properties":{"path":{"type":"string"}}});
                let tool = |name: &str| ToolDef {
                    name: name.into(),
                    description: name.into(),
                    input_schema: schema.clone(),
                    constrained_sampling: preference
                        .map(|strict| ConstrainedSampling::JsonSchema { strict }),
                };
                let source = OpenAiResponsesSourceCompat {
                    supports_strict_mode: Some(true),
                    supports_mid_convo_system_messages: Some(true),
                    supports_additional_tools: Some(true),
                    ..Default::default()
                };
                let model = source_model(None, source);
                let mut request = request("openai-responses", CacheRetention::Disabled);
                request.context =
                    opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                        system_prompt: Some("leading".into()),
                        tools: vec![tool("initial")],
                        messages: vec![
                            Message::User(UserMessage {
                                content: vec![InputContent::Text {
                                    text: "before".into(),
                                }],
                                timestamp_ms: 0,
                            }),
                            Message::System(opi_ai::transcript::SystemMessage {
                                tools_added: vec![tool("inline")],
                                ..Default::default()
                            }),
                            Message::User(UserMessage {
                                content: vec![InputContent::Text {
                                    text: "after".into(),
                                }],
                                timestamp_ms: 1,
                            }),
                        ],
                    })
                    .unwrap();
                let canonical = request.context.clone();
                let message = if native {
                    let (models, handle) =
                        registered_responses_models(server.uri(), model.clone()).await;
                    models
                        .complete(
                            ChatModelRecord::new("openai-responses", model),
                            request.clone(),
                            handle.options(Default::default()),
                        )
                        .await
                        .unwrap()
                } else {
                    let provider = provider(server.uri(), "openai-responses", model);
                    opi_ai::stream::ChatStream::new(provider.stream_prepared(
                        request.clone(),
                        opi_ai::test_support::resolved_bearer_auth(),
                    ))
                    .result()
                    .await
                    .unwrap()
                };
                assert!(message.projection.is_some());
                assert_eq!(request.context, canonical);
                let sent = server.received_requests().await.unwrap();
                assert_eq!(sent.len(), 1);
                let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
                let inline = body["input"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|item| item["type"] == "additional_tools")
                    .unwrap();
                for declaration in [&body["tools"][0], &inline["tools"][0]] {
                    assert_eq!(
                        declaration["strict"],
                        preference.is_some(),
                        "native={native}, preference={preference:?}"
                    );
                    if preference.is_none() {
                        assert_eq!(
                            declaration["parameters"], schema,
                            "capability alone never rewrites trusted optional properties"
                        );
                    } else {
                        assert_eq!(declaration["parameters"]["additionalProperties"], false);
                        assert_eq!(
                            declaration["parameters"]["required"],
                            serde_json::json!(["path"])
                        );
                    }
                }
                server.verify().await;
            }
        }
    }

    #[tokio::test]
    async fn responses_cache_controls_are_independent_of_max_role_and_each_other() {
        for (explicit, long, retention, expected_options, expected_legacy) in [
            (
                Some(true),
                Some(false),
                CacheRetention::Disabled,
                Some(serde_json::json!({"mode":"explicit"})),
                false,
            ),
            (Some(true), Some(false), CacheRetention::Long, None, false),
            (
                Some(true),
                None,
                CacheRetention::Long,
                Some(serde_json::json!({"ttl":"30m"})),
                false,
            ),
            (Some(true), Some(true), CacheRetention::Short, None, false),
            (Some(false), Some(true), CacheRetention::Long, None, true),
            (None, Some(false), CacheRetention::Long, None, false),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(responses_sse_ok()),
                )
                .expect(1)
                .mount(&server)
                .await;
            let source = OpenAiResponsesSourceCompat {
                supports_explicit_prompt_cache_mode: explicit,
                supports_long_cache_retention: long,
                supports_mid_convo_system_messages: Some(true),
                ..Default::default()
            };
            let provider = provider(server.uri(), "openai-responses", source_model(None, source));
            let mut request = request("openai-responses", retention);
            request.session_id = None;
            opi_ai::stream::ChatStream::new(
                provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()),
            )
            .result()
            .await
            .unwrap();
            let sent = server.received_requests().await.unwrap();
            assert_eq!(sent.len(), 1);
            let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
            assert_eq!(body.get("prompt_cache_options"), expected_options.as_ref());
            assert_eq!(
                body.get("prompt_cache_retention")
                    .and_then(serde_json::Value::as_str),
                expected_legacy.then_some("24h")
            );
            assert!(
                body.get("prompt_cache_key").is_none(),
                "cache mode and retention do not require a session/cache key"
            );
            assert_eq!(body["max_output_tokens"], 16);
            assert!(body["tools"][0].get("strict").is_none());
            assert!(
                body["input"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| item["role"] == "developer"
                        && item["content"] == "later instruction")
            );
            server.verify().await;
        }
    }

    #[tokio::test]
    async fn native_and_advanced_signin_use_prepared_auth_once_and_keep_known_failure_facts() {
        struct CountingAuth(Arc<AtomicUsize>);
        impl opi_ai::auth::AuthResolver for CountingAuth {
            fn resolve(
                &self,
            ) -> opi_ai::credential::BoxAuthFuture<
                '_,
                Result<opi_ai::auth::ResolvedAuth, ProviderError>,
            > {
                self.0.fetch_add(1, Ordering::SeqCst);
                Box::pin(async {
                    let mut auth = opi_ai::test_support::resolved_bearer_auth();
                    auth.secret = secrecy::SecretString::from("offline-signin-key");
                    Ok(auth)
                })
            }
        }
        for native in [false, true] {
            let fallback = MockServer::start().await;
            let source = OpenAiResponsesSourceCompat {
                supports_mid_convo_system_messages: Some(true),
                supports_mid_convo_effort: Some(true),
                supports_explicit_prompt_cache_mode: Some(true),
                ..Default::default()
            };
            let model = source_model(Some("https://api.openai.com/v1".into()), source);
            let provider = provider(fallback.uri(), "openai", model.clone());
            let auth_calls = Arc::new(AtomicUsize::new(0));
            let auth = Arc::new(CountingAuth(auth_calls.clone()));
            let bodies = Arc::new(Mutex::new(Vec::new()));
            let recorder = bodies.clone();
            let mut request = request("openai", CacheRetention::Long);
            request.callbacks =
                opi_ai::callbacks::RequestCallbacks::new().with_on_payload(Arc::new(move |body| {
                    recorder.lock().unwrap().push(body.clone());
                    Err(opi_ai::callbacks::CallbackError::new(
                        "intentional prepared capture sentinel",
                    ))
                }));
            let mut models = Models::new();
            let failure = if native {
                let api = OpenAiResponsesChatApi::new(Arc::new(provider), model.clone()).unwrap();
                let handle = models
                    .register_chat_api(Arc::new(api), auth, opi_ai::AuthProvenanceSource::Static)
                    .unwrap();
                let mut options = OpenAiResponsesOptions::default();
                options.reasoning_effort = Some(ThinkingLevel::High);
                models
                    .complete(
                        ChatModelRecord::new("openai", model),
                        request,
                        handle.options(options),
                    )
                    .await
                    .unwrap_err()
            } else {
                models
                    .register_route(
                        Box::new(provider),
                        auth,
                        opi_ai::AuthProvenanceSource::Static,
                        Default::default(),
                    )
                    .unwrap();
                let call = models
                    .prepare_call_with_policy(
                        "openai:o3",
                        request,
                        opi_ai::retry::AcquisitionPolicy::with_max_retries(1),
                    )
                    .await
                    .unwrap();
                let failure = opi_ai::stream::ChatStream::new(call.start_attempt().unwrap())
                    .result()
                    .await
                    .unwrap_err();
                assert!(
                    call.retry_admission().is_none(),
                    "callback refusal does not authorize transport retry"
                );
                failure
            };
            assert!(matches!(failure.cause(), ProviderError::CallbackPhase(_)));
            assert!(failure.message().projection.is_some());
            assert_eq!(auth_calls.load(Ordering::SeqCst), 1);
            let bodies = bodies.lock().unwrap().clone();
            assert_eq!(bodies.len(), 1);
            let body = &bodies[0];
            for field in [
                "max_output_tokens",
                "temperature",
                "prompt_cache_options",
                "prompt_cache_retention",
            ] {
                assert!(body.get(field).is_none(), "{field}, native={native}");
            }
            assert!(
                !body["input"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| item["type"] == "configuration_update")
            );
            assert!(fallback.received_requests().await.unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn responses_affinity_formats_are_independent_of_disabled_cache() {
        for format in [
            SessionAffinityFormat::OpenAi,
            SessionAffinityFormat::OpenAiNoSession,
            SessionAffinityFormat::OpenRouter,
        ] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(responses_sse_ok()),
                )
                .expect(1)
                .mount(&server)
                .await;
            let source = OpenAiResponsesSourceCompat {
                session_affinity_format: Some(format),
                supports_mid_convo_system_messages: Some(true),
                ..Default::default()
            };
            let provider = provider(server.uri(), "openai-responses", source_model(None, source));
            let mut request = request("openai-responses", CacheRetention::Disabled);
            let full_id = "full-sdk-session-id-".repeat(8);
            request.session_id = Some(full_id.clone());
            opi_ai::stream::ChatStream::new(
                provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()),
            )
            .result()
            .await
            .unwrap();
            let sent = server.received_requests().await.unwrap();
            let headers = &sent[0].headers;
            assert_eq!(
                headers.contains_key("session_id"),
                format == SessionAffinityFormat::OpenAi
            );
            assert_eq!(
                headers.contains_key("x-client-request-id"),
                format != SessionAffinityFormat::OpenRouter
            );
            assert_eq!(
                headers.contains_key("x-session-id"),
                format == SessionAffinityFormat::OpenRouter
            );
            if let Some(value) = headers.get(if format == SessionAffinityFormat::OpenRouter {
                "x-session-id"
            } else {
                "session_id"
            }) {
                assert_eq!(value.to_str().unwrap(), full_id);
            }
            if let Some(value) = headers.get("x-client-request-id") {
                assert_eq!(value.to_str().unwrap(), full_id);
            }
            let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
            assert!(body.get("prompt_cache_key").is_none());
            server.verify().await;
        }
    }

    #[tokio::test]
    async fn responses_affinity_overrides_are_merged_before_transform_and_http() {
        for (mode, expected, before_transform) in [
            ("default", Some("raw-session-id"), Some("raw-session-id")),
            ("caller", Some("caller-value"), Some("caller-value")),
            ("model", Some("model-value"), Some("model-value")),
            ("remove", None, None),
            (
                "transform",
                Some("transformed-value"),
                Some("raw-session-id"),
            ),
            ("route", Some("route-value"), Some("route-value")),
            (
                "configured",
                Some("configured-value"),
                Some("configured-value"),
            ),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(responses_sse_ok()),
                )
                .expect(1)
                .mount(&server)
                .await;
            let source = OpenAiResponsesSourceCompat {
                session_affinity_format: Some(SessionAffinityFormat::OpenRouter),
                supports_mid_convo_system_messages: Some(true),
                ..Default::default()
            };
            let mut model = source_model(None, source.clone());
            if mode == "model" {
                model
                    .headers
                    .push(("X-Session-Id".into(), Some("model-value".into())));
            }
            if mode == "remove" {
                model.headers.push(("x-session-id".into(), None));
            }
            let provider = if mode == "route" {
                OpenAiResponsesProvider::with_auth_extra(
                    Some(server.uri()),
                    ResponsesConfig {
                        source,
                        ..Default::default()
                    },
                    "openai-responses".into(),
                    vec![("X-Session-Id".into(), "route-value".into())],
                    Arc::new(opi_ai::http::HttpClient::new()),
                )
                .with_affinity_policy(opi_ai::openai_responses::ResponsesAffinityPolicy::Direct)
            } else {
                let configured = if mode == "configured" {
                    vec![("X-Session-Id".into(), "configured-value".into())]
                } else {
                    vec![]
                };
                OpenAiResponsesProvider::for_route(
                    Some(server.uri()),
                    "openai-responses".into(),
                    opi_ai::provider_headers::ProviderHeaders::try_new(configured).unwrap(),
                    vec![model],
                    Arc::new(opi_ai::http::HttpClient::new()),
                )
                .with_affinity_policy(opi_ai::openai_responses::ResponsesAffinityPolicy::Direct)
            };
            let seen = Arc::new(Mutex::new(Vec::new()));
            let recorder = seen.clone();
            let calls = Arc::new(AtomicUsize::new(0));
            let counter = calls.clone();
            let mut request = request("openai-responses", CacheRetention::Disabled);
            if mode == "caller" {
                request
                    .extra_headers
                    .push(("X-Session-Id".into(), "caller-value".into()));
            }
            request.callbacks = opi_ai::callbacks::RequestCallbacks::new()
                .with_async_header_transform(move |mut headers| {
                    let recorder = recorder.clone();
                    let counter = counter.clone();
                    async move {
                        counter.fetch_add(1, Ordering::SeqCst);
                        *recorder.lock().unwrap() = headers.clone();
                        if mode == "transform" {
                            headers.retain(|(name, _)| !name.eq_ignore_ascii_case("x-session-id"));
                            headers.push(("x-session-id".into(), "transformed-value".into()));
                        }
                        Ok(headers)
                    }
                });
            let message = opi_ai::stream::ChatStream::new(
                provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()),
            )
            .result()
            .await
            .unwrap();
            assert!(message.projection.is_some());
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            let seen = seen.lock().unwrap().clone();
            let observed: Vec<_> = seen
                .iter()
                .filter(|(name, _)| name.eq_ignore_ascii_case("x-session-id"))
                .map(|(_, value)| value.as_str())
                .collect();
            assert_eq!(
                observed,
                before_transform.into_iter().collect::<Vec<_>>(),
                "mode={mode}: transform sees the merged generated default"
            );
            let sent = server.received_requests().await.unwrap();
            assert_eq!(sent.len(), 1);
            let values: Vec<_> = sent[0]
                .headers
                .get_all("x-session-id")
                .iter()
                .map(|value| value.to_str().unwrap())
                .collect();
            assert_eq!(
                values,
                expected.into_iter().collect::<Vec<_>>(),
                "mode={mode}: one effective ordinary header survives"
            );
            server.verify().await;
        }
    }

    #[tokio::test]
    async fn responses_affinity_inference_uses_the_source_url_rule_and_explicit_override() {
        for (provider_id, path, format, router) in [
            ("proxy", "/openrouter.ai", None, true),
            (
                "proxy",
                "/openrouter.ai",
                Some(SessionAffinityFormat::OpenAiNoSession),
                false,
            ),
            ("openrouter", "/ordinary", None, true),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(responses_sse_ok()),
                )
                .expect(1)
                .mount(&server)
                .await;
            let source = OpenAiResponsesSourceCompat {
                session_affinity_format: format,
                supports_mid_convo_system_messages: Some(true),
                ..Default::default()
            };
            let provider = provider(
                server.uri(),
                provider_id,
                source_model(Some(format!("{}{path}", server.uri())), source),
            );
            let request = request(provider_id, CacheRetention::Disabled);
            opi_ai::stream::ChatStream::new(
                provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()),
            )
            .result()
            .await
            .unwrap();
            let sent = server.received_requests().await.unwrap();
            assert_eq!(sent.len(), 1);
            assert_eq!(sent[0].headers.contains_key("x-session-id"), router);
            assert_eq!(sent[0].headers.contains_key("x-client-request-id"), !router);
            assert!(!sent[0].headers.contains_key("session_id"));
            assert_eq!(
                sent[0].headers[if router {
                    "x-session-id"
                } else {
                    "x-client-request-id"
                }],
                "raw-session-id"
            );
            server.verify().await;
        }
    }

    #[tokio::test]
    async fn exact_prepared_signin_suppresses_only_source_qualified_branches_without_sends() {
        for (provider_id, base, key, signin) in [
            ("openai", "https://api.openai.com/v1", "oauth-access", true),
            ("openai", "https://api.openai.com/v1", "", true),
            (
                "openai",
                "https://api.openai.com/v1",
                "sk-project-key",
                false,
            ),
            (
                "openai-responses",
                "https://api.openai.com/v1",
                "oauth-access",
                false,
            ),
            ("openai", "https://api.openai.com", "oauth-access", false),
            (
                "openai",
                "https://api.openai.com/v1/",
                "oauth-access",
                false,
            ),
            (
                "openai",
                "https://api.openai.com.evil.invalid/v1",
                "oauth-access",
                false,
            ),
        ] {
            let fallback = MockServer::start().await;
            let source = OpenAiResponsesSourceCompat {
                supports_mid_convo_system_messages: Some(true),
                supports_mid_convo_effort: Some(true),
                supports_explicit_prompt_cache_mode: Some(true),
                ..Default::default()
            };
            let provider = provider(
                fallback.uri(),
                provider_id,
                source_model(Some(base.into()), source),
            );
            let mut request = request(provider_id, CacheRetention::Long);
            let absent_auth_body = provider.build_request_body(&request).unwrap();
            assert!(
                absent_auth_body.get("max_output_tokens").is_some(),
                "no prepared key means no signin classification"
            );
            let seen = Arc::new(Mutex::new(Vec::new()));
            let recorder = seen.clone();
            let response_calls = Arc::new(AtomicUsize::new(0));
            let response_counter = response_calls.clone();
            request.callbacks = opi_ai::callbacks::RequestCallbacks::new()
                .with_on_payload(Arc::new(move |body| {
                    recorder.lock().unwrap().push(body.clone());
                    Err(opi_ai::callbacks::CallbackError::new(
                        "intentional post-auth capture sentinel",
                    ))
                }))
                .with_on_response(Arc::new(move |_, _| {
                    response_counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }));
            let mut auth = opi_ai::test_support::resolved_bearer_auth();
            auth.secret = secrecy::SecretString::from(key);
            let failure = opi_ai::stream::ChatStream::new(provider.stream_prepared(request, auth))
                .result()
                .await
                .unwrap_err();
            assert!(matches!(failure.cause(), ProviderError::CallbackPhase(_)));
            assert!(failure.message().projection.is_some());
            let bodies = seen.lock().unwrap().clone();
            assert_eq!(bodies.len(), 1);
            let body = &bodies[0];
            assert_eq!(
                body.get("max_output_tokens").is_none(),
                signin,
                "{provider_id} {base}"
            );
            assert_eq!(body.get("temperature").is_none(), signin);
            assert_eq!(body.get("prompt_cache_options").is_none(), signin);
            assert!(body.get("prompt_cache_retention").is_none());
            let updates = body["input"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|item| item["type"] == "configuration_update")
                .count();
            assert_eq!(updates, usize::from(!signin));
            assert_eq!(response_calls.load(Ordering::SeqCst), 0);
            assert!(
                fallback.received_requests().await.unwrap().is_empty(),
                "capture sentinel precedes every transport send, including canonical external host"
            );
        }
    }
}
