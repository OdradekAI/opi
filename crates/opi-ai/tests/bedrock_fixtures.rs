//! Bedrock provider fixture tests (task 3.1).
//!
//! Tests cover: text streaming, tool calls, usage, provider errors, error mapping,
//! model-family routing, credential redaction, and no live AWS calls.

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_core::Stream;
use futures_util::{StreamExt, pin_mut};
use opi_ai::bedrock::BedrockProvider;
use opi_ai::bedrock::event_stream;
use opi_ai::bedrock::map_bedrock_status;
use opi_ai::bedrock::sigv4::AwsCredentials;
use opi_ai::credential::BoxAuthFuture;
use opi_ai::http::HttpClient;
use opi_ai::message::{
    AssistantContent, AssistantMessage, ImageSource, InputContent, MediaType, Message,
    OutputContent, ToolCall, ToolDef, ToolResultMessage, UserMessage,
};
use opi_ai::provider::{CacheRetention, Provider, ProviderError, ProviderErrorCategory, Request};
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use opi_ai::{
    AuthFallback, AuthProvenance, AuthProvenanceSource, AuthResolver, AuthScheme,
    AwsCredentialSource, CompatMetadata, Models, ModelsError, ResolvedAuth,
};
use secrecy::SecretString;
use tokio::sync::Notify;
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
        expires_at: None,
    }
}

fn test_auth() -> ResolvedAuth {
    ResolvedAuth::aws_sigv4(
        test_credentials(),
        AuthProvenance {
            source: AuthProvenanceSource::AwsSigV4 {
                source: AwsCredentialSource::ExplicitConfig,
            },
            fallback: AuthFallback::NotAttempted,
        },
    )
}

fn text_stream_request() -> Request {
    Request {
        model: "anthropic.claude-sonnet-4-20250514-v2:0".into(),
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
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
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
        })
        .expect("valid context declarations"),
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

async fn collect_outcome(
    stream: Pin<Box<dyn Stream<Item = Result<AssistantStreamEvent, ProviderError>> + Send>>,
) -> Result<AssistantStreamEvent, ProviderError> {
    pin_mut!(stream);
    while let Some(item) = stream.next().await {
        match item {
            Ok(event) => {
                if event.is_terminal() {
                    return Ok(event);
                }
            }
            Err(error) => return Err(error),
        }
    }
    panic!("stream ended without a terminal event or error");
}

// ---------------------------------------------------------------------------
// Provider construction and metadata
// ---------------------------------------------------------------------------

#[test]
fn provider_id_is_bedrock() {
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    assert_eq!(provider.id(), "bedrock");
}

#[test]
fn provider_has_models() {
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
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
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    for model in provider.models() {
        assert!(!model.id.is_empty(), "model id should not be empty");
        assert!(
            !model.display_name.is_empty(),
            "display_name should not be empty"
        );
        assert!(
            model.capabilities.context_window > 0,
            "context_window should be positive"
        );
        assert!(
            model.capabilities.max_output_tokens > 0,
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

    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));

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

#[tokio::test]
async fn crc_invalid_complete_frame_emits_one_error_and_stops() {
    let mut events_data = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":1,"outputTokens":1}}"#,
        ),
    ]);
    let first_frame_len = u32::from_be_bytes(events_data[..4].try_into().unwrap()) as usize;
    events_data[first_frame_len - 1] ^= 0xff;

    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    let request = text_stream_request();
    let results: Vec<_> = provider
        .stream_from_fixture(&events_data, request.cancel)
        .collect()
        .await;

    assert!(
        matches!(results.as_slice(), [Err(cause)] if cause.category() == opi_ai::provider::ProviderErrorCategory::Stream),
        "a CRC-invalid complete frame must produce exactly one terminal stream error: {results:?}"
    );
}

#[tokio::test]
async fn exception_frame_does_not_expose_upstream_message() {
    let canary = "bedrock-provider-error-secret-canary";
    let payload = format!(r#"{{"message":"{canary}"}}"#);
    let events_data = build_bedrock_stream(&[("exception", &payload)]);
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    let request = text_stream_request();
    let stream = provider.stream_from_fixture(&events_data, request.cancel);
    pin_mut!(stream);
    let mut items: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();
    while let Some(item) = stream.next().await {
        let is_terminal = match &item {
            Ok(event) => event.is_terminal(),
            Err(_) => true,
        };
        items.push(item);
        if is_terminal {
            break;
        }
    }
    let rendered = format!("{items:?}");
    // Current producers stop the dual terminal format: the exception frame is
    // a typed stream failure and never yields an Error event.
    assert!(
        items.iter().any(|item| item.as_ref().is_err_and(
            |cause| cause.category() == opi_ai::provider::ProviderErrorCategory::Stream
        ))
    );
    assert!(
        !items
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Error { .. })))
    );
    assert!(
        !rendered.contains(canary),
        "Bedrock exception leaked upstream message: {rendered}"
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

    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));

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

    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));

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

    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));

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
    let error = map_bedrock_status(status, &headers);
    assert!(matches!(error, ProviderError::AuthFailed(_)));
}

#[test]
fn throttling_mapped_to_rate_limited() {
    let status = reqwest::StatusCode::from_u16(429).unwrap();
    let headers = reqwest::header::HeaderMap::new();
    let error = map_bedrock_status(status, &headers);
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
    let error = map_bedrock_status(status, &headers);
    assert!(
        matches!(error, ProviderError::RateLimited { retry_after_ms: Some(ms) } if ms == 5000),
        "expected retry_after_ms=5000 from retry-after header"
    );
}

#[test]
fn timeout_mapped_correctly() {
    let status = reqwest::StatusCode::from_u16(504).unwrap();
    let headers = reqwest::header::HeaderMap::new();
    let error = map_bedrock_status(status, &headers);
    assert!(matches!(error, ProviderError::Timeout));
}

#[test]
fn server_error_mapped_to_provider_side() {
    let status = reqwest::StatusCode::from_u16(500).unwrap();
    let headers = reqwest::header::HeaderMap::new();
    let error = map_bedrock_status(status, &headers);
    assert!(matches!(error, ProviderError::ProviderSide(_)));
}

// ---------------------------------------------------------------------------
// Model-family routing
// ---------------------------------------------------------------------------

#[test]
fn supported_model_families() {
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    let families = provider.supported_model_families();
    assert!(
        families.contains(&"anthropic"),
        "should support anthropic family"
    );
}

#[test]
fn unsupported_model_family_returns_error() {
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    let result = provider.validate_model_id("unknown.family-v1:0");
    assert!(result.is_err(), "unsupported family should return error");
}

#[test]
fn supported_model_family_validates() {
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
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
        expires_at: None,
    };
    let debug_str = format!("{creds:?}");
    assert!(
        !debug_str.contains("AKIAIOSFODNN7EXAMPLE"),
        "access key id should not appear in debug output"
    );
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
    assert!(!redacted.contains("AKIA"));
    assert!(!redacted.contains("super-secret-key"));
    assert!(redacted.contains("***"));
}

// ---------------------------------------------------------------------------
// Shared HTTP client reuse
// ---------------------------------------------------------------------------

#[test]
fn bedrock_provider_accepts_shared_client() {
    let client = Arc::new(HttpClient::new());
    let provider = BedrockProvider::new(None, client.clone());
    assert!(Arc::ptr_eq(&client, provider.http_client()));
}

// ---------------------------------------------------------------------------
// URL image validation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn url_image_rejected_with_clear_error() {
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    let request = Request {
        model: "bedrock:anthropic.claude-sonnet-4-20250514-v2:0".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
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
        })
        .expect("valid context declarations"),
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
    let stream = provider.stream_prepared(request, test_auth());
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

fn assistant_call(id: &str, name: &str) -> Message {
    Message::Assistant(AssistantMessage {
        content: vec![AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: id.into(),
                name: name.into(),
                arguments: "{}".into(),
                arguments_error: None,
            },
        }],
        api: opi_ai::ApiKind::Anthropic,
        provider: "bedrock".into(),
        model: "m".into(),
        response_model: None,
        wire: Some(opi_ai::model_info::WireApi::BedrockConverseStream),
        response_id: None,
        usage: Default::default(),
        stop_reason: opi_ai::stream::StopReason::ToolUse,
        error_message: None,
        timestamp_ms: 0,
    })
}

#[test]
fn tool_result_image_placeholder_preserves_media_type() {
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    let mut request = text_stream_request();
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![
            // The result pairs with its batch's call (AIR-012).
            assistant_call("tool-1", "screenshot"),
            Message::ToolResult(ToolResultMessage {
                not_executed: None,
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
            }),
        ],
        tools: vec![],
    })
    .expect("valid context declarations");

    let body = provider.build_converse_body(&request).unwrap();
    let text = body["messages"][1]["content"][0]["toolResult"]["content"][0]["text"]
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

    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
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

    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
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

    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
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
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    let thinking: Vec<_> = provider
        .models()
        .iter()
        .filter(|m| m.capabilities.supports_thinking)
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

    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
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

#[derive(Clone, Copy)]
enum BedrockStallPoint {
    BeforeHeaders,
    ResponseBody,
}

async fn spawn_stalled_bedrock_server(stall_point: BedrockStallPoint) -> (String, Arc<Notify>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled Bedrock server");
    let addr = listener.local_addr().expect("stalled Bedrock server addr");
    let stalled = Arc::new(Notify::new());
    let server_stalled = stalled.clone();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept Bedrock request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read Bedrock request");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        if matches!(stall_point, BedrockStallPoint::ResponseBody) {
            tokio::io::AsyncWriteExt::write_all(
                &mut socket,
                b"HTTP/1.1 200 OK\r\nContent-Type: application/vnd.amazon.eventstream\r\nContent-Length: 1024\r\nConnection: close\r\n\r\n",
            )
            .await
            .expect("write Bedrock response headers");
            tokio::io::AsyncWriteExt::flush(&mut socket)
                .await
                .expect("flush Bedrock response headers");
        }
        server_stalled.notify_one();
        std::future::pending::<()>().await;
    });

    (format!("http://{addr}"), stalled)
}

async fn assert_bedrock_cancelled(stall_point: BedrockStallPoint) {
    let (server, stalled) = spawn_stalled_bedrock_server(stall_point).await;
    let cancel = CancellationToken::new();
    let provider = BedrockProvider::new(Some(server), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.cancel = cancel.clone();
    let mut stream = provider.stream_prepared(request, test_auth());

    tokio::time::timeout(std::time::Duration::from_secs(1), stalled.notified())
        .await
        .expect("Bedrock server must reach the selected stall point");
    cancel.cancel();

    let remaining = tokio::time::timeout(std::time::Duration::from_secs(1), async move {
        let mut remaining = Vec::new();
        while let Some(item) = stream.next().await {
            remaining.push(item);
        }
        remaining
    })
    .await
    .expect("Bedrock cancellation must terminate without waiting for HTTP");
    assert!(
        matches!(remaining.as_slice(), [Err(cause)] if cause.category() == opi_ai::provider::ProviderErrorCategory::Cancelled),
        "Bedrock cancellation must yield exactly one typed error, got {remaining:?}"
    );
}

#[tokio::test]
async fn cancellation_before_response_headers_is_typed_and_prompt() {
    assert_bedrock_cancelled(BedrockStallPoint::BeforeHeaders).await;
}

#[tokio::test]
async fn cancellation_during_response_body_is_typed_and_prompt() {
    assert_bedrock_cancelled(BedrockStallPoint::ResponseBody).await;
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

    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));

    let events =
        collect_events(provider.stream_prepared(lifecycle_text_request(), test_auth())).await;

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

    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));

    let events =
        collect_events(provider.stream_prepared(lifecycle_text_request(), test_auth())).await;

    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::Done { .. })),
        "HTTP Bedrock stream must flush pending Done when metadata is absent"
    );
}

#[tokio::test]
async fn terminal_stream_with_truncated_trailer_fails_closed() {
    let mut body_bytes = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"partial terminal"},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
    ]);
    let trailer = event_stream::build_test_frame("metadata", "application/json", b"{}");
    body_bytes.extend_from_slice(&trailer[..trailer.len() - 1]);

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body_bytes, "application/vnd.amazon.eventstream"),
        )
        .mount(&server)
        .await;

    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut stream = provider.stream_prepared(lifecycle_text_request(), test_auth());
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event);
    }

    assert_eq!(
        events
            .iter()
            .filter(|event| event.as_ref().is_err_and(
                |cause| cause.category() == opi_ai::provider::ProviderErrorCategory::Stream
            ))
            .count(),
        1,
        "truncated Bedrock trailer must produce exactly one StreamError: {events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Ok(AssistantStreamEvent::Done { .. }))),
        "truncated Bedrock trailer must not flush Done: {events:?}"
    );
}

#[tokio::test]
async fn auth_error_bodies_are_absent_from_public_errors() {
    let canary = "AKIA_AUTH_ERROR_CANARY";
    for status in [401, 403] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
            .respond_with(ResponseTemplate::new(status).set_body_string(canary))
            .mount(&server)
            .await;

        let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));

        let mut stream = provider.stream_prepared(lifecycle_text_request(), test_auth());
        let error = stream
            .next()
            .await
            .expect("auth failure should produce an event")
            .expect_err("auth failure should produce ProviderError");
        assert!(matches!(error, ProviderError::AuthFailed(_)));
        let rendered = format!("{error} {error:?}");
        assert!(
            !rendered.contains(canary),
            "Bedrock HTTP {status} body leaked through ProviderError: {rendered}"
        );
    }
}

#[tokio::test]
async fn bedrock_http_errors_never_echo_aws_credential_canaries() {
    let canaries = [
        "AKIA_HTTP_ERROR_CANARY",
        "aws-secret-access-key-error-canary",
        "aws-session-token-error-canary",
    ];
    let body = canaries.join(" ");

    for status in [400, 401, 403, 408, 429, 500, 504] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body.clone()))
            .mount(&server)
            .await;
        let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
        let mut stream = provider.stream_prepared(lifecycle_text_request(), test_auth());
        let error = stream
            .next()
            .await
            .expect("HTTP failure should produce an event")
            .expect_err("HTTP failure should produce ProviderError");
        let rendered = format!("{error:?} {error}");
        for canary in canaries {
            assert!(
                !rendered.contains(canary),
                "HTTP {status} echoed AWS credential material: {rendered}"
            );
        }
    }
}

/// Bedrock 5xx responses retain the shared `provider` classification without
/// carrying any upstream response body into the public error.
#[tokio::test]
async fn stream_500_classifies_as_provider_with_bodyless_error() {
    let secret = "sk-proj-1234567890abcdefghijklmnopqrstuv";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
        .respond_with(
            ResponseTemplate::new(500).set_body_string(format!("rejected token {secret}")),
        )
        .mount(&server)
        .await;

    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let stream = provider.stream_prepared(lifecycle_text_request(), test_auth());
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
                "bedrock error must omit the upstream body: {err}"
            );
        }
        other => panic!("expected provider error from HTTP 500, got {other:?}"),
    }
}

#[tokio::test]
async fn request_enrichment_reaches_bedrock_http_boundary() {
    let body_bytes = bedrock_text_lifecycle_bytes();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
        .and(wiremock::matchers::header(
            "content-type",
            "application/json",
        ))
        .and(wiremock::matchers::header("x-opi-request", "bedrock"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body_bytes, "application/vnd.amazon.eventstream"),
        )
        .mount(&server)
        .await;
    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.extra_headers = vec![("X-Opi-Request".into(), "bedrock".into())];

    let events = collect_events(provider.stream_prepared(request, test_auth())).await;

    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantStreamEvent::Done { .. }))
    );
    server.verify().await;
    let received = server.received_requests().await.expect("recorded requests");
    let authorization = received[0]
        .headers
        .get("authorization")
        .expect("SigV4 authorization")
        .to_str()
        .expect("authorization is ASCII");
    assert!(
        authorization.contains("x-opi-request"),
        "Bedrock request enrichment must participate in the SigV4 signed-header set"
    );
    assert!(received[0].headers.contains_key("x-amz-date"));
    assert!(received[0].headers.contains_key("x-amz-content-sha256"));
}

#[tokio::test]
async fn request_timeout_maps_to_typed_timeout_at_bedrock_boundary() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(200)))
        .mount(&server)
        .await;
    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.timeout = Some(std::time::Duration::from_millis(20));
    let mut stream = provider.stream_prepared(request, test_auth());

    let result = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("Bedrock request timeout must resolve promptly")
        .expect("Bedrock timeout must produce a stream item");

    assert!(matches!(result, Err(ProviderError::Timeout)));
}

#[tokio::test]
async fn request_header_cannot_override_bedrock_signature_routing() {
    let server = MockServer::start().await;
    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.extra_headers = vec![("x-amz-date".into(), "override".into())];
    let mut stream = provider.stream_prepared(request, test_auth());

    assert!(matches!(
        stream.next().await,
        Some(Err(ProviderError::RequestFailed(_)))
    ));
    assert!(
        server
            .received_requests()
            .await
            .expect("recorded requests")
            .is_empty(),
        "reserved Bedrock request headers must fail before dispatch"
    );
}

#[tokio::test]
async fn request_header_cannot_duplicate_bedrock_session_token() {
    let server = MockServer::start().await;
    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.extra_headers = vec![("x-amz-security-token".into(), "override".into())];
    let credentials = AwsCredentials {
        session_token: Some("real-session-token".into()),
        ..test_credentials()
    };
    let auth = ResolvedAuth::aws_sigv4(
        credentials,
        AuthProvenance {
            source: AuthProvenanceSource::AwsSigV4 {
                source: AwsCredentialSource::ExplicitConfig,
            },
            fallback: AuthFallback::NotAttempted,
        },
    );
    let mut stream = provider.stream_prepared(request, auth);

    assert!(matches!(
        stream.next().await,
        Some(Err(ProviderError::RequestFailed(_)))
    ));
    assert!(
        server
            .received_requests()
            .await
            .expect("recorded requests")
            .is_empty(),
        "Bedrock session-token collisions must fail before dispatch"
    );
}

#[tokio::test]
async fn request_header_cannot_supply_bedrock_session_token_when_auth_has_none() {
    let server = MockServer::start().await;
    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.extra_headers = vec![("x-amz-security-token".into(), "request-token".into())];
    let mut stream = provider.stream_prepared(request, test_auth());

    assert!(matches!(
        stream.next().await,
        Some(Err(ProviderError::RequestFailed(_)))
    ));
    assert!(
        server
            .received_requests()
            .await
            .expect("recorded requests")
            .is_empty(),
        "Bedrock session-token headers must be provider-managed even when prepared auth has none"
    );
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

    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));

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

// ---------------------------------------------------------------------------
// Per-call prepared AWS SigV4 authentication
// ---------------------------------------------------------------------------

fn prepared_bedrock_request(cancel: CancellationToken) -> Request {
    let mut request = text_stream_request();
    request.model = "bedrock:anthropic.claude-sonnet-4-20250514-v2:0".into();
    request.cancel = cancel;
    request
}

fn aws_provenance(source: AwsCredentialSource) -> AuthProvenance {
    AuthProvenance {
        source: AuthProvenanceSource::AwsSigV4 { source },
        fallback: AuthFallback::NotAttempted,
    }
}

fn prepared_aws_auth(call: usize) -> ResolvedAuth {
    ResolvedAuth::aws_sigv4(
        AwsCredentials {
            access_key_id: format!("AKIAPREPARED{call:04}").into(),
            secret_access_key: format!("prepared-secret-{call:04}").into(),
            session_token: Some(format!("prepared-session-{call:04}").into()),
            region: "us-east-1".into(),
            expires_at: None,
        },
        aws_provenance(AwsCredentialSource::Environment),
    )
}

struct CountingBedrockResolver {
    resolutions: Arc<AtomicUsize>,
}

impl AuthResolver for CountingBedrockResolver {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        let call = self.resolutions.fetch_add(1, Ordering::SeqCst) + 1;
        Box::pin(async move { Ok(prepared_aws_auth(call)) })
    }
}

fn prepared_bedrock_collection(base_url: String, resolutions: Arc<AtomicUsize>) -> Models {
    let mut collection = Models::new();
    collection
        .register_route(
            Box::new(BedrockProvider::new(
                Some(base_url),
                Arc::new(HttpClient::new()),
            )),
            Arc::new(CountingBedrockResolver { resolutions }),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .expect("register Bedrock route");
    collection
}

async fn finish_timeout_attempt(prepared: &opi_ai::PreparedProviderCall) {
    let mut attempt = prepared.start_attempt().expect("start attempt");
    assert!(
        matches!(attempt.next().await, Some(Err(ProviderError::Timeout))),
        "fixture should terminate the attempt with the configured 504"
    );
}

async fn finish_provider_side_attempt(prepared: &opi_ai::PreparedProviderCall) {
    let mut attempt = prepared.start_attempt().expect("start attempt");
    assert!(
        matches!(
            attempt.next().await,
            Some(Err(ProviderError::ProviderSide(_)))
        ),
        "fixture should terminate the attempt with the configured 500"
    );
}

#[tokio::test]
async fn bedrock_resolves_once_per_logical_call_and_reuses_frozen_auth_for_retry() {
    let server = MockServer::start().await;
    // 504 maps to the retryable Timeout classification so the acquisition
    // owner admits the second send of the episode; the frozen SigV4
    // credential must be reused, not re-resolved.
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(504).set_body_string("retryable fixture failure"))
        .mount(&server)
        .await;
    let resolutions = Arc::new(AtomicUsize::new(0));
    let collection = prepared_bedrock_collection(server.uri(), resolutions.clone());
    let spec = "bedrock:anthropic.claude-sonnet-4-20250514-v2:0";
    let prepared = collection
        .prepare_call_with_policy(
            spec,
            prepared_bedrock_request(CancellationToken::new()),
            opi_ai::retry::AcquisitionPolicy::with_max_retries(1),
        )
        .await
        .expect("prepare Bedrock call");

    finish_timeout_attempt(&prepared).await;
    // Wait out the SDK-computed admission delay before the admitted retry.
    let delay_ms = prepared
        .retry_admission()
        .expect("the timeout with budget admits the retry")
        .delay_ms;
    tokio::time::sleep(std::time::Duration::from_millis(delay_ms + 5)).await;
    finish_timeout_attempt(&prepared).await;

    assert_eq!(
        resolutions.load(Ordering::SeqCst),
        1,
        "retry attempts must reuse one frozen Bedrock credential"
    );
    let received = server.received_requests().await.expect("recorded requests");
    assert_eq!(received.len(), 2);
    for request in received {
        let authorization = request
            .headers
            .get("authorization")
            .expect("SigV4 authorization")
            .to_str()
            .expect("authorization is ASCII");
        assert!(authorization.contains("AKIAPREPARED0001"));
    }
}

#[tokio::test]
async fn separate_bedrock_logical_calls_resolve_fresh_credentials() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500).set_body_string("fixture failure"))
        .mount(&server)
        .await;
    let resolutions = Arc::new(AtomicUsize::new(0));
    let collection = prepared_bedrock_collection(server.uri(), resolutions.clone());
    let spec = "bedrock:anthropic.claude-sonnet-4-20250514-v2:0";

    for _ in 0..2 {
        let prepared = collection
            .prepare_call(spec, prepared_bedrock_request(CancellationToken::new()))
            .await
            .expect("prepare Bedrock call");
        finish_provider_side_attempt(&prepared).await;
    }

    assert_eq!(resolutions.load(Ordering::SeqCst), 2);
    let received = server.received_requests().await.expect("recorded requests");
    let authorizations: Vec<_> = received
        .iter()
        .map(|request| {
            request
                .headers
                .get("authorization")
                .expect("SigV4 authorization")
                .to_str()
                .expect("authorization is ASCII")
                .to_owned()
        })
        .collect();
    assert!(authorizations[0].contains("AKIAPREPARED0001"));
    assert!(authorizations[1].contains("AKIAPREPARED0002"));
}

// ---------------------------------------------------------------------------
// Bearer / ProxyPlaceholderSigning adapter wiring and Models-level
// selected-local-source acceptance
// ---------------------------------------------------------------------------

fn bearer_auth(token: &str, base_url: Option<String>) -> ResolvedAuth {
    ResolvedAuth {
        scheme: AuthScheme::Bearer,
        secret: SecretString::from(token),
        base_url,
        account_id: None,
        provenance: AuthProvenance {
            source: AuthProvenanceSource::Explicit,
            fallback: AuthFallback::NotAttempted,
        },
    }
}

#[tokio::test]
async fn bearer_prepared_auth_streams_without_sigv4_attachment() {
    let body_bytes = bedrock_text_lifecycle_bytes();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body_bytes, "application/vnd.amazon.eventstream"),
        )
        .mount(&server)
        .await;

    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    let auth = bearer_auth("bedrock-api-key-fixture", Some(server.uri()));
    let events = collect_events(provider.stream_prepared(lifecycle_text_request(), auth)).await;
    assert!(
        events.iter().any(
            |e| matches!(e, AssistantStreamEvent::TextDelta { delta, .. } if delta == "Hello!")
        ),
        "bearer authentication must still stream the event lifecycle"
    );

    let received = server.received_requests().await.expect("recorded requests");
    let recorded = &received[0];
    let authorization = recorded
        .headers
        .get("authorization")
        .expect("bearer authorization header")
        .to_str()
        .expect("authorization is ASCII");
    assert_eq!(authorization, "Bearer bedrock-api-key-fixture");
    for sigv4_header in ["x-amz-date", "x-amz-content-sha256", "x-amz-security-token"] {
        assert!(
            !recorded.headers.contains_key(sigv4_header),
            "bearer attachment must not stack SigV4 header '{sigv4_header}'"
        );
    }
    assert!(!authorization.starts_with("AWS4-HMAC-SHA256"));
}

#[tokio::test]
async fn bearer_without_any_endpoint_is_a_typed_config_error() {
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    let auth = bearer_auth("bedrock-api-key-fixture", None);
    let outcome = collect_outcome(provider.stream_prepared(lifecycle_text_request(), auth)).await;
    assert!(
        matches!(outcome, Err(ProviderError::Config(_))),
        "bearer without a provider or resolver endpoint must fail typed, got {outcome:?}"
    );
}

#[tokio::test]
async fn standard_aws_endpoints_reject_proxy_placeholder_signing() {
    let error = opi_ai::bedrock::proxy_placeholder_auth(
        "https://bedrock-runtime.us-east-1.amazonaws.com",
        "us-east-1",
    )
    .expect_err("standard AWS endpoints reject the placeholder mode");
    assert!(matches!(error, ProviderError::Config(_)));

    // The provider side rejects it too, even with a placeholder client.
    let provider = BedrockProvider::new(
        Some("https://bedrock-runtime.eu-west-1.amazonaws.com".to_owned()),
        Arc::new(HttpClient::new()),
    )
    .with_placeholder_client(Arc::new(HttpClient::new()));
    let auth = opi_ai::bedrock::proxy_placeholder_auth("http://127.0.0.1:9", "us-east-1")
        .expect("nonstandard endpoint is accepted");
    let outcome = collect_outcome(provider.stream_prepared(lifecycle_text_request(), auth)).await;
    assert!(
        matches!(outcome, Err(ProviderError::Config(_))),
        "provider with a standard endpoint must reject placeholder signing, got {outcome:?}"
    );
}

#[tokio::test]
async fn proxy_placeholder_signing_without_placeholder_client_is_a_typed_error() {
    let provider = BedrockProvider::new(
        Some("http://127.0.0.1:9".to_owned()),
        Arc::new(HttpClient::new()),
    );
    let auth = opi_ai::bedrock::proxy_placeholder_auth("http://127.0.0.1:9", "us-east-1")
        .expect("nonstandard endpoint is accepted");
    let outcome = collect_outcome(provider.stream_prepared(lifecycle_text_request(), auth)).await;
    assert!(
        matches!(outcome, Err(ProviderError::Config(_))),
        "placeholder signing requires the explicit HTTP/1.1 placeholder client, got {outcome:?}"
    );
}

#[tokio::test]
async fn proxy_placeholder_signing_uses_h1_and_fixed_identity_at_a_local_proxy() {
    let body_bytes = bedrock_text_lifecycle_bytes();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local proxy");
    let addr = listener.local_addr().expect("local proxy addr");
    let endpoint = format!("http://{addr}");

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept placeholder request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 4096];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read placeholder request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let mut response = Vec::new();
        response.extend_from_slice(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/vnd.amazon.eventstream\r\nConnection: close\r\n\r\n",
        );
        response.extend_from_slice(&body_bytes);
        tokio::io::AsyncWriteExt::write_all(&mut socket, &response)
            .await
            .expect("write placeholder response");
        request
    });

    let http1_client = Arc::new(
        opi_ai::http::HttpClientBuilder::new()
            .tls(opi_ai::http::TlsOptions::default().http1_only())
            .build()
            .expect("HTTP/1.1-only placeholder client"),
    );
    let provider = BedrockProvider::new(Some(endpoint.clone()), http1_client.clone())
        .with_placeholder_client(http1_client);
    let auth = opi_ai::bedrock::proxy_placeholder_auth(&endpoint, "us-east-1")
        .expect("local proxy endpoint is nonstandard");

    // Ordinary Models assembly: a static placeholder route reads zero real
    // credential sources even while ambient-looking values exist.
    struct CountingLookupContext {
        env_reads: AtomicUsize,
    }
    impl AuthContext for CountingLookupContext {
        fn env(&self, _name: &str) -> Result<Option<String>, AuthContextError> {
            self.env_reads.fetch_add(1, Ordering::SeqCst);
            Ok(Some("AMBIENT-CANARY-LEAK-CHECK".to_owned()))
        }
        fn file_exists(&self, _path: &str) -> Result<bool, AuthContextError> {
            Ok(true)
        }
    }
    let context = Arc::new(CountingLookupContext {
        env_reads: AtomicUsize::new(0),
    });
    struct FixedAuthResolver(ResolvedAuth);
    impl AuthResolver for FixedAuthResolver {
        fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
            Box::pin(async { Ok(self.0.clone()) })
        }
    }
    let mut collection = Models::new();
    collection.set_auth_context(context.clone());
    collection
        .register_route(
            Box::new(provider),
            Arc::new(FixedAuthResolver(auth)),
            AuthProvenanceSource::AwsSigV4 {
                source: AwsCredentialSource::ProxyPlaceholder,
            },
            CompatMetadata::default(),
        )
        .expect("register placeholder route");
    let resolved = collection
        .get_auth("bedrock", Default::default())
        .await
        .expect("placeholder route resolves")
        .expect("placeholder route is configured");
    assert_eq!(
        resolved.provenance.source,
        AuthProvenanceSource::AwsSigV4 {
            source: AwsCredentialSource::ProxyPlaceholder,
        }
    );
    assert_eq!(
        context.env_reads.load(Ordering::SeqCst),
        0,
        "the placeholder route must read zero environment sources"
    );

    let mut request = lifecycle_text_request();
    request.model = "bedrock:anthropic.claude-sonnet-4-20250514-v2:0".into();
    let prepared = collection
        .prepare_call("bedrock:anthropic.claude-sonnet-4-20250514-v2:0", request)
        .await
        .expect("prepare placeholder call");
    let mut attempt = prepared.start_attempt().expect("start placeholder attempt");
    let mut saw_done = false;
    while let Some(item) = attempt.next().await {
        if matches!(item, Ok(AssistantStreamEvent::Done { .. })) {
            saw_done = true;
        }
    }
    assert!(saw_done, "placeholder route must stream the lifecycle");
    drop(attempt);

    let raw_request = server.await.expect("local proxy handled one request");
    let text = String::from_utf8_lossy(&raw_request);
    let request_line = text.lines().next().expect("request line");
    assert!(
        request_line.ends_with("HTTP/1.1"),
        "placeholder signing must use HTTP/1.1, got '{request_line}'"
    );
    let authorization = text
        .lines()
        .find(|line| line.to_ascii_lowercase().starts_with("authorization:"))
        .expect("placeholder authorization header");
    assert!(
        authorization.contains("AWS4-HMAC-SHA256"),
        "placeholder mode keeps fixed placeholder SigV4, got '{authorization}'"
    );
    assert!(
        authorization.contains("dummy-access-key"),
        "placeholder identity is the fixed nonsecret value"
    );
    assert!(
        authorization.contains("us-east-1/bedrock/aws4_request"),
        "frozen region participates in the placeholder credential scope"
    );
    assert!(!text.contains("AMBIENT-CANARY-LEAK-CHECK"));
}

/// Resolver that runs the local AWS chain with fixed injected inputs, so a
/// Models route resolves real selected local credentials per call.
struct LocalChainBedrockResolver {
    ambient: MapAuthContext,
    credentials_file: Option<std::path::PathBuf>,
    config_file: Option<std::path::PathBuf>,
}

impl AuthResolver for LocalChainBedrockResolver {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async move {
            let runner = ScriptedProcessRunner {
                calls: std::sync::Mutex::new(Vec::new()),
                result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
            };
            let input = AwsLocalChainInput {
                selection: AwsAuthSelection::Auto,
                scoped: ScopedAwsEnv::empty(),
                ambient: &self.ambient,
                credentials_file: self.credentials_file.as_deref(),
                config_file: self.config_file.as_deref(),
                process: &runner,
                region: None,
                default_region: "us-east-1",
            };
            resolve_local_aws_auth(&input)
                .await
                .map_err(|error| match error {
                    CredentialResolutionError::Exhausted => ProviderError::CredentialNeeded {
                        provider_id: "bedrock".into(),
                    },
                    other => ProviderError::Config(other.into_safe_provider_summary()),
                })
        })
    }
}

#[tokio::test]
async fn models_route_streams_selected_local_credentials_through_the_real_adapter() {
    let body_bytes = bedrock_text_lifecycle_bytes();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body_bytes, "application/vnd.amazon.eventstream"),
        )
        .mount(&server)
        .await;

    let mut collection = Models::new();
    collection
        .register_route(
            Box::new(BedrockProvider::new(
                Some(server.uri()),
                Arc::new(HttpClient::new()),
            )),
            Arc::new(LocalChainBedrockResolver {
                ambient: MapAuthContext(std::collections::HashMap::from([
                    ("AWS_ACCESS_KEY_ID", "AKIAMODELSLOCAL"),
                    ("AWS_SECRET_ACCESS_KEY", "models-local-secret"),
                ])),
                credentials_file: None,
                config_file: None,
            }),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .expect("register Models Bedrock route");

    let mut request = lifecycle_text_request();
    request.model = "bedrock:anthropic.claude-sonnet-4-20250514-v2:0".into();
    let prepared = collection
        .prepare_call("bedrock:anthropic.claude-sonnet-4-20250514-v2:0", request)
        .await
        .expect("prepare local-source call");
    let mut attempt = prepared.start_attempt().expect("start attempt");
    let mut saw_done = false;
    while let Some(item) = attempt.next().await {
        if matches!(item, Ok(AssistantStreamEvent::Done { .. })) {
            saw_done = true;
        }
    }
    drop(attempt);
    assert!(
        saw_done,
        "selected local credentials must stream the lifecycle"
    );

    let received = server.received_requests().await.expect("recorded requests");
    let recorded = &received[0];
    let authorization = recorded
        .headers
        .get("authorization")
        .expect("SigV4 authorization from the selected local source")
        .to_str()
        .expect("authorization is ASCII");
    assert!(authorization.contains("AKIAMODELSLOCAL"));
}

#[tokio::test]
async fn failing_local_source_resolution_sends_zero_generation_requests() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let mut collection = Models::new();
    collection
        .register_route(
            Box::new(BedrockProvider::new(
                Some(server.uri()),
                Arc::new(HttpClient::new()),
            )),
            Arc::new(LocalChainBedrockResolver {
                // A partial scoped pair fails closed and borrows no ambient
                // half; generation must never dispatch.
                ambient: MapAuthContext(std::collections::HashMap::from([(
                    "AWS_SECRET_ACCESS_KEY",
                    "ambient-secret-half",
                )])),
                credentials_file: None,
                config_file: None,
            }),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .expect("register Models Bedrock route");
    // Give the resolver a partial scoped pair through a tiny wrapper.
    struct PartialScopedResolver;
    impl AuthResolver for PartialScopedResolver {
        fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
            Box::pin(async {
                let runner = ScriptedProcessRunner {
                    calls: std::sync::Mutex::new(Vec::new()),
                    result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
                };
                let ambient = MapAuthContext(std::collections::HashMap::from([(
                    "AWS_SECRET_ACCESS_KEY",
                    "ambient-secret-half",
                )]));
                let input = AwsLocalChainInput {
                    selection: AwsAuthSelection::Auto,
                    scoped: ScopedAwsEnv {
                        access_key_id: Some(SecretString::from("SCOPED_ACCESS_ONLY")),
                        ..ScopedAwsEnv::empty()
                    },
                    ambient: &ambient,
                    credentials_file: None,
                    config_file: None,
                    process: &runner,
                    region: None,
                    default_region: "us-east-1",
                };
                resolve_local_aws_auth(&input)
                    .await
                    .map_err(|error| ProviderError::Config(error.into_safe_provider_summary()))
            })
        }
    }
    let mut failing = Models::new();
    failing
        .register_route(
            Box::new(BedrockProvider::new(
                Some(server.uri()),
                Arc::new(HttpClient::new()),
            )),
            Arc::new(PartialScopedResolver),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .expect("register failing Bedrock route");

    let mut request = lifecycle_text_request();
    request.model = "bedrock:anthropic.claude-sonnet-4-20250514-v2:0".into();
    let error = failing
        .prepare_call("bedrock:anthropic.claude-sonnet-4-20250514-v2:0", request)
        .await
        .expect_err("partial scoped pair must fail preparation");
    assert!(
        error.to_string().contains("incomplete"),
        "typed incomplete-source failure, got {error}"
    );
    assert!(
        !error.to_string().contains("SCOPED_ACCESS_ONLY"),
        "failures stay redacted"
    );
    let received = server.received_requests().await.expect("recorded requests");
    assert!(
        received.is_empty(),
        "a failing local source must mean zero generation sends"
    );
}

// ---------------------------------------------------------------------------
// Local AWS source chain (env -> INI static overlay -> credential_process)
// ---------------------------------------------------------------------------

use opi_ai::auth::aws::local::{
    AwsAuthSelection, AwsLocalChainInput, CredentialResolutionError, CredentialSource,
    ScopedAwsEnv, resolve_local_aws_auth,
};
use opi_ai::auth::aws::process::{
    CredentialProcessOutput, CredentialProcessRunner, CredentialProcessSpec,
};
use opi_ai::auth::{AuthContext, AuthContextError};
use secrecy::ExposeSecret;

/// Injected ambient environment for chain tests: no process-env access.
struct MapAuthContext(std::collections::HashMap<&'static str, &'static str>);

impl AuthContext for MapAuthContext {
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        Ok(self.0.get(name).map(|value| (*value).to_owned()))
    }
    fn file_exists(&self, path: &str) -> Result<bool, AuthContextError> {
        Ok(std::path::Path::new(path).exists())
    }
}

struct ScriptedProcessRunner {
    calls: std::sync::Mutex<Vec<String>>,
    result: Result<CredentialProcessOutput, opi_ai::auth::aws::process::CredentialProcessError>,
}

impl CredentialProcessRunner for ScriptedProcessRunner {
    fn run<'a>(
        &'a self,
        spec: &'a CredentialProcessSpec,
    ) -> opi_ai::credential::BoxAuthFuture<
        'a,
        Result<CredentialProcessOutput, opi_ai::auth::aws::process::CredentialProcessError>,
    > {
        Box::pin(async move {
            self.calls
                .lock()
                .expect("process call log")
                .push(spec.command.clone());
            self.result.clone()
        })
    }
}

fn process_output(access: &str, secret: &str) -> CredentialProcessOutput {
    CredentialProcessOutput {
        version: 1,
        access_key_id: SecretString::from(access),
        secret_access_key: SecretString::from(secret),
        session_token: None,
        expiration: Some(
            time::OffsetDateTime::from_unix_timestamp(4_102_444_800).expect("fixed future expiry"),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn chain_input<'a>(
    ambient: &'a MapAuthContext,
    credentials_file: Option<&'a std::path::Path>,
    config_file: Option<&'a std::path::Path>,
    process: &'a ScriptedProcessRunner,
    selection: AwsAuthSelection,
    scoped: ScopedAwsEnv,
) -> AwsLocalChainInput<'a> {
    AwsLocalChainInput {
        selection,
        scoped,
        ambient,
        credentials_file,
        config_file,
        process,
        region: None,
        default_region: "us-east-1",
    }
}

async fn chain_source(input: &AwsLocalChainInput<'_>) -> CredentialSource {
    let auth = resolve_local_aws_auth(input)
        .await
        .expect("local AWS chain should resolve");
    let AuthProvenanceSource::AwsSigV4 { source } = auth.provenance.source else {
        panic!(
            "expected typed AWS provenance, got {:?}",
            auth.provenance.source
        )
    };
    match source {
        AwsCredentialSource::ExplicitConfig => CredentialSource::ExplicitConfig,
        AwsCredentialSource::ConfiguredEnvironment { .. } => {
            CredentialSource::ConfiguredEnvironment {
                secret_access_key_env: String::new(),
                session_token_env: None,
            }
        }
        AwsCredentialSource::Environment => CredentialSource::Environment,
        AwsCredentialSource::ScopedEnvironment => CredentialSource::ScopedEnvironment,
        AwsCredentialSource::ProfileFile => CredentialSource::ProfileFile,
        AwsCredentialSource::ConfigFile => CredentialSource::ConfigFile,
        AwsCredentialSource::CredentialProcess => CredentialSource::CredentialProcess,
        AwsCredentialSource::ProxyPlaceholder => unreachable!("no placeholder in chain tests"),
    }
}

#[tokio::test]
async fn bedrock_resolution_reports_environment_profile_config_and_process_provenance() {
    let ambient = MapAuthContext(std::collections::HashMap::from([
        ("AWS_ACCESS_KEY_ID", "ENV_ACCESS"),
        ("AWS_SECRET_ACCESS_KEY", "ENV_SECRET"),
        ("AWS_SESSION_TOKEN", "ENV_SESSION"),
        ("AWS_REGION", "us-west-2"),
    ]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    // Complete ambient environment pair is selected when no profile governs.
    let environment = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    );
    assert_eq!(
        chain_source(&environment).await,
        CredentialSource::Environment
    );

    let dir = tempfile::tempdir().unwrap();
    let credentials_file = dir.path().join("credentials");
    std::fs::write(
        &credentials_file,
        "[fixture]\naws_access_key_id=PROFILE_ACCESS\naws_secret_access_key=PROFILE_SECRET\n",
    )
    .unwrap();
    let profile = chain_input(
        &ambient,
        Some(credentials_file.as_path()),
        None,
        &runner,
        AwsAuthSelection::Profile("fixture".to_owned()),
        ScopedAwsEnv::empty(),
    );
    assert_eq!(chain_source(&profile).await, CredentialSource::ProfileFile);

    let config_file = dir.path().join("config-static");
    std::fs::write(
        &config_file,
        "[profile fixture]\naws_access_key_id=CONFIG_ACCESS\naws_secret_access_key=CONFIG_SECRET\n",
    )
    .unwrap();
    let config = chain_input(
        &ambient,
        None,
        Some(config_file.as_path()),
        &runner,
        AwsAuthSelection::Profile("fixture".to_owned()),
        ScopedAwsEnv::empty(),
    );
    assert_eq!(chain_source(&config).await, CredentialSource::ConfigFile);

    let process_config = dir.path().join("config-process");
    std::fs::write(
        &process_config,
        "[profile fixture]\ncredential_process=fixture-credential-process\n",
    )
    .unwrap();
    let process = chain_input(
        &ambient,
        None,
        Some(process_config.as_path()),
        &runner,
        AwsAuthSelection::Profile("fixture".to_owned()),
        ScopedAwsEnv::empty(),
    );
    assert_eq!(
        chain_source(&process).await,
        CredentialSource::CredentialProcess
    );
    assert_eq!(
        runner.calls.lock().expect("process log").as_slice(),
        ["fixture-credential-process"],
        "one process execution per discovery round"
    );
}

#[tokio::test]
async fn complete_ambient_pair_wins_over_ambient_profile_without_explicit_selection() {
    // Only an ambient AWS_PROFILE does not beat a complete ambient pair; the
    // verified pi adapter injects the env pair unless an explicit/scoped
    // profile governs.
    let ambient = MapAuthContext(std::collections::HashMap::from([
        ("AWS_ACCESS_KEY_ID", "ENV_ACCESS"),
        ("AWS_SECRET_ACCESS_KEY", "ENV_SECRET"),
        ("AWS_PROFILE", "fixture"),
    ]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dir = tempfile::tempdir().unwrap();
    let credentials_file = dir.path().join("credentials");
    std::fs::write(
        &credentials_file,
        "[fixture]\naws_access_key_id=PROFILE_ACCESS\naws_secret_access_key=PROFILE_SECRET\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        Some(credentials_file.as_path()),
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    );
    assert_eq!(chain_source(&input).await, CredentialSource::Environment);
    assert!(
        runner.calls.lock().expect("process log").is_empty(),
        "no lower source runs once the env pair is selected"
    );
}

#[tokio::test]
async fn scoped_profile_skips_the_ambient_environment_pair() {
    let ambient = MapAuthContext(std::collections::HashMap::from([
        ("AWS_ACCESS_KEY_ID", "ENV_ACCESS"),
        ("AWS_SECRET_ACCESS_KEY", "ENV_SECRET"),
    ]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dir = tempfile::tempdir().unwrap();
    let credentials_file = dir.path().join("credentials");
    std::fs::write(
        &credentials_file,
        "[fixture]\naws_access_key_id=PROFILE_ACCESS\naws_secret_access_key=PROFILE_SECRET\n",
    )
    .unwrap();
    let scoped = ScopedAwsEnv {
        profile: Some("fixture".to_owned()),
        ..ScopedAwsEnv::empty()
    };
    let input = chain_input(
        &ambient,
        Some(credentials_file.as_path()),
        None,
        &runner,
        AwsAuthSelection::Auto,
        scoped,
    );
    assert_eq!(chain_source(&input).await, CredentialSource::ProfileFile);
}

#[tokio::test]
async fn partial_scoped_pair_fails_closed_without_borrowing_ambient_halves() {
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_SECRET_ACCESS_KEY",
        "ENV_SECRET",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let scoped = ScopedAwsEnv {
        access_key_id: Some(SecretString::from("SCOPED_ACCESS")),
        ..ScopedAwsEnv::empty()
    };
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        scoped,
    );
    assert_eq!(
        resolve_local_aws_auth(&input).await.unwrap_err(),
        CredentialResolutionError::IncompleteSource {
            credential_source: CredentialSource::ScopedEnvironment,
        }
    );
    assert!(runner.calls.lock().expect("process log").is_empty());
}

#[tokio::test]
async fn partial_ambient_pair_continues_to_the_ini_profile() {
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_ACCESS_KEY_ID",
        "ENV_ACCESS",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dir = tempfile::tempdir().unwrap();
    let credentials_file = dir.path().join("credentials");
    std::fs::write(
        &credentials_file,
        "[default]\naws_access_key_id=PROFILE_ACCESS\naws_secret_access_key=PROFILE_SECRET\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        Some(credentials_file.as_path()),
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    );
    assert_eq!(chain_source(&input).await, CredentialSource::ProfileFile);
}

#[tokio::test]
async fn static_ini_fields_overlay_config_and_credentials_files_per_field() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dir = tempfile::tempdir().unwrap();
    let credentials_file = dir.path().join("credentials");
    let config_file = dir.path().join("config");
    // Split pair across files: access from credentials, secret from config,
    // session token only in config attaches after the per-field merge.
    std::fs::write(
        &credentials_file,
        "[fixture]\naws_access_key_id=CREDENTIALS_ACCESS\n",
    )
    .unwrap();
    std::fs::write(
        &config_file,
        "[profile fixture]\naws_secret_access_key=CONFIG_SECRET\naws_session_token=CONFIG_SESSION\nregion=eu-north-1\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        Some(credentials_file.as_path()),
        Some(config_file.as_path()),
        &runner,
        AwsAuthSelection::Profile("fixture".to_owned()),
        ScopedAwsEnv::empty(),
    );
    let auth = resolve_local_aws_auth(&input).await.expect("overlay pair");
    match auth.scheme {
        AuthScheme::AwsSigV4(credentials) => {
            assert_eq!(
                credentials.access_key_id.expose_secret(),
                "CREDENTIALS_ACCESS"
            );
            assert_eq!(
                credentials.secret_access_key.expose_secret(),
                "CONFIG_SECRET"
            );
            assert_eq!(
                credentials
                    .session_token
                    .as_ref()
                    .map(secrecy::ExposeSecret::expose_secret),
                Some("CONFIG_SESSION")
            );
            assert_eq!(credentials.region, "eu-north-1");
        }
        other => panic!("expected SigV4 credentials, got {other:?}"),
    }
    // The identity anchor (access key id) came from the credentials file.
    assert_eq!(chain_source(&input).await, CredentialSource::ProfileFile);

    // Per-field precedence: a credentials-file value overrides the config
    // file for the same key.
    std::fs::write(
        &credentials_file,
        "[fixture]\naws_access_key_id=CREDENTIALS_ACCESS\naws_secret_access_key=CREDENTIALS_SECRET\n",
    )
    .unwrap();
    let auth = resolve_local_aws_auth(&input).await.expect("override pair");
    match auth.scheme {
        AuthScheme::AwsSigV4(credentials) => assert_eq!(
            credentials.secret_access_key.expose_secret(),
            "CREDENTIALS_SECRET"
        ),
        other => panic!("expected SigV4 credentials, got {other:?}"),
    }
}

#[tokio::test]
async fn partial_static_ini_pair_advances_to_credential_process() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dir = tempfile::tempdir().unwrap();
    let config_file = dir.path().join("config");
    std::fs::write(
        &config_file,
        "[profile fixture]\naws_access_key_id=PARTIAL_ACCESS\ncredential_process=fixture-process\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        None,
        Some(config_file.as_path()),
        &runner,
        AwsAuthSelection::Profile("fixture".to_owned()),
        ScopedAwsEnv::empty(),
    );
    assert_eq!(
        chain_source(&input).await,
        CredentialSource::CredentialProcess
    );
    assert_eq!(
        runner.calls.lock().expect("process log").as_slice(),
        ["fixture-process"]
    );
}

#[tokio::test]
async fn credential_process_failure_is_continue_eligible_and_exhausts_typed() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Err(opi_ai::auth::aws::process::CredentialProcessError::Timeout),
    };
    let dir = tempfile::tempdir().unwrap();
    let config_file = dir.path().join("config");
    std::fs::write(
        &config_file,
        "[profile fixture]\ncredential_process=fixture-process\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        None,
        Some(config_file.as_path()),
        &runner,
        AwsAuthSelection::Profile("fixture".to_owned()),
        ScopedAwsEnv::empty(),
    );
    let error = resolve_local_aws_auth(&input).await.unwrap_err();
    assert_eq!(error, CredentialResolutionError::Exhausted);
    let rendered = format!("{error:?} {error}");
    assert!(!rendered.contains("PROCESS_"));
    assert!(!rendered.contains("fixture-process"));
}

#[tokio::test]
async fn blank_credential_process_continues_without_spawning() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dir = tempfile::tempdir().unwrap();
    let config_file = dir.path().join("config");
    std::fs::write(&config_file, "[profile fixture]\ncredential_process=   \n").unwrap();
    let input = chain_input(
        &ambient,
        None,
        Some(config_file.as_path()),
        &runner,
        AwsAuthSelection::Profile("fixture".to_owned()),
        ScopedAwsEnv::empty(),
    );
    assert_eq!(
        resolve_local_aws_auth(&input).await.unwrap_err(),
        CredentialResolutionError::Exhausted
    );
    assert!(runner.calls.lock().expect("process log").is_empty());
}

#[tokio::test]
async fn credential_process_expiry_is_retained_on_the_selected_credentials() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dir = tempfile::tempdir().unwrap();
    let config_file = dir.path().join("config");
    std::fs::write(
        &config_file,
        "[profile fixture]\ncredential_process=fixture-process\nregion=us-west-1\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        None,
        Some(config_file.as_path()),
        &runner,
        AwsAuthSelection::Profile("fixture".to_owned()),
        ScopedAwsEnv::empty(),
    );
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("process source");
    match auth.scheme {
        AuthScheme::AwsSigV4(credentials) => {
            assert_eq!(
                credentials.expires_at,
                Some(
                    time::OffsetDateTime::from_unix_timestamp(4_102_444_800)
                        .expect("fixed future expiry")
                )
            );
            assert_eq!(credentials.region, "us-west-1");
        }
        other => panic!("expected SigV4 credentials, got {other:?}"),
    }
}

#[tokio::test]
async fn unreadable_profile_files_continue_to_lower_sources() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    // A directory in place of the credentials file fails the read probe; the
    // chain must continue (fixed Node corpus) instead of failing closed.
    let dir = tempfile::tempdir().unwrap();
    let unreadable = dir.path().join("credentials-directory");
    std::fs::create_dir(&unreadable).unwrap();
    let config_file = dir.path().join("config");
    std::fs::write(
        &config_file,
        "[profile fixture]\naws_access_key_id=CONFIG_ACCESS\naws_secret_access_key=CONFIG_SECRET\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        Some(unreadable.as_path()),
        Some(config_file.as_path()),
        &runner,
        AwsAuthSelection::Profile("fixture".to_owned()),
        ScopedAwsEnv::empty(),
    );
    assert_eq!(chain_source(&input).await, CredentialSource::ConfigFile);
}

#[tokio::test]
async fn explicit_selection_reads_no_source_and_wins_over_auto() {
    let ambient = MapAuthContext(std::collections::HashMap::from([
        ("AWS_ACCESS_KEY_ID", "ENV_ACCESS"),
        ("AWS_SECRET_ACCESS_KEY", "ENV_SECRET"),
        ("AWS_BEARER_TOKEN_BEDROCK", "ambient-bearer"),
    ]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let credentials = AwsCredentials {
        access_key_id: "AKIASELECTED".into(),
        secret_access_key: "selected-secret".into(),
        session_token: None,
        region: "ap-south-1".into(),
        expires_at: None,
    };
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::SigV4(credentials),
        ScopedAwsEnv::empty(),
    );
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("explicit selection resolves");
    match auth.scheme {
        AuthScheme::AwsSigV4(credentials) => {
            assert_eq!(credentials.access_key_id.expose_secret(), "AKIASELECTED");
        }
        other => panic!("expected SigV4 credentials, got {other:?}"),
    }
    assert_eq!(auth.provenance.source, AuthProvenanceSource::Explicit);
    assert!(runner.calls.lock().expect("process log").is_empty());
}

#[tokio::test]
async fn bearer_token_from_scoped_or_ambient_env_selects_bearer_scheme() {
    let ambient = MapAuthContext(std::collections::HashMap::from([
        ("AWS_ACCESS_KEY_ID", "ENV_ACCESS"),
        ("AWS_SECRET_ACCESS_KEY", "ENV_SECRET"),
        ("AWS_BEARER_TOKEN_BEDROCK", "ambient-bearer-token"),
    ]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let ambient_bearer = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    );
    let auth = resolve_local_aws_auth(&ambient_bearer)
        .await
        .expect("ambient bearer resolves");
    assert_eq!(auth.scheme, AuthScheme::Bearer);
    assert_eq!(auth.secret.expose_secret(), "ambient-bearer-token");
    assert_eq!(
        auth.provenance.source,
        AuthProvenanceSource::Environment {
            name: "AWS_BEARER_TOKEN_BEDROCK".to_owned(),
        }
    );
    assert_eq!(
        auth.base_url.as_deref(),
        Some("https://bedrock-runtime.us-east-1.amazonaws.com"),
        "bearer resolution wires the bedrock runtime endpoint for the region"
    );

    // Scoped bearer beats ambient bearer.
    let scoped = ScopedAwsEnv {
        bearer_token_bedrock: Some(SecretString::from("scoped-bearer-token")),
        ..ScopedAwsEnv::empty()
    };
    let scoped_bearer = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        scoped,
    );
    let auth = resolve_local_aws_auth(&scoped_bearer)
        .await
        .expect("scoped bearer resolves");
    assert_eq!(auth.secret.expose_secret(), "scoped-bearer-token");
}

async fn selected_region(input: &AwsLocalChainInput<'_>) -> String {
    match resolve_local_aws_auth(input).await.expect("resolve").scheme {
        AuthScheme::AwsSigV4(credentials) => credentials.region,
        other => panic!("expected SigV4 credentials, got {other:?}"),
    }
}

#[tokio::test]
async fn region_precedence_spans_config_scoped_and_ambient_sources() {
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };

    let pair_ambient = |entries: &[(&'static str, &'static str)]| {
        let mut map = std::collections::HashMap::from([
            ("AWS_ACCESS_KEY_ID", "ENV_ACCESS"),
            ("AWS_SECRET_ACCESS_KEY", "ENV_SECRET"),
        ]);
        for (name, value) in entries {
            map.insert(name, value);
        }
        MapAuthContext(map)
    };

    // Ambient AWS_REGION beats ambient AWS_DEFAULT_REGION.
    let ambient = pair_ambient(&[
        ("AWS_REGION", "us-west-2"),
        ("AWS_DEFAULT_REGION", "us-west-3"),
    ]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    );
    assert_eq!(selected_region(&input).await, "us-west-2");

    // The AWS_REGION tier is read scoped-then-ambient, so ambient AWS_REGION
    // still beats scoped AWS_DEFAULT_REGION (verified adapter order).
    let scoped_default = || ScopedAwsEnv {
        default_region: Some("sa-east-1".to_owned()),
        ..ScopedAwsEnv::empty()
    };
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        scoped_default(),
    );
    assert_eq!(selected_region(&input).await, "us-west-2");

    // Scoped AWS_DEFAULT_REGION wins once the AWS_REGION tier is empty.
    let default_only = pair_ambient(&[("AWS_DEFAULT_REGION", "us-west-3")]);
    let input = chain_input(
        &default_only,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        scoped_default(),
    );
    assert_eq!(selected_region(&input).await, "sa-east-1");

    // Scoped AWS_REGION beats ambient AWS_REGION and scoped default.
    let scoped_region = ScopedAwsEnv {
        region: Some("ap-northeast-3".to_owned()),
        default_region: Some("sa-east-1".to_owned()),
        ..ScopedAwsEnv::empty()
    };
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        scoped_region,
    );
    assert_eq!(selected_region(&input).await, "ap-northeast-3");

    // An explicit region option beats every environment source.
    let mut input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    );
    input.region = Some("eu-central-1");
    assert_eq!(selected_region(&input).await, "eu-central-1");
}

#[tokio::test]
async fn no_local_source_resolves_to_typed_exhaustion() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    );
    assert_eq!(
        resolve_local_aws_auth(&input).await.unwrap_err(),
        CredentialResolutionError::Exhausted
    );
}

struct PendingBedrockResolver {
    entered: Arc<Notify>,
}

impl AuthResolver for PendingBedrockResolver {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        let entered = self.entered.clone();
        Box::pin(async move {
            entered.notify_one();
            std::future::pending().await
        })
    }
}

#[tokio::test]
async fn cancelling_during_bedrock_resolution_fails_closed_without_dispatch() {
    let server = MockServer::start().await;
    let entered = Arc::new(Notify::new());
    let mut collection = Models::new();
    collection
        .register_route(
            Box::new(BedrockProvider::new(
                Some(server.uri()),
                Arc::new(HttpClient::new()),
            )),
            Arc::new(PendingBedrockResolver {
                entered: entered.clone(),
            }),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .expect("register Bedrock route");
    let collection = Arc::new(collection);
    let cancel = CancellationToken::new();
    let task_cancel = cancel.clone();
    let task = tokio::spawn(async move {
        collection
            .prepare_call(
                "bedrock:anthropic.claude-sonnet-4-20250514-v2:0",
                prepared_bedrock_request(task_cancel),
            )
            .await
    });

    entered.notified().await;
    cancel.cancel();
    let error = task
        .await
        .expect("preparation task did not panic")
        .expect_err("cancelled resolution must fail");
    assert!(matches!(error, ModelsError::CallCancelled));
    assert!(
        server
            .received_requests()
            .await
            .expect("recorded requests")
            .is_empty(),
        "cancelled auth resolution must not dispatch Bedrock"
    );
}

#[test]
fn prepared_sigv4_auth_debug_redacts_access_secret_and_session_values() {
    let access = "AKIA_PREPARED_DEBUG_CANARY";
    let secret = "prepared-secret-debug-canary";
    let session = "prepared-session-debug-canary";
    let auth = ResolvedAuth::aws_sigv4(
        AwsCredentials {
            access_key_id: access.into(),
            secret_access_key: secret.into(),
            session_token: Some(session.into()),
            region: "us-east-1".into(),
            expires_at: None,
        },
        aws_provenance(AwsCredentialSource::ExplicitConfig),
    );
    let debug = format!("{auth:?}");
    for canary in [access, secret, session] {
        assert!(!debug.contains(canary), "credential leaked: {debug}");
    }
    assert!(debug.contains("ExplicitConfig"), "provenance is non-secret");
}

#[tokio::test]
async fn bedrock_wrong_prepared_auth_error_does_not_expose_secret() {
    let secret = "wrong-auth-secret-canary";
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    let auth = ResolvedAuth {
        scheme: AuthScheme::ApiKey,
        secret: SecretString::from(secret),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    };
    let mut stream =
        provider.stream_prepared(prepared_bedrock_request(CancellationToken::new()), auth);
    let error = stream
        .next()
        .await
        .expect("wrong auth produces an error")
        .expect_err("wrong auth must fail closed");
    assert!(matches!(error, ProviderError::Config(_)));
    assert!(!format!("{error:?} {error}").contains(secret));
}
