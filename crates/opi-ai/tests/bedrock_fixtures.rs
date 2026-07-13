//! Bedrock provider fixture tests (task 3.1).
//!
//! Tests cover: text streaming, tool calls, usage, provider errors, error mapping,
//! model-family routing, credential redaction, and no live AWS calls.

use std::pin::Pin;
use std::sync::Arc;

use futures_core::Stream;
use futures_util::{StreamExt, pin_mut};
use opi_ai::bedrock::BedrockProvider;
use opi_ai::bedrock::event_stream;
use opi_ai::bedrock::map_bedrock_status;
use opi_ai::bedrock::sigv4::AwsCredentials;
use opi_ai::http::HttpClient;
use opi_ai::message::{
    ImageSource, InputContent, MediaType, Message, OutputContent, ToolDef, ToolResultMessage,
    UserMessage,
};
use opi_ai::provider::{Provider, ProviderError, ProviderErrorCategory, Request, CacheRetention};
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_credentials() -> AwsCredentials {
    AwsCredentials {
        access_key_id: "AKIAIOSFODNN7EXAMPLE".into(),
        secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into(),
        session_token: None,
        region: "us-east-1".into(),
    }
}

fn text_stream_request() -> Request {
    Request {
        model: "anthropic.claude-sonnet-4-20250514-v2:0".into(),
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
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

fn tool_call_request() -> Request {
    Request {
        model: "anthropic.claude-sonnet-4-20250514-v2:0".into(),
        system: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "Read the file".into(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![ToolDef {
            name: "read".into(),
            description: "Read a file".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"]
            }),
        }],
        max_tokens: Some(1024),
        temperature: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

/// Build a Bedrock Converse-Stream response as event-stream bytes.
fn build_bedrock_stream(events: &[(&str, &str)]) -> Vec<u8> {
    let mut buffer = Vec::new();
    for (event_type, json_payload) in events {
        let frame =
            event_stream::build_test_frame(event_type, "application/json", json_payload.as_bytes());
        buffer.extend_from_slice(&frame);
    }
    buffer
}

/// Collect all events from a stream.
async fn collect_events(
    stream: Pin<Box<dyn Stream<Item = Result<AssistantStreamEvent, ProviderError>> + Send>>,
) -> Vec<AssistantStreamEvent> {
    pin_mut!(stream);
    let mut events = Vec::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(event) => {
                let is_terminal = event.is_terminal();
                events.push(event);
                if is_terminal {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    events
}

// ---------------------------------------------------------------------------
// Provider construction and metadata
// ---------------------------------------------------------------------------

#[test]
fn provider_id_is_bedrock() {
    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));
    assert_eq!(provider.id(), "bedrock");
}

#[test]
fn provider_has_models() {
    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));
    let models = provider.models();
    assert!(
        !models.is_empty(),
        "bedrock provider should list at least one model"
    );
    // Should contain Claude models
    assert!(
        models.iter().any(|m| m.id.contains("claude")),
        "should list Claude models"
    );
}

#[test]
fn models_have_required_fields() {
    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));
    for model in provider.models() {
        assert!(!model.id.is_empty(), "model id should not be empty");
        assert!(
            !model.display_name.is_empty(),
            "display_name should not be empty"
        );
        assert!(
            model.context_window > 0,
            "context_window should be positive"
        );
        assert!(
            model.max_output_tokens > 0,
            "max_output_tokens should be positive"
        );
    }
}

// ---------------------------------------------------------------------------
// Text streaming from fixture
// ---------------------------------------------------------------------------

#[tokio::test]
async fn text_streaming_from_fixture() {
    let events_data = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"Hello!"},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":10,"outputTokens":5}}"#,
        ),
    ]);

    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));

    let request = text_stream_request();
    let stream = provider.stream_from_fixture(&events_data, request.cancel);
    let events = collect_events(stream).await;

    assert!(!events.is_empty(), "should produce stream events");

    // Should have Start event
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Start { .. })),
        "should have Start"
    );

    // Should have text content
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::TextDelta { .. })),
        "should have TextDelta"
    );

    // Should end with Done
    let last = events.last().expect("should have events");
    assert!(
        matches!(last, AssistantStreamEvent::Done { .. }),
        "should end with Done"
    );
}

// ---------------------------------------------------------------------------
// Tool call from fixture
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tool_call_from_fixture() {
    let events_data = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"toolUse":{"toolUseId":"tool-1","name":"read"}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"toolUse":{"input":"{\"path\":"}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"toolUse":{"input":"\"/tmp/f\"}"}},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        ("messageStop", r#"{"stopReason":"tool_use"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":15,"outputTokens":20}}"#,
        ),
    ]);

    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));

    let request = tool_call_request();
    let stream = provider.stream_from_fixture(&events_data, request.cancel);
    let events = collect_events(stream).await;

    // Should have tool call events
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::ToolCallStart { .. })),
        "should have ToolCallStart"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::ToolCallDelta { .. })),
        "should have ToolCallDelta"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::ToolCallEnd { .. })),
        "should have ToolCallEnd"
    );

    let partial_arguments: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            AssistantStreamEvent::ToolCallDelta { partial, .. } => {
                partial.content.iter().find_map(|content| match content {
                    opi_ai::message::AssistantContent::ToolCall { tool_call } => {
                        Some(tool_call.arguments.clone())
                    }
                    _ => None,
                })
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        partial_arguments,
        vec![
            r#"{"path":"#.to_string(),
            r#"{"path":"/tmp/f"}"#.to_string()
        ],
        "ToolCallDelta partial.content must accumulate streamed tool arguments across deltas"
    );

    // Done should have ToolUse stop reason
    if let Some(AssistantStreamEvent::Done { reason, .. }) = events.last() {
        assert_eq!(*reason, StopReason::ToolUse);
    } else {
        panic!("expected Done event with ToolUse reason");
    }
}

// ---------------------------------------------------------------------------
// Usage tracking
// ---------------------------------------------------------------------------

#[tokio::test]
async fn usage_tracked_from_metadata() {
    let events_data = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":0}"#,
        ),
        ("contentBlockDelta", r#"{"delta":{"text":"hi"}}"#),
        ("contentBlockStop", r#"{}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":100,"outputTokens":50,"cacheReadInputTokens":10}}"#,
        ),
    ]);

    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));

    let request = text_stream_request();
    let stream = provider.stream_from_fixture(&events_data, request.cancel);
    let events = collect_events(stream).await;

    if let Some(AssistantStreamEvent::Done { message, .. }) = events.last() {
        assert_eq!(message.usage.input_tokens, 100);
        assert_eq!(message.usage.output_tokens, 50);
        assert_eq!(message.usage.cache_read_tokens, 10);
    } else {
        panic!("expected Done event with usage");
    }
}

#[tokio::test]
async fn cache_write_tokens_tracked_from_metadata() {
    // Phase 12 task 12.6, DoD clause 6: cacheCreationInputTokens -> cache_write_tokens.
    let events_data = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":0}"#,
        ),
        ("contentBlockDelta", r#"{"delta":{"text":"hi"}}"#),
        ("contentBlockStop", r#"{}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":100,"outputTokens":50,"cacheReadInputTokens":10,"cacheCreationInputTokens":40}}"#,
        ),
    ]);

    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));

    let request = text_stream_request();
    let stream = provider.stream_from_fixture(&events_data, request.cancel);
    let events = collect_events(stream).await;

    if let Some(AssistantStreamEvent::Done { message, .. }) = events.last() {
        assert_eq!(message.usage.cache_read_tokens, 10);
        assert_eq!(
            message.usage.cache_write_tokens, 40,
            "cacheCreationInputTokens must map to cache_write_tokens"
        );
    } else {
        panic!("expected Done event with usage");
    }
}

// ---------------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------------

#[test]
fn access_denied_mapped_to_auth_failed() {
    let status = reqwest::StatusCode::from_u16(403).unwrap();
    let headers = reqwest::header::HeaderMap::new();
    let error = map_bedrock_status(status, "Access denied", &headers);
    assert!(matches!(error, ProviderError::AuthFailed(_)));
}

#[test]
fn throttling_mapped_to_rate_limited() {
    let status = reqwest::StatusCode::from_u16(429).unwrap();
    let headers = reqwest::header::HeaderMap::new();
    let error = map_bedrock_status(status, "Too many requests", &headers);
    assert!(matches!(
        error,
        ProviderError::RateLimited {
            retry_after_ms: None
        }
    ));
}

#[test]
fn throttling_parses_retry_after_header() {
    let status = reqwest::StatusCode::from_u16(429).unwrap();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("retry-after", "5".parse().unwrap());
    let error = map_bedrock_status(status, "Too many requests", &headers);
    assert!(
        matches!(error, ProviderError::RateLimited { retry_after_ms: Some(ms) } if ms == 5000),
        "expected retry_after_ms=5000 from retry-after header"
    );
}

#[test]
fn timeout_mapped_correctly() {
    let status = reqwest::StatusCode::from_u16(504).unwrap();
    let headers = reqwest::header::HeaderMap::new();
    let error = map_bedrock_status(status, "Gateway timeout", &headers);
    assert!(matches!(error, ProviderError::Timeout));
}

#[test]
fn server_error_mapped_to_provider_side() {
    let status = reqwest::StatusCode::from_u16(500).unwrap();
    let headers = reqwest::header::HeaderMap::new();
    let error = map_bedrock_status(status, "Internal error", &headers);
    assert!(matches!(error, ProviderError::ProviderSide(_)));
}

// ---------------------------------------------------------------------------
// Model-family routing
// ---------------------------------------------------------------------------

#[test]
fn supported_model_families() {
    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));
    let families = provider.supported_model_families();
    assert!(
        families.contains(&"anthropic"),
        "should support anthropic family"
    );
}

#[test]
fn unsupported_model_family_returns_error() {
    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));
    let result = provider.validate_model_id("unknown.family-v1:0");
    assert!(result.is_err(), "unsupported family should return error");
}

#[test]
fn supported_model_family_validates() {
    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));
    let result = provider.validate_model_id("anthropic.claude-sonnet-4-20250514-v2:0");
    assert!(result.is_ok(), "supported family should validate");
}

// ---------------------------------------------------------------------------
// Secret redaction
// ---------------------------------------------------------------------------

#[test]
fn credentials_redacted_in_debug() {
    let creds = AwsCredentials {
        access_key_id: "AKIAIOSFODNN7EXAMPLE".into(),
        secret_access_key: "super-secret-key".into(),
        session_token: Some("secret-token".into()),
        region: "us-east-1".into(),
    };
    let debug_str = format!("{creds:?}");
    assert!(
        !debug_str.contains("super-secret-key"),
        "secret key should not appear in debug output"
    );
    assert!(
        !debug_str.contains("secret-token"),
        "session token should not appear in debug output"
    );
}

#[test]
fn redact_credentials_hides_secrets() {
    let redacted = opi_ai::bedrock::redact_credentials("AKIAIOSFODNN7EXAMPLE", "super-secret-key");
    assert!(!redacted.contains("super-secret-key"));
    assert!(redacted.contains("***"));
}

// ---------------------------------------------------------------------------
// Shared HTTP client reuse
// ---------------------------------------------------------------------------

#[test]
fn bedrock_provider_accepts_shared_client() {
    let client = Arc::new(HttpClient::new());
    let provider = BedrockProvider::new(test_credentials(), None, client.clone());
    assert!(Arc::ptr_eq(&client, provider.http_client()));
}

// ---------------------------------------------------------------------------
// URL image validation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn url_image_rejected_with_clear_error() {
    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));
    let request = Request {
        model: "bedrock:anthropic.claude-sonnet-4-20250514-v2:0".into(),
        system: None,
        messages: vec![Message::User(UserMessage {
            content: vec![
                InputContent::Text {
                    text: "describe".into(),
                },
                InputContent::Image {
                    source: ImageSource::Url {
                        url: "https://example.com/img.png".into(),
                    },
                    media_type: MediaType::Png,
                },
            ],
            timestamp_ms: 0,
        })],
        tools: vec![],
        max_tokens: Some(1024),
        temperature: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };
    let stream = provider.stream(request);
    use futures_util::StreamExt;
    let events: Vec<_> = stream.collect().await;
    assert_eq!(events.len(), 1, "expected exactly one event");
    match &events[0] {
        Err(ProviderError::UnsupportedCapability(msg)) => {
            assert!(
                msg.contains("URL-sourced images are not supported"),
                "unexpected error: {msg}"
            );
        }
        other => panic!("expected UnsupportedCapability error, got {other:?}"),
    }
}

#[test]
fn tool_result_image_placeholder_preserves_media_type() {
    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));
    let mut request = text_stream_request();
    request.messages = vec![Message::ToolResult(ToolResultMessage {
        tool_call_id: "tool-1".into(),
        tool_name: "screenshot".into(),
        content: vec![OutputContent::Image {
            source: ImageSource::Bytes {
                data: vec![0xff, 0xd8, 0xff],
            },
            media_type: MediaType::Jpeg,
        }],
        details: None,
        is_error: false,
        truncated: false,
        timestamp_ms: 0,
    })];

    let body = provider.build_converse_body(&request);
    let text = body["messages"][0]["content"][0]["toolResult"]["content"][0]["text"]
        .as_str()
        .unwrap();

    assert_eq!(text, "[image: image/jpeg]");
}

// ---------------------------------------------------------------------------
// Phase 12 task 12.4 — tool-call conversion breadth (scenarios 3/5/6)
//
// DoD: multiple tool calls, malformed JSON arguments, and provider tool-call
// IDs. The Bedrock mapper accumulates per-block `partial_input` and surfaces
// `toolUseId` as ToolCall.id without parsing the arguments, so malformed JSON
// is preserved for the agent loop.

#[tokio::test]
async fn multi_tool_call_produces_two_calls() {
    let events_data = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"toolUse":{"toolUseId":"tool-a","name":"read"}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"toolUse":{"input":"{\"path\":\"a.rs\"}"}},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        (
            "contentBlockStart",
            r#"{"start":{"toolUse":{"toolUseId":"tool-b","name":"bash"}},"contentBlockIndex":1}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"toolUse":{"input":"{\"cmd\":\"ls\"}"}},"contentBlockIndex":1}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":1}"#),
        ("messageStop", r#"{"stopReason":"tool_use"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":40,"outputTokens":20}}"#,
        ),
    ]);

    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));
    let request = text_stream_request();
    let events = collect_events(provider.stream_from_fixture(&events_data, request.cancel)).await;

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
    assert_eq!(content_len, 2, "Done message carries both toolUse blocks");
}

#[tokio::test]
async fn tool_call_id_round_trips_the_tool_use_id() {
    // Scenario 6: Bedrock `toolUseId` MUST surface as ToolCall.id (the value a
    // subsequent toolResult must echo back as toolUseId).
    let events_data = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"toolUse":{"toolUseId":"tool-roundtrip","name":"read"}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"toolUse":{"input":"{\"path\":\"f\"}"}},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        ("messageStop", r#"{"stopReason":"tool_use"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":5,"outputTokens":2}}"#,
        ),
    ]);

    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));
    let request = text_stream_request();
    let events = collect_events(provider.stream_from_fixture(&events_data, request.cancel)).await;

    let end_id = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::ToolCallEnd { tool_call, .. } => Some(tool_call.id.clone()),
            _ => None,
        })
        .expect("ToolCallEnd emitted");
    assert_eq!(
        end_id, "tool-roundtrip",
        "ToolCall.id must be the Bedrock toolUseId"
    );
}

#[tokio::test]
async fn malformed_tool_args_pass_raw_string_without_panic() {
    let events_data = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"toolUse":{"toolUseId":"tool-bad","name":"read"}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"toolUse":{"input":"{not-json"}},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        ("messageStop", r#"{"stopReason":"tool_use"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":5,"outputTokens":2}}"#,
        ),
    ]);

    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));
    let request = text_stream_request();
    let events = collect_events(provider.stream_from_fixture(&events_data, request.cancel)).await;

    let end = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::ToolCallEnd { tool_call, .. } => Some(tool_call.clone()),
            _ => None,
        })
        .expect("ToolCallEnd emitted despite malformed argument JSON");
    assert_eq!(end.arguments, "{not-json");
    assert_eq!(end.id, "tool-bad");
    assert_eq!(end.name, "read");
}

// ---------------------------------------------------------------------------
// Phase 12 task 12.5 — Bedrock supports_thinking protocol limitation
//
// DoD clause: "Bedrock supports_thinking models have a named positive fixture
// or named negative assertion documenting the protocol limitation instead of a
// silent gap." Bedrock's Claude models declare `supports_thinking: true` (see
// the model list in `bedrock/mod.rs`), but the converse-stream parser has no
// `ReasoningContent` block/delta variant — `BedrockBlockType` and
// `BedrockDelta` are `Text` | `ToolUse` only. A `reasoningContent` block is
// silently routed to an empty `Text` block and emits no `Thinking*` lifecycle
// events. These tests document that limitation by name so the gap between
// advertised capability and delivered stream behavior is explicit, not silent.

#[test]
fn bedrock_models_advertise_supports_thinking() {
    // Context for the negative assertion below: the Bedrock model list DOES
    // claim thinking support, even though the stream parser cannot deliver it.
    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));
    let thinking: Vec<_> = provider
        .models()
        .iter()
        .filter(|m| m.supports_thinking)
        .collect();
    assert!(
        !thinking.is_empty(),
        "bedrock should advertise supports_thinking on at least one model"
    );
}

#[tokio::test]
async fn bedrock_reasoning_content_blocks_not_parsed_as_thinking() {
    // A Bedrock converse-stream reasoning block, shaped per the AWS Converse
    // Stream schema (contentBlockStart.start.reasoningContent +
    // contentBlockDelta.delta.reasoningContent.text), followed by a normal
    // text block so we can prove the parser still runs on ordinary content.
    let events_data = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"reasoningContent":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"reasoningContent":{"text":"reasoning step one"}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"reasoningContent":{"text":"reasoning step two"}},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":1}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"final answer"},"contentBlockIndex":1}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":1}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":10,"outputTokens":5}}"#,
        ),
    ]);

    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));
    let request = text_stream_request();
    let events = collect_events(provider.stream_from_fixture(&events_data, request.cancel)).await;

    // The parser must have run: Start and a terminal Done are present.
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Start { .. })),
        "parser must emit Start"
    );
    let done = events.last().expect("stream must produce a terminal event");
    assert!(
        matches!(done, AssistantStreamEvent::Done { .. }),
        "stream must end with Done"
    );

    // Negative assertion: NO thinking lifecycle events are emitted for the
    // reasoningContent block. This is the named protocol-limitation guard.
    let any_thinking = events.iter().any(|e| {
        matches!(
            e,
            AssistantStreamEvent::ThinkingStart { .. }
                | AssistantStreamEvent::ThinkingDelta { .. }
                | AssistantStreamEvent::ThinkingEnd { .. }
        )
    });
    assert!(
        !any_thinking,
        "bedrock parser must not emit Thinking* events; reasoningContent is not recognized"
    );

    // The reasoning text must be silently dropped (not surfaced as text):
    // `delta.reasoningContent.text` does not match the parser's `delta.text`
    // arm, so the deltas fall through to empty Text deltas.
    let rendered: String = events
        .iter()
        .filter_map(|e| match e {
            AssistantStreamEvent::TextDelta { delta, .. } => Some(delta.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        !rendered.contains("reasoning step one") && !rendered.contains("reasoning step two"),
        "reasoningContent text must not leak into Text deltas: {rendered:?}"
    );

    // The parser still handles ordinary text blocks correctly, proving the
    // limitation is reasoning-specific rather than a general parse failure.
    assert!(
        rendered.contains("final answer"),
        "normal text block must still stream: {rendered:?}"
    );

    // The Done message carries no Thinking content block — the reasoning block
    // was mis-routed to an empty Text block, not to a Thinking block.
    if let AssistantStreamEvent::Done { message, .. } = done {
        assert!(
            message
                .content
                .iter()
                .all(|c| !matches!(c, opi_ai::message::AssistantContent::Thinking { .. })),
            "Done message must contain no Thinking content block: {:?}",
            message.content
        );
    }
}

// ---------------------------------------------------------------------------
// Production Provider::stream lifecycle through a real HTTP exchange
// ---------------------------------------------------------------------------
//
// The offline `stream_from_fixture` tests above exercise the event-stream
// parser and mapper but bypass the production HTTP transport (`Provider::stream`
// -> `stream_http`), so they cannot prove the Converse request body, SigV4
// header set, or end-to-end stream draining through the adapter path. These
// wiremock tests cover that contract using local mock HTTP only.

fn lifecycle_text_request() -> Request {
    // Use a `bedrock:`-prefixed model id without a colon-bearing version
    // suffix so `stream()`'s `split_once(':')` model extraction resolves to
    // `anthropic.claude-sonnet-4` (family `anthropic`) and the request path
    // stays free of URL-encoded version colons.
    Request {
        model: "bedrock:anthropic.claude-sonnet-4".into(),
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
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

fn bedrock_text_lifecycle_bytes() -> Vec<u8> {
    build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"Hello!"},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":10,"outputTokens":5}}"#,
        ),
    ])
}

#[tokio::test]
async fn stream_drains_text_lifecycle_through_http() {
    let body_bytes = bedrock_text_lifecycle_bytes();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
        .and(body_partial_json(serde_json::json!({
            "messages": [{"role": "user", "content": [{"text": "Hello"}]}],
            "system": [{"text": "You are helpful."}],
            "inferenceConfig": {"maxTokens": 1024}
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body_bytes, "application/vnd.amazon.eventstream"),
        )
        .mount(&server)
        .await;

    let provider = BedrockProvider::new(
        test_credentials(),
        Some(server.uri()),
        Arc::new(HttpClient::new()),
    );

    let events = collect_events(provider.stream(lifecycle_text_request())).await;

    // Lifecycle: Start -> TextDelta("Hello!") -> Done.
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Start { .. })),
        "should emit Start through the HTTP adapter path"
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            AssistantStreamEvent::TextDelta { delta, .. } if delta == "Hello!"
        )),
        "should emit TextDelta carrying the streamed text"
    );
    let done = events
        .last()
        .expect("stream should produce a terminal event");
    match done {
        AssistantStreamEvent::Done { reason, message } => {
            // Bedrock `end_turn` must map to the shared StopReason::Stop.
            assert_eq!(*reason, StopReason::Stop);
            assert_eq!(message.usage.input_tokens, 10);
            assert_eq!(message.usage.output_tokens, 5);
        }
        other => panic!("expected Done, got {other:?}"),
    }

    // The body+path matchers above are the request-shape assertion; verify()
    // confirms the production request actually carried that Converse body and
    // the signed SigV4 header set.
    server.verify().await;
    let received = server
        .received_requests()
        .await
        .expect("should have recorded the provider request");
    let recorded = &received[0];
    assert!(
        recorded.headers.contains_key("authorization"),
        "Bedrock request must carry a SigV4 authorization header"
    );
    assert!(
        recorded.headers.contains_key("x-amz-date"),
        "Bedrock request must carry an x-amz-date header"
    );
    assert!(
        recorded.headers.contains_key("x-amz-content-sha256"),
        "Bedrock request must carry an x-amz-content-sha256 header"
    );
}

#[tokio::test]
async fn stream_http_flushes_done_without_metadata() {
    let body_bytes = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"Hello without metadata"},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
    ]);
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body_bytes, "application/vnd.amazon.eventstream"),
        )
        .mount(&server)
        .await;

    let provider = BedrockProvider::new(
        test_credentials(),
        Some(server.uri()),
        Arc::new(HttpClient::new()),
    );

    let events = collect_events(provider.stream(lifecycle_text_request())).await;

    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::Done { .. })),
        "HTTP Bedrock stream must flush pending Done when metadata is absent"
    );
}

#[tokio::test]
async fn stream_http_error_maps_to_auth_failed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
        .respond_with(ResponseTemplate::new(403).set_body_string("access denied"))
        .mount(&server)
        .await;

    let provider = BedrockProvider::new(
        test_credentials(),
        Some(server.uri()),
        Arc::new(HttpClient::new()),
    );

    let stream = provider.stream(lifecycle_text_request());
    pin_mut!(stream);
    let first = stream.next().await.expect("should produce an event");
    match first {
        Err(ProviderError::AuthFailed(msg)) => {
            assert!(
                msg.contains("access denied") || msg.contains("Bedrock"),
                "auth error should mention the denial: {msg}"
            );
        }
        other => panic!("expected AuthFailed from HTTP 403, got {other:?}"),
    }
}

/// Phase 12 task 12.2 — bedrock 5xx classifies as the shared `provider` class
/// with a redacted body excerpt through the production stream path (closes the
/// every-family coverage matrix alongside the other 8 HTTP families).
#[tokio::test]
async fn stream_500_classifies_as_provider_with_redacted_excerpt() {
    let secret = "sk-proj-1234567890abcdefghijklmnopqrstuv";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
        .respond_with(
            ResponseTemplate::new(500).set_body_string(format!("rejected token {secret}")),
        )
        .mount(&server)
        .await;

    let provider = BedrockProvider::new(
        test_credentials(),
        Some(server.uri()),
        Arc::new(HttpClient::new()),
    );
    let stream = provider.stream(lifecycle_text_request());
    pin_mut!(stream);
    let first = stream.next().await.expect("should produce an event");
    match first {
        Err(err) => {
            assert_eq!(
                err.category(),
                ProviderErrorCategory::Provider,
                "5xx must classify as provider: {err:?}"
            );
            assert!(
                !err.to_string().contains(secret),
                "bedrock error excerpt must redact the secret: {err}"
            );
        }
        other => panic!("expected provider error from HTTP 500, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Fixture-path cancellation: named negative (Phase 12 task 12.7 DoD clause 7)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn fixture_path_does_not_observe_cancel_documented_http_only_limitation() {
    // Adapter-level cancellation for bedrock is implemented on the signed HTTP
    // body-stream path (bedrock/mod.rs `cancel.cancelled()` select arm), which
    // consumes AWS binary event-stream frames. The fixture suite drives the
    // local `stream_from_fixture` decoder instead, and that path binds the
    // token as a no-op placeholder. So a pre-cancelled token must NOT interrupt
    // the fixture stream — this test pins that substrate limitation so
    // cancellation coverage is not silently claimed for bedrock from the
    // fixture path. Real cancellation for bedrock is exercised only through the
    // signed HTTP path; mounting a successful binary event-stream response
    // through wiremock is out of fixture scope.
    let events_data = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"Hello!"},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":10,"outputTokens":5}}"#,
        ),
    ]);

    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));

    let cancel = CancellationToken::new();
    cancel.cancel(); // pre-cancelled before the stream starts
    let mut request = text_stream_request();
    request.cancel = cancel;

    let stream = provider.stream_from_fixture(&events_data, request.cancel);
    let events = collect_events(stream).await;

    // The fixture stream completes normally despite the cancelled token: the
    // fixture decoder path does not poll cancel (by design).
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Done { .. })),
        "fixture-path stream must complete normally regardless of a cancelled token; \
         bedrock adapter-level cancel is exercised only on the signed HTTP path"
    );
}
