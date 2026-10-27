use super::deadlines::read_frame;
use super::*;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type Creates = Arc<Mutex<Vec<(usize, Value)>>>;

struct PoolPeer {
    models: Models,
    creates: Creates,
    closed: Arc<AtomicUsize>,
    stop: CancellationToken,
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
    task: tokio::task::JoinHandle<()>,
    keep_awake: tokio::task::JoinHandle<()>,
}

impl PoolPeer {
    async fn new(hold_second: bool) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let creates: Creates = Arc::default();
        let closed = Arc::new(AtomicUsize::new(0));
        let stop = CancellationToken::new();
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let task = tokio::spawn({
            let (creates, closed, stop, entered, release) = (
                creates.clone(),
                closed.clone(),
                stop.clone(),
                entered.clone(),
                release.clone(),
            );
            async move {
                let mut workers = tokio::task::JoinSet::new();
                let mut connection = 0;
                loop {
                    let socket = tokio::select! {
                        _ = stop.cancelled() => break,
                        next = listener.accept() => next.unwrap().0,
                    };
                    connection += 1;
                    let (creates, closed, entered, release) = (
                        creates.clone(),
                        closed.clone(),
                        entered.clone(),
                        release.clone(),
                    );
                    workers.spawn(async move {
                        let mut socket = socket;
                        let mut bytes = Vec::new();
                        while !bytes.ends_with(b"\r\n\r\n") {
                            bytes.push(socket.read_u8().await.unwrap());
                        }
                        let head = ws_peer::RequestHead::read(&mut std::io::Cursor::new(bytes));
                        socket
                            .write_all(&ws_peer::response_101(&head, &[]))
                            .await
                            .unwrap();
                        loop {
                            let (opcode, payload) = read_frame(&mut socket).await;
                            if opcode == 8 {
                                socket.write_all(&[0x88, 0]).await.unwrap();
                                closed.fetch_add(1, Ordering::SeqCst);
                                break;
                            }
                            assert_eq!(opcode, 1);
                            let count = {
                                let mut frames = creates.lock().unwrap();
                                frames
                                    .push((connection, serde_json::from_slice(&payload).unwrap()));
                                frames.len()
                            };
                            if hold_second && count == 2 {
                                entered.notify_one();
                                release.notified().await;
                            }
                            let payload = completed(&format!("response-{count}")).to_string();
                            let mut frame = vec![0x81, 126];
                            frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
                            frame.extend_from_slice(payload.as_bytes());
                            socket.write_all(&frame).await.unwrap();
                        }
                    });
                }
                while let Some(worker) = workers.join_next().await {
                    worker.unwrap();
                }
            }
        });
        let provider = Arc::new(OpenAiCodexResponsesProvider::new(
            Some(format!("http://{address}")),
            vec![],
            Arc::new(HttpClient::new()),
        ));
        let mut models = Models::new();
        models
            .register_chat_api(
                Arc::new(CodexChatApi::new(provider, model(false)).unwrap()),
                Arc::new(LocalAuth),
                AuthProvenanceSource::Static,
            )
            .unwrap();
        tokio::time::pause();
        let keep_awake = tokio::spawn(async {
            loop {
                tokio::task::yield_now().await;
            }
        });
        Self {
            models,
            creates,
            closed,
            stop,
            entered,
            release,
            task,
            keep_awake,
        }
    }

    fn call(&self) -> opi_ai::ChatCall {
        let mut request = request();
        request.transport = TransportPreference::WebsocketCached;
        request.session_id = Some("pool-lifetime".into());
        request.cache_retention = opi_ai::provider::CacheRetention::Short;
        self.models.stream_simple(
            ChatModelRecord::new("openai-codex", model(false)),
            request,
            SimpleOptions::default(),
        )
    }

    async fn finish(self) {
        let report = self.models.cleanup_session_resources(None).await.unwrap();
        assert!(report.unconfirmed_effects.is_empty());
        self.stop.cancel();
        self.task.await.unwrap();
        tokio::time::resume();
        self.keep_awake.abort();
        let connections = self
            .creates
            .lock()
            .unwrap()
            .iter()
            .map(|(id, _)| *id)
            .max()
            .unwrap();
        assert_eq!(
            self.closed.load(Ordering::SeqCst),
            connections,
            "every local connection exited after cleanup"
        );
    }
}

async fn advance(duration: Duration) {
    tokio::time::advance(duration).await;
}

#[tokio::test]
async fn idle_pool_expires_on_next_acquire_at_five_minutes() {
    for (millis, expired) in [(299_999, false), (300_000, true), (300_001, true)] {
        let peer = PoolPeer::new(false).await;
        drain(peer.call()).await;
        advance(Duration::from_millis(millis)).await;
        assert_eq!(
            peer.closed.load(Ordering::SeqCst),
            0,
            "idle eviction waits for acquire"
        );
        drain(peer.call()).await;
        {
            let frames = peer.creates.lock().unwrap();
            assert_eq!(frames[1].0 != frames[0].0, expired, "idle {millis}ms");
            if expired {
                assert!(frames[1].1.get("previous_response_id").is_none());
                assert_eq!(frames[1].1["input"].as_array().unwrap().len(), 1);
            } else {
                assert_eq!(frames[1].1["previous_response_id"], "response-1");
                assert_eq!(frames[1].1["input"], json!([]));
            }
        }
        peer.finish().await;
    }
}

#[tokio::test]
async fn releases_reset_idle_but_not_fifty_five_minute_connection_age() {
    for (last_millis, expired) in [(179_999, false), (180_000, true), (180_001, true)] {
        let peer = PoolPeer::new(false).await;
        drain(peer.call()).await;
        for _ in 0..13 {
            advance(Duration::from_secs(4 * 60)).await;
            drain(peer.call()).await;
        }
        assert!(
            peer.creates.lock().unwrap().iter().all(|(id, _)| *id == 1),
            "each release resets idle"
        );
        advance(Duration::from_millis(last_millis)).await;
        drain(peer.call()).await;
        {
            let frames = peer.creates.lock().unwrap();
            let (id, body) = frames.last().unwrap();
            assert_eq!(*id != 1, expired, "age 52 minutes + {last_millis}ms");
            assert_eq!(body.get("previous_response_id").is_none(), expired);
            assert_eq!(
                body["input"].as_array().unwrap().len(),
                usize::from(expired)
            );
        }
        peer.finish().await;
    }
}

#[tokio::test]
async fn busy_connection_survives_age_until_release_and_next_acquire() {
    let peer = PoolPeer::new(true).await;
    drain(peer.call()).await;
    let call = tokio::spawn(drain(peer.call()));
    peer.entered.notified().await;
    advance(Duration::from_secs(60 * 60)).await;
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    assert!(!call.is_finished(), "age cannot interrupt a busy response");
    assert_eq!(peer.closed.load(Ordering::SeqCst), 0);
    peer.release.notify_one();
    call.await.unwrap();
    drain(peer.call()).await;
    {
        let frames = peer.creates.lock().unwrap();
        assert_eq!(frames[1].0, 1);
        assert_eq!(frames[2].0, 2, "expired age is checked after release");
        assert!(frames[2].1.get("previous_response_id").is_none());
        assert_eq!(frames[2].1["input"].as_array().unwrap().len(), 1);
    }
    peer.finish().await;
}
