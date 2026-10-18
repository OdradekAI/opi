//! Behavioral tests for task 1.1: message and stream types.
//!
//! DoD: "serialize where needed; terminal stream events tested"

use opi_ai::provider::{ProviderError, ProviderErrorSummary};
use opi_ai::stream::{AssistantStreamEvent, CallFailure, ChatStream, StopReason, Usage};
use opi_ai::{
    ApiKind,
    message::{
        AssistantContent, AssistantMessage, InputContent, Message, OutputContent, ToolCall,
        ToolDef, ToolResultMessage, UserMessage,
    },
};

fn sample_assistant_message() -> AssistantMessage {
    AssistantMessage {
        content: vec![AssistantContent::Text {
            text: "hello".into(),
        }],
        api: ApiKind::Anthropic,
        provider: "anthropic".into(),
        model: "claude-sonnet-4-5-20250514".into(),
        response_model: None,
        wire: None,
        response_id: None,
        usage: Usage::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms: 1000,
    }
}

// --- Terminal stream event tests ---

#[test]
fn done_event_is_terminal() {
    let msg = sample_assistant_message();
    let event = AssistantStreamEvent::Done {
        reason: StopReason::Stop,
        message: msg,
    };
    assert!(event.is_terminal());
}

#[test]
fn error_event_is_terminal() {
    let msg = sample_assistant_message();
    let event = AssistantStreamEvent::Error {
        reason: StopReason::Error,
        message: msg,
    };
    assert!(event.is_terminal());
}

#[test]
fn start_event_is_not_terminal() {
    let msg = sample_assistant_message();
    let event = AssistantStreamEvent::Start { partial: msg };
    assert!(!event.is_terminal());
}

#[test]
fn text_delta_is_not_terminal() {
    let msg = sample_assistant_message();
    let event = AssistantStreamEvent::TextDelta {
        content_index: 0,
        delta: "hi".into(),
        partial: msg,
    };
    assert!(!event.is_terminal());
}

// --- Serialization round-trip tests ---

#[test]
fn user_message_round_trips_through_json() {
    let msg = Message::User(UserMessage {
        content: vec![InputContent::Text {
            text: "Hello, world".into(),
        }],
        timestamp_ms: 42,
    });
    let json = serde_json::to_string(&msg).expect("serialize");
    let back: Message = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(msg, back);
}

#[test]
fn assistant_message_round_trips_through_json() {
    let msg = Message::Assistant(sample_assistant_message());
    let json = serde_json::to_string(&msg).expect("serialize");
    let back: Message = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(msg, back);
}

#[test]
fn tool_result_message_round_trips_through_json() {
    let msg = Message::ToolResult(ToolResultMessage {
        tool_call_id: "call_123".into(),
        tool_name: "read_file".into(),
        content: vec![OutputContent::Text {
            text: "file contents".into(),
        }],
        details: None,
        is_error: false,
        truncated: false,
        timestamp_ms: 99,
    });
    let json = serde_json::to_string(&msg).expect("serialize");
    let back: Message = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(msg, back);
}

#[test]
fn tool_result_error_message_round_trips() {
    let msg = Message::ToolResult(ToolResultMessage {
        tool_call_id: "call_err".into(),
        tool_name: "bash".into(),
        content: vec![OutputContent::Text {
            text: "command failed".into(),
        }],
        details: Some(serde_json::json!({"exit_code": 1})),
        is_error: true,
        truncated: false,
        timestamp_ms: 100,
    });
    let json = serde_json::to_string(&msg).expect("serialize");
    let back: Message = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(msg, back);
}

#[test]
fn done_event_serializes() {
    let msg = sample_assistant_message();
    let event = AssistantStreamEvent::Done {
        reason: StopReason::Stop,
        message: msg,
    };
    let json = serde_json::to_string(&event).expect("serialize");
    assert!(json.contains("\"done\""));
}

#[test]
fn error_event_serializes() {
    let msg = sample_assistant_message();
    let event = AssistantStreamEvent::Error {
        reason: StopReason::Error,
        message: msg,
    };
    let json = serde_json::to_string(&event).expect("serialize");
    assert!(json.contains("\"error\""));
}

#[test]
fn legacy_error_event_still_deserializes() {
    // Versioned read-only compatibility: historical saved streams keep
    // deserializing their in-band error terminals, and the payload survives
    // round-trip intact for the ChatStream conversion path.
    let mut message = sample_assistant_message();
    message.error_message = Some("historical failure".into());
    let event = AssistantStreamEvent::Error {
        reason: StopReason::Error,
        message,
    };
    let json = serde_json::to_string(&event).expect("serialize");
    let back: AssistantStreamEvent = serde_json::from_str(&json).expect("legacy read");
    assert_eq!(back, event);
}

// --- Stop reason tests ---

#[test]
fn stop_reasons_match_pi() {
    assert_eq!(StopReason::Stop.as_str(), "stop");
    assert_eq!(StopReason::Length.as_str(), "length");
    assert_eq!(StopReason::ToolUse.as_str(), "tool_use");
    assert_eq!(StopReason::Error.as_str(), "error");
    assert_eq!(StopReason::Aborted.as_str(), "aborted");
}

// --- ToolDef and ToolCall tests ---

#[test]
fn tool_def_round_trips_through_json() {
    let def = ToolDef {
        name: "read_file".into(),
        description: "Read a file".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"}
            },
            "required": ["path"]
        }),
    };
    let json = serde_json::to_string(&def).expect("serialize");
    let back: ToolDef = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(def, back);
}

#[test]
fn tool_call_end_event_carries_arguments_error_through_json() {
    // AIR-048: the optional final-parse error survives event serialization
    // and stays absent for clean calls; historical events keep deserializing.
    let tool_call = ToolCall {
        id: "t1".into(),
        name: "read".into(),
        arguments: r#"{"path": "trunc"#.into(),
        arguments_error: Some("final tool arguments were not a complete JSON object".into()),
    };
    let event = AssistantStreamEvent::ToolCallEnd {
        content_index: 0,
        tool_call,
        partial: sample_assistant_message(),
    };
    let value = serde_json::to_value(&event).unwrap();
    assert_eq!(
        value["tool_call"]["arguments_error"],
        "final tool arguments were not a complete JSON object"
    );
    let back: AssistantStreamEvent = serde_json::from_value(value).unwrap();
    match back {
        AssistantStreamEvent::ToolCallEnd { tool_call, .. } => {
            assert_eq!(
                tool_call.arguments_error.as_deref(),
                Some("final tool arguments were not a complete JSON object")
            );
            assert_eq!(tool_call.arguments, r#"{"path": "trunc"#);
        }
        other => panic!("expected ToolCallEnd, got {other:?}"),
    }
}

#[test]
fn legacy_tool_call_end_event_without_arguments_error_deserializes() {
    let legacy = serde_json::json!({
        "type": "tool_call_end",
        "content_index": 0,
        "tool_call": {"id": "t2", "name": "read", "arguments": "{}"},
        "partial": sample_assistant_message(),
    });
    let event: AssistantStreamEvent = serde_json::from_value(legacy).unwrap();
    match event {
        AssistantStreamEvent::ToolCallEnd { tool_call, .. } => {
            assert_eq!(tool_call.arguments_error, None);
        }
        other => panic!("expected ToolCallEnd, got {other:?}"),
    }
}

#[test]
fn tool_call_in_stream_event() {
    let msg = sample_assistant_message();
    let tc = ToolCall {
        id: "call_abc".into(),
        name: "read_file".into(),
        arguments: "{\"path\":\"/tmp/x\"}".into(),
        arguments_error: None,
    };
    let event = AssistantStreamEvent::ToolCallEnd {
        content_index: 0,
        tool_call: tc,
        partial: msg,
    };
    assert!(!event.is_terminal());
    let json = serde_json::to_string(&event).expect("serialize");
    assert!(json.contains("call_abc"));
}

// --- Usage tests ---

#[test]
fn usage_default_is_zero() {
    let u = Usage::default();
    assert_eq!(u.input_tokens, 0);
    assert_eq!(u.output_tokens, 0);
}

// ---------------------------------------------------------------------------
// Task 20.2: ChatStream / CallFailure — one final Result for one logical call.
//
// P20-FINAL closure tests (AIR-003/AIR-047, V-03/V-40): iteration is
// `Stream<Item = Result<AssistantStreamEvent, CallFailure>>`; the first
// terminal is absorbed into permanent EOF; `result(self)` actively drives the
// remaining bounded stream and reuses the delivered terminal cache; a missing
// terminal yields exactly one typed protocol failure.
// ---------------------------------------------------------------------------

use futures_util::StreamExt;

fn assistant_text_of(message: &AssistantMessage) -> String {
    message
        .content
        .iter()
        .filter_map(|content| match content {
            AssistantContent::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// Build `count` text deltas over progressively longer partial snapshots,
/// plus an optional terminal. Returns the event items and the expected final
/// text.
fn text_events(
    count: usize,
    terminal: Option<AssistantStreamEvent>,
) -> (Vec<Result<AssistantStreamEvent, ProviderError>>, String) {
    let mut events: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();
    let mut text = String::new();
    events.push(Ok(AssistantStreamEvent::Start {
        partial: sample_assistant_message(),
    }));
    for _ in 0..count {
        text.push('x');
        let mut partial = sample_assistant_message();
        partial.content = vec![AssistantContent::Text { text: text.clone() }];
        events.push(Ok(AssistantStreamEvent::TextDelta {
            content_index: 0,
            delta: "x".into(),
            partial,
        }));
    }
    if let Some(terminal) = terminal {
        events.push(Ok(terminal));
    }
    (events, text)
}

fn done_terminal(text: &str, reason: StopReason) -> AssistantStreamEvent {
    let mut message = sample_assistant_message();
    message.content = vec![AssistantContent::Text { text: text.into() }];
    message.stop_reason = reason;
    AssistantStreamEvent::Done { reason, message }
}

fn chat_stream(events: Vec<Result<AssistantStreamEvent, ProviderError>>) -> ChatStream {
    ChatStream::new(Box::pin(futures_util::stream::iter(events)))
}

#[tokio::test]
async fn chat_stream_result_drains_over_64_events_to_one_ok_outcome() {
    let (events, text) = text_events(100, None);
    let mut events = events;
    events.push(Ok(done_terminal(&text, StopReason::Stop)));
    // Tail after the terminal must never be delivered.
    events.push(Ok(AssistantStreamEvent::TextDelta {
        content_index: 0,
        delta: "tail".into(),
        partial: sample_assistant_message(),
    }));

    let result = chat_stream(events).result().await;
    let message = result.expect(">64-event stream must collect to Ok");
    assert_eq!(assistant_text_of(&message), text);
    assert_eq!(message.stop_reason, StopReason::Stop);
}

#[tokio::test]
async fn chat_stream_iteration_and_result_share_one_outcome() {
    // V-40 three-way equivalence: result without iteration, full iteration
    // then result on the same handle, and partial iteration then result all
    // yield the same outcome.
    let (events, text) = text_events(8, None);
    let mut events = events;
    events.push(Ok(done_terminal(&text, StopReason::ToolUse)));

    // (a) result directly.
    let direct = chat_stream(events.clone()).result().await;

    // (b) full iteration first, then result on the same handle.
    let mut iterated = chat_stream(events.clone());
    let mut seen_terminals = 0;
    while let Some(item) = iterated.next().await {
        match item {
            Ok(event) if event.is_terminal() => seen_terminals += 1,
            Ok(_) => {}
            Err(failure) => panic!("unexpected failure: {failure:?}"),
        }
    }
    assert_eq!(seen_terminals, 1, "exactly one terminal event is delivered");
    let after_iteration = iterated.result().await;

    // (c) partial iteration, then result.
    let mut partial = chat_stream(events);
    for _ in 0..3 {
        let _ = partial.next().await.expect("partial iteration has events");
    }
    let after_partial = partial.result().await;

    let direct = direct.expect("direct result is Ok");
    let after_iteration =
        after_iteration.expect("result after iteration reuses the terminal cache");
    let after_partial = after_partial.expect("result after partial iteration is Ok");
    assert_eq!(direct, after_iteration);
    assert_eq!(direct, after_partial);
    assert_eq!(direct.stop_reason, StopReason::ToolUse);
}

#[tokio::test]
async fn chat_stream_absorbs_first_terminal_into_permanent_eof() {
    let (events, text) = text_events(2, None);
    let mut events = events;
    events.push(Ok(done_terminal(&text, StopReason::Stop)));
    // A second terminal and tail events after the first terminal.
    events.push(Ok(done_terminal(&text, StopReason::Stop)));
    events.push(Ok(AssistantStreamEvent::TextDelta {
        content_index: 0,
        delta: "tail".into(),
        partial: sample_assistant_message(),
    }));

    let mut stream = chat_stream(events);
    let mut delivered_terminals = 0;
    while let Some(item) = stream.next().await {
        match item {
            Ok(event) if event.is_terminal() => delivered_terminals += 1,
            Ok(_) => {}
            Err(failure) => panic!("unexpected failure: {failure:?}"),
        }
    }
    assert_eq!(
        delivered_terminals, 1,
        "only the first terminal is delivered"
    );
    // Permanent EOF: further polls stay exhausted.
    assert!(stream.next().await.is_none());
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn chat_stream_yields_one_protocol_error_on_premature_eof() {
    // No terminal: partial content, then inner EOF.
    let (events, text) = text_events(3, None);
    let mut stream = chat_stream(events);
    let mut failures = 0;
    let mut last: Option<CallFailure> = None;
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => {}
            Err(failure) => {
                failures += 1;
                last = Some(failure);
            }
        }
    }
    assert_eq!(failures, 1, "premature EOF yields exactly one failure");
    let failure = last.expect("failure observed");
    assert!(
        matches!(failure.cause(), ProviderError::StreamError(summary) if summary.contains("terminal")),
        "typed protocol failure, got {:?}",
        failure.cause()
    );
    // Failure partial is preserved from the last event-time snapshot.
    assert_eq!(assistant_text_of(failure.message()), text);
    assert_eq!(failure.message().stop_reason, StopReason::Error);
    assert!(
        stream.next().await.is_none(),
        "EOF is permanent after the failure"
    );
}

#[tokio::test]
async fn chat_stream_cancel_before_delivery_maps_to_aborted_setup_failure() {
    let stream = chat_stream(vec![Err(ProviderError::Cancelled)]);
    let failure = stream
        .result()
        .await
        .expect_err("cancellation before any delivery is a failure");
    assert_eq!(failure.message().stop_reason, StopReason::Aborted);
    assert!(
        failure.message().content.is_empty(),
        "setup has no generated content"
    );
    assert_eq!(
        failure.message().usage,
        Usage::default(),
        "setup usage is unknown"
    );
    assert!(matches!(failure.cause(), ProviderError::Cancelled));
}

#[tokio::test]
async fn chat_stream_mid_stream_transport_failure_preserves_partial_and_usage() {
    let (events, text) = text_events(2, None);
    let mut events = events;
    // Enrich the last partial snapshot with known usage.
    if let Some(Ok(AssistantStreamEvent::TextDelta { partial, .. })) = events.last_mut() {
        partial.usage = Usage {
            input_tokens: 11,
            output_tokens: 22,
            ..Usage::default()
        };
    }
    events.push(Err(
        ProviderError::Network(ProviderErrorSummary::redacted()),
    ));

    let failure = chat_stream(events)
        .result()
        .await
        .expect_err("mid-stream transport failure");
    assert!(matches!(failure.cause(), ProviderError::Network(_)));
    assert_eq!(assistant_text_of(failure.message()), text);
    assert_eq!(failure.message().usage.input_tokens, 11);
    assert_eq!(failure.message().usage.output_tokens, 22);
    assert_eq!(failure.message().stop_reason, StopReason::Error);
    assert!(
        failure.message().error_message.is_some(),
        "failure diagnostics are preserved on the canonical message"
    );
}

#[tokio::test]
async fn chat_stream_converts_legacy_error_event_to_call_failure() {
    // Legacy/mock producers and historical replays can still deliver an
    // in-band Error event; the public seam converts it, retaining original
    // facts, and never forwards the event itself.
    let mut message = sample_assistant_message();
    message.content = vec![AssistantContent::Text {
        text: "partial".into(),
    }];
    message.error_message = Some("boom".into());
    let mut stream = chat_stream(vec![
        Ok(AssistantStreamEvent::Start {
            partial: sample_assistant_message(),
        }),
        Ok(AssistantStreamEvent::Error {
            reason: StopReason::Error,
            message,
        }),
        Ok(done_terminal("tail", StopReason::Stop)),
    ]);

    assert!(matches!(
        stream.next().await,
        Some(Ok(AssistantStreamEvent::Start { .. }))
    ));
    let failure = match stream.next().await {
        Some(Err(failure)) => failure,
        other => panic!("legacy Error event must surface as Err(CallFailure), got {other:?}"),
    };
    assert_eq!(failure.message().error_message.as_deref(), Some("boom"));
    assert_eq!(assistant_text_of(failure.message()), "partial");
    assert_eq!(failure.message().stop_reason, StopReason::Error);
    assert!(matches!(failure.cause(), ProviderError::StreamError(_)));
    // Absorbed: permanent EOF, no tail delivery.
    assert!(stream.next().await.is_none());
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn chat_stream_error_and_aborted_done_reasons_are_failures() {
    // AIR-047: Stop/Length/ToolUse (and later Deferred) are Ok;
    // Error/Aborted are Err.
    for reason in [StopReason::Stop, StopReason::Length, StopReason::ToolUse] {
        let (events, text) = text_events(1, None);
        let mut events = events;
        events.push(Ok(done_terminal(&text, reason)));
        let message = chat_stream(events)
            .result()
            .await
            .expect("non-error stop reason completes Ok");
        assert_eq!(message.stop_reason, reason);
    }
    for reason in [StopReason::Error, StopReason::Aborted] {
        let (events, text) = text_events(1, None);
        let mut events = events;
        events.push(Ok(done_terminal(&text, reason)));
        let failure = chat_stream(events)
            .result()
            .await
            .expect_err("error/aborted stop reasons are Err");
        assert_eq!(failure.message().stop_reason, reason);
        assert_eq!(assistant_text_of(failure.message()), text);
    }
}

#[tokio::test]
async fn chat_stream_events_carry_immutable_event_time_partials() {
    let (events, _) = text_events(5, None);
    let mut stream = chat_stream(events);
    let mut prefix = String::new();
    let mut snapshots = 0;
    while let Some(item) = stream.next().await {
        if let Ok(AssistantStreamEvent::TextDelta { partial, .. }) = item {
            prefix.push('x');
            assert_eq!(
                assistant_text_of(&partial),
                prefix,
                "each event carries its own event-time snapshot"
            );
            snapshots += 1;
        }
    }
    assert_eq!(snapshots, 5);
}

#[tokio::test]
async fn chat_stream_drop_without_poll_performs_no_inner_work() {
    use std::pin::Pin;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::{Context, Poll};

    struct CountingStream {
        polls: Arc<AtomicUsize>,
    }
    impl futures_core::Stream for CountingStream {
        type Item = Result<AssistantStreamEvent, ProviderError>;
        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            self.polls.fetch_add(1, Ordering::SeqCst);
            Poll::Ready(None)
        }
    }

    let polls = Arc::new(AtomicUsize::new(0));
    let inner = CountingStream {
        polls: polls.clone(),
    };
    let stream = ChatStream::new(Box::pin(inner));
    // Lazy: constructing and dropping without polling never touches the inner
    // producer (zero source/network work before first poll).
    drop(stream);
    assert_eq!(polls.load(Ordering::SeqCst), 0);
}
