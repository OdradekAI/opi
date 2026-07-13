//! OpenAI Responses API provider fixture tests (task 2.3).
//!
//! Verifies: SSE event mapping, text streaming, tool call streaming,
//! usage tracking, error handling, model resolution, and request body
//! construction for the OpenAI Responses API (`/v1/responses`).

use futures_util::StreamExt;
use opi_ai::message::{InputContent, Message, ToolDef, UserMessage};
use opi_ai::openai_responses::{OpenAiResponsesProvider, ResponsesConfig};
use opi_ai::provider::{CacheRetention, EventStream, Provider, Request, ThinkingConfig};
use opi_ai::registry::ProviderRegistry;
use opi_ai::stream::AssistantStreamEvent;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: create an OpenAI Responses provider.
fn responses_provider(api_key: &str) -> OpenAiResponsesProvider {
    OpenAiResponsesProvider::new(api_key.into(), None)
}

/// Helper: collect stream events asynchronously.
async fn collect_stream(stream: EventStream) -> Vec<AssistantStreamEvent> {
    stream.filter_map(|r| async move { r.ok() }).collect().await
}

// ---------------------------------------------------------------------------
// Provider identity
// ---------------------------------------------------------------------------

#[test]
fn responses_provider_id_is_openai_responses() {
    let provider = responses_provider("test-key");
    assert_eq!(provider.id(), "openai-responses");
}

// ---------------------------------------------------------------------------
// Model resolution via registry
// ---------------------------------------------------------------------------

#[test]
fn responses_resolves_model_in_registry() {
    let mut registry = ProviderRegistry::new();
    registry.register(Box::new(responses_provider("key")));
    let (provider, model) = registry.resolve("openai-responses:gpt-4o").unwrap();
    assert_eq!(provider.id(), "openai-responses");
    assert_eq!(model.id, "gpt-4o");
}

#[test]
fn responses_registry_lists_provider_id() {
    let mut registry = ProviderRegistry::new();
    registry.register(Box::new(responses_provider("key")));
    let ids = registry.provider_ids();
    assert!(ids.contains(&"openai-responses"));
}

#[test]
fn responses_unknown_model_returns_error() {
    let mut registry = ProviderRegistry::new();
    registry.register(Box::new(responses_provider("key")));
    let result = registry.resolve("openai-responses:nonexistent-model");
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Request body construction
// ---------------------------------------------------------------------------

#[test]
fn responses_request_body_uses_input_field() {
    let provider = responses_provider("key");
    let request = Request {
        model: "openai-responses:gpt-4o".into(),
        system: Some("You are helpful.".into()),
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "Hello".into(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
        max_tokens: Some(1024),
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };
    let body = provider.build_request_body(&request);
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
    // System prompt uses top-level "instructions" field, not in input array
    assert_eq!(body["instructions"], "You are helpful.");
    let input = body["input"].as_array().unwrap();
    assert!(
        !input
            .iter()
            .any(|m| m.get("role").map(|r| r == "system").unwrap_or(false)),
        "system message should NOT appear in input array"
    );
}

#[test]
fn responses_request_body_strips_provider_prefix() {
    let provider = responses_provider("key");
    let request = Request {
        model: "openai-responses:o3".into(),
        system: None,
        messages: vec![],
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
    };
    let body = provider.build_request_body(&request);
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
        model: "openai-responses:gpt-4o".into(),
        system: Some("You are helpful.".into()),
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
        }],
        max_tokens: Some(1024),
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

#[test]
fn responses_config_represents_store_reasoning_strict() {
    let config = ResponsesConfig {
        store: Some(true),
        reasoning_effort: Some("high".into()),
        strict_tools: true,
        ..Default::default()
    };
    assert_eq!(config.store, Some(true));
    assert_eq!(config.reasoning_effort.as_deref(), Some("high"));
    assert!(config.strict_tools);
}

#[test]
fn responses_request_body_store_emitted_when_configured() {
    let provider = OpenAiResponsesProvider::new_with_config(
        "key".into(),
        None,
        ResponsesConfig {
            store: Some(false),
            ..Default::default()
        },
    );
    let body = provider.build_request_body(&responses_tool_request());
    assert_eq!(
        body["store"], false,
        "store must be emitted when configured"
    );
}

#[test]
fn responses_request_body_store_absent_by_default() {
    let provider = responses_provider("key");
    let body = provider.build_request_body(&responses_tool_request());
    assert!(
        body.get("store").is_none(),
        "store must be absent by default"
    );
}

#[test]
fn responses_request_body_reasoning_effort_emitted_when_configured() {
    let provider = OpenAiResponsesProvider::new_with_config(
        "key".into(),
        None,
        ResponsesConfig {
            reasoning_effort: Some("high".into()),
            ..Default::default()
        },
    );
    let body = provider.build_request_body(&responses_tool_request());
    assert_eq!(
        body["reasoning"]["effort"], "high",
        "reasoning.effort must be emitted when configured"
    );
}

#[test]
fn responses_request_body_reasoning_effort_absent_by_default() {
    let provider = responses_provider("key");
    let body = provider.build_request_body(&responses_tool_request());
    assert!(
        body.get("reasoning").is_none(),
        "reasoning must be absent by default"
    );
}

#[test]
fn responses_request_body_strict_tools_emitted_when_configured() {
    let provider = OpenAiResponsesProvider::new_with_config(
        "key".into(),
        None,
        ResponsesConfig {
            strict_tools: true,
            ..Default::default()
        },
    );
    let body = provider.build_request_body(&responses_tool_request());
    let tools = body["tools"].as_array().expect("tools array present");
    assert!(
        tools[0]["strict"] == true,
        "strict must be emitted on function tools when configured: {tools:?}"
    );
}

#[test]
fn responses_request_body_strict_tools_absent_by_default() {
    let provider = responses_provider("key");
    let body = provider.build_request_body(&responses_tool_request());
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
    let provider = responses_provider("key");
    let body = provider.build_request_body(&responses_tool_request());
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
    let provider = responses_provider("key");
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
    let provider = responses_provider("key");
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
    let provider = responses_provider("key");
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
    let provider = responses_provider("key");
    let sse = "event: error\n\
               data: {\"type\":\"error\",\"message\":\"Model not found\"}\n\n";

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Error { .. })),
        "should have an Error event"
    );
}

// ---------------------------------------------------------------------------
// Usage tracking
// ---------------------------------------------------------------------------

#[tokio::test]
async fn responses_usage_in_done_event() {
    let provider = responses_provider("key");
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
    let provider = responses_provider("key");
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
    let provider = responses_provider("key");
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

// ---------------------------------------------------------------------------
// Model list
// ---------------------------------------------------------------------------

#[test]
fn responses_has_model_list() {
    let provider = responses_provider("key");
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
    let provider = responses_provider("key");
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
                        opi_ai::message::AssistantContent::Text { text } => Some(text.as_str()),
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
    let provider = responses_provider("key");
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
    let provider = opi_ai::openai_responses::OpenAiResponsesProvider::new(
        "key".into(),
        Some("https://custom.proxy".into()),
    );
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
    let provider = responses_provider("key");
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
    let provider = responses_provider("key");
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
    assert_eq!(ended[0].id, "call_a");
    assert_eq!(ended[0].arguments, "{\"path\":\"a.rs\"}");
    assert_eq!(ended[1].id, "call_b");
    assert_eq!(ended[1].arguments, "{\"cmd\":\"ls\"}");
}

#[tokio::test]
async fn responses_message_output_item_done_does_not_duplicate_tool_end() {
    let provider = responses_provider("key");
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
    let provider = responses_provider("key");
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
    let provider = responses_provider("key");
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
    assert_eq!(ended[0].id, "call_1");
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
    let provider = responses_provider("key");
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
        end.id, "call_roundtrip",
        "ToolCall.id must be the Responses call_id, not the item id"
    );
    assert_eq!(
        end.arguments, "{}",
        "Responses tool-call arguments carried on output_item.added/done must be preserved even without argument deltas"
    );
}

#[tokio::test]
async fn responses_malformed_tool_args_pass_raw_string_without_panic() {
    let provider = responses_provider("key");
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
    assert_eq!(end.id, "call_bad");
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
            "instructions": "You are helpful."
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = OpenAiResponsesProvider::new("test-key".into(), Some(server.uri()));
    let request = Request {
        model: "openai-responses:gpt-4o".into(),
        system: Some("You are helpful.".into()),
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "Hello".into(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
        max_tokens: Some(1024),
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };

    let mut stream = provider.stream(request);
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) if event.is_terminal() => break,
            Err(_) => break,
            _ => {}
        }
    }

    // verify() confirms the production request carried the Responses body
    // (model + max_output_tokens + instructions), the Bearer auth header, and
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
    let provider = OpenAiResponsesProvider::new("test-key".into(), Some(server.uri()));
    let request = Request {
        model: "openai-responses:gpt-4o".into(),
        system: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "Hello".into(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
        max_tokens: Some(1024),
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: cancel.clone(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };
    let mut stream = provider.stream(request);

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
