//! Bedrock wire and projection contract tests.

use super::*;

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

#[test]
fn invalid_prelude_is_rejected_before_payload_wait() {
    fn prelude(total_len: u32, headers_len: u32) -> Vec<u8> {
        let mut bytes = Vec::from(total_len.to_be_bytes());
        bytes.extend_from_slice(&headers_len.to_be_bytes());
        let mut crc = !0u32;
        for &byte in &bytes {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xEDB8_8320 * (crc & 1));
            }
        }
        bytes.extend_from_slice(&(!crc).to_be_bytes());
        bytes
    }

    let mut bad_crc = prelude(u32::MAX, 0);
    bad_crc[8] ^= 1;
    for prefix in [bad_crc, prelude(15, 0), prelude(u32::MAX, u32::MAX)] {
        for buffered in 12..=16 {
            let mut bytes = prefix.clone();
            bytes.resize(buffered, 0);
            assert_eq!(
                event_stream::parse_frames(&mut bytes),
                Err(event_stream::FrameParseError::MalformedCompleteFrame),
                "invalid prelude must fail with only {buffered} bytes buffered"
            );
        }
    }
}

#[test]
fn valid_prelude_remains_buffered_until_frame_complete() {
    let frame = event_stream::build_test_frame("messageStart", "application/json", b"{}");
    let expected = event_stream::parse_frames(&mut frame.clone()).unwrap();
    for split in 0..frame.len() {
        let mut buffer = frame[..split].to_vec();
        assert!(event_stream::parse_frames(&mut buffer).unwrap().is_empty());
        assert_eq!(buffer, frame[..split]);
        buffer.extend_from_slice(&frame[split..]);
        assert_eq!(event_stream::parse_frames(&mut buffer).unwrap(), expected);
        assert!(buffer.is_empty());
    }
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
    let result = provider.validate_model_id("anthropic.fixture-converse-model");
    assert!(result.is_ok(), "supported family should validate");
}

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
    let redacted =
        opi_ai::providers::bedrock::redact_credentials("AKIAIOSFODNN7EXAMPLE", "super-secret-key");
    assert!(!redacted.contains("AKIA"));
    assert!(!redacted.contains("super-secret-key"));
    assert!(redacted.contains("***"));
}

#[test]
fn bedrock_provider_accepts_shared_client() {
    let client = Arc::new(HttpClient::new());
    let provider = BedrockProvider::new(None, client.clone());
    assert!(Arc::ptr_eq(&client, provider.http_client()));
}

#[tokio::test]
async fn url_image_rejected_with_clear_error() {
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "bedrock:us.anthropic.claude-sonnet-4-20250514-v1:0".into(),
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
    match events[0].as_ref().map_err(direct_error_cause) {
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
    // Task 20.42: success tool-result images ride real inline image blocks
    // (pi `convertToolResultContent`); the phase-12 text placeholder is
    // superseded by the replacement proof in the 20.42 adapter tests.
    let image = &body["messages"][1]["content"][0]["toolResult"]["content"][0]["image"];
    assert_eq!(image["format"], serde_json::json!("jpeg"));
    assert_eq!(
        image["source"]["bytes"],
        serde_json::json!("/9j/") // base64 of [0xff, 0xd8, 0xff]
    );
}

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

#[test]
fn bedrock_models_advertise_supports_thinking() {
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
