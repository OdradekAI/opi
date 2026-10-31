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

fn direct_cause(error: &ProviderError) -> &ProviderError {
    match error {
        ProviderError::StreamFailure(failure) => failure.cause(),
        cause => cause,
    }
}

fn make_request(cancel: CancellationToken) -> Request {
    Request {
        transport: Default::default(),
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
        telemetry_context: None,
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
                opi_ai::message::AssistantContent::Text { text, .. } => Some(text.as_str()),
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
    match result.as_ref().map_err(direct_cause) {
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
    match result.as_ref().map_err(direct_cause) {
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
            opi_ai::message::AssistantContent::Text { text, .. } => Some(text.clone()),
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

    assert!(futures_util::poll!(stream.next()).is_pending());
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

async fn google_public_result(
    sse: &str,
    vertex: bool,
    simple: bool,
    callbacks: opi_ai::callbacks::RequestCallbacks,
) -> Result<opi_ai::message::AssistantMessage, opi_ai::stream::CallFailure> {
    let mut request = make_request(CancellationToken::new());
    request.callbacks = callbacks;
    google_public_request(sse, vertex, simple, request).await
}

async fn google_public_request(
    sse: &str,
    vertex: bool,
    simple: bool,
    request: Request,
) -> Result<opi_ai::message::AssistantMessage, opi_ai::stream::CallFailure> {
    use std::sync::Arc;
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
    let wire = if vertex {
        opi_ai::WireApi::GoogleVertex
    } else {
        opi_ai::WireApi::GoogleGenerativeAi
    };
    let model = opi_ai::ModelInfo::new(
        "gemini-2.5-flash",
        "fixture",
        wire,
        opi_ai::ModelCapabilities::new(10000, 1000).with_streaming(true),
    );
    let record =
        opi_ai::ChatModelRecord::new(if vertex { "vertex" } else { "gemini" }, model.clone());
    let mut models = opi_ai::Models::new();
    let result = if vertex {
        let provider = opi_ai::vertex::VertexProvider::new(
            "p1".into(),
            "us-central1".into(),
            Some(server.uri()),
        );
        let api = opi_ai::vertex::GoogleVertexChatApi::new(Arc::new(provider), model).unwrap();
        let handle = models
            .register_chat_api(
                Arc::new(api),
                Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::AuthScheme::Bearer,
                    "fixture-token".into(),
                )),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        if simple {
            models
                .complete_simple(record, request, Default::default())
                .await
        } else {
            models
                .complete(
                    record,
                    request,
                    handle.options(opi_ai::vertex::GoogleVertexOptions::default()),
                )
                .await
        }
    } else {
        let provider = GeminiProvider::new(Some(server.uri()));
        let api =
            opi_ai::gemini::GoogleGenerativeAiChatApi::new(Arc::new(provider), model).unwrap();
        let handle = models
            .register_chat_api(
                Arc::new(api),
                Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::AuthScheme::ApiKey,
                    "fixture-key".into(),
                )),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        if simple {
            models
                .complete_simple(record, request, Default::default())
                .await
        } else {
            models
                .complete(
                    record,
                    request,
                    handle.options(opi_ai::gemini::GoogleGenerativeAiOptions::default()),
                )
                .await
        }
    };
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    result
}

fn google_usage_frame(finish: Option<&str>, input: u32, output: u32, total: u32) -> String {
    let mut value = serde_json::json!({"candidates":[{"content":{"parts":[{"text":"partial"}]}}],"usageMetadata":{"promptTokenCount":input,"candidatesTokenCount":output,"totalTokenCount":total}});
    if let Some(finish) = finish {
        value["candidates"][0]["finishReason"] = finish.into();
    }
    format!("data: {value}\n\n")
}

fn google_direct_stream(
    base: String,
    vertex: bool,
    request: Request,
) -> opi_ai::provider::EventStream {
    if vertex {
        opi_ai::vertex::VertexProvider::new("p1".into(), "us-central1".into(), Some(base))
            .stream_prepared(request, opi_ai::test_support::resolved_bearer_auth())
    } else {
        GeminiProvider::new(Some(base))
            .stream_prepared(request, opi_ai::test_support::resolved_auth())
    }
}

fn assert_google_failed_response(message: &opi_ai::message::AssistantMessage, vertex: bool) {
    assert_eq!(message.provider, if vertex { "vertex" } else { "gemini" });
    assert_eq!(message.model, "gemini-2.5-flash");
    assert_eq!(
        message.wire,
        Some(if vertex {
            opi_ai::WireApi::GoogleVertex
        } else {
            opi_ai::WireApi::GoogleGenerativeAi
        })
    );
    assert!(message.duration_ms.is_some());
    assert!(!message.usage.reported);
    assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Error);
}

#[tokio::test]
async fn google_models_preserve_independently_reported_total() {
    for vertex in [false, true] {
        for simple in [false, true] {
            let message = google_public_result(
                &google_usage_frame(Some("STOP"), 11, 7, 99),
                vertex,
                simple,
                Default::default(),
            )
            .await
            .unwrap();
            assert!(message.usage.reported);
            assert_eq!(message.usage.bucket_total_tokens(), 18);
            assert_eq!(message.usage.total_tokens, Some(99));
        }
    }
}

#[tokio::test]
async fn google_usage_updates_before_and_after_finish_reach_final_result() {
    for vertex in [false, true] {
        for trailing in [false, true] {
            let sse = if trailing {
                format!(
                    "{}data: {{\"usageMetadata\":{{\"promptTokenCount\":20,\"candidatesTokenCount\":10,\"totalTokenCount\":30}}}}\n\n",
                    google_usage_frame(Some("STOP"), 11, 7, 18)
                )
            } else {
                format!(
                    "{}data: {{\"candidates\":[{{\"finishReason\":\"STOP\"}}]}}\n\n",
                    google_usage_frame(None, 20, 10, 30)
                )
            };
            let message = google_public_result(&sse, vertex, false, Default::default())
                .await
                .unwrap();
            assert!(message.usage.reported);
            assert_eq!(
                (
                    message.usage.input_tokens,
                    message.usage.output_tokens,
                    message.usage.total_tokens
                ),
                (20, 10, Some(30))
            );
        }
    }
}

#[tokio::test]
async fn google_started_failures_retain_received_usage() {
    use std::sync::Arc;
    for vertex in [false, true] {
        for failure in ["eof", "parser", "callback", "after_finish"] {
            let mut sse =
                google_usage_frame((failure == "after_finish").then_some("STOP"), 11, 7, 99);
            let callbacks = if failure == "callback" {
                sse.push_str("data: {\"fail_callback\":true}\n\n");
                opi_ai::callbacks::RequestCallbacks::new().with_on_raw(Arc::new(|event| {
                    if event.get("fail_callback").is_some() {
                        Err(opi_ai::callbacks::CallbackError::new("callback fixture"))
                    } else {
                        Ok(())
                    }
                }))
            } else {
                if failure == "parser" || failure == "after_finish" {
                    sse.push_str("data: {broken\n\n");
                }
                Default::default()
            };
            let error = google_public_result(&sse, vertex, false, callbacks)
                .await
                .unwrap_err();
            let message = error.message();
            assert!(message.usage.reported, "{vertex}/{failure}: {message:?}");
            assert_eq!(
                (
                    message.usage.input_tokens,
                    message.usage.output_tokens,
                    message.usage.total_tokens
                ),
                (11, 7, Some(99))
            );
            assert!(message.duration_ms.is_some());
        }
    }
}

#[tokio::test]
async fn google_models_retain_source_stop_reason_and_duration() {
    for vertex in [false, true] {
        for reason in ["STOP", "MAX_TOKENS", "SAFETY"] {
            let result = google_public_result(
                &google_usage_frame(Some(reason), 11, 7, 18),
                vertex,
                false,
                Default::default(),
            )
            .await;
            let message = match result {
                Ok(message) => message,
                Err(error) => error.into_parts().0,
            };
            assert_eq!(message.raw_stop_reason.as_deref(), Some(reason));
            assert!(message.duration_ms.is_some());
            let reload: opi_ai::message::AssistantMessage =
                serde_json::from_slice(&serde_json::to_vec(&message).unwrap()).unwrap();
            assert_eq!(reload, message);
        }
    }
}

#[tokio::test(start_paused = true)]
async fn google_direct_duration_starts_at_activation_and_includes_payload_wait() {
    use std::sync::Arc;
    for vertex in [false, true] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(text_sse_fixture())
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let observed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut request = make_request(CancellationToken::new());
        request.callbacks = opi_ai::callbacks::RequestCallbacks::new().with_async_on_payload({
            let entered = entered.clone();
            let release = release.clone();
            let observed = observed.clone();
            move |_| {
                let entered = entered.clone();
                let release = release.clone();
                let observed = observed.clone();
                async move {
                    observed.store(true, std::sync::atomic::Ordering::SeqCst);
                    entered.notify_one();
                    release.notified().await;
                    Ok(None)
                }
            }
        });
        let stream = if vertex {
            opi_ai::vertex::VertexProvider::new(
                "p1".into(),
                "us-central1".into(),
                Some(server.uri()),
            )
            .stream_prepared(request, opi_ai::test_support::resolved_bearer_auth())
        } else {
            GeminiProvider::new(Some(server.uri()))
                .stream_prepared(request, opi_ai::test_support::resolved_auth())
        };
        tokio::time::advance(std::time::Duration::from_secs(60)).await;
        assert!(!observed.load(std::sync::atomic::Ordering::SeqCst));
        assert!(server.received_requests().await.unwrap().is_empty());
        let call = tokio::spawn(opi_ai::stream::ChatStream::new(stream).result());
        entered.notified().await;
        tokio::time::advance(std::time::Duration::from_secs(7)).await;
        release.notify_one();
        let message = call.await.unwrap().unwrap();
        assert_eq!(message.duration_ms, Some(7000));
    }
}

#[tokio::test]
async fn google_usage_distinguishes_missing_partial_and_explicit_zero() {
    for vertex in [false, true] {
        for usage in [
            serde_json::json!(null),
            serde_json::json!({"promptTokenCount":11,"totalTokenCount":99}),
            serde_json::json!({"promptTokenCount":0,"candidatesTokenCount":0,"thoughtsTokenCount":0,"totalTokenCount":0}),
        ] {
            let mut frame = serde_json::json!({"candidates":[{"finishReason":"STOP"}]});
            if !usage.is_null() {
                frame["usageMetadata"] = usage.clone();
            }
            let message = google_public_result(
                &format!("data: {frame}\n\n"),
                vertex,
                false,
                Default::default(),
            )
            .await
            .unwrap();
            assert_eq!(
                message.usage.reported,
                usage.get("candidatesTokenCount").is_some()
            );
            assert_eq!(
                message.usage.total_tokens,
                usage
                    .get("totalTokenCount")
                    .and_then(serde_json::Value::as_u64)
            );
            assert_eq!(
                message.usage.reasoning_tokens,
                usage
                    .get("thoughtsTokenCount")
                    .and_then(serde_json::Value::as_u64)
            );
        }
    }
}

#[tokio::test]
async fn google_usage_overflow_fails_without_losing_previous_accounting() {
    for vertex in [false, true] {
        let sse = format!(
            "{}data: {{\"usageMetadata\":{{\"candidatesTokenCount\":4294967295,\"thoughtsTokenCount\":1}}}}\n\n",
            google_usage_frame(None, 11, 7, 99)
        );
        let failure = google_public_result(&sse, vertex, false, Default::default())
            .await
            .unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::StreamError(_)));
        assert_eq!(failure.message().usage.total_tokens, Some(99));
        assert_eq!(failure.message().usage.output_tokens, 7);
    }
}

#[tokio::test]
async fn google_cancel_retains_usage_that_emitted_no_progress_event() {
    use std::sync::Arc;
    for vertex in [false, true] {
        let entered = Arc::new(tokio::sync::Notify::new());
        let mut request = make_request(CancellationToken::new());
        let cancel = request.cancel.clone();
        request.callbacks = opi_ai::callbacks::RequestCallbacks::new().with_async_on_raw({
            let entered = entered.clone();
            move |value| {
                let entered = entered.clone();
                async move {
                    if value.get("gate").is_some() {
                        entered.notify_one();
                        std::future::pending::<()>().await;
                    }
                    Ok(())
                }
            }
        });
        let sse = format!(
            "{}data: {{\"gate\":true}}\n\n",
            google_usage_frame(None, 11, 7, 99)
        );
        let call =
            tokio::spawn(async move { google_public_request(&sse, vertex, false, request).await });
        entered.notified().await;
        cancel.cancel();
        let failure = call.await.unwrap().unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::Cancelled));
        assert_eq!(failure.message().usage.total_tokens, Some(99));
        assert_eq!(failure.message().usage.input_tokens, 11);
        assert!(failure.message().duration_ms.is_some());
    }
}

#[tokio::test(start_paused = true)]
async fn google_models_duration_excludes_auth_wait() {
    use std::sync::Arc;
    struct GatedAuth {
        entered: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
        vertex: bool,
    }
    impl opi_ai::AuthResolver for GatedAuth {
        fn resolve(
            &self,
        ) -> opi_ai::credential::BoxAuthFuture<'_, Result<opi_ai::ResolvedAuth, ProviderError>>
        {
            Box::pin(async move {
                self.entered.notify_one();
                self.release.notified().await;
                Ok(if self.vertex {
                    opi_ai::test_support::resolved_bearer_auth()
                } else {
                    opi_ai::test_support::resolved_auth()
                })
            })
        }
    }
    for vertex in [false, true] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(text_sse_fixture())
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let auth = Arc::new(GatedAuth {
            entered: entered.clone(),
            release: release.clone(),
            vertex,
        });
        let wire = if vertex {
            opi_ai::WireApi::GoogleVertex
        } else {
            opi_ai::WireApi::GoogleGenerativeAi
        };
        let model = opi_ai::ModelInfo::new(
            "gemini-2.5-flash",
            "fixture",
            wire,
            opi_ai::ModelCapabilities::new(10000, 1000).with_streaming(true),
        );
        let record =
            opi_ai::ChatModelRecord::new(if vertex { "vertex" } else { "gemini" }, model.clone());
        let mut models = opi_ai::Models::new();
        let request = make_request(CancellationToken::new());
        let call = if vertex {
            let api = opi_ai::vertex::GoogleVertexChatApi::new(
                Arc::new(opi_ai::vertex::VertexProvider::new(
                    "p1".into(),
                    "us-central1".into(),
                    Some(server.uri()),
                )),
                model,
            )
            .unwrap();
            let handle = models
                .register_chat_api(Arc::new(api), auth, opi_ai::AuthProvenanceSource::Static)
                .unwrap();
            models.stream(
                record,
                request,
                handle.options(opi_ai::vertex::GoogleVertexOptions::default()),
            )
        } else {
            let api = opi_ai::gemini::GoogleGenerativeAiChatApi::new(
                Arc::new(GeminiProvider::new(Some(server.uri()))),
                model,
            )
            .unwrap();
            let handle = models
                .register_chat_api(Arc::new(api), auth, opi_ai::AuthProvenanceSource::Static)
                .unwrap();
            models.stream(
                record,
                request,
                handle.options(opi_ai::gemini::GoogleGenerativeAiOptions::default()),
            )
        };
        tokio::time::advance(std::time::Duration::from_secs(60)).await;
        assert!(server.received_requests().await.unwrap().is_empty());
        let call = tokio::spawn(call.result());
        entered.notified().await;
        tokio::time::advance(std::time::Duration::from_secs(60)).await;
        release.notify_one();
        let message = call.await.unwrap().unwrap();
        assert_eq!(message.duration_ms, Some(0));
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn google_direct_terminal_absorbs_late_cancel() {
    for vertex in [false, true] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(text_sse_fixture())
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let request = make_request(CancellationToken::new());
        let cancel = request.cancel.clone();
        let mut stream = if vertex {
            opi_ai::vertex::VertexProvider::new(
                "p1".into(),
                "us-central1".into(),
                Some(server.uri()),
            )
            .stream_prepared(request, opi_ai::test_support::resolved_bearer_auth())
        } else {
            GeminiProvider::new(Some(server.uri()))
                .stream_prepared(request, opi_ai::test_support::resolved_auth())
        };
        loop {
            let event = stream.next().await.expect("terminal").unwrap();
            if let AssistantStreamEvent::Done { message, .. } = event {
                assert!(message.duration_ms.is_some());
                break;
            }
        }
        cancel.cancel();
        assert!(stream.next().await.is_none());
        assert!(stream.next().await.is_none());
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn google_activated_http_failures_retain_identity_and_duration() {
    for vertex in [false, true] {
        for status in [403, 429, 500] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(ResponseTemplate::new(status))
                .expect(1)
                .mount(&server)
                .await;
            let stream =
                google_direct_stream(server.uri(), vertex, make_request(CancellationToken::new()));
            let failure = opi_ai::stream::ChatStream::new(stream)
                .result()
                .await
                .unwrap_err();
            assert_google_failed_response(failure.message(), vertex);
            match status {
                403 => assert!(matches!(failure.cause(), ProviderError::AuthFailed(_))),
                429 => assert!(matches!(failure.cause(), ProviderError::RateLimited { .. })),
                _ => assert!(matches!(failure.cause(), ProviderError::ProviderSide(_))),
            }
            assert_eq!(server.received_requests().await.unwrap().len(), 1);
        }
    }
}

#[tokio::test]
async fn google_activated_connect_failure_retains_identity_and_duration() {
    for vertex in [false, true] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        let stream = google_direct_stream(base, vertex, make_request(CancellationToken::new()));
        let failure = opi_ai::stream::ChatStream::new(stream)
            .result()
            .await
            .unwrap_err();
        assert_google_failed_response(failure.message(), vertex);
        assert!(matches!(failure.cause(), ProviderError::Network(_)));
    }
}

#[tokio::test]
async fn google_preflight_failure_before_owner_remains_untimed() {
    for vertex in [false, true] {
        let server = MockServer::start().await;
        let mut request = make_request(CancellationToken::new());
        request.callbacks = opi_ai::callbacks::RequestCallbacks::new()
            .with_on_response(std::sync::Arc::new(|_, _| Ok(())));
        let stream = google_direct_stream(server.uri(), vertex, request);
        let failure = opi_ai::stream::ChatStream::new(stream)
            .result()
            .await
            .unwrap_err();
        assert!(matches!(
            failure.cause(),
            ProviderError::UnsupportedCapability(_)
        ));
        assert_eq!(failure.message().duration_ms, None);
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}
