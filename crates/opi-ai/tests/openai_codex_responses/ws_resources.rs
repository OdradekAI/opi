//! Task 20.17: instance resource generation, pool cleanup and awaited
//! local release through the public `Models::cleanup_session_resources`
//! entry (P20-RESOURCES owner consumed by the Codex provider).

fn request() -> Request {
    let mut request = super::request();
    request.transport = opi_ai::provider::TransportPreference::Auto;
    request
}

use super::ws_path::{header, read_frame, read_head, response_101, write_frame};
use super::*;
use opi_ai::codex_resources::CleanupError;
use opi_ai::http::{HttpClientBuilder, RedirectPolicy};
use opi_ai::provider::TransportPreference;
use opi_ai::{ChatModelRecord, Models, SimpleOptions};
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

const DELTA: &str = r#"{"type":"response.output_text.delta","delta":"Hello"}"#;

struct StaticBearer {
    secret: &'static str,
    account: &'static str,
}

impl opi_ai::auth::AuthResolver for StaticBearer {
    fn resolve(
        &self,
    ) -> opi_ai::credential::BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async move {
            Ok(ResolvedAuth {
                env: Default::default(),
                scheme: AuthScheme::Bearer,
                secret: SecretString::from(self.secret),
                base_url: None,
                account_id: Some(self.account.to_owned()),
                provenance: opi_ai::AuthProvenance::default(),
            })
        })
    }
}

fn provider(base_url: String) -> OpenAiCodexResponsesProvider {
    OpenAiCodexResponsesProvider::new(
        Some(base_url),
        vec![model(None)],
        Arc::new(
            HttpClientBuilder::new()
                .redirects(RedirectPolicy::None)
                .build()
                .unwrap(),
        ),
    )
}

pub(super) fn models_with(base_url: String) -> Models {
    let mut models = Models::new();
    models
        .register_route(
            Box::new(provider(base_url)),
            Arc::new(StaticBearer {
                secret: "ws-secret",
                account: "acct-1",
            }),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .expect("register codex route");
    models
}

fn session_request(session: &str) -> Request {
    let mut request = request();
    request.session_id = Some(session.into());
    request
}

fn sessionless_request() -> Request {
    let mut request = request();
    request.cache_retention = CacheRetention::Disabled;
    request.session_id = None;
    request
}

fn is_cancelled(failure: &opi_ai::stream::CallFailure) -> bool {
    matches!(failure.cause(), ProviderError::Cancelled)
}

/// Construct the independently executing call and wait for its first
/// event within the fixture window. Cutoff tests also observe peer progress.
async fn start_call(
    models: &Models,
    request: Request,
) -> std::pin::Pin<Box<opi_ai::models::ChatCall>> {
    let record = ChatModelRecord::new("openai-codex", model(None));
    let call = models.stream_simple(
        record,
        request,
        SimpleOptions {
            tool_choice: None,
            temperature: None,
            max_tokens: None,
            timeout: None,
            stop_sequences: vec![],
        },
    );
    let mut call = Box::pin(call);
    // Consume the first event if it arrives within the fixture window.
    // Execution continues independently when this borrowed wait times out.
    let _ = tokio::time::timeout(std::time::Duration::from_millis(400), call.next()).await;
    call
}

/// Drain a started call to its single terminal outcome.
async fn finish_call(
    mut call: std::pin::Pin<Box<opi_ai::models::ChatCall>>,
) -> Result<opi_ai::message::AssistantMessage, opi_ai::stream::CallFailure> {
    use futures_util::StreamExt;
    let mut terminal = None;
    while let Some(item) = call.next().await {
        match item {
            Ok(AssistantStreamEvent::Done { message, .. }) => terminal = Some(Ok(message)),
            Ok(_) => {}
            Err(failure) => {
                terminal = Some(Err(failure));
                break;
            }
        }
    }
    terminal.expect("call produced a terminal")
}

/// A scripted multi-connection peer: each accepted connection consumes
/// the next script entry; `slow_post` replaces the fixed SSE answer with
/// a partial write that holds the SSE leg open.
struct ScriptPeer {
    port: u16,
    heads: Arc<Mutex<Vec<Vec<String>>>>,
    join: Option<std::thread::JoinHandle<()>>,
}

type ConnScript = Box<dyn Fn(&mut TcpStream, &[String]) -> Result<(), String> + Send>;

impl ScriptPeer {
    fn spawn(scripts: Vec<ConnScript>, slow_post: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let heads = Arc::new(Mutex::new(Vec::new()));
        let mut scripts: std::collections::VecDeque<ConnScript> = scripts.into();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(6);
        let head_store = heads.clone();
        let join = std::thread::spawn(move || {
            let mut accepted = 0;
            while std::time::Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        accepted += 1;
                        stream.set_nodelay(true).unwrap();
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(std::time::Duration::from_secs(15)))
                            .unwrap();
                        let lines = read_head(&mut stream);
                        if header(&lines, "upgrade").is_some() {
                            head_store.lock().unwrap().push(lines.clone());
                            let script = scripts
                                .pop_front()
                                .expect("websocket connection without script");
                            script(&mut stream, &lines).expect("ws script");
                        } else {
                            let length = header(&lines, "content-length")
                                .and_then(|value| value.parse::<usize>().ok())
                                .expect("SDK sends a complete JSON body");
                            let mut body = vec![0; length];
                            use std::io::Read;
                            stream
                                .read_exact(&mut body)
                                .expect("consume POST body before replying");
                            head_store.lock().unwrap().push(lines);
                            if slow_post {
                                let response = concat!(
                                    "HTTP/1.1 200 OK\r\n",
                                    "content-type: text/event-stream\r\n\r\n",
                                    "event: response.output_text.delta\n",
                                    "data: {\"type\":\"response.output_text.delta\",\"delta\":\"Hi\"}\n\n",
                                );
                                stream.write_all(response.as_bytes()).unwrap();
                                stream.flush().unwrap();
                                std::thread::sleep(std::time::Duration::from_millis(1500));
                                let rest = concat!(
                                    "event: response.completed\n",
                                    "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"response-sse\",\"model\":\"gpt-5.4\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n",
                                );
                                let _ = stream.write_all(rest.as_bytes());
                            } else {
                                let response = format!(
                                    "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                                    codex_completed_sse().len(),
                                    codex_completed_sse()
                                );
                                stream.write_all(response.as_bytes()).unwrap();
                            }
                            let _ = stream.shutdown(std::net::Shutdown::Both);
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
                if scripts.is_empty() && accepted >= 1 && false {
                    break;
                }
            }
        });
        ScriptPeer {
            port,
            heads,
            join: Some(join),
        }
    }

    fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn finish(mut self) {
        self.join.take().unwrap().join().expect("peer thread");
    }
}

/// Upgrade + serve one completed response; optionally wait for the
/// client Close and echo it (well-behaved pooled close).
fn ws_serve_completed(creates: &Arc<Mutex<Vec<String>>>, closes: &Arc<Mutex<usize>>) -> ConnScript {
    let creates = creates.clone();
    let closes = closes.clone();
    Box::new(move |stream: &mut TcpStream, lines: &[String]| {
        stream.write_all(&response_101(lines, &[])).unwrap();
        loop {
            // A read timeout or EOF means the client kept the pooled
            // connection open: the script ends without asserting a Close.
            let (opcode, payload) = match read_frame(stream) {
                Ok(frame) => frame,
                Err(_) => return Ok(()),
            };
            match opcode {
                1 => {
                    creates
                        .lock()
                        .unwrap()
                        .push(String::from_utf8_lossy(&payload).into_owned());
                    write_frame(stream, 1, super::ws_path::completed_event().as_bytes());
                }
                8 => {
                    *closes.lock().unwrap() += 1;
                    write_frame(stream, 8, &[]);
                    return Ok(());
                }
                other => return Err(format!("unexpected opcode {other}")),
            }
        }
    })
}

/// Upgrade, send one delta, then hold: the call is busy. Returns when
/// the client Close arrives (asserting no retry create was sent).
fn ws_hold_busy(creates: &Arc<Mutex<Vec<String>>>) -> ConnScript {
    let creates = creates.clone();
    Box::new(move |stream: &mut TcpStream, lines: &[String]| {
        stream.write_all(&response_101(lines, &[])).unwrap();
        let (opcode, payload) = read_frame(stream)?;
        assert_eq!(opcode, 1);
        creates
            .lock()
            .unwrap()
            .push(String::from_utf8_lossy(&payload).into_owned());
        write_frame(stream, 1, DELTA.as_bytes());
        if let Ok((close_opcode, _)) = read_frame(stream) {
            assert_eq!(close_opcode, 8, "cancelled busy call must close, not retry");
            write_frame(stream, 8, &[]);
        }
        Ok(())
    })
}

/// Respond 503 to the upgrade: transport-class pre-start failure that
/// routes this call to the SSE fallback leg.
fn ws_reject_503() -> ConnScript {
    Box::new(move |stream: &mut TcpStream, _lines: &[String]| {
        stream
            .write_all(b"HTTP/1.1 503 Service Unavailable\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
            .unwrap();
        Ok(())
    })
}

fn ws_reject_connection_limit() -> ConnScript {
    Box::new(|stream: &mut TcpStream, lines: &[String]| {
        stream.write_all(&response_101(lines, &[])).unwrap();
        assert_eq!(read_frame(stream)?.0, 1);
        write_frame(
            stream,
            1,
            connection_limit_fixture()["error"].to_string().as_bytes(),
        );
        super::ws_path::drain_to_eof(stream)?;
        Ok(())
    })
}

#[allow(clippy::type_complexity)]
async fn run_call(
    models: &Models,
    request: Request,
) -> Result<opi_ai::message::AssistantMessage, opi_ai::stream::CallFailure> {
    let record = ChatModelRecord::new("openai-codex", model(None));
    let call = models.stream_simple(
        record,
        request,
        SimpleOptions {
            tool_choice: None,
            temperature: None,
            max_tokens: None,
            timeout: None,
            stop_sequences: vec![],
        },
    );
    tokio::time::timeout(std::time::Duration::from_secs(20), call.result())
        .await
        .expect("call completes within the deadline")
}

// -- distinct cleanup cases ------------------------------------------------

#[tokio::test]
async fn cleanup_session_closes_idle_pooled_connection() {
    let creates = Arc::new(Mutex::new(Vec::new()));
    let closes = Arc::new(Mutex::new(0));
    let peer = ScriptPeer::spawn(vec![ws_serve_completed(&creates, &closes)], false);
    let models = models_with(peer.base_url());
    run_call(&models, session_request("s1"))
        .await
        .expect("call");
    let report = models
        .cleanup_session_resources(Some("s1"))
        .await
        .expect("cleanup");
    assert_eq!(report.closed_connections, 1);
    assert_eq!(report.cancelled_generations, 0);
    assert_eq!(report.cleared_fallback_sessions, 0);
    assert!(report.unconfirmed_effects.is_empty());
    peer.finish();
    assert_eq!(*closes.lock().unwrap(), 1, "peer observed the Close");
}

#[tokio::test]
async fn cleanup_none_cancels_inflight_sessionless_and_closes_pool() {
    let creates = Arc::new(Mutex::new(Vec::new()));
    let closes = Arc::new(Mutex::new(0));
    let peer = ScriptPeer::spawn(vec![ws_serve_completed(&creates, &closes)], false);
    let models = models_with(peer.base_url());
    run_call(&models, session_request("s2"))
        .await
        .expect("pooled call");
    let mut parent = sessionless_request();
    let parent_token = parent.cancel.clone();
    let entered = Arc::new(tokio::sync::Notify::new());
    let signal = entered.clone();
    parent.callbacks =
        opi_ai::callbacks::RequestCallbacks::new().with_async_on_payload(move |_| {
            signal.notify_one();
            std::future::pending()
        });
    let call = Box::pin(models.stream_simple(
        ChatModelRecord::new("openai-codex", model(None)),
        parent,
        SimpleOptions::default(),
    ));
    tokio::time::timeout(std::time::Duration::from_secs(2), entered.notified())
        .await
        .expect("sessionless execution registers and enters payload before cutoff");
    let cleanup = models.cleanup_session_resources(None);
    let (outcome, report) = tokio::join!(finish_call(call), cleanup);
    let failure = outcome.expect_err("sessionless call is cancelled");
    assert!(
        is_cancelled(&failure),
        "sessionless in-flight work is cancelled by cleanup(None), got {failure:?}"
    );
    assert!(report.is_ok());
    let report = report.unwrap();
    assert!(report.cancelled_generations >= 1);
    assert!(report.closed_connections >= 1);
    assert!(!parent_token.is_cancelled(), "caller parent token intact");
    peer.finish();
}

#[tokio::test]
async fn cleanup_empty_session_is_typed_failure_and_keeps_pool() {
    let creates = Arc::new(Mutex::new(Vec::new()));
    let closes = Arc::new(Mutex::new(0));
    let peer = ScriptPeer::spawn(vec![ws_serve_completed(&creates, &closes)], false);
    let models = models_with(peer.base_url());
    run_call(&models, session_request("s1"))
        .await
        .expect("first call");
    let error = models
        .cleanup_session_resources(Some(""))
        .await
        .expect_err("empty session id is a typed failure");
    assert!(matches!(error, CleanupError::EmptySessionId), "{error:?}");
    // The pool survived: the second call reuses the same connection.
    run_call(&models, session_request("s1"))
        .await
        .expect("reuse call");
    assert_eq!(
        creates.lock().unwrap().len(),
        2,
        "second create reuses the pooled connection"
    );
    peer.finish();
}

#[tokio::test]
async fn cleanup_unmatched_session_is_noop_and_retains_reuse() {
    let creates = Arc::new(Mutex::new(Vec::new()));
    let closes = Arc::new(Mutex::new(0));
    let peer = ScriptPeer::spawn(vec![ws_serve_completed(&creates, &closes)], false);
    let models = models_with(peer.base_url());
    run_call(&models, session_request("s1"))
        .await
        .expect("call");
    let report = models
        .cleanup_session_resources(Some("other"))
        .await
        .expect("unmatched cleanup is a no-op");
    assert_eq!(report.closed_connections, 0);
    assert_eq!(report.cancelled_generations, 0);
    assert_eq!(report.cleared_fallback_sessions, 0);
    run_call(&models, session_request("s1"))
        .await
        .expect("reuse");
    assert_eq!(creates.lock().unwrap().len(), 2, "idle pool still reused");
    peer.finish();
}

// -- cancellation coverage -------------------------------------------------

#[tokio::test]
async fn busy_call_cancelled_forbids_late_publish_retry_and_sse() {
    let creates = Arc::new(Mutex::new(Vec::new()));
    let closes = Arc::new(Mutex::new(0));
    let peer = ScriptPeer::spawn(vec![ws_hold_busy(&creates)], false);
    let models = models_with(peer.base_url());
    let call = start_call(&models, session_request("busy")).await;
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    let cleanup = models.cleanup_session_resources(Some("busy"));
    let (outcome, report) = tokio::join!(finish_call(call), cleanup);
    let failure = outcome.expect_err("busy call cancelled");
    assert!(is_cancelled(&failure), "got {failure:?}");
    let report = report.expect("cleanup");
    assert!(report.cancelled_generations >= 1);
    peer.finish();
    assert_eq!(
        creates.lock().unwrap().len(),
        1,
        "exactly one create, no retry"
    );
    let _ = closes;
}

#[tokio::test]
async fn connecting_attempt_cancelled_before_request_frame() {
    let creates: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let script_creates = creates.clone();
    let peer = ScriptPeer::spawn(
        vec![Box::new(move |stream: &mut TcpStream, lines: &[String]| {
            stream.write_all(&response_101(lines, &[])).unwrap();
            // Never serve events; the client cancels while waiting. Hold
            // the connection open until its Close (or an idle timeout).
            loop {
                let (opcode, payload) = match read_frame(stream) {
                    Ok(frame) => frame,
                    Err(_) => return Ok(()),
                };
                match opcode {
                    1 => script_creates
                        .lock()
                        .unwrap()
                        .push(String::from_utf8_lossy(&payload).into_owned()),
                    8 => return Ok(()),
                    other => return Err(format!("unexpected opcode {other}")),
                }
            }
        })],
        false,
    );
    let models = models_with(peer.base_url());
    let call = start_call(&models, session_request("early")).await;
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    let report = models
        .cleanup_session_resources(Some("early"))
        .await
        .expect("cleanup");
    let outcome = finish_call(call).await;
    let failure = outcome.expect_err("early call cancelled");
    assert!(is_cancelled(&failure), "got {failure:?}");
    assert!(report.cancelled_generations >= 1);
    assert_eq!(
        creates.lock().unwrap().len(),
        1,
        "exactly one create reached the wire, no retry"
    );
    peer.finish();
}

#[tokio::test]
async fn failed_call_generation_released_without_leak() {
    let peer = ScriptPeer::spawn(vec![], false);
    let models = models_with(peer.base_url());
    let mut request = session_request("broken");
    request.extra_headers = vec![("authorization".into(), "x".into())];
    let error = run_call(&models, request)
        .await
        .expect_err("invalid header");
    assert!(
        !is_cancelled(&error),
        "pre-wire failure is typed: {error:?}"
    );
    let report = models
        .cleanup_session_resources(Some("broken"))
        .await
        .expect("cleanup after failed call");
    assert_eq!(report.cancelled_generations, 0, "generation released");
    assert_eq!(report.closed_connections, 0);
    peer.finish();
}

#[tokio::test]
async fn active_fallback_sse_leg_cancelled_and_marker_cleared() {
    for mut rejects in [
        vec![ws_reject_503()],
        vec![ws_reject_connection_limit(), ws_reject_connection_limit()],
    ] {
        let rejected_upgrades = rejects.len();
        let creates = Arc::new(Mutex::new(Vec::new()));
        let closes = Arc::new(Mutex::new(0));
        rejects.push(ws_serve_completed(&creates, &closes));
        let peer = ScriptPeer::spawn(rejects, true);
        let models = models_with(peer.base_url());
        let request = session_request("fallback");
        let call = start_call(&models, request).await;
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while peer.heads.lock().unwrap().len() < rejected_upgrades + 1 {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("peer received the fallback POST");
        let report = models
            .cleanup_session_resources(Some("fallback"))
            .await
            .expect("cleanup");
        let outcome = finish_call(call).await;
        let failure = outcome.expect_err("sse leg cancelled");
        assert!(is_cancelled(&failure), "got {failure:?}");
        assert!(report.cancelled_generations >= 1);
        assert_eq!(report.cleared_fallback_sessions, 1);
        run_call(&models, session_request("fallback"))
            .await
            .expect("ws again");
        let heads = peer.heads.lock().unwrap().clone();
        peer.finish();
        let kinds: Vec<bool> = heads
            .iter()
            .map(|lines| header(lines, "upgrade").is_some())
            .collect();
        let mut expected = vec![true; rejected_upgrades];
        expected.extend([false, true]);
        assert_eq!(
            kinds, expected,
            "rejected upgrades, SSE POST, fresh upgrade"
        );
        assert_eq!(creates.lock().unwrap().len(), 1);
    }
}

// -- drop / barrier semantics ----------------------------------------------

#[tokio::test]
async fn consumer_drop_detaches_and_cleanup_joins_the_remaining_execution() {
    let creates = Arc::new(Mutex::new(Vec::new()));
    let peer = ScriptPeer::spawn(vec![ws_hold_busy(&creates)], false);
    let models = models_with(peer.base_url());
    let call = start_call(&models, session_request("dropped")).await;
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while creates.lock().unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the peer observes the active request before consumer Drop");
    // Consumer Drop leaves the producer live. The explicit cleanup owns
    // cancellation and confirms its release.
    drop(call);
    let report = models
        .cleanup_session_resources(Some("dropped"))
        .await
        .expect("barrier completes after drop");
    assert_eq!(report.cancelled_generations, 1);
    assert_eq!(report.closed_connections, 0);
    peer.finish();
}

#[tokio::test]
async fn dropped_cleanup_retains_pending_idle_close_for_the_next_barrier() {
    let (close_seen_tx, close_seen_rx) = tokio::sync::oneshot::channel();
    let close_seen_tx = Mutex::new(Some(close_seen_tx));
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let peer = ScriptPeer::spawn(
        vec![Box::new(move |stream, lines| {
            stream.write_all(&response_101(lines, &[])).unwrap();
            let (opcode, _) = read_frame(stream)?;
            assert_eq!(opcode, 1);
            write_frame(stream, 1, super::ws_path::completed_event().as_bytes());
            let (opcode, payload) = read_frame(stream)?;
            assert_eq!(opcode, 8);
            close_seen_tx
                .lock()
                .unwrap()
                .take()
                .unwrap()
                .send(())
                .unwrap();
            release_rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap();
            write_frame(stream, 8, &payload);
            stream.shutdown(std::net::Shutdown::Write).unwrap();
            Ok(())
        })],
        false,
    );
    let models = models_with(peer.base_url());
    run_call(&models, session_request("drop-cleanup"))
        .await
        .expect("pooled call");
    let mut first = Box::pin(models.cleanup_session_resources(Some("drop-cleanup")));
    tokio::select! {
        result = &mut first => panic!("Close completed before peer acknowledgment: {result:?}"),
        result = close_seen_rx => result.expect("peer saw Close"),
    }
    drop(first);
    let unrelated = tokio::time::timeout(
        std::time::Duration::from_millis(40),
        models.cleanup_session_resources(Some("unrelated-cleanup")),
    )
    .await
    .expect("another session must not wait for this Close")
    .expect("unmatched selector");
    assert_eq!(unrelated.closed_connections, 0);
    let mut repeated = Box::pin(models.cleanup_session_resources(Some("drop-cleanup")));
    let premature = tokio::time::timeout(std::time::Duration::from_millis(40), &mut repeated).await;
    release_tx.send(()).unwrap();
    assert!(
        premature.is_err(),
        "repeat cleanup must retain and await the pending Close: {premature:?}"
    );
    let report = repeated.await.expect("peer confirmed retained Close");
    assert_eq!(report.closed_connections, 1);
    peer.finish();
}

#[tokio::test]
async fn cleanup_close_failure_continues_and_reports_unconfirmed() {
    let creates: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let closes = Arc::new(Mutex::new(0));
    let broken = creates.clone();
    let peer = ScriptPeer::spawn(
        vec![
            Box::new(move |stream: &mut TcpStream, lines: &[String]| {
                stream.write_all(&response_101(lines, &[])).unwrap();
                let (opcode, payload) = read_frame(stream)?;
                assert_eq!(opcode, 1);
                broken
                    .lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&payload).into_owned());
                write_frame(stream, 1, super::ws_path::completed_event().as_bytes());
                // Vanish before the client Close: the close write cannot
                // be confirmed.
                let _ = stream.shutdown(std::net::Shutdown::Both);
                std::thread::sleep(std::time::Duration::from_millis(300));
                Ok(())
            }),
            ws_serve_completed(&creates, &closes),
        ],
        false,
    );
    let models = models_with(peer.base_url());
    run_call(&models, session_request("bad"))
        .await
        .expect("call 1");
    run_call(&models, session_request("good"))
        .await
        .expect("call 2");
    let error = models
        .cleanup_session_resources(None)
        .await
        .expect_err("unconfirmed release is a typed cleanup failure");
    let CleanupError::Incomplete { report } = error else {
        panic!("expected confirmed effects and unconfirmed releases");
    };
    assert!(
        !report.unconfirmed_effects.is_empty(),
        "unconfirmed close is reported"
    );
    assert!(report.closed_connections >= 1, "other closes still ran");
    peer.finish();
}

#[tokio::test]
async fn two_models_providers_isolated_on_shared_session_ids() {
    let creates_a = Arc::new(Mutex::new(Vec::new()));
    let creates_b = Arc::new(Mutex::new(Vec::new()));
    let closes_b = Arc::new(Mutex::new(0));
    let peer_a = ScriptPeer::spawn(vec![ws_hold_busy(&creates_a)], false);
    let peer_b = ScriptPeer::spawn(vec![ws_serve_completed(&creates_b, &closes_b)], false);
    let models_a = models_with(peer_a.base_url());
    let models_b = models_with(peer_b.base_url());
    let parent_token = {
        let request = session_request("shared");
        let token = request.cancel.clone();
        run_call(&models_b, request).await.expect("pooled on B");
        token
    };
    let busy = start_call(&models_a, session_request("shared")).await;
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    let cleanup = models_a.cleanup_session_resources(Some("shared"));
    let (outcome, report) = tokio::join!(finish_call(busy), cleanup);
    let failure = outcome.expect_err("A cancelled");
    assert!(is_cancelled(&failure), "got {failure:?}");
    let report = report.expect("cleanup");
    assert!(report.cancelled_generations >= 1);
    // B untouched: its pool still serves a reuse on the same session id.
    run_call(&models_b, session_request("shared"))
        .await
        .expect("B reuse");
    assert_eq!(creates_b.lock().unwrap().len(), 2, "B pool retained");
    assert!(!parent_token.is_cancelled(), "parent token intact");
    peer_a.finish();
    peer_b.finish();
}

#[tokio::test]
async fn pure_sse_preference_call_is_not_registered() {
    let peer = ScriptPeer::spawn(vec![], true);
    let models = models_with(peer.base_url());
    let mut request = session_request("sse-only");
    request.transport = TransportPreference::Sse;
    let call = run_call(&models, request);
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let report = models
        .cleanup_session_resources(None)
        .await
        .expect("cleanup");
    assert_eq!(report.cancelled_generations, 0, "sse-only not registered");
    let message = call
        .await
        .expect("pure sse call survives cleanup and completes");
    assert_eq!(message.response_id.as_deref(), Some("response-sse"));
    peer.finish();
}
