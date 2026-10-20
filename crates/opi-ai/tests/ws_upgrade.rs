//! Strict Upgrade -> from-upgraded Connection integration (RDF-24, 20.15).
//!
//! A scripted raw-TCP loopback peer completes the HTTP handshake against the
//! real source-qualified `HttpClient` (hyper/reqwest transport, explicit
//! proxy/TLS profile) and then speaks RFC 6455 frames. The tests prove the
//! strict handshake validation (status/version/Accept/token/singleton/
//! extension rules), the RFC 6455 §1.3 known answer vector, and the bridging
//! of negotiated permessage-deflate parameters into the neutral `opi_ws`
//! connection owner, including a full masked-send and explicit-Close
//! exchange over the detached stream.

use opi_ai::http::{HttpClient, HttpClientBuilder, RedirectPolicy};
use opi_ai::ws::{self, PmdOffer, WsUpgradeError, WsUpgradeRequest};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use url::Url;

/// What the scripted server should do after the handshake head.
enum After101 {
    /// Complete a full exchange: read one client frame, send a peer Close,
    /// shut down the write side, then wait for the client EOF.
    Exchange { expect_rsv1: bool },
    /// Immediately close the socket after the scripted head.
    Nothing,
}

struct Peer {
    address: SocketAddr,
    handle: Option<std::thread::JoinHandle<Result<(), String>>>,
}

impl Peer {
    /// Spawn a scripted server thread. `head_fn` receives the parsed request
    /// head and returns the raw 101 response bytes; the follow-up behavior
    /// is selected by `after`.
    fn spawn<F>(after: After101, head_fn: F) -> Peer
    where
        F: FnOnce(&RequestHead) -> Vec<u8> + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let head = {
                let mut reader = BufReader::new(&stream);
                RequestHead::read(&mut reader)
            };
            let mut writer = &stream;
            writer.write_all(&head_fn(&head)).unwrap();
            match after {
                After101::Nothing => Ok(()),
                After101::Exchange { expect_rsv1 } => exchange(stream, expect_rsv1),
            }
        });
        Peer {
            address,
            handle: Some(handle),
        }
    }

    fn url(&self) -> Url {
        Url::parse(&format!("ws://{}/socket", self.address)).unwrap()
    }

    fn finish(mut self) -> Result<(), String> {
        self.handle
            .take()
            .expect("peer thread")
            .join()
            .unwrap_or_else(|_| Err("server thread panicked".into()))
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Read one small client frame (< 126 payload bytes), assert the mask bit
/// and optional RSV1, then run the explicit Close exchange.
fn exchange(stream: TcpStream, expect_rsv1: bool) -> Result<(), String> {
    let mut stream = stream;
    let mut reader = BufReader::new(&mut stream);
    let mut header = [0_u8; 2];
    reader.read_exact(&mut header).map_err(|e| e.to_string())?;
    if header[1] & 0x80 == 0 {
        return Err("client frame was not masked".into());
    }
    if expect_rsv1 && header[0] & 0x40 == 0 {
        return Err("negotiated PMD frame missing RSV1".into());
    }
    if !expect_rsv1 && header[0] & 0x40 != 0 {
        return Err("unnegotiated PMD frame set RSV1".into());
    }
    let length = (header[1] & 0x7f) as usize;
    let mut rest = vec![0_u8; 4 + length];
    reader.read_exact(&mut rest).map_err(|e| e.to_string())?;
    let mask: [u8; 4] = rest[..4].try_into().unwrap();
    let payload = rest[4..]
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ mask[i % 4])
        .collect::<Vec<_>>();
    if !payload.is_empty() && payload[0] == 0x00 {
        return Err("payload looks unmasked".into());
    }
    // Peer Close, then the masked client response, then write shutdown so
    // the client's finish_close observes a clean transport EOF.
    stream.write_all(&[0x88, 0]).map_err(|e| e.to_string())?;
    let mut response = [0_u8; 6];
    stream
        .read_exact(&mut response)
        .map_err(|e| e.to_string())?;
    if response[0] != 0x88 || response[1] != 0x80 {
        return Err("expected masked empty Close response".into());
    }
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Debug)]
struct RequestHead {
    lines: Vec<String>,
}

impl RequestHead {
    fn read<R: BufRead + Read>(reader: &mut R) -> Self {
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                break;
            }
            lines.push(line.trim_end().to_owned());
        }
        Self { lines }
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.lines
            .iter()
            .filter_map(|l| l.split_once(':'))
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.trim())
    }

    fn request_line(&self) -> String {
        self.lines.first().cloned().unwrap_or_default()
    }
}

fn upgrade_client() -> HttpClient {
    HttpClientBuilder::new()
        .redirects(RedirectPolicy::None)
        .build()
        .unwrap()
}

fn nonce_from_key(key: &str) -> [u8; 16] {
    use base64::Engine;
    let mut nonce = [0_u8; 16];
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(key.as_bytes())
        .unwrap();
    nonce.copy_from_slice(&bytes);
    nonce
}

/// Build a 101 response for the request head with optional extra headers.
fn response_101(head: &RequestHead, extra: &[(&str, &str)]) -> Vec<u8> {
    let accept = ws::accept_token(&nonce_from_key(head.header("sec-websocket-key").unwrap()));
    let mut response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {accept}\r\n"
    );
    for (name, value) in extra {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str("\r\n");
    response.into_bytes()
}

#[test]
fn accept_token_matches_rfc6455_known_vector() {
    assert_eq!(
        ws::accept_token(&nonce_from_key("dGhlIHNhbXBsZSBub25jZQ==")),
        "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
    );
}

#[test]
fn fresh_nonce_and_mask_use_platform_entropy() {
    let a = ws::fresh_nonce();
    let b = ws::fresh_nonce();
    assert_ne!(a, b, "nonce generation must not repeat");
    assert_ne!(ws::fresh_mask(), ws::fresh_mask());
}

fn bridge_request(url: Url, offer: bool) -> WsUpgradeRequest {
    let mut request = WsUpgradeRequest::new(url).subprotocols(["chat.proto".to_owned()]);
    if offer {
        request = request.permessage_deflate(PmdOffer {
            client_no_context_takeover: true,
            client_max_window_bits: Some(Some(12)),
        });
    }
    request
}

async fn drive_bridged_exchange(extensions: &str, expect_rsv1: bool) -> Result<(), String> {
    let extensions = extensions.to_owned();
    let peer = Peer::spawn(After101::Exchange { expect_rsv1 }, move |head| {
        response_101(head, &[("Sec-WebSocket-Extensions", extensions.as_str())])
    });
    let url = peer.url();
    let client = upgrade_client();
    let upgraded = ws::upgrade(&client, bridge_request(url, expect_rsv1))
        .await
        .map_err(|error| format!("upgrade failed: {error}"))?;
    let (pmd_enabled, server_no_context) = upgraded.negotiation.driver_parameters();
    assert_eq!(pmd_enabled, expect_rsv1);
    assert!(!server_no_context);
    let mut driver = opi_ws::io::Driver::new(upgraded.stream, pmd_enabled, server_no_context)
        .map_err(|error| format!("driver failed: {error}"))?;
    driver
        .queue(1, "桥接 canary".as_bytes(), expect_rsv1, ws::fresh_mask())
        .map_err(|error| format!("queue failed: {error}"))?;
    driver
        .flush()
        .await
        .map_err(|error| format!("flush failed: {error}"))?;
    let event = driver
        .next()
        .await
        .map_err(|error| format!("next failed: {error}"))?
        .ok_or("connection ended before Close")?;
    assert_eq!(event.opcode, 8);
    driver
        .queue(8, &event.payload, false, ws::fresh_mask())
        .map_err(|error| format!("close queue failed: {error}"))?;
    driver
        .finish_close(None)
        .await
        .map_err(|error| format!("close failed: {error}"))?;
    peer.finish()
}

#[tokio::test]
async fn strict_upgrade_bridges_negotiated_pmd_into_the_neutral_connection() {
    drive_bridged_exchange(
        "permessage-deflate; client_no_context_takeover; client_max_window_bits=10",
        true,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn strict_upgrade_bridges_plain_connections_without_pmd() {
    drive_bridged_exchange("", false).await.unwrap();
}

#[tokio::test]
async fn upgrade_sends_the_strict_handshake_request() {
    let peer = Peer::spawn(After101::Nothing, |head| {
        assert!(
            head.request_line().starts_with("GET /socket HTTP/1.1"),
            "{}",
            head.request_line()
        );
        assert!(
            head.header("upgrade")
                .unwrap()
                .eq_ignore_ascii_case("websocket")
        );
        let connection = head.header("connection").unwrap();
        assert!(
            connection
                .split(',')
                .any(|t| t.trim().eq_ignore_ascii_case("upgrade")),
            "{connection}"
        );
        assert_eq!(head.header("sec-websocket-version").unwrap(), "13");
        assert_eq!(head.header("sec-websocket-protocol").unwrap(), "chat.proto");
        let extensions = head.header("sec-websocket-extensions").unwrap();
        assert!(
            extensions.contains("client_no_context_takeover")
                && extensions.contains("client_max_window_bits=12"),
            "{extensions}"
        );
        response_101(head, &[])
    });
    let client = upgrade_client();
    // The scripted thread asserted the request head before answering; a
    // successful detach (the peer stays open) completes the proof.
    let upgraded = ws::upgrade(&client, bridge_request(peer.url(), true))
        .await
        .expect("strict upgrade succeeds");
    // The scripted head did not select a subprotocol (a legal server
    // choice), and no extensions were answered.
    assert_eq!(upgraded.negotiation.subprotocol, None);
    assert!(upgraded.negotiation.pmd.is_none());
    drop(upgraded);
    peer.finish().unwrap();
}

#[tokio::test]
async fn upgrade_rejects_non_101_status_without_following() {
    let peer = Peer::spawn(After101::Nothing, |_| {
        "HTTP/1.1 307 Temporary Redirect\r\nLocation: /elsewhere\r\nContent-Length: 0\r\n\r\n"
            .as_bytes()
            .to_vec()
    });
    let client = upgrade_client();
    let result = ws::upgrade(&client, WsUpgradeRequest::new(peer.url())).await;
    assert!(matches!(result, Err(WsUpgradeError::Status(307))));
}

#[tokio::test]
async fn upgrade_rejects_mismatched_accept_value() {
    let peer = Peer::spawn(After101::Nothing, |head| {
        let mut response = response_101(head, &[]);
        let marker = b"Sec-WebSocket-Accept: ".as_slice();
        let at = response
            .windows(marker.len())
            .position(|w| w == marker)
            .expect("accept header present");
        response[at + marker.len()] = if response[at + marker.len()] == b'A' {
            b'B'
        } else {
            b'A'
        };
        response
    });
    let client = upgrade_client();
    let result = ws::upgrade(&client, WsUpgradeRequest::new(peer.url())).await;
    assert!(matches!(
        result,
        Err(WsUpgradeError::Accept("value does not match the nonce"))
    ));
}

#[tokio::test]
async fn upgrade_rejects_missing_and_duplicated_accept() {
    for extra in [
        [("Sec-WebSocket-Accept", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==")],
        [("Sec-WebSocket-Version", "13")],
    ] {
        let peer = Peer::spawn(After101::Nothing, move |head| response_101(head, &extra));
        let client = upgrade_client();
        let result = ws::upgrade(&client, WsUpgradeRequest::new(peer.url())).await;
        match result {
            Err(WsUpgradeError::Accept(reason)) => {
                assert!(matches!(
                    reason,
                    "value does not match the nonce" | "header duplicated"
                ));
            }
            Err(WsUpgradeError::UnsolicitedField { field }) => {
                assert_eq!(field, "Sec-WebSocket-Version");
            }
            Err(other) => panic!("expected strict rejection, got {other:?}"),
            Ok(_) => panic!("expected strict rejection"),
        }
    }
}

#[tokio::test]
async fn upgrade_rejects_connection_header_without_upgrade_token() {
    let peer = Peer::spawn(After101::Nothing, |head| {
        let text = String::from_utf8_lossy(&response_101(head, &[]))
            .replace("Connection: Upgrade", "Connection: keep-alive, Upgrade2");
        text.into_bytes()
    });
    let client = upgrade_client();
    let result = ws::upgrade(&client, WsUpgradeRequest::new(peer.url())).await;
    assert!(matches!(
        result,
        Err(WsUpgradeError::ConnectionTokens("upgrade token missing"))
    ));
}

#[tokio::test]
async fn upgrade_accepts_token_wise_connection_lists() {
    let peer = Peer::spawn(After101::Nothing, |head| {
        let text = String::from_utf8_lossy(&response_101(head, &[]))
            .replace("Connection: Upgrade", "Connection: keep-alive, UPGRADE");
        text.into_bytes()
    });
    let client = upgrade_client();
    let result = ws::upgrade(&client, WsUpgradeRequest::new(peer.url())).await;
    match result {
        Ok(upgraded) => {
            assert_eq!(upgraded.negotiation.driver_parameters(), (false, false));
            drop(upgraded);
            peer.finish().unwrap();
        }
        Err(other) => panic!("token list was rejected: {other:?}"),
    }
}

#[tokio::test]
async fn upgrade_rejects_unoffered_subprotocol() {
    let peer = Peer::spawn(After101::Nothing, |head| {
        response_101(head, &[("Sec-WebSocket-Protocol", "chat.other")])
    });
    let client = upgrade_client();
    let request = WsUpgradeRequest::new(peer.url()).subprotocols(["chat.proto".to_owned()]);
    let result = ws::upgrade(&client, request).await;
    assert!(matches!(result, Err(WsUpgradeError::UnofferedSubprotocol)));
}

#[tokio::test]
async fn upgrade_validates_pmd_negotiation_strictly() {
    for extensions in [
        "permessage-deflate; server_max_window_bits=7",
        "permessage-deflate; client_max_window_bits=9",
        "permessage-deflate; mystery_param",
        "permessage-deflate, permessage-deflate",
        "x-webkit-deflate-frame",
        "permessage-deflate; client_no_context_takeover=1",
    ] {
        let peer = Peer::spawn(After101::Nothing, move |head| {
            response_101(head, &[("Sec-WebSocket-Extensions", extensions)])
        });
        let client = upgrade_client();
        let request = WsUpgradeRequest::new(peer.url()).permessage_deflate(PmdOffer {
            client_no_context_takeover: true,
            client_max_window_bits: Some(Some(8)),
        });
        let result = ws::upgrade(&client, request).await;
        assert!(
            matches!(result, Err(WsUpgradeError::Extension { .. })),
            "accepted {extensions}"
        );
    }
}

#[tokio::test]
async fn upgrade_rejects_extensions_that_were_never_offered() {
    let peer = Peer::spawn(After101::Nothing, |head| {
        response_101(head, &[("Sec-WebSocket-Extensions", "permessage-deflate")])
    });
    let client = upgrade_client();
    let result = ws::upgrade(&client, WsUpgradeRequest::new(peer.url())).await;
    assert!(matches!(
        result,
        Err(WsUpgradeError::Extension {
            reason: "extensions were negotiated without an offer"
        })
    ));
}

#[tokio::test]
async fn upgrade_refuses_clients_without_the_none_redirect_policy() {
    let client = HttpClient::new();
    let error = match ws::upgrade(
        &client,
        WsUpgradeRequest::new(Url::parse("ws://127.0.0.1:1").unwrap()),
    )
    .await
    {
        Err(error) => error,
        Ok(_) => panic!("upgrade unexpectedly succeeded"),
    };
    assert!(matches!(error, WsUpgradeError::Detach(message) if message.contains("RedirectPolicy")));
}
