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
        projection: Default::default(),
        transport: Default::default(),
        model: "openai-responses:gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
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
                opi_ai::message::AssistantContent::Text { text, .. } => Some(text.as_str()),
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

    let error = stream.next().await.expect("should have event").unwrap_err();
    let ProviderError::StreamFailure(failure) = error else {
        panic!("accepted projection retains facts")
    };
    assert!(failure.message().projection.is_some());
    match failure.cause() {
        ProviderError::AuthFailed(msg) => {
            assert!(
                msg.contains("provider rejected credentials"),
                "should mention auth failure: {msg}"
            );
        }
        other => panic!("expected AuthFailed, got: {other:?}"),
    }
    assert!(stream.next().await.is_none());
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
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

    let error = stream.next().await.expect("should have event").unwrap_err();
    let ProviderError::StreamFailure(failure) = error else {
        panic!("accepted projection retains facts")
    };
    assert!(failure.message().projection.is_some());
    match failure.cause() {
        ProviderError::RateLimited { retry_after_ms } => {
            assert!(retry_after_ms.is_some(), "should parse retry-after header");
            // 5 seconds -> 5000 ms
            assert_eq!(retry_after_ms.unwrap(), 5000);
        }
        other => panic!("expected RateLimited, got: {other:?}"),
    }
    assert!(stream.next().await.is_none());
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
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
            Err(msg)
                if msg.category() == opi_ai::provider::ProviderErrorCategory::Stream
                    && msg.to_string().contains("terminal event") =>
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
            opi_ai::message::AssistantContent::Text { text, .. } => Some(text.clone()),
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

async fn responses_duration_on_peer(
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
                        headers.starts_with("post /v1/responses http/1.1"),
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
        let body = text_sse_fixture();
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
    let provider = OpenAiResponsesProvider::new(Some(format!("http://{address}")));
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
    let mut request = make_request(CancellationToken::new());
    request.callbacks = opi_ai::callbacks::RequestCallbacks::new().with_async_on_response(
        move |status, _| async move {
            assert_eq!(status, 200);
            tokio::time::sleep(setup_delay).await;
            Ok(())
        },
    );
    let started = std::time::Instant::now();
    let call = models.stream_simple(
        opi_ai::ChatModelRecord::new("openai-responses", model),
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
async fn responses_duration_includes_pending_first_body_poll() {
    use std::time::Duration;
    let (control, _) = responses_duration_on_peer(Duration::ZERO, Duration::ZERO).await;
    let (delayed, elapsed) =
        responses_duration_on_peer(Duration::from_millis(350), Duration::ZERO).await;
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
async fn responses_duration_excludes_auth_setup_callback_and_unpolled_time() {
    use std::time::Duration;
    // Four independent150ms setup intervals: unpolled handle, resolver,
    // response headers and response callback. None belongs to body polling.
    let (message, elapsed) =
        responses_duration_on_peer(Duration::ZERO, Duration::from_millis(150)).await;
    let measured = Duration::from_millis(message.duration_ms.unwrap());
    assert!(
        elapsed.saturating_sub(measured) >= Duration::from_millis(500),
        "duration included setup: measured={measured:?}, elapsed={elapsed:?}"
    );
}
