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
use opi_ai::provider::{CacheRetention, EventStream, Provider, validate_request_capabilities};
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
    let mut stream_events: Vec<_> = events.into_iter().flat_map(|e| mapper.process(e)).collect();
    if let Some(done) = mapper.flush_pending_done() {
        stream_events.push(done);
    }
    stream_events
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
    let mut stream_events: Vec<_> = events.into_iter().flat_map(|e| mapper.process(e)).collect();
    if let Some(done) = mapper.flush_pending_done() {
        stream_events.push(done);
    }
    stream_events
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

/// Helper: collect stream events asynchronously.
async fn collect_stream(stream: EventStream) -> Vec<AssistantStreamEvent> {
    stream.filter_map(|r| async move { r.ok() }).collect().await
}

async fn write_chunk(socket: &mut tokio::net::TcpStream, body: &str) -> std::io::Result<()> {
    let chunk = format!("{:X}\r\n{}\r\n", body.len(), body);
    tokio::io::AsyncWriteExt::write_all(socket, chunk.as_bytes()).await
}

async fn spawn_stalled_openai_chat_server() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled OpenAI Chat server");
    let addr = listener.local_addr().expect("stalled server addr");

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept stalled stream");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];

        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read request");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        let request_text = String::from_utf8_lossy(&request);
        assert!(
            request_text.starts_with("POST /v1/chat/completions "),
            "unexpected request line: {request_text}"
        );

        tokio::io::AsyncWriteExt::write_all(
            &mut socket,
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
        )
        .await
        .expect("write response headers");
        write_chunk(&mut socket, stalled_openai_chat_start_chunk())
            .await
            .expect("write initial SSE chunk");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let _ = write_chunk(&mut socket, stalled_openai_chat_terminal_chunk()).await;
        let _ = tokio::io::AsyncWriteExt::write_all(&mut socket, b"0\r\n\r\n").await;
    });

    format!("http://{addr}")
}

fn stalled_openai_chat_start_chunk() -> &'static str {
    "data: {\"id\":\"chatcmpl-slow\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n"
}

fn stalled_openai_chat_terminal_chunk() -> &'static str {
    concat!(
        "data: {\"id\":\"chatcmpl-slow\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"late\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-slow\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n",
        "data: [DONE]\n\n",
    )
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
        assert!(message.usage.is_reported());
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
        assert!(message.usage.is_reported());
        assert_eq!(message.usage.input_tokens, 50);
        assert_eq!(message.usage.output_tokens, 30);
    } else {
        panic!("expected Done event");
    }
}

// --- Response ID propagation (Phase 12 task 12.6, DoD clause 4) ---

#[test]
fn chat_chunk_id_round_trips_into_response_id() {
    let stream_events = map_fixture(text_fixture());

    if let AssistantStreamEvent::Done { message, .. } = &stream_events[5] {
        assert_eq!(
            message.response_id,
            Some("chatcmpl-abc123".to_string()),
            "OpenAI Chat chunk id must round-trip into AssistantMessage::response_id instead of being dropped"
        );
    } else {
        panic!("expected Done event");
    }
}

#[tokio::test]
async fn content_first_chunk_id_round_trips_into_response_id() {
    let provider = OpenAiChatProvider::new("key".into(), None);
    let sse = r#"data: {"id":"chatcmpl-content-first","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-content-first","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1}}

"#;

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;
    assert!(
        matches!(events.first(), Some(AssistantStreamEvent::Start { .. })),
        "content-first chunks must still emit Start before TextStart/TextDelta: {events:?}"
    );

    let (response_id, model) = events
        .into_iter()
        .find_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => {
                Some((message.response_id, message.model))
            }
            _ => None,
        })
        .expect("Done event");

    assert_eq!(response_id.as_deref(), Some("chatcmpl-content-first"));
    assert_eq!(model, "gpt-4o");
}

#[tokio::test]
async fn non_terminal_usage_chunk_updates_done_usage() {
    let provider = OpenAiChatProvider::new("key".into(), None);
    let sse = r#"data: {"id":"chatcmpl-usage-early","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}],"usage":{"prompt_tokens":7,"completion_tokens":0}}

data: {"id":"chatcmpl-usage-early","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"hello"},"finish_reason":null}],"usage":{"prompt_tokens":7,"completion_tokens":1}}

data: {"id":"chatcmpl-usage-early","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

"#;

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;
    let usage = events
        .into_iter()
        .find_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => Some(message.usage),
            _ => None,
        })
        .expect("done event");

    assert_eq!(usage.input_tokens, 7);
    assert_eq!(usage.output_tokens, 1);
}

#[tokio::test]
async fn usage_only_empty_choices_chunk_updates_done_usage() {
    let provider = OpenAiChatProvider::new("key".into(), None);
    let sse = r#"data: {"id":"chatcmpl-usage-only","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}

data: {"id":"chatcmpl-usage-only","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-usage-only","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: {"id":"chatcmpl-usage-only","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[],"usage":{"prompt_tokens":7,"completion_tokens":2}}

"#;

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;
    let usage = events
        .into_iter()
        .filter_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => Some(message.usage),
            _ => None,
        })
        .next_back()
        .expect("done event");

    assert_eq!(usage.input_tokens, 7);
    assert_eq!(usage.output_tokens, 2);
}

// --- Cache token fields (Phase 12 task 12.6, DoD clause 6) ---

fn cache_tokens_fixture() -> &'static str {
    r#"data: {"id":"chatcmpl-cache","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-cache","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":100,"completion_tokens":20,"total_tokens":120,"prompt_tokens_details":{"cached_tokens":400}}}

data: [DONE]

"#
}

#[test]
fn cache_tokens_captured_from_final_chunk() {
    let stream_events = map_fixture(cache_tokens_fixture());

    let done = stream_events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("expected Done event");
    if let AssistantStreamEvent::Done { message, .. } = done {
        assert_eq!(
            message.usage.cache_read_tokens, 400,
            "prompt_tokens_details.cached_tokens must map to cache_read_tokens"
        );
    }
}

// --- Reasoning tokens (Phase 14 task 14.5) ---

fn reasoning_tokens_fixture() -> &'static str {
    r#"data: {"id":"chatcmpl-reason","object":"chat.completion.chunk","created":1720000000,"model":"o3","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-reason","object":"chat.completion.chunk","created":1720000000,"model":"o3","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":100,"completion_tokens":500,"total_tokens":600,"completion_tokens_details":{"reasoning_tokens":300}}}

data: [DONE]

"#
}

#[test]
fn reasoning_tokens_captured_from_final_chunk() {
    let stream_events = map_fixture(reasoning_tokens_fixture());
    let done = stream_events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("expected Done event");
    if let AssistantStreamEvent::Done { message, .. } = done {
        assert_eq!(message.usage.output_tokens, 500);
        assert_eq!(message.usage.reasoning_tokens, Some(300));
        assert!(message.usage.reasoning_tokens.unwrap() <= u64::from(message.usage.output_tokens));
    }
}

#[test]
fn reasoning_absent_zero_and_equality_are_preserved() {
    let absent = reasoning_tokens_fixture().replace(
        ",\"completion_tokens_details\":{\"reasoning_tokens\":300}",
        "",
    );
    let zero =
        reasoning_tokens_fixture().replace("\"reasoning_tokens\":300", "\"reasoning_tokens\":0");
    let equal =
        reasoning_tokens_fixture().replace("\"reasoning_tokens\":300", "\"reasoning_tokens\":500");

    for (fixture, expected) in [
        (absent.as_str(), None),
        (zero.as_str(), Some(0)),
        (equal.as_str(), Some(500)),
    ] {
        let stream_events = map_fixture(fixture);
        let usage = stream_events
            .iter()
            .find_map(|event| match event {
                AssistantStreamEvent::Done { message, .. } => Some(&message.usage),
                _ => None,
            })
            .expect("expected Done event");
        assert_eq!(usage.reasoning_tokens, expected);
    }
}

fn reasoning_malformed_fixture() -> &'static str {
    // reasoning_tokens (800) > completion_tokens (500) is malformed.
    r#"data: {"id":"chatcmpl-bad","object":"chat.completion.chunk","created":1720000000,"model":"o3","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-bad","object":"chat.completion.chunk","created":1720000000,"model":"o3","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":100,"completion_tokens":500,"total_tokens":600,"completion_tokens_details":{"reasoning_tokens":800}}}

data: [DONE]

"#
}

#[tokio::test]
async fn reasoning_malformed_subset_stops_production_stream_with_non_retryable_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(reasoning_malformed_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::new("test-key".into(), Some(server.uri()));
    let results: Vec<_> = provider.stream(make_test_request()).collect().await;
    let errors: Vec<_> = results
        .iter()
        .filter_map(|result| result.as_ref().err())
        .collect();
    assert_eq!(errors.len(), 1, "invalid usage must emit one error");
    assert!(matches!(errors[0], ProviderError::StreamError(_)));
    assert!(!errors[0].is_retryable());
    assert!(matches!(
        results.last(),
        Some(Err(ProviderError::StreamError(_)))
    ));
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
    // Final chunk carries finish_reason but no `usage` field.
    r#"data: {"id":"chatcmpl-nousage","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-nousage","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"ok"}}]}

data: {"id":"chatcmpl-nousage","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]

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
        // No usage chunk in the stream -> zero usage, no panic. This path is shared
        // by OpenRouter/Mistral/Azure, which inherit the OpenAI Chat mapper.
        assert!(!message.usage.is_reported());
        assert_eq!(message.usage.input_tokens, 0);
        assert_eq!(message.usage.output_tokens, 0);
        assert_eq!(message.usage.cache_read_tokens, 0);
        assert_eq!(message.usage.cache_write_tokens, 0);
    }
}

fn reported_zero_usage_fixture() -> &'static str {
    r#"data: {"id":"chatcmpl-zero-usage","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-zero-usage","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"ok"}}]}

data: {"id":"chatcmpl-zero-usage","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":0,"completion_tokens":0,"total_tokens":0}}

data: [DONE]

"#
}

#[test]
fn reported_zero_usage_remains_reported() {
    let stream_events = map_fixture(reported_zero_usage_fixture());

    let done = stream_events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("expected Done event");
    if let AssistantStreamEvent::Done { message, .. } = done {
        assert!(
            message.usage.is_reported(),
            "provider-reported zero usage must stay distinct from missing usage"
        );
        assert_eq!(message.usage.input_tokens, 0);
        assert_eq!(message.usage.output_tokens, 0);
        assert_eq!(message.usage.cache_read_tokens, 0);
        assert_eq!(message.usage.cache_write_tokens, 0);
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
        let err = message.error_message.as_ref().unwrap();
        assert!(
            err.contains("openai chat stream error"),
            "error_message must be the neutral literal, got: {err}"
        );
        assert!(
            !err.contains("Rate limit"),
            "raw upstream error text must not leak into the public error_message: {err}"
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
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
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

#[test]
fn build_request_body_usage_in_stream_emits_stream_options() {
    let provider = OpenAiChatProvider::new_for_profile(
        "key".into(),
        "https://example.test".into(),
        "compat".into(),
        CompatConfig {
            usage_in_stream: true,
            ..CompatConfig::default()
        },
        vec![],
        vec![ModelInfo::new(
            "model",
            "model",
            opi_ai::WireApi::OpenAiCompletions,
            ModelCapabilities::new(128000, 4096).with_streaming(true),
        )],
    );

    let mut request = make_test_request();
    request.model = "compat:model".into();
    let body = provider.build_request_body(&request);
    assert_eq!(body["stream_options"]["include_usage"], true);
}

// --- Phase 12 task 12.3 — OpenAI-compatible profile compatibility flags ---
//
// DoD: config-driven profile metadata represents strict tool schema, reasoning
// effort, cache control, session-affinity headers, assistant-after-tool-result,
// and provider/model override precedence. These tests pin the representation
// (CompatConfig fields) and the wire effects for the flags whose semantics the
// modern OpenAI Chat Completions API defines.

use opi_ai::message::{ImageSource, MediaType, OutputContent, ToolDef, ToolResultMessage};
use opi_ai::provider::{ModelInfo, ProviderError, ProviderErrorCategory};
use opi_ai::registry::ModelCapabilities;

#[test]
fn compat_config_represents_strict_tool_schema() {
    let config = CompatConfig {
        strict_tool_schema: true,
        ..Default::default()
    };
    assert!(config.strict_tool_schema);
}

#[test]
fn compat_config_represents_reasoning_effort() {
    let config = CompatConfig {
        reasoning_effort: Some("high".into()),
        ..Default::default()
    };
    assert_eq!(config.reasoning_effort.as_deref(), Some("high"));
}

#[test]
fn compat_config_represents_cache_key() {
    let config = CompatConfig {
        cache_key: Some("sess-abc".into()),
        ..Default::default()
    };
    assert_eq!(config.cache_key.as_deref(), Some("sess-abc"));
}

#[test]
fn compat_config_represents_assistant_after_tool_result() {
    // Metadata-only flag: modern OpenAI Chat Completions does not require a
    // synthetic assistant turn after a tool result, so the shared adapter
    // records the flag for compatibility metadata without altering wire
    // ordering. Any compatible provider requiring legacy synthesis would need a
    // reviewed first-class adapter (Phase 12 non-goal guard).
    let config = CompatConfig {
        require_assistant_after_tool_result: true,
        ..Default::default()
    };
    assert!(config.require_assistant_after_tool_result);
}

fn make_tool_request() -> Request {
    Request {
        model: "openai:gpt-4o".into(),
        system: Some("You are helpful.".into()),
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "list files".into(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![ToolDef {
            name: "list_dir".into(),
            description: "list a directory".into(),
            input_schema: serde_json::json!({"type":"object","properties":{}}),
        }],
        max_tokens: Some(1024),
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

#[test]
fn build_request_body_strict_tool_schema_emits_strict_flag() {
    let config = CompatConfig {
        strict_tool_schema: true,
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat("test-key".into(), None, config);
    let body = provider.build_request_body(&make_tool_request());
    let tools = body["tools"].as_array().expect("tools array present");
    assert!(
        tools[0]["function"]["strict"] == true,
        "strict flag must be emitted when configured: {tools:?}"
    );
}

#[test]
fn build_request_body_strict_tool_schema_off_by_default() {
    let provider = OpenAiChatProvider::new("test-key".into(), None);
    let body = provider.build_request_body(&make_tool_request());
    let tools = body["tools"].as_array().expect("tools array present");
    assert!(
        tools[0]["function"].get("strict").is_none(),
        "strict must be absent by default: {tools:?}"
    );
}

#[test]
fn static_compat_reasoning_effort_does_not_drive_wire_output() {
    let config = CompatConfig {
        reasoning_effort: Some("high".into()),
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat("test-key".into(), None, config);
    let body = provider.build_request_body(&make_test_request());
    assert!(body.get("reasoning_effort").is_none());
}

#[test]
fn build_request_body_reasoning_effort_absent_by_default() {
    let provider = OpenAiChatProvider::new("test-key".into(), None);
    let body = provider.build_request_body(&make_test_request());
    assert!(body.get("reasoning_effort").is_none());
}

#[test]
fn chat_reasoning_effort_uses_request_thinking_and_model_map() {
    use opi_ai::{ThinkingLevel, ThinkingLevelMap, ThinkingLevelMapping};

    let provider = OpenAiChatProvider::new("test-key".into(), None);
    let mut identity = make_test_request();
    identity.model = "openai:o3".into();
    identity.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: None,
        level: ThinkingLevel::High,
    };
    assert_eq!(
        provider.build_request_body(&identity)["reasoning_effort"],
        "high"
    );

    let mapped_model = ModelInfo::new(
        "mapped",
        "Mapped",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(128_000, 16_384)
            .with_streaming(true)
            .with_thinking(true),
    )
    .with_thinking_level_map(ThinkingLevelMap::reasoning_default().with_mapping(
        ThinkingLevel::High,
        ThinkingLevelMapping::Mapped("provider-high".into()),
    ));
    let mapped_provider = OpenAiChatProvider::new_for_profile(
        "key".into(),
        "https://example.test".into(),
        "mapped".into(),
        CompatConfig {
            reasoning_effort: Some("static-must-not-win".into()),
            ..Default::default()
        },
        vec![],
        vec![mapped_model],
    );
    let mut remapped = identity;
    remapped.model = "mapped:mapped".into();
    assert_eq!(
        mapped_provider.build_request_body(&remapped)["reasoning_effort"],
        "provider-high"
    );

    let mut disabled = remapped;
    disabled.thinking.enabled = false;
    assert!(
        mapped_provider
            .build_request_body(&disabled)
            .get("reasoning_effort")
            .is_none()
    );

    let mut off = disabled;
    off.thinking.enabled = true;
    off.thinking.level = ThinkingLevel::None;
    assert!(
        mapped_provider
            .build_request_body(&off)
            .get("reasoning_effort")
            .is_none()
    );

    let unsupported_model = ModelInfo::new(
        "unsupported",
        "Unsupported",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(128_000, 16_384)
            .with_streaming(true)
            .with_thinking(true),
    )
    .with_thinking_level_map(ThinkingLevelMap::disabled());
    let unsupported_provider = OpenAiChatProvider::new_for_profile(
        "key".into(),
        "https://example.test".into(),
        "unsupported".into(),
        Default::default(),
        vec![],
        vec![unsupported_model],
    );
    let mut unsupported = off;
    unsupported.model = "unsupported:unsupported".into();
    unsupported.thinking.level = ThinkingLevel::High;
    assert!(
        unsupported_provider
            .build_request_body(&unsupported)
            .get("reasoning_effort")
            .is_none()
    );
}

#[test]
fn build_request_body_cache_key_emits_prompt_cache_key() {
    let config = CompatConfig {
        cache_key: Some("sess-abc".into()),
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat("test-key".into(), None, config);
    let body = provider.build_request_body(&make_test_request());
    assert_eq!(body["prompt_cache_key"], "sess-abc");
}

#[test]
fn build_request_body_cache_key_absent_by_default() {
    let provider = OpenAiChatProvider::new("test-key".into(), None);
    let body = provider.build_request_body(&make_test_request());
    assert!(body.get("prompt_cache_key").is_none());
}

fn make_tool_result_request() -> Request {
    Request {
        model: "openai:gpt-4o".into(),
        system: None,
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "run it".into(),
                }],
                timestamp_ms: 0,
            }),
            Message::ToolResult(ToolResultMessage {
                tool_call_id: "call_1".into(),
                tool_name: "list_dir".into(),
                content: vec![OutputContent::Text {
                    text: "a.rs".into(),
                }],
                details: None,
                is_error: false,
                truncated: false,
                timestamp_ms: 0,
            }),
        ],
        tools: vec![],
        max_tokens: Some(1024),
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

#[test]
fn build_request_body_tool_result_name_field_wire_shape() {
    let config = CompatConfig {
        tool_result_name_field: true,
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat("test-key".into(), None, config);
    let body = provider.build_request_body(&make_tool_result_request());
    let messages = body["messages"].as_array().unwrap();
    let tool_msg = messages
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("tool message present");
    assert_eq!(tool_msg["tool_call_id"], "call_1");
    assert_eq!(
        tool_msg["name"], "list_dir",
        "name field must be present when tool_result_name_field is set"
    );
    assert_eq!(tool_msg["content"], "a.rs");
}

#[test]
fn build_request_body_tool_result_name_field_absent_by_default() {
    let provider = OpenAiChatProvider::new("test-key".into(), None);
    let body = provider.build_request_body(&make_tool_result_request());
    let messages = body["messages"].as_array().unwrap();
    let tool_msg = messages
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("tool message present");
    assert!(
        tool_msg.get("name").is_none(),
        "name field must be absent by default"
    );
}

#[test]
fn validate_rejects_image_on_text_only_openai_compatible_profile() {
    // A config-driven OpenAI-compatible profile advertising a text-only model
    // rejects image input through the shared capability preflight before the
    // live call. This proves the unsupported-capability diagnostic flows from
    // the shared profile path (DoD), not a per-adapter special case.
    let text_only = ModelInfo::new(
        "text-only-model",
        "Text Only",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(8000, 1024).with_streaming(true),
    );
    let provider = OpenAiChatProvider::new_for_profile(
        "test-key".into(),
        "https://example.test".into(),
        "compatprof".into(),
        CompatConfig::default(),
        vec![],
        vec![text_only],
    );
    let image_request = Request {
        model: "compatprof:text-only-model".into(),
        system: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Image {
                source: ImageSource::Url {
                    url: "https://example.test/i.png".into(),
                },
                media_type: MediaType::Png,
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
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };
    let err = validate_request_capabilities(&provider, &image_request)
        .expect_err("text-only profile model must reject image preflight");
    assert_eq!(err.category(), ProviderErrorCategory::Capability);
    assert!(
        matches!(err, ProviderError::UnsupportedCapability(_)),
        "must be UnsupportedCapability: {err:?}"
    );
}

#[test]
fn model_level_override_takes_precedence_over_provider_profile() {
    // Phase 12 task 12.3: provider/model override precedence. The profile sets
    // a provider-level default (system role, max_tokens). A model-level override
    // wins for the model that declares it; other models inherit the profile
    // default. Provider-level flags (strict_tool_schema) apply to all models.
    use opi_ai::openai_chat::ModelCompatOverride;
    let base = CompatConfig {
        system_role_override: Some("system".into()),
        max_tokens_field: "max_tokens".into(),
        strict_tool_schema: true,
        ..Default::default()
    };
    let mut overrides = std::collections::HashMap::new();
    overrides.insert(
        "o3".into(),
        ModelCompatOverride {
            system_role_override: Some("developer".into()),
            max_tokens_field: Some("max_completion_tokens".into()),
        },
    );
    let provider = OpenAiChatProvider::new_for_profile(
        "key".into(),
        "https://example.test".into(),
        "prof".into(),
        base,
        vec![],
        vec![],
    )
    .with_model_overrides(overrides);

    // o3: model-level override wins for role + max_tokens field.
    let mut req_o3 = make_tool_request();
    req_o3.model = "prof:o3".into();
    let body = provider.build_request_body(&req_o3);
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages[0]["role"], "developer", "model override wins");
    assert!(
        body.get("max_completion_tokens").is_some(),
        "model override selects max_completion_tokens"
    );
    assert!(body.get("max_tokens").is_none());
    // Provider-level strict_tool_schema still applies under the override.
    let tools = body["tools"].as_array().expect("tools present");
    assert!(
        tools[0]["function"]["strict"] == true,
        "provider-level strict still applies"
    );

    // gpt-4o: no model override -> profile default applies.
    let mut req_plain = make_tool_request();
    req_plain.model = "prof:gpt-4o".into();
    let body2 = provider.build_request_body(&req_plain);
    let messages2 = body2["messages"].as_array().unwrap();
    assert_eq!(
        messages2[0]["role"], "system",
        "profile default applies when no model override"
    );
    assert!(
        body2.get("max_tokens").is_some(),
        "profile default max_tokens_field applies"
    );
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

// --- Phase 12 task 12.4 — malformed tool-call arguments (scenario 5) ---
//
// DoD: malformed JSON arguments reach agent/runtime validation, not provider
// panics. The OpenAI Chat mapper accumulates the raw `arguments` string without
// parsing, so a malformed value is preserved byte-for-byte and handed to the
// agent loop. (Scenario 3 multi-tool is already covered by
// `multi_tool_fixture_produces_two_tool_calls` above.)

fn malformed_tool_args_fixture() -> &'static str {
    r#"data: {"id":"chatcmpl-bad","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":null},"finish_reason":null}]}

data: {"id":"chatcmpl-bad","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_bad","type":"function","function":{"name":"read_file","arguments":""}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-bad","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{not-json"}}]},"finish_reason":null}]}

data: {"id":"chatcmpl-bad","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":5,"completion_tokens":3,"total_tokens":8}}

data: [DONE]

"#
}

#[test]
fn malformed_tool_args_pass_raw_string_without_panic() {
    let stream_events = map_fixture(malformed_tool_args_fixture());

    let end = stream_events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::ToolCallEnd { tool_call, .. } => Some(tool_call.clone()),
            _ => None,
        })
        .expect("ToolCallEnd emitted despite malformed argument JSON");
    assert_eq!(end.arguments, "{not-json");
    assert_eq!(end.id, "call_bad");
    assert_eq!(end.name, "read_file");
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
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
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

#[tokio::test]
async fn profile_extra_headers_reach_the_http_wire() {
    // Phase 12 task 12.3 (DoD "headers ... from the shared profile path"):
    // a config-driven OpenAI-compatible profile's session-affinity headers,
    // threaded through `new_for_profile` as `extra_headers`, are attached to
    // every outbound HTTP request by the shared adapter.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .and(header("X-Session-Id", "sess-1"))
        .and(header("X-Affinity", "region-eu"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::new_for_profile(
        "test-key".into(),
        server.uri(),
        "affinityprof".into(),
        CompatConfig::default(),
        vec![
            ("X-Session-Id".into(), "sess-1".into()),
            ("X-Affinity".into(), "region-eu".into()),
        ],
        vec![],
    );
    let request = Request {
        model: "affinityprof:any-model".into(),
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
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };

    let mut stream = provider.stream(request);
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) if event.is_terminal() => break,
            Err(_) => break,
            _ => {}
        }
    }

    // verify() confirms both profile-declared session-affinity headers were
    // sent on the production HTTP request through the shared adapter path.
    server.verify().await;
}

// ---------------------------------------------------------------------------
// Provider stream cancellation (Phase 12 task 12.7 DoD clause 6)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_cancellation_aborts_before_completion() {
    // The CancellationToken is threaded into the OpenAI Chat adapter's HTTP
    // body-stream loop (openai_chat.rs `cancel.cancelled()` select arm).
    // This fixture sends one Start-producing chunk, then stalls for a full
    // second before the terminal chunk. Cancellation must close the stream well
    // before that delayed terminal data could arrive.
    let server = spawn_stalled_openai_chat_server().await;

    let cancel = CancellationToken::new();
    let provider = OpenAiChatProvider::new("test-key".into(), Some(server));
    let request = Request {
        model: "openai:gpt-4o".into(),
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
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };
    let mut stream = provider.stream(request);

    let first = stream
        .next()
        .await
        .expect("stream should produce at least one event")
        .expect("first event should be valid");
    assert!(matches!(first, AssistantStreamEvent::Start { .. }));
    cancel.cancel();

    let next = tokio::time::timeout(std::time::Duration::from_millis(200), stream.next())
        .await
        .expect("stream must surface cancellation before the delayed terminal fixture completes");
    match next {
        Some(Err(ProviderError::Cancelled)) => { /* typed cancellation, as contracted */ }
        other => {
            panic!("expected Err(ProviderError::Cancelled) on cancellation, got: {other:?}")
        }
    }
}
