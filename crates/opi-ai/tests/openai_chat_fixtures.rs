//! Behavioral tests for task 2.1: OpenAI-compatible chat provider.
//!
//! DoD: "fixtures cover text, tool call, usage, error; implements Provider trait
//! with SSE streaming; exposes compat config points for role mapping
//! (developer/system), usage-in-stream, max_tokens field naming, and tool_result
//! name field so downstream profiles (OpenRouter, Mistral) can override behavior"
//!
//! All tests use fixture strings — no live provider calls (red flag #10).

use futures_util::StreamExt;
use opi_ai::message::{AssistantContent, AssistantMessage, ToolCall};
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
            ParsedEvent::UsageError(_) => Vec::new(),
            ParsedEvent::Malformed { .. } => Vec::new(),
        })
        .collect();
    let mut mapper = OpenAiChatMapper::new(
        opi_ai::ApiKind::OpenAi,
        "openai",
        opi_ai::model_info::WireApi::OpenAiCompletions,
        "",
    );
    let mut stream_events: Vec<_> = events
        .into_iter()
        .flat_map(|e| mapper.process(e))
        .map(|item| item.expect("fixture maps to stream events"))
        .collect();
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
            ParsedEvent::UsageError(_) => Vec::new(),
            ParsedEvent::Malformed { .. } => Vec::new(),
        })
        .collect();
    let mut mapper = OpenAiChatMapper::new(
        api,
        provider,
        opi_ai::model_info::WireApi::OpenAiCompletions,
        "",
    );
    let mut stream_events: Vec<_> = events
        .into_iter()
        .flat_map(|e| mapper.process(e))
        .map(|item| item.expect("fixture maps to stream events"))
        .collect();
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
            ParsedEvent::UsageError(_) => Vec::new(),
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
    let provider = OpenAiChatProvider::new(None);
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
            AssistantStreamEvent::Done { message, .. } => Some((
                message.response_id,
                message.response_model.unwrap_or_default(),
            )),
            _ => None,
        })
        .expect("Done event");

    assert_eq!(response_id.as_deref(), Some("chatcmpl-content-first"));
    // The provider-reported model is a separate fact from the requested
    // identity (AIR-010/RP-02): it now rides response_model.
    assert_eq!(model, "gpt-4o");
}

#[tokio::test]
async fn non_terminal_usage_chunk_updates_done_usage() {
    let provider = OpenAiChatProvider::new(None);
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
    let provider = OpenAiChatProvider::new(None);
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

    let provider = OpenAiChatProvider::new(Some(server.uri()));
    let results: Vec<_> = provider
        .stream_prepared(
            make_test_request(),
            opi_ai::test_support::resolved_bearer_auth(),
        )
        .collect()
        .await;
    let errors: Vec<_> = results
        .iter()
        .filter_map(|result| result.as_ref().err())
        .collect();
    assert_eq!(errors.len(), 1, "invalid usage must emit one error");
    assert!(errors[0].category() == opi_ai::provider::ProviderErrorCategory::Stream);
    assert_eq!(
        errors[0].to_string(),
        "stream error: reasoning_tokens (800) exceeds completion_tokens (500)"
    );
    assert!(!errors[0].is_retryable());
    assert!(matches!(
        results.last(),
        Some(Err(cause)) if cause.category() == opi_ai::provider::ProviderErrorCategory::Stream
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
    let events: Vec<OpenAiChatEvent> = parse_sse_events(error_fixture())
        .flat_map(|p| match p {
            ParsedEvent::Valid(evts) => evts,
            ParsedEvent::UsageError(_) => Vec::new(),
            ParsedEvent::Malformed { .. } => Vec::new(),
        })
        .collect();
    let mut mapper = OpenAiChatMapper::new(
        opi_ai::ApiKind::OpenAi,
        "openai",
        opi_ai::model_info::WireApi::OpenAiCompletions,
        "",
    );
    let items: Vec<_> = events.into_iter().flat_map(|e| mapper.process(e)).collect();

    assert_eq!(items.len(), 1);
    // Current producers stop the dual terminal format: the in-band error
    // event is a stream failure with a bounded typed cause.
    match &items[0] {
        Err(cause) if cause.category() == opi_ai::provider::ProviderErrorCategory::Stream => {
            let summary = cause.to_string();
            assert!(
                summary.contains("openai chat stream error"),
                "cause must be the neutral literal, got: {summary}"
            );
            assert!(
                !summary.contains("Rate limit"),
                "raw upstream error text must not leak into the cause: {summary}"
            );
        }
        other => panic!("expected stream error item, got {other:?}"),
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
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0], OpenAiChatEvent::RoleDelta { .. }));
    assert!(matches!(&events[1], OpenAiChatEvent::ContentDelta { content, .. } if content == "Hi"));
}

// --- Provider Tests ---

#[test]
fn openai_chat_provider_id() {
    let provider = OpenAiChatProvider::new(None);
    assert_eq!(provider.id(), "openai");
}

#[test]
fn openai_chat_provider_models_not_empty() {
    let provider = OpenAiChatProvider::new(None);
    assert!(!provider.models().is_empty());
}

#[tokio::test]
async fn stream_from_sse_produces_events() {
    let provider = OpenAiChatProvider::new(None);
    let cancel = tokio_util::sync::CancellationToken::new();
    let mut stream = provider.stream_from_sse(text_fixture(), cancel);

    let first = stream.next().await.expect("should have an event");
    assert!(first.is_ok());
    assert!(matches!(first.unwrap(), AssistantStreamEvent::Start { .. }));
}

// --- Compat Config Tests ---
// Field-representation assertions were merged into the wire-effect tests
// above and the task 20.36 normalization block (V-11 disposition); the
// retired `system_role_override`/static `reasoning_effort` keys are exercised
// as typed migration errors in the catalog/config parser suites.

// --- Build Request Body Tests ---

use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::Request;
use opi_ai::provider::ThinkingConfig;
use tokio_util::sync::CancellationToken;

fn make_test_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
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
    }
}

#[test]
fn build_request_body_standard_openai_uses_max_completion_tokens() {
    // Pi 1.3.0 baseline: the standard OpenAI endpoint sends
    // `max_completion_tokens`; `max_tokens` remains the explicit override for
    // legacy-compatible endpoints (deepseek/zai/together/…).
    let provider = OpenAiChatProvider::new(None);
    let body = provider.build_request_body(&make_test_request()).unwrap();
    assert!(body.get("max_tokens").is_none());
    assert_eq!(body["max_completion_tokens"], 4096);
}

#[test]
fn build_request_body_with_compat_max_completion_tokens() {
    let config = CompatConfig {
        max_tokens_field: Some("max_completion_tokens".into()),
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat(None, config);
    let body = provider.build_request_body(&make_test_request()).unwrap();
    assert!(body.get("max_tokens").is_none());
    assert_eq!(body["max_completion_tokens"], 4096);
}

#[test]
fn build_request_body_system_role_default() {
    let provider = OpenAiChatProvider::new(None);
    let body = provider.build_request_body(&make_test_request()).unwrap();
    // Default: system message uses "system" role
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages[0]["role"], "system");
}

#[test]
fn build_request_body_developer_role_override() {
    let config = CompatConfig {
        instruction_role: Some(ChatInstructionRole::Developer),
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat(None, config);
    let body = provider.build_request_body(&make_test_request()).unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages[0]["role"], "developer");
}

#[test]
fn build_request_body_usage_disabled_omits_stream_options() {
    let provider = OpenAiChatProvider::new_for_profile(
        "https://example.test".into(),
        "compat".into(),
        CompatConfig {
            usage_in_stream: Some(false),
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
    let body = provider.build_request_body(&request).unwrap();
    assert!(body.get("stream_options").is_none());
}

// ---------------------------------------------------------------------------
// Task 20.36 (AIR-022 / V-11): single compat normalization — store, role and
// field defaults from the adapter's frozen provider inference, with explicit
// config overriding inference per field.
// ---------------------------------------------------------------------------

use opi_ai::model_info::{ChatInstructionRole, ChatStoreField};

fn thinking_chat_model(id: &str, thinking: bool) -> ModelInfo {
    let mut model = ModelInfo::new(
        id,
        id,
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(128_000, 16_384)
            .with_streaming(true)
            .with_thinking(thinking),
    );
    if thinking {
        model = model.with_thinking_level_map(opi_ai::ThinkingLevelMap::reasoning_default());
    }
    model
}

fn profile_provider(
    provider_id: &str,
    compat: opi_ai::openai_chat::CompatConfig,
    models: Vec<ModelInfo>,
) -> OpenAiChatProvider {
    OpenAiChatProvider::new_for_profile(
        format!("https://{provider_id}.example.test"),
        provider_id.into(),
        compat,
        vec![],
        models,
    )
}

fn request_for(provider: &str, model: &str) -> Request {
    let mut request = make_test_request();
    request.model = format!("{provider}:{model}");
    request
}

fn thinking_request(provider: &str, model: &str, level: Option<opi_ai::ThinkingLevel>) -> Request {
    let mut request = request_for(provider, model);
    request.thinking = ThinkingConfig {
        enabled: level.is_some(),
        budget_tokens: None,
        level: level.unwrap_or(opi_ai::ThinkingLevel::None),
    };
    request
}

#[test]
fn chat_zai_format_projects_thinking_toggle_and_mapped_effort() {
    // zai is inference-driven: provider id zai selects the zai format.
    let provider = profile_provider(
        "zai",
        Default::default(),
        vec![thinking_chat_model("glm-4.6", true)],
    );

    let body = provider
        .build_request_body(&thinking_request(
            "zai",
            "glm-4.6",
            Some(opi_ai::ThinkingLevel::High),
        ))
        .unwrap();
    assert_eq!(
        body["thinking"],
        serde_json::json!({"type": "enabled", "clear_thinking": false})
    );
    // The zai baseline never sends the standard effort field.
    assert!(body.get("reasoning_effort").is_none());

    let body = provider
        .build_request_body(&thinking_request("zai", "glm-4.6", None))
        .unwrap();
    assert_eq!(body["thinking"], serde_json::json!({"type": "disabled"}));
}

#[test]
fn chat_qwen_format_projects_enable_thinking_toggle() {
    use opi_ai::model_info::ChatThinkingFormat;
    let compat = CompatConfig {
        thinking_format: Some(ChatThinkingFormat::Qwen),
        ..Default::default()
    };
    let provider = profile_provider(
        "qwen-host",
        compat,
        vec![thinking_chat_model("qwen3", true)],
    );

    let body = provider
        .build_request_body(&thinking_request(
            "qwen-host",
            "qwen3",
            Some(opi_ai::ThinkingLevel::High),
        ))
        .unwrap();
    assert_eq!(body["enable_thinking"], serde_json::json!(true));
    // Standard endpoints accept the effort field alongside the toggle.
    assert_eq!(body["reasoning_effort"], serde_json::json!("high"));

    let body = provider
        .build_request_body(&thinking_request("qwen-host", "qwen3", None))
        .unwrap();
    assert_eq!(body["enable_thinking"], serde_json::json!(false));
    assert!(body.get("reasoning_effort").is_none());
}

#[test]
fn chat_qwen_chat_template_format_projects_fixed_kwargs() {
    use opi_ai::model_info::ChatThinkingFormat;
    let compat = CompatConfig {
        thinking_format: Some(ChatThinkingFormat::QwenChatTemplate),
        ..Default::default()
    };
    let provider = profile_provider(
        "qwen-ct",
        compat,
        vec![thinking_chat_model("qwen3-ct", true)],
    );

    let body = provider
        .build_request_body(&thinking_request(
            "qwen-ct",
            "qwen3-ct",
            Some(opi_ai::ThinkingLevel::High),
        ))
        .unwrap();
    assert_eq!(
        body["chat_template_kwargs"],
        serde_json::json!({"enable_thinking": true, "preserve_thinking": true})
    );

    let body = provider
        .build_request_body(&thinking_request("qwen-ct", "qwen3-ct", None))
        .unwrap();
    assert_eq!(
        body["chat_template_kwargs"],
        serde_json::json!({"enable_thinking": false, "preserve_thinking": true})
    );
}

#[test]
fn chat_template_format_resolves_dollar_var_kwargs() {
    use opi_ai::model_info::{ChatTemplateKwargValue, ChatTemplateKwargVar, ChatThinkingFormat};
    let mut kwargs = std::collections::BTreeMap::new();
    kwargs.insert(
        "enable_thinking".into(),
        ChatTemplateKwargValue::Var {
            var: ChatTemplateKwargVar::ThinkingEnabled,
            omit_when_off: false,
        },
    );
    kwargs.insert(
        "effort".into(),
        ChatTemplateKwargValue::Var {
            var: ChatTemplateKwargVar::ThinkingEffort,
            omit_when_off: true,
        },
    );
    kwargs.insert(
        "static".into(),
        ChatTemplateKwargValue::Literal(serde_json::json!(42)),
    );
    let compat = CompatConfig {
        thinking_format: Some(ChatThinkingFormat::ChatTemplate),
        chat_template_kwargs: Some(kwargs),
        ..Default::default()
    };
    let provider = profile_provider("ct", compat, vec![thinking_chat_model("ct-model", true)]);

    let body = provider
        .build_request_body(&thinking_request(
            "ct",
            "ct-model",
            Some(opi_ai::ThinkingLevel::Medium),
        ))
        .unwrap();
    assert_eq!(
        body["chat_template_kwargs"],
        serde_json::json!({"enable_thinking": true, "effort": "medium", "static": 42})
    );

    // Off drops the omitWhenOff key and disables the toggle.
    let body = provider
        .build_request_body(&thinking_request("ct", "ct-model", None))
        .unwrap();
    assert_eq!(
        body["chat_template_kwargs"],
        serde_json::json!({"enable_thinking": false, "static": 42})
    );
}

#[test]
fn chat_baseten_format_projects_template_args_and_off_effort() {
    use opi_ai::model_info::{ChatTemplateKwargValue, ChatTemplateKwargVar, ChatThinkingFormat};
    let mut args = std::collections::BTreeMap::new();
    args.insert(
        "thinking".into(),
        ChatTemplateKwargValue::Var {
            var: ChatTemplateKwargVar::ThinkingEnabled,
            omit_when_off: false,
        },
    );
    let compat = CompatConfig {
        thinking_format: Some(ChatThinkingFormat::Baseten),
        chat_template_args: Some(args),
        ..Default::default()
    };
    let model = thinking_chat_model("b1", true).with_thinking_level_map(
        opi_ai::ThinkingLevelMap::reasoning_default().with_mapping(
            opi_ai::ThinkingLevel::None,
            opi_ai::ThinkingLevelMapping::Mapped("off-value".into()),
        ),
    );
    let provider = profile_provider("baseten", compat, vec![model]);

    let body = provider
        .build_request_body(&thinking_request(
            "baseten",
            "b1",
            Some(opi_ai::ThinkingLevel::Low),
        ))
        .unwrap();
    assert_eq!(
        body["chat_template_args"],
        serde_json::json!({"thinking": true})
    );
    assert_eq!(body["reasoning_effort"], serde_json::json!("low"));

    let body = provider
        .build_request_body(&thinking_request("baseten", "b1", None))
        .unwrap();
    assert_eq!(
        body["chat_template_args"],
        serde_json::json!({"thinking": false})
    );
    // Off sends the model's declared off value.
    assert_eq!(body["reasoning_effort"], serde_json::json!("off-value"));
}

#[test]
fn chat_deepseek_format_gates_disabled_on_off_mapping() {
    // deepseek format via inference; the reasoning_content replay marker is
    // covered separately.
    let provider = profile_provider(
        "deepseek",
        Default::default(),
        vec![thinking_chat_model("r1", true)],
    );

    let body = provider
        .build_request_body(&thinking_request(
            "deepseek",
            "r1",
            Some(opi_ai::ThinkingLevel::High),
        ))
        .unwrap();
    assert_eq!(body["thinking"], serde_json::json!({"type": "enabled"}));
    assert_eq!(body["reasoning_effort"], serde_json::json!("high"));

    // reasoning_default maps off to identity (no explicit off value): the
    // disabled marker is sent.
    let body = provider
        .build_request_body(&thinking_request("deepseek", "r1", None))
        .unwrap();
    assert_eq!(body["thinking"], serde_json::json!({"type": "disabled"}));

    // A model whose map cannot represent off (Pi `off: null`) never receives
    // the disable marker.
    let null_off = thinking_chat_model("r1-null", true).with_thinking_level_map(
        opi_ai::ThinkingLevelMap::reasoning_default().with_mapping(
            opi_ai::ThinkingLevel::None,
            opi_ai::ThinkingLevelMapping::Unsupported,
        ),
    );
    let provider = profile_provider("deepseek", Default::default(), vec![null_off]);
    let body = provider
        .build_request_body(&thinking_request("deepseek", "r1-null", None))
        .unwrap();
    assert!(body.get("thinking").is_none(), "off-null never disables");
}

#[test]
fn chat_openrouter_format_projects_nested_reasoning_effort() {
    let provider = profile_provider(
        "openrouter",
        Default::default(),
        vec![thinking_chat_model("some/model", true)],
    );

    let body = provider
        .build_request_body(&thinking_request(
            "openrouter",
            "some/model",
            Some(opi_ai::ThinkingLevel::High),
        ))
        .unwrap();
    assert_eq!(body["reasoning"], serde_json::json!({"effort": "high"}));
    assert!(body.get("reasoning_effort").is_none());

    // Off with no declared off value falls back to the format default.
    let body = provider
        .build_request_body(&thinking_request("openrouter", "some/model", None))
        .unwrap();
    assert_eq!(body["reasoning"], serde_json::json!({"effort": "none"}));

    // An explicit off value wins over the default.
    let mapped = thinking_chat_model("mapped/model", true).with_thinking_level_map(
        opi_ai::ThinkingLevelMap::reasoning_default().with_mapping(
            opi_ai::ThinkingLevel::None,
            opi_ai::ThinkingLevelMapping::Mapped("disable".into()),
        ),
    );
    let provider = profile_provider("openrouter", Default::default(), vec![mapped]);
    let body = provider
        .build_request_body(&thinking_request("openrouter", "mapped/model", None))
        .unwrap();
    assert_eq!(body["reasoning"], serde_json::json!({"effort": "disable"}));
}

#[test]
fn chat_ant_ling_format_only_sends_mapped_effort() {
    let provider = profile_provider(
        "ant-ling",
        Default::default(),
        vec![thinking_chat_model("ant-1", true)],
    );

    let body = provider
        .build_request_body(&thinking_request(
            "ant-ling",
            "ant-1",
            Some(opi_ai::ThinkingLevel::High),
        ))
        .unwrap();
    assert_eq!(body["reasoning"], serde_json::json!({"effort": "high"}));

    // Off sends nothing: the ant-ling branch is effort-gated.
    let body = provider
        .build_request_body(&thinking_request("ant-ling", "ant-1", None))
        .unwrap();
    assert!(body.get("reasoning").is_none());
}

#[test]
fn chat_together_format_projects_reasoning_enabled() {
    let provider = profile_provider(
        "together",
        Default::default(),
        vec![thinking_chat_model("t-model", true)],
    );

    let body = provider
        .build_request_body(&thinking_request(
            "together",
            "t-model",
            Some(opi_ai::ThinkingLevel::High),
        ))
        .unwrap();
    assert_eq!(body["reasoning"], serde_json::json!({"enabled": true}));
    // The together baseline never sends the standard effort field.
    assert!(body.get("reasoning_effort").is_none());

    let body = provider
        .build_request_body(&thinking_request("together", "t-model", None))
        .unwrap();
    assert_eq!(body["reasoning"], serde_json::json!({"enabled": false}));
}

#[test]
fn chat_string_thinking_format_projects_thinking_string() {
    use opi_ai::model_info::ChatThinkingFormat;
    let compat = CompatConfig {
        thinking_format: Some(ChatThinkingFormat::StringThinking),
        ..Default::default()
    };
    let provider = profile_provider("st", compat, vec![thinking_chat_model("st-model", true)]);

    let body = provider
        .build_request_body(&thinking_request(
            "st",
            "st-model",
            Some(opi_ai::ThinkingLevel::Low),
        ))
        .unwrap();
    assert_eq!(body["thinking"], serde_json::json!("low"));

    let body = provider
        .build_request_body(&thinking_request("st", "st-model", None))
        .unwrap();
    assert_eq!(body["thinking"], serde_json::json!("none"));
}

#[test]
fn chat_thinking_token_budget_field_is_explicit_and_answer_clamped() {
    use opi_ai::model_info::{ChatThinkingFormat, ChatThinkingTokenBudgetField};
    let compat = CompatConfig {
        thinking_format: Some(ChatThinkingFormat::Qwen),
        thinking_token_budget_field: Some(ChatThinkingTokenBudgetField::ThinkingBudget),
        ..Default::default()
    };
    let provider = profile_provider("qwen-budget", compat, vec![thinking_chat_model("q3", true)]);

    let mut request = thinking_request("qwen-budget", "q3", Some(opi_ai::ThinkingLevel::High));
    request.max_tokens = Some(8192);
    let body = provider.build_request_body(&request).unwrap();
    // high defaults to 16384, clamped to 8192-1024.
    assert_eq!(body["thinking_budget"], serde_json::json!(7168));

    // Explicit per-call budget wins over the level default.
    let mut request = thinking_request("qwen-budget", "q3", Some(opi_ai::ThinkingLevel::High));
    request.max_tokens = Some(8192);
    request.thinking.budget_tokens = Some(2000);
    let body = provider.build_request_body(&request).unwrap();
    assert_eq!(body["thinking_budget"], serde_json::json!(2000));

    // No effort intent: no budget field even when configured.
    let mut request = thinking_request("qwen-budget", "q3", None);
    request.max_tokens = Some(8192);
    let body = provider.build_request_body(&request).unwrap();
    assert!(body.get("thinking_budget").is_none());
}

#[test]
fn chat_thinking_token_budget_alias_selects_vllm_field() {
    use opi_ai::model_info::ChatThinkingFormat;
    let compat = CompatConfig {
        thinking_format: Some(ChatThinkingFormat::ChatTemplate),
        supports_thinking_token_budget: true,
        ..Default::default()
    };
    let provider = profile_provider("vllm", compat, vec![thinking_chat_model("v1", true)]);
    let mut request = thinking_request("vllm", "v1", Some(opi_ai::ThinkingLevel::Medium));
    request.max_tokens = Some(8192);
    let body = provider.build_request_body(&request).unwrap();
    // medium defaults to 8192 and the shared 8192 ceiling leaves 1024 for the
    // answer: the alias field carries the clamped 7168 budget.
    assert_eq!(body["thinking_token_budget"], serde_json::json!(7168));
}

#[test]
fn chat_zai_tool_stream_and_vllm_priority_are_explicit_only() {
    use opi_ai::model_info::ChatThinkingFormat;
    let mut request = make_tool_request_for("zai-ts");
    request.thinking = ThinkingConfig::default();

    // Off by default: no tool_stream, no priority.
    let plain = profile_provider(
        "zai-ts",
        Default::default(),
        vec![thinking_chat_model("glm", true)],
    );
    let body = plain.build_request_body(&request).unwrap();
    assert!(body.get("tool_stream").is_none());
    assert!(body.get("priority").is_none());

    let compat = CompatConfig {
        zai_tool_stream: true,
        vllm_priority: Some(3.into()),
        thinking_format: Some(ChatThinkingFormat::Qwen),
        ..Default::default()
    };
    let provider = profile_provider("zai-ts", compat, vec![thinking_chat_model("glm", true)]);
    let body = provider.build_request_body(&request).unwrap();
    assert_eq!(body["tool_stream"], serde_json::json!(true));
    assert_eq!(body["priority"], serde_json::json!(3));
}

fn make_tool_request_for(provider: &str) -> Request {
    let mut request = make_tool_request();
    request.model = format!("{provider}:list_dir");
    // The tool fixture's model id is not in the catalog; register one that is.
    request.model = format!("{provider}:glm");
    request
}

#[test]
fn chat_baseline_inference_standard_openai_wire_defaults() {
    // Frozen SE-01 inference for the standard OpenAI endpoint: `store:false`,
    // developer instruction role only for reasoning models, the modern
    // `max_completion_tokens` field and streaming usage accounting.
    let provider = OpenAiChatProvider::new(None);

    let mut reasoning = request_for("openai", "o3");
    reasoning.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: None,
        level: opi_ai::ThinkingLevel::High,
    };
    let body = provider.build_request_body(&reasoning).unwrap();
    assert_eq!(
        body["store"],
        serde_json::json!(false),
        "store:false on SendFalse"
    );
    assert_eq!(body["max_completion_tokens"], serde_json::json!(4096));
    assert!(body.get("max_tokens").is_none());
    assert_eq!(
        body["stream_options"]["include_usage"],
        serde_json::json!(true)
    );
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(
        messages[0]["role"], "developer",
        "reasoning model uses developer"
    );
    assert_eq!(body["reasoning_effort"], serde_json::json!("high"));

    let plain = provider
        .build_request_body(&request_for("openai", "gpt-4o"))
        .unwrap();
    let messages = plain["messages"].as_array().unwrap();
    assert_eq!(
        messages[0]["role"], "system",
        "non-reasoning model keeps system"
    );
    assert_eq!(
        plain["store"],
        serde_json::json!(false),
        "store is model-independent"
    );
}

#[test]
fn chat_baseline_inference_non_standard_endpoint_omits_store_and_effort() {
    // zai is a frozen non-standard endpoint: no `store` field, no standard
    // `reasoning_effort` field, the legacy `max_tokens` field name, system role.
    let provider = profile_provider(
        "zai",
        Default::default(),
        vec![thinking_chat_model("glm-4.6", true)],
    );
    let mut request = request_for("zai", "glm-4.6");
    request.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: None,
        level: opi_ai::ThinkingLevel::High,
    };
    let body = provider.build_request_body(&request).unwrap();
    assert!(
        body.get("store").is_none(),
        "non-standard endpoint omits store"
    );
    assert!(
        body.get("reasoning_effort").is_none(),
        "zai baseline never sends the standard effort field"
    );
    assert_eq!(body["max_tokens"], serde_json::json!(4096));
    assert!(body.get("max_completion_tokens").is_none());
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages[0]["role"], "system");
}

#[test]
fn chat_explicit_store_and_role_overrides_beat_inference() {
    let compat = opi_ai::openai_chat::CompatConfig {
        store: Some(ChatStoreField::Omit),
        instruction_role: Some(ChatInstructionRole::System),
        ..Default::default()
    };
    let provider = profile_provider("openai", compat, vec![thinking_chat_model("o3", true)]);
    let mut request = request_for("openai", "o3");
    request.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: None,
        level: opi_ai::ThinkingLevel::High,
    };
    let body = provider.build_request_body(&request).unwrap();
    assert!(
        body.get("store").is_none(),
        "explicit Omit wins over SendFalse inference"
    );
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(
        messages[0]["role"], "system",
        "explicit System wins for a reasoning model"
    );

    let developer_fixed = opi_ai::openai_chat::CompatConfig {
        instruction_role: Some(ChatInstructionRole::Developer),
        ..Default::default()
    };
    let provider = profile_provider(
        "openai",
        developer_fixed,
        vec![thinking_chat_model("gpt-4o", false)],
    );
    let body = provider
        .build_request_body(&request_for("openai", "gpt-4o"))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(
        messages[0]["role"], "developer",
        "explicit Developer is unconditional"
    );
}

#[test]
fn chat_usage_in_stream_explicit_false_omits_stream_options() {
    let compat = opi_ai::openai_chat::CompatConfig {
        usage_in_stream: Some(false),
        ..Default::default()
    };
    let provider = profile_provider("openai", compat, vec![thinking_chat_model("gpt-4o", false)]);
    let body = provider
        .build_request_body(&request_for("openai", "gpt-4o"))
        .unwrap();
    assert!(
        body.get("stream_options").is_none(),
        "explicit false disables usage accounting"
    );
}

#[test]
fn chat_max_tokens_field_explicit_override_beats_inference() {
    let compat = opi_ai::openai_chat::CompatConfig {
        max_tokens_field: Some("max_tokens".into()),
        ..Default::default()
    };
    let provider = profile_provider("openai", compat, vec![thinking_chat_model("gpt-4o", false)]);
    let body = provider
        .build_request_body(&request_for("openai", "gpt-4o"))
        .unwrap();
    assert_eq!(body["max_tokens"], serde_json::json!(4096));
    assert!(body.get("max_completion_tokens").is_none());
}

#[test]
fn chat_model_level_override_beats_profile_and_inference() {
    use std::collections::HashMap;

    let compat = opi_ai::openai_chat::CompatConfig {
        instruction_role: Some(ChatInstructionRole::System),
        ..Default::default()
    };
    let mut overrides = HashMap::new();
    overrides.insert(
        "o3".into(),
        opi_ai::openai_chat::ModelCompatOverride {
            instruction_role: Some(ChatInstructionRole::Developer),
            ..Default::default()
        },
    );
    let provider = profile_provider("openai", compat, vec![thinking_chat_model("o3", true)])
        .with_model_overrides(overrides);

    let body = provider
        .build_request_body(&request_for("openai", "o3"))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(
        messages[0]["role"], "developer",
        "model override wins over profile"
    );

    let body = provider
        .build_request_body(&request_for("openai", "gpt-4o"))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(
        messages[0]["role"], "system",
        "profile default applies without override"
    );
}

use opi_ai::message::{ImageSource, MediaType, OutputContent, ToolDef, ToolResultMessage};
use opi_ai::provider::{ModelInfo, ProviderError, ProviderErrorCategory};
use opi_ai::registry::ModelCapabilities;
#[test]
fn build_request_body_strict_capability_preserves_unconstrained_false() {
    let config = CompatConfig {
        strict_tool_schema: true,
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat(None, config);
    let body = provider.build_request_body(&make_tool_request()).unwrap();
    let tools = body["tools"].as_array().expect("tools array present");
    assert!(
        tools[0]["function"]["strict"] == false,
        "unconstrained strict:false must be emitted on a capable wire: {tools:?}"
    );
}

#[test]
fn build_request_body_strict_tool_schema_off_by_default() {
    let provider = OpenAiChatProvider::new(None);
    let body = provider.build_request_body(&make_tool_request()).unwrap();
    let tools = body["tools"].as_array().expect("tools array present");
    assert!(
        tools[0]["function"].get("strict").is_none(),
        "strict must be absent by default: {tools:?}"
    );
}

fn make_tool_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("You are helpful.".into()),
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
                constrained_sampling: None,
            }],
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
    }
}

#[test]
fn build_request_body_reasoning_effort_absent_by_default() {
    let provider = OpenAiChatProvider::new(None);
    let body = provider.build_request_body(&make_test_request()).unwrap();
    assert!(body.get("reasoning_effort").is_none());
}

#[test]
fn chat_reasoning_effort_uses_request_thinking_and_model_map() {
    use opi_ai::{ThinkingLevel, ThinkingLevelMap, ThinkingLevelMapping};

    let provider = OpenAiChatProvider::new(None);
    let mut identity = make_test_request();
    identity.model = "openai:o3".into();
    identity.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: None,
        level: ThinkingLevel::High,
    };
    assert_eq!(
        provider.build_request_body(&identity).unwrap()["reasoning_effort"],
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
        "https://example.test".into(),
        "mapped".into(),
        CompatConfig::default(),
        vec![],
        vec![mapped_model],
    );
    let mut remapped = identity;
    remapped.model = "mapped:mapped".into();
    assert_eq!(
        mapped_provider.build_request_body(&remapped).unwrap()["reasoning_effort"],
        "provider-high"
    );

    let mut disabled = remapped;
    disabled.thinking.enabled = false;
    assert!(
        mapped_provider
            .build_request_body(&disabled)
            .unwrap()
            .get("reasoning_effort")
            .is_none()
    );

    let mut off = disabled;
    off.thinking.enabled = true;
    off.thinking.level = ThinkingLevel::None;
    assert!(
        mapped_provider
            .build_request_body(&off)
            .unwrap()
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
            .unwrap()
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
    let provider = OpenAiChatProvider::new_with_compat(None, config);
    let body = provider.build_request_body(&make_test_request()).unwrap();
    assert_eq!(body["prompt_cache_key"], "sess-abc");
}

#[test]
fn build_request_body_cache_key_absent_by_default() {
    let provider = OpenAiChatProvider::new(None);
    let body = provider.build_request_body(&make_test_request()).unwrap();
    assert!(body.get("prompt_cache_key").is_none());
}

// ---------------------------------------------------------------------------
// Task 20.36 (AIR-022 / V-11): assistant bridge, tool-result image projection
// and thinking/reasoning replay on the Chat wire.
// ---------------------------------------------------------------------------

fn tool_loop_context(
    tool_content: Vec<OutputContent>,
    is_error: bool,
    follow_up_user: bool,
) -> opi_ai::transcript::Context {
    let mut messages = vec![
        Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "run it".into(),
            }],
            timestamp_ms: 0,
        }),
        Message::Assistant(AssistantMessage {
            projection: None,
            content: vec![AssistantContent::ToolCall {
                tool_call: ToolCall {
                    id: "call_1".into(),
                    name: "list_dir".into(),
                    arguments: "{}".into(),
                    arguments_error: None,
                    signature: None,
                },
            }],
            api: opi_ai::ApiKind::OpenAi,
            provider: "openai".into(),
            model: "gpt-4o".into(),
            response_model: None,
            wire: Some(opi_ai::model_info::WireApi::OpenAiCompletions),
            response_id: None,
            usage: Default::default(),
            stop_reason: opi_ai::stream::StopReason::ToolUse,
            error_message: None,
            timestamp_ms: 0,
            diagnostics: Vec::new(),
            duration_ms: None,
            raw_stop_reason: None,
            provider_thinking_level: None,

            deferred: None,
        }),
        Message::ToolResult(ToolResultMessage {
            not_executed: None,
            tool_call_id: "call_1".into(),
            tool_name: "list_dir".into(),
            content: tool_content,
            details: None,
            is_error,
            truncated: false,
            timestamp_ms: 0,
        }),
    ];
    if follow_up_user {
        messages.push(Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "continue".into(),
            }],
            timestamp_ms: 1,
        }));
    }
    opi_ai::transcript::Context {
        system_prompt: None,
        messages,
        tools: vec![],
    }
}

fn tool_loop_request(
    tool_content: Vec<OutputContent>,
    is_error: bool,
    follow_up_user: bool,
) -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(tool_loop_context(
            tool_content,
            is_error,
            follow_up_user,
        ))
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
    }
}

fn image_tool_content() -> Vec<OutputContent> {
    vec![
        OutputContent::Text {
            text: "chart data".into(),
        },
        OutputContent::Image {
            source: ImageSource::Base64 {
                data: "aW1n".into(),
            },
            media_type: MediaType::Png,
        },
    ]
}

#[test]
fn chat_assistant_bridge_inserted_between_tool_result_and_user() {
    let compat = CompatConfig {
        require_assistant_after_tool_result: true,
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat(None, compat);
    let body = provider
        .build_request_body(&tool_loop_request(
            vec![OutputContent::Text {
                text: "a.rs".into(),
            }],
            false,
            true,
        ))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    let roles: Vec<&str> = messages
        .iter()
        .map(|m| m["role"].as_str().unwrap())
        .collect();
    assert_eq!(
        roles,
        vec!["user", "assistant", "tool", "assistant", "user"],
        "fixed assistant bridge lands between the tool result and the next user"
    );
    let bridge = &messages[3];
    assert_eq!(bridge["content"], "I have processed the tool results.");
    // The tool-call assistant turn keeps content "" (not null) on bridge wires.
    assert_eq!(messages[1]["content"], "");
}

#[test]
fn chat_no_bridge_by_default_between_tool_result_and_user() {
    let provider = OpenAiChatProvider::new(None);
    let body = provider
        .build_request_body(&tool_loop_request(
            vec![OutputContent::Text {
                text: "a.rs".into(),
            }],
            false,
            true,
        ))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    let roles: Vec<&str> = messages
        .iter()
        .map(|m| m["role"].as_str().unwrap())
        .collect();
    assert_eq!(roles, vec!["user", "assistant", "tool", "user"]);
    // No bridge flag: tool-call assistant turns keep content null.
    assert_eq!(messages[1]["content"], serde_json::Value::Null);
}

#[test]
fn chat_tool_result_images_project_to_trailing_user_message() {
    let provider = OpenAiChatProvider::new(None);
    let body = provider
        .build_request_body(&tool_loop_request(image_tool_content(), false, false))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    let roles: Vec<&str> = messages
        .iter()
        .map(|m| m["role"].as_str().unwrap())
        .collect();
    assert_eq!(
        roles,
        vec!["user", "assistant", "tool", "user"],
        "image parts ride a trailing user message, not the tool message"
    );
    let tool_msg = &messages[2];
    assert_eq!(
        tool_msg["content"], "chart data",
        "text stays on the tool turn"
    );
    assert!(
        tool_msg.get("content").and_then(|c| c.as_array()).is_none(),
        "no image parts inline on the tool message"
    );
    let image_user = &messages[3];
    let parts = image_user["content"].as_array().unwrap();
    assert_eq!(parts.len(), 2, "label text plus one image part");
    assert_eq!(parts[0]["type"], "text");
    assert_eq!(parts[0]["text"], "Attached image(s) from tool result:");
    assert_eq!(parts[1]["type"], "image_url");
    assert_eq!(parts[1]["image_url"]["url"], "data:image/png;base64,aW1n");
}

#[test]
fn chat_tool_result_image_bridge_wire_orders_tool_bridge_then_image_user() {
    let compat = CompatConfig {
        require_assistant_after_tool_result: true,
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat(None, compat);
    let body = provider
        .build_request_body(&tool_loop_request(image_tool_content(), false, false))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    let roles: Vec<&str> = messages
        .iter()
        .map(|m| m["role"].as_str().unwrap())
        .collect();
    assert_eq!(
        roles,
        vec!["user", "assistant", "tool", "assistant", "user"]
    );
    assert_eq!(messages[3]["content"], "I have processed the tool results.");
    assert_eq!(
        messages[4]["content"][0]["text"],
        "Attached image(s) from tool result:"
    );
}

#[test]
fn chat_tool_result_placeholders_follow_source_semantics() {
    let provider = OpenAiChatProvider::new(None);

    // Only images: text placeholder names the attachment, not a MIME stub.
    let body = provider
        .build_request_body(&tool_loop_request(
            vec![OutputContent::Image {
                source: ImageSource::Base64 {
                    data: "aW1n".into(),
                },
                media_type: MediaType::Png,
            }],
            false,
            false,
        ))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages[2]["content"], "(see attached image)");

    // No content at all: explicit no-output placeholder.
    let body = provider
        .build_request_body(&tool_loop_request(vec![], false, false))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages[2]["content"], "(no tool output)");

    // Error results keep the deterministic failure marker before the text.
    let body = provider
        .build_request_body(&tool_loop_request(
            vec![OutputContent::Text {
                text: "boom".into(),
            }],
            true,
            false,
        ))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages[2]["content"], "[tool_error] boom");
}

#[test]
fn chat_assistant_thinking_replay_uses_signature_reasoning_field() {
    let provider = OpenAiChatProvider::new(None);
    let mut request = make_test_request();
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "think".into(),
                }],
                timestamp_ms: 0,
            }),
            Message::Assistant(AssistantMessage {
                projection: None,
                content: vec![
                    AssistantContent::Thinking {
                        thinking: "step one".into(),
                        signature: Some("reasoning_content".into()),
                    },
                    AssistantContent::Text {
                        text: "answer".into(),
                        signature: None,
                    },
                ],
                api: opi_ai::ApiKind::OpenAi,
                provider: "openai".into(),
                model: "gpt-4o".into(),
                response_model: None,
                wire: Some(opi_ai::model_info::WireApi::OpenAiCompletions),
                response_id: None,
                usage: Default::default(),
                stop_reason: opi_ai::stream::StopReason::Stop,
                error_message: None,
                timestamp_ms: 0,
                diagnostics: Vec::new(),
                duration_ms: None,
                raw_stop_reason: None,
                provider_thinking_level: None,

                deferred: None,
            }),
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "again".into(),
                }],
                timestamp_ms: 1,
            }),
        ],
        tools: vec![],
    })
    .expect("valid context declarations");
    let body = provider.build_request_body(&request).unwrap();
    let messages = body["messages"].as_array().unwrap();
    let replayed = &messages[1];
    assert_eq!(replayed["role"], "assistant");
    assert_eq!(
        replayed["content"], "answer",
        "answer text stays plain string"
    );
    assert_eq!(
        replayed["reasoning_content"], "step one",
        "signed thinking rides its declared reasoning field"
    );
}

#[test]
fn chat_empty_assistant_turns_are_skipped() {
    let provider = OpenAiChatProvider::new(None);
    let mut request = make_test_request();
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text { text: "hi".into() }],
                timestamp_ms: 0,
            }),
            Message::Assistant(AssistantMessage {
                projection: None,
                content: vec![],
                api: opi_ai::ApiKind::OpenAi,
                provider: "openai".into(),
                model: "gpt-4o".into(),
                response_model: None,
                wire: Some(opi_ai::model_info::WireApi::OpenAiCompletions),
                response_id: None,
                usage: Default::default(),
                stop_reason: opi_ai::stream::StopReason::Stop,
                error_message: None,
                timestamp_ms: 0,
                diagnostics: Vec::new(),
                duration_ms: None,
                raw_stop_reason: None,
                provider_thinking_level: None,

                deferred: None,
            }),
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "again".into(),
                }],
                timestamp_ms: 1,
            }),
        ],
        tools: vec![],
    })
    .expect("valid context declarations");
    let body = provider.build_request_body(&request).unwrap();
    let messages = body["messages"].as_array().unwrap();
    let roles: Vec<&str> = messages
        .iter()
        .map(|m| m["role"].as_str().unwrap())
        .collect();
    assert_eq!(
        roles,
        vec!["user", "user"],
        "content/tool-call-free turns drop"
    );
}

#[test]
fn chat_deepseek_baseline_adds_empty_reasoning_content_on_assistant_messages() {
    let provider = profile_provider(
        "deepseek",
        Default::default(),
        vec![thinking_chat_model("deepseek-reasoner", true)],
    );
    let mut request = request_for("deepseek", "deepseek-reasoner");
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text { text: "hi".into() }],
                timestamp_ms: 0,
            }),
            Message::Assistant(AssistantMessage {
                projection: None,
                content: vec![AssistantContent::Text {
                    text: "answer".into(),
                    signature: None,
                }],
                api: opi_ai::ApiKind::OpenAi,
                provider: "deepseek".into(),
                model: "deepseek-reasoner".into(),
                response_model: None,
                wire: Some(opi_ai::model_info::WireApi::OpenAiCompletions),
                response_id: None,
                usage: Default::default(),
                stop_reason: opi_ai::stream::StopReason::Stop,
                error_message: None,
                timestamp_ms: 0,
                diagnostics: Vec::new(),
                duration_ms: None,
                raw_stop_reason: None,
                provider_thinking_level: None,

                deferred: None,
            }),
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "again".into(),
                }],
                timestamp_ms: 1,
            }),
        ],
        tools: vec![],
    })
    .expect("valid context declarations");
    let body = provider.build_request_body(&request).unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(
        messages[1]["reasoning_content"], "",
        "deepseek baseline marks replayed assistant turns for reasoning replay"
    );
}

fn make_tool_result_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![
                Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "run it".into(),
                    }],
                    timestamp_ms: 0,
                }),
                Message::Assistant(AssistantMessage {
                    projection: None,
                    content: vec![AssistantContent::ToolCall {
                        tool_call: ToolCall {
                            id: "call_1".into(),
                            name: "list_dir".into(),
                            arguments: "{}".into(),
                            arguments_error: None,
                            signature: None,
                        },
                    }],
                    api: opi_ai::ApiKind::OpenAi,
                    provider: "openai".into(),
                    model: "gpt-4o".into(),
                    response_model: None,
                    wire: Some(opi_ai::model_info::WireApi::OpenAiCompletions),
                    response_id: None,
                    usage: Default::default(),
                    stop_reason: opi_ai::stream::StopReason::ToolUse,
                    error_message: None,
                    timestamp_ms: 0,
                    diagnostics: Vec::new(),
                    duration_ms: None,
                    raw_stop_reason: None,
                    provider_thinking_level: None,

                    deferred: None,
                }),
                Message::ToolResult(ToolResultMessage {
                    not_executed: None,
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
    }
}

#[test]
fn build_request_body_tool_result_name_field_wire_shape() {
    let config = CompatConfig {
        tool_result_name_field: true,
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat(None, config);
    let body = provider
        .build_request_body(&make_tool_result_request())
        .unwrap();
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
    let provider = OpenAiChatProvider::new(None);
    let body = provider
        .build_request_body(&make_tool_result_request())
        .unwrap();
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
        "https://example.test".into(),
        "compatprof".into(),
        CompatConfig::default(),
        vec![],
        vec![text_only],
    );
    let image_request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "compatprof:text-only-model".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
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
    // Phase 12 task 12.3 + task 20.36 normalization: provider/model override
    // precedence. The profile sets a provider-level default (role, max tokens
    // field). A model-level override wins for the model that declares it;
    // other models inherit the profile default. Provider-level flags
    // (strict_tool_schema) apply to all models.
    use opi_ai::model_info::ChatInstructionRole;
    use opi_ai::openai_chat::ModelCompatOverride;
    let base = CompatConfig {
        instruction_role: Some(ChatInstructionRole::System),
        max_tokens_field: Some("max_tokens".into()),
        strict_tool_schema: true,
        ..Default::default()
    };
    let mut overrides = std::collections::HashMap::new();
    overrides.insert(
        "o3".into(),
        ModelCompatOverride {
            instruction_role: Some(ChatInstructionRole::Developer),
            max_tokens_field: Some("max_completion_tokens".into()),
            ..Default::default()
        },
    );
    let provider = OpenAiChatProvider::new_for_profile(
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
    let body = provider.build_request_body(&req_o3).unwrap();
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
        tools[0]["function"]["strict"] == false,
        "provider-level strict still applies"
    );

    // gpt-4o: no model override -> profile default applies.
    let mut req_plain = make_tool_request();
    req_plain.model = "prof:gpt-4o".into();
    let body2 = provider.build_request_body(&req_plain).unwrap();
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
            "max_completion_tokens": 1024
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::new(Some(server.uri()));
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
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

    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
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
        projection: Default::default(),
        transport: Default::default(),
        model: "affinityprof:any-model".into(),
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

    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
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
    let provider = OpenAiChatProvider::new(Some(server));
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
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
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());

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
        Some(Err(cause))
            if cause.category() == opi_ai::provider::ProviderErrorCategory::Cancelled =>
        { /* typed cancellation, as contracted */ }
        other => {
            panic!("expected Err(ProviderError::Cancelled) on cancellation, got: {other:?}")
        }
    }
}

// ---------------------------------------------------------------------------
// Task 20.5 (AIR-006/V-02/AIR-048): final tool arguments on the Chat wire
// (Mistral/Azure/OpenRouter inherit through the shared mapper) and
// keepalive/large-event acceptance.
// ---------------------------------------------------------------------------

fn chat_tool_fixture(arguments: &str) -> String {
    let escaped = arguments.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        "data: {{\"id\":\"chatcmpl-a\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{{\"index\":0,\"delta\":{{\"tool_calls\":[{{\"index\":0,\"id\":\"call_a\",\"type\":\"function\",\"function\":{{\"name\":\"read_file\",\"arguments\":\"\"}}}}]}},\"finish_reason\":null}}]}}\n\n\
         data: {{\"id\":\"chatcmpl-a\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{{\"index\":0,\"delta\":{{\"tool_calls\":[{{\"index\":0,\"function\":{{\"arguments\":\"{escaped}\"}}}}]}},\"finish_reason\":null}}]}}\n\n\
         : keepalive comment frames are ignored\n\n\
         data: {{\"id\":\"chatcmpl-a\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"tool_calls\"}}],\"usage\":{{\"prompt_tokens\":5,\"completion_tokens\":3}}}}\n\n\
         data: [DONE]\n\n"
    )
}

async fn chat_final_tool_call(arguments: &str) -> AssistantStreamEvent {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_tool_fixture(arguments))
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = OpenAiChatProvider::new(Some(server.uri()));
    let request = opi_ai::provider::Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: tokio_util::sync::CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };
    use opi_ai::provider::Provider as _;
    let stream = provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    futures_util::StreamExt::collect::<Vec<_>>(stream)
        .await
        .into_iter()
        .find_map(|result| match result.expect("valid chat tool stream") {
            AssistantStreamEvent::ToolCallEnd { tool_call, .. } => {
                Some(AssistantStreamEvent::ToolCallEnd {
                    content_index: 0,
                    tool_call,
                    partial: opi_ai::test_support::base_assistant(),
                })
            }
            _ => None,
        })
        .expect("chat tool call terminal")
}

fn tool_call_of(event: AssistantStreamEvent) -> opi_ai::message::ToolCall {
    match event {
        AssistantStreamEvent::ToolCallEnd { tool_call, .. } => tool_call,
        _ => panic!("expected ToolCallEnd"),
    }
}

#[tokio::test]
async fn chat_final_tool_arguments_empty_input_normalizes_to_object() {
    let tool_call = tool_call_of(chat_final_tool_call("").await);
    assert_eq!(tool_call.arguments, "{}");
    assert_eq!(tool_call.arguments_error, None);
}

#[tokio::test]
async fn chat_final_tool_arguments_invalid_escape_repairs_literal_text() {
    let tool_call = tool_call_of(chat_final_tool_call(r#"{"cmd":"\usepackage{x}","n":2}"#).await);
    assert_eq!(tool_call.arguments_error, None);
    let parsed: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&tool_call.arguments).expect("repaired arguments parse");
    assert_eq!(parsed["cmd"], r#"\usepackage{x}"#);
    assert_eq!(parsed["n"], 2);
}

#[tokio::test]
async fn chat_final_tool_arguments_truncated_keeps_raw_and_marks_error() {
    let raw = r#"{"path": "trunc"#;
    let tool_call = tool_call_of(chat_final_tool_call(raw).await);
    assert_eq!(tool_call.arguments, raw);
    assert_eq!(
        tool_call.arguments_error.as_deref(),
        Some("final tool arguments were not a complete JSON object")
    );
}

#[tokio::test]
async fn chat_keepalive_frames_are_ignored_between_events() {
    // Covered inline by chat_tool_fixture's keepalive frame: the fixture above
    // parses to completion with the keepalive line present.
    let events: Vec<_> = parse_sse_events(&chat_tool_fixture("{}"))
        .flat_map(|parsed| match parsed {
            ParsedEvent::Valid(evts) => evts,
            _ => Vec::new(),
        })
        .collect();
    assert!(!events.is_empty(), "keepalive frame does not break parsing");
}

#[test]
fn initial_chat_tool_call_arguments_are_emitted_after_start() {
    let chunk = serde_json::json!({
        "id": "response-1",
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "delta": {
                "tool_calls": [{
                    "index": 0,
                    "id": "call-1",
                    "type": "function",
                    "function": {"name": "read", "arguments": "{\"path\":"}
                }]
            }
        }]
    });
    let sse = format!("data: {chunk}\n\n");
    let events = match opi_ai::openai_chat::parse_sse_events(&sse)
        .next()
        .expect("one frame")
    {
        opi_ai::openai_chat::ParsedEvent::Valid(events) => events,
        opi_ai::openai_chat::ParsedEvent::UsageError(error) => panic!("{error}"),
        opi_ai::openai_chat::ParsedEvent::Malformed { error, .. } => panic!("{error}"),
    };

    assert!(matches!(
        events.first(),
        Some(OpenAiChatEvent::ToolCallStart { .. })
    ));
    assert!(matches!(
        events.get(1),
        Some(OpenAiChatEvent::ToolCallDelta { arguments, .. }) if arguments == "{\"path\":"
    ));
}

// ---------------------------------------------------------------------------
// Task 20.51: functional callbacks at the OpenAI Chat wire (P20-S51)
// ---------------------------------------------------------------------------

use opi_ai::callbacks::RequestCallbacks;

fn chat_callbacks_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
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
        sampling_params: serde_json::json!({"top_p": 0.9})
            .as_object()
            .cloned()
            .unwrap(),
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

fn chat_callbacks_sse() -> String {
    concat!(
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{\"content\":\"Hi\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n",
        "data: [DONE]\n\n",
    )
    .into()
}

#[tokio::test]
async fn chat_payload_callback_sees_merged_sampling_and_wire_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_callbacks_sse())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let seen: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let provider = OpenAiChatProvider::new(Some(server.uri()));
    let mut request = chat_callbacks_request();
    request.temperature = Some(0.5);
    request.callbacks = RequestCallbacks::new().with_on_payload(std::sync::Arc::new(move |body| {
        recorder.lock().unwrap().push(body.clone());
        Ok(None)
    }));

    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    while let Some(item) = stream.next().await {
        if let Err(error) = item {
            panic!("unexpected error: {error}");
        }
    }

    let bodies = seen.lock().unwrap().clone();
    assert_eq!(bodies.len(), 1);
    let sent = server.received_requests().await.unwrap()[0].clone();
    let wire_body: serde_json::Value =
        serde_json::from_slice(&sent.body).expect("captured body is JSON");
    assert_eq!(
        bodies[0], wire_body,
        "payload callback observes the exact wire body after sampling merge"
    );
    assert_eq!(bodies[0]["top_p"], serde_json::json!(0.9));
    assert_eq!(bodies[0]["temperature"], serde_json::json!(0.5));
}

#[tokio::test]
async fn chat_payload_replacement_reaches_the_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wiremock::matchers::body_partial_json(
            serde_json::json!({"chat_marker": "replaced"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_callbacks_sse())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::new(Some(server.uri()));
    let mut request = chat_callbacks_request();
    request.callbacks = RequestCallbacks::new().with_on_payload(std::sync::Arc::new(|body| {
        let mut replacement = body.clone();
        replacement["chat_marker"] = serde_json::json!("replaced");
        Ok(Some(replacement))
    }));

    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    while let Some(item) = stream.next().await {
        if let Err(error) = item {
            panic!("unexpected error: {error}");
        }
    }
    server.verify().await;
}

#[tokio::test]
async fn chat_async_response_and_raw_callbacks_fire_in_source_order() {
    let order: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let raw_recorder = order.clone();
    let response_recorder = order.clone();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_callbacks_sse())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::new(Some(server.uri()));
    let mut request = chat_callbacks_request();
    request.callbacks = RequestCallbacks::new()
        .with_async_header_transform(|headers| async move {
            tokio::task::yield_now().await;
            Ok(headers)
        })
        .with_async_on_payload(|_| async {
            tokio::task::yield_now().await;
            Ok(None)
        })
        .with_async_on_response(move |status, _headers| {
            let response_recorder = response_recorder.clone();
            async move {
                tokio::task::yield_now().await;
                response_recorder
                    .lock()
                    .unwrap()
                    .push(format!("response:{status}"));
                Ok(())
            }
        })
        .with_async_on_raw(move |event| {
            let raw_recorder = raw_recorder.clone();
            async move {
                tokio::task::yield_now().await;
                // The sentinel `[DONE]` frame is not JSON and is never a raw event.
                raw_recorder
                    .lock()
                    .unwrap()
                    .push(format!("raw:{}", event["object"].as_str().unwrap_or("?")));
                Ok(())
            }
        });

    let norm_recorder = order.clone();
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    while let Some(item) = stream.next().await {
        if item.is_ok() {
            norm_recorder.lock().unwrap().push("norm".into());
        }
    }

    let order = order.lock().unwrap();
    let markers = order.iter().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(markers.first(), Some(&"response:200"));
    // Three JSON chunks, each raw before its normalized emission.
    let raw_count = markers
        .iter()
        .filter(|m| **m == "raw:chat.completion.chunk")
        .count();
    assert_eq!(raw_count, 3);
    assert!(markers.iter().filter(|m| **m == "norm").count() >= 3);
    assert!(
        markers
            .iter()
            .position(|m| *m == "raw:chat.completion.chunk")
            .unwrap()
            < markers.iter().position(|m| *m == "norm").unwrap(),
        "raw precedes normalization"
    );
}

mod constrained_guard_tests {
    use super::*;
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    use opi_ai::message::ToolDef;
    use opi_ai::provider::ProviderError;

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
            projection: Default::default(),
            transport: Default::default(),
            model: "openai:gpt-4o".into(),
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
        let provider = OpenAiChatProvider::new(None);
        let error = provider
            .build_request_body(&guarded_request(vec![constrained_tool(
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
        let provider = OpenAiChatProvider::new(None);
        let body = provider
            .build_request_body(&guarded_request(vec![constrained_tool(
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

// ---------------------------------------------------------------------------
// Task 20.36 (AIR-049/057, ONB-03): public native/simple/complete/result and
// direct adapter calls through the registered openai-completions adapter.
// ---------------------------------------------------------------------------

use opi_ai::models::{ChatModelRecord, Models, SimpleOptions, SimpleToolChoice};
use opi_ai::openai_chat::{OpenAiCompletionsChatApi, OpenAiCompletionsOptions, OpenAiToolChoice};
use std::sync::Arc as StdArc;

fn chat_sse_ok() -> String {
    concat!(
        "data: {\"id\":\"chatcmpl-p\",\"object\":\"chat.completion.chunk\",\"model\":\"o3\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-p\",\"object\":\"chat.completion.chunk\",\"model\":\"o3\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-p\",\"object\":\"chat.completion.chunk\",\"model\":\"o3\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n",
        "data: [DONE]\n\n",
    )
    .into()
}

fn chat_public_request(model: &str, thinking: Option<opi_ai::ThinkingLevel>) -> Request {
    let mut request = make_test_request();
    request.model = format!("openai:{model}");
    if let Some(level) = thinking {
        request.thinking = ThinkingConfig {
            enabled: true,
            budget_tokens: None,
            level,
        };
    }
    request
}

async fn registered_chat_models(
    server_uri: String,
    model: ModelInfo,
) -> (
    Models,
    opi_ai::models::ChatApiHandle<OpenAiCompletionsOptions>,
) {
    let provider = StdArc::new(
        OpenAiChatProvider::new_with_compat(Some(server_uri), Default::default())
            .with_model_overrides(Default::default()),
    );
    let api = OpenAiCompletionsChatApi::new(provider, model).expect("valid chat record");
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            StdArc::new(api),
            StdArc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::Bearer,
                secrecy::SecretString::from("offline-chat-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register chat api");
    (models, handle)
}

fn o3_record() -> ChatModelRecord {
    let model = ModelInfo::new(
        "o3",
        "o3",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 100_000)
            .with_images(true)
            .with_streaming(true)
            .with_thinking(true),
    );
    ChatModelRecord::new("openai", model)
}

#[tokio::test]
async fn native_sampling_uses_native_effort_and_request_overrides() {
    for (effort, request_temperature, expected) in [
        (None, None, 0.2),
        (Some(opi_ai::ThinkingLevel::High), None, 0.9),
        (Some(opi_ai::ThinkingLevel::High), Some(0.4), 0.4),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_string(chat_sse_ok()))
            .expect(1)
            .mount(&server)
            .await;
        let model = ModelInfo::new(
            "o3",
            "o3",
            opi_ai::WireApi::OpenAiCompletions,
            ModelCapabilities::new(200_000, 100_000).with_thinking(true),
        )
        .with_sampling_params(
            [("temperature".into(), serde_json::json!(0.1))]
                .into_iter()
                .collect(),
        )
        .with_sampling_level_params(
            opi_ai::ThinkingLevel::None,
            [("temperature".into(), serde_json::json!(0.2))]
                .into_iter()
                .collect(),
        )
        .with_sampling_level_params(
            opi_ai::ThinkingLevel::High,
            [("temperature".into(), serde_json::json!(0.9))]
                .into_iter()
                .collect(),
        );
        let (models, handle) = registered_chat_models(server.uri(), model.clone()).await;
        let mut request = chat_public_request("o3", Some(opi_ai::ThinkingLevel::Low));
        if let Some(value) = request_temperature {
            request
                .sampling_params
                .insert("temperature".into(), serde_json::json!(value));
        }
        let mut options = OpenAiCompletionsOptions::default();
        options.reasoning_effort = effort;
        models
            .complete(
                ChatModelRecord::new("openai", model),
                request,
                handle.options(options),
            )
            .await
            .expect("native call completes");
        let sent = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
        assert_eq!(body["temperature"], serde_json::json!(expected));
        server.verify().await;
    }
}

#[tokio::test]
async fn models_native_options_reach_the_chat_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let model = ModelInfo::new(
        "o3",
        "o3",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 100_000)
            .with_streaming(true)
            .with_thinking(true),
    );
    let (models, handle) = registered_chat_models(server.uri(), model).await;

    let mut budgets = std::collections::BTreeMap::new();
    budgets.insert(opi_ai::ThinkingLevel::High, 512u64);
    let mut value = OpenAiCompletionsOptions::default();
    value.tool_choice = Some(OpenAiToolChoice::Named("list_dir".into()));
    value.reasoning_effort = Some(opi_ai::ThinkingLevel::High);
    value.thinking_budgets = budgets;
    let options = handle.options(value);

    let request = chat_public_request("o3", Some(opi_ai::ThinkingLevel::Minimal));
    // Native options own effort: the neutral field must not leak.
    let message = models
        .complete(o3_record(), request.clone(), options)
        .await
        .expect("native chat call completes");
    assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Stop);

    let sent = &server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
    assert_eq!(
        body["tool_choice"],
        serde_json::json!({"type": "function", "function": {"name": "list_dir"}})
    );
    assert_eq!(body["reasoning_effort"], serde_json::json!("high"));
    assert_eq!(
        body["store"],
        serde_json::json!(false),
        "public native path carries the standard OpenAI baseline"
    );
    assert_eq!(
        body["stream_options"]["include_usage"],
        serde_json::json!(true)
    );
    server.verify().await;
}

#[tokio::test]
async fn models_native_unsupported_effort_fails_typed_before_auth() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(0)
        .mount(&server)
        .await;

    let model = ModelInfo::new(
        "o3",
        "o3",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 100_000)
            .with_streaming(true)
            .with_thinking(true),
    );
    let (models, handle) = registered_chat_models(server.uri(), model).await;

    let mut value = OpenAiCompletionsOptions::default();
    value.reasoning_effort = Some(opi_ai::ThinkingLevel::XHigh);
    let options = handle.options(value);
    let failure = models
        .complete(o3_record(), chat_public_request("o3", None), options)
        .await
        .expect_err("unsupported native effort fails typed preflight");
    assert!(
        matches!(failure.cause(), opi_ai::provider::ProviderError::Config(_)),
        "typed preflight failure, got {:?}",
        failure.cause()
    );
    server.verify().await;
}

#[tokio::test]
async fn models_simple_neutral_options_and_clamp_reach_the_chat_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(2)
        .mount(&server)
        .await;

    let model = ModelInfo::new(
        "o3",
        "o3",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 100_000)
            .with_streaming(true)
            .with_thinking(true),
    );
    let (models, _handle) = registered_chat_models(server.uri(), model).await;

    // Neutral tool choice plus neutral effort through the simple entry.
    let simple = SimpleOptions {
        tool_choice: Some(SimpleToolChoice::Auto),
        ..Default::default()
    };
    let request = chat_public_request("o3", Some(opi_ai::ThinkingLevel::High));
    models
        .complete_simple(o3_record(), request, simple)
        .await
        .expect("simple chat call completes");

    // XHigh is unsupported on the default map: the neutral entry clamps to
    // the nearest supported level (High) instead of failing.
    let request = chat_public_request("o3", Some(opi_ai::ThinkingLevel::XHigh));
    models
        .complete_simple(o3_record(), request, Default::default())
        .await
        .expect("clamped simple chat call completes");

    let sent = server.received_requests().await.unwrap();
    let first: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
    assert_eq!(first["tool_choice"], serde_json::json!("auto"));
    assert_eq!(first["reasoning_effort"], serde_json::json!("high"));
    let second: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
    assert_eq!(
        second["reasoning_effort"],
        serde_json::json!("high"),
        "neutral clamp downgrades XHigh to the nearest supported level"
    );
    server.verify().await;
}

#[tokio::test]
async fn models_direct_adapter_uses_injected_auth_and_returns_result() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer injected-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let model = ModelInfo::new(
        "o3",
        "o3",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 100_000)
            .with_streaming(true)
            .with_thinking(true),
    );
    let (models, handle) = registered_chat_models(server.uri(), model).await;

    let options = handle.options(OpenAiCompletionsOptions::default());
    let mut injected = opi_ai::test_support::resolved_bearer_auth();
    injected.secret = secrecy::SecretString::from("injected-key");
    let message = models
        .complete_direct(
            o3_record(),
            chat_public_request("o3", None),
            options,
            injected,
        )
        .await
        .expect("direct adapter call completes");
    assert_eq!(message.content.len(), 1);
    server.verify().await;
}

#[test]
fn chat_direct_adapter_rejects_wrong_wire_records() {
    let responses_model = ModelInfo::new(
        "gpt-5.4",
        "GPT-5.4",
        opi_ai::WireApi::OpenAiResponses,
        ModelCapabilities::new(200_000, 100_000).with_streaming(true),
    );
    let provider = StdArc::new(OpenAiChatProvider::new(None));
    match OpenAiCompletionsChatApi::new(provider, responses_model) {
        Err(opi_ai::provider::ProviderError::Config(_)) => {}
        other => panic!("wrong-wire rejection is typed, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Task 20.36 (AIR-061 / V-53): owned duration and chunk-boundary acceptance.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn chat_done_message_carries_measured_response_duration() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        chat_public_request("gpt-4o", None),
        opi_ai::test_support::resolved_bearer_auth(),
    );
    let mut duration = None;
    while let Some(item) = stream.next().await {
        if let Ok(AssistantStreamEvent::Done { message, .. }) = item {
            duration = message.duration_ms;
        }
    }
    assert!(
        duration.is_some(),
        "successful terminal carries the measured response-stream duration"
    );
    server.verify().await;
}

#[tokio::test]
async fn chat_setup_failure_before_activation_has_no_duration() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        chat_public_request("gpt-4o", None),
        opi_ai::test_support::resolved_bearer_auth(),
    );
    let mut saw_done = false;
    while let Some(item) = stream.next().await {
        match item {
            Ok(AssistantStreamEvent::Done { .. }) => saw_done = true,
            Ok(_) => {}
            Err(cause) => {
                assert!(
                    !cause.is_retryable(),
                    "unauthorized is a terminal setup failure: {cause}"
                );
            }
        }
    }
    assert!(!saw_done, "setup failure never reaches a terminal Done");
    server.verify().await;
}

#[tokio::test]
async fn chat_sse_frames_split_across_tcp_chunks_reassemble() {
    use tokio::io::AsyncWriteExt;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let body = chat_sse_ok();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = String::new();
        use tokio::io::AsyncReadExt;
        let mut buf = vec![0u8; 4096];
        // Read until the end of the request headers/body boundary heuristically:
        // read once (headers + small body) then answer.
        let n = socket.read(&mut buf).await.unwrap();
        request.push_str(&String::from_utf8_lossy(&buf[..n]));
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        // Write the SSE body split at awkward byte boundaries.
        for byte in body.bytes() {
            socket.write_all(&[byte]).await.unwrap();
            tokio::task::yield_now().await;
        }
        socket.shutdown().await.unwrap();
    });

    let provider = OpenAiChatProvider::new(Some(format!("http://{addr}")));
    let mut stream = provider.stream_prepared(
        chat_public_request("gpt-4o", None),
        opi_ai::test_support::resolved_bearer_auth(),
    );
    let mut saw_done = false;
    while let Some(item) = stream.next().await {
        match item {
            Ok(AssistantStreamEvent::Done { reason, .. }) => {
                assert_eq!(reason, opi_ai::stream::StopReason::Stop);
                saw_done = true;
            }
            Ok(_) => {}
            Err(cause) => panic!("byte-split frames must reassemble: {cause}"),
        }
    }
    assert!(saw_done, "split-chunk stream still reaches its terminal");
}

#[tokio::test]
async fn multiline_sse_line_endings_survive_http_chunk_splits() {
    async fn run(body: String, split: Option<usize>) -> Vec<AssistantStreamEvent> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 65536];
            let received = socket.read(&mut request).await.unwrap();
            assert!(
                received > 0,
                "client sent request bytes before the response"
            );
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes()).await.unwrap();
            if let Some(split) = split {
                socket.write_all(&body.as_bytes()[..split]).await.unwrap();
                socket.flush().await.unwrap();
                tokio::time::sleep(std::time::Duration::from_millis(40)).await;
                socket.write_all(&body.as_bytes()[split..]).await.unwrap();
            } else {
                socket.write_all(body.as_bytes()).await.unwrap();
            }
            socket.shutdown().await.unwrap();
        });
        let provider = OpenAiChatProvider::new(Some(format!("http://{address}")));
        let results: Vec<_> = provider
            .stream_prepared(
                chat_public_request("gpt-4o", None),
                opi_ai::test_support::resolved_bearer_auth(),
            )
            .collect()
            .await;
        peer.await.unwrap();
        results
            .into_iter()
            .map(|event| event.expect("legal complete multiline event"))
            .collect()
    }

    let body = text_fixture().to_owned();
    for newline in ["\n", "\r\n", "\r"] {
        let multiline = body
            .lines()
            .map(|line| {
                if line.starts_with("data: {") {
                    line.replacen(',', &format!(",{newline}data: "), 1)
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join(newline)
            + newline;
        let split = multiline.find(&format!("{newline}data: ")).unwrap() + 1;
        let whole = run(multiline.clone(), None).await;
        let split = run(multiline, Some(split)).await;
        let kinds = |events: &[AssistantStreamEvent]| {
            events
                .iter()
                .map(|event| serde_json::to_value(event).unwrap()["type"].clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(kinds(&split), kinds(&whole), "newline={newline:?}");
        let final_message = |events: Vec<AssistantStreamEvent>| {
            events
                .into_iter()
                .find_map(|event| match event {
                    AssistantStreamEvent::Done { message, .. } => Some(message),
                    _ => None,
                })
                .expect("terminal outcome")
        };
        let whole = final_message(whole);
        let split = final_message(split);
        assert_eq!(split.content, whole.content);
        assert_eq!(split.stop_reason, whole.stop_reason);
        assert_eq!(split.usage, whole.usage);
        assert!(!split.content.is_empty());
    }
}

async fn chat_duration_on_peer(
    body_delay: std::time::Duration,
    setup_delay: std::time::Duration,
) -> (opi_ai::message::AssistantMessage, std::time::Duration) {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut bytes = [0; 4096];
        loop {
            let read = socket.read(&mut bytes).await.unwrap();
            assert!(read > 0, "request terminated before its full body");
            request.extend_from_slice(&bytes[..read]);
            if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                let length = headers
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("content-length:")
                            .map(|value| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap();
                if request.len() >= end + 4 + length {
                    assert!(
                        headers.starts_with("post /v1/chat/completions http/1.1"),
                        "{headers}"
                    );
                    assert!(headers.contains("\r\nauthorization: bearer test-key"));
                    let body: serde_json::Value =
                        serde_json::from_slice(&request[end + 4..end + 4 + length]).unwrap();
                    assert_eq!(body["model"], "gpt-4o");
                    break;
                }
            }
        }
        tokio::time::sleep(setup_delay).await;
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").await.unwrap();
        socket.flush().await.unwrap();
        tokio::time::sleep(body_delay).await;
        let body = chat_sse_ok();
        socket
            .write_all(format!("{:x}\r\n{}\r\n0\r\n\r\n", body.len(), body).as_bytes())
            .await
            .unwrap();
        socket.flush().await.unwrap();
    });

    struct DelayedAuth {
        delay: std::time::Duration,
        calls: Arc<AtomicUsize>,
    }
    impl opi_ai::auth::AuthResolver for DelayedAuth {
        fn resolve(
            &self,
        ) -> opi_ai::credential::BoxAuthFuture<'_, Result<opi_ai::ResolvedAuth, ProviderError>>
        {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                tokio::time::sleep(self.delay).await;
                Ok(opi_ai::test_support::resolved_bearer_auth())
            })
        }
    }
    let provider = OpenAiChatProvider::new(Some(format!("http://{address}")));
    let model = provider
        .models()
        .iter()
        .find(|model| model.id == "gpt-4o")
        .unwrap()
        .clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let mut models = opi_ai::Models::new();
    models
        .register_route(
            Box::new(provider),
            Arc::new(DelayedAuth {
                delay: setup_delay,
                calls: calls.clone(),
            }),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let mut request = chat_public_request("gpt-4o", None);
    request.callbacks = opi_ai::callbacks::RequestCallbacks::new().with_async_on_response(
        move |status, _| async move {
            assert_eq!(status, 200);
            tokio::time::sleep(setup_delay).await;
            Ok(())
        },
    );
    let started = std::time::Instant::now();
    let call = models.stream_simple(
        opi_ai::ChatModelRecord::new("openai", model),
        request,
        Default::default(),
    );
    tokio::time::sleep(setup_delay).await;
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "an unpolled handle starts no auth"
    );
    let message = call
        .result()
        .await
        .expect("controlled real adapter succeeds");
    let elapsed = started.elapsed();
    peer.await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Stop);
    assert_eq!(message.usage.input_tokens, 10);
    assert_eq!(message.usage.output_tokens, 5);
    (message, elapsed)
}

#[tokio::test]
async fn chat_duration_includes_pending_first_body_poll() {
    use std::time::Duration;
    let (control, _) = chat_duration_on_peer(Duration::ZERO, Duration::ZERO).await;
    let (delayed, elapsed) =
        chat_duration_on_peer(Duration::from_millis(350), Duration::ZERO).await;
    assert_eq!(delayed.content, control.content);
    assert!(!delayed.content.is_empty());
    assert_eq!(delayed.usage, control.usage);
    assert!(elapsed >= Duration::from_millis(300));
    assert!(
        delayed.duration_ms.unwrap() >= 300,
        "first Pending body poll belongs to response duration: {:?}",
        delayed.duration_ms
    );
}

#[tokio::test]
async fn chat_duration_excludes_auth_setup_callback_and_unpolled_time() {
    use std::time::Duration;
    // Four independent150ms setup intervals: unpolled handle, resolver,
    // response headers and response callback. None belongs to body polling.
    let (message, elapsed) =
        chat_duration_on_peer(Duration::ZERO, Duration::from_millis(150)).await;
    let measured = Duration::from_millis(message.duration_ms.unwrap());
    assert!(
        elapsed.saturating_sub(measured) >= Duration::from_millis(500),
        "duration included setup: measured={measured:?}, elapsed={elapsed:?}"
    );
}

#[test]
fn source_compat_chat_cache_markers_and_retention_have_exact_owners() {
    use opi_ai::model_info::CacheControlFormat;
    for retention in [
        CacheRetention::Disabled,
        CacheRetention::Short,
        CacheRetention::Long,
    ] {
        for long in [false, true] {
            let provider = OpenAiChatProvider::new_with_compat(
                None,
                CompatConfig {
                    cache_control_format: Some(CacheControlFormat::Anthropic),
                    supports_long_cache_retention: Some(long),
                    ..Default::default()
                },
            );
            let mut request = make_tool_request();
            request.cache_retention = retention;
            request.session_id = Some("raw-session".repeat(12));
            let canonical = serde_json::to_vec(request.context.messages()).unwrap();
            let body = provider.build_request_body(&request).unwrap();
            if retention == CacheRetention::Disabled {
                assert!(!body.to_string().contains("cache_control"));
                assert!(body.get("prompt_cache_key").is_none());
            } else {
                let marker = if retention == CacheRetention::Long && long {
                    serde_json::json!({"type":"ephemeral","ttl":"1h"})
                } else {
                    serde_json::json!({"type":"ephemeral"})
                };
                assert_eq!(body["messages"][0]["content"][0]["cache_control"], marker);
                assert_eq!(body["messages"][1]["content"][0]["cache_control"], marker);
                assert_eq!(body["tools"][0]["cache_control"], marker);
                assert!(body["tools"][0]["function"].get("cache_control").is_none());
                assert_eq!(
                    body["prompt_cache_key"].as_str().unwrap().chars().count(),
                    64
                );
            }
            assert_eq!(
                body.get("prompt_cache_retention"),
                (retention == CacheRetention::Long && long).then_some(&serde_json::json!("24h"))
            );
            assert_eq!(
                serde_json::to_vec(request.context.messages()).unwrap(),
                canonical
            );
        }
    }
    for (provider_id, expected) in [
        ("together", false),
        ("cloudflare-workers-ai", false),
        ("cloudflare-ai-gateway", false),
        ("nvidia", false),
        ("ant-ling", false),
        ("ordinary", true),
    ] {
        let mut request = make_tool_request();
        request.model = format!("{provider_id}:gpt-4o");
        request.cache_retention = CacheRetention::Long;
        let provider = profile_provider(
            provider_id,
            CompatConfig::default(),
            vec![thinking_chat_model("gpt-4o", false)],
        );
        assert_eq!(
            provider
                .build_request_body(&request)
                .unwrap()
                .get("prompt_cache_retention")
                .is_some(),
            expected,
            "{provider_id}"
        );
    }
}

#[test]
fn source_compat_chat_template_empty_map_clears_and_priority_is_exact() {
    use opi_ai::model_info::{ChatTemplateKwargValue, ChatThinkingFormat};
    let mut model = thinking_chat_model("clear", true);
    model.compat = opi_ai::WireCompat::OpenAiCompletions(CompatConfig {
        thinking_format: Some(ChatThinkingFormat::ChatTemplate),
        chat_template_kwargs: Some(Default::default()),
        vllm_priority: Some(serde_json::Number::from(i64::MAX)),
        ..Default::default()
    });
    let provider = OpenAiChatProvider::for_route(
        Some("http://127.0.0.1:1".into()),
        "custom".into(),
        Default::default(),
        vec![model],
        StdArc::new(opi_ai::http::HttpClient::new()),
    );
    let body = provider
        .build_request_body(&thinking_request(
            "custom",
            "clear",
            Some(opi_ai::ThinkingLevel::Low),
        ))
        .unwrap();
    assert!(body.get("chat_template_kwargs").is_none());
    assert_eq!(body["priority"].as_i64(), Some(i64::MAX));
    let kwargs = std::collections::BTreeMap::from([(
        "literal".into(),
        ChatTemplateKwargValue::Literal(
            serde_json::json!({"$var":"thinking.enabled","omitWhenOff":true}),
        ),
    )]);
    let provider = profile_provider(
        "custom",
        CompatConfig {
            thinking_format: Some(ChatThinkingFormat::ChatTemplate),
            chat_template_kwargs: Some(kwargs),
            vllm_priority: Some(serde_json::Number::from_f64(-0.125).unwrap()),
            ..Default::default()
        },
        vec![thinking_chat_model("clear", true)],
    );
    let body = provider
        .build_request_body(&thinking_request("custom", "clear", None))
        .unwrap();
    assert_eq!(body["priority"], -0.125);
    assert_eq!(
        body["chat_template_kwargs"]["literal"],
        serde_json::json!({"$var":"thinking.enabled","omitWhenOff":true})
    );
}

#[tokio::test]
async fn source_compat_chat_affinity_formats_survive_disabled_cache_and_raw_id() {
    use opi_ai::model_info::SessionAffinityFormat;
    for (format, names) in [
        (
            SessionAffinityFormat::OpenAi,
            vec!["session_id", "x-client-request-id", "x-session-affinity"],
        ),
        (
            SessionAffinityFormat::OpenAiNoSession,
            vec!["x-client-request-id", "x-session-affinity"],
        ),
        (SessionAffinityFormat::OpenRouter, vec!["x-session-id"]),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(stalled_openai_chat_terminal_chunk())
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let provider = OpenAiChatProvider::new_with_compat(
            Some(server.uri()),
            CompatConfig {
                send_session_affinity_headers: Some(true),
                session_affinity_format: Some(format),
                ..Default::default()
            },
        );
        let mut request = make_test_request();
        request.cache_retention = CacheRetention::Disabled;
        request.session_id = Some("unclamped-session-".repeat(8));
        let id = request.session_id.clone().unwrap();
        let events = provider
            .stream_prepared(request, opi_ai::test_support::resolved_bearer_auth())
            .collect::<Vec<_>>()
            .await;
        assert!(events.iter().all(Result::is_ok));
        let sent = server.received_requests().await.unwrap();
        for name in [
            "session_id",
            "x-client-request-id",
            "x-session-affinity",
            "x-session-id",
        ] {
            if names.contains(&name) {
                assert_eq!(sent[0].headers.get(name).unwrap(), id.as_str());
            } else {
                assert!(!sent[0].headers.contains_key(name));
            }
        }
        let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
        assert!(body.get("prompt_cache_key").is_none());
    }
}

#[tokio::test]
async fn source_compat_chat_finishless_eof_closes_text_and_tools_without_raw_reason() {
    for supports in [None, Some(true), Some(false)] {
        for tool in [false, true] {
            let delta = if tool {
                serde_json::json!({"role":"assistant","tool_calls":[{"index":0,"id":"call-eof","type":"function","function":{"name":"list_dir","arguments":"{\"path\":"}}]})
            } else {
                serde_json::json!({"role":"assistant","content":"hello"})
            };
            let sse = format!(
                "data: {}\n\ndata: {}\n\n",
                serde_json::json!({"id":"finishless","model":"gpt-4o","choices":[{"index":0,"delta":delta,"finish_reason":null}]}),
                serde_json::json!({"choices":[],"usage":{"prompt_tokens":5,"completion_tokens":3}})
            );
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string(sse)
                        .insert_header("content-type", "text/event-stream"),
                )
                .expect(1)
                .mount(&server)
                .await;
            let provider = OpenAiChatProvider::new_with_compat(
                Some(server.uri()),
                CompatConfig {
                    supports_finish_reason: supports,
                    ..Default::default()
                },
            );
            let request = make_tool_request();
            let cancel = request.cancel.clone();
            let mut stream =
                provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
            let mut items = Vec::new();
            while let Some(item) = stream.next().await {
                items.push(item);
            }
            if supports == Some(false) {
                assert!(items.iter().all(Result::is_ok), "{items:?}");
                let message = items
                    .iter()
                    .find_map(|item| match item {
                        Ok(AssistantStreamEvent::Done { message, .. }) => Some(message),
                        _ => None,
                    })
                    .expect("inferred terminal");
                assert_eq!(
                    message.stop_reason,
                    if tool {
                        StopReason::ToolUse
                    } else {
                        StopReason::Stop
                    }
                );
                assert_eq!(message.raw_stop_reason, None);
                assert!(message.projection.is_some());
                assert!(message.duration_ms.is_some());
                assert_eq!(message.usage.output_tokens, 3);
                if tool {
                    let call = message
                        .content
                        .iter()
                        .find_map(|part| match part {
                            AssistantContent::ToolCall { tool_call } => Some(tool_call),
                            _ => None,
                        })
                        .unwrap();
                    assert!(call.arguments_error.is_some());
                    assert_eq!(call.arguments, "{\"path\":");
                }
            } else {
                assert_eq!(items.iter().filter(|item| item.is_err()).count(), 1);
                assert!(
                    !items
                        .iter()
                        .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
                );
            }
            cancel.cancel();
            assert!(stream.next().await.is_none());
            assert!(stream.next().await.is_none());
        }
    }
}

#[tokio::test]
async fn source_compat_chat_strict_grammar_and_normalizer_controls() {
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    for strict_cap in [false, true] {
        for intent in [StrictPreference::Prefer, StrictPreference::Require] {
            let mut request = make_tool_request();
            let tool = ToolDef {
                name: "list_dir".into(),
                description: "list".into(),
                input_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"},"optional":{"type":"number"}},"required":["path"]}),
                constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict: intent }),
            };
            request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                system_prompt: None,
                messages: vec![],
                tools: vec![tool],
            })
            .unwrap();
            let before = serde_json::to_vec(request.context.messages()).unwrap();
            let provider = OpenAiChatProvider::new_with_compat(
                None,
                CompatConfig {
                    strict_tool_schema: strict_cap,
                    ..Default::default()
                },
            );
            let body = provider.build_request_body(&request);
            if !strict_cap && intent == StrictPreference::Require {
                assert!(matches!(body, Err(ProviderError::Config(_))));
                continue;
            }
            let body = body.unwrap();
            assert_eq!(
                body["tools"][0]["function"].get("strict"),
                strict_cap.then_some(&serde_json::json!(true))
            );
            if strict_cap {
                assert_eq!(
                    body["tools"][0]["function"]["parameters"]["additionalProperties"],
                    false
                );
                assert_eq!(
                    body["tools"][0]["function"]["parameters"]["required"],
                    serde_json::json!(["path", "optional"])
                );
            }
            assert_eq!(
                serde_json::to_vec(request.context.messages()).unwrap(),
                before
            );
        }
    }
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_tool_fixture("{\"path\":\"x\",\"optional\":null}"))
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let provider = OpenAiChatProvider::new_with_compat(
        Some(server.uri()),
        CompatConfig {
            strict_tool_schema: true,
            ..Default::default()
        },
    );
    let mut request = make_tool_request();
    let tool = ToolDef {
        name: "read_file".into(),
        description: "read".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"},"optional":{"type":"number"}},"required":["path"]}),
        constrained_sampling: Some(ConstrainedSampling::JsonSchema {
            strict: StrictPreference::Require,
        }),
    };
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![],
        tools: vec![tool],
    })
    .unwrap();
    let events = provider
        .stream_prepared(request, opi_ai::test_support::resolved_bearer_auth())
        .collect::<Vec<_>>()
        .await;
    let call = events
        .into_iter()
        .find_map(|event| match event.unwrap() {
            AssistantStreamEvent::ToolCallEnd { tool_call, .. } => Some(tool_call),
            _ => None,
        })
        .unwrap();
    assert_eq!(call.arguments_error, None);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&call.arguments).unwrap(),
        serde_json::json!({"path":"x"})
    );
    for grammar_cap in [false, true] {
        let mut request = make_tool_request();
        request.context=opi_ai::transcript::normalize_context(opi_ai::transcript::Context{system_prompt:None,messages:vec![],tools:vec![ToolDef{name:"lookup".into(),description:"lookup".into(),input_schema:serde_json::json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}),constrained_sampling:Some(ConstrainedSampling::Grammar{openai_lark:Some("start: /.+/".into()),openai_regex:Some("regex-loses".into())})}]}).unwrap();
        let provider = OpenAiChatProvider::new_with_compat(
            None,
            CompatConfig {
                supports_grammar_tools: Some(grammar_cap),
                ..Default::default()
            },
        );
        let body = provider.build_request_body(&request).unwrap();
        assert_eq!(
            body["tools"][0]["type"],
            if grammar_cap { "custom" } else { "function" }
        );
        if grammar_cap {
            assert_eq!(body["tools"][0]["custom"]["format"]["syntax"], "lark");
        }
    }
}

#[tokio::test]
async fn source_compat_chat_frozen_model_controls_reach_native_and_simple_http() {
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    use opi_ai::model_info::{
        CacheControlFormat, ChatInstructionRole, ChatStoreField, ChatThinkingFormat,
        SessionAffinityFormat,
    };
    let server = MockServer::start().await;
    let sse = format!(
        "data: {}\n\n",
        serde_json::json!({"id":"native-source","model":"o3","choices":[{"index":0,"delta":{"role":"assistant","content":"done"},"finish_reason":null}]})
    );
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(2)
        .mount(&server)
        .await;
    let routing = serde_json::json!({"allow_fallbacks":false,"only":[],"sort":{"by":"latency","partition":null}});
    let compat = CompatConfig {
        store: Some(ChatStoreField::Omit),
        instruction_role: Some(ChatInstructionRole::System),
        thinking_format: Some(ChatThinkingFormat::ChatTemplate),
        supports_reasoning_effort: Some(false),
        usage_in_stream: Some(false),
        supports_finish_reason: Some(false),
        max_tokens_field: Some("max_tokens".into()),
        supports_grammar_tools: Some(true),
        strict_tool_schema: true,
        open_router_routing: Some(serde_json::from_value(routing.clone()).unwrap()),
        vercel_gateway_routing: Some(
            serde_json::from_value(serde_json::json!({"only":[]})).unwrap(),
        ),
        cache_control_format: Some(CacheControlFormat::Anthropic),
        session_affinity_format: Some(SessionAffinityFormat::OpenAiNoSession),
        send_session_affinity_headers: Some(true),
        supports_long_cache_retention: Some(true),
        vllm_priority: Some(serde_json::Number::from_f64(1.25).unwrap()),
        chat_template_kwargs: Some(std::collections::BTreeMap::from([(
            "zero".into(),
            opi_ai::model_info::ChatTemplateKwargValue::Literal(serde_json::json!(0)),
        )])),
        supports_mid_convo_system_messages: Some(true),
        supports_mid_convo_tool_additions: Some(true),
        ..Default::default()
    };
    let model = ModelInfo::new(
        "o3",
        "o3",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(8192, 1024)
            .with_streaming(true)
            .with_thinking(true),
    )
    .with_compat(opi_ai::WireCompat::OpenAiCompletions(compat))
    .unwrap();
    let (models, handle) = registered_chat_models(server.uri(), model.clone()).await;
    let schema_tool = ToolDef {
        name: "structured".into(),
        description: "schema".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
        constrained_sampling: Some(ConstrainedSampling::JsonSchema {
            strict: StrictPreference::Require,
        }),
    };
    let grammar_tool = ToolDef {
        name: "grammar".into(),
        description: "grammar".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}),
        constrained_sampling: Some(ConstrainedSampling::Grammar {
            openai_lark: Some("start: /.+/".into()),
            openai_regex: None,
        }),
    };
    let change = opi_ai::transcript::SystemMessage {
        content: "later policy".into(),
        tools_added: vec![grammar_tool],
        ..Default::default()
    };
    let mut request = chat_public_request("o3", Some(opi_ai::ThinkingLevel::Low));
    request.cache_retention = CacheRetention::Long;
    request.session_id = Some("native-full-session".repeat(7));
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: Some("initial policy".into()),
        tools: vec![schema_tool],
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "before".into(),
                }],
                timestamp_ms: 0,
            }),
            Message::System(change),
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "after".into(),
                }],
                timestamp_ms: 1,
            }),
        ],
    })
    .unwrap();
    let canonical = serde_json::to_vec(request.context.messages()).unwrap();
    let mut options = OpenAiCompletionsOptions::default();
    options.reasoning_effort = Some(opi_ai::ThinkingLevel::High);
    let native = models
        .complete(
            ChatModelRecord::new("openai", model.clone()),
            request.clone(),
            handle.options(options),
        )
        .await
        .unwrap();
    let simple = models
        .complete_simple(
            ChatModelRecord::new("openai", model),
            request.clone(),
            Default::default(),
        )
        .await
        .unwrap();
    assert_eq!(native.raw_stop_reason, None);
    assert_eq!(simple.raw_stop_reason, None);
    assert!(native.projection.is_some());
    assert!(simple.projection.is_some());
    assert_eq!(
        serde_json::to_vec(request.context.messages()).unwrap(),
        canonical
    );
    for sent in server.received_requests().await.unwrap() {
        let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
        assert_eq!(body["provider"], routing);
        assert_eq!(
            body["providerOptions"],
            serde_json::json!({"gateway":{"only":[]}})
        );
        assert_eq!(body["priority"], 1.25);
        assert_eq!(body["chat_template_kwargs"], serde_json::json!({"zero":0}));
        assert!(body.get("store").is_none());
        assert!(body.get("stream_options").is_none());
        assert!(body.get("reasoning_effort").is_none());
        assert!(body.get("max_tokens").is_some());
        assert!(body.get("max_completion_tokens").is_none());
        assert_eq!(body["prompt_cache_retention"], "24h");
        assert_eq!(body["tools"][0]["function"]["strict"], true);
        assert_eq!(body["messages"][2]["tools"][0]["type"], "custom");
        assert_eq!(body["messages"][3]["role"], "system");
        assert_eq!(body["messages"][3]["content"], "later policy");
        assert_eq!(
            body["messages"][4]["content"][0]["cache_control"]["ttl"],
            "1h"
        );
        assert!(!sent.headers.contains_key("session_id"));
        assert!(!sent.headers.contains_key("x-session-id"));
        assert_eq!(
            sent.headers.get("x-session-affinity").unwrap(),
            request.session_id.as_deref().unwrap()
        );
        assert_eq!(
            sent.headers.get("x-client-request-id").unwrap(),
            request.session_id.as_deref().unwrap()
        );
    }
}

#[test]
fn source_compat_chat_cache_marks_last_eligible_assistant_tool_and_projected_text() {
    use opi_ai::model_info::CacheControlFormat;
    let provider = OpenAiChatProvider::new_with_compat(
        None,
        CompatConfig {
            cache_control_format: Some(CacheControlFormat::Anthropic),
            ..Default::default()
        },
    );
    for images in [false, true] {
        let mut request = tool_loop_request(
            if images {
                image_tool_content()
            } else {
                vec![OutputContent::Text {
                    text: "tool output".into(),
                }]
            },
            false,
            false,
        );
        request.cache_retention = CacheRetention::Short;
        let canonical = serde_json::to_vec(request.context.messages()).unwrap();
        let body = provider.build_request_body(&request).unwrap();
        let messages = body["messages"].as_array().unwrap();
        let last = messages.last().unwrap();
        assert_eq!(last["role"], if images { "user" } else { "tool" });
        let parts = last["content"].as_array().unwrap();
        assert_eq!(
            parts
                .iter()
                .filter(|part| part.get("cache_control").is_some())
                .count(),
            1
        );
        let text = parts
            .iter()
            .rev()
            .find(|part| part["type"] == "text")
            .unwrap();
        assert_eq!(
            text["cache_control"],
            serde_json::json!({"type":"ephemeral"})
        );
        assert_eq!(
            serde_json::to_vec(request.context.messages()).unwrap(),
            canonical
        );
    }
    let mut assistant = opi_ai::test_support::base_assistant();
    assistant.provider = "openai".into();
    assistant.model = "gpt-4o".into();
    assistant.wire = Some(opi_ai::WireApi::OpenAiCompletions);
    assistant.content = vec![AssistantContent::Text {
        text: "cached answer".into(),
        signature: None,
    }];
    let mut request = make_test_request();
    request.cache_retention = CacheRetention::Short;
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        messages: vec![Message::Assistant(assistant)],
        ..Default::default()
    })
    .unwrap();
    let body = provider.build_request_body(&request).unwrap();
    assert_eq!(body["messages"][0]["role"], "assistant");
    assert_eq!(
        body["messages"][0]["content"][0]["cache_control"],
        serde_json::json!({"type":"ephemeral"})
    );
}

#[test]
fn reviewed_chat_plain_thinking_keeps_text_calls_and_independent_required_content() {
    let signature_details =
        serde_json::json!([{"type":"reasoning.text","text":"opaque","id":"trace"}]).to_string();
    for (plain, required, signature) in [
        (true, false, "reasoning_content".to_owned()),
        (false, true, "reasoning".to_owned()),
        (false, true, "reasoning_text".to_owned()),
        (true, true, "reasoning_text".to_owned()),
        (false, true, signature_details.clone()),
        (true, true, signature_details),
    ] {
        let provider = profile_provider(
            "openai",
            CompatConfig {
                requires_thinking_as_text: plain,
                requires_reasoning_content_on_assistant_messages: Some(required),
                ..Default::default()
            },
            vec![thinking_chat_model("gpt-4o", true)],
        );
        let mut assistant = opi_ai::test_support::base_assistant();
        assistant.api = opi_ai::ApiKind::OpenAi;
        assistant.provider = "openai".into();
        assistant.model = "gpt-4o".into();
        assistant.wire = Some(opi_ai::WireApi::OpenAiCompletions);
        assistant.content = vec![
            AssistantContent::Thinking {
                thinking: "plain thought".into(),
                signature: Some(signature.clone()),
            },
            AssistantContent::Text {
                text: "answer".into(),
                signature: None,
            },
            AssistantContent::ToolCall {
                tool_call: ToolCall {
                    id: "review-call".into(),
                    name: "read".into(),
                    arguments: "{}".into(),
                    arguments_error: None,
                    signature: None,
                },
            },
        ];
        let mut request = make_test_request();
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            messages: vec![Message::Assistant(assistant)],
            ..Default::default()
        })
        .unwrap();
        let canonical = serde_json::to_vec(request.context.messages()).unwrap();
        let body = provider.build_request_body(&request).unwrap();
        let assistant = body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["role"] == "assistant")
            .unwrap();
        assert_eq!(assistant["tool_calls"][0]["function"]["name"], "read");
        if plain {
            assert_eq!(
                assistant["content"],
                serde_json::json!([{"type":"text","text":"plain thought"},{"type":"text","text":"answer"}])
            );
            if !required {
                assert!(assistant.get("reasoning_content").is_none());
            }
            assert!(!assistant.to_string().contains("<thinking>"));
        }
        if required {
            assert_eq!(assistant["reasoning_content"], "");
            if signature == "reasoning" || signature == "reasoning_text" {
                if plain {
                    assert!(assistant.get(&signature).is_none());
                } else {
                    assert_eq!(assistant[&signature], "plain thought");
                }
            } else {
                assert_eq!(
                    assistant["reasoning_details"],
                    serde_json::from_str::<serde_json::Value>(&signature).unwrap()
                );
            }
        }
        assert_eq!(
            serde_json::to_vec(request.context.messages()).unwrap(),
            canonical
        );
    }
}

#[tokio::test]
async fn reviewed_chat_zero_max_is_omitted_for_each_field_and_real_entry() {
    for field in ["max_tokens", "max_completion_tokens"] {
        for value in [0, 32] {
            for native in [false, true] {
                let server = MockServer::start().await;
                Mock::given(method("POST"))
                    .respond_with(
                        ResponseTemplate::new(200)
                            .set_body_string(chat_sse_ok())
                            .insert_header("content-type", "text/event-stream"),
                    )
                    .expect(1)
                    .mount(&server)
                    .await;
                let compat = CompatConfig {
                    max_tokens_field: Some(field.into()),
                    ..Default::default()
                };
                let mut request = chat_public_request("o3", None);
                request.max_tokens = Some(value);
                if native {
                    let model = ModelInfo::new(
                        "o3",
                        "o3",
                        opi_ai::WireApi::OpenAiCompletions,
                        ModelCapabilities::new(8192, 1024).with_streaming(true),
                    )
                    .with_compat(opi_ai::WireCompat::OpenAiCompletions(compat))
                    .unwrap();
                    let (models, handle) =
                        registered_chat_models(server.uri(), model.clone()).await;
                    models
                        .complete(
                            ChatModelRecord::new("openai", model),
                            request,
                            handle.options(Default::default()),
                        )
                        .await
                        .unwrap();
                } else {
                    let provider = OpenAiChatProvider::new_with_compat(Some(server.uri()), compat);
                    let events = provider
                        .stream_prepared(request, opi_ai::test_support::resolved_bearer_auth())
                        .collect::<Vec<_>>()
                        .await;
                    assert!(events.iter().all(Result::is_ok), "{events:?}");
                }
                let sent = server.received_requests().await.unwrap();
                let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
                assert_eq!(
                    body.get(field),
                    (value != 0).then_some(&serde_json::json!(value)),
                    "field={field},value={value},native={native}"
                );
                let other = if field == "max_tokens" {
                    "max_completion_tokens"
                } else {
                    "max_tokens"
                };
                assert!(body.get(other).is_none());
            }
        }
    }
}

#[tokio::test]
async fn reviewed_chat_optional_profile_and_model_precedence_reaches_native_and_simple() {
    use opi_ai::model_info::{ChatTemplateKwargValue, ChatThinkingFormat};
    for explicit_model in [false, true] {
        let server = MockServer::start().await;
        let peer = if explicit_model {
            chat_sse_ok()
        } else {
            format!(
                "data: {}\n\n",
                serde_json::json!({"model":"o3","choices":[{"index":0,"delta":{"role":"assistant","content":"done"},"finish_reason":null}]})
            )
        };
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(peer)
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(2)
            .mount(&server)
            .await;
        let profile = CompatConfig {
            supports_finish_reason: Some(false),
            supports_grammar_tools: Some(true),
            thinking_format: Some(ChatThinkingFormat::ChatTemplate),
            chat_template_kwargs: Some(std::collections::BTreeMap::from([(
                "profile".into(),
                ChatTemplateKwargValue::Literal(serde_json::json!(false)),
            )])),
            vllm_priority: Some(serde_json::Number::from_f64(1.5).unwrap()),
            ..Default::default()
        };
        let model_compat = if explicit_model {
            CompatConfig {
                supports_finish_reason: Some(true),
                supports_grammar_tools: Some(false),
                chat_template_kwargs: Some(Default::default()),
                vllm_priority: Some(0.into()),
                ..Default::default()
            }
        } else {
            CompatConfig::default()
        };
        let model = ModelInfo::new(
            "o3",
            "o3",
            opi_ai::WireApi::OpenAiCompletions,
            ModelCapabilities::new(8192, 1024)
                .with_streaming(true)
                .with_thinking(true),
        )
        .with_compat(opi_ai::WireCompat::OpenAiCompletions(model_compat))
        .unwrap();
        let provider = StdArc::new(OpenAiChatProvider::new_with_compat(
            Some(server.uri()),
            profile,
        ));
        let api = OpenAiCompletionsChatApi::new(provider, model.clone()).unwrap();
        let mut models = Models::new();
        let handle = models
            .register_chat_api(
                StdArc::new(api),
                StdArc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::Bearer,
                    secrecy::SecretString::from("offline-profile-key"),
                )),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        let tool = ToolDef {
            name: "lookup".into(),
            description: "lookup".into(),
            input_schema: serde_json::json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}),
            constrained_sampling: Some(
                opi_ai::constrained_sampling::ConstrainedSampling::Grammar {
                    openai_lark: Some("start: /.+/".into()),
                    openai_regex: None,
                },
            ),
        };
        let mut request = chat_public_request("o3", Some(opi_ai::ThinkingLevel::Low));
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            tools: vec![tool],
            ..Default::default()
        })
        .unwrap();
        let mut options = OpenAiCompletionsOptions::default();
        options.reasoning_effort = Some(opi_ai::ThinkingLevel::High);
        models
            .complete(
                ChatModelRecord::new("openai", model.clone()),
                request.clone(),
                handle.options(options),
            )
            .await
            .unwrap();
        models
            .complete_simple(
                ChatModelRecord::new("openai", model),
                request,
                Default::default(),
            )
            .await
            .unwrap();
        for sent in server.received_requests().await.unwrap() {
            let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
            assert_eq!(
                body["tools"][0]["type"],
                if explicit_model { "function" } else { "custom" }
            );
            assert_eq!(
                body["priority"],
                if explicit_model {
                    serde_json::json!(0)
                } else {
                    serde_json::json!(1.5)
                }
            );
            if explicit_model {
                assert!(body.get("chat_template_kwargs").is_none());
            } else {
                assert_eq!(
                    body["chat_template_kwargs"],
                    serde_json::json!({"profile":false})
                );
            }
        }
    }
}

#[test]
fn reviewed_chat_model_optional_overrides_apply_to_profile_routes() {
    let model = thinking_chat_model("gpt-4o", false)
        .with_compat(opi_ai::WireCompat::OpenAiCompletions(CompatConfig {
            usage_in_stream: Some(false),
            vllm_priority: Some(0.into()),
            open_router_routing: Some(Default::default()),
            ..Default::default()
        }))
        .unwrap();
    let profile = CompatConfig {
        usage_in_stream: Some(true),
        vllm_priority: Some(3.into()),
        open_router_routing: Some(
            serde_json::from_value(serde_json::json!({"only":["profile"]})).unwrap(),
        ),
        ..Default::default()
    };
    let provider = profile_provider("openai", profile, vec![model]);
    let body = provider.build_request_body(&make_test_request()).unwrap();
    assert!(body.get("stream_options").is_none());
    assert_eq!(body["priority"], 0);
    assert_eq!(body["provider"], serde_json::json!({}));
}

#[tokio::test]
async fn reviewed_chat_required_constraints_fail_before_dispatch_at_real_entries() {
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct CountAuth(StdArc<AtomicUsize>);
    impl opi_ai::auth::AuthResolver for CountAuth {
        fn resolve(
            &self,
        ) -> opi_ai::credential::BoxAuthFuture<'_, Result<opi_ai::ResolvedAuth, ProviderError>>
        {
            self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(opi_ai::test_support::resolved_bearer_auth()) })
        }
    }
    for (cap, schema, valid) in [
        (
            false,
            serde_json::json!({"type":"object","properties":{}}),
            false,
        ),
        (
            true,
            serde_json::json!({"type":"array","items":{"type":"string"}}),
            false,
        ),
        (
            true,
            serde_json::json!({"type":"object","properties":{},"additionalProperties":true}),
            false,
        ),
        (
            true,
            serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
            true,
        ),
    ] {
        for entry in ["native", "simple", "advanced"] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string(chat_sse_ok())
                        .insert_header("content-type", "text/event-stream"),
                )
                .expect(u64::from(valid))
                .mount(&server)
                .await;
            let model = ModelInfo::new(
                "o3",
                "o3",
                opi_ai::WireApi::OpenAiCompletions,
                ModelCapabilities::new(8192, 1024).with_streaming(true),
            )
            .with_compat(opi_ai::WireCompat::OpenAiCompletions(CompatConfig {
                strict_tool_schema: cap,
                ..Default::default()
            }))
            .unwrap();
            let auth = StdArc::new(AtomicUsize::new(0));
            let payload = StdArc::new(AtomicUsize::new(0));
            let mut models = Models::new();
            let handle = if entry == "advanced" {
                let provider = OpenAiChatProvider::for_route(
                    Some(server.uri()),
                    "openai".into(),
                    Default::default(),
                    vec![model.clone()],
                    StdArc::new(opi_ai::http::HttpClient::new()),
                );
                models
                    .register_route(
                        Box::new(provider),
                        StdArc::new(CountAuth(auth.clone())),
                        opi_ai::AuthProvenanceSource::Static,
                        Default::default(),
                    )
                    .unwrap();
                None
            } else {
                let api = OpenAiCompletionsChatApi::new(
                    StdArc::new(OpenAiChatProvider::new(Some(server.uri()))),
                    model.clone(),
                )
                .unwrap();
                Some(
                    models
                        .register_chat_api(
                            StdArc::new(api),
                            StdArc::new(CountAuth(auth.clone())),
                            opi_ai::AuthProvenanceSource::Static,
                        )
                        .unwrap(),
                )
            };
            let mut request = chat_public_request("o3", None);
            request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                tools: vec![ToolDef {
                    name: "required".into(),
                    description: "strict requirement".into(),
                    input_schema: schema.clone(),
                    constrained_sampling: Some(ConstrainedSampling::JsonSchema {
                        strict: StrictPreference::Require,
                    }),
                }],
                ..Default::default()
            })
            .unwrap();
            let sink = payload.clone();
            request.callbacks = opi_ai::callbacks::RequestCallbacks::new().with_on_payload(
                StdArc::new(move |_| {
                    sink.fetch_add(1, Ordering::SeqCst);
                    Ok(None)
                }),
            );
            assert_eq!(auth.load(Ordering::SeqCst), 0);
            assert_eq!(payload.load(Ordering::SeqCst), 0);
            let result = match entry {
                "native" => {
                    models
                        .complete(
                            ChatModelRecord::new("openai", model),
                            request,
                            handle.as_ref().unwrap().options(Default::default()),
                        )
                        .await
                }
                "simple" => {
                    models
                        .complete_simple(
                            ChatModelRecord::new("openai", model),
                            request,
                            Default::default(),
                        )
                        .await
                }
                _ => {
                    let call = models
                        .prepare_call("openai:o3", request)
                        .await
                        .expect("valid structure prepares the advanced route");
                    opi_ai::stream::ChatStream::new(call.start_attempt().unwrap())
                        .result()
                        .await
                }
            };
            if valid {
                assert!(result.is_ok(), "{entry}: {result:?}");
                assert_eq!(auth.load(Ordering::SeqCst), 1);
                assert_eq!(payload.load(Ordering::SeqCst), 1);
                assert_eq!(server.received_requests().await.unwrap().len(), 1);
            } else {
                let failure =
                    result.expect_err("required constraints never dispatch unsupported tools");
                assert!(
                    matches!(failure.cause(), ProviderError::Config(_)),
                    "{entry}: {:?}",
                    failure.cause()
                );
                assert_eq!(auth.load(Ordering::SeqCst), usize::from(entry != "native"));
                assert_eq!(payload.load(Ordering::SeqCst), 0);
                assert!(server.received_requests().await.unwrap().is_empty());
            }
        }
    }
}
