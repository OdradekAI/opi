use crate as opi_ai;
use base64::Engine as _;
use futures_util::StreamExt as _;
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::credential::BoxAuthFuture;
use opi_ai::http::{HttpClient, HttpClientBuilder, ProxyConfig};
use opi_ai::openai_codex_responses::OpenAiCodexResponsesProvider;
use opi_ai::provider::{CacheRetention, ProviderError, Request, TransportPreference};
use opi_ai::{AuthProvenance, ChatModelRecord, ModelCapabilities, ModelInfo, Models, WireApi};
use secrecy::SecretString;
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
struct Probe {
    calls: Arc<AtomicUsize>,
    late: Arc<AtomicUsize>,
    cancel: CancellationToken,
    cancel_first: bool,
}
thread_local! {
    static PROBE: RefCell<Option<Probe>> = const { RefCell::new(None) };
}

struct ProbeScope;
impl ProbeScope {
    fn install(probe: Probe) -> Self {
        PROBE.with(|current| assert!(current.borrow_mut().replace(probe).is_none()));
        Self
    }
}
impl Drop for ProbeScope {
    fn drop(&mut self) {
        PROBE.with(|current| {
            current.borrow_mut().take();
        });
    }
}
pub(crate) fn after_inflate() {
    let probe = PROBE.with(|current| current.borrow().clone());
    if let Some(probe) = probe {
        if probe.cancel.is_cancelled() {
            probe.late.fetch_add(1, Ordering::SeqCst);
        }
        let previous = probe.calls.fetch_add(1, Ordering::SeqCst);
        if previous == 0 && probe.cancel_first {
            probe.cancel.cancel();
        }
    }
}

async fn actual_codex_case(cancel_first: bool) -> (usize, usize, usize) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let peer = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            head.push(stream.read_u8().await.unwrap());
        }
        let head = String::from_utf8(head).unwrap();
        assert!(head.starts_with("GET /codex/responses"));
        assert!(
            head.to_ascii_lowercase()
                .contains("authorization: bearer chain-secret")
        );
        let key = head
            .lines()
            .filter_map(|line| line.split_once(": "))
            .find(|(name, _)| name.eq_ignore_ascii_case("sec-websocket-key"))
            .unwrap()
            .1
            .trim();
        let digest = aws_lc_rs::digest::digest(
            &aws_lc_rs::digest::SHA1_FOR_LEGACY_USE_ONLY,
            format!("{key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11").as_bytes(),
        );
        let accept = base64::engine::general_purpose::STANDARD.encode(digest.as_ref());
        let head = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\nSec-WebSocket-Extensions: permessage-deflate; server_max_window_bits=8; client_max_window_bits=8; server_no_context_takeover; client_no_context_takeover\r\n\r\n"
        );
        stream.write_all(head.as_bytes()).await.unwrap();
        let (opcode, _) = read_server_frame(&mut stream).await.unwrap();
        assert_eq!(
            opcode, 0x41,
            "actual Codex response.create must use negotiated PMD"
        );
        let completed = br#"{"type":"response.completed","response":{"id":"response-cancel-probe","model":"gpt-5.4","output":[],"usage":{"input_tokens":1,"output_tokens":1}}}"#;
        stream
            .write_all(&stored_pmd_frame(completed))
            .await
            .unwrap();
        if let Ok((opcode, _)) = read_server_frame(&mut stream).await {
            assert_eq!(opcode, 8);
            write_server_frame(&mut stream, 8, &[]).await.unwrap();
            stream.shutdown().await.unwrap();
        }
    });
    let (models, record, mut request) = codex_setup(port).unwrap();
    let raw = Arc::new(AtomicUsize::new(0));
    let observed = raw.clone();
    request.callbacks =
        crate::callbacks::RequestCallbacks::new().with_on_raw(Arc::new(move |_| {
            observed.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));
    let probe = Probe {
        calls: Arc::new(AtomicUsize::new(0)),
        late: Arc::new(AtomicUsize::new(0)),
        cancel: request.cancel.clone(),
        cancel_first,
    };
    let _scope = ProbeScope::install(probe.clone());
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        models
            .stream_simple(record, request, Default::default())
            .result(),
    )
    .await
    .unwrap();
    if cancel_first {
        assert!(
            matches!(outcome, Err(ref e) if matches!(e.cause(), ProviderError::Cancelled)),
            "{outcome:?}"
        );
        assert_eq!(
            raw.load(Ordering::SeqCst),
            0,
            "logical cancellation suppresses public callbacks"
        );
        peer.abort();
        let _ = peer.await;
    } else {
        assert_eq!(
            outcome.unwrap().response_id.as_deref(),
            Some("response-cancel-probe")
        );
        assert_eq!(raw.load(Ordering::SeqCst), 1);
        peer.await.unwrap();
    }
    (
        probe.calls.load(Ordering::SeqCst),
        probe.late.load(Ordering::SeqCst),
        raw.load(Ordering::SeqCst),
    )
}

#[tokio::test(flavor = "current_thread")]
async fn ws023_actual_codex_observes_logical_cancel_before_each_backend_call() {
    let control = actual_codex_case(false).await;
    assert!(
        control.0 > 1,
        "control must exercise multiple actual inflate calls: {control:?}"
    );
    assert_eq!(control.1, 0);
    let cancelled = actual_codex_case(true).await;
    eprintln!("ws023 control={control:?} cancelled={cancelled:?}");
    assert_eq!(
        cancelled.1, 0,
        "no backend inflate may run after the same logical Request token is cancelled"
    );
    assert_eq!(cancelled.0, 1);
}

#[tokio::test(flavor = "current_thread")]
async fn pooled_codex_decoder_binds_each_active_request_token() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let upgrades = Arc::new(AtomicUsize::new(0));
    let creates = Arc::new(AtomicUsize::new(0));
    let observed_upgrades = upgrades.clone();
    let observed_creates = creates.clone();
    let peer = tokio::spawn(async move {
        for connection in 0..2 {
            let (mut stream, _) = listener.accept().await.unwrap();
            observed_upgrades.fetch_add(1, Ordering::SeqCst);
            accept_upgrade(&mut stream).await;
            let turns = if connection == 0 { 3 } else { 1 };
            for _ in 0..turns {
                let (opcode, _) = read_server_frame(&mut stream).await.unwrap();
                assert_eq!(opcode, 0x41, "every actual create uses negotiated PMD");
                let turn = observed_creates.fetch_add(1, Ordering::SeqCst) + 1;
                let completed = format!(
                    r#"{{"type":"response.completed","response":{{"id":"pool-response-{turn}","model":"gpt-5.4","output":[],"usage":{{"input_tokens":1,"output_tokens":1}}}}}}"#
                );
                stream
                    .write_all(&stored_pmd_frame(completed.as_bytes()))
                    .await
                    .unwrap();
            }
            if let Ok((opcode, _)) = read_server_frame(&mut stream).await {
                assert_eq!(opcode, 8);
                write_server_frame(&mut stream, 8, &[]).await.unwrap();
                stream.shutdown().await.unwrap();
            }
        }
    });
    let (models, record, mut request) = codex_setup(port).unwrap();
    request.transport = TransportPreference::Websocket;
    request.session_id = Some("logical-cancel-pool".into());
    request.cache_retention = CacheRetention::Short;

    let first_token = request.cancel.clone();
    let (first, first_counts) =
        observed_call(&models, record.clone(), request.clone(), false).await;
    assert_eq!(
        first.unwrap().response_id.as_deref(),
        Some("pool-response-1")
    );
    assert!(first_counts.0 > 1);
    assert_eq!(first_counts.1, 0);
    assert_eq!(first_counts.2, 1);
    first_token.cancel();

    request.cancel = CancellationToken::new();
    let (second, second_counts) =
        observed_call(&models, record.clone(), request.clone(), false).await;
    assert_eq!(
        second.unwrap().response_id.as_deref(),
        Some("pool-response-2")
    );
    assert!(
        second_counts.0 > 1,
        "fresh B must actually inflate after completed A cancellation"
    );
    assert_eq!(second_counts.1, 0);
    assert_eq!(second_counts.2, 1);
    assert_eq!(
        upgrades.load(Ordering::SeqCst),
        1,
        "A and B reuse the same upgraded connection"
    );

    request.cancel = CancellationToken::new();
    let (cancelled, cancelled_counts) =
        observed_call(&models, record.clone(), request.clone(), true).await;
    assert!(
        matches!(cancelled, Err(ref e) if matches!(e.cause(), ProviderError::Cancelled)),
        "{cancelled:?}"
    );
    assert_eq!(
        upgrades.load(Ordering::SeqCst),
        1,
        "active cancellation occurs on the same reused decoder"
    );

    request.cancel = CancellationToken::new();
    let (last, last_counts) = observed_call(&models, record, request, false).await;
    assert_eq!(
        last.unwrap().response_id.as_deref(),
        Some("pool-response-4")
    );
    assert!(last_counts.0 > 1);
    assert_eq!(last_counts.1, 0);
    assert_eq!(last_counts.2, 1);
    assert_eq!(
        upgrades.load(Ordering::SeqCst),
        2,
        "the failed decoder is discarded before D"
    );
    assert_eq!(creates.load(Ordering::SeqCst), 4);
    drop(models);
    tokio::time::timeout(std::time::Duration::from_secs(10), peer)
        .await
        .unwrap()
        .unwrap();
    eprintln!(
        "pool A={first_counts:?} B={second_counts:?} cancelled={cancelled_counts:?} D={last_counts:?}"
    );
    assert_eq!(
        cancelled_counts.0, 1,
        "cancelled request runs exactly its first backend call"
    );
    assert_eq!(
        cancelled_counts.1, 0,
        "no backend call after active logical cancellation"
    );
    assert_eq!(
        cancelled_counts.2, 0,
        "no callback after active logical cancellation"
    );
}

async fn observed_call(
    models: &Models,
    record: ChatModelRecord,
    mut request: Request,
    cancel_first: bool,
) -> (
    Result<crate::message::AssistantMessage, crate::stream::CallFailure>,
    (usize, usize, usize),
) {
    let raw = Arc::new(AtomicUsize::new(0));
    let observed = raw.clone();
    request.callbacks =
        crate::callbacks::RequestCallbacks::new().with_on_raw(Arc::new(move |_| {
            observed.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));
    let probe = Probe {
        calls: Arc::new(AtomicUsize::new(0)),
        late: Arc::new(AtomicUsize::new(0)),
        cancel: request.cancel.clone(),
        cancel_first,
    };
    let _scope = ProbeScope::install(probe.clone());
    let mut call = models.stream_simple(record, request, Default::default());
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while call.next().await.is_some() {}
        call.result().await
    })
    .await
    .unwrap();
    (
        outcome,
        (
            probe.calls.load(Ordering::SeqCst),
            probe.late.load(Ordering::SeqCst),
            raw.load(Ordering::SeqCst),
        ),
    )
}

async fn accept_upgrade(stream: &mut TcpStream) {
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        head.push(stream.read_u8().await.unwrap());
    }
    let head = String::from_utf8(head).unwrap();
    assert!(head.starts_with("GET /codex/responses"));
    assert!(
        head.to_ascii_lowercase()
            .contains("authorization: bearer chain-secret")
    );
    let key = head
        .lines()
        .filter_map(|line| line.split_once(": "))
        .find(|(name, _)| name.eq_ignore_ascii_case("sec-websocket-key"))
        .unwrap()
        .1
        .trim();
    let digest = aws_lc_rs::digest::digest(
        &aws_lc_rs::digest::SHA1_FOR_LEGACY_USE_ONLY,
        format!("{key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11").as_bytes(),
    );
    let accept = base64::engine::general_purpose::STANDARD.encode(digest.as_ref());
    let head = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\nSec-WebSocket-Extensions: permessage-deflate; server_max_window_bits=8; client_max_window_bits=8; server_no_context_takeover; client_no_context_takeover\r\n\r\n"
    );
    stream.write_all(head.as_bytes()).await.unwrap();
}

fn client() -> Result<HttpClient, String> {
    HttpClientBuilder::new()
        .proxy(ProxyConfig {
            url: None,
            no_proxy: None,
        })
        .build()
        .map_err(|error| error.to_string())
}

struct StaticBearer {
    secret: &'static str,
    account: &'static str,
}

impl AuthResolver for StaticBearer {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async move {
            Ok(ResolvedAuth {
                env: Default::default(),
                scheme: AuthScheme::Bearer,
                secret: SecretString::from(self.secret),
                base_url: None,
                account_id: Some(self.account.to_owned()),
                provenance: AuthProvenance::default(),
            })
        })
    }
}

async fn read_server_frame(stream: &mut TcpStream) -> Result<(u8, Vec<u8>), String> {
    let first = stream.read_u8().await.map_err(|e| e.to_string())?;
    read_server_frame_from_first(stream, first).await
}

async fn read_server_frame_from_first(
    stream: &mut TcpStream,
    first: u8,
) -> Result<(u8, Vec<u8>), String> {
    let head = [first, stream.read_u8().await.map_err(|e| e.to_string())?];
    let opcode = head[0] & 0x4f;
    let masked = head[1] & 0x80 != 0;
    let len = match head[1] & 0x7f {
        126 => {
            let mut extended = [0; 2];
            stream
                .read_exact(&mut extended)
                .await
                .map_err(|e| e.to_string())?;
            u16::from_be_bytes(extended) as usize
        }
        127 => {
            let mut extended = [0; 8];
            stream
                .read_exact(&mut extended)
                .await
                .map_err(|e| e.to_string())?;
            u64::from_be_bytes(extended) as usize
        }
        value => value as usize,
    };
    if !masked {
        return Err("client frames must be masked".into());
    }
    let mut mask = [0; 4];
    if masked {
        stream
            .read_exact(&mut mask)
            .await
            .map_err(|e| e.to_string())?;
    }
    let mut payload = vec![0; len];
    stream
        .read_exact(&mut payload)
        .await
        .map_err(|e| e.to_string())?;
    if masked {
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[index % 4];
        }
    }
    Ok((opcode, payload))
}

async fn write_server_frame(
    stream: &mut TcpStream,
    opcode: u8,
    payload: &[u8],
) -> Result<(), String> {
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
    stream.write_all(&frame).await.map_err(|e| e.to_string())?;
    stream.flush().await.map_err(|e| e.to_string())
}

fn codex_setup(port: u16) -> Result<(Models, ChatModelRecord, Request), String> {
    let mut model = ModelInfo::new(
        "gpt-5.4",
        "GPT-5.4",
        WireApi::OpenAiCodexResponses,
        ModelCapabilities::new(272_000, 128_000).with_streaming(true),
    );
    model.base_url = Some(format!("http://127.0.0.1:{port}"));
    let provider =
        OpenAiCodexResponsesProvider::new(None, vec![model.clone()], Arc::new(client()?));
    let mut models = Models::new();
    models
        .register_route(
            Box::new(provider),
            Arc::new(StaticBearer {
                secret: "chain-secret",
                account: "chain-account",
            }),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .map_err(|e| e.to_string())?;
    let request = Request {
        projection: Default::default(),
        transport: TransportPreference::Auto,
        model: "openai-codex:gpt-5.4".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("system".into()),
            messages: vec![opi_ai::message::Message::User(
                opi_ai::message::UserMessage {
                    content: vec![opi_ai::message::InputContent::Text {
                        text: "hello".into(),
                    }],
                    timestamp_ms: 0,
                },
            )],
            tools: vec![],
        })
        .map_err(|e| e.to_string())?,
        max_tokens: Some(64),
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::Disabled,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };
    let record = ChatModelRecord::new("openai-codex", model);
    Ok((models, record, request))
}

fn stored_pmd_frame(payload: &[u8]) -> Vec<u8> {
    let length = u16::try_from(payload.len()).expect("bounded test message");
    let mut compressed = vec![0];
    compressed.extend_from_slice(&length.to_le_bytes());
    compressed.extend_from_slice(&(!length).to_le_bytes());
    compressed.extend_from_slice(payload);
    compressed.push(0); // Empty stored-block header; RFC7692 omits LEN/NLEN.
    let mut frame = vec![0xc1];
    match compressed.len() {
        0..=125 => frame.push(compressed.len() as u8),
        _ => {
            frame.push(126);
            frame.extend_from_slice(&(compressed.len() as u16).to_be_bytes());
        }
    }
    frame.extend_from_slice(&compressed);
    frame
}
