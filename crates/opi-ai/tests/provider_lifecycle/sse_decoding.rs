//! Anthropic SSE framing, event mapping and tool argument completion.

use crate::fixtures::{make_request, text_sse_fixture};
use futures_util::StreamExt;
use opi_ai::provider::Provider;
use opi_ai::providers::anthropic::AnthropicProvider;
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn tool_call_sse_fixture() -> &'static str {
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_tc","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4","stop_reason":null,"usage":{"input_tokens":20,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"read_file","input":{}}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"/tmp/x\"}"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":30}}

event: message_stop
data: {"type":"message_stop"}

"#
}

fn incomplete_sse_fixture() -> &'static str {
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_inc","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4","stop_reason":null,"usage":{"input_tokens":5,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Partial..."}}

"#
}

#[tokio::test]
async fn stream_text_response_produces_correct_events() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_auth(),
    );

    let mut events = Vec::new();
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                let is_terminal = event.is_terminal();
                events.push(event);
                if is_terminal {
                    break;
                }
            }
            Err(e) => panic!("unexpected error: {e}"),
        }
    }

    assert!(matches!(events[0], AssistantStreamEvent::Start { .. }));
    assert!(events.iter().any(|e| matches!(
        e,
        AssistantStreamEvent::TextDelta { delta, .. } if delta == "Hello"
    )));
    assert!(events.iter().any(|e| matches!(
        e,
        AssistantStreamEvent::TextDelta { delta, .. } if delta == " world"
    )));

    let done = events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("should have Done event");
    if let AssistantStreamEvent::Done { reason, message } = done {
        assert_eq!(*reason, StopReason::Stop);
        let text: String = message
            .content
            .iter()
            .filter_map(|c| match c {
                opi_ai::message::AssistantContent::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(text, "Hello world");
    }
}

#[tokio::test]
async fn stream_tool_call_response_produces_correct_events() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(tool_call_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_auth(),
    );

    let mut events = Vec::new();
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                let is_terminal = event.is_terminal();
                events.push(event);
                if is_terminal {
                    break;
                }
            }
            Err(e) => panic!("unexpected error: {e}"),
        }
    }

    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::ToolCallStart { .. }))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::ToolCallEnd { .. }))
    );

    let done = events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("should have Done event");
    if let AssistantStreamEvent::Done { reason, .. } = done {
        assert_eq!(*reason, StopReason::ToolUse);
    }
}

#[tokio::test]
async fn stream_no_terminal_event_produces_stream_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(incomplete_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_auth(),
    );

    let mut saw_stream_error = false;
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                if event.is_terminal() {
                    break;
                }
            }
            Err(error)
                if error.category() == opi_ai::provider::ProviderErrorCategory::Stream
                    && error.to_string().contains("terminal event") =>
            {
                saw_stream_error = true;
                break;
            }
            Err(_) => break,
        }
    }
    assert!(
        saw_stream_error,
        "should produce StreamError about missing terminal event"
    );
}

/// A raw-TCP SSE server: reads the request head+body, then writes the given
/// payload as a chunked response with one byte per chunk. Returns
/// `(base_url, shutdown)`: awaiting `shutdown` completes once the client-side
/// connection is observed closed (EOF on the server socket).
async fn byte_chunked_sse_server(payload: Vec<u8>) -> (String, tokio::task::JoinHandle<()>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut received = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            let n = socket.read(&mut buffer).await.unwrap();
            assert_ne!(n, 0, "client must send its request before streaming");
            received.extend_from_slice(&buffer[..n]);
            if let Some(end) = received.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = std::str::from_utf8(&received[..end]).unwrap();
                let length: usize = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse().unwrap())
                    })
                    .unwrap();
                if received.len() >= end + 4 + length {
                    break;
                }
            }
        }
        let mut response =
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
                .to_vec();
        for byte in payload {
            response.extend_from_slice(b"1\r\n");
            response.push(byte);
            response.extend_from_slice(b"\r\n");
        }
        response.extend_from_slice(b"0\r\n\r\n");
        let _ = socket.write_all(&response).await;
    });
    (url, server)
}

fn text_with_multibyte_fixture(separator: &str, keepalive: bool) -> Vec<u8> {
    let mut body = String::new();
    if keepalive {
        body.push_str(": keepalive\n\n");
        body.push_str(": ping\n\n");
    }
    for (name, payload) in [
        (
            "message_start",
            r#"{"type":"message_start","message":{"id":"msg_mb","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4","stop_reason":null,"usage":{"input_tokens":10,"output_tokens":0}}}"#,
        ),
        (
            "content_block_start",
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
        ),
        (
            "content_block_delta",
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"你好🌍"}}"#,
        ),
        (
            "content_block_delta",
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" trailing"}}"#,
        ),
        (
            "content_block_stop",
            r#"{"type":"content_block_stop","index":0}"#,
        ),
        (
            "message_delta",
            r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":5}}"#,
        ),
        ("message_stop", r#"{"type":"message_stop"}"#),
    ] {
        body.push_str(&format!("event: {name}\ndata: {payload}\n"));
        body.push_str(separator);
    }
    body.into_bytes()
}

#[tokio::test]
async fn anthropic_every_byte_splits_with_crlf_keepalive_and_multibyte_frame_identically() {
    for separator in ["\n", "\r\n", "\r"] {
        let (url, server) =
            byte_chunked_sse_server(text_with_multibyte_fixture(separator, true)).await;
        let provider = AnthropicProvider::new(Some(url));
        let results: Vec<_> = provider
            .stream_prepared(
                make_request(CancellationToken::new()),
                opi_ai::test_support::resolved_auth(),
            )
            .collect()
            .await;
        let _ = server.await;
        let message = results
            .into_iter()
            .find_map(|result| match result.expect("valid fragmented stream") {
                AssistantStreamEvent::Done { message, .. } => Some(message),
                _ => None,
            })
            .expect("complete fragmented stream reaches its terminal");
        let text = message
            .content
            .iter()
            .find_map(|content| match content {
                opi_ai::message::AssistantContent::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .expect("text block present");
        assert_eq!(text, "你好🌍 trailing", "separator={separator:?}");
    }
}

#[tokio::test]
async fn anthropic_large_lawful_event_beyond_withdrawn_caps_is_accepted() {
    // A single lawful SSE event far beyond the withdrawn 16 KiB-era defaults
    // must stream without a default size cap. (Single-shot body: the
    // byte-split suites cover framing; this one covers absence of caps.)
    let large_text = "x".repeat(300 * 1024);
    let mut body = String::new();
    body.push_str("event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_big\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[],\"model\":\"claude-sonnet-4\",\"stop_reason\":null,\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n");
    body.push_str("event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n");
    body.push_str(&format!(
        "event: content_block_delta\ndata: {{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{{\"type\":\"text_delta\",\"text\":\"{large_text}\"}}}}\n\n"
    ));
    body.push_str(
        "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
    );
    body.push_str("event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":5}}\n\n");
    body.push_str("event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n");

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(body)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = AnthropicProvider::new(Some(server.uri()));
    let results: Vec<_> = provider
        .stream_prepared(
            make_request(CancellationToken::new()),
            opi_ai::test_support::resolved_auth(),
        )
        .collect()
        .await;
    let message = results
        .into_iter()
        .find_map(|result| match result.expect("large lawful stream") {
            AssistantStreamEvent::Done { message, .. } => Some(message),
            _ => None,
        })
        .expect("large lawful event completes");
    let text = message
        .content
        .iter()
        .find_map(|content| match content {
            opi_ai::message::AssistantContent::Text { text, .. } => Some(text.len()),
            _ => None,
        })
        .expect("text block present");
    assert_eq!(text, 300 * 1024);
}

#[tokio::test]
async fn anthropic_invalid_utf8_bytes_are_replaced_not_fatal() {
    // AIR-006: preserve the adapter's incremental TextDecoder replacement
    // semantics rather than adding a fatal SSE UTF-8 rejection.
    let mut body: Vec<u8> = b"event: message_start\ndata: ".to_vec();
    body.extend_from_slice(
        br#"{"type":"message_start","message":{"id":"msg_inv","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4","stop_reason":null,"usage":{"input_tokens":1,"output_tokens":0}}}"#.as_slice(),
    );
    body.extend_from_slice(b"\n\n");
    body.extend_from_slice(b"event: content_block_start\ndata: ");
    // A lone continuation byte is invalid UTF-8 anywhere it appears.
    body.extend_from_slice(
        br#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#
            .as_slice(),
    );
    body.extend_from_slice(b"\n\n");
    let mut delta: Vec<u8> =
        br#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"a"#
            .to_vec();
    delta.push(0x80); // invalid lone continuation byte inside the JSON string
    delta.extend_from_slice(br#"b"}}"#.as_slice());
    body.extend_from_slice(b"event: content_block_delta\ndata: ");
    body.extend_from_slice(&delta);
    body.extend_from_slice(b"\n\n");
    body.extend_from_slice(
        b"\n\nevent: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\nevent: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":2}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
    );

    let (url, server) = byte_chunked_sse_server(body).await;
    let provider = AnthropicProvider::new(Some(url));
    let results: Vec<_> = provider
        .stream_prepared(
            make_request(CancellationToken::new()),
            opi_ai::test_support::resolved_auth(),
        )
        .collect()
        .await;
    let _ = server.await;
    let outcome = results
        .into_iter()
        .find_map(|result| match result {
            Ok(AssistantStreamEvent::Done { message, .. }) => Some(Ok(message)),
            Err(error) => Some(Err(error)),
            Ok(_) => None,
        })
        .expect("stream reaches a terminal or typed failure");
    match outcome {
        Ok(message) => {
            let text = message
                .content
                .iter()
                .find_map(|content| match content {
                    opi_ai::message::AssistantContent::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .expect("text block present");
            assert_eq!(text, "a\u{FFFD}b", "invalid byte is replaced, not fatal");
        }
        Err(error) => panic!("invalid UTF-8 must be replaced, not fatal: {error}"),
    }
}

fn tool_arguments_fixture(partial_json: &str) -> String {
    let escaped = partial_json.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        "event: message_start\ndata: {{\"type\":\"message_start\",\"message\":{{\"id\":\"msg_args\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[],\"model\":\"claude-sonnet-4\",\"stop_reason\":null,\"usage\":{{\"input_tokens\":20,\"output_tokens\":0}}}}}}\n\n\
         event: content_block_start\ndata: {{\"type\":\"content_block_start\",\"index\":0,\"content_block\":{{\"type\":\"tool_use\",\"id\":\"toolu_args\",\"name\":\"read_file\",\"input\":{{}}}}}}\n\n\
         event: content_block_delta\ndata: {{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{{\"type\":\"input_json_delta\",\"partial_json\":\"{escaped}\"}}}}\n\n\
         event: content_block_stop\ndata: {{\"type\":\"content_block_stop\",\"index\":0}}\n\n\
         event: message_delta\ndata: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"tool_use\"}},\"usage\":{{\"output_tokens\":30}}}}\n\n\
         event: message_stop\ndata: {{\"type\":\"message_stop\"}}\n\n"
    )
}

async fn anthropic_final_tool_call(partial_json: &str) -> opi_ai::message::ToolCall {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(tool_arguments_fixture(partial_json))
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = AnthropicProvider::new(Some(server.uri()));
    let results: Vec<_> = provider
        .stream_prepared(
            make_request(CancellationToken::new()),
            opi_ai::test_support::resolved_auth(),
        )
        .collect()
        .await;
    results
        .into_iter()
        .find_map(|result| match result.expect("valid tool stream") {
            AssistantStreamEvent::ToolCallEnd { tool_call, .. } => Some(tool_call),
            _ => None,
        })
        .expect("tool call terminal event")
}

#[tokio::test]
async fn anthropic_final_tool_arguments_empty_input_normalizes_to_object() {
    let tool_call = anthropic_final_tool_call("").await;
    assert_eq!(tool_call.arguments, "{}");
    assert_eq!(tool_call.arguments_error, None);
}

#[tokio::test]
async fn anthropic_final_tool_arguments_invalid_escape_repairs_literal_text() {
    let tool_call = anthropic_final_tool_call(r#"{"cmd":"\usepackage{x}","n":2}"#).await;
    assert_eq!(tool_call.arguments_error, None);
    let parsed: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&tool_call.arguments).expect("repaired arguments parse");
    assert_eq!(parsed["cmd"], r#"\usepackage{x}"#);
    assert_eq!(parsed["n"], 2);
}

#[tokio::test]
async fn anthropic_final_tool_arguments_truncated_keeps_raw_and_marks_error() {
    let raw = r#"{"path": "trunc"#;
    let tool_call = anthropic_final_tool_call(raw).await;
    assert_eq!(tool_call.arguments, raw);
    assert_eq!(
        tool_call.arguments_error.as_deref(),
        Some("final tool arguments were not a complete JSON object")
    );
}

#[tokio::test]
async fn anthropic_final_tool_arguments_non_object_keeps_raw_and_marks_error() {
    let tool_call = anthropic_final_tool_call("[1,2]").await;
    assert_eq!(tool_call.arguments, "[1,2]");
    assert!(tool_call.arguments_error.is_some());
}

#[tokio::test]
async fn anthropic_failures_preserve_unpublished_mapper_state() {
    use opi_ai::message::AssistantContent;
    use opi_ai::stream::ChatStream;

    let start = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"partial\",\"model\":\"claude\",\"usage\":{\"input_tokens\":10,\"output_tokens\":0}}}\n\n";
    let middle = concat!(
        "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"x\",\"name\":\"effect\",\"input\":{}}}\n\n",
        "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"x\\\":\"}}\n\n",
        "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"redacted_thinking\",\"data\":\"opaque-artificial\"}}\n\n",
        "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":7}}\n\n",
    );
    for ending in [
        "event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"secret-error-canary\"}}\n\n",
        "",
        "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{},\"usage\":{\"output_tokens\":-1}}\n\n",
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(format!("{start}{middle}{ending}")),
            )
            .mount(&server)
            .await;
        let provider = AnthropicProvider::new(Some(server.uri()));
        let failure = ChatStream::new(provider.stream_prepared(
            make_request(CancellationToken::new()),
            opi_ai::test_support::resolved_auth(),
        ))
        .result()
        .await
        .expect_err("provider error or premature EOF");
        assert_eq!(failure.message().usage.output_tokens, 7);
        assert_eq!(failure.message().usage.input_tokens, 10);
        assert!(
            matches!(&failure.message().content[0], AssistantContent::ToolCall { tool_call } if tool_call.arguments == "{\"x\":")
        );
        assert!(
            matches!(&failure.message().content[1], AssistantContent::RedactedThinking { data } if data == "opaque-artificial")
        );
        assert!(!format!("{:?}", failure.cause()).contains("secret-error-canary"));
    }
}
