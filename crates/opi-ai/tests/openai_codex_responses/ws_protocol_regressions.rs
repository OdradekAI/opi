fn request() -> Request {
    let mut request = super::request();
    request.transport = opi_ai::provider::TransportPreference::Auto;
    request
}

use super::ws_path::{header, read_frame, read_head, response_101, write_frame};
use super::*;
use opi_ai::http::{HttpClientBuilder, RedirectPolicy};
use opi_ai::{ChatModelRecord, Models, SimpleOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Bearer;
impl opi_ai::auth::AuthResolver for Bearer {
    fn resolve(
        &self,
    ) -> opi_ai::credential::BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async { Ok(auth("fake", None, Some("account".into()))) })
    }
}

fn window8_request_text() -> String {
    let mut state = 0x35a9_17cdu32;
    let block: String = (0..512)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            char::from(b'a' + (state >> 28) as u8)
        })
        .collect();
    block.repeat(16)
}

#[tokio::test]
async fn default_http_client_auto_uses_the_real_websocket_path() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let peer = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let head = read_head(&mut socket);
        if head[0].starts_with("GET ") {
            socket.write_all(&response_101(&head, &[])).unwrap();
            read_frame(&mut socket).unwrap();
            write_frame(&mut socket, 1, super::ws_path::completed_event().as_bytes());
            assert_eq!(read_small_frame(&mut socket).unwrap().0, 8);
            write_frame(&mut socket, 8, &[]);
        } else {
            let length = header(&head, "content-length").unwrap().parse().unwrap();
            let mut body = vec![0; length];
            socket.read_exact(&mut body).unwrap();
            socket
                .write_all(&super::ws_path::sse_response(&codex_completed_sse(), 200))
                .unwrap();
        }
        head[0].clone()
    });
    let mut models = Models::new();
    models
        .register_route(
            Box::new(OpenAiCodexResponsesProvider::new(
                Some(format!("http://127.0.0.1:{port}")),
                vec![model(None)],
                Arc::new(HttpClient::new()),
            )),
            Arc::new(Bearer),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let mut request = request();
    request.cache_retention = CacheRetention::Disabled;
    request.session_id = None;
    models
        .complete_simple(
            ChatModelRecord::new("openai-codex", model(None)),
            request,
            Default::default(),
        )
        .await
        .unwrap();
    assert!(
        peer.join().unwrap().starts_with("GET "),
        "Auto must upgrade with the default HttpClient"
    );
}

#[tokio::test]
async fn default_http_client_upgrade_rejects_redirect_without_a_destination_send() {
    let destination = TcpListener::bind("127.0.0.1:0").unwrap();
    destination.set_nonblocking(true).unwrap();
    let target = destination.local_addr().unwrap();
    let source = TcpListener::bind("127.0.0.1:0").unwrap();
    source.set_nonblocking(true).unwrap();
    let port = source.local_addr().unwrap().port();
    let peer = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
        while std::time::Instant::now() < deadline {
            if let Ok((mut socket, _)) = source.accept() {
                socket.set_nonblocking(false).unwrap();
                let head = read_head(&mut socket);
                assert!(head[0].starts_with("GET "));
                write!(socket, "HTTP/1.1 302 Found\r\nLocation: http://{target}/redirected\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        false
    });
    let result = opi_ai::ws::upgrade(
        &HttpClient::new(),
        opi_ai::ws::WsUpgradeRequest::new(
            url::Url::parse(&format!("ws://127.0.0.1:{port}/socket")).unwrap(),
        ),
    )
    .await;
    let sent = peer.join().unwrap();
    assert!(
        matches!(destination.accept(), Err(ref error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "redirect target must receive no send"
    );
    assert!(
        sent,
        "default client must reach the original Upgrade endpoint"
    );
    assert!(
        matches!(result, Err(opi_ai::ws::WsUpgradeError::Status(302))),
        "3xx must be the typed terminal Upgrade status"
    );
}

#[tokio::test]
async fn pooled_connection_does_not_cross_the_selected_model_endpoint() {
    fn endpoint(id: &'static str) -> (u16, std::thread::JoinHandle<usize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let thread = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            let mut accepted = None;
            while std::time::Instant::now() < deadline {
                if let Ok((socket, _)) = listener.accept() {
                    accepted = Some(socket);
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            let Some(mut socket) = accepted else {
                return 0;
            };
            socket.set_nonblocking(false).unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let head = read_head(&mut socket);
            socket.write_all(&response_101(&head, &[])).unwrap();
            let mut creates = 0;
            while let Ok((opcode, _)) = read_frame(&mut socket) {
                if opcode == 8 {
                    write_frame(&mut socket, 8, &[]);
                    break;
                }
                assert_eq!(opcode, 1);
                creates += 1;
                let completed = serde_json::json!({"type":"response.completed", "response":{"id":id,"model":"gpt-5.4","output":[]}}).to_string();
                write_frame(&mut socket, 1, completed.as_bytes());
            }
            creates
        });
        (port, thread)
    }
    let (port_a, peer_a) = endpoint("endpoint-a");
    let (port_b, peer_b) = endpoint("endpoint-b");
    let mut a = model(None);
    a.base_url = Some(format!("http://127.0.0.1:{port_a}"));
    let mut b = model(None);
    b.id = "gpt-5.4-other".into();
    b.base_url = Some(format!("http://127.0.0.1:{port_b}"));
    let mut models = Models::new();
    models
        .register_route(
            Box::new(OpenAiCodexResponsesProvider::new(
                None,
                vec![a.clone(), b.clone()],
                Arc::new(
                    HttpClientBuilder::new()
                        .redirects(RedirectPolicy::None)
                        .build()
                        .unwrap(),
                ),
            )),
            Arc::new(Bearer),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let mut request = request();
    request.session_id = Some("same-session-two-endpoints".into());
    let first = models
        .complete_simple(
            ChatModelRecord::new("openai-codex", a),
            request.clone(),
            Default::default(),
        )
        .await
        .unwrap();
    let second = models
        .complete_simple(
            ChatModelRecord::new("openai-codex", b),
            request,
            Default::default(),
        )
        .await
        .unwrap();
    models.cleanup_session_resources(None).await.unwrap();
    let count_a = peer_a.join().unwrap();
    let count_b = peer_b.join().unwrap();
    assert_eq!(first.response_id.as_deref(), Some("endpoint-a"));
    assert_eq!(second.response_id.as_deref(), Some("endpoint-b"));
    assert_eq!(
        (count_a, count_b),
        (1, 1),
        "each frozen endpoint must receive its own call"
    );
}

#[tokio::test]
async fn low_level_sse_attempt_drop_cancels_only_its_internal_token() {
    let mut request = request();
    request.transport = opi_ai::provider::TransportPreference::Sse;
    let started = Arc::new(tokio::sync::Notify::new());
    let callback_started = started.clone();
    request.callbacks =
        opi_ai::callbacks::RequestCallbacks::new().with_async_on_payload(move |_| {
            let started = callback_started.clone();
            async move {
                started.notify_one();
                std::future::pending::<
                    Result<Option<serde_json::Value>, opi_ai::callbacks::CallbackError>,
                >()
                .await
            }
        });
    let parent = request.cancel.clone();
    let sibling = parent.child_token();
    let provider = OpenAiCodexResponsesProvider::new(
        Some("http://127.0.0.1:9".into()),
        vec![model(None)],
        Arc::new(HttpClient::new()),
    );
    let stream = provider.stream_attempt(
        request,
        auth("fake", None, Some("account".into())),
        opi_ai::provider::AttemptContext {
            invocation_id: "direct-attempt".into(),
            attempt: 1,
            max_attempts: 1,
            payload_variant: opi_ai::provider::PayloadVariant::Ordinary,
        },
    );
    started.notified().await;
    drop(stream);
    let _ = tokio::time::timeout(std::time::Duration::from_millis(40), parent.cancelled()).await;
    assert!(
        !parent.is_cancelled(),
        "receiver drop must not cancel the caller parent"
    );
    assert!(
        !sibling.is_cancelled(),
        "a sibling sharing that parent must survive"
    );
}

enum Script {
    MaskedFrame,
    InvalidPmd,
    SilentProviderEvent,
    IgnoredProviderEvent(bool),
    FailedResponse,
    MissingPreviousAfterDelta,
    TypelessEvent,
    PreEventEof(Arc<AtomicUsize>),
    ZeroIdleTimeout,
    OutboundPmd,
    AsyncRaw(Arc<AtomicUsize>),
    Ping,
    Binary,
}

fn read_small_frame(stream: &mut TcpStream) -> Result<(u8, Vec<u8>), String> {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .unwrap();
    let mut head = [0; 2];
    stream.read_exact(&mut head).map_err(|e| e.to_string())?;
    if head[1] & 0x80 == 0 || head[1] & 0x7f > 125 {
        return Err("expected a small masked control frame".into());
    }
    let mut mask = [0; 4];
    stream.read_exact(&mut mask).map_err(|e| e.to_string())?;
    let mut payload = vec![0; (head[1] & 0x7f) as usize];
    stream.read_exact(&mut payload).map_err(|e| e.to_string())?;
    for (index, byte) in payload.iter_mut().enumerate() {
        *byte ^= mask[index % 4];
    }
    Ok((head[0] & 0x0f, payload))
}

async fn run(script: Script) -> (Result<AssistantMessage, opi_ai::stream::CallFailure>, usize) {
    let outbound_pmd = matches!(script, Script::OutboundPmd);
    let fallback_probe = matches!(script, Script::PreEventEof(_));
    let zero_idle = matches!(script, Script::ZeroIdleTimeout);
    let raw_counter = match &script {
        Script::AsyncRaw(count) | Script::PreEventEof(count) => Some(count.clone()),
        _ => None,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let posts = Arc::new(AtomicUsize::new(0));
    let recorded = posts.clone();
    let thread = std::thread::spawn(move || -> Result<(), String> {
        let (mut socket, _) = listener.accept().map_err(|e| e.to_string())?;
        socket.set_nodelay(true).unwrap();
        let head = read_head(&mut socket);
        let extra = if matches!(script, Script::InvalidPmd) {
            vec![("Sec-WebSocket-Extensions", "permessage-deflate")]
        } else if matches!(script, Script::OutboundPmd) {
            vec![(
                "Sec-WebSocket-Extensions",
                "permessage-deflate; client_max_window_bits=8; server_max_window_bits=8; client_no_context_takeover; server_no_context_takeover",
            )]
        } else {
            vec![]
        };
        socket
            .write_all(&response_101(&head, &extra))
            .map_err(|e| e.to_string())?;
        let (opcode, payload, compressed) =
            super::ws_path::read_frame_with_compression(&mut socket)?;
        assert_eq!(opcode, 1);
        if matches!(script, Script::OutboundPmd) {
            assert!(
                compressed,
                "negotiated PMD must encode an eligible response.create"
            );
            let (plain, maximum_distance) =
                super::deflate_audit::decode_pmd(&payload, &mut Vec::new(), 8).unwrap();
            assert!(maximum_distance <= 256);
            let body: serde_json::Value = serde_json::from_slice(&plain).unwrap();
            assert_eq!(body["input"][0]["content"], window8_request_text());
        }
        match script {
            Script::MaskedFrame => socket.write_all(&[0x81, 0x80, 0, 0, 0, 0]).unwrap(),
            Script::InvalidPmd => socket.write_all(&[0xc1, 1, 0xff]).unwrap(),
            Script::SilentProviderEvent => {
                write_frame(&mut socket, 1, br#"{"type":"response.output_item.added","output_index":0,"item":{"type":"message","id":"message-1","content":[]}}"#);
            }
            Script::IgnoredProviderEvent(reconnect) => {
                let fixture = connection_limit_fixture();
                write_frame(&mut socket, 1, fixture["progress"].to_string().as_bytes());
                if reconnect {
                    write_frame(
                        &mut socket,
                        1,
                        fixture["error"].to_string().as_bytes(),
                    );
                }
            }
            Script::FailedResponse => write_frame(
                &mut socket,
                1,
                br#"{"type":"response.failed","response":{"status":"failed","error":{"message":"Upstream HTTP/2 stream failed. PRIVATE_CANARY"}}}"#,
            ),
            Script::MissingPreviousAfterDelta => {
                for event in [
                    r#"{"type":"response.output_item.added","item":{"type":"message","id":"msg_1","content":[]}}"#,
                    r#"{"type":"response.output_text.delta","delta":"partial"}"#,
                    r#"{"type":"error","error":{"code":"previous_response_not_found","message":"Upstream HTTP/2 stream failed. PRIVATE_CANARY"}}"#,
                ] {
                    write_frame(&mut socket, 1, event.as_bytes());
                }
            }
            Script::ZeroIdleTimeout => {
                std::thread::sleep(std::time::Duration::from_millis(50));
                write_frame(&mut socket, 1, super::ws_path::completed_event().as_bytes());
                if let Ok((8, _)) = read_small_frame(&mut socket) {
                    write_frame(&mut socket, 8, &[]);
                }
            }
            Script::PreEventEof(_) => {}
            Script::TypelessEvent => write_frame(&mut socket, 1, br#"{"unrelated":true}"#),
            Script::OutboundPmd => {
                write_frame(&mut socket, 1, super::ws_path::completed_event().as_bytes());
                assert_eq!(read_small_frame(&mut socket)?.0, 8);
                write_frame(&mut socket, 8, &[]);
            }
            Script::Ping => {
                write_frame(&mut socket, 9, b"probe");
                let (opcode, payload) = read_small_frame(&mut socket)?;
                assert_eq!((opcode, payload.as_slice()), (10, b"probe".as_slice()));
                write_frame(&mut socket, 1, super::ws_path::completed_event().as_bytes());
                let (opcode, _) = read_small_frame(&mut socket)?;
                assert_eq!(opcode, 8);
                write_frame(&mut socket, 8, &[]);
            }
            Script::Binary | Script::AsyncRaw(_) => {
                write_frame(&mut socket, 2, super::ws_path::completed_event().as_bytes());
                if let Ok((8, _)) = read_small_frame(&mut socket) {
                    write_frame(&mut socket, 8, &[]);
                }
            }
        }
        let _ = socket.shutdown(std::net::Shutdown::Both);
        listener.set_nonblocking(true).unwrap();
        let until = std::time::Instant::now() + std::time::Duration::from_millis(200);
        while std::time::Instant::now() < until {
            match listener.accept() {
                Ok((mut socket, _)) => {
                    socket.set_nonblocking(false).unwrap();
                    let head = read_head(&mut socket);
                    assert!(head[0].starts_with("POST "));
                    let length = header(&head, "content-length").unwrap().parse().unwrap();
                    let mut bytes = vec![0; length];
                    socket.read_exact(&mut bytes).unwrap();
                    recorded.fetch_add(1, Ordering::SeqCst);
                    socket
                        .write_all(&super::ws_path::sse_response(&codex_completed_sse(), 200))
                        .unwrap();
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(2))
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(())
    });
    let mut models = Models::new();
    models
        .register_route(
            Box::new(OpenAiCodexResponsesProvider::new(
                Some(format!("http://127.0.0.1:{port}")),
                vec![model(None)],
                Arc::new(
                    HttpClientBuilder::new()
                        .redirects(RedirectPolicy::None)
                        .build()
                        .unwrap(),
                ),
            )),
            Arc::new(Bearer),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let mut request = request();
    request.session_id = None;
    request.cache_retention = CacheRetention::Disabled;
    request.timeout = Some(if zero_idle {
        std::time::Duration::ZERO
    } else {
        std::time::Duration::from_millis(100)
    });
    if let Some(counter) = raw_counter {
        request.callbacks =
            opi_ai::callbacks::RequestCallbacks::new().with_async_on_raw(move |_| {
                let counter = counter.clone();
                async move {
                    tokio::task::yield_now().await;
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            });
    }
    if fallback_probe {
        request.context.append_system_change(opi_ai::transcript::SystemMessage {
            tools_added: vec![ToolDef {
                name: "grammar-tool".into(), description: "local".into(),
                input_schema: serde_json::json!({"type":"object","properties":{"input":{"type":"string"}},"required":["input"]}),
                constrained_sampling: Some(opi_ai::constrained_sampling::ConstrainedSampling::Grammar {
                    openai_lark: Some("start: WORD".into()), openai_regex: None,
                }),
            }],
            ..Default::default()
        }).unwrap();
    }
    if outbound_pmd {
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: window8_request_text(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .unwrap();
    }
    let result = models
        .complete_simple(
            ChatModelRecord::new("openai-codex", model(None)),
            request,
            SimpleOptions::default(),
        )
        .await;
    thread
        .join()
        .expect("script thread")
        .expect("script completes");
    (result, posts.load(Ordering::SeqCst))
}

#[tokio::test]
async fn masked_server_frame_is_terminal_without_sse_replay() {
    let (result, posts) = run(Script::MaskedFrame).await;
    assert_eq!(
        posts, 0,
        "a framing protocol failure must never resend over SSE"
    );
    assert!(result.is_err());
}

#[tokio::test]
async fn invalid_pmd_is_terminal_without_sse_replay() {
    let (result, posts) = run(Script::InvalidPmd).await;
    assert_eq!(
        posts, 0,
        "a compression protocol failure must never resend over SSE"
    );
    assert!(result.is_err());
}

#[tokio::test]
async fn ping_receives_a_matching_masked_pong_before_generation_continues() {
    let (result, posts) = run(Script::Ping).await;
    assert_eq!(posts, 0);
    assert_eq!(
        result
            .expect("valid control flow succeeds")
            .response_id
            .as_deref(),
        Some("response-1")
    );
}

#[tokio::test]
async fn binary_provider_json_is_projected_like_text() {
    let (result, posts) = run(Script::Binary).await;
    assert_eq!(posts, 0);
    assert_eq!(
        result
            .expect("valid binary provider event succeeds")
            .response_id
            .as_deref(),
        Some("response-1")
    );
}

#[tokio::test]
async fn asynchronous_raw_observer_is_awaited_once_before_the_terminal() {
    let observed = Arc::new(AtomicUsize::new(0));
    let (result, posts) = run(Script::AsyncRaw(observed.clone())).await;
    assert_eq!(posts, 0);
    assert!(
        result.is_ok(),
        "an asynchronous functional raw hook is supported on WS"
    );
    assert_eq!(observed.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn valid_provider_event_seals_fallback_before_normalized_output() {
    let (result, posts) = run(Script::SilentProviderEvent).await;
    assert_eq!(
        posts, 0,
        "a valid provider event closes fallback even when it emits no public content"
    );
    assert!(result.is_err());
}

#[tokio::test]
async fn ignored_provider_event_seals_fallback() {
    let (result, posts) = run(Script::IgnoredProviderEvent(false)).await;
    assert_eq!(
        posts, 0,
        "in_progress is a provider event even without mapped content"
    );
    assert!(result.is_err());
}

#[tokio::test]
async fn ignored_provider_event_seals_connection_limit_reconnect() {
    use opi_ai::retry::{WholeCallRetryClassification, classify_whole_call_retry};

    let (result, posts) = run(Script::IgnoredProviderEvent(true)).await;
    assert_eq!(posts, 0);
    let failure = result.unwrap_err();
    assert!(matches!(failure.cause(), ProviderError::StreamError(_)));
    assert!(failure.message().content.is_empty());
    assert_eq!(
        classify_whole_call_retry(failure.message()),
        WholeCallRetryClassification::Transient
    );
    assert!(!format!("{failure:?}").contains("PRIVATE_CANARY"));
}

#[tokio::test]
async fn response_failed_on_websocket_carries_the_safe_diagnostic_without_fallback() {
    use opi_ai::retry::{WholeCallRetryClassification, classify_whole_call_retry};
    let (result, posts) = run(Script::FailedResponse).await;
    let failure = result.unwrap_err();
    assert_eq!(posts, 0);
    assert!(matches!(failure.cause(), ProviderError::StreamError(_)));
    assert_eq!(failure.message().raw_stop_reason.as_deref(), Some("failed"));
    assert_eq!(
        classify_whole_call_retry(failure.message()),
        WholeCallRetryClassification::Transient
    );
    assert!(!format!("{failure:?}").contains("PRIVATE_CANARY"));
}

#[tokio::test]
async fn missing_previous_after_a_public_delta_does_not_replay() {
    use opi_ai::retry::{WholeCallRetryClassification, classify_whole_call_retry};
    let (result, posts) = run(Script::MissingPreviousAfterDelta).await;
    let failure = result.unwrap_err();
    assert_eq!(posts, 0);
    assert!(matches!(failure.cause(), ProviderError::StreamError(_)));
    assert!(matches!(failure.message().content.as_slice(),
        [AssistantContent::Text { text, .. }] if text == "partial"));
    assert_eq!(
        classify_whole_call_retry(failure.message()),
        WholeCallRetryClassification::Transient
    );
    assert!(!format!("{failure:?}").contains("PRIVATE_CANARY"));
}

#[tokio::test]
async fn typeless_json_keeps_pre_event_fallback() {
    let (result, posts) = run(Script::TypelessEvent).await;
    assert_eq!(posts, 1);
    assert!(result.is_ok());
}

#[tokio::test]
async fn pre_event_eof_successful_sse_retains_transport_and_sampling_diagnostics() {
    let raw = Arc::new(AtomicUsize::new(0));
    let (result, posts) = run(Script::PreEventEof(raw.clone())).await;
    let message = result.expect("pre-event EOF may fall back once");
    assert_eq!(posts, 1);
    assert_eq!(
        raw.load(Ordering::SeqCst),
        1,
        "only the real SSE event invokes raw"
    );
    assert!(
        message
            .diagnostics
            .iter()
            .any(|d| d.kind == "constrained_sampling_fallback")
    );
    let diagnostic = message
        .diagnostics
        .iter()
        .find(|d| d.kind == "codex_websocket_fallback")
        .expect("successful SSE must retain the WS failure reason");
    assert_eq!(diagnostic.reason, "transport_eof");
    assert!(
        diagnostic.detail.is_none(),
        "no raw errors, endpoints or credentials"
    );
}

#[tokio::test]
async fn zero_idle_timeout_waits_for_pending_websocket_event() {
    let (result, posts) = run(Script::ZeroIdleTimeout).await;
    assert_eq!(posts, 0);
    assert!(
        result.is_ok(),
        "zero disables the Codex WS pending-read timer: {result:?}"
    );
}

#[tokio::test]
async fn negotiated_pmd_encodes_large_request_with_actual_window8() {
    let (result, posts) = run(Script::OutboundPmd).await;
    assert_eq!(posts, 0);
    assert!(result.is_ok());
}

#[tokio::test]
async fn successful_upgrade_seals_advanced_acquisition_even_if_fallback_fails() {
    use super::ws_path::{CodexPeer, PeerCapture};
    use std::sync::Mutex;

    for websocket_sends in [1, 2] {
        let handler =
            move |mut socket: TcpStream, head: Vec<String>, capture: &Arc<Mutex<PeerCapture>>| {
                if websocket_sends == 2 {
                    return super::ws_branches::reject_connection_limit(socket, head, capture);
                }
                socket.write_all(&response_101(&head, &[])).unwrap();
                assert_eq!(read_frame(&mut socket)?.0, 1);
                Ok(())
            };
        // Close the SSE connection after consuming the POST and before
        // sending headers, which would usually admit an acquisition retry.
        let peer = CodexPeer::spawn_many_with_response(
            vec![handler; websocket_sends],
            websocket_sends + 1,
            Vec::new(),
        );
        let mut models = Models::new();
        models
            .register_route(
                Box::new(OpenAiCodexResponsesProvider::new(
                    Some(peer.base_url()),
                    vec![model(None)],
                    Arc::new(
                        HttpClientBuilder::new()
                            .redirects(RedirectPolicy::None)
                            .build()
                            .unwrap(),
                    ),
                )),
                Arc::new(Bearer),
                opi_ai::AuthProvenanceSource::Static,
                Default::default(),
            )
            .unwrap();
        let prepared = models
            .prepare_call_with_policy(
                "openai-codex:gpt-5.4",
                request(),
                opi_ai::retry::AcquisitionPolicy {
                    max_retries: 1,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let mut attempt = prepared.start_attempt().unwrap();
        assert!(attempt.next().await.unwrap().is_err());
        assert!(attempt.next().await.is_none());
        assert!(
            prepared.retry_admission().is_none(),
            "a successful WS acquisition cannot reset into another generic attempt"
        );
        assert_eq!(
            prepared.acquisition_seal(),
            Some(opi_ai::retry::AcquisitionSeal::PostHandoff)
        );
        let capture = peer.finish().expect("peer script");
        let capture = capture.lock().unwrap();
        assert_eq!(capture.heads.len(), websocket_sends + 1);
        assert_eq!(capture.posts.len(), 1);
    }
}

#[tokio::test]
async fn overlapping_session_call_uses_a_temporary_connection() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let temporary_closed = Arc::new(AtomicUsize::new(0));
    let observed = temporary_closed.clone();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let (second_tx, second_rx) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || -> Result<(), String> {
        let (mut first, _) = listener.accept().map_err(|e| e.to_string())?;
        let first_peer = std::thread::spawn(move || -> Result<(), String> {
            let head = read_head(&mut first);
            first.write_all(&response_101(&head, &[])).unwrap();
            assert_eq!(read_frame(&mut first)?.0, 1);
            ready_tx.send(()).unwrap();
            second_rx.recv().unwrap();
            write_frame(&mut first, 1, super::ws_path::completed_event().as_bytes());
            assert_eq!(read_small_frame(&mut first)?.0, 8);
            write_frame(&mut first, 8, &[]);
            let _ = first.shutdown(std::net::Shutdown::Both);
            Ok(())
        });
        let (mut second, _) = listener.accept().map_err(|e| e.to_string())?;
        let head = read_head(&mut second);
        second.write_all(&response_101(&head, &[])).unwrap();
        assert_eq!(read_frame(&mut second)?.0, 1);
        write_frame(&mut second, 1, super::ws_path::completed_event().as_bytes());
        if let Ok((8, _)) = read_small_frame(&mut second) {
            observed.fetch_add(1, Ordering::SeqCst);
            write_frame(&mut second, 8, &[]);
        }
        let _ = second.shutdown(std::net::Shutdown::Both);
        second_tx.send(()).unwrap();
        first_peer.join().unwrap()
    });
    let mut models = Models::new();
    models
        .register_route(
            Box::new(OpenAiCodexResponsesProvider::new(
                Some(format!("http://127.0.0.1:{port}")),
                vec![model(None)],
                Arc::new(
                    HttpClientBuilder::new()
                        .redirects(RedirectPolicy::None)
                        .build()
                        .unwrap(),
                ),
            )),
            Arc::new(Bearer),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    async fn drain(mut call: opi_ai::models::ChatCall) {
        let mut terminal = false;
        while let Some(item) = call.next().await {
            terminal |= matches!(
                item.expect("valid generation"),
                AssistantStreamEvent::Done { .. }
            );
        }
        assert!(terminal);
    }
    let record = ChatModelRecord::new("openai-codex", model(None));
    let first = models.stream_simple(record.clone(), request(), SimpleOptions::default());
    let second = async {
        ready_rx.await.unwrap();
        drain(models.stream_simple(record, request(), SimpleOptions::default())).await;
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(drain(first), second);
    })
    .await
    .unwrap();
    models.cleanup_session_resources(None).await.unwrap();
    peer.join().expect("peer thread").expect("peer script");
    assert_eq!(
        temporary_closed.load(Ordering::SeqCst),
        1,
        "a busy-overflow connection is temporary and must complete Close"
    );
}
