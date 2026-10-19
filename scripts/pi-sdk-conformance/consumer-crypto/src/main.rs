//! Packaged-crypto public consumer proof (task 20.14, RDF-23).
//!
//! An ordinary workspace-external Cargo consumer exercising the real public
//! `opi-ai` TLS/Chat path from the extracted `cargo package` artifact:
//! `HttpClientBuilder::tls(TlsOptions)` -> `OpenAiChatProvider::with_client`
//! -> `Models::register_route` -> `prepare_call` -> `start_attempt`, against
//! a loopback rustls TLS peer. No raw reqwest/rustls client construction
//! for the SDK path, no privileged helper, no test-only SDK API.
//!
//! With `caller-union` the consumer additionally installs a process-global
//! ring provider first, proving the SDK's explicit instance provider is
//! independent of host/global state (the ordinary run asserts no global
//! provider exists at all).
//!
//! Prints one JSON report: {"verdict", "total", "failed", "checks": [...]}.

#![forbid(unsafe_code)]

mod fixtures;

use std::{
    convert::Infallible,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use http_body_util::{BodyExt, Full};
use hyper::{Request as HyperRequest, Response, body::Incoming, service::service_fn};
use hyper_util::rt::{TokioExecutor, TokioIo};
use opi_ai::auth::{AuthScheme, AuthProvenanceSource, StaticAuthResolver};
use opi_ai::http::{HttpClient, HttpClientBuilder, TlsOptions};
use opi_ai::models::Models;
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::provider::{CacheRetention, ProviderError, Request, ThinkingConfig};
use opi_ai::stream::ChatStream;
use opi_ai::transcript;
use rustls::{
    ServerConfig,
    crypto::CryptoProvider,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_util::sync::CancellationToken;

const SSE_BODY: &str = concat!(
    "data: {\"id\":\"chatcmpl-pkg\",\"object\":\"chat.completion.chunk\",\"created\":1,",
    "\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},",
    "\"finish_reason\":null}]}\n\n",
    "data: {\"id\":\"chatcmpl-pkg\",\"object\":\"chat.completion.chunk\",\"created\":1,",
    "\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],",
    "\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n",
);

fn check(report: &mut Vec<serde_json::Value>, name: &str, passed: bool, detail: String) {
    report.push(serde_json::json!({"name": name, "passed": passed, "detail": detail}));
}

fn server_provider() -> Arc<CryptoProvider> {
    use rustls::crypto::aws_lc_rs::kx_group::{
        SECP256R1, SECP384R1, X25519, X25519MLKEM768,
    };
    let mut provider = rustls::crypto::aws_lc_rs::default_provider();
    provider.kx_groups = vec![X25519MLKEM768, X25519, SECP256R1, SECP384R1];
    Arc::new(provider)
}

struct Server {
    url: String,
    http: Arc<AtomicUsize>,
    accepts: Arc<AtomicUsize>,
}

async fn start_server(
    tls13: bool,
    h2: bool,
    require_client_cert: bool,
) -> Result<Server, String> {
    let cert = CertificateDer::from_pem_slice(fixtures::SERVER_CERT_PEM.as_bytes())
        .map_err(|_| "server cert".to_owned())?;
    let key = PrivateKeyDer::from_pem_slice(fixtures::SERVER_KEY_PEM.as_bytes())
        .map_err(|_| "server key".to_owned())?;
    let provider = server_provider();
    let version: &rustls::SupportedProtocolVersion = if tls13 {
        &rustls::version::TLS13
    } else {
        &rustls::version::TLS12
    };
    let builder = ServerConfig::builder_with_provider(provider.clone())
        .with_protocol_versions(&[version])
        .map_err(|_| "versions".to_owned())?;
    let builder = if require_client_cert {
        let mut roots = rustls::RootCertStore::empty();
        let ca = CertificateDer::from_pem_slice(fixtures::CA_PEM.as_bytes())
            .map_err(|_| "ca".to_owned())?;
        roots.add(ca).map_err(|_| "ca root".to_owned())?;
        let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
            Arc::new(roots),
            provider,
        )
        .build()
        .map_err(|_| "verifier".to_owned())?;
        builder.with_client_cert_verifier(verifier)
    } else {
        builder.with_no_client_auth()
    };
    let mut config = builder
        .with_single_cert(vec![cert], key)
        .map_err(|_| "identity".to_owned())?;
    config.alpn_protocols = vec![if h2 { b"h2".to_vec() } else { b"http/1.1".to_vec() }];
    let acceptor = TlsAcceptor::from(Arc::new(config));

    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|_| "bind".to_owned())?;
    let addr = listener.local_addr().map_err(|_| "address".to_owned())?;
    let http = Arc::new(AtomicUsize::new(0));
    let accepts = Arc::new(AtomicUsize::new(0));
    let h = http.clone();
    let a = accepts.clone();
    tokio::spawn(async move {
        loop {
            let Ok((tcp, _)) = listener.accept().await else { break };
            a.fetch_add(1, Ordering::SeqCst);
            let acceptor = acceptor.clone();
            let h = h.clone();
            tokio::spawn(async move {
                let Ok(tls) = acceptor.accept(tcp).await else { return };
                let service = service_fn(move |req: HyperRequest<Incoming>| {
                    let h = h.clone();
                    async move {
                        h.fetch_add(1, Ordering::SeqCst);
                        let _ = req.into_body().collect().await;
                        Ok::<_, Infallible>(Response::builder()
                            .status(200)
                            .header("content-type", "text/event-stream")
                            .body(Full::new(SSE_BODY.as_bytes()))
                            .unwrap())
                    }
                });
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
            });
        }
    });
    Ok(Server {
        url: format!("https://{addr}"),
        http,
        accepts,
    })
}

fn tls_client(identity: bool, http1_only: bool) -> Result<HttpClient, String> {
    let mut options = TlsOptions::new()
        .with_explicit_roots_pem(fixtures::CA_PEM.as_bytes())
        .map_err(|e| format!("roots: {e}"))?;
    if identity {
        options = options
            .with_client_identity_pem(
                fixtures::CLIENT_CERT_PEM.as_bytes(),
                fixtures::CLIENT_KEY_PEM.as_bytes(),
            )
            .map_err(|e| format!("identity: {e}"))?;
    }
    if http1_only {
        options = options.http1_only();
    }
    HttpClientBuilder::new()
        .max_idle_per_host(0)
        .tls(options)
        .build()
        .map_err(|e| format!("client build: {e}"))
}

fn chat_request() -> Request {
    Request {
        model: "openai:gpt-4o".into(),
        context: transcript::normalize_context(transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context"),
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

async fn chat_call(
    server_url: &str,
    client: Arc<HttpClient>,
) -> Result<String, ProviderError> {
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
                secrecy::SecretString::from("packaged-consumer-key"),
            )),
            AuthProvenanceSource::Static,
            opi_ai::models::CompatMetadata {
                openai_compatible: true,
                profile: None,
            },
        )
        .map_err(|e| ProviderError::Config(opi_ai::provider::ProviderErrorSummary::from_untrusted(format!("{e}"))))?;
    let prepared = models
        .prepare_call("openai:gpt-4o", chat_request())
        .await
        .map_err(|e| ProviderError::Config(opi_ai::provider::ProviderErrorSummary::from_untrusted(format!("{e:?}"))))?;
    let stream = prepared.start_attempt().map_err(|e| {
        ProviderError::Config(opi_ai::provider::ProviderErrorSummary::from_untrusted(
            format!("{e:?}"),
        ))
    })?;
    ChatStream::new(stream)
        .result()
        .await
        .map(|message| message.response_id.unwrap_or_default())
        .map_err(|failure| failure.into_parts().1)
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    let mut report: Vec<serde_json::Value> = Vec::new();
    let union = cfg!(feature = "caller-union");

    // Union profile: install a host ring provider before anything else.
    if union {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("fresh consumer process accepts a host provider");
        check(
            &mut report,
            "caller-union-host-ring-installed",
            CryptoProvider::get_default().is_some(),
            "host ring provider installed as the process default".to_owned(),
        );
    } else {
        check(
            &mut report,
            "ordinary-no-global-provider",
            CryptoProvider::get_default().is_none(),
            "fresh ordinary consumer has no process-global provider".to_owned(),
        );
    }

    // TLS 1.3 H2 with mutual TLS through the public Models/Chat path.
    let server = start_server(true, true, true).await.expect("server");
    let client = Arc::new(tls_client(true, false).expect("client"));
    let outcome = tokio::time::timeout(Duration::from_secs(10), chat_call(&server.url, client))
        .await
        .expect("h2 mTLS round trip timeout");
    let passed = matches!(&outcome, Ok(id) if id == "chatcmpl-pkg");
    let accepts = server.accepts.load(Ordering::SeqCst);
    let http = server.http.load(Ordering::SeqCst);
    check(
        &mut report,
        "chat-mtls-tls13-h2-round-trip",
        passed && accepts == 1 && http == 1,
        format!("response_id={outcome:?} accepts={accepts} http={http}"),
    );

    // TLS 1.2 with explicit H1 selection.
    let server12 = start_server(false, false, false).await.expect("server12");
    let client12 = Arc::new(tls_client(false, true).expect("client12"));
    let outcome12 =
        tokio::time::timeout(Duration::from_secs(10), chat_call(&server12.url, client12))
            .await
            .expect("h1 round trip timeout");
    check(
        &mut report,
        "chat-tls12-h1-explicit-selection",
        matches!(&outcome12, Ok(id) if id == "chatcmpl-pkg")
            && server12.accepts.load(Ordering::SeqCst) == 1,
        format!("response_id={outcome12:?}"),
    );

    // Trust negative: untrusted server root, zero business HTTP.
    let bad = start_server(true, true, false).await.expect("bad server");
    let bad_client = Arc::new(
        TlsOptions::new()
            .with_explicit_roots_pem(fixtures::CLIENT_CERT_PEM.as_bytes())
            .map_err(|e| format!("roots: {e}"))
            .and_then(|options| {
                HttpClientBuilder::new()
                    .max_idle_per_host(0)
                    .tls(options)
                    .build()
                    .map_err(|e| format!("build: {e}"))
            })
            .expect("wrong-root client"),
    );
    let bad_outcome =
        tokio::time::timeout(Duration::from_secs(10), chat_call(&bad.url, bad_client))
            .await
            .expect("negative timeout");
    check(
        &mut report,
        "untrusted-roots-zero-business-http",
        bad_outcome.is_err() && bad.http.load(Ordering::SeqCst) == 0,
        format!("error={bad_outcome:?} http={}", bad.http.load(Ordering::SeqCst)),
    );

    // Admission/assembly negative: an RSA client certificate paired with
    // an individually admitted P-521 key must fail at build, before any
    // socket, with a typed redacted error.
    let assembly = TlsOptions::new()
        .with_explicit_roots_pem(fixtures::CA_PEM.as_bytes())
        .expect("roots")
        .with_client_identity_pem(
            fixtures::CLIENT_CERT_PEM.as_bytes(),
            fixtures::CLIENT_P521_KEY_PEM.as_bytes(),
        )
        .map_err(|e| format!("admission: {e}"))
        .and_then(|options| {
            HttpClientBuilder::new()
                .max_idle_per_host(0)
                .tls(options)
                .build()
                .map_err(|e| format!("build: {e}"))
        });
    let build_error = assembly.err();
    let typed_rejection = build_error
        .as_deref()
        .is_some_and(|text| text.contains("TLS configuration rejected"));
    check(
        &mut report,
        "mismatched-identity-rejected-before-io",
        typed_rejection,
        format!("build_error={build_error:?}"),
    );

    // Host-provider state is unchanged by the SDK path.
    let provider_now = CryptoProvider::get_default().is_some();
    check(
        &mut report,
        "global-provider-state-unchanged",
        provider_now == union,
        format!("global present={provider_now} (union={union})"),
    );

    let failed = report.iter().filter(|c| c["passed"] != true).count();
    let verdict = if failed == 0 && report.len() == EXPECTED_CHECKS { "PASS" } else { "FAIL" };
    println!(
        "{}",
        serde_json::json!({
            "verdict": verdict,
            "profile": if union { "caller-union" } else { "ordinary" },
            "total": report.len(),
            "failed": failed,
            "checks": report,
        })
    );
    if verdict != "PASS" {
        std::process::exit(1);
    }
}

const EXPECTED_CHECKS: usize = 6;
