//! Shared Bedrock protocol and lifecycle fixtures.
//!
//! This module owns helpers consumed by the root Bedrock target and its
//! wire, lifecycle, builtin-route and Converse adapter topics.

use super::*;

pub(super) fn test_credentials() -> AwsCredentials {
    AwsCredentials {
        access_key_id: "AKIAIOSFODNN7EXAMPLE".into(),
        secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into(),
        session_token: None,
        region: "us-east-1".into(),
        expires_at: None,
    }
}

pub(super) fn test_auth() -> ResolvedAuth {
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

pub(super) fn text_stream_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "us.anthropic.claude-sonnet-4-20250514-v1:0".into(),
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

pub(super) fn tool_call_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "us.anthropic.claude-sonnet-4-20250514-v1:0".into(),
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

pub(super) fn build_bedrock_stream(events: &[(&str, &str)]) -> Vec<u8> {
    let mut buffer = Vec::new();
    for (event_type, json_payload) in events {
        let frame =
            event_stream::build_test_frame(event_type, "application/json", json_payload.as_bytes());
        buffer.extend_from_slice(&frame);
    }
    buffer
}

pub(super) async fn collect_events(
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

pub(super) fn direct_error_cause(error: &ProviderError) -> &ProviderError {
    match error {
        ProviderError::StreamFailure(failure) => failure.cause(),
        other => other,
    }
}

pub(super) fn assert_direct_projection_prefix(message: &AssistantMessage, request: &Request) {
    use sha2::Digest as _;
    let facts = message
        .projection
        .as_ref()
        .expect("accepted projection is known");
    let bytes = serde_json::to_vec(request.context.messages()).unwrap();
    assert_eq!(
        facts.source_prefix.message_count,
        request.context.messages().len()
    );
    assert_eq!(
        facts.source_prefix.sha256,
        format!("{:x}", sha2::Sha256::digest(bytes))
    );
    assert_eq!(
        facts.temporal.mode,
        opi_ai::transcript::ProjectionMode::Lossless
    );
    assert!(facts.temporal.affected_ranges.is_empty());
    assert!(facts.replay.losses.is_empty());
}

pub(super) async fn collect_outcome(
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

pub(super) fn assistant_call(id: &str, name: &str) -> Message {
    Message::Assistant(AssistantMessage {
        projection: None,
        content: vec![AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: id.into(),
                name: name.into(),
                arguments: "{}".into(),
                arguments_error: None,
                signature: None,
            },
        }],
        api: opi_ai::ApiKind::Anthropic,
        deferred: None,
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
        raw_stop_reason: None,
        provider_thinking_level: None,
    })
}

pub(super) fn lifecycle_text_request() -> Request {
    // Use a `bedrock:`-prefixed model id without a colon-bearing version
    // suffix so `stream()`'s `split_once(':')` model extraction resolves to
    // `anthropic.claude-sonnet-4` (family `anthropic`) and the request path
    // stays free of URL-encoded version colons.
    Request {
        projection: Default::default(),
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

pub(super) fn bedrock_text_lifecycle_bytes() -> Vec<u8> {
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
