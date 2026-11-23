//! HTTPS proxy CONNECT, inner TLS, and scoped NO_PROXY bypass.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use opi_ai::http::{HttpClient, HttpClientBuilder, ProxyConfig, TlsOptions};
use rustls::{
    ServerConfig,
    crypto::aws_lc_rs,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;

use super::fixtures::{
    RawStats, RecordedRequest, Responder, Script, bearer_of, chat_call, find_head_end,
    serve_scripted, sse_chunked_response,
};
use super::tls_fixtures;

fn tls_server_config(cert: &str, key: &str, alpn_h1: bool) -> ServerConfig {
    let cert = CertificateDer::from_pem_slice(cert.as_bytes()).expect("cert pem");
    let key = PrivateKeyDer::from_pem_slice(key.as_bytes()).expect("key pem");
    let provider = Arc::new(aws_lc_rs::default_provider());
    let mut config = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
        .expect("server versions")
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .expect("server identity");
    // The proxy layer sees no ALPN offer (reqwest clears proxy-connection
    // ALPN); the inner target selects HTTP/1.1 from the client's offer.
    config.alpn_protocols = if alpn_h1 {
        vec![b"http/1.1".to_vec()]
    } else {
        vec![]
    };
    config
}

/// A TLS-fronted scripted SSE target server.
async fn start_tls_target() -> (String, Arc<RawStats>, oneshot::Sender<()>, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind tls target");
    let addr = listener.local_addr().expect("tls target address");
    let acceptor = TlsAcceptor::from(Arc::new(tls_server_config(
        tls_fixtures::CERT_SERVER_RSA,
        tls_fixtures::KEY_SERVER_RSA,
        true,
    )));
    let stats = Arc::new(RawStats::default());
    let server_stats = stats.clone();
    let (stop, mut stop_rx) = oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        let mut connections = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                _ = &mut stop_rx => break,
                accepted = listener.accept() => {
                    let Ok((stream, _)) = accepted else { break };
                    let acceptor = acceptor.clone();
                    let stats = server_stats.clone();
                    connections.spawn(async move {
                        let Ok(tls) = acceptor.accept(stream).await else { return };
                        let mut tls = tls;
                        let responder: Responder = Arc::new(|_: usize, _: &RecordedRequest| {
                            Script::coalesced(sse_chunked_response())
                        });
                        let _ = serve_scripted(&mut tls, stats, responder).await;
                    });
                }
                Some(_) = connections.join_next(), if !connections.is_empty() => {}
            }
        }
        connections.abort_all();
        while connections.join_next().await.is_some() {}
    });
    (format!("https://{addr}"), stats, stop, task)
}

/// An HTTPS proxy: outer TLS terminated at the proxy, then CONNECT tunnel.
async fn start_https_proxy(
    target_addr: std::net::SocketAddr,
) -> (
    String,
    Arc<AtomicUsize>,
    oneshot::Sender<()>,
    JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind https proxy");
    let addr = listener.local_addr().expect("https proxy address");
    let acceptor = TlsAcceptor::from(Arc::new(tls_server_config(
        tls_fixtures::CERT_SERVER_RSA,
        tls_fixtures::KEY_SERVER_RSA,
        false,
    )));
    let connects = Arc::new(AtomicUsize::new(0));
    let proxy_connects = connects.clone();
    let (stop, mut stop_rx) = oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        let mut tunnels = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                _ = &mut stop_rx => break,
                accepted = listener.accept() => {
                    let Ok((stream, _)) = accepted else { break };
                    let acceptor = acceptor.clone();
                    let connects = proxy_connects.clone();
                    tunnels.spawn(async move {
                        let Ok(mut tls) = acceptor.accept(stream).await else { return };
                        connects.fetch_add(1, Ordering::SeqCst);
                        // Read the CONNECT head.
                        let mut buf = Vec::with_capacity(512);
                        let mut scratch = [0u8; 1024];
                        let head_end = loop {
                            if let Some(pos) = find_head_end(&buf) { break pos; }
                            let n = match tls.read(&mut scratch).await { Ok(n) => n, Err(_) => return };
                            if n == 0 { return; }
                            buf.extend_from_slice(&scratch[..n]);
                        };
                        let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
                        if !head.starts_with("CONNECT ") { return; }
                        let _ = tls.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n").await;
                        let _ = tls.flush().await;
                        let Ok(mut target) = TcpStream::connect(target_addr).await else { return };
                        let _ = tokio::io::copy_bidirectional(&mut tls, &mut target).await;
                    });
                }
                Some(_) = tunnels.join_next(), if !tunnels.is_empty() => {}
            }
        }
        tunnels.abort_all();
        while tunnels.join_next().await.is_some() {}
    });
    (format!("https://{addr}"), connects, stop, task)
}

fn tls_proxy_client(proxy_url: &str, no_proxy: Option<&str>) -> Arc<HttpClient> {
    let options = TlsOptions::new()
        .with_explicit_roots_pem(tls_fixtures::CERT_CA.as_bytes())
        .expect("explicit roots");
    Arc::new(
        HttpClientBuilder::new()
            .max_idle_per_host(0)
            .tls(options)
            .proxy(ProxyConfig {
                url: Some(proxy_url.to_owned()),
                no_proxy: no_proxy.map(str::to_owned),
            })
            .build()
            .expect("proxied TLS client build"),
    )
}

#[tokio::test]
async fn https_proxy_outer_tls_connect_tunnel_inner_tls() {
    let (target_url, target_stats, target_stop, target_task) = start_tls_target().await;
    let target_addr: std::net::SocketAddr = target_url
        .trim_start_matches("https://")
        .parse()
        .expect("target address");
    let (proxy_url, connects, proxy_stop, proxy_task) = start_https_proxy(target_addr).await;
    let message = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        chat_call(&target_url, tls_proxy_client(&proxy_url, None)),
    )
    .await
    .expect("tunneled call terminates")
    .expect("outer TLS -> CONNECT -> inner TLS round trip");
    assert_eq!(message.response_id.as_deref(), Some("chatcmpl-acc"));
    assert_eq!(
        connects.load(Ordering::SeqCst),
        1,
        "exactly one CONNECT, no double wrap"
    );
    let requests = target_stats.requests();
    assert_eq!(requests.len(), 1, "one tunneled request to the target");
    assert_eq!(
        bearer_of(&requests[0]),
        Some("Bearer acceptance-contract-key"),
        "the tunneled request keeps its protected authorization header"
    );
    let _ = proxy_stop.send(());
    let _ = proxy_task.await;
    let _ = target_stop.send(());
    let _ = target_task.await;
}

#[tokio::test]
async fn scoped_no_proxy_bypasses_the_https_proxy() {
    let (target_url, target_stats, target_stop, target_task) = start_tls_target().await;
    let target_addr: std::net::SocketAddr = target_url
        .trim_start_matches("https://")
        .parse()
        .expect("target address");
    let (proxy_url, connects, proxy_stop, proxy_task) = start_https_proxy(target_addr).await;
    let message = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        chat_call(&target_url, tls_proxy_client(&proxy_url, Some("127.0.0.1"))),
    )
    .await
    .expect("bypassed call terminates")
    .expect("scoped NO_PROXY keeps the direct TLS path");
    assert_eq!(message.response_id.as_deref(), Some("chatcmpl-acc"));
    assert_eq!(
        connects.load(Ordering::SeqCst),
        0,
        "the scoped NO_PROXY entry bypasses the proxy entirely"
    );
    let requests = target_stats.requests();
    assert_eq!(requests.len(), 1, "direct connection to the target");
    let _ = proxy_stop.send(());
    let _ = proxy_task.await;
    let _ = target_stop.send(());
    let _ = target_task.await;
}
