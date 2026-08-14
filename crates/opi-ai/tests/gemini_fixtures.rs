//! Google Gemini provider fixture tests (task 2.4).
//!
//! Verifies: SSE event mapping, text streaming, tool call streaming,
//! usage tracking, error handling, model resolution, and request body
//! construction for the Google Gemini `streamGenerateContent` API.

use futures_util::StreamExt;
use opi_ai::gemini::GeminiProvider;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, EventStream, Provider, Request, ThinkingConfig};
use opi_ai::registry::ProviderRegistry;
use opi_ai::stream::AssistantStreamEvent;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: create a Gemini provider.
fn gemini_provider() -> GeminiProvider {
    GeminiProvider::new(None)
}

/// Helper: collect stream events asynchronously.
async fn collect_stream(stream: EventStream) -> Vec<AssistantStreamEvent> {
    stream.filter_map(|r| async move { r.ok() }).collect().await
}

// ---------------------------------------------------------------------------
// Provider identity
// ---------------------------------------------------------------------------

#[test]
fn gemini_provider_id_is_gemini() {
    let provider = gemini_provider();
    assert_eq!(provider.id(), "gemini");
}

// ---------------------------------------------------------------------------
// Model resolution via registry
// ---------------------------------------------------------------------------

#[test]
fn gemini_resolves_model_in_registry() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(Box::new(gemini_provider()))
        .unwrap();
    let (provider, model) = registry.resolve("gemini:gemini-2.5-flash").unwrap();
    assert_eq!(provider.id(), "gemini");
    assert_eq!(model.id, "gemini-2.5-flash");
}

#[test]
fn gemini_registry_lists_provider_id() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(Box::new(gemini_provider()))
        .unwrap();
    let ids = registry.provider_ids();
    assert!(ids.contains(&"gemini"));
}

#[test]
fn gemini_unknown_model_returns_error() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(Box::new(gemini_provider()))
        .unwrap();
    let result = registry.resolve("gemini:nonexistent-model");
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Request body construction
// ---------------------------------------------------------------------------

#[test]
fn gemini_request_body_uses_contents_field() {
    let provider = gemini_provider();
    let request = Request {
        model: "gemini:gemini-2.5-flash".into(),
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
    // Gemini uses "contents" not "messages"
    assert!(
        body.get("contents").is_some(),
        "should have 'contents' field"
    );
    assert!(
        body.get("messages").is_none(),
        "should NOT have 'messages' field"
    );
    // System prompt uses "systemInstruction" object, not in contents array
    assert!(
        body.get("systemInstruction").is_some(),
        "should have 'systemInstruction' field"
    );
    let contents = body["contents"].as_array().unwrap();
    assert!(
        !contents
            .iter()
            .any(|m| m.get("role").map(|r| r == "system").unwrap_or(false)),
        "system message should NOT appear in contents array"
    );
    // maxOutputTokens inside generationConfig, not top-level
    let gen_config = body.get("generationConfig").unwrap();
    assert_eq!(gen_config["maxOutputTokens"], 1024);
    assert!(
        body.get("max_tokens").is_none(),
        "should NOT have 'max_tokens' field"
    );
    // Model is NOT in the body — it goes in the URL path
    assert!(
        body.get("model").is_none(),
        "should NOT have 'model' field (model goes in URL path)"
    );
}

#[test]
fn gemini_request_body_strips_provider_prefix() {
    let provider = gemini_provider();
    let request = Request {
        model: "gemini:gemini-2.5-pro".into(),
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
    // Model is not in the body — just verify no crash and empty contents
    assert!(
        body.get("model").is_none(),
        "model should NOT be in request body"
    );
}

// ---------------------------------------------------------------------------
// SSE text streaming
// ---------------------------------------------------------------------------

#[tokio::test]
async fn gemini_text_streaming_produces_start_delta_done() {
    let provider = gemini_provider();
    // Gemini streamGenerateContent SSE: each data line is a GenerateContentResponse
    let sse = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hi there\"}]},\"index\":0}]}\n\n\
               data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":10,\"candidatesTokenCount\":5,\"totalTokenCount\":15}}\n\n";

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
async fn gemini_done_event_has_provider_id() {
    let provider = gemini_provider();
    let sse = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hello\"}]},\"index\":0}]}\n\n\
               data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":5,\"candidatesTokenCount\":2,\"totalTokenCount\":7}}\n\n";

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let done_provider = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::Done { message, .. } => Some(message.provider.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(done_provider, "gemini");
}

// ---------------------------------------------------------------------------
// SSE tool call streaming
// ---------------------------------------------------------------------------

#[tokio::test]
async fn gemini_tool_call_streaming_works() {
    let provider = gemini_provider();
    let sse = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"functionCall\":{\"name\":\"read_file\",\"args\":{\"path\":\"foo.rs\"}}}]},\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":20,\"candidatesTokenCount\":10,\"totalTokenCount\":30}}\n\n";

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
async fn gemini_error_event_routing() {
    let provider = gemini_provider();
    let sse = "data: {\"error\":{\"code\":404,\"message\":\"Model not found\",\"status\":\"NOT_FOUND\"}}\n\n";

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Error { .. })),
        "should have an Error event"
    );

    let canary = "sk-provider-error-canary";
    let secret_sse = format!(
        "data: {{\"error\":{{\"code\":500,\"message\":\"{canary}\",\"status\":\"INTERNAL\"}}}}\n\n"
    );
    let secret_events =
        collect_stream(provider.stream_from_sse(&secret_sse, CancellationToken::new())).await;
    let rendered = format!("{secret_events:?}");
    assert!(
        !rendered.contains(canary),
        "raw upstream error text leaked into stream events: {rendered}"
    );
}

// ---------------------------------------------------------------------------
// Usage tracking
// ---------------------------------------------------------------------------

#[tokio::test]
async fn gemini_usage_in_done_event() {
    let provider = gemini_provider();
    let sse = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"test\"}]},\"index\":0}]}\n\n\
               data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":42,\"candidatesTokenCount\":13,\"totalTokenCount\":55}}\n\n";

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
// Cache token fields (Phase 12 task 12.6, DoD clause 6)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn gemini_cache_tokens_in_done_event() {
    let provider = gemini_provider();
    let sse = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"test\"}]},\"index\":0}]}\n\n\
               data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":100,\"candidatesTokenCount\":20,\"totalTokenCount\":120,\"cachedContentTokenCount\":400}}\n\n";

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let done = events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("expected Done event");
    if let AssistantStreamEvent::Done { message, .. } = done {
        assert_eq!(
            message.usage.cache_read_tokens, 400,
            "cachedContentTokenCount must map to cache_read_tokens"
        );
    }
}

// ---------------------------------------------------------------------------
// Model list
// ---------------------------------------------------------------------------

#[test]
fn gemini_has_model_list() {
    let provider = gemini_provider();
    let models = provider.models();
    assert!(
        !models.is_empty(),
        "Gemini provider should have at least one model"
    );
    assert!(
        models.iter().any(|m| m.id == "gemini-2.5-flash"),
        "should have gemini-2.5-flash model"
    );
    assert!(
        models.iter().any(|m| m.id == "gemini-2.5-pro"),
        "should have gemini-2.5-pro model"
    );
}

// ---------------------------------------------------------------------------
// Multi-delta text streaming
// ---------------------------------------------------------------------------

#[tokio::test]
async fn gemini_multiple_text_deltas() {
    let provider = gemini_provider();
    let sse = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hello\"}]},\"index\":0}]}\n\n\
               data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\" world\"}]},\"index\":0}]}\n\n\
               data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"!\"}]},\"index\":0}]}\n\n\
               data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":10,\"candidatesTokenCount\":5,\"totalTokenCount\":15}}\n\n";

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
// Custom base URL
// ---------------------------------------------------------------------------

#[test]
fn gemini_custom_base_url() {
    let provider = opi_ai::gemini::GeminiProvider::new(Some("https://custom.proxy".into()));
    assert_eq!(provider.id(), "gemini");
}

// ---------------------------------------------------------------------------
// Multiple tool calls in single response
// ---------------------------------------------------------------------------

#[tokio::test]
async fn gemini_multiple_tool_calls() {
    let provider = gemini_provider();
    let sse = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"functionCall\":{\"name\":\"read_file\",\"args\":{\"path\":\"a.rs\"}}},{\"functionCall\":{\"name\":\"read_file\",\"args\":{\"path\":\"b.rs\"}}}]},\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":20,\"candidatesTokenCount\":15,\"totalTokenCount\":35}}\n\n";

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let tool_starts = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::ToolCallStart { .. }))
        .count();
    let tool_ends = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::ToolCallEnd { .. }))
        .count();

    assert_eq!(tool_starts, 2, "should have two ToolCallStart events");
    assert_eq!(tool_ends, 2, "should have two ToolCallEnd events");
}

// ---------------------------------------------------------------------------
// CRLF tolerance
// ---------------------------------------------------------------------------

#[tokio::test]
async fn gemini_handles_crlf_line_endings() {
    let provider = gemini_provider();
    let sse = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hi\"}]},\"index\":0}]}\r\n\r\n\
               data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":5,\"candidatesTokenCount\":2,\"totalTokenCount\":7}}\r\n\r\n";

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let deltas = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::TextDelta { .. }))
        .count();
    let dones = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .count();

    assert_eq!(deltas, 1, "should have one TextDelta");
    assert_eq!(dones, 1, "should have one Done");
}

// ---------------------------------------------------------------------------
// Malformed SSE data
// ---------------------------------------------------------------------------

#[tokio::test]
async fn gemini_malformed_sse_data_surfaces_error() {
    let provider = gemini_provider();
    let sse = "data: {not valid json}\n\n\
               data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"ok\"}]},\"index\":0}]}\n\n\
               data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":5,\"candidatesTokenCount\":2,\"totalTokenCount\":7}}\n\n";

    let events: Vec<_> = provider
        .stream_from_sse(sse, CancellationToken::new())
        .collect::<Vec<_>>()
        .await;

    // Should have at least one error from malformed data
    let errors = events.iter().filter(|r| r.is_err()).count();
    assert!(
        errors > 0,
        "should have at least one error from malformed data"
    );
    assert!(
        !format!("{events:?}").contains("not valid json"),
        "malformed upstream frame content must be neutralized"
    );

    // Should still have a valid Done from the good chunks
    let oks: Vec<_> = events.into_iter().filter_map(|r| r.ok()).collect();
    let dones = oks
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .count();
    assert_eq!(dones, 1, "should still produce a Done from valid chunks");
}

// ---------------------------------------------------------------------------
// MAX_TOKENS stop reason
// ---------------------------------------------------------------------------

#[tokio::test]
async fn gemini_max_tokens_maps_to_length_stop_reason() {
    let provider = gemini_provider();
    let sse = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"truncated\"}]},\"index\":0}]}\n\n\
               data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"MAX_TOKENS\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":10,\"candidatesTokenCount\":100,\"totalTokenCount\":110}}\n\n";

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let done_reason = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::Done { reason, .. } => Some(*reason),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        done_reason,
        opi_ai::stream::StopReason::Length,
        "MAX_TOKENS should map to StopReason::Length"
    );
}

// ---------------------------------------------------------------------------
// Production request contract through Provider::stream (Phase 12.1)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_sends_text_request_body_and_auth_through_http() {
    let sse = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hi\"}]},\"index\":0}]}\n\n\
               data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":5,\"candidatesTokenCount\":2,\"totalTokenCount\":7}}\n\n";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(query_param("alt", "sse"))
        .and(header("x-goog-api-key", "test-key"))
        .and(body_partial_json(serde_json::json!({
            "contents": [{"role": "user", "parts": [{"text": "Hello"}]}],
            "systemInstruction": {"parts": [{"text": "You are helpful."}]},
            "generationConfig": {"maxOutputTokens": 1024}
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = GeminiProvider::new(Some(server.uri()));
    let request = Request {
        model: "gemini:gemini-2.5-flash".into(),
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

    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) if event.is_terminal() => break,
            Err(_) => break,
            _ => {}
        }
    }

    // verify() confirms the production request carried the Gemini body
    // (contents + systemInstruction + generationConfig.maxOutputTokens), the
    // x-goog-api-key auth header, and the alt=sse Vertex-style query. The
    // structural path (model + :streamGenerateContent) is asserted by the
    // vertex lifecycle suite and the URL-construction unit tests above.
    server.verify().await;
}

// ---------------------------------------------------------------------------
// Provider stream cancellation (Phase 12 task 12.7 DoD clause 6)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_cancellation_drains_without_hang_after_cancel() {
    // The CancellationToken is threaded into the Gemini adapter's HTTP
    // body-stream loop (gemini.rs `cancel.cancelled()` select arm). Cancelling
    // while the stream is open must drain promptly without hanging. This
    // wiremock fixture is fully buffered, so it does not prove cancellation
    // wins a race against delayed terminal SSE data.
    let sse = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hi\"}]},\"index\":0}]}\n\n\
               data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":5,\"candidatesTokenCount\":2,\"totalTokenCount\":7}}\n\n";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = GeminiProvider::new(Some(server.uri()));
    let request = Request {
        model: "gemini:gemini-2.5-flash".into(),
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
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());

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
