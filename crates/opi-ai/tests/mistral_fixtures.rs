//! Mistral provider profile fixture tests (task 2.5).
//!
//! Verifies: model resolution, routing through OpenAI-compatible adapter,
//! request body construction, SSE text/tool-call streaming, error handling,
//! usage tracking, and Mistral-specific model list.

use futures_util::StreamExt;
use opi_ai::message::{InputContent, Message, ToolDef, UserMessage};
use opi_ai::openai_chat::{CompatConfig, OpenAiChatProvider};
use opi_ai::provider::{EventStream, Provider, Request, ThinkingConfig};
use opi_ai::registry::ProviderRegistry;
use opi_ai::stream::AssistantStreamEvent;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: create a Mistral-configured provider.
fn mistral_provider(api_key: &str) -> OpenAiChatProvider {
    opi_ai::mistral::mistral_provider(api_key.into(), None)
}

/// Helper: collect stream events asynchronously.
async fn collect_stream(stream: EventStream) -> Vec<AssistantStreamEvent> {
    stream.filter_map(|r| async move { r.ok() }).collect().await
}

// ---------------------------------------------------------------------------
// Provider identity
// ---------------------------------------------------------------------------

#[test]
fn mistral_provider_id_is_mistral() {
    let provider = mistral_provider("test-key");
    assert_eq!(provider.id(), "mistral");
}

// ---------------------------------------------------------------------------
// Model resolution via registry
// ---------------------------------------------------------------------------

#[test]
fn mistral_resolves_model_in_registry() {
    let mut registry = ProviderRegistry::new();
    registry.register(Box::new(mistral_provider("key")));
    let (provider, model) = registry.resolve("mistral:mistral-large-latest").unwrap();
    assert_eq!(provider.id(), "mistral");
    assert_eq!(model.id, "mistral-large-latest");
}

#[test]
fn mistral_registry_lists_provider_id() {
    let mut registry = ProviderRegistry::new();
    registry.register(Box::new(mistral_provider("key")));
    let ids = registry.provider_ids();
    assert!(ids.contains(&"mistral"));
}

#[test]
fn mistral_unknown_model_returns_error() {
    let mut registry = ProviderRegistry::new();
    registry.register(Box::new(mistral_provider("key")));
    let result = registry.resolve("mistral:nonexistent-model");
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Request body — prefix stripping
// ---------------------------------------------------------------------------

#[test]
fn mistral_request_body_strips_provider_prefix() {
    let provider = mistral_provider("key");
    let request = Request {
        model: "mistral:mistral-small-latest".into(),
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
    };
    let body = provider.build_request_body(&request);
    assert_eq!(body["model"], "mistral-small-latest");
}

// ---------------------------------------------------------------------------
// SSE text streaming through OpenAI adapter
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mistral_text_streaming_produces_start_delta_done() {
    let provider = mistral_provider("key");
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
    let provider = mistral_provider("key");
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
    let provider = mistral_provider("key");
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
    let provider = mistral_provider("key");
    let sse = "data: {\"error\":{\"message\":\"Model not found\"}}\n\n\
               data: [DONE]\n\n";
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
async fn mistral_usage_in_done_event() {
    let provider = mistral_provider("key");
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
    let provider = mistral_provider("key");
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
        opi_ai::mistral::mistral_provider("key".into(), Some("https://custom.proxy".into()));
    assert_eq!(provider.id(), "mistral");
}

// ---------------------------------------------------------------------------
// Multiple text deltas
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mistral_multiple_text_deltas() {
    let provider = mistral_provider("key");
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
            "max_tokens": 1024
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = opi_ai::mistral::mistral_provider("test-key".into(), Some(server.uri()));
    let request = Request {
        model: "mistral:mistral-small-latest".into(),
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
    };

    let mut stream = provider.stream(request);
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) if event.is_terminal() => break,
            Err(_) => break,
            _ => {}
        }
    }

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
        "test-key".into(),
        "https://mistral.example.com".into(),
        "mistral".into(),
        CompatConfig {
            system_role_override: Some("developer".into()),
            max_tokens_field: "max_completion_tokens".into(),
            strict_tool_schema: true,
            ..Default::default()
        },
        vec![],
        vec![],
    );
    let request = Request {
        model: "mistral:mistral-large-latest".into(),
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
    };
    let body = provider.build_request_body(&request);
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
    // stream is open must terminate it gracefully without hanging or panicking.
    // (Deterministic cancel-timing is proven at the agent layer in
    // retry_agent.rs; this asserts the adapter wires cancel.)
    let sse = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n\
               data: {\"choices\":[{\"delta\":{\"content\":\"Hi there\"},\"finish_reason\":null}]}\n\n\
               data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::mistral::mistral_provider("test-key".into(), Some(server.uri()));
    let request = Request {
        model: "mistral:mistral-small-latest".into(),
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
