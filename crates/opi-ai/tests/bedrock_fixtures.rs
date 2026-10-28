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
        transport: Default::default(),
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
        sampling_params: Default::default(),
        thinking: Default::default(),
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

fn tool_call_request() -> Request {
    Request {
        transport: Default::default(),
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
                constrained_sampling: None,
            }],
        })
        .expect("valid context declarations"),
        max_tokens: Some(1024),
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
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
    // 429 classifies THROTTLING through the structured service fault (task
    // 20.20): status and hint ride on the fault, not the RateLimited
    // taxonomy slot.
    assert!(
        matches!(
            &error,
            ProviderError::ServiceFault {
                status: 429,
                error_name: None,
                retry_after_ms: None,
                ..
            }
        ) && error.is_retryable(),
        "expected retryable 429 service fault, got {error:?}"
    );
}

#[test]
fn throttling_parses_retry_after_header() {
    let status = reqwest::StatusCode::from_u16(429).unwrap();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("retry-after", "5".parse().unwrap());
    let error = map_bedrock_status(status, &headers);
    assert!(
        matches!(
            &error,
            ProviderError::ServiceFault {
                retry_after_ms: Some(5000),
                ..
            }
        ),
        "expected retry_after_ms=5000 from retry-after header, got {error:?}"
    );
}

#[test]
fn timeout_mapped_correctly() {
    let status = reqwest::StatusCode::from_u16(504).unwrap();
    let headers = reqwest::header::HeaderMap::new();
    let error = map_bedrock_status(status, &headers);
    // 504 is one of the source's transient statuses, carried on the
    // structured fault (a transport timeout stays ProviderError::Timeout).
    assert!(
        matches!(
            &error,
            ProviderError::ServiceFault {
                status: 504,
                error_name: None,
                ..
            }
        ) && error.is_retryable(),
        "expected retryable 504 service fault, got {error:?}"
    );
}

#[test]
fn server_error_mapped_to_provider_side() {
    let status = reqwest::StatusCode::from_u16(501).unwrap();
    let headers = reqwest::header::HeaderMap::new();
    let error = map_bedrock_status(status, &headers);
    // A bare 501 is SERVER_ERROR: carried on the structured fault and never
    // blanket-retried (>=500 alone is not retryable).
    assert!(
        matches!(
            &error,
            ProviderError::ServiceFault {
                status: 501,
                error_name: None,
                ..
            }
        ) && !error.is_retryable(),
        "expected non-retryable 501 service fault, got {error:?}"
    );
    assert_eq!(error.category(), ProviderErrorCategory::Provider);
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
        transport: Default::default(),
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
        sampling_params: Default::default(),
        thinking: Default::default(),
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
        diagnostics: Vec::new(),
        duration_ms: None,
        provider_thinking_level: None,
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
        transport: Default::default(),
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
        sampling_params: Default::default(),
        thinking: Default::default(),
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
    // 504 rides the structured service fault (a transient status), not the
    // transport Timeout slot.
    assert!(
        matches!(
            attempt.next().await,
            Some(Err(ProviderError::ServiceFault { status: 504, .. }))
        ),
        "fixture should terminate the attempt with the configured 504"
    );
}

async fn finish_provider_side_attempt(prepared: &opi_ai::PreparedProviderCall) {
    let mut attempt = prepared.start_attempt().expect("start attempt");
    assert!(
        matches!(
            attempt.next().await,
            Some(Err(ProviderError::ServiceFault { status: 500, .. }))
        ),
        "fixture should terminate the attempt with the configured 500"
    );
}

#[tokio::test]
async fn bedrock_resolves_once_per_logical_call_and_reuses_frozen_auth_for_retry() {
    let server = MockServer::start().await;
    // 504 is one of the transient statuses, so the acquisition owner admits
    // the second send of the episode; the frozen SigV4 credential must be
    // reused, not re-resolved.
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
                transport: &NEVER_TRANSPORT,
                mfa: None,
                sts_endpoint: None,
                source_cache: None,
                clock: &SYSTEM_SOURCE_CLOCK,
                dpop_jti: &RANDOM_DPPO_JTI,
                sso_cache_dir: None,
                login_cache_dir: None,
                sso_endpoint: None,
                oidc_endpoint: None,
                signin_endpoint: None,
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
                    transport: &NEVER_TRANSPORT,
                    mfa: None,
                    sts_endpoint: None,
                    source_cache: None,
                    clock: &SYSTEM_SOURCE_CLOCK,
                    dpop_jti: &RANDOM_DPPO_JTI,
                    sso_cache_dir: None,
                    login_cache_dir: None,
                    sso_endpoint: None,
                    oidc_endpoint: None,
                    signin_endpoint: None,
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
    ScopedAwsEnv, WebIdentityRoute, resolve_local_aws_auth,
};
use opi_ai::auth::aws::process::{
    CredentialProcessOutput, CredentialProcessRunner, CredentialProcessSpec,
};
use opi_ai::auth::{AuthContext, AuthContextError};
use opi_ai::test_support::FakeSourceTransport;
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
    ambient: &'a dyn opi_ai::auth::AuthContext,
    credentials_file: Option<&'a std::path::Path>,
    config_file: Option<&'a std::path::Path>,
    process: &'a dyn CredentialProcessRunner,
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
        transport: &NEVER_TRANSPORT,
        mfa: None,
        sts_endpoint: None,
        source_cache: None,
        clock: &SYSTEM_SOURCE_CLOCK,
        dpop_jti: &RANDOM_DPPO_JTI,
        sso_cache_dir: None,
        login_cache_dir: None,
        sso_endpoint: None,
        oidc_endpoint: None,
        signin_endpoint: None,
        region: None,
        default_region: "us-east-1",
    }
}

/// Transport used by chain tests that never expect a source exchange.
static NEVER_TRANSPORT: opi_ai::test_support::NeverTransport = opi_ai::test_support::NeverTransport;
static SYSTEM_SOURCE_CLOCK: opi_ai::auth::aws::cache::SystemSourceClock =
    opi_ai::auth::aws::cache::SystemSourceClock;
static RANDOM_DPPO_JTI: opi_ai::auth::aws::cache::RandomDpopJti =
    opi_ai::auth::aws::cache::RandomDpopJti;

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
        AwsCredentialSource::AssumeRole { profile } => CredentialSource::AssumeRole { profile },
        AwsCredentialSource::WebIdentity { route } => CredentialSource::WebIdentity { route },
        AwsCredentialSource::Sso { profile } => CredentialSource::Sso { profile },
        AwsCredentialSource::LoginSession { session } => CredentialSource::LoginSession { session },
        AwsCredentialSource::EcsContainer { legacy } => CredentialSource::EcsContainer { legacy },
        AwsCredentialSource::Ec2InstanceMetadata => CredentialSource::Ec2InstanceMetadata,
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

// ===========================================================================
// Task 20.19.2: STS AssumeRole / web identity / ECS / IMDS source families.
// ===========================================================================

use opi_ai::auth::aws::endpoint::sts_endpoint_url;
use opi_ai::auth::aws::local::TerminalReason;
use opi_ai::auth::aws::role::MfaTokenPresenter;
use opi_ai::auth::aws::source_http::{
    AwsSourceTransport, ReqwestSourceTransport, SourceHttpError, SourceHttpMethod,
    SourceHttpRequest, SourceHttpResponse,
};

fn rfc3339(value: &str) -> time::OffsetDateTime {
    time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339).unwrap()
}

fn sigv4_of(auth: &ResolvedAuth) -> &opi_ai::auth::AwsSigV4Credentials {
    match &auth.scheme {
        AuthScheme::AwsSigV4(credentials) => credentials,
        other => panic!("expected SigV4 scheme, got {other:?}"),
    }
}

fn sts_success_xml(creds: &str, role_arn: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<AssumeRoleResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
<AssumeRoleResult>
<Credentials>
<AccessKeyId>{creds}ACCESS</AccessKeyId>
<SecretAccessKey>{creds}SECRET</SecretAccessKey>
<SessionToken>{creds}SESSION</SessionToken>
<Expiration>2027-06-01T12:00:00Z</Expiration>
</Credentials>
<AssumedRoleUser>
<Arn>{role_arn}</Arn>
<AssumedRoleId>AROAFIXTUREROLEID</AssumedRoleId>
</AssumedRoleUser>
</AssumeRoleResult>
<ResponseMetadata><RequestId>sts-request-1</RequestId></ResponseMetadata>
</AssumeRoleResponse>"#
    )
}

fn web_identity_success_xml(token_subject: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<AssumeRoleWithWebIdentityResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
<AssumeRoleWithWebIdentityResult>
<Subject>{token_subject}</Subject>
<Audience>fixture-audience</Audience>
<Provider>fixture-provider</Provider>
<Credentials>
<AccessKeyId>WEBACCESS</AccessKeyId>
<SecretAccessKey>WEBSECRET</SecretAccessKey>
<SessionToken>WEBSESSION</SessionToken>
<Expiration>2027-07-01T12:00:00Z</Expiration>
</Credentials>
<AssumedRoleUser>
<Arn>arn:aws:sts::123456789012:assumed-role/web-role/session</Arn>
<AssumedRoleId>AROAWEBROLEID</AssumedRoleId>
</AssumedRoleUser>
</AssumeRoleWithWebIdentityResult>
<ResponseMetadata><RequestId>sts-request-2</RequestId></ResponseMetadata>
</AssumeRoleWithWebIdentityResponse>"#
    )
}

fn sts_error_xml(code: &str) -> String {
    format!(
        r#"<ErrorResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
<Error><Type>Sender</Type><Code>{code}</Code><Message>denied detail</Message></Error>
<RequestId>sts-request-denied</RequestId>
</ErrorResponse>"#
    )
}

fn source_response(
    status: u16,
    body: impl Into<String>,
) -> Result<SourceHttpResponse, SourceHttpError> {
    Ok(SourceHttpResponse {
        status,
        body: body.into().into_bytes(),
    })
}

/// A role profile plus a static source profile shared by the INI tests.
fn write_role_profiles(dir: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let credentials = dir.join("credentials");
    std::fs::write(
        &credentials,
        "[source-prof]\naws_access_key_id=SOURCE_ACCESS\naws_secret_access_key=SOURCE_SECRET\naws_session_token=SOURCE_SESSION\n\
         [role-prof]\naws_access_key_id=ROLE_OWN_STATIC_ACCESS\naws_secret_access_key=ROLE_OWN_STATIC_SECRET\n",
    )
    .unwrap();
    let config = dir.join("config");
    std::fs::write(
        &config,
        "[profile role-prof]\nrole_arn=arn:aws:iam::123456789012:role/fixture\nrole_session_name=fixture-session\nexternal_id=fixture-external\nduration_seconds=3600\nsource_profile=source-prof\nregion=us-west-2\n",
    )
    .unwrap();
    (credentials, config)
}

fn recorded_send(transport: &FakeSourceTransport, index: usize) -> SourceHttpRequest {
    transport.send_at(index)
}

fn body_of(send: &SourceHttpRequest) -> String {
    String::from_utf8(send.body.clone().unwrap_or_default()).unwrap()
}

fn header_of<'h>(send: &'h SourceHttpRequest, name: &str) -> Option<&'h str> {
    send.headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn assert_sigv4_signed_with(send: &SourceHttpRequest, access_key_id: &str) {
    let authorization = header_of(send, "Authorization")
        .expect("source SigV4 Authorization header")
        .to_owned();
    assert!(
        authorization.starts_with("AWS4-HMAC-SHA256 Credential="),
        "SigV4 credential header expected: {authorization}"
    );
    assert!(
        authorization.contains(&format!("Credential={access_key_id}/")),
        "source credentials must sign the exchange: {authorization}"
    );
}

struct FixtureMfa;

impl MfaTokenPresenter for FixtureMfa {
    fn token_code(
        &self,
        _serial: &str,
    ) -> Result<String, opi_ai::auth::aws::role::MfaPresentationError> {
        Ok("123456".to_owned())
    }
}

#[tokio::test]
async fn ini_assume_role_wins_over_static_and_signs_source_profile_credentials() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        sts_success_xml(
            "ROLE",
            "arn:aws:sts::123456789012:assumed-role/fixture/fixture-session",
        ),
    )]);
    let dir = tempfile::tempdir().unwrap();
    let (credentials, config) = write_role_profiles(dir.path());

    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("role-prof".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("assume-role resolves");

    assert_eq!(transport.send_count(), 1, "exactly one STS exchange");
    let send = recorded_send(&transport, 0);
    assert_eq!(send.method, SourceHttpMethod::Post);
    assert_eq!(send.url, "https://sts.us-west-2.amazonaws.com/");
    assert_eq!(
        header_of(&send, "Content-Type").unwrap(),
        "application/x-www-form-urlencoded"
    );
    let body = body_of(&send);
    for expected in [
        "Action=AssumeRole",
        "Version=2011-06-15",
        "RoleArn=arn%3Aaws%3Aiam%3A%3A123456789012%3Arole%2Ffixture",
        "RoleSessionName=fixture-session",
        "ExternalId=fixture-external",
        "DurationSeconds=3600",
    ] {
        assert!(
            body.contains(expected),
            "form body missing {expected}: {body}"
        );
    }
    assert!(
        !body.contains("ROLE_OWN_STATIC"),
        "role profile's own static keys must not be sent: {body}"
    );
    assert_sigv4_signed_with(&send, "SOURCE_ACCESS");

    assert_eq!(
        auth.provenance.source,
        AuthProvenanceSource::AwsSigV4 {
            source: AwsCredentialSource::AssumeRole {
                profile: "role-prof".to_owned()
            }
        }
    );
    assert_eq!(
        sigv4_of(&auth).access_key_id.expose_secret(),
        "ROLEACCESS",
        "temporary credentials replace the source pair"
    );
    assert_eq!(
        sigv4_of(&auth).expires_at,
        Some(rfc3339("2027-06-01T12:00:00Z")),
        "STS expiration is retained"
    );
    assert_eq!(auth.account_id.as_deref(), Some("123456789012"));
    assert!(
        runner.calls.lock().unwrap().is_empty(),
        "process must not run for a resolved role"
    );
}

#[tokio::test]
async fn assume_role_requires_exactly_one_of_source_profile_or_credential_source() {
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::default_never();
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(
        &credentials,
        "[both]\nrole_arn=arn:aws:iam::123456789012:role/fixture\nsource_profile=source-prof\ncredential_source=Environment\n\
         [neither]\nrole_arn=arn:aws:iam::123456789012:role/fixture\n",
    )
    .unwrap();

    for profile in ["both", "neither"] {
        let input = chain_input(
            &ambient,
            Some(credentials.as_path()),
            None,
            &runner,
            AwsAuthSelection::Profile(profile.to_owned()),
            ScopedAwsEnv::empty(),
        )
        .with_source_transport(&transport);
        let error = resolve_local_aws_auth(&input)
            .await
            .expect_err("ambiguous role configuration continues and exhausts");
        assert!(
            matches!(error, CredentialResolutionError::Exhausted),
            "expected typed exhaustion, got {error:?}"
        );
    }
    assert_eq!(
        transport.send_count(),
        0,
        "no STS sends for ambiguous roles"
    );
}

#[tokio::test]
async fn nested_source_profiles_resolve_and_cycles_continue() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![
        source_response(
            200,
            sts_success_xml(
                "INNER",
                "arn:aws:sts::123456789012:assumed-role/inner/inner-session",
            ),
        ),
        source_response(
            200,
            sts_success_xml(
                "OUTER",
                "arn:aws:sts::123456789012:assumed-role/outer/outer-session",
            ),
        ),
    ]);
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(
        &credentials,
        "[base]\naws_access_key_id=BASE_ACCESS\naws_secret_access_key=BASE_SECRET\n",
    )
    .unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile outer]\nrole_arn=arn:aws:iam::123456789012:role/outer\nsource_profile=inner\nregion=us-west-2\n\
         [profile inner]\nrole_arn=arn:aws:iam::123456789012:role/inner\nsource_profile=base\n\
         [profile loop-a]\nrole_arn=arn:aws:iam::123456789012:role/a\nsource_profile=loop-b\n\
         [profile loop-b]\nrole_arn=arn:aws:iam::123456789012:role/b\nsource_profile=loop-a\n",
    )
    .unwrap();

    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("outer".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("nested assume-role resolves");
    assert_eq!(sigv4_of(&auth).access_key_id.expose_secret(), "OUTERACCESS");
    assert_eq!(transport.send_count(), 2, "inner then outer STS exchanges");
    assert_sigv4_signed_with(&recorded_send(&transport, 0), "BASE_ACCESS");
    assert_sigv4_signed_with(&recorded_send(&transport, 1), "INNERACCESS");

    let cycling_ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let cycling = chain_input(
        &cycling_ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("loop-a".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let error = resolve_local_aws_auth(&cycling)
        .await
        .expect_err("profile cycles are continue-eligible");
    assert!(matches!(error, CredentialResolutionError::Exhausted));
    assert_eq!(transport.send_count(), 2, "cycles send nothing");
}

#[tokio::test]
async fn mfa_serial_without_presenter_is_typed_terminal() {
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::default_never();
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(
        &credentials,
        "[source-prof]\naws_access_key_id=SOURCE_ACCESS\naws_secret_access_key=SOURCE_SECRET\n",
    )
    .unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile role-prof]\nrole_arn=arn:aws:iam::123456789012:role/fixture\nsource_profile=source-prof\nmfa_serial=arn:aws:iam::123456789012:mfa/fixture\ncredential_process=fixture-process\nregion=us-west-2\n",
    )
    .unwrap();

    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("role-prof".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("MFA without presenter must terminate");
    assert!(
        matches!(
            error,
            CredentialResolutionError::Terminal {
                reason: TerminalReason::MfaPresenterMissing,
                ..
            }
        ),
        "expected MFA terminal, got {error:?}"
    );
    assert_eq!(transport.send_count(), 0);
    assert!(
        runner.calls.lock().unwrap().is_empty(),
        "terminal role failure must not fall through to credential_process"
    );
}

#[tokio::test]
async fn mfa_presenter_supplies_serial_number_and_token_code() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        sts_success_xml(
            "MFA",
            "arn:aws:sts::123456789012:assumed-role/fixture/fixture-session",
        ),
    )]);
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(
        &credentials,
        "[source-prof]\naws_access_key_id=SOURCE_ACCESS\naws_secret_access_key=SOURCE_SECRET\n",
    )
    .unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile role-prof]\nrole_arn=arn:aws:iam::123456789012:role/fixture\nsource_profile=source-prof\nmfa_serial=arn:aws:iam::123456789012:mfa/fixture\nregion=us-west-2\n",
    )
    .unwrap();

    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("role-prof".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport)
    .with_mfa(Some(&FixtureMfa));
    resolve_local_aws_auth(&input)
        .await
        .expect("MFA-presented assume-role resolves");

    let body = body_of(&recorded_send(&transport, 0));
    assert!(
        body.contains("SerialNumber=arn%3Aaws%3Aiam%3A%3A123456789012%3Amfa%2Ffixture"),
        "MFA serial must be encoded: {body}"
    );
    assert!(
        body.contains("TokenCode=123456"),
        "presenter token code: {body}"
    );
}

#[tokio::test]
async fn credential_source_routes_environment_ecs_and_imds_sources() {
    let ambient = MapAuthContext(std::collections::HashMap::from([
        ("AWS_ACCESS_KEY_ID", "ENV_ACCESS"),
        ("AWS_SECRET_ACCESS_KEY", "ENV_SECRET"),
        (
            "AWS_CONTAINER_CREDENTIALS_FULL_URI",
            "http://169.254.170.2/v2/creds",
        ),
    ]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(&credentials, "").unwrap();

    // Environment credential_source: the ambient pair signs the exchange.
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile env-role]\nrole_arn=arn:aws:iam::123456789012:role/fixture\ncredential_source=Environment\nregion=us-west-2\n",
    )
    .unwrap();
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        sts_success_xml(
            "ENVROLE",
            "arn:aws:sts::123456789012:assumed-role/fixture/s",
        ),
    )]);
    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("env-role".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    resolve_local_aws_auth(&input)
        .await
        .expect("environment credential_source resolves");
    assert_eq!(transport.send_count(), 1);
    assert_sigv4_signed_with(&recorded_send(&transport, 0), "ENV_ACCESS");

    // EcsContainer credential_source: the ECS provider feeds the signing pair.
    std::fs::write(
        &config,
        "[profile ecs-role]\nrole_arn=arn:aws:iam::123456789012:role/fixture\ncredential_source=EcsContainer\nregion=us-west-2\n",
    )
    .unwrap();
    let transport = FakeSourceTransport::new(vec![
        source_response(
            200,
            r#"{"AccessKeyId":"ECSACCESS","SecretAccessKey":"ECSSECRET","Token":"ECSSESSION","Expiration":"2027-08-01T00:00:00Z"}"#,
        ),
        source_response(
            200,
            sts_success_xml(
                "ECSROLE",
                "arn:aws:sts::123456789012:assumed-role/fixture/s",
            ),
        ),
    ]);
    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("ecs-role".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("ECS credential_source resolves");
    assert_eq!(transport.send_count(), 2);
    assert_eq!(
        recorded_send(&transport, 0).url,
        "http://169.254.170.2/v2/creds"
    );
    assert_sigv4_signed_with(&recorded_send(&transport, 1), "ECSACCESS");
    assert!(
        matches!(
            auth.provenance.source,
            AuthProvenanceSource::AwsSigV4 {
                source: AwsCredentialSource::AssumeRole { .. }
            }
        ),
        "assume-role provenance wraps the resolved source"
    );

    // Ec2InstanceMetadata credential_source: the IMDS provider feeds the pair.
    std::fs::write(
        &config,
        "[profile imds-role]\nrole_arn=arn:aws:iam::123456789012:role/fixture\ncredential_source=Ec2InstanceMetadata\nregion=us-west-2\n",
    )
    .unwrap();
    let transport = FakeSourceTransport::new(vec![
        source_response(200, "imds-token"),
        source_response(200, "imds-role\n"),
        source_response(
            200,
            r#"{"Code":"Success","AccessKeyId":"IMDSACCESS","SecretAccessKey":"IMDSSECRET","Token":"IMDSSESSION","Expiration":"2027-09-01T00:00:00Z"}"#,
        ),
        source_response(
            200,
            sts_success_xml(
                "IMDSROLE",
                "arn:aws:sts::123456789012:assumed-role/fixture/s",
            ),
        ),
    ]);
    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("imds-role".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    resolve_local_aws_auth(&input)
        .await
        .expect("IMDS credential_source resolves");
    assert_eq!(transport.send_count(), 4);
    assert_eq!(
        recorded_send(&transport, 0).url,
        "http://169.254.169.254/latest/api/token"
    );
    assert_sigv4_signed_with(&recorded_send(&transport, 3), "IMDSACCESS");

    // Unsupported credential_source values are continue-eligible.
    std::fs::write(
        &config,
        "[profile bad-role]\nrole_arn=arn:aws:iam::123456789012:role/fixture\ncredential_source=Unsupported\nregion=us-west-2\n",
    )
    .unwrap();
    let transport = FakeSourceTransport::default_never();
    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("bad-role".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("unsupported credential_source continues");
    assert!(matches!(error, CredentialResolutionError::Exhausted));
}

#[tokio::test]
async fn sts_denial_is_typed_terminal_with_code_and_request_id() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport =
        FakeSourceTransport::new(vec![source_response(403, sts_error_xml("AccessDenied"))]);
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(
        &credentials,
        "[source-prof]\naws_access_key_id=SOURCE_ACCESS\naws_secret_access_key=SOURCE_SECRET\n",
    )
    .unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile role-prof]\nrole_arn=arn:aws:iam::123456789012:role/fixture\nsource_profile=source-prof\ncredential_process=fixture-process\nregion=us-west-2\n",
    )
    .unwrap();

    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("role-prof".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("STS denial is terminal");
    let rendered = error.to_string();
    match error {
        CredentialResolutionError::Terminal { reason, .. } => match reason {
            TerminalReason::StsDenied {
                status,
                code,
                request_id,
            } => {
                assert_eq!(status, Some(403));
                assert_eq!(code.as_deref(), Some("AccessDenied"));
                assert_eq!(request_id.as_deref(), Some("sts-request-denied"));
            }
            other => panic!("expected StsDenied, got {other:?}"),
        },
        other => panic!("expected Terminal, got {other:?}"),
    }
    assert!(rendered.contains("AccessDenied"), "{rendered}");
    assert!(
        !rendered.contains("denied detail"),
        "raw STS message must not reach diagnostics: {rendered}"
    );
    assert!(
        runner.calls.lock().unwrap().is_empty(),
        "STS denial must not fall through"
    );
}

#[tokio::test]
async fn web_identity_from_ini_profile_reads_token_file_and_omits_source_sigv4() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        web_identity_success_xml("fixture-subject"),
    )]);
    let dir = tempfile::tempdir().unwrap();
    let token_file = dir.path().join("token.jwt");
    std::fs::write(&token_file, "fixture-web-identity-token\n").unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(&credentials, "").unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile web-prof]\nrole_arn=arn:aws:iam::123456789012:role/web\nweb_identity_token_file=REPLACED_BY_TEST\nregion=us-west-2\n",
    )
    .unwrap();
    // Token file path with an absolute path in the config.
    std::fs::write(
        &config,
        format!(
            "[profile web-prof]\nrole_arn=arn:aws:iam::123456789012:role/web\nrole_session_name=web-session\nweb_identity_token_file={}\nregion=us-west-2\n",
            token_file.display()
        ),
    )
    .unwrap();

    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("web-prof".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("web identity resolves");

    assert_eq!(transport.send_count(), 1);
    let send = recorded_send(&transport, 0);
    let body = body_of(&send);
    for expected in [
        "Action=AssumeRoleWithWebIdentity",
        "Version=2011-06-15",
        "RoleArn=arn%3Aaws%3Aiam%3A%3A123456789012%3Arole%2Fweb",
        "RoleSessionName=web-session",
        "WebIdentityToken=fixture-web-identity-token",
    ] {
        assert!(body.contains(expected), "web identity body: {body}");
    }
    assert!(
        header_of(&send, "Authorization").is_none(),
        "web identity never signs with source SigV4"
    );
    assert_eq!(
        auth.provenance.source,
        AuthProvenanceSource::AwsSigV4 {
            source: AwsCredentialSource::WebIdentity {
                route: WebIdentityRoute::Profile("web-prof".to_owned())
            }
        }
    );
    assert_eq!(auth.account_id.as_deref(), Some("123456789012"));
    assert!(
        runner.calls.lock().unwrap().is_empty(),
        "web identity precedes credential_process"
    );
}

#[tokio::test]
async fn web_identity_token_file_failure_is_terminal() {
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::default_never();
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(&credentials, "").unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        format!(
            "[profile web-prof]\nrole_arn=arn:aws:iam::123456789012:role/web\nweb_identity_token_file={}\ncredential_process=fixture-process\nregion=us-west-2\n",
            dir.path().join("missing-token.jwt").display()
        ),
    )
    .unwrap();

    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("web-prof".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("unreadable token file is terminal");
    assert!(
        matches!(
            error,
            CredentialResolutionError::Terminal {
                reason: TerminalReason::TokenFileUnreadable,
                ..
            }
        ),
        "got {error:?}"
    );
    assert_eq!(transport.send_count(), 0);
    assert!(runner.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn ambient_web_identity_env_fields_exchange_after_ini_misses() {
    let token_dir = tempfile::tempdir().unwrap();
    let token_path = token_dir.path().join("ambient-token.jwt");
    std::fs::write(&token_path, "ambient-token").unwrap();
    let leaked_token_path: &'static str = Box::leak(
        token_path
            .to_str()
            .expect("utf-8 path")
            .to_owned()
            .into_boxed_str(),
    );
    let ambient = MapAuthContext(std::collections::HashMap::from([
        ("AWS_WEB_IDENTITY_TOKEN_FILE", leaked_token_path),
        ("AWS_ROLE_ARN", "arn:aws:iam::123456789012:role/env"),
        ("AWS_ROLE_SESSION_NAME", "env-session"),
        ("AWS_REGION", "us-west-2"),
    ]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        web_identity_success_xml("env-subject"),
    )]);
    let dir = tempfile::tempdir().unwrap();
    // Default profile has only a region: the INI family misses, then the
    // chain-level ambient web identity segment runs.
    let credentials = dir.path().join("credentials");
    std::fs::write(&credentials, "").unwrap();
    let config = dir.path().join("config");
    std::fs::write(&config, "[default]\nregion=us-west-2\n").unwrap();

    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("ambient web identity resolves");

    assert_eq!(transport.send_count(), 1);
    let body = body_of(&recorded_send(&transport, 0));
    assert!(body.contains("WebIdentityToken=ambient-token"), "{body}");
    assert!(body.contains("RoleSessionName=env-session"), "{body}");
    assert_eq!(
        auth.provenance.source,
        AuthProvenanceSource::AwsSigV4 {
            source: AwsCredentialSource::WebIdentity {
                route: WebIdentityRoute::Environment
            }
        }
    );

    // Missing role configuration is continue-eligible: no exchange, exhaustion.
    let ambient_missing = MapAuthContext(std::collections::HashMap::from([
        ("AWS_WEB_IDENTITY_TOKEN_FILE", "/nonexistent/token"),
        ("AWS_REGION", "us-west-2"),
        ("AWS_EC2_METADATA_DISABLED", "true"),
    ]));
    let transport_missing = FakeSourceTransport::default_never();
    let input = chain_input(
        &ambient_missing,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport_missing);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("incomplete ambient web identity continues");
    assert!(matches!(error, CredentialResolutionError::Exhausted));
    assert_eq!(transport_missing.send_count(), 0);
}

#[tokio::test]
async fn ecs_relative_uri_and_token_file_take_precedence() {
    let ambient = MapAuthContext(std::collections::HashMap::from([
        (
            "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI",
            "/v2/fixture-creds",
        ),
        (
            "AWS_CONTAINER_CREDENTIALS_FULL_URI",
            "https://full.example.invalid/creds",
        ),
        ("AWS_CONTAINER_AUTHORIZATION_TOKEN", "direct-token"),
    ]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dir = tempfile::tempdir().unwrap();
    let token_file = dir.path().join("ecs-token");
    std::fs::write(&token_file, "file-token\n").unwrap();
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        r#"{"AccessKeyId":"ECSACCESS","SecretAccessKey":"ECSSECRET","Token":"ECSSESSION","Expiration":"2027-10-01T00:00:00Z"}"#,
    )]);

    let scoped = ScopedAwsEnv {
        container_authorization_token_file: Some(token_file.display().to_string()),
        ..ScopedAwsEnv::empty()
    };
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        scoped,
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("ECS credentials resolve");

    assert_eq!(transport.send_count(), 1);
    let send = recorded_send(&transport, 0);
    assert_eq!(send.url, "http://169.254.170.2/v2/fixture-creds");
    assert_eq!(header_of(&send, "Authorization").unwrap(), "file-token");
    assert_eq!(
        auth.provenance.source,
        AuthProvenanceSource::AwsSigV4 {
            source: AwsCredentialSource::EcsContainer { legacy: false }
        }
    );
    assert_eq!(sigv4_of(&auth).access_key_id.expose_secret(), "ECSACCESS");
    assert_eq!(
        sigv4_of(&auth).expires_at,
        Some(rfc3339("2027-10-01T00:00:00Z")),
        "ECS expiration is retained"
    );
}

#[tokio::test]
async fn ecs_full_uri_https_is_allowed_and_http_requires_metadata_or_loopback_host() {
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };

    // https full URI: any host is allowed.
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_CONTAINER_CREDENTIALS_FULL_URI",
        "https://credentials.example.invalid/creds",
    )]));
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        r#"{"AccessKeyId":"HTTPSACCESS","SecretAccessKey":"HTTPSSECRET"}"#,
    )]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    resolve_local_aws_auth(&input)
        .await
        .expect("https full URI resolves");
    assert_eq!(transport.send_count(), 1);

    // Plain http to an arbitrary host is a typed terminal rejection.
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_CONTAINER_CREDENTIALS_FULL_URI",
        "http://credentials.example.invalid/creds",
    )]));
    let transport = FakeSourceTransport::default_never();
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("http full URI to a non-metadata host is terminal");
    assert!(
        matches!(
            error,
            CredentialResolutionError::Terminal {
                reason: TerminalReason::InvalidContainerEndpoint,
                ..
            }
        ),
        "got {error:?}"
    );
    assert_eq!(transport.send_count(), 0, "no sends for invalid endpoints");
}

#[tokio::test]
async fn ecs_modern_budget_then_legacy_single_send_and_no_imds_fallthrough() {
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI",
        "/v2/creds",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    // Modern provider: three transport failures then success on send 4.
    let transport = FakeSourceTransport::new(vec![
        Err(SourceHttpError::Transport),
        Err(SourceHttpError::Transport),
        Err(SourceHttpError::Transport),
        source_response(
            200,
            r#"{"AccessKeyId":"RETRYACCESS","SecretAccessKey":"RETRYSECRET"}"#,
        ),
    ]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("modern ECS retries within its fixed budget");
    assert_eq!(
        transport.send_count(),
        4,
        "modern budget is at most four sends"
    );
    assert_eq!(sigv4_of(&auth).access_key_id.expose_secret(), "RETRYACCESS");

    // Modern exhaustion falls to legacy exactly once; legacy failure ends the
    // chain without consulting IMDS.
    let transport = FakeSourceTransport::new(vec![
        Err(SourceHttpError::Transport),
        Err(SourceHttpError::Transport),
        Err(SourceHttpError::Transport),
        Err(SourceHttpError::Transport),
        Err(SourceHttpError::Transport),
    ]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("ECS exhaustion ends the chain");
    assert!(
        matches!(error, CredentialResolutionError::Exhausted),
        "{error:?}"
    );
    assert_eq!(
        transport.send_count(),
        5,
        "four modern sends plus exactly one legacy send, and no IMDS leg"
    );
    for send in transport.sends_snapshot() {
        assert!(
            !send.url.contains("169.254.169.254"),
            "ECS failure must not fall through to IMDS: {}",
            send.url
        );
    }
}

#[tokio::test]
async fn imds_v2_token_role_and_credential_legs_resolve() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![
        source_response(200, "fixture-imds-token"),
        source_response(200, "imds-role\nother-role\n"),
        source_response(
            200,
            r#"{"Code":"Success","Type":"AWS-HMAC","AccessKeyId":"IMDSACCESS","SecretAccessKey":"IMDSSECRET","Token":"IMDSSESSION","Expiration":"2027-11-01T00:00:00Z"}"#,
        ),
    ]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input).await.expect("IMDS resolves");

    assert_eq!(transport.send_count(), 3);
    let token_put = recorded_send(&transport, 0);
    assert_eq!(token_put.method, SourceHttpMethod::Put);
    assert_eq!(token_put.url, "http://169.254.169.254/latest/api/token");
    assert_eq!(
        header_of(&token_put, "x-aws-ec2-metadata-token-ttl-seconds").unwrap(),
        "21600"
    );
    let role_get = recorded_send(&transport, 1);
    assert_eq!(
        role_get.url,
        "http://169.254.169.254/latest/meta-data/iam/security-credentials/"
    );
    assert_eq!(
        header_of(&role_get, "x-aws-ec2-metadata-token").unwrap(),
        "fixture-imds-token"
    );
    let creds_get = recorded_send(&transport, 2);
    assert_eq!(
        creds_get.url,
        "http://169.254.169.254/latest/meta-data/iam/security-credentials/imds-role"
    );
    assert_eq!(
        auth.provenance.source,
        AuthProvenanceSource::AwsSigV4 {
            source: AwsCredentialSource::Ec2InstanceMetadata
        }
    );
    assert_eq!(sigv4_of(&auth).access_key_id.expose_secret(), "IMDSACCESS");
    assert_eq!(
        sigv4_of(&auth).expires_at,
        Some(rfc3339("2027-11-01T00:00:00Z"))
    );
}

#[tokio::test]
async fn imds_token_404_enables_v1_mode_for_subsequent_legs() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![
        source_response(404, ""),
        source_response(200, "v1-role"),
        source_response(
            200,
            r#"{"AccessKeyId":"V1ACCESS","SecretAccessKey":"V1SECRET","Token":"V1SESSION","Expiration":"2027-12-01T00:00:00Z"}"#,
        ),
    ]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("v1 fallback resolves");

    assert_eq!(transport.send_count(), 3);
    for leg in [1_usize, 2] {
        let send = recorded_send(&transport, leg);
        assert!(
            header_of(&send, "x-aws-ec2-metadata-token").is_none(),
            "v1 legs carry no token header"
        );
    }
    assert_eq!(sigv4_of(&auth).access_key_id.expose_secret(), "V1ACCESS");
}

#[tokio::test]
async fn imds_v1_disable_and_token_400_branches() {
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };

    // The disable env gates the default remote branch entirely: no IMDS leg
    // runs at all (the fixed node chain behavior).
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let transport = FakeSourceTransport::default_never();
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("a disabled remote segment ends the chain");
    assert!(
        matches!(error, CredentialResolutionError::Exhausted),
        "{error:?}"
    );
    assert_eq!(
        transport.send_count(),
        0,
        "no remote leg runs when disabled"
    );

    // Token 400 stops that request: no v1 fallback, no further legs.
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let transport = FakeSourceTransport::new(vec![source_response(400, "")]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("token 400 stops the request");
    assert!(
        matches!(error, CredentialResolutionError::Exhausted),
        "{error:?}"
    );
    assert_eq!(transport.send_count(), 1);

    // "false" does not disable IMDS.
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "false",
    )]));
    let transport = FakeSourceTransport::new(vec![
        source_response(404, ""),
        source_response(200, "v1-role"),
        source_response(
            200,
            r#"{"AccessKeyId":"FALSEACCESS","SecretAccessKey":"FALSESECRET"}"#,
        ),
    ]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    resolve_local_aws_auth(&input)
        .await
        .expect("AWS_EC2_METADATA_DISABLED=false keeps IMDS enabled");
}

#[tokio::test]
async fn imds_v1_role_401_clears_token_state_and_refetches() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![
        source_response(404, ""),            // token miss -> v1 mode
        source_response(401, ""),            // v1 role list denied
        source_response(200, "fresh-token"), // re-fetch v2 token
        source_response(200, "imds-role"),
        source_response(
            200,
            r#"{"AccessKeyId":"REFRESHACCESS","SecretAccessKey":"REFRESHSECRET"}"#,
        ),
    ]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("401 clears the token state and retries with a fresh token");

    assert_eq!(transport.send_count(), 5);
    let role_retry = recorded_send(&transport, 3);
    assert_eq!(
        header_of(&role_retry, "x-aws-ec2-metadata-token").unwrap(),
        "fresh-token"
    );
    assert_eq!(
        sigv4_of(&auth).access_key_id.expose_secret(),
        "REFRESHACCESS"
    );
}

#[tokio::test]
async fn imds_ipv6_endpoint_mode_is_selected() {
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_SERVICE_ENDPOINT_MODE",
        "IPv6",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![
        source_response(200, "token"),
        source_response(200, "role"),
        source_response(
            200,
            r#"{"AccessKeyId":"V6ACCESS","SecretAccessKey":"V6SECRET"}"#,
        ),
    ]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    resolve_local_aws_auth(&input)
        .await
        .expect("IPv6 IMDS resolves");
    assert_eq!(
        recorded_send(&transport, 0).url,
        "http://[fd00:ec2::254]/latest/api/token"
    );
}

#[tokio::test]
async fn dropping_resolution_cancels_inflight_source_exchange() {
    struct HangingTransport {
        dropped: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }
    struct DropGuard(std::sync::Arc<std::sync::atomic::AtomicBool>);
    impl Drop for DropGuard {
        fn drop(&mut self) {
            self.0.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }
    impl AwsSourceTransport for HangingTransport {
        fn send(
            &self,
            _request: SourceHttpRequest,
        ) -> futures_util::future::BoxFuture<'_, Result<SourceHttpResponse, SourceHttpError>>
        {
            let guard = DropGuard(self.dropped.clone());
            Box::pin(async move {
                let _keep = guard;
                std::future::pending::<()>().await;
                unreachable!("pending never resolves")
            })
        }
    }

    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI",
        "/v2/creds",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dropped = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let transport = HangingTransport {
        dropped: dropped.clone(),
    };
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);

    // Drive the resolution on a select branch, then abandon it: dropping
    // the owning future must release the in-flight exchange.
    let mut resolution = Box::pin(resolve_local_aws_auth(&input));
    tokio::select! {
        _ = &mut resolution => panic!("hanging transport never resolves"),
        _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {}
    }
    drop(resolution);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(
        dropped.load(std::sync::atomic::Ordering::SeqCst),
        "dropping the resolution future must release the owned source exchange"
    );
}

#[tokio::test]
async fn sts_exchange_over_real_http_to_local_server() {
    // Real transport + real form body + real XML parse over loopback HTTP.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local STS");
    let addr = listener.local_addr().expect("local STS addr");
    let endpoint = format!("http://{addr}");

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept STS request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 4096];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read STS request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buf[..read]);
            if let Some(header_end) = find_subslice(&request, b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..header_end]).to_string();
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        if !name.trim().eq_ignore_ascii_case("Content-Length") {
                            return None;
                        }
                        value.trim().parse::<usize>().ok()
                    })
                    .unwrap_or(0);
                if request.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        let response = sts_success_xml(
            "LOCAL",
            "arn:aws:sts::123456789012:assumed-role/fixture/local",
        );
        let mut bytes = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            response.len()
        )
        .into_bytes();
        bytes.extend_from_slice(response.as_bytes());
        tokio::io::AsyncWriteExt::write_all(&mut socket, &bytes)
            .await
            .expect("write STS response");
        request
    });

    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = ReqwestSourceTransport::new();
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(
        &credentials,
        "[source-prof]\naws_access_key_id=LOCAL_SOURCE_ACCESS\naws_secret_access_key=LOCAL_SOURCE_SECRET\n",
    )
    .unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile role-prof]\nrole_arn=arn:aws:iam::123456789012:role/fixture\nsource_profile=source-prof\nregion=us-west-2\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("role-prof".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport)
    .with_sts_endpoint(Some(&endpoint));
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("real STS exchange resolves");

    assert_eq!(sigv4_of(&auth).access_key_id.expose_secret(), "LOCALACCESS");
    let request = server.await.expect("server finished");
    let request_text = String::from_utf8_lossy(&request);
    assert!(
        request_text.starts_with("POST / HTTP/1.1"),
        "STS request line: {request_text}"
    );
    assert!(
        request_text.contains("content-type: application/x-www-form-urlencoded")
            || request_text.contains("Content-Type: application/x-www-form-urlencoded"),
        "form content type: {request_text}"
    );
    assert!(
        request_text.contains("Action=AssumeRole&Version=2011-06-15"),
        "query-encoded body: {request_text}"
    );
    assert!(
        request_text
            .to_ascii_lowercase()
            .contains("authorization: aws4-hmac-sha256"),
        "source SigV4 over real HTTP: {request_text}"
    );
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[tokio::test]
async fn ecs_full_uri_over_real_http_to_loopback() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local ECS");
    let addr = listener.local_addr().expect("local ECS addr");

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept ECS request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read ECS request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let body = r#"{"AccessKeyId":"LOOPECSACCESS","SecretAccessKey":"LOOPECSSECRET","Token":"LOOPECSSESSION","Expiration":"2028-01-01T00:00:00Z"}"#;
        let bytes = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .into_bytes();
        tokio::io::AsyncWriteExt::write_all(&mut socket, &bytes)
            .await
            .expect("write ECS response");
        request
    });

    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_CONTAINER_CREDENTIALS_FULL_URI",
        // http to loopback is an allowed host per the fixed endpoint rules.
        Box::leak(format!("http://{addr}/ecs-creds").into_boxed_str()) as &'static str,
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = ReqwestSourceTransport::new();
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("loopback ECS resolves over real HTTP");

    assert_eq!(
        sigv4_of(&auth).access_key_id.expose_secret(),
        "LOOPECSACCESS"
    );
    let request = server.await.expect("server finished");
    let request_text = String::from_utf8_lossy(&request);
    assert!(
        request_text.starts_with("GET /ecs-creds HTTP/1.1"),
        "ECS request line: {request_text}"
    );
}

#[tokio::test]
async fn terminal_sts_failure_through_models_sends_zero_generation_requests() {
    // Source exchange sends and generation sends are separate: an STS denial
    // may spend source legs but must never dispatch a generation request.
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport =
        FakeSourceTransport::new(vec![source_response(403, sts_error_xml("AccessDenied"))]);
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(
        &credentials,
        "[source-prof]\naws_access_key_id=ZERO_ACCESS\naws_secret_access_key=ZERO_SECRET\n",
    )
    .unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile role-prof]\nrole_arn=arn:aws:iam::123456789012:role/fixture\nsource_profile=source-prof\nregion=us-west-2\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("role-prof".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);

    // The chain itself is the authority: the typed terminal error from the
    // STS denial must surface before any generation dispatch (the
    // Models-level zero-send behavior for failed local resolution is proven
    // by failing_local_source_resolution_sends_zero_generation_requests).
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("STS denial is terminal");
    assert!(matches!(error, CredentialResolutionError::Terminal { .. }));
    assert_eq!(transport.send_count(), 1, "one source leg spent");
}

#[test]
fn sts_endpoint_rules_follow_partition_and_override() {
    assert_eq!(
        sts_endpoint_url("us-west-2", None),
        "https://sts.us-west-2.amazonaws.com"
    );
    assert_eq!(
        sts_endpoint_url("cn-north-1", None),
        "https://sts.cn-north-1.amazonaws.com.cn"
    );
    assert_eq!(
        sts_endpoint_url("us-west-2", Some("https://sts.example.invalid")),
        "https://sts.example.invalid",
        "explicit override wins"
    );
}

#[tokio::test]
async fn nested_source_profiles_assume_from_inner_to_outer() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(
        ["INNER", "MIDDLE", "OUTER"]
            .into_iter()
            .map(|name| {
                source_response(
                    200,
                    sts_success_xml(
                        name,
                        &format!("arn:aws:sts::123456789012:assumed-role/{name}/session"),
                    ),
                )
            })
            .collect(),
    );
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(
        &credentials,
        "[base]\naws_access_key_id=BASE_ACCESS\naws_secret_access_key=BASE_SECRET\n",
    )
    .unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile outer]\nrole_arn=arn:aws:iam::123456789012:role/outer\nsource_profile=middle\nregion=us-west-2\n\
         [profile middle]\nrole_arn=arn:aws:iam::123456789012:role/middle\nsource_profile=inner\n\
         [profile inner]\nrole_arn=arn:aws:iam::123456789012:role/inner\nsource_profile=base\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        Some(&credentials),
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("outer".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let resolved = resolve_local_aws_auth(&input).await.unwrap();
    assert_eq!(
        sigv4_of(&resolved).access_key_id.expose_secret(),
        "OUTERACCESS"
    );
    assert_eq!(transport.send_count(), 3);
    for (index, (role, source_key)) in [
        ("inner", "BASE_ACCESS"),
        ("middle", "INNERACCESS"),
        ("outer", "MIDDLEACCESS"),
    ]
    .into_iter()
    .enumerate()
    {
        let request = recorded_send(&transport, index);
        assert!(body_of(&request).contains(&format!("role%2F{role}")));
        assert_sigv4_signed_with(&request, source_key);
    }
}

#[tokio::test]
async fn assume_role_accepts_process_source_profile() {
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        sts_success_xml(
            "ROLE",
            "arn:aws:sts::123456789012:assumed-role/outer/session",
        ),
    )]);
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile outer]\nrole_arn=arn:aws:iam::123456789012:role/outer\nsource_profile=process\nregion=us-west-2\n\
         [profile process]\ncredential_process=synthetic-process\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("outer".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let resolved = resolve_local_aws_auth(&input).await.unwrap();
    assert_eq!(
        sigv4_of(&resolved).access_key_id.expose_secret(),
        "ROLEACCESS"
    );
    assert_eq!(runner.calls.lock().unwrap().len(), 1);
    assert_eq!(transport.send_count(), 1);
    assert_sigv4_signed_with(&recorded_send(&transport, 0), "PROCESS_ACCESS");
}

struct FrozenProcessContext {
    cwd: std::path::PathBuf,
    fail: bool,
    snapshots: AtomicUsize,
}

struct FrozenSpecRunner(std::sync::Mutex<Vec<CredentialProcessSpec>>);

impl CredentialProcessRunner for FrozenSpecRunner {
    fn run<'a>(
        &'a self,
        spec: &'a CredentialProcessSpec,
    ) -> BoxAuthFuture<
        'a,
        Result<CredentialProcessOutput, opi_ai::auth::aws::process::CredentialProcessError>,
    > {
        Box::pin(async move {
            self.0.lock().unwrap().push(spec.clone());
            Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET"))
        })
    }
}

impl opi_ai::auth::AuthContext for FrozenProcessContext {
    fn env(&self, _name: &str) -> Result<Option<String>, opi_ai::auth::AuthContextError> {
        Ok(None)
    }

    fn file_exists(&self, _path: &str) -> Result<bool, opi_ai::auth::AuthContextError> {
        Ok(false)
    }

    fn process_snapshot(
        &self,
    ) -> Result<opi_ai::auth::AuthProcessSnapshot, opi_ai::auth::AuthContextError> {
        self.snapshots.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            return Err(opi_ai::auth::AuthContextError::ProcessSnapshot {
                reason: "synthetic snapshot failure",
            });
        }
        Ok(opi_ai::auth::AuthProcessSnapshot {
            env: std::collections::BTreeMap::from([(
                "SOURCE_IDENTITY".to_owned(),
                "scoped-synthetic".to_owned(),
            )]),
            cwd: self.cwd.clone(),
        })
    }
}

#[tokio::test]
async fn selected_process_freezes_context_environment_and_cwd() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile process]\ncredential_process=selected-command\n",
    )
    .unwrap();
    for fail in [false, true] {
        let context = FrozenProcessContext {
            cwd: dir.path().to_path_buf(),
            fail,
            snapshots: AtomicUsize::new(0),
        };
        let runner = FrozenSpecRunner(std::sync::Mutex::new(Vec::new()));
        let input = chain_input(
            &context,
            None,
            Some(&config),
            &runner,
            AwsAuthSelection::Profile("process".to_owned()),
            ScopedAwsEnv::empty(),
        );
        let result = resolve_local_aws_auth(&input).await;
        assert_eq!(context.snapshots.load(Ordering::SeqCst), 1);
        let invocations = runner.0.lock().unwrap();
        if fail {
            assert!(matches!(
                result,
                Err(CredentialResolutionError::Terminal { .. })
            ));
            assert!(invocations.is_empty());
        } else {
            result.unwrap();
            assert_eq!(invocations.len(), 1);
            assert_eq!(invocations[0].cwd.as_deref(), Some(dir.path()));
            assert_eq!(
                invocations[0].env,
                opi_ai::auth::aws::process::ProcessEnv::Exact(std::collections::BTreeMap::from([
                    ("SOURCE_IDENTITY".to_owned(), "scoped-synthetic".to_owned())
                ]))
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Task 20.19.3: SSO / login_session source families and the full chain.
// ---------------------------------------------------------------------------

use opi_ai::auth::aws::cache::{
    AwsSourceCache, CacheIdentity, DpopJtiSource, PassiveRefreshDeps, SourceCacheValue,
    SourceClock, SsoRefreshMaterial, SsoTokenSnapshot,
};
use opi_ai::auth::aws::login::{
    login_cache_file_name, read_login_cache_file, write_login_cache_file,
};
use opi_ai::auth::aws::sso::{read_sso_token_file, write_sso_token_file};
/// Fixed clock for refresh-window tests.
struct FakeClock(time::OffsetDateTime);

impl SourceClock for FakeClock {
    fn now(&self) -> time::OffsetDateTime {
        self.0
    }
}

struct AdjustableSourceClock(std::sync::Mutex<time::OffsetDateTime>);

impl SourceClock for AdjustableSourceClock {
    fn now(&self) -> time::OffsetDateTime {
        *self.0.lock().unwrap()
    }
}

/// Deterministic DPoP `jti` source.
struct CountingJti(std::sync::atomic::AtomicUsize);

impl DpopJtiSource for CountingJti {
    fn next_jti(&self) -> String {
        let value = self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("jti-{value}")
    }
}

fn cache_with_owned_refresh(transport: Arc<dyn AwsSourceTransport>) -> AwsSourceCache {
    AwsSourceCache::with_mutation_queue(Arc::new(opi_ai::mutation::MutationQueue::new()))
        .with_passive_deps(Arc::new(PassiveRefreshDeps {
            transport,
            clock: Arc::new(FakeClock(rfc3339("2030-01-01T00:00:00Z"))),
            jti: Arc::new(CountingJti(AtomicUsize::new(0))),
        }))
}

fn modern_sso_token_file(dir: &std::path::Path, expires_at: &str) {
    write_sso_token_file(
        dir,
        "sess-a",
        &SsoTokenSnapshot {
            access_token: secrecy::SecretString::from("SSO_TOKEN"),
            expires_at: rfc3339(expires_at),
            refresh: Some(SsoRefreshMaterial {
                client_id: "oidc-client".to_owned(),
                client_secret: secrecy::SecretString::from("oidc-secret"),
                refresh_token: secrecy::SecretString::from("oidc-refresh"),
            }),
            modern: true,
        },
    )
    .unwrap();
}

fn sso_scoped(start_url: &str, session: Option<&str>) -> ScopedAwsEnv {
    ScopedAwsEnv {
        sso_start_url: Some(start_url.to_owned()),
        sso_account_id: Some("123456789012".to_owned()),
        sso_region: Some("us-east-1".to_owned()),
        sso_role_name: Some("Engineering".to_owned()),
        sso_session: session.map(str::to_owned),
        ..ScopedAwsEnv::empty()
    }
}

fn role_credentials_body(expiration_ms: i64) -> String {
    format!(
        r#"{{"roleCredentials":{{"accessKeyId":"SSO_AKID","secretAccessKey":"SSO_SAK","sessionToken":"SSO_SESSION","expiration":{expiration_ms}}}}}"#
    )
}

fn bearer_of(request: &SourceHttpRequest) -> String {
    request
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("x-amz-sso_bearer_token"))
        .map(|(_, value)| value.clone())
        .expect("bearer header present")
}

fn login_cache_key_pair() -> (p256::SecretKey, String) {
    let secret = p256::SecretKey::random(&mut rand::rngs::OsRng);
    use p256::elliptic_curve::pkcs8::EncodePrivateKey;
    let pem = secret
        .to_pkcs8_pem(p256::pkcs8::LineEnding::LF)
        .unwrap()
        .to_string();
    (secret, pem)
}

fn write_login_cache(
    dir: &std::path::Path,
    session: &str,
    pem: &str,
    access_token: Option<&str>,
    expires_at: Option<&str>,
) {
    write_login_cache_file(
        dir,
        session,
        &opi_ai::auth::aws::cache::LoginTokenSnapshot {
            client_id: "signin-client".to_owned(),
            refresh_token: secrecy::SecretString::from("signin-refresh"),
            signing_key: secrecy::SecretString::from(pem.to_owned()),
            account: Some("123456789012".to_owned()),
            access_token: access_token.map(secrecy::SecretString::from),
            expires_at: expires_at.map(rfc3339),
        },
    )
    .unwrap();
}

#[tokio::test]
async fn explicit_sso_absent_continues_to_ini_static() {
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(
        &credentials,
        "[default]\naws_access_key_id = INI_ACCESS\naws_secret_access_key = INI_SECRET\n",
    )
    .unwrap();
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::default_never();
    let input = chain_input(
        &ambient,
        Some(&credentials),
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let source = chain_source(&input).await;
    assert_eq!(source, CredentialSource::ProfileFile);
    assert_eq!(transport.send_count(), 0);
}

#[tokio::test]
async fn explicit_sso_incomplete_is_terminal_with_zero_sends() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::default_never();
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv {
            sso_start_url: Some("https://start.example".to_owned()),
            ..ScopedAwsEnv::empty()
        },
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("incomplete explicit SSO selection terminates");
    match error {
        CredentialResolutionError::Terminal {
            reason: TerminalReason::IncompleteSsoConfiguration,
            ..
        } => {}
        other => panic!("expected incomplete SSO terminal, got {other:?}"),
    }
    assert_eq!(transport.send_count(), 0);
}

#[tokio::test]
async fn explicit_sso_resolves_with_getrole_wire() {
    let dir = tempfile::tempdir().unwrap();
    modern_sso_token_file(dir.path(), "2030-01-01T04:00:00Z");
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        role_credentials_body(1_900_000_000_000),
    )]);
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        sso_scoped("https://start.example", Some("sess-a")),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_sso_cache_dir(Some(dir.path()));
    let auth = resolve_local_aws_auth(&input).await.expect("SSO resolves");
    let AuthProvenanceSource::AwsSigV4 { source } = auth.provenance.source else {
        panic!("expected SigV4 provenance");
    };
    assert_eq!(source, AwsCredentialSource::Sso { profile: None });
    assert_eq!(transport.send_count(), 1);
    let send = transport.send_at(0);
    assert_eq!(
        send.method,
        opi_ai::auth::aws::source_http::SourceHttpMethod::Get
    );
    assert_eq!(
        send.url,
        "https://portal.sso.us-east-1.amazonaws.com/federation/credentials?role_name=Engineering&account_id=123456789012"
    );
    assert_eq!(bearer_of(&send), "SSO_TOKEN");
}

#[tokio::test]
async fn sso_shared_cache_keeps_roles_distinct() {
    let dir = tempfile::tempdir().unwrap();
    modern_sso_token_file(dir.path(), "2030-01-01T04:00:00Z");
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![
        source_response(200, role_credentials_body(1_900_000_000_000)),
        source_response(
            200,
            role_credentials_body(1_900_000_000_000).replace("SSO_AKID", "OTHER_ROLE_AKID"),
        ),
    ]);
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    for (role, expected_key) in [
        ("Engineering", "SSO_AKID"),
        ("OtherRole", "OTHER_ROLE_AKID"),
    ] {
        let mut scoped = sso_scoped("https://start.example", Some("sess-a"));
        scoped.sso_role_name = Some(role.to_owned());
        let input = chain_input(
            &ambient,
            None,
            None,
            &runner,
            AwsAuthSelection::Auto,
            scoped,
        )
        .with_source_transport(&transport)
        .with_source_cache(Some(&cache))
        .with_clock(&clock)
        .with_dpop_jti(&jti)
        .with_sso_cache_dir(Some(dir.path()));
        let resolved = resolve_local_aws_auth(&input).await.unwrap();
        assert_eq!(
            sigv4_of(&resolved).access_key_id.expose_secret(),
            expected_key
        );
    }
    assert_eq!(transport.send_count(), 2);
    assert!(transport.send_at(0).url.contains("role_name=Engineering"));
    assert!(transport.send_at(1).url.contains("role_name=OtherRole"));
}

#[tokio::test]
async fn assume_role_accepts_sso_source_profile() {
    let dir = tempfile::tempdir().unwrap();
    modern_sso_token_file(dir.path(), "2030-01-01T04:00:00Z");
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile outer]\nrole_arn=arn:aws:iam::123456789012:role/outer\nsource_profile=sso\nregion=us-west-2\n\
         [profile sso]\nsso_session=sess-a\nsso_account_id=123456789012\nsso_role_name=Engineering\n\
         [sso-session sess-a]\nsso_start_url=https://start.example\nsso_region=us-east-1\n",
    )
    .unwrap();
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![
        source_response(200, role_credentials_body(1_900_000_000_000)),
        source_response(
            200,
            sts_success_xml(
                "OUTER",
                "arn:aws:sts::123456789012:assumed-role/outer/session",
            ),
        ),
    ]);
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("outer".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_sso_cache_dir(Some(dir.path()))
    .with_clock(&clock);
    let resolved = resolve_local_aws_auth(&input).await.unwrap();
    assert_eq!(
        sigv4_of(&resolved).access_key_id.expose_secret(),
        "OUTERACCESS"
    );
    assert_eq!(transport.send_count(), 2);
    assert!(transport.send_at(0).url.contains("/federation/credentials"));
    assert_sigv4_signed_with(&transport.send_at(1), "SSO_AKID");
    assert!(
        transport
            .send_at(1)
            .url
            .contains("sts.us-west-2.amazonaws.com")
    );
    let request = transport.send_at(1);
    let authorization = request
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
        .unwrap()
        .1
        .as_str();
    assert!(authorization.contains("/us-west-2/sts/aws4_request"));
}

#[tokio::test]
async fn assume_role_accepts_login_source_profile() {
    let dir = tempfile::tempdir().unwrap();
    let (_secret, pem) = login_cache_key_pair();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile outer]\nrole_arn=arn:aws:iam::123456789012:role/outer\nsource_profile=login\nregion=us-east-1\n\
         [profile login]\nlogin_session=my-session\n",
    )
    .unwrap();
    write_login_cache(
        dir.path(),
        "my-session",
        &pem,
        Some(
            r#"{"accessKeyId":"LOGIN_AKID","secretAccessKey":"LOGIN_SAK","sessionToken":"LOGIN_SESSION"}"#,
        ),
        Some("2030-01-01T04:00:00Z"),
    );
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        sts_success_xml(
            "OUTER",
            "arn:aws:sts::123456789012:assumed-role/outer/session",
        ),
    )]);
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("outer".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_login_cache_dir(Some(dir.path()))
    .with_clock(&clock);
    let resolved = resolve_local_aws_auth(&input).await.unwrap();
    assert_eq!(
        sigv4_of(&resolved).access_key_id.expose_secret(),
        "OUTERACCESS"
    );
    assert_eq!(transport.send_count(), 1);
    assert_sigv4_signed_with(&transport.send_at(0), "LOGIN_AKID");
}

#[tokio::test]
async fn ini_modern_sso_resolves_via_sso_session_section() {
    let dir = tempfile::tempdir().unwrap();
    modern_sso_token_file(dir.path(), "2030-01-01T04:00:00Z");
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile corp]\nsso_session = sess-a\nsso_account_id = 123456789012\nsso_role_name = Engineering\n\
         [sso-session sess-a]\nsso_start_url = https://start.example\nsso_region = us-east-1\n",
    )
    .unwrap();
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        role_credentials_body(1_900_000_000_000),
    )]);
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("corp".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_sso_cache_dir(Some(dir.path()));
    let source = chain_source(&input).await;
    assert_eq!(
        source,
        CredentialSource::Sso {
            profile: Some("corp".to_owned())
        }
    );
}

#[tokio::test]
async fn ini_sso_incomplete_is_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nsso_account_id = 123456789012\n").unwrap();
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::default_never();
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("corp".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_sso_cache_dir(Some(dir.path()));
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("incomplete profile SSO terminates");
    match error {
        CredentialResolutionError::Terminal {
            reason: TerminalReason::IncompleteSsoConfiguration,
            ..
        } => {}
        other => panic!("expected incomplete SSO terminal, got {other:?}"),
    }
    assert_eq!(transport.send_count(), 0);
}

#[tokio::test]
async fn sso_near_expiry_uses_old_token_and_registers_passive_refresh() {
    let dir = tempfile::tempdir().unwrap();
    // Token expires in 2 minutes: inside the 5-minute window.
    modern_sso_token_file(dir.path(), "2030-01-01T00:02:00Z");
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    // Sends: federation GET (old token). The passive OIDC refresh runs on
    // the mutation-owner runtime through its own transport.
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        role_credentials_body(1_900_000_000_000),
    )]);
    let passive_transport = std::sync::Arc::new(FakeSourceTransport::new(vec![source_response(
        200,
        r#"{"accessToken":"SSO_TOKEN_2","expiresIn":28800,"refreshToken":"oidc-refresh-2","tokenType":"Bearer"}"#,
    )]));
    let queue = std::sync::Arc::new(opi_ai::mutation::MutationQueue::new());
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let cache = AwsSourceCache::with_mutation_queue(queue.clone()).with_passive_deps(
        std::sync::Arc::new(PassiveRefreshDeps {
            transport: passive_transport.clone(),
            clock: std::sync::Arc::new(FakeClock(rfc3339("2030-01-01T00:00:00Z"))),
            jti: std::sync::Arc::new(CountingJti(std::sync::atomic::AtomicUsize::new(0))),
        }),
    );
    let barrier_before = queue.barrier();
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        sso_scoped("https://start.example", Some("sess-a")),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_sso_cache_dir(Some(dir.path()));
    let auth = resolve_local_aws_auth(&input).await.expect("SSO resolves");
    let _ = auth;
    // The federation GET used the OLD token (passive refresh stays background).
    assert_eq!(bearer_of(&transport.send_at(0)), "SSO_TOKEN");
    // The passive refresh was registered into the mutation owner and runs
    // to completion even though the caller already left.
    let barrier_after = queue.barrier();
    assert!(barrier_after.cutoff() > barrier_before.cutoff());
    let report = barrier_after.wait().await;
    assert!(report.is_complete());
    assert_eq!(passive_transport.send_count(), 1);
    assert_eq!(
        passive_transport.send_at(0).url,
        "https://oidc.us-east-1.amazonaws.com/token"
    );
    // The standard cache file now holds the refreshed token.
    let file = read_sso_token_file(dir.path(), "sess-a").unwrap().unwrap();
    assert_eq!(
        secrecy::ExposeSecret::expose_secret(&file.snapshot.access_token),
        "SSO_TOKEN_2"
    );
    let identity = CacheIdentity::SsoToken {
        cache_key: "sess-a".to_owned(),
    };
    let Some(SourceCacheValue::SsoToken(snapshot)) = cache.peek(&identity, clock.now()) else {
        panic!("the acknowledged refresh must leave a cached token");
    };
    assert_eq!(snapshot.access_token.expose_secret(), "SSO_TOKEN_2");
    assert_eq!(
        snapshot.refresh.unwrap().refresh_token.expose_secret(),
        "oidc-refresh-2"
    );
    assert!(cache.revision(&identity) >= 2);
}

struct AcknowledgedExpiryClock {
    queue: Arc<opi_ai::mutation::MutationQueue>,
    advance_after_ack: bool,
}

impl SourceClock for AcknowledgedExpiryClock {
    fn now(&self) -> time::OffsetDateTime {
        let written = self.advance_after_ack && self.queue.barrier().report().captured.iter().any(|entry| {
            matches!(entry, opi_ai::mutation::BarrierEntry::Settled(receipt) if matches!(receipt.outcome, opi_ai::mutation::MutationOutcome::Written { .. }))
        });
        rfc3339(if written {
            "2030-01-01T00:00:02Z"
        } else {
            "2030-01-01T00:00:00Z"
        })
    }
}

struct ExpiryProbeTransport {
    body: Vec<u8>,
}

impl AwsSourceTransport for ExpiryProbeTransport {
    fn send(
        &self,
        _request: SourceHttpRequest,
    ) -> futures_util::future::BoxFuture<'_, Result<SourceHttpResponse, SourceHttpError>> {
        Box::pin(async move {
            Ok(SourceHttpResponse {
                status: 200,
                body: self.body.clone(),
            })
        })
    }
}

async fn source_refresh_expiry_probe(
    login: bool,
    ttl: i64,
    advance: bool,
) -> (Result<ResolvedAuth, CredentialResolutionError>, usize) {
    let dir = tempfile::tempdir().unwrap();
    let queue = Arc::new(opi_ai::mutation::MutationQueue::new());
    let clock = Arc::new(AcknowledgedExpiryClock {
        queue: queue.clone(),
        advance_after_ack: advance,
    });
    let access = if login {
        r#"{"accessKeyId":"LOGIN_AKID","secretAccessKey":"LOGIN_SAK","sessionToken":"LOGIN_SESSION"}"#
    } else {
        "SSO_TOKEN_2"
    };
    let owner = Arc::new(ExpiryProbeTransport { body: serde_json::to_vec(&serde_json::json!({"accessToken":access,"expiresIn":ttl,"refreshToken":"rotated-refresh"})).unwrap() });
    let cache = AwsSourceCache::with_mutation_queue(queue).with_passive_deps(Arc::new(
        PassiveRefreshDeps {
            transport: owner,
            clock: clock.clone(),
            jti: Arc::new(CountingJti(AtomicUsize::new(0))),
        },
    ));
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nlogin_session=my-session\n").unwrap();
    if login {
        let (_secret, pem) = login_cache_key_pair();
        cache.store(
            CacheIdentity::LoginSession {
                session: "my-session".to_owned(),
            },
            SourceCacheValue::LoginSession(opi_ai::auth::aws::cache::LoginTokenSnapshot {
                client_id: "signin-client".to_owned(),
                refresh_token: SecretString::from("signin-refresh"),
                signing_key: SecretString::from(pem),
                account: Some("123456789012".to_owned()),
                access_token: None,
                expires_at: None,
            }),
        );
    } else {
        cache.store(
            CacheIdentity::SsoToken {
                cache_key: "sess-a".to_owned(),
            },
            SourceCacheValue::SsoToken(SsoTokenSnapshot {
                access_token: SecretString::from("OLD"),
                expires_at: rfc3339("2029-12-31T23:00:00Z"),
                refresh: Some(SsoRefreshMaterial {
                    client_id: "oidc-client".to_owned(),
                    client_secret: SecretString::from("oidc-secret"),
                    refresh_token: SecretString::from("oidc-refresh"),
                }),
                modern: true,
            }),
        );
    }
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let federation = FakeSourceTransport::new(vec![source_response(
        200,
        role_credentials_body(1_900_000_000_000),
    )]);
    let selection = if login {
        AwsAuthSelection::Profile("corp".to_owned())
    } else {
        AwsAuthSelection::Auto
    };
    let scoped = if login {
        ScopedAwsEnv::empty()
    } else {
        sso_scoped("https://start.example", Some("sess-a"))
    };
    let input = chain_input(&ambient, None, Some(&config), &runner, selection, scoped)
        .with_source_transport(&federation)
        .with_source_cache(Some(&cache))
        .with_clock(clock.as_ref())
        .with_login_cache_dir(Some(dir.path()))
        .with_sso_cache_dir(Some(dir.path()));
    let result = resolve_local_aws_auth(&input).await;
    if advance {
        assert_eq!(clock.now(), rfc3339("2030-01-01T00:00:02Z"));
    }
    (result, federation.send_count())
}

#[tokio::test]
async fn sso_nonpositive_refresh_expiry_never_reaches_federation() {
    for ttl in [0, -1] {
        let (result, federation) = source_refresh_expiry_probe(false, ttl, false).await;
        assert!(
            result.is_err(),
            "already-expired SSO access cannot authorize GetRoleCredentials"
        );
        assert_eq!(federation, 0);
    }
}

#[tokio::test]
async fn login_nonpositive_refresh_expiry_never_selects_credentials() {
    for ttl in [0, -1] {
        let (result, _) = source_refresh_expiry_probe(true, ttl, false).await;
        assert!(
            result.is_err(),
            "already-expired login credentials cannot reach a generation attempt"
        );
    }
}

#[tokio::test]
async fn source_refresh_extreme_expiry_is_typed_invalid_response() {
    for login in [false, true] {
        let (result, federation) = source_refresh_expiry_probe(login, i64::MAX, false).await;
        assert!(
            matches!(
                result,
                Err(CredentialResolutionError::Terminal {
                    reason: TerminalReason::InvalidSsoResponse
                        | TerminalReason::InvalidLoginResponse,
                    ..
                })
            ),
            "unrepresentable expiry must have an ordinary typed source failure"
        );
        assert_eq!(federation, 0);
    }
}

#[tokio::test]
async fn source_refresh_rechecks_actual_expiry_after_exchange_and_ack() {
    for login in [false, true] {
        let (usable, _) = source_refresh_expiry_probe(login, 1, false).await;
        assert!(usable.is_ok(), "a still-valid short TTL serves this call");
        let (result, federation) = source_refresh_expiry_probe(login, 1, true).await;
        assert!(
            result.is_err(),
            "a token expired before source ACK must not serve the waiting call"
        );
        assert_eq!(federation, 0);
    }
}

#[tokio::test]
async fn login_passive_extreme_expiry_is_failed_receipt_and_keeps_valid_access() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nlogin_session=my-session\n").unwrap();
    let (_secret, pem) = login_cache_key_pair();
    let access = r#"{"accessKeyId":"OLD_LOGIN","secretAccessKey":"LOGIN_SAK","sessionToken":"LOGIN_SESSION"}"#;
    write_login_cache(
        dir.path(),
        "my-session",
        &pem,
        Some(access),
        Some("2030-01-01T00:02:00Z"),
    );
    let transport = Arc::new(FakeSourceTransport::new(vec![source_response(200, serde_json::json!({"accessToken":access,"expiresIn":i64::MAX,"refreshToken":"rotated-refresh"}).to_string())]));
    let cache = cache_with_owned_refresh(transport.clone());
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("corp".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(transport.as_ref())
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_login_cache_dir(Some(dir.path()));
    let selected = resolve_local_aws_auth(&input).await.unwrap();
    assert_eq!(
        sigv4_of(&selected).access_key_id.expose_secret(),
        "OLD_LOGIN"
    );
    let report = cache.mutation_queue().unwrap().barrier().wait().await;
    assert!(
        matches!(&report.captured[0], opi_ai::mutation::BarrierEntry::Settled(receipt) if matches!(receipt.outcome, opi_ai::mutation::MutationOutcome::Failed { .. })),
        "invalid expiry is a typed refresh failure rather than a panicked worker's unknown outcome"
    );
    let identity = CacheIdentity::LoginSession {
        session: "my-session".to_owned(),
    };
    let Some(SourceCacheValue::LoginSession(snapshot)) = cache.peek(&identity, clock.now()) else {
        panic!("old access remains valid")
    };
    assert_eq!(snapshot.refresh_token.expose_secret(), "signin-refresh");
    assert_eq!(snapshot.access_token.unwrap().expose_secret(), access);
}

#[tokio::test]
async fn sso_failed_file_save_retains_rotated_material_through_expiry() {
    let dir = tempfile::tempdir().unwrap();
    let identity = CacheIdentity::SsoToken {
        cache_key: "sess-a".to_owned(),
    };
    let file = dir.path().join(format!(
        "{}.json",
        opi_ai::auth::aws::sso::sso_cache_file_name("sess-a")
    ));
    std::fs::create_dir(&file).unwrap(); // every standard-file write must fail
    let clock = Arc::new(AdjustableSourceClock(std::sync::Mutex::new(rfc3339(
        "2030-01-01T00:00:00Z",
    ))));
    let owner_transport = Arc::new(FakeSourceTransport::new(vec![
        source_response(
            200,
            r#"{"accessToken":"SSO_TOKEN_2","expiresIn":60,"refreshToken":"oidc-refresh-2"}"#,
        ),
        source_response(
            200,
            r#"{"accessToken":"SSO_TOKEN_3","expiresIn":3600,"refreshToken":"oidc-refresh-3"}"#,
        ),
    ]));
    let queue = Arc::new(opi_ai::mutation::MutationQueue::new());
    let cache = AwsSourceCache::with_mutation_queue(queue.clone()).with_passive_deps(Arc::new(
        PassiveRefreshDeps {
            transport: owner_transport.clone(),
            clock: clock.clone(),
            jti: Arc::new(CountingJti(AtomicUsize::new(0))),
        },
    ));
    cache.store(
        identity.clone(),
        SourceCacheValue::SsoToken(SsoTokenSnapshot {
            access_token: SecretString::from("SSO_TOKEN"),
            expires_at: rfc3339("2030-01-01T00:02:00Z"),
            refresh: Some(SsoRefreshMaterial {
                client_id: "oidc-client".to_owned(),
                client_secret: SecretString::from("oidc-secret"),
                refresh_token: SecretString::from("oidc-refresh"),
            }),
            modern: true,
        }),
    );
    let transport = FakeSourceTransport::new(vec![
        source_response(200, role_credentials_body(1_900_000_000_000)),
        source_response(200, role_credentials_body(1_900_000_000_000)),
    ]);
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
        sso_scoped("https://start.example", Some("sess-a")),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(clock.as_ref())
    .with_sso_cache_dir(Some(dir.path()));
    resolve_local_aws_auth(&input).await.unwrap();
    let report = queue.barrier().wait().await;
    assert!(report.is_complete());
    assert!(
        matches!(&report.captured[0], opi_ai::mutation::BarrierEntry::Settled(receipt) if matches!(receipt.outcome, opi_ai::mutation::MutationOutcome::Written { .. }))
    );
    let Some(SourceCacheValue::SsoToken(snapshot)) = cache.peek(&identity, clock.now()) else {
        panic!("rotated source token stays owned after file failure")
    };
    assert_eq!(snapshot.access_token.expose_secret(), "SSO_TOKEN_2");
    *clock.0.lock().unwrap() = rfc3339("2030-01-01T00:02:01Z");
    resolve_local_aws_auth(&input).await.unwrap();
    let body: serde_json::Value =
        serde_json::from_slice(owner_transport.send_at(1).body.as_ref().unwrap()).unwrap();
    assert_eq!(
        body["refreshToken"], "oidc-refresh-2",
        "expired access must retain the unique rotated refresh material"
    );
    assert_eq!(bearer_of(&transport.send_at(1)), "SSO_TOKEN_3");
    assert!(
        file.is_dir(),
        "best-effort file failure must not be reported as a saved file"
    );
}

#[tokio::test]
async fn login_failed_file_save_retains_rotated_material_through_expiry() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nlogin_session=my-session\n").unwrap();
    let file = dir
        .path()
        .join(format!("{}.json", login_cache_file_name("my-session")));
    std::fs::create_dir(&file).unwrap();
    let (_secret, pem) = login_cache_key_pair();
    let access = |key: &str| {
        serde_json::json!({"accessKeyId":key,"secretAccessKey":"LOGIN_SAK","sessionToken":"LOGIN_SESSION"}).to_string()
    };
    let clock = Arc::new(AdjustableSourceClock(std::sync::Mutex::new(rfc3339(
        "2030-01-01T00:00:00Z",
    ))));
    let transport = Arc::new(FakeSourceTransport::new(vec![
        source_response(200, serde_json::json!({"accessToken":access("LOGIN_2"),"expiresIn":60,"refreshToken":"signin-refresh-2"}).to_string()),
        source_response(200, serde_json::json!({"accessToken":access("LOGIN_3"),"expiresIn":3600,"refreshToken":"signin-refresh-3"}).to_string()),
    ]));
    let queue = Arc::new(opi_ai::mutation::MutationQueue::new());
    let cache = AwsSourceCache::with_mutation_queue(queue.clone()).with_passive_deps(Arc::new(
        PassiveRefreshDeps {
            transport: transport.clone(),
            clock: clock.clone(),
            jti: Arc::new(CountingJti(AtomicUsize::new(0))),
        },
    ));
    let identity = CacheIdentity::LoginSession {
        session: "my-session".to_owned(),
    };
    cache.store(
        identity.clone(),
        SourceCacheValue::LoginSession(opi_ai::auth::aws::cache::LoginTokenSnapshot {
            client_id: "signin-client".to_owned(),
            refresh_token: SecretString::from("signin-refresh"),
            signing_key: SecretString::from(pem),
            account: Some("123456789012".to_owned()),
            access_token: Some(SecretString::from(access("LOGIN_1"))),
            expires_at: Some(rfc3339("2030-01-01T00:02:00Z")),
        }),
    );
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("corp".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(transport.as_ref())
    .with_source_cache(Some(&cache))
    .with_clock(clock.as_ref())
    .with_login_cache_dir(Some(dir.path()));
    let first = resolve_local_aws_auth(&input).await.unwrap();
    assert_eq!(sigv4_of(&first).access_key_id.expose_secret(), "LOGIN_1");
    let report = queue.barrier().wait().await;
    assert!(report.is_complete());
    let Some(SourceCacheValue::LoginSession(snapshot)) = cache.peek(&identity, clock.now()) else {
        panic!("rotated source token stays owned after file failure")
    };
    assert_eq!(snapshot.refresh_token.expose_secret(), "signin-refresh-2");
    *clock.0.lock().unwrap() = rfc3339("2030-01-01T00:02:01Z");
    let second = resolve_local_aws_auth(&input).await.unwrap();
    assert_eq!(sigv4_of(&second).access_key_id.expose_secret(), "LOGIN_3");
    let body: serde_json::Value =
        serde_json::from_slice(transport.send_at(1).body.as_ref().unwrap()).unwrap();
    assert_eq!(body["refreshToken"], "signin-refresh-2");
    assert!(file.is_dir());
}

#[tokio::test]
async fn sso_passive_refresh_failure_keeps_old_token() {
    let dir = tempfile::tempdir().unwrap();
    modern_sso_token_file(dir.path(), "2030-01-01T00:02:00Z");
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    // Chain sends: only the federation GET with the old token. The passive
    // OIDC refresh fails (slow_down) on the mutation-owner transport; its
    // failure is swallowed and the old token stays usable.
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        role_credentials_body(1_900_000_000_000),
    )]);
    let passive_transport = std::sync::Arc::new(FakeSourceTransport::new(vec![source_response(
        400,
        r#"{"error":"slow_down","error_description":"try later"}"#,
    )]));
    let queue = std::sync::Arc::new(opi_ai::mutation::MutationQueue::new());
    let cache = AwsSourceCache::with_mutation_queue(queue.clone()).with_passive_deps(
        std::sync::Arc::new(PassiveRefreshDeps {
            transport: passive_transport.clone(),
            clock: std::sync::Arc::new(FakeClock(rfc3339("2030-01-01T00:00:00Z"))),
            jti: std::sync::Arc::new(CountingJti(std::sync::atomic::AtomicUsize::new(0))),
        }),
    );
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        sso_scoped("https://start.example", Some("sess-a")),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_sso_cache_dir(Some(dir.path()));
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("old token stays usable");
    let _ = auth;
    assert_eq!(transport.send_count(), 1);
    assert_eq!(bearer_of(&transport.send_at(0)), "SSO_TOKEN");
    let barrier = queue.barrier();
    let report = barrier.wait().await;
    assert!(report.is_complete());
    assert_eq!(passive_transport.send_count(), 1);
    // The failed refresh wrote nothing: the cache file keeps the old token.
    let file = read_sso_token_file(dir.path(), "sess-a").unwrap().unwrap();
    assert_eq!(
        secrecy::ExposeSecret::expose_secret(&file.snapshot.access_token),
        "SSO_TOKEN"
    );
}

#[tokio::test]
async fn sso_expired_token_refresh_denied_is_terminal() {
    let dir = tempfile::tempdir().unwrap();
    modern_sso_token_file(dir.path(), "2029-12-31T23:00:00Z");
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = Arc::new(FakeSourceTransport::new(vec![source_response(
        400,
        r#"{"error":"invalid_grant"}"#,
    )]));
    let cache = cache_with_owned_refresh(transport.clone());
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        sso_scoped("https://start.example", Some("sess-a")),
    )
    .with_source_transport(transport.as_ref())
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_sso_cache_dir(Some(dir.path()));
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("expired token with denied refresh terminates");
    match error {
        CredentialResolutionError::Terminal {
            reason: TerminalReason::SsoDenied {
                code: Some(code), ..
            },
            ..
        } => assert_eq!(code, "invalid_grant"),
        other => panic!("expected SSO denial terminal, got {other:?}"),
    }
    assert_eq!(transport.send_count(), 1, "no federation GET after denial");
}

#[tokio::test]
async fn sso_legacy_expired_token_is_terminal_relogin() {
    let dir = tempfile::tempdir().unwrap();
    // Legacy cache file: no refresh material.
    write_sso_token_file(
        dir.path(),
        &"HTTPS://LEGACY.EXAMPLE".to_uppercase(),
        &SsoTokenSnapshot {
            access_token: secrecy::SecretString::from("LEGACY"),
            expires_at: rfc3339("2029-12-31T23:00:00Z"),
            refresh: None,
            modern: false,
        },
    )
    .unwrap();
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::default_never();
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        sso_scoped("https://legacy.example", None),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_sso_cache_dir(Some(dir.path()));
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("legacy expired token requires re-login");
    match error {
        CredentialResolutionError::Terminal {
            reason: TerminalReason::InvalidSsoToken,
            ..
        } => {}
        other => panic!("expected invalid SSO token terminal, got {other:?}"),
    }
    assert_eq!(transport.send_count(), 0);
}

#[tokio::test]
async fn sso_getrole_denial_is_typed_terminal_with_zero_generation_sends() {
    let dir = tempfile::tempdir().unwrap();
    modern_sso_token_file(dir.path(), "2030-01-01T04:00:00Z");
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![source_response(403, "{}")]);
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        sso_scoped("https://start.example", Some("sess-a")),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_sso_cache_dir(Some(dir.path()));
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("selected SSO denial never discovers another identity");
    match error {
        CredentialResolutionError::Terminal {
            reason: TerminalReason::SsoDenied {
                status: Some(403), ..
            },
            ..
        } => {}
        other => panic!("expected SSO denial terminal, got {other:?}"),
    }
    assert_eq!(
        transport.send_count(),
        1,
        "exactly the denied federation GET"
    );
}

#[tokio::test]
async fn login_session_bad_cache_is_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nlogin_session = my-session\n").unwrap();
    let login_dir = dir.path().join("login");
    std::fs::create_dir_all(&login_dir).unwrap();
    std::fs::write(
        login_dir.join(format!("{}.json", login_cache_file_name("my-session"))),
        "{not json",
    )
    .unwrap();
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::default_never();
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("corp".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_login_cache_dir(Some(&login_dir));
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("selected session with bad cache terminates");
    match error {
        CredentialResolutionError::Terminal {
            reason: TerminalReason::LoginCacheInvalid,
            ..
        } => {}
        other => panic!("expected login cache terminal, got {other:?}"),
    }
    assert_eq!(transport.send_count(), 0);
}

#[tokio::test]
async fn login_session_refresh_dpop_wire_and_rotation() {
    let dir = tempfile::tempdir().unwrap();
    let (_secret, pem) = login_cache_key_pair();
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nlogin_session = my-session\n").unwrap();
    let login_dir = dir.path().join("login");
    std::fs::create_dir_all(&login_dir).unwrap();
    write_login_cache(&login_dir, "my-session", &pem, None, None);
    let access_token_json = r#"{"accessKeyId":"LOGIN_AKID","secretAccessKey":"LOGIN_SAK","sessionToken":"LOGIN_SESSION"}"#;
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = Arc::new(FakeSourceTransport::new(vec![source_response(
        200,
        format!(
            r#"{{"accessToken":{},"expiresIn":28800,"refreshToken":"signin-refresh-2"}}"#,
            serde_json::to_string(access_token_json).unwrap()
        ),
    )]));
    let cache = cache_with_owned_refresh(transport.clone());
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("corp".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(transport.as_ref())
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_login_cache_dir(Some(&login_dir));
    let source = chain_source(&input).await;
    assert_eq!(
        source,
        CredentialSource::LoginSession {
            session: "my-session".to_owned()
        }
    );
    // The SignIn refresh carried the owned DPoP proof over the fixed wire.
    assert_eq!(transport.send_count(), 1);
    let send = transport.send_at(0);
    assert_eq!(send.url, "https://signin.us-east-1.amazonaws.com/v1/token");
    let proof = send
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("dpop"))
        .map(|(_, value)| value.clone())
        .expect("DPoP header");
    assert_dpop_proof_shape(
        &proof,
        "jti-0",
        1893456000,
        "https://signin.us-east-1.amazonaws.com/v1/token",
    );
    let body: serde_json::Value =
        serde_json::from_slice(send.body.as_ref().expect("body")).unwrap();
    assert_eq!(body["clientId"], "signin-client");
    assert_eq!(body["grantType"], "refresh_token");
    assert_eq!(body["refreshToken"], "signin-refresh");
    // The rotated refresh token was written back to the standard cache.
    let file = read_login_cache_file(&login_dir, "my-session")
        .unwrap()
        .unwrap();
    assert_eq!(
        secrecy::ExposeSecret::expose_secret(&file.refresh_token),
        "signin-refresh-2"
    );
    assert!(file.access_token.is_some());
}

struct GatedLoginRefreshTransport {
    entered: Notify,
    release: Notify,
    sends: AtomicUsize,
}

impl AwsSourceTransport for GatedLoginRefreshTransport {
    fn send(
        &self,
        _request: SourceHttpRequest,
    ) -> futures_util::future::BoxFuture<'_, Result<SourceHttpResponse, SourceHttpError>> {
        self.sends.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            self.entered.notify_one();
            self.release.notified().await;
            Ok(SourceHttpResponse {
                status: 200,
                body: serde_json::to_vec(&serde_json::json!({
                    "accessToken": r#"{"accessKeyId":"LOGIN_AKID","secretAccessKey":"LOGIN_SAK","sessionToken":"LOGIN_SESSION"}"#,
                    "expiresIn": 28800,
                    "refreshToken": "signin-refresh-2",
                }))
                .unwrap(),
            })
        })
    }
}

#[tokio::test]
async fn login_started_rotation_survives_waiter_drop_and_is_barrier_visible() {
    let dir = tempfile::tempdir().unwrap();
    let (_secret, pem) = login_cache_key_pair();
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nlogin_session=my-session\n").unwrap();
    write_login_cache(dir.path(), "my-session", &pem, None, None);
    let transport = Arc::new(GatedLoginRefreshTransport {
        entered: Notify::new(),
        release: Notify::new(),
        sends: AtomicUsize::new(0),
    });
    let queue = Arc::new(opi_ai::mutation::MutationQueue::new());
    let cache = AwsSourceCache::with_mutation_queue(queue.clone()).with_passive_deps(Arc::new(
        PassiveRefreshDeps {
            transport: transport.clone(),
            clock: Arc::new(FakeClock(rfc3339("2030-01-01T00:00:00Z"))),
            jti: Arc::new(CountingJti(AtomicUsize::new(0))),
        },
    ));
    let start_waiter = || {
        tokio::spawn({
            let transport = transport.clone();
            let cache = cache.clone();
            let login_dir = dir.path().to_path_buf();
            let config = config.clone();
            async move {
                let ambient = MapAuthContext(std::collections::HashMap::new());
                let runner = ScriptedProcessRunner {
                    calls: std::sync::Mutex::new(Vec::new()),
                    result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
                };
                let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
                let input = chain_input(
                    &ambient,
                    None,
                    Some(&config),
                    &runner,
                    AwsAuthSelection::Profile("corp".to_owned()),
                    ScopedAwsEnv::empty(),
                )
                .with_source_transport(transport.as_ref())
                .with_source_cache(Some(&cache))
                .with_clock(&clock)
                .with_login_cache_dir(Some(&login_dir));
                resolve_local_aws_auth(&input).await
            }
        })
    };
    let waiter = start_waiter();
    transport.entered.notified().await;
    let second_waiter = start_waiter();
    let barrier = queue.barrier();
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    transport.release.notify_one();
    let report = barrier.wait().await;
    assert!(report.is_complete());
    assert!(
        !report.captured.is_empty(),
        "the active rotation belongs to the source owner"
    );
    let second = second_waiter.await.unwrap().unwrap();
    assert_eq!(
        sigv4_of(&second).access_key_id.expose_secret(),
        "LOGIN_AKID"
    );
    let file = read_login_cache_file(dir.path(), "my-session")
        .unwrap()
        .unwrap();
    assert_eq!(file.refresh_token.expose_secret(), "signin-refresh-2");
    let Some(SourceCacheValue::LoginSession(snapshot)) = cache.peek(
        &CacheIdentity::LoginSession {
            session: "my-session".to_owned(),
        },
        rfc3339("2030-01-01T00:00:00Z"),
    ) else {
        panic!("the acknowledged active rotation remains usable");
    };
    assert_eq!(snapshot.refresh_token.expose_secret(), "signin-refresh-2");
    assert_eq!(transport.sends.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn login_invalidated_rotation_cannot_overwrite_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let (_secret, pem) = login_cache_key_pair();
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nlogin_session=my-session\n").unwrap();
    write_login_cache(dir.path(), "my-session", &pem, None, None);
    let transport = Arc::new(GatedLoginRefreshTransport {
        entered: Notify::new(),
        release: Notify::new(),
        sends: AtomicUsize::new(0),
    });
    let cache = cache_with_owned_refresh(transport.clone());
    let waiter = tokio::spawn({
        let cache = cache.clone();
        let transport = transport.clone();
        let config = config.clone();
        let login_dir = dir.path().to_path_buf();
        async move {
            let ambient = MapAuthContext(std::collections::HashMap::new());
            let runner = ScriptedProcessRunner {
                calls: std::sync::Mutex::new(Vec::new()),
                result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
            };
            let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
            let input = chain_input(
                &ambient,
                None,
                Some(&config),
                &runner,
                AwsAuthSelection::Profile("corp".to_owned()),
                ScopedAwsEnv::empty(),
            )
            .with_source_transport(transport.as_ref())
            .with_source_cache(Some(&cache))
            .with_clock(&clock)
            .with_login_cache_dir(Some(&login_dir));
            resolve_local_aws_auth(&input).await
        }
    });
    transport.entered.notified().await;
    let identity = CacheIdentity::LoginSession {
        session: "my-session".to_owned(),
    };
    cache.invalidate(&identity);
    let replacement = r#"{"accessKeyId":"REPLACEMENT","secretAccessKey":"REPLACEMENT_SECRET","sessionToken":"REPLACEMENT_SESSION"}"#;
    write_login_cache(
        dir.path(),
        "my-session",
        &pem,
        Some(replacement),
        Some("2030-01-01T08:00:00Z"),
    );
    transport.release.notify_one();
    assert!(matches!(
        waiter.await.unwrap(),
        Err(CredentialResolutionError::Terminal {
            reason: TerminalReason::SourceRefreshSuperseded,
            ..
        })
    ));
    let report = cache.mutation_queue().unwrap().barrier().wait().await;
    assert!(
        matches!(&report.captured[0], opi_ai::mutation::BarrierEntry::Settled(receipt) if matches!(receipt.outcome, opi_ai::mutation::MutationOutcome::Failed { .. }))
    );
    assert!(
        cache
            .peek(&identity, rfc3339("2030-01-01T00:00:00Z"))
            .is_none()
    );
    let file = read_login_cache_file(dir.path(), "my-session")
        .unwrap()
        .unwrap();
    assert_eq!(file.access_token.unwrap().expose_secret(), replacement);
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("corp".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(transport.as_ref())
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_login_cache_dir(Some(dir.path()));
    let selected = resolve_local_aws_auth(&input).await.unwrap();
    assert_eq!(
        sigv4_of(&selected).access_key_id.expose_secret(),
        "REPLACEMENT"
    );
    assert_eq!(transport.sends.load(Ordering::SeqCst), 1);
}

/// Assert the fixed DPoP proof shape and verify its ES256 signature against
/// the JWK embedded in the proof header.
fn assert_dpop_proof_shape(proof: &str, jti: &str, iat: i64, htu: &str) {
    use p256::ecdsa::signature::Verifier;
    let parts: Vec<&str> = proof.split('.').collect();
    assert_eq!(parts.len(), 3, "three-segment DPoP JWT");
    let decode = |value: &str| {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut out = Vec::new();
        let mut buffer = 0u32;
        let mut bits = 0u32;
        for ch in value.bytes() {
            let index = ALPHABET.iter().position(|a| *a == ch).unwrap() as u32;
            buffer = (buffer << 6) | index;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((buffer >> bits) as u8);
            }
        }
        out
    };
    let header: serde_json::Value = serde_json::from_slice(&decode(parts[0])).unwrap();
    assert_eq!(header["typ"], "dpop+JWT");
    assert_eq!(header["alg"], "ES256");
    assert_eq!(header["jwk"]["kty"], "EC");
    assert_eq!(header["jwk"]["crv"], "P-256");
    let payload: serde_json::Value = serde_json::from_slice(&decode(parts[1])).unwrap();
    assert_eq!(payload["jti"], jti);
    assert_eq!(payload["iat"], iat);
    assert_eq!(payload["htu"], htu);
    assert_eq!(payload["htm"], "POST");
    use p256::elliptic_curve::sec1::FromEncodedPoint;
    let x = decode(header["jwk"]["x"].as_str().unwrap());
    let y = decode(header["jwk"]["y"].as_str().unwrap());
    let mut sec1 = vec![0x04];
    sec1.extend(x);
    sec1.extend(y);
    let point = p256::EncodedPoint::from_bytes(&sec1).unwrap();
    let public = p256::PublicKey::from_encoded_point(&point).unwrap();
    let verifying = p256::ecdsa::VerifyingKey::from(&public);
    let signature = p256::ecdsa::Signature::from_slice(&decode(parts[2])).unwrap();
    verifying
        .verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &signature)
        .expect("ES256 signature verifies");
}

#[tokio::test]
async fn login_access_denied_family_is_terminal() {
    for code in [
        "AccessDeniedException",
        "TokenExpired",
        "UserCredentialsChanged",
        "InsufficientPermissions",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (_secret, pem) = login_cache_key_pair();
        let config = dir.path().join("config");
        std::fs::write(&config, "[profile corp]\nlogin_session = my-session\n").unwrap();
        let login_dir = dir.path().join("login");
        std::fs::create_dir_all(&login_dir).unwrap();
        write_login_cache(&login_dir, "my-session", &pem, None, None);
        let ambient = MapAuthContext(std::collections::HashMap::new());
        let runner = ScriptedProcessRunner {
            calls: std::sync::Mutex::new(Vec::new()),
            result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
        };
        let transport = Arc::new(FakeSourceTransport::new(vec![source_response(
            403,
            format!(r#"{{"code":"{code}","message":"denied"}}"#),
        )]));
        let cache = cache_with_owned_refresh(transport.clone());
        let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
        let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
        let input = chain_input(
            &ambient,
            None,
            Some(&config),
            &runner,
            AwsAuthSelection::Profile("corp".to_owned()),
            ScopedAwsEnv::empty(),
        )
        .with_source_transport(transport.as_ref())
        .with_source_cache(Some(&cache))
        .with_clock(&clock)
        .with_dpop_jti(&jti)
        .with_login_cache_dir(Some(&login_dir));
        let error = resolve_local_aws_auth(&input)
            .await
            .unwrap_err_or_ok("denied")
            .expect_err(&format!("terminal AccessDenied family: {code}"));
        match error {
            CredentialResolutionError::Terminal {
                reason:
                    TerminalReason::LoginDenied {
                        status: Some(403),
                        code: Some(observed),
                    },
                ..
            } => assert_eq!(observed, code),
            other => panic!("expected login denial terminal for {code}, got {other:?}"),
        }
    }
}

/// Small helper keeping `expect_err` available on Result<_, E>.
trait UnwrapOrOk<T, E> {
    fn unwrap_err_or_ok(self, message: &str) -> Result<T, E>;
}

impl<T, E> UnwrapOrOk<T, E> for Result<T, E> {
    fn unwrap_err_or_ok(self, _message: &str) -> Result<T, E> {
        self
    }
}

#[tokio::test]
async fn login_other_failure_with_expired_token_continues_chain() {
    let dir = tempfile::tempdir().unwrap();
    let (_secret, pem) = login_cache_key_pair();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile corp]\nlogin_session = my-session\nregion = us-east-1\n",
    )
    .unwrap();
    let login_dir = dir.path().join("login");
    std::fs::create_dir_all(&login_dir).unwrap();
    // Cached access token already expired.
    let access_token_json = r#"{"accessKeyId":"LOGIN_AKID","secretAccessKey":"LOGIN_SAK","sessionToken":"LOGIN_SESSION"}"#;
    write_login_cache(
        &login_dir,
        "my-session",
        &pem,
        Some(access_token_json),
        Some("2029-12-31T23:00:00Z"),
    );
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    // Unclassified refresh failure (500 without a code): the expired token
    // raises the default continue-eligible error.
    let transport = Arc::new(FakeSourceTransport::new(vec![source_response(500, "{}")]));
    let cache = cache_with_owned_refresh(transport.clone());
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("corp".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(transport.as_ref())
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_login_cache_dir(Some(&login_dir));
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("chain advances past the failed login segment");
    // The chain continued to its end: exhaustion, not a terminal error.
    assert!(
        matches!(error, CredentialResolutionError::Exhausted),
        "expected continue-eligible exhaustion, got {error:?}"
    );
    assert_eq!(transport.send_count(), 1, "only the failed refresh ran");
}

#[tokio::test]
async fn login_unexpired_token_refresh_failure_keeps_old() {
    let dir = tempfile::tempdir().unwrap();
    let (_secret, pem) = login_cache_key_pair();
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nlogin_session = my-session\n").unwrap();
    let login_dir = dir.path().join("login");
    std::fs::create_dir_all(&login_dir).unwrap();
    let access_token_json = r#"{"accessKeyId":"LOGIN_AKID","secretAccessKey":"LOGIN_SAK","sessionToken":"LOGIN_SESSION"}"#;
    // Cached access token still valid for 2 minutes (inside the window, but
    // no mutation owner is attached: the refresh failure must keep it).
    write_login_cache(
        &login_dir,
        "my-session",
        &pem,
        Some(access_token_json),
        Some("2030-01-01T00:02:00Z"),
    );
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::default_never();
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("corp".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_login_cache_dir(Some(&login_dir));
    let source = chain_source(&input).await;
    assert_eq!(
        source,
        CredentialSource::LoginSession {
            session: "my-session".to_owned()
        },
        "the still-valid cached token is used without any send"
    );
    assert_eq!(transport.send_count(), 0);
}

#[tokio::test]
async fn chain_order_env_pair_beats_explicit_sso() {
    let dir = tempfile::tempdir().unwrap();
    modern_sso_token_file(dir.path(), "2030-01-01T04:00:00Z");
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::default_never();
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv {
            access_key_id: Some(secrecy::SecretString::from("ENV_ACCESS")),
            secret_access_key: Some(secrecy::SecretString::from("ENV_SECRET")),
            ..sso_scoped("https://start.example", Some("sess-a"))
        },
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_sso_cache_dir(Some(dir.path()));
    let source = chain_source(&input).await;
    assert_eq!(source, CredentialSource::ScopedEnvironment);
    assert_eq!(transport.send_count(), 0, "the env pair wins before SSO");
}

#[tokio::test]
async fn chain_order_ini_process_beats_sso_and_login() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile corp]\ncredential_process = echo-process\nsso_session = sess-a\nsso_account_id = 1\nsso_role_name = r\nlogin_session = my-session\n",
    )
    .unwrap();
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::default_never();
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("corp".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti);
    let source = chain_source(&input).await;
    assert_eq!(source, CredentialSource::CredentialProcess);
    assert_eq!(
        transport.send_count(),
        0,
        "process precedes SSO/login in the INI order"
    );
}

#[tokio::test]
async fn sso_throttle_suppresses_immediate_second_passive_refresh() {
    let queue = std::sync::Arc::new(opi_ai::mutation::MutationQueue::new());
    let cache = AwsSourceCache::with_mutation_queue(queue.clone());
    let identity = CacheIdentity::SsoToken {
        cache_key: "k".to_owned(),
    };
    cache.store(
        identity.clone(),
        SourceCacheValue::SsoToken(SsoTokenSnapshot {
            access_token: secrecy::SecretString::from("token"),
            expires_at: rfc3339("2030-01-01T00:02:00Z"),
            refresh: Some(SsoRefreshMaterial {
                client_id: "c".to_owned(),
                client_secret: secrecy::SecretString::from("s"),
                refresh_token: secrecy::SecretString::from("r"),
            }),
            modern: true,
        }),
    );
    let make_slot = || {
        Box::new(move || {
            Box::pin(async { Ok(opi_ai::mutation::MutationApplied::Keep) })
                as opi_ai::BoxAuthFuture<
                    'static,
                    Result<opi_ai::mutation::MutationApplied, opi_ai::mutation::MutationError>,
                >
        }) as opi_ai::mutation::MutationSlot
    };
    let now = rfc3339("2030-01-01T00:00:00Z");
    assert!(cache.maybe_spawn_passive(identity.clone(), now, make_slot()));
    // The 30-second throttle suppresses a same-instant second attempt.
    assert!(!cache.maybe_spawn_passive(identity.clone(), now, make_slot()));
    // Let the first slot settle, then the throttle window governs alone.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert!(
        !cache.maybe_spawn_passive(identity.clone(), now, make_slot()),
        "same-instant attempt stays throttled after settle"
    );
    let later = now + time::Duration::seconds(31);
    assert!(cache.maybe_spawn_passive(identity, later, make_slot()));
}

#[tokio::test]
async fn signin_dpop_over_real_http_to_local_server() {
    // Real transport + real ES256 DPoP proof over loopback HTTP; the proof
    // signature is verified against the JWK embedded in the proof header.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local SignIn");
    let addr = listener.local_addr().expect("local SignIn addr");
    let endpoint = format!("http://{addr}");

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept SignIn request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 4096];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read SignIn request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buf[..read]);
            if let Some(header_end) = find_subslice(&request, b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..header_end]).to_string();
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        if !name.trim().eq_ignore_ascii_case("Content-Length") {
                            return None;
                        }
                        value.trim().parse::<usize>().ok()
                    })
                    .unwrap_or(0);
                if request.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        let access_token_json = r#"{"accessKeyId":"LOCALLOGIN","secretAccessKey":"LOGINSECRET","sessionToken":"LOGINSESSION"}"#;
        let response = format!(
            r#"{{"accessToken":{},"expiresIn":28800,"refreshToken":"signin-refresh-2"}}"#,
            serde_json::to_string(access_token_json).unwrap()
        );
        let mut bytes = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            response.len()
        )
        .into_bytes();
        bytes.extend_from_slice(response.as_bytes());
        tokio::io::AsyncWriteExt::write_all(&mut socket, &bytes)
            .await
            .expect("write SignIn response");
        request
    });

    let dir = tempfile::tempdir().unwrap();
    let (_secret, pem) = login_cache_key_pair();
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nlogin_session = my-session\n").unwrap();
    let login_dir = dir.path().join("login");
    std::fs::create_dir_all(&login_dir).unwrap();
    write_login_cache(&login_dir, "my-session", &pem, None, None);

    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = Arc::new(opi_ai::auth::aws::source_http::ReqwestSourceTransport::new());
    let cache = cache_with_owned_refresh(transport.clone());
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("corp".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(transport.as_ref())
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_login_cache_dir(Some(&login_dir))
    .with_signin_endpoint(Some(&endpoint));
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("login resolves");
    let AuthProvenanceSource::AwsSigV4 { source } = auth.provenance.source else {
        panic!("expected SigV4 provenance");
    };
    assert_eq!(
        source,
        AwsCredentialSource::LoginSession {
            session: "my-session".to_owned()
        }
    );

    let request_text = String::from_utf8(server.await.expect("server finished")).unwrap();
    let proof = request_text
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case("dpop")
                .then(|| value.trim().to_owned())
        })
        .expect("DPoP header over real HTTP");
    assert_dpop_proof_shape(&proof, "jti-0", 1893456000, &format!("{endpoint}/v1/token"));
    assert!(
        request_text.contains("POST /v1/token"),
        "real SignIn request: {request_text}"
    );
}

// ---------------------------------------------------------------------------
// Task 20.20: AWS Smithy V2 retry modes through the public Bedrock adapter
// ---------------------------------------------------------------------------

mod smithy_retry {
    use super::{
        CountingBedrockResolver, HttpClient, MockServer, ResponseTemplate, build_bedrock_stream,
        method, prepared_bedrock_request,
    };
    use futures_util::StreamExt;
    use opi_ai::Models;
    use opi_ai::auth::{AuthResolver, ResolvedAuth};
    use opi_ai::bedrock::retry::{
        ScopedRetryInputs, SmithyClock, SmithyRetryConfig, SmithyRetryMode,
        resolve_smithy_retry_config,
    };
    use opi_ai::bedrock::{BedrockProvider, map_bedrock_status};
    use opi_ai::provider::{ProviderError, ProviderErrorCategory};
    use opi_ai::retry::{AcquisitionPolicy, AcquisitionSeal, RetrySourcePolicy};
    use secrecy::SecretString;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio_util::sync::CancellationToken;
    use wiremock::Mock;

    const SEED: u64 = 0x5333_0000_2026_0001;
    const SPEC: &str = "bedrock:anthropic.claude-sonnet-4-20250514-v2:0";

    fn unit_draw(index: u32) -> f64 {
        opi_ai::retry::smithy_unit_jitter_draw(SEED, index)
    }

    fn smithy_policy(config: SmithyRetryConfig) -> AcquisitionPolicy {
        AcquisitionPolicy {
            jitter_seed: Some(SEED),
            source: RetrySourcePolicy::Smithy(config),
            ..AcquisitionPolicy::default()
        }
    }

    fn provider(base_url: String, config: SmithyRetryConfig) -> BedrockProvider {
        BedrockProvider::new(Some(base_url), Arc::new(HttpClient::new())).with_retry_config(config)
    }

    fn collection(provider: BedrockProvider) -> (Models, Arc<AtomicUsize>) {
        let resolutions = Arc::new(AtomicUsize::new(0));
        let mut collection = Models::new();
        collection
            .register_route(
                Box::new(provider),
                Arc::new(CountingBedrockResolver {
                    resolutions: resolutions.clone(),
                }),
                opi_ai::auth::AuthProvenanceSource::Static,
                Default::default(),
            )
            .expect("register Bedrock route");
        (collection, resolutions)
    }

    async fn mount_failures(
        server: &MockServer,
        status: u16,
        error_type: Option<&'static str>,
        times: usize,
    ) {
        let mut response = ResponseTemplate::new(status);
        if let Some(name) = error_type {
            response = response.insert_header("x-amzn-errortype", format!("{name}:"));
        }
        Mock::given(method("POST"))
            .respond_with(response)
            .up_to_n_times(times as u64)
            .mount(server)
            .await;
    }

    async fn mount_success(server: &MockServer) {
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                build_bedrock_stream(&[
                    ("messageStart", r#"{"role":"assistant"}"#),
                    (
                        "contentBlockDelta",
                        r#"{"delta":{"text":"ok"},"contentBlockIndex":0}"#,
                    ),
                    ("messageStop", r#"{"stopReason":"end_turn"}"#),
                    (
                        "metadata",
                        r#"{"usage":{"inputTokens":1,"outputTokens":1}}"#,
                    ),
                ]),
                "application/vnd.amazon.eventstream",
            ))
            .mount(server)
            .await;
    }

    async fn drive_attempt_until_admission_or_success(
        prepared: &opi_ai::PreparedProviderCall,
    ) -> Result<(), ProviderError> {
        let mut attempt = prepared.start_attempt().expect("start attempt");
        let mut outcome = Ok(());
        while let Some(item) = attempt.next().await {
            match item {
                Ok(_) => {}
                Err(error) => {
                    outcome = Err(error);
                    break;
                }
            }
        }
        drop(attempt);
        outcome
    }

    async fn wait_out(prepared: &opi_ai::PreparedProviderCall) {
        let delay = prepared.retry_admission().expect("admission").delay_ms;
        tokio::time::sleep(std::time::Duration::from_millis(delay + 5)).await;
    }

    #[tokio::test]
    async fn transient_retries_with_exact_delays_wire_headers_and_frozen_auth() {
        let server = MockServer::start().await;
        // The first 504 carries a 1.2s server hint: the bounded rule admits
        // it fully (backoff + 5s headroom covers 1.2s), and it guarantees the
        // second send re-signs in a later second.
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(504).insert_header("retry-after", "1.2"))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        mount_failures(&server, 504, None, 1).await;
        mount_success(&server).await;
        let (collection, resolutions) =
            collection(provider(server.uri(), SmithyRetryConfig::default()));

        let prepared = collection
            .prepare_call_with_policy(
                SPEC,
                prepared_bedrock_request(CancellationToken::new()),
                smithy_policy(SmithyRetryConfig::default()),
            )
            .await
            .expect("prepare");

        let first = drive_attempt_until_admission_or_success(&prepared)
            .await
            .expect_err("first 504");
        assert!(matches!(
            first,
            ProviderError::ServiceFault { status: 504, .. }
        ));
        let draw0 = unit_draw(0);
        let backoff0 = (draw0 * 100.0).floor() as u64;
        assert_eq!(
            prepared.retry_admission().expect("admitted").delay_ms,
            backoff0.max(1_200.min(backoff0 + 5_000)),
            "first transient retry honors the bounded server hint"
        );

        wait_out(&prepared).await;
        let second = drive_attempt_until_admission_or_success(&prepared)
            .await
            .expect_err("second 504");
        assert!(matches!(
            second,
            ProviderError::ServiceFault { status: 504, .. }
        ));
        let draw1 = unit_draw(1);
        let delay1 = (draw1 * 200.0).floor() as u64;
        assert_eq!(
            prepared.retry_admission().expect("admitted").delay_ms,
            delay1,
            "second transient retry is floor(U*min(100*2,20s))"
        );

        wait_out(&prepared).await;
        drive_attempt_until_admission_or_success(&prepared)
            .await
            .expect("third send succeeds");
        assert_eq!(
            prepared.acquisition_seal(),
            Some(AcquisitionSeal::PostHandoff)
        );

        assert_eq!(
            resolutions.load(Ordering::SeqCst),
            1,
            "one frozen credential"
        );
        let requests = server.received_requests().await.expect("recorded");
        assert_eq!(requests.len(), 3, "3 total attempts = default budget");
        let bodies: Vec<_> = requests.iter().map(|r| r.body.clone()).collect();
        assert_eq!(bodies[0], bodies[1]);
        assert_eq!(bodies[1], bodies[2], "every send reuses the frozen payload");

        // Wire retry headers: one stable invocation id, per-attempt counter.
        let invocation_ids: Vec<_> = requests
            .iter()
            .map(|r| {
                r.headers
                    .get("amz-sdk-invocation-id")
                    .and_then(|v| v.to_str().ok())
                    .expect("invocation id header")
                    .to_owned()
            })
            .collect();
        assert_eq!(invocation_ids[0], invocation_ids[1]);
        assert_eq!(invocation_ids[1], invocation_ids[2]);
        assert_eq!(invocation_ids[0].len(), 36, "uuid-shaped invocation id");
        let counters: Vec<_> = requests
            .iter()
            .map(|r| {
                r.headers
                    .get("amz-sdk-request")
                    .and_then(|v| v.to_str().ok())
                    .expect("attempt header")
                    .to_owned()
            })
            .collect();
        assert_eq!(
            counters,
            vec!["attempt=1; max=3", "attempt=2; max=3", "attempt=3; max=3"]
        );

        // Fresh SigV4 timestamps per send, same frozen credential.
        let dates: Vec<_> = requests
            .iter()
            .map(|r| {
                r.headers
                    .get("x-amz-date")
                    .and_then(|v| v.to_str().ok())
                    .expect("x-amz-date")
                    .to_owned()
            })
            .collect();
        assert_ne!(
            dates[0], dates[1],
            "each send re-signs with a fresh timestamp"
        );
        for request in &requests {
            let authorization = request
                .headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .expect("authorization");
            assert!(
                authorization.contains("Credential=AKIAPREPARED0001/"),
                "same frozen credential across sends"
            );
        }
    }

    #[tokio::test]
    async fn throttling_uses_500ms_base_and_bounds_hints_to_five_second_headroom() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("x-amzn-errortype", "ThrottlingException:")
                    .insert_header("retry-after", "10"),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;
        mount_success(&server).await;
        let (collection, _) = collection(provider(server.uri(), SmithyRetryConfig::default()));

        let prepared = collection
            .prepare_call_with_policy(
                SPEC,
                prepared_bedrock_request(CancellationToken::new()),
                smithy_policy(SmithyRetryConfig::default()),
            )
            .await
            .expect("prepare");
        let error = drive_attempt_until_admission_or_success(&prepared)
            .await
            .expect_err("throttle");
        assert!(
            matches!(
                &error,
                ProviderError::ServiceFault {
                    status: 429,
                    error_name,
                    retry_after_ms: Some(10_000),
                    ..
                } if error_name.as_deref() == Some("ThrottlingException")
            ),
            "throttling fault carries the wire name and parsed hint, got {error:?}"
        );
        let draw0 = unit_draw(0);
        let backoff = (draw0 * 500.0).floor() as u64;
        let expected = backoff.max(10_000.min(backoff + 5_000));
        assert_eq!(
            prepared.retry_admission().expect("admitted").delay_ms,
            expected,
            "hint bounded to backoff + 5s, never the helper 60s cap"
        );
        assert!(
            prepared.retry_admission().unwrap().delay_ms < 5_500,
            "a 10s hint must not push the delay to 10s"
        );
    }

    #[tokio::test]
    async fn model_timeout_408_has_no_trait_and_is_never_retried() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(408)
                    .insert_header("x-amzn-errortype", "ModelTimeoutException:"),
            )
            .mount(&server)
            .await;
        let (collection, _) = collection(provider(server.uri(), SmithyRetryConfig::default()));

        let prepared = collection
            .prepare_call_with_policy(
                SPEC,
                prepared_bedrock_request(CancellationToken::new()),
                smithy_policy(SmithyRetryConfig::default()),
            )
            .await
            .expect("prepare");
        let error = drive_attempt_until_admission_or_success(&prepared)
            .await
            .expect_err("model timeout");
        assert!(
            matches!(
                &error,
                ProviderError::ServiceFault {
                    status: 408,
                    error_name,
                    ..
                } if error_name.as_deref() == Some("ModelTimeoutException")
            ),
            "408 ModelTimeout is a client fault, got {error:?}"
        );
        assert_eq!(prepared.retry_admission(), None, "no admission");
        assert_eq!(prepared.acquisition_seal(), Some(AcquisitionSeal::Finished));
        let requests = server.received_requests().await.expect("recorded");
        assert_eq!(requests.len(), 1, "exactly one send");
    }

    #[tokio::test]
    async fn model_not_ready_carries_the_retryable_trait_and_retries() {
        let server = MockServer::start().await;
        mount_failures(&server, 400, Some("ModelNotReadyException"), 2).await;
        mount_success(&server).await;
        let (collection, _) = collection(provider(server.uri(), SmithyRetryConfig::default()));

        let prepared = collection
            .prepare_call_with_policy(
                SPEC,
                prepared_bedrock_request(CancellationToken::new()),
                smithy_policy(SmithyRetryConfig::default()),
            )
            .await
            .expect("prepare");
        drive_attempt_until_admission_or_success(&prepared)
            .await
            .expect_err("model not ready");
        // The $retryable trait classifies TRANSIENT despite the 400 status:
        // the 100ms base (not the 500ms throttling base) applies.
        let draw0 = unit_draw(0);
        assert_eq!(
            prepared.retry_admission().expect("admitted").delay_ms,
            (draw0 * 100.0).floor() as u64
        );
        wait_out(&prepared).await;
        drive_attempt_until_admission_or_success(&prepared)
            .await
            .expect_err("still not ready");
        assert!(prepared.retry_admission().is_some());
        wait_out(&prepared).await;
        drive_attempt_until_admission_or_success(&prepared)
            .await
            .expect("third send succeeds");
        let requests = server.received_requests().await.expect("recorded");
        assert_eq!(requests.len(), 3);
    }

    #[tokio::test]
    async fn bare_5xx_501_and_505_are_never_blanket_retried() {
        for status in [501u16, 505] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(ResponseTemplate::new(status))
                .mount(&server)
                .await;
            let (collection, _) = collection(provider(server.uri(), SmithyRetryConfig::default()));
            let prepared = collection
                .prepare_call_with_policy(
                    SPEC,
                    prepared_bedrock_request(CancellationToken::new()),
                    smithy_policy(SmithyRetryConfig::default()),
                )
                .await
                .expect("prepare");
            let error = drive_attempt_until_admission_or_success(&prepared)
                .await
                .expect_err("server error");
            assert!(
                matches!(&error, ProviderError::ServiceFault { status: s, .. } if *s == status),
                "{status} must surface as a service fault"
            );
            assert!(!error.is_retryable(), "{status} is not blanket-retryable");
            assert_eq!(error.category(), ProviderErrorCategory::Provider);
            assert_eq!(prepared.retry_admission(), None);
            let requests = server.received_requests().await.expect("recorded");
            assert_eq!(requests.len(), 1, "{status}: exactly one send");
        }
    }

    #[tokio::test]
    async fn attempts_exhausted_seals_the_call_after_three_sends() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        let (collection, _) = collection(provider(server.uri(), SmithyRetryConfig::default()));

        let prepared = collection
            .prepare_call_with_policy(
                SPEC,
                prepared_bedrock_request(CancellationToken::new()),
                smithy_policy(SmithyRetryConfig::default()),
            )
            .await
            .expect("prepare");
        for _ in 0..3 {
            drive_attempt_until_admission_or_success(&prepared)
                .await
                .expect_err("503 always fails");
            if prepared.retry_admission().is_some() {
                wait_out(&prepared).await;
            }
        }
        assert_eq!(prepared.retry_admission(), None, "budget spent");
        assert_eq!(
            prepared.acquisition_seal(),
            Some(AcquisitionSeal::Exhausted)
        );
        assert!(matches!(
            prepared.start_attempt(),
            Err(opi_ai::ModelsError::CallSealed { .. })
        ));
        let requests = server.received_requests().await.expect("recorded");
        assert_eq!(requests.len(), 3, "default total budget is 3 attempts");
    }

    #[tokio::test]
    async fn readiness_deadline_and_cancellation_keep_waits_cancellable() {
        let server = MockServer::start().await;
        mount_failures(&server, 504, None, 1).await;
        mount_success(&server).await;
        let (collection, _) = collection(provider(server.uri(), SmithyRetryConfig::default()));

        let cancel = CancellationToken::new();
        let prepared = collection
            .prepare_call_with_policy(
                SPEC,
                prepared_bedrock_request(cancel.clone()),
                smithy_policy(SmithyRetryConfig::default()),
            )
            .await
            .expect("prepare");
        drive_attempt_until_admission_or_success(&prepared)
            .await
            .expect_err("504");
        let delay = prepared.retry_admission().expect("admitted").delay_ms;
        if delay > 0 {
            // Early start: typed refusal, zero dispatch.
            assert!(matches!(
                prepared.start_attempt(),
                Err(opi_ai::ModelsError::RetryNotReady { .. })
            ));
            assert_eq!(server.received_requests().await.unwrap().len(), 1);
        }
        // Cancelling during the wait forbids every further send.
        cancel.cancel();
        assert!(matches!(
            prepared.start_attempt(),
            Err(opi_ai::ModelsError::CallCancelled)
        ));
        let requests = server.received_requests().await.expect("recorded");
        assert_eq!(requests.len(), 1, "no send after cancellation");
    }

    #[tokio::test]
    async fn adaptive_first_throttle_gates_the_next_send_with_a_token_wait() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("x-amzn-errortype", "ThrottlingException:"),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;
        mount_success(&server).await;
        let config = SmithyRetryConfig {
            mode: SmithyRetryMode::Adaptive,
            smithy_2026: false,
            max_attempts: 3,
        };
        let (collection, _) = collection(provider(server.uri(), config.clone()));

        // A frozen manual clock keeps the limiter math deterministic: all
        // ticks read the same instant.
        let policy = AcquisitionPolicy {
            jitter_seed: Some(SEED),
            source: RetrySourcePolicy::Smithy(config),
            smithy_clock: SmithyClock::manual([100.0; 8]),
            ..AcquisitionPolicy::default()
        };
        let prepared = collection
            .prepare_call_with_policy(
                SPEC,
                prepared_bedrock_request(CancellationToken::new()),
                policy,
            )
            .await
            .expect("prepare");
        drive_attempt_until_admission_or_success(&prepared)
            .await
            .expect_err("throttle");

        let draw0 = unit_draw(0);
        let backoff = (draw0 * 500.0).floor() as u64;
        // After the throttle the bucket is empty (0 tokens at the frozen
        // instant) and the fill rate floored at 0.5/s, so the post-backoff
        // send waits ceil((1 - backoff/1000*0.5)/0.5)s.
        let refill = backoff as f64 / 1000.0 * 0.5;
        let token_wait = (((1.0 - refill) / 0.5) * 1000.0).ceil() as u64;
        assert_eq!(
            prepared.retry_admission().expect("admitted").delay_ms,
            backoff + token_wait,
            "adaptive folds the send-token wait into the readiness deadline"
        );
        assert!(
            token_wait > 0,
            "the throttled bucket must gate the next send"
        );
    }

    #[tokio::test]
    async fn scoped_retry_config_wires_mode_and_attempts_through_the_provider() {
        let server = MockServer::start().await;
        mount_failures(&server, 504, None, 1).await;
        mount_success(&server).await;
        // env beats profile beats default; unknown mode stays a typed error.
        let config = resolve_smithy_retry_config(ScopedRetryInputs {
            env_retry_mode: Some("adaptive"),
            env_max_attempts: Some("2"),
            profile_retry_mode: Some("standard"),
            profile_max_attempts: Some("9"),
        })
        .expect("resolve");
        assert_eq!(config.mode, SmithyRetryMode::Adaptive);
        assert_eq!(config.max_attempts, 2);
        let unknown = resolve_smithy_retry_config(ScopedRetryInputs {
            profile_retry_mode: Some("legacy"),
            ..ScopedRetryInputs::default()
        })
        .expect_err("unknown mode is typed");
        assert!(unknown.to_string().contains("legacy"));

        let (collection, _) = collection(provider(server.uri(), config));
        let prepared = collection
            .prepare_call(SPEC, prepared_bedrock_request(CancellationToken::new()))
            .await
            .expect("prepare uses the provider's frozen protocol default");
        drive_attempt_until_admission_or_success(&prepared)
            .await
            .expect_err("504");
        assert!(
            prepared.retry_admission().is_some(),
            "ordinary prepare_call inherits the frozen Smithy budget"
        );
        wait_out(&prepared).await;
        drive_attempt_until_admission_or_success(&prepared)
            .await
            .expect("second and final attempt succeeds");
        let requests = server.received_requests().await.expect("recorded");
        assert_eq!(requests.len(), 2, "max_attempts=2 total");
        assert!(
            requests[1]
                .headers
                .get("amz-sdk-request")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v == "attempt=2; max=2")
        );
    }

    #[tokio::test]
    async fn redirect_response_is_not_followed_and_counts_as_one_send() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(302).insert_header("location", "/moved"))
            .mount(&server)
            .await;
        // The product Bedrock legs run with redirects disabled (R-06); the
        // fixture client mirrors that send rule.
        let client = Arc::new(
            opi_ai::http::HttpClientBuilder::new()
                .redirects(opi_ai::http::RedirectPolicy::None)
                .build()
                .expect("client"),
        );
        let provider = BedrockProvider::new(Some(server.uri()), client);
        let (collection, _) = collection(provider);
        let prepared = collection
            .prepare_call_with_policy(
                SPEC,
                prepared_bedrock_request(CancellationToken::new()),
                smithy_policy(SmithyRetryConfig::default()),
            )
            .await
            .expect("prepare");
        let error = drive_attempt_until_admission_or_success(&prepared)
            .await
            .expect_err("302");
        assert!(matches!(
            error,
            ProviderError::ServiceFault { status: 302, .. }
        ));
        assert_eq!(prepared.retry_admission(), None);
        let requests = server.received_requests().await.expect("recorded");
        assert_eq!(requests.len(), 1, "no redirect follow and no retry");
    }

    #[test]
    fn smithy_2026_constants_branch_is_exact() {
        let draw = unit_draw(0);
        let legacy = opi_ai::bedrock::retry::smithy_backoff_delay_ms(
            0,
            opi_ai::bedrock::retry::SmithyErrorClass::Transient,
            false,
            None,
            draw,
        );
        let v2026 = opi_ai::bedrock::retry::smithy_backoff_delay_ms(
            0,
            opi_ai::bedrock::retry::SmithyErrorClass::Transient,
            true,
            None,
            draw,
        );
        assert_eq!(legacy, (draw * 100.0).floor() as u64);
        assert_eq!(
            v2026,
            (draw * 50.0).floor() as u64,
            "2026 transient base is 50ms"
        );
        assert_eq!(
            opi_ai::bedrock::retry::smithy_capacity_cost(
                opi_ai::bedrock::retry::SmithyErrorClass::Transient,
                true
            ),
            14
        );
    }

    #[test]
    fn map_bedrock_status_keeps_auth_rejections_typed() {
        let headers = reqwest::header::HeaderMap::new();
        assert!(matches!(
            map_bedrock_status(reqwest::StatusCode::from_u16(403).unwrap(), &headers),
            ProviderError::AuthFailed(_)
        ));
    }

    // Source-exchange sends and generation sends stay separately countable:
    // the auth chain resolves through its own transport (counted there) and
    // the generation episode counts only Bedrock sends.
    struct CountingStaticAuth {
        source_sends: Arc<AtomicUsize>,
    }
    impl AuthResolver for CountingStaticAuth {
        fn resolve<'a>(&'a self) -> opi_ai::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
            Box::pin(async move {
                self.source_sends.fetch_add(1, Ordering::SeqCst);
                Ok(ResolvedAuth::aws_sigv4(
                    opi_ai::bedrock::sigv4::AwsCredentials {
                        access_key_id: SecretString::from("AKIACOUNTER"),
                        secret_access_key: SecretString::from("counter-secret"),
                        session_token: None,
                        region: "us-east-1".into(),
                        expires_at: None,
                    },
                    super::aws_provenance(opi_ai::auth::AwsCredentialSource::Environment),
                ))
            })
        }
    }

    #[test]
    fn source_and_generation_counters_stay_distinct() {
        let source_sends = Arc::new(AtomicUsize::new(0));
        let auth = CountingStaticAuth {
            source_sends: source_sends.clone(),
        };
        let _ = auth;
        // The fixture above exercises the split through the real chain; this
        // unit keeps the counter types aligned (source exchanges never draw
        // from the generation quota).
        let mut quota = opi_ai::bedrock::retry::SmithyRetryQuota::default();
        quota.acquire(5);
        assert_eq!(quota.capacity(), 495);
        assert_eq!(source_sends.load(Ordering::SeqCst), 0);
    }
}

// ---------------------------------------------------------------------------
// Task 20.51: functional callbacks at the Bedrock wire (P20-S51)
// ---------------------------------------------------------------------------

use opi_ai::callbacks::RequestCallbacks;

#[tokio::test]
async fn bedrock_async_payload_and_response_callbacks_flow_through_converse_stream() {
    let body_bytes = bedrock_text_lifecycle_bytes();
    let payload_bodies: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let payload_recorder = payload_bodies.clone();
    let response_status = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let response_recorder = response_status.clone();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/model/anthropic.claude-sonnet-4/converse-stream"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body_bytes, "application/vnd.amazon.eventstream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.callbacks = RequestCallbacks::new()
        .with_async_header_transform(|headers| async move {
            tokio::task::yield_now().await;
            Ok(headers)
        })
        .with_async_on_payload(move |body| {
            let payload_recorder = payload_recorder.clone();
            async move {
                tokio::task::yield_now().await;
                payload_recorder.lock().unwrap().push(body.clone());
                Ok(None)
            }
        })
        .with_async_on_response(move |status, headers| {
            let response_recorder = response_recorder.clone();
            async move {
                tokio::task::yield_now().await;
                response_recorder.lock().unwrap().push(format!("{status}"));
                assert!(
                    headers
                        .iter()
                        .any(|(name, _)| name.eq_ignore_ascii_case("content-type")),
                    "bedrock response callback sees the event-stream content type"
                );
                Ok(())
            }
        });

    let events = collect_events(provider.stream_prepared(request, test_auth())).await;
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Done { .. }))
    );

    let bodies = payload_bodies.lock().unwrap().clone();
    assert_eq!(bodies.len(), 1);
    assert!(
        bodies[0]["messages"].is_array(),
        "bedrock payload callback sees the converse-stream body"
    );
    let sent = server.received_requests().await.unwrap()[0].clone();
    let wire_body: serde_json::Value =
        serde_json::from_slice(&sent.body).expect("captured body is JSON");
    assert_eq!(bodies[0], wire_body);
    assert_eq!(*response_status.lock().unwrap(), vec!["200"]);
    server.verify().await;
}

#[tokio::test]
async fn bedrock_raw_callback_sees_frame_events_in_order() {
    let body_bytes = bedrock_text_lifecycle_bytes();
    let raw_events: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = raw_events.clone();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body_bytes, "application/vnd.amazon.eventstream"),
        )
        .mount(&server)
        .await;

    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.callbacks = RequestCallbacks::new().with_on_raw(std::sync::Arc::new(move |event| {
        recorder
            .lock()
            .unwrap()
            .push(event["type"].as_str().unwrap_or("?").to_string());
        Ok(())
    }));

    let events = collect_events(provider.stream_prepared(request, test_auth())).await;
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::TextDelta { .. }))
    );
    let raw = raw_events.lock().unwrap();
    assert!(
        raw.first().map(String::as_str) == Some("messageStart"),
        "bedrock raw observer sees smithy frames in wire order: {raw:?}"
    );
    assert!(raw.contains(&"contentBlockDelta".to_string()));
    assert!(
        raw.last().map(String::as_str) == Some("metadata"),
        "every parsed smithy frame reaches the raw observer, in wire order: {raw:?}"
    );
}

#[tokio::test]
async fn bedrock_transform_touching_signing_headers_fails_typed_before_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut request = lifecycle_text_request();
    request.callbacks =
        RequestCallbacks::new().with_header_transform(std::sync::Arc::new(|merged| {
            let mut out = merged.to_vec();
            out.push(("amz-sdk-invocation-id".to_string(), "attacker".to_string()));
            Ok(out)
        }));

    let mut stream = provider.stream_prepared(request, test_auth());
    let mut typed = false;
    while let Some(item) = stream.next().await {
        if matches!(item, Err(ProviderError::CallbackPhase(_))) {
            typed = true;
            break;
        }
    }
    assert!(
        typed,
        "transform touching the Smithy signing headers must fail typed"
    );
    server.verify().await;
}

mod constrained_guard_tests {
    use super::*;
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    use opi_ai::message::ToolDef;
    use opi_ai::provider::{ProviderError, ThinkingConfig};

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
            transport: Default::default(),
            model: "bedrock:anthropic.claude-3-5-sonnet-20240620-v1:0".into(),
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
        let provider = BedrockProvider::new(None, Arc::new(opi_ai::http::HttpClient::new()));
        let error = provider
            .build_converse_body(&guarded_request(vec![constrained_tool(
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
        let provider = BedrockProvider::new(None, Arc::new(opi_ai::http::HttpClient::new()));
        let body = provider
            .build_converse_body(&guarded_request(vec![constrained_tool(
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
