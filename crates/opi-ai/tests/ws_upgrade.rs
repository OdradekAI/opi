//! Public strict Upgrade, negotiation and source-qualified WSS transport contracts.
//!
//! Loopback peers exercise the public HttpClient/Upgrade interface and detached
//! raw byte stream. Codec execution and provider routing have separate coverage.

use opi_ai::http::{HttpClient, HttpClientBuilder, RedirectPolicy};
use opi_ai::ws::{self, PmdOffer, WsUpgradeError, WsUpgradeRequest};
use url::Url;

mod support {
    #[allow(dead_code)]
    pub(crate) mod tls_fixtures;
    pub(crate) mod ws_peer;
}
use support::tls_fixtures;
use support::ws_peer::{Peer, RequestHead, nonce_from_key, response_101};

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
    use opi_ai::http::{ProxyConfig, TlsOptions};
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
        let mut stream = upgraded.stream;
        let mut tail = [0; 6];
        stream.read_exact(&mut tail).await.unwrap();
        assert_eq!(&tail, b"\x82\x04wss!");
        let mut close = vec![0x88, 0x80];
        close.extend_from_slice(&ws::fresh_mask());
        stream.write_all(&close).await.unwrap();
        stream.flush().await.unwrap();
        let (alpn, http, authenticated) = peer.await.unwrap();
        assert_eq!(alpn.as_deref(), Some(b"http/1.1".as_slice()));
        assert_eq!(http, 1);
        assert_eq!(authenticated, identity);
        drop(stream);
        if let Some(proxy) = proxy {
            proxy.await.unwrap();
        }
    }
}

#[tokio::test]
async fn wss_upgrade_preserves_trust_rejection_before_http() {
    use opi_ai::http::TlsOptions;
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

#[tokio::test]
async fn upgrade_transfers_all_directional_pmd_parameters() {
    for bits in [8, 9, 10, 15] {
        for server_no_context in [false, true] {
            let peer = Peer::spawn(
                |_| Ok(()),
                move |head| {
                    let server_flag = if server_no_context {
                        "; server_no_context_takeover"
                    } else {
                        ""
                    };
                    let extension = format!(
                        "permessage-deflate; client_no_context_takeover{server_flag}; server_max_window_bits={bits}; client_max_window_bits={bits}"
                    );
                    response_101(head, &[("Sec-WebSocket-Extensions", &extension)])
                },
            );
            let request = WsUpgradeRequest::new(peer.url()).permessage_deflate(PmdOffer {
                client_no_context_takeover: true,
                client_max_window_bits: Some(None),
            });
            let upgraded = ws::upgrade(&upgrade_client(), request).await.unwrap();
            let parameters = upgraded.negotiation.driver_parameters().unwrap();
            assert_eq!(
                parameters,
                ws::NegotiatedPmd {
                    server_no_context_takeover: server_no_context,
                    client_no_context_takeover: true,
                    server_max_window_bits: bits,
                    client_max_window_bits: Some(bits),
                }
            );
            drop(upgraded);
            peer.finish().unwrap();
        }
    }
}

fn upgrade_client() -> HttpClient {
    HttpClientBuilder::new()
        .redirects(RedirectPolicy::None)
        .build()
        .unwrap()
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

#[tokio::test]
async fn upgrade_sends_the_strict_handshake_request() {
    let peer = Peer::spawn(
        |_| Ok(()),
        |head| {
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
        },
    );
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
    let peer = Peer::spawn(
        |_| Ok(()),
        |_| {
            "HTTP/1.1 307 Temporary Redirect\r\nLocation: /elsewhere\r\nContent-Length: 0\r\n\r\n"
                .as_bytes()
                .to_vec()
        },
    );
    let client = upgrade_client();
    let result = ws::upgrade(&client, WsUpgradeRequest::new(peer.url())).await;
    assert!(matches!(result, Err(WsUpgradeError::Status(307))));
}

#[tokio::test]
async fn upgrade_rejects_mismatched_accept_value() {
    let peer = Peer::spawn(
        |_| Ok(()),
        |head| {
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
        },
    );
    let client = upgrade_client();
    let result = ws::upgrade(&client, WsUpgradeRequest::new(peer.url())).await;
    assert!(matches!(
        result,
        Err(WsUpgradeError::Accept("value does not match the nonce"))
    ));
}

#[tokio::test]
async fn upgrade_rejects_missing_and_duplicated_accept() {
    let peer = Peer::spawn(
        |_| Ok(()),
        |head| {
            String::from_utf8(response_101(head, &[]))
                .unwrap()
                .split_inclusive("\r\n")
                .filter(|line| !line.starts_with("Sec-WebSocket-Accept:"))
                .collect::<String>()
                .into_bytes()
        },
    );
    let result = ws::upgrade(&upgrade_client(), WsUpgradeRequest::new(peer.url())).await;
    assert!(matches!(
        result,
        Err(WsUpgradeError::Accept("header missing"))
    ));
    peer.finish().unwrap();

    for extra in [
        [("Sec-WebSocket-Accept", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==")],
        [("Sec-WebSocket-Version", "13")],
    ] {
        let peer = Peer::spawn(|_| Ok(()), move |head| response_101(head, &extra));
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
async fn upgrade_rejects_invalid_singleton_response_headers_without_resending() {
    use std::io::ErrorKind;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[derive(Clone, Copy, Debug)]
    enum Case {
        MissingUpgrade,
        WrongUpgrade,
        DuplicateUpgrade,
        DuplicateSelectedSubprotocol,
    }

    for case in [
        Case::MissingUpgrade,
        Case::WrongUpgrade,
        Case::DuplicateUpgrade,
        Case::DuplicateSelectedSubprotocol,
    ] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = Url::parse(&format!("ws://{}/socket", listener.local_addr().unwrap())).unwrap();
        let (stop, mut stopped) = tokio::sync::oneshot::channel();
        let peer = tokio::spawn(async move {
            let mut sends = 0;
            loop {
                let (mut stream, _) = tokio::select! {
                    biased;
                    accepted = listener.accept() => accepted.unwrap(),
                    _ = &mut stopped => break,
                };
                let mut head = Vec::new();
                while !head.ends_with(b"\r\n\r\n") {
                    head.push(stream.read_u8().await.unwrap());
                }
                sends += 1;
                let request = RequestHead::read(&mut std::io::Cursor::new(head));
                assert_eq!(request.header("sec-websocket-protocol"), Some("chat.proto"));
                let response = match case {
                    Case::MissingUpgrade => String::from_utf8(response_101(&request, &[]))
                        .unwrap()
                        .replace("Upgrade: websocket\r\n", "")
                        .into_bytes(),
                    Case::WrongUpgrade => String::from_utf8(response_101(&request, &[]))
                        .unwrap()
                        .replace("Upgrade: websocket\r\n", "Upgrade: h2c\r\n")
                        .into_bytes(),
                    Case::DuplicateUpgrade => response_101(&request, &[("Upgrade", "websocket")]),
                    Case::DuplicateSelectedSubprotocol => response_101(
                        &request,
                        &[
                            ("Sec-WebSocket-Protocol", "chat.proto"),
                            ("Sec-WebSocket-Protocol", "chat.proto"),
                        ],
                    ),
                };
                stream.write_all(&response).await.unwrap();
                assert_eq!(
                    stream.read(&mut [0; 1]).await.unwrap(),
                    0,
                    "{case:?}: rejection must release the connection without further output"
                );
            }
            match listener.into_std().unwrap().accept() {
                Err(error) if error.kind() == ErrorKind::WouldBlock => {}
                Err(error) => panic!("{case:?}: checking pending connections: {error}"),
                Ok(_) => panic!("{case:?}: unexpected additional connection"),
            }
            sends
        });
        let request = WsUpgradeRequest::new(url).subprotocols(["chat.proto".to_owned()]);
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            ws::upgrade(&upgrade_client(), request),
        )
        .await
        .unwrap_or_else(|_| panic!("{case:?}: upgrade did not terminate"));
        let error = match result {
            Err(error) => Some(error),
            Ok(upgraded) => {
                drop(upgraded);
                None
            }
        };
        stop.send(()).unwrap();
        let sends = tokio::time::timeout(Duration::from_secs(5), peer)
            .await
            .unwrap_or_else(|_| panic!("{case:?}: peer did not finish"))
            .unwrap();
        assert_eq!(sends, 1, "{case:?}: exactly one Upgrade request");
        let error = error.unwrap_or_else(|| panic!("{case:?}: invalid response was accepted"));
        let expected_rejection = match case {
            Case::MissingUpgrade => {
                matches!(&error, WsUpgradeError::UpgradeHeader("header missing"))
            }
            Case::WrongUpgrade | Case::DuplicateUpgrade => matches!(
                &error,
                WsUpgradeError::UpgradeHeader("must be a single websocket value")
            ),
            Case::DuplicateSelectedSubprotocol => matches!(
                &error,
                WsUpgradeError::UnsolicitedField {
                    field: "Sec-WebSocket-Protocol"
                }
            ),
        };
        assert!(
            expected_rejection,
            "{case:?}: unexpected rejection: {error:?}"
        );
    }
}

#[tokio::test]
async fn upgrade_rejects_connection_header_without_upgrade_token() {
    let peer = Peer::spawn(
        |_| Ok(()),
        |head| {
            let text = String::from_utf8_lossy(&response_101(head, &[]))
                .replace("Connection: Upgrade", "Connection: keep-alive, Upgrade2");
            text.into_bytes()
        },
    );
    let client = upgrade_client();
    let result = ws::upgrade(&client, WsUpgradeRequest::new(peer.url())).await;
    assert!(matches!(
        result,
        Err(WsUpgradeError::ConnectionTokens("upgrade token missing"))
    ));
}

#[tokio::test]
async fn upgrade_accepts_token_wise_connection_lists() {
    let peer = Peer::spawn(
        |_| Ok(()),
        |head| {
            let text = String::from_utf8_lossy(&response_101(head, &[]))
                .replace("Connection: Upgrade", "Connection: keep-alive, UPGRADE");
            text.into_bytes()
        },
    );
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
    let peer = Peer::spawn(
        |_| Ok(()),
        |head| response_101(head, &[("Sec-WebSocket-Protocol", "chat.other")]),
    );
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
        let peer = Peer::spawn(
            |_| Ok(()),
            move |head| response_101(head, &[("Sec-WebSocket-Extensions", extensions)]),
        );
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
    let peer = Peer::spawn(
        |_| Ok(()),
        |head| response_101(head, &[("Sec-WebSocket-Extensions", "permessage-deflate")]),
    );
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
async fn default_redirect_policy_still_uses_the_dedicated_upgrade_client() {
    let peer = Peer::spawn(|_| Ok(()), |head| response_101(head, &[]));
    let upgraded = ws::upgrade(&HttpClient::new(), WsUpgradeRequest::new(peer.url()))
        .await
        .expect("ordinary HTTP redirects do not disable strict Upgrade");
    drop(upgraded);
    peer.finish().unwrap();
}
