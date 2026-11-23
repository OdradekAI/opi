fn request() -> Request {
    let mut request = super::request();
    request.transport = opi_ai::provider::TransportPreference::Auto;
    request
}

use super::ws_path::{
    CodexPeer, completed_event, drain_to_eof, read_frame, response_101, write_frame,
};
use super::*;
use opi_ai::http::{HttpClientBuilder, RedirectPolicy};
use opi_ai::provider::TransportPreference;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

impl CodexPeer {
    /// Spawn a peer accepting connections until `total` arrive or 10s of
    /// idle elapse. WebSocket connections consume the next handler;
    /// plain POST requests are answered with the shared SSE corpus.
    pub(crate) fn spawn_many<H>(handlers: Vec<H>, total: usize) -> Self
    where
        H: Fn(
                TcpStream,
                Vec<String>,
                &Arc<Mutex<super::ws_path::PeerCapture>>,
            ) -> Result<(), String>
            + Send
            + 'static,
    {
        Self::spawn_many_with_response(
            handlers,
            total,
            super::ws_path::sse_response(&codex_completed_sse(), 200),
        )
    }

    pub(crate) fn spawn_many_with_response<H>(
        handlers: Vec<H>,
        total: usize,
        response: Vec<u8>,
    ) -> Self
    where
        H: Fn(
                TcpStream,
                Vec<String>,
                &Arc<Mutex<super::ws_path::PeerCapture>>,
            ) -> Result<(), String>
            + Send
            + 'static,
    {
        Self::spawn_many_until(handlers, total, response, || false)
    }

    pub(crate) fn spawn_many_until<H>(
        handlers: Vec<H>,
        total: usize,
        response: Vec<u8>,
        finished: impl Fn() -> bool + Send + 'static,
    ) -> Self
    where
        H: Fn(
                TcpStream,
                Vec<String>,
                &Arc<Mutex<super::ws_path::PeerCapture>>,
            ) -> Result<(), String>
            + Send
            + 'static,
    {
        use std::collections::VecDeque;
        use std::io::{Read, Write};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener
            .set_nonblocking(true)
            .expect("nonblocking listener");
        let port = listener.local_addr().unwrap().port();
        let capture = Arc::new(Mutex::new(super::ws_path::PeerCapture {
            heads: Vec::new(),
            frames: Vec::new(),
            posts: Vec::new(),
        }));
        let shared = capture.clone();
        let mut handlers: VecDeque<H> = handlers.into();
        let handle = std::thread::spawn(move || {
            let mut accepted = 0;
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while accepted < total && !finished() {
                if std::time::Instant::now() > deadline {
                    break;
                }
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        accepted += 1;
                        // Windows accepted sockets inherit nonblocking mode.
                        // Every scripted read below expects a blocking socket.
                        stream.set_nonblocking(false).unwrap();
                        stream.set_nodelay(true).unwrap();
                        let lines = super::ws_path::read_head(&mut stream);
                        shared.lock().unwrap().heads.push(lines.clone());
                        if super::ws_path::header(&lines, "upgrade").is_some() {
                            let handler = handlers
                                .pop_front()
                                .ok_or("websocket connection without handler")?;
                            handler(stream, lines, &shared)?;
                        } else {
                            let length = super::ws_path::header(&lines, "content-length")
                                .and_then(|v| v.parse::<usize>().ok())
                                .unwrap_or(0);
                            let mut body = vec![0; length];
                            stream
                                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                                .unwrap();
                            stream.read_exact(&mut body).map_err(|e| e.to_string())?;
                            let value: serde_json::Value =
                                serde_json::from_slice(&body).unwrap_or_default();
                            shared.lock().unwrap().posts.push(value);
                            stream.write_all(&response).map_err(|e| e.to_string())?;
                            let _ = stream.shutdown(std::net::Shutdown::Both);
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(error) => return Err(error.to_string()),
                }
            }
            Ok(())
        });
        CodexPeer {
            port,
            capture,
            handle: Some(handle),
        }
    }
}

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

/// A second-turn request: same system prompt and tools, one more user
/// message than the shared `request()` helper.
pub(crate) fn second_turn_request() -> Request {
    let mut request = request();
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: Some("system prompt".into()),
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "hello".into(),
                }],
                timestamp_ms: 0,
            }),
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "second turn".into(),
                }],
                timestamp_ms: 0,
            }),
        ],
        tools: vec![ToolDef {
            name: "read".into(),
            description: "Read a file".into(),
            input_schema: serde_json::json!({"type":"object"}),
            constrained_sampling: None,
        }],
    })
    .expect("valid context declarations");
    request
}

pub(crate) async fn frame_call(provider: &OpenAiCodexResponsesProvider, request: Request) {
    // Drain to stream end: the producer finishing guarantees pooled
    // connections are released before the next call acquires them.
    let mut stream =
        provider.stream_prepared(request, auth("ws-secret", None, Some("acct-1".into())));
    let mut saw_done = false;
    while let Some(result) = stream.next().await {
        if let AssistantStreamEvent::Done { .. } =
            result.expect("scripted Codex response must stream successfully")
        {
            saw_done = true;
        }
    }
    assert!(saw_done, "stream must finish with Done");
}

#[tokio::test]
async fn cached_continuation_sends_delta_over_one_pooled_connection() {
    let peer = CodexPeer::spawn_many(
        vec![|mut stream: TcpStream,
              lines: Vec<String>,
              capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
            stream.write_all(&response_101(&lines, &[])).unwrap();
            let (opcode, payload) = read_frame(&mut stream)?;
            assert_eq!(opcode, 1);
            let first: serde_json::Value = serde_json::from_slice(&payload).unwrap();
            capture.lock().unwrap().frames.push(first);
            let terminal = serde_json::json!({
                "type": "response.done",
                "response": {
                    "id": "response-1", "status": "completed", "end_turn": false,
                    "usage": {"input_tokens": 1, "output_tokens": 1},
                },
            });
            write_frame(&mut stream, 1, terminal.to_string().as_bytes());
            // The pooled connection stays open; the second request reuses it.
            let (opcode, payload) = read_frame(&mut stream)?;
            assert_eq!(opcode, 1);
            let second: serde_json::Value = serde_json::from_slice(&payload).unwrap();
            capture.lock().unwrap().frames.push(second);
            write_frame(&mut stream, 1, completed_event().as_bytes());
            // No Close handshake: the connection returns to the pool and
            // closes silently when the provider is dropped.
            drain_to_eof(&mut stream)?;
            Ok(())
        }],
        1,
    );
    let provider = ws_provider(peer.base_url());
    let mut first = request();
    first.transport = TransportPreference::WebsocketCached;
    let first = typed_terminal(
        &provider,
        first,
        auth("ws-secret", None, Some("acct-1".into())),
    )
    .await;
    assert_eq!(first.end_turn(), Some(false));
    let mut second = second_turn_request();
    second.transport = TransportPreference::WebsocketCached;
    frame_call(&provider, second).await;
    let capture = peer.finish().expect("peer script");
    {
        let guard = capture.lock().unwrap();
        assert_eq!(guard.heads.len(), 1, "both calls share one connection");
        let first = &guard.frames[0];
        let second = &guard.frames[1];
        assert!(
            first.get("previous_response_id").is_none(),
            "first request is full input"
        );
        assert_eq!(first["input"].as_array().unwrap().len(), 1);
        assert_eq!(
            second["previous_response_id"], "response-1",
            "second request continues from the published response id"
        );
        let delta = second["input"].as_array().unwrap();
        assert_eq!(delta.len(), 1, "delta input carries only the new turn");
        assert_eq!(
            delta[0]["content"], "second turn",
            "the delta item is exactly the new turn's input item"
        );
    }
}

#[tokio::test]
async fn cached_continuation_falls_back_to_full_input_on_noninput_change() {
    let peer = CodexPeer::spawn_many(
        vec![|mut stream: TcpStream,
              lines: Vec<String>,
              capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
            stream.write_all(&response_101(&lines, &[])).unwrap();
            for _turn in 0..2 {
                let (opcode, payload) = read_frame(&mut stream)?;
                assert_eq!(opcode, 1);
                capture
                    .lock()
                    .unwrap()
                    .frames
                    .push(serde_json::from_slice(&payload).unwrap());
                write_frame(&mut stream, 1, completed_event().as_bytes());
            }
            // No Close handshake: the pooled connection is reused.
            drain_to_eof(&mut stream)?;
            Ok(())
        }],
        1,
    );
    let provider = ws_provider(peer.base_url());
    let mut first = request();
    first.transport = TransportPreference::WebsocketCached;
    frame_call(&provider, first).await;
    let mut second = second_turn_request();
    second.transport = TransportPreference::WebsocketCached;
    second.temperature = Some(0.7);
    frame_call(&provider, second).await;
    let capture = peer.finish().expect("peer script");
    {
        let guard = capture.lock().unwrap();
        let second = &guard.frames[1];
        assert!(
            second.get("previous_response_id").is_none(),
            "changed non-input fields force full input"
        );
        assert_eq!(second["input"].as_array().unwrap().len(), 2);
    }
}

#[tokio::test]
async fn continuation_not_published_without_response_id() {
    let idless =
        "{\"type\":\"response.completed\",\"response\":{\"model\":\"gpt-5.4\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}".to_owned();
    let peer = CodexPeer::spawn_many(
        vec![move |mut stream: TcpStream,
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
                let event = if turn == 0 {
                    idless.as_bytes().to_vec()
                } else {
                    completed_event().into_bytes()
                };
                write_frame(&mut stream, 1, &event);
            }
            // No Close handshake: the pooled connection is reused.
            drain_to_eof(&mut stream)?;
            Ok(())
        }],
        1,
    );
    let provider = ws_provider(peer.base_url());
    let mut first = request();
    first.transport = TransportPreference::WebsocketCached;
    // The id-less completion still finishes the call...
    let mut stream =
        provider.stream_prepared(first, auth("ws-secret", None, Some("acct-1".into())));
    while let Some(result) = stream.next().await {
        assert!(result.is_ok(), "id-less completion must still stream");
    }
    // ...but publishes no continuation: the next request is full input.
    let mut second = second_turn_request();
    second.transport = TransportPreference::WebsocketCached;
    frame_call(&provider, second).await;
    let capture = peer.finish().expect("peer script");
    {
        let guard = capture.lock().unwrap();
        assert!(guard.frames[1].get("previous_response_id").is_none());
        assert_eq!(guard.frames[1]["input"].as_array().unwrap().len(), 2);
    }
}

#[tokio::test]
async fn sessionless_requests_use_temporary_connections() {
    let peer = CodexPeer::spawn_many(
        vec![
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
                assert_eq!(read_frame(&mut stream)?.0, 8);
                write_frame(&mut stream, 8, &[]);
                drain_to_eof(&mut stream)?;
                Ok(())
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
                assert_eq!(read_frame(&mut stream)?.0, 8);
                write_frame(&mut stream, 8, &[]);
                drain_to_eof(&mut stream)?;
                Ok(())
            },
        ],
        2,
    );
    let provider = ws_provider(peer.base_url());
    for turn in 0..2 {
        let mut call = if turn == 0 {
            request()
        } else {
            second_turn_request()
        };
        call.cache_retention = CacheRetention::Disabled;
        call.transport = TransportPreference::WebsocketCached;
        frame_call(&provider, call).await;
    }
    let capture = peer.finish().expect("peer script");
    {
        let guard = capture.lock().unwrap();
        assert_eq!(guard.heads.len(), 2, "sessionless calls never pool");
        for frame in &guard.frames {
            assert!(frame.get("previous_response_id").is_none());
        }
    }
}
