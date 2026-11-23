//! Continued execution after observer drop and release of owned HTTP attempts.

use crate::fixtures::{make_request, text_sse_fixture};
use futures_util::StreamExt;
use opi_ai::provider::Provider;
use opi_ai::providers::anthropic::AnthropicProvider;
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn started_anthropic_call_completes_after_consumer_is_dropped() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let provider = AnthropicProvider::new(Some(server.uri()));
    let cancel = CancellationToken::new();
    let (completed, observed) = tokio::sync::oneshot::channel();
    let mut completed = Some(completed);
    let source = provider
        .stream_prepared(
            make_request(cancel.clone()),
            opi_ai::test_support::resolved_auth(),
        )
        .inspect(move |event| {
            if let Ok(AssistantStreamEvent::Done { message, .. }) = event {
                let _ = completed.take().unwrap().send(message.clone());
            }
        });
    drop(opi_ai::ChatStream::new(Box::pin(source)).start());
    let message = tokio::time::timeout(std::time::Duration::from_secs(5), observed)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(message.stop_reason, StopReason::Stop);
    assert_eq!(message.usage.output_tokens, 5);
    assert!(!cancel.is_cancelled());
    server.verify().await;
}

#[tokio::test]
async fn stream_cancellation_drains_without_hang_after_cancel() {
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

    let cancel = CancellationToken::new();
    let provider = AnthropicProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(cancel.clone()),
        opi_ai::test_support::resolved_auth(),
    );

    // Read at least one event then cancel
    let first = stream.next().await.expect("should have Start event");
    assert!(first.is_ok(), "first event should be Ok");
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
        .expect("stream must drain promptly after cancellation (no hang)");
}

/// Staged raw-TCP SSE server: reads the request, writes the prologue bytes,
/// signals `sent`, then holds the connection open until the client closes it
/// (EOF), when it acks on the returned receiver.
async fn staged_sse_server(
    prologue: Vec<u8>,
) -> (
    String,
    tokio::sync::oneshot::Receiver<()>,
    tokio::sync::oneshot::Receiver<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (sent_tx, sent_rx) = tokio::sync::oneshot::channel();
    let (closed_tx, closed_rx) = tokio::sync::oneshot::channel();
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
        let head = b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
        let chunked: Vec<u8> = std::iter::once(&head[..])
            .chain(
                prologue
                    .chunks(1)
                    .flat_map(|byte| [b"1\r\n".as_slice(), byte, b"\r\n".as_slice()]),
            )
            .flatten()
            .copied()
            .collect();
        let _ = socket.write_all(&chunked).await;
        let _ = sent_tx.send(());
        let mut sink = Vec::new();
        let _ = socket.read_to_end(&mut sink).await;
        let _ = closed_tx.send(());
    });
    (url, sent_rx, closed_rx)
}

fn many_delta_events_fixture(count: usize) -> Vec<u8> {
    let mut body = String::new();
    body.push_str("event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_q\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[],\"model\":\"claude-sonnet-4\",\"stop_reason\":null,\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n");
    body.push_str("event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n");
    for i in 0..count {
        body.push_str(&format!(
            "event: content_block_delta\ndata: {{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{{\"type\":\"text_delta\",\"text\":\"d{i}\"}}}}\n\n"
        ));
    }
    body.into_bytes()
}

#[tokio::test]
async fn anthropic_cancel_discards_buffered_events_at_the_queue_barrier() {
    // Start the lazy producer, then fill the bounded queue far beyond its
    // 64 slots before consuming events. After cancellation the next item is typed
    // cancellation, never a pre-cancel buffered event.
    let (url, sent_rx, _closed_rx) = staged_sse_server(many_delta_events_fixture(200)).await;
    let provider = AnthropicProvider::new(Some(url));
    let cancel = CancellationToken::new();
    let mut stream = provider.stream_prepared(
        make_request(cancel.clone()),
        opi_ai::test_support::resolved_auth(),
    );
    assert!(futures_util::poll!(stream.next()).is_pending());

    // Confirm the wait point: the server handed the whole payload to the
    // socket, so the producer has parsed events and is parked on the queue.
    tokio::time::timeout(std::time::Duration::from_secs(2), sent_rx)
        .await
        .expect("producer starts after the first poll")
        .expect("server wrote payload");

    // Confirm the queue wait point: one event is demonstrably buffered
    // behind the next, so pre-cancel events exist to discard.
    use futures_util::StreamExt as _;
    let first = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("producer delivers into the queue")
        .expect("stream alive");
    assert!(first.is_ok());

    cancel.cancel();
    let first = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("cancellation must be visible without draining the queue");
    match first {
        Some(Err(cause))
            if cause.category() == opi_ai::provider::ProviderErrorCategory::Cancelled => {}
        other => panic!("cancellation must take priority over buffered events, got {other:?}"),
    }
    assert!(
        stream.next().await.is_none(),
        "stream must stay at EOF after the cancellation barrier"
    );
}

#[tokio::test]
async fn anthropic_drop_ends_delivery_and_a_new_attempt_is_unaffected() {
    // Drop after the first delivered event while the server still streams:
    // the dropped attempt delivers nothing further (its queue and read waits
    // end with the attempt), and a fresh attempt on the same provider
    // completes normally. Socket teardown timing is a transport internal and
    // is not asserted; attempt-slot release at the public ChatStream seam is
    // covered by the stream_events suite.
    let (url, sent_rx, _closed_rx) = staged_sse_server(many_delta_events_fixture(4)).await;
    let provider = AnthropicProvider::new(Some(url));
    let cancel = CancellationToken::new();
    let mut stream =
        provider.stream_prepared(make_request(cancel), opi_ai::test_support::resolved_auth());

    use futures_util::StreamExt as _;
    let first = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("first event arrives")
        .expect("stream alive");
    assert!(first.is_ok());
    drop(stream);
    let _ = sent_rx.await;

    // A fresh logical attempt on the same provider completes normally.
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
    let mut next = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_auth(),
    );
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(2), next.next())
        .await
        .expect("fresh attempt delivers")
        .expect("fresh attempt alive");
    assert!(outcome.is_ok());
}

#[tokio::test]
async fn anthropic_cancel_at_the_connect_barrier_is_immediately_visible() {
    // The server accepts the connection and reads the request but never
    // writes a response head: cancellation during the connect/response wait
    // must surface as the one typed cancellation without any delivery.
    use tokio::io::AsyncReadExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (connected_tx, connected_rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut sink = Vec::new();
        let mut buffer = [0u8; 4096];
        // Read the full request head+body, then hold the response open.
        loop {
            let n = socket.read(&mut buffer).await.unwrap();
            if n == 0 {
                break;
            }
            sink.extend_from_slice(&buffer[..n]);
            if let Some(end) = sink.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = std::str::from_utf8(&sink[..end]).unwrap();
                let length: usize = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse().unwrap())
                    })
                    .unwrap();
                if sink.len() >= end + 4 + length {
                    break;
                }
            }
        }
        let _ = connected_tx.send(());
        let mut drain = Vec::new();
        let _ = socket.read_to_end(&mut drain).await;
    });

    let provider = AnthropicProvider::new(Some(url));
    let cancel = CancellationToken::new();
    let mut stream = provider.stream_prepared(
        make_request(cancel.clone()),
        opi_ai::test_support::resolved_auth(),
    );

    assert!(futures_util::poll!(stream.next()).is_pending());
    tokio::time::timeout(std::time::Duration::from_secs(2), connected_rx)
        .await
        .expect("lazy producer reaches the connect barrier")
        .expect("server accepted the request");
    cancel.cancel();
    use futures_util::StreamExt as _;
    let first = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("cancellation must be visible during the connect wait");
    match first {
        Some(Err(cause))
            if cause.category() == opi_ai::provider::ProviderErrorCategory::Cancelled => {}
        other => panic!("expected typed cancellation, got {other:?}"),
    }
    assert!(stream.next().await.is_none());
}

async fn assert_drop_closes_owned_http_attempt(bedrock: bool, response_started: bool) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let cancel = CancellationToken::new();
    let provider: Box<dyn Provider> = if bedrock {
        Box::new(opi_ai::providers::bedrock::BedrockProvider::new(
            Some(url),
            std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        ))
    } else {
        Box::new(opi_ai::providers::gemini::GeminiProvider::new(Some(url)))
    };
    let auth = if bedrock {
        opi_ai::ResolvedAuth::aws_sigv4(
            opi_ai::AwsSigV4Credentials {
                access_key_id: "offline-public-id".into(),
                secret_access_key: "offline-public-secret".into(),
                session_token: None,
                region: "us-east-1".into(),
                expires_at: None,
            },
            Default::default(),
        )
    } else {
        opi_ai::test_support::resolved_auth()
    };
    let mut request = make_request(cancel.clone());
    request.model = if bedrock {
        "bedrock:amazon.nova-lite-v1:0"
    } else {
        "gemini:gemini-2.5-flash"
    }
    .into();
    let mut stream = provider.stream_prepared(request, auth);
    assert!(futures_util::poll!(stream.next()).is_pending());
    let (mut socket, _) =
        tokio::time::timeout(std::time::Duration::from_secs(5), listener.accept())
            .await
            .unwrap()
            .unwrap();
    let mut received = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let count = socket.read(&mut buf).await.unwrap();
        assert_ne!(count, 0);
        received.extend_from_slice(&buf[..count]);
        if let Some(end) = received.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
            let headers = std::str::from_utf8(&received[..end]).unwrap();
            let length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap();
            if received.len() >= end + 4 + length {
                break;
            }
        }
    }
    if response_started {
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
        let body = if bedrock {
            opi_ai::api::bedrock::event_stream::build_test_frame(
                "messageStart",
                "application/json",
                br#"{"role":"assistant"}"#,
            )
        } else {
            b"data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"barrier\"}]}}]}\n\n"
                .to_vec()
        };
        socket
            .write_all(format!("{:x}\r\n", body.len()).as_bytes())
            .await
            .unwrap();
        socket.write_all(&body).await.unwrap();
        socket.write_all(b"\r\n").await.unwrap();
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
                .await
                .unwrap()
                .unwrap()
                .is_ok()
        );
    }
    drop(stream);
    let closed =
        tokio::time::timeout(std::time::Duration::from_secs(2), socket.read(&mut buf)).await;
    assert!(
        !cancel.is_cancelled(),
        "drop must preserve the caller token"
    );
    cancel.cancel(); // also release the producer when this regression fails
    assert!(
        matches!(closed, Ok(Ok(0))),
        "dropped owned attempt must release its HTTP wait: {closed:?}"
    );
}

#[tokio::test]
async fn gemini_drop_releases_connect_and_read_waits() {
    for response_started in [false, true] {
        assert_drop_closes_owned_http_attempt(false, response_started).await;
    }
}

#[tokio::test]
async fn bedrock_drop_releases_connect_and_read_waits() {
    for response_started in [false, true] {
        assert_drop_closes_owned_http_attempt(true, response_started).await;
    }
}
