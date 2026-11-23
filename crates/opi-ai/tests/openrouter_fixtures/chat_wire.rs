//! OpenRouter-compatible Chat wire assertions.

use super::*;

// ---------------------------------------------------------------------------
// Provider identity
// ---------------------------------------------------------------------------

#[test]
fn openrouter_provider_id_is_openrouter() {
    let provider = openrouter_provider();
    assert_eq!(provider.id(), "openrouter");
}

// ---------------------------------------------------------------------------
// Model resolution via registry
// ---------------------------------------------------------------------------

#[test]
fn openrouter_resolves_model_in_registry() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(Box::new(openrouter_provider()))
        .unwrap();
    let (provider, model) = registry.resolve("openrouter:openai/gpt-4o").unwrap();
    assert_eq!(provider.id(), "openrouter");
    assert_eq!(model.id, "openai/gpt-4o");
}

#[test]
fn openrouter_registry_lists_provider_id() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(Box::new(openrouter_provider()))
        .unwrap();
    let ids = registry.provider_ids();
    assert!(ids.contains(&"openrouter"));
}

#[test]
fn openrouter_unknown_model_returns_error() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(Box::new(openrouter_provider()))
        .unwrap();
    let result = registry.resolve("openrouter:nonexistent-model");
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Request body — prefix stripping
// ---------------------------------------------------------------------------

#[test]
fn openrouter_request_body_strips_provider_prefix() {
    let provider = openrouter_provider();
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openrouter:openai/gpt-4o".into(),
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
    assert_eq!(body["model"], "openai/gpt-4o");
}

// ---------------------------------------------------------------------------
// SSE text streaming through OpenAI adapter
// ---------------------------------------------------------------------------

#[tokio::test]
async fn openrouter_text_streaming_produces_start_delta_done() {
    let provider = openrouter_provider();
    let sse = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":null}}],\"model\":\"anthropic/claude-sonnet-4\"}\n\n\
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
async fn openrouter_done_message_has_openrouter_provider() {
    let provider = openrouter_provider();
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
    assert_eq!(done, "openrouter");
}

// ---------------------------------------------------------------------------
// SSE tool call streaming
// ---------------------------------------------------------------------------

#[tokio::test]
async fn openrouter_tool_call_streaming_works() {
    let provider = openrouter_provider();
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
async fn openrouter_error_event_routing() {
    let provider = openrouter_provider();
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
        ))
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
async fn openrouter_usage_in_done_event() {
    let provider = openrouter_provider();
    let sse = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":null}}]}\n\n\
               data: {\"choices\":[{\"delta\":{\"content\":\"test\"}}]}\n\n\
               data: {\"choices\":[{\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":42,\"completion_tokens\":13}}\n\n\
               data: [DONE]\n\n";
    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;

    let done = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::Done { message, .. } => Some(message.usage.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(done.input_tokens, 42);
    assert_eq!(done.output_tokens, 13);
}

// ---------------------------------------------------------------------------
// Model list
// ---------------------------------------------------------------------------

#[test]
fn openrouter_has_model_list() {
    let provider = openrouter_provider();
    let models = provider.models();
    assert!(
        !models.is_empty(),
        "OpenRouter should have at least one model"
    );
    assert!(
        models.iter().any(|m| m.id.contains('/')),
        "OpenRouter model IDs should use provider/model format"
    );
}

// ---------------------------------------------------------------------------
// Custom base URL
// ---------------------------------------------------------------------------

#[test]
fn openrouter_custom_base_url() {
    let provider =
        opi_ai::providers::openrouter::openrouter_provider(Some("https://custom.proxy".into()));
    assert_eq!(provider.id(), "openrouter");
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
        .and(header("HTTP-Referer", "https://github.com/OdradekAI/opi"))
        .and(header("X-Title", "opi"))
        .and(body_partial_json(serde_json::json!({
            "model": "openai/gpt-4o",
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

    let provider = opi_ai::providers::openrouter::openrouter_provider(Some(server.uri()));
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openrouter:openai/gpt-4o".into(),
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

    // verify() confirms the production request carried the OpenRouter chat body
    // (prefix-stripped model + system/user messages + max_tokens), the Bearer
    // auth header, the HTTP-Referer + X-Title identification headers, and the
    // /v1/chat/completions path.
    server.verify().await;
}
