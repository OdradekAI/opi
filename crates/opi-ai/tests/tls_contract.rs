//! Real TLS/roots/mTLS/ALPN contract tests for the public `HttpClient` and
//! Models -> Chat injection path (task 20.14, RDF-23).
//!
//! Every case drives the real production seams: clients are built through
//! `opi_ai::http::HttpClientBuilder::tls(TlsOptions)` and injected into
//! `OpenAiChatProvider::with_client` -> `Models::register_route` ->
//! `prepare_call` -> `start_attempt`, or exercise the same `HttpClient`
//! transport directly for lifecycle cases. The loopback peers below run a
//! real rustls TLS server (instance AWS-LC provider) over hyper; no raw
//! reqwest/rustls client is constructed by the tests.
//!
//! Proven here: pinned-Node-default-with-extra and explicit root trust,
//! opt-in host-root scope, H2/H1 ALPN negotiation over TLS 1.3/1.2, the
//! admitted client-identity profile (RSA-2048..8192 incl. e=3, P-521) and
//! P-521 server identity, typed pre-I/O rejection of RSA-1024/multi-prime/
//! encrypted/mismatched identities, trust/hostname/EKU/ALPN negatives with
//! zero business HTTP and no hidden fallback or extra sends, per-connection
//! identity isolation, cancellation during handshake/response/partial body
//! with owned cleanup and no late bytes, no process-global provider
//! mutation, and independence from a host-preinstalled ring provider.

#![forbid(unsafe_code)]

mod support {
    pub(crate) mod tls_fixtures;
}

use support::tls_fixtures::*;

use std::{
    convert::Infallible,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use futures_util::StreamExt;
use http_body_util::{BodyExt, Full};
use hyper::{Request as HyperRequest, Response, body::Incoming, service::service_fn};
use hyper_util::rt::{TokioExecutor, TokioIo};
use opi_ai::auth::{AuthProvenanceSource, AuthScheme, StaticAuthResolver};
use opi_ai::http::{
    HttpClient, HttpClientBuildError, HttpClientBuilder, TlsConfigError, TlsKeyError, TlsOptions,
};
use opi_ai::message::AssistantMessage;
use opi_ai::models::Models;
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::provider::{CacheRetention, EventStream, ProviderError};
use opi_ai::stream::ChatStream;
use opi_ai::transcript;
use rustls::{
    RootCertStore, ServerConfig,
    crypto::CryptoProvider,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
};
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
    task::JoinHandle,
    time::{sleep, timeout},
};
use tokio_rustls::TlsAcceptor;
use tokio_util::sync::CancellationToken;

use opi_ai::provider::{Request, ThinkingConfig};

// ---------------------------------------------------------------------------
// Loopback TLS server harness
// ---------------------------------------------------------------------------

/// The server-side key-exchange selection for a case.
enum KxProfile {
    /// Match the frozen client profile.
    Declared,
    /// Offer only P-521 (no overlap with the frozen client profile). The
    /// upstream rustls/aws-lc-rs backend intentionally does not ship a
    /// P-521 group (RDF-23 omits P-521 key exchange), so the loopback peer
    /// implements the group itself over the public aws-lc-rs ECDH API.
    P521Only,
}

#[derive(Default)]
struct Stats {
    active: AtomicUsize,
    accepts: AtomicUsize,
    handshakes: AtomicUsize,
    rejected: AtomicUsize,
    http: AtomicUsize,
    closed: AtomicUsize,
    peers: Mutex<Vec<serde_json::Value>>,
}

struct CountActive(Arc<Stats>);
impl Drop for CountActive {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);
    }
}

struct Server {
    url: String,
    stats: Arc<Stats>,
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

fn instance_provider(profile: &KxProfile) -> Arc<CryptoProvider> {
    use rustls::crypto::aws_lc_rs::kx_group::{SECP256R1, SECP384R1, X25519, X25519MLKEM768};
    let mut provider = rustls::crypto::aws_lc_rs::default_provider();
    provider.kx_groups = match profile {
        KxProfile::Declared => vec![X25519MLKEM768, X25519, SECP256R1, SECP384R1],
        KxProfile::P521Only => vec![&P521TestKx],
    };
    Arc::new(provider)
}

/// Loopback-only P-521 key-exchange group built on the public aws-lc-r
/// ECDH API. Exists solely so a test peer can offer a group the frozen
/// client profile deliberately omits; it is not part of the product.
#[derive(Debug)]
struct P521TestKx;

impl rustls::crypto::SupportedKxGroup for P521TestKx {
    fn start(&self) -> Result<Box<dyn rustls::crypto::ActiveKeyExchange>, rustls::Error> {
        let key = aws_lc_rs::agreement::PrivateKey::generate(&aws_lc_rs::agreement::ECDH_P521)
            .map_err(|_| rustls::Error::General("p521 test key generation".into()))?;
        let pub_key = key
            .compute_public_key()
            .map_err(|_| rustls::Error::General("p521 test public key".into()))?;
        Ok(Box::new(P521Active {
            key,
            pub_key: pub_key.as_ref().to_vec(),
        }))
    }

    fn name(&self) -> rustls::NamedGroup {
        rustls::NamedGroup::secp521r1
    }
}

struct P521Active {
    key: aws_lc_rs::agreement::PrivateKey,
    pub_key: Vec<u8>,
}

impl rustls::crypto::ActiveKeyExchange for P521Active {
    fn complete(
        self: Box<Self>,
        peer: &[u8],
    ) -> Result<rustls::crypto::SharedSecret, rustls::Error> {
        let peer_key =
            aws_lc_rs::agreement::UnparsedPublicKey::new(&aws_lc_rs::agreement::ECDH_P521, peer);
        aws_lc_rs::agreement::agree(
            &self.key,
            peer_key,
            rustls::Error::General("p521 test agreement".into()),
            |secret| Ok(rustls::crypto::SharedSecret::from(secret)),
        )
    }

    fn group(&self) -> rustls::NamedGroup {
        rustls::NamedGroup::secp521r1
    }

    fn pub_key(&self) -> &[u8] {
        &self.pub_key
    }
}

fn fingerprint(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn server_config(
    identity_cert: &str,
    identity_key: &str,
    tls13: bool,
    h2: bool,
    require_client_cert: bool,
    kx: &KxProfile,
) -> Result<ServerConfig, String> {
    let cert = CertificateDer::from_pem_slice(identity_cert.as_bytes())
        .map_err(|_| "cert pem".to_owned())?;
    let key =
        PrivateKeyDer::from_pem_slice(identity_key.as_bytes()).map_err(|_| "key pem".to_owned())?;
    let provider = instance_provider(kx);
    let version: &rustls::SupportedProtocolVersion = if tls13 {
        &rustls::version::TLS13
    } else {
        &rustls::version::TLS12
    };
    let builder = ServerConfig::builder_with_provider(provider.clone())
        .with_protocol_versions(&[version])
        .map_err(|_| "server versions".to_owned())?;
    let builder = if require_client_cert {
        let mut roots = RootCertStore::empty();
        let ca =
            CertificateDer::from_pem_slice(CERT_CA.as_bytes()).map_err(|_| "ca pem".to_owned())?;
        roots.add(ca).map_err(|_| "ca root".to_owned())?;
        let verifier =
            rustls::server::WebPkiClientVerifier::builder_with_provider(Arc::new(roots), provider)
                .build()
                .map_err(|_| "client verifier".to_owned())?;
        builder.with_client_cert_verifier(verifier)
    } else {
        builder.with_no_client_auth()
    };
    let mut config = builder
        .with_single_cert(vec![cert], key)
        .map_err(|_| "server identity".to_owned())?;
    config.alpn_protocols = vec![if h2 {
        b"h2".to_vec()
    } else {
        b"http/1.1".to_vec()
    }];
    Ok(config)
}

const SSE_BODY: &str = concat!(
    "data: {\"id\":\"chatcmpl-tls\",\"object\":\"chat.completion.chunk\",\"created\":1,",
    "\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},",
    "\"finish_reason\":null}]}\n\n",
    "data: {\"id\":\"chatcmpl-tls\",\"object\":\"chat.completion.chunk\",\"created\":1,",
    "\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],",
    "\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n",
);

async fn start_server(
    identity_cert: &str,
    identity_key: &str,
    tls13: bool,
    h2: bool,
    require_client_cert: bool,
    kx: KxProfile,
) -> Result<Server, String> {
    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|_| "bind")?;
    let addr = listener.local_addr().map_err(|_| "address")?;
    let url = format!("https://{addr}");
    let acceptor = TlsAcceptor::from(Arc::new(server_config(
        identity_cert,
        identity_key,
        tls13,
        h2,
        require_client_cert,
        &kx,
    )?));
    let stats = Arc::new(Stats::default());
    let s = stats.clone();
    let (tx, mut rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        let mut owned = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                _ = &mut rx => break,
                accepted = listener.accept() => {
                    let Ok((tcp, _)) = accepted else { break };
                    s.accepts.fetch_add(1, Ordering::SeqCst);
                    s.active.fetch_add(1, Ordering::SeqCst);
                    let acceptor = acceptor.clone();
                    let stats = s.clone();
                    owned.spawn(async move {
                        let _guard = CountActive(stats.clone());
                        let tls = match acceptor.accept(tcp).await {
                            Ok(tls) => tls,
                            Err(_) => {
                                stats.rejected.fetch_add(1, Ordering::SeqCst);
                                return;
                            }
                        };
                        stats.handshakes.fetch_add(1, Ordering::SeqCst);
                        let conn = tls.get_ref().1;
                        let peer = conn
                            .peer_certificates()
                            .and_then(|p| p.first())
                            .map(|c| fingerprint(c.as_ref()));
                        stats.peers.lock().unwrap().push(serde_json::json!({
                            "fingerprint": peer,
                            "alpn": conn
                                .alpn_protocol()
                                .map(|v| String::from_utf8_lossy(v).into_owned()),
                            "version": format!("{:?}", conn.protocol_version()),
                            "kx": format!("{:?}", conn.negotiated_key_exchange_group().map(|g| g.name())),
                        }));
                        let service_stats = stats.clone();
                        let service = service_fn(
                            move |req: HyperRequest<Incoming>| {
                                let stats = service_stats.clone();
                                async move {
                                    stats.http.fetch_add(1, Ordering::SeqCst);
                                    let _ = req.into_body().collect().await;
                                    Ok::<_, Infallible>(Response::builder()
                                        .status(200)
                                        .header("content-type", "text/event-stream")
                                        .body(Full::new(SSE_BODY.as_bytes()))
                                        .unwrap())
                                }
                            },
                        );
                        if h2 {
                            let _ = hyper::server::conn::http2::Builder::new(TokioExecutor::new())
                                .serve_connection(TokioIo::new(tls), service)
                                .await;
                        } else {
                            let _ = hyper::server::conn::http1::Builder::new()
                                .keep_alive(false)
                                .serve_connection(TokioIo::new(tls), service)
                                .await;
                        }
                        stats.closed.fetch_add(1, Ordering::SeqCst);
                    });
                }
                Some(_) = owned.join_next(), if !owned.is_empty() => {}
            }
        }
        owned.abort_all();
        while owned.join_next().await.is_some() {}
    });
    Ok(Server {
        url,
        stats,
        stop: tx,
        task,
    })
}

impl Server {
    async fn finish(self) -> Result<(), String> {
        let _ = self.stop.send(());
        let mut task = self.task;
        match timeout(Duration::from_secs(5), &mut task).await {
            Ok(joined) => joined.map_err(|_| "server owner panicked".to_owned()),
            Err(_) => {
                task.abort();
                let _ = task.await;
                Err("server join timeout".to_owned())
            }
        }
    }

    async fn wait_idle(&self) -> Result<(), String> {
        timeout(Duration::from_secs(5), async {
            while self.stats.active.load(Ordering::SeqCst) != 0 {
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .map_err(|_| "containment timeout".to_owned())
    }
}

// ---------------------------------------------------------------------------
// Client construction through the public builder seam
// ---------------------------------------------------------------------------

enum Trust<'a> {
    Explicit(&'a str),
    PinnedDefaultWithExtra(&'a str),
    HostSystem,
}

fn build_tls_client(
    trust: Trust<'_>,
    identity: Option<(&str, &str)>,
    http1_only: bool,
) -> Result<HttpClient, HttpClientBuildError> {
    let mut options = TlsOptions::new();
    options = match trust {
        Trust::Explicit(pem) => options.with_explicit_roots_pem(pem.as_bytes())?,
        Trust::PinnedDefaultWithExtra(pem) => options.with_extra_roots_pem(pem.as_bytes())?,
        Trust::HostSystem => options.use_host_system_roots(),
    };
    if let Some((chain, key)) = identity {
        options = options.with_client_identity_pem(chain.as_bytes(), key.as_bytes())?;
    }
    if http1_only {
        options = options.http1_only();
    }
    HttpClientBuilder::new()
        .max_idle_per_host(0)
        .tls(options)
        .build()
}

// ---------------------------------------------------------------------------
// Models -> Chat driver over the real injection path
// ---------------------------------------------------------------------------

fn chat_request() -> Request {
    Request {
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
        context: transcript::normalize_context(transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

async fn chat_models_call(
    server_url: &str,
    client: Arc<HttpClient>,
) -> Result<AssistantMessage, ProviderError> {
    chat_models_call_request(server_url, client, chat_request()).await
}

async fn chat_models_call_request(
    server_url: &str,
    client: Arc<HttpClient>,
    request: Request,
) -> Result<AssistantMessage, ProviderError> {
    let prepared = prepare_chat_request(server_url, client, request).await;
    let stream = prepared.start_attempt().expect("attempt start");
    ChatStream::new(stream)
        .result()
        .await
        .map_err(|failure| failure.into_parts().1)
}

async fn prepare_chat_request(
    server_url: &str,
    client: Arc<HttpClient>,
    request: Request,
) -> opi_ai::PreparedProviderCall {
    let provider = OpenAiChatProvider::with_client(
        Some(server_url.to_owned()),
        "openai".to_owned(),
        vec![],
        client,
    );
    let mut models = Models::new();
    models
        .register_route(
            Box::new(provider),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from("tls-contract-key"),
            )),
            AuthProvenanceSource::Static,
            opi_ai::models::CompatMetadata {
                openai_compatible: true,
                profile: None,
            },
        )
        .expect("route registration");
    models
        .prepare_call("openai:gpt-4o", request)
        .await
        .expect("call preparation")
}

/// Drain one attempt stream to its terminal, returning the collected error
/// (for negative cases).
async fn chat_models_error(server_url: &str, client: Arc<HttpClient>) -> ProviderError {
    let provider = OpenAiChatProvider::with_client(
        Some(server_url.to_owned()),
        "openai".to_owned(),
        vec![],
        client,
    );
    let mut models = Models::new();
    models
        .register_route(
            Box::new(provider),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from("tls-contract-key"),
            )),
            AuthProvenanceSource::Static,
            opi_ai::models::CompatMetadata {
                openai_compatible: true,
                profile: None,
            },
        )
        .expect("route registration");
    let prepared = models
        .prepare_call("openai:gpt-4o", chat_request())
        .await
        .expect("call preparation");
    let mut stream: EventStream = prepared.start_attempt().expect("attempt start");
    while let Some(item) = stream.next().await {
        if let Err(error) = item {
            return error;
        }
    }
    panic!("negative TLS case completed without a stream error");
}

// ---------------------------------------------------------------------------
// Positives: negotiated protocol, versions, identity profiles
// ---------------------------------------------------------------------------

async fn assert_round_trip(
    server: &Server,
    trust: Trust<'_>,
    identity: Option<(&str, &str)>,
    http1_only: bool,
    expect_alpn: &str,
) -> Result<AssistantMessage, String> {
    let client = Arc::new(
        build_tls_client(trust, identity, http1_only).map_err(|e| format!("client build: {e}"))?,
    );
    let message = chat_models_call(&server.url, client.clone())
        .await
        .map_err(|e| format!("chat call: {e:?}"))?;
    drop(client);
    server.wait_idle().await?;
    let peers = server.stats.peers.lock().unwrap().clone();
    let alpn_ok = peers.len() == 1
        && peers[0]["alpn"].as_str() == Some(expect_alpn)
        && server.stats.http.load(Ordering::SeqCst) == 1
        && server.stats.accepts.load(Ordering::SeqCst) == 1;
    if !alpn_ok {
        return Err(format!("alpn/http observation mismatch: {peers:?}"));
    }
    Ok(message)
}

#[tokio::test]
async fn chat_tls13_h2_negotiation_with_explicit_roots() {
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        true,
        false,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let outcome = assert_round_trip(&server, Trust::Explicit(CERT_CA), None, false, "h2").await;
    server.finish().await.expect("server finish");
    let message = outcome.expect("round trip");
    assert_eq!(
        message.response_id.as_deref(),
        Some("chatcmpl-tls"),
        "chat terminal must carry the loopback response id"
    );
}

#[tokio::test]
async fn chat_tls13_h1_explicit_selection_with_extra_roots() {
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        false,
        false,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let outcome = assert_round_trip(
        &server,
        Trust::PinnedDefaultWithExtra(CERT_CA),
        None,
        true,
        "http/1.1",
    )
    .await;
    server.finish().await.expect("server finish");
    outcome.expect("round trip");
}

#[tokio::test]
async fn chat_tls12_h2_negotiation() {
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        false,
        true,
        false,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let outcome = assert_round_trip(&server, Trust::Explicit(CERT_CA), None, false, "h2").await;
    let stats = server.stats.clone();
    server.finish().await.expect("server finish");
    let message = outcome.expect("round trip");
    let peers = stats.peers.lock().unwrap().clone();
    assert_eq!(
        peers[0]["version"].as_str(),
        Some("Some(TLSv1_2)"),
        "server must observe TLS 1.2"
    );
    assert!(message.response_id.is_some());
}

#[tokio::test]
async fn admitted_client_identity_profiles_over_mtls() {
    for (label, cert, key) in [
        ("rsa2048", CERT_CLIENT_RSA2048, KEY_CLIENT_RSA2048),
        ("rsa-e3", CERT_CLIENT_RSA_E3, KEY_CLIENT_RSA_E3),
        ("rsa8192", CERT_CLIENT_RSA8192, KEY_CLIENT_RSA8192),
        ("p521", CERT_CLIENT_P521, KEY_CLIENT_P521),
    ] {
        let server = start_server(
            CERT_SERVER_RSA,
            KEY_SERVER_RSA,
            true,
            true,
            true,
            KxProfile::Declared,
        )
        .await
        .expect("server start");
        let outcome = assert_round_trip(
            &server,
            Trust::Explicit(CERT_CA),
            Some((cert, key)),
            false,
            "h2",
        )
        .await;
        let stats = server.stats.clone();
        server.finish().await.expect("server finish");
        outcome.unwrap_or_else(|e| panic!("{label} identity: {e}"));
        let peers = stats.peers.lock().unwrap().clone();
        let expected = fingerprint(
            CertificateDer::from_pem_slice(cert.as_bytes())
                .expect("identity cert")
                .as_ref(),
        );
        assert_eq!(
            peers[0]["fingerprint"].as_str(),
            Some(expected.as_str()),
            "{label} peer certificate fingerprint mismatch"
        );
    }
}

#[tokio::test]
async fn p521_server_identity_and_client_identity() {
    let server = start_server(
        CERT_SERVER_P521,
        KEY_SERVER_P521,
        true,
        true,
        true,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let outcome = assert_round_trip(
        &server,
        Trust::Explicit(CERT_CA),
        Some((CERT_CLIENT_RSA2048, KEY_CLIENT_RSA2048)),
        false,
        "h2",
    )
    .await;
    server.finish().await.expect("server finish");
    outcome.expect("round trip");
}

#[tokio::test]
async fn connection_source_identity_isolation() {
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        true,
        true,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let identities = [
        (CERT_CLIENT_RSA2048, KEY_CLIENT_RSA2048),
        (CERT_CLIENT_P521, KEY_CLIENT_P521),
    ];
    for (cert, key) in identities {
        let client = Arc::new(
            build_tls_client(Trust::Explicit(CERT_CA), Some((cert, key)), false)
                .expect("client build"),
        );
        chat_models_call(&server.url, client.clone())
            .await
            .expect("identity round trip");
        drop(client);
        server.wait_idle().await.expect("idle");
    }
    let stats = server.stats.clone();
    server.finish().await.expect("server finish");
    let peers = stats.peers.lock().unwrap().clone();
    let first = fingerprint(
        CertificateDer::from_pem_slice(CERT_CLIENT_RSA2048.as_bytes())
            .expect("cert")
            .as_ref(),
    );
    let second = fingerprint(
        CertificateDer::from_pem_slice(CERT_CLIENT_P521.as_bytes())
            .expect("cert")
            .as_ref(),
    );
    assert_eq!(stats.http.load(Ordering::SeqCst), 2);
    assert_eq!(stats.accepts.load(Ordering::SeqCst), 2);
    assert_eq!(peers.len(), 2);
    assert_eq!(peers[0]["fingerprint"].as_str(), Some(first.as_str()));
    assert_eq!(peers[1]["fingerprint"].as_str(), Some(second.as_str()));
    assert_ne!(
        peers[0]["fingerprint"].as_str(),
        peers[1]["fingerprint"].as_str(),
        "source identities must not mix between connection owners"
    );
}

// ---------------------------------------------------------------------------
// Negatives: trust, hostname, time, EKU, ALPN, key exchange — zero business HTTP
// ---------------------------------------------------------------------------

async fn assert_negative_zero_http(
    server: &Server,
    trust: Trust<'_>,
    identity: Option<(&str, &str)>,
    http1_only: bool,
) -> Result<(), String> {
    let client = Arc::new(
        build_tls_client(trust, identity, http1_only).map_err(|e| format!("client build: {e}"))?,
    );
    let error = chat_models_error(&server.url, client.clone()).await;
    drop(client);
    server.wait_idle().await?;
    let ProviderError::Network(summary) = &error else {
        return Err(format!(
            "expected typed redacted Network failure, got {error:?}"
        ));
    };
    assert!(
        !summary.as_str().is_empty(),
        "network failure must carry a bounded redacted summary"
    );
    let rendered = format!("{error:?}");
    assert!(
        !rendered.contains("PRIVATE"),
        "network failure must stay redacted: {rendered}"
    );
    if server.stats.http.load(Ordering::SeqCst) != 0
        || server.stats.accepts.load(Ordering::SeqCst) > 1
    {
        return Err(format!(
            "TLS negative reached business HTTP or retried: http={}, accepts={}",
            server.stats.http.load(Ordering::SeqCst),
            server.stats.accepts.load(Ordering::SeqCst)
        ));
    }
    Ok(())
}

#[tokio::test]
async fn untrusted_server_root_fails_zero_http() {
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        true,
        false,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let outcome =
        assert_negative_zero_http(&server, Trust::Explicit(CERT_OTHER_CA), None, false).await;
    let stats = server.stats.clone();
    server.finish().await.expect("server finish");
    outcome.expect("negative");
    assert_eq!(stats.rejected.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn wrong_hostname_and_expired_server_fail_zero_http() {
    for (label, cert, key) in [
        (
            "wrong-hostname",
            CERT_SERVER_WRONGHOST,
            KEY_SERVER_WRONGHOST,
        ),
        ("expired", CERT_SERVER_EXPIRED, KEY_SERVER_EXPIRED),
    ] {
        let server = start_server(cert, key, true, true, false, KxProfile::Declared)
            .await
            .expect("server start");
        let outcome =
            assert_negative_zero_http(&server, Trust::Explicit(CERT_CA), None, false).await;
        server.finish().await.expect("server finish");
        outcome.unwrap_or_else(|e| panic!("{label}: {e}"));
    }
}

#[tokio::test]
async fn host_system_roots_do_not_trust_the_loopback_ca() {
    // Opt-in host-root mode is scoped to this client only. The synthetic
    // loopback CA is not installed in the host store, so verification must
    // fail closed with zero business HTTP (no fallback to another trust
    // mode). Positive host-root trust stays with real host CAs and is not
    // claimed here.
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        true,
        false,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let outcome = assert_negative_zero_http(&server, Trust::HostSystem, None, false).await;
    server.finish().await.expect("server finish");
    outcome.expect("negative");
}

#[tokio::test]
async fn wrong_client_identities_fail_zero_http() {
    for (label, identity) in [
        ("missing", None),
        (
            "untrusted",
            Some((CERT_CLIENT_UNTRUSTED, KEY_CLIENT_UNTRUSTED)),
        ),
        ("expired", Some((CERT_CLIENT_EXPIRED, KEY_CLIENT_EXPIRED))),
        (
            "wrong-eku",
            Some((CERT_CLIENT_WRONG_EKU, KEY_CLIENT_WRONG_EKU)),
        ),
    ] {
        let server = start_server(
            CERT_SERVER_RSA,
            KEY_SERVER_RSA,
            true,
            true,
            true,
            KxProfile::Declared,
        )
        .await
        .expect("server start");
        let outcome =
            assert_negative_zero_http(&server, Trust::Explicit(CERT_CA), identity, false).await;
        server.finish().await.expect("server finish");
        outcome.unwrap_or_else(|e| panic!("{label} client identity: {e}"));
    }
}

#[tokio::test]
async fn alpn_no_overlap_fails_zero_http() {
    // The client is explicitly H1-only; the server offers only h2.
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        true,
        false,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let outcome = assert_negative_zero_http(&server, Trust::Explicit(CERT_CA), None, true).await;
    let stats = server.stats.clone();
    server.finish().await.expect("server finish");
    outcome.expect("negative");
    assert!(
        stats.handshakes.load(Ordering::SeqCst) == 0 || stats.http.load(Ordering::SeqCst) == 0,
        "no negotiated connection may carry business HTTP"
    );
}

#[tokio::test]
async fn p521_only_key_exchange_fails_zero_http() {
    // The server offers only P-521 key exchange, which the frozen client
    // profile does not include: the handshake must fail with zero business
    // HTTP (typed redacted transport failure), never downgrade.
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        true,
        false,
        KxProfile::P521Only,
    )
    .await
    .expect("server start");
    let outcome = assert_negative_zero_http(&server, Trust::Explicit(CERT_CA), None, false).await;
    let stats = server.stats.clone();
    server.finish().await.expect("server finish");
    outcome.expect("negative");
    assert_eq!(
        stats.handshakes.load(Ordering::SeqCst),
        0,
        "no handshake may complete against a no-common-group peer"
    );
}

// ---------------------------------------------------------------------------
// Pre-I/O admission and assembly failures (typed, redacted, no socket)
// ---------------------------------------------------------------------------

#[test]
fn unsupported_client_keys_fail_typed_at_admission() {
    let chain = CERT_CLIENT_RSA2048;
    let cases = [
        ("rsa-1024", KEY_PEM_RSA1024, TlsKeyError::BackendLoad),
        ("multi-prime", KEY_PEM_MULTIPRIME, TlsKeyError::BackendLoad),
        ("encrypted", KEY_PEM_ENCRYPTED, TlsKeyError::Decode),
    ];
    for (label, key, expected) in cases {
        let error = TlsOptions::new()
            .with_client_identity_pem(chain.as_bytes(), key.as_bytes())
            .expect_err("unsupported key must fail admission");
        assert_eq!(error, TlsConfigError::KeyAdmission(expected), "{label}");
        let rendered = format!("{error:?}");
        assert!(
            !rendered.contains("PRIVATE") && !rendered.contains("BEGIN"),
            "admission failure must stay redacted: {rendered}"
        );
    }
}

#[test]
fn root_input_negatives_fail_typed() {
    assert_eq!(
        TlsOptions::new()
            .with_explicit_roots_pem(b"not a pem")
            .unwrap_err(),
        TlsConfigError::RootDecode
    );
    assert_eq!(
        TlsOptions::new().with_explicit_roots_pem(b"").unwrap_err(),
        TlsConfigError::RootDecode
    );
    assert_eq!(
        TlsOptions::new().with_extra_roots_pem(b"").unwrap_err(),
        TlsConfigError::RootDecode
    );
}

#[tokio::test]
async fn mismatched_identity_fails_before_any_socket() {
    // RSA-2048 certificate paired with a P-521 key: the identity must be
    // rejected at build, before any connection exists.
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        true,
        true,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let error = build_tls_client(
        Trust::Explicit(CERT_CA),
        Some((CERT_CLIENT_RSA2048, KEY_CLIENT_P521)),
        false,
    )
    .expect_err("mismatched identity must fail at build");
    assert!(
        matches!(
            error,
            HttpClientBuildError::Tls(TlsConfigError::IdentityAssembly)
        ),
        "unexpected error: {error:?}"
    );
    assert_eq!(
        server.stats.accepts.load(Ordering::SeqCst),
        0,
        "no socket may be opened for a rejected identity"
    );
    server.finish().await.expect("server finish");
}

#[test]
fn build_error_displays_redacted() {
    let error = build_tls_client(
        Trust::Explicit("not a pem"),
        Some((CERT_CLIENT_RSA2048, KEY_PEM_RSA1024)),
        false,
    )
    .expect_err("invalid inputs");
    let rendered = format!("{error}");
    assert!(
        rendered.contains("TLS configuration rejected"),
        "{rendered}"
    );
    assert!(!rendered.contains("PRIVATE"), "{rendered}");
}

// ---------------------------------------------------------------------------
// Cancellation, partial bodies, EOF and owned cleanup
// ---------------------------------------------------------------------------

async fn raw_cancel_case(
    response_phase: bool,
    abort: bool,
    accepts_or_active: Arc<AtomicUsize>,
    eof: Arc<AtomicUsize>,
    late_bytes: Arc<AtomicUsize>,
    ready: Arc<tokio::sync::Notify>,
) -> Result<String, String> {
    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|_| "bind")?;
    let addr = listener.local_addr().map_err(|_| "address")?;
    let acceptor = TlsAcceptor::from(Arc::new(
        server_config(
            CERT_SERVER_RSA,
            KEY_SERVER_RSA,
            true,
            false,
            true,
            &KxProfile::Declared,
        )
        .expect("cancel server config"),
    ));
    let r = ready.clone();
    let a = accepts_or_active.clone();
    let e = eof.clone();
    let late = late_bytes.clone();
    let mut server = tokio::spawn(async move {
        let (mut tcp, _) = listener.accept().await.map_err(|_| "accept".to_owned())?;
        a.fetch_add(1, Ordering::SeqCst);
        struct Guard(Arc<AtomicUsize>);
        impl Drop for Guard {
            fn drop(&mut self) {
                self.0.fetch_sub(1, Ordering::SeqCst);
            }
        }
        let _guard = Guard(a.clone());
        if response_phase {
            let mut tls = acceptor
                .accept(tcp)
                .await
                .map_err(|_| "handshake".to_owned())?;
            let mut data = Vec::new();
            let mut buf = [0u8; 1024];
            loop {
                let n = tls
                    .read(&mut buf)
                    .await
                    .map_err(|_| "request read".to_owned())?;
                if n == 0 {
                    return Err("cancelled before request headers".to_owned());
                }
                data.extend_from_slice(&buf[..n]);
                if let Some(position) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                    let length = String::from_utf8_lossy(&data[..position])
                        .lines()
                        .find_map(|line| {
                            line.split_once(':')
                                .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                                .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    let expected = position + 4 + length;
                    while data.len() < expected {
                        let n = tls
                            .read(&mut buf)
                            .await
                            .map_err(|_| "request body".to_owned())?;
                        if n == 0 {
                            return Err("truncated request body".to_owned());
                        }
                        data.extend_from_slice(&buf[..n]);
                    }
                    late.fetch_add(data.len() - expected, Ordering::SeqCst);
                    break;
                }
            }
            r.notify_one();
            loop {
                match tls.read(&mut buf).await {
                    Ok(0) | Err(_) => {
                        e.fetch_add(1, Ordering::SeqCst);
                        break;
                    }
                    Ok(n) => {
                        late.fetch_add(n, Ordering::SeqCst);
                    }
                }
            }
        } else {
            let mut buf = [0u8; 4096];
            let first = tcp
                .read(&mut buf)
                .await
                .map_err(|_| "hello read".to_owned())?;
            if first == 0 {
                return Err("no ClientHello".to_owned());
            }
            r.notify_one();
            loop {
                match tcp.read(&mut buf).await {
                    Ok(0) | Err(_) => {
                        e.fetch_add(1, Ordering::SeqCst);
                        break;
                    }
                    Ok(_) => {}
                }
            }
        }
        if tokio::time::timeout(Duration::from_millis(75), listener.accept())
            .await
            .is_ok()
        {
            return Err("late connection after cancellation".to_owned());
        }
        Ok::<_, String>(())
    });
    let client = Arc::new(
        build_tls_client(
            Trust::Explicit(CERT_CA),
            Some((CERT_CLIENT_RSA2048, KEY_CLIENT_RSA2048)),
            false,
        )
        .expect("client"),
    );
    let url = format!("https://{addr}/paused");
    if abort {
        // Abort path: the owning task is aborted after the server reports
        // readiness; the client and its identity stay owned by the task.
        let c = client.clone();
        let u = url.clone();
        let task = tokio::spawn(async move { chat_models_call(&u, c).await });
        if timeout(Duration::from_secs(5), ready.notified())
            .await
            .is_err()
        {
            task.abort();
            let _ = task.await;
            server.abort();
            let _ = (&mut server).await;
            return Err("cancel ready".to_owned());
        }
        task.abort();
        if !task.await.is_err_and(|e| e.is_cancelled()) {
            server.abort();
            let _ = (&mut server).await;
            return Err("request owner not cancelled".to_owned());
        }
    } else {
        // Drop path: the request future (and then the whole client) is
        // dropped after the server reports readiness.
        let c = client.clone();
        let mut request = Box::pin({
            let c = c.clone();
            let u = url.clone();
            async move { chat_models_call(&u, c).await }
        });
        tokio::select! {
            _ = ready.notified() => {}
            _ = &mut request => {
                server.abort();
                let _ = (&mut server).await;
                return Err("request ended before controlled drop".to_owned());
            }
        }
        drop(request);
        drop(client);
    }
    match timeout(Duration::from_secs(5), &mut server).await {
        Ok(joined) => joined.map_err(|_| "cancel server task".to_owned())??,
        Err(_) => {
            server.abort();
            let _ = (&mut server).await;
            return Err("cancel EOF timeout".to_owned());
        }
    }
    if accepts_or_active.load(Ordering::SeqCst) != 0
        || eof.load(Ordering::SeqCst) != 1
        || late_bytes.load(Ordering::SeqCst) != 0
    {
        return Err("cancel cleanup counters".to_owned());
    }
    Ok(format!(
        "phase={} method={}",
        if response_phase {
            "awaiting response"
        } else {
            "TLS handshake"
        },
        if abort {
            "abort+await owned task"
        } else {
            "drop future+client"
        }
    ))
}

#[tokio::test]
async fn cancellation_during_handshake_and_response_releases_transport() {
    for response_phase in [false, true] {
        for abort in [false, true] {
            let counters = Arc::new(AtomicUsize::new(0));
            let eof = Arc::new(AtomicUsize::new(0));
            let late = Arc::new(AtomicUsize::new(0));
            let ready = Arc::new(tokio::sync::Notify::new());
            let outcome = raw_cancel_case(response_phase, abort, counters, eof, late, ready).await;
            outcome.unwrap_or_else(|e| {
                panic!(
                    "cancel {} {} case: {e}",
                    if response_phase {
                        "response"
                    } else {
                        "handshake"
                    },
                    if abort { "abort" } else { "drop" }
                )
            });
        }
    }
}

async fn partial_body_case(abort: bool) -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|_| "bind")?;
    let addr = listener.local_addr().map_err(|_| "address")?;
    let active = Arc::new(AtomicUsize::new(0));
    let eof = Arc::new(AtomicUsize::new(0));
    let late_bytes = Arc::new(AtomicUsize::new(0));
    let acceptor = TlsAcceptor::from(Arc::new(
        server_config(
            CERT_SERVER_RSA,
            KEY_SERVER_RSA,
            true,
            false,
            true,
            &KxProfile::Declared,
        )
        .map_err(|e| e.to_owned())?,
    ));
    let a = active.clone();
    let e = eof.clone();
    let late = late_bytes.clone();
    let mut server = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.map_err(|_| "accept".to_owned())?;
        a.fetch_add(1, Ordering::SeqCst);
        struct Guard(Arc<AtomicUsize>);
        impl Drop for Guard {
            fn drop(&mut self) {
                self.0.fetch_sub(1, Ordering::SeqCst);
            }
        }
        let _guard = Guard(a.clone());
        let mut tls = acceptor
            .accept(tcp)
            .await
            .map_err(|_| "handshake".to_owned())?;
        let mut data = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            let n = tls.read(&mut buf).await.map_err(|_| "headers".to_owned())?;
            if n == 0 {
                return Err("missing request".to_owned());
            }
            data.extend_from_slice(&buf[..n]);
            if let Some(position) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                late.fetch_add(data.len() - (position + 4), Ordering::SeqCst);
                break;
            }
        }
        tls.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Length: 100000\r\nConnection: close\r\n\r\npartial",
        )
        .await
        .map_err(|_| "response write".to_owned())?;
        tls.flush().await.map_err(|_| "flush".to_owned())?;
        loop {
            match tls.read(&mut buf).await {
                Ok(0) | Err(_) => {
                    e.fetch_add(1, Ordering::SeqCst);
                    break;
                }
                Ok(n) => {
                    late.fetch_add(n, Ordering::SeqCst);
                }
            }
        }
        if tokio::time::timeout(Duration::from_millis(75), listener.accept())
            .await
            .is_ok()
        {
            return Err("late body connection".to_owned());
        }
        Ok::<_, String>(())
    });
    let client = Arc::new(
        build_tls_client(
            Trust::Explicit(CERT_CA),
            Some((CERT_CLIENT_RSA2048, KEY_CLIENT_RSA2048)),
            false,
        )
        .map_err(|e| e.to_string())?,
    );
    let url = format!("https://{addr}/partial");
    let pending = Arc::new(AtomicUsize::new(0));
    let outcome = async {
        let mut response = client
            .client()
            .get(&url)
            .send()
            .await
            .map_err(|_| "partial HTTPS".to_owned())?;
        let first = response
            .chunk()
            .await
            .map_err(|_| "first chunk")?
            .ok_or("missing first chunk")?;
        if first.as_ref() != b"partial" {
            return Err("partial bytes changed".to_owned());
        }
        if abort {
            let ready = Arc::new(tokio::sync::Notify::new());
            let r = ready.clone();
            let p = pending.clone();
            let owned = client.clone();
            let task = tokio::spawn(async move {
                let _owner = owned;
                let mut next = Box::pin(response.chunk());
                if !futures_util::poll!(&mut next).is_pending() {
                    return Err("body completed before controlled abort".to_owned());
                }
                p.fetch_add(1, Ordering::SeqCst);
                r.notify_one();
                next.await
                    .map(|_| ())
                    .map_err(|_| "pending body read".to_owned())
            });
            if timeout(Duration::from_secs(5), ready.notified())
                .await
                .is_err()
            {
                task.abort();
                let _ = task.await;
                return Err("body owner ready".to_owned());
            }
            task.abort();
            if !task.await.is_err_and(|e| e.is_cancelled()) {
                return Err("body owner not cancelled".to_owned());
            }
        } else {
            {
                let mut next = Box::pin(response.chunk());
                if !futures_util::poll!(&mut next).is_pending() {
                    return Err("body completed before controlled drop".to_owned());
                }
                pending.fetch_add(1, Ordering::SeqCst);
            }
            drop(response);
        }
        Ok::<_, String>(())
    }
    .await;
    outcome.map_err(|e| e.to_string())?;
    drop(client);
    match timeout(Duration::from_secs(5), &mut server).await {
        Ok(joined) => joined.map_err(|_| "body server task".to_owned())??,
        Err(_) => {
            server.abort();
            let _ = server.await;
            return Err("body EOF timeout".to_owned());
        }
    }
    if active.load(Ordering::SeqCst) != 0
        || eof.load(Ordering::SeqCst) != 1
        || pending.load(Ordering::SeqCst) != 1
        || late_bytes.load(Ordering::SeqCst) != 0
    {
        return Err("body cleanup or pending/late counters".to_owned());
    }
    Ok(())
}

#[tokio::test]
async fn accepted_partial_body_cancellation_cleanup() {
    partial_body_case(false).await.expect("drop case");
    partial_body_case(true).await.expect("abort case");
}

#[tokio::test]
async fn public_tls_partial_cancellation_preserves_known_usage_and_eof() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "https://localhost:{}",
        listener.local_addr().unwrap().port()
    );
    let config = server_config(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        false,
        false,
        &KxProfile::Declared,
    )
    .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let peer = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let mut tls = acceptor.accept(tcp).await.unwrap();
        let mut data = Vec::new();
        let mut bytes = [0; 4096];
        loop {
            let n = tls.read(&mut bytes).await.unwrap();
            assert_ne!(n, 0);
            data.extend_from_slice(&bytes[..n]);
            if let Some(at) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                let length = String::from_utf8_lossy(&data[..at])
                    .lines()
                    .find_map(|line| {
                        line.split_once(':')
                            .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                            .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap();
                if data.len() >= at + 4 + length {
                    break;
                }
            }
        }
        let event = concat!(
            "data: {\"id\":\"partial-tls\",\"model\":\"gpt-4o\",",
            "\"choices\":[{\"index\":0,\"delta\":{\"content\":\"retained\"},\"finish_reason\":null}],",
            "\"usage\":{\"prompt_tokens\":7,\"completion_tokens\":3}}\n\n"
        );
        tls.write_all(format!("HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\n{:x}\r\n{event}\r\n", event.len()).as_bytes()).await.unwrap();
        tls.flush().await.unwrap();
        // Only the connection shutdown may follow cancellation; no further
        // business bytes or reconnect are allowed.
        assert!(matches!(tls.read(&mut bytes).await, Ok(0) | Err(_)));
        assert!(
            timeout(Duration::from_millis(75), listener.accept())
                .await
                .is_err()
        );
    });
    let request = chat_request();
    let cancel = request.cancel.clone();
    let client = Arc::new(build_tls_client(Trust::Explicit(CERT_CA), None, true).unwrap());
    let call = prepare_chat_request(&url, client, request).await;
    let mut stream = ChatStream::new(call.start_attempt().unwrap());
    loop {
        let event = timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        if let opi_ai::AssistantStreamEvent::TextDelta { delta, partial, .. } = event {
            assert_eq!(delta, "retained");
            assert!(partial.usage.reported);
            assert_eq!(
                (partial.usage.input_tokens, partial.usage.output_tokens),
                (7, 3)
            );
            break;
        }
    }
    cancel.cancel();
    let failure = timeout(Duration::from_secs(5), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(matches!(failure.cause(), ProviderError::Cancelled));
    assert_eq!(
        failure.message().stop_reason,
        opi_ai::stream::StopReason::Aborted
    );
    assert!(failure.message().usage.reported);
    assert_eq!(
        (
            failure.message().usage.input_tokens,
            failure.message().usage.output_tokens
        ),
        (7, 3)
    );
    assert!(
        matches!(&failure.message().content[0], opi_ai::message::AssistantContent::Text { text } if text == "retained")
    );
    assert!(stream.next().await.is_none());
    assert!(stream.next().await.is_none());
    assert!(matches!(
        call.start_attempt(),
        Err(opi_ai::ModelsError::CallCancelled)
    ));
    drop(stream);
    timeout(Duration::from_secs(5), peer)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn chat_level_cancellation_stops_the_attempt_without_extra_sends() {
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        false,
        false,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let client = Arc::new(build_tls_client(Trust::Explicit(CERT_CA), None, false).expect("client"));
    let provider = OpenAiChatProvider::with_client(
        Some(server.url.clone()),
        "openai".to_owned(),
        vec![],
        client,
    );
    let mut models = Models::new();
    models
        .register_route(
            Box::new(provider),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from("tls-contract-key"),
            )),
            AuthProvenanceSource::Static,
            opi_ai::models::CompatMetadata {
                openai_compatible: true,
                profile: None,
            },
        )
        .unwrap();
    let request = chat_request();
    let token = request.cancel.clone();
    let prepared = models
        .prepare_call("openai:gpt-4o", request)
        .await
        .expect("prepare");
    let mut stream: EventStream = prepared.start_attempt().expect("attempt");
    // Wait for the first stream event (the request has been sent and the
    // response has begun), then cancel mid-stream and drop the attempt.
    let first = timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("first event timeout")
        .expect("stream must produce a first event")
        .expect("first event must not be an error");
    assert!(
        !first.is_terminal(),
        "premature terminal before cancellation"
    );
    token.cancel();
    let mut saw_terminal = false;
    while let Some(item) = timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("stream must terminate after cancellation")
    {
        match item {
            Ok(event) if event.is_terminal() => saw_terminal = true,
            Err(ProviderError::Cancelled) => saw_terminal = true,
            Err(ProviderError::StreamFailure(failure))
                if matches!(failure.cause(), ProviderError::Cancelled) =>
            {
                saw_terminal = true
            }
            Err(other) => panic!("unexpected stream error: {other:?}"),
            _ => {}
        }
    }
    drop(stream);
    assert!(saw_terminal, "cancelled attempt must reach one terminal");
    let stats = server.stats.clone();
    server.finish().await.expect("server finish");
    assert_eq!(
        stats.http.load(Ordering::SeqCst),
        1,
        "exactly one request before cancellation"
    );
    assert_eq!(
        stats.accepts.load(Ordering::SeqCst),
        1,
        "cancellation must not trigger a second send"
    );
}

// ---------------------------------------------------------------------------
// Process-global provider controls
// ---------------------------------------------------------------------------

#[tokio::test]
async fn instance_tls_never_installs_a_process_global_provider() {
    let before = CryptoProvider::get_default();
    let _default_client = HttpClient::new();
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        true,
        false,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let outcome = assert_round_trip(&server, Trust::Explicit(CERT_CA), None, false, "h2").await;
    server.finish().await.expect("server finish");
    outcome.expect("round trip");
    match (before, CryptoProvider::get_default()) {
        (None, None) => {}
        (Some(a), Some(b)) => assert!(Arc::ptr_eq(a, b), "global provider changed"),
        _ => panic!("global provider appeared or disappeared"),
    }
}

#[tokio::test]
async fn default_roots_reject_unconfigured_loopback_ca_without_business_http() {
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        true,
        false,
        KxProfile::Declared,
    )
    .await
    .unwrap();
    let error = timeout(
        Duration::from_secs(5),
        chat_models_error(&server.url, Arc::new(HttpClient::new())),
    )
    .await
    .unwrap();
    assert!(matches!(error, ProviderError::Network(_)), "{error:?}");
    assert_eq!(server.stats.http.load(Ordering::SeqCst), 0);
    assert_eq!(server.stats.accepts.load(Ordering::SeqCst), 1);
    server.finish().await.unwrap();
}

fn ring_control_inner() {
    // Host control: a preinstalled process-global ring provider must not
    // change the instance TLS path.
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("fresh control process");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("control runtime");
    runtime.block_on(async {
        assert!(CryptoProvider::get_default().is_some());
        let server = start_server(
            CERT_SERVER_RSA,
            KEY_SERVER_RSA,
            true,
            true,
            true,
            KxProfile::Declared,
        )
        .await
        .expect("server start");
        let outcome = assert_round_trip(
            &server,
            Trust::Explicit(CERT_CA),
            Some((CERT_CLIENT_RSA2048, KEY_CLIENT_RSA2048)),
            false,
            "h2",
        )
        .await;
        let stats = server.stats.clone();
        server.finish().await.expect("server finish");
        outcome.expect("instance TLS under a host ring provider");
        assert_eq!(
            stats.http.load(Ordering::SeqCst),
            1,
            "control round trip must carry business HTTP"
        );
        assert!(CryptoProvider::get_default().is_some());
    });
}

#[test]
fn host_preinstalled_ring_provider_control() {
    if std::env::var("OPI_TLS_RING_CONTROL").as_deref() == Ok("run") {
        ring_control_inner();
        return;
    }
    // Parent: rerun this exact test in a fresh child process with the
    // control marker set.
    let status = std::process::Command::new(std::env::current_exe().expect("test binary"))
        .args([
            "--exact",
            "host_preinstalled_ring_provider_control",
            "--nocapture",
        ])
        .env("OPI_TLS_RING_CONTROL", "run")
        .status()
        .expect("control subprocess");
    assert!(status.success(), "host ring control subprocess failed");
}

#[test]
fn default_tls_ignores_host_provider_profile() {
    if std::env::var("OPI_TLS_DEFAULT_CONTROL").as_deref() != Ok("run") {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "default_tls_ignores_host_provider_profile",
                "--nocapture",
            ])
            .env("OPI_TLS_DEFAULT_CONTROL", "run")
            .status()
            .unwrap();
        assert!(status.success(), "default TLS profile control failed");
        return;
    }
    let mut host = rustls::crypto::aws_lc_rs::default_provider();
    host.kx_groups = vec![rustls::crypto::aws_lc_rs::kx_group::X25519];
    host.install_default().unwrap();
    let before = CryptoProvider::get_default().unwrap().clone();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            for client in [
                HttpClient::new(),
                HttpClientBuilder::new()
                    .tls(TlsOptions::new())
                    .build()
                    .unwrap(),
            ] {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let url = format!(
                    "https://localhost:{}",
                    listener.local_addr().unwrap().port()
                );
                let peer = tokio::spawn(async move {
                    let (mut tcp, _) = listener.accept().await.unwrap();
                    let mut acceptor = rustls::server::Acceptor::default();
                    let mut bytes = [0; 8192];
                    loop {
                        let n = tcp.read(&mut bytes).await.unwrap();
                        assert_ne!(n, 0, "ClientHello expected");
                        acceptor
                            .read_tls(&mut std::io::Cursor::new(&bytes[..n]))
                            .unwrap();
                        if let Some(accepted) = acceptor.accept().unwrap() {
                            return accepted.client_hello().named_groups().unwrap().to_vec();
                        }
                    }
                });
                assert!(
                    timeout(
                        Duration::from_secs(5),
                        chat_models_call(&url, Arc::new(client))
                    )
                    .await
                    .unwrap()
                    .is_err()
                );
                assert_eq!(
                    peer.await.unwrap(),
                    vec![
                        rustls::NamedGroup::X25519MLKEM768,
                        rustls::NamedGroup::X25519,
                        rustls::NamedGroup::secp256r1,
                        rustls::NamedGroup::secp384r1,
                    ]
                );
            }
        });
    assert!(Arc::ptr_eq(&before, CryptoProvider::get_default().unwrap()));
}

#[tokio::test]
async fn h2_refused_stream_does_not_trigger_backend_retry() {
    let mut config = ServerConfig::builder_with_provider(instance_provider(&KxProfile::Declared))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from_pem_slice(CERT_SERVER_RSA.as_bytes()).unwrap()],
            PrivateKeyDer::from_pem_slice(KEY_SERVER_RSA.as_bytes()).unwrap(),
        )
        .unwrap();
    config.alpn_protocols = vec![b"h2".to_vec()];
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "https://localhost:{}",
        listener.local_addr().unwrap().port()
    );
    let sends = Arc::new(AtomicUsize::new(0));
    let counter = sends.clone();
    let peer = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let tls = acceptor.accept(tcp).await.unwrap();
        let mut connection = h2::server::handshake(tls).await.unwrap();
        while let Some(Ok((_request, mut respond))) = connection.accept().await {
            if counter.fetch_add(1, Ordering::SeqCst) == 0 {
                respond.send_reset(h2::Reason::REFUSED_STREAM);
            } else {
                let response = Response::builder()
                    .status(200)
                    .header("content-type", "text/event-stream")
                    .body(())
                    .unwrap();
                let mut body = respond.send_response(response, false).unwrap();
                body.send_data(hyper::body::Bytes::from_static(b"data: [DONE]\n\n"), true)
                    .unwrap();
            }
        }
    });
    let client = HttpClientBuilder::new()
        .tls(
            TlsOptions::new()
                .with_explicit_roots_pem(CERT_CA.as_bytes())
                .unwrap(),
        )
        .build()
        .unwrap();
    let result = timeout(
        Duration::from_secs(5),
        chat_models_call(&url, Arc::new(client)),
    )
    .await
    .unwrap();
    peer.abort();
    let _ = peer.await;
    assert_eq!(
        sends.load(Ordering::SeqCst),
        1,
        "only the SDK may admit another send"
    );
    assert!(
        matches!(result, Err(ProviderError::Network(_))),
        "{result:?}"
    );
}
