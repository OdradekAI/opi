//! Copilot OAuth fixtures shared by the dedicated and cross-provider targets.

use std::sync::Arc;
use std::time::Duration;

#[allow(unused_imports)]
pub use opi_ai::auth::oauth::copilot::{
    self, COPILOT_CLIENT_ID, COPILOT_DEFAULT_BASE_URL, COPILOT_DEVICE_SCOPE, COPILOT_PUBLIC_DOMAIN,
    CopilotUrls, GitHubCopilotOAuthProvider,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::oauth_assembly::oauth_assembly_sse;
use super::oauth_fixtures::ScriptedPresenter;
use super::oauth_tls_fixtures;

/// One HTTPS inference request on the endpoint selected by a synthetic
/// Copilot token. Trust is confined to the existing local test CA.
pub async fn copilot_https_target() -> (
    String,
    Arc<opi_ai::http::HttpClient>,
    tokio::task::JoinHandle<String>,
) {
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("https://{}", listener.local_addr().unwrap());
    let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![
            CertificateDer::from_pem_slice(oauth_tls_fixtures::CERT_SERVER_RSA.as_bytes()).unwrap(),
        ],
        PrivateKeyDer::from_pem_slice(oauth_tls_fixtures::KEY_SERVER_RSA.as_bytes()).unwrap(),
    )
    .unwrap();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut tls = tokio_rustls::TlsAcceptor::from(Arc::new(config))
            .accept(stream)
            .await
            .unwrap();
        let mut bytes = Vec::new();
        let mut expected_length = None;
        loop {
            let mut chunk = [0u8; 4096];
            let len = tls.read(&mut chunk).await.unwrap();
            assert_ne!(len, 0, "inference request ended before its body");
            bytes.extend_from_slice(&chunk[..len]);
            if expected_length.is_none()
                && let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n")
            {
                let headers = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
                let body_len: usize = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .expect("bounded inference body")
                    .trim()
                    .parse()
                    .unwrap();
                expected_length = Some(end + 4 + body_len);
            }
            if expected_length.is_some_and(|len| bytes.len() >= len) {
                break;
            }
        }
        let body = oauth_assembly_sse(opi_ai::WireApi::OpenAiCompletions);
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        tls.write_all(response.as_bytes()).await.unwrap();
        let _ = tls.shutdown().await;
        String::from_utf8(bytes).unwrap()
    });
    let tls = opi_ai::http::TlsOptions::new()
        .with_explicit_roots_pem(oauth_tls_fixtures::CERT_CA.as_bytes())
        .unwrap()
        .http1_only();
    let client = opi_ai::http::HttpClientBuilder::new()
        .tls(tls)
        .build()
        .unwrap();
    (endpoint, Arc::new(client), task)
}

pub fn copilot_token_body() -> serde_json::Value {
    let expires = time::OffsetDateTime::now_utc().unix_timestamp() + 3600;
    serde_json::json!({
        "token": "tid=1;exp=2;proxy-ep=proxy.individual.githubcopilot.com;sch=sha256",
        "expires_at": expires,
    })
}

/// Models catalog entries: (id, picker_enabled, policy_state, supports_tools).
pub fn copilot_models_body(items: &[(&str, bool, &str, bool)]) -> serde_json::Value {
    serde_json::json!({
        "data": items.iter().map(|(id, picker, state, tools)| {
            serde_json::json!({
                "id": id,
                "model_picker_enabled": picker,
                "policy": {"state": state},
                "capabilities": {"supports": {"tool_calls": tools}},
            })
        }).collect::<Vec<_>>(),
    })
}

pub async fn mount_copilot_device_start(server: &MockServer, verification_uri: String) {
    Mock::given(method("POST"))
        .and(path("/login/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "dc-secret-1",
            "user_code": "ABCD-1234",
            "verification_uri": verification_uri,
            "interval": 0,
            "expires_in": 900,
        })))
        .mount(server)
        .await;
}

pub async fn mount_copilot_poll(server: &MockServer, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

pub async fn mount_copilot_poll_complete(server: &MockServer) {
    mount_copilot_poll(server, serde_json::json!({"access_token": "gh-access-1"})).await;
}

pub async fn mount_copilot_exchange(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/copilot_internal/v2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(copilot_token_body()))
        .mount(server)
        .await;
}

pub async fn mount_copilot_models(server: &MockServer, body: serde_json::Value) {
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

/// The offline copilot flow: every endpoint maps onto one wiremock server,
/// enterprise domains get their own path prefix, and the models base URL is
/// derived from the copilot token's proxy endpoint mapped back onto the
/// same server (the production closure derives `https://api.*` hosts).
pub fn copilot_flow(
    server_uri: &str,
    whitelist: Vec<String>,
    total_budget: Duration,
) -> GitHubCopilotOAuthProvider {
    let uri = server_uri.to_owned();
    let models_uri = server_uri.to_owned();
    GitHubCopilotOAuthProvider::new_with_endpoints(
        CopilotUrls {
            device_code_url: format!("{uri}/login/device/code"),
            access_token_url: format!("{uri}/login/oauth/access_token"),
            copilot_token_url: format!("{uri}/copilot_internal/v2/token"),
        },
        Arc::new(move |domain: &str| CopilotUrls {
            device_code_url: format!("{uri}/{domain}/login/device/code"),
            access_token_url: format!("{uri}/{domain}/login/oauth/access_token"),
            copilot_token_url: format!("{uri}/{domain}/copilot_internal/v2/token"),
        }),
        Arc::new(move |token: &str, _domain: Option<&str>| {
            // Prove the derivation runs on the issued copilot token: the
            // test token carries a proxy endpoint that must survive the
            // mapping (offline servers cannot host `https://api.*`).
            let host = token
                .split("proxy-ep=")
                .nth(1)
                .and_then(|rest| rest.split(';').next())
                .unwrap_or_default();
            assert!(
                host.starts_with("proxy."),
                "models base must be derived from the token's proxy endpoint"
            );
            models_uri.clone()
        }),
        whitelist,
        total_budget,
        reqwest::Client::new(),
    )
}

pub fn copilot_presenter_with(answer: &str) -> Arc<ScriptedPresenter> {
    let presenter = ScriptedPresenter::default();
    *presenter.prompt_answers.lock().unwrap() = vec![answer.to_owned()];
    Arc::new(presenter)
}
