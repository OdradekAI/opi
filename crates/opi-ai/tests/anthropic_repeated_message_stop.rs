use futures_util::StreamExt;
use opi_ai::providers::anthropic::AnthropicProvider;
use opi_ai::stream::AssistantStreamEvent;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

fn fixture_body() -> (String, Value) {
    let fixture: Value =
        serde_json::from_str(include_str!("data/anthropic-repeated-message-stop.json"))
            .expect("valid repeated message_stop fixture");
    let body = fixture["events"]
        .as_array()
        .expect("fixture events")
        .iter()
        .map(|event| {
            format!(
                "event: {}\ndata: {}\n\n",
                event["type"].as_str().expect("event type"),
                event
            )
        })
        .collect();
    (body, fixture)
}

fn event_type(event: &AssistantStreamEvent) -> &'static str {
    match event {
        AssistantStreamEvent::Start { .. } => "start",
        AssistantStreamEvent::TextStart { .. } => "text_start",
        AssistantStreamEvent::TextDelta { .. } => "text_delta",
        AssistantStreamEvent::TextEnd { .. } => "text_end",
        AssistantStreamEvent::ThinkingStart { .. } => "thinking_start",
        AssistantStreamEvent::ThinkingDelta { .. } => "thinking_delta",
        AssistantStreamEvent::ThinkingEnd { .. } => "thinking_end",
        AssistantStreamEvent::ToolCallStart { .. } => "toolcall_start",
        AssistantStreamEvent::ToolCallDelta { .. } => "toolcall_delta",
        AssistantStreamEvent::ToolCallEnd { .. } => "toolcall_end",
        AssistantStreamEvent::Done { .. } => "done",
        _ => "unknown",
    }
}

#[tokio::test]
async fn repeated_message_stop_usage_is_deferred_until_eof() {
    let (body, fixture) = fixture_body();
    let provider = AnthropicProvider::new(None);
    let mut cases = Vec::new();
    for route in ["simple", "direct"] {
        let events: Vec<_> = provider
            .stream_from_sse(&body, CancellationToken::new())
            .collect()
            .await;
        let done = events
            .iter()
            .find_map(|event| match event {
                Ok(AssistantStreamEvent::Done { message, .. }) => Some(message),
                _ => None,
            })
            .expect("one deferred Done");
        assert_eq!(done.stop_reason, opi_ai::stream::StopReason::Stop);
        assert_eq!(done.raw_stop_reason.as_deref(), Some("end_turn"));
        assert_eq!(done.usage.input_tokens, fixture["expected"]["input"]);
        assert_eq!(done.usage.output_tokens, fixture["expected"]["output"]);
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Ok(AssistantStreamEvent::Done { .. })))
                .count(),
            1
        );
        cases.push(serde_json::json!({
            "route": route,
            "stop": done.stop_reason,
            "raw_stop": done.raw_stop_reason,
            "usage": {"input": done.usage.input_tokens, "output": done.usage.output_tokens},
            "done_count": events.iter().filter(|event| matches!(event, Ok(AssistantStreamEvent::Done { .. }))).count(),
            "event_types": events.iter().filter_map(|event| event.as_ref().ok().map(event_type)).collect::<Vec<_>>(),
            "requests": 1,
        }));
    }
    let trace = serde_json::json!({"case": fixture["id"], "cases": cases});
    println!("{trace}");
}

#[tokio::test]
async fn usage_error_after_message_stop_does_not_flush_a_deferred_done() {
    let body = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg-1\",\"model\":\"claude-test\",\"usage\":{\"input_tokens\":10,\"output_tokens\":0}}}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":1}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n",
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg-2\",\"model\":\"claude-test\",\"usage\":{\"input_tokens\":10,\"output_tokens\":0,\"cache_creation_input_tokens\":1,\"cache_creation_input_tokens_1h\":2}}}\n\n",
    );
    let events: Vec<_> = AnthropicProvider::new(None)
        .stream_from_sse(body, CancellationToken::new())
        .collect()
        .await;
    assert!(events.iter().any(Result::is_err));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Ok(AssistantStreamEvent::Done { .. })))
    );
}
