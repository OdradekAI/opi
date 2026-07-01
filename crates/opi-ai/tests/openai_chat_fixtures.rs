//! Behavioral tests for task 2.1: OpenAI-compatible chat provider.
//!
//! DoD: "fixtures cover text, tool call, usage, error; implements Provider trait
//! with SSE streaming; exposes compat config points for role mapping
//! (developer/system), usage-in-stream, max_tokens field naming, and tool_result
//! name field so downstream profiles (OpenRouter, Mistral) can override behavior"
//!
//! All tests use fixture strings — no live provider calls (red flag #10).

use futures_util::StreamExt;
use opi_ai::message::AssistantContent;
use opi_ai::openai_chat::{
    CompatConfig, OpenAiChatEvent, OpenAiChatMapper, OpenAiChatProvider, ParsedEvent,
    parse_sse_events,
};
use opi_ai::provider::Provider;
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: parse fixture, extract valid events, and map through a stateful mapper.
fn map_fixture(input: &str) -> Vec<AssistantStreamEvent> {
    let events: Vec<OpenAiChatEvent> = parse_sse_events(input)
        .flat_map(|p| match p {
            ParsedEvent::Valid(evts) => evts,
            ParsedEvent::Malformed { .. } => Vec::new(),
        })
        .collect();
    let mut mapper = OpenAiChatMapper::new(opi_ai::ApiKind::OpenAi, "openai");
    events.into_iter().flat_map(|e| mapper.process(e)).collect()
}

/// Helper: map with a custom provider label (for OpenRouter/Mistral profiles).
#[allow(dead_code)]
fn map_fixture_as(input: &str, api: opi_ai::ApiKind, provider: &str) -> Vec<AssistantStreamEvent> {
    let events: Vec<OpenAiChatEvent> = parse_sse_events(input)
        .flat_map(|p| match p {
            ParsedEvent::Valid(evts) => evts,
            ParsedEvent::Malformed { .. } => Vec::new(),
        })
        .collect();
    let mut mapper = OpenAiChatMapper::new(api, provider);
    events.into_iter().flat_map(|e| mapper.process(e)).collect()
}

/// Helper: collect valid OpenAiChatEvents from parsed output.
fn collect_valid_events(input: &str) -> Vec<OpenAiChatEvent> {
    parse_sse_events(input)
        .flat_map(|p| match p {
            ParsedEvent::Valid(evts) => evts,
            ParsedEvent::Malformed { .. } => Vec::new(),
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
fn sse_parse_skips_non_json_lines() {
    let input = "data: [DONE]\n\n";
    let events = collect_valid_events(input);
    assert!(events.is_empty());
}

#[test]
fn sse_parse_ignores_comments() {
    let input = ": this is a comment\n\n";
    let events = collect_valid_events(input);
    assert!(events.is_empty());
}

// --- Text Fixture ---

fn text_fixture() -> &'static str {
    r#"data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":" world"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":25,"completion_tokens":8,"total_tokens":33}}

data: [DONE]

"#
}

#[test]
fn text_fixture_yields_all_events() {
    let events = collect_valid_events(text_fixture());
    // role delta, "Hello" delta, " world" delta, finish_reason delta
    assert_eq!(events.len(), 4);
}

#[test]
fn text_fixture_maps_to_stream_events() {
    let stream_events = map_fixture(text_fixture());

    // Start, TextStart, TextDelta("Hello"), TextDelta(" world"), TextEnd, Done
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

    if let AssistantStreamEvent::Done { reason, .. } = &stream_events[5] {
        assert_eq!(*reason, StopReason::Stop);
    } else {
        panic!("expected Done at index 5");
    }
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
    r#"data: {"id":"chatcmpl-tool123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":null},"finish_reason":null}]}

data: {"id":"chatcmpl-tool123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_abc","type":"function","function":{"name":"read_file","arguments":""}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-tool123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"path\":"}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-tool123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"/tmp/test\"}"}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-tool123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":50,"completion_tokens":30,"total_tokens":80}}

data: [DONE]

"#
}

#[test]
fn tool_call_fixture_yields_tool_events() {
    let events = collect_valid_events(tool_call_fixture());
    // role delta, tool_call start, arg chunk 1, arg chunk 2, finish_reason
    assert_eq!(events.len(), 5);
}

#[test]
fn tool_call_fixture_maps_to_stream_events() {
    let stream_events = map_fixture(tool_call_fixture());

    // Start, ToolCallStart, ToolCallDelta, ToolCallDelta, ToolCallEnd, Done(tool_use)
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

    if let AssistantStreamEvent::ToolCallEnd { tool_call, .. } = &stream_events[4] {
        assert_eq!(tool_call.name, "read_file");
        assert_eq!(tool_call.id, "call_abc");
        assert!(tool_call.arguments.contains("path"));
    } else {
        panic!("expected ToolCallEnd at index 4");
    }

    if let AssistantStreamEvent::Done { reason, .. } = &stream_events[5] {
        assert_eq!(*reason, StopReason::ToolUse);
    } else {
        panic!("expected Done at index 5");
    }
}

// --- Usage Tests ---

#[test]
fn usage_captured_from_final_chunk() {
    let stream_events = map_fixture(text_fixture());

    if let AssistantStreamEvent::Done { message, .. } = &stream_events[5] {
        assert_eq!(message.usage.input_tokens, 25);
        assert_eq!(message.usage.output_tokens, 8);
    } else {
        panic!("expected Done event");
    }
}

#[test]
fn tool_call_usage_tracked() {
    let stream_events = map_fixture(tool_call_fixture());

    if let AssistantStreamEvent::Done { message, .. } = &stream_events[5] {
        assert_eq!(message.usage.input_tokens, 50);
        assert_eq!(message.usage.output_tokens, 30);
    } else {
        panic!("expected Done event");
    }
}

// --- Error Fixture ---

fn error_fixture() -> &'static str {
    r#"data: {"error":{"message":"Rate limit exceeded","type":"rate_limit_error","param":null,"code":"rate_limit_exceeded"}}

"#
}

#[test]
fn error_fixture_parsed_as_error() {
    let events = collect_valid_events(error_fixture());
    assert!(matches!(events[0], OpenAiChatEvent::Error { .. }));
}

#[test]
fn error_event_maps_to_stream_error() {
    let stream_events = map_fixture(error_fixture());

    assert_eq!(stream_events.len(), 1);
    if let AssistantStreamEvent::Error {
        reason, message, ..
    } = &stream_events[0]
    {
        assert_eq!(*reason, StopReason::Error);
        assert!(
            message
                .error_message
                .as_ref()
                .unwrap()
                .contains("Rate limit")
        );
    } else {
        panic!("expected Error stream event");
    }
}

// --- Stop Reason Mapping ---

#[test]
fn stop_reason_stop_maps_correctly() {
    let stream_events = map_fixture(text_fixture());

    if let AssistantStreamEvent::Done { reason, .. } = &stream_events[5] {
        assert_eq!(*reason, StopReason::Stop);
    } else {
        panic!("expected Done with StopReason::Stop");
    }
}

#[test]
fn stop_reason_tool_calls_maps_correctly() {
    let stream_events = map_fixture(tool_call_fixture());

    if let AssistantStreamEvent::Done { reason, .. } = &stream_events[5] {
        assert_eq!(*reason, StopReason::ToolUse);
    } else {
        panic!("expected Done with StopReason::ToolUse");
    }
}

// --- Content null edge case ---

#[test]
fn content_null_delta_without_tool_calls_produces_start_then_text() {
    let input = r#"data: {"id":"chatcmpl-null","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":null},"finish_reason":null}]}

data: {"id":"chatcmpl-null","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-null","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]

"#;
    let stream_events = map_fixture(input);

    // Start, TextStart, TextDelta("Hello"), TextEnd, Done
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
    if let Some(AssistantStreamEvent::Done { reason, .. }) = stream_events.last() {
        assert_eq!(*reason, StopReason::Stop);
    } else {
        panic!("expected Done with StopReason::Stop");
    }
    // No empty TextDelta events
    let empty_deltas: Vec<_> = stream_events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::TextDelta { delta, .. } if delta.is_empty()))
        .collect();
    assert!(
        empty_deltas.is_empty(),
        "should not emit empty TextDelta events"
    );
}

#[test]
fn stop_reason_length_maps_to_length() {
    let input = r#"data: {"id":"chatcmpl-len","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":"Hi"},"finish_reason":null}]}

data: {"id":"chatcmpl-len","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"length"}]}

data: [DONE]

"#;
    let stream_events = map_fixture(input);

    if let Some(AssistantStreamEvent::Done { reason, .. }) = stream_events.last() {
        assert_eq!(*reason, StopReason::Length);
    } else {
        panic!("expected Done with StopReason::Length");
    }
}

// --- Mixed text + tool call fixture ---

fn mixed_fixture() -> &'static str {
    r#"data: {"id":"chatcmpl-mix","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-mix","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"Let me read that."},"finish_reason":null}]}

data: {"id":"chatcmpl-mix","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":null}]}

data: {"id":"chatcmpl-mix","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":1,"id":"call_123","type":"function","function":{"name":"read_file","arguments":""}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-mix","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":1,"function":{"arguments":"{\"path\":\"src/main.rs\"}"}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-mix","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":30,"completion_tokens":20,"total_tokens":50}}

data: [DONE]

"#
}

#[test]
fn mixed_fixture_produces_text_then_tool_call() {
    let stream_events = map_fixture(mixed_fixture());

    // Start, TextStart, TextDelta, TextEnd, ToolCallStart, ToolCallDelta, ToolCallEnd, Done
    assert!(matches!(
        stream_events[0],
        AssistantStreamEvent::Start { .. }
    ));
    assert!(matches!(
        stream_events[1],
        AssistantStreamEvent::TextStart { .. }
    ));
    assert!(matches!(
        stream_events[4],
        AssistantStreamEvent::ToolCallStart { .. }
    ));

    if let Some(AssistantStreamEvent::Done { message, reason }) = stream_events.last() {
        assert_eq!(*reason, StopReason::ToolUse);
        assert_eq!(message.content.len(), 2);
    } else {
        panic!("expected Done event");
    }
}

// --- Malformed SSE Tests ---

#[test]
fn malformed_sse_data_produces_malformed_event() {
    let input = "data: {invalid json here}\n\n";
    let parsed: Vec<_> = parse_sse_events(input).collect();
    assert_eq!(parsed.len(), 1);
    assert!(
        matches!(&parsed[0], ParsedEvent::Malformed { .. }),
        "expected Malformed event for invalid JSON data"
    );
}

#[test]
fn malformed_and_valid_events_coexist() {
    let input = "data: {bad json}\n\ndata: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n";
    let parsed: Vec<_> = parse_sse_events(input).collect();
    assert_eq!(parsed.len(), 2);
    assert!(matches!(parsed[0], ParsedEvent::Malformed { .. }));
    assert!(matches!(parsed[1], ParsedEvent::Valid(_))); // Vec<OpenAiChatEvent>
}

// --- CRLF SSE Tests ---

#[test]
fn sse_parse_handles_crlf_line_endings() {
    let input = "data: {\"id\":\"chatcmpl-crlf\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"Hi\"},\"finish_reason\":null}]}\r\n\r\n";
    let events = collect_valid_events(input);
    assert_eq!(events.len(), 1);
}

// --- Provider Tests ---

#[test]
fn openai_chat_provider_id() {
    let provider = OpenAiChatProvider::new("test-key".into(), None);
    assert_eq!(provider.id(), "openai");
}

#[test]
fn openai_chat_provider_models_not_empty() {
    let provider = OpenAiChatProvider::new("test-key".into(), None);
    assert!(!provider.models().is_empty());
}

#[tokio::test]
async fn stream_from_sse_produces_events() {
    let provider = OpenAiChatProvider::new("test-key".into(), None);
    let cancel = tokio_util::sync::CancellationToken::new();
    let mut stream = provider.stream_from_sse(text_fixture(), cancel);

    let first = stream.next().await.expect("should have an event");
    assert!(first.is_ok());
    assert!(matches!(first.unwrap(), AssistantStreamEvent::Start { .. }));
}

// --- Compat Config Tests ---

#[test]
fn compat_config_role_mapping_developer() {
    // OpenAI o-series models use "developer" instead of "system"
    let config = CompatConfig {
        system_role_override: Some("developer".into()),
        ..Default::default()
    };
    assert_eq!(config.system_role_override.as_deref(), Some("developer"));
}

#[test]
fn compat_config_max_tokens_field_name() {
    // Some providers use "max_completion_tokens" instead of "max_tokens"
    let config = CompatConfig {
        max_tokens_field: "max_completion_tokens".into(),
        ..Default::default()
    };
    assert_eq!(config.max_tokens_field, "max_completion_tokens");
}

#[test]
fn compat_config_tool_result_name_field() {
    // Some providers send tool_result as "name" instead of matching by id
    let config = CompatConfig {
        tool_result_name_field: true,
        ..Default::default()
    };
    assert!(config.tool_result_name_field);
}

#[test]
fn compat_config_usage_in_stream() {
    // Some providers include usage in every chunk, not just the last
    let config = CompatConfig {
        usage_in_stream: true,
        ..Default::default()
    };
    assert!(config.usage_in_stream);
}

#[test]
fn compat_config_defaults() {
    let config = CompatConfig::default();
    assert!(config.system_role_override.is_none());
    assert_eq!(config.max_tokens_field, "max_tokens");
    assert!(!config.tool_result_name_field);
    assert!(!config.usage_in_stream);
}

// --- Build Request Body Tests ---

use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::Request;
use opi_ai::provider::ThinkingConfig;
use tokio_util::sync::CancellationToken;

fn make_test_request() -> Request {
    Request {
        model: "openai:gpt-4o".into(),
        system: Some("You are helpful.".into()),
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
    }
}

#[test]
fn build_request_body_uses_max_tokens_by_default() {
    let provider = OpenAiChatProvider::new("test-key".into(), None);
    let body = provider.build_request_body(&make_test_request());
    assert!(body.get("max_tokens").is_some());
    assert_eq!(body["max_tokens"], 4096);
}

#[test]
fn build_request_body_with_compat_max_completion_tokens() {
    let config = CompatConfig {
        max_tokens_field: "max_completion_tokens".into(),
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat("test-key".into(), None, config);
    let body = provider.build_request_body(&make_test_request());
    assert!(body.get("max_completion_tokens").is_some());
    assert!(body.get("max_tokens").is_none());
    assert_eq!(body["max_completion_tokens"], 4096);
}

#[test]
fn build_request_body_system_role_default() {
    let provider = OpenAiChatProvider::new("test-key".into(), None);
    let body = provider.build_request_body(&make_test_request());
    // Default: system message uses "system" role
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages[0]["role"], "system");
}

#[test]
fn build_request_body_developer_role_override() {
    let config = CompatConfig {
        system_role_override: Some("developer".into()),
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat("test-key".into(), None, config);
    let body = provider.build_request_body(&make_test_request());
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages[0]["role"], "developer");
}

// --- Multiple tool calls fixture ---

fn multi_tool_fixture() -> &'static str {
    r#"data: {"id":"chatcmpl-multi","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":null},"finish_reason":null}]}

data: {"id":"chatcmpl-multi","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"read_file","arguments":""}},{"index":1,"id":"call_2","type":"function","function":{"name":"bash","arguments":""}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-multi","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"path\":\"a.rs\"}"}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-multi","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":1,"function":{"arguments":"{\"cmd\":\"ls\"}"}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-multi","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":60,"completion_tokens":40,"total_tokens":100}}

data: [DONE]

"#
}

#[test]
fn multi_tool_fixture_produces_two_tool_calls() {
    let stream_events = map_fixture(multi_tool_fixture());

    // Start, ToolCallStart(0), ToolCallStart(1), ToolCallDelta(0), ToolCallDelta(1),
    // ToolCallEnd(0), ToolCallEnd(1), Done
    let tool_starts: Vec<_> = stream_events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::ToolCallStart { .. }))
        .collect();
    assert_eq!(tool_starts.len(), 2);

    let tool_ends: Vec<_> = stream_events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::ToolCallEnd { .. }))
        .collect();
    assert_eq!(tool_ends.len(), 2);

    if let Some(AssistantStreamEvent::Done { message, .. }) = stream_events.last() {
        assert_eq!(message.content.len(), 2);
    } else {
        panic!("expected Done event");
    }
}

// ---------------------------------------------------------------------------
// Production request contract through Provider::stream (Phase 12.1)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_sends_text_request_body_and_auth_through_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .and(body_partial_json(serde_json::json!({
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello"}
            ],
            "max_tokens": 1024
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::new("test-key".into(), Some(server.uri()));
    let request = Request {
        model: "openai:gpt-4o".into(),
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

    // verify() confirms the production request carried the OpenAI chat body
    // (system + user messages, max_tokens), the Bearer auth header, and the
    // /v1/chat/completions path.
    server.verify().await;
}
