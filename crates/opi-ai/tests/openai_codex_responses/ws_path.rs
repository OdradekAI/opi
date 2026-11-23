fn request() -> Request {
    let mut request = super::request();
    request.transport = opi_ai::provider::TransportPreference::Auto;
    request
}

use super::*;
use opi_ai::http::{HttpClientBuilder, RedirectPolicy};
use opi_ai::provider::TransportPreference;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

// -- scripted local peer ------------------------------------------------

pub(crate) struct PeerCapture {
    pub heads: Vec<Vec<String>>,
    pub frames: Vec<serde_json::Value>,
    pub posts: Vec<serde_json::Value>,
}

pub(crate) struct CodexPeer {
    pub port: u16,
    pub capture: Arc<Mutex<PeerCapture>>,
    pub(crate) handle: Option<std::thread::JoinHandle<Result<(), String>>>,
}

pub(crate) fn accept_token_for(key: &str) -> String {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(key.as_bytes())
        .unwrap();
    let mut nonce = [0_u8; 16];
    nonce.copy_from_slice(&bytes);
    opi_ai::ws::accept_token(&nonce)
}

/// Byte-wise head reader: no internal buffering, so request bodies that
/// arrive with the head are never consumed ahead of the caller.
pub(crate) fn read_head(stream: &mut TcpStream) -> Vec<String> {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let mut lines = Vec::new();
    let mut line = Vec::new();
    let mut byte = [0; 1];
    loop {
        match stream.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                if byte[0] == b'\n' {
                    if line == b"\r" || line.is_empty() {
                        break;
                    }
                    lines.push(String::from_utf8_lossy(&line).trim_end().to_owned());
                    line.clear();
                } else {
                    line.push(byte[0]);
                }
            }
            Err(_error) => break,
        }
    }
    lines
}

pub(crate) fn header<'a>(lines: &'a [String], name: &str) -> Option<&'a str> {
    lines
        .iter()
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.trim())
}

pub(crate) fn response_101(lines: &[String], extra: &[(&str, &str)]) -> Vec<u8> {
    let accept = accept_token_for(header(lines, "sec-websocket-key").unwrap());
    let mut response = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n"
    );
    for (name, value) in extra {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str("\r\n");
    response.into_bytes()
}

pub(crate) fn sse_response(events: &str, status: u16) -> Vec<u8> {
    let reason = if status == 200 { "OK" } else { "Bad Request" };
    format!("HTTP/1.1 {status} {reason}\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{events}", events.len()).into_bytes()
}

/// Read one masked client frame; returns (opcode, payload).
pub(crate) fn read_frame(stream: &mut TcpStream) -> Result<(u8, Vec<u8>), String> {
    read_frame_with_compression(stream).map(|(opcode, payload, _)| (opcode, payload))
}

pub(crate) fn read_frame_with_compression(
    stream: &mut TcpStream,
) -> Result<(u8, Vec<u8>, bool), String> {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let mut head = [0; 2];
    stream.read_exact(&mut head).map_err(|e| e.to_string())?;
    let opcode = head[0] & 0x0f;
    let masked = head[1] & 0x80 != 0;
    let len = match head[1] & 0x7f {
        126 => {
            let mut extended = [0; 2];
            stream
                .read_exact(&mut extended)
                .map_err(|e| e.to_string())?;
            u16::from_be_bytes(extended) as usize
        }
        127 => {
            let mut extended = [0; 8];
            stream
                .read_exact(&mut extended)
                .map_err(|e| e.to_string())?;
            u64::from_be_bytes(extended) as usize
        }
        value => value as usize,
    };
    let mut mask = [0; 4];
    if masked {
        stream.read_exact(&mut mask).map_err(|e| e.to_string())?;
    }
    let mut payload = vec![0; len];
    stream.read_exact(&mut payload).map_err(|e| e.to_string())?;
    if masked {
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[i % 4];
        }
    }
    Ok((opcode, payload, head[0] & 0x40 != 0))
}

/// Write one unmasked server frame.
pub(crate) fn write_frame(stream: &mut impl Write, opcode: u8, payload: &[u8]) {
    let mut frame = vec![0x80 | opcode];
    match payload.len() {
        0..=125 => frame.push(payload.len() as u8),
        126..=65535 => {
            frame.push(126);
            frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        }
        length => {
            frame.push(127);
            frame.extend_from_slice(&(length as u64).to_be_bytes());
        }
    }
    frame.extend_from_slice(payload);
    stream.write_all(&frame).unwrap();
    stream.flush().unwrap();
}

pub(crate) fn drain_to_eof(stream: &mut TcpStream) -> Result<usize, String> {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let mut total = 0;
    let mut buffer = [0; 512];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => return Ok(total),
            Ok(n) => total += n,
            Err(error) => {
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut
                {
                    return Ok(total);
                }
                return Err(error.to_string());
            }
        }
    }
}

pub(crate) fn completed_event() -> String {
    "{\"type\":\"response.completed\",\"response\":{\"id\":\"response-1\",\"model\":\"gpt-5.4\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}".to_owned()
}

impl CodexPeer {
    /// Spawn a peer thread running `handler` once per accepted connection.
    pub(crate) fn spawn<H>(handler: H) -> Self
    where
        H: Fn(TcpStream, Vec<String>, &Arc<Mutex<PeerCapture>>) -> Result<(), String>
            + Send
            + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let capture = Arc::new(Mutex::new(PeerCapture {
            heads: Vec::new(),
            frames: Vec::new(),
            posts: Vec::new(),
        }));
        let shared = capture.clone();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().map_err(|e| e.to_string())?;
            stream.set_nodelay(true).unwrap();
            let lines = read_head(&mut stream);
            shared.lock().unwrap().heads.push(lines.clone());
            if header(&lines, "upgrade").is_some() {
                handler(stream, lines, &shared)
            } else {
                // POST: SSE-style response; record the JSON body.
                let length = header(&lines, "content-length")
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(0);
                let mut body = vec![0; length];
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                stream.read_exact(&mut body).map_err(|e| e.to_string())?;
                let value: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
                shared.lock().unwrap().posts.push(value);
                handler(stream, lines, &shared)
            }
        });
        CodexPeer {
            port,
            capture,
            handle: Some(handle),
        }
    }

    pub(crate) fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub(crate) fn finish(mut self) -> Result<Arc<Mutex<PeerCapture>>, String> {
        self.handle
            .take()
            .expect("peer thread")
            .join()
            .map_err(|_| "server thread panicked".to_string())??;
        Ok(self.capture)
    }
}

fn none_redirect_client() -> Arc<opi_ai::http::HttpClient> {
    Arc::new(
        HttpClientBuilder::new()
            .redirects(RedirectPolicy::None)
            .build()
            .expect("client"),
    )
}

// -- transport resolution -------------------------------------------------

#[tokio::test]
async fn default_auto_uses_real_websocket_path() {
    let peer = CodexPeer::spawn(
        |mut stream: TcpStream,
         lines: Vec<String>,
         capture: &Arc<Mutex<super::ws_path::PeerCapture>>| {
            assert_eq!(
                header(&lines, "openai-beta").unwrap(),
                "responses_websockets=2026-02-06",
                "WS handshake must carry the websocket beta header"
            );
            assert_eq!(header(&lines, "authorization").unwrap(), "Bearer ws-secret");
            assert_eq!(header(&lines, "chatgpt-account-id").unwrap(), "acct-1");
            assert_eq!(header(&lines, "originator").unwrap(), "opi");
            assert_eq!(header(&lines, "session-id").unwrap(), "session-fixed");
            assert_eq!(
                header(&lines, "x-client-request-id").unwrap(),
                "session-fixed"
            );
            assert!(
                header(&lines, "content-type").is_none(),
                "WS handshake drops the SSE content-type header"
            );
            stream.write_all(&response_101(&lines, &[])).unwrap();
            let (opcode, payload) = read_frame(&mut stream)?;
            assert_eq!(opcode, 1, "response.create travels as a text frame");
            let frame: serde_json::Value = serde_json::from_slice(&payload).unwrap();
            capture.lock().unwrap().frames.push(frame);
            write_frame(&mut stream, 1, completed_event().as_bytes());
            // Auto with an active session keeps the connection pooled: no
            // Close handshake; the socket closes silently at provider drop.
            drain_to_eof(&mut stream)?;
            Ok(())
        },
    );
    let provider = OpenAiCodexResponsesProvider::new(
        Some(peer.base_url()),
        vec![model(None)],
        none_redirect_client(),
    );
    let terminal = typed_terminal(
        &provider,
        request(),
        auth("ws-secret", None, Some("acct-1".into())),
    )
    .await;
    assert_eq!(terminal.response_id.as_deref(), Some("response-1"));
    let capture = peer.finish().expect("peer script");
    let frame = capture.lock().unwrap().frames[0].clone();
    assert_eq!(frame["type"], "response.create");
    assert_eq!(frame["model"], "gpt-5.4");
    assert_eq!(frame["store"], false);
    assert_eq!(frame["stream"], true);
    assert_eq!(frame["prompt_cache_key"], "session-fixed");
    assert!(!frame["input"].as_array().unwrap().is_empty());
    // ONB-03: the transport preference is a typed transport-layer hint
    // only; it never leaks into the wire body under any mode.
    assert!(
        frame.get("transport").is_none(),
        "transport preference must not appear in the request body"
    );
}

#[tokio::test]
async fn explicit_sse_never_attempts_websocket() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/codex/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(codex_completed_sse()),
        )
        .mount(&server)
        .await;
    // A listener that fails any WS upgrade attempt.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let guard = tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let mut buffer = [0; 512];
            use tokio::io::AsyncReadExt;
            let _ = socket.read(&mut buffer).await;
            use tokio::io::AsyncWriteExt;
            let _ = socket
                .write_all(b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 0\r\n\r\n")
                .await;
        }
    });
    let provider = OpenAiCodexResponsesProvider::new(
        Some(server.uri()),
        vec![model(None)],
        Arc::new(HttpClient::new()),
    );
    let mut call = request();
    call.transport = TransportPreference::Sse;
    let terminal =
        typed_terminal(&provider, call, auth("sse-secret", None, Some("a".into()))).await;
    assert_eq!(terminal.response_id.as_deref(), Some("response-1"));
    guard.abort();
}

#[tokio::test]
async fn explicit_websocket_reuses_connection_without_continuation() {
    let peer = CodexPeer::spawn(
        |mut stream: TcpStream, lines: Vec<String>, capture: &Arc<Mutex<PeerCapture>>| {
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
            // Pooled connection: no Close handshake; the socket closes
            // silently at provider drop.
            drain_to_eof(&mut stream)?;
            Ok(())
        },
    );
    let provider = OpenAiCodexResponsesProvider::new(
        Some(peer.base_url()),
        vec![model(None)],
        none_redirect_client(),
    );
    for call in 0..2 {
        let mut call_request = request();
        call_request.transport = TransportPreference::Websocket;
        let mut stream =
            provider.stream_prepared(call_request, auth("ws-secret", None, Some("acct-1".into())));
        let mut saw_done = false;
        while let Some(result) = stream.next().await {
            if let AssistantStreamEvent::Done { message, .. } =
                result.expect("scripted Codex response must stream successfully")
            {
                assert_eq!(message.response_id.as_deref(), Some("response-1"));
                saw_done = true;
            }
        }
        assert!(saw_done, "call {call} must finish with Done");
    }
    let capture = peer.finish().expect("peer script");
    {
        let guard = capture.lock().unwrap();
        assert_eq!(
            guard.heads.len(),
            1,
            "non-cached websocket still pools the idle connection"
        );
        assert_eq!(guard.frames.len(), 2);
        for frame in &guard.frames {
            assert!(
                frame.get("previous_response_id").is_none(),
                "non-cached websocket never sends continuation deltas"
            );
            assert_eq!(frame["input"].as_array().unwrap().len(), 1);
        }
    }
}

#[tokio::test]
async fn websocket_producer_capture_persist_reload_replays_opaque_reasoning_and_dual_ids() {
    use opi_ai::message::{OutputContent, ToolResultMessage};
    use opi_ai::{ChatModelRecord, Models};

    struct ReplayAuth;
    impl opi_ai::auth::AuthResolver for ReplayAuth {
        fn resolve(
            &self,
        ) -> opi_ai::credential::BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
            Box::pin(async { Ok(auth("fixture-ws-key", None, Some("fixture-account".into()))) })
        }
    }

    let peer = CodexPeer::spawn(|mut socket, lines, capture| {
        assert_eq!(
            header(&lines, "authorization"),
            Some("Bearer fixture-ws-key")
        );
        assert_eq!(
            header(&lines, "chatgpt-account-id"),
            Some("fixture-account")
        );
        socket.write_all(&response_101(&lines, &[])).unwrap();
        let (opcode, payload) = read_frame(&mut socket)?;
        assert_eq!(opcode, 1);
        capture
            .lock()
            .unwrap()
            .frames
            .push(serde_json::from_slice(&payload).unwrap());
        let reasoning = serde_json::json!({
            "type": "reasoning", "id": "rs-ws-captured",
            "summary": [{"type": "summary_text", "text": "Plan from the WS producer"}],
            "encrypted_content": "OPAQUE-WS-REASONING",
        });
        let tool = serde_json::json!({
            "type": "function_call", "id": "fc_ws_captured", "call_id": "call-ws-captured",
            "name": "read", "arguments": "{\"path\":\"a.rs\"}", "status": "completed",
        });
        for event in [
            serde_json::json!({"type": "response.created", "response": {"id": "ws-captured-response", "model": "reported-gpt-5.4-revision", "status": "in_progress", "output": []}}),
            serde_json::json!({"type": "response.output_item.added", "output_index": 0, "item": {"type": "reasoning", "id": "rs-ws-captured"}}),
            serde_json::json!({"type": "response.output_item.done", "output_index": 0, "item": reasoning}),
            serde_json::json!({"type": "response.output_item.added", "output_index": 1, "item": {"type": "function_call", "id": "fc_ws_captured", "call_id": "call-ws-captured", "name": "read", "arguments": ""}}),
            serde_json::json!({"type": "response.function_call_arguments.delta", "output_index": 1, "item_id": "fc_ws_captured", "delta": "{\"path\":\"a.rs\"}"}),
            serde_json::json!({"type": "response.function_call_arguments.done", "output_index": 1, "item_id": "fc_ws_captured", "arguments": "{\"path\":\"a.rs\"}"}),
            serde_json::json!({"type": "response.output_item.done", "output_index": 1, "item": tool}),
            serde_json::json!({"type": "response.completed", "response": {"id": "ws-captured-response", "model": "reported-gpt-5.4-revision", "status": "completed", "output": [reasoning, tool], "usage": {"input_tokens": 7, "output_tokens": 3}}}),
        ] {
            write_frame(&mut socket, 1, event.to_string().as_bytes());
        }
        let (opcode, payload) = read_frame(&mut socket)?;
        assert_eq!(opcode, 1);
        capture
            .lock()
            .unwrap()
            .frames
            .push(serde_json::from_slice(&payload).unwrap());
        write_frame(&mut socket, 1, completed_event().as_bytes());
        let (opcode, payload) = read_frame(&mut socket)?;
        assert_eq!(opcode, 8, "Models cleanup closes the pooled WS connection");
        write_frame(&mut socket, 8, &payload);
        Ok(())
    });
    let mut models = Models::new();
    models
        .register_route(
            Box::new(OpenAiCodexResponsesProvider::new(
                Some(peer.base_url()),
                vec![model(None)],
                none_redirect_client(),
            )),
            Arc::new(ReplayAuth),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let record = ChatModelRecord::new("openai-codex", model(None));
    let mut first_request = request();
    // Explicit non-cached WS requires full replay input on the next call.
    first_request.transport = TransportPreference::Websocket;
    let initial = first_request.context.clone();
    let mut first = models.stream_simple(record.clone(), first_request, Default::default());
    let mut captured = None;
    while let Some(event) = first.next().await {
        assert!(captured.is_none(), "the first terminal is absorbing");
        if let AssistantStreamEvent::Done { reason, message } = event.unwrap() {
            assert_eq!(reason, StopReason::ToolUse);
            captured = Some(message);
        }
    }
    assert!(first.next().await.is_none());
    let captured = captured.expect("actual WS producer terminal");
    assert_eq!(captured.provider, "openai-codex");
    assert_eq!(captured.model, "gpt-5.4");
    assert_eq!(captured.wire, Some(WireApi::OpenAiCodexResponses));
    assert_eq!(
        captured.response_model.as_deref(),
        Some("reported-gpt-5.4-revision")
    );
    assert_eq!(
        captured.response_id.as_deref(),
        Some("ws-captured-response")
    );
    assert_eq!(captured.usage.input_tokens, 7);
    assert_eq!(captured.content.len(), 2);
    let AssistantContent::Thinking {
        thinking,
        signature,
    } = &captured.content[0]
    else {
        panic!("producer reasoning block")
    };
    assert_eq!(thinking, "Plan from the WS producer");
    let signature: serde_json::Value = serde_json::from_str(signature.as_deref().unwrap()).unwrap();
    assert_eq!(signature["id"], "rs-ws-captured");
    assert_eq!(signature["encrypted_content"], "OPAQUE-WS-REASONING");
    let AssistantContent::ToolCall { tool_call } = &captured.content[1] else {
        panic!("producer function call")
    };
    assert_eq!(tool_call.id, "call-ws-captured|fc_ws_captured");
    let tool_result = ToolResultMessage {
        tool_call_id: tool_call.id.clone(),
        tool_name: tool_call.name.clone(),
        content: vec![OutputContent::Text {
            text: "fixture contents".into(),
        }],
        details: None,
        is_error: false,
        not_executed: None,
        truncated: false,
        timestamp_ms: 0,
    };
    let directory = tempfile::tempdir().unwrap();
    let stored = directory.path().join("assistant.json");
    let encoded = serde_json::to_vec(&captured).unwrap();
    std::fs::write(&stored, &encoded).unwrap();
    let reloaded: AssistantMessage =
        serde_json::from_slice(&std::fs::read(&stored).unwrap()).unwrap();
    assert_eq!(reloaded, captured);
    let mut history = initial.messages().to_vec();
    history.push(Message::Assistant(reloaded.clone()));
    history.push(Message::ToolResult(tool_result));
    let mut replay = request();
    replay.transport = TransportPreference::Websocket;
    replay.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: history,
        tools: vec![],
    })
    .unwrap();
    let second = models
        .complete_simple(record, replay, Default::default())
        .await
        .unwrap();
    assert_eq!(second.stop_reason, StopReason::Stop);
    models.cleanup_session_resources(None).await.unwrap();
    let capture = peer.finish().expect("WS capture/reload peer");
    let capture = capture.lock().unwrap();
    assert_eq!(capture.heads.len(), 1);
    assert!(capture.posts.is_empty(), "both generations must use WS");
    assert_eq!(capture.frames.len(), 2);
    for frame in &capture.frames {
        assert_eq!(frame["type"], "response.create");
        assert!(frame.get("previous_response_id").is_none());
    }
    let input = capture.frames[1]["input"].as_array().unwrap();
    let reasoning: Vec<_> = input
        .iter()
        .filter(|item| item["type"] == "reasoning")
        .collect();
    assert_eq!(reasoning.len(), 1);
    assert_eq!(reasoning[0]["id"], "rs-ws-captured");
    assert_eq!(reasoning[0]["encrypted_content"], "OPAQUE-WS-REASONING");
    let tool: Vec<_> = input
        .iter()
        .filter(|item| item["type"] == "function_call")
        .collect();
    assert_eq!(tool.len(), 1);
    assert_eq!(tool[0]["id"], "fc_ws_captured");
    assert_eq!(tool[0]["call_id"], "call-ws-captured");
    assert_eq!(tool[0]["arguments"], "{\"path\":\"a.rs\"}");
    let output: Vec<_> = input
        .iter()
        .filter(|item| item["type"] == "function_call_output")
        .collect();
    assert_eq!(output.len(), 1);
    assert_eq!(output[0]["call_id"], "call-ws-captured");
    assert_eq!(output[0]["output"], "fixture contents");
    assert_eq!(serde_json::to_vec(&reloaded).unwrap(), encoded);
}
