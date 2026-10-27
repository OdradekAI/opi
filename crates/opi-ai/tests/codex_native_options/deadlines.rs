use super::*;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(super) async fn read_frame(socket: &mut tokio::net::TcpStream) -> (u8, Vec<u8>) {
    let first = socket.read_u8().await.unwrap();
    let second = socket.read_u8().await.unwrap();
    assert_ne!(second & 0x80, 0);
    let len = match second & 0x7f {
        126 => socket.read_u16().await.unwrap() as usize,
        127 => socket.read_u64().await.unwrap() as usize,
        len => len as usize,
    };
    let mut mask = [0; 4];
    socket.read_exact(&mut mask).await.unwrap();
    let mut bytes = vec![0; len];
    socket.read_exact(&mut bytes).await.unwrap();
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte ^= mask[index % 4];
    }
    (first & 0x0f, bytes)
}

async fn connect_case(option: Option<Duration>, elapsed: Duration, expires: bool) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let peer = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        while !bytes.ends_with(b"\r\n\r\n") {
            bytes.push(socket.read_u8().await.unwrap());
        }
        let head = ws_peer::RequestHead::read(&mut std::io::Cursor::new(bytes));
        ready_tx.send(()).unwrap();
        release_rx.await.unwrap();
        if expires {
            return;
        }
        socket
            .write_all(&ws_peer::response_101(&head, &[]))
            .await
            .unwrap();
        let (opcode, payload) = read_frame(&mut socket).await;
        assert_eq!(opcode, 1);
        let body: Value = serde_json::from_slice(&payload).unwrap();
        assert!(body.get("websocket_connect_timeout").is_none());
        assert!(body.get("websocketConnectTimeoutMs").is_none());
        let payload = completed("connect-deadline").to_string();
        let mut frame = vec![0x81, 126];
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        frame.extend_from_slice(payload.as_bytes());
        socket.write_all(&frame).await.unwrap();
        let (opcode, _) = read_frame(&mut socket).await;
        assert_eq!(opcode, 8);
        socket.write_all(&[0x88, 0]).await.unwrap();
    });
    let provider = Arc::new(OpenAiCodexResponsesProvider::new(
        Some(format!("http://{address}")),
        vec![],
        Arc::new(HttpClient::new()),
    ));
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            Arc::new(CodexChatApi::new(provider, model(false)).unwrap()),
            Arc::new(LocalAuth),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let mut options = CodexOptions::default();
    options.websocket_connect_timeout = option;
    let mut request = request();
    request.transport = TransportPreference::Websocket;
    // This idle budget does not shorten the independent connect phase.
    request.timeout = Some(Duration::from_secs(1));
    let call = models.stream(
        ChatModelRecord::new("openai-codex", model(false)),
        request,
        handle.options(options.clone()),
    );
    options.websocket_connect_timeout = Some(Duration::from_nanos(1));
    assert_eq!(
        options.websocket_connect_timeout,
        Some(Duration::from_nanos(1))
    );
    tokio::time::pause();
    // Keep the runtime runnable so paused time never jumps while real loopback IO waits.
    let keep_awake = tokio::spawn(async {
        loop {
            tokio::task::yield_now().await;
        }
    });
    let task = tokio::spawn(call.result());
    ready_rx.await.unwrap();
    tokio::time::advance(elapsed).await;
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    assert_eq!(
        task.is_finished(),
        expires,
        "connect option {option:?}, elapsed {elapsed:?}"
    );
    release_tx.send(()).unwrap();
    tokio::time::resume();
    keep_awake.abort();
    let result = task.await.unwrap();
    if expires {
        assert!(matches!(
            result.unwrap_err().cause(),
            ProviderError::Timeout
        ));
    } else {
        assert_eq!(
            result.unwrap().response_id.as_deref(),
            Some("connect-deadline")
        );
    }
    models.cleanup_session_resources(None).await.unwrap();
    peer.await.unwrap();
}

#[tokio::test]
async fn default_connect_timeout_has_fifteen_second_boundary() {
    for (millis, expires) in [(14_999, false), (15_001, true)] {
        connect_case(None, Duration::from_millis(millis), expires).await;
    }
}

#[tokio::test]
async fn native_connect_timeout_overrides_default_independently_of_idle_timeout() {
    connect_case(Some(Duration::from_secs(3)), Duration::from_secs(2), false).await;
    connect_case(
        Some(Duration::from_secs(3)),
        Duration::from_millis(3_001),
        true,
    )
    .await;
}

#[tokio::test]
async fn native_zero_connect_timeout_disables_timer() {
    connect_case(Some(Duration::ZERO), Duration::from_secs(16), false).await;
}
