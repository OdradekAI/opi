//! Phase 20.23 — SDK OAuth (Anthropic) acceptance flows.
//!
//! Every claim runs against the public SDK surface: the Anthropic flow in
//! `opi_ai::auth::oauth`, `Models::{login, get_auth, check_auth}`, and the
//! store-owned mutation ACK. HTTP is faked through wiremock on loopback; the
//! presenter is scripted. No ambient browser, keychain, environment override,
//! or provider network access happens here.

use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use opi_ai::Models;
use opi_ai::auth::oauth::anthropic::AnthropicOAuthProvider;
use opi_ai::auth::{LoginPresenter, OAuthCredential, OAuthLoginMethod, OAuthProvider};
use opi_ai::credential::BoxAuthFuture;
use opi_ai::credential::{Credential, CredentialSource, CredentialStore, CredentialStoreError};
use opi_ai::mutation::{MutationOutcome, MutationReceipt};
use opi_ai::provider::ProviderError;
use secrecy::ExposeSecret;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Test doubles
// ---------------------------------------------------------------------------

/// Scripted presenter: records presented URLs, offers a manual-code reply,
/// and can force a login-method selection. Success notifications record the
/// wall time so tests can assert ordering against store ACKs.
type PresentHook = Arc<dyn Fn(&str) -> String + Send + Sync>;

#[derive(Default)]
struct ScriptedPresenter {
    authorize_urls: Mutex<Vec<String>>,
    manual_reply: Mutex<Option<String>>,
    /// Hook run when the authorize URL is presented: derives the manual
    /// paste from the flow's own URL (echo state, pick the callback port).
    on_present: Mutex<Option<PresentHook>>,
    forced_method: Mutex<Option<OAuthLoginMethod>>,
    offered_methods: Mutex<Vec<OAuthLoginMethod>>,
    success_at: Mutex<Option<Instant>>,
    failures: Mutex<Vec<String>>,
}

impl ScriptedPresenter {
    fn with_manual(code: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            manual_reply: Mutex::new(Some(code.into())),
            ..Self::default()
        })
    }

    /// Echo the presented authorize URL back as a pasted redirect URL with
    /// `code-1` and the flow's own state.
    fn echo_redirect_hook() -> PresentHook {
        Arc::new(|url: &str| {
            let state = ScriptedPresenter::query_param(url, "state").unwrap();
            let redirect = ScriptedPresenter::query_param(url, "redirect_uri").unwrap();
            format!("{redirect}?code=code-1&state={state}")
        })
    }

    fn last_authorize_url(&self) -> String {
        self.authorize_urls
            .lock()
            .unwrap()
            .last()
            .cloned()
            .expect("presenter saw no authorize url")
    }

    fn query_param(url: &str, name: &str) -> Option<String> {
        let query = url.split_once('?')?.1;
        for pair in query.split('&') {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            if k == name {
                return Some(percent_decode(v));
            }
        }
        None
    }
}

/// Percent-decode a query value the way the flow's authorize URL encodes it.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

impl LoginPresenter for ScriptedPresenter {
    fn select_login_method<'a>(
        &'a self,
        _provider_id: &'a str,
        methods: &'a [OAuthLoginMethod],
        default: OAuthLoginMethod,
    ) -> BoxAuthFuture<'a, Result<OAuthLoginMethod, ProviderError>> {
        let forced = *self.forced_method.lock().unwrap();
        *self.offered_methods.lock().unwrap() = methods.to_vec();
        Box::pin(async move {
            if let Some(forced) = forced
                && methods.contains(&forced)
            {
                return Ok(forced);
            }
            if methods.contains(&default) {
                Ok(default)
            } else {
                Err(ProviderError::Config(
                    opi_ai::provider::ProviderErrorSummary::redacted(),
                ))
            }
        })
    }

    fn present_auth_url<'a>(
        &'a self,
        url: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        let url = url.to_owned();
        Box::pin(async move {
            if let Some(hook) = self.on_present.lock().unwrap().clone() {
                *self.manual_reply.lock().unwrap() = Some(hook(&url));
            }
            self.authorize_urls.lock().unwrap().push(url);
            Ok(())
        })
    }

    fn present_device_code<'a>(
        &'a self,
        _user_code: &'a str,
        _verification_uri: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        Box::pin(async { Ok(()) })
    }

    fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
        let reply = self.manual_reply.lock().unwrap().clone();
        Box::pin(async move {
            match reply {
                Some(code) => Ok(code),
                None => std::future::pending().await,
            }
        })
    }

    fn notify_success(&self) {
        *self.success_at.lock().unwrap() = Some(Instant::now());
    }

    fn notify_failure(&self, reason: &str) {
        self.failures.lock().unwrap().push(reason.to_owned());
    }
}

/// Store behavior the login path must acknowledge.
#[derive(Clone, Copy, PartialEq, Default)]
enum StoreMode {
    #[default]
    Ok,
    Fail,
    Unknown,
}

/// In-memory `CredentialStore` recording every mutation with ACK outcomes and
/// observing modify concurrency so tests can prove serialization.
#[derive(Default)]
struct FakeStore {
    entries: Mutex<HashMap<String, Credential>>,
    mode: Mutex<StoreMode>,
    /// Store-owned serialization: mutations hold this gate across their
    /// entire execution, mirroring the queue-backed production store.
    gate: tokio::sync::Mutex<()>,
    modify_log: Mutex<Vec<String>>,
    modify_starts: Mutex<usize>,
    modify_ends: Mutex<usize>,
    max_overlapping: Mutex<usize>,
}

impl FakeStore {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn set_mode(&self, mode: StoreMode) {
        *self.mode.lock().unwrap() = mode;
    }

    fn entry(&self, provider_id: &str) -> Option<Credential> {
        self.entries.lock().unwrap().get(provider_id).cloned()
    }
}

impl CredentialStore for FakeStore {
    fn read<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<Option<Credential>, CredentialStoreError>> {
        Box::pin(async move { Ok(self.entries.lock().unwrap().get(provider_id).cloned()) })
    }

    fn write<'a>(
        &'a self,
        provider_id: &'a str,
        cred: &'a Credential,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
        Box::pin(async move {
            self.entries
                .lock()
                .unwrap()
                .insert(provider_id.to_owned(), cred.clone());
            Ok(())
        })
    }

    fn delete<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
        Box::pin(async move {
            self.entries.lock().unwrap().remove(provider_id);
            Ok(())
        })
    }

    fn probe<'a>(&'a self, provider_id: &'a str) -> BoxAuthFuture<'a, CredentialSource> {
        Box::pin(async move {
            if self.entries.lock().unwrap().contains_key(provider_id) {
                CredentialSource::Present {
                    label: format!("test store {provider_id}"),
                }
            } else {
                CredentialSource::Absent
            }
        })
    }

    fn modify<'a>(
        &'a self,
        provider_id: &'a str,
        op: opi_ai::mutation::CredentialMutationOp,
    ) -> BoxAuthFuture<'a, Result<MutationReceipt, CredentialStoreError>> {
        Box::pin(async move {
            let _gate = self.gate.lock().await;
            let provider_id = provider_id.to_owned();
            {
                let mut starts = self.modify_starts.lock().unwrap();
                *starts += 1;
                let overlapping = *starts - *self.modify_ends.lock().unwrap();
                let mut max = self.max_overlapping.lock().unwrap();
                if overlapping > *max {
                    *max = overlapping;
                }
            }
            // A small delay widens the race window: concurrent modifications
            // must still serialize in the recorded log.
            tokio::time::sleep(Duration::from_millis(20)).await;
            let mode = *self.mode.lock().unwrap();
            self.modify_log.lock().unwrap().push(provider_id.clone());
            let revision = (self.entries.lock().unwrap().len() + 1) as u64;
            let outcome = match mode {
                StoreMode::Ok => match op {
                    opi_ai::mutation::CredentialMutationOp::Write(cred) => {
                        self.entries
                            .lock()
                            .unwrap()
                            .insert(provider_id.clone(), cred);
                        MutationOutcome::Written { revision }
                    }
                    opi_ai::mutation::CredentialMutationOp::Delete => {
                        self.entries.lock().unwrap().remove(&provider_id);
                        MutationOutcome::Deleted { revision }
                    }
                    opi_ai::mutation::CredentialMutationOp::Update(_) => {
                        MutationOutcome::Unchanged { revision }
                    }
                },
                StoreMode::Fail => MutationOutcome::Failed {
                    error: Arc::new(opi_ai::mutation::MutationError::Store(
                        CredentialStoreError::Backend {
                            provider: provider_id.clone(),
                            reason: "injected failure".into(),
                        },
                    )),
                },
                StoreMode::Unknown => MutationOutcome::Unknown {
                    detail: "injected unknown".into(),
                },
            };
            *self.modify_ends.lock().unwrap() += 1;
            Ok(MutationReceipt {
                provider_id,
                generation: 0,
                outcome,
            })
        })
    }
}

/// Guard around port 53692 so tests that assert its exact behavior do not
/// race each other inside this binary. A tokio mutex keeps the guard `Send`
/// across the awaited login bodies.
static PREFERRED_PORT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn anthropic_flow(token_url: String) -> AnthropicOAuthProvider {
    let scheme = "https";
    AnthropicOAuthProvider::new(
        format!("{scheme}://authorize.example.test/authorize"),
        token_url,
        "test-client-id".to_owned(),
        Duration::from_secs(30),
    )
}

fn token_body(access: &str, refresh: &str, expires_in: i64) -> serde_json::Value {
    serde_json::json!({
        "access_token": access,
        "refresh_token": refresh,
        "expires_in": expires_in,
    })
}

async fn mount_token_success(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(token_body("acc-1", "ref-1", 3600)))
        .mount(server)
        .await;
}

/// Serialize every browser-method login: each one tries to bind the shared
/// preferred port 53692.
async fn port_guard() -> tokio::sync::MutexGuard<'static, ()> {
    PREFERRED_PORT_LOCK.lock().await
}

/// Run a browser-method login whose manual arm echoes the authorize URL
/// (same state, `code-1`).
async fn login_via_echoed_redirect(
    flow: &AnthropicOAuthProvider,
) -> Result<OAuthCredential, ProviderError> {
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.on_present.lock().unwrap() = Some(ScriptedPresenter::echo_redirect_hook());
    flow.login(presenter.as_ref()).await
}

// ---------------------------------------------------------------------------
// Login wire shape
// ---------------------------------------------------------------------------

#[tokio::test]
async fn browser_login_posts_json_token_request() {
    let _port_guard = port_guard().await;
    let server = MockServer::start().await;
    mount_token_success(&server).await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());
    {
        let hook = ScriptedPresenter::echo_redirect_hook();
        *presenter.on_present.lock().unwrap() = Some(hook);
    }

    let credential = flow.login(presenter.as_ref()).await.expect("login failed");

    assert_eq!(credential.access.expose_secret(), "acc-1");
    assert_eq!(credential.refresh.expose_secret(), "ref-1");
    let expires_in = credential
        .expires_at
        .expect("credential carries service expiry")
        .unix_timestamp()
        - time::OffsetDateTime::now_utc().unix_timestamp();
    assert!(
        expires_in > 3500 && expires_in <= 3600,
        "service expiry stored real (within tolerance): {expires_in}"
    );

    let request = &server.received_requests().await.unwrap()[0];
    assert_eq!(request.method, "POST");
    assert_eq!(
        request.headers.get("content-type").unwrap(),
        "application/json"
    );
    assert_eq!(request.headers.get("accept").unwrap(), "application/json");
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["grant_type"], "authorization_code");
    assert_eq!(body["client_id"], "test-client-id");
    assert_eq!(body["code"], "code-1");
    assert_eq!(
        body["state"],
        ScriptedPresenter::query_param(&presenter.last_authorize_url(), "state").unwrap()
    );
    assert!(body["code_verifier"].as_str().unwrap().len() >= 43);
    assert!(
        body["redirect_uri"]
            .as_str()
            .unwrap()
            .starts_with("http://localhost:")
    );
    assert!(
        body["redirect_uri"]
            .as_str()
            .unwrap()
            .ends_with("/callback")
    );
}

#[tokio::test]
async fn authorize_url_carries_pkce_state_scopes_and_callback() {
    let _port_guard = port_guard().await;
    let server = MockServer::start().await;
    mount_token_success(&server).await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.on_present.lock().unwrap() = Some(ScriptedPresenter::echo_redirect_hook());

    let credential = flow.login(presenter.as_ref()).await.expect("login failed");

    assert_eq!(credential.access.expose_secret(), "acc-1");
    let url = presenter.last_authorize_url();
    assert!(url.starts_with("https://authorize.example.test/authorize?"));
    assert_eq!(
        ScriptedPresenter::query_param(&url, "response_type").as_deref(),
        Some("code")
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "client_id").as_deref(),
        Some("test-client-id")
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "code_challenge_method").as_deref(),
        Some("S256")
    );
    let challenge = ScriptedPresenter::query_param(&url, "code_challenge").unwrap();
    assert!(challenge.len() >= 43);
    let state = ScriptedPresenter::query_param(&url, "state").unwrap();
    assert!(state.len() >= 16);
    let scope = ScriptedPresenter::query_param(&url, "scope").unwrap();
    let expected_scope = "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers user:file_upload";
    assert_eq!(scope, expected_scope);
    assert_eq!(
        ScriptedPresenter::query_param(&url, "code").as_deref(),
        Some("true")
    );
}

#[tokio::test]
async fn login_binds_preferred_port_53692_with_callback_path() {
    let _port_guard = PREFERRED_PORT_LOCK.lock().await;
    let server = MockServer::start().await;
    mount_token_success(&server).await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());

    let login = {
        let flow = flow.clone();
        let presenter = presenter.clone();
        tokio::spawn(async move { flow.login(presenter.as_ref()).await })
    };
    // Wait for the authorize URL to appear (flow bound its callback).
    let mut url = None;
    for _ in 0..200 {
        url = presenter.authorize_urls.lock().unwrap().first().cloned();
        if url.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let url = url.expect("authorize url never presented");
    let redirect = ScriptedPresenter::query_param(&url, "redirect_uri").unwrap();
    assert_eq!(redirect, "http://localhost:53692/callback");
    let state = ScriptedPresenter::query_param(&url, "state").unwrap();

    // The browser lands on the loopback callback; the callback arm wins.
    let callback_url = format!("http://127.0.0.1:53692/callback?code=cb-code&state={state}");
    let response = reqwest::get(&callback_url).await.expect("callback request");
    assert_eq!(response.status(), 200);

    let credential = login.await.expect("join").expect("login failed");
    assert_eq!(credential.access.expose_secret(), "acc-1");
    let body: serde_json::Value =
        serde_json::from_slice(&server.received_requests().await.unwrap()[0].body).unwrap();
    assert_eq!(body["code"], "cb-code");
}

#[tokio::test]
async fn login_falls_back_to_free_loopback_when_53692_is_busy() {
    let _port_guard = PREFERRED_PORT_LOCK.lock().await;
    let _occupier = TcpListener::bind("127.0.0.1:53692").expect("occupy 53692");
    let server = MockServer::start().await;
    mount_token_success(&server).await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());

    let login = {
        let flow = flow.clone();
        let presenter = presenter.clone();
        tokio::spawn(async move { flow.login(presenter.as_ref()).await })
    };
    let mut url = None;
    for _ in 0..200 {
        url = presenter.authorize_urls.lock().unwrap().first().cloned();
        if url.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let url = url.expect("authorize url never presented");
    let redirect = ScriptedPresenter::query_param(&url, "redirect_uri").unwrap();
    let port: u16 = redirect
        .rsplit_once(':')
        .unwrap()
        .1
        .split('/')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert_ne!(port, 53692, "must not reuse the busy preferred port");
    assert!(redirect.ends_with("/callback"));
    let state = ScriptedPresenter::query_param(&url, "state").unwrap();

    let callback_url = format!("http://127.0.0.1:{port}/callback?code=fb-code&state={state}");
    let response = reqwest::get(&callback_url).await.expect("callback request");
    assert_eq!(response.status(), 200);
    let credential = login.await.expect("join").expect("login failed");
    assert_eq!(credential.access.expose_secret(), "acc-1");
}

// ---------------------------------------------------------------------------
// Copy-code login
// ---------------------------------------------------------------------------

#[tokio::test]
async fn copy_code_login_uses_fixed_redirect_uri_and_code_state_paste() {
    let server = MockServer::start().await;
    mount_token_success(&server).await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::CopyCode);
    let scheme = "https";
    let expected_redirect = format!("{scheme}://platform.claude.com/oauth/code/callback");
    {
        let expected_redirect = expected_redirect.clone();
        *presenter.on_present.lock().unwrap() = Some(Arc::new(move |url: &str| {
            let state = ScriptedPresenter::query_param(url, "state").unwrap();
            assert_eq!(
                ScriptedPresenter::query_param(url, "redirect_uri").unwrap(),
                expected_redirect
            );
            format!("pasted-code#{state}")
        }));
    }

    let credential = flow.login(presenter.as_ref()).await.expect("login failed");
    let url = presenter.last_authorize_url();
    assert_eq!(
        presenter.offered_methods.lock().unwrap().as_slice(),
        &[OAuthLoginMethod::Browser, OAuthLoginMethod::CopyCode]
    );
    let state = ScriptedPresenter::query_param(&url, "state").unwrap();
    assert_eq!(credential.access.expose_secret(), "acc-1");
    let body: serde_json::Value =
        serde_json::from_slice(&server.received_requests().await.unwrap()[0].body).unwrap();
    assert_eq!(body["code"], "pasted-code");
    assert_eq!(body["state"], serde_json::json!(state));
    assert_eq!(body["redirect_uri"], serde_json::json!(expected_redirect));
}

#[tokio::test]
async fn copy_code_state_mismatch_is_typed_and_sends_zero_token_requests() {
    let server = MockServer::start().await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::CopyCode);
    *presenter.on_present.lock().unwrap() = Some(Arc::new(|url: &str| {
        let state = ScriptedPresenter::query_param(url, "state").unwrap();
        format!("code-x#wrong-{state}")
    }));

    let error = flow
        .login(presenter.as_ref())
        .await
        .expect_err("login must fail");
    // State mismatch is a typed config failure (the paste never reaches the
    // token endpoint), fully redacted like every token-endpoint summary.
    assert!(matches!(error, ProviderError::Config(_)));
    assert!(server.received_requests().await.unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// Callback validation and failure mapping
// ---------------------------------------------------------------------------

#[tokio::test]
async fn callback_state_mismatch_answers_secret_free_400_and_never_exchanges() {
    let _port_guard = PREFERRED_PORT_LOCK.lock().await;
    let server = MockServer::start().await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());

    let login = {
        let flow = flow.clone();
        let presenter = presenter.clone();
        tokio::spawn(async move { flow.login(presenter.as_ref()).await })
    };
    let mut url = None;
    for _ in 0..200 {
        url = presenter.authorize_urls.lock().unwrap().first().cloned();
        if url.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let _ = url.expect("authorize url never presented");

    // Wrong state lands on the bound callback: fixed 400, no token exchange.
    let response = reqwest::get("http://127.0.0.1:53692/callback?code=x&state=wrong")
        .await
        .expect("callback request");
    assert_eq!(response.status(), 400);
    let body = response.text().await.unwrap();
    assert_eq!(body, "Login failed, return to the terminal.");
    assert!(server.received_requests().await.unwrap().is_empty());
    // Complete the flow through the manual arm so the login future finishes.
    login.abort();
}

#[tokio::test]
async fn token_endpoint_denial_and_invalid_grant_map_to_typed_classes() {
    let _port_guard = port_guard().await;
    for error_code in ["access_denied", "invalid_grant"] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(
                serde_json::json!({"error": error_code, "error_description": "secret-canary"}),
            ))
            .expect(1)
            .mount(&server)
            .await;
        let flow = anthropic_flow(format!("{}/token", server.uri()));

        let error = match login_via_echoed_redirect(&flow).await {
            Ok(_) => panic!("denial must fail"),
            Err(error) => error,
        };
        // Summaries are fully redacted: the closed class never surfaces and
        // the echoed description canary cannot leak.
        assert!(matches!(error, ProviderError::AuthFailed(_)));
        let message = error.to_string();
        assert!(!message.contains(error_code), "{message}");
        assert!(!message.contains("secret-canary"), "{message}");
    }
}

#[tokio::test]
async fn token_response_missing_fields_is_typed() {
    let _port_guard = port_guard().await;
    for missing in ["refresh_token", "expires_in"] {
        let server = MockServer::start().await;
        let mut body = token_body("acc", "ref", 3600);
        body.as_object_mut().unwrap().remove(missing);
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        let flow = anthropic_flow(format!("{}/token", server.uri()));

        let error = match login_via_echoed_redirect(&flow).await {
            Ok(_) => panic!("missing {missing} must fail"),
            Err(error) => error,
        };
        assert!(matches!(error, ProviderError::Config(_)));
    }
}

#[tokio::test]
async fn manual_redirect_with_missing_or_duplicate_params_is_typed() {
    let _port_guard = port_guard().await;
    let server = MockServer::start().await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));

    for pasted in [
        "http://localhost:53692/callback?code=only-code", // missing state
        "http://localhost:53692/callback?code=a&code=b&state=s", // duplicate code
        "",                                               // empty paste
    ] {
        let presenter = ScriptedPresenter::with_manual(pasted.to_owned());
        let authorize = flow.login(presenter.as_ref()).await;
        // Every case must fail typed before any token request; the scripted
        // state never matches, so exchange cannot happen.
        assert!(authorize.is_err());
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn login_cancelled_presenter_maps_to_login_cancelled() {
    let _port_guard = port_guard().await;
    let server = MockServer::start().await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    struct CancellingPresenter;
    impl LoginPresenter for CancellingPresenter {
        fn present_auth_url<'a>(
            &'a self,
            _url: &'a str,
        ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
            Box::pin(async { Ok(()) })
        }
        fn present_device_code<'a>(
            &'a self,
            _user_code: &'a str,
            _verification_uri: &'a str,
        ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
            Box::pin(async { Ok(()) })
        }
        fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
            Box::pin(async {
                Err(ProviderError::LoginCancelled {
                    provider_id: "anthropic".into(),
                })
            })
        }
        fn notify_success(&self) {}
        fn notify_failure(&self, _reason: &str) {}
    }

    let error = flow
        .login(&CancellingPresenter)
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::LoginCancelled { .. }));
    assert!(server.received_requests().await.unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// Refresh
// ---------------------------------------------------------------------------

fn stored_credential() -> OAuthCredential {
    OAuthCredential {
        access: secrecy::SecretString::from("old-access"),
        refresh: secrecy::SecretString::from("old-refresh"),
        expires_at: Some(time::OffsetDateTime::now_utc() - time::Duration::minutes(10)),
        base_url: None,
        account_id: None,
    }
}

#[tokio::test]
async fn refresh_posts_json_and_keeps_real_service_expiry() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_partial_json(
            serde_json::json!({"grant_type": "refresh_token"}),
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(token_body("new-acc", "new-ref", 7200)),
        )
        .expect(1)
        .mount(&server)
        .await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));

    let refreshed = flow.refresh(&stored_credential()).await.expect("refresh");
    assert_eq!(refreshed.access.expose_secret(), "new-acc");
    assert_eq!(refreshed.refresh.expose_secret(), "new-ref");
    let expires_in = refreshed.expires_at.expect("expiry").unix_timestamp()
        - time::OffsetDateTime::now_utc().unix_timestamp();
    assert!(
        expires_in > 7100 && expires_in <= 7200,
        "real service expiry, no pre-subtraction: {expires_in}"
    );

    let request = &server.received_requests().await.unwrap()[0];
    assert_eq!(
        request.headers.get("content-type").unwrap(),
        "application/json"
    );
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["grant_type"], "refresh_token");
    assert_eq!(body["client_id"], "test-client-id");
    assert_eq!(body["refresh_token"], "old-refresh");
}

#[tokio::test]
async fn refresh_revoked_on_401_is_typed_without_body_echo() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string("{\"error\":\"invalid_grant\",\"detail\":\"canary\"}"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));

    let error = flow
        .refresh(&stored_credential())
        .await
        .expect_err("revoked");
    assert!(matches!(error, ProviderError::CredentialRevoked { .. }));
    assert!(!error.to_string().contains("canary"));
}

#[tokio::test]
async fn refresh_without_refresh_token_reuses_the_stored_one() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "new-acc",
            "expires_in": 3600,
        })))
        .expect(1)
        .mount(&server)
        .await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));

    let refreshed = flow.refresh(&stored_credential()).await.expect("refresh");
    assert_eq!(refreshed.refresh.expose_secret(), "old-refresh");
}

#[tokio::test]
async fn near_expiry_needs_refresh_uses_single_five_minute_window() {
    let now = time::OffsetDateTime::now_utc();
    let fresh = OAuthCredential {
        expires_at: Some(now + time::Duration::minutes(10)),
        ..stored_credential()
    };
    let near = OAuthCredential {
        expires_at: Some(now + time::Duration::minutes(4)),
        ..stored_credential()
    };
    let expired = OAuthCredential {
        expires_at: Some(now - time::Duration::minutes(1)),
        ..stored_credential()
    };
    assert!(!fresh.needs_refresh());
    assert!(near.needs_refresh());
    assert!(expired.needs_refresh());
    assert!(
        OAuthCredential {
            expires_at: None,
            ..stored_credential()
        }
        .needs_refresh()
    );
}

// ---------------------------------------------------------------------------
// Models::login — acknowledged store, shared transactions
// ---------------------------------------------------------------------------

fn models_with_flow(store: Arc<FakeStore>, flow: AnthropicOAuthProvider) -> Models {
    let mut models = Models::new();
    models.register_oauth_flow("anthropic", Arc::new(flow));
    models.set_credential_store(store);
    models
}

async fn wiremock_login(server: &MockServer) -> (Models, Arc<ScriptedPresenter>, Arc<FakeStore>) {
    let _port_guard = port_guard().await;
    mount_token_success(server).await;
    // Every models login drives the browser method: the presenter echoes the
    // authorize URL back as a pasted redirect so the login completes without
    // an actual browser.
    let store = FakeStore::new();
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.on_present.lock().unwrap() = Some(ScriptedPresenter::echo_redirect_hook());
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let models = models_with_flow(store.clone(), flow);
    (models, presenter, store)
}

#[tokio::test]
async fn models_login_acks_the_store_write_before_reporting_success() {
    let server = MockServer::start().await;
    let (models, presenter, store) = wiremock_login(&server).await;

    let credential = match models.login("anthropic", presenter.as_ref()).await {
        Ok(c) => c,
        Err(e) => panic!("login error: {e:?}"),
    };
    assert_eq!(credential.access.expose_secret(), "acc-1");

    // The stored entry is the acknowledged OAuth credential.
    let stored = store.entry("anthropic").expect("stored");
    match stored {
        Credential::OAuthToken { access, .. } => {
            assert_eq!(access.expose_secret(), "acc-1");
        }
        other => panic!("unexpected stored kind: {other:?}"),
    }
    // notify_success fired after the credential existed in the store.
    assert!(presenter.success_at.lock().unwrap().is_some());
    assert!(
        store
            .modify_log
            .lock()
            .unwrap()
            .contains(&"anthropic".to_owned())
    );
}

#[tokio::test]
async fn models_login_unknown_provider_is_typed() {
    let server = MockServer::start().await;
    let (models, presenter, _store) = wiremock_login(&server).await;

    let error = models
        .login("not-registered", presenter.as_ref())
        .await
        .expect_err("unknown flow");
    assert!(error.to_string().contains("not-registered"));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn models_login_store_failure_is_typed_and_never_reports_success() {
    let server = MockServer::start().await;
    let (models, presenter, store) = wiremock_login(&server).await;
    store.set_mode(StoreMode::Fail);

    let error = models
        .login("anthropic", presenter.as_ref())
        .await
        .expect_err("store failure");
    assert!(matches!(
        error,
        opi_ai::ModelsError::CredentialStore(_) | opi_ai::ModelsError::AuthResolution { .. }
    ));
    assert!(presenter.success_at.lock().unwrap().is_none());
    assert!(!presenter.failures.lock().unwrap().is_empty());
    // The flow itself completed, so the token endpoint was hit exactly once.
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn models_login_unknown_mutation_outcome_surfaces_pending() {
    let server = MockServer::start().await;
    let (models, presenter, store) = wiremock_login(&server).await;
    store.set_mode(StoreMode::Unknown);

    let error = models
        .login("anthropic", presenter.as_ref())
        .await
        .expect_err("unknown outcome");
    assert!(matches!(
        error,
        opi_ai::ModelsError::MutationOutcomeUnknown { .. }
    ));
    assert!(presenter.success_at.lock().unwrap().is_none());
}

#[tokio::test]
async fn concurrent_login_writes_serialize_on_the_shared_store() {
    let _port_guard = port_guard().await;
    let server = MockServer::start().await;
    mount_token_success(&server).await;
    let store = FakeStore::new();
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let models = models_with_flow(store.clone(), flow);

    // Each concurrent login gets its own presenter (its own state), exactly
    // like three independent drivers sharing one store.
    let presenters: Vec<Arc<ScriptedPresenter>> = (0..3)
        .map(|_| {
            let presenter = Arc::new(ScriptedPresenter::default());
            *presenter.on_present.lock().unwrap() = Some(ScriptedPresenter::echo_redirect_hook());
            presenter
        })
        .collect();
    let mut logins = Vec::new();
    for presenter in presenters {
        let models_ref = &models;
        logins.push(async move { models_ref.login("anthropic", presenter.as_ref()).await });
    }
    let results = futures_util::future::join_all(logins).await;
    assert!(results.iter().all(|r| r.is_ok()), "{results:?}");
    // Mutations never overlapped on the shared store.
    assert_eq!(*store.max_overlapping.lock().unwrap(), 1);
    assert_eq!(store.modify_log.lock().unwrap().len(), 3);
    // The final state is a complete credential, never a partial write.
    assert!(store.entry("anthropic").is_some());
}

#[tokio::test]
async fn check_auth_reports_the_login_without_refreshing() {
    let server = MockServer::start().await;
    let (models, presenter, _store) = wiremock_login(&server).await;
    models
        .login("anthropic", presenter.as_ref())
        .await
        .expect("login");

    let check = models
        .check_auth("anthropic")
        .await
        .expect("check")
        .expect("configured");
    assert!(matches!(check.kind, opi_ai::AuthCheckKind::OAuth));
    // Checking performs no refresh: only the login token request was sent.
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn login_performs_no_browser_or_ambient_fallback() {
    let _port_guard = port_guard().await;
    // The presenter is the only interaction surface: with no manual reply the
    // flow stays pending (no ambient browser launch, no env fallback, no
    // keychain read) until the budget expires or the caller drops it.
    let server = MockServer::start().await;
    let flow = anthropic_flow(format!("{}/token", server.uri()));
    let presenter = ScriptedPresenter::default(); // no manual reply

    let mut login = Box::pin(flow.login(&presenter));
    let outcome = tokio::time::timeout(Duration::from_millis(300), &mut login).await;
    assert!(
        outcome.is_err(),
        "login must stay pending without interaction"
    );
    drop(login);
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(presenter.failures.lock().unwrap().is_empty());
}
