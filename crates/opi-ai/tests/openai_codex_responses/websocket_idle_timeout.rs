use std::io::Write;
use std::net::{Shutdown, TcpStream};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;

use futures_util::StreamExt;
use opi_ai::http::{HttpClientBuilder, RedirectPolicy};
use opi_ai::provider::TransportPreference;
use opi_ai::retry::{WholeCallRetryClassification, classify_whole_call_retry};
use opi_ai::{ChatStream, RequestCallbacks};
use serde_json::{Value, json};

use super::ws_continuation::second_turn_request;
use super::ws_path::{CodexPeer, PeerCapture, read_frame, response_101, sse_response, write_frame};
use super::*;

#[derive(Default)]
struct TrafficCapture {
    creates: Vec<Value>,
    early_close: Option<Value>,
    saw_pong: bool,
}

fn framed(events: &Value) -> Vec<u8> {
    let mut frames = Vec::new();
    for event in events.as_array().unwrap() {
        write_frame(&mut frames, 1, event.to_string().as_bytes());
    }
    frames
}

fn read_create(
    capture: &Arc<Mutex<PeerCapture>>,
    traffic: &Mutex<TrafficCapture>,
    data: &[u8],
) -> bool {
    let body: Value = serde_json::from_slice(data).unwrap();
    assert_eq!(body["type"], "response.create");
    let connection = capture.lock().unwrap().heads.len();
    let mut traffic = traffic.lock().unwrap();
    traffic.creates.push(json!({
        "connection": connection, "items": body["input"].as_array().unwrap().len(),
        "previous": body["previous_response_id"],
    }));
    traffic.creates.len() == 1
}

fn serve_traffic(
    mut socket: TcpStream,
    heads: Vec<String>,
    capture: &Arc<Mutex<PeerCapture>>,
    traffic: &Mutex<TrafficCapture>,
    fixture: &Value,
    row: &Value,
) -> Result<(), String> {
    socket.write_all(&response_101(&heads, &[])).unwrap();
    let (opcode, data) = read_frame(&mut socket)?;
    assert_eq!(opcode, 1);
    let first = read_create(capture, traffic, &data);
    let next = framed(&fixture["frames"]["next"]);
    if !first {
        socket.write_all(&next).unwrap();
        return Ok(());
    }
    let prefix = framed(&fixture["frames"][row["prefix"].as_str().unwrap()]);
    socket.write_all(&prefix).unwrap();
    let mut writer = socket.try_clone().unwrap();
    let (stop, stopped) = mpsc::channel::<()>();
    let sent_late = AtomicBool::new(false);
    std::thread::scope(|scope| {
        let sent_late = &sent_late;
        let task = scope.spawn(move || {
            let tick = Duration::from_millis(fixture["tickMs"].as_u64().unwrap());
            let mut bytes = Vec::new();
            match row["traffic"].as_str().unwrap() {
                "ping" => write_frame(&mut bytes, 9, b"heartbeat"),
                "pong" => write_frame(&mut bytes, 10, b"heartbeat"),
                "progress" => bytes = framed(&fixture["frames"]["progress"]),
                "silent" => {}
                other => panic!("unexpected traffic {other}"),
            }
            for _ in 0..fixture["ticks"].as_u64().unwrap() {
                if stopped.recv_timeout(tick) != Err(mpsc::RecvTimeoutError::Timeout) {
                    return;
                }
                if writer.write_all(&bytes).is_err() {
                    return;
                }
            }
            sent_late.store(true, Ordering::SeqCst);
            let _ = writer.write_all(&framed(&fixture["frames"]["late"]));
        });
        let mut close = None;
        while let Ok((opcode, data)) = read_frame(&mut socket) {
            match opcode {
                1 => {
                    assert!(!read_create(capture, traffic, &data));
                    socket.write_all(&next).unwrap();
                }
                8 => {
                    if !sent_late.load(Ordering::SeqCst) {
                        let code = u16::from_be_bytes(data[..2].try_into().unwrap());
                        let reason = std::str::from_utf8(&data[2..]).unwrap();
                        traffic.lock().unwrap().early_close =
                            Some(json!({ "code": code, "reason": reason }));
                    }
                    close = Some(data);
                    break;
                }
                10 => {
                    assert_eq!(data, b"heartbeat");
                    traffic.lock().unwrap().saw_pong = true;
                }
                other => panic!("unexpected client opcode {other}"),
            }
        }
        drop(stop);
        task.join().unwrap();
        if let Some(data) = close {
            let mut reply = Vec::new();
            write_frame(&mut reply, 8, &data);
            let _ = socket.write_all(&reply);
            let _ = socket.shutdown(Shutdown::Write);
        }
    });
    Ok(())
}

async fn observe_call(
    provider: &OpenAiCodexResponsesProvider,
    mut request: Request,
    hold: Duration,
) -> Value {
    let raw = Arc::new(Mutex::new(Vec::new()));
    let observed = raw.clone();
    request.callbacks = RequestCallbacks::new().with_async_on_raw(move |event| {
        let observed = observed.clone();
        async move {
            observed.lock().unwrap().push(event["type"].clone());
            if event["type"] == "response.in_progress" && !hold.is_zero() {
                tokio::time::sleep(hold).await;
            }
            Ok(())
        }
    });
    let mut stream = ChatStream::new(
        provider.stream_prepared(request, auth("fixture", None, Some("acct-1".into()))),
    );
    let (terminals, result) = tokio::time::timeout(Duration::from_secs(4), async {
        let mut terminals = Vec::new();
        while let Some(item) = stream.next().await {
            match item {
                Ok(AssistantStreamEvent::Done { .. }) => terminals.push("done"),
                Err(_) => terminals.push("error"),
                _ => {}
            }
        }
        (terminals, stream.result().await)
    })
    .await
    .expect("loopback call settled");
    let message = result.as_ref().unwrap_or_else(|failure| failure.message());
    let text: String = message
        .content
        .iter()
        .filter_map(|part| match part {
            AssistantContent::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    let fallback = message
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.kind == "codex_websocket_fallback")
        .map(|diagnostic| &diagnostic.reason);
    json!({
        "stop": message.stop_reason, "responseId": message.response_id, "text": text,
        "totalTokens": message.usage.total_tokens, "terminal": terminals,
        "raw": *raw.lock().unwrap(), "fallback": fallback,
        "error": result.as_ref().err().map(|failure| failure.cause().to_string()),
        "retryable": result.as_ref().err().map(|failure| failure.cause().is_retryable()),
        "wholeCallRetryable": result.as_ref().err().map(|failure| {
            classify_whole_call_retry(failure.message()) == WholeCallRetryClassification::Transient
        }),
    })
}

#[tokio::test]
async fn application_idle_timeout_ignores_websocket_control_traffic() {
    let fixture: Value =
        serde_json::from_str(include_str!("../data/codex-websocket-idle-timeout.json")).unwrap();
    let sse: String = fixture["frames"]["sse"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect();
    let mut differences = Vec::new();
    for row in fixture["cases"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let outcome = row["outcome"].as_str().unwrap();
        let expires = outcome != "complete";
        let timeout = Duration::from_millis(
            row["timeoutMs"]
                .as_u64()
                .unwrap_or_else(|| fixture["idleMs"].as_u64().unwrap()),
        );
        let hold = if row["holdCallback"] == true {
            Duration::from_millis(fixture["callbackHoldMs"].as_u64().unwrap())
        } else {
            Duration::ZERO
        };
        for (transport, preference) in [
            ("auto", TransportPreference::Auto),
            ("websocket-cached", TransportPreference::WebsocketCached),
            ("websocket", TransportPreference::Websocket),
        ] {
            let traffic = Arc::new(Mutex::new(TrafficCapture::default()));
            let finished = Arc::new(AtomicBool::new(false));
            let served = traffic.clone();
            let fixture_data = fixture.clone();
            let row_data = row.clone();
            let serve = move |socket, heads, capture: &Arc<Mutex<PeerCapture>>| {
                serve_traffic(socket, heads, capture, &served, &fixture_data, &row_data)
            };
            let complete = finished.clone();
            let peer = CodexPeer::spawn_many_until(
                vec![serve.clone(), serve],
                4,
                sse_response(&sse, 200),
                move || complete.load(Ordering::SeqCst),
            );
            let provider = OpenAiCodexResponsesProvider::new(
                Some(peer.base_url()),
                vec![model(None)],
                Arc::new(
                    HttpClientBuilder::new()
                        .redirects(RedirectPolicy::None)
                        .build()
                        .unwrap(),
                ),
            );
            let mut first_request = request();
            first_request.transport = preference;
            first_request.timeout = Some(timeout);
            let first = observe_call(&provider, first_request, hold).await;
            let first_posts = peer.capture.lock().unwrap().posts.len();
            let next = if expires {
                let mut request = second_turn_request();
                request.transport = preference;
                observe_call(&provider, request, Duration::ZERO).await
            } else {
                Value::Null
            };
            drop(provider);
            finished.store(true, Ordering::SeqCst);
            let capture = tokio::task::spawn_blocking(move || peer.finish())
                .await
                .unwrap()
                .unwrap();
            let capture = capture.lock().unwrap();
            let traffic = traffic.lock().unwrap();
            let posts: Vec<_> = capture.posts.iter().map(|body| json!({
                "items": body["input"].as_array().unwrap().len(), "previous": body["previous_response_id"],
            })).collect();
            let observation = json!({
                "name": name, "transport": transport, "creates": traffic.creates,
                "firstPosts": first_posts, "posts": posts, "close": traffic.early_close,
                "sawPong": traffic.saw_pong, "first": first, "next": next,
            });
            println!("codex_idle_timeout {observation}");
            assert_eq!(
                traffic.saw_pong,
                row["traffic"] == "ping",
                "{name}/{transport}"
            );
            let expected_posts = match outcome {
                "fallback" => json!([{"items":1,"previous":null}, {"items":2,"previous":null}]),
                "terminal" => json!([{"items":2,"previous":null}]),
                _ => json!([]),
            };
            let mut expected_raw: Vec<_> = fixture["frames"][row["prefix"].as_str().unwrap()]
                .as_array()
                .unwrap()
                .iter()
                .map(|frame| frame["type"].clone())
                .collect();
            if row["traffic"] == "progress" {
                expected_raw.extend(std::iter::repeat_n(
                    json!("response.in_progress"),
                    fixture["ticks"].as_u64().unwrap() as usize,
                ));
            }
            if outcome != "terminal" {
                expected_raw.push(json!("response.done"));
            }
            let matches_first = match outcome {
                "fallback" => {
                    first["responseId"] == "resp_sse"
                        && first["totalTokens"] == 3
                        && first["stop"] == "stop"
                        && first["fallback"] == "idle_timeout"
                        && first["terminal"] == json!(["done"])
                }
                "terminal" => {
                    first["stop"] == "error"
                        && first["terminal"] == json!(["error"])
                        && first["retryable"] == false
                        && first["wholeCallRetryable"] == true
                        && first["totalTokens"].is_null()
                        && first["error"]
                            .as_str()
                            .is_some_and(|message| message.contains("idle timeout"))
                        && if row["prefix"] == "partial" {
                            first["responseId"] == "resp_partial"
                                && first["text"] == "partial answer"
                        } else {
                            first["responseId"].is_null() && first["text"] == ""
                        }
                }
                _ => {
                    first["responseId"] == "resp_late"
                        && first["totalTokens"] == 7
                        && first["stop"] == "stop"
                        && first["terminal"] == json!(["done"])
                }
            };
            let expected_close = expires.then(|| json!({"code":1000,"reason":"idle_timeout"}));
            if !matches_first
                || first["raw"] != json!(expected_raw)
                || traffic.creates != [json!({"connection":1,"items":1,"previous":null})]
                || observation["posts"] != expected_posts
                || first_posts != usize::from(outcome == "fallback")
                || traffic.early_close != expected_close
                || (expires
                    && (next["responseId"] != "resp_sse"
                        || next["fallback"] != "session_fallback"
                        || next["terminal"] != json!(["done"])))
            {
                differences.push(format!("{name}/{transport}"));
            }
        }
    }
    assert!(
        differences.is_empty(),
        "application idle timeout: {differences:?}"
    );
}
