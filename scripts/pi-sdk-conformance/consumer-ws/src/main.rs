//! Workspace-external public Upgrade consumer of the actual `opi-ai` package.
//!
//! This battery exercises HTTP Upgrade, negotiation and the detached raw
//! stream. It does not instantiate the private frame/PMD/Close implementation
//! or prove the provider-to-codec chain reserved for the Codex adapter.

use base64::Engine as _;
use opi_ai::http::{HttpClient, HttpClientBuilder, ProxyConfig, RedirectPolicy};
use opi_ai::ws::{self, PmdOffer, WsUpgradeError, WsUpgradeRequest};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use url::Url;

type Peer = tokio::task::JoinHandle<Result<(), String>>;

fn client() -> Result<HttpClient, String> {
    HttpClientBuilder::new()
        .redirects(RedirectPolicy::None)
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
            "scope": "external public HTTP Upgrade and raw stream; private codec not exercised",
            "public_codec_status": "OPEN: task 20.16",
            "failed": failed, "total": checks.len(), "checks": checks,
        })
    );
}
