fn request() -> Request {
    let mut request = super::request();
    request.transport = opi_ai::provider::TransportPreference::Auto;
    request
}

use super::ws_continuation::frame_call;
use super::ws_continuation::second_turn_request;
use super::ws_path::{
    CodexPeer, completed_event, drain_to_eof, read_frame, response_101, write_frame,
};
use super::*;
use opi_ai::http::{HttpClientBuilder, RedirectPolicy};
use opi_ai::provider::TransportPreference;
use std::io::Write;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

fn ws_provider(base_url: String) -> OpenAiCodexResponsesProvider {
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

fn auth_ws() -> ResolvedAuth {
    auth("ws-secret", None, Some("acct-1".into()))
}

/// Read one frame and reply with a JSON error event.
fn reply_error(stream: &mut TcpStream, code: &str) -> Result<(), String> {
    let (opcode, _) = read_frame(stream)?;
    assert_eq!(opcode, 1);
    let error = format!(r#"{{"type":"error","code":"{code}"}}"#);
    write_frame(stream, 1, error.as_bytes());
    drain_to_eof(stream)?;
    Ok(())
}

#[tokio::test]
async fn missing_previous_response_allows_one_full_input_retry() {
    let peer = CodexPeer::spawn_many(
        vec![
            // One pooled connection: call 1's full input, then call 2's
            // delta, which the server rejects.
            |mut stream: TcpStream,
             lines: Vec<String>,
             capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                stream.write_all(&response_101(&lines, &[])).unwrap();
                for turn in 0..2 {
                    let (opcode, payload) = read_frame(&mut stream)?;
                    assert_eq!(opcode, 1);
                    capture
                        .lock()
                        .unwrap()
                        .frames
                        .push(serde_json::from_slice(&payload).unwrap());
                    if turn == 0 {
                        write_frame(&mut stream, 1, completed_event().as_bytes());
                    } else {
                        // An ignored provider event seals ordinary fallback and reconnect,
                        // but the separate missing-previous branch still precedes public output.
                        write_frame(&mut stream, 1, br#"{"type":"response.in_progress"}"#);
                        let error =
                            r#"{"type":"error","error":{"code":"previous_response_not_found"}}"#;
                        write_frame(&mut stream, 1, error.as_bytes());
                    }
                }
                drain_to_eof(&mut stream)?;
                Ok(())
            },
            // Call 2 attempt 2: full input on a fresh connection.
            |mut stream: TcpStream,
             lines: Vec<String>,
             capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                stream.write_all(&response_101(&lines, &[])).unwrap();
                let (opcode, payload) = read_frame(&mut stream)?;
                assert_eq!(opcode, 1);
                capture
                    .lock()
                    .unwrap()
                    .frames
                    .push(serde_json::from_slice(&payload).unwrap());
                write_frame(&mut stream, 1, completed_event().as_bytes());
                drain_to_eof(&mut stream)?;
                Ok(())
            },
        ],
        2,
    );
    let provider = ws_provider(peer.base_url());
    let mut first = request();
    first.transport = TransportPreference::WebsocketCached;
    frame_call(&provider, first).await;
    let mut second = second_turn_request();
    second.transport = TransportPreference::WebsocketCached;
    // The retry happens before public output: the stream still finishes
    // with exactly one Done.
    frame_call(&provider, second).await;
    let capture = peer.finish().expect("peer script");
    {
        let guard = capture.lock().unwrap();
        assert_eq!(guard.frames.len(), 3);
        assert!(guard.posts.is_empty());
        assert_eq!(guard.frames[1]["previous_response_id"], "response-1");
        assert!(
            guard.frames[2].get("previous_response_id").is_none(),
            "the retry sends full input"
        );
        assert_eq!(guard.frames[2]["input"].as_array().unwrap().len(), 2);
    }
}

#[tokio::test]
async fn missing_previous_response_retry_is_bounded_to_one() {
    let peer = CodexPeer::spawn_many(
        vec![
            |mut stream: TcpStream,
             lines: Vec<String>,
             _capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                stream.write_all(&response_101(&lines, &[])).unwrap();
                let (opcode, _) = read_frame(&mut stream)?;
                assert_eq!(opcode, 1);
                write_frame(&mut stream, 1, completed_event().as_bytes());
                reply_error(&mut stream, "previous_response_not_found")
            },
            reply_missing,
        ],
        2,
    );
    let provider = ws_provider(peer.base_url());
    let mut first = request();
    first.transport = TransportPreference::WebsocketCached;
    frame_call(&provider, first).await;
    let mut second = second_turn_request();
    second.transport = TransportPreference::WebsocketCached;
    let error = drain(&provider, second, auth_ws()).await;
    assert!(
        error.is_some(),
        "the second missing-id rejection is terminal"
    );
    peer.finish().expect("peer script");
}

fn reply_missing(
    mut stream: TcpStream,
    lines: Vec<String>,
    _capture: &Arc<Mutex<super::ws_path::PeerCapture>>,
) -> Result<(), String> {
    stream.write_all(&response_101(&lines, &[])).unwrap();
    reply_error(&mut stream, "previous_response_not_found")
}

#[tokio::test]
async fn connection_limit_before_start_allows_one_reconnect() {
    let peer = CodexPeer::spawn_many(
        vec![
            |mut stream: TcpStream,
             lines: Vec<String>,
             _capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                stream.write_all(&response_101(&lines, &[])).unwrap();
                reply_error(&mut stream, "websocket_connection_limit_reached")
            },
            |mut stream: TcpStream,
             lines: Vec<String>,
             capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                stream.write_all(&response_101(&lines, &[])).unwrap();
                let (opcode, payload) = read_frame(&mut stream)?;
                assert_eq!(opcode, 1);
                capture
                    .lock()
                    .unwrap()
                    .frames
                    .push(serde_json::from_slice(&payload).unwrap());
                write_frame(&mut stream, 1, completed_event().as_bytes());
                drain_to_eof(&mut stream)?;
                Ok(())
            },
        ],
        2,
    );
    let provider = ws_provider(peer.base_url());
    let mut call = request();
    call.transport = TransportPreference::WebsocketCached;
    frame_call(&provider, call).await;
    let capture = peer.finish().expect("peer script");
    {
        let guard = capture.lock().unwrap();
        assert_eq!(guard.heads.len(), 2, "one reconnect, then success");
        assert!(
            guard.frames[0].get("previous_response_id").is_none(),
            "the successful reconnect sends full input"
        );
    }
}

#[tokio::test]
async fn response_failed_uses_the_codex_recovery_budgets() {
    use opi_ai::retry::{WholeCallRetryClassification, classify_whole_call_retry};
    use serde_json::Value;

    let fixture: Value =
        serde_json::from_str(include_str!("../data/codex-failed-recovery.json")).unwrap();
    let events_for = |groups: &Value| -> Vec<Value> {
        groups
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|group| {
                fixture["frames"][group.as_str().unwrap()]
                    .as_array()
                    .unwrap()
            })
            .cloned()
            .collect()
    };
    let mut mismatches = Vec::new();
    for row in fixture["cases"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let warm = row["warm"].as_bool().unwrap_or(false);
        let ws = row["ws"].as_array().unwrap();
        let post = row.get("sse").is_some();
        let expected_ws = row["rustWs"]
            .as_u64()
            .map_or(ws.len(), |count| count as usize);
        let sse: String = events_for(&row["sse"])
            .iter()
            .map(|event| format!("data: {event}\n\n"))
            .collect();
        let handlers = ws
            .iter()
            .enumerate()
            .map(|(index, groups)| {
                let events = events_for(groups);
                let warmup =
                    (warm && index == 0).then(|| fixture["frames"]["warmup"][0].to_string());
                move |mut socket: TcpStream,
                      head: Vec<String>,
                      capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                    socket.write_all(&response_101(&head, &[])).unwrap();
                    if let Some(warmup) = &warmup {
                        record_create(&mut socket, capture)?;
                        write_frame(&mut socket, 1, warmup.as_bytes());
                    }
                    record_create(&mut socket, capture)?;
                    for event in &events {
                        write_frame(&mut socket, 1, event.to_string().as_bytes());
                    }
                    if let Ok((opcode, payload)) = read_frame(&mut socket) {
                        assert_eq!(
                            opcode, 8,
                            "a finished attempt never sends another create on its socket"
                        );
                        write_frame(&mut socket, 8, &payload);
                        drain_to_eof(&mut socket)?;
                    }
                    Ok(())
                }
            })
            .collect();
        // Keep the recovery socket available in the post-delta control.
        // A regression would send the second scripted request and succeed.
        let peer = CodexPeer::spawn_many_with_response(
            handlers,
            ws.len() + usize::from(post),
            super::ws_path::sse_response(&sse, 200),
        );
        let provider = ws_provider(peer.base_url());
        let mut call = if warm {
            let mut warmup = request();
            warmup.transport = TransportPreference::WebsocketCached;
            frame_call(&provider, warmup).await;
            let mut call = second_turn_request();
            call.transport = TransportPreference::WebsocketCached;
            call
        } else {
            let mut call = request();
            if ws.is_empty() {
                call.transport = TransportPreference::Sse;
            }
            call
        };
        let full_body = provider.build_request_body(&call).unwrap();
        let raw_events = Arc::new(Mutex::new(Vec::new()));
        let observed_raw = raw_events.clone();
        call.callbacks = RequestCallbacks::new().with_on_raw(Arc::new(move |event| {
            observed_raw
                .lock()
                .unwrap()
                .push(event["type"].as_str().unwrap().to_owned());
            Ok(())
        }));
        let (outcome, public_events, delivered_text) =
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                let mut source = opi_ai::ChatStream::new(provider.stream_prepared(call, auth_ws()));
                let mut events = Vec::new();
                let mut text = String::new();
                while let Some(item) = source.next().await {
                    events.push(match item {
                        Ok(AssistantStreamEvent::Start { .. }) => "start",
                        Ok(AssistantStreamEvent::TextStart { .. }) => "text_start",
                        Ok(AssistantStreamEvent::TextDelta { delta, .. }) => {
                            text.push_str(&delta);
                            "text_delta"
                        }
                        Ok(AssistantStreamEvent::TextEnd { .. }) => "text_end",
                        Ok(AssistantStreamEvent::Done { .. }) => "done",
                        Err(_) => "error",
                        other => panic!("unexpected Codex event: {other:?}"),
                    });
                }
                (source.result().await, events, text)
            })
            .await
            .expect("call terminates within the peer deadline");
        let message = outcome.as_ref().unwrap_or_else(|failure| failure.message());
        let text: String = message
            .content
            .iter()
            .filter_map(|part| match part {
                AssistantContent::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        let (ws_sends, sse_sends) = {
            let capture = peer.capture.lock().unwrap();
            let sent = &capture.frames[usize::from(warm)..];
            let inputs: Vec<_> = sent
                .iter()
                .map(|body| {
                    serde_json::json!({
                        "previous": body["previous_response_id"],
                        "items": body["input"].as_array().unwrap().len(),
                    })
                })
                .collect();
            println!(
                "codex_failed_recovery {}",
                serde_json::json!({
                    "name": name, "ws": sent.len(), "sse": capture.posts.len(), "inputs": inputs,
                    "stop": message.stop_reason, "text": text, "deliveredText": delivered_text,
                    "responseId": message.response_id, "totalTokens": message.usage.total_tokens,
                    "rawStop": message.raw_stop_reason, "publicEvents": public_events,
                    "rawEvents": *raw_events.lock().unwrap(), "diagnostic": message.error_message,
                })
            );
            (sent.len(), capture.posts.len())
        };
        drop(provider);
        if (ws_sends, sse_sends) != (expected_ws, usize::from(post)) {
            mismatches.push(format!("{name}: {ws_sends} WS and {sse_sends} SSE sends"));
            continue;
        }
        assert_eq!(text, row["text"].as_str().unwrap(), "{name}");
        assert_eq!(delivered_text, text, "{name}");
        assert_eq!(
            public_events
                .iter()
                .filter(|&&kind| matches!(kind, "done" | "error"))
                .count(),
            1,
            "{name}"
        );
        if row["stop"] == "error" {
            let failure = outcome.as_ref().expect_err(name);
            assert!(
                matches!(failure.cause(), ProviderError::StreamError(_)),
                "{name}"
            );
            assert_eq!(message.raw_stop_reason.as_deref(), Some("failed"), "{name}");
            assert_eq!(
                classify_whole_call_retry(message),
                WholeCallRetryClassification::Transient,
                "{name}"
            );
            assert!(!format!("{failure:?}").contains("PRIVATE_CANARY"), "{name}");
            if !text.is_empty() {
                assert_eq!(message.response_id.as_deref(), Some("resp_partial"));
            }
        } else {
            assert!(outcome.is_ok(), "{name}");
            assert_eq!(message.stop_reason, StopReason::Stop, "{name}");
            assert_eq!(
                message.response_id.as_deref(),
                Some("resp_recovered"),
                "{name}"
            );
            assert_eq!(
                message.raw_stop_reason.as_deref(),
                Some("completed"),
                "{name}"
            );
            assert_eq!(message.usage.total_tokens, Some(8), "{name}");
            assert!(message.error_message.is_none(), "{name}");
        }
        let capture = peer.finish().expect(name);
        let capture = capture.lock().unwrap();
        assert_eq!(
            capture.frames.len(),
            expected_ws + usize::from(warm),
            "{name}"
        );
        assert_eq!(capture.posts.len(), usize::from(post), "{name}");
        let frames = &capture.frames[usize::from(warm)..];
        if warm {
            assert_eq!(frames[0]["previous_response_id"], "resp_warm", "{name}");
            assert_eq!(frames[0]["input"].as_array().unwrap().len(), 1, "{name}");
        }
        for mut body in frames[usize::from(warm)..].iter().cloned() {
            assert_eq!(
                body.as_object_mut().unwrap().remove("type").unwrap(),
                "response.create"
            );
            assert_eq!(
                body, full_body,
                "{name}: every full WS request uses the original input"
            );
        }
        for body in &capture.posts {
            assert_eq!(body, &full_body, "{name}: SSE uses the original input");
        }
    }
    assert!(
        mismatches.is_empty(),
        "Codex recovery send counts: {mismatches:?}"
    );
}

pub(crate) fn reject_connection_limit(
    mut stream: TcpStream,
    lines: Vec<String>,
    capture: &Arc<Mutex<super::ws_path::PeerCapture>>,
) -> Result<(), String> {
    stream.write_all(&response_101(&lines, &[])).unwrap();
    reject_limit_on_socket(&mut stream, capture)
}

fn record_create(
    stream: &mut TcpStream,
    capture: &Arc<Mutex<super::ws_path::PeerCapture>>,
) -> Result<(), String> {
    let (opcode, body) = read_frame(stream)?;
    assert_eq!(opcode, 1);
    capture
        .lock()
        .unwrap()
        .frames
        .push(serde_json::from_slice(&body).unwrap());
    Ok(())
}

fn reject_limit_on_socket(
    stream: &mut TcpStream,
    capture: &Arc<Mutex<super::ws_path::PeerCapture>>,
) -> Result<(), String> {
    use std::io::Read;

    record_create(stream, capture)?;
    write_frame(
        stream,
        1,
        connection_limit_fixture()["error"].to_string().as_bytes(),
    );
    assert_eq!(
        stream.read(&mut [0; 1]).map_err(|e| e.to_string())?,
        0,
        "release the rejected socket before accepting the next send"
    );
    Ok(())
}

#[tokio::test]
async fn repeated_connection_limit_falls_back_once_in_each_websocket_mode() {
    let fixture = connection_limit_fixture();
    let completion = fixture["completion"].as_array().unwrap();
    let sse: String = completion
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect();
    for (mode, transport, warm) in [
        ("auto", TransportPreference::Auto, false),
        ("websocket", TransportPreference::Websocket, false),
        (
            "websocket-cached",
            TransportPreference::WebsocketCached,
            false,
        ),
        (
            "websocket-cached",
            TransportPreference::WebsocketCached,
            true,
        ),
    ] {
        let handler = |warm| {
            move |mut stream: TcpStream,
                  lines: Vec<String>,
                  capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
                stream.write_all(&response_101(&lines, &[])).unwrap();
                if warm {
                    record_create(&mut stream, capture)?;
                    write_frame(
                        &mut stream,
                        1,
                        connection_limit_fixture()["warmup"].to_string().as_bytes(),
                    );
                }
                reject_limit_on_socket(&mut stream, capture)
            }
        };
        let peer = CodexPeer::spawn_many_with_response(
            vec![handler(warm), handler(false)],
            4,
            super::ws_path::sse_response(&sse, 200),
        );
        let provider = ws_provider(peer.base_url());
        let mut call = if warm {
            let mut warmup = request();
            warmup.transport = transport;
            frame_call(&provider, warmup).await;
            second_turn_request()
        } else {
            request()
        };
        call.transport = transport;
        let order = Arc::new(Mutex::new(Vec::new()));
        let payload_order = order.clone();
        let raw_order = order.clone();
        let response_order = order.clone();
        call.callbacks = RequestCallbacks::new()
            .with_on_payload(Arc::new(move |_| {
                payload_order.lock().unwrap().push("payload".into());
                Ok(None)
            }))
            .with_on_raw(Arc::new(move |frame| {
                raw_order
                    .lock()
                    .unwrap()
                    .push(format!("raw:{}", frame["type"].as_str().unwrap()));
                Ok(())
            }))
            .with_on_response(Arc::new(move |status, _| {
                response_order
                    .lock()
                    .unwrap()
                    .push(format!("response:{status}"));
                Ok(())
            }));
        let (events, first) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let mut stream =
                opi_ai::ChatStream::new(provider.stream_prepared(call.clone(), auth_ws()));
            let mut events = Vec::new();
            while let Some(item) = stream.next().await {
                events.push(match item {
                    Ok(AssistantStreamEvent::TextStart { .. }) => "text_start",
                    Ok(AssistantStreamEvent::TextDelta { .. }) => "text_delta",
                    Ok(AssistantStreamEvent::TextEnd { .. }) => "text_end",
                    Ok(AssistantStreamEvent::Done { .. }) => "done",
                    Err(_) => "error",
                    other => panic!("unexpected Codex event: {other:?}"),
                });
            }
            (events, stream.result().await)
        })
        .await
        .expect("first call terminates within the peer deadline");
        let first_callbacks = order.lock().unwrap().clone();
        {
            let capture = peer.capture.lock().unwrap();
            let first_sends: Vec<_> = capture
                .heads
                .iter()
                .map(|lines| {
                    if super::ws_path::header(lines, "upgrade").is_some() {
                        "websocket"
                    } else {
                        "sse"
                    }
                })
                .collect();
            let message = first.as_ref().unwrap_or_else(|failure| failure.message());
            let text: String = message
                .content
                .iter()
                .filter_map(|part| match part {
                    AssistantContent::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            println!(
                "codex_connection_limit {}",
                serde_json::json!({
                    "transport": mode,
                    "warm": warm,
                    "firstSends": first_sends,
                    "firstCallbacks": first_callbacks,
                    "body": capture.posts.first(),
                    "first": {
                        "stop": message.stop_reason,
                        "text": text,
                        "responseId": message.response_id,
                        "totalTokens": message.usage.total_tokens,
                        "events": events,
                        "diagnostic": message.diagnostics.first(),
                        "error": first.as_ref().err().map(|failure| failure.cause().to_string()),
                    },
                })
            );
            assert_eq!(
                first_sends,
                ["websocket", "websocket", "sse"],
                "{mode}: {first:?}"
            );
            assert_eq!(capture.frames.len(), 2 + usize::from(warm));
            assert_eq!(capture.posts.len(), 1);
        }
        let first = first.expect("SSE finishes the same logical call");
        assert_eq!(events, ["text_start", "text_delta", "text_end", "done"]);
        assert_eq!(first.stop_reason, StopReason::Stop);
        assert_eq!(first.response_id.as_deref(), Some("resp_sse"));
        assert!(matches!(first.content.as_slice(),
            [AssistantContent::Text { text, .. }] if text == "Hello from SSE"));
        assert_eq!(first.usage.total_tokens, Some(8));
        assert!(!format!("{first:?}").contains("PRIVATE_CANARY"));
        let diagnostic = first
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.kind == "codex_websocket_fallback")
            .expect("fixed fallback diagnostic");
        assert_eq!(diagnostic.reason, "websocket_connection_limit_reached");
        assert!(diagnostic.detail.is_none());
        let mut expected_callbacks = vec![
            "payload".to_owned(),
            "raw:error".into(),
            "raw:error".into(),
            "response:200".into(),
        ];
        expected_callbacks.extend(
            completion
                .iter()
                .map(|event| format!("raw:{}", event["type"].as_str().unwrap())),
        );
        assert_eq!(first_callbacks, expected_callbacks);

        let next = typed_terminal(&provider, call, auth_ws()).await;
        assert_eq!(next.response_id, first.response_id);
        assert_eq!(next.stop_reason, first.stop_reason);
        assert_eq!(next.content, first.content);
        assert_eq!(next.usage, first.usage);
        assert!(next.diagnostics.iter().any(|diagnostic| {
            diagnostic.kind == "codex_websocket_fallback" && diagnostic.reason == "session_fallback"
        }));
        let capture = peer.finish().expect("peer script");
        {
            let capture = capture.lock().unwrap();
            let methods: Vec<_> = capture
                .heads
                .iter()
                .map(|lines| lines[0].as_str())
                .collect();
            assert_eq!(
                methods,
                [
                    "GET /codex/responses HTTP/1.1",
                    "GET /codex/responses HTTP/1.1",
                    "POST /codex/responses HTTP/1.1",
                    "POST /codex/responses HTTP/1.1",
                ]
            );
            assert!(capture.heads.iter().all(|lines| {
                super::ws_path::header(lines, "chatgpt-account-id") == Some("acct-1")
            }));
            let first_rejected = usize::from(warm);
            assert_eq!(capture.frames.len(), 2 + first_rejected);
            assert_eq!(capture.posts.len(), 2);
            assert_eq!(capture.posts[0], capture.posts[1]);
            if warm {
                let delta = &capture.frames[first_rejected];
                assert_eq!(delta["previous_response_id"], "resp_warm");
                assert_eq!(delta["input"].as_array().unwrap().len(), 1);
                assert_eq!(capture.posts[0]["input"].as_array().unwrap().len(), 2);
            }
            let first_full = first_rejected + usize::from(warm);
            for mut frame in capture.frames[first_full..].iter().cloned() {
                assert_eq!(
                    frame.as_object_mut().unwrap().remove("type").unwrap(),
                    "response.create"
                );
                assert!(frame.get("previous_response_id").is_none());
                assert_eq!(
                    frame, capture.posts[0],
                    "the SSE POST reuses the full payload"
                );
            }
        }
        let cleanup = provider
            .session_resources()
            .unwrap()
            .cleanup_session_resources(Some("session-fixed"))
            .resolve()
            .await
            .unwrap();
        assert_eq!(cleanup.cleared_fallback_sessions, 1);
        assert_eq!(
            cleanup.closed_connections, 0,
            "both rejected sockets were released"
        );
    }
}

#[tokio::test]
async fn transport_failure_before_events_falls_back_to_sse() {
    let peer = CodexPeer::spawn_many(
        vec![|mut stream: TcpStream,
              lines: Vec<String>,
              _capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
            stream.write_all(&response_101(&lines, &[])).unwrap();
            // Abrupt transport loss before any provider event.
            let _ = read_frame(&mut stream);
            drop(stream);
            Ok(())
        }],
        2,
    );
    let provider = ws_provider(peer.base_url());
    let mut call = request();
    call.transport = TransportPreference::Auto;
    let terminal = typed_terminal(&provider, call, auth_ws()).await;
    assert_eq!(terminal.response_id.as_deref(), Some("response-1"));
    let capture = peer.finish().expect("peer script");
    {
        let guard = capture.lock().unwrap();
        assert_eq!(guard.heads.len(), 2, "one WS attempt then one SSE POST");
        assert!(
            guard.heads[1].first().unwrap().starts_with("POST"),
            "the fallback request is the SSE transport"
        );
        assert_eq!(guard.posts.len(), 1);
    }
}

#[tokio::test]
async fn upgrade_auth_status_is_terminal_without_fallback() {
    let peer = CodexPeer::spawn_many(
        vec![|mut stream: TcpStream,
              _lines: Vec<String>,
              _capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
            let _ = read_frame(&mut stream);
            stream
                .write_all(b"HTTP/1.1 401 Unauthorized\r\ncontent-length: 0\r\n\r\n")
                .unwrap();
            Ok(())
        }],
        1,
    );
    let provider = ws_provider(peer.base_url());
    let mut call = request();
    call.transport = TransportPreference::Websocket;
    let error = drain(&provider, call, auth_ws()).await;
    assert!(
        matches!(error, Some(ProviderError::CredentialRevoked { .. })),
        "401 on upgrade stays a revoked-credential failure, got {error:?}"
    );
    let capture = peer.finish().expect("peer script");
    {
        let guard = capture.lock().unwrap();
        assert!(
            guard.posts.is_empty(),
            "auth failure never masquerades as transport loss"
        );
    }
}

#[tokio::test]
async fn started_failure_is_terminal_and_ends_the_stream() {
    let events = [
        r#"{"type":"response.output_item.added","item":{"type":"message","id":"msg_1","role":"assistant","status":"in_progress","content":[]}}"#,
        r#"{"type":"response.content_part.added","part":{"type":"output_text","text":""}}"#,
        r#"{"type":"response.output_text.delta","delta":"Hello"}"#,
    ];
    let peer = CodexPeer::spawn_many(
        vec![move |mut stream: TcpStream,
                   lines: Vec<String>,
                   _capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
            stream.write_all(&response_101(&lines, &[])).unwrap();
            let (opcode, _) = read_frame(&mut stream)?;
            assert_eq!(opcode, 1);
            // Effective provider events, then abrupt transport loss.
            for event in events {
                write_frame(&mut stream, 1, event.as_bytes());
            }
            let _ = read_frame(&mut stream);
            drop(stream);
            Ok(())
        }],
        1,
    );
    let provider = ws_provider(peer.base_url());
    let mut call = request();
    call.transport = TransportPreference::WebsocketCached;
    let error = drain(&provider, call, auth_ws()).await;
    assert!(
        matches!(
            error,
            Some(ProviderError::StreamError(_)) | Some(ProviderError::StreamFailure(_))
        ),
        "an output-delivered error ends the stream, got {error:?}"
    );
    let capture = peer.finish().expect("peer script");
    assert!(
        capture.lock().unwrap().posts.is_empty(),
        "no SSE splice after output"
    );
}

#[tokio::test]
async fn fallback_is_sticky_for_the_session() {
    let peer = CodexPeer::spawn_many(
        vec![|mut stream: TcpStream,
              lines: Vec<String>,
              _capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
            stream.write_all(&response_101(&lines, &[])).unwrap();
            let _ = read_frame(&mut stream);
            drop(stream);
            Ok(())
        }],
        3,
    );
    let provider = ws_provider(peer.base_url());
    for call_index in 0..2 {
        let mut call = if call_index == 0 {
            request()
        } else {
            second_turn_request()
        };
        call.transport = TransportPreference::Auto;
        let terminal = typed_terminal(&provider, call, auth_ws()).await;
        assert_eq!(terminal.response_id.as_deref(), Some("response-1"));
    }
    let capture = peer.finish().expect("peer script");
    {
        let guard = capture.lock().unwrap();
        assert_eq!(
            guard.heads.len(),
            3,
            "first call: WS + SSE; second call: SSE only"
        );
        assert!(
            guard.heads[1].first().unwrap().starts_with("POST")
                && guard.heads[2].first().unwrap().starts_with("POST"),
            "the sticky marker keeps later calls on SSE"
        );
    }
}

#[tokio::test]
async fn idle_timeout_before_events_falls_back_to_sse() {
    let peer = CodexPeer::spawn_many(
        vec![|mut stream: TcpStream,
              lines: Vec<String>,
              _capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
            stream.write_all(&response_101(&lines, &[])).unwrap();
            // Hold the connection open without sending any event; the
            // hold runs off-thread so the peer can serve the SSE fallback.
            std::thread::spawn(move || {
                let mut held = stream;
                let (opcode, _) = read_frame(&mut held).unwrap_or((0, Vec::new()));
                assert_eq!(opcode, 1);
                std::thread::sleep(std::time::Duration::from_millis(600));
            });
            Ok(())
        }],
        2,
    );
    let provider = ws_provider(peer.base_url());
    let mut call = request();
    call.transport = TransportPreference::WebsocketCached;
    call.timeout = Some(std::time::Duration::from_millis(150));
    let terminal = typed_terminal(&provider, call, auth_ws()).await;
    assert_eq!(terminal.response_id.as_deref(), Some("response-1"));
    let capture = peer.finish().expect("peer script");
    assert_eq!(
        capture.lock().unwrap().posts.len(),
        1,
        "idle timeout falls back once"
    );
}
