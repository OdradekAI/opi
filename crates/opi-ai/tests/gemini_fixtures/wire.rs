//! Gemini wire and provider identity assertions.

use super::*;

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

#[tokio::test]
async fn gemini_retired_models_are_rejected_by_the_standalone_factory() {
    let server = MockServer::start().await;
    let provider = GeminiProvider::new(None).with_base_url(server.uri());
    for id in [
        "gemini-2.0-flash",
        "gemini-3.1-flash-lite-image",
        "gemini-3.1-flash-live-preview",
    ] {
        assert!(provider.models().iter().all(|model| model.id != id));
        let mut request = gemini_http_request();
        request.model = format!("gemini:{id}");
        let result = provider
            .stream_prepared(request, opi_ai::test_support::resolved_auth())
            .next()
            .await;
        assert!(matches!(
            result,
            Some(Err(ProviderError::UnknownModel { provider_id, model_id }))
                if provider_id == "gemini" && model_id == id
        ));
    }
    assert!(server.received_requests().await.unwrap().is_empty());
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
    let provider = opi_ai::providers::gemini::google_provider(
        None,
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
    );
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
                assert_eq!(message.provider, "google");
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
    let provider =
        opi_ai::providers::gemini::GeminiProvider::new(Some("https://custom.proxy".into()));
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

    // A malformed frame is a terminal adapter fault; it must not be followed
    // by a stale Done synthesized from a later valid finish marker.
    let oks: Vec<_> = events.into_iter().filter_map(|r| r.ok()).collect();
    let dones = oks
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .count();
    assert_eq!(dones, 0, "malformed data must suppress a deferred Done");
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
