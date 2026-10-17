//! Contract tests for OpenAiResponsesProvider::stream() -- real HTTP-level verification.
//!
//! Covers: SSE streaming, HTTP error mapping, no-terminal-event detection.
//!
//! Uses wiremock to simulate OpenAI's Responses API endpoint without live API calls.

use futures_util::StreamExt;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::openai_responses::OpenAiResponsesProvider;
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::stream::AssistantStreamEvent;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn make_request(cancel: CancellationToken) -> Request {
    Request {
        model: "openai-responses:gpt-4o".into(),
        system: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "hello".into(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

fn text_sse_fixture() -> &'static str {
    "event: response.created\n\
     data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n\
     event: response.output_item.added\n\
     data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"in_progress\",\"role\":\"assistant\",\"content\":[]}}\n\n\
     event: response.content_part.added\n\
     data: {\"type\":\"response.content_part.added\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"output_text\",\"text\":\"\"}}\n\n\
     event: response.output_text.delta\n\
     data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\"Hello\"}\n\n\
     event: response.output_text.done\n\
     data: {\"type\":\"response.output_text.done\",\"output_index\":0,\"content_index\":0,\"text\":\"Hello\"}\n\n\
     event: response.output_item.done\n\
     data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"completed\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hello\"}]}}\n\n\
     event: response.completed\n\
     data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hello\"}]}],\"usage\":{\"input_tokens\":10,\"output_tokens\":5}}}\n\n"
}

fn incomplete_sse_fixture() -> &'static str {
    "event: response.created\n\
     data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n\
     event: response.output_item.added\n\
     data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"in_progress\",\"role\":\"assistant\",\"content\":[]}}\n\n\
     event: response.content_part.added\n\
     data: {\"type\":\"response.content_part.added\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"output_text\",\"text\":\"\"}}\n\n\
     event: response.output_text.delta\n\
     data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\"Partial\"}\n\n"
}

// ---------------------------------------------------------------------------
// Happy path: SSE streaming
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_success() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = OpenAiResponsesProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_bearer_auth(),
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
            .any(|e| matches!(e, AssistantStreamEvent::Start { .. })),
        "should have Start event"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::TextDelta { .. })),
        "should have TextDelta event"
    );

    let done = events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("should have Done event");
    if let AssistantStreamEvent::Done { reason, message } = done {
        assert_eq!(*reason, opi_ai::stream::StopReason::Stop);
        let text: String = message
            .content
            .iter()
            .filter_map(|c| match c {
                opi_ai::message::AssistantContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(text, "Hello");
    }
}

#[tokio::test]
async fn stream_accepts_canonical_data_only_responses_frames() {
    let server = MockServer::start().await;
    let fixture = [
        r#"data: {"type":"response.created","response":{"id":"resp_1","status":"in_progress","model":"gpt-4o","output":[]}}"#,
        r#"data: {"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":"Hello"}"#,
        r#"data: {"type":"response.completed","response":{"id":"resp_1","status":"completed","model":"gpt-4o","output":[],"usage":{"input_tokens":10,"output_tokens":5}}}"#,
        "data: [DONE]",
    ]
    .join("\n\n");
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(format!("{fixture}\n\n"))
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = OpenAiResponsesProvider::new(Some(server.uri()));
    let results: Vec<_> = provider
        .stream_prepared(
            make_request(CancellationToken::new()),
            opi_ai::test_support::resolved_bearer_auth(),
        )
        .collect()
        .await;

    assert!(results.iter().all(Result::is_ok), "{results:?}");
    assert!(results.iter().any(|result| matches!(
        result,
        Ok(AssistantStreamEvent::TextDelta { delta, .. }) if delta == "Hello"
    )));
    assert!(
        results
            .iter()
            .any(|result| matches!(result, Ok(AssistantStreamEvent::Done { .. })))
    );
}

// ---------------------------------------------------------------------------
// HTTP error mapping: 401 -> AuthFailed
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_auth_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string(r#"{"error":{"message":"invalid api key"}}"#),
        )
        .mount(&server)
        .await;

    let provider = OpenAiResponsesProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_bearer_auth(),
    );

    let result = stream.next().await.expect("should have event");
    match result {
        Err(ProviderError::AuthFailed(msg)) => {
            assert!(
                msg.contains("provider rejected credentials"),
                "should mention auth failure: {msg}"
            );
        }
        other => panic!("expected AuthFailed, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// HTTP error mapping: 429 -> RateLimited
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_rate_limited() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_string(r#"{"error":{"message":"too many requests"}}"#)
                .insert_header("retry-after", "5"),
        )
        .mount(&server)
        .await;

    let provider = OpenAiResponsesProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_bearer_auth(),
    );

    let result = stream.next().await.expect("should have event");
    match result {
        Err(ProviderError::RateLimited { retry_after_ms }) => {
            assert!(retry_after_ms.is_some(), "should parse retry-after header");
            // 5 seconds -> 5000 ms
            assert_eq!(retry_after_ms.unwrap(), 5000);
        }
        other => panic!("expected RateLimited, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// No terminal event -> StreamError
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_no_terminal_event() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(incomplete_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = OpenAiResponsesProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_bearer_auth(),
    );

    let mut saw_stream_error = false;
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                if event.is_terminal() {
                    break;
                }
            }
            Err(ProviderError::StreamError(msg)) if msg.contains("terminal event") => {
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

// ---------------------------------------------------------------------------
// Task 20.5 (AIR-006/V-02/AIR-048/V-04): byte-exact framing, replacement
// decoding, final tool arguments, and producer barriers.
// ---------------------------------------------------------------------------

use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Raw-TCP chunked SSE server writing `payload` one byte per chunk; returns
/// `(base_url, written_rx)`. `written_rx` fires once the whole payload has
/// been handed to the socket.
async fn chunked_sse_server(payload: Vec<u8>) -> (String, tokio::sync::oneshot::Receiver<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (written_tx, written_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut received = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            let n = socket.read(&mut buffer).await.unwrap();
            assert_ne!(n, 0);
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
        let _ = written_tx.send(());
        let mut sink = Vec::new();
        let _ = socket.read_to_end(&mut sink).await;
    });
    (url, written_rx)
}

fn responses_multibyte_fixture() -> Vec<u8> {
    let body = "event: response.created\n\
         data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_mb\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n\
         event: response.output_item.added\n\
         data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"in_progress\",\"role\":\"assistant\",\"content\":[]}}\n\n\
         event: response.content_part.added\n\
         data: {\"type\":\"response.content_part.added\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"output_text\",\"text\":\"\"}}\n\n\
         event: response.output_text.delta\n\
         data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\"你好🌍\"}\n\n\
         event: response.output_text.done\n\
         data: {\"type\":\"response.output_text.done\",\"output_index\":0,\"content_index\":0,\"text\":\"你好🌍\"}\n\n\
         event: response.output_item.done\n\
         data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"message\",\"status\":\"completed\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"你好🌍\"}]}}\n\n\
         event: response.completed\n\
         data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_mb\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"你好🌍\"}]}],\"usage\":{\"input_tokens\":10,\"output_tokens\":5}}}\n\n";
    body.as_bytes().to_vec()
}

#[tokio::test]
async fn responses_multibyte_text_survives_every_byte_splits() {
    let (url, _written) = chunked_sse_server(responses_multibyte_fixture()).await;
    let provider = OpenAiResponsesProvider::new(Some(url));
    let results: Vec<_> = provider
        .stream_prepared(
            make_request(CancellationToken::new()),
            opi_ai::test_support::resolved_bearer_auth(),
        )
        .collect()
        .await;
    let message = results
        .into_iter()
        .find_map(
            |result| match result.expect("valid fragmented responses stream") {
                AssistantStreamEvent::Done { message, .. } => Some(message),
                _ => None,
            },
        )
        .expect("complete fragmented responses stream");
    let text = message
        .content
        .iter()
        .find_map(|content| match content {
            opi_ai::message::AssistantContent::Text { text } => Some(text.clone()),
            _ => None,
        })
        .expect("text block present");
    assert_eq!(text, "你好🌍");
}

fn responses_tool_fixture(arguments: &str) -> String {
    let escaped = arguments.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        "event: response.created\n\
         data: {{\"type\":\"response.created\",\"response\":{{\"id\":\"resp_tool\",\"status\":\"in_progress\",\"model\":\"gpt-4o\",\"output\":[]}}}}\n\n\
         event: response.output_item.added\n\
         data: {{\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{{\"type\":\"function_call\",\"id\":\"fc_args\",\"call_id\":\"call_args\",\"name\":\"read_file\",\"arguments\":\"\"}}}}\n\n\
         event: response.function_call_arguments.delta\n\
         data: {{\"type\":\"response.function_call_arguments.delta\",\"output_index\":0,\"item_id\":\"fc_args\",\"call_id\":\"call_args\",\"delta\":\"{escaped}\"}}\n\n\
         event: response.output_item.done\n\
         data: {{\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{{\"type\":\"function_call\",\"id\":\"fc_args\",\"call_id\":\"call_args\",\"name\":\"read_file\",\"arguments\":\"{escaped}\"}}}}\n\n\
         event: response.completed\n\
         data: {{\"type\":\"response.completed\",\"response\":{{\"id\":\"resp_tool\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[{{\"type\":\"function_call\",\"id\":\"fc_args\",\"call_id\":\"call_args\",\"name\":\"read_file\",\"arguments\":\"{escaped}\"}}],\"usage\":{{\"input_tokens\":20,\"output_tokens\":10}}}}}}\n\n"
    )
}

async fn responses_final_tool_call(arguments: &str) -> opi_ai::message::ToolCall {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(responses_tool_fixture(arguments))
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = OpenAiResponsesProvider::new(Some(server.uri()));
    let results: Vec<_> = provider
        .stream_prepared(
            make_request(CancellationToken::new()),
            opi_ai::test_support::resolved_bearer_auth(),
        )
        .collect()
        .await;
    results
        .into_iter()
        .find_map(
            |result| match result.expect("valid responses tool stream") {
                AssistantStreamEvent::ToolCallEnd { tool_call, .. } => Some(tool_call),
                _ => None,
            },
        )
        .expect("responses tool call terminal")
}

#[tokio::test]
async fn responses_final_tool_arguments_empty_input_normalizes_to_object() {
    let tool_call = responses_final_tool_call("").await;
    assert_eq!(tool_call.arguments, "{}");
    assert_eq!(tool_call.arguments_error, None);
}

#[tokio::test]
async fn responses_final_tool_arguments_invalid_escape_repairs_literal_text() {
    let tool_call = responses_final_tool_call(r#"{"cmd":"\usepackage{x}","n":2}"#).await;
    assert_eq!(tool_call.arguments_error, None);
    let parsed: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&tool_call.arguments).expect("repaired arguments parse");
    assert_eq!(parsed["cmd"], r#"\usepackage{x}"#);
    assert_eq!(parsed["n"], 2);
}

#[tokio::test]
async fn responses_final_tool_arguments_truncated_keeps_raw_and_marks_error() {
    let raw = r#"{"path": "trunc"#;
    let tool_call = responses_final_tool_call(raw).await;
    assert_eq!(tool_call.arguments, raw);
    assert_eq!(
        tool_call.arguments_error.as_deref(),
        Some("final tool arguments were not a complete JSON object")
    );
}

#[tokio::test]
async fn responses_final_tool_arguments_non_object_keeps_raw_and_marks_error() {
    let tool_call = responses_final_tool_call("\"text\"").await;
    assert_eq!(tool_call.arguments, "\"text\"");
    assert!(tool_call.arguments_error.is_some());
}
