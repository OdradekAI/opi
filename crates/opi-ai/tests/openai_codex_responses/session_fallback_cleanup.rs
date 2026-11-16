use std::io::Write;
use std::net::{Shutdown, TcpStream};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use opi_ai::provider::TransportPreference;
use opi_ai::{ChatModelRecord, Models, SimpleOptions};
use serde_json::{Value, json};

use super::ws_continuation::second_turn_request;
use super::ws_path::{
    CodexPeer, PeerCapture, header, read_frame, response_101, sse_response, write_frame,
};
use super::ws_resources::models_with;
use super::*;

fn request_fields(body: &Value) -> Value {
    let input: Vec<String> = body["input"]
        .as_array()
        .expect("input array")
        .iter()
        .map(|item| {
            assert_eq!(item["role"], "user");
            let content = &item["content"];
            match content.as_str() {
                Some(text) => text.to_owned(),
                None => content
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|part| {
                        assert_eq!(part["type"], "input_text");
                        part["text"].as_str().unwrap()
                    })
                    .collect(),
            }
        })
        .collect();
    json!({"cacheKey": body["prompt_cache_key"], "input": input, "previous": body["previous_response_id"]})
}

fn send_on_create(
    socket: &mut TcpStream,
    capture: &Mutex<PeerCapture>,
    connection: usize,
    payload: &[u8],
    events: &Value,
) {
    let body: Value = serde_json::from_slice(payload).unwrap();
    assert_eq!(body["type"], "response.create");
    capture.lock().unwrap().frames.push(json!({
        "connection": connection, "body": request_fields(&body),
    }));
    for event in events.as_array().unwrap() {
        write_frame(socket, 1, event.to_string().as_bytes());
    }
}

fn serve(
    mut socket: TcpStream,
    heads: Vec<String>,
    capture: &Mutex<PeerCapture>,
    closes: &Mutex<Vec<Value>>,
    frames: &Value,
) -> Result<(), String> {
    assert_eq!(heads[0], "GET /codex/responses HTTP/1.1");
    let connection = capture
        .lock()
        .unwrap()
        .heads
        .iter()
        .filter(|head| header(head, "upgrade").is_some())
        .count();
    socket.write_all(&response_101(&heads, &[])).unwrap();
    let (opcode, payload) = read_frame(&mut socket)?;
    assert_eq!(opcode, 1);
    let events = match connection {
        1 => &frames["none"],
        2 => &frames["partial"],
        _ => &frames["next"],
    };
    send_on_create(&mut socket, capture, connection, &payload, events);
    if connection > 3 {
        return Ok(());
    }

    // The first two sockets expire. The third stays pooled until either
    // public cleanup closes it or the control call reuses it.
    let (opcode, payload) = read_frame(&mut socket)?;
    if connection == 3 && opcode == 1 {
        send_on_create(&mut socket, capture, connection, &payload, events);
        return Ok(());
    }
    assert_eq!(opcode, 8);
    let code = u16::from_be_bytes(payload[..2].try_into().unwrap());
    let reason = std::str::from_utf8(&payload[2..]).unwrap();
    closes.lock().unwrap().push(json!({
        "connection": connection, "code": code, "reason": reason,
    }));
    write_frame(&mut socket, 8, &payload);
    socket.shutdown(Shutdown::Write).unwrap();
    Ok(())
}

async fn observe_call(
    models: &Models,
    peer: &CodexPeer,
    mut request: Request,
    step: &str,
) -> Value {
    let (creates_before, posts_before) = {
        let capture = peer.capture.lock().unwrap();
        (capture.frames.len(), capture.posts.len())
    };
    let raw = Arc::new(Mutex::new(Vec::new()));
    let observed = raw.clone();
    request.callbacks = RequestCallbacks::new().with_async_on_raw(move |event| {
        let observed = observed.clone();
        async move {
            observed.lock().unwrap().push(event["type"].clone());
            Ok(())
        }
    });
    let mut call = models.stream_simple(
        ChatModelRecord::new("openai-codex", model(None)),
        request,
        SimpleOptions::default(),
    );
    let (terminal, result) = tokio::time::timeout(Duration::from_secs(4), async {
        let mut terminal = Vec::new();
        while let Some(event) = call.next().await {
            match event {
                Ok(AssistantStreamEvent::Done { .. }) => terminal.push("done"),
                Err(_) => terminal.push("error"),
                _ => {}
            }
        }
        (terminal, call.result().await)
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
    let capture = peer.capture.lock().unwrap();
    let posts: Vec<_> = capture.posts[posts_before..]
        .iter()
        .map(request_fields)
        .collect();
    json!({
        "step": step, "creates": &capture.frames[creates_before..], "posts": posts,
        "stop": message.stop_reason, "responseId": message.response_id, "text": text,
        "totalTokens": message.usage.total_tokens, "terminal": terminal,
        "raw": *raw.lock().unwrap(), "error": result.as_ref().err().map(|failure| failure.cause().to_string()),
    })
}

#[tokio::test]
async fn public_cleanup_reopens_websocket_for_selected_sticky_sessions() {
    let fixture: Value =
        serde_json::from_str(include_str!("../data/codex-websocket-idle-timeout.json")).unwrap();
    let frames = &fixture["frames"];
    let sse: String = frames["sse"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect();
    let idle = Duration::from_millis(fixture["idleMs"].as_u64().unwrap());
    for scope in ["target", "global"] {
        let global = scope == "global";
        for (transport, preference) in [
            ("auto", TransportPreference::Auto),
            ("websocket-cached", TransportPreference::WebsocketCached),
            ("websocket", TransportPreference::Websocket),
        ] {
            let target = format!("cleanup-{scope}-{transport}-target");
            let other = format!("cleanup-{scope}-{transport}-other");
            let control = format!("cleanup-{scope}-{transport}-control");
            let closes = Arc::new(Mutex::new(Vec::new()));
            let observed_closes = closes.clone();
            let served = frames.clone();
            let handler = move |socket, head, capture: &Arc<Mutex<PeerCapture>>| {
                serve(socket, head, capture, &observed_closes, &served)
            };
            let finished = Arc::new(AtomicBool::new(false));
            let done = finished.clone();
            let peer = CodexPeer::spawn_many_until(
                vec![handler; 6],
                10,
                sse_response(&sse, 200),
                move || done.load(Ordering::SeqCst),
            );
            let models = models_with(peer.base_url());
            let call = |step, session: &str, second, timeout| {
                let mut request = if second {
                    second_turn_request()
                } else {
                    request()
                };
                request.session_id = Some(session.to_owned());
                request.transport = preference;
                request.timeout = timeout;
                observe_call(&models, &peer, request, step)
            };
            let mut calls = vec![
                call("targetSeed", &target, false, Some(idle)).await,
                call("otherSeed", &other, false, Some(idle)).await,
                call("targetSticky", &target, true, None).await,
                call("otherSticky", &other, true, None).await,
                call("controlWarm", &control, false, None).await,
            ];
            let selector = (!global).then_some(target.as_str());
            let report = tokio::time::timeout(
                Duration::from_secs(4),
                models.cleanup_session_resources(selector),
            )
            .await
            .expect("public cleanup settled")
            .expect("confirmed cleanup");
            calls.extend([
                call("controlAfter", &control, true, None).await,
                call("targetAfter", &target, true, None).await,
                call("otherAfter", &other, true, None).await,
            ]);
            drop(models);
            finished.store(true, Ordering::SeqCst);
            let capture = tokio::task::spawn_blocking(move || peer.finish())
                .await
                .unwrap()
                .unwrap();
            let capture = capture.lock().unwrap();
            for head in &capture.heads {
                if header(head, "upgrade").is_none() {
                    assert_eq!(head[0], "POST /codex/responses HTTP/1.1");
                }
            }
            let closes = closes.lock().unwrap();
            println!(
                "codex_cleanup {}",
                json!({
                    "scope": scope, "transport": transport, "selector": selector, "calls": calls,
                    "connections": capture.heads.iter().filter(|head| header(head, "upgrade").is_some()).count(),
                    "closes": *closes, "cleanup": {
                        "cancelledGenerations": report.cancelled_generations,
                        "closedConnections": report.closed_connections,
                        "clearedFallbackSessions": report.cleared_fallback_sessions,
                        "unconfirmedEffects": report.unconfirmed_effects,
                    },
                })
            );

            assert_eq!(report.cancelled_generations, 0);
            assert_eq!(report.cleared_fallback_sessions, if global { 2 } else { 1 });
            assert_eq!(report.closed_connections, usize::from(global));
            assert!(report.unconfirmed_effects.is_empty());
            let mut expected_closes = vec![
                json!({"connection": 1, "code": 1000, "reason": "idle_timeout"}),
                json!({"connection": 2, "code": 1000, "reason": "idle_timeout"}),
            ];
            if global {
                expected_closes.push(json!({"connection": 3, "code": 1000, "reason": ""}));
            }
            assert_eq!(*closes, expected_closes);
            assert_eq!(capture.heads.len(), if global { 9 } else { 8 });
            let first = ["hello"];
            let full = ["hello", "second turn"];
            let body = |session: &str, input: &[&str], previous: Option<&str>| json!({"cacheKey": session, "input": input, "previous": previous});
            let ws = |connection, body| json!({"connection": connection, "body": body});
            let post = |session: &str, input: &[&str]| body(session, input, None);
            let control_delta = !global && preference != TransportPreference::Websocket;
            let control_body = if control_delta {
                body(&control, &["second turn"], Some("resp_next_ws"))
            } else {
                post(&control, &full)
            };
            let empty = json!([]);
            let expected = [
                (
                    json!([ws(1, post(&target, &first))]),
                    json!([post(&target, &first)]),
                    "resp_sse",
                    3,
                ),
                (
                    json!([ws(2, post(&other, &first))]),
                    empty.clone(),
                    "resp_partial",
                    0,
                ),
                (empty.clone(), json!([post(&target, &full)]), "resp_sse", 3),
                (empty.clone(), json!([post(&other, &full)]), "resp_sse", 3),
                (
                    json!([ws(3, post(&control, &first))]),
                    empty.clone(),
                    "resp_next_ws",
                    7,
                ),
                (
                    json!([ws(if global { 4 } else { 3 }, control_body)]),
                    empty.clone(),
                    "resp_next_ws",
                    7,
                ),
                (
                    json!([ws(if global { 5 } else { 4 }, post(&target, &full))]),
                    empty.clone(),
                    "resp_next_ws",
                    7,
                ),
                if global {
                    (
                        json!([ws(6, post(&other, &full))]),
                        empty.clone(),
                        "resp_next_ws",
                        7,
                    )
                } else {
                    (empty, json!([post(&other, &full)]), "resp_sse", 3)
                },
            ];
            for (call, (creates, posts, id, tokens)) in calls.iter().zip(expected) {
                assert_eq!(call["creates"], creates, "{call}");
                assert_eq!(call["posts"], posts, "{call}");
                assert_eq!(call["responseId"], id, "{call}");
                if tokens == 0 {
                    assert_eq!(call["stop"], "error");
                    assert_eq!(call["terminal"], json!(["error"]));
                    assert_eq!(call["text"], "partial answer");
                    assert!(call["totalTokens"].is_null());
                    assert!(
                        call["error"]
                            .as_str()
                            .is_some_and(|error| error.contains("idle timeout"))
                    );
                    let partial_types: Vec<_> = frames["partial"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|event| &event["type"])
                        .collect();
                    assert_eq!(call["raw"], json!(partial_types));
                } else {
                    assert_eq!(call["stop"], "stop");
                    assert_eq!(call["terminal"], json!(["done"]));
                    assert_eq!(call["text"], "");
                    assert_eq!(call["totalTokens"], tokens);
                    assert_eq!(call["raw"], json!(["response.done"]));
                    assert!(call["error"].is_null());
                }
            }
        }
    }
}
