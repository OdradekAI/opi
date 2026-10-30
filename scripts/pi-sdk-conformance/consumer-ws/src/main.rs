//! Workspace-external consumer of the actual `opi-ai` package.
//!
//! This battery exercises HTTP Upgrade, negotiation and the detached raw
//! stream, plus (task 20.16) the real `Models` -> Codex adapter -> strict
//! Upgrade -> private frame/PMD/Close chain against a local scripted peer.
//! The private codec itself remains internal to the package; the real-chain
//! check observes it only through the public provider surface.

use base64::Engine as _;
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::credential::BoxAuthFuture;
use opi_ai::http::{HttpClient, HttpClientBuilder, ProxyConfig};
use opi_ai::openai_codex_responses::OpenAiCodexResponsesProvider;
use opi_ai::provider::{CacheRetention, ProviderError, Request, TransportPreference};
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::ws::{self, PmdOffer, WsUpgradeError, WsUpgradeRequest};
use opi_ai::{AuthProvenance, ChatModelRecord, ModelCapabilities, ModelInfo, Models, WireApi};
use secrecy::SecretString;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;
use url::Url;

mod deflate_audit;

type Peer = tokio::task::JoinHandle<Result<(), String>>;

fn client() -> Result<HttpClient, String> {
    HttpClientBuilder::new()
        .proxy(ProxyConfig {
            url: None,
            no_proxy: None,
        })
        .build()
        .map_err(|error| error.to_string())
}

async fn response_peer(
    status: &'static str,
    extra: &'static str,
    corrupt_accept: bool,
    tail: &'static [u8],
) -> Result<(Url, Peer), String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let url = Url::parse(&format!(
        "ws://{}/socket",
        listener.local_addr().map_err(|e| e.to_string())?
    ))
    .map_err(|e| e.to_string())?;
    let peer = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.map_err(|e| e.to_string())?;
        respond(&mut stream, status, extra, corrupt_accept, tail).await
    });
    Ok((url, peer))
}

async fn respond(
    stream: &mut TcpStream,
    status: &str,
    extra: &str,
    corrupt_accept: bool,
    tail: &[u8],
) -> Result<(), String> {
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        head.push(stream.read_u8().await.map_err(|e| e.to_string())?);
    }
    let head = String::from_utf8(head).map_err(|e| e.to_string())?;
    if !head.starts_with("GET /socket HTTP/1.1\r\n") {
        return Err("not a classic HTTP/1.1 Upgrade request".into());
    }
    let key = head
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("sec-websocket-key")
                .then(|| value.trim())
        })
        .ok_or("request key missing")?;
    // The loopback server calculates the response independently of the
    // SDK's accept_token helper. No production codec is copied or included.
    let input = format!("{key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    let digest = aws_lc_rs::digest::digest(
        &aws_lc_rs::digest::SHA1_FOR_LEGACY_USE_ONLY,
        input.as_bytes(),
    );
    let accept = if corrupt_accept {
        "invalid".into()
    } else {
        base64::engine::general_purpose::STANDARD.encode(digest.as_ref())
    };
    let mut response = format!("HTTP/1.1 {status}\r\nConnection: keep-alive, Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Accept: {accept}\r\n{extra}\r\n").into_bytes();
    response.extend_from_slice(tail);
    stream
        .write_all(&response)
        .await
        .map_err(|e| e.to_string())?;
    stream.shutdown().await.map_err(|e| e.to_string())
}

async fn strict_upgrade_and_raw_tail_once() -> Result<(), String> {
    if ws::accept_token(b"the sample nonce") != "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=" {
        return Err("RFC 6455 accept known answer mismatch".into());
    }
    // Opaque bytes, deliberately not parsed as WS messages by this consumer.
    let tail = b"\x82\x04tail";
    let (url, peer) = response_peer("101 Switching Protocols", "", false, tail).await?;
    let result = ws::upgrade(&client()?, WsUpgradeRequest::new(url)).await;
    let peer_result = peer.await.map_err(|e| e.to_string())?;
    peer_result?;
    let mut upgraded = result.map_err(|e| e.to_string())?;
    if upgraded.negotiation.subprotocol.is_some() || upgraded.negotiation.pmd.is_some() {
        return Err("unsolicited negotiation facts".into());
    }
    let mut received = Vec::new();
    upgraded
        .stream
        .read_to_end(&mut received)
        .await
        .map_err(|e| e.to_string())?;
    if received != tail {
        return Err("coalesced trailing raw bytes were lost or repeated".into());
    }
    Ok(())
}

async fn direction_specific_pmd_facts() -> Result<(), String> {
    let (url, peer) = response_peer("101 Switching Protocols",
        "Sec-WebSocket-Protocol: codex\r\nSec-WebSocket-Extensions: permessage-deflate; server_no_context_takeover; client_no_context_takeover; server_max_window_bits=8; client_max_window_bits=9\r\n",
        false, &[]).await?;
    let request = WsUpgradeRequest::new(url)
        .subprotocols(["codex".to_owned()])
        .permessage_deflate(PmdOffer {
            client_no_context_takeover: true,
            client_max_window_bits: Some(Some(10)),
        });
    let result = ws::upgrade(&client()?, request).await;
    peer.await.map_err(|e| e.to_string())??;
    let upgraded = result.map_err(|e| e.to_string())?;
    let parameters = upgraded
        .negotiation
        .driver_parameters()
        .ok_or("PMD facts missing")?;
    if upgraded.negotiation.subprotocol.as_deref() != Some("codex")
        || !parameters.server_no_context_takeover
        || !parameters.client_no_context_takeover
        || parameters.server_max_window_bits != 8
        || parameters.client_max_window_bits != Some(9)
    {
        return Err("direction-specific negotiation facts changed".into());
    }
    Ok(())
}

async fn rejection(
    status: &'static str,
    extra: &'static str,
    corrupt: bool,
    expected: fn(&WsUpgradeError) -> bool,
) -> Result<(), String> {
    let (url, peer) = response_peer(status, extra, corrupt, &[]).await?;
    let result = ws::upgrade(&client()?, WsUpgradeRequest::new(url)).await;
    peer.await.map_err(|e| e.to_string())??;
    match result {
        Err(error) if expected(&error) => Ok(()),
        Err(error) => Err(format!("wrong typed rejection: {error:?}")),
        Ok(_) => Err("invalid Upgrade was accepted".into()),
    }
}

/// Task 20.16 real-chain check: `Models::stream_simple` drives the registered
/// Codex provider through the strict Upgrade bridge into the private
/// frame/PMD/Close protocol owner. The compressed variant negotiates real
/// window8 and decodes provider frames coalesced with the Upgrade response.
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

async fn codex_real_chain(compressed: bool) -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let port = listener.local_addr().unwrap().port();
    let peer = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.map_err(|e| e.to_string())?;
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            head.push(stream.read_u8().await.map_err(|e| e.to_string())?);
        }
        let head = String::from_utf8(head).map_err(|e| e.to_string())?;
        let lowered = head.to_ascii_lowercase();
        if !head.starts_with("GET /codex/responses") {
            return Err("real chain must upgrade /codex/responses".to_owned());
        }
        if !lowered.contains("openai-beta: responses_websockets=2026-02-06") {
            return Err("real chain must send the websocket beta header".to_owned());
        }
        if !lowered.contains("authorization: bearer chain-secret") {
            return Err("real chain must send bearer auth on the handshake".to_owned());
        }
        let key = head
            .lines()
            .filter_map(|line| line.split_once(": "))
            .find(|(name, _)| name.eq_ignore_ascii_case("sec-websocket-key"))
            .map(|(_, value)| value.trim().to_owned())
            .ok_or("handshake missing Sec-WebSocket-Key".to_owned())?;
        let input = format!("{key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
        let digest = aws_lc_rs::digest::digest(
            &aws_lc_rs::digest::SHA1_FOR_LEGACY_USE_ONLY,
            input.as_bytes(),
        );
        let accept = base64::engine::general_purpose::STANDARD.encode(digest.as_ref());
        let extension = if compressed {
            "Sec-WebSocket-Extensions: permessage-deflate; server_max_window_bits=8; client_max_window_bits=8; server_no_context_takeover; client_no_context_takeover\r\n"
        } else {
            ""
        };
        let mut response = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n{extension}\r\n"
        ).into_bytes();
        if compressed {
            let created = br#"{"type":"response.created","response":{"id":"response-1","model":"gpt-5.4","status":"in_progress","output":[]}}"#;
            response.extend_from_slice(&stored_pmd_frame(created));
        }
        stream
            .write_all(&response)
            .await
            .map_err(|e| e.to_string())?;
        let (opcode, mut payload) = read_server_frame(&mut stream).await?;
        if compressed {
            if opcode != 0x41 {
                return Err("negotiated response.create must use PMD".into());
            }
            let (plain, maximum_distance) =
                deflate_audit::decode_pmd(&payload, &mut Vec::new(), 8)?;
            if maximum_distance > 256 {
                return Err("outbound PMD exceeded actual window8".into());
            }
            payload = plain;
        } else if opcode != 1 {
            return Err("response.create must be a text frame".to_owned());
        }
        let frame: serde_json::Value =
            serde_json::from_slice(&payload).map_err(|e| e.to_string())?;
        if compressed && frame["input"][0]["content"] != window8_text() {
            return Err("window8 outbound request text changed".into());
        }
        if frame["type"] != "response.create" || frame["store"] != false {
            return Err("unexpected response.create body".to_owned());
        }
        let completed = r#"{"type":"response.completed","response":{"id":"response-1","model":"gpt-5.4","output":[],"usage":{"input_tokens":1,"output_tokens":1}}}"#;
        if compressed {
            let delta =
                serde_json::json!({"type":"response.output_text.delta", "delta": window8_text()})
                    .to_string();
            stream
                .write_all(&stored_pmd_frame(delta.as_bytes()))
                .await
                .map_err(|e| e.to_string())?;
            stream
                .write_all(&stored_pmd_frame(completed.as_bytes()))
                .await
                .map_err(|e| e.to_string())?;
        } else {
            write_server_frame(&mut stream, 1, completed.as_bytes()).await?;
        }
        let (close_opcode, _) = read_server_frame(&mut stream).await?;
        if close_opcode != 8 {
            return Err("client must complete an explicit Close".to_owned());
        }
        write_server_frame(&mut stream, 8, &[]).await?;
        let mut sink = [0; 64];
        let _ = stream.read(&mut sink).await;
        Ok(())
    });

    let (models, record, mut request) = codex_setup(port)?;
    if compressed {
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("system".into()),
            messages: vec![opi_ai::message::Message::User(
                opi_ai::message::UserMessage {
                    content: vec![opi_ai::message::InputContent::Text {
                        text: window8_text(),
                    }],
                    timestamp_ms: 0,
                },
            )],
            tools: vec![],
        })
        .map_err(|e| e.to_string())?;
    }
    let call = models.stream_simple(
        record,
        request,
        opi_ai::SimpleOptions {
            tool_choice: None,
            temperature: None,
            max_tokens: None,
            timeout: None,
            stop_sequences: vec![],
        },
    );
    let message = tokio::time::timeout(std::time::Duration::from_secs(20), call.result())
        .await
        .map_err(|_| "real-chain call timed out")?
        .map_err(|e| format!("real-chain call failed: {e:?}"))?;
    peer.await.map_err(|e| e.to_string())??;
    if message.response_id.as_deref() != Some("response-1") {
        return Err(format!("unexpected response id: {:?}", message.response_id));
    }
    if message.stop_reason != opi_ai::stream::StopReason::Stop {
        return Err(format!("unexpected stop reason: {:?}", message.stop_reason));
    }
    if compressed {
        let text: String = message
            .content
            .iter()
            .filter_map(|block| match block {
                opi_ai::message::AssistantContent::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        if text != window8_text() {
            return Err("window8 provider text changed or trailing frames were lost".into());
        }
    }
    let _ = AssistantStreamEvent::Start {
        partial: message.clone(),
    };
    Ok(())
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

fn window8_text() -> String {
    let mut state = 0x35a9_17cdu32;
    let block: String = (0..512)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            char::from(b'a' + (state >> 28) as u8)
        })
        .collect();
    block.repeat(16)
}

/// Independent RFC1951 stored blocks: every reference distance is zero, so
/// this lawful message also exercises actual window8 negotiation at >256B.
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

async fn codex_cleanup_chain(active: bool) -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let peer = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.map_err(|e| e.to_string())?;
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            head.push(stream.read_u8().await.map_err(|e| e.to_string())?);
        }
        let head = String::from_utf8(head).map_err(|e| e.to_string())?;
        let key = head
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("sec-websocket-key"))
            .map(|(_, value)| value.trim())
            .ok_or("missing Upgrade key")?;
        let input = format!("{key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
        let digest = aws_lc_rs::digest::digest(
            &aws_lc_rs::digest::SHA1_FOR_LEGACY_USE_ONLY,
            input.as_bytes(),
        );
        let accept = base64::engine::general_purpose::STANDARD.encode(digest.as_ref());
        let response = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
        );
        stream
            .write_all(response.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        if read_server_frame(&mut stream).await?.0 != 1 {
            return Err("missing response.create".into());
        }
        if !active {
            write_server_frame(&mut stream, 1, br#"{"type":"response.completed","response":{"id":"response-1","model":"gpt-5.4","output":[]}}"#).await?;
        }
        ready_tx
            .send(())
            .map_err(|_| "call disappeared before cleanup")?;
        let (opcode, payload) = if active {
            // Active cutoff may abort IO directly. EOF before any frame is
            // local release; an attempted Close must still be valid and finish.
            let mut first = [0];
            if stream.read(&mut first).await.map_err(|e| e.to_string())? == 0 {
                return Ok(());
            }
            if first[0] != 0x88 {
                return Err("active release wrote an invalid Close".into());
            }
            read_server_frame_from_first(&mut stream, first[0]).await?
        } else {
            read_server_frame(&mut stream).await?
        };
        if opcode != 8 || payload != 1000_u16.to_be_bytes() {
            return Err("cleanup must write the owner's masked normal Close".into());
        }
        write_server_frame(&mut stream, 8, &payload).await?;
        stream.shutdown().await.map_err(|e| e.to_string())?;
        let mut byte = [0];
        if stream.read(&mut byte).await.map_err(|e| e.to_string())? != 0 {
            return Err("bytes after cleanup Close".into());
        }
        Ok::<_, String>(())
    });
    let (models, record, mut request) = codex_setup(port)?;
    request.cache_retention = CacheRetention::Short;
    request.session_id = Some("package-cleanup".into());
    let parent = request.cancel.clone();
    let call = models.stream_simple(record, request, Default::default());
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        if active {
            let cleanup = async {
                ready_rx.await.map_err(|_| "peer did not observe the active request")?;
                models.cleanup_session_resources(Some("package-cleanup")).await.map_err(|e| format!("{e:?}"))?;
                Ok::<_, String>(())
            };
            let (result, cleaned) = tokio::join!(call.result(), cleanup);
            cleaned?;
            if !matches!(result, Err(ref failure) if matches!(failure.cause(), ProviderError::Cancelled)) { return Err("active cleanup did not end the call as Aborted".into()); }
        } else {
            call.result().await.map_err(|e| format!("{e:?}"))?;
            ready_rx.await.map_err(|_| "peer did not observe the completed request")?;
            models.cleanup_session_resources(Some("package-cleanup")).await.map_err(|e| format!("{e:?}"))?;
        }
        peer.await.map_err(|e| e.to_string())??;
        if parent.is_cancelled() { return Err("cleanup cancelled the caller parent".into()); }
        Ok(())
    }).await.map_err(|_| "cleanup did not await local release".to_owned())?
}

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("consumer runtime");
    let checks = runtime.block_on(async {
        [
            (
                "strict-upgrade-and-raw-tail-once",
                strict_upgrade_and_raw_tail_once().await,
            ),
            (
                "models-codex-upgrade-private-codec-chain",
                codex_real_chain(false).await,
            ),
            (
                "models-codex-window8-pmd-trailing-close",
                codex_real_chain(true).await,
            ),
            (
                "models-codex-active-cleanup",
                codex_cleanup_chain(true).await,
            ),
            (
                "models-codex-idle-cleanup",
                codex_cleanup_chain(false).await,
            ),
            (
                "direction-specific-pmd-facts",
                direction_specific_pmd_facts().await,
            ),
            (
                "reject-mismatched-accept",
                rejection("101 Switching Protocols", "", true, |e| {
                    matches!(e, WsUpgradeError::Accept(_))
                })
                .await,
            ),
            (
                "reject-unoffered-subprotocol",
                rejection(
                    "101 Switching Protocols",
                    "Sec-WebSocket-Protocol: unoffered\r\n",
                    false,
                    |e| matches!(e, WsUpgradeError::UnofferedSubprotocol),
                )
                .await,
            ),
            (
                "reject-unsolicited-extension",
                rejection(
                    "101 Switching Protocols",
                    "Sec-WebSocket-Extensions: permessage-deflate\r\n",
                    false,
                    |e| matches!(e, WsUpgradeError::Extension { .. }),
                )
                .await,
            ),
            (
                "redirect-is-terminal",
                rejection(
                    "302 Found",
                    "Location: /must-not-follow\r\nContent-Length: 0\r\n",
                    false,
                    |e| matches!(e, WsUpgradeError::Status(302)),
                )
                .await,
            ),
        ]
        .into_iter()
        .map(|(name, result)| {
            serde_json::json!({
                "name": name, "passed": result.is_ok(), "detail": result.err().unwrap_or_default(),
            })
        })
        .collect::<Vec<_>>()
    });
    let failed = checks
        .iter()
        .filter(|check| check["passed"] != true)
        .count();
    println!(
        "{}",
        serde_json::json!({
            "verdict": if failed == 0 { "PASS" } else { "FAIL" },
            "scope": "external public HTTP Upgrade, raw stream and the real Models -> Codex -> Upgrade -> private-codec chain",
            "public_codec_status": "CLOSED: real-chain window8/PMD/tail/Close/cleanup checks",
            "failed": failed, "total": checks.len(), "checks": checks,
        })
    );
}
