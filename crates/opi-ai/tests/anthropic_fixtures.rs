//! Behavioral tests for task 1.3: Anthropic SSE provider.
//!
//! DoD: "fixtures cover text, tool call, usage, error"
//! All tests use fixture strings — no live provider calls (red flag #10).

use futures_util::StreamExt;
use opi_ai::anthropic::{
    AnthropicEvent, AnthropicMapper, AnthropicProvider, ParsedEvent, parse_sse_events,
};
use opi_ai::message::{AssistantContent, InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: parse fixture, extract valid events, and map through a stateful mapper.
/// Fixtures are valid streams, so every item must be an event.
fn map_fixture(input: &str) -> Vec<AssistantStreamEvent> {
    map_fixture_items(input)
        .into_iter()
        .map(|item| item.expect("fixture maps to stream events"))
        .collect()
}

/// Helper: map a fixture to raw stream items (events and typed failures).
fn map_fixture_items(input: &str) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
    let events: Vec<AnthropicEvent> = parse_sse_events(input)
        .filter_map(|p| match p {
            ParsedEvent::Valid(e) => Some(e),
            ParsedEvent::UsageError(_) => None,
            ParsedEvent::Malformed { .. } => None,
        })
        .collect();
    let mut mapper = AnthropicMapper::new();
    events.into_iter().flat_map(|e| mapper.process(e)).collect()
}

/// Helper: collect valid AnthropicEvents from parsed output.
fn collect_valid_events(input: &str) -> Vec<AnthropicEvent> {
    parse_sse_events(input)
        .filter_map(|p| match p {
            ParsedEvent::Valid(e) => Some(e),
            ParsedEvent::UsageError(_) => None,
            ParsedEvent::Malformed { .. } => None,
        })
        .collect()
}

// --- SSE Parsing Tests ---

#[test]
fn sse_parse_empty_input_yields_no_events() {
    let events = collect_valid_events("");
    assert!(events.is_empty());
}

#[test]
fn sse_parse_single_event() {
    let input = r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_1","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":10,"output_tokens":0}}}

"#;
    let events = collect_valid_events(input);
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], AnthropicEvent::MessageStart { .. }));
}

#[test]
fn sse_parse_ignores_comments() {
    let input = ": this is a comment\n\n";
    let events = collect_valid_events(input);
    assert!(events.is_empty());
}

#[test]
fn sse_parse_skips_unknown_event_types() {
    let input = "event: ping\ndata: {}\n\nevent: done\ndata: [DONE]\n\n";
    let events = collect_valid_events(input);
    assert!(events.is_empty());
}

#[test]
fn anthropic_delta_before_block_start_is_dropped_not_panicked() {
    // C-2.2: a content_block_delta arriving before any content_block_start
    // must not underflow `blocks.len() - 1`. The orphan delta is dropped.
    let input = r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_1","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":10,"output_tokens":0}}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"orphan"}}

"#;
    let events = map_fixture(input);
    let text_deltas = events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::TextDelta { .. }))
        .count();
    assert_eq!(
        text_deltas, 0,
        "orphan delta before block start must be dropped, not panicked on"
    );
}

#[test]
fn anthropic_stop_before_block_start_is_dropped_not_panicked() {
    // C-2.2: a content_block_stop for a block that was never started must not
    // underflow. It produces no end event.
    let input = r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_1","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":10,"output_tokens":0}}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

"#;
    let events = map_fixture(input);
    assert!(
        events
            .iter()
            .all(|e| matches!(e, AssistantStreamEvent::Start { .. })),
        "stop before start must not emit a block-end event"
    );
}

// --- Text Fixture ---

fn text_fixture() -> &'static str {
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_abc","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":25,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" world"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":15}}

event: message_stop
data: {"type":"message_stop"}

"#
}

#[test]
fn text_fixture_yields_all_events() {
    let events = collect_valid_events(text_fixture());
    assert_eq!(events.len(), 7);
}

#[test]
fn text_fixture_maps_to_stream_events() {
    let stream_events = map_fixture(text_fixture());

    // Expected: Start, TextStart, TextDelta("Hello"), TextDelta(" world"), TextEnd, Done
    assert!(matches!(
        stream_events[0],
        AssistantStreamEvent::Start { .. }
    ));
    assert!(matches!(
        stream_events[1],
        AssistantStreamEvent::TextStart { .. }
    ));

    if let AssistantStreamEvent::TextDelta { delta, .. } = &stream_events[2] {
        assert_eq!(delta, "Hello");
    } else {
        panic!("expected TextDelta at index 2");
    }

    if let AssistantStreamEvent::TextDelta { delta, .. } = &stream_events[3] {
        assert_eq!(delta, " world");
    } else {
        panic!("expected TextDelta at index 3");
    }

    assert!(matches!(
        stream_events[4],
        AssistantStreamEvent::TextEnd { .. }
    ));
    assert!(matches!(
        &stream_events[5],
        AssistantStreamEvent::Done { reason, .. } if *reason == StopReason::Stop
    ));
}

#[test]
fn text_fixture_done_event_has_full_content() {
    let stream_events = map_fixture(text_fixture());

    if let AssistantStreamEvent::Done { message, .. } = &stream_events[5] {
        let text_content: Vec<_> = message
            .content
            .iter()
            .filter_map(|c| match c {
                AssistantContent::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(text_content, vec!["Hello world"]);
    } else {
        panic!("expected Done event");
    }
}

// --- Tool Call Fixture ---

fn tool_call_fixture() -> &'static str {
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_tool","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":50,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_abc","name":"read_file","input":{}}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"/tmp/test\"}"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":100}}

event: message_stop
data: {"type":"message_stop"}

"#
}

#[test]
fn tool_call_fixture_yields_tool_events() {
    let events = collect_valid_events(tool_call_fixture());
    assert_eq!(events.len(), 6);
}

#[test]
fn tool_call_fixture_maps_to_stream_events() {
    let stream_events = map_fixture(tool_call_fixture());

    // Start, ToolCallStart, ToolCallDelta, ToolCallEnd, Done(tool_use)
    assert!(matches!(
        stream_events[0],
        AssistantStreamEvent::Start { .. }
    ));
    assert!(matches!(
        stream_events[1],
        AssistantStreamEvent::ToolCallStart { .. }
    ));

    if let AssistantStreamEvent::ToolCallDelta { delta, .. } = &stream_events[2] {
        assert!(delta.contains("path"));
    } else {
        panic!("expected ToolCallDelta at index 2");
    }

    if let AssistantStreamEvent::ToolCallEnd { tool_call, .. } = &stream_events[3] {
        assert_eq!(tool_call.name, "read_file");
        assert_eq!(tool_call.id, "toolu_abc");
    } else {
        panic!("expected ToolCallEnd at index 3");
    }

    if let AssistantStreamEvent::Done { reason, .. } = &stream_events[4] {
        assert_eq!(*reason, StopReason::ToolUse);
    } else {
        panic!("expected Done at index 4");
    }
}

// --- Usage Fixture ---

#[test]
fn usage_captured_from_message_start() {
    let stream_events = map_fixture(text_fixture());

    if let AssistantStreamEvent::Start { partial } = &stream_events[0] {
        assert_eq!(partial.usage.input_tokens, 25);
    } else {
        panic!("expected Start event");
    }
}

#[test]
fn usage_updated_in_done_event() {
    let stream_events = map_fixture(text_fixture());

    if let AssistantStreamEvent::Done { message, .. } = &stream_events[5] {
        assert!(message.usage.is_reported());
        assert_eq!(message.usage.output_tokens, 15);
        assert_eq!(message.usage.input_tokens, 25);
    } else {
        panic!("expected Done event");
    }
}

#[test]
fn tool_call_usage_tracked() {
    let stream_events = map_fixture(tool_call_fixture());

    if let AssistantStreamEvent::Done { message, .. } = &stream_events[4] {
        assert!(message.usage.is_reported());
        assert_eq!(message.usage.input_tokens, 50);
        assert_eq!(message.usage.output_tokens, 100);
    } else {
        panic!("expected Done event");
    }
}

// --- Cache token fields (Phase 12 task 12.6, DoD clause 6) ---

fn cache_tokens_fixture() -> &'static str {
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_cache","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":100,"output_tokens":0,"cache_read_input_tokens":250,"cache_creation_input_tokens":75}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"cached"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":20}}

event: message_stop
data: {"type":"message_stop"}

"#
}

#[test]
fn cache_tokens_captured_from_message_start() {
    let stream_events = map_fixture(cache_tokens_fixture());

    let done = stream_events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("expected Done event");
    if let AssistantStreamEvent::Done { message, .. } = done {
        assert_eq!(
            message.usage.cache_read_tokens, 250,
            "cache_read_input_tokens must map to cache_read_tokens"
        );
        assert_eq!(
            message.usage.cache_write_tokens, 75,
            "cache_creation_input_tokens must map to cache_write_tokens"
        );
    }
}

// --- 1h cache write subset (Phase 14 task 14.5) ---

fn cache_1h_fixture() -> &'static str {
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_1h","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":100,"output_tokens":0,"cache_read_input_tokens":0,"cache_creation_input_tokens":200,"cache_creation_input_tokens_1h":150}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"1h"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":20}}

event: message_stop
data: {"type":"message_stop"}

"#
}

#[test]
fn cache_1h_tokens_parsed_as_subset_of_cache_write() {
    let stream_events = map_fixture(cache_1h_fixture());
    let done = stream_events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("expected Done event");
    if let AssistantStreamEvent::Done { message, .. } = done {
        assert_eq!(message.usage.cache_write_tokens, 200);
        assert_eq!(message.usage.cache_write_1h_tokens, Some(150));
        assert!(
            message.usage.cache_write_1h_tokens.unwrap()
                <= u64::from(message.usage.cache_write_tokens)
        );
        assert!(message.usage.is_reported());
    }
}

#[test]
fn cache_1h_absent_zero_and_equality_are_preserved() {
    let absent = cache_1h_fixture().replace(",\"cache_creation_input_tokens_1h\":150", "");
    let zero = cache_1h_fixture().replace(
        "\"cache_creation_input_tokens_1h\":150",
        "\"cache_creation_input_tokens_1h\":0",
    );
    let equal = cache_1h_fixture().replace(
        "\"cache_creation_input_tokens_1h\":150",
        "\"cache_creation_input_tokens_1h\":200",
    );

    for (fixture, expected) in [
        (absent.as_str(), None),
        (zero.as_str(), Some(0)),
        (equal.as_str(), Some(200)),
    ] {
        let stream_events = map_fixture(fixture);
        let usage = stream_events
            .iter()
            .find_map(|event| match event {
                AssistantStreamEvent::Done { message, .. } => Some(&message.usage),
                _ => None,
            })
            .expect("expected Done event");
        assert_eq!(usage.cache_write_1h_tokens, expected);
    }
}

fn cache_1h_malformed_fixture() -> &'static str {
    // cache_creation_input_tokens_1h (500) > cache_creation_input_tokens (200) is malformed.
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_bad","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":100,"output_tokens":0,"cache_read_input_tokens":0,"cache_creation_input_tokens":200,"cache_creation_input_tokens_1h":500}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"bad"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":20}}

event: message_stop
data: {"type":"message_stop"}

"#
}

#[tokio::test]
async fn cache_1h_malformed_subset_stops_production_stream_with_non_retryable_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(cache_1h_malformed_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new(Some(server.uri()));
    let request = Request {
        transport: Default::default(),
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
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

    let results: Vec<_> = provider
        .stream_prepared(request, opi_ai::test_support::resolved_auth())
        .collect()
        .await;
    assert_eq!(results.len(), 1, "invalid usage must stop the stream");
    let error = results[0].as_ref().expect_err("expected StreamError");
    assert!(error.category() == opi_ai::provider::ProviderErrorCategory::Stream);
    assert!(!error.is_retryable());
    assert!(
        !results.iter().any(|result| matches!(
            result,
            Ok(AssistantStreamEvent::Done { .. } | AssistantStreamEvent::Error { .. })
        )),
        "no completion event may follow malformed usage"
    );
}

// --- Missing-usage graceful handling (Phase 12 task 12.6, DoD clause 3) ---

fn missing_usage_fixture() -> &'static str {
    // message_start carries no `usage` object; message_delta carries an empty usage.
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_nousage","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"ok"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{}}

event: message_stop
data: {"type":"message_stop"}

"#
}

#[test]
fn missing_usage_yields_graceful_zero_tokens() {
    let stream_events = map_fixture(missing_usage_fixture());

    let done = stream_events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("expected Done event");
    if let AssistantStreamEvent::Done { message, .. } = done {
        // A stream that reports no usage must map to zero usage without panicking.
        assert!(!message.usage.is_reported());
        assert_eq!(message.usage.input_tokens, 0);
        assert_eq!(message.usage.output_tokens, 0);
        assert_eq!(message.usage.cache_read_tokens, 0);
        assert_eq!(message.usage.cache_write_tokens, 0);
    }
}

// --- Error Fixture ---

fn error_fixture() -> &'static str {
    r#"event: error
data: {"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}

"#
}

#[test]
fn error_fixture_parsed_as_error() {
    let events = collect_valid_events(error_fixture());
    assert!(matches!(events[0], AnthropicEvent::Error { .. }));
}

#[test]
fn error_event_maps_to_stream_error() {
    let stream_events = map_fixture_items(error_fixture());

    assert_eq!(stream_events.len(), 1);
    // Current producers stop the dual terminal format: the in-band error
    // event is a stream failure with a bounded typed cause.
    match &stream_events[0] {
        Err(cause) if cause.category() == opi_ai::provider::ProviderErrorCategory::Stream => {
            let summary = cause.to_string();
            assert!(
                summary.contains("anthropic stream error"),
                "cause must be the neutral literal, got: {summary}"
            );
            assert!(
                !summary.contains("Overloaded"),
                "raw upstream error text must not leak into the cause: {summary}"
            );
        }
        other => panic!("expected stream error item, got {other:?}"),
    }
}

// --- Stop Reason Mapping Tests ---

#[test]
fn stop_reason_end_turn_maps_to_stop() {
    let stream_events = map_fixture(text_fixture());

    if let AssistantStreamEvent::Done { reason, .. } = &stream_events[5] {
        assert_eq!(*reason, StopReason::Stop);
    } else {
        panic!("expected Done with StopReason::Stop");
    }
}

#[test]
fn stop_reason_tool_use_maps_correctly() {
    let stream_events = map_fixture(tool_call_fixture());

    if let AssistantStreamEvent::Done { reason, .. } = &stream_events[4] {
        assert_eq!(*reason, StopReason::ToolUse);
    } else {
        panic!("expected Done with StopReason::ToolUse");
    }
}

// --- AnthropicProvider Tests ---

#[test]
fn anthropic_provider_id() {
    let provider = opi_ai::anthropic::AnthropicProvider::new(None);
    assert_eq!(provider.id(), "anthropic");
}

#[test]
fn anthropic_provider_models_not_empty() {
    let provider = opi_ai::anthropic::AnthropicProvider::new(None);
    assert!(!provider.models().is_empty());
}

// --- Mixed text + tool call fixture ---

fn mixed_fixture() -> &'static str {
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_mix","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":30,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Let me read that file."}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: content_block_start
data: {"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_123","name":"read_file","input":{}}}

event: content_block_delta
data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"src/main.rs\"}"}}

event: content_block_stop
data: {"type":"content_block_stop","index":1}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":45}}

event: message_stop
data: {"type":"message_stop"}

"#
}

#[test]
fn mixed_fixture_produces_text_then_tool_call() {
    let stream_events = map_fixture(mixed_fixture());

    // Start, TextStart, TextDelta, TextEnd, ToolCallStart, ToolCallDelta, ToolCallEnd, Done
    assert_eq!(stream_events.len(), 8);
    assert!(matches!(
        stream_events[1],
        AssistantStreamEvent::TextStart { .. }
    ));
    assert!(matches!(
        stream_events[4],
        AssistantStreamEvent::ToolCallStart { .. }
    ));

    if let AssistantStreamEvent::Done { message, reason } = &stream_events[7] {
        assert_eq!(*reason, StopReason::ToolUse);
        assert_eq!(message.content.len(), 2);
    } else {
        panic!("expected Done event");
    }
}

// --- Malformed SSE Tests ---

#[test]
fn malformed_sse_data_produces_malformed_event() {
    let input = r#"event: message_start
data: {invalid json here}

"#;
    let parsed: Vec<_> = parse_sse_events(input).collect();
    assert_eq!(parsed.len(), 1);
    assert!(
        matches!(&parsed[0], ParsedEvent::Malformed { event_type, .. } if event_type == "message_start"),
        "expected Malformed event for invalid JSON data"
    );
}

#[test]
fn malformed_and_valid_events_coexist() {
    let input = r#"event: message_start
data: {bad json}

event: message_stop
data: {"type":"message_stop"}

"#;
    let parsed: Vec<_> = parse_sse_events(input).collect();
    assert_eq!(parsed.len(), 2);
    assert!(matches!(parsed[0], ParsedEvent::Malformed { .. }));
    assert!(matches!(parsed[1], ParsedEvent::Valid(_)));
}

// --- CRLF SSE Tests ---

#[test]
fn sse_parse_handles_crlf_line_endings() {
    // Simulate real HTTP SSE with CRLF line endings
    let input = "event: message_start\r\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[],\"model\":\"claude-sonnet-4-5-20250514\",\"stop_reason\":null,\"usage\":{\"input_tokens\":10,\"output_tokens\":0}}}\r\n\r\n";

    let events = collect_valid_events(input);
    assert_eq!(events.len(), 1);
    assert!(
        matches!(events[0], AnthropicEvent::MessageStart { .. }),
        "CRLF-delimited SSE should parse correctly"
    );
}

#[test]
fn sse_parse_handles_crlf_full_fixture() {
    // Build a CRLF version of the text fixture
    let lf_fixture = text_fixture();
    let crlf_fixture = lf_fixture.replace('\n', "\r\n");

    let events = collect_valid_events(&crlf_fixture);
    assert_eq!(
        events.len(),
        7,
        "CRLF fixture should parse same as LF fixture"
    );
}

#[tokio::test]
async fn drain_sse_events_handles_crlf_stream() {
    // Build a CRLF SSE stream with event separator
    let lf_input = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_crlf\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[],\"model\":\"claude-sonnet-4-5-20250514\",\"stop_reason\":null,\"usage\":{\"input_tokens\":10,\"output_tokens\":0}}}\n\n";
    let crlf_input = lf_input.replace('\n', "\r\n");

    let provider = AnthropicProvider::new(None);
    let cancel = CancellationToken::new();
    let mut stream = provider.stream_from_sse(&crlf_input, cancel);

    use futures_util::StreamExt;
    let first = stream.next().await.expect("should have an event");
    assert!(first.is_ok(), "CRLF SSE should produce valid stream events");
}

// --- Thinking Fixture (task 2.9) ---

fn thinking_fixture() -> &'static str {
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_think","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":30,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Let me analyze this problem."}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":" Step 1: identify the key constraint."}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: content_block_start
data: {"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"The answer is 42."}}

event: content_block_stop
data: {"type":"content_block_stop","index":1}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":120}}

event: message_stop
data: {"type":"message_stop"}

"#
}

#[test]
fn thinking_fixture_yields_thinking_start_delta_end() {
    let stream_events = map_fixture(thinking_fixture());

    // Start, ThinkingStart, ThinkingDelta("Let me..."), ThinkingDelta(" Step 1..."),
    // ThinkingEnd, TextStart, TextDelta, TextEnd, Done
    assert!(matches!(
        stream_events[0],
        AssistantStreamEvent::Start { .. }
    ));

    assert!(
        matches!(
            &stream_events[1],
            AssistantStreamEvent::ThinkingStart { content_index, .. } if *content_index == 0
        ),
        "expected ThinkingStart at index 1"
    );

    if let AssistantStreamEvent::ThinkingDelta {
        delta,
        content_index,
        ..
    } = &stream_events[2]
    {
        assert_eq!(delta, "Let me analyze this problem.");
        assert_eq!(*content_index, 0);
    } else {
        panic!("expected ThinkingDelta at index 2");
    }

    if let AssistantStreamEvent::ThinkingDelta { delta, .. } = &stream_events[3] {
        assert_eq!(delta, " Step 1: identify the key constraint.");
    } else {
        panic!("expected ThinkingDelta at index 3");
    }

    if let AssistantStreamEvent::ThinkingEnd {
        content,
        content_index,
        ..
    } = &stream_events[4]
    {
        assert_eq!(
            content,
            "Let me analyze this problem. Step 1: identify the key constraint."
        );
        assert_eq!(*content_index, 0);
    } else {
        panic!("expected ThinkingEnd at index 4");
    }
}

#[test]
fn thinking_fixture_text_after_thinking() {
    let stream_events = map_fixture(thinking_fixture());

    // After thinking events, text events follow at content_index 1
    assert!(matches!(
        &stream_events[5],
        AssistantStreamEvent::TextStart { content_index, .. } if *content_index == 1
    ));

    if let AssistantStreamEvent::TextDelta { delta, .. } = &stream_events[6] {
        assert_eq!(delta, "The answer is 42.");
    } else {
        panic!("expected TextDelta at index 6");
    }

    if let AssistantStreamEvent::TextEnd {
        content,
        content_index,
        ..
    } = &stream_events[7]
    {
        assert_eq!(content, "The answer is 42.");
        assert_eq!(*content_index, 1);
    } else {
        panic!("expected TextEnd at index 7");
    }
}

#[test]
fn thinking_fixture_done_has_both_thinking_and_text() {
    let stream_events = map_fixture(thinking_fixture());

    if let AssistantStreamEvent::Done { message, reason } = &stream_events[8] {
        assert_eq!(*reason, StopReason::Stop);
        assert_eq!(
            message.content.len(),
            2,
            "should have thinking + text content"
        );

        // First content block is thinking
        if let AssistantContent::Thinking { thinking, .. } = &message.content[0] {
            assert_eq!(
                thinking,
                "Let me analyze this problem. Step 1: identify the key constraint."
            );
        } else {
            panic!("expected Thinking content at index 0");
        }

        // Second content block is text
        if let AssistantContent::Text { text, .. } = &message.content[1] {
            assert_eq!(text, "The answer is 42.");
        } else {
            panic!("expected Text content at index 1");
        }
    } else {
        panic!("expected Done event at index 8");
    }
}

#[test]
fn thinking_fixture_usage_tracked() {
    let stream_events = map_fixture(thinking_fixture());

    if let AssistantStreamEvent::Done { message, .. } = &stream_events[8] {
        assert_eq!(message.usage.input_tokens, 30);
        assert_eq!(message.usage.output_tokens, 120);
    } else {
        panic!("expected Done event");
    }
}

// --- budget_tokens request body test ---

#[test]
fn build_request_body_includes_thinking_when_enabled() {
    let provider = AnthropicProvider::new(None);
    let request = Request {
        transport: Default::default(),
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(4096),
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig {
            enabled: true,
            budget_tokens: Some(5000),
            level: opi_ai::ThinkingLevel::Medium,
        },
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

    let thinking = body
        .get("thinking")
        .expect("thinking field should be present");
    assert_eq!(thinking["type"], "enabled");
    assert_eq!(thinking["budget_tokens"], 5000);
}

#[test]
fn build_request_body_uses_default_budget_when_none() {
    let provider = AnthropicProvider::new(None);
    let request = Request {
        transport: Default::default(),
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(4096),
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig {
            enabled: true,
            budget_tokens: None,
            level: opi_ai::ThinkingLevel::Medium,
        },
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

    let thinking = body
        .get("thinking")
        .expect("thinking field should be present");
    // Task 20.39: the source native default is 1024 (pinned
    // `thinkingBudgetTokens || 1024`); the old 10000 default is replaced.
    assert_eq!(
        thinking["budget_tokens"], 1024,
        "default budget should be the source native default 1024"
    );
}

#[test]
fn build_request_body_omits_thinking_when_disabled() {
    let provider = AnthropicProvider::new(None);
    let request = Request {
        transport: Default::default(),
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(4096),
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

    assert!(
        body.get("thinking").is_none(),
        "thinking should be absent when disabled"
    );
}

// --- Thinking-only fixture (no text after thinking) ---

fn thinking_only_fixture() -> &'static str {
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_think_only","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":15,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Reasoning only"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":50}}

event: message_stop
data: {"type":"message_stop"}

"#
}

#[test]
fn thinking_only_fixture_produces_correct_events() {
    let stream_events = map_fixture(thinking_only_fixture());

    // Start, ThinkingStart, ThinkingDelta, ThinkingEnd, Done
    assert_eq!(stream_events.len(), 5);

    assert!(matches!(
        &stream_events[1],
        AssistantStreamEvent::ThinkingStart { content_index, .. } if *content_index == 0
    ));
    assert!(matches!(
        &stream_events[2],
        AssistantStreamEvent::ThinkingDelta { .. }
    ));
    assert!(matches!(
        &stream_events[3],
        AssistantStreamEvent::ThinkingEnd { .. }
    ));

    if let AssistantStreamEvent::Done { message, reason } = &stream_events[4] {
        assert_eq!(*reason, StopReason::Stop);
        assert_eq!(message.content.len(), 1);
        if let AssistantContent::Thinking { thinking, .. } = &message.content[0] {
            assert_eq!(thinking, "Reasoning only");
        } else {
            panic!("expected Thinking content");
        }
    } else {
        panic!("expected Done event");
    }
}

// --- Empty thinking block (zero deltas) ---

fn empty_thinking_fixture() -> &'static str {
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_empty_think","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":10,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: content_block_start
data: {"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Short answer."}}

event: content_block_stop
data: {"type":"content_block_stop","index":1}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":20}}

event: message_stop
data: {"type":"message_stop"}

"#
}

#[test]
fn empty_thinking_block_emits_empty_thinking_end() {
    let stream_events = map_fixture(empty_thinking_fixture());

    // Start, ThinkingStart, ThinkingEnd, TextStart, TextDelta, TextEnd, Done
    assert_eq!(stream_events.len(), 7);

    // ThinkingStart at index 1
    assert!(matches!(
        &stream_events[1],
        AssistantStreamEvent::ThinkingStart { content_index, .. } if *content_index == 0
    ));

    // ThinkingEnd with empty content at index 2 (no deltas between start and end)
    if let AssistantStreamEvent::ThinkingEnd {
        content,
        content_index,
        ..
    } = &stream_events[2]
    {
        assert_eq!(
            content, "",
            "empty thinking block should produce empty content"
        );
        assert_eq!(*content_index, 0);
    } else {
        panic!("expected ThinkingEnd at index 2");
    }

    // Done message should have Thinking with empty string
    if let AssistantStreamEvent::Done { message, .. } = &stream_events[6] {
        if let AssistantContent::Thinking { thinking, .. } = &message.content[0] {
            assert_eq!(thinking, "");
        } else {
            panic!("expected Thinking content in Done");
        }
    } else {
        panic!("expected Done event");
    }
}

// --- Thinking + tool call interleave ---

fn thinking_then_tool_fixture() -> &'static str {
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_think_tool","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":40,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"I need to read the file."}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: content_block_start
data: {"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_rt","name":"read_file","input":{}}}

event: content_block_delta
data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"/etc/hosts\"}"}}

event: content_block_stop
data: {"type":"content_block_stop","index":1}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":80}}

event: message_stop
data: {"type":"message_stop"}

"#
}

#[test]
fn thinking_then_tool_call_tracks_content_indices() {
    let stream_events = map_fixture(thinking_then_tool_fixture());

    // Start, ThinkingStart(0), ThinkingDelta(0), ThinkingEnd(0),
    // ToolCallStart(1), ToolCallDelta(1), ToolCallEnd(1), Done
    assert_eq!(stream_events.len(), 8);

    // Thinking at content_index 0
    assert!(matches!(
        &stream_events[1],
        AssistantStreamEvent::ThinkingStart { content_index, .. } if *content_index == 0
    ));
    assert!(matches!(
        &stream_events[2],
        AssistantStreamEvent::ThinkingDelta { content_index, .. } if *content_index == 0
    ));
    if let AssistantStreamEvent::ThinkingEnd {
        content,
        content_index,
        ..
    } = &stream_events[3]
    {
        assert_eq!(content, "I need to read the file.");
        assert_eq!(*content_index, 0);
    } else {
        panic!("expected ThinkingEnd at index 3");
    }

    // Tool call at content_index 1
    assert!(matches!(
        &stream_events[4],
        AssistantStreamEvent::ToolCallStart { content_index, .. } if *content_index == 1
    ));
    assert!(matches!(
        &stream_events[5],
        AssistantStreamEvent::ToolCallDelta { content_index, .. } if *content_index == 1
    ));
    if let AssistantStreamEvent::ToolCallEnd {
        tool_call,
        content_index,
        ..
    } = &stream_events[6]
    {
        assert_eq!(tool_call.name, "read_file");
        assert_eq!(*content_index, 1);
    } else {
        panic!("expected ToolCallEnd at index 6");
    }

    // Done with tool_use reason and both content blocks
    if let AssistantStreamEvent::Done { message, reason } = &stream_events[7] {
        assert_eq!(*reason, StopReason::ToolUse);
        assert_eq!(
            message.content.len(),
            2,
            "should have thinking + tool_call content"
        );
        assert!(matches!(
            &message.content[0],
            AssistantContent::Thinking { .. }
        ));
        assert!(matches!(
            &message.content[1],
            AssistantContent::ToolCall { .. }
        ));
    } else {
        panic!("expected Done event");
    }
}

// ---------------------------------------------------------------------------
// Phase 12 task 12.4 — tool-call conversion breadth (scenarios 3/4/5)
//
// DoD: fixtures cover multiple tool calls, chunked arguments, and malformed
// JSON arguments. The Anthropic mapper accumulates per-block `partial_json`
// and emits ToolCallEnd carrying the raw argument string, so malformed JSON
// passes through unchanged to the agent-loop validation path (task 12.4
// opi-agent side), never a provider-layer panic.

fn multi_tool_fixture() -> &'static str {
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_multi","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":40,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_a","name":"read_file","input":{}}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"a.rs\"}"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: content_block_start
data: {"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_b","name":"bash","input":{}}}

event: content_block_delta
data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"cmd\":\"ls\"}"}}

event: content_block_stop
data: {"type":"content_block_stop","index":1}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":90}}

event: message_stop
data: {"type":"message_stop"}

"#
}

#[test]
fn multi_tool_fixture_produces_two_tool_calls() {
    let stream_events = map_fixture(multi_tool_fixture());

    let starts = stream_events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::ToolCallStart { .. }))
        .count();
    let ends = stream_events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::ToolCallEnd { .. }))
        .count();
    assert_eq!(starts, 2, "two ToolCallStart events");
    assert_eq!(ends, 2, "two ToolCallEnd events");

    if let Some(AssistantStreamEvent::Done { message, reason }) = stream_events.last() {
        assert_eq!(*reason, StopReason::ToolUse);
        assert_eq!(
            message.content.len(),
            2,
            "Done message carries both tool calls"
        );
    } else {
        panic!("expected Done event");
    }
}

fn chunked_args_fixture() -> &'static str {
    // One tool_use block receives multiple input_json_delta chunks; the mapper
    // MUST concatenate them and surface the merged string at ToolCallEnd.
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_chunk","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":20,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_chunk","name":"edit_file","input":{}}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"path\":"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"\"src/lib.rs\","}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"\"old\":\"x\"}"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":30}}

event: message_stop
data: {"type":"message_stop"}

"#
}

#[test]
fn chunked_args_fixture_merges_into_one_tool_call() {
    let stream_events = map_fixture(chunked_args_fixture());

    let deltas = stream_events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::ToolCallDelta { .. }))
        .count();
    assert_eq!(
        deltas, 3,
        "three ToolCallDelta events accumulate into one block"
    );

    let ends: Vec<_> = stream_events
        .iter()
        .filter_map(|e| match e {
            AssistantStreamEvent::ToolCallEnd { tool_call, .. } => Some(tool_call.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(ends.len(), 1, "one ToolCallEnd");
    assert_eq!(
        ends[0].arguments, r#"{"path":"src/lib.rs","old":"x"}"#,
        "merged argument string at ToolCallEnd"
    );
    assert_eq!(ends[0].id, "toolu_chunk");
}

fn malformed_args_fixture() -> &'static str {
    // The SSE event is well-formed; only the tool argument VALUE is malformed
    // JSON. The provider layer must NOT parse arguments, so it hands the raw
    // string to the agent loop without panic (DoD: malformed arguments reach
    // runtime validation, not provider crashes).
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_bad","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":10,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_bad","name":"read_file","input":{}}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{not-json"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":5}}

event: message_stop
data: {"type":"message_stop"}

"#
}

#[test]
fn malformed_args_fixture_passes_raw_string_without_panic() {
    // Draining the stream is itself the no-panic assertion.
    let stream_events = map_fixture(malformed_args_fixture());

    let end = stream_events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::ToolCallEnd { tool_call, .. } => Some(tool_call.clone()),
            _ => None,
        })
        .expect("ToolCallEnd emitted despite malformed argument JSON");
    // Raw malformed string preserved byte-for-byte for the agent loop.
    assert_eq!(end.arguments, "{not-json");
    assert_eq!(end.id, "toolu_bad");
    assert_eq!(end.name, "read_file");
}

// ---------------------------------------------------------------------------
// Production request contract through Provider::stream (Phase 12.1)
// ---------------------------------------------------------------------------
//
// Proves a representative text request sent through the production
// `Provider::stream` HTTP path carries the Anthropic body shape, x-api-key +
// anthropic-version headers, and /v1/messages endpoint.

#[tokio::test]
async fn stream_sends_text_request_body_and_auth_through_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key"))
        .and(header("anthropic-version", "2023-06-01"))
        .and(body_partial_json(serde_json::json!({
            "messages": [{"role": "user", "content": [{"type": "text", "text": "Hello"}]}],
            "system": [{
                "type": "text",
                "text": "You are helpful.",
                "cache_control": {"type": "ephemeral"}
            }],
            "max_tokens": 1024
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new(Some(server.uri()));
    let request = Request {
        transport: Default::default(),
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
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

    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
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

    // verify() confirms the production request carried the Anthropic body,
    // x-api-key + anthropic-version headers, and the /v1/messages path.
    server.verify().await;
}

// ---------------------------------------------------------------------------
// Provider stream cancellation (Phase 12 task 12.7 DoD clause 6)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_cancellation_drains_without_hang_after_cancel() {
    // The CancellationToken is threaded into the Anthropic adapter's HTTP
    // body-stream loop (anthropic.rs `cancel.cancelled()` select arm).
    // This wiremock fixture is fully buffered, so it only proves the adapter
    // drains promptly after cancellation; it does not prove cancellation wins
    // a race against delayed terminal SSE data.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = AnthropicProvider::new(Some(server.uri()));
    let request = Request {
        transport: Default::default(),
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
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
        cancel: cancel.clone(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());

    // Read one event so the stream is open, then cancel.
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

mod request_compatibility {
    use std::sync::Arc;

    use futures_util::StreamExt;
    use opi_ai::anthropic::AnthropicProvider;
    use opi_ai::http::HttpClient;
    use opi_ai::model_info::{AnthropicMessagesCompat, ThinkingLevel, WireCompat};
    use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
    use opi_ai::{ModelCapabilities, ModelInfo, WireApi};
    use tokio_util::sync::CancellationToken;

    fn request(model: &str) -> Request {
        Request {
            transport: Default::default(),
            model: model.into(),
            context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                system_prompt: None,
                messages: Vec::new(),
                tools: Vec::new(),
            })
            .expect("valid context declarations"),
            max_tokens: None,
            temperature: None,
            sampling_params: Default::default(),
            thinking: ThinkingConfig::default(),
            stop_sequences: Vec::new(),
            metadata: None,
            cancel: CancellationToken::new(),
            timeout: None,
            extra_headers: Vec::new(),
            cache_retention: CacheRetention::None,
            session_id: None,
            telemetry_context: None,
            callbacks: Default::default(),
        }
    }
    #[tokio::test]
    async fn direct_anthropic_rejects_unsupported_thinking_before_auth() {
        let model = ModelInfo::new(
            "thinking-model",
            "Thinking Model",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(1000, 100).with_thinking(true),
        );
        let provider = AnthropicProvider::for_route(
            "test".into(),
            vec![model],
            None,
            Default::default(),
            Arc::new(HttpClient::new()),
            false,
        );
        let mut request = request("test:thinking-model");
        request.thinking = ThinkingConfig {
            enabled: true,
            budget_tokens: None,
            level: ThinkingLevel::XHigh,
        };

        let error = provider
            .stream_prepared(request, opi_ai::test_support::resolved_auth())
            .next()
            .await
            .expect("preflight result")
            .expect_err("unsupported thinking must fail");

        assert!(matches!(error, ProviderError::UnsupportedCapability { .. }));
    }
    #[test]
    fn anthropic_request_honors_adaptive_thinking_and_temperature_compat() {
        let model = ModelInfo::new(
            "copilot-model",
            "Copilot Model",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(1000, 100).with_thinking(true),
        )
        .with_compat(WireCompat::AnthropicMessages(AnthropicMessagesCompat {
            supports_eager_tool_input_streaming: false,
            force_adaptive_thinking: true,
            supports_temperature: false,
            supports_strict_tools: false,
            supports_mid_convo_system_messages: false,
            supports_mid_convo_tool_changes: false,
        }))
        .unwrap();
        let provider = AnthropicProvider::for_route(
            "copilot".into(),
            vec![model],
            None,
            Default::default(),
            Arc::new(HttpClient::new()),
            false,
        );
        let mut request = request("copilot:copilot-model");
        request.temperature = Some(0.4);
        request.thinking = ThinkingConfig {
            enabled: true,
            budget_tokens: Some(4096),
            level: ThinkingLevel::High,
        };

        let body = provider.build_request_body(&request).unwrap();

        assert!(body.get("temperature").is_none());
        // Task 20.39 replacement proof: adaptive thinking carries the display
        // default (summarized) per pi buildParams.
        assert_eq!(
            body["thinking"],
            serde_json::json!({"type": "adaptive", "display": "summarized"})
        );
    }
    #[test]
    fn anthropic_direct_catalog_preserves_fixed_thinking_and_suppresses_temperature() {
        let provider = AnthropicProvider::new(None);
        let mut request = request("anthropic:claude-sonnet-4-5-20250514");
        request.temperature = Some(0.4);
        request.thinking = ThinkingConfig {
            enabled: true,
            budget_tokens: Some(4096),
            level: ThinkingLevel::High,
        };

        let body = provider.build_request_body(&request).unwrap();

        assert!(body.get("temperature").is_none());
        // Task 20.39 replacement proof: enabled thinking always carries the
        // display field (default summarized) per pi buildParams.
        assert_eq!(
            body["thinking"],
            serde_json::json!({"type": "enabled", "budget_tokens": 4096, "display": "summarized"})
        );
    }
}

// ---------------------------------------------------------------------------
// Task 20.51: functional callbacks at the Anthropic wire (P20-S51)
// ---------------------------------------------------------------------------

use opi_ai::callbacks::RequestCallbacks;

fn callbacks_request() -> Request {
    Request {
        transport: Default::default(),
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text { text: "hi".into() }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig {
            enabled: true,
            budget_tokens: Some(1024),
            level: opi_ai::ThinkingLevel::Medium,
        },
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

fn callbacks_sse() -> String {
    concat!(
        "event: message_start\r\n",
        "data: {\"type\":\"message_start\",\"message\":{\"model\":\"claude\",\"usage\":{\"input_tokens\":3,\"output_tokens\":0}}}\r\n\r\n",
        "event: content_block_start\r\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\r\n\r\n",
        "event: message_stop\r\n",
        "data: {\"type\":\"message_stop\"}\r\n\r\n",
    )
    .into()
}

#[tokio::test]
async fn payload_callback_observes_final_wire_body_after_projection() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_string(callbacks_sse()))
        .expect(1)
        .mount(&server)
        .await;

    let seen: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let provider = AnthropicProvider::new(Some(server.uri()));
    let mut request = callbacks_request();
    request.callbacks = RequestCallbacks::new().with_on_payload(std::sync::Arc::new(move |body| {
        recorder.lock().unwrap().push(body.clone());
        Ok(None)
    }));

    stream_prepared_and_drain(&provider, request).await;

    let bodies = seen.lock().unwrap().clone();
    assert_eq!(bodies.len(), 1);
    let sent = server.received_requests().await.unwrap()[0].clone();
    let wire_body: serde_json::Value =
        serde_json::from_slice(&sent.body).expect("captured body is JSON");
    assert_eq!(
        bodies[0], wire_body,
        "payload callback observes the exact post-projection wire body"
    );
    assert!(
        bodies[0]["thinking"]["budget_tokens"] == serde_json::json!(1024),
        "thinking projection precedes the payload callback"
    );
}

#[tokio::test]
async fn async_response_callback_precedes_every_stream_event() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(callbacks_sse()))
        .mount(&server)
        .await;

    let order: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let response_recorder = order.clone();
    let provider = AnthropicProvider::new(Some(server.uri()));
    let mut request = callbacks_request();
    request.callbacks = RequestCallbacks::new().with_async_on_response(move |status, headers| {
        let response_recorder = response_recorder.clone();
        async move {
            tokio::task::yield_now().await;
            response_recorder
                .lock()
                .unwrap()
                .push(format!("response:{status}"));
            assert!(
                headers
                    .iter()
                    .any(|(name, _)| name.eq_ignore_ascii_case("content-type")),
                "response callback sees the wire response headers"
            );
            Ok(())
        }
    });

    let event_recorder = order.clone();
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    while let Some(item) = stream.next().await {
        if item.is_ok() {
            event_recorder.lock().unwrap().push("event".into());
        }
    }

    let order = order.lock().unwrap().clone();
    assert_eq!(order.first().map(String::as_str), Some("response:200"));
    assert!(
        order.iter().filter(|m| m.as_str() == "event").count() >= 1,
        "normalized stream events follow the response callback"
    );
}

#[tokio::test]
async fn cancelled_async_anthropic_payload_exits_before_dispatch() {
    use std::sync::Arc;
    struct NotifyDrop(Arc<tokio::sync::Notify>);
    impl Drop for NotifyDrop {
        fn drop(&mut self) {
            self.0.notify_one();
        }
    }
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let entered = Arc::new(tokio::sync::Notify::new());
    let dropped = Arc::new(tokio::sync::Notify::new());
    let marker = entered.clone();
    let release = dropped.clone();
    let mut request = callbacks_request();
    let cancel = request.cancel.clone();
    request.callbacks = RequestCallbacks::new().with_async_on_payload(move |_| {
        let marker = marker.clone();
        let release = release.clone();
        async move {
            let _drop = NotifyDrop(release);
            marker.notify_one();
            std::future::pending::<()>().await;
            Ok(None)
        }
    });
    let provider = AnthropicProvider::new(Some(server.uri()));
    let stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    // The direct adapter is lazy; poll the result concurrently with the barrier.
    let result = opi_ai::stream::ChatStream::new(stream).result();
    tokio::pin!(result);
    tokio::select! {_ = entered.notified()=>{}, outcome=&mut result=>panic!("callback ended early: {outcome:?}"), _=tokio::time::sleep(std::time::Duration::from_secs(1))=>panic!("payload not polled")}
    cancel.cancel();
    let failure = tokio::time::timeout(std::time::Duration::from_secs(1), &mut result)
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(
        failure.cause().category(),
        opi_ai::provider::ProviderErrorCategory::Cancelled
    );
    tokio::time::timeout(std::time::Duration::from_secs(1), dropped.notified())
        .await
        .unwrap();
    server.verify().await;
}

#[tokio::test]
async fn raw_callback_sees_wire_events_in_order_before_normalization() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(callbacks_sse()))
        .mount(&server)
        .await;

    let raw_types: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let raw_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let raw_recorder = raw_types.clone();
    let raw_counter = raw_count.clone();
    let provider = AnthropicProvider::new(Some(server.uri()));
    let mut request = callbacks_request();
    request.callbacks = RequestCallbacks::new().with_on_raw(std::sync::Arc::new(move |event| {
        raw_recorder
            .lock()
            .unwrap()
            .push(event["type"].as_str().unwrap_or("<unknown>").to_string());
        raw_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }));

    // Happens-before proof: normalized event #i is only sent after
    // notify_raw(#i), so at the moment the consumer observes the i-th event
    // the raw count must already be >= i + 1.
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let mut observed = 0usize;
    while let Some(item) = stream.next().await {
        if item.is_ok() {
            observed += 1;
            assert!(
                raw_count.load(std::sync::atomic::Ordering::SeqCst) >= observed,
                "raw event {observed} must fire before its normalized emission"
            );
        }
    }
    assert_eq!(observed, 3);

    let raw = raw_types.lock().unwrap();
    assert_eq!(
        raw.iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["message_start", "content_block_start", "message_stop"],
        "raw wire events arrive in stream order"
    );
}

#[tokio::test]
async fn raw_callback_failure_preserves_partial_and_ends_typed() {
    let sse = concat!(
        "event: message_start\r\n",
        "data: {\"type\":\"message_start\",\"message\":{\"model\":\"claude\",\"usage\":{\"input_tokens\":3,\"output_tokens\":0}}}\r\n\r\n",
        "event: content_block_start\r\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\r\n\r\n",
        "event: content_block_delta\r\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"partial \"}}\r\n\r\n",
        "event: content_block_delta\r\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"lost\"}}\r\n\r\n",
        "event: message_stop\r\n",
        "data: {\"type\":\"message_stop\"}\r\n\r\n",
    );
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sse))
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new(Some(server.uri()));
    let mut request = callbacks_request();
    request.callbacks = RequestCallbacks::new().with_on_raw(std::sync::Arc::new(|event| {
        if event["type"] == "content_block_delta" && event["delta"]["text"] == "lost" {
            return Err(opi_ai::callbacks::CallbackError::new("raw observer failed"));
        }
        Ok(())
    }));

    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let mut saw_delta = false;
    let mut typed_failure = false;
    while let Some(item) = stream.next().await {
        match item {
            Ok(AssistantStreamEvent::TextDelta { .. }) => saw_delta = true,
            Err(ProviderError::StreamFailure(failure))
                if matches!(failure.cause(), ProviderError::CallbackPhase(_)) =>
            {
                typed_failure = true;
                break;
            }
            Err(_) => break,
            _ => {}
        }
    }
    assert!(saw_delta, "events before the raw failure stay delivered");
    assert!(
        typed_failure,
        "raw failure ends the call typed with the partial preserved"
    );
}

async fn stream_prepared_and_drain(provider: &AnthropicProvider, request: Request) {
    use opi_ai::provider::Provider;
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    while let Some(item) = stream.next().await {
        if let Err(error) = item {
            panic!("unexpected error: {error}");
        }
    }
}

// --- task 20.11: constrained sampling substrate (V-34/AIR-040) ---

mod constrained_sampling_tests {
    use opi_ai::constrained_sampling::{
        AnthropicStrictKeywordAdmission, ConstrainedSampling, GrammarInputBuffer, StrictPreference,
        append_grammar_tool_input_delta, make_strict_json_schema, normalize_wire_arguments_nulls,
        resolve_grammar_constrained_sampling, resolve_json_schema_strict_sampling,
    };
    use opi_ai::message::ToolDef;
    use serde_json::json;

    fn object_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "required_str": {"type": "string"},
                "optional_num": {"type": "number"},
                "nullable_opt": {"type": ["string", "null"]}
            },
            "required": ["required_str"]
        })
    }

    fn strict_tool(strict: StrictPreference) -> ToolDef {
        ToolDef {
            name: "tool".into(),
            description: "d".into(),
            input_schema: object_schema(),
            constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict }),
        }
    }

    #[test]
    fn strict_conversion_clones_and_forces_required_nullable() {
        let original = object_schema();
        let converted = make_strict_json_schema(&original, &AnthropicStrictKeywordAdmission)
            .expect("plain object schema converts");
        assert_eq!(
            original,
            object_schema(),
            "the trusted schema is never mutated"
        );
        assert_eq!(converted["additionalProperties"], json!(false));
        assert_eq!(
            converted["required"],
            json!(["required_str", "optional_num", "nullable_opt"]),
            "every property becomes required"
        );
        assert_eq!(
            converted["properties"]["optional_num"],
            json!({"anyOf": [{"type": "number"}, {"type": "null"}]}),
            "optional non-nullable properties are wrapped nullable"
        );
        assert_eq!(
            converted["properties"]["nullable_opt"],
            json!({"type": ["string", "null"]}),
            "already-nullable properties are not double-wrapped"
        );
        assert_eq!(
            converted["properties"]["required_str"],
            json!({"type": "string"}),
            "required properties are unchanged"
        );
    }

    #[test]
    fn strict_conversion_rejects_unsupported_shapes() {
        for schema in [
            json!({"type": "object", "$ref": "#/x"}),
            json!({"type": "object", "oneOf": [{"type": "object"}]}),
            json!({"type": "array", "items": [{"type": "string"}]}),
            json!(true),
            json!({"properties": {"a": {"type": "string"}}}),
            json!({"type": "object", "additionalProperties": true, "properties": {}}),
            json!({"type": "object", "properties": {"a": {"type": "string"}}, "required": ["b"]}),
            json!({"type": "object", "anyOf": [{"type": "object"}]}),
            json!({"type": "object", "properties": {"a": {"anyOf": []}}}),
            json!({"type": "string"}),
        ] {
            assert!(
                make_strict_json_schema(&schema, &AnthropicStrictKeywordAdmission).is_err(),
                "schema {schema} must be rejected for strict wire projection"
            );
        }
    }

    #[test]
    fn anthropic_keyword_admission_rejects_numeric_and_unknown_formats() {
        // The Anthropic strict subset rejects these keywords with any value.
        for schema in [
            json!({"type": "object", "properties": {"a": {"type": "integer", "minimum": 1}}}),
            json!({"type": "object", "properties": {"a": {"type": "array", "maxItems": 2}}}),
            json!({"type": "object", "properties": {"a": {"type": "string", "format": "semver"}}}),
            json!({"type": "object", "properties": {"a": {"type": "array", "minItems": 2}}}),
        ] {
            assert!(
                make_strict_json_schema(&schema, &AnthropicStrictKeywordAdmission).is_err(),
                "schema {schema} hits an Anthropic-rejected keyword"
            );
        }
        // minItems 0/1 and the documented string formats stay admissible.
        for schema in [
            json!({"type": "object", "properties": {"a": {"type": "array", "items": {"type": "string"}, "minItems": 0}}}),
            json!({"type": "object", "properties": {"a": {"type": "string", "format": "uuid"}}}),
        ] {
            assert!(
                make_strict_json_schema(&schema, &AnthropicStrictKeywordAdmission).is_ok(),
                "schema {schema} is inside the Anthropic strict subset"
            );
        }
    }

    #[test]
    fn prefer_falls_back_non_strict_require_fails_typed() {
        // Wire without strict support: prefer projects non-strict.
        assert_eq!(
            resolve_json_schema_strict_sampling(
                &strict_tool(StrictPreference::Prefer),
                false,
                &AnthropicStrictKeywordAdmission
            )
            .expect("prefer never fails on unsupported wires"),
            None
        );
        // ...but require fails before dispatch.
        let require_unsupported = resolve_json_schema_strict_sampling(
            &strict_tool(StrictPreference::Require),
            false,
            &AnthropicStrictKeywordAdmission,
        );
        assert!(require_unsupported.is_err(), "require must fail typed");

        // Wire with strict support: both prefer and require resolve strict.
        for preference in [StrictPreference::Prefer, StrictPreference::Require] {
            assert_eq!(
                resolve_json_schema_strict_sampling(
                    &strict_tool(preference),
                    true,
                    &AnthropicStrictKeywordAdmission
                )
                .unwrap(),
                Some(true)
            );
        }
        // A schema the wire cannot convert: prefer falls back, require fails.
        let mut unsupported = strict_tool(StrictPreference::Prefer);
        unsupported.input_schema =
            json!({"type": "object", "properties": {"a": {"type": "integer", "minimum": 1}}});
        assert_eq!(
            resolve_json_schema_strict_sampling(
                &unsupported,
                true,
                &AnthropicStrictKeywordAdmission
            )
            .unwrap(),
            None
        );
        unsupported.constrained_sampling = Some(ConstrainedSampling::JsonSchema {
            strict: StrictPreference::Require,
        });
        assert!(
            resolve_json_schema_strict_sampling(
                &unsupported,
                true,
                &AnthropicStrictKeywordAdmission
            )
            .is_err()
        );
        // A tool without constrained sampling is never strict.
        let mut plain = strict_tool(StrictPreference::Prefer);
        plain.constrained_sampling = None;
        assert_eq!(
            resolve_json_schema_strict_sampling(&plain, true, &AnthropicStrictKeywordAdmission)
                .unwrap(),
            None
        );
    }

    fn grammar_tool(variants: serde_json::Value) -> ToolDef {
        ToolDef {
            name: "grammar-tool".into(),
            description: "d".into(),
            input_schema: json!({
                "type": "object",
                "properties": {"input": {"type": "string"}},
                "required": ["input"]
            }),
            constrained_sampling: Some(ConstrainedSampling::Grammar {
                openai_lark: variants
                    .get("lark")
                    .and_then(|v| v.as_str().map(String::from)),
                openai_regex: variants
                    .get("regex")
                    .and_then(|v| v.as_str().map(String::from)),
            }),
        }
    }

    #[test]
    fn grammar_resolution_prefers_lark_over_regex() {
        let both = grammar_tool(json!({"lark": "start: WORD", "regex": "^a+$"}));
        let grammar = resolve_grammar_constrained_sampling(&both, true)
            .expect("valid grammar tool resolves")
            .expect("supported target yields a grammar");
        assert_eq!(grammar.format, "lark");
        assert_eq!(grammar.definition, "start: WORD");
        assert_eq!(grammar.input_property, "input");

        let regex_only = grammar_tool(json!({"regex": "^a+$"}));
        let grammar = resolve_grammar_constrained_sampling(&regex_only, true)
            .unwrap()
            .expect("regex-only grammar resolves");
        assert_eq!(grammar.format, "regex");
    }

    #[test]
    fn grammar_unsupported_target_falls_back_to_plain_json() {
        let both = grammar_tool(json!({"lark": "start: WORD"}));
        // Unsupported target: plain-JSON projection, no failure.
        assert!(
            resolve_grammar_constrained_sampling(&both, false)
                .expect("unsupported target never fails")
                .is_none()
        );
    }

    #[test]
    fn grammar_supported_target_without_variant_or_schema_fails() {
        for tool in [
            grammar_tool(json!({})),
            grammar_tool(json!({"lark": "   "})),
        ] {
            assert!(
                resolve_grammar_constrained_sampling(&tool, true).is_err(),
                "supported target with no valid variant fails before dispatch"
            );
        }
        // Schema shape violations fail with details.
        let mut bad_schema = grammar_tool(json!({"lark": "start: A"}));
        bad_schema.input_schema = json!({"type": "object", "properties": {"a": {"type": "string"}}, "required": ["a", "b"]});
        assert!(resolve_grammar_constrained_sampling(&bad_schema, true).is_err());
        bad_schema.input_schema =
            json!({"type": "object", "properties": {"a": {"type": "number"}}, "required": ["a"]});
        assert!(resolve_grammar_constrained_sampling(&bad_schema, true).is_err());
    }

    #[test]
    fn grammar_input_buffer_wraps_increments_and_closes() {
        let mut buffer = GrammarInputBuffer::new("input".into(), String::new());
        let first = append_grammar_tool_input_delta(&mut buffer, "hel", false)
            .expect("first delta appends");
        assert_eq!(first, Some(r#"{"input":"hel"#.into()));
        let next = append_grammar_tool_input_delta(&mut buffer, "hello", false)
            .expect("monotonic delta appends");
        assert_eq!(next, Some(r#"lo"#.into()));
        // Empty non-closing delta produces nothing.
        assert_eq!(
            append_grammar_tool_input_delta(&mut buffer, "hello", false).unwrap(),
            None
        );
        let closed = append_grammar_tool_input_delta(&mut buffer, "hello!", true)
            .expect("close wraps the object");
        assert_eq!(closed, Some(r#"!""#.to_string() + "}"));

        // Non-monotonic input fails typed.
        let mut buffer = GrammarInputBuffer::new("input".into(), String::new());
        append_grammar_tool_input_delta(&mut buffer, "abc", false).unwrap();
        assert!(append_grammar_tool_input_delta(&mut buffer, "abd", false).is_err());
        // Changed-after-close fails typed.
        let mut buffer = GrammarInputBuffer::new("input".into(), String::new());
        append_grammar_tool_input_delta(&mut buffer, "abc", true).unwrap();
        assert!(append_grammar_tool_input_delta(&mut buffer, "abcd", true).is_err());
    }

    #[test]
    fn wire_null_arguments_normalize_per_original_schema() {
        let original = object_schema();
        let mut arguments = json!({
            "required_str": "x",
            "optional_num": null,
            "nullable_opt": null
        });
        normalize_wire_arguments_nulls(&original, &mut arguments);
        assert_eq!(
            arguments,
            json!({"required_str": "x", "nullable_opt": null}),
            "nulls for originally optional non-nullable properties are dropped; \
             explicitly nullable positions keep their null"
        );
    }
}

// --- task 20.11: Anthropic strict wire projection (V-34/AIR-040) ---

mod anthropic_strict_wire_tests {
    use super::*;
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    use opi_ai::message::ToolDef;
    use opi_ai::model_info::{
        AnthropicMessagesCompat, ModelCapabilities, ModelInfo, WireApi, WireCompat,
    };
    use opi_ai::transcript::Context;
    use std::sync::Arc;

    fn strict_tool(name: &str, strict: StrictPreference, schema: serde_json::Value) -> ToolDef {
        ToolDef {
            name: name.into(),
            description: "d".into(),
            input_schema: schema,
            constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict }),
        }
    }

    fn plain_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "limit": {"type": "number"}
            },
            "required": ["path"]
        })
    }

    fn strict_capable_provider(base: Option<String>, strict_tools: bool) -> AnthropicProvider {
        let model = ModelInfo::new(
            "strict-model",
            "Strict Model",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(1000, 100),
        )
        .with_compat(WireCompat::AnthropicMessages(AnthropicMessagesCompat {
            supports_eager_tool_input_streaming: false,
            force_adaptive_thinking: false,
            supports_temperature: true,
            supports_strict_tools: strict_tools,
            supports_mid_convo_system_messages: false,
            supports_mid_convo_tool_changes: false,
        }))
        .unwrap();
        AnthropicProvider::for_route(
            "test".into(),
            vec![model],
            base,
            Default::default(),
            Arc::new(opi_ai::http::HttpClient::new()),
            false,
        )
    }

    fn request_with_tools(tools: Vec<ToolDef>) -> Request {
        Request {
            transport: Default::default(),
            model: "test:strict-model".into(),
            context: opi_ai::transcript::normalize_context(Context {
                system_prompt: None,
                messages: vec![Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "Hello".into(),
                    }],
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

    fn tool_def_in_body(
        provider: &AnthropicProvider,
        request: &Request,
        index: usize,
    ) -> serde_json::Value {
        let body = provider.build_request_body(request).expect("body builds");
        body["tools"][index].clone()
    }

    #[test]
    fn strict_prefer_and_require_project_converted_wire_schema() {
        let provider = strict_capable_provider(None, true);
        let prefer_tool = tool_def_in_body(
            &provider,
            &request_with_tools(vec![strict_tool(
                "prefer-tool",
                StrictPreference::Prefer,
                plain_schema(),
            )]),
            0,
        );
        assert_eq!(prefer_tool["strict"], serde_json::json!(true));
        assert_eq!(
            prefer_tool["input_schema"]["additionalProperties"],
            serde_json::json!(false)
        );
        assert_eq!(
            prefer_tool["input_schema"]["required"],
            serde_json::json!(["path", "limit"])
        );
        assert_eq!(
            prefer_tool["input_schema"]["properties"]["limit"],
            serde_json::json!({"anyOf": [{"type": "number"}, {"type": "null"}]})
        );

        let require_tool = tool_def_in_body(
            &provider,
            &request_with_tools(vec![strict_tool(
                "require-tool",
                StrictPreference::Require,
                plain_schema(),
            )]),
            0,
        );
        assert_eq!(require_tool["strict"], serde_json::json!(true));

        let plain_tool = tool_def_in_body(
            &provider,
            &request_with_tools(vec![ToolDef {
                name: "plain-tool".into(),
                description: "d".into(),
                input_schema: plain_schema(),
                constrained_sampling: None,
            }]),
            0,
        );
        assert!(
            plain_tool.get("strict").is_none(),
            "tools without constrained sampling never carry strict"
        );
        assert_eq!(
            plain_tool["input_schema"],
            plain_schema(),
            "the trusted schema projects unchanged for non-strict tools"
        );
    }

    #[test]
    fn strict_prefer_unconvertible_schema_stays_non_strict_on_the_wire() {
        let provider = strict_capable_provider(None, true);
        let unconvertible = serde_json::json!({
            "type": "object",
            "properties": {"path": {"type": "string", "minLength": 1}}
        });
        let _ = unconvertible;
        let unconvertible = serde_json::json!({
            "type": "object",
            "properties": {"n": {"type": "integer", "minimum": 1}}
        });
        let tool = tool_def_in_body(
            &provider,
            &request_with_tools(vec![strict_tool(
                "prefer-tool",
                StrictPreference::Prefer,
                unconvertible.clone(),
            )]),
            0,
        );
        assert!(tool.get("strict").is_none());
        assert_eq!(tool["input_schema"], unconvertible);
    }

    #[test]
    fn strict_require_fails_typed_before_any_dispatch_when_unconvertible() {
        let provider = strict_capable_provider(None, true);
        let unconvertible = serde_json::json!({
            "type": "object",
            "properties": {"n": {"type": "integer", "minimum": 1}}
        });
        let error = provider
            .build_request_body(&request_with_tools(vec![strict_tool(
                "require-tool",
                StrictPreference::Require,
                unconvertible,
            )]))
            .expect_err("require with an unconvertible schema must fail");
        assert!(
            matches!(error, ProviderError::Config(_)),
            "typed pre-dispatch failure, got {error:?}"
        );
    }

    #[tokio::test]
    async fn strict_require_fails_before_http_on_strict_unsupported_wire() {
        let server = MockServer::start().await;
        let provider = strict_capable_provider(Some(server.uri()), false);
        let error = provider
            .stream_prepared(
                request_with_tools(vec![strict_tool(
                    "require-tool",
                    StrictPreference::Require,
                    plain_schema(),
                )]),
                opi_ai::test_support::resolved_auth(),
            )
            .next()
            .await
            .expect("result")
            .expect_err("require on a strict-unsupported wire fails");
        assert!(
            matches!(error, ProviderError::Config(_)),
            "typed pre-dispatch failure, got {error:?}"
        );
        assert!(
            server.received_requests().await.unwrap().is_empty(),
            "zero HTTP dispatch for an unmet require"
        );
    }
}

// --- task 20.11: the single source-qualified Anthropic strict downgrade
// variant (V-34/V-36/AIR-042/P20-S11) ---

mod anthropic_strict_variant_tests {
    use super::*;
    use opi_ai::auth::{AuthProvenanceSource, StaticAuthResolver};
    use opi_ai::callbacks::RequestCallbacks;
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    use opi_ai::http::HttpClientBuilder;
    use opi_ai::message::ToolDef;
    use opi_ai::model_info::{
        AnthropicMessagesCompat, ModelCapabilities, ModelInfo, WireApi, WireCompat,
    };
    use opi_ai::models::Models;
    use opi_ai::retry::AcquisitionPolicy;
    use opi_ai::stream::ChatStream;
    use opi_ai::transcript::Context;
    use secrecy::SecretString;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn strict_tool(name: &str, strict: StrictPreference) -> ToolDef {
        ToolDef {
            name: name.into(),
            description: "d".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"]
            }),
            constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict }),
        }
    }

    fn anthropic_sse_ok() -> String {
        text_fixture().to_owned()
    }

    async fn strict_variant_setup(
        server: &MockServer,
        first_status: u16,
        first_body: String,
    ) -> Models {
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(first_status).set_body_string(first_body))
            .up_to_n_times(1)
            .mount(server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_string(anthropic_sse_ok()))
            .mount(server)
            .await;
        strict_models(server.uri())
    }

    fn strict_models(base_url: String) -> Models {
        strict_models_with_inline(base_url, false)
    }

    fn strict_models_with_inline(base_url: String, inline: bool) -> Models {
        let model = ModelInfo::new(
            "strict-model",
            "Strict Model",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(1000, 100),
        )
        .with_compat(WireCompat::AnthropicMessages(AnthropicMessagesCompat {
            supports_eager_tool_input_streaming: false,
            force_adaptive_thinking: false,
            supports_temperature: true,
            supports_strict_tools: true,
            supports_mid_convo_system_messages: inline,
            supports_mid_convo_tool_changes: inline,
        }))
        .unwrap();
        let client = Arc::new(HttpClientBuilder::new().build().expect("client build"));
        let provider = AnthropicProvider::for_route(
            "test".into(),
            vec![model],
            Some(base_url),
            Default::default(),
            client,
            false,
        );
        let mut models = Models::new();
        models
            .register_route(
                Box::new(provider),
                Arc::new(StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::ApiKey,
                    SecretString::from("test-key"),
                )),
                AuthProvenanceSource::Static,
                opi_ai::models::CompatMetadata {
                    openai_compatible: false,
                    profile: None,
                },
            )
            .expect("register route");
        models
    }

    fn variant_request(tools: Vec<ToolDef>, callbacks: RequestCallbacks) -> Request {
        Request {
            transport: Default::default(),
            model: "test:strict-model".into(),
            context: opi_ai::transcript::normalize_context(Context {
                system_prompt: None,
                messages: vec![Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "Hello".into(),
                    }],
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
            callbacks,
        }
    }

    fn strict_limits_400() -> String {
        r#"{"type":"error","error":{"type":"invalid_request_error","message":"Too many strict tools (65). Reduce the number of strict tools or convert some to non-strict."}}"#.into()
    }

    async fn wait_retry(admission: opi_ai::retry::RetryAdmission) {
        tokio::time::sleep(std::time::Duration::from_millis(admission.delay_ms.max(1))).await;
    }

    async fn ordinary_strict_variant_contract(direct: bool) {
        let server = MockServer::start().await;
        let models = strict_variant_setup(&server, 400, strict_limits_400()).await;
        let payload_calls = Arc::new(AtomicUsize::new(0));
        let payload_probe = payload_calls.clone();
        let header_calls = Arc::new(AtomicUsize::new(0));
        let header_probe = header_calls.clone();
        let responses = Arc::new(std::sync::Mutex::new(Vec::new()));
        let response_probe = responses.clone();
        let request = variant_request(
            vec![
                strict_tool("prefer-tool", StrictPreference::Prefer),
                strict_tool("require-tool", StrictPreference::Require),
            ],
            RequestCallbacks::new()
                .with_on_payload(Arc::new(move |_| {
                    payload_probe.fetch_add(1, Ordering::SeqCst);
                    Ok(None)
                }))
                .with_header_transform(Arc::new(move |headers| {
                    header_probe.fetch_add(1, Ordering::SeqCst);
                    Ok(headers.to_vec())
                }))
                .with_on_response(Arc::new(move |status, _| {
                    response_probe.lock().unwrap().push(status);
                    Ok(())
                })),
        );
        let message = if direct {
            let (provider, _) = models.resolve("test:strict-model").unwrap();
            ChatStream::new(
                provider.stream_prepared(request, opi_ai::test_support::resolved_auth()),
            )
            .result()
            .await
        } else {
            models
                .complete_simple(
                    opi_ai::ChatModelRecord::new(
                        "test",
                        models.get_model("test", "strict-model").unwrap(),
                    ),
                    request,
                    Default::default(),
                )
                .await
        }
        .expect("ordinary entry drives its single admitted strict variant");
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 2);
        let first: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
        let second: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
        assert_eq!(first["tools"][0]["strict"], true);
        assert!(second["tools"][0].get("strict").is_none());
        assert_eq!(second["tools"][1]["strict"], true);
        assert_eq!(first["model"], second["model"]);
        assert_eq!(sent[0].headers["x-api-key"], sent[1].headers["x-api-key"]);
        assert_eq!(payload_calls.load(Ordering::SeqCst), 2);
        assert_eq!(header_calls.load(Ordering::SeqCst), 1);
        assert_eq!(*responses.lock().unwrap(), vec![200]);
        assert_eq!(
            message
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.kind == "anthropic_strict_tools_fallback")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn ordinary_models_drives_the_single_strict_variant() {
        ordinary_strict_variant_contract(false).await;
    }

    #[tokio::test]
    async fn inline_preferred_tools_participate_in_the_single_strict_variant() {
        for initial_preferred in [false, true] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/v1/messages"))
                .respond_with(ResponseTemplate::new(400).set_body_string(strict_limits_400()))
                .up_to_n_times(1)
                .mount(&server)
                .await;
            Mock::given(method("POST"))
                .and(path("/v1/messages"))
                .respond_with(ResponseTemplate::new(200).set_body_string(anthropic_sse_ok()))
                .mount(&server)
                .await;
            let models = strict_models_with_inline(server.uri(), true);
            let mut initial = strict_tool("initial", StrictPreference::Prefer);
            if !initial_preferred {
                initial.constrained_sampling = None;
            }
            let payload_calls = Arc::new(AtomicUsize::new(0));
            let observed = payload_calls.clone();
            let mut request = variant_request(
                vec![],
                RequestCallbacks::new().with_on_payload(Arc::new(move |_| {
                    observed.fetch_add(1, Ordering::SeqCst);
                    Ok(None)
                })),
            );
            request.context = opi_ai::transcript::normalize_context(Context {
                messages: vec![
                    Message::System(opi_ai::transcript::SystemMessage {
                        tools_added: vec![initial],
                        ..Default::default()
                    }),
                    Message::User(UserMessage {
                        content: vec![InputContent::Text { text: "go".into() }],
                        timestamp_ms: 0,
                    }),
                    Message::System(opi_ai::transcript::SystemMessage {
                        tools_added: vec![
                            ToolDef {
                                description: "inline replacement".into(),
                                ..strict_tool("initial", StrictPreference::Prefer)
                            },
                            strict_tool("later-require", StrictPreference::Require),
                        ],
                        ..Default::default()
                    }),
                ],
                ..Default::default()
            })
            .unwrap();
            models
                .complete_simple(
                    opi_ai::ChatModelRecord::new(
                        "test",
                        models.get_model("test", "strict-model").unwrap(),
                    ),
                    request,
                    Default::default(),
                )
                .await
                .expect("inline Prefer admits the strict variant");
            let sent = server.received_requests().await.unwrap();
            assert_eq!(sent.len(), 2);
            assert_eq!(payload_calls.load(Ordering::SeqCst), 2);
            for (index, request) in sent.iter().enumerate() {
                let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
                let definitions: Vec<_> = body["messages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .flat_map(|message| message["content"].as_array().into_iter().flatten())
                    .filter(|block| block["type"] == "tool_addition")
                    .map(|block| &block["tool"]["definition"])
                    .collect();
                assert_eq!(definitions.len(), 2);
                assert_eq!(definitions[0]["description"], "inline replacement");
                assert_eq!(definitions[0].get("strict").is_some(), index == 0);
                assert_eq!(definitions[1]["strict"], true);
            }
        }
    }

    #[tokio::test]
    async fn ordinary_direct_adapter_drives_the_single_strict_variant() {
        ordinary_strict_variant_contract(true).await;
    }

    #[tokio::test]
    async fn successful_headers_seal_acquisition_before_a_body_timeout() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                socket.read_exact(&mut byte).unwrap();
                head.push(byte[0]);
            }
            socket
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\n")
                .unwrap();
            socket.flush().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(250));
        });
        let models = strict_models(format!("http://127.0.0.1:{port}"));
        let mut request = variant_request(vec![], RequestCallbacks::default());
        request.timeout = Some(std::time::Duration::from_millis(50));
        let call = models
            .prepare_call_with_policy(
                "test:strict-model",
                request,
                AcquisitionPolicy::with_max_retries(1),
            )
            .await
            .unwrap();
        let error = ChatStream::new(call.start_attempt().unwrap())
            .result()
            .await
            .expect_err("successful headers precede the body timeout");
        assert!(matches!(error.cause(), ProviderError::Timeout));
        assert!(call.retry_admission().is_none(), "no post-handoff resend");
        assert_eq!(
            call.acquisition_seal(),
            Some(opi_ai::retry::AcquisitionSeal::PostHandoff)
        );
        peer.join().unwrap();
    }

    #[tokio::test]
    async fn strict_marker_400_rebuilds_one_variant_with_prefer_removed() {
        let server = MockServer::start().await;
        let models = strict_variant_setup(&server, 400, strict_limits_400()).await;

        let payload_calls = Arc::new(AtomicUsize::new(0));
        let payload_probe = Arc::clone(&payload_calls);
        let transform_calls = Arc::new(AtomicUsize::new(0));
        let transform_probe = Arc::clone(&transform_calls);
        let response_statuses = Arc::new(std::sync::Mutex::new(Vec::<u16>::new()));
        let response_probe = Arc::clone(&response_statuses);

        let request = variant_request(
            vec![
                strict_tool("prefer-tool", StrictPreference::Prefer),
                strict_tool("require-tool", StrictPreference::Require),
            ],
            RequestCallbacks::new()
                .with_on_payload(Arc::new(move |_body| {
                    payload_probe.fetch_add(1, Ordering::SeqCst);
                    Ok(None)
                }))
                .with_header_transform(Arc::new(move |merged| {
                    transform_probe.fetch_add(1, Ordering::SeqCst);
                    Ok(merged.to_vec())
                }))
                .with_on_response(Arc::new(move |status, _headers| {
                    response_probe.lock().unwrap().push(status);
                    Ok(())
                })),
        );

        let mut policy = AcquisitionPolicy::with_max_retries(0);
        policy.jitter_seed = Some(0);
        let call = models
            .prepare_call_with_policy("test:strict-model", request, policy)
            .await
            .expect("prepare call");

        let first = ChatStream::new(call.start_attempt().expect("first attempt"))
            .result()
            .await
            .expect_err("the strict-limit 400 fails the first variant");
        assert!(
            matches!(first.cause(), ProviderError::StrictToolLimits { .. }),
            "source-qualified strict-limit 400 is typed, got {:?}",
            first.cause()
        );
        // Not an ordinary transport retry: no admission, but one variant.
        assert!(call.retry_admission().is_none());
        let strict_admission = call
            .strict_variant_admission()
            .expect("exactly one downgrade variant is admitted");
        assert_eq!(strict_admission.reason, "strict_tool_limits_exceeded");

        let message = ChatStream::new(call.start_attempt().expect("variant dispatch"))
            .result()
            .await
            .expect("variant completes");
        // The variant admission is consumed.
        assert!(call.strict_variant_admission().is_none());

        let sent = server.received_requests().await.unwrap();
        assert_eq!(
            sent.len(),
            2,
            "N=0 still allows two sends (one per variant)"
        );
        let first_body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
        let second_body: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
        assert_eq!(first_body["tools"][0]["strict"], serde_json::json!(true));
        assert_eq!(first_body["tools"][1]["strict"], serde_json::json!(true));
        assert!(
            second_body["tools"][0].get("strict").is_none(),
            "the rebuilt variant removes only the prefer-strict flag"
        );
        assert_eq!(
            second_body["tools"][1]["strict"],
            serde_json::json!(true),
            "require stays strict in the rebuilt variant"
        );

        assert_eq!(
            payload_calls.load(Ordering::SeqCst),
            2,
            "onPayload re-runs exactly once on the new variant"
        );
        assert_eq!(
            transform_calls.load(Ordering::SeqCst),
            1,
            "the header transform does not re-run on the strict branch"
        );
        assert_eq!(
            *response_statuses.lock().unwrap(),
            vec![200],
            "onResponse fires only on the successfully acquired response"
        );
        assert_eq!(
            message
                .diagnostics
                .iter()
                .filter(|d| d.kind == "anthropic_strict_tools_fallback")
                .count(),
            1,
            "exactly one downgrade diagnostic is recorded on the message"
        );
    }

    #[tokio::test]
    async fn non_marker_400_does_not_branch() {
        let server = MockServer::start().await;
        let models = strict_variant_setup(
            &server,
            400,
            r#"{"type":"error","error":{"type":"invalid_request_error","message":"max_tokens is required"}}"#.into(),
        )
        .await;
        let call = models
            .prepare_call_with_policy(
                "test:strict-model",
                variant_request(
                    vec![strict_tool("prefer-tool", StrictPreference::Prefer)],
                    RequestCallbacks::default(),
                ),
                AcquisitionPolicy::with_max_retries(0),
            )
            .await
            .expect("prepare call");
        let first = ChatStream::new(call.start_attempt().expect("first attempt"))
            .result()
            .await
            .expect_err("plain 400 fails");
        assert!(!matches!(
            first.cause(),
            ProviderError::StrictToolLimits { .. }
        ));
        assert!(call.retry_admission().is_none());
        assert!(
            call.strict_variant_admission().is_none(),
            "other 400s never branch"
        );
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 1);
    }

    #[tokio::test]
    async fn marker_400_without_prefer_strict_tools_does_not_branch() {
        let server = MockServer::start().await;
        let models = strict_variant_setup(&server, 400, strict_limits_400()).await;
        let call = models
            .prepare_call_with_policy(
                "test:strict-model",
                variant_request(
                    vec![strict_tool("require-tool", StrictPreference::Require)],
                    RequestCallbacks::default(),
                ),
                AcquisitionPolicy::with_max_retries(0),
            )
            .await
            .expect("prepare call");
        ChatStream::new(call.start_attempt().expect("first attempt"))
            .result()
            .await
            .expect_err("marker 400 without prefer-strict tools fails");
        assert!(
            call.strict_variant_admission().is_none(),
            "the downgrade is qualified by an actually-dispatched prefer-strict tool"
        );
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 1);
    }

    #[tokio::test]
    async fn second_variant_marker_400_surfaces_without_a_third_dispatch() {
        let server = MockServer::start().await;
        // Both sends answer with the marker 400.
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(400).set_body_string(strict_limits_400()))
            .mount(&server)
            .await;
        let models = strict_models(server.uri());
        let call = models
            .prepare_call_with_policy(
                "test:strict-model",
                variant_request(
                    vec![strict_tool("prefer-tool", StrictPreference::Prefer)],
                    RequestCallbacks::default(),
                ),
                AcquisitionPolicy::with_max_retries(0),
            )
            .await
            .expect("prepare call");
        ChatStream::new(call.start_attempt().expect("first attempt"))
            .result()
            .await
            .expect_err("first variant fails");
        assert!(call.strict_variant_admission().is_some());
        ChatStream::new(call.start_attempt().expect("variant dispatch"))
            .result()
            .await
            .expect_err("the rebuilt variant surfaces the second 400");
        assert!(
            call.strict_variant_admission().is_none(),
            "at most one downgrade variant exists"
        );
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 2);
    }

    #[tokio::test]
    async fn ordinary_retry_budget_is_per_variant_with_total_bound() {
        let server = MockServer::start().await;
        // Variant 1: 429 (retryable), then the marker 400.
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(429))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(400).set_body_string(strict_limits_400()))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        // Variant 2: 429, then success.
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(429))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_string(anthropic_sse_ok()))
            .mount(&server)
            .await;
        let models = strict_models(server.uri());

        let mut policy = AcquisitionPolicy::with_max_retries(1);
        policy.jitter_seed = Some(0);
        let call = models
            .prepare_call_with_policy(
                "test:strict-model",
                variant_request(
                    vec![strict_tool("prefer-tool", StrictPreference::Prefer)],
                    RequestCallbacks::default(),
                ),
                policy,
            )
            .await
            .expect("prepare call");

        ChatStream::new(call.start_attempt().expect("v1 send 1"))
            .result()
            .await
            .expect_err("429");
        wait_retry(call.retry_admission().expect("ordinary retry")).await;
        ChatStream::new(call.start_attempt().expect("v1 send 2"))
            .result()
            .await
            .expect_err("marker 400");
        assert!(call.retry_admission().is_none(), "per-variant budget spent");
        assert!(call.strict_variant_admission().is_some());
        ChatStream::new(call.start_attempt().expect("v2 send 1"))
            .result()
            .await
            .expect_err("429 again");
        wait_retry(call.retry_admission().expect("fresh per-variant budget")).await;
        ChatStream::new(call.start_attempt().expect("v2 send 2"))
            .result()
            .await
            .expect("variant 2 completes within its own 1+N budget");

        let sent = server.received_requests().await.unwrap();
        assert_eq!(
            sent.len(),
            4,
            "total bound 2x(1+N) with N=1: two per variant, no more"
        );
    }

    #[tokio::test]
    async fn callback_phase_failure_never_branches() {
        let server = MockServer::start().await;
        let models = strict_variant_setup(&server, 400, strict_limits_400()).await;
        let callbacks = RequestCallbacks::new().with_on_payload(Arc::new(|_body| {
            Err(opi_ai::callbacks::CallbackError::new("boom"))
        }));
        let call = models
            .prepare_call_with_policy(
                "test:strict-model",
                variant_request(
                    vec![strict_tool("prefer-tool", StrictPreference::Prefer)],
                    callbacks,
                ),
                AcquisitionPolicy::with_max_retries(0),
            )
            .await
            .expect("prepare call");
        let first = ChatStream::new(call.start_attempt().expect("first attempt"))
            .result()
            .await
            .expect_err("callback failure fails the call");
        assert!(matches!(first.cause(), ProviderError::CallbackPhase(_)));
        assert!(call.retry_admission().is_none());
        assert!(
            call.strict_variant_admission().is_none(),
            "callback errors stop dispatch"
        );
        let sent = server.received_requests().await.unwrap();
        assert!(sent.is_empty(), "zero HTTP dispatch for a callback failure");
    }

    #[tokio::test]
    async fn post_payload_wire_without_prefer_strict_does_not_admit_downgrade() {
        let server = MockServer::start().await;
        let models = strict_variant_setup(&server, 400, strict_limits_400()).await;
        let callbacks = RequestCallbacks::new().with_on_payload(Arc::new(|body| {
            let mut body = body.clone();
            for tool in body["tools"].as_array_mut().unwrap() {
                tool.as_object_mut().unwrap().remove("strict");
            }
            Ok(Some(body))
        }));
        let call = models
            .prepare_call_with_policy(
                "test:strict-model",
                variant_request(
                    vec![strict_tool("prefer-tool", StrictPreference::Prefer)],
                    callbacks,
                ),
                AcquisitionPolicy::with_max_retries(0),
            )
            .await
            .unwrap();
        ChatStream::new(call.start_attempt().unwrap())
            .result()
            .await
            .expect_err("source marker without actual strict wire stays final");
        assert!(call.strict_variant_admission().is_none());
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

// ---------------------------------------------------------------------------
// Task 20.39: complete anthropic-messages chat adapter — native options,
// thinking states, inline-tools chronology, error-turn relocation, and the
// public direct adapter. Sources: pi 1.3.0 anthropic-messages.ts,
// transform-messages.ts, compat-schema.ts (annex rows 37/50/165/176/244).
// ---------------------------------------------------------------------------

use opi_ai::anthropic::{
    AnthropicMessagesChatApi, AnthropicMessagesOptions, AnthropicThinkingDisplay,
    AnthropicToolChoice,
};
use opi_ai::http::HttpClient;
use opi_ai::message::ToolDef;
use opi_ai::model_info::{
    AnthropicMessagesCompat, ModelCapabilities, ModelInfo, WireApi, WireCompat,
};
use opi_ai::models::{Models, SimpleOptions};
use opi_ai::telemetry::TelemetryContext;

fn caps_39() -> ModelCapabilities {
    ModelCapabilities::new(200_000, 64_000)
        .with_images(true)
        .with_streaming(true)
        .with_thinking(true)
}

fn compat_39(
    force_adaptive: bool,
    mid_convo_system: bool,
    mid_convo_tools: bool,
) -> AnthropicMessagesCompat {
    AnthropicMessagesCompat {
        supports_eager_tool_input_streaming: false,
        force_adaptive_thinking: force_adaptive,
        supports_temperature: true,
        supports_strict_tools: false,
        supports_mid_convo_system_messages: mid_convo_system,
        supports_mid_convo_tool_changes: mid_convo_tools,
    }
}

fn model_39(id: &str, compat: AnthropicMessagesCompat) -> ModelInfo {
    ModelInfo::new(id, id, WireApi::AnthropicMessages, caps_39())
        .with_compat(WireCompat::AnthropicMessages(compat))
        .expect("matching wire compat")
}

fn provider_39(model: &ModelInfo, options: AnthropicMessagesOptions) -> AnthropicProvider {
    AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model.clone()],
        None,
        Default::default(),
        std::sync::Arc::new(HttpClient::new()),
        false,
    )
    .with_native_options(options)
}

fn native_options_39() -> AnthropicMessagesOptions {
    AnthropicMessagesOptions::default()
}

fn request_39(thinking: ThinkingConfig) -> Request {
    request_39_for("anthropic:claude-sonnet-4-5", thinking)
}

fn request_39_for(model: &str, thinking: ThinkingConfig) -> Request {
    Request {
        transport: Default::default(),
        model: model.into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(4096),
        temperature: None,
        sampling_params: Default::default(),
        thinking,
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

fn body_39(provider: &AnthropicProvider, request: &Request) -> serde_json::Value {
    provider.build_request_body(request).expect("body compiles")
}

#[test]
fn native_enabled_without_budget_uses_source_default_1024_and_summarized() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let mut options = native_options_39();
    options.thinking_enabled = Some(true);
    let provider = provider_39(&model, options);
    let body = body_39(&provider, &request_39(ThinkingConfig::default()));

    let thinking = &body["thinking"];
    assert_eq!(thinking["type"], "enabled");
    assert_eq!(thinking["budget_tokens"], 1024, "source native default");
    assert_eq!(
        thinking["display"], "summarized",
        "enabled thinking defaults to the summarized display"
    );
}

#[test]
fn native_zero_budget_falls_back_to_1024_per_source_or_semantics() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let mut options = native_options_39();
    options.thinking_enabled = Some(true);
    options.thinking_budget_tokens = Some(0);
    let provider = provider_39(&model, options);
    let body = body_39(&provider, &request_39(ThinkingConfig::default()));

    assert_eq!(body["thinking"]["budget_tokens"], 1024);
}

#[test]
fn native_explicit_budget_and_detailed_display_reach_the_wire() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let mut options = native_options_39();
    options.thinking_enabled = Some(true);
    options.thinking_budget_tokens = Some(5000);
    options.thinking_display = Some(AnthropicThinkingDisplay::Detailed);
    let provider = provider_39(&model, options);
    let body = body_39(&provider, &request_39(ThinkingConfig::default()));

    assert_eq!(body["thinking"]["budget_tokens"], 5000);
    assert_eq!(body["thinking"]["display"], "detailed");
}

#[test]
fn native_disabled_projects_typed_disabled_on_reasoning_models() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let mut options = native_options_39();
    options.thinking_enabled = Some(false);
    let provider = provider_39(&model, options);
    let body = body_39(&provider, &request_39(ThinkingConfig::default()));

    assert_eq!(body["thinking"]["type"], "disabled");
    assert!(
        body.get("betas")
            .is_none_or(|b| b
                .as_array()
                .is_none_or(|betas| !betas.iter().any(|beta| beta
                    .as_str()
                    .is_some_and(|beta| beta.starts_with("interleaved"))))),
        "disabled thinking never requests the interleaved beta"
    );
}

#[test]
fn native_omitted_thinking_keeps_neutral_request_path() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let provider = provider_39(&model, native_options_39());
    let request = request_39(ThinkingConfig {
        enabled: true,
        budget_tokens: Some(5000),
        level: opi_ai::ThinkingLevel::High,
    });
    let body = body_39(&provider, &request);

    // Omitted native tri-state: the neutral request drives the wire and the
    // native default does not override an explicit neutral budget.
    assert_eq!(body["thinking"]["budget_tokens"], 5000);
    assert_eq!(body["thinking"]["display"], "summarized");
}

#[test]
fn interleaved_beta_defaults_true_explicit_false_omits_adaptive_never() {
    let numeric = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let mut on = native_options_39();
    on.thinking_enabled = Some(true);
    let body = body_39(
        &provider_39(&numeric, on),
        &request_39(ThinkingConfig::default()),
    );
    let betas = body["betas"].as_array().expect("default interleaved beta");
    assert!(
        betas
            .iter()
            .any(|beta| beta == "interleaved-thinking-2025-05-14")
    );

    let mut off = native_options_39();
    off.thinking_enabled = Some(true);
    off.interleaved_thinking = Some(false);
    let body = body_39(
        &provider_39(&numeric, off),
        &request_39(ThinkingConfig::default()),
    );
    assert!(
        body.get("betas").is_none(),
        "explicit interleaved=false must omit the beta"
    );

    let adaptive = model_39("claude-opus-4-7", compat_39(true, false, false));
    let mut adaptive_on = native_options_39();
    adaptive_on.thinking_enabled = Some(true);
    let body = body_39(
        &provider_39(&adaptive, adaptive_on),
        &request_39_for("anthropic:claude-opus-4-7", ThinkingConfig::default()),
    );
    assert!(
        body.get("betas").is_none(),
        "adaptive thinking models ignore the interleaved beta"
    );
}

#[test]
fn adaptive_native_effort_uses_output_config_and_display_without_budget() {
    let adaptive = model_39("claude-opus-4-7", compat_39(true, false, false));
    let mut options = native_options_39();
    options.thinking_enabled = Some(true);
    options.effort = Some(opi_ai::ThinkingLevel::High);
    let provider = provider_39(&adaptive, options);
    let body = body_39(
        &provider,
        &request_39_for("anthropic:claude-opus-4-7", ThinkingConfig::default()),
    );

    assert_eq!(body["thinking"]["type"], "adaptive");
    assert_eq!(body["thinking"]["display"], "summarized");
    assert_eq!(body["output_config"]["effort"], "high");
    assert!(
        body["thinking"].get("budget_tokens").is_none(),
        "adaptive thinking carries no budget"
    );
}

#[test]
fn adaptive_native_without_effort_omits_output_config() {
    let adaptive = model_39("claude-opus-4-7", compat_39(true, false, false));
    let mut options = native_options_39();
    options.thinking_enabled = Some(true);
    let provider = provider_39(&adaptive, options);
    let body = body_39(
        &provider,
        &request_39_for("anthropic:claude-opus-4-7", ThinkingConfig::default()),
    );

    assert_eq!(body["thinking"]["type"], "adaptive");
    assert!(body.get("output_config").is_none());
}

#[test]
fn native_tool_choice_variants_reach_the_wire() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let request = request_39(ThinkingConfig::default());
    for (choice, expected) in [
        (
            AnthropicToolChoice::Auto,
            serde_json::json!({"type": "auto"}),
        ),
        (AnthropicToolChoice::Any, serde_json::json!({"type": "any"})),
        (
            AnthropicToolChoice::None_,
            serde_json::json!({"type": "none"}),
        ),
        (
            AnthropicToolChoice::Named("get_weather".into()),
            serde_json::json!({"type": "tool", "name": "get_weather"}),
        ),
    ] {
        let mut options = native_options_39();
        options.tool_choice = Some(choice);
        let provider = provider_39(&model, options);
        let body = body_39(&provider, &request);
        assert_eq!(body["tool_choice"], expected);
    }
}

fn tool_def_39(name: &str) -> ToolDef {
    ToolDef {
        name: name.into(),
        description: format!("tool {name}"),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {"x": {"type": "string"}},
            "required": ["x"]
        }),
        constrained_sampling: None,
    }
}

fn inline_tools_context_39() -> opi_ai::transcript::Context {
    opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![
            Message::System(opi_ai::transcript::SystemMessage {
                content: "Base prompt.".into(),
                sections: vec![],
                tools_added: vec![tool_def_39("initial_tool")],
                tools_removed: vec![],
                timestamp_ms: 0,
            }),
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 1,
            }),
            Message::System(opi_ai::transcript::SystemMessage {
                content: "Update.".into(),
                sections: vec![],
                tools_added: vec![
                    tool_def_39("added_tool"),
                    ToolDef {
                        description: "redefined initial tool".into(),
                        ..tool_def_39("initial_tool")
                    },
                ],
                tools_removed: vec![opi_ai::transcript::ToolReference {
                    name: "removed_tool".into(),
                }],
                timestamp_ms: 2,
            }),
        ],
        tools: vec![],
    }
}

#[test]
fn inline_tools_placeholder_anchors_initial_tools_and_declares_the_beta() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, true, true));
    let provider = provider_39(&model, native_options_39());
    let mut request = request_39(ThinkingConfig::default());
    request.context =
        opi_ai::transcript::normalize_context(inline_tools_context_39()).expect("valid");
    let body = body_39(&provider, &request);

    let betas = body["betas"].as_array().expect("inline-tools beta");
    assert!(betas.iter().any(|beta| beta == "inline-tools-2026-09-15"));

    let tools = body["tools"].as_array().expect("tools list");
    assert_eq!(tools.len(), 2, "initial tools plus the placeholder");
    assert_eq!(tools[0]["name"], "initial_tool");
    assert_eq!(tools[1]["name"], "__pi_deferred_placeholder__");
    assert_eq!(tools[1]["defer_loading"], true);
}

#[test]
fn inline_tools_by_value_additions_and_same_name_redefinitions() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, true, true));
    let provider = provider_39(&model, native_options_39());
    let mut request = request_39(ThinkingConfig::default());
    request.context =
        opi_ai::transcript::normalize_context(inline_tools_context_39()).expect("valid");
    let body = body_39(&provider, &request);

    // The update system message carries by-value tool_addition blocks (the
    // redefinition of initial_tool arrives as a fresh definition, no removal)
    // plus a tool_removal reference for the genuinely removed name.
    let system = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|message| message["role"] == "system")
        .expect("mid-convo system message stays native");
    let blocks = system["content"].as_array().unwrap();
    let additions: Vec<&serde_json::Value> = blocks
        .iter()
        .filter(|block| block["type"] == "tool_addition")
        .collect();
    assert_eq!(additions.len(), 2, "added_tool and the redefinition");
    assert_eq!(additions[0]["tool"]["definition"]["name"], "added_tool");
    assert_eq!(
        additions[1]["tool"]["definition"]["name"], "initial_tool",
        "same-name redefinition is a by-value addition"
    );
    assert_eq!(
        additions[1]["tool"]["definition"]["description"],
        "redefined initial tool"
    );
    let removals: Vec<&serde_json::Value> = blocks
        .iter()
        .filter(|block| block["type"] == "tool_removal")
        .collect();
    assert_eq!(removals.len(), 1, "redefined names are not removed");
    assert_eq!(removals[0]["tool"]["type"], "tool_reference");
    assert_eq!(removals[0]["tool"]["name"], "removed_tool");
}

#[test]
fn inline_tools_stay_off_without_the_compat_flags() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let provider = provider_39(&model, native_options_39());
    let mut request = request_39(ThinkingConfig::default());
    request.context =
        opi_ai::transcript::normalize_context(inline_tools_context_39()).expect("valid");
    let body = body_39(&provider, &request);

    assert!(
        body.get("betas").is_none(),
        "no inline-tools beta without the compat flags"
    );
    let tools = body["tools"].as_array().expect("collapsed current list");
    assert_eq!(
        tools.len(),
        2,
        "current tool set: added_tool + redefined initial"
    );
    assert!(
        !tools
            .iter()
            .any(|tool| tool["name"] == "__pi_deferred_placeholder__")
    );
    assert!(
        body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|message| message["role"] != "system"),
        "system changes collapse without native mid-convo support"
    );
}

#[test]
fn error_tool_result_images_follow_all_results_in_one_user_message() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let provider = AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model],
        None,
        Default::default(),
        std::sync::Arc::new(HttpClient::new()),
        false,
    );
    let png = opi_ai::message::ImageSource::Base64 {
        data: "aWNn".into(),
    };
    let messages = vec![
        Message::User(UserMessage {
            content: vec![InputContent::Text { text: "q".into() }],
            timestamp_ms: 0,
        }),
        Message::Assistant(opi_ai::message::AssistantMessage {
            content: vec![
                opi_ai::message::AssistantContent::ToolCall {
                    tool_call: opi_ai::message::ToolCall {
                        id: "call_1".into(),
                        name: "probe".into(),
                        arguments: "{}".into(),
                        arguments_error: None,
                        signature: None,
                    },
                },
                opi_ai::message::AssistantContent::ToolCall {
                    tool_call: opi_ai::message::ToolCall {
                        id: "call_2".into(),
                        name: "broken".into(),
                        arguments: "{}".into(),
                        arguments_error: None,
                        signature: None,
                    },
                },
            ],
            api: opi_ai::ApiKind::Anthropic,
            deferred: None,
            provider: "anthropic".into(),
            model: "claude-sonnet-4-5".into(),
            response_model: None,
            wire: Some(WireApi::AnthropicMessages),
            response_id: None,
            usage: Default::default(),
            stop_reason: StopReason::ToolUse,
            error_message: None,
            timestamp_ms: 0,
            diagnostics: vec![],
            duration_ms: None,
            provider_thinking_level: None,
        }),
        Message::ToolResult(opi_ai::message::ToolResultMessage {
            tool_call_id: "call_1".into(),
            tool_name: "probe".into(),
            content: vec![opi_ai::message::OutputContent::Text { text: "ok".into() }],
            details: None,
            is_error: false,
            not_executed: None,
            truncated: false,
            timestamp_ms: 1,
        }),
        Message::ToolResult(opi_ai::message::ToolResultMessage {
            tool_call_id: "call_2".into(),
            tool_name: "broken".into(),
            content: vec![
                opi_ai::message::OutputContent::Text {
                    text: "boom ".into(),
                },
                opi_ai::message::OutputContent::Image {
                    source: png,
                    media_type: opi_ai::message::MediaType::Png,
                },
            ],
            details: None,
            is_error: true,
            not_executed: None,
            truncated: false,
            timestamp_ms: 2,
        }),
    ];
    let mut request = request_39(ThinkingConfig::default());
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages,
        tools: vec![],
    })
    .expect("valid");
    let body = body_39(&provider, &request);

    // One user message carries both tool results, then the relocated error
    // images (caption first) in the same message (pinned #10794 shape).
    let user_messages: Vec<&serde_json::Value> = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|message| message["role"] == "user")
        .collect();
    let tool_turn = user_messages
        .iter()
        .find(|message| {
            message["content"]
                .as_array()
                .is_some_and(|blocks| blocks.iter().any(|block| block["type"] == "tool_result"))
        })
        .expect("tool turn");
    let blocks = tool_turn["content"].as_array().unwrap();
    assert_eq!(
        blocks.len(),
        4,
        "two tool results, a caption, and the image in one user message"
    );
    assert_eq!(blocks[0]["tool_use_id"], "call_1");
    assert_eq!(blocks[1]["tool_use_id"], "call_2");
    assert_eq!(blocks[1]["is_error"], true);
    assert_eq!(
        blocks[1]["content"],
        serde_json::json!([
            {"type": "text", "text": "boom "},
            {"type": "text", "text": "(image attached after the tool results)"}
        ]),
        "error text is preserved and images become a placeholder"
    );
    assert_eq!(blocks[2]["type"], "text");
    assert_eq!(
        blocks[2]["text"],
        "Images from the error result of tool call call_2 (broken):"
    );
    assert_eq!(blocks[3]["type"], "image");
}

#[test]
fn consecutive_success_tool_results_group_into_one_user_message() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let provider = AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model],
        None,
        Default::default(),
        std::sync::Arc::new(HttpClient::new()),
        false,
    );
    let messages = vec![
        Message::User(UserMessage {
            content: vec![InputContent::Text { text: "q".into() }],
            timestamp_ms: 0,
        }),
        Message::Assistant(opi_ai::message::AssistantMessage {
            content: vec![
                opi_ai::message::AssistantContent::ToolCall {
                    tool_call: opi_ai::message::ToolCall {
                        id: "call_1".into(),
                        name: "probe".into(),
                        arguments: "{}".into(),
                        arguments_error: None,
                        signature: None,
                    },
                },
                opi_ai::message::AssistantContent::ToolCall {
                    tool_call: opi_ai::message::ToolCall {
                        id: "call_2".into(),
                        name: "probe".into(),
                        arguments: "{}".into(),
                        arguments_error: None,
                        signature: None,
                    },
                },
            ],
            api: opi_ai::ApiKind::Anthropic,
            deferred: None,
            provider: "anthropic".into(),
            model: "claude-sonnet-4-5".into(),
            response_model: None,
            wire: Some(WireApi::AnthropicMessages),
            response_id: None,
            usage: Default::default(),
            stop_reason: StopReason::ToolUse,
            error_message: None,
            timestamp_ms: 0,
            diagnostics: vec![],
            duration_ms: None,
            provider_thinking_level: None,
        }),
        Message::ToolResult(opi_ai::message::ToolResultMessage {
            tool_call_id: "call_1".into(),
            tool_name: "probe".into(),
            content: vec![opi_ai::message::OutputContent::Text { text: "a".into() }],
            details: None,
            is_error: false,
            not_executed: None,
            truncated: false,
            timestamp_ms: 1,
        }),
        Message::ToolResult(opi_ai::message::ToolResultMessage {
            tool_call_id: "call_2".into(),
            tool_name: "probe".into(),
            content: vec![opi_ai::message::OutputContent::Text { text: "b".into() }],
            details: None,
            is_error: false,
            not_executed: None,
            truncated: false,
            timestamp_ms: 2,
        }),
    ];
    let mut request = request_39(ThinkingConfig::default());
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages,
        tools: vec![],
    })
    .expect("valid");
    let body = body_39(&provider, &request);

    let user_messages: Vec<&serde_json::Value> = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|message| message["role"] == "user")
        .collect();
    let tool_blocks: Vec<&serde_json::Value> = user_messages
        .iter()
        .flat_map(|message| message["content"].as_array().into_iter().flatten())
        .filter(|block| block["type"] == "tool_result")
        .collect();
    assert_eq!(
        user_messages
            .iter()
            .filter(|message| message["content"]
                .as_array()
                .is_some_and(|blocks| blocks.iter().any(|b| b["type"] == "tool_result")))
            .count(),
        1,
        "consecutive tool results share one user message"
    );
    assert_eq!(tool_blocks.len(), 2);
}

#[tokio::test]
async fn native_options_from_simple_map_neutral_tool_choice_only() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(body_partial_json(serde_json::json!({
            "tool_choice": {"type": "none"},
            "thinking": {
                "type": "enabled",
                "budget_tokens": 8192,
                "display": "summarized"
            }
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let model =
        model_39("claude-sonnet-4-5", compat_39(false, false, false)).with_base_url(server.uri());
    let provider = std::sync::Arc::new(AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model.clone()],
        None,
        Default::default(),
        std::sync::Arc::new(HttpClient::new()),
        false,
    ));
    let api = AnthropicMessagesChatApi::new(provider, model.clone()).expect("valid record");
    let mut models = Models::new();
    models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                secrecy::SecretString::from("test-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register");

    // The neutral entry maps only the tool choice; the neutral level-budget
    // policy (8192 for medium) stays distinct from the native no-budget
    // default 1024.
    let mut request = request_39(ThinkingConfig {
        enabled: true,
        budget_tokens: None,
        level: opi_ai::ThinkingLevel::Medium,
    });
    // No explicit output cap: the compiler uses the model cap, so the
    // medium level budget (8192) survives the answer-room clamp.
    request.max_tokens = None;
    let message = models
        .complete_simple(
            opi_ai::models::ChatModelRecord::new("anthropic", model),
            request,
            SimpleOptions {
                tool_choice: Some(opi_ai::models::SimpleToolChoice::None),
                ..Default::default()
            },
        )
        .await
        .expect("simple entry completes");
    assert_eq!(message.stop_reason, StopReason::Stop);
}

#[tokio::test]
async fn anthropic_direct_adapter_native_lifecycle_reaches_the_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key"))
        .and(body_partial_json(serde_json::json!({
            "thinking": {
                "type": "enabled",
                "budget_tokens": 1024,
                "display": "summarized"
            }
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let model =
        model_39("claude-sonnet-4-5", compat_39(false, false, false)).with_base_url(server.uri());
    let provider = std::sync::Arc::new(AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model.clone()],
        None,
        Default::default(),
        std::sync::Arc::new(HttpClient::new()),
        false,
    ));
    let mut options = native_options_39();
    options.thinking_enabled = Some(true);
    let api = AnthropicMessagesChatApi::new(provider, model.clone()).expect("valid record");
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                secrecy::SecretString::from("test-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register");

    let options = handle.options(options);
    let message = models
        .complete(
            opi_ai::models::ChatModelRecord::new("anthropic", model),
            request_39(ThinkingConfig::default()),
            options,
        )
        .await
        .expect("native lifecycle completes");
    assert_eq!(message.stop_reason, StopReason::Stop);
}

async fn native_defaults_body(thinking_enabled: bool) -> serde_json::Value {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_string(text_fixture()))
        .expect(1)
        .mount(&server)
        .await;
    let mut model =
        model_39("claude-sonnet-4-5", compat_39(false, false, false)).with_base_url(server.uri());
    model.capabilities.max_output_tokens = 12_345;
    let provider = std::sync::Arc::new(AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model.clone()],
        None,
        Default::default(),
        std::sync::Arc::new(HttpClient::new()),
        false,
    ));
    let api = AnthropicMessagesChatApi::new(provider, model.clone()).unwrap();
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                secrecy::SecretString::from("offline-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .unwrap();
    let mut request = request_39(ThinkingConfig::default());
    request.max_tokens = None;
    request.temperature = Some(0.3);
    let mut options = native_options_39();
    options.thinking_enabled = Some(thinking_enabled);
    models
        .complete(
            opi_ai::models::ChatModelRecord::new("anthropic", model),
            request,
            handle.options(options),
        )
        .await
        .unwrap();
    let sent = server.received_requests().await.unwrap();
    server.verify().await;
    serde_json::from_slice(&sent[0].body).unwrap()
}

#[tokio::test]
async fn native_absent_max_tokens_uses_the_selected_model_cap() {
    let body = native_defaults_body(false).await;
    assert_eq!(body["max_tokens"], 12_345);
    assert_eq!(body["temperature"], serde_json::json!(0.3));
}

#[tokio::test]
async fn native_enabled_thinking_suppresses_temperature() {
    let body = native_defaults_body(true).await;
    assert_eq!(body["thinking"]["budget_tokens"], 1024);
    assert!(body.get("temperature").is_none());
}

#[test]
fn anthropic_direct_adapter_rejects_wrong_wire_records() {
    let wrong = ModelInfo::new(
        "gpt-4o",
        "GPT-4o",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(1000, 100),
    );
    let provider = std::sync::Arc::new(AnthropicProvider::new(None));
    let error = AnthropicMessagesChatApi::new(provider, wrong).expect_err("wrong wire");
    assert!(matches!(error, ProviderError::Config { .. }));
}

#[tokio::test]
async fn anthropic_setup_failure_before_activation_is_typed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_string(
            r#"{"type":"error","error":{"type":"authentication_error","message":"bad key"}}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    let model =
        model_39("claude-sonnet-4-5", compat_39(false, false, false)).with_base_url(server.uri());
    let provider = std::sync::Arc::new(AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model.clone()],
        None,
        Default::default(),
        std::sync::Arc::new(HttpClient::new()),
        false,
    ));
    let api = AnthropicMessagesChatApi::new(provider, model.clone()).expect("valid record");
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                secrecy::SecretString::from("test-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register");

    let error = models
        .complete(
            opi_ai::models::ChatModelRecord::new("anthropic", model),
            request_39(ThinkingConfig::default()),
            handle.options(native_options_39()),
        )
        .await
        .expect_err("401 setup failure is typed");
    assert!(
        !format!("{error:?}").contains("bad key"),
        "owned failure detail"
    );
}

#[tokio::test]
async fn anthropic_direct_adapter_replays_same_identity_signed_thinking() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(body_partial_json(serde_json::json!({
            "messages": [
                {"role": "user", "content": [{"type": "text", "text": "q"}]},
                {"role": "assistant", "content": [
                    {"type": "thinking", "thinking": "prior", "signature": "sig-1"}
                ]},
                {"role": "user", "content": [{"type": "text", "text": "next"}]}
            ]
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let model =
        model_39("claude-sonnet-4-5", compat_39(false, false, false)).with_base_url(server.uri());
    let provider = std::sync::Arc::new(AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model.clone()],
        None,
        Default::default(),
        std::sync::Arc::new(HttpClient::new()),
        false,
    ));
    let api = AnthropicMessagesChatApi::new(provider, model.clone()).expect("valid record");
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                secrecy::SecretString::from("test-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register");

    let history = Message::Assistant(opi_ai::message::AssistantMessage {
        content: vec![opi_ai::message::AssistantContent::Thinking {
            thinking: "prior".into(),
            signature: Some("sig-1".into()),
        }],
        api: opi_ai::ApiKind::Anthropic,
        deferred: None,
        provider: "anthropic".into(),
        model: "claude-sonnet-4-5".into(),
        response_model: None,
        wire: Some(WireApi::AnthropicMessages),
        response_id: None,
        usage: Default::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms: 0,
        diagnostics: vec![],
        duration_ms: None,
        provider_thinking_level: None,
    });
    let mut request = request_39(ThinkingConfig::default());
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text { text: "q".into() }],
                timestamp_ms: 0,
            }),
            history,
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "next".into(),
                }],
                timestamp_ms: 1,
            }),
        ],
        tools: vec![],
    })
    .expect("valid");

    let message = models
        .complete(
            opi_ai::models::ChatModelRecord::new("anthropic", model),
            request,
            handle.options(native_options_39()),
        )
        .await
        .expect("replay call completes");
    assert_eq!(message.stop_reason, StopReason::Stop);
}

#[tokio::test]
async fn anthropic_telemetry_carrier_instruments_the_producer() {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tracing_core::span::{Attributes, Id, Record};
    use tracing_core::{Dispatch, Event, Metadata, Subscriber, dispatcher};

    #[derive(Default)]
    struct Recording {
        next: u64,
        spans: Vec<(&'static str, Option<u64>)>,
        entered: HashMap<std::thread::ThreadId, Vec<Id>>,
    }
    struct Recorder(std::sync::Arc<Mutex<Recording>>);

    impl Subscriber for Recorder {
        fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, attributes: &Attributes<'_>) -> Id {
            let mut state = self.0.lock().unwrap();
            state.next += 1;
            let id = Id::from_u64(state.next);
            let parent = attributes.parent().cloned().or_else(|| {
                attributes
                    .is_contextual()
                    .then(|| {
                        state
                            .entered
                            .get(&std::thread::current().id())
                            .and_then(|stack| stack.last())
                            .cloned()
                    })
                    .flatten()
            });
            state
                .spans
                .push((attributes.metadata().name(), parent.map(|id| id.into_u64())));
            id
        }
        fn record(&self, _: &Id, _: &Record<'_>) {}
        fn record_follows_from(&self, _: &Id, _: &Id) {}
        fn event(&self, _: &Event<'_>) {}
        fn enter(&self, id: &Id) {
            self.0
                .lock()
                .unwrap()
                .entered
                .entry(std::thread::current().id())
                .or_default()
                .push(id.clone());
        }
        fn exit(&self, id: &Id) {
            self.0
                .lock()
                .unwrap()
                .entered
                .entry(std::thread::current().id())
                .or_default()
                .pop();
            let _ = id;
        }
    }

    let recorder = std::sync::Arc::new(Mutex::new(Recording::default()));
    let source = Dispatch::new(Recorder(recorder.clone()));
    let parent = dispatcher::with_default(&source, || tracing::info_span!("source_parent"));
    let context = TelemetryContext::from_span(parent);

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let model =
        model_39("claude-sonnet-4-5", compat_39(false, false, false)).with_base_url(server.uri());
    let provider = AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model],
        None,
        Default::default(),
        std::sync::Arc::new(HttpClient::new()),
        false,
    );
    let seen = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = seen.clone();
    let mut request = request_39(ThinkingConfig::default());
    request.telemetry_context = Some(context);
    request.callbacks = RequestCallbacks::new().with_async_on_payload(move |_| {
        let counter = counter.clone();
        async move {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            tracing::info_span!("payload_probe").in_scope(|| ());
            Ok(None)
        }
    });

    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    while let Some(result) = stream.next().await {
        result.expect("stream completes under the carrier");
    }
    assert_eq!(
        seen.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "payload callback ran exactly once"
    );

    // The producer (and its payload callback) ran under the frozen source
    // dispatch: the probe span is recorded on that carrier.
    let state = recorder.lock().unwrap();
    assert!(
        state.spans.iter().any(|(name, _)| *name == "payload_probe"),
        "producer telemetry flows to the frozen carrier: {:?}",
        state.spans
    );
}

#[tokio::test]
async fn direct_unknown_model_without_max_tokens_fails_typed_before_send() {
    let server = MockServer::start().await;
    let provider = AnthropicProvider::new(Some(server.uri()));
    let mut request = request_39(ThinkingConfig::default());
    request.model = "anthropic:unknown-model".into();
    request.max_tokens = None;
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    assert!(matches!(
        stream.next().await,
        Some(Err(ProviderError::Config(_)))
    ));
    assert!(stream.next().await.is_none());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn oauth_key_projects_identity_inline_tools_and_capture_reload_replay() {
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    use opi_ai::message::{AssistantContent, AssistantMessage, OutputContent, ToolResultMessage};
    use std::sync::{Arc, Mutex};
    let server = MockServer::start().await;
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let reply = [
        serde_json::json!({"type":"message_start","message":{"id":"oauth-fixture","model":"claude-sonnet-4-5","usage":{"input_tokens":1,"output_tokens":0}}}),
        serde_json::json!({"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"call-read","name":"Read","input":{}}}),
        serde_json::json!({"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"x\":\"fixture\"}"}}),
        serde_json::json!({"type":"content_block_stop","index":0}),
        serde_json::json!({"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":1}}),
        serde_json::json!({"type":"message_stop"}),
    ].into_iter().map(|event| format!("event: {}\ndata: {event}\n\n", event["type"].as_str().unwrap())).collect::<String>();
    let count = calls.clone();
    Mock::given(method("POST")).respond_with(move |_: &wiremock::Request| {
        if count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
            ResponseTemplate::new(400).set_body_json(serde_json::json!({"type":"error","error":{"type":"invalid_request_error","message":"Schemas contains too many parameters with union types (fixture)"}}))
        } else { ResponseTemplate::new(200).insert_header("content-type", "text/event-stream").set_body_string(reply.clone()) }
    }).mount(&server).await;
    let mut compat = compat_39(false, true, true);
    compat.supports_strict_tools = true;
    let model = model_39("claude-sonnet-4-5", compat).with_base_url(server.uri());
    let api = AnthropicMessagesChatApi::new(
        Arc::new(provider_39(&model, native_options_39())),
        model.clone(),
    )
    .unwrap();
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            Arc::new(api),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                "sk-ant-oat-fixture-only".into(),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .unwrap();
    let strict = |name: &str, preference| ToolDef {
        constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict: preference }),
        ..tool_def_39(name)
    };
    let context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![
            Message::System(opi_ai::transcript::SystemMessage {
                content: "caller prompt".into(),
                sections: vec![],
                tools_added: vec![
                    strict("read", StrictPreference::Prefer),
                    tool_def_39("write"),
                ],
                tools_removed: vec![],
                timestamp_ms: 0,
            }),
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "inspect".into(),
                }],
                timestamp_ms: 1,
            }),
            Message::System(opi_ai::transcript::SystemMessage {
                content: "later prompt".into(),
                sections: vec![],
                tools_added: vec![
                    strict("read", StrictPreference::Prefer),
                    strict("bash", StrictPreference::Require),
                ],
                tools_removed: vec![opi_ai::transcript::ToolReference {
                    name: "write".into(),
                }],
                timestamp_ms: 2,
            }),
        ],
        tools: vec![],
    })
    .unwrap();
    let raw_names = Arc::new(Mutex::new(Vec::new()));
    let recorder = raw_names.clone();
    let mut request = request_39(ThinkingConfig::default());
    request.context = context.clone();
    request.callbacks = RequestCallbacks::new().with_on_raw(Arc::new(move |event| {
        if let Some(name) = event["content_block"]["name"].as_str() {
            recorder.lock().unwrap().push(name.to_owned());
        }
        Ok(())
    }));
    let result = models
        .complete(
            opi_ai::ChatModelRecord::new("anthropic", model.clone()),
            request,
            handle.options(native_options_39()),
        )
        .await
        .unwrap();
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 2, "one lawful strict variant");
    assert_eq!(
        sent[0]
            .headers
            .get("authorization")
            .map(|value| value.to_str().unwrap()),
        Some("Bearer sk-ant-oat-fixture-only")
    );
    assert!(!sent[0].headers.contains_key("x-api-key"));
    assert_eq!(sent[0].headers["user-agent"], "claude-cli/2.1.280");
    assert_eq!(sent[0].headers["x-app"], "cli");
    for (index, sent) in sent.iter().enumerate() {
        let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
        assert_eq!(
            body["system"][0]["text"],
            "You are Claude Code, Anthropic's official CLI for Claude."
        );
        assert_eq!(body["system"][1]["text"], "caller prompt");
        assert_eq!(body["tools"][0]["name"], "Read");
        assert_eq!(
            body["tools"][0]
                .get("strict")
                .and_then(|value| value.as_bool()),
            (index == 0).then_some(true)
        );
        let blocks: Vec<_> = body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|message| message["content"].as_array().into_iter().flatten())
            .collect();
        assert!(
            blocks
                .iter()
                .any(|block| block["type"] == "tool_removal" && block["tool"]["name"] == "Write")
        );
        assert!(blocks.iter().any(|block| block["type"] == "tool_addition"
            && block["tool"]["definition"]["name"] == "Bash"
            && block["tool"]["definition"]["strict"] == true));
    }
    assert_eq!(*raw_names.lock().unwrap(), vec!["Read"]);
    let tool_call = result
        .content
        .iter()
        .find_map(|block| match block {
            AssistantContent::ToolCall { tool_call } => Some(tool_call),
            _ => None,
        })
        .unwrap();
    assert_eq!(tool_call.name, "read");
    let reloaded: AssistantMessage =
        serde_json::from_str(&serde_json::to_string(&result).unwrap()).unwrap();
    let mut messages = context.messages().to_vec();
    messages.push(Message::Assistant(reloaded));
    messages.push(Message::ToolResult(ToolResultMessage {
        tool_call_id: "call-read".into(),
        tool_name: "read".into(),
        content: vec![OutputContent::Text {
            text: "done".into(),
        }],
        details: None,
        is_error: false,
        not_executed: None,
        truncated: false,
        timestamp_ms: 3,
    }));
    let mut next = request_39(ThinkingConfig::default());
    next.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages,
        tools: vec![],
    })
    .unwrap();
    models
        .complete(
            opi_ai::ChatModelRecord::new("anthropic", model),
            next,
            handle.options(native_options_39()),
        )
        .await
        .unwrap();
    let sent = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&sent[2].body).unwrap();
    assert!(
        body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|message| message["content"].as_array().into_iter().flatten())
            .any(|block| block["type"] == "tool_use" && block["name"] == "Read")
    );
}

mod federation_fixtures {
    use super::*;
    use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
    use opi_ai::credential::BoxAuthFuture;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Clone)]
    struct FederationAuth(ResolvedAuth);
    impl AuthResolver for FederationAuth {
        fn resolve(&self) -> BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
            Box::pin(async { Ok(self.0.clone()) })
        }
    }

    fn prepared(file: &std::path::Path) -> ResolvedAuth {
        let mut auth = opi_ai::test_support::resolved_auth();
        auth.scheme = AuthScheme::AnthropicFederation;
        auth.secret = "".into();
        auth.env = [
            ("ANTHROPIC_FEDERATION_RULE_ID", "rule-fixture".to_owned()),
            ("ANTHROPIC_ORGANIZATION_ID", "org-fixture".to_owned()),
            (
                "ANTHROPIC_IDENTITY_TOKEN_FILE",
                file.to_string_lossy().into_owned(),
            ),
            ("ANTHROPIC_SERVICE_ACCOUNT_ID", "service-fixture".to_owned()),
            ("ANTHROPIC_WORKSPACE_ID", "workspace-fixture".to_owned()),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.into()))
        .collect();
        auth
    }

    fn setup(server: &MockServer, auth: ResolvedAuth) -> (Models, opi_ai::ChatModelRecord) {
        let model = model_39("claude-sonnet-4-5", compat_39(false, false, false))
            .with_base_url(server.uri());
        let provider = Arc::new(provider_39(&model, native_options_39()));
        let api = AnthropicMessagesChatApi::new(provider, model.clone()).unwrap();
        let mut models = Models::new();
        models
            .register_chat_api(
                Arc::new(api),
                Arc::new(FederationAuth(auth)),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        (models, opi_ai::ChatModelRecord::new("anthropic", model))
    }

    fn success() -> ResponseTemplate {
        let body = [
            serde_json::json!({"type":"message_start","message":{"id":"federated","model":"claude-sonnet-4-5","usage":{"input_tokens":1,"output_tokens":0}}}),
            serde_json::json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":"ok"}}),
            serde_json::json!({"type":"content_block_stop","index":0}),
            serde_json::json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":1}}),
            serde_json::json!({"type":"message_stop"}),
        ].into_iter().map(|e| format!("event: {}\ndata: {e}\n\n",e["type"].as_str().unwrap())).collect::<String>();
        ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string(body)
    }

    #[tokio::test]
    async fn federation_real_models_exchanges_caches_and_keeps_non_subscription_wire() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("identity.jwt");
        let (models, record) = setup(&server, prepared(&file));
        let call = models.complete_simple(
            record.clone(),
            request_39(Default::default()),
            SimpleOptions::default(),
        );
        assert!(server.received_requests().await.unwrap().is_empty());
        std::fs::write(&file, "  fixture-identity-jwt\n").unwrap();
        Mock::given(method("POST")).and(path("/v1/oauth/token"))
            .and(wiremock::matchers::headers("anthropic-beta", vec!["oauth-2025-04-20", "oidc-federation-2026-04-01"]))
            .and(body_partial_json(serde_json::json!({"grant_type":"urn:ietf:params:oauth:grant-type:jwt-bearer","assertion":"fixture-identity-jwt","federation_rule_id":"rule-fixture","organization_id":"org-fixture","service_account_id":"service-fixture","workspace_id":"workspace-fixture"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"access_token":"federated-access","expires_in":3600,"token_type":"Bearer"})))
            .expect(1).mount(&server).await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("authorization", "Bearer federated-access"))
            .respond_with(success())
            .expect(2)
            .mount(&server)
            .await;
        call.await.expect("real federation exchange and generation");
        std::fs::remove_file(&file).unwrap();
        models
            .complete_simple(record, request_39(Default::default()), Default::default())
            .await
            .expect("cached token needs no identity file");
        let requests = server.received_requests().await.unwrap();
        let generation = requests
            .iter()
            .filter(|r| r.url.path() == "/v1/messages")
            .collect::<Vec<_>>();
        assert_eq!(generation.len(), 2);
        for request in generation {
            assert_eq!(request.headers["anthropic-beta"], "oauth-2025-04-20");
            assert!(!request.headers.contains_key("x-api-key"));
            assert!(!request.headers.contains_key("x-app"));
            let body = String::from_utf8(request.body.clone()).unwrap();
            assert!(!body.contains("Claude Code"));
            assert!(!body.contains("fixture-identity-jwt"));
        }
        server.verify().await;
    }

    #[tokio::test]
    async fn federation_401_invalidates_next_call_without_hidden_generation_retry() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("identity.jwt");
        std::fs::write(&file, "first-identity").unwrap();
        let (models, record) = setup(&server, prepared(&file));
        let exchanges = Arc::new(AtomicUsize::new(0));
        let count = exchanges.clone();
        Mock::given(method("POST"))
            .and(path("/v1/oauth/token"))
            .respond_with(move |_: &wiremock::Request| {
                let n = count.fetch_add(1, Ordering::SeqCst) + 1;
                ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"access_token":format!("access-{n}"),"expires_in":3600}),
                )
            })
            .expect(2)
            .mount(&server)
            .await;
        let sends = Arc::new(AtomicUsize::new(0));
        let count = sends.clone();
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(move |_: &wiremock::Request| {
                if count.fetch_add(1, Ordering::SeqCst) == 1 {
                    ResponseTemplate::new(401)
                } else {
                    success()
                }
            })
            .expect(3)
            .mount(&server)
            .await;
        models
            .complete_simple(
                record.clone(),
                request_39(Default::default()),
                Default::default(),
            )
            .await
            .unwrap();
        let failure = models
            .complete_simple(
                record.clone(),
                request_39(Default::default()),
                Default::default(),
            )
            .await
            .unwrap_err();
        assert_eq!(
            failure.cause().category(),
            opi_ai::provider::ProviderErrorCategory::Auth
        );
        assert_eq!(sends.load(Ordering::SeqCst), 2);
        assert_eq!(exchanges.load(Ordering::SeqCst), 1);
        std::fs::write(&file, "second-identity").unwrap();
        models
            .complete_simple(record, request_39(Default::default()), Default::default())
            .await
            .unwrap();
        let requests = server.received_requests().await.unwrap();
        let auths = requests
            .iter()
            .filter(|r| r.url.path() == "/v1/messages")
            .map(|r| r.headers["authorization"].to_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            auths,
            vec!["Bearer access-1", "Bearer access-1", "Bearer access-2"]
        );
        let tokens = requests
            .iter()
            .filter(|r| r.url.path() == "/v1/oauth/token")
            .map(|r| {
                serde_json::from_slice::<serde_json::Value>(&r.body).unwrap()["assertion"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(tokens, vec!["first-identity", "second-identity"]);
        server.verify().await;
    }

    #[tokio::test]
    async fn federation_expiry_and_complete_model_base_select_fresh_exchange() {
        let first = MockServer::start().await;
        let second = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("identity");
        std::fs::write(&file, "rotating-1").unwrap();
        let (models, record) = setup(&first, prepared(&file));
        for server in [&first, &second] {
            Mock::given(method("POST"))
                .and(path("/v1/oauth/token"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"access_token":"short-token","expires_in":0}),
                ))
                .mount(server)
                .await;
            Mock::given(method("POST"))
                .and(path("/v1/messages"))
                .respond_with(success())
                .mount(server)
                .await;
        }
        models
            .complete_simple(
                record.clone(),
                request_39(Default::default()),
                Default::default(),
            )
            .await
            .unwrap();
        std::fs::write(&file, "rotating-2").unwrap();
        models
            .complete_simple(
                record.clone(),
                request_39(Default::default()),
                Default::default(),
            )
            .await
            .unwrap();
        let mut changed = record.info().clone();
        changed.base_url = Some(second.uri());
        models
            .complete_simple(
                opi_ai::ChatModelRecord::new("anthropic", changed),
                request_39(Default::default()),
                Default::default(),
            )
            .await
            .unwrap();
        let sent = first.received_requests().await.unwrap();
        let assertions = sent
            .iter()
            .filter(|r| r.url.path() == "/v1/oauth/token")
            .map(|r| {
                serde_json::from_slice::<serde_json::Value>(&r.body).unwrap()["assertion"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(assertions, vec!["rotating-1", "rotating-2"]);
        let second = second.received_requests().await.unwrap();
        assert_eq!(
            second
                .iter()
                .filter(|r| r.url.path() == "/v1/oauth/token")
                .count(),
            1
        );
        assert_eq!(
            second
                .iter()
                .filter(|r| r.url.path() == "/v1/messages")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn federation_callback_failure_and_unsafe_base_do_no_auth_io() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        let (models, record) = setup(&server, prepared(&missing));
        let mut request = request_39(Default::default());
        request.callbacks = opi_ai::RequestCallbacks::new().with_on_payload(Arc::new(|_| {
            Err(opi_ai::CallbackError::new("fixture callback failed"))
        }));
        let failure = models
            .complete_simple(record.clone(), request, Default::default())
            .await
            .unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::CallbackPhase(_)));
        assert!(server.received_requests().await.unwrap().is_empty());
        let mut unsafe_model = record.info().clone();
        unsafe_model.base_url = Some("http://unsafe-federation.invalid".into());
        let failure = models
            .complete_simple(
                opi_ai::ChatModelRecord::new("anthropic", unsafe_model),
                request_39(Default::default()),
                Default::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::Config(_)));
        assert!(server.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn federation_failed_exchange_never_generates_or_exposes_identity_material() {
        for (status, response) in [
            (
                401,
                serde_json::json!({"assertion":"identity-secret-canary","access_token":"access-secret-canary"}),
            ),
            (
                200,
                serde_json::json!({"access_token":"access-secret-canary","expires_in":"not-finite"}),
            ),
            (
                200,
                serde_json::json!({"access_token":"access-secret-canary","expires_in":3600,"token_type":"Mac"}),
            ),
            (
                200,
                serde_json::json!({"access_token":"","expires_in":3600}),
            ),
        ] {
            let server = MockServer::start().await;
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("identity");
            std::fs::write(&file, "identity-secret-canary").unwrap();
            let (models, record) = setup(&server, prepared(&file));
            Mock::given(method("POST"))
                .and(path("/v1/oauth/token"))
                .respond_with(
                    ResponseTemplate::new(status)
                        .insert_header("request-id", "federation-request-fixture")
                        .set_body_json(response),
                )
                .expect(1)
                .mount(&server)
                .await;
            let failure = models
                .complete_simple(record, request_39(Default::default()), Default::default())
                .await
                .unwrap_err();
            assert!(matches!(failure.cause(), ProviderError::AuthFailed(_)));
            let diagnostic = format!("{}", failure.cause());
            assert!(diagnostic.contains(&status.to_string()), "{diagnostic}");
            assert!(
                diagnostic.contains("federation-request-fixture"),
                "{diagnostic}"
            );
            for text in [format!("{}", failure.cause()), format!("{failure:?}")] {
                assert!(!text.contains("identity-secret-canary"));
                assert!(!text.contains("access-secret-canary"));
            }
            assert_eq!(server.received_requests().await.unwrap().len(), 1);
            server.verify().await;
        }
    }

    #[tokio::test]
    async fn federation_wait_can_be_cancelled_without_generation_send() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("identity");
        std::fs::write(&file, "fixture-identity").unwrap();
        let (models, record) = setup(&server, prepared(&file));
        let exchanged = Arc::new(tokio::sync::Notify::new());
        let seen = exchanged.clone();
        Mock::given(method("POST"))
            .and(path("/v1/oauth/token"))
            .respond_with(move |_: &wiremock::Request| {
                seen.notify_one();
                ResponseTemplate::new(200)
                    .set_delay(std::time::Duration::from_secs(60))
                    .set_body_json(serde_json::json!({"access_token":"delayed","expires_in":3600}))
            })
            .expect(1)
            .mount(&server)
            .await;
        let request = request_39(Default::default());
        let cancel = request.cancel.clone();
        let call = models.complete_simple(record, request, Default::default());
        let caller = tokio::spawn(call);
        tokio::time::timeout(std::time::Duration::from_secs(3), exchanged.notified())
            .await
            .unwrap();
        cancel.cancel();
        let failure = tokio::time::timeout(std::time::Duration::from_secs(3), caller)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::Cancelled));
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn federation_builtin_env_only_credential_selects_then_generates_without_early_file_io() {
        use opi_ai::credential::CredentialStore;
        struct EmptyContext;
        impl opi_ai::auth::AuthContext for EmptyContext {
            fn env(&self, _: &str) -> Result<Option<String>, opi_ai::auth::AuthContextError> {
                Ok(None)
            }
            fn file_exists(&self, _: &str) -> Result<bool, opi_ai::auth::AuthContextError> {
                panic!("configuration checks must not inspect federation identity file")
            }
        }
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("not-created-until-generation");
        let env = [
            ("ANTHROPIC_FEDERATION_RULE_ID", "stored-rule".to_owned()),
            ("ANTHROPIC_ORGANIZATION_ID", "stored-org".to_owned()),
            (
                "ANTHROPIC_IDENTITY_TOKEN_FILE",
                file.to_string_lossy().into_owned(),
            ),
        ]
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value.into()))
        .collect();
        let store = Arc::new(opi_ai::mutation::InMemoryCredentialStore::new());
        store
            .write(
                "anthropic",
                &opi_ai::Credential::ApiKey(opi_ai::ApiKeyCredential::new(None, env)),
            )
            .await
            .unwrap();
        let mut models = Models::new();
        models.set_credential_store(store);
        let mut overrides = opi_ai::provider_collection::BuiltinOverrides::default();
        overrides.base_urls.insert("anthropic".into(), server.uri());
        opi_ai::provider_collection::register_builtin(
            &mut models,
            Arc::new(EmptyContext),
            Arc::new(HttpClient::new()),
            &overrides,
        )
        .unwrap();
        let available = models.get_available(Some("anthropic")).await.unwrap();
        assert!(!available.is_empty());
        let auth = models
            .get_auth("anthropic", Default::default())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(auth.scheme, AuthScheme::AnthropicFederation);
        assert!(!file.exists());
        assert!(server.received_requests().await.unwrap().is_empty());
        Mock::given(method("POST")).and(path("/v1/oauth/token"))
            .and(body_partial_json(serde_json::json!({"assertion":"stored-identity","federation_rule_id":"stored-rule","organization_id":"stored-org"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"access_token":"stored-fed-token","expires_in":3600})))
            .expect(1).mount(&server).await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("authorization", "Bearer stored-fed-token"))
            .respond_with(success())
            .expect(1)
            .mount(&server)
            .await;
        let (provider, info) = available.into_iter().next().unwrap();
        let call = models.complete_simple(
            opi_ai::ChatModelRecord::new(provider, info),
            request_39(Default::default()),
            Default::default(),
        );
        std::fs::write(&file, "stored-identity").unwrap();
        call.await.expect("real federation exchange and generation");
        server.verify().await;
    }

    #[tokio::test]
    async fn federation_identity_file_empty_and_source_utf16_assertion_limit_are_observable() {
        for (identity, allowed) in [
            (" \n\t".to_owned(), false),
            ("x".repeat(16 * 1024 + 1), false),
            ("\u{1f600}".repeat(8192), true),
        ] {
            let server = MockServer::start().await;
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("identity");
            std::fs::write(&file, identity).unwrap();
            let (models, record) = setup(&server, prepared(&file));
            Mock::given(method("POST"))
                .and(path("/v1/oauth/token"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"access_token":"accepted-token","expires_in":"3600"}),
                ))
                .expect(if allowed { 1 } else { 0 })
                .mount(&server)
                .await;
            Mock::given(method("POST"))
                .and(path("/v1/messages"))
                .respond_with(success())
                .expect(if allowed { 1 } else { 0 })
                .mount(&server)
                .await;
            let result = models
                .complete_simple(record, request_39(Default::default()), Default::default())
                .await;
            if allowed {
                result.unwrap();
            } else {
                assert!(matches!(
                    result.unwrap_err().cause(),
                    ProviderError::AuthFailed(_)
                ));
            }
            server.verify().await;
        }
    }

    #[tokio::test]
    async fn federation_identity_uses_node_utf8_replacement_and_javascript_trim() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("identity");
        let mut bytes = b"\xef\xbb\xbf \tidentity-".to_vec();
        bytes.push(0xff);
        bytes.extend_from_slice(b"-token\xef\xbb\xbf\n");
        std::fs::write(&file, bytes).unwrap();
        let (models, record) = setup(&server, prepared(&file));
        Mock::given(method("POST"))
            .and(path("/v1/oauth/token"))
            .and(body_partial_json(
                serde_json::json!({"assertion":"identity-\u{fffd}-token"}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"access_token":"decoded-token","expires_in":3600}),
            ))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(success())
            .expect(1)
            .mount(&server)
            .await;
        models
            .complete_simple(record, request_39(Default::default()), Default::default())
            .await
            .expect("Node UTF8 and JS trim identity projection");
        server.verify().await;
    }

    #[tokio::test]
    async fn federation_expires_in_uses_source_javascript_number_conversion() {
        for expires in [
            serde_json::json!("0xE10"),
            serde_json::json!("0b111000010000"),
            serde_json::json!("0o7020"),
            serde_json::json!("\u{feff}3600\u{feff}"),
            serde_json::json!(null),
            serde_json::json!(false),
            serde_json::json!([]),
            serde_json::json!([null]),
            serde_json::json!(["3600"]),
            serde_json::json!([["3600"]]),
        ] {
            let server = MockServer::start().await;
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("identity");
            std::fs::write(&file, "numeric-fixture-identity").unwrap();
            let (models, record) = setup(&server, prepared(&file));
            Mock::given(method("POST"))
                .and(path("/v1/oauth/token"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"access_token":"numeric-token","expires_in":expires}),
                ))
                .expect(1)
                .mount(&server)
                .await;
            Mock::given(method("POST"))
                .and(path("/v1/messages"))
                .respond_with(success())
                .expect(1)
                .mount(&server)
                .await;
            models
                .complete_simple(record, request_39(Default::default()), Default::default())
                .await
                .expect("source Number(expires_in) remains finite");
            server.verify().await;
        }
    }

    #[tokio::test]
    async fn federation_token_response_uses_source_utf8_decoder() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("identity");
        std::fs::write(&file, "fixture-identity").unwrap();
        let (models, record) = setup(&server, prepared(&file));
        let mut response=b"\xef\xbb\xbf{\"access_token\":\"decoded-token\",\"expires_in\":3600,\"annotation\":\"".to_vec();
        response.push(0xff);
        response.extend_from_slice(b"\"}");
        Mock::given(method("POST"))
            .and(path("/v1/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(response, "application/json"))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("authorization", "Bearer decoded-token"))
            .respond_with(success())
            .expect(1)
            .mount(&server)
            .await;
        models
            .complete_simple(record, request_39(Default::default()), Default::default())
            .await
            .expect("source TextDecoder response projection");
        server.verify().await;
    }
}
