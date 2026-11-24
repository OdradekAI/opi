//! Mistral provider and shared OpenAI-compatible wire assertions.

use super::*;

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
        projection: Default::default(),
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
    let provider =
        opi_ai::providers::mistral::mistral_provider(Some("https://custom.proxy".into()));
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

    let provider = opi_ai::providers::mistral::mistral_provider(Some(server.uri()));
    let request = Request {
        projection: Default::default(),
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
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};

    // The profile's strict capability enables explicit per-tool intent;
    // unconstrained tools retain the shared Chat default.
    let input_schema = serde_json::json!({
        "type": "object",
        "properties": {"path": {"type": "string"}}
    });
    let build_provider = |strict_tool_schema| {
        OpenAiChatProvider::new_for_profile(
            "https://mistral.example.com".into(),
            "mistral".into(),
            CompatConfig {
                instruction_role: Some(opi_ai::model_info::ChatInstructionRole::Developer),
                max_tokens_field: Some("max_completion_tokens".into()),
                strict_tool_schema,
                ..Default::default()
            },
            vec![],
            vec![],
        )
    };
    let build_request = |preference: Option<StrictPreference>| Request {
        projection: Default::default(),
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
                input_schema: input_schema.clone(),
                constrained_sampling: preference
                    .map(|strict| ConstrainedSampling::JsonSchema { strict }),
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
    for strict_tool_schema in [false, true] {
        for preference in [
            None,
            Some(StrictPreference::Prefer),
            Some(StrictPreference::Require),
        ] {
            let provider = build_provider(strict_tool_schema);
            let request = build_request(preference);
            let original_context = request.context.clone();
            let body = provider.build_request_body(&request);
            assert_eq!(request.context, original_context);
            if !strict_tool_schema && preference == Some(StrictPreference::Require) {
                assert!(matches!(body, Err(ProviderError::Config(_))));
                continue;
            }
            let body = body.unwrap();
            let messages = body["messages"].as_array().unwrap();
            assert_eq!(
                messages[0]["role"], "developer",
                "Mistral inherits developer-role override from the shared compat path"
            );
            assert_eq!(
                body["max_completion_tokens"], 1024,
                "Mistral inherits max_completion_tokens field from the shared compat path"
            );
            let function = &body["tools"][0]["function"];
            let expected_strict =
                strict_tool_schema.then(|| serde_json::json!(preference.is_some()));
            assert_eq!(function.get("strict"), expected_strict.as_ref());
            if strict_tool_schema && preference.is_some() {
                let parameters = &function["parameters"];
                assert_eq!(parameters["additionalProperties"], false);
                assert_eq!(parameters["required"], serde_json::json!(["path"]));
                assert_eq!(
                    parameters["properties"]["path"]["anyOf"],
                    serde_json::json!([{"type": "string"}, {"type": "null"}])
                );
            } else {
                assert_eq!(function["parameters"], input_schema);
            }
        }
    }
}

// ---------------------------------------------------------------------------
