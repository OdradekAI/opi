//! Shared Gemini protocol fixtures.

use super::*;

pub(crate) fn direct_cause(error: &ProviderError) -> &ProviderError {
    match error {
        ProviderError::StreamFailure(failure) => failure.cause(),
        cause => cause,
    }
}

pub(crate) fn gemini_provider() -> GeminiProvider {
    GeminiProvider::new(None)
}

/// Helper: collect stream events asynchronously.
pub(crate) async fn collect_stream(stream: EventStream) -> Vec<AssistantStreamEvent> {
    stream.filter_map(|r| async move { r.ok() }).collect().await
}

pub(crate) fn gemini_http_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "gemini:gemini-2.5-flash".into(),
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
    }
}

pub(crate) async fn spawn_stalled_gemini_error_body_server(
    status: u16,
) -> (String, std::sync::Arc<tokio::sync::Notify>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled Gemini error server");
    let addr = listener.local_addr().expect("stalled error server addr");
    let headers_flushed = std::sync::Arc::new(tokio::sync::Notify::new());
    let server_headers_flushed = headers_flushed.clone();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept Gemini request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read Gemini request");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let reason = match status {
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            _ => "Error",
        };
        let headers = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Length: 32\r\nConnection: close\r\n\r\n"
        );
        tokio::io::AsyncWriteExt::write_all(&mut socket, headers.as_bytes())
            .await
            .expect("write Gemini error headers");
        tokio::io::AsyncWriteExt::flush(&mut socket)
            .await
            .expect("flush Gemini error headers");
        server_headers_flushed.notify_one();
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    });

    (format!("http://{addr}"), headers_flushed)
}

#[derive(Clone, Copy)]
pub(crate) enum GeminiStallPoint {
    BeforeHeaders,
    ResponseBody,
    ErrorBody,
}

pub(crate) async fn spawn_stalled_gemini_server(
    stall_point: GeminiStallPoint,
) -> (String, std::sync::Arc<tokio::sync::Notify>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled Gemini server");
    let addr = listener.local_addr().expect("stalled Gemini server addr");
    let stalled = std::sync::Arc::new(tokio::sync::Notify::new());
    let server_stalled = stalled.clone();

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept Gemini request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read Gemini request");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        if !matches!(stall_point, GeminiStallPoint::BeforeHeaders) {
            let status = if matches!(stall_point, GeminiStallPoint::ErrorBody) {
                "400 Bad Request"
            } else {
                "200 OK"
            };
            tokio::io::AsyncWriteExt::write_all(
                &mut socket,
                format!(
                    "HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nContent-Length: 1024\r\nConnection: close\r\n\r\n"
                )
                .as_bytes(),
            )
            .await
            .expect("write Gemini response headers");
            tokio::io::AsyncWriteExt::flush(&mut socket)
                .await
                .expect("flush Gemini response headers");
        }
        server_stalled.notify_one();
        std::future::pending::<()>().await;
    });

    (format!("http://{addr}"), stalled)
}

// ---------------------------------------------------------------------------
pub(crate) async fn assert_gemini_cancelled(stall_point: GeminiStallPoint) {
    let (server, stalled) = spawn_stalled_gemini_server(stall_point).await;
    let cancel = CancellationToken::new();
    let provider = GeminiProvider::new(Some(server));
    let mut request = gemini_http_request();
    request.cancel = cancel.clone();
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());

    assert!(futures_util::poll!(stream.next()).is_pending());
    tokio::time::timeout(std::time::Duration::from_secs(1), stalled.notified())
        .await
        .expect("Gemini server must reach the selected stall point");
    cancel.cancel();

    let remaining = tokio::time::timeout(std::time::Duration::from_secs(1), async move {
        let mut remaining = Vec::new();
        while let Some(item) = stream.next().await {
            remaining.push(item);
        }
        remaining
    })
    .await
    .expect("Gemini cancellation must terminate without waiting for HTTP");
    assert!(
        matches!(
            remaining.as_slice(),
            [Err(cause)] if cause.category() == opi_ai::provider::ProviderErrorCategory::Cancelled
        ),
        "Gemini cancellation must yield exactly one typed error, got {remaining:?}"
    );
}
