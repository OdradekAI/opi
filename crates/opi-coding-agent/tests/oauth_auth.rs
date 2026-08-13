//! Phase 14.2 閳?OAuth auth source, per-request refresh, and command flows.
//!
//! Slice 3 covers `AuthSource::{Baked, Store, EnvOAuthToken}` and the
//! `CredentialResolver` OAuth refresh bridge (double-checked locking, 5-minute
//! skew, locked refresh-HTTP, post-failure re-read, no partial write). Later
//! slices append login/logout, run-mode, and provider-mapping tests.

mod common;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{MockLoginPresenter, extract_query_param, extract_redirect_port};
use opi_agent::AgentError;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::auth::{
    AuthResolver, AuthScheme, LoginPresenter, OAuthCredential, OAuthLoginMethod, OAuthProvider,
    ResolvedAuth,
};
use opi_ai::credential::{BoxAuthFuture, Credential, CredentialStore};
use opi_ai::http::HttpClient;
use opi_ai::provider::{
    CacheRetention, EventStream, ModelInfo, Provider, ProviderError as AiProviderError, Request,
    ThinkingConfig,
};
use opi_ai::registry::ModelCapabilities;
use opi_ai::{AuthProvenanceSource, CompatMetadata, ProviderCollection};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::credential_store::{
    AuthSource, CredentialResolver, EnvLookup, FakeKeyringBackend, KEYCHAIN_SERVICE,
    KeychainCredentialStore,
};
use opi_coding_agent::harness::CodingHarness;
use opi_coding_agent::oauth::{
    AnthropicOAuthProvider, CodexOAuthProvider, CopilotOAuthProvider, OAuthProviderRegistry,
    RegistryError, TuiLoginPresenter, login_oauth, logout_credential,
};
use opi_coding_agent::policy::ToolSelection;
use opi_coding_agent::provider_factory::build_provider_with_oauth;
use opi_coding_agent::rpc::{RpcCommand, RpcRunner};
use secrecy::{ExposeSecret, SecretString};
use serde_json::json;
use tempfile::TempDir;
use time::OffsetDateTime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use futures_util::{FutureExt, StreamExt, stream};

const AUTHORIZE_URL: &str = "https://authorize.example/oauth/authorize";

/// Token response JSON for a successful exchange. `expires_in` is relative
/// seconds from now.
fn token_body(access: &str, refresh: &str, expires_in: i64) -> serde_json::Value {
    json!({"access_token": access, "refresh_token": refresh, "expires_in": expires_in})
}

/// Mount a stub responding to `POST /oauth/token` with `body`.
async fn mount_token_stub(server: &MockServer, status: u16, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .mount(server)
        .await;
}

struct RawJsonRoute {
    method: &'static str,
    path: &'static str,
    body: serde_json::Value,
    body_delay: Duration,
    probe: RawRouteProbe,
}

#[derive(Clone)]
struct RawRouteProbe {
    method: &'static str,
    path: &'static str,
    reached: Arc<AtomicBool>,
    headers_sent: Arc<AtomicBool>,
    body_boundary_reached: Arc<AtomicBool>,
    changed: Arc<tokio::sync::Notify>,
}

impl RawRouteProbe {
    fn new(method: &'static str, path: &'static str) -> Self {
        Self {
            method,
            path,
            reached: Arc::new(AtomicBool::new(false)),
            headers_sent: Arc::new(AtomicBool::new(false)),
            body_boundary_reached: Arc::new(AtomicBool::new(false)),
            changed: Arc::new(tokio::sync::Notify::new()),
        }
    }

    fn mark(flag: &AtomicBool, changed: &tokio::sync::Notify) {
        flag.store(true, Ordering::SeqCst);
        changed.notify_waiters();
    }

    async fn wait_for(&self, flag: &AtomicBool, stage: &str) {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let changed = self.changed.notified();
                if flag.load(Ordering::SeqCst) {
                    break;
                }
                changed.await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("{} {} never reached {stage}", self.method, self.path));
    }

    async fn wait_for_headers(&self) {
        self.wait_for(&self.headers_sent, "response headers").await;
        assert!(self.reached.load(Ordering::SeqCst));
    }

    async fn assert_body_boundary_reached(&self) {
        self.wait_for(&self.body_boundary_reached, "response body boundary")
            .await;
        assert!(self.reached.load(Ordering::SeqCst));
        assert!(self.headers_sent.load(Ordering::SeqCst));
    }
}

fn raw_json_route(
    method: &'static str,
    path: &'static str,
    body: serde_json::Value,
    body_delay: Duration,
) -> (RawJsonRoute, RawRouteProbe) {
    let probe = RawRouteProbe::new(method, path);
    (
        RawJsonRoute {
            method,
            path,
            body,
            body_delay,
            probe: probe.clone(),
        },
        probe,
    )
}

struct RawBodyDelayServer {
    uri: String,
    task: tokio::task::JoinHandle<()>,
}

impl RawBodyDelayServer {
    async fn start(routes: Vec<RawJsonRoute>) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind raw body-delay server");
        let uri = format!(
            "http://{}",
            listener
                .local_addr()
                .expect("raw body-delay server address")
        );
        let task = tokio::spawn(async move {
            for _ in 0..routes.len() {
                let (mut socket, _) = listener
                    .accept()
                    .await
                    .expect("accept raw body-delay request");
                let mut request = Vec::new();
                let header_end = loop {
                    let mut chunk = [0_u8; 1024];
                    let read = socket
                        .read(&mut chunk)
                        .await
                        .expect("read raw body-delay request");
                    assert!(read > 0, "request closed before headers");
                    request.extend_from_slice(&chunk[..read]);
                    if let Some(end) = request
                        .windows(4)
                        .position(|window| window == b"\r\n\r\n")
                        .map(|index| index + 4)
                    {
                        break end;
                    }
                };
                let headers =
                    std::str::from_utf8(&request[..header_end]).expect("ASCII request headers");
                let request_line = headers.lines().next().expect("request line");
                let mut request_parts = request_line.split_ascii_whitespace();
                let method = request_parts.next().expect("request method").to_owned();
                let path = request_parts.next().expect("request path").to_owned();
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().expect("content length"))
                    })
                    .unwrap_or(0);
                while request.len() - header_end < content_length {
                    let mut chunk = [0_u8; 1024];
                    let read = socket
                        .read(&mut chunk)
                        .await
                        .expect("read raw body-delay request body");
                    assert!(read > 0, "request closed before body");
                    request.extend_from_slice(&chunk[..read]);
                }

                let route = routes
                    .iter()
                    .find(|route| route.method == method && route.path == path)
                    .unwrap_or_else(|| panic!("unexpected raw request: {method} {path}"));
                RawRouteProbe::mark(&route.probe.reached, &route.probe.changed);
                let body = serde_json::to_vec(&route.body).expect("serialize raw JSON body");
                let headers = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                    body.len()
                );
                socket
                    .write_all(headers.as_bytes())
                    .await
                    .expect("write raw response headers");
                socket.flush().await.expect("flush raw response headers");
                RawRouteProbe::mark(&route.probe.headers_sent, &route.probe.changed);
                tokio::time::sleep(route.body_delay).await;
                RawRouteProbe::mark(&route.probe.body_boundary_reached, &route.probe.changed);
                if let Err(error) = socket.write_all(&body).await {
                    assert!(
                        matches!(
                            error.kind(),
                            std::io::ErrorKind::BrokenPipe
                                | std::io::ErrorKind::ConnectionAborted
                                | std::io::ErrorKind::ConnectionReset
                        ),
                        "write delayed raw response body: {error}"
                    );
                }
            }
        });
        Self { uri, task }
    }

    fn uri(&self) -> &str {
        &self.uri
    }
}

impl Drop for RawBodyDelayServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn anthropic_provider(token_url: String, timeout: Duration) -> AnthropicOAuthProvider {
    AnthropicOAuthProvider::new(
        AUTHORIZE_URL.to_owned(),
        token_url,
        "client-id".to_owned(),
        timeout,
    )
}

fn codex_provider(token_url: String, timeout: Duration) -> CodexOAuthProvider {
    CodexOAuthProvider::new(
        AUTHORIZE_URL.to_owned(),
        token_url,
        "codex-client-id".to_owned(),
        timeout,
    )
}

#[derive(Clone, Copy, Debug)]
enum BrowserPkceProvider {
    Anthropic,
    Codex,
}

impl BrowserPkceProvider {
    fn build(self, token_url: String) -> Box<dyn OAuthProvider> {
        self.build_with_timeout(token_url, Duration::from_secs(60))
    }

    fn build_with_timeout(self, token_url: String, timeout: Duration) -> Box<dyn OAuthProvider> {
        match self {
            Self::Anthropic => Box::new(anthropic_provider(token_url, timeout)),
            Self::Codex => Box::new(codex_provider(token_url, timeout)),
        }
    }

    fn provider_id(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::Codex => "openai-codex",
        }
    }

    fn success_access(self) -> String {
        match self {
            Self::Anthropic => "anthropic-access".to_owned(),
            Self::Codex => codex_jwt(Some("browser-pkce-account")),
        }
    }
}

const BROWSER_PKCE_PROVIDERS: [BrowserPkceProvider; 2] =
    [BrowserPkceProvider::Anthropic, BrowserPkceProvider::Codex];

fn codex_device_provider(server_uri: &str, timeout: Duration) -> CodexOAuthProvider {
    codex_provider_with_flow_timeouts(server_uri, Duration::from_secs(60), timeout)
}

fn codex_provider_with_flow_timeouts(
    server_uri: &str,
    browser_timeout: Duration,
    device_timeout: Duration,
) -> CodexOAuthProvider {
    CodexOAuthProvider::new_with_device_endpoints(
        AUTHORIZE_URL.to_owned(),
        format!("{server_uri}/oauth/token"),
        format!("{server_uri}/device/usercode"),
        format!("{server_uri}/device/token"),
        "https://auth.openai.com/codex/device".to_owned(),
        "https://auth.openai.com/deviceauth/callback".to_owned(),
        "codex-client-id".to_owned(),
        browser_timeout,
        device_timeout,
    )
}

type LoginSelection = (String, Vec<OAuthLoginMethod>, OAuthLoginMethod);

struct MethodPresenter {
    inner: MockLoginPresenter,
    method: Option<OAuthLoginMethod>,
    selections: Arc<Mutex<Vec<LoginSelection>>>,
    selection_started: Arc<tokio::sync::Notify>,
    device_code_presented: Arc<tokio::sync::Notify>,
    cancel_device_login: Arc<tokio::sync::Notify>,
    selection_delay: Duration,
    block_method_selection: bool,
    block_device_presentation: bool,
}

impl MethodPresenter {
    fn new(method: Option<OAuthLoginMethod>) -> Self {
        Self {
            inner: MockLoginPresenter::new(),
            method,
            selections: Arc::new(Mutex::new(Vec::new())),
            selection_started: Arc::new(tokio::sync::Notify::new()),
            device_code_presented: Arc::new(tokio::sync::Notify::new()),
            cancel_device_login: Arc::new(tokio::sync::Notify::new()),
            selection_delay: Duration::ZERO,
            block_method_selection: false,
            block_device_presentation: false,
        }
    }

    fn blocking_method_selection() -> Self {
        Self {
            block_method_selection: true,
            ..Self::new(Some(OAuthLoginMethod::DeviceCode))
        }
    }

    fn delayed_method_selection(method: OAuthLoginMethod, selection_delay: Duration) -> Self {
        Self {
            selection_delay,
            ..Self::new(Some(method))
        }
    }

    fn cancelling_active_device_flow() -> Self {
        Self {
            block_device_presentation: true,
            ..Self::new(Some(OAuthLoginMethod::DeviceCode))
        }
    }
}

impl LoginPresenter for MethodPresenter {
    fn select_login_method<'a>(
        &'a self,
        provider_id: &'a str,
        methods: &'a [OAuthLoginMethod],
        default: OAuthLoginMethod,
    ) -> BoxAuthFuture<'a, Result<OAuthLoginMethod, AiProviderError>> {
        let provider_id = provider_id.to_owned();
        let methods = methods.to_vec();
        let selections = self.selections.clone();
        let method = self.method;
        let selection_started = self.selection_started.clone();
        let selection_delay = self.selection_delay;
        let block = self.block_method_selection;
        Box::pin(async move {
            selections
                .lock()
                .unwrap()
                .push((provider_id.clone(), methods, default));
            selection_started.notify_one();
            tokio::time::sleep(selection_delay).await;
            if block {
                return std::future::pending::<Result<OAuthLoginMethod, AiProviderError>>().await;
            }
            method.ok_or(AiProviderError::LoginCancelled { provider_id })
        })
    }

    fn present_auth_url<'a>(
        &'a self,
        url: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), AiProviderError>> {
        self.inner.present_auth_url(url)
    }

    fn present_device_code<'a>(
        &'a self,
        user_code: &'a str,
        verification_uri: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), AiProviderError>> {
        let presented = self.device_code_presented.clone();
        let block = self.block_device_presentation;
        Box::pin(async move {
            self.inner
                .present_device_code(user_code, verification_uri)
                .await?;
            presented.notify_one();
            if block {
                return std::future::pending::<Result<(), AiProviderError>>().await;
            }
            Ok(())
        })
    }

    fn await_login_cancelled<'a>(&'a self) -> BoxAuthFuture<'a, Result<(), AiProviderError>> {
        let cancel = self.cancel_device_login.clone();
        Box::pin(async move {
            cancel.notified().await;
            Ok(())
        })
    }

    fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, AiProviderError>> {
        self.inner.await_manual_code()
    }

    fn notify_success(&self) {
        self.inner.notify_success();
    }

    fn notify_failure(&self, reason: &str) {
        self.inner.notify_failure(reason);
    }
}

fn oauth_cred(access: &str, refresh: &str, base_url: Option<String>) -> OAuthCredential {
    OAuthCredential {
        access: secret(access),
        refresh: secret(refresh),
        expires_at: Some(OffsetDateTime::now_utc() + Duration::from_secs(3600)),
        base_url,
        account_id: None,
    }
}

fn codex_jwt(account_id: Option<&str>) -> String {
    use base64::Engine;
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
    let payload = match account_id {
        Some(account_id) => json!({
            "https://api.openai.com/auth": {
                "chatgpt_account_id": account_id,
            }
        }),
        None => json!({"sub":"synthetic-user"}),
    };
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&payload).unwrap());
    format!("{header}.{payload}.synthetic-signature")
}

const PROVIDER: &str = "anthropic";
const FRESH_ACCESS: &str = "atk-fresh-DO-NOT-LEAK";
const REFRESHED_ACCESS: &str = "atk-refreshed-DO-NOT-LEAK";

struct CredentialNeededRpcProvider {
    models: Vec<ModelInfo>,
}

impl CredentialNeededRpcProvider {
    fn new() -> Self {
        Self {
            models: vec![ModelInfo::new(
                "mock-model",
                "Mock Model",
                opi_ai::WireApi::OpenAiCompletions,
                ModelCapabilities::new(8_192, 1_024).with_streaming(true),
            )],
        }
    }
}

impl Provider for CredentialNeededRpcProvider {
    fn id(&self) -> &str {
        "mock"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream_prepared(&self, _request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        Box::pin(stream::once(async {
            Err(AiProviderError::CredentialNeeded {
                provider_id: "anthropic".into(),
            })
        }))
    }
}

#[tokio::test]
async fn rpc_credential_needed_fails_without_blocking() {
    let workspace = tempfile::tempdir().unwrap();
    let runner = RpcRunner::new(
        Box::new(CredentialNeededRpcProvider::new()),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        ToolSelection::Disabled,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .unwrap();
    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut runner = runner;
        runner.run_with_channels(command_rx, output_tx).await
    });

    assert_eq!(output_rx.recv().await.unwrap()["type"], "rpc_ready");
    command_tx
        .send(RpcCommand::prompt {
            id: Some("auth-1".into()),
            message: "hello".into(),
        })
        .unwrap();

    let credential_needed = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let line = output_rx.recv().await.expect("RPC output remains open");
            if line["type"] == "CredentialNeeded" {
                break line;
            }
        }
    })
    .await
    .expect("RPC must report CredentialNeeded without prompting or blocking");

    assert_eq!(credential_needed["provider_id"], "anthropic");
    assert_eq!(credential_needed["remediation"], "/login anthropic");
    assert_eq!(
        credential_needed["diagnostic"]["code"],
        "provider_credential_needed"
    );
    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    assert_eq!(task.await.unwrap(), 0);
}

const ENV_TOKEN: &str = "atk-env-oauth-DO-NOT-LEAK";

fn secret(value: &str) -> SecretString {
    SecretString::new(value.to_owned().into_boxed_str())
}

fn oauth_credential(access: &str, expires_at: Option<OffsetDateTime>) -> Credential {
    Credential::OAuthToken {
        access: secret(access),
        refresh: secret("rtk-DO-NOT-LEAK"),
        expires_at,
        base_url: None,
        account_id: None,
    }
}

fn fresh_expiry() -> OffsetDateTime {
    OffsetDateTime::now_utc() + Duration::from_secs(3600)
}

fn near_expiry() -> OffsetDateTime {
    OffsetDateTime::now_utc() + Duration::from_secs(30)
}

/// A store over a fresh temp user-config root + fake backend, returning the
/// backend clone so a mock can inject concurrent writes into shared state.
fn store_with(
    backend: FakeKeyringBackend,
) -> (TempDir, Arc<KeychainCredentialStore>, FakeKeyringBackend) {
    let dir = TempDir::new().unwrap();
    let backend_clone = backend.clone();
    let store = Arc::new(KeychainCredentialStore::with_lock_timeout(
        Box::new(backend),
        dir.path().to_path_buf(),
        Duration::from_secs(2),
    ));
    (dir, store, backend_clone)
}

fn resolver_with(store: Arc<KeychainCredentialStore>) -> CredentialResolver {
    CredentialResolver::new(store, Arc::new(|_: &str| None))
}

/// Mock OAuthProvider. `fail` makes refresh return an error; `inject_fresh`
/// writes a fresh credential to the cloned backend before failing (simulating a
/// concurrent writer that refreshed under a separate lock holder).
struct MockOAuthProvider {
    refresh_calls: AtomicUsize,
    fail: bool,
    inject_fresh: Mutex<Option<Credential>>,
    backend: Option<FakeKeyringBackend>,
}

impl MockOAuthProvider {
    fn succeeding() -> Self {
        Self {
            refresh_calls: AtomicUsize::new(0),
            fail: false,
            inject_fresh: Mutex::new(None),
            backend: None,
        }
    }
}

impl OAuthProvider for MockOAuthProvider {
    fn id(&self) -> &str {
        "mock"
    }
    fn login<'a>(
        &'a self,
        _: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, AiProviderError>> {
        Box::pin(async {
            Err(AiProviderError::Config(
                "login not implemented in mock".into(),
            ))
        })
    }
    fn refresh<'a>(
        &'a self,
        cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, AiProviderError>> {
        self.refresh_calls.fetch_add(1, Ordering::SeqCst);
        let inject = self.inject_fresh.lock().unwrap().take();
        let backend = self.backend.clone();
        let fail = self.fail;
        let refresh_secret = cred.refresh.clone();
        let base_url = cred.base_url.clone();
        let account_id = cred.account_id.clone();
        Box::pin(async move {
            if let (Some(fresh), Some(backend)) = (inject, backend) {
                backend.seed_credential(KEYCHAIN_SERVICE, PROVIDER, &fresh);
            }
            if fail {
                return Err(AiProviderError::Network("refresh HTTP failed".into()));
            }
            Ok(OAuthCredential {
                access: secret(REFRESHED_ACCESS),
                refresh: refresh_secret,
                expires_at: Some(fresh_expiry()),
                base_url,
                account_id,
            })
        })
    }
}

// --- PKCE helpers (RFC 7636) ---

#[test]
fn pkce_code_challenge_s256_matches_rfc7636_vector() {
    // RFC 7636 Appendix B test vector.
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let challenge = opi_coding_agent::oauth::code_challenge_s256(verifier);
    assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
}

#[test]
fn pkce_code_verifier_is_url_safe_unique_and_in_range() {
    let v1 = opi_coding_agent::oauth::generate_code_verifier();
    let v2 = opi_coding_agent::oauth::generate_code_verifier();
    assert!((43..=128).contains(&v1.len()), "verifier len {}", v1.len());
    assert!(
        v1.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    );
    assert_ne!(v1, v2);
    assert_ne!(opi_coding_agent::oauth::code_challenge_s256(&v1), v1);
}

#[test]
fn pkce_state_is_url_safe_and_unique() {
    let s1 = opi_coding_agent::oauth::generate_state();
    let s2 = opi_coding_agent::oauth::generate_state();
    assert!(
        s1.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    );
    assert_ne!(s1, s2);
}

// --- AuthSource::Baked ---

#[tokio::test]
async fn auth_source_baked_resolves_api_key_scheme() {
    let src = AuthSource::Baked(secret("sk-baked"));
    let resolved = src.resolve().await.expect("baked resolve");
    assert_eq!(resolved.scheme, AuthScheme::ApiKey);
    assert_eq!(resolved.secret.expose_secret(), "sk-baked");
}

// --- AuthSource::EnvOAuthToken ---

fn env_lookup(value: Option<&str>) -> EnvLookup {
    let value = value.map(|s| s.to_owned());
    Arc::new(move |_: &str| value.clone())
}

#[tokio::test]
async fn auth_source_env_oauth_token_present_resolves_bearer() {
    let src = AuthSource::EnvOAuthToken {
        provider_id: "anthropic".into(),
        env_var: "ANTHROPIC_OAUTH_TOKEN".into(),
        env_lookup: env_lookup(Some(ENV_TOKEN)),
    };
    let resolved = src.resolve().await.expect("env resolve");
    assert_eq!(resolved.scheme, AuthScheme::Bearer);
    assert_eq!(resolved.secret.expose_secret(), ENV_TOKEN);
}

#[tokio::test]
async fn auth_source_env_oauth_token_absent_yields_credential_needed() {
    let src = AuthSource::EnvOAuthToken {
        provider_id: "anthropic".into(),
        env_var: "ANTHROPIC_OAUTH_TOKEN".into(),
        env_lookup: env_lookup(None),
    };
    match src.resolve().await {
        Err(AiProviderError::CredentialNeeded { provider_id }) => {
            assert_eq!(provider_id, "anthropic");
        }
        other => panic!("expected CredentialNeeded, got {other:?}"),
    }
}

// --- CredentialResolver::resolve_oauth ---

#[tokio::test]
async fn resolve_oauth_present_and_fresh_returns_bearer_without_refresh() {
    let (_dir, store, _backend) = store_with(FakeKeyringBackend::new());
    store
        .write(
            PROVIDER,
            &oauth_credential(FRESH_ACCESS, Some(fresh_expiry())),
        )
        .await
        .unwrap();
    let resolver = resolver_with(store);
    let oauth = MockOAuthProvider::succeeding();
    let resolved = resolver
        .resolve_oauth(PROVIDER, &oauth)
        .await
        .expect("fresh resolve");
    assert_eq!(resolved.scheme, AuthScheme::Bearer);
    assert_eq!(resolved.secret.expose_secret(), FRESH_ACCESS);
    assert_eq!(oauth.refresh_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn resolve_oauth_absent_yields_credential_needed() {
    let (_dir, store, _backend) = store_with(FakeKeyringBackend::new());
    let resolver = resolver_with(store);
    let oauth = MockOAuthProvider::succeeding();
    match resolver.resolve_oauth(PROVIDER, &oauth).await {
        Err(AiProviderError::CredentialNeeded { provider_id }) => assert_eq!(provider_id, PROVIDER),
        other => panic!("expected CredentialNeeded, got {other:?}"),
    }
    assert_eq!(oauth.refresh_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn resolve_oauth_near_expiry_refreshes_and_writes_new_token() {
    let (_dir, store, _backend) = store_with(FakeKeyringBackend::new());
    store
        .write(
            PROVIDER,
            &oauth_credential(FRESH_ACCESS, Some(near_expiry())),
        )
        .await
        .unwrap();
    let resolver = resolver_with(store.clone());
    let oauth = MockOAuthProvider::succeeding();
    let resolved = resolver
        .resolve_oauth(PROVIDER, &oauth)
        .await
        .expect("refresh resolve");
    assert_eq!(resolved.secret.expose_secret(), REFRESHED_ACCESS);
    assert_eq!(oauth.refresh_calls.load(Ordering::SeqCst), 1);
    // The refreshed token is persisted (no partial write): a fresh resolver
    // reading the store sees the refreshed access token, not the stale one.
    let reread = store.read(PROVIDER).await.unwrap().unwrap();
    match reread {
        Credential::OAuthToken { access, .. } => {
            assert_eq!(access.expose_secret(), REFRESHED_ACCESS)
        }
        other => panic!("expected OAuthToken, got {other:?}"),
    }
}

#[tokio::test]
async fn resolve_oauth_refresh_failure_rereads_fresh_token_without_partial_write() {
    let (_dir, store, backend) = store_with(FakeKeyringBackend::new());
    store
        .write(
            PROVIDER,
            &oauth_credential(FRESH_ACCESS, Some(near_expiry())),
        )
        .await
        .unwrap();
    let resolver = resolver_with(store.clone());
    // Refresh fails, but a concurrent writer (simulated via the cloned backend)
    // has already written a fresh token to the shared store.
    let mut oauth = MockOAuthProvider::succeeding();
    oauth.fail = true;
    oauth.backend = Some(backend);
    oauth.inject_fresh = Mutex::new(Some(oauth_credential(
        REFRESHED_ACCESS,
        Some(fresh_expiry()),
    )));
    let resolved = resolver
        .resolve_oauth(PROVIDER, &oauth)
        .await
        .expect("reread resolve");
    assert_eq!(resolved.secret.expose_secret(), REFRESHED_ACCESS);
    assert_eq!(oauth.refresh_calls.load(Ordering::SeqCst), 1);
    // The store holds the concurrent-writer's fresh token (no partial write by us).
    let reread = store.read(PROVIDER).await.unwrap().unwrap();
    match reread {
        Credential::OAuthToken { access, .. } => {
            assert_eq!(access.expose_secret(), REFRESHED_ACCESS)
        }
        other => panic!("expected OAuthToken, got {other:?}"),
    }
}

#[tokio::test]
async fn resolve_oauth_refresh_failure_and_still_expired_returns_error() {
    let (_dir, store, _backend) = store_with(FakeKeyringBackend::new());
    store
        .write(
            PROVIDER,
            &oauth_credential(FRESH_ACCESS, Some(near_expiry())),
        )
        .await
        .unwrap();
    let resolver = resolver_with(store);
    let mut oauth = MockOAuthProvider::succeeding();
    oauth.fail = true;
    match resolver.resolve_oauth(PROVIDER, &oauth).await {
        Err(AiProviderError::Network(_)) => {}
        other => panic!("expected Network refresh error, got {other:?}"),
    }
    assert_eq!(oauth.refresh_calls.load(Ordering::SeqCst), 1);
}

// --- AuthSource::Store wraps resolve_oauth ---

#[tokio::test]
async fn auth_source_store_resolves_via_resolver_and_coalesces_to_bearer() {
    let (_dir, store, _backend) = store_with(FakeKeyringBackend::new());
    store
        .write(
            PROVIDER,
            &oauth_credential(FRESH_ACCESS, Some(fresh_expiry())),
        )
        .await
        .unwrap();
    let resolver = Arc::new(resolver_with(store));
    let src = AuthSource::Store {
        resolver,
        provider_id: PROVIDER.to_owned(),
        oauth: Arc::new(MockOAuthProvider::succeeding()),
    };
    let resolved = src.resolve().await.expect("store resolve");
    assert_eq!(resolved.scheme, AuthScheme::Bearer);
    assert_eq!(resolved.secret.expose_secret(), FRESH_ACCESS);
}

// ===========================================================================
// Slice 4 閳?MockLoginPresenter seam + AnthropicOAuthProvider PKCE flow
// ===========================================================================

// --- MockLoginPresenter seam ---

#[tokio::test]
async fn mock_login_presenter_await_manual_code_returns_preset_code() {
    let presenter = MockLoginPresenter::new();
    presenter.supply_manual_code("CODE-123");
    let code = presenter.await_manual_code().await.expect("manual code");
    assert_eq!(code, "CODE-123");
}

#[tokio::test]
async fn mock_login_presenter_await_manual_code_pending_when_no_sender_set() {
    // Without supply_manual_code, await_manual_code must stay pending so the
    // callback or timeout arm wins the race. Race it against a tiny sleep; the
    // sleep must win (proves the future is pending, not early-Err).
    let presenter = MockLoginPresenter::new();
    let outcome = tokio::select! {
        _ = presenter.await_manual_code() => "manual",
        _ = tokio::time::sleep(Duration::from_millis(50)) => "sleep",
    };
    assert_eq!(outcome, "sleep");
}

#[test]
fn extract_redirect_port_helper_parses_127_0_0_1_port_from_authorize_url() {
    let url = "https://authorize.example/oauth/authorize?response_type=code&client_id=cid&redirect_uri=http%3A%2F%2F127.0.0.1%3A54321%2F&code_challenge=xxx&code_challenge_method=S256&state=st";
    assert_eq!(extract_redirect_port(url), Some(54321));
    // Non-loopback host rejected.
    let url2 = "https://x?redirect_uri=http%3A%2F%2F10.0.0.1%3A80%2F";
    assert_eq!(extract_redirect_port(url2), None);
    // Absent param.
    assert_eq!(extract_redirect_port("https://x?foo=bar"), None);
}

#[test]
fn extract_query_param_decodes_state() {
    let url = "https://x?response_type=code&state=abc-def_123&redirect_uri=y";
    assert_eq!(
        extract_query_param(url, "state"),
        Some("abc-def_123".to_owned())
    );
    assert_eq!(extract_query_param(url, "missing"), None);
}

// --- AnthropicOAuthProvider ---

#[tokio::test]
async fn anthropic_oauth_provider_id_is_anthropic() {
    let provider = anthropic_provider("https://token.example".to_owned(), Duration::from_secs(60));
    assert_eq!(provider.id(), "anthropic");
}

#[tokio::test]
async fn anthropic_oauth_login_manual_code_wins_race() {
    let server = MockServer::start().await;
    mount_token_stub(&server, 200, token_body("atk-123", "rtk-123", 3600)).await;
    let provider = anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let presenter = MockLoginPresenter::new();
    presenter.supply_manual_code("AUTHCODE-123");
    let cred = provider.login(&presenter).await.expect("login");
    assert_eq!(cred.access.expose_secret(), "atk-123");
    assert_eq!(cred.refresh.expose_secret(), "rtk-123");
    assert!(cred.expires_at.is_some());
    assert_eq!(presenter.notify_success_count.load(Ordering::SeqCst), 1);
    let authorize_url = presenter.captured_url().expect("authorize URL");
    assert_eq!(
        extract_query_param(&authorize_url, "code").as_deref(),
        Some("true")
    );
    assert_eq!(
        extract_query_param(&authorize_url, "scope").as_deref(),
        Some(
            "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers user:file_upload"
        )
    );
    // The token POST received the manual code + a code_verifier + grant_type.
    let requests = server.received_requests().await.expect("recorded");
    let token_req = requests
        .iter()
        .find(|r| r.method == "POST" && r.url.as_str().contains("/oauth/token"))
        .expect("token POST");
    let body = std::str::from_utf8(&token_req.body).unwrap();
    assert!(
        body.contains("grant_type=authorization_code"),
        "body: {body}"
    );
    assert!(body.contains("code=AUTHCODE-123"), "body: {body}");
    assert!(body.contains("code_verifier="), "body: {body}");
    assert!(body.contains("client_id=client-id"), "body: {body}");
}

#[tokio::test]
async fn pkce_manual_redirect_url_is_normalized_for_anthropic_and_codex() {
    for provider_kind in BROWSER_PKCE_PROVIDERS {
        let server = MockServer::start().await;
        mount_token_stub(
            &server,
            200,
            token_body(&provider_kind.success_access(), "encoded-url-refresh", 3600),
        )
        .await;
        let provider = provider_kind.build(format!("{}/oauth/token", server.uri()));
        let presenter = MockLoginPresenter::new();
        let manual_code_tx = presenter.manual_code_sender();

        let login = provider.login(&presenter);
        let paste_redirect = async {
            presenter.wait_for_auth_url().await;
            let authorize_url = presenter.captured_url().expect("authorize URL");
            let state = extract_query_param(&authorize_url, "state").expect("state");
            let encoded_state = state
                .bytes()
                .map(|byte| format!("%{byte:02X}"))
                .collect::<Vec<_>>()
                .concat();
            manual_code_tx
                .send(format!(
                    "http://127.0.0.1/callback?code=encoded+code%2Fpart&state={encoded_state}"
                ))
                .expect("manual code receiver");
        };
        let (credential, ()) = tokio::join!(login, paste_redirect);

        credential.unwrap_or_else(|error| {
            panic!("{provider_kind:?} encoded redirect login failed: {error:?}")
        });
        let requests = server.received_requests().await.expect("requests");
        assert_eq!(requests.len(), 1);
        let body = std::str::from_utf8(&requests[0].body).expect("form body");
        assert!(body.contains("code=encoded+code%2Fpart"), "{body}");
    }
}

#[tokio::test]
async fn pkce_manual_redirect_rejections_prevent_token_exchange_for_both_providers() {
    for provider_kind in BROWSER_PKCE_PROVIDERS {
        for case in [
            "missing state",
            "state mismatch",
            "missing code",
            "malformed escape",
            "invalid UTF-8",
            "malformed URL",
            "duplicate code before state",
            "duplicate code after state",
            "duplicate state before code",
            "duplicate state after code",
        ] {
            let server = MockServer::start().await;
            mount_token_stub(
                &server,
                200,
                token_body(&provider_kind.success_access(), "unused-refresh", 3600),
            )
            .await;
            let provider = provider_kind.build(format!("{}/oauth/token", server.uri()));
            let presenter = MockLoginPresenter::new();
            let manual_code_tx = presenter.manual_code_sender();

            let login = provider.login(&presenter);
            let paste_redirect = async {
                presenter.wait_for_auth_url().await;
                let authorize_url = presenter.captured_url().expect("authorize URL");
                let state = extract_query_param(&authorize_url, "state").expect("state");
                let input = match case {
                    "missing state" => {
                        "http://127.0.0.1/callback?code=rejected-code-canary".to_owned()
                    }
                    "state mismatch" => {
                        "http://127.0.0.1/callback?code=rejected-code-canary&state=wrong-state"
                            .to_owned()
                    }
                    "missing code" => {
                        format!("http://127.0.0.1/callback?state={state}")
                    }
                    "malformed escape" => {
                        format!("http://127.0.0.1/callback?code=rejected%ZZcode&state={state}")
                    }
                    "invalid UTF-8" => {
                        format!("http://127.0.0.1/callback?code=invalid%FFcanary&state={state}")
                    }
                    "malformed URL" => "http://[::1/callback?code=rejected-code-canary".to_owned(),
                    "duplicate code before state" => format!(
                        "http://127.0.0.1/callback?code=first-code-canary&code=second-code-canary&state={state}"
                    ),
                    "duplicate code after state" => format!(
                        "http://127.0.0.1/callback?code=first-code-canary&state={state}&code=second-code-canary"
                    ),
                    "duplicate state before code" => format!(
                        "http://127.0.0.1/callback?state={state}&state=second-state-canary&code=code-canary"
                    ),
                    "duplicate state after code" => format!(
                        "http://127.0.0.1/callback?state={state}&code=code-canary&state=second-state-canary"
                    ),
                    _ => unreachable!(),
                };
                manual_code_tx.send(input).expect("manual code receiver");
            };
            let (result, ()) = tokio::join!(login, paste_redirect);

            let error = result.unwrap_err();
            let expected = match case {
                "missing state" => "oauth redirect missing state",
                "state mismatch" => "oauth state mismatch",
                "missing code" => "oauth redirect missing code",
                "malformed escape" => "oauth redirect query escape malformed",
                "invalid UTF-8" => "oauth redirect query is not valid UTF-8",
                "malformed URL" => "oauth redirect URL malformed",
                "duplicate code before state" | "duplicate code after state" => {
                    "oauth redirect has duplicate code"
                }
                "duplicate state before code" | "duplicate state after code" => {
                    "oauth redirect has duplicate state"
                }
                _ => unreachable!(),
            };
            match &error {
                AiProviderError::Config(message) => assert_eq!(message, expected),
                other => panic!("{provider_kind:?} {case}: expected Config, got {other:?}"),
            }
            assert_eq!(
                presenter.notify_failure_reasons.lock().unwrap().as_slice(),
                &["manual redirect error"]
            );
            assert_oauth_error_surfaces_are_redacted(
                &error,
                Some(&presenter),
                &[
                    "rejected-code-canary",
                    "rejected%ZZcode",
                    "wrong-state",
                    "invalid%FFcanary",
                    "first-code-canary",
                    "second-code-canary",
                    "second-state-canary",
                ],
            );
            assert!(
                server.received_requests().await.unwrap().is_empty(),
                "{provider_kind:?} {case} reached token endpoint"
            );
            assert_eq!(presenter.notify_success_count.load(Ordering::SeqCst), 0);
        }
    }
}

#[tokio::test]
async fn pkce_empty_manual_code_is_rejected_before_token_exchange_for_both_providers() {
    for provider_kind in BROWSER_PKCE_PROVIDERS {
        for input in ["", " \t\r\n "] {
            let server = MockServer::start().await;
            mount_token_stub(
                &server,
                200,
                token_body(&provider_kind.success_access(), "unused-refresh", 3600),
            )
            .await;
            let provider = provider_kind.build(format!("{}/oauth/token", server.uri()));
            let presenter = MockLoginPresenter::new();
            presenter.supply_manual_code(input);

            let error = provider
                .login(&presenter)
                .await
                .expect_err("empty manual code must fail");

            match &error {
                AiProviderError::Config(message) => {
                    assert_eq!(message, "oauth redirect missing code");
                }
                other => panic!("{provider_kind:?} expected Config, got {other:?}"),
            }
            assert_eq!(
                presenter.notify_failure_reasons.lock().unwrap().as_slice(),
                &["manual redirect error"]
            );
            assert_eq!(presenter.notify_success_count.load(Ordering::SeqCst), 0);
            assert!(
                server.received_requests().await.unwrap().is_empty(),
                "{provider_kind:?} posted an empty manual code"
            );
        }
    }
}

#[tokio::test]
async fn anthropic_oauth_login_loopback_callback_wins_race() {
    let server = MockServer::start().await;
    mount_token_stub(&server, 200, token_body("atk-cb", "rtk-cb", 3600)).await;
    let provider = anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let presenter = MockLoginPresenter::new();
    let login_fut = provider.login(&presenter);
    let drive_fut = async {
        presenter.wait_for_auth_url().await;
        let url = presenter.captured_url().expect("auth url");
        let port = extract_redirect_port(&url).expect("loopback port");
        let state = extract_query_param(&url, "state").expect("state");
        let encoded_state = state
            .bytes()
            .map(|byte| format!("%{byte:02X}"))
            .collect::<Vec<_>>()
            .concat();
        // The authorize URL carries code_challenge, never the verifier.
        assert!(url.contains("code_challenge="));
        assert!(!url.contains("code_verifier"));
        let _ = reqwest::get(format!(
            "http://127.0.0.1:{port}/?code=CB+CODE&state={encoded_state}"
        ))
        .await
        .expect("callback GET");
    };
    let (cred, _) = tokio::join!(login_fut, drive_fut);
    let cred = cred.expect("login");
    assert_eq!(cred.access.expose_secret(), "atk-cb");
    // Token POST used the callback code.
    let requests = server.received_requests().await.expect("recorded");
    let body = std::str::from_utf8(&requests.last().unwrap().body).unwrap();
    assert!(body.contains("code=CB+CODE"), "body: {body}");
    assert_eq!(presenter.notify_success_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn anthropic_oauth_login_callback_response_writes_minimal_200() {
    let server = MockServer::start().await;
    mount_token_stub(&server, 200, token_body("atk", "rtk", 3600)).await;
    let provider = anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let presenter = MockLoginPresenter::new();
    let login_fut = provider.login(&presenter);
    let drive_fut = async {
        presenter.wait_for_auth_url().await;
        let url = presenter.captured_url().expect("url");
        let port = extract_redirect_port(&url).expect("port");
        let state = extract_query_param(&url, "state").expect("state");
        let resp = reqwest::get(format!("http://127.0.0.1:{port}/?code=CB&state={state}"))
            .await
            .expect("GET");
        assert_eq!(resp.status(), 200);
        let body = resp.text().await.unwrap();
        assert!(!body.is_empty(), "empty callback response");
        // No secret in the callback response page.
        assert!(!body.contains("atk"), "token in callback page: {body}");
        assert!(!body.contains("CB"), "code in callback page: {body}");
    };
    let (cred, _) = tokio::join!(login_fut, drive_fut);
    cred.expect("login");
}

#[tokio::test]
async fn anthropic_oauth_login_state_mismatch_rejects_and_notifies_failure() {
    let server = MockServer::start().await;
    mount_token_stub(&server, 200, token_body("atk", "rtk", 3600)).await;
    let provider = anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let presenter = MockLoginPresenter::new();
    let login_fut = provider.login(&presenter);
    let drive_fut = async {
        presenter.wait_for_auth_url().await;
        let url = presenter.captured_url().expect("auth url");
        let port = extract_redirect_port(&url).expect("port");
        // Send a deliberately wrong state.
        let _ = reqwest::get(format!(
            "http://127.0.0.1:{port}/?code=CB&state=WRONG-STATE"
        ))
        .await;
    };
    let (outcome, _) = tokio::join!(login_fut, drive_fut);
    let err = outcome.expect_err("state mismatch should fail");
    assert!(matches!(err, AiProviderError::Config(_)), "got {err:?}");
    let reasons = presenter.notify_failure_reasons.lock().unwrap().clone();
    assert!(
        reasons.iter().any(|r| r == "state mismatch"),
        "reasons: {reasons:?}"
    );
    // The mismatched state value must not leak into the reason.
    assert!(
        !reasons.iter().any(|r| r.contains("WRONG-STATE")),
        "leaked state: {reasons:?}"
    );
    drop(reasons);
    // No token exchange happened.
    assert!(server.received_requests().await.unwrap().is_empty());
    assert_eq!(presenter.notify_success_count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn pkce_loopback_callback_rejections_are_fixed_and_redacted_for_both_providers() {
    for provider_kind in BROWSER_PKCE_PROVIDERS {
        for case in [
            "missing code",
            "missing state",
            "state mismatch",
            "malformed escape",
            "invalid UTF-8",
            "duplicate code",
            "duplicate state",
        ] {
            let server = MockServer::start().await;
            mount_token_stub(
                &server,
                200,
                token_body(&provider_kind.success_access(), "unused-refresh", 3600),
            )
            .await;
            let provider = provider_kind.build(format!("{}/oauth/token", server.uri()));
            let presenter = MockLoginPresenter::new();

            let login = provider.login(&presenter);
            let callback = async {
                presenter.wait_for_auth_url().await;
                let authorize_url = presenter.captured_url().expect("authorize URL");
                let port = extract_redirect_port(&authorize_url).expect("redirect port");
                let state = extract_query_param(&authorize_url, "state").expect("state");
                let query = match case {
                    "missing code" => format!("state={state}"),
                    "missing state" => "code=callback-code-canary".to_owned(),
                    "state mismatch" => {
                        "code=callback-code-canary&state=callback-state-canary".to_owned()
                    }
                    "malformed escape" => {
                        format!("code=callback%ZZcode&state={state}")
                    }
                    "invalid UTF-8" => format!("code=%FF&state={state}"),
                    "duplicate code" => {
                        format!("code=first-code-canary&state={state}&code=second-code-canary")
                    }
                    "duplicate state" => {
                        format!("state={state}&code=callback-code-canary&state=second-state-canary")
                    }
                    _ => unreachable!(),
                };
                reqwest::get(format!("http://127.0.0.1:{port}/?{query}"))
                    .await
                    .expect("callback GET")
                    .status()
                    .as_u16()
            };
            let (result, status) = tokio::time::timeout(Duration::from_secs(2), async {
                tokio::join!(login, callback)
            })
            .await
            .expect("callback login must not hang");

            assert_eq!(
                status, 400,
                "{provider_kind:?} {case}: invalid callback must receive a 400 response, not a success page (C-4.1)"
            );

            let error = result.expect_err("invalid callback must fail");
            let (expected_error, expected_notification) = match case {
                "missing code" => ("oauth redirect missing code", "callback parse error"),
                "missing state" => ("oauth redirect missing state", "callback parse error"),
                "state mismatch" => ("oauth state mismatch", "state mismatch"),
                "malformed escape" => (
                    "oauth redirect query escape malformed",
                    "callback parse error",
                ),
                "invalid UTF-8" => (
                    "oauth redirect query is not valid UTF-8",
                    "callback parse error",
                ),
                "duplicate code" => ("oauth redirect has duplicate code", "callback parse error"),
                "duplicate state" => ("oauth redirect has duplicate state", "callback parse error"),
                _ => unreachable!(),
            };
            match &error {
                AiProviderError::Config(message) => assert_eq!(message, expected_error),
                other => panic!("{provider_kind:?} {case}: expected Config, got {other:?}"),
            }
            assert_eq!(
                presenter.notify_failure_reasons.lock().unwrap().as_slice(),
                &[expected_notification]
            );
            assert_oauth_error_surfaces_are_redacted(
                &error,
                Some(&presenter),
                &[
                    "callback-code-canary",
                    "callback-state-canary",
                    "first-code-canary",
                    "second-code-canary",
                    "second-state-canary",
                ],
            );
            assert_eq!(presenter.notify_success_count.load(Ordering::SeqCst), 0);
            assert!(
                server.received_requests().await.unwrap().is_empty(),
                "{provider_kind:?} {case} reached the token endpoint"
            );
        }
    }
}

#[tokio::test]
async fn anthropic_oauth_login_timeout_returns_error_and_notifies_failure() {
    let server = MockServer::start().await;
    let provider = anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_millis(100),
    );
    let presenter = MockLoginPresenter::new();
    // No manual code, no callback GET -> timeout wins.
    let err = provider.login(&presenter).await.expect_err("timeout");
    assert!(matches!(err, AiProviderError::Timeout), "got {err:?}");
    let reasons = presenter.notify_failure_reasons.lock().unwrap().clone();
    assert!(
        reasons.iter().any(|r| r == "timeout"),
        "reasons: {reasons:?}"
    );
    assert_eq!(presenter.notify_success_count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn oauth_flow_budget_rejects_unrepresentable_duration_without_panicking() {
    let provider = anthropic_provider("http://127.0.0.1:1/oauth/token".to_owned(), Duration::MAX);
    let presenter = MockLoginPresenter::new();

    let outcome = std::panic::AssertUnwindSafe(provider.login(&presenter))
        .catch_unwind()
        .await;
    let result = outcome.expect("an oversized OAuth flow duration must not panic");
    assert!(
        matches!(
            result,
            Err(AiProviderError::Config(ref message))
                if message == "OAuth login timeout is too large"
        ),
        "{result:?}"
    );
}

#[tokio::test]
async fn pkce_total_budget_includes_manual_wait_and_token_response_for_both_providers() {
    for provider_kind in BROWSER_PKCE_PROVIDERS {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(token_body(
                        &provider_kind.success_access(),
                        "budget-refresh",
                        3600,
                    ))
                    .set_delay(Duration::from_millis(80)),
            )
            .mount(&server)
            .await;
        let provider = provider_kind.build_with_timeout(
            format!("{}/oauth/token", server.uri()),
            Duration::from_millis(100),
        );
        let presenter = MockLoginPresenter::new();
        let manual_code = presenter.manual_code_sender();

        let login = provider.login(&presenter);
        let delayed_code = async {
            presenter.wait_for_auth_url().await;
            tokio::time::sleep(Duration::from_millis(60)).await;
            manual_code.send("budget-code".to_owned()).unwrap();
        };
        let (result, ()) = tokio::join!(login, delayed_code);

        assert!(
            matches!(result, Err(AiProviderError::Timeout)),
            "{provider_kind:?} restarted its timeout before token response: {result:?}"
        );
        assert_eq!(
            presenter.notify_failure_reasons.lock().unwrap().as_slice(),
            &["timeout"]
        );
        assert_eq!(presenter.notify_success_count.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn pkce_total_budget_bounds_token_response_body_after_headers() {
    for provider_kind in BROWSER_PKCE_PROVIDERS {
        let (token_route, token_probe) = raw_json_route(
            "POST",
            "/oauth/token",
            token_body(
                &provider_kind.success_access(),
                "delayed-body-refresh",
                3600,
            ),
            Duration::from_millis(250),
        );
        let server = RawBodyDelayServer::start(vec![token_route]).await;
        let provider = provider_kind.build_with_timeout(
            format!("{}/oauth/token", server.uri()),
            Duration::from_millis(100),
        );
        let presenter = MockLoginPresenter::new();
        presenter.supply_manual_code("one-use-code");

        let result = provider.login(&presenter).await;
        token_probe.assert_body_boundary_reached().await;

        assert!(
            matches!(result, Err(AiProviderError::Timeout)),
            "{provider_kind:?} did not bound token response body decoding: {result:?}"
        );
        assert_eq!(
            presenter.notify_failure_reasons.lock().unwrap().as_slice(),
            &["timeout"]
        );
        assert_eq!(presenter.notify_success_count.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn pkce_cancellation_is_biased_and_prevents_exchange_and_persistence() {
    for provider_kind in BROWSER_PKCE_PROVIDERS {
        let server = MockServer::start().await;
        mount_token_stub(
            &server,
            200,
            token_body(&provider_kind.success_access(), "unused-refresh", 3600),
        )
        .await;
        let provider: Arc<dyn OAuthProvider> = match provider_kind {
            BrowserPkceProvider::Anthropic => Arc::new(anthropic_provider(
                format!("{}/oauth/token", server.uri()),
                Duration::from_secs(60),
            )),
            BrowserPkceProvider::Codex => Arc::new(codex_provider(
                format!("{}/oauth/token", server.uri()),
                Duration::from_secs(60),
            )),
        };
        let mut registry = OAuthProviderRegistry::new();
        registry.register(provider).unwrap();
        let (_dir, store, _backend) = store_with(FakeKeyringBackend::new());
        let presenter = MockLoginPresenter::new();
        let cancel = presenter.login_cancelled_sender();
        let manual_code = presenter.manual_code_sender();
        let login = login_oauth(provider_kind.provider_id(), &registry, &store, &presenter);
        let make_both_ready = async {
            presenter.wait_for_auth_url().await;
            manual_code.send("ready-code-must-lose".to_owned()).unwrap();
            cancel.send(()).unwrap();
        };
        let (result, ()) = tokio::join!(login, make_both_ready);
        let error = result.expect_err("ready cancellation must win the browser fallback race");

        assert!(matches!(
            error,
            AiProviderError::LoginCancelled { ref provider_id }
                if provider_id == provider_kind.provider_id()
        ));
        assert_eq!(
            presenter.notify_failure_reasons.lock().unwrap().as_slice(),
            &["login cancelled"]
        );
        assert_eq!(presenter.notify_success_count.load(Ordering::SeqCst), 0);
        assert!(
            server.received_requests().await.unwrap().is_empty(),
            "{provider_kind:?} exchanged a code after cancellation"
        );
        assert!(
            store
                .read(provider_kind.provider_id())
                .await
                .unwrap()
                .is_none()
        );
        let authorize_url = presenter.captured_url().expect("authorization URL");
        let port = extract_redirect_port(&authorize_url).expect("loopback port");
        assert!(
            reqwest::get(format!("http://127.0.0.1:{port}/?code=late&state=late"))
                .await
                .is_err(),
            "{provider_kind:?} left its callback listener open"
        );
    }
}

#[tokio::test]
async fn pkce_post_code_cancellation_is_ignored_but_original_deadline_still_applies() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(token_body("late-access", "late-refresh", 3600))
                .set_delay(Duration::from_millis(80)),
        )
        .mount(&server)
        .await;
    let provider = anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_millis(100),
    );
    let presenter = MockLoginPresenter::new();
    let manual_code = presenter.manual_code_sender();
    let cancel = presenter.login_cancelled_sender();

    let login = provider.login(&presenter);
    let drive = async {
        presenter.wait_for_auth_url().await;
        tokio::time::sleep(Duration::from_millis(40)).await;
        manual_code.send("one-use-code".to_owned()).unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        let _ = cancel.send(());
    };
    let (result, ()) = tokio::join!(login, drive);

    assert!(
        matches!(result, Err(AiProviderError::Timeout)),
        "post-code cancellation must not replace the bounded exchange result: {result:?}"
    );
    assert_eq!(
        presenter.notify_failure_reasons.lock().unwrap().as_slice(),
        &["timeout"]
    );
}

#[tokio::test]
async fn anthropic_oauth_login_token_endpoint_non_2xx_returns_auth_error_without_body() {
    let server = MockServer::start().await;
    mount_token_stub(
        &server,
        400,
        json!({"error":"invalid_grant","error_description":"bad code"}),
    )
    .await;
    let provider = anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let presenter = MockLoginPresenter::new();
    presenter.supply_manual_code("AUTHCODE-LEAK-CANARY");
    let err = provider.login(&presenter).await.expect_err("non-2xx");
    let msg = match &err {
        AiProviderError::AuthFailed(m) => m,
        other => panic!("expected AuthFailed, got {other:?}"),
    };
    assert_eq!(
        msg, "token endpoint: 400 Bad Request invalid_grant",
        "known protocol error must map to its fixed class"
    );
    assert_oauth_error_surfaces_are_redacted(&err, Some(&presenter), &["AUTHCODE-LEAK-CANARY"]);
    let reasons = presenter.notify_failure_reasons.lock().unwrap().clone();
    assert!(
        reasons.iter().any(|r| r.contains("token")),
        "reasons: {reasons:?}"
    );
    assert!(
        !reasons.iter().any(|r| r.contains("AUTHCODE-LEAK-CANARY")),
        "leaked: {reasons:?}"
    );
}

#[tokio::test]
async fn anthropic_oauth_login_missing_refresh_token_is_hard_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"access_token":"atk","expires_in":3600})),
        )
        .mount(&server)
        .await;
    let provider = anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let presenter = MockLoginPresenter::new();
    presenter.supply_manual_code("CODE");
    let err = provider
        .login(&presenter)
        .await
        .expect_err("missing refresh");
    assert!(matches!(err, AiProviderError::Config(_)), "got {err:?}");
}

#[tokio::test]
async fn anthropic_oauth_login_missing_expires_in_is_hard_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"access_token":"atk","refresh_token":"rtk"})),
        )
        .mount(&server)
        .await;
    let provider = anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let presenter = MockLoginPresenter::new();
    presenter.supply_manual_code("CODE");
    let err = provider
        .login(&presenter)
        .await
        .expect_err("missing expires_in");
    assert!(matches!(err, AiProviderError::Config(_)), "got {err:?}");
}

#[tokio::test]
async fn anthropic_oauth_login_listener_dropped_after_manual_code_wins() {
    let server = MockServer::start().await;
    mount_token_stub(&server, 200, token_body("atk", "rtk", 3600)).await;
    let provider = anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let presenter = MockLoginPresenter::new();
    presenter.supply_manual_code("CODE");
    let _ = provider.login(&presenter).await.expect("login");
    let url = presenter.captured_url().expect("url");
    let port = extract_redirect_port(&url).expect("port");
    // The listener must have been dropped; a GET to the port is now refused.
    let refused = reqwest::get(format!("http://127.0.0.1:{port}/?code=X&state=Y")).await;
    assert!(refused.is_err(), "listener not dropped: {refused:?}");
}

// --- AnthropicOAuthProvider::refresh ---

#[tokio::test]
async fn anthropic_oauth_refresh_exchanges_refresh_token_and_preserves_base_url_none() {
    let server = MockServer::start().await;
    mount_token_stub(&server, 200, token_body("atk-new", "rtk-new", 3600)).await;
    let provider = anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let cred = oauth_cred("atk-old", "rtk-old", None);
    let refreshed = provider.refresh(&cred).await.expect("refresh");
    assert_eq!(refreshed.access.expose_secret(), "atk-new");
    assert_eq!(refreshed.refresh.expose_secret(), "rtk-new");
    assert!(refreshed.base_url.is_none());
    let requests = server.received_requests().await.unwrap();
    let body = std::str::from_utf8(&requests[0].body).unwrap();
    assert!(body.contains("grant_type=refresh_token"), "body: {body}");
    assert!(body.contains("refresh_token=rtk-old"), "body: {body}");
}

#[tokio::test]
async fn anthropic_oauth_refresh_reuses_old_refresh_token_when_response_omits_it() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"access_token":"atk-new","expires_in":3600})),
        )
        .mount(&server)
        .await;
    let provider = anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let cred = oauth_cred("atk-old", "rtk-old", None);
    let refreshed = provider.refresh(&cred).await.expect("refresh");
    assert_eq!(refreshed.refresh.expose_secret(), "rtk-old");
    assert_eq!(refreshed.access.expose_secret(), "atk-new");
}

#[tokio::test]
async fn anthropic_oauth_refresh_preserves_non_none_base_url() {
    let server = MockServer::start().await;
    mount_token_stub(&server, 200, token_body("atk-new", "rtk-new", 3600)).await;
    let provider = anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let cred = oauth_cred(
        "atk-old",
        "rtk-old",
        Some("https://enterprise.example".to_owned()),
    );
    let refreshed = provider.refresh(&cred).await.expect("refresh");
    assert_eq!(
        refreshed.base_url.as_deref(),
        Some("https://enterprise.example")
    );
}

#[tokio::test]
async fn anthropic_oauth_refresh_token_endpoint_non_2xx_returns_auth_error_without_body() {
    let server = MockServer::start().await;
    mount_token_stub(&server, 400, json!({"error":"invalid_grant"})).await;
    let provider = anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let cred = oauth_cred("atk", "rtk-LEAK-CANARY", None);
    let err = provider.refresh(&cred).await.expect_err("non-2xx");
    let msg = match err {
        AiProviderError::AuthFailed(m) => m,
        other => panic!("expected AuthFailed, got {other:?}"),
    };
    assert!(
        !msg.contains("rtk-LEAK-CANARY"),
        "refresh token leaked: {msg}"
    );
}

#[tokio::test]
async fn anthropic_oauth_refresh_401_returns_credential_revoked() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({"error":"invalid_token"})))
        .mount(&server)
        .await;
    let provider = anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let cred = oauth_cred("atk", "rtk-LEAK-CANARY", None);
    let err = provider.refresh(&cred).await.expect_err("401");
    match err {
        AiProviderError::CredentialRevoked { provider_id } => assert_eq!(provider_id, "anthropic"),
        other => panic!("expected CredentialRevoked, got {other:?}"),
    }
}

// --- Cross-flow secret-leak canary ---

#[tokio::test]
async fn login_secret_leak_canary_no_auth_code_in_outputs() {
    let server = MockServer::start().await;
    mount_token_stub(
        &server,
        400,
        json!({"error":"invalid_grant","error_description":"bad"}),
    )
    .await;
    let provider = anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let presenter = MockLoginPresenter::new();
    presenter.supply_manual_code("AUTHCODE-LEAK-CANARY");
    let err = provider.login(&presenter).await.expect_err("fail");
    let msg = err.to_string();
    assert!(
        !msg.contains("AUTHCODE-LEAK-CANARY"),
        "code in error: {msg}"
    );
    let reasons = presenter.notify_failure_reasons.lock().unwrap().clone();
    for r in reasons.iter() {
        assert!(
            !r.contains("AUTHCODE-LEAK-CANARY"),
            "code in notify_failure: {r}"
        );
    }
    drop(reasons);
    let url = presenter.captured_url().expect("url");
    assert!(
        !url.contains("AUTHCODE-LEAK-CANARY"),
        "code in auth_url: {url}"
    );
}

// ===========================================================================
// Slice 4 閳?CodexOAuthProvider (PKCE, delegates to the shared runner)
// ===========================================================================

fn assert_oauth_error_surfaces_are_redacted(
    error: &AiProviderError,
    presenter: Option<&MockLoginPresenter>,
    canaries: &[&str],
) {
    let mut surfaces = vec![
        error.to_string(),
        format!("{error:?}"),
        format!("{:?}", opi_agent::Diagnostic::from(error)),
    ];
    if let Some(presenter) = presenter {
        surfaces.extend(presenter.notify_failure_reasons.lock().unwrap().clone());
    }
    let rendered = surfaces.join("\n");
    for canary in canaries {
        assert!(!rendered.contains(canary), "leaked {canary}: {rendered}");
    }
}

#[tokio::test]
async fn pkce_token_endpoint_unknown_error_code_is_closed_and_redacted() {
    const AUTHORIZATION_CODE: &str = "authorization-code-server-error-canary";
    const VERIFIER: &str = "verifier-server-error-canary";

    let server = MockServer::start().await;
    mount_token_stub(
        &server,
        400,
        json!({
            "error": format!("invalid_grant:{AUTHORIZATION_CODE}:{VERIFIER}"),
            "error_description": format!("{AUTHORIZATION_CODE}:{VERIFIER}")
        }),
    )
    .await;
    let provider = anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let presenter = MockLoginPresenter::new();
    presenter.supply_manual_code(AUTHORIZATION_CODE);

    let error = provider.login(&presenter).await.expect_err("token error");

    match &error {
        AiProviderError::AuthFailed(message) => assert_eq!(
            message,
            "token endpoint: 400 Bad Request unknown_oauth_error"
        ),
        other => panic!("expected AuthFailed, got {other:?}"),
    }
    assert_oauth_error_surfaces_are_redacted(
        &error,
        Some(&presenter),
        &[AUTHORIZATION_CODE, VERIFIER],
    );
}

#[tokio::test]
async fn refresh_token_endpoint_unknown_error_code_is_closed_and_redacted() {
    const REFRESH_TOKEN: &str = "refresh-token-server-error-canary";

    let server = MockServer::start().await;
    mount_token_stub(
        &server,
        400,
        json!({
            "error": format!("invalid_grant:{REFRESH_TOKEN}"),
            "error_description": REFRESH_TOKEN
        }),
    )
    .await;
    let provider = anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let credential = oauth_cred("old-access", REFRESH_TOKEN, None);

    let error = provider
        .refresh(&credential)
        .await
        .expect_err("refresh error");

    match &error {
        AiProviderError::AuthFailed(message) => assert_eq!(
            message,
            "token endpoint: 400 Bad Request unknown_oauth_error"
        ),
        other => panic!("expected AuthFailed, got {other:?}"),
    }
    assert_oauth_error_surfaces_are_redacted(&error, None, &[REFRESH_TOKEN]);
}

#[tokio::test]
async fn codex_oauth_provider_id_is_codex() {
    let provider = codex_provider("https://token.example".to_owned(), Duration::from_secs(60));
    assert_eq!(provider.id(), "openai-codex");
}

#[tokio::test]
async fn codex_login_manual_code_wins_drives_token_post_and_returns_credential() {
    let server = MockServer::start().await;
    let access = codex_jwt(Some("account-browser"));
    mount_token_stub(&server, 200, token_body(&access, "codex-rtk", 3600)).await;
    let provider = codex_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let presenter = MockLoginPresenter::new();
    presenter.supply_manual_code("CODE-XYZ");
    let cred = provider.login(&presenter).await.expect("login");
    assert_eq!(cred.access.expose_secret(), access);
    assert_eq!(cred.refresh.expose_secret(), "codex-rtk");
    assert_eq!(cred.account_id.as_deref(), Some("account-browser"));
    assert_eq!(presenter.notify_success_count.load(Ordering::SeqCst), 1);
    let authorize_url = presenter.captured_url().expect("authorize URL");
    assert_eq!(
        extract_query_param(&authorize_url, "scope").as_deref(),
        Some("openid profile email offline_access")
    );
    assert_eq!(
        extract_query_param(&authorize_url, "id_token_add_organizations").as_deref(),
        Some("true")
    );
    assert_eq!(
        extract_query_param(&authorize_url, "codex_cli_simplified_flow").as_deref(),
        Some("true")
    );
    assert_eq!(
        extract_query_param(&authorize_url, "originator").as_deref(),
        Some("opi")
    );
    let requests = server.received_requests().await.unwrap();
    let body = std::str::from_utf8(&requests[0].body).unwrap();
    assert!(body.contains("code=CODE-XYZ"), "body: {body}");
    assert!(body.contains("code_verifier="), "body: {body}");
    assert!(body.contains("client_id=codex-client-id"), "body: {body}");
}

#[tokio::test]
async fn codex_login_callback_wins_completes_token_exchange_and_returns_credential() {
    let server = MockServer::start().await;
    let access = codex_jwt(Some("account-callback"));
    mount_token_stub(&server, 200, token_body(&access, "codex-rtk", 3600)).await;
    let provider = codex_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let presenter = MockLoginPresenter::new();
    let login_fut = provider.login(&presenter);
    let drive_fut = async {
        presenter.wait_for_auth_url().await;
        let url = presenter.captured_url().expect("url");
        let port = extract_redirect_port(&url).expect("port");
        let state = extract_query_param(&url, "state").expect("state");
        let _ = reqwest::get(format!(
            "http://127.0.0.1:{port}/?code=CB-CODEX&state={state}"
        ))
        .await
        .expect("GET");
    };
    let (cred, _) = tokio::join!(login_fut, drive_fut);
    let cred = cred.expect("login");
    assert_eq!(cred.access.expose_secret(), access);
    assert_eq!(cred.account_id.as_deref(), Some("account-callback"));
}

#[tokio::test]
async fn codex_login_state_mismatch_rejects_callback_and_notifies_failure() {
    let server = MockServer::start().await;
    mount_token_stub(
        &server,
        200,
        token_body(&codex_jwt(Some("unused-account")), "codex-rtk", 3600),
    )
    .await;
    let provider = codex_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let presenter = MockLoginPresenter::new();
    let login_fut = provider.login(&presenter);
    let drive_fut = async {
        presenter.wait_for_auth_url().await;
        let url = presenter.captured_url().expect("url");
        let port = extract_redirect_port(&url).expect("port");
        let _ = reqwest::get(format!("http://127.0.0.1:{port}/?code=CB&state=WRONG")).await;
    };
    let (outcome, _) = tokio::join!(login_fut, drive_fut);
    let err = outcome.expect_err("mismatch");
    assert!(matches!(err, AiProviderError::Config(_)), "got {err:?}");
    let reasons = presenter.notify_failure_reasons.lock().unwrap().clone();
    assert!(
        reasons.iter().any(|r| r == "state mismatch"),
        "reasons: {reasons:?}"
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn codex_login_timeout_returns_error_without_token_exchange() {
    let server = MockServer::start().await;
    let provider = codex_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_millis(100),
    );
    let presenter = MockLoginPresenter::new();
    let err = provider.login(&presenter).await.expect_err("timeout");
    assert!(matches!(err, AiProviderError::Timeout), "got {err:?}");
    let reasons = presenter.notify_failure_reasons.lock().unwrap().clone();
    assert!(
        reasons.iter().any(|r| r == "timeout"),
        "reasons: {reasons:?}"
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn codex_refresh_preserves_base_url_and_handles_missing_refresh_token() {
    let server = MockServer::start().await;
    // Response omits refresh_token -> Codex reuses the old one (rotation-optional).
    let new_access = codex_jwt(Some("account-refreshed"));
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"access_token":new_access,"expires_in":3600})),
        )
        .mount(&server)
        .await;
    let provider = codex_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let cred = oauth_cred(
        "codex-old",
        "codex-rtk-old",
        Some("https://enterprise".to_owned()),
    );
    let refreshed = provider.refresh(&cred).await.expect("refresh");
    assert_eq!(refreshed.access.expose_secret(), new_access);
    assert_eq!(refreshed.refresh.expose_secret(), "codex-rtk-old");
    assert_eq!(refreshed.base_url.as_deref(), Some("https://enterprise"));
    assert_eq!(refreshed.account_id.as_deref(), Some("account-refreshed"));
}

#[tokio::test]
async fn openai_codex_login_rejects_token_without_chatgpt_account_id() {
    let server = MockServer::start().await;
    let missing_account_jwt = codex_jwt(None);
    mount_token_stub(
        &server,
        200,
        token_body(&missing_account_jwt, "sentinel-refresh", 3600),
    )
    .await;
    let provider = codex_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let presenter = MockLoginPresenter::new();
    presenter.supply_manual_code("sentinel-authorization-code");
    let error = provider.login(&presenter).await.expect_err("account id");
    assert!(matches!(
        error,
        AiProviderError::AccountIdMissing { ref provider_id }
            if provider_id == "openai-codex"
    ));
    let rendered = format!("{error:?} {error}");
    for sentinel in [
        missing_account_jwt.as_str(),
        "sentinel-refresh",
        "sentinel-authorization-code",
    ] {
        assert!(!rendered.contains(sentinel));
    }
    assert!(!error.is_retryable());
}

#[tokio::test]
async fn openai_codex_refresh_rejects_token_without_chatgpt_account_id() {
    let server = MockServer::start().await;
    let missing_account_jwt = codex_jwt(None);
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": missing_account_jwt,
            "refresh_token": "sentinel-refresh-new",
            "expires_in": 3600
        })))
        .mount(&server)
        .await;
    let provider = codex_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let mut credential = oauth_cred("sentinel-access-old", "sentinel-refresh-old", None);
    credential.account_id = Some("account-old".into());
    let error = provider
        .refresh(&credential)
        .await
        .expect_err("new account id missing");
    assert!(matches!(
        error,
        AiProviderError::AccountIdMissing { ref provider_id }
            if provider_id == "openai-codex"
    ));
    let rendered = format!("{error:?} {error}");
    for sentinel in [
        missing_account_jwt.as_str(),
        "sentinel-refresh-new",
        "sentinel-refresh-old",
        "sentinel-access-old",
    ] {
        assert!(!rendered.contains(sentinel));
    }
}

#[tokio::test]
async fn codex_login_does_not_leak_verifier_or_tokens_into_error_strings() {
    let server = MockServer::start().await;
    mount_token_stub(
        &server,
        500,
        json!({
            "error": "server_error",
            "error_description": "sentinel-token-endpoint-echo"
        }),
    )
    .await;
    let provider = codex_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let presenter = MockLoginPresenter::new();
    presenter.supply_manual_code("CODE-VERIFIER-TEST");
    let err = provider.login(&presenter).await.expect_err("500");
    let msg = err.to_string();
    // Capture the verifier actually submitted in the POST body and prove it is
    // not echoed into the error (no reqwest::Error::to_string into ProviderError).
    let requests = server.received_requests().await.unwrap();
    let body = std::str::from_utf8(&requests[0].body).unwrap();
    let verifier = body
        .split("code_verifier=")
        .nth(1)
        .and_then(|s| s.split('&').next())
        .unwrap_or("");
    assert!(!verifier.is_empty(), "no verifier submitted: {body}");
    assert!(!msg.contains(verifier), "verifier leaked: {msg}");
    assert!(
        !msg.contains("CODE-VERIFIER-TEST"),
        "auth code leaked: {msg}"
    );
    assert!(
        !msg.contains("sentinel-token-endpoint-echo"),
        "token endpoint detail leaked: {msg}"
    );
}

// ===========================================================================
// Slice 4 閳?OAuthProviderRegistry
// ===========================================================================

async fn mount_codex_device_start(
    server: &MockServer,
    device_auth_id: &str,
    user_code: &str,
    interval: u64,
) {
    Mock::given(method("POST"))
        .and(path("/device/usercode"))
        .and(body_json(json!({"client_id":"codex-client-id"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_auth_id": device_auth_id,
            "user_code": user_code,
            "interval": interval
        })))
        .mount(server)
        .await;
}

async fn mount_codex_device_poll(
    server: &MockServer,
    status: u16,
    body: serde_json::Value,
    times: Option<u64>,
) {
    let mut mock = Mock::given(method("POST"))
        .and(path("/device/token"))
        .and(body_json(json!({
            "device_auth_id": "device-auth-sentinel",
            "user_code": "PUBLIC-CODE"
        })))
        .respond_with(ResponseTemplate::new(status).set_body_json(body));
    if let Some(times) = times {
        mock = mock.up_to_n_times(times);
    }
    mock.mount(server).await;
}

async fn mount_codex_device_exchange(server: &MockServer, account_id: &str) {
    mount_token_stub(
        server,
        200,
        token_body(
            &codex_jwt(Some(account_id)),
            "device-refresh-sentinel",
            3600,
        ),
    )
    .await;
}

#[tokio::test]
async fn openai_codex_browser_is_default_and_preserves_pkce_manual_race() {
    let server = MockServer::start().await;
    mount_token_stub(
        &server,
        200,
        token_body(
            &codex_jwt(Some("browser-default-account")),
            "browser-refresh",
            3600,
        ),
    )
    .await;
    let provider = codex_device_provider(&server.uri(), Duration::from_secs(60));
    let presenter = MethodPresenter::new(Some(OAuthLoginMethod::Browser));
    presenter.inner.supply_manual_code("browser-manual-code");

    let credential = provider.login(&presenter).await.expect("browser login");

    assert_eq!(
        credential.account_id.as_deref(),
        Some("browser-default-account")
    );
    assert_eq!(presenter.inner.manual_code_calls.load(Ordering::SeqCst), 1);
    let selections = presenter.selections.lock().unwrap();
    assert_eq!(
        selections.as_slice(),
        &[(
            "openai-codex".to_owned(),
            vec![OAuthLoginMethod::Browser, OAuthLoginMethod::DeviceCode],
            OAuthLoginMethod::Browser,
        )]
    );
}

#[tokio::test]
async fn openai_codex_browser_budget_includes_method_selection() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(token_body(
                    &codex_jwt(Some("slow-selection-account")),
                    "slow-selection-refresh",
                    3600,
                ))
                .set_delay(Duration::from_millis(80)),
        )
        .mount(&server)
        .await;
    let provider = codex_provider_with_flow_timeouts(
        &server.uri(),
        Duration::from_millis(100),
        Duration::from_secs(2),
    );
    let presenter = MethodPresenter::delayed_method_selection(
        OAuthLoginMethod::Browser,
        Duration::from_millis(60),
    );
    presenter.inner.supply_manual_code("slow-selection-code");
    let started = tokio::time::Instant::now();

    let error = provider
        .login(&presenter)
        .await
        .expect_err("Browser selection must consume the Browser flow budget");

    assert!(matches!(error, AiProviderError::Timeout), "{error:?}");
    assert!(
        started.elapsed() < Duration::from_millis(140),
        "Browser flow received a fresh post-selection deadline"
    );
    assert_eq!(
        presenter
            .inner
            .notify_failure_reasons
            .lock()
            .unwrap()
            .as_slice(),
        &["timeout"]
    );
}

#[tokio::test]
async fn openai_codex_device_selection_preserves_its_longer_budget() {
    let server = MockServer::start().await;
    mount_codex_device_start(&server, "device-auth-sentinel", "PUBLIC-CODE", 0).await;
    mount_codex_device_poll(
        &server,
        200,
        json!({
            "authorization_code":"device-authorization-code",
            "code_verifier":"device-code-verifier"
        }),
        None,
    )
    .await;
    mount_codex_device_exchange(&server, "device-selection-account").await;
    let provider = codex_provider_with_flow_timeouts(
        &server.uri(),
        Duration::from_millis(50),
        Duration::from_millis(500),
    );
    let presenter = MethodPresenter::delayed_method_selection(
        OAuthLoginMethod::DeviceCode,
        Duration::from_millis(100),
    );

    let credential = provider
        .login(&presenter)
        .await
        .expect("Device selection must use its distinct longer budget");

    assert_eq!(
        credential.account_id.as_deref(),
        Some("device-selection-account")
    );
}

#[tokio::test]
async fn openai_codex_typed_method_selection_cancellation_notifies_once() {
    let server = MockServer::start().await;
    let provider = codex_device_provider(&server.uri(), Duration::from_secs(2));
    let presenter = MethodPresenter::new(None);

    let error = provider
        .login(&presenter)
        .await
        .expect_err("typed selection cancellation");

    assert!(matches!(
        error,
        AiProviderError::LoginCancelled { ref provider_id }
            if provider_id == "openai-codex"
    ));
    assert_eq!(
        presenter
            .inner
            .notify_failure_reasons
            .lock()
            .unwrap()
            .as_slice(),
        &["login cancelled"]
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn openai_codex_cancellation_is_installed_before_method_selection() {
    let server = MockServer::start().await;
    let provider = codex_device_provider(&server.uri(), Duration::from_secs(60));
    let presenter = MethodPresenter::blocking_method_selection();

    let login = provider.login(&presenter);
    let cancel_selection = async {
        presenter.selection_started.notified().await;
        presenter.cancel_device_login.notify_one();
    };
    let (result, ()) = tokio::time::timeout(Duration::from_millis(500), async {
        tokio::join!(login, cancel_selection)
    })
    .await
    .expect("Codex method selection ignored cancellation");
    let error = result.expect_err("method selection must be cancellable");

    assert!(matches!(
        error,
        AiProviderError::LoginCancelled { ref provider_id }
            if provider_id == "openai-codex"
    ));
    assert_eq!(
        presenter
            .inner
            .notify_failure_reasons
            .lock()
            .unwrap()
            .as_slice(),
        &["login cancelled"]
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn openai_codex_device_budget_includes_method_selection() {
    let server = MockServer::start().await;
    let provider = codex_provider_with_flow_timeouts(
        &server.uri(),
        Duration::from_millis(500),
        Duration::from_millis(50),
    );
    let presenter = MethodPresenter::delayed_method_selection(
        OAuthLoginMethod::DeviceCode,
        Duration::from_millis(75),
    );

    let error = tokio::time::timeout(Duration::from_millis(500), provider.login(&presenter))
        .await
        .expect("Codex method selection escaped its flow budget")
        .expect_err("method selection must consume the device flow budget");

    assert!(matches!(error, AiProviderError::Timeout), "{error:?}");
    assert_eq!(
        presenter
            .inner
            .notify_failure_reasons
            .lock()
            .unwrap()
            .as_slice(),
        &["device authorization timed out"]
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn openai_codex_device_budget_bounds_initial_body_after_headers() {
    let (device_route, device_probe) = raw_json_route(
        "POST",
        "/device/usercode",
        json!({
            "device_auth_id":"device-auth-sentinel",
            "user_code":"PUBLIC-CODE",
            "interval":0
        }),
        Duration::from_millis(250),
    );
    let server = RawBodyDelayServer::start(vec![device_route]).await;
    let provider = codex_device_provider(server.uri(), Duration::from_millis(100));
    let presenter = MethodPresenter::new(Some(OAuthLoginMethod::DeviceCode));

    let error = provider
        .login(&presenter)
        .await
        .expect_err("initial device body decode must use the flow budget");
    device_probe.assert_body_boundary_reached().await;

    assert!(matches!(error, AiProviderError::Timeout), "{error:?}");
    assert_eq!(
        presenter
            .inner
            .notify_failure_reasons
            .lock()
            .unwrap()
            .as_slice(),
        &["device authorization timed out"]
    );
    assert!(
        presenter
            .inner
            .captured_device_codes
            .lock()
            .unwrap()
            .is_empty(),
        "device code was presented after its response-body deadline"
    );
}

#[tokio::test]
async fn openai_codex_device_budget_bounds_final_exchange_body_after_headers() {
    let (device_route, device_probe) = raw_json_route(
        "POST",
        "/device/usercode",
        json!({
            "device_auth_id":"device-auth-sentinel",
            "user_code":"PUBLIC-CODE",
            "interval":0
        }),
        Duration::ZERO,
    );
    let (poll_route, poll_probe) = raw_json_route(
        "POST",
        "/device/token",
        json!({
            "authorization_code":"authorization-code-sentinel",
            "code_verifier":"device-verifier-sentinel"
        }),
        Duration::ZERO,
    );
    let (exchange_route, exchange_probe) = raw_json_route(
        "POST",
        "/oauth/token",
        token_body(
            &codex_jwt(Some("raw-final-account")),
            "raw-final-refresh",
            3600,
        ),
        Duration::from_millis(250),
    );
    let server = RawBodyDelayServer::start(vec![device_route, poll_route, exchange_route]).await;
    let provider = codex_device_provider(server.uri(), Duration::from_millis(100));
    let presenter = MethodPresenter::new(Some(OAuthLoginMethod::DeviceCode));

    let error = provider
        .login(&presenter)
        .await
        .expect_err("final exchange body decode must use the original flow budget");
    device_probe.assert_body_boundary_reached().await;
    poll_probe.assert_body_boundary_reached().await;
    exchange_probe.assert_body_boundary_reached().await;

    assert!(matches!(error, AiProviderError::Timeout), "{error:?}");
    assert_eq!(
        presenter
            .inner
            .notify_failure_reasons
            .lock()
            .unwrap()
            .as_slice(),
        &["device authorization timed out"]
    );
    assert_eq!(
        presenter.inner.notify_success_count.load(Ordering::SeqCst),
        0
    );
}

#[tokio::test]
async fn openai_codex_device_post_code_cancellation_is_ignored() {
    let (device_route, device_probe) = raw_json_route(
        "POST",
        "/device/usercode",
        json!({
            "device_auth_id":"device-auth-sentinel",
            "user_code":"PUBLIC-CODE",
            "interval":0
        }),
        Duration::ZERO,
    );
    let (poll_route, poll_probe) = raw_json_route(
        "POST",
        "/device/token",
        json!({
            "authorization_code":"authorization-code-sentinel",
            "code_verifier":"device-verifier-sentinel"
        }),
        Duration::ZERO,
    );
    let (exchange_route, exchange_probe) = raw_json_route(
        "POST",
        "/oauth/token",
        token_body(
            &codex_jwt(Some("post-code-account")),
            "post-code-refresh",
            3600,
        ),
        Duration::from_millis(250),
    );
    let server = RawBodyDelayServer::start(vec![device_route, poll_route, exchange_route]).await;
    let provider = codex_device_provider(server.uri(), Duration::from_secs(2));
    let presenter = MethodPresenter::new(Some(OAuthLoginMethod::DeviceCode));

    let login = provider.login(&presenter);
    let cancel_after_code = async {
        exchange_probe.wait_for_headers().await;
        presenter.cancel_device_login.notify_one();
    };
    let (result, ()) = tokio::join!(login, cancel_after_code);
    device_probe.assert_body_boundary_reached().await;
    poll_probe.assert_body_boundary_reached().await;
    exchange_probe.assert_body_boundary_reached().await;
    let credential = result.expect("post-code cancellation must not burn the one-use code");

    assert_eq!(credential.account_id.as_deref(), Some("post-code-account"));
    assert_eq!(
        presenter.inner.notify_success_count.load(Ordering::SeqCst),
        1
    );
    assert!(
        presenter
            .inner
            .notify_failure_reasons
            .lock()
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn openai_codex_device_code_success_exchanges_authorization_code() {
    let server = MockServer::start().await;
    mount_codex_device_start(&server, "device-auth-sentinel", "PUBLIC-CODE", 0).await;
    mount_codex_device_poll(
        &server,
        200,
        json!({
            "authorization_code": "authorization-code-sentinel",
            "code_verifier": "device-verifier-sentinel"
        }),
        None,
    )
    .await;
    mount_codex_device_exchange(&server, "device-account").await;
    let provider = codex_device_provider(&server.uri(), Duration::from_secs(60));
    let presenter = MethodPresenter::new(Some(OAuthLoginMethod::DeviceCode));

    let credential = provider.login(&presenter).await.expect("device login");

    assert_eq!(credential.account_id.as_deref(), Some("device-account"));
    assert_eq!(
        presenter
            .inner
            .captured_device_codes
            .lock()
            .unwrap()
            .as_slice(),
        &[(
            "PUBLIC-CODE".to_owned(),
            "https://auth.openai.com/codex/device".to_owned()
        )]
    );
    let requests = server.received_requests().await.expect("requests");
    let token_exchange = requests
        .iter()
        .find(|request| request.url.path() == "/oauth/token")
        .expect("token exchange");
    let body = std::str::from_utf8(&token_exchange.body).expect("form body");
    assert!(body.contains("code=authorization-code-sentinel"), "{body}");
    assert!(
        body.contains("code_verifier=device-verifier-sentinel"),
        "{body}"
    );
    assert!(
        body.contains("redirect_uri=https%3A%2F%2Fauth.openai.com%2Fdeviceauth%2Fcallback"),
        "{body}"
    );
}

#[tokio::test]
async fn openai_codex_device_code_pending_then_success() {
    let server = MockServer::start().await;
    mount_codex_device_start(&server, "device-auth-sentinel", "PUBLIC-CODE", 0).await;
    mount_codex_device_poll(
        &server,
        403,
        json!({"error":"deviceauth_authorization_pending"}),
        Some(1),
    )
    .await;
    mount_codex_device_poll(
        &server,
        400,
        json!({"error":{"code":"deviceauth_authorization_pending"}}),
        Some(1),
    )
    .await;
    mount_codex_device_poll(
        &server,
        200,
        json!({
            "authorization_code": "authorization-code",
            "code_verifier": "device-verifier"
        }),
        None,
    )
    .await;
    mount_codex_device_exchange(&server, "pending-account").await;
    let provider = codex_device_provider(&server.uri(), Duration::from_secs(60));
    let presenter = MethodPresenter::new(Some(OAuthLoginMethod::DeviceCode));

    let credential = provider.login(&presenter).await.expect("device login");

    assert_eq!(credential.account_id.as_deref(), Some("pending-account"));
    let polls = server
        .received_requests()
        .await
        .expect("requests")
        .iter()
        .filter(|request| request.url.path() == "/device/token")
        .count();
    assert_eq!(polls, 3);
}

#[tokio::test]
async fn openai_codex_device_code_slow_down_increases_poll_delay() {
    let server = MockServer::start().await;
    mount_codex_device_start(&server, "device-auth-sentinel", "PUBLIC-CODE", 0).await;
    mount_codex_device_poll(&server, 400, json!({"error":"slow_down"}), Some(1)).await;
    mount_codex_device_poll(
        &server,
        200,
        json!({
            "authorization_code": "authorization-code",
            "code_verifier": "device-verifier"
        }),
        None,
    )
    .await;
    mount_codex_device_exchange(&server, "slow-account").await;
    let provider = codex_device_provider(&server.uri(), Duration::from_secs(60));
    let presenter = MethodPresenter::new(Some(OAuthLoginMethod::DeviceCode));
    let started = tokio::time::Instant::now();

    provider.login(&presenter).await.expect("device login");

    assert!(
        tokio::time::Instant::now().duration_since(started) >= Duration::from_secs(5),
        "slow_down must persistently add five seconds to the poll delay"
    );
}

async fn codex_device_terminal_error(error_code: &str, status: u16) -> AiProviderError {
    let server = MockServer::start().await;
    mount_codex_device_start(&server, "device-auth-sentinel", "PUBLIC-CODE", 0).await;
    mount_codex_device_poll(
        &server,
        status,
        json!({"error":{"code":error_code},"echo":"device-verifier-sentinel"}),
        None,
    )
    .await;
    let provider = codex_device_provider(&server.uri(), Duration::from_secs(60));
    let presenter = MethodPresenter::new(Some(OAuthLoginMethod::DeviceCode));
    provider
        .login(&presenter)
        .await
        .expect_err("terminal error")
}

#[tokio::test]
async fn openai_codex_device_code_denial_is_typed_and_redacted() {
    // The structured `access_denied` code must classify as `Denied` regardless
    // of HTTP status. A terminal code delivered on 403/404 must NOT fall
    // through to the status-based `Pending` branch (which hangs ~15 min until
    // the outer device-flow timeout fires).
    for status in [400u16, 403, 404] {
        let error = codex_device_terminal_error("access_denied", status).await;
        assert!(
            matches!(
                error,
                AiProviderError::CredentialRevoked { ref provider_id }
                    if provider_id == "openai-codex"
            ),
            "status {status}: {error:?}"
        );
        let rendered = format!("{error:?} {error}");
        assert!(
            !rendered.contains("device-auth-sentinel"),
            "status {status}"
        );
        assert!(
            !rendered.contains("device-verifier-sentinel"),
            "status {status}"
        );
    }
}

#[tokio::test]
async fn openai_codex_device_code_expiry_is_typed_and_redacted() {
    // The structured `expired_token` code must classify as `Expired` regardless
    // of HTTP status. A terminal code delivered on 403/404 must NOT fall
    // through to the status-based `Pending` branch (which hangs ~15 min until
    // the outer device-flow timeout fires).
    for status in [400u16, 403, 404] {
        let error = codex_device_terminal_error("expired_token", status).await;
        assert!(
            matches!(
                error,
                AiProviderError::CredentialRevoked { ref provider_id }
                    if provider_id == "openai-codex"
            ),
            "status {status}: {error:?}"
        );
        let rendered = format!("{error:?} {error}");
        assert!(
            !rendered.contains("device-auth-sentinel"),
            "status {status}"
        );
        assert!(
            !rendered.contains("device-verifier-sentinel"),
            "status {status}"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn openai_codex_device_code_timeout_is_15_minutes_under_paused_time() {
    let server = MockServer::start().await;
    mount_codex_device_start(&server, "device-auth-sentinel", "PUBLIC-CODE", 901).await;
    mount_codex_device_poll(
        &server,
        403,
        json!({"error":"deviceauth_authorization_pending"}),
        None,
    )
    .await;
    let provider = codex_device_provider(&server.uri(), Duration::from_secs(15 * 60));
    let presenter = MethodPresenter::new(Some(OAuthLoginMethod::DeviceCode));
    let started = tokio::time::Instant::now();

    let error = provider.login(&presenter).await.expect_err("timeout");

    assert!(matches!(error, AiProviderError::Timeout), "{error:?}");
    assert_eq!(
        tokio::time::Instant::now().duration_since(started),
        Duration::from_secs(15 * 60)
    );
}

#[tokio::test]
async fn openai_codex_device_code_cancellation_writes_nothing() {
    let server = MockServer::start().await;
    mount_codex_device_start(&server, "device-auth-sentinel", "PUBLIC-CODE", 0).await;
    let provider = Arc::new(codex_device_provider(
        &server.uri(),
        Duration::from_secs(15 * 60),
    ));
    let mut registry = OAuthProviderRegistry::new();
    registry.register(provider).unwrap();
    let (_dir, store, _backend) = store_with(FakeKeyringBackend::new());
    let presenter = MethodPresenter::cancelling_active_device_flow();

    let login = login_oauth("openai-codex", &registry, &store, &presenter);
    let cancel_after_presentation = async {
        presenter.device_code_presented.notified().await;
        presenter.cancel_device_login.notify_one();
    };
    let (result, ()) = tokio::join!(login, cancel_after_presentation);
    let error = result.expect_err("active device login cancelled");

    assert!(matches!(
        error,
        AiProviderError::LoginCancelled { ref provider_id }
            if provider_id == "openai-codex"
    ));
    assert!(!error.is_retryable());
    assert_eq!(
        presenter
            .inner
            .captured_device_codes
            .lock()
            .unwrap()
            .as_slice(),
        &[(
            "PUBLIC-CODE".to_owned(),
            "https://auth.openai.com/codex/device".to_owned()
        )]
    );
    assert_eq!(presenter.inner.manual_code_calls.load(Ordering::SeqCst), 0);
    assert!(store.read("openai-codex").await.unwrap().is_none());
    let requests = server.received_requests().await.expect("captured requests");
    assert_eq!(requests.len(), 1, "{requests:?}");
    assert_eq!(requests[0].url.path(), "/device/usercode");
}

#[tokio::test]
async fn openai_codex_device_code_never_calls_await_manual_code() {
    let server = MockServer::start().await;
    mount_codex_device_start(&server, "device-auth-sentinel", "PUBLIC-CODE", 0).await;
    mount_codex_device_poll(
        &server,
        200,
        json!({
            "authorization_code": "authorization-code",
            "code_verifier": "device-verifier"
        }),
        None,
    )
    .await;
    mount_codex_device_exchange(&server, "no-manual-account").await;
    let provider = codex_device_provider(&server.uri(), Duration::from_secs(60));
    let presenter = MethodPresenter::new(Some(OAuthLoginMethod::DeviceCode));

    provider.login(&presenter).await.expect("device login");

    assert_eq!(
        presenter.inner.manual_code_calls.load(Ordering::SeqCst),
        0,
        "device code must not enter the manual-code race"
    );
    assert!(presenter.inner.captured_urls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn anthropic_and_copilot_never_invoke_login_method_selector() {
    let anthropic_server = MockServer::start().await;
    mount_token_stub(
        &anthropic_server,
        200,
        token_body("anthropic-access", "anthropic-refresh", 3600),
    )
    .await;
    let anthropic = anthropic_provider(
        format!("{}/oauth/token", anthropic_server.uri()),
        Duration::from_secs(60),
    );
    let anthropic_presenter = MethodPresenter::new(None);
    anthropic_presenter
        .inner
        .supply_manual_code("anthropic-code");
    anthropic
        .login(&anthropic_presenter)
        .await
        .expect("Anthropic login");
    assert!(anthropic_presenter.selections.lock().unwrap().is_empty());

    let copilot_server = MockServer::start().await;
    mount_device_auth(
        &copilot_server,
        json!({
            "device_code": "copilot-device",
            "user_code": "PUBLIC-CODE",
            "verification_uri": "https://github.com/login/device",
            "interval": 0
        }),
    )
    .await;
    mount_device_poll(
        &copilot_server,
        json!({"access_token": "copilot-github-token"}),
        None,
    )
    .await;
    mount_copilot_token(
        &copilot_server,
        200,
        json!({"token": "copilot-access", "expires_at": copilot_expires_soon()}),
    )
    .await;
    let copilot = copilot_provider(copilot_server.uri(), Duration::from_secs(60));
    let copilot_presenter = MethodPresenter::new(None);
    copilot
        .login(&copilot_presenter)
        .await
        .expect("Copilot login");
    assert!(copilot_presenter.selections.lock().unwrap().is_empty());
}

async fn capture_codex_factory_sse(body: String) -> Vec<String> {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/codex/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(body)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let (_dir, store, _backend) = store_with(FakeKeyringBackend::new());
    store
        .write(
            "openai-codex",
            &stored_oauth_for(
                "openai-codex",
                "synthetic-sse-access",
                "synthetic-sse-refresh",
                Some(server.uri()),
            ),
        )
        .await
        .unwrap();
    let resolver = resolver_with(store);
    let mut config = OpiConfig::default();
    config.defaults.model = "openai-codex:gpt-5.4".into();
    let provider = build_provider_with_oauth(
        &config,
        &resolver,
        &OAuthProviderRegistry::registry_with_builtins(),
    )
    .await
    .expect("dedicated provider");
    let mut stream = provider.stream_prepared(
        factory_request("openai-codex:gpt-5.4"),
        opi_ai::test_support::resolved_auth(),
    );
    let mut captures = Vec::new();
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                captures.push(format!("{event:?}"));
                captures.push(serde_json::to_string(&event).expect("serialize stream event"));
                if event.is_terminal() {
                    break;
                }
            }
            Err(error) => {
                captures.push(format!("{error:?} {error}"));
                captures.push(format!("{:?}", opi_agent::Diagnostic::from(&error)));
                break;
            }
        }
    }
    captures
}

#[tokio::test]
async fn openai_codex_bounded_redaction_scenario() {
    const ACCESS: &str = "sentinel-bounded-access";
    const REFRESH: &str = "sentinel-bounded-refresh";
    const AUTH_CODE: &str = "sentinel-bounded-authorization-code";
    const DEVICE_SECRET: &str = "device-auth-sentinel";
    const DEVICE_VERIFIER: &str = "device-verifier-sentinel";
    const SERIALIZED_ENVELOPE: &str =
        r#"{"version":1,"type":"oauth_token","access":"sentinel-envelope-access"}"#;
    const MALFORMED_SSE: &str = "sentinel-bounded-malformed-sse";
    const SSE_MESSAGE: &str = "sentinel-bounded-sse-message";
    const SSE_EVENT: &str = "sentinel-bounded-sse-event";

    let missing_account_jwt = codex_jwt(None);
    let persisted_jwt = codex_jwt(Some("bounded-account"));
    let mut captures = Vec::new();

    // Browser login: the actual authorization code, refresh token, and
    // synthetic JWT must not survive the typed missing-account failure.
    let browser_server = MockServer::start().await;
    mount_token_stub(
        &browser_server,
        200,
        token_body(&missing_account_jwt, REFRESH, 3600),
    )
    .await;
    let browser = codex_provider(
        format!("{}/oauth/token", browser_server.uri()),
        Duration::from_secs(60),
    );
    let browser_presenter = MockLoginPresenter::new();
    browser_presenter.supply_manual_code(AUTH_CODE);
    let browser_error = browser
        .login(&browser_presenter)
        .await
        .expect_err("missing browser account id");
    captures.push(format!("{browser_error:?} {browser_error}"));
    captures.push(format!("{:?}", opi_agent::Diagnostic::from(&browser_error)));
    captures.extend(
        browser_presenter
            .notify_failure_reasons
            .lock()
            .unwrap()
            .clone(),
    );
    captures.extend(browser_presenter.captured_urls.lock().unwrap().clone());

    // Device Code: terminal server data may echo the device identifier or a
    // verifier, but neither is allowed into errors, diagnostics, or presenter
    // output.
    let device_server = MockServer::start().await;
    mount_codex_device_start(&device_server, DEVICE_SECRET, "PUBLIC-CODE", 0).await;
    mount_codex_device_poll(
        &device_server,
        400,
        json!({
            "error": {"code": "access_denied"},
            "echo": DEVICE_VERIFIER
        }),
        None,
    )
    .await;
    let device = codex_device_provider(&device_server.uri(), Duration::from_secs(60));
    let device_presenter = MethodPresenter::new(Some(OAuthLoginMethod::DeviceCode));
    let device_error = device
        .login(&device_presenter)
        .await
        .expect_err("device denial");
    captures.push(format!("{device_error:?} {device_error}"));
    captures.push(format!("{:?}", opi_agent::Diagnostic::from(&device_error)));
    captures.push(format!(
        "{:?}",
        device_presenter
            .inner
            .captured_device_codes
            .lock()
            .unwrap()
            .clone()
    ));

    // Refresh: old access/refresh material and the returned synthetic JWT are
    // all absent from the strict missing-account failure.
    let refresh_server = MockServer::start().await;
    mount_token_stub(
        &refresh_server,
        200,
        token_body(&missing_account_jwt, REFRESH, 3600),
    )
    .await;
    let refresh = codex_provider(
        format!("{}/oauth/token", refresh_server.uri()),
        Duration::from_secs(60),
    );
    let mut old_credential = oauth_cred(ACCESS, REFRESH, None);
    old_credential.account_id = Some("old-account".into());
    let refresh_error = refresh
        .refresh(&old_credential)
        .await
        .expect_err("missing refreshed account id");
    captures.push(format!("{refresh_error:?} {refresh_error}"));
    captures.push(format!("{:?}", opi_agent::Diagnostic::from(&refresh_error)));

    // Persistence failure: successful browser exchange material never reaches
    // the error or a raw persisted capture.
    let persistence_server = MockServer::start().await;
    mount_token_stub(
        &persistence_server,
        200,
        token_body(&persisted_jwt, REFRESH, 3600),
    )
    .await;
    let mut registry = OAuthProviderRegistry::new();
    registry
        .register(Arc::new(codex_provider(
            format!("{}/oauth/token", persistence_server.uri()),
            Duration::from_secs(60),
        )))
        .unwrap();
    let unavailable_backend = FakeKeyringBackend::new().with_unavailable();
    let (_dir, unavailable_store, backend_capture) = store_with(unavailable_backend);
    let persistence_presenter = MockLoginPresenter::new();
    persistence_presenter.supply_manual_code(AUTH_CODE);
    let persistence_error = login_oauth(
        "openai-codex",
        &registry,
        &unavailable_store,
        &persistence_presenter,
    )
    .await
    .expect_err("persistence failure");
    captures.push(format!("{persistence_error:?} {persistence_error}"));
    captures.push(format!(
        "{:?}",
        opi_agent::Diagnostic::from(&persistence_error)
    ));
    captures.push(format!(
        "{:?}",
        backend_capture.raw_entry(KEYCHAIN_SERVICE, "openai-codex")
    ));

    // Dedicated provider failure: even an upstream body that echoes every
    // bounded sentinel and a serialized envelope remains redacted.
    let provider_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/codex/responses"))
        .respond_with(ResponseTemplate::new(500).set_body_string(format!(
            "{ACCESS} {REFRESH} {AUTH_CODE} {DEVICE_SECRET} {DEVICE_VERIFIER} \
             {missing_account_jwt} {persisted_jwt} {SERIALIZED_ENVELOPE}"
        )))
        .mount(&provider_server)
        .await;
    let (_dir, provider_store, _backend) = store_with(FakeKeyringBackend::new());
    let persisted_credential =
        stored_oauth_for("openai-codex", ACCESS, REFRESH, Some(provider_server.uri()));
    captures.push(format!("{persisted_credential:?}"));
    provider_store
        .write("openai-codex", &persisted_credential)
        .await
        .unwrap();
    let resolver = resolver_with(provider_store);
    let mut config = OpiConfig::default();
    config.defaults.model = "openai-codex:gpt-5.4".into();
    let provider = build_provider_with_oauth(
        &config,
        &resolver,
        &OAuthProviderRegistry::registry_with_builtins(),
    )
    .await
    .expect("dedicated provider");
    let mut stream = provider.stream_prepared(
        factory_request("openai-codex:gpt-5.4"),
        opi_ai::test_support::resolved_auth(),
    );
    let provider_error = loop {
        match stream.next().await {
            Some(Err(error)) => break error,
            Some(Ok(_)) => {}
            None => panic!("provider failure stream ended without error"),
        }
    };
    captures.push(format!("{provider_error:?} {provider_error}"));
    captures.push(format!(
        "{:?}",
        opi_agent::Diagnostic::from(&provider_error)
    ));

    // Dedicated streaming failures: malformed data becomes a fixed typed
    // ProviderError/diagnostic, while valid upstream error messages and
    // unknown event names become fixed AssistantStreamEvent output.
    captures.extend(
        capture_codex_factory_sse(format!(
            "event: response.output_text.delta\ndata: {{{MALFORMED_SSE}\n\n"
        ))
        .await,
    );
    captures.extend(
        capture_codex_factory_sse(format!(
            "event: error\ndata: {{\"message\":\"{SSE_MESSAGE}\"}}\n\n"
        ))
        .await,
    );
    captures.extend(capture_codex_factory_sse(format!("event: {SSE_EVENT}\ndata: {{}}\n\n")).await);

    let rendered = captures.join("\n");
    for sentinel in [
        ACCESS,
        REFRESH,
        AUTH_CODE,
        DEVICE_SECRET,
        DEVICE_VERIFIER,
        missing_account_jwt.as_str(),
        persisted_jwt.as_str(),
        SERIALIZED_ENVELOPE,
        MALFORMED_SSE,
        SSE_MESSAGE,
        SSE_EVENT,
    ] {
        assert!(
            !rendered.contains(sentinel),
            "bounded redaction leaked {sentinel}: {rendered}"
        );
    }
}

fn anthropic_arc(token_url: String) -> Arc<dyn OAuthProvider> {
    Arc::new(anthropic_provider(token_url, Duration::from_secs(60)))
}

#[test]
fn registry_new_is_empty_and_lookup_misses() {
    let registry = OAuthProviderRegistry::new();
    assert!(registry.lookup("anthropic").is_none());
    assert!(registry.ids().is_empty());
}

#[test]
fn registry_register_then_lookup_hits_with_correct_id() {
    let mut registry = OAuthProviderRegistry::new();
    registry
        .register(anthropic_arc("https://token.example".to_owned()))
        .unwrap();
    let oauth = registry.lookup("anthropic").expect("hit");
    assert_eq!(oauth.id(), "anthropic");
    assert_eq!(registry.ids(), vec!["anthropic"]);
}

#[tokio::test]
async fn registry_lookup_returns_owned_arc_usable_independently() {
    let mut registry = OAuthProviderRegistry::new();
    registry
        .register(anthropic_arc("https://token.example".to_owned()))
        .unwrap();
    let oauth = registry.lookup("anthropic").expect("hit");
    // Drop the registry; the owned Arc must remain usable (proves lookup
    // returns an owned Arc<dyn OAuthProvider>, not a &Arc borrow tied to &self).
    drop(registry);
    assert_eq!(oauth.id(), "anthropic");
}

#[test]
fn registry_register_duplicate_id_is_rejected() {
    let mut registry = OAuthProviderRegistry::new();
    registry
        .register(anthropic_arc("https://a".to_owned()))
        .unwrap();
    // A second provider with the same id is rejected; no silent overwrite.
    match registry.register(anthropic_arc("https://b".to_owned())) {
        Err(RegistryError::DuplicateId { id }) => assert_eq!(id, "anthropic"),
        other => panic!("expected DuplicateId, got {other:?}"),
    }
    // The first registration is intact (not overwritten by the rejected one).
    let oauth = registry.lookup("anthropic").expect("original intact");
    assert_eq!(oauth.id(), "anthropic");
    assert_eq!(registry.ids(), vec!["anthropic"]);
}

#[test]
fn registry_holds_heterogeneous_providers_behind_dyn() {
    let mut registry = OAuthProviderRegistry::new();
    registry
        .register(anthropic_arc("https://a".to_owned()))
        .unwrap();
    let codex: Arc<dyn OAuthProvider> = Arc::new(codex_provider(
        "https://b".to_owned(),
        Duration::from_secs(60),
    ));
    registry.register(codex).unwrap();
    assert_eq!(
        registry.lookup("anthropic").map(|p| p.id().to_owned()),
        Some("anthropic".to_owned())
    );
    assert_eq!(
        registry.lookup("openai-codex").map(|p| p.id().to_owned()),
        Some("openai-codex".to_owned())
    );
    assert_eq!(registry.ids(), vec!["anthropic", "openai-codex"]);
}

#[test]
fn registry_debug_does_not_leak_secrets_or_provider_internals() {
    let mut registry = OAuthProviderRegistry::new();
    registry
        .register(anthropic_arc(
            "https://token-DO-NOT-LEAK.example".to_owned(),
        ))
        .unwrap();
    let dbg = format!("{registry:?}");
    // The id list is shown...
    assert!(dbg.contains("anthropic"), "missing id: {dbg}");
    // ...but no provider internals (token URLs, client ids, secrets) recurse.
    assert!(!dbg.contains("DO-NOT-LEAK"), "internal leaked: {dbg}");
    assert!(!dbg.contains("client-id"), "internal leaked: {dbg}");
}

// ===========================================================================
// Slice 5 閳?registry_with_builtins (production OAuth providers)
// ===========================================================================

#[test]
fn registry_with_builtins_registers_anthropic_codex_copilot() {
    let registry = OAuthProviderRegistry::registry_with_builtins();
    assert_eq!(
        registry.ids(),
        vec!["anthropic", "github-copilot", "openai-codex"],
        "registry_with_builtins must register all three OAuth providers"
    );
    for id in ["anthropic", "github-copilot", "openai-codex"] {
        let provider = registry
            .lookup(id)
            .unwrap_or_else(|| panic!("registry_with_builtins missing {id}"));
        assert_eq!(provider.id(), id);
    }
}

#[tokio::test]
async fn resolver_read_oauth_base_url_and_presence_reflect_stored_cred() {
    let backend = FakeKeyringBackend::new();
    let (_dir, store, _backend) = store_with(backend);
    let resolver = resolver_with(store.clone());

    // No cred initially.
    assert!(
        !resolver
            .has_oauth_credential("github-copilot")
            .await
            .unwrap()
    );
    assert_eq!(
        resolver
            .read_oauth_base_url("github-copilot")
            .await
            .unwrap(),
        None
    );

    // Store an OAuth cred with an enterprise base_url.
    let cred = Credential::OAuthToken {
        access: secret("access-copilot"),
        refresh: secret("refresh-copilot"),
        expires_at: Some(fresh_expiry()),
        base_url: Some("https://enterprise.githubcopilot.com".into()),
        account_id: None,
    };
    store.write("github-copilot", &cred).await.unwrap();
    assert!(
        resolver
            .has_oauth_credential("github-copilot")
            .await
            .unwrap()
    );
    assert_eq!(
        resolver
            .read_oauth_base_url("github-copilot")
            .await
            .unwrap(),
        Some("https://enterprise.githubcopilot.com".to_owned()),
    );

    // Anthropic-style cred with no base_url: present, base_url None.
    let anthropic_cred = Credential::OAuthToken {
        access: secret("access-anthropic"),
        refresh: secret("refresh-anthropic"),
        expires_at: Some(fresh_expiry()),
        base_url: None,
        account_id: None,
    };
    store.write("anthropic", &anthropic_cred).await.unwrap();
    assert!(resolver.has_oauth_credential("anthropic").await.unwrap());
    assert_eq!(
        resolver.read_oauth_base_url("anthropic").await.unwrap(),
        None
    );
}

#[tokio::test]
async fn openai_codex_resolver_propagates_account_id_without_secret_logging() {
    let (_dir, store, _backend) = store_with(FakeKeyringBackend::new());
    store
        .write(
            "openai-codex",
            &Credential::OAuthToken {
                access: secret("sentinel-resolver-access"),
                refresh: secret("sentinel-resolver-refresh"),
                expires_at: Some(fresh_expiry()),
                base_url: None,
                account_id: Some("account-resolved".into()),
            },
        )
        .await
        .unwrap();
    let resolver = resolver_with(store);
    let registry = OAuthProviderRegistry::registry_with_builtins();
    let oauth = registry.lookup("openai-codex").unwrap();
    let resolved = resolver
        .resolve_oauth("openai-codex", &*oauth)
        .await
        .expect("resolved Codex credential");
    assert_eq!(resolved.account_id.as_deref(), Some("account-resolved"));
    let rendered = format!("{resolved:?}");
    assert!(!rendered.contains("sentinel-resolver-access"));
    assert!(!rendered.contains("sentinel-resolver-refresh"));
}

// ===========================================================================
// Slice 4 閳?CopilotOAuthProvider (GitHub device-code)
// ===========================================================================

fn copilot_provider(server_uri: String, total_budget: Duration) -> CopilotOAuthProvider {
    CopilotOAuthProvider::new(
        format!("{server_uri}/device/code"),
        format!("{server_uri}/login/oauth/access_token"),
        format!("{server_uri}/copilot_internal/v2/token"),
        "copilot-client-id".to_owned(),
        "read:user".to_owned(),
        total_budget,
    )
}

async fn mount_device_auth(server: &MockServer, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

async fn mount_device_poll(server: &MockServer, body: serde_json::Value, times: Option<u64>) {
    let mut mock = Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body));
    if let Some(n) = times {
        mock = mock.up_to_n_times(n);
    }
    mock.mount(server).await;
}

async fn mount_copilot_token(server: &MockServer, status: u16, body: serde_json::Value) {
    Mock::given(method("GET"))
        .and(path("/copilot_internal/v2/token"))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .mount(server)
        .await;
}

fn copilot_expires_soon() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp() + 3600
}

#[tokio::test]
async fn copilot_login_success_exchanges_github_token_for_copilot_token() {
    let server = MockServer::start().await;
    mount_device_auth(&server, json!({"device_code":"dc","user_code":"ABCD-WXYZ","verification_uri":"https://github.com/login/device","interval":0})).await;
    mount_device_poll(&server, json!({"error":"authorization_pending"}), Some(1)).await;
    mount_device_poll(&server, json!({"access_token":"ghub-123"}), None).await;
    mount_copilot_token(&server, 200, json!({"token":"copilot-tok","expires_at":copilot_expires_soon(),"endpoints":{"api":"https://api.githubcopilot.com"}})).await;
    let provider = copilot_provider(server.uri(), Duration::from_secs(60));
    let presenter = MockLoginPresenter::new();
    let cred = provider.login(&presenter).await.expect("login");
    assert_eq!(cred.access.expose_secret(), "copilot-tok");
    assert_eq!(cred.refresh.expose_secret(), "ghub-123");
    assert_eq!(
        cred.base_url.as_deref(),
        Some("https://api.githubcopilot.com")
    );
    assert!(cred.expires_at.is_some());
    let dc = presenter.captured_device_codes.lock().unwrap().clone();
    assert_eq!(dc[0].0, "ABCD-WXYZ");
    assert_eq!(dc[0].1, "https://github.com/login/device");
}

#[tokio::test]
async fn copilot_login_present_auth_url_never_called() {
    let server = MockServer::start().await;
    mount_device_auth(
        &server,
        json!({"device_code":"dc","user_code":"UC","verification_uri":"https://x","interval":0}),
    )
    .await;
    mount_device_poll(&server, json!({"access_token":"ghub"}), None).await;
    mount_copilot_token(
        &server,
        200,
        json!({"token":"cop","expires_at":copilot_expires_soon()}),
    )
    .await;
    let provider = copilot_provider(server.uri(), Duration::from_secs(60));
    let presenter = MockLoginPresenter::new();
    provider.login(&presenter).await.expect("login");
    assert!(
        presenter.captured_urls.lock().unwrap().is_empty(),
        "Copilot must not call present_auth_url"
    );
}

#[tokio::test]
async fn copilot_login_does_not_call_await_manual_code() {
    let server = MockServer::start().await;
    mount_device_auth(
        &server,
        json!({"device_code":"dc","user_code":"UC","verification_uri":"https://x","interval":0}),
    )
    .await;
    mount_device_poll(&server, json!({"access_token":"ghub"}), None).await;
    mount_copilot_token(
        &server,
        200,
        json!({"token":"cop","expires_at":copilot_expires_soon()}),
    )
    .await;
    let provider = copilot_provider(server.uri(), Duration::from_secs(60));
    let presenter = MockLoginPresenter::new();
    provider.login(&presenter).await.expect("login");
    assert_eq!(
        presenter.manual_code_calls.load(Ordering::SeqCst),
        0,
        "Copilot must not call await_manual_code"
    );
}

#[tokio::test]
async fn copilot_login_authorization_pending_polls_until_success() {
    let server = MockServer::start().await;
    mount_device_auth(
        &server,
        json!({"device_code":"dc","user_code":"UC","verification_uri":"https://x","interval":0}),
    )
    .await;
    mount_device_poll(&server, json!({"error":"authorization_pending"}), Some(2)).await;
    mount_device_poll(&server, json!({"access_token":"ghub"}), None).await;
    mount_copilot_token(
        &server,
        200,
        json!({"token":"cop","expires_at":copilot_expires_soon()}),
    )
    .await;
    let provider = copilot_provider(server.uri(), Duration::from_secs(60));
    let presenter = MockLoginPresenter::new();
    let cred = provider.login(&presenter).await.expect("login");
    assert_eq!(cred.access.expose_secret(), "cop");
    let polls = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r| r.url.as_str().contains("/login/oauth/access_token"))
        .count();
    assert!(polls >= 3, "expected >=3 polls, got {polls}");
}

#[tokio::test]
async fn copilot_login_slow_down_is_non_terminal_and_gated_by_budget() {
    let server = MockServer::start().await;
    mount_device_auth(
        &server,
        json!({"device_code":"dc","user_code":"UC","verification_uri":"https://x","interval":0}),
    )
    .await;
    // slow_down is NOT a terminal error (unlike access_denied/expired_token): the
    // impl increases the interval by exactly 5s (RFC 8628 鎼?.5, persistent) and
    // continues polling. With a total budget shorter than the post-slow_down
    // sleep, the flow times out during that sleep rather than returning a
    // CredentialRevoked denial 閳?proving slow_down was handled as a backoff.
    mount_device_poll(&server, json!({"error":"slow_down"}), None).await;
    let provider = copilot_provider(server.uri(), Duration::from_millis(80));
    let presenter = MockLoginPresenter::new();
    let err = provider
        .login(&presenter)
        .await
        .expect_err("budget exceeded after slow_down");
    assert!(
        matches!(err, AiProviderError::Timeout),
        "slow_down must not be terminal (got {err:?})"
    );
    let polls = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r| r.url.as_str().contains("/login/oauth/access_token"))
        .count();
    assert!(polls >= 1, "expected a poll receiving slow_down");
}

#[tokio::test]
async fn copilot_login_access_denied_returns_typed_error_no_exchange() {
    let server = MockServer::start().await;
    mount_device_auth(
        &server,
        json!({"device_code":"dc","user_code":"UC","verification_uri":"https://x","interval":0}),
    )
    .await;
    mount_device_poll(&server, json!({"error":"access_denied"}), None).await;
    let provider = copilot_provider(server.uri(), Duration::from_secs(60));
    let presenter = MockLoginPresenter::new();
    let err = provider.login(&presenter).await.expect_err("denied");
    match err {
        AiProviderError::CredentialRevoked { provider_id } => {
            assert_eq!(provider_id, "github-copilot")
        }
        other => panic!("expected CredentialRevoked, got {other:?}"),
    }
    // No Copilot token exchange happened.
    assert!(
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .all(|r| !r.url.as_str().contains("/copilot_internal")),
        "Copilot token endpoint should not be hit on access_denied"
    );
}

#[tokio::test]
async fn copilot_login_expired_token_returns_typed_error() {
    let server = MockServer::start().await;
    mount_device_auth(
        &server,
        json!({"device_code":"dc","user_code":"UC","verification_uri":"https://x","interval":0}),
    )
    .await;
    mount_device_poll(&server, json!({"error":"expired_token"}), None).await;
    let provider = copilot_provider(server.uri(), Duration::from_secs(60));
    let presenter = MockLoginPresenter::new();
    let err = provider.login(&presenter).await.expect_err("expired");
    match err {
        AiProviderError::CredentialRevoked { provider_id } => {
            assert_eq!(provider_id, "github-copilot")
        }
        other => panic!("expected CredentialRevoked, got {other:?}"),
    }
}

#[tokio::test]
async fn copilot_login_total_budget_timeout_returns_timeout() {
    let server = MockServer::start().await;
    mount_device_auth(
        &server,
        json!({"device_code":"dc","user_code":"UC","verification_uri":"https://x","interval":10}),
    )
    .await;
    mount_device_poll(&server, json!({"error":"authorization_pending"}), None).await;
    let provider = copilot_provider(server.uri(), Duration::from_millis(100));
    let presenter = MockLoginPresenter::new();
    let err = provider.login(&presenter).await.expect_err("timeout");
    assert!(matches!(err, AiProviderError::Timeout), "got {err:?}");
    let reasons = presenter.notify_failure_reasons.lock().unwrap().clone();
    assert!(
        reasons.iter().any(|r| r.contains("timed out")),
        "reasons: {reasons:?}"
    );
}

#[tokio::test]
async fn copilot_total_budget_bounds_initial_device_authorization_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/device/code"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "device_code":"dc",
                    "user_code":"UC",
                    "verification_uri":"https://x",
                    "interval":0
                }))
                .set_delay(Duration::from_millis(100)),
        )
        .mount(&server)
        .await;
    mount_device_poll(&server, json!({"access_token":"ghub"}), None).await;
    mount_copilot_token(
        &server,
        200,
        json!({"token":"cop","expires_at":copilot_expires_soon()}),
    )
    .await;
    let provider = copilot_provider(server.uri(), Duration::from_millis(50));
    let presenter = MockLoginPresenter::new();

    let error = provider
        .login(&presenter)
        .await
        .expect_err("initial authorization body must consume the flow budget");

    assert!(matches!(error, AiProviderError::Timeout), "{error:?}");
    assert_eq!(
        presenter.notify_failure_reasons.lock().unwrap().as_slice(),
        &["device authorization timed out"]
    );
    assert!(presenter.captured_device_codes.lock().unwrap().is_empty());
}

#[tokio::test]
async fn copilot_total_budget_bounds_initial_authorization_body_after_headers() {
    let (device_route, device_probe) = raw_json_route(
        "POST",
        "/device/code",
        json!({
            "device_code":"dc",
            "user_code":"UC",
            "verification_uri":"https://x",
            "interval":0
        }),
        Duration::from_millis(250),
    );
    let server = RawBodyDelayServer::start(vec![device_route]).await;
    let provider = copilot_provider(server.uri().to_owned(), Duration::from_millis(100));
    let presenter = MockLoginPresenter::new();

    let error = provider
        .login(&presenter)
        .await
        .expect_err("initial authorization body decode must use the flow budget");
    device_probe.assert_body_boundary_reached().await;

    assert!(matches!(error, AiProviderError::Timeout), "{error:?}");
    assert_eq!(
        presenter.notify_failure_reasons.lock().unwrap().as_slice(),
        &["device authorization timed out"]
    );
    assert!(
        presenter.captured_device_codes.lock().unwrap().is_empty(),
        "device code was presented after its response-body deadline"
    );
}

#[tokio::test]
async fn copilot_cancellation_is_installed_before_initial_response_body() {
    let (device_route, device_probe) = raw_json_route(
        "POST",
        "/device/code",
        json!({
            "device_code":"dc",
            "user_code":"UC",
            "verification_uri":"https://x",
            "interval":0
        }),
        Duration::from_millis(750),
    );
    let server = RawBodyDelayServer::start(vec![device_route]).await;
    let provider = copilot_provider(server.uri().to_owned(), Duration::from_secs(2));
    let presenter = MockLoginPresenter::new();
    let cancel = presenter.login_cancelled_sender();

    let login = provider.login(&presenter);
    let cancel_during_body = async {
        device_probe.wait_for_headers().await;
        cancel.send(()).unwrap();
    };
    let (result, ()) = tokio::time::timeout(Duration::from_millis(300), async {
        tokio::join!(login, cancel_during_body)
    })
    .await
    .expect("Copilot waited for the initial body before observing cancellation");
    device_probe.assert_body_boundary_reached().await;
    let error = result.expect_err("initial device authorization must be cancellable");

    assert!(matches!(
        error,
        AiProviderError::LoginCancelled { ref provider_id }
            if provider_id == "github-copilot"
    ));
    assert_eq!(
        presenter.notify_failure_reasons.lock().unwrap().as_slice(),
        &["login cancelled"]
    );
    assert!(presenter.captured_device_codes.lock().unwrap().is_empty());
}

#[tokio::test]
async fn copilot_ready_cancellation_prevents_initial_request() {
    let server = MockServer::start().await;
    let provider = copilot_provider(server.uri(), Duration::from_secs(2));
    let presenter = MockLoginPresenter::new();
    presenter.login_cancelled_sender().send(()).unwrap();

    let error = provider
        .login(&presenter)
        .await
        .expect_err("ready cancellation must win before the initial request");

    assert!(matches!(
        error,
        AiProviderError::LoginCancelled { ref provider_id }
            if provider_id == "github-copilot"
    ));
    assert_eq!(
        presenter.notify_failure_reasons.lock().unwrap().as_slice(),
        &["login cancelled"]
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn copilot_total_budget_bounds_final_exchange_after_poll_response_delay() {
    let server = MockServer::start().await;
    mount_device_auth(
        &server,
        json!({
            "device_code":"dc",
            "user_code":"UC",
            "verification_uri":"https://x",
            "interval":0
        }),
    )
    .await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"access_token":"ghub"}))
                .set_delay(Duration::from_millis(60)),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/copilot_internal/v2/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "token":"cop",
                    "expires_at":copilot_expires_soon()
                }))
                .set_delay(Duration::from_millis(80)),
        )
        .mount(&server)
        .await;
    let provider = copilot_provider(server.uri(), Duration::from_millis(100));
    let presenter = MockLoginPresenter::new();

    let error = provider
        .login(&presenter)
        .await
        .expect_err("final exchange must use the poll flow's remaining budget");

    assert!(matches!(error, AiProviderError::Timeout), "{error:?}");
    assert_eq!(
        presenter.notify_failure_reasons.lock().unwrap().as_slice(),
        &["device authorization timed out"]
    );
    assert_eq!(presenter.notify_success_count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn copilot_total_budget_bounds_final_exchange_body_after_headers() {
    let (device_route, device_probe) = raw_json_route(
        "POST",
        "/device/code",
        json!({
            "device_code":"dc",
            "user_code":"UC",
            "verification_uri":"https://x",
            "interval":0
        }),
        Duration::ZERO,
    );
    let (poll_route, poll_probe) = raw_json_route(
        "POST",
        "/login/oauth/access_token",
        json!({"access_token":"ghub"}),
        Duration::ZERO,
    );
    let (exchange_route, exchange_probe) = raw_json_route(
        "GET",
        "/copilot_internal/v2/token",
        json!({
            "token":"cop",
            "expires_at":copilot_expires_soon()
        }),
        Duration::from_millis(250),
    );
    let server = RawBodyDelayServer::start(vec![device_route, poll_route, exchange_route]).await;
    let provider = copilot_provider(server.uri().to_owned(), Duration::from_millis(100));
    let presenter = MockLoginPresenter::new();

    let error = provider
        .login(&presenter)
        .await
        .expect_err("final exchange body decode must use the remaining flow budget");
    device_probe.assert_body_boundary_reached().await;
    poll_probe.assert_body_boundary_reached().await;
    exchange_probe.assert_body_boundary_reached().await;

    assert!(matches!(error, AiProviderError::Timeout), "{error:?}");
    assert_eq!(
        presenter.notify_failure_reasons.lock().unwrap().as_slice(),
        &["device authorization timed out"]
    );
    assert_eq!(presenter.notify_success_count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn copilot_cancellation_prevents_final_exchange_and_persistence() {
    let server = MockServer::start().await;
    mount_device_auth(
        &server,
        json!({
            "device_code":"dc",
            "user_code":"UC",
            "verification_uri":"https://x",
            "interval":60
        }),
    )
    .await;
    mount_device_poll(&server, json!({"error":"authorization_pending"}), None).await;
    mount_copilot_token(
        &server,
        200,
        json!({"token":"unused","expires_at":copilot_expires_soon()}),
    )
    .await;
    let provider: Arc<dyn OAuthProvider> =
        Arc::new(copilot_provider(server.uri(), Duration::from_millis(250)));
    let mut registry = OAuthProviderRegistry::new();
    registry.register(provider).unwrap();
    let (_dir, store, _backend) = store_with(FakeKeyringBackend::new());
    let presenter = MockLoginPresenter::new();
    let cancel = presenter.login_cancelled_sender();

    let login = login_oauth("github-copilot", &registry, &store, &presenter);
    let cancel_after_presentation = async {
        presenter.wait_for_device_code().await;
        cancel.send(()).unwrap();
    };
    let (result, ()) = tokio::join!(login, cancel_after_presentation);
    let error = result.expect_err("active Copilot login must be cancellable");

    assert!(matches!(
        error,
        AiProviderError::LoginCancelled { ref provider_id }
            if provider_id == "github-copilot"
    ));
    assert_eq!(
        presenter.notify_failure_reasons.lock().unwrap().as_slice(),
        &["login cancelled"]
    );
    assert_eq!(presenter.notify_success_count.load(Ordering::SeqCst), 0);
    assert!(store.read("github-copilot").await.unwrap().is_none());
    assert!(
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .all(|request| request.url.path() != "/copilot_internal/v2/token"),
        "Copilot final exchange ran after cancellation"
    );
}

#[tokio::test]
async fn copilot_post_token_cancellation_is_ignored_during_bounded_final_exchange() {
    let server = MockServer::start().await;
    mount_device_auth(
        &server,
        json!({
            "device_code":"dc",
            "user_code":"UC",
            "verification_uri":"https://x",
            "interval":0
        }),
    )
    .await;
    mount_device_poll(&server, json!({"access_token":"ghub"}), None).await;
    let final_exchange_started = Arc::new(tokio::sync::Notify::new());
    let notify = final_exchange_started.clone();
    Mock::given(method("GET"))
        .and(path("/copilot_internal/v2/token"))
        .respond_with(move |_: &wiremock::Request| {
            notify.notify_one();
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "token":"cop",
                    "expires_at":copilot_expires_soon()
                }))
                .set_delay(Duration::from_millis(80))
        })
        .mount(&server)
        .await;
    let provider = copilot_provider(server.uri(), Duration::from_millis(500));
    let presenter = MockLoginPresenter::new();
    let cancel = presenter.login_cancelled_sender();

    let login = provider.login(&presenter);
    let cancel_after_token = async {
        final_exchange_started.notified().await;
        cancel.send(()).unwrap();
    };
    let (result, ()) = tokio::join!(login, cancel_after_token);

    result.expect("post-token cancellation must not burn the acquired token");
    assert_eq!(presenter.notify_success_count.load(Ordering::SeqCst), 1);
    assert!(presenter.notify_failure_reasons.lock().unwrap().is_empty());
}

#[tokio::test]
async fn copilot_login_device_code_never_leaks_to_presenter_or_errors() {
    let server = MockServer::start().await;
    mount_device_auth(&server, json!({"device_code":"DEVICE-DO-NOT-LEAK","user_code":"USERCODE","verification_uri":"https://x","interval":0})).await;
    mount_device_poll(&server, json!({"error":"access_denied"}), None).await;
    let provider = copilot_provider(server.uri(), Duration::from_secs(60));
    let presenter = MockLoginPresenter::new();
    let err = provider.login(&presenter).await.expect_err("denied");
    assert!(
        !err.to_string().contains("DEVICE-DO-NOT-LEAK"),
        "device_code in error: {err}"
    );
    match &err {
        AiProviderError::CredentialRevoked { .. } => (),
        other => panic!("expected CredentialRevoked, got {other:?}"),
    }
    let reasons = presenter.notify_failure_reasons.lock().unwrap().clone();
    for r in &reasons {
        assert!(
            !r.contains("DEVICE-DO-NOT-LEAK"),
            "device_code in reason: {r}"
        );
    }
    let dc = presenter.captured_device_codes.lock().unwrap().clone();
    for (uc, _uri) in &dc {
        assert_eq!(uc, "USERCODE", "must show user_code, not device_code");
    }
    let urls = presenter.captured_urls.lock().unwrap().clone();
    for u in &urls {
        assert!(!u.contains("DEVICE-DO-NOT-LEAK"), "device_code in url: {u}");
    }
}

#[tokio::test]
async fn copilot_poll_unknown_error_code_is_closed_and_redacted() {
    const DEVICE_CODE: &str = "device-code-server-error-canary";

    let server = MockServer::start().await;
    mount_device_auth(
        &server,
        json!({
            "device_code": DEVICE_CODE,
            "user_code": "PUBLIC-CODE",
            "verification_uri": "https://x",
            "interval": 0
        }),
    )
    .await;
    mount_device_poll(
        &server,
        json!({
            "error": format!("authorization_pending:{DEVICE_CODE}"),
            "error_description": DEVICE_CODE
        }),
        None,
    )
    .await;
    let provider = copilot_provider(server.uri(), Duration::from_secs(60));
    let presenter = MockLoginPresenter::new();

    let error = provider.login(&presenter).await.expect_err("poll error");

    match &error {
        AiProviderError::Config(message) => {
            assert_eq!(message, "device authorization error: unknown_oauth_error");
        }
        other => panic!("expected Config, got {other:?}"),
    }
    assert_oauth_error_surfaces_are_redacted(&error, Some(&presenter), &[DEVICE_CODE]);
}

#[tokio::test]
async fn copilot_exchange_unknown_error_code_is_closed_and_redacted() {
    const GITHUB_TOKEN: &str = "github-token-server-error-canary";

    let server = MockServer::start().await;
    mount_device_auth(
        &server,
        json!({
            "device_code": "device-code",
            "user_code": "PUBLIC-CODE",
            "verification_uri": "https://x",
            "interval": 0
        }),
    )
    .await;
    mount_device_poll(&server, json!({"access_token": GITHUB_TOKEN}), None).await;
    mount_copilot_token(
        &server,
        400,
        json!({
            "error": format!("invalid_token:{GITHUB_TOKEN}"),
            "error_description": GITHUB_TOKEN
        }),
    )
    .await;
    let provider = copilot_provider(server.uri(), Duration::from_secs(60));
    let presenter = MockLoginPresenter::new();

    let error = provider
        .login(&presenter)
        .await
        .expect_err("exchange error");

    match &error {
        AiProviderError::AuthFailed(message) => assert_eq!(
            message,
            "token endpoint: 400 Bad Request unknown_oauth_error"
        ),
        other => panic!("expected AuthFailed, got {other:?}"),
    }
    assert_oauth_error_surfaces_are_redacted(&error, Some(&presenter), &[GITHUB_TOKEN]);
}

#[tokio::test]
async fn copilot_refresh_re_exchanges_github_token_preserves_base_url() {
    let server = MockServer::start().await;
    // Response omits endpoints -> base_url must be preserved from the cred.
    mount_copilot_token(
        &server,
        200,
        json!({"token":"copilot-new","expires_at":copilot_expires_soon()}),
    )
    .await;
    let provider = copilot_provider(server.uri(), Duration::from_secs(60));
    let cred = oauth_cred(
        "copilot-old",
        "ghub-old",
        Some("https://enterprise".to_owned()),
    );
    let refreshed = provider.refresh(&cred).await.expect("refresh");
    assert_eq!(refreshed.access.expose_secret(), "copilot-new");
    assert_eq!(refreshed.refresh.expose_secret(), "ghub-old");
    assert_eq!(refreshed.base_url.as_deref(), Some("https://enterprise"));
    // The GitHub token (stored as refresh) is sent as the Bearer to re-exchange.
    let reqs = server.received_requests().await.unwrap();
    let auth = reqs[0]
        .headers
        .get("authorization")
        .expect("auth header")
        .to_str()
        .unwrap();
    assert_eq!(auth, "Bearer ghub-old");
}

#[tokio::test]
async fn copilot_refresh_401_returns_credential_revoked() {
    let server = MockServer::start().await;
    mount_copilot_token(&server, 401, json!({"error":"invalid_token"})).await;
    let provider = copilot_provider(server.uri(), Duration::from_secs(60));
    let cred = oauth_cred("copilot-old", "ghub-LEAK-CANARY", None);
    let err = provider.refresh(&cred).await.expect_err("401");
    match &err {
        AiProviderError::CredentialRevoked { provider_id } => {
            assert_eq!(provider_id, "github-copilot")
        }
        other => panic!("expected CredentialRevoked, got {other:?}"),
    }
    assert!(
        !err.to_string().contains("ghub-LEAK-CANARY"),
        "GitHub token leaked: {err}"
    );
}

// ===========================================================================
// Slice 4 閳?TuiLoginPresenter (production, print-only substrate)
// ===========================================================================

#[tokio::test]
async fn tui_login_presenter_print_only_methods_succeed_without_panic() {
    // Slice 4 ships a print-only presenter; the TUI modal + interactive
    // await_manual_code are wired by slice 6. This smoke test exercises the
    // print/sync paths (present_auth_url returns Ok; notify_* do not panic).
    // await_manual_code is NOT exercised here (it blocks on stdin).
    let presenter = TuiLoginPresenter::new();
    presenter
        .present_auth_url("https://authorize.example/oauth/authorize?state=abc")
        .await
        .expect("present_auth_url");
    presenter
        .present_device_code("USER-CODE", "https://github.com/login/device")
        .await
        .expect("present_device_code");
    presenter.notify_success();
    presenter.notify_failure("state mismatch");
}

// ===========================================================================
// Slice 5 閳?factory OAuth wiring (product-loop via build_provider_with_oauth)
// ===========================================================================

fn factory_request(model: &str) -> Request {
    Request {
        model: model.into(),
        system: None,
        messages: vec![],
        tools: vec![],
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

/// Drain a provider event stream to completion without hanging.
async fn drain_stream(stream: &mut opi_ai::provider::EventStream) {
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) if event.is_terminal() => break,
            Err(_) => break,
            _ => {}
        }
    }
}

/// Phase 17.5: build a dispatchable [`ProviderCollection`] for the active
/// provider, mirroring `build_provider_with_oauth`'s routing.
///
/// Authentication moved off the provider object onto the collection route, so
/// the store-backed/layered resolver that `build_provider_with_oauth` used to
/// install inside the provider is now registered here via `register_route`.
/// `prepare_call` then resolves the live credential on each logical call.
fn dispatch_collection(
    provider: Box<dyn Provider>,
    provider_id: &str,
    config: &OpiConfig,
    resolver: CredentialResolver,
    registry: &OAuthProviderRegistry,
) -> ProviderCollection {
    let auth_resolver = oauth_auth_resolver_for(provider_id, config, resolver, registry);
    let mut collection = ProviderCollection::new();
    collection
        .register_route(
            provider,
            auth_resolver,
            AuthProvenanceSource::OAuth {
                kind: provider_id.into(),
            },
            CompatMetadata::default(),
        )
        .expect("register dispatch route");
    collection
}

/// Phase 17.5: build the per-route auth resolver matching
/// `build_provider_with_oauth`'s routing, for handoff to a `CodingHarness`
/// builder via `.auth_resolver(...)`. Anthropic uses the layered precedence
/// (stored OAuth > `ANTHROPIC_OAUTH_TOKEN` > API key); Copilot/Codex use the
/// store-backed OAuth resolver.
fn oauth_auth_resolver_for(
    provider_id: &str,
    config: &OpiConfig,
    resolver: CredentialResolver,
    registry: &OAuthProviderRegistry,
) -> Arc<dyn AuthResolver> {
    match provider_id {
        "anthropic" => Arc::new(AuthSource::Layered {
            resolver: Arc::new(resolver),
            provider_id: "anthropic".into(),
            oauth: registry
                .lookup("anthropic")
                .expect("anthropic OAuth provider registered"),
            oauth_env_var: "ANTHROPIC_OAUTH_TOKEN".into(),
            api_key_env_var: config.providers.anthropic.api_key_env.clone(),
        }),
        "github-copilot" | "openai-codex" => Arc::new(AuthSource::Store {
            resolver: Arc::new(resolver),
            provider_id: provider_id.into(),
            oauth: registry
                .lookup(provider_id)
                .expect("OAuth provider registered for {provider_id}"),
        }),
        other => panic!("oauth_auth_resolver_for: unsupported OAuth provider {other}"),
    }
}

/// Phase 17.5: drain a dispatchable collection route to completion (or first
/// stream error), mirroring a harness turn through `prepare_call` + `start_attempt`.
async fn drain_route(collection: &ProviderCollection, spec: &str) {
    let prepared = collection
        .prepare_call(spec, factory_request(spec))
        .await
        .expect("prepare_call resolves the dispatch route");
    let mut stream = prepared.start_attempt().expect("start_attempt");
    drain_stream(&mut stream).await;
}

/// A fresh (non-expiring) stored OAuth credential. `base_url` redirects dispatch
/// to a mock when `Some`.
fn stored_oauth(access: &str, refresh: &str, base_url: Option<String>) -> Credential {
    Credential::OAuthToken {
        access: secret(access),
        refresh: secret(refresh),
        expires_at: Some(fresh_expiry()),
        base_url,
        account_id: None,
    }
}

fn stored_oauth_for(
    provider_id: &str,
    access: &str,
    refresh: &str,
    base_url: Option<String>,
) -> Credential {
    let mut credential = stored_oauth(access, refresh, base_url);
    if provider_id == "openai-codex" {
        let Credential::OAuthToken { account_id, .. } = &mut credential else {
            unreachable!("stored_oauth always returns OAuthToken");
        };
        *account_id = Some("account-test".into());
    }
    credential
}

#[tokio::test]
async fn factory_routes_github_copilot_models_by_declared_wire() {
    let server = MockServer::start().await;
    for request_path in ["/v1/messages", "/chat/completions", "/responses"] {
        Mock::given(method("POST"))
            .and(path(request_path))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("")
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;
    }

    let backend = FakeKeyringBackend::new();
    let (_dir, store, _b) = store_with(backend);
    store
        .write(
            "github-copilot",
            &stored_oauth(
                "copilot-access-fake",
                "copilot-refresh-fake",
                Some(server.uri()),
            ),
        )
        .await
        .unwrap();
    let resolver = resolver_with(store);
    let registry = OAuthProviderRegistry::registry_with_builtins();

    let mut config = OpiConfig::default();
    config.defaults.model = "github-copilot:gpt-4.1".into();

    let provider = build_provider_with_oauth(&config, &resolver, &registry)
        .await
        .expect("copilot OAuth provider builds");
    assert_eq!(provider.id(), "github-copilot");
    // Phase 17.5: auth moved off the provider object onto the collection route.
    // Register the store-backed Copilot resolver so prepare_call resolves the
    // stored OAuth token on each logical call across all three wires.
    let collection = dispatch_collection(provider, "github-copilot", &config, resolver, &registry);
    for model in ["claude-sonnet-4.5", "gpt-4.1", "gpt-5.4"] {
        drain_route(&collection, &format!("github-copilot:{model}")).await;
    }

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 3, "one request per declared wire");
    assert_eq!(
        requests
            .iter()
            .map(|request| request.url.path())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["/v1/messages", "/chat/completions", "/responses",])
    );
    for request in requests {
        assert_eq!(
            request
                .headers
                .get("authorization")
                .map(|value| value.to_str().unwrap()),
            Some("Bearer copilot-access-fake")
        );
        assert!(request.headers.get("x-api-key").is_none());
    }
}

#[tokio::test]
async fn factory_builds_codex_without_promoting_credential_base_url_into_its_catalog() {
    let backend = FakeKeyringBackend::new();
    let (_dir, store, _b) = store_with(backend);
    store
        .write(
            "openai-codex",
            &stored_oauth_for(
                "openai-codex",
                "codex-access-fake",
                "codex-refresh-fake",
                Some("https://stale-credential-host.invalid".into()),
            ),
        )
        .await
        .unwrap();
    let resolver = resolver_with(store);
    let registry = OAuthProviderRegistry::registry_with_builtins();

    let mut config = OpiConfig::default();
    config.defaults.model = "openai-codex:gpt-5.4".into();

    let provider = build_provider_with_oauth(&config, &resolver, &registry)
        .await
        .expect("codex OAuth provider builds");
    assert_eq!(provider.id(), "openai-codex");
    assert!(provider.models().iter().all(|model| {
        model.base_url.as_deref() != Some("https://stale-credential-host.invalid")
    }));
}

#[tokio::test]
async fn factory_routes_anthropic_to_oauth_when_cred_stored() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let backend = FakeKeyringBackend::new();
    let (_dir, store, _b) = store_with(backend);
    // Anthropic OAuth cred (PKCE: no base_url). Config base_url redirects.
    store
        .write(
            "anthropic",
            &stored_oauth("anthropic-oauth-fake", "anthropic-refresh-fake", None),
        )
        .await
        .unwrap();
    let resolver = resolver_with(store);
    let registry = OAuthProviderRegistry::registry_with_builtins();

    let mut config = OpiConfig::default();
    config.defaults.model = "anthropic:claude-sonnet-4-5-20250514".into();
    config.providers.anthropic.base_url = Some(server.uri());

    let provider = build_provider_with_oauth(&config, &resolver, &registry)
        .await
        .expect("anthropic OAuth provider builds");
    assert_eq!(provider.id(), "anthropic");
    // Phase 17.5: auth moved off the provider object onto the collection route.
    // Register the layered Anthropic resolver so prepare_call resolves the stored
    // OAuth credential and emits Bearer + the beta header (no x-api-key).
    let collection = dispatch_collection(provider, "anthropic", &config, resolver, &registry);
    drain_route(&collection, "anthropic:claude-sonnet-4-5-20250514").await;

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "exactly one Anthropic Messages request");
    let req = &requests[0];
    // OAuth path: Bearer + the beta header, NO x-api-key.
    assert_eq!(
        req.headers
            .get("authorization")
            .map(|v| v.to_str().unwrap()),
        Some("Bearer anthropic-oauth-fake")
    );
    assert_eq!(
        req.headers
            .get("anthropic-beta")
            .map(|v| v.to_str().unwrap()),
        Some("claude-code-20250219,oauth-2025-04-20")
    );
    assert!(
        req.headers.get("x-api-key").is_none(),
        "Anthropic OAuth path must not send x-api-key"
    );
}

#[tokio::test]
async fn concurrent_near_expiry_resolves_coalesce_to_single_refresh() {
    let backend = FakeKeyringBackend::new();
    let (_dir, store, _b) = store_with(backend);
    // A near-expiry stored Anthropic OAuth cred.
    store
        .write(
            PROVIDER,
            &oauth_credential("expiring-access", Some(near_expiry())),
        )
        .await
        .unwrap();
    let resolver = Arc::new(resolver_with(store));
    let mock = Arc::new(MockOAuthProvider::succeeding());

    let r1 = resolver.clone();
    let m1 = mock.clone();
    let h1 = tokio::spawn(async move { r1.resolve_oauth(PROVIDER, &*m1).await });

    let r2 = resolver.clone();
    let m2 = mock.clone();
    let h2 = tokio::spawn(async move { r2.resolve_oauth(PROVIDER, &*m2).await });

    let (a, b) = tokio::join!(h1, h2);
    let a = a.unwrap().unwrap();
    let b = b.unwrap().unwrap();

    // Exactly one refresh: the lock serialized the two resolves; the second
    // re-read the fresh credential written by the first and skipped refresh.
    assert_eq!(
        mock.refresh_calls.load(Ordering::SeqCst),
        1,
        "concurrent near-expiry resolves must coalesce to a single refresh"
    );
    assert_eq!(a.scheme, AuthScheme::Bearer);
    assert_eq!(b.scheme, AuthScheme::Bearer);
}

#[tokio::test]
async fn anthropic_oauth_env_skips_unreadable_lower_priority_api_key() {
    struct UnreadableApiKeyBackend {
        protected_get_calls: Arc<AtomicUsize>,
    }

    impl opi_coding_agent::credential_store::KeyringBackend for UnreadableApiKeyBackend {
        fn get(
            &self,
            service: &str,
            _provider_id: &str,
        ) -> Result<Option<String>, opi_coding_agent::credential_store::BackendError> {
            if service == opi_coding_agent::credential_store::KEYCHAIN_PRESENCE_SERVICE {
                Ok(Some("api_key".to_owned()))
            } else {
                self.protected_get_calls.fetch_add(1, Ordering::SeqCst);
                Err(opi_coding_agent::credential_store::BackendError::Other(
                    "protected API-key entry is unreadable".to_owned(),
                ))
            }
        }

        fn set(
            &self,
            _service: &str,
            _provider_id: &str,
            _value: &str,
        ) -> Result<(), opi_coding_agent::credential_store::BackendError> {
            Err(opi_coding_agent::credential_store::BackendError::Other(
                "unused set".to_owned(),
            ))
        }

        fn delete(
            &self,
            _service: &str,
            _provider_id: &str,
        ) -> Result<(), opi_coding_agent::credential_store::BackendError> {
            Err(opi_coding_agent::credential_store::BackendError::Other(
                "unused delete".to_owned(),
            ))
        }
    }

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let protected_get_calls = Arc::new(AtomicUsize::new(0));
    let dir = TempDir::new().unwrap();
    let store = Arc::new(KeychainCredentialStore::new(
        Box::new(UnreadableApiKeyBackend {
            protected_get_calls: Arc::clone(&protected_get_calls),
        }),
        dir.path().to_path_buf(),
    ));
    let resolver = CredentialResolver::new(
        store,
        Arc::new(|name: &str| {
            (name == "ANTHROPIC_OAUTH_TOKEN").then(|| "higher-priority-oauth-token".to_owned())
        }),
    );
    let registry = OAuthProviderRegistry::registry_with_builtins();
    let mut config = OpiConfig::default();
    config.defaults.model = "anthropic:claude-sonnet-4-5-20250514".into();
    config.providers.anthropic.base_url = Some(server.uri());

    let provider = build_provider_with_oauth(&config, &resolver, &registry)
        .await
        .expect("OAuth env must bypass unreadable lower-priority API-key entry");
    assert_eq!(
        protected_get_calls.load(Ordering::SeqCst),
        0,
        "construction must not read the lower-priority API-key entry"
    );

    // Phase 17.5: auth resolution moved to prepare_call on the collection route.
    let collection = dispatch_collection(provider, "anthropic", &config, resolver, &registry);
    drain_route(&collection, "anthropic:claude-sonnet-4-5-20250514").await;
    assert_eq!(
        protected_get_calls.load(Ordering::SeqCst),
        0,
        "stream auth selection must not read the lower-priority API-key entry"
    );
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests[0]
            .headers
            .get("authorization")
            .and_then(|value| value.to_str().ok()),
        Some("Bearer higher-priority-oauth-token")
    );
    assert!(requests[0].headers.get("x-api-key").is_none());
}

#[tokio::test]
async fn anthropic_env_oauth_token_precedence_stored_wins_env_fallback() {
    // 1. ANTHROPIC_OAUTH_TOKEN present, no stored cred -> EnvOAuthToken path.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let backend = FakeKeyringBackend::new();
    let (_dir, store, _b) = store_with(backend);
    let resolver = CredentialResolver::new(
        store,
        Arc::new(|name: &str| {
            if name == "ANTHROPIC_OAUTH_TOKEN" {
                Some("env-oauth-token".into())
            } else {
                None
            }
        }),
    );
    let registry = OAuthProviderRegistry::registry_with_builtins();
    let mut config = OpiConfig::default();
    config.defaults.model = "anthropic:claude-sonnet-4-5-20250514".into();
    config.providers.anthropic.base_url = Some(server.uri());

    let provider = build_provider_with_oauth(&config, &resolver, &registry)
        .await
        .expect("env-oauth provider builds");
    // Phase 17.5: auth resolution moved to prepare_call on the collection route.
    let collection = dispatch_collection(provider, "anthropic", &config, resolver, &registry);
    drain_route(&collection, "anthropic:claude-sonnet-4-5-20250514").await;

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let req = &requests[0];
    // EnvOAuthToken emits Bearer (with the beta header), no x-api-key.
    assert_eq!(
        req.headers
            .get("authorization")
            .map(|v| v.to_str().unwrap()),
        Some("Bearer env-oauth-token")
    );
    assert_eq!(
        req.headers
            .get("anthropic-beta")
            .map(|v| v.to_str().unwrap()),
        Some("claude-code-20250219,oauth-2025-04-20")
    );
    assert!(req.headers.get("x-api-key").is_none());

    // 2. Stored cred present + env token also set -> stored wins (Store path).
    let server2 = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server2)
        .await;

    let backend2 = FakeKeyringBackend::new();
    let (_dir2, store2, _b2) = store_with(backend2);
    store2
        .write("anthropic", &stored_oauth("stored-wins", "ref", None))
        .await
        .unwrap();
    let resolver2 = CredentialResolver::new(
        store2,
        Arc::new(|name: &str| {
            if name == "ANTHROPIC_OAUTH_TOKEN" {
                Some("env-token-should-not-win".into())
            } else {
                None
            }
        }),
    );
    let mut config2 = OpiConfig::default();
    config2.defaults.model = "anthropic:claude-sonnet-4-5-20250514".into();
    config2.providers.anthropic.base_url = Some(server2.uri());
    let provider2 = build_provider_with_oauth(&config2, &resolver2, &registry)
        .await
        .unwrap();
    let collection2 = dispatch_collection(provider2, "anthropic", &config2, resolver2, &registry);
    drain_route(&collection2, "anthropic:claude-sonnet-4-5-20250514").await;

    let requests2 = server2.received_requests().await.unwrap();
    assert_eq!(requests2.len(), 1);
    let req2 = &requests2[0];
    // Stored cred wins: Bearer is "stored-wins", NOT the env token.
    assert_eq!(
        req2.headers
            .get("authorization")
            .map(|v| v.to_str().unwrap()),
        Some("Bearer stored-wins"),
        "stored OAuth cred must take precedence over ANTHROPIC_OAUTH_TOKEN env"
    );
}

// ===========================================================================
// Slice 6 閳?login_oauth / logout_credential store integration
// ===========================================================================

#[tokio::test]
async fn login_oauth_writes_oauth_credential_to_store() {
    let server = MockServer::start().await;
    mount_token_stub(
        &server,
        200,
        token_body("access-token", "refresh-token", 3600),
    )
    .await;
    let provider = Arc::new(anthropic_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    ));
    let mut registry = OAuthProviderRegistry::new();
    registry.register(provider).unwrap();

    let backend = FakeKeyringBackend::new();
    let (_dir, store, _b) = store_with(backend);

    let presenter = MockLoginPresenter::new();
    presenter.supply_manual_code("test-auth-code");

    let store_ref = &*store;
    login_oauth("anthropic", &registry, store_ref, &presenter)
        .await
        .expect("login succeeds");

    let stored = store
        .read("anthropic")
        .await
        .unwrap()
        .expect("credential stored");
    match stored {
        Credential::OAuthToken {
            access,
            refresh,
            base_url,
            ..
        } => {
            let access_str: &str = access.expose_secret();
            let refresh_str: &str = refresh.expose_secret();
            assert_eq!(access_str, "access-token");
            assert_eq!(refresh_str, "refresh-token");
            assert_eq!(base_url, None);
        }
        other => panic!("expected OAuthToken, got {other:?}"),
    }
}

#[tokio::test]
async fn login_oauth_unknown_provider_errors() {
    let backend = FakeKeyringBackend::new();
    let (_dir, store, _b) = store_with(backend);
    let registry = OAuthProviderRegistry::new();
    let presenter = MockLoginPresenter::new();

    let store_ref = &*store;
    let err = login_oauth("nonexistent", &registry, store_ref, &presenter)
        .await
        .expect_err("unknown provider errors");
    let message = err.to_string();
    assert!(
        message.contains("nonexistent"),
        "error mentions provider id: {message}"
    );
}

#[tokio::test]
async fn login_oauth_store_failure_stays_typed_and_does_not_report_success() {
    let server = MockServer::start().await;
    mount_token_stub(
        &server,
        200,
        token_body("access-token", "refresh-token", 3600),
    )
    .await;
    let mut registry = OAuthProviderRegistry::new();
    registry
        .register(Arc::new(anthropic_provider(
            format!("{}/oauth/token", server.uri()),
            Duration::from_secs(60),
        )))
        .unwrap();
    let (_dir, store, _backend) = store_with(FakeKeyringBackend::new().with_unavailable());
    let presenter = MockLoginPresenter::new();
    presenter.supply_manual_code("test-auth-code");

    let error = login_oauth("anthropic", &registry, &store, &presenter)
        .await
        .expect_err("unavailable store must fail login");

    assert!(matches!(error, AiProviderError::Config(_)), "{error:?}");
    assert_eq!(presenter.notify_success_count.load(Ordering::SeqCst), 0);
    let reasons = presenter.notify_failure_reasons.lock().unwrap().clone();
    assert_eq!(
        reasons,
        vec!["credential store write failed"],
        "persistence failure must emit exactly one fixed notification"
    );
    drop(reasons);
    assert_oauth_error_surfaces_are_redacted(
        &error,
        Some(&presenter),
        &["access-token", "refresh-token", "test-auth-code"],
    );
}

#[tokio::test]
async fn logout_credential_deletes_stored_entry() {
    let backend = FakeKeyringBackend::new();
    let (_dir, store, _b) = store_with(backend);
    store
        .write("anthropic", &stored_oauth("access", "refresh", None))
        .await
        .unwrap();
    assert!(store.read("anthropic").await.unwrap().is_some());

    let store_ref = &*store;
    logout_credential("anthropic", store_ref)
        .await
        .expect("logout succeeds");

    assert!(store.read("anthropic").await.unwrap().is_none());
}

#[tokio::test]
async fn logout_credential_missing_entry_is_noop() {
    let backend = FakeKeyringBackend::new();
    let (_dir, store, _b) = store_with(backend);
    // No credential stored 閳?delete is a no-op.
    let store_ref = &*store;
    logout_credential("anthropic", store_ref)
        .await
        .expect("logout on missing entry is a no-op");
}

#[tokio::test]
async fn logout_credential_store_failure_stays_typed() {
    let (_dir, store, _backend) = store_with(FakeKeyringBackend::new().with_unavailable());

    let error = logout_credential("anthropic", &store)
        .await
        .expect_err("unavailable store must fail logout");

    assert!(matches!(error, AiProviderError::Config(_)), "{error:?}");
}

// ===========================================================================
// Slice 6 閳?acceptance scenarios
// ===========================================================================

#[tokio::test]
async fn all_builtin_flows_support_manual_fallback() {
    // --- Anthropic PKCE: manual code ---
    let server_a = MockServer::start().await;
    mount_token_stub(&server_a, 200, token_body("atk-anth", "rtk-anth", 3600)).await;
    let provider_anth = Arc::new(anthropic_provider(
        format!("{}/oauth/token", server_a.uri()),
        Duration::from_secs(60),
    ));
    let mut registry = OAuthProviderRegistry::new();
    registry.register(provider_anth).unwrap();
    let backend = FakeKeyringBackend::new();
    let (_dir, store, _b) = store_with(backend);
    let store_ref = &*store;
    let presenter = MockLoginPresenter::new();
    presenter.supply_manual_code("anth-code");

    login_oauth("anthropic", &registry, store_ref, &presenter)
        .await
        .expect("anthropic login");
    let cred = store.read("anthropic").await.unwrap().unwrap();
    assert!(
        matches!(cred, Credential::OAuthToken { .. }),
        "anthropic cred stored"
    );

    // --- Codex PKCE: manual code ---
    let server_cx = MockServer::start().await;
    mount_token_stub(
        &server_cx,
        200,
        token_body(&codex_jwt(Some("account-manual")), "rtk-codex", 3600),
    )
    .await;
    let provider_codex = Arc::new(codex_provider(
        format!("{}/oauth/token", server_cx.uri()),
        Duration::from_secs(60),
    ));
    let mut registry2 = OAuthProviderRegistry::new();
    registry2.register(provider_codex).unwrap();
    let backend2 = FakeKeyringBackend::new();
    let (_dir2, store2, _b2) = store_with(backend2);
    let store_ref2 = &*store2;
    let presenter2 = MockLoginPresenter::new();
    presenter2.supply_manual_code("codex-code");

    login_oauth("openai-codex", &registry2, store_ref2, &presenter2)
        .await
        .expect("codex login");
    let cred2 = store2.read("openai-codex").await.unwrap().unwrap();
    assert!(
        matches!(cred2, Credential::OAuthToken { .. }),
        "codex cred stored"
    );

    // --- Copilot device code: no manual code, just device polling ---
    let server_cp = MockServer::start().await;
    mount_device_auth(
        &server_cp,
        json!({"device_code":"dc","user_code":"ABCD-WXYZ","verification_uri":"https://github.com/login/device","interval":0}),
    )
    .await;
    mount_device_poll(&server_cp, json!({"access_token":"ghub-123"}), None).await;
    mount_copilot_token(
        &server_cp,
        200,
        json!({"token":"copilot-tok","expires_at":copilot_expires_soon()}),
    )
    .await;
    let provider_copilot = Arc::new(copilot_provider(server_cp.uri(), Duration::from_secs(60)));
    let mut registry3 = OAuthProviderRegistry::new();
    registry3.register(provider_copilot).unwrap();
    let backend3 = FakeKeyringBackend::new();
    let (_dir3, store3, _b3) = store_with(backend3);
    let store_ref3 = &*store3;
    let presenter3 = MockLoginPresenter::new();
    // Copilot never calls await_manual_code (device code only).
    login_oauth("github-copilot", &registry3, store_ref3, &presenter3)
        .await
        .expect("copilot login");
    let cred3 = store3.read("github-copilot").await.unwrap().unwrap();
    assert!(
        matches!(cred3, Credential::OAuthToken { .. }),
        "copilot cred stored"
    );
    // Prove Copilot never attempts manual code.
    assert_eq!(presenter3.manual_code_calls.load(Ordering::SeqCst), 0);
}

// ===========================================================================
// Slice 6 閳?revoked/no-auto-relogin acceptance
// ===========================================================================

/// A resolver that returns a fixed Bearer credential, then `CredentialRevoked`.
/// Used to prove the provider propagates the non-retryable error and the turn
/// stops without retry or auto-relogin.
struct OneShotThenRevokedResolver {
    bearer: SecretString,
    provider_id: String,
    revoked: std::sync::atomic::AtomicBool,
}

impl AuthResolver for OneShotThenRevokedResolver {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, AiProviderError>> {
        let secret = self.bearer.clone();
        let provider_id = self.provider_id.clone();
        let already = self.revoked.swap(true, Ordering::SeqCst);
        Box::pin(async move {
            if already {
                Err(AiProviderError::CredentialRevoked { provider_id })
            } else {
                Ok(ResolvedAuth {
                    scheme: AuthScheme::Bearer,
                    secret,
                    base_url: None,
                    account_id: None,
                    provenance: opi_ai::AuthProvenance::default(),
                })
            }
        })
    }
}

#[tokio::test]
async fn anthropic_oauth_revoked_stops_turn_without_retry_or_relogin() {
    let server = MockServer::start().await;
    // First request: succeeds (mock response).
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    // Phase 17.5: auth resolution lives in ProviderCollection::prepare_call.
    // Register the one-shot resolver on the route so the revoked-credential path
    // surfaces from prepare_call (the resolver returns CredentialRevoked on its
    // second resolution, before any HTTP request).
    let resolver: Arc<dyn AuthResolver> = Arc::new(OneShotThenRevokedResolver {
        bearer: SecretString::new("oauth-token-revoked-test".into()),
        provider_id: "anthropic".into(),
        revoked: std::sync::atomic::AtomicBool::new(false),
    });
    let provider = AnthropicProvider::with_client(Some(server.uri()), Arc::new(HttpClient::new()));
    let mut collection = ProviderCollection::new();
    collection
        .register_route(
            Box::new(provider),
            resolver,
            AuthProvenanceSource::OAuth {
                kind: "anthropic".into(),
            },
            CompatMetadata::default(),
        )
        .expect("register anthropic revoked-test route");
    let spec = "anthropic:claude-sonnet-4-5-20250514";
    let prepared = collection
        .prepare_call(spec, factory_request(spec))
        .await
        .expect("first prepare_call resolves Bearer");
    let mut stream = prepared.start_attempt().expect("first start_attempt");
    drain_stream(&mut stream).await;
    // First call succeeded.

    // Second call: the resolver now returns CredentialRevoked, which must surface
    // from prepare_call (no retry, no other request sent).
    let err_provider = match collection.prepare_call(spec, factory_request(spec)).await {
        Err(opi_ai::CollectionError::Provider(p)) => p,
        other => panic!("expected CollectionError::Provider(CredentialRevoked), got {other:?}"),
    };
    match &err_provider {
        AiProviderError::CredentialRevoked { provider_id } => {
            assert_eq!(provider_id, "anthropic");
        }
        other => panic!("expected CredentialRevoked, got {other:?}"),
    }
    assert!(
        !err_provider.is_retryable(),
        "CredentialRevoked must be non-retryable"
    );
    // Prove no second HTTP request was sent (the error came from the resolver,
    // before any HTTP call).
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests.len(),
        1,
        "revoked credential must not trigger an HTTP request"
    );
}

#[test]
fn login_logout_commands_are_discoverable() {
    let binary = option_env!("CARGO_BIN_EXE_opi")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("../../target/debug/opi");
            if cfg!(windows) {
                path.set_extension("exe");
            }
            path
        });
    let output = std::process::Command::new(&binary)
        .arg("--help")
        .output()
        .unwrap_or_else(|error| panic!("failed to run {} --help: {error}", binary.display()));
    assert!(output.status.success(), "opi --help failed: {output:?}");
    let stdout = String::from_utf8(output.stdout).expect("opi --help is UTF-8");

    for command in ["/login <provider>", "/logout <provider>"] {
        assert!(
            stdout.contains(command),
            "opi --help must make interactive command `{command}` discoverable; got:\n{stdout}"
        );
    }
    assert!(
        stdout.contains("Anthropic")
            && stdout.contains("GitHub Copilot")
            && stdout.contains("OpenAI Codex"),
        "opi --help must name the three approved OAuth profiles; got:\n{stdout}"
    );
}

#[tokio::test]
async fn factory_built_approved_profiles_resolve_auth_inside_each_stream() {
    for (provider_id, model) in [
        ("anthropic", "claude-sonnet-4-5-20250514"),
        ("github-copilot", "gpt-4.1"),
        ("openai-codex", "gpt-5.4"),
    ] {
        let server = MockServer::start().await;
        let (_dir, store, _backend) = store_with(FakeKeyringBackend::new());
        let resolver = resolver_with(store);
        let registry = OAuthProviderRegistry::registry_with_builtins();
        let mut config = OpiConfig::default();
        config.defaults.model = format!("{provider_id}:{model}");
        if provider_id == "anthropic" {
            config.providers.anthropic.base_url = Some(server.uri());
        }

        let provider = build_provider_with_oauth(&config, &resolver, &registry)
            .await
            .unwrap_or_else(|error| {
                panic!("{provider_id} must construct without an available credential: {error}")
            });
        // Phase 17.5: auth resolution moved to prepare_call on the collection
        // route. With no credential stored, prepare_call fails with
        // CredentialNeeded before any HTTP request.
        let collection = dispatch_collection(provider, provider_id, &config, resolver, &registry);
        let spec = config.defaults.model.clone();
        let first = match collection.prepare_call(&spec, factory_request(&spec)).await {
            Err(opi_ai::CollectionError::Provider(p)) => p,
            other => panic!("{provider_id} expected CredentialNeeded, got {other:?}"),
        };
        match first {
            AiProviderError::CredentialNeeded {
                provider_id: actual,
            } => assert_eq!(actual, provider_id),
            other => panic!("{provider_id} expected CredentialNeeded, got {other:?}"),
        }
        if provider_id != "github-copilot" {
            assert!(
                server.received_requests().await.unwrap().is_empty(),
                "{provider_id} must resolve auth before any HTTP request"
            );
            continue;
        }

        // Copilot alone owns credential-supplied enterprise routing metadata.
        // Seed that route, construct, then remove the credential so the next
        // prepare_call proves auth fails before HTTP.
        let (_routed_dir, routed_store, _routed_backend) = store_with(FakeKeyringBackend::new());
        routed_store
            .write(
                provider_id,
                &stored_oauth_for(
                    provider_id,
                    "route-only",
                    "route-refresh",
                    Some(server.uri()),
                ),
            )
            .await
            .unwrap();
        let routed_resolver = resolver_with(routed_store.clone());
        let routed_provider = build_provider_with_oauth(&config, &routed_resolver, &registry)
            .await
            .unwrap();
        routed_store.delete(provider_id).await.unwrap();

        let routed_collection = dispatch_collection(
            routed_provider,
            provider_id,
            &config,
            routed_resolver,
            &registry,
        );
        let routed_error =
            match collection_prepare_call_error(&routed_collection, &config.defaults.model).await {
                Some(p) => p,
                None => panic!("{provider_id} routed prepare_call unexpectedly succeeded"),
            };
        match routed_error {
            AiProviderError::CredentialNeeded {
                provider_id: actual,
            } => assert_eq!(actual, provider_id),
            other => panic!("{provider_id} expected routed CredentialNeeded, got {other:?}"),
        }
        assert!(
            server.received_requests().await.unwrap().is_empty(),
            "{provider_id} must resolve auth before any routed HTTP request"
        );
    }
}

/// Phase 17.5 helper: run prepare_call and return the wrapped ProviderError if
/// the collection rejected the call, or `None` if it resolved a route.
async fn collection_prepare_call_error(
    collection: &ProviderCollection,
    spec: &str,
) -> Option<AiProviderError> {
    match collection.prepare_call(spec, factory_request(spec)).await {
        Err(opi_ai::CollectionError::Provider(p)) => Some(p),
        Err(other) => panic!("expected Provider error from prepare_call, got {other:?}"),
        Ok(_) => None,
    }
}

#[tokio::test]
async fn factory_stream_reresolves_after_store_change() {
    for (provider_id, model, request_path) in [
        ("anthropic", "claude-sonnet-4-5-20250514", "/v1/messages"),
        ("github-copilot", "gpt-4.1", "/chat/completions"),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(request_path))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("")
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;

        let (_dir, store, _backend) = store_with(FakeKeyringBackend::new());
        let old = format!("old-{provider_id}-credential");
        if provider_id == "anthropic" {
            store
                .write(provider_id, &Credential::ApiKey(secret(&old)))
                .await
                .unwrap();
        } else {
            store
                .write(
                    provider_id,
                    &stored_oauth_for(provider_id, &old, "old-refresh", Some(server.uri())),
                )
                .await
                .unwrap();
        }
        let resolver = resolver_with(store.clone());
        let registry = OAuthProviderRegistry::registry_with_builtins();
        let mut config = OpiConfig::default();
        config.defaults.model = format!("{provider_id}:{model}");
        if provider_id == "anthropic" {
            config.providers.anthropic.base_url = Some(server.uri());
        }
        let provider = build_provider_with_oauth(&config, &resolver, &registry)
            .await
            .unwrap();
        // Phase 17.5: auth moved to the collection route. Each prepare_call
        // re-resolves the live credential, so changing the store between calls
        // is observed by the next logical call.
        let collection = dispatch_collection(provider, provider_id, &config, resolver, &registry);

        drain_route(&collection, &config.defaults.model).await;

        let new = format!("new-{provider_id}-credential");
        let base_url = (provider_id != "anthropic").then(|| server.uri());
        store
            .write(
                provider_id,
                &stored_oauth_for(provider_id, &new, "new-refresh", base_url),
            )
            .await
            .unwrap();
        drain_route(&collection, &config.defaults.model).await;

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 2, "{provider_id} request count");
        if provider_id == "anthropic" {
            assert_eq!(
                requests[0]
                    .headers
                    .get("x-api-key")
                    .map(|value| value.to_str().unwrap()),
                Some(old.as_str())
            );
            assert_eq!(
                requests[1]
                    .headers
                    .get("authorization")
                    .map(|value| value.to_str().unwrap()),
                Some(format!("Bearer {new}").as_str())
            );
        } else {
            assert_eq!(
                requests[0]
                    .headers
                    .get("authorization")
                    .map(|value| value.to_str().unwrap()),
                Some(format!("Bearer {old}").as_str())
            );
            assert_eq!(
                requests[1]
                    .headers
                    .get("authorization")
                    .map(|value| value.to_str().unwrap()),
                Some(format!("Bearer {new}").as_str())
            );
        }
    }
}

struct RefreshDropFlag(Arc<AtomicBool>);

impl Drop for RefreshDropFlag {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

struct HangingRefreshProvider {
    dropped: Arc<AtomicBool>,
}

impl OAuthProvider for HangingRefreshProvider {
    fn id(&self) -> &str {
        PROVIDER
    }

    fn login<'a>(
        &'a self,
        _presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, AiProviderError>> {
        Box::pin(async { Err(AiProviderError::Config("unused login".into())) })
    }

    fn refresh<'a>(
        &'a self,
        _cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, AiProviderError>> {
        let dropped = self.dropped.clone();
        Box::pin(async move {
            let _drop_flag = RefreshDropFlag(dropped);
            std::future::pending::<Result<OAuthCredential, AiProviderError>>().await
        })
    }
}

#[tokio::test]
async fn refresh_timeout_releases_lock_and_preserves_prior_credential() {
    let (dir, store, backend) = store_with(FakeKeyringBackend::new());
    let prior = oauth_credential("prior-access", Some(near_expiry()));
    store.write(PROVIDER, &prior).await.unwrap();
    let raw_before = backend
        .raw_entry(KEYCHAIN_SERVICE, PROVIDER)
        .expect("prior credential envelope");
    let resolver = CredentialResolver::with_refresh_timeout(
        store.clone(),
        Arc::new(|_: &str| None),
        Duration::from_millis(25),
    );
    let dropped = Arc::new(AtomicBool::new(false));
    let oauth = HangingRefreshProvider {
        dropped: dropped.clone(),
    };

    let error = resolver
        .resolve_oauth(PROVIDER, &oauth)
        .await
        .expect_err("hung refresh must be bounded");
    assert!(matches!(error, AiProviderError::AuthFailed(_)), "{error:?}");
    assert!(!error.is_retryable());
    assert!(error.to_string().contains(PROVIDER));
    assert!(
        dropped.load(Ordering::SeqCst),
        "refresh future was not dropped"
    );
    assert_eq!(
        backend.raw_entry(KEYCHAIN_SERVICE, PROVIDER).as_deref(),
        Some(raw_before.as_str()),
        "timeout must not partially replace the credential"
    );

    let competing_store = KeychainCredentialStore::with_lock_timeout(
        Box::new(backend),
        dir.path().to_path_buf(),
        Duration::from_millis(100),
    );
    tokio::time::timeout(
        Duration::from_millis(250),
        competing_store.write("lock-probe", &Credential::ApiKey(secret("probe"))),
    )
    .await
    .expect("refresh timeout must release the mutation lock")
    .expect("lock probe write succeeds");
}

#[tokio::test]
async fn factory_built_approved_profiles_map_revocation_without_retry() {
    for (provider_id, model, request_path) in [
        ("anthropic", "claude-sonnet-4-5-20250514", "/v1/messages"),
        ("github-copilot", "gpt-4.1", "/chat/completions"),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(request_path))
            .respond_with(ResponseTemplate::new(401).set_body_string("revoked-secret-canary"))
            .mount(&server)
            .await;
        let (_dir, store, _backend) = store_with(FakeKeyringBackend::new());
        let base_url = (provider_id != "anthropic").then(|| server.uri());
        store
            .write(
                provider_id,
                &stored_oauth_for(provider_id, "revoked-access", "revoked-refresh", base_url),
            )
            .await
            .unwrap();
        let resolver = resolver_with(store);
        let registry = OAuthProviderRegistry::registry_with_builtins();
        let mut config = OpiConfig::default();
        config.defaults.model = format!("{provider_id}:{model}");
        if provider_id == "anthropic" {
            config.providers.anthropic.base_url = Some(server.uri());
        }
        let provider = build_provider_with_oauth(&config, &resolver, &registry)
            .await
            .unwrap();
        // Phase 17.5: pass the store-backed resolver so the harness's
        // prepare_call resolves the stored OAuth credential (the dummy resolver
        // would bypass it and never exercise the revocation path).
        let auth_resolver = oauth_auth_resolver_for(provider_id, &config, resolver, &registry);
        let workspace = tempfile::tempdir().unwrap();
        let mut harness = CodingHarness::builder(
            provider,
            config.defaults.model.clone(),
            config,
            workspace.path().to_path_buf(),
            opi_coding_agent::project_trust::TrustDecision::Trusted,
        )
        .tool_selection(ToolSelection::Disabled)
        .auth_resolver(auth_resolver)
        .build();

        let error = tokio::time::timeout(Duration::from_secs(2), harness.prompt("hello"))
            .await
            .unwrap_or_else(|_| panic!("{provider_id} revocation blocked"))
            .expect_err("auth-invalid response must fail the agent turn");
        match &error {
            AgentError::CredentialRevoked {
                provider_id: actual,
            } => assert_eq!(actual, provider_id),
            other => panic!("{provider_id} expected CredentialRevoked, got {other:?}"),
        }
        assert!(
            !error.to_string().contains("revoked-secret-canary"),
            "{provider_id} leaked the auth-invalid body"
        );
        tokio::task::yield_now().await;
        assert_eq!(
            server.received_requests().await.unwrap().len(),
            1,
            "{provider_id} must issue exactly one request"
        );
    }
}
