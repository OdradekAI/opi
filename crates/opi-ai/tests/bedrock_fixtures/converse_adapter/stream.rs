//! Bedrock Converse stream, stop-reason and usage cases.

use super::*;

async fn collect_last_42(events: &[(&str, &str)]) -> Result<AssistantStreamEvent, ProviderError> {
    let data = build_bedrock_stream(events);
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    let request = text_stream_request();
    let mut stream = provider.stream_from_fixture(&data, request.cancel);
    let mut last = None;
    while let Some(item) = stream.next().await {
        last = Some(item);
        if matches!(last, Some(Ok(AssistantStreamEvent::Done { .. })))
            || matches!(last, Some(Err(_)))
        {
            break;
        }
    }
    last.expect("terminal event")
}

#[tokio::test]
async fn model_context_window_exceeded_maps_to_length() {
    let last = collect_last_42(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"hi"},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        (
            "messageStop",
            r#"{"stopReason":"model_context_window_exceeded"}"#,
        ),
    ])
    .await
    .expect("length stop completes");
    match last {
        AssistantStreamEvent::Done { message, .. } => {
            assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Length);
        }
        other => panic!("unexpected terminal event: {other:?}"),
    }
}

#[tokio::test]
async fn unmapped_stop_reason_fails_typed_with_raw_reason() {
    let last = collect_last_42(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"hi"},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        ("messageStop", r#"{"stopReason":"content_filtered"}"#),
    ])
    .await
    .expect_err("unmapped stop reason fails typed");
    let detail = format!("{error}", error = last);
    assert!(
        detail.contains("content_filtered"),
        "raw reason must ride the typed failure: {detail}"
    );
    assert!(detail.contains("Provider stopped with"), "{detail}");
}

#[tokio::test]
async fn reasoning_content_deltas_stream_as_thinking_blocks() {
    let events_data = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"reasoningContent":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"reasoningContent":{"text":"step one"}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"reasoningContent":{"text":" step two"}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"reasoningContent":{"signature":"c2ln"}},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":1}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"answer"},"contentBlockIndex":1}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":1}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":3,"outputTokens":4}}"#,
        ),
    ]);
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    let request = text_stream_request();
    let events = collect_events(provider.stream_from_fixture(&events_data, request.cancel)).await;

    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::ThinkingStart { .. }))
    );
    assert!(events.iter().any(
        |e| matches!(e, AssistantStreamEvent::ThinkingDelta { delta, .. } if delta == "step one")
    ));
    let done = events.last().expect("terminal event");
    let AssistantStreamEvent::Done { message, .. } = done else {
        panic!("stream must end with Done: {done:?}")
    };
    assert!(matches!(
        message.content.first(),
        Some(AssistantContent::Thinking { thinking, signature })
            if thinking == "step one step two" && signature.as_deref() == Some("c2ln")
    ));
}

#[tokio::test]
async fn redacted_content_accumulates_into_redacted_thinking() {
    // Two one-byte chunks ("QQ==" -> 0x41, "Qg==" -> 0x42) concatenate
    // to the two-byte payload "AB" whose base64 is "QUI=".
    let events_data = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"reasoningContent":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"reasoningContent":{"redactedContent":"QQ=="}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"reasoningContent":{"redactedContent":"Qg=="}},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":1,"outputTokens":1}}"#,
        ),
    ]);
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    let request = text_stream_request();
    let events = collect_events(provider.stream_from_fixture(&events_data, request.cancel)).await;
    let done = events.last().expect("terminal event");
    let AssistantStreamEvent::Done { message, .. } = done else {
        panic!("stream must end with Done: {done:?}")
    };
    assert!(matches!(
        message.content.first(),
        Some(AssistantContent::RedactedThinking { data }) if data == "QUI="
    ));
}

#[tokio::test]
async fn usage_cache_write_1h_sums_one_hour_details() {
    let events_data = build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"hi"},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":10,"outputTokens":5,"cacheWriteInputTokens":9,"cacheDetails":[{"ttl":"1h","inputTokens":6},{"ttl":"5m","inputTokens":2},{"ttl":"1h","inputTokens":1}]}}"#,
        ),
    ]);
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    let request = text_stream_request();
    let events = collect_events(provider.stream_from_fixture(&events_data, request.cancel)).await;
    let done = events.last().expect("terminal event");
    let AssistantStreamEvent::Done { message, .. } = done else {
        panic!("stream must end with Done: {done:?}")
    };
    assert_eq!(message.usage.cache_write_tokens, 9);
    assert_eq!(message.usage.cache_write_1h_tokens, Some(7));
}

#[test]
fn strict_tool_flag_projects_converted_schema() {
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    let mut model = model_42(SONNET4, "Claude (Bedrock)");
    model.compat = opi_ai::model_info::WireCompat::BedrockConverseStream(
        opi_ai::model_info::BedrockConverseStreamCompat {
            supports_strict_tools: true,
        },
    );
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new())).with_models(vec![model]);
    let mut request = tool_call_request();
    request.model = SONNET4.into();
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text { text: "go".into() }],
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
            constrained_sampling: Some(ConstrainedSampling::JsonSchema {
                strict: StrictPreference::Require,
            }),
        }],
    })
    .expect("valid context declarations");
    let body = body_42(&provider, &request);
    let spec = &body["toolConfig"]["tools"][0]["toolSpec"];
    assert_eq!(spec["strict"], serde_json::json!(true));
    // The strict projection makes every property required.
    assert_eq!(
        spec["inputSchema"]["json"]["properties"]["path"],
        serde_json::json!({"type": "string"})
    );
    assert_eq!(
        spec["inputSchema"]["json"]["required"],
        serde_json::json!(["path"])
    );
}
