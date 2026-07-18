//! Phase 14.2 — OAuth auth source, per-request refresh, and command flows.
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
    AuthResolver, AuthScheme, LoginPresenter, OAuthCredential, OAuthProvider, ResolvedAuth,
};
use opi_ai::credential::{BoxAuthFuture, Credential, CredentialStore};
use opi_ai::http::HttpClient;
use opi_ai::provider::{
    CacheRetention, EventStream, ModelInfo, Provider, ProviderError as AiProviderError, Request,
    ThinkingConfig,
};
use opi_ai::registry::ModelCapabilities;
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
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use futures_util::{StreamExt, stream};

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

fn oauth_cred(access: &str, refresh: &str, base_url: Option<String>) -> OAuthCredential {
    OAuthCredential {
        access: secret(access),
        refresh: secret(refresh),
        expires_at: Some(OffsetDateTime::now_utc() + Duration::from_secs(3600)),
        base_url,
    }
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

    fn stream(&self, _request: Request) -> EventStream {
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
// Slice 4 — MockLoginPresenter seam + AnthropicOAuthProvider PKCE flow
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
        // The authorize URL carries code_challenge, never the verifier.
        assert!(url.contains("code_challenge="));
        assert!(!url.contains("code_verifier"));
        let _ = reqwest::get(format!(
            "http://127.0.0.1:{port}/?code=CB-CODE&state={state}"
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
    assert!(body.contains("code=CB-CODE"), "body: {body}");
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
    let msg = match err {
        AiProviderError::AuthFailed(m) => m,
        other => panic!("expected AuthFailed, got {other:?}"),
    };
    // OAuth error fields may be surfaced...
    assert!(
        msg.contains("invalid_grant") || msg.contains("400"),
        "msg: {msg}"
    );
    // ...but the auth code must NOT.
    assert!(!msg.contains("AUTHCODE-LEAK-CANARY"), "code leaked: {msg}");
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
// Slice 4 — CodexOAuthProvider (PKCE, delegates to the shared runner)
// ===========================================================================

#[tokio::test]
async fn codex_oauth_provider_id_is_codex() {
    let provider = codex_provider("https://token.example".to_owned(), Duration::from_secs(60));
    assert_eq!(provider.id(), "openai-codex");
}

#[tokio::test]
async fn codex_login_manual_code_wins_drives_token_post_and_returns_credential() {
    let server = MockServer::start().await;
    mount_token_stub(&server, 200, token_body("codex-atk", "codex-rtk", 3600)).await;
    let provider = codex_provider(
        format!("{}/oauth/token", server.uri()),
        Duration::from_secs(60),
    );
    let presenter = MockLoginPresenter::new();
    presenter.supply_manual_code("CODE-XYZ");
    let cred = provider.login(&presenter).await.expect("login");
    assert_eq!(cred.access.expose_secret(), "codex-atk");
    assert_eq!(cred.refresh.expose_secret(), "codex-rtk");
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
    mount_token_stub(&server, 200, token_body("codex-atk", "codex-rtk", 3600)).await;
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
    assert_eq!(cred.access.expose_secret(), "codex-atk");
}

#[tokio::test]
async fn codex_login_state_mismatch_rejects_callback_and_notifies_failure() {
    let server = MockServer::start().await;
    mount_token_stub(&server, 200, token_body("codex-atk", "codex-rtk", 3600)).await;
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
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"access_token":"codex-new","expires_in":3600})),
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
    assert_eq!(refreshed.access.expose_secret(), "codex-new");
    assert_eq!(refreshed.refresh.expose_secret(), "codex-rtk-old");
    assert_eq!(refreshed.base_url.as_deref(), Some("https://enterprise"));
}

#[tokio::test]
async fn codex_login_does_not_leak_verifier_or_tokens_into_error_strings() {
    let server = MockServer::start().await;
    mount_token_stub(&server, 500, json!({"error":"server_error"})).await;
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
}

// ===========================================================================
// Slice 4 — OAuthProviderRegistry
// ===========================================================================

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
// Slice 5 — registry_with_builtins (production OAuth providers)
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
    };
    store.write("anthropic", &anthropic_cred).await.unwrap();
    assert!(resolver.has_oauth_credential("anthropic").await.unwrap());
    assert_eq!(
        resolver.read_oauth_base_url("anthropic").await.unwrap(),
        None
    );
}

// ===========================================================================
// Slice 4 — CopilotOAuthProvider (GitHub device-code)
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
    // impl increases the interval by exactly 5s (RFC 8628 §3.5, persistent) and
    // continues polling. With a total budget shorter than the post-slow_down
    // sleep, the flow times out during that sleep rather than returning a
    // CredentialRevoked denial — proving slow_down was handled as a backoff.
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
// Slice 4 — TuiLoginPresenter (production, print-only substrate)
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
// Slice 5 — factory OAuth wiring (product-loop via build_provider_with_oauth)
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

/// A fresh (non-expiring) stored OAuth credential. `base_url` redirects dispatch
/// to a mock when `Some`.
fn stored_oauth(access: &str, refresh: &str, base_url: Option<String>) -> Credential {
    Credential::OAuthToken {
        access: secret(access),
        refresh: secret(refresh),
        expires_at: Some(fresh_expiry()),
        base_url,
    }
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
    for model in ["claude-sonnet-4.5", "gpt-4.1", "gpt-5.4"] {
        let mut stream = provider.stream(factory_request(&format!("github-copilot:{model}")));
        drain_stream(&mut stream).await;
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
async fn factory_routes_codex_to_codex_responses_with_oauth_wire_shape() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/codex/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let backend = FakeKeyringBackend::new();
    let (_dir, store, _b) = store_with(backend);
    // Production Codex PKCE carries no base_url (CODEX_DEFAULT_BASE_URL wins);
    // the stored base_url here is a test seam redirecting dispatch to the mock.
    store
        .write(
            "openai-codex",
            &stored_oauth(
                "codex-access-fake",
                "codex-refresh-fake",
                Some(server.uri()),
            ),
        )
        .await
        .unwrap();
    let resolver = resolver_with(store);
    let registry = OAuthProviderRegistry::registry_with_builtins();

    let mut config = OpiConfig::default();
    config.defaults.model = "openai-codex:gpt-5".into();

    let provider = build_provider_with_oauth(&config, &resolver, &registry)
        .await
        .expect("codex OAuth provider builds");
    assert_eq!(provider.id(), "openai-codex");
    let mut stream = provider.stream(factory_request("openai-codex:gpt-5"));
    drain_stream(&mut stream).await;

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "exactly one Codex Responses request");
    let req = &requests[0];
    assert_eq!(req.url.path(), "/codex/responses");
    assert_eq!(
        req.headers
            .get("authorization")
            .map(|v| v.to_str().unwrap()),
        Some("Bearer codex-access-fake")
    );
    assert_eq!(
        req.headers.get("OpenAI-Beta").map(|v| v.to_str().unwrap()),
        Some("responses=experimental")
    );
    assert_eq!(
        req.headers.get("originator").map(|v| v.to_str().unwrap()),
        Some("opi")
    );
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
    let mut stream = provider.stream(factory_request("anthropic:claude-sonnet-4-5-20250514"));
    drain_stream(&mut stream).await;

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
    let mut stream = provider.stream(factory_request("anthropic:claude-sonnet-4-5-20250514"));
    drain_stream(&mut stream).await;

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
    let mut stream2 = provider2.stream(factory_request("anthropic:claude-sonnet-4-5-20250514"));
    drain_stream(&mut stream2).await;

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
// Slice 6 — login_oauth / logout_credential store integration
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
    let message = error.to_string();
    assert!(
        !message.contains("access-token"),
        "secret leaked: {message}"
    );
    assert!(
        !message.contains("refresh-token"),
        "secret leaked: {message}"
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
    // No credential stored — delete is a no-op.
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
// Slice 6 — acceptance scenarios
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
    mount_token_stub(&server_cx, 200, token_body("atk-codex", "rtk-codex", 3600)).await;
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
// Slice 6 — revoked/no-auto-relogin acceptance
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

    let resolver = Arc::new(OneShotThenRevokedResolver {
        bearer: SecretString::new("oauth-token-revoked-test".into()),
        provider_id: "anthropic".into(),
        revoked: std::sync::atomic::AtomicBool::new(false),
    });
    let provider =
        AnthropicProvider::with_auth(resolver, Some(server.uri()), Arc::new(HttpClient::new()));
    let mut stream = provider.stream(factory_request("anthropic:claude-sonnet-4-5-20250514"));
    drain_stream(&mut stream).await;
    // First call succeeded.

    // Second call: the resolver now returns CredentialRevoked, which must
    // surface as the first stream event (no retry, no other request sent).
    let mut stream2 = provider.stream(factory_request("anthropic:claude-sonnet-4-5-20250514"));
    let err = stream2
        .next()
        .await
        .expect("an event")
        .expect_err("CredentialRevoked is an error");
    match &err {
        AiProviderError::CredentialRevoked { provider_id } => {
            assert_eq!(provider_id, "anthropic");
        }
        other => panic!("expected CredentialRevoked, got {other:?}"),
    }
    assert!(
        !err.is_retryable(),
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
        ("openai-codex", "gpt-5"),
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
        let mut stream = provider.stream(factory_request(&config.defaults.model));
        let first = tokio::time::timeout(Duration::from_secs(2), stream.next())
            .await
            .unwrap_or_else(|_| panic!("{provider_id} auth resolution blocked"))
            .unwrap_or_else(|| panic!("{provider_id} stream ended without auth result"))
            .expect_err("missing credential must fail before provider output");
        match first {
            AiProviderError::CredentialNeeded {
                provider_id: actual,
            } => assert_eq!(actual, provider_id),
            other => panic!("{provider_id} expected CredentialNeeded, got {other:?}"),
        }
        if provider_id == "anthropic" {
            assert!(
                server.received_requests().await.unwrap().is_empty(),
                "{provider_id} must resolve auth before any HTTP request"
            );
            continue;
        }

        // Copilot and Codex derive their API base URL from the stored OAuth
        // profile during construction. Seed that route, construct, then remove
        // the credential so the first stream proves auth fails before HTTP.
        let (_routed_dir, routed_store, _routed_backend) = store_with(FakeKeyringBackend::new());
        routed_store
            .write(
                provider_id,
                &stored_oauth("route-only", "route-refresh", Some(server.uri())),
            )
            .await
            .unwrap();
        let routed_resolver = resolver_with(routed_store.clone());
        let routed_provider = build_provider_with_oauth(&config, &routed_resolver, &registry)
            .await
            .unwrap();
        routed_store.delete(provider_id).await.unwrap();

        let mut routed_stream = routed_provider.stream(factory_request(&config.defaults.model));
        let routed_error = tokio::time::timeout(Duration::from_secs(2), routed_stream.next())
            .await
            .unwrap_or_else(|_| panic!("{provider_id} routed auth resolution blocked"))
            .unwrap_or_else(|| panic!("{provider_id} routed stream ended without auth result"))
            .expect_err("removed credential must fail before routed provider output");
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

#[tokio::test]
async fn factory_stream_reresolves_after_store_change() {
    for (provider_id, model, request_path) in [
        ("anthropic", "claude-sonnet-4-5-20250514", "/v1/messages"),
        ("github-copilot", "gpt-4.1", "/chat/completions"),
        ("openai-codex", "gpt-5", "/codex/responses"),
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
                    &stored_oauth(&old, "old-refresh", Some(server.uri())),
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

        let mut first = provider.stream(factory_request(&config.defaults.model));
        drain_stream(&mut first).await;

        let new = format!("new-{provider_id}-credential");
        let base_url = (provider_id != "anthropic").then(|| server.uri());
        store
            .write(provider_id, &stored_oauth(&new, "new-refresh", base_url))
            .await
            .unwrap();
        let mut second = provider.stream(factory_request(&config.defaults.model));
        drain_stream(&mut second).await;

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
        ("openai-codex", "gpt-5", "/codex/responses"),
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
                &stored_oauth("revoked-access", "revoked-refresh", base_url),
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
        let workspace = tempfile::tempdir().unwrap();
        let mut harness = CodingHarness::builder(
            provider,
            config.defaults.model.clone(),
            config,
            workspace.path().to_path_buf(),
        )
        .tool_selection(ToolSelection::Disabled)
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
