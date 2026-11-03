//! Google Gemini provider fixture tests (task 2.4).
//!
//! Verifies: SSE event mapping, text streaming, tool call streaming,
//! usage tracking, error handling, model resolution, and request body
//! construction for the Google Gemini `streamGenerateContent` API.

use futures_util::StreamExt;
use opi_ai::gemini::GeminiProvider;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{
    CacheRetention, EventStream, Provider, ProviderError, Request, ThinkingConfig,
};
use opi_ai::registry::ProviderRegistry;
use opi_ai::stream::AssistantStreamEvent;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: create a Gemini provider.
fn direct_cause(error: &ProviderError) -> &ProviderError {
    match error {
        ProviderError::StreamFailure(failure) => failure.cause(),
        cause => cause,
    }
}

fn gemini_provider() -> GeminiProvider {
    GeminiProvider::new(None)
}

/// Helper: collect stream events asynchronously.
async fn collect_stream(stream: EventStream) -> Vec<AssistantStreamEvent> {
    stream.filter_map(|r| async move { r.ok() }).collect().await
}

#[tokio::test]
async fn auth_error_bodies_are_absent_from_public_errors() {
    let canaries = [
        "gemini-access-canary-with-no-known-token-shape",
        "gemini-secret-canary-with-no-known-token-shape",
        "gemini-session-canary-with-no-known-token-shape",
        "gemini-token-canary-with-no-known-token-shape",
    ];
    let body = canaries.join(" ");

    for status in [401, 403] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body.clone()))
            .mount(&server)
            .await;
        let provider = GeminiProvider::new(Some(server.uri()));
        let mut stream =
            provider.stream_prepared(gemini_http_request(), opi_ai::test_support::resolved_auth());
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
                "Gemini HTTP {status} body leaked through ProviderError: {rendered}"
            );
        }
    }
}

#[tokio::test]
async fn direct_auth_statuses_do_not_wait_for_stalled_bodies() {
    for status in [401, 403] {
        let (server, headers_flushed) = spawn_stalled_gemini_error_body_server(status).await;
        let provider = GeminiProvider::new(Some(server));
        let mut stream =
            provider.stream_prepared(gemini_http_request(), opi_ai::test_support::resolved_auth());

        assert!(futures_util::poll!(stream.next()).is_pending());
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            headers_flushed.notified(),
        )
        .await
        .expect("Gemini auth-error headers must be flushed before the body stalls");

        let result = tokio::time::timeout(std::time::Duration::from_millis(500), stream.next())
            .await
            .expect("Gemini direct auth status must not wait for its body")
            .expect("Gemini direct auth status must produce a stream item");

        let error = result.expect_err("Gemini direct auth status must fail");
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
        .respond_with(ResponseTemplate::new(400).set_body_string(body))
        .mount(&server)
        .await;
    let provider = GeminiProvider::new(Some(server.uri()));
    let mut stream =
        provider.stream_prepared(gemini_http_request(), opi_ai::test_support::resolved_auth());

    let error = stream
        .next()
        .await
        .expect("oversized Gemini error body must produce a stream item")
        .expect_err("oversized Gemini error body must fail");

    assert!(matches!(
        direct_cause(&error),
        opi_ai::provider::ProviderError::ProviderSide(_)
    ));
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
        projection: Default::default(),
        transport: Default::default(),
        model: "gemini:gemini-2.5-flash".into(),
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
        projection: Default::default(),
        transport: Default::default(),
        model: "gemini:gemini-2.5-pro".into(),
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

    let canary = "sk-provider-error-canary";
    let secret_sse = format!(
        "data: {{\"error\":{{\"code\":500,\"message\":\"{canary}\",\"status\":\"INTERNAL\"}}}}\n\n"
    );
    let secret_items: Vec<Result<AssistantStreamEvent, ProviderError>> = provider
        .stream_from_sse(&secret_sse, CancellationToken::new())
        .collect()
        .await;
    let rendered = format!("{secret_items:?}");
    assert!(
        !rendered.contains(canary),
        "raw upstream error text leaked into stream items: {rendered}"
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
        // The optional child count retains its absence under RDF-07.
        assert_eq!(message.usage.reasoning_tokens, None);
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
        .expect(1)
        .mount(&server)
        .await;

    let provider = GeminiProvider::new(Some(server.uri()));
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "gemini:gemini-2.5-flash".into(),
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

    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let mut saw_done = false;
    while let Some(result) = stream.next().await {
        match result.expect("matched text request must stream successfully") {
            AssistantStreamEvent::Done { reason, message } => {
                assert!(!saw_done, "text request must have one terminal event");
                assert_eq!(reason, opi_ai::stream::StopReason::Stop);
                // Independent totals and absent child counts retain their wire facts.
                assert_eq!(
                    message.usage,
                    opi_ai::stream::Usage::reported(5, 2, 0, 0, None, None)
                        .with_total_tokens(Some(7)),
                    "HTTP usage must retain the fixture's exact token values"
                );
                saw_done = true;
            }
            AssistantStreamEvent::Error { .. } => {
                panic!("text fixture must complete with Done");
            }
            _ => {}
        }
    }
    assert!(saw_done, "text fixture must produce a terminal Done event");

    // verify() confirms the production request carried the Gemini body
    // (contents + systemInstruction + generationConfig.maxOutputTokens), the
    // x-goog-api-key auth header, and the alt=sse Vertex-style query. The
    // structural path (model + :streamGenerateContent) is asserted by the
    // vertex lifecycle suite and the URL-construction unit tests above.
    server.verify().await;
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn request_enrichment_reaches_gemini_http_boundary() {
    let sse = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hi\"}]},\"finishReason\":\"STOP\",\"index\":0}]}\n\n";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("x-goog-api-key", "test-key"))
        .and(header("content-type", "application/json"))
        .and(header("x-opi-request", "gemini"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = GeminiProvider::new(Some(server.uri()));
    let mut request = gemini_http_request();
    request.extra_headers = vec![("X-Opi-Request".into(), "gemini".into())];

    let events =
        collect_stream(provider.stream_prepared(request, opi_ai::test_support::resolved_auth()))
            .await;

    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::Done { .. }))
    );
    server.verify().await;
}

#[tokio::test]
async fn request_timeout_maps_to_typed_timeout_at_gemini_boundary() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(200)))
        .mount(&server)
        .await;
    let provider = GeminiProvider::new(Some(server.uri()));
    let mut request = gemini_http_request();
    request.timeout = Some(std::time::Duration::from_millis(20));
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());

    let result = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("Gemini request timeout must resolve promptly")
        .expect("Gemini timeout must produce a stream item");

    assert!(matches!(
        result.as_ref().err().map(direct_cause),
        Some(opi_ai::provider::ProviderError::Timeout)
    ));
}

#[tokio::test]
async fn stalled_embedded_auth_body_uses_a_fixed_default_deadline() {
    let (server, headers_flushed) = spawn_stalled_gemini_error_body_server(400).await;
    let provider = GeminiProvider::new(Some(server));
    let mut stream =
        provider.stream_prepared(gemini_http_request(), opi_ai::test_support::resolved_auth());

    assert!(futures_util::poll!(stream.next()).is_pending());
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        headers_flushed.notified(),
    )
    .await
    .expect("Gemini error headers must be flushed before the body stalls");

    let result = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("Gemini error-body timeout must resolve promptly")
        .expect("Gemini error-body timeout must produce a stream item");

    assert!(matches!(
        result.as_ref().err().map(direct_cause),
        Some(opi_ai::provider::ProviderError::Timeout)
    ));
}

#[tokio::test]
async fn request_header_cannot_override_gemini_auth_routing() {
    let server = MockServer::start().await;
    let provider = GeminiProvider::new(Some(server.uri()));
    let mut request = gemini_http_request();
    request.extra_headers = vec![("x-goog-api-key".into(), "override".into())];
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());

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
        "reserved Gemini request headers must fail before dispatch"
    );
}

fn gemini_http_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "gemini:gemini-2.5-flash".into(),
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
        callbacks: Default::default(),
    }
}

async fn spawn_stalled_gemini_error_body_server(
    status: u16,
) -> (String, std::sync::Arc<tokio::sync::Notify>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled Gemini error server");
    let addr = listener.local_addr().expect("stalled error server addr");
    let headers_flushed = std::sync::Arc::new(tokio::sync::Notify::new());
    let server_headers_flushed = headers_flushed.clone();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept Gemini request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read Gemini request");
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
            .expect("write Gemini error headers");
        tokio::io::AsyncWriteExt::flush(&mut socket)
            .await
            .expect("flush Gemini error headers");
        server_headers_flushed.notify_one();
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    });

    (format!("http://{addr}"), headers_flushed)
}

#[derive(Clone, Copy)]
enum GeminiStallPoint {
    BeforeHeaders,
    ResponseBody,
    ErrorBody,
}

async fn spawn_stalled_gemini_server(
    stall_point: GeminiStallPoint,
) -> (String, std::sync::Arc<tokio::sync::Notify>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled Gemini server");
    let addr = listener.local_addr().expect("stalled Gemini server addr");
    let stalled = std::sync::Arc::new(tokio::sync::Notify::new());
    let server_stalled = stalled.clone();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept Gemini request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read Gemini request");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        if !matches!(stall_point, GeminiStallPoint::BeforeHeaders) {
            let status = if matches!(stall_point, GeminiStallPoint::ErrorBody) {
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
            .expect("write Gemini response headers");
            tokio::io::AsyncWriteExt::flush(&mut socket)
                .await
                .expect("flush Gemini response headers");
        }
        server_stalled.notify_one();
        std::future::pending::<()>().await;
    });

    (format!("http://{addr}"), stalled)
}

// ---------------------------------------------------------------------------
// Provider stream cancellation (Phase 12 task 12.7 DoD clause 6)
// ---------------------------------------------------------------------------

async fn assert_gemini_cancelled(stall_point: GeminiStallPoint) {
    let (server, stalled) = spawn_stalled_gemini_server(stall_point).await;
    let cancel = CancellationToken::new();
    let provider = GeminiProvider::new(Some(server));
    let mut request = gemini_http_request();
    request.cancel = cancel.clone();
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());

    assert!(futures_util::poll!(stream.next()).is_pending());
    tokio::time::timeout(std::time::Duration::from_secs(1), stalled.notified())
        .await
        .expect("Gemini server must reach the selected stall point");
    cancel.cancel();

    let remaining = tokio::time::timeout(std::time::Duration::from_secs(1), async move {
        let mut remaining = Vec::new();
        while let Some(item) = stream.next().await {
            remaining.push(item);
        }
        remaining
    })
    .await
    .expect("Gemini cancellation must terminate without waiting for HTTP");
    assert!(
        matches!(
            remaining.as_slice(),
            [Err(cause)] if cause.category() == opi_ai::provider::ProviderErrorCategory::Cancelled
        ),
        "Gemini cancellation must yield exactly one typed error, got {remaining:?}"
    );
}

#[tokio::test]
async fn cancellation_before_response_headers_is_typed_and_prompt() {
    assert_gemini_cancelled(GeminiStallPoint::BeforeHeaders).await;
}

#[tokio::test]
async fn cancellation_during_response_body_is_typed_and_prompt() {
    assert_gemini_cancelled(GeminiStallPoint::ResponseBody).await;
}

#[tokio::test]
async fn cancellation_during_error_body_is_typed_and_prompt() {
    assert_gemini_cancelled(GeminiStallPoint::ErrorBody).await;
}

// ---------------------------------------------------------------------------
// Task 20.51: callbacks at the Google wire — payload flows, observers typed
// unsupported (P20-S51)
// ---------------------------------------------------------------------------

use opi_ai::callbacks::RequestCallbacks;

#[tokio::test]
async fn gemini_payload_callback_applies_to_wire_body() {
    let sse = "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hi\"}]},\"finishReason\":\"STOP\",\"index\":0}]}\n\n";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::body_partial_json(
            serde_json::json!({"gemini_marker": "replaced"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = GeminiProvider::new(Some(server.uri()));
    let mut request = gemini_http_request();
    request.callbacks = RequestCallbacks::new().with_on_payload(std::sync::Arc::new(|body| {
        let mut replacement = body.clone();
        replacement["gemini_marker"] = serde_json::json!("replaced");
        Ok(Some(replacement))
    }));

    let events =
        collect_stream(provider.stream_prepared(request, opi_ai::test_support::resolved_auth()))
            .await;
    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::Done { .. }))
    );
    server.verify().await;
}

#[tokio::test]
async fn gemini_response_callback_is_typed_unsupported_before_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let provider = GeminiProvider::new(Some(server.uri()));
    let mut request = gemini_http_request();
    let callbacks = RequestCallbacks::new();
    request.callbacks = callbacks.with_on_response(std::sync::Arc::new(|_status, _headers| Ok(())));

    let expected_prefix = request.context.messages().len();
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let error = stream.next().await.expect("typed refusal").unwrap_err();
    let ProviderError::StreamFailure(failure) = error else {
        panic!("accepted projection retains failure facts")
    };
    assert!(matches!(
        failure.cause(),
        ProviderError::UnsupportedCapability(_)
    ));
    assert_eq!(
        failure
            .message()
            .projection
            .as_ref()
            .unwrap()
            .source_prefix
            .message_count,
        expected_prefix
    );
    assert!(stream.next().await.is_none());
    server.verify().await;
}

#[tokio::test]
async fn gemini_raw_callbacks_observe_unmodified_chunk_before_normalized_events() {
    let raw = serde_json::json!({"unknown_wire_field":"retained", "candidates":[{"content":{"parts":[{"text":"Hi"}]},"finishReason":"STOP"}]});
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!("data: {raw}\n\n")),
        )
        .mount(&server)
        .await;
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let mut request = gemini_http_request();
    request.callbacks = RequestCallbacks::new().with_on_raw(std::sync::Arc::new(move |value| {
        recorder.lock().unwrap().push(value.clone());
        Ok(())
    }));
    let provider = GeminiProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let mut done = false;
    while let Some(item) = stream.next().await {
        let event = item.expect("raw observer is supported");
        assert_eq!(seen.lock().unwrap().as_slice(), std::slice::from_ref(&raw));
        done |= matches!(event, AssistantStreamEvent::Done { .. });
    }
    assert!(done);
}

#[tokio::test]
async fn separate_gemini_calls_reusing_callbacks_freeze_independently() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).insert_header("content-type", "text/event-stream").set_body_string("data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hi\"}]},\"finishReason\":\"STOP\"}]}\n\n")).expect(2).mount(&server).await;
    let payload_calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = payload_calls.clone();
    let callbacks = RequestCallbacks::new().with_on_payload(std::sync::Arc::new(move |_| {
        count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(None)
    }));
    let provider = GeminiProvider::new(Some(server.uri()));
    for text in ["first", "second"] {
        let mut request = gemini_http_request();
        request.callbacks = callbacks.clone();
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text { text: text.into() }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .unwrap();
        opi_ai::stream::ChatStream::new(
            provider.stream_prepared(request, opi_ai::test_support::resolved_auth()),
        )
        .result()
        .await
        .expect("separate call succeeds");
    }
    let requests = server.received_requests().await.unwrap();
    assert_eq!(payload_calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_ne!(
        requests[0].body, requests[1].body,
        "second call must retain its own input"
    );
}

#[tokio::test]
async fn async_google_raw_observation_blocks_normalized_delivery_until_ready() {
    let raw = serde_json::json!({"wire_only":"retained", "candidates":[{"content":{"parts":[{"text":"Hi"}]},"finishReason":"STOP"}]});
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!("data: {raw}\n\n")),
        )
        .mount(&server)
        .await;
    let entered = std::sync::Arc::new(tokio::sync::Notify::new());
    let release = std::sync::Arc::new(tokio::sync::Notify::new());
    let marker = entered.clone();
    let gate = release.clone();
    let mut request = gemini_http_request();
    request.callbacks = RequestCallbacks::new().with_async_on_raw(move |value| {
        let marker = marker.clone();
        let gate = gate.clone();
        async move {
            assert_eq!(value["wire_only"], "retained");
            marker.notify_one();
            gate.notified().await;
            Ok(())
        }
    });
    let provider = GeminiProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    assert!(futures_util::poll!(stream.next()).is_pending());
    tokio::time::timeout(std::time::Duration::from_secs(1), entered.notified())
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(10), stream.next())
            .await
            .is_err()
    );
    release.notify_one();
    opi_ai::stream::ChatStream::new(stream)
        .result()
        .await
        .expect("released observer allows final output");
}

#[tokio::test]
async fn cancelled_async_payload_is_dropped_without_http_dispatch() {
    struct NotifyDrop(std::sync::Arc<tokio::sync::Notify>);
    impl Drop for NotifyDrop {
        fn drop(&mut self) {
            self.0.notify_one();
        }
    }
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let entered = std::sync::Arc::new(tokio::sync::Notify::new());
    let dropped = std::sync::Arc::new(tokio::sync::Notify::new());
    let marker = entered.clone();
    let release = dropped.clone();
    let mut request = gemini_http_request();
    let cancel = request.cancel.clone();
    request.callbacks = RequestCallbacks::new().with_async_on_payload(move |_| {
        let marker = marker.clone();
        let release = release.clone();
        async move {
            let _drop = NotifyDrop(release);
            marker.notify_one();
            std::future::pending::<()>().await;
            Ok(None)
        }
    });
    let provider = GeminiProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    assert!(futures_util::poll!(stream.next()).is_pending());
    tokio::time::timeout(std::time::Duration::from_secs(1), entered.notified())
        .await
        .unwrap();
    cancel.cancel();
    let failure = opi_ai::stream::ChatStream::new(stream)
        .result()
        .await
        .unwrap_err();
    assert_eq!(
        failure.cause().category(),
        opi_ai::provider::ProviderErrorCategory::Cancelled
    );
    tokio::time::timeout(std::time::Duration::from_secs(1), dropped.notified())
        .await
        .unwrap();
    server.verify().await;
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
            model: "gemini:gemini-2.5-flash".into(),
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
        let provider = gemini_provider();
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
        let provider = gemini_provider();
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
// Task 20.40 (DEL-11): google-generative-ai complete chat adapter.
//
// Native options inventory (pi GoogleOptions): toolChoice auto/none/any and
// thinking { enabled, budgetTokens (-1 dynamic, 0 disabled), level }; the
// level-vs-budget wire mode follows the pinned pi model regexes; tool
// declarations ride the original JSON Schema (`parametersJsonSchema`) and
// Gemini 3+ strict tools select VALIDATED function calling.
// ---------------------------------------------------------------------------

mod google_40 {
    use super::*;
    use opi_ai::WireApi;
    use opi_ai::gemini::{
        GeminiProvider, GoogleApiThinkingLevel, GoogleGenerativeAiChatApi,
        GoogleGenerativeAiOptions, GoogleThinkingConfig, GoogleToolChoice,
    };
    use opi_ai::message::{AssistantContent, OutputContent, ToolCall, ToolDef, ToolResultMessage};
    use opi_ai::model_info::{ModelCapabilities, ModelInfo, ThinkingLevel, ThinkingLevelMapping};
    use opi_ai::stream::StopReason;
    use opi_ai::transcript::Context;

    fn model_40(id: &str) -> ModelInfo {
        ModelInfo::new(
            id,
            id,
            WireApi::GoogleGenerativeAi,
            ModelCapabilities::new(1_000_000, 65_536)
                .with_images(true)
                .with_streaming(true)
                .with_thinking(true),
        )
    }

    fn plain_request_40(model_id: &str, thinking: ThinkingConfig) -> Request {
        Request {
            projection: Default::default(),
            transport: Default::default(),
            model: format!("gemini:{model_id}"),
            context: opi_ai::transcript::normalize_context(Context {
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
            cancel: CancellationToken::new(),
            timeout: None,
            extra_headers: vec![],
            cache_retention: CacheRetention::None,
            session_id: None,
            telemetry_context: None,
            callbacks: Default::default(),
        }
    }

    fn native_40(
        thinking: Option<GoogleThinkingConfig>,
        tool_choice: Option<GoogleToolChoice>,
    ) -> GoogleGenerativeAiOptions {
        GoogleGenerativeAiOptions::native(thinking, tool_choice)
    }

    fn body_40(provider: &GeminiProvider, request: &Request) -> serde_json::Value {
        provider
            .build_request_body(request)
            .expect("google body compiles")
    }

    fn thinking_config_of(body: &serde_json::Value) -> &serde_json::Value {
        body["generationConfig"]
            .get("thinkingConfig")
            .expect("thinkingConfig is present")
    }

    #[test]
    fn google_native_level_mode_uses_thinking_level_enum() {
        let provider = GeminiProvider::new(None)
            .with_models(vec![model_40("gemini-3-pro-preview")])
            .with_native_options(native_40(
                Some(GoogleThinkingConfig {
                    enabled: true,
                    budget_tokens: None,
                    level: Some(GoogleApiThinkingLevel::High),
                }),
                None,
            ));
        let body = body_40(
            &provider,
            &plain_request_40("gemini-3-pro-preview", ThinkingConfig::default()),
        );
        assert_eq!(
            thinking_config_of(&body),
            &serde_json::json!({
                "includeThoughts": true,
                "thinkingLevel": "HIGH",
            })
        );
    }

    #[test]
    fn google_native_budget_mode_sends_budget_with_dynamic_minus_one() {
        for (budget, expected) in [(-1i64, -1i64), (0, 0), (2048, 2048)] {
            let provider = GeminiProvider::new(None).with_native_options(native_40(
                Some(GoogleThinkingConfig {
                    enabled: true,
                    budget_tokens: Some(budget),
                    level: None,
                }),
                None,
            ));
            let body = body_40(
                &provider,
                &plain_request_40("gemini-2.5-flash", ThinkingConfig::default()),
            );
            assert_eq!(
                thinking_config_of(&body),
                &serde_json::json!({
                    "includeThoughts": true,
                    "thinkingBudget": expected,
                }),
                "budget {budget}"
            );
        }
    }

    #[test]
    fn google_native_level_takes_precedence_over_budget() {
        let provider = GeminiProvider::new(None)
            .with_models(vec![model_40("gemini-3-flash-preview")])
            .with_native_options(native_40(
                Some(GoogleThinkingConfig {
                    enabled: true,
                    budget_tokens: Some(4096),
                    level: Some(GoogleApiThinkingLevel::Low),
                }),
                None,
            ));
        let body = body_40(
            &provider,
            &plain_request_40("gemini-3-flash-preview", ThinkingConfig::default()),
        );
        let config = thinking_config_of(&body);
        assert_eq!(config.get("thinkingLevel"), Some(&serde_json::json!("LOW")));
        assert!(
            config.get("thinkingBudget").is_none(),
            "budget is not sent alongside a level"
        );
    }

    #[test]
    fn google_native_disabled_budget_mode_sends_zero() {
        let provider = GeminiProvider::new(None).with_native_options(native_40(
            Some(GoogleThinkingConfig {
                enabled: false,
                budget_tokens: None,
                level: None,
            }),
            None,
        ));
        let body = body_40(
            &provider,
            &plain_request_40("gemini-2.5-flash", ThinkingConfig::default()),
        );
        assert_eq!(
            thinking_config_of(&body),
            &serde_json::json!({ "thinkingBudget": 0 })
        );
    }

    #[test]
    fn google_native_disabled_level_mode_uses_off_fallback_level() {
        // Without an off mapping the pi fallback is budget zero.
        let provider = GeminiProvider::new(None)
            .with_models(vec![model_40("gemini-3-pro-preview")])
            .with_native_options(native_40(
                Some(GoogleThinkingConfig {
                    enabled: false,
                    budget_tokens: None,
                    level: None,
                }),
                None,
            ));
        let body = body_40(
            &provider,
            &plain_request_40("gemini-3-pro-preview", ThinkingConfig::default()),
        );
        assert_eq!(
            thinking_config_of(&body),
            &serde_json::json!({ "thinkingBudget": 0 })
        );

        // A mapped off level takes the level wire instead.
        let model = model_40("gemini-3-pro-preview").with_thinking_level_map(
            opi_ai::model_info::ThinkingLevelMap::reasoning_default().with_mapping(
                ThinkingLevel::None,
                ThinkingLevelMapping::Mapped("minimal".into()),
            ),
        );
        let provider = GeminiProvider::new(None)
            .with_models(vec![model])
            .with_native_options(native_40(
                Some(GoogleThinkingConfig {
                    enabled: false,
                    budget_tokens: None,
                    level: None,
                }),
                None,
            ));
        let body = body_40(
            &provider,
            &plain_request_40("gemini-3-pro-preview", ThinkingConfig::default()),
        );
        assert_eq!(
            thinking_config_of(&body),
            &serde_json::json!({ "thinkingLevel": "MINIMAL" })
        );
    }

    #[test]
    fn google_native_omitted_thinking_sends_no_config() {
        let provider = GeminiProvider::new(None).with_native_options(native_40(None, None));
        let body = body_40(
            &provider,
            &plain_request_40("gemini-2.5-pro", ThinkingConfig::default()),
        );
        assert!(
            body["generationConfig"].get("thinkingConfig").is_none(),
            "native omission leaves the wire without thinkingConfig: {body}"
        );
    }

    #[test]
    fn google_native_overrides_request_thinking() {
        let provider = GeminiProvider::new(None).with_native_options(native_40(
            Some(GoogleThinkingConfig {
                enabled: true,
                budget_tokens: Some(2048),
                level: None,
            }),
            None,
        ));
        let body = body_40(
            &provider,
            &plain_request_40(
                "gemini-2.5-flash",
                ThinkingConfig {
                    enabled: false,
                    budget_tokens: None,
                    level: ThinkingLevel::None,
                },
            ),
        );
        assert_eq!(
            thinking_config_of(&body).get("includeThoughts"),
            Some(&serde_json::json!(true))
        );
    }

    fn tool_request_40(model_id: &str, tools: Vec<ToolDef>) -> Request {
        let mut request = plain_request_40(model_id, ThinkingConfig::default());
        request.context = opi_ai::transcript::normalize_context(Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text { text: "go".into() }],
                timestamp_ms: 0,
            })],
            tools,
        })
        .expect("valid context");
        request
    }

    fn plain_tool_40() -> ToolDef {
        ToolDef {
            name: "lookup".into(),
            description: "look".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "note": {"type": "string"},
                },
                "required": ["path"],
            }),
            constrained_sampling: None,
        }
    }

    #[test]
    fn google_native_tool_choice_modes_reach_the_wire() {
        for (choice, mode) in [
            (GoogleToolChoice::Auto, "AUTO"),
            (GoogleToolChoice::None_, "NONE"),
            (GoogleToolChoice::Any, "ANY"),
        ] {
            let provider =
                GeminiProvider::new(None).with_native_options(native_40(None, Some(choice)));
            let body = body_40(
                &provider,
                &tool_request_40("gemini-2.5-flash", vec![plain_tool_40()]),
            );
            assert_eq!(
                body.pointer("/toolConfig/functionCallingConfig/mode"),
                Some(&serde_json::json!(mode)),
                "tool choice {mode}"
            );
        }
    }

    #[test]
    fn google_no_tool_choice_without_strict_sends_no_mode() {
        let provider = GeminiProvider::new(None);
        let body = body_40(
            &provider,
            &tool_request_40("gemini-2.5-flash", vec![plain_tool_40()]),
        );
        assert!(
            body.get("toolConfig").is_none(),
            "plain tools carry no forced mode: {body}"
        );
    }

    #[test]
    fn google_tools_carry_the_original_schema_json() {
        let tool = ToolDef {
            name: "shaped".into(),
            description: "d".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pick": {
                        "anyOf": [
                            {"type": "string"},
                            {"type": "number"},
                        ]
                    }
                },
                "required": ["pick"],
            }),
            constrained_sampling: None,
        };
        let provider = GeminiProvider::new(None);
        let body = body_40(&provider, &tool_request_40("gemini-2.5-flash", vec![tool]));
        let declaration = &body["tools"][0]["functionDeclarations"][0];
        assert!(
            declaration.get("parameters").is_none(),
            "legacy OpenAPI parameters stay off the public Google wire"
        );
        let schema = declaration
            .get("parametersJsonSchema")
            .expect("original schema rides parametersJsonSchema");
        assert!(
            schema["properties"]["pick"].get("anyOf").is_some(),
            "original schema survives unmodified: {schema}"
        );
    }

    #[test]
    fn google_gemini3_strict_prefer_selects_validated_mode() {
        use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
        let mut tool = plain_tool_40();
        tool.constrained_sampling = Some(ConstrainedSampling::JsonSchema {
            strict: StrictPreference::Prefer,
        });
        let provider = GeminiProvider::new(None);
        let body = body_40(
            &provider,
            &tool_request_40("gemini-3-flash-preview", vec![tool]),
        );
        assert_eq!(
            body.pointer("/toolConfig/functionCallingConfig/mode"),
            Some(&serde_json::json!("VALIDATED"))
        );
        let schema = &body["tools"][0]["functionDeclarations"][0]
            .get("parametersJsonSchema")
            .expect("strict projection keeps the original-schema carrier");
        assert_eq!(
            schema["properties"]["note"]["anyOf"],
            serde_json::json!([
                {"type": "string"},
                {"type": "null"},
            ]),
            "strict projection wraps optional properties with a null arm"
        );
        assert_eq!(
            schema["required"],
            serde_json::json!(["path", "note"]),
            "strict projection forces every property required"
        );
    }

    #[test]
    fn google_gemini2_strict_prefer_stays_downgraded() {
        // Existing wire safety (task 20.11): gemini-2 models have no
        // validated calling mode; prefer downgrades silently.
        use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
        let mut tool = plain_tool_40();
        tool.constrained_sampling = Some(ConstrainedSampling::JsonSchema {
            strict: StrictPreference::Prefer,
        });
        let provider = GeminiProvider::new(None);
        let body = body_40(&provider, &tool_request_40("gemini-2.5-flash", vec![tool]));
        assert!(
            body.get("toolConfig").is_none(),
            "gemini-2 has no validated mode and prefers silently downgrade"
        );
    }

    fn history_request_40(model_id: &str, messages: Vec<Message>) -> Request {
        let mut request = plain_request_40(model_id, ThinkingConfig::default());
        request.context = opi_ai::transcript::normalize_context(Context {
            system_prompt: None,
            messages,
            tools: vec![],
        })
        .expect("valid context");
        request
    }

    fn tool_result_40(id: &str, name: &str, text: &str, is_error: bool) -> Message {
        Message::ToolResult(ToolResultMessage {
            tool_call_id: id.into(),
            tool_name: name.into(),
            content: vec![OutputContent::Text { text: text.into() }],
            details: None,
            is_error,
            not_executed: None,
            truncated: false,
            timestamp_ms: 0,
        })
    }

    fn assistant_call_40(id: &str, name: &str) -> AssistantContent {
        AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: id.into(),
                name: name.into(),
                arguments: "{}".into(),
                arguments_error: None,
                signature: None,
            },
        }
    }

    #[test]
    fn google_function_responses_use_output_and_error_keys() {
        let messages = vec![
            Message::Assistant(opi_ai::message::AssistantMessage {
                content: vec![
                    assistant_call_40("c1", "lookup"),
                    assistant_call_40("c2", "lookup"),
                ],
                ..assistant_shell_40("gemini-2.5-flash")
            }),
            tool_result_40("c1", "lookup", "found it", false),
            tool_result_40("c2", "lookup", "boom", true),
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "next".into(),
                }],
                timestamp_ms: 0,
            }),
        ];
        let provider = GeminiProvider::new(None);
        let body = body_40(&provider, &history_request_40("gemini-2.5-flash", messages));
        let contents = body["contents"].as_array().expect("contents");
        // Consecutive tool results merge into one user turn (pi merges
        // function responses into a single user message).
        let merged = contents
            .iter()
            .filter(|c| {
                c["role"] == "user"
                    && c["parts"].as_array().is_some_and(|parts| {
                        parts.iter().any(|p| p.get("functionResponse").is_some())
                    })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            merged.len(),
            1,
            "consecutive function responses share one user turn: {contents:?}"
        );
        let responses: Vec<&serde_json::Value> = merged[0]["parts"]
            .as_array()
            .expect("parts")
            .iter()
            .filter_map(|p| p.get("functionResponse"))
            .collect();
        assert_eq!(responses.len(), 2);
        assert_eq!(
            responses[0]["response"],
            serde_json::json!({"output": "found it"}),
            "success uses the output key"
        );
        assert_eq!(
            responses[1]["response"],
            serde_json::json!({"error": "boom"}),
            "failure uses the error key with the text"
        );
    }

    fn assistant_shell_40(model: &str) -> opi_ai::message::AssistantMessage {
        opi_ai::message::AssistantMessage {
            projection: None,
            content: vec![],
            api: opi_ai::ApiKind::Google,
            provider: "gemini".into(),
            deferred: None,
            model: model.into(),
            response_model: None,
            wire: Some(WireApi::GoogleGenerativeAi),
            response_id: None,
            usage: Default::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp_ms: 0,
            diagnostics: vec![],
            duration_ms: None,
            raw_stop_reason: None,
            provider_thinking_level: None,
        }
    }

    #[test]
    fn google_gemini3_tool_calls_and_responses_carry_normalized_ids() {
        let messages = vec![
            Message::Assistant(opi_ai::message::AssistantMessage {
                content: vec![assistant_call_40("call id/with spaces!!", "lookup")],
                ..assistant_shell_40("gemini-3-pro-preview")
            }),
            tool_result_40("call id/with spaces!!", "lookup", "ok", false),
        ];
        let provider = GeminiProvider::new(None);
        let body = body_40(
            &provider,
            &history_request_40("gemini-3-pro-preview", messages),
        );
        let call_part = &body["contents"][0]["parts"][0]["functionCall"];
        assert_eq!(
            call_part.get("id"),
            Some(&serde_json::json!("call_id_with_spaces__")),
            "gemini-3 normalizes tool ids to [a-zA-Z0-9_-] within 64"
        );
        let response_part = &body["contents"][1]["parts"][0]["functionResponse"];
        assert_eq!(
            response_part.get("id"),
            Some(&serde_json::json!("call_id_with_spaces__"))
        );

        // Gemini 2 keeps the legacy shape: no ids on either side.
        let messages = vec![
            Message::Assistant(opi_ai::message::AssistantMessage {
                content: vec![assistant_call_40("c1", "lookup")],
                ..assistant_shell_40("gemini-2.5-flash")
            }),
            tool_result_40("c1", "lookup", "ok", false),
        ];
        let body = body_40(&provider, &history_request_40("gemini-2.5-flash", messages));
        assert!(
            body["contents"][0]["parts"][0]["functionCall"]
                .get("id")
                .is_none(),
            "gemini-2 does not require tool call ids"
        );
        assert!(
            body["contents"][1]["parts"][0]["functionResponse"]
                .get("id")
                .is_none()
        );
    }

    fn image_result_40(id: &str, name: &str) -> Message {
        Message::ToolResult(ToolResultMessage {
            tool_call_id: id.into(),
            tool_name: name.into(),
            content: vec![OutputContent::Image {
                media_type: opi_ai::message::MediaType::Png,
                source: opi_ai::message::ImageSource::Base64 {
                    data: "aGk=".into(),
                },
            }],
            details: None,
            is_error: false,
            not_executed: None,
            truncated: false,
            timestamp_ms: 0,
        })
    }

    #[test]
    fn google_gemini3_nests_images_inside_function_responses() {
        let messages = vec![
            Message::Assistant(opi_ai::message::AssistantMessage {
                content: vec![assistant_call_40("c1", "lookup")],
                ..assistant_shell_40("gemini-3-flash-preview")
            }),
            image_result_40("c1", "lookup"),
        ];
        let provider = GeminiProvider::new(None);
        let body = body_40(
            &provider,
            &history_request_40("gemini-3-flash-preview", messages),
        );
        let response = &body["contents"][1]["parts"][0]["functionResponse"];
        assert!(
            response
                .get("parts")
                .is_some_and(|p| p.as_array().is_some_and(|a| !a.is_empty())),
            "gemini-3 multimodal function responses nest image parts: {response}"
        );
        assert_eq!(
            body["contents"].as_array().expect("contents").len(),
            2,
            "no separate trailing image turn on gemini-3: {body}"
        );
    }

    #[test]
    fn google_gemini2_images_ride_a_separate_user_turn() {
        let messages = vec![
            Message::Assistant(opi_ai::message::AssistantMessage {
                content: vec![assistant_call_40("c1", "lookup")],
                ..assistant_shell_40("gemini-2.5-flash")
            }),
            image_result_40("c1", "lookup"),
        ];
        let provider = GeminiProvider::new(None);
        let body = body_40(&provider, &history_request_40("gemini-2.5-flash", messages));
        let contents = body["contents"].as_array().expect("contents");
        let image_turn = contents
            .iter()
            .find(|c| {
                c["parts"].as_array().is_some_and(|parts| {
                    parts
                        .iter()
                        .any(|p| p.get("text") == Some(&serde_json::json!("Tool result image:")))
                })
            })
            .expect("separate image turn follows the results");
        assert!(
            image_turn["parts"]
                .as_array()
                .expect("parts")
                .iter()
                .any(|p| p.get("inlineData").is_some()),
            "image data rides the separate turn"
        );
    }

    #[test]
    fn google_thought_signatures_replay_same_model_only() {
        let same = opi_ai::message::AssistantMessage {
            content: vec![
                AssistantContent::Text {
                    text: "answer".into(),
                    signature: Some("c2lnMQ==".into()),
                },
                AssistantContent::Thinking {
                    thinking: "hm".into(),
                    signature: Some("c2lnMg==".into()),
                },
                AssistantContent::ToolCall {
                    tool_call: ToolCall {
                        id: "c1".into(),
                        name: "lookup".into(),
                        arguments: "{}".into(),
                        arguments_error: None,
                        signature: Some("c2lnMw==".into()),
                    },
                },
            ],
            ..assistant_shell_40("gemini-2.5-flash")
        };
        let messages = vec![
            Message::Assistant(same),
            tool_result_40("c1", "lookup", "ok", false),
        ];
        let provider = GeminiProvider::new(None);
        let body = body_40(&provider, &history_request_40("gemini-2.5-flash", messages));
        let parts = body["contents"][0]["parts"].as_array().expect("parts");
        assert_eq!(parts.len(), 3, "text, thinking and call all ride parts");
        assert_eq!(
            parts[0].get("thoughtSignature"),
            Some(&serde_json::json!("c2lnMQ=="))
        );
        assert_eq!(parts[1].get("thought"), Some(&serde_json::json!(true)));
        assert_eq!(
            parts[1].get("thoughtSignature"),
            Some(&serde_json::json!("c2lnMg=="))
        );
        assert_eq!(
            parts[2].get("thoughtSignature"),
            Some(&serde_json::json!("c2lnMw==")),
            "the signature rides the part beside functionCall"
        );

        // Cross-model history drops all thinking and unusable signatures.
        let foreign = opi_ai::message::AssistantMessage {
            content: vec![
                AssistantContent::Text {
                    text: "answer".into(),
                    signature: Some("c2lnMQ==".into()),
                },
                AssistantContent::Thinking {
                    thinking: "hm".into(),
                    signature: Some("c2lnMg==".into()),
                },
            ],
            ..assistant_shell_40("gemini-2.5-pro")
        };
        let messages = vec![Message::Assistant(foreign)];
        let body = body_40(&provider, &history_request_40("gemini-2.5-flash", messages));
        let parts = body["contents"][0]["parts"].as_array().expect("parts");
        // Cross-identity thinking is dropped by the canonical replay
        // projection (task 20.7 contract) before this compiler runs; the
        // text part remains without its unusable signature.
        assert_eq!(parts.len(), 1);
        assert!(parts.iter().all(|p| p.get("thoughtSignature").is_none()));
        assert!(parts[0].get("thought").is_none());

        // Invalid base64 signatures never reach the wire even same-model.
        let invalid = opi_ai::message::AssistantMessage {
            content: vec![AssistantContent::Text {
                text: "answer".into(),
                signature: Some("not base64!!".into()),
            }],
            ..assistant_shell_40("gemini-2.5-flash")
        };
        let body = body_40(
            &provider,
            &history_request_40("gemini-2.5-flash", vec![Message::Assistant(invalid)]),
        );
        assert!(
            body["contents"][0]["parts"][0]
                .get("thoughtSignature")
                .is_none(),
            "non-base64 signatures are dropped"
        );

        // Empty text with a signature is preserved (pi keeps signature-only
        // parts; dropping them breaks the reasoning chain).
        let empty = opi_ai::message::AssistantMessage {
            content: vec![AssistantContent::Text {
                text: "  ".into(),
                signature: Some("c2lnNA==".into()),
            }],
            ..assistant_shell_40("gemini-2.5-flash")
        };
        let body = body_40(
            &provider,
            &history_request_40("gemini-2.5-flash", vec![Message::Assistant(empty)]),
        );
        assert!(
            body["contents"][0]["parts"][0]
                .get("thoughtSignature")
                .is_some(),
            "a signature-only part survives"
        );
    }

    #[test]
    fn google_neutral_unresolvable_level_fails_typed() {
        let model = model_40("gemini-2.5-flash")
            .with_thinking_level_map(opi_ai::model_info::ThinkingLevelMap::disabled());
        let provider = GeminiProvider::new(None).with_models(vec![model]);
        let error = provider
            .build_request_body(&plain_request_40(
                "gemini-2.5-flash",
                ThinkingConfig {
                    enabled: true,
                    budget_tokens: None,
                    level: ThinkingLevel::Medium,
                },
            ))
            .expect_err("unsupported mapping fails before dispatch");
        assert!(
            matches!(error, ProviderError::Config { .. }),
            "got {error:?}"
        );
    }

    #[test]
    fn google_neutral_level_mode_projects_resolved_level() {
        let provider =
            GeminiProvider::new(None).with_models(vec![model_40("gemini-3-pro-preview")]);
        let body = body_40(
            &provider,
            &plain_request_40(
                "gemini-3-pro-preview",
                ThinkingConfig {
                    enabled: true,
                    budget_tokens: None,
                    level: ThinkingLevel::Medium,
                },
            ),
        );
        assert_eq!(
            thinking_config_of(&body),
            &serde_json::json!({
                "includeThoughts": true,
                "thinkingLevel": "MEDIUM",
            })
        );
    }

    #[test]
    fn google_neutral_budget_mode_uses_model_tables_and_dynamic_fallback() {
        for (model_id, level, expected) in [
            ("gemini-2.5-pro", ThinkingLevel::High, 32768i64),
            ("gemini-2.5-pro", ThinkingLevel::Minimal, 128),
            ("gemini-2.5-flash", ThinkingLevel::Medium, 8192),
            ("gemini-2.5-flash-lite", ThinkingLevel::High, 24576),
            ("gemini-2.5-flash-lite", ThinkingLevel::Minimal, 512),
            // Budget-mode models outside the 2.5 families fall back to the
            // dynamic `-1` (level-mode models like gemini-9 never reach the
            // budget table).
            ("gemini-2.0-flash", ThinkingLevel::Medium, -1),
        ] {
            let provider = table_provider_40();
            let body = body_40(
                &provider,
                &plain_request_40(
                    model_id,
                    ThinkingConfig {
                        enabled: true,
                        budget_tokens: None,
                        level,
                    },
                ),
            );
            assert_eq!(
                thinking_config_of(&body).get("thinkingBudget"),
                Some(&serde_json::json!(expected)),
                "{model_id} {level:?}"
            );
        }

        // An explicit neutral budget wins over the table.
        let body = body_40(
            &provider_neutral(),
            &plain_request_40(
                "gemini-2.5-flash",
                ThinkingConfig {
                    enabled: true,
                    budget_tokens: Some(1234),
                    level: ThinkingLevel::Medium,
                },
            ),
        );
        assert_eq!(
            thinking_config_of(&body).get("thinkingBudget"),
            Some(&serde_json::json!(1234))
        );
    }

    fn provider_neutral() -> GeminiProvider {
        GeminiProvider::new(None)
    }

    fn table_provider_40() -> GeminiProvider {
        let ids = [
            "gemini-2.5-pro",
            "gemini-2.5-flash",
            "gemini-2.5-flash-lite",
            "gemini-2.0-flash",
        ];
        GeminiProvider::new(None).with_models(ids.iter().map(|id| model_40(id)).collect())
    }

    #[test]
    fn google_neutral_disabled_on_reasoning_sends_disabled_config() {
        let provider = GeminiProvider::new(None);
        let body = body_40(
            &provider,
            &plain_request_40(
                "gemini-2.5-flash",
                ThinkingConfig {
                    enabled: false,
                    budget_tokens: None,
                    level: ThinkingLevel::None,
                },
            ),
        );
        assert_eq!(
            thinking_config_of(&body),
            &serde_json::json!({ "thinkingBudget": 0 })
        );
    }

    #[test]
    fn google_from_simple_maps_tool_choice_only() {
        let simple = opi_ai::models::SimpleOptions {
            tool_choice: Some(opi_ai::models::SimpleToolChoice::Auto),
            ..Default::default()
        };
        let options = GoogleGenerativeAiOptions::from_simple(&simple);
        assert!(options.is_simple());
        assert!(matches!(options.tool_choice, Some(GoogleToolChoice::Auto)));
        assert!(options.thinking.is_none());
    }

    #[test]
    fn google_direct_adapter_rejects_wrong_wire_records() {
        let wrong = ModelInfo::new(
            "gpt-4o",
            "GPT-4o",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(1000, 100),
        );
        let provider = std::sync::Arc::new(GeminiProvider::new(None));
        let error = GoogleGenerativeAiChatApi::new(provider, wrong).expect_err("wrong wire");
        assert!(matches!(error, ProviderError::Config { .. }));
    }

    fn text_sse_40() -> String {
        let chunk = serde_json::json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "hi"}],
                },
                "finishReason": "STOP",
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 3,
                "thoughtsTokenCount": 7,
                "cachedContentTokenCount": 4,
                "totalTokenCount": 20,
            },
            "responseId": "resp-1",
        });
        format!("data: {chunk}\n\n")
    }

    async fn drive_40(provider: GeminiProvider, request: Request) -> Vec<AssistantStreamEvent> {
        let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
        let mut events = Vec::new();
        while let Some(result) = stream.next().await {
            events.push(result.expect("stream event"));
        }
        events
    }

    fn stream_request_40() -> Request {
        plain_request_40("gemini-2.5-flash", ThinkingConfig::default())
    }

    #[tokio::test]
    async fn google_stream_counts_thoughts_in_output_and_reasoning() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(text_sse_40())
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;
        let provider = GeminiProvider::new(Some(server.uri()));
        let events = drive_40(provider, stream_request_40()).await;
        let done = events
            .iter()
            .find_map(|e| match e {
                AssistantStreamEvent::Done { message, .. } => Some(message.clone()),
                _ => None,
            })
            .expect("done");
        // input = prompt - cached; output = candidates + thoughts;
        // cacheRead = cached; reasoning = thoughts.
        assert_eq!(done.usage.input_tokens, 6);
        assert_eq!(done.usage.output_tokens, 10);
        assert_eq!(done.usage.cache_read_tokens, 4);
        assert_eq!(done.usage.reasoning_tokens, Some(7));
        assert_eq!(done.response_id.as_deref(), Some("resp-1"));
    }

    fn sse_40(chunks: &[serde_json::Value]) -> String {
        chunks.iter().map(|c| format!("data: {c}\n\n")).collect()
    }

    #[tokio::test]
    async fn google_unsigned_and_duplicate_tool_call_ids_are_regenerated() {
        let chunk = serde_json::json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [
                        {"functionCall": {"name": "lookup", "args": {"a": 1}}},
                        {"functionCall": {"name": "lookup", "args": {"b": 2}}},
                    ],
                },
                "finishReason": "STOP",
            }],
        });
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(sse_40(&[chunk]))
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;
        let provider = GeminiProvider::new(Some(server.uri()));
        let events = drive_40(provider, stream_request_40()).await;
        let done = events
            .iter()
            .find_map(|e| match e {
                AssistantStreamEvent::Done { message, .. } => Some(message.clone()),
                _ => None,
            })
            .expect("done");
        let ids: Vec<&str> = done
            .content
            .iter()
            .filter_map(|c| match c {
                AssistantContent::ToolCall { tool_call } => Some(tool_call.id.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1], "duplicate or missing ids never collide");
        assert!(
            ids.iter().all(|id| id.starts_with("lookup_")),
            "generated ids derive from the tool name: {ids:?}"
        );
        // Tool calls plus STOP map to toolUse.
        assert_eq!(done.stop_reason, StopReason::ToolUse);
    }

    #[tokio::test]
    async fn google_thought_parts_stream_as_thinking_with_signature() {
        let first = serde_json::json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [
                        {"text": "ponder", "thought": true, "thoughtSignature": "c2lnMQ=="},
                    ],
                },
            }],
        });
        let second = serde_json::json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [
                        {"text": " more", "thought": true},
                    ],
                },
                "finishReason": "STOP",
            }],
        });
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(sse_40(&[first, second]))
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;
        let provider = GeminiProvider::new(Some(server.uri()));
        let events = drive_40(provider, stream_request_40()).await;
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AssistantStreamEvent::ThinkingStart { .. })),
            "thought parts open thinking blocks"
        );
        let done = events
            .iter()
            .find_map(|e| match e {
                AssistantStreamEvent::Done { message, .. } => Some(message.clone()),
                _ => None,
            })
            .expect("done");
        match &done.content[0] {
            AssistantContent::Thinking {
                thinking,
                signature,
            } => {
                assert_eq!(thinking, "ponder more");
                // The signature from the first delta survives later deltas
                // that omit it.
                assert_eq!(signature.as_deref(), Some("c2lnMQ=="));
            }
            other => panic!("expected a thinking block, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn google_unknown_finish_reason_is_error_with_raw_text() {
        let chunk = serde_json::json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "partial"}]},
                "finishReason": "SAFETY",
            }],
        });
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(sse_40(&[chunk]))
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;
        let provider = GeminiProvider::new(Some(server.uri()));
        let mut stream =
            provider.stream_prepared(stream_request_40(), opi_ai::test_support::resolved_auth());
        let mut failure = None;
        while let Some(result) = stream.next().await {
            if let Err(error) = result {
                failure = Some(error);
                break;
            }
        }
        let error = failure.expect("typed failure for an error finish");
        assert!(
            error.to_string().contains("SAFETY"),
            "raw finish reason rides the failure text: {error}"
        );
    }

    #[tokio::test]
    async fn google_native_lifecycle_reaches_the_wire() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_partial_json(serde_json::json!({
                "generationConfig": { "thinkingConfig": {
                    "includeThoughts": true,
                    "thinkingBudget": 2048,
                } }
            })))
            .and(query_param("alt", "sse"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(text_sse_40())
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let model = model_40("gemini-2.5-flash").with_base_url(server.uri());
        let provider = std::sync::Arc::new(
            GeminiProvider::new(None)
                .with_models(vec![model.clone()])
                .with_base_url(server.uri()),
        );
        let options = native_40(
            Some(GoogleThinkingConfig {
                enabled: true,
                budget_tokens: Some(2048),
                level: None,
            }),
            None,
        );
        let api = GoogleGenerativeAiChatApi::new(provider, model.clone()).expect("valid record");
        let mut models = opi_ai::models::Models::new();
        let handle = models
            .register_chat_api(
                std::sync::Arc::new(api),
                std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::ApiKey,
                    secrecy::SecretString::from("test-key"),
                )),
                opi_ai::AuthProvenanceSource::Static,
            )
            .expect("register");

        let message = models
            .complete(
                opi_ai::models::ChatModelRecord::new("gemini", model),
                stream_request_40(),
                handle.options(options),
            )
            .await
            .expect("native lifecycle completes");
        assert_eq!(message.stop_reason, StopReason::Stop);
        assert_eq!(message.response_id.as_deref(), Some("resp-1"));
    }

    #[tokio::test]
    async fn google_telemetry_carrier_instruments_the_producer() {
        use std::collections::HashMap;
        use std::sync::Mutex;
        use tracing_core::span::{Attributes, Id, Record};
        use tracing_core::{Dispatch, Event, Metadata, Subscriber, dispatcher};

        #[derive(Default)]
        struct Recording {
            next: u64,
            spans: Vec<(&'static str, Option<u64>)>,
            entered: HashMap<std::thread::ThreadId, Vec<Id>>,
        }
        struct Recorder(std::sync::Arc<Mutex<Recording>>);

        impl Subscriber for Recorder {
            fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
                true
            }
            fn new_span(&self, attributes: &Attributes<'_>) -> Id {
                let mut state = self.0.lock().unwrap();
                state.next += 1;
                let id = Id::from_u64(state.next);
                let parent = attributes.parent().cloned().or_else(|| {
                    attributes
                        .is_contextual()
                        .then(|| {
                            state
                                .entered
                                .get(&std::thread::current().id())
                                .and_then(|stack| stack.last())
                                .cloned()
                        })
                        .flatten()
                });
                state
                    .spans
                    .push((attributes.metadata().name(), parent.map(|id| id.into_u64())));
                id
            }
            fn record(&self, _: &Id, _: &Record<'_>) {}
            fn record_follows_from(&self, _: &Id, _: &Id) {}
            fn event(&self, _: &Event<'_>) {}
            fn enter(&self, id: &Id) {
                self.0
                    .lock()
                    .unwrap()
                    .entered
                    .entry(std::thread::current().id())
                    .or_default()
                    .push(id.clone());
            }
            fn exit(&self, id: &Id) {
                self.0
                    .lock()
                    .unwrap()
                    .entered
                    .entry(std::thread::current().id())
                    .or_default()
                    .pop();
                let _ = id;
            }
        }

        let recorder = std::sync::Arc::new(Mutex::new(Recording::default()));
        let source = Dispatch::new(Recorder(recorder.clone()));
        let parent = dispatcher::with_default(&source, || tracing::info_span!("source_parent"));
        let context = opi_ai::telemetry::TelemetryContext::from_span(parent);

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(text_sse_40())
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;

        let model = model_40("gemini-2.5-flash").with_base_url(server.uri());
        let provider = GeminiProvider::new(None)
            .with_models(vec![model.clone()])
            .with_base_url(server.uri());
        let seen = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = seen.clone();
        let mut request = stream_request_40();
        request.telemetry_context = Some(context);
        request.callbacks =
            opi_ai::callbacks::RequestCallbacks::new().with_async_on_payload(move |_| {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    tracing::info_span!("payload_probe_40").in_scope(|| ());
                    Ok(None)
                }
            });

        let events = drive_40(provider, request).await;
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        );
        assert_eq!(seen.load(std::sync::atomic::Ordering::SeqCst), 1);
        let state = recorder.lock().unwrap();
        assert!(
            state
                .spans
                .iter()
                .any(|(name, _)| *name == "payload_probe_40"),
            "producer telemetry flows to the frozen carrier: {:?}",
            state.spans
        );
    }

    #[tokio::test]
    async fn node5_google_explicit_model_base_is_not_given_an_extra_version() {
        for suffix in ["", "/proxy/custom", "/v1", "/v1beta/"] {
            let server = MockServer::start().await;
            Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).set_body_string("data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"ok\"}]},\"finishReason\":\"STOP\"}]}\n\n").insert_header("content-type","text/event-stream")).expect(1).mount(&server).await;
            let mut model = model_40("gemini-3-pro");
            model.base_url = Some(format!("{}{suffix}", server.uri()));
            let provider = std::sync::Arc::new(
                GeminiProvider::new(Some(format!("{}/factory", server.uri())))
                    .with_models(vec![model.clone()]),
            );
            let api = GoogleGenerativeAiChatApi::new(provider, model.clone()).unwrap();
            let mut models = opi_ai::models::Models::new();
            let handle = models
                .register_chat_api(
                    std::sync::Arc::new(api),
                    std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                        opi_ai::auth::AuthScheme::ApiKey,
                        secrecy::SecretString::from("fixture"),
                    )),
                    opi_ai::AuthProvenanceSource::Static,
                )
                .unwrap();
            models
                .complete(
                    opi_ai::models::ChatModelRecord::new("gemini", model),
                    plain_request_40("gemini-3-pro", Default::default()),
                    handle.options(native_40(None, None)),
                )
                .await
                .unwrap();
            let sent = server.received_requests().await.unwrap();
            assert_eq!(
                sent[0].url.path(),
                format!(
                    "{}/models/gemini-3-pro:streamGenerateContent",
                    suffix.trim_end_matches('/')
                )
            );
        }
    }

    #[tokio::test]
    async fn node5_google_direct_host_constructor_keeps_default_version() {
        let server = MockServer::start().await;
        Mock::given(method("POST")).and(wiremock::matchers::path("/v1beta/models/gemini-2.5-flash:streamGenerateContent")).respond_with(ResponseTemplate::new(200).set_body_string("data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"ok\"}]},\"finishReason\":\"STOP\"}]}\n\n").insert_header("content-type","text/event-stream")).expect(1).mount(&server).await;
        let events: Vec<_> = GeminiProvider::new(Some(server.uri()))
            .stream_prepared(gemini_http_request(), opi_ai::test_support::resolved_auth())
            .collect()
            .await;
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Ok(AssistantStreamEvent::Done { .. })))
        );
        server.verify().await;
    }

    #[tokio::test]
    async fn node5_google_tool_signature_capture_reload_and_real_replay() {
        let server = MockServer::start().await;
        let raw = serde_json::json!({"candidates":[{"content":{"parts":[{"functionCall":{"id":"call-1","name":"lookup","args":{}},"thoughtSignature":"c2ln"}]},"finishReason":"STOP"}]});
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(format!("data: {raw}\n\n"))
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(2)
            .mount(&server)
            .await;
        let model = model_40("gemini-3-pro");
        let api = GoogleGenerativeAiChatApi::new(
            std::sync::Arc::new(
                GeminiProvider::new(Some(server.uri())).with_models(vec![model.clone()]),
            ),
            model.clone(),
        )
        .unwrap();
        let mut models = opi_ai::models::Models::new();
        let handle = models
            .register_chat_api(
                std::sync::Arc::new(api),
                std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::ApiKey,
                    secrecy::SecretString::from("fixture"),
                )),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        let first = models
            .complete(
                opi_ai::models::ChatModelRecord::new("gemini", model.clone()),
                plain_request_40("gemini-3-pro", Default::default()),
                handle.options(native_40(None, None)),
            )
            .await
            .unwrap();
        let AssistantContent::ToolCall { tool_call } = &first.content[0] else {
            panic!("captured tool")
        };
        assert_eq!(tool_call.signature.as_deref(), Some("c2ln"));
        let reload = serde_json::from_slice(&serde_json::to_vec(&first).unwrap()).unwrap();
        let mut request = plain_request_40("gemini-3-pro", Default::default());
        request.context = opi_ai::transcript::normalize_context(Context {
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
                opi_ai::models::ChatModelRecord::new("gemini", model),
                request,
                Default::default(),
            )
            .await
            .unwrap();
        let sent = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
        let parts = body["contents"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|content| content["parts"].as_array().unwrap());
        assert!(
            parts.into_iter().any(
                |part| part.get("functionCall").is_some() && part["thoughtSignature"] == "c2ln"
            )
        );
    }
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
        let provider = GeminiProvider::new(Some(format!("http://{address}")));
        let results: Vec<_> = provider
            .stream_prepared(gemini_http_request(), opi_ai::test_support::resolved_auth())
            .collect()
            .await;
        peer.await.unwrap();
        results
            .into_iter()
            .map(|event| event.expect("legal complete multiline event"))
            .collect()
    }

    let body = [
        serde_json::json!({"candidates":[{"content":{"role":"model","parts":[{"text":"hello"}]},"index":0}]}),
        serde_json::json!({"candidates":[{"content":{"role":"model","parts":[{"text":""}]},"finishReason":"STOP","index":0}],"usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":5,"totalTokenCount":15}}),
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
