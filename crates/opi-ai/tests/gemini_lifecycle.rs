//! Contract tests for GeminiProvider::stream() -- real HTTP-level verification.
//!
//! Covers: SSE streaming, HTTP error mapping, no-terminal-event detection.
//!
//! Uses wiremock to simulate the Gemini streamGenerateContent API endpoint
//! without live API calls.

use futures_util::StreamExt;
use opi_ai::gemini::GeminiProvider;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::stream::AssistantStreamEvent;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn make_request(cancel: CancellationToken) -> Request {
    Request {
        model: "gemini:gemini-2.5-flash".into(),
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
        callbacks: Default::default(),
    }
}

fn text_sse_fixture() -> &'static str {
    "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hello\"}]},\"index\":0}]}\n\n\
     data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":10,\"candidatesTokenCount\":5,\"totalTokenCount\":15}}\n\n"
}

fn incomplete_sse_fixture() -> &'static str {
    "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Partial\"}]},\"index\":0}]}\n\n"
}

// ---------------------------------------------------------------------------
// Happy path: SSE streaming
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_success() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1beta/models/gemini-2.5-flash:streamGenerateContent",
        ))
        .and(header("x-goog-api-key", "test-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = GeminiProvider::new(Some(server.uri()));
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

// ---------------------------------------------------------------------------
// HTTP error mapping: auth error -> AuthFailed
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_auth_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1beta/models/gemini-2.5-flash:streamGenerateContent",
        ))
        .respond_with(ResponseTemplate::new(400).set_body_string(
            r#"{"error":{"code":401,"message":"API key not valid","status":"INVALID_ARGUMENT"}}"#,
        ))
        .mount(&server)
        .await;

    let provider = GeminiProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_auth(),
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
        .and(path(
            "/v1beta/models/gemini-2.5-flash:streamGenerateContent",
        ))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_string(r#"{"error":{"code":429,"message":"Resource exhausted"}}"#)
                .insert_header("retry-after", "5"),
        )
        .mount(&server)
        .await;

    let provider = GeminiProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_auth(),
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
        .and(path(
            "/v1beta/models/gemini-2.5-flash:streamGenerateContent",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(incomplete_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = GeminiProvider::new(Some(server.uri()));
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
// Task 20.5 (AIR-006/V-02/V-04): replacement decoding at byte boundaries and
// cancellation priority over buffered events.
// ---------------------------------------------------------------------------

use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

async fn gemini_chunked_server(payload: Vec<u8>) -> (String, tokio::sync::oneshot::Receiver<()>) {
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

#[tokio::test]
async fn gemini_multibyte_text_survives_byte_chunked_stream_with_replacement() {
    use serde_json::json;
    let opening =
        json!({"candidates":[{"content":{"role":"model","parts":[{"text":""}]},"index":0}]});
    // Serialize with an ASCII placeholder, then splice a raw invalid byte so
    // the wire carries a genuinely malformed UTF-8 sequence inside the JSON
    // string (serde would otherwise escape it into a legal \u0080 form).
    let delta = json!({"candidates":[{"content":{"role":"model","parts":[{"text":"a<INVALID>b你好🌍"}]},"index":0}]});
    let closing = json!({"candidates":[{"content":{"role":"model","parts":[{"text":""}]},"finishReason":"STOP","index":0}],"usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":5,"totalTokenCount":15}});
    let mut body: Vec<u8> = Vec::new();
    for event in [&opening, &delta, &closing] {
        let mut frame = format!("data: {event}\n\n").into_bytes();
        body.append(&mut frame);
    }
    // Replace the placeholder bytes <INVALID> with one raw invalid byte.
    let marker = b"<INVALID>";
    if let Some(pos) = find_subslice(&body, marker) {
        body.splice(
            pos..pos + marker.len(),
            [0x80u8].into_iter().collect::<Vec<u8>>(),
        );
    }

    let (url, _written) = gemini_chunked_server(body).await;
    let provider = GeminiProvider::new(Some(url));
    let results: Vec<_> = provider
        .stream_prepared(
            make_request(CancellationToken::new()),
            opi_ai::test_support::resolved_auth(),
        )
        .collect()
        .await;
    let message = results
        .into_iter()
        .find_map(|result| match result.expect("valid gemini stream") {
            AssistantStreamEvent::Done { message, .. } => Some(message),
            _ => None,
        })
        .expect("complete gemini stream");
    let text = message
        .content
        .iter()
        .find_map(|content| match content {
            opi_ai::message::AssistantContent::Text { text } => Some(text.clone()),
            _ => None,
        })
        .expect("text block present");
    assert_eq!(text, "a\u{FFFD}b你好🌍");
}

#[tokio::test]
async fn gemini_cancel_discards_buffered_events_at_the_queue_barrier() {
    let mut body: Vec<u8> = Vec::new();
    for _ in 0..200 {
        body.extend_from_slice(
            b"data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"d\"}]},\"index\":0}]}\n\n".as_slice(),
        );
    }
    let (url, written_rx) = gemini_chunked_server(body).await;
    let provider = GeminiProvider::new(Some(url));
    let cancel = CancellationToken::new();
    let mut stream = provider.stream_prepared(
        make_request(cancel.clone()),
        opi_ai::test_support::resolved_auth(),
    );

    written_rx.await.expect("server wrote payload");
    use futures_util::StreamExt as _;
    let first = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("producer delivers into the queue")
        .expect("stream alive");
    assert!(first.is_ok());

    cancel.cancel();
    let next = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("cancellation must be visible without draining the queue");
    match next {
        Some(Err(cause))
            if cause.category() == opi_ai::provider::ProviderErrorCategory::Cancelled => {}
        other => panic!("cancellation must take priority, got {other:?}"),
    }
    assert!(stream.next().await.is_none());
}
