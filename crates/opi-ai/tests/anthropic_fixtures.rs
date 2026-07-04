//! Behavioral tests for task 1.3: Anthropic SSE provider.
//!
//! DoD: "fixtures cover text, tool call, usage, error"
//! All tests use fixture strings — no live provider calls (red flag #10).

use futures_util::StreamExt;
use opi_ai::anthropic::{
    AnthropicEvent, AnthropicMapper, AnthropicProvider, ParsedEvent, parse_sse_events,
};
use opi_ai::message::{AssistantContent, InputContent, Message, UserMessage};
use opi_ai::provider::{Provider, Request, ThinkingConfig};
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: parse fixture, extract valid events, and map through a stateful mapper.
fn map_fixture(input: &str) -> Vec<AssistantStreamEvent> {
    let events: Vec<AnthropicEvent> = parse_sse_events(input)
        .filter_map(|p| match p {
            ParsedEvent::Valid(e) => Some(e),
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
                AssistantContent::Text { text } => Some(text.as_str()),
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
    let stream_events = map_fixture(error_fixture());

    assert_eq!(stream_events.len(), 1);
    assert!(matches!(
        &stream_events[0],
        AssistantStreamEvent::Error { reason, .. } if *reason == StopReason::Error
    ));
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
    let provider = opi_ai::anthropic::AnthropicProvider::new("test-key".into(), None);
    assert_eq!(provider.id(), "anthropic");
}

#[test]
fn anthropic_provider_models_not_empty() {
    let provider = opi_ai::anthropic::AnthropicProvider::new("test-key".into(), None);
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

    let provider = AnthropicProvider::new("test-key".into(), None);
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
        if let AssistantContent::Thinking { thinking } = &message.content[0] {
            assert_eq!(
                thinking,
                "Let me analyze this problem. Step 1: identify the key constraint."
            );
        } else {
            panic!("expected Thinking content at index 0");
        }

        // Second content block is text
        if let AssistantContent::Text { text } = &message.content[1] {
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
    let provider = AnthropicProvider::new("test-key".into(), None);
    let request = Request {
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
        system: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "Hello".into(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
        max_tokens: Some(4096),
        temperature: None,
        thinking: ThinkingConfig {
            enabled: true,
            budget_tokens: Some(5000),
        },
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
    };

    let body = provider.build_request_body(&request);

    let thinking = body
        .get("thinking")
        .expect("thinking field should be present");
    assert_eq!(thinking["type"], "enabled");
    assert_eq!(thinking["budget_tokens"], 5000);
}

#[test]
fn build_request_body_uses_default_budget_when_none() {
    let provider = AnthropicProvider::new("test-key".into(), None);
    let request = Request {
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
        system: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "Hello".into(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
        max_tokens: Some(4096),
        temperature: None,
        thinking: ThinkingConfig {
            enabled: true,
            budget_tokens: None,
        },
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
    };

    let body = provider.build_request_body(&request);

    let thinking = body
        .get("thinking")
        .expect("thinking field should be present");
    assert_eq!(
        thinking["budget_tokens"], 10000,
        "default budget should be 10000"
    );
}

#[test]
fn build_request_body_omits_thinking_when_disabled() {
    let provider = AnthropicProvider::new("test-key".into(), None);
    let request = Request {
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
        system: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "Hello".into(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
        max_tokens: Some(4096),
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
    };

    let body = provider.build_request_body(&request);

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
        if let AssistantContent::Thinking { thinking } = &message.content[0] {
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
        if let AssistantContent::Thinking { thinking } = &message.content[0] {
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
            "system": "You are helpful.",
            "max_tokens": 1024
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new("test-key".into(), Some(server.uri()));
    let request = Request {
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
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
    let provider = AnthropicProvider::new("test-key".into(), Some(server.uri()));
    let request = Request {
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
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
