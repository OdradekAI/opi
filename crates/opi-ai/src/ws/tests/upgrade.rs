//! Strict Upgrade -> from-upgraded Connection integration (RDF-24, 20.15).
//!
//! A scripted raw-TCP loopback peer completes the HTTP handshake against the
//! real source-qualified `HttpClient` (hyper/reqwest transport, explicit
//! proxy/TLS profile) and then speaks RFC 6455 frames. The tests prove the
//! strict handshake validation (status/version/Accept/token/singleton/
//! extension rules), the RFC 6455 §1.3 known answer vector, and the bridging
//! of negotiated permessage-deflate parameters into the private protocol
//! connection owner, including a full masked-send and explicit-Close
//! exchange over the detached stream.

use crate::http::{HttpClient, HttpClientBuilder, RedirectPolicy};
use crate::ws::{self, PmdOffer, WsUpgradeError, WsUpgradeRequest};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use url::Url;

use super::{deflate_audit, tls_fixtures};

async fn wss_peer(
    require_identity: bool,
) -> (Url, tokio::task::JoinHandle<(Option<Vec<u8>>, usize, bool)>) {
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let builder = rustls::ServerConfig::builder_with_provider(provider.clone())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap();
    let builder = if require_identity {
        let mut roots = rustls::RootCertStore::empty();
        roots
            .add(CertificateDer::from_pem_slice(tls_fixtures::CERT_CA.as_bytes()).unwrap())
            .unwrap();
        builder.with_client_cert_verifier(
            rustls::server::WebPkiClientVerifier::builder_with_provider(Arc::new(roots), provider)
                .build()
                .unwrap(),
        )
    } else {
        builder.with_no_client_auth()
    };
    let mut config = builder
        .with_single_cert(
            vec![CertificateDer::from_pem_slice(tls_fixtures::CERT_SERVER_RSA.as_bytes()).unwrap()],
            PrivateKeyDer::from_pem_slice(tls_fixtures::KEY_SERVER_RSA.as_bytes()).unwrap(),
        )
        .unwrap();
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = Url::parse(&format!(
        "wss://localhost:{}/socket",
        listener.local_addr().unwrap().port()
    ))
    .unwrap();
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let peer = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let Ok(mut stream) = acceptor.accept(stream).await else {
            return (None, 0, false);
        };
        let alpn = stream.get_ref().1.alpn_protocol().map(Vec::from);
        let authenticated = stream
            .get_ref()
            .1
            .peer_certificates()
            .is_some_and(|c| !c.is_empty());
        if alpn.as_deref() != Some(b"http/1.1") {
            return (alpn, 0, authenticated);
        }
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            head.push(stream.read_u8().await.unwrap());
        }
        let request = RequestHead::read(&mut std::io::Cursor::new(head));
        let mut response = response_101(&request, &[]);
        response.extend_from_slice(&[0x82, 4, b'w', b's', b's', b'!']);
        stream.write_all(&response).await.unwrap();
        let mut masked = [0; 6];
        stream.read_exact(&mut masked).await.unwrap();
        assert_eq!(&masked[..2], &[0x88, 0x80]);
        (alpn, 1, authenticated)
    });
    (url, peer)
}

#[tokio::test]
async fn wss_upgrade_uses_h1_with_default_alpn_and_frozen_mtls() {
    use crate::http::{ProxyConfig, TlsOptions};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    for identity in [false, true] {
        let (url, peer) = wss_peer(identity).await;
        let mut tls = TlsOptions::new()
            .with_extra_roots_pem(tls_fixtures::CERT_CA.as_bytes())
            .unwrap();
        if identity {
            tls = tls
                .with_client_identity_pem(
                    tls_fixtures::CERT_CLIENT_RSA2048.as_bytes(),
                    tls_fixtures::KEY_CLIENT_RSA2048.as_bytes(),
                )
                .unwrap();
        }
        let mut builder = HttpClientBuilder::new()
            .tls(tls)
            .redirects(RedirectPolicy::None);
        let proxy = if identity {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let authority = url[url::Position::BeforeHost..url::Position::AfterPort].to_owned();
            let port = url.port().unwrap();
            let task = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut head = Vec::new();
                while !head.ends_with(b"\r\n\r\n") {
                    head.push(stream.read_u8().await.unwrap());
                }
                assert!(
                    String::from_utf8(head)
                        .unwrap()
                        .starts_with(&format!("CONNECT {authority} HTTP/1.1\r\n"))
                );
                let mut target = tokio::net::TcpStream::connect(("127.0.0.1", port))
                    .await
                    .unwrap();
                stream
                    .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
                    .await
                    .unwrap();
                let _ = tokio::io::copy_bidirectional(&mut stream, &mut target).await;
            });
            builder = builder.proxy(ProxyConfig {
                url: Some(format!("http://{address}")),
                no_proxy: None,
            });
            Some(task)
        } else {
            None
        };
        let client = builder.build().unwrap();
        let upgraded = ws::upgrade(&client, WsUpgradeRequest::new(url)).await;
        let upgraded = upgraded.unwrap_or_else(|e| panic!("classic WSS must use H1: {e:?}"));
        let mut driver = crate::ws::io::Driver::new(upgraded.stream, None).unwrap();
        assert_eq!(driver.next().await.unwrap().unwrap().payload, b"wss!");
        driver.queue(8, &[], false, ws::fresh_mask()).unwrap();
        driver.flush().await.unwrap();
        let (alpn, http, authenticated) = peer.await.unwrap();
        assert_eq!(alpn.as_deref(), Some(b"http/1.1".as_slice()));
        assert_eq!(http, 1);
        assert_eq!(authenticated, identity);
        drop(driver);
        if let Some(proxy) = proxy {
            proxy.await.unwrap();
        }
    }
}

#[tokio::test]
async fn wss_upgrade_preserves_trust_rejection_before_http() {
    use crate::http::TlsOptions;
    let (url, peer) = wss_peer(false).await;
    let tls = TlsOptions::new()
        .with_explicit_roots_pem(tls_fixtures::CERT_OTHER_CA.as_bytes())
        .unwrap();
    let client = HttpClientBuilder::new()
        .tls(tls)
        .redirects(RedirectPolicy::None)
        .build()
        .unwrap();
    assert!(matches!(
        ws::upgrade(&client, WsUpgradeRequest::new(url)).await,
        Err(WsUpgradeError::Transport(_))
    ));
    let (alpn, http, authenticated) = peer.await.unwrap();
    assert_eq!(alpn, None);
    assert_eq!(http, 0);
    assert!(!authenticated);
}

/// What the scripted server should do after the handshake head.
enum After101 {
    /// Complete a full exchange: read one client frame, send a peer Close,
    /// shut down the write side, then wait for the client EOF.
    Exchange {
        expect_rsv1: bool,
    },
    /// Immediately close the socket after the scripted head.
    Nothing,
    IndependentMessages(Vec<u8>),
    Window8 {
        expected: Vec<u8>,
        no_context: bool,
    },
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
                After101::IndependentMessages(expected) => independent_messages(stream, &expected),
                After101::Window8 {
                    expected,
                    no_context,
                } => window8_messages(stream, &expected, no_context),
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

fn independent_messages(mut stream: TcpStream, expected: &[u8]) -> Result<(), String> {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    for message in 0..2 {
        let mut header = [0; 2];
        stream.read_exact(&mut header).map_err(|e| e.to_string())?;
        assert_eq!(header[0], 0xc2);
        assert_ne!(header[1] & 0x80, 0);
        let length = match header[1] & 0x7f {
            126 => {
                let mut length = [0; 2];
                stream.read_exact(&mut length).map_err(|e| e.to_string())?;
                u16::from_be_bytes(length) as usize
            }
            127 => return Err("unexpected large frame".into()),
            length => length as usize,
        };
        let mut mask = [0; 4];
        stream.read_exact(&mut mask).map_err(|e| e.to_string())?;
        let mut payload = vec![0; length];
        stream.read_exact(&mut payload).map_err(|e| e.to_string())?;
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[i % 4];
        }
        let decoded = crate::ws::pmd::Decoder::new(true).unwrap().decode(&payload);
        assert_eq!(
            decoded.unwrap_or_else(|e| panic!("message {message} needs forbidden history: {e}")),
            expected
        );
    }
    Ok(())
}

fn window8_messages(
    mut stream: TcpStream,
    expected: &[u8],
    no_context: bool,
) -> Result<(), String> {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let mut history = Vec::new();
    for message in 0..2 {
        let mut head = [0; 2];
        stream.read_exact(&mut head).map_err(|e| e.to_string())?;
        assert_eq!(head[0], 0xc2);
        assert_ne!(head[1] & 0x80, 0);
        let length = match head[1] & 127 {
            126 => {
                let mut n = [0; 2];
                stream.read_exact(&mut n).unwrap();
                usize::from(u16::from_be_bytes(n))
            }
            127 => return Err("unexpected large frame".into()),
            n => usize::from(n),
        };
        let mut mask = [0; 4];
        stream.read_exact(&mut mask).unwrap();
        let mut payload = vec![0; length];
        stream.read_exact(&mut payload).unwrap();
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[i % 4];
        }
        if no_context {
            history.clear();
        }
        let (decoded, maximum) = deflate_audit::decode_pmd(&payload, &mut history, 8)?;
        assert_eq!(decoded, expected);
        assert!(
            (1..=256).contains(&maximum),
            "wire must exercise actual backreferences"
        );
        if !no_context && message == 1 {
            assert!(
                deflate_audit::decode_pmd(&payload, &mut Vec::new(), 8).is_err(),
                "takeover fixture must really depend on prior history"
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn upgrade_window8_wire_has_independent_decode_and_distance_proof() {
    let mut seed = 7_u32;
    let pattern: Vec<_> = (0..200)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed as u8
        })
        .collect();
    let expected = pattern.repeat(257);
    for no_context in [false, true] {
        let inbound = expected.clone();
        let peer = Peer::spawn(
            After101::Window8 {
                expected: expected.clone(),
                no_context,
            },
            move |head| {
                let flags = if no_context {
                    "; server_no_context_takeover; client_no_context_takeover"
                } else {
                    ""
                };
                let extension = format!(
                    "permessage-deflate; server_max_window_bits=8; client_max_window_bits=8{flags}"
                );
                let mut response = response_101(head, &[("Sec-WebSocket-Extensions", &extension)]);
                let mut encoder = crate::ws::pmd::Encoder::new(8, no_context).unwrap();
                let mut history = Vec::new();
                for _ in 0..2 {
                    let payload = encoder.encode(&inbound).unwrap();
                    if no_context {
                        history.clear();
                    }
                    let (decoded, maximum) =
                        deflate_audit::decode_pmd(&payload, &mut history, 8).unwrap();
                    assert_eq!(decoded, inbound);
                    assert!((1..=256).contains(&maximum));
                    if payload.len() < 126 {
                        response.extend_from_slice(&[0xc2, payload.len() as u8]);
                    } else {
                        response.extend_from_slice(&[0xc2, 126]);
                        response.extend_from_slice(
                            &u16::try_from(payload.len()).unwrap().to_be_bytes(),
                        );
                    }
                    response.extend_from_slice(&payload);
                }
                response
            },
        );
        let request = WsUpgradeRequest::new(peer.url()).permessage_deflate(PmdOffer {
            client_no_context_takeover: no_context,
            client_max_window_bits: Some(Some(8)),
        });
        let upgraded = ws::upgrade(&upgrade_client(), request).await.unwrap();
        let parameters = driver_parameters(&upgraded.negotiation).unwrap();
        assert_eq!(parameters.server_max_window_bits, 8);
        assert_eq!(parameters.client_max_window_bits, 8);
        let mut driver = crate::ws::io::Driver::new(upgraded.stream, Some(parameters)).unwrap();
        for _ in 0..2 {
            assert_eq!(driver.next().await.unwrap().unwrap().payload, expected);
            driver.queue(2, &expected, true, ws::fresh_mask()).unwrap();
            driver.flush().await.unwrap();
        }
        peer.finish().unwrap();
    }
}

#[tokio::test]
async fn upgrade_preserves_directional_context_and_delivers_coalesced_tail_once() {
    let expected = b"retained-history-window:".repeat(1000);
    let server_messages = expected.clone();
    let peer = Peer::spawn(
        After101::IndependentMessages(expected.clone()),
        move |head| {
            let mut response = response_101(
                head,
                &[(
                    "Sec-WebSocket-Extensions",
                    "permessage-deflate; client_no_context_takeover",
                )],
            );
            response.extend_from_slice(&[0x82, 4, b't', b'a', b'i', b'l']);
            let mut encoder = crate::ws::pmd::Encoder::new(15, false).unwrap();
            for _ in 0..2 {
                let payload = encoder.encode(&server_messages).unwrap();
                if payload.len() < 126 {
                    response.extend_from_slice(&[0xc2, payload.len() as u8]);
                } else {
                    response.extend_from_slice(&[0xc2, 126]);
                    response
                        .extend_from_slice(&u16::try_from(payload.len()).unwrap().to_be_bytes());
                }
                response.extend_from_slice(&payload);
            }
            response
        },
    );
    let client = upgrade_client();
    let upgraded = ws::upgrade(&client, bridge_request(peer.url(), true))
        .await
        .unwrap();
    let pmd = upgraded.negotiation.pmd.unwrap();
    assert!(pmd.client_no_context_takeover);
    assert!(!pmd.server_no_context_takeover);
    let parameters = driver_parameters(&upgraded.negotiation);
    let mut driver = crate::ws::io::Driver::new(upgraded.stream, parameters).unwrap();
    assert_eq!(driver.next().await.unwrap().unwrap().payload, b"tail");
    for _ in 0..2 {
        assert_eq!(driver.next().await.unwrap().unwrap().payload, expected);
    }
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(10), driver.next())
            .await
            .is_err()
    );
    for _ in 0..2 {
        driver.queue(2, &expected, true, ws::fresh_mask()).unwrap();
        driver.flush().await.unwrap();
    }
    peer.finish().unwrap();
}

#[tokio::test]
async fn upgrade_transfers_all_directional_pmd_parameters() {
    for bits in [8, 9, 10, 15] {
        for server_no_context in [false, true] {
            let peer = Peer::spawn(After101::Nothing, move |head| {
                let server_flag = if server_no_context {
                    "; server_no_context_takeover"
                } else {
                    ""
                };
                let extension = format!(
                    "permessage-deflate; client_no_context_takeover{server_flag}; server_max_window_bits={bits}; client_max_window_bits={bits}"
                );
                response_101(head, &[("Sec-WebSocket-Extensions", &extension)])
            });
            let request = WsUpgradeRequest::new(peer.url()).permessage_deflate(PmdOffer {
                client_no_context_takeover: true,
                client_max_window_bits: Some(None),
            });
            let upgraded = ws::upgrade(&upgrade_client(), request).await.unwrap();
            let parameters = driver_parameters(&upgraded.negotiation).unwrap();
            assert_eq!(
                parameters,
                crate::ws::io::PmdParameters {
                    server_no_context_takeover: server_no_context,
                    client_no_context_takeover: true,
                    server_max_window_bits: bits,
                    client_max_window_bits: bits,
                }
            );
            drop(crate::ws::io::Driver::new(upgraded.stream, Some(parameters)).unwrap());
            peer.finish().unwrap();
        }
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

fn driver_parameters(negotiation: &ws::Negotiation) -> Option<crate::ws::io::PmdParameters> {
    negotiation
        .driver_parameters()
        .map(|pmd| crate::ws::io::PmdParameters {
            server_no_context_takeover: pmd.server_no_context_takeover,
            client_no_context_takeover: pmd.client_no_context_takeover,
            server_max_window_bits: pmd.server_max_window_bits,
            client_max_window_bits: pmd.client_max_window_bits.unwrap_or(15),
        })
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
    let parameters = driver_parameters(&upgraded.negotiation);
    assert_eq!(parameters.is_some(), expect_rsv1);
    assert!(!parameters.is_some_and(|pmd| pmd.server_no_context_takeover));
    let mut driver = crate::ws::io::Driver::new(upgraded.stream, parameters)
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
            assert_eq!(upgraded.negotiation.driver_parameters(), None);
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
