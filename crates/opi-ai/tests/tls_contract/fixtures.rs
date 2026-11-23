//! Loopback TLS peer and public HttpClient/Models driver shared by this target.

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
use opi_ai::http::{HttpClient, HttpClientBuildError, HttpClientBuilder, TlsOptions};
use opi_ai::message::AssistantMessage;
use opi_ai::models::Models;
use opi_ai::provider::{CacheRetention, EventStream, ProviderError, Request, ThinkingConfig};
use opi_ai::providers::openai::OpenAiChatProvider;
use opi_ai::stream::ChatStream;
use opi_ai::transcript;
use rustls::{
    RootCertStore, ServerConfig,
    crypto::CryptoProvider,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
};
use sha2::{Digest, Sha256};
use tokio::{
    net::TcpListener,
    sync::oneshot,
    task::JoinHandle,
    time::{sleep, timeout},
};
use tokio_rustls::TlsAcceptor;
use tokio_util::sync::CancellationToken;

use super::tls_fixtures::CERT_CA;

/// The server-side key-exchange selection for a case.
pub(super) enum KxProfile {
    /// Match the frozen client profile.
    Declared,
    /// Offer only P-521 (no overlap with the frozen client profile). The
    /// upstream rustls/aws-lc-rs backend intentionally does not ship a
    /// P-521 group (RDF-23 omits P-521 key exchange), so the loopback peer
    /// implements the group itself over the public aws-lc-rs ECDH API.
    P521Only,
}

#[derive(Default)]
pub(super) struct Stats {
    active: AtomicUsize,
    pub(super) accepts: AtomicUsize,
    pub(super) handshakes: AtomicUsize,
    pub(super) rejected: AtomicUsize,
    pub(super) http: AtomicUsize,
    closed: AtomicUsize,
    pub(super) peers: Mutex<Vec<serde_json::Value>>,
}

struct CountActive(Arc<Stats>);
impl Drop for CountActive {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);
    }
}

pub(super) struct Server {
    pub(super) url: String,
    pub(super) stats: Arc<Stats>,
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

pub(super) fn instance_provider(profile: &KxProfile) -> Arc<CryptoProvider> {
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

pub(super) fn fingerprint(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub(super) fn server_config(
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

pub(super) async fn start_server(
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
    pub(super) async fn finish(self) -> Result<(), String> {
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

    pub(super) async fn wait_idle(&self) -> Result<(), String> {
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

pub(super) enum Trust<'a> {
    Explicit(&'a str),
    PinnedDefaultWithExtra(&'a str),
    HostSystem,
}

pub(super) fn build_tls_client(
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

pub(super) fn chat_request() -> Request {
    Request {
        projection: Default::default(),
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

pub(super) async fn chat_models_call(
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

pub(super) async fn prepare_chat_request(
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
pub(super) async fn chat_models_error(server_url: &str, client: Arc<HttpClient>) -> ProviderError {
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
    let request = chat_request();
    let prepared = models
        .prepare_call("openai:gpt-4o", request.clone())
        .await
        .expect("call preparation");
    let mut stream: EventStream = prepared.start_attempt().expect("attempt start");
    while let Some(item) = stream.next().await {
        if let Err(error) = item {
            accepted_failure_cause(&error, &request);
            assert!(
                stream.next().await.is_none(),
                "negative call has one terminal failure"
            );
            return error;
        }
    }
    panic!("negative TLS case completed without a stream error");
}

// ---------------------------------------------------------------------------
// Positives: negotiated protocol, versions, identity profiles
// ---------------------------------------------------------------------------

pub(super) async fn assert_round_trip(
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

pub(super) async fn assert_negative_zero_http(
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
    let ProviderError::Network(summary) = accepted_failure_cause(&error, &chat_request()) else {
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

pub(super) fn accepted_failure_cause<'a>(
    error: &'a opi_ai::provider::ProviderError,
    request: &opi_ai::provider::Request,
) -> &'a opi_ai::provider::ProviderError {
    use opi_ai::provider::ProviderError;
    use sha2::Digest as _;
    let ProviderError::StreamFailure(failure) = error else {
        panic!("accepted projection must retain its failure carrier: {error:?}");
    };
    let facts = failure
        .message()
        .projection
        .as_ref()
        .expect("accepted projection is known");
    let bytes = serde_json::to_vec(request.context.messages()).unwrap();
    assert_eq!(
        facts.source_prefix.message_count,
        request.context.messages().len()
    );
    assert_eq!(
        facts.source_prefix.sha256,
        format!("{:x}", sha2::Sha256::digest(bytes))
    );
    assert_eq!(
        facts.temporal.mode,
        opi_ai::transcript::ProjectionMode::Lossless
    );
    assert!(facts.temporal.affected_ranges.is_empty());
    assert!(facts.replay.losses.is_empty());
    failure.cause()
}
