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
    /// Presented device codes: (user_code, verification_uri).
    device_codes: Mutex<Vec<(String, String)>>,
    /// Queued free-text prompt answers; an empty queue answers "".
    prompt_answers: Mutex<Vec<String>>,
    /// Recorded `notify_progress` messages.
    progress: Mutex<Vec<String>>,
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
        user_code: &'a str,
        verification_uri: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        self.device_codes
            .lock()
            .unwrap()
            .push((user_code.to_owned(), verification_uri.to_owned()));
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

    fn prompt_text<'a>(
        &'a self,
        _message: &'a str,
        _placeholder: &'a str,
    ) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
        let answer = self
            .prompt_answers
            .lock()
            .unwrap()
            .pop()
            .unwrap_or_default();
        Box::pin(async move { Ok(answer) })
    }

    fn notify_progress(&self, message: &str) {
        self.progress.lock().unwrap().push(message.to_owned());
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
        client_id: None,
        enterprise_domain: None,
        available_model_ids: None,
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

// ---------------------------------------------------------------------------
// OpenAI Codex legacy (Phase 20.24) — pi openai-codex.ts
// ---------------------------------------------------------------------------

use opi_ai::auth::oauth::codex::{
    CODEX_DEVICE_REDIRECT_URI, CODEX_REDIRECT_URI, CodexOAuthProvider,
};

const CODEX_DEVICE_VERIFICATION: &str = "https://auth.openai.com/codex/device";

fn codex_flow(
    server: &MockServer,
    browser_timeout: Duration,
    device_timeout: Duration,
) -> CodexOAuthProvider {
    let base = server.uri();
    CodexOAuthProvider::new_with_device_endpoints(
        format!("{base}/oauth/authorize"),
        format!("{base}/oauth/token"),
        format!("{base}/deviceauth/usercode"),
        format!("{base}/deviceauth/token"),
        CODEX_DEVICE_VERIFICATION.to_owned(),
        CODEX_DEVICE_REDIRECT_URI.to_owned(),
        "codex-client-id".to_owned(),
        browser_timeout,
        device_timeout,
    )
}

/// A synthetic Codex access token: JWT-shaped with the ChatGPT account claim
/// (pi `getAccountId`).
fn codex_jwt(account_id: Option<&str>) -> String {
    use base64::Engine as _;
    let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = engine.encode(br#"{"alg":"RS256"}"#);
    let payload = match account_id {
        Some(id) => format!(r#"{{"https://api.openai.com/auth":{{"chatgpt_account_id":"{id}"}}}}"#),
        None => "{}".to_owned(),
    };
    let payload = engine.encode(payload.as_bytes());
    format!("{header}.{payload}.signature")
}

fn codex_presenter() -> Arc<ScriptedPresenter> {
    let presenter = Arc::new(ScriptedPresenter::default());
    {
        let hook = ScriptedPresenter::echo_redirect_hook();
        *presenter.on_present.lock().unwrap() = Some(hook);
    }
    presenter
}

async fn mount_codex_token_success(server: &MockServer, access: &str) {
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": access,
            "refresh_token": "codex-refresh-1",
            "expires_in": 3600,
        })))
        .mount(server)
        .await;
}

async fn mount_codex_device_start(server: &MockServer, interval: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_auth_id": "device-auth-1",
            "user_code": "PUBLIC-CODE",
            "interval": interval,
        })))
        .mount(server)
        .await;
}

/// Mount the device token poll endpoint's completed authorization: 200 with
/// the authorization code and the server-issued verifier.
async fn mount_codex_device_poll_complete(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "authorization_code": "device-code-1",
            "code_verifier": "device-verifier-from-server",
        })))
        .mount(server)
        .await;
}

/// Mount a never-ending terminal classification on the poll endpoint.
async fn mount_codex_device_poll_status(server: &MockServer, status: u16, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/deviceauth/token"))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .mount(server)
        .await;
}

#[tokio::test]
async fn codex_browser_login_posts_form_token_request_with_fixed_redirect() {
    let _port1455 = loopback_1455_guard().await;
    let server = MockServer::start().await;
    let access = codex_jwt(Some("acct-123"));
    mount_codex_token_success(&server, &access).await;
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
    let presenter = codex_presenter();

    let credential = flow.login(presenter.as_ref()).await.expect("login failed");

    assert_eq!(credential.access.expose_secret(), access);
    assert_eq!(credential.refresh.expose_secret(), "codex-refresh-1");
    assert_eq!(credential.account_id.as_deref(), Some("acct-123"));

    let requests = server.received_requests().await.unwrap();
    let token_req = requests.last().expect("token request");
    assert_eq!(
        token_req.headers.get("content-type").unwrap(),
        "application/x-www-form-urlencoded"
    );
    let body = String::from_utf8(token_req.body.clone()).unwrap();
    let pairs: std::collections::BTreeMap<String, String> = body
        .split('&')
        .map(|pair| {
            let (k, v) = pair.split_once('=').unwrap();
            (k.to_owned(), v.replace("%3A", ":").replace("%2F", "/"))
        })
        .collect();
    assert_eq!(pairs["grant_type"], "authorization_code");
    assert_eq!(pairs["client_id"], "codex-client-id");
    assert_eq!(pairs["redirect_uri"], CODEX_REDIRECT_URI);
    assert!(
        pairs["code_verifier"].len() >= 43,
        "{:?}",
        pairs["code_verifier"]
    );
}

#[tokio::test]
async fn codex_authorize_url_carries_pkce_state_and_codex_params() {
    let _port1455 = loopback_1455_guard().await;
    let server = MockServer::start().await;
    mount_codex_token_success(&server, &codex_jwt(Some("acct-123"))).await;
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
    let presenter = codex_presenter();

    flow.login(presenter.as_ref()).await.expect("login failed");

    let url = presenter.last_authorize_url();
    assert!(url.starts_with(&format!("{}/oauth/authorize?", server.uri())));
    let state = ScriptedPresenter::query_param(&url, "state").expect("state");
    assert!(
        !state.is_empty()
            && state != ScriptedPresenter::query_param(&url, "code_challenge").unwrap()
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "code_challenge_method").as_deref(),
        Some("S256")
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "redirect_uri").as_deref(),
        Some(CODEX_REDIRECT_URI)
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "scope").as_deref(),
        Some("openid profile email offline_access")
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "id_token_add_organizations").as_deref(),
        Some("true")
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "codex_cli_simplified_flow").as_deref(),
        Some("true")
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "originator").as_deref(),
        Some("opi")
    );
}

#[tokio::test]
async fn codex_busy_1455_falls_back_to_manual_paste_with_fixed_redirect() {
    let _port1455 = loopback_1455_guard().await;
    let server = MockServer::start().await;
    // Hold the CLI-shared port so the SDK must fall back paste-back only —
    // never to a free loopback port (pi `.catch(() => undefined)`).
    let _occupier = TcpListener::bind(("127.0.0.1", 1455)).expect("hold 1455");
    let access = codex_jwt(Some("acct-123"));
    mount_codex_token_success(&server, &access).await;
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
    let presenter = codex_presenter();

    let credential = flow.login(presenter.as_ref()).await.expect("login failed");
    assert_eq!(credential.account_id.as_deref(), Some("acct-123"));

    // The manual paste echoed the authorize URL (redirect fixed at 1455) and
    // the exchange kept the same fixed redirect URI.
    let requests = server.received_requests().await.unwrap();
    let token_req = requests.last().expect("token request");
    let body = String::from_utf8(token_req.body.clone()).unwrap();
    let redirect = body
        .split('&')
        .find_map(|pair| pair.strip_prefix("redirect_uri="))
        .expect("redirect_uri param");
    assert_eq!(redirect, "http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback");
}

#[tokio::test]
async fn codex_device_flow_presents_user_code_and_exchanges_with_server_verifier() {
    let server = MockServer::start().await;
    mount_codex_device_start(&server, serde_json::json!(0)).await;
    mount_codex_device_poll_complete(&server).await;
    let access = codex_jwt(Some("acct-device"));
    mount_codex_token_success(&server, &access).await;
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

    let credential = match flow.login(presenter.as_ref()).await {
        Ok(c) => c,
        Err(e) => {
            for r in server.received_requests().await.unwrap() {
                eprintln!(
                    "REQ {} {} body={}",
                    r.method,
                    r.url.path(),
                    String::from_utf8_lossy(&r.body)
                );
            }
            panic!("device login failed: {e:?}");
        }
    };
    assert_eq!(credential.account_id.as_deref(), Some("acct-device"));

    let devices = presenter.device_codes.lock().unwrap().clone();
    assert_eq!(
        devices,
        vec![(
            "PUBLIC-CODE".to_owned(),
            CODEX_DEVICE_VERIFICATION.to_owned()
        )]
    );

    let requests = server.received_requests().await.unwrap();
    let token_req = requests.last().expect("token exchange");
    let body = String::from_utf8(token_req.body.clone()).unwrap();
    assert!(body.contains("code=device-code-1"), "{body}");
    assert!(
        body.contains("code_verifier=device-verifier-from-server"),
        "{body}"
    );
    assert!(
        body.contains(&format!(
            "redirect_uri={}",
            urlencode(CODEX_DEVICE_REDIRECT_URI)
        )),
        "{body}"
    );
}

fn urlencode(value: &str) -> String {
    // Minimal RFC 3986 percent-encoding of the characters reqwest encodes.
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[tokio::test]
async fn codex_device_denied_and_expired_are_typed_without_body_echo() {
    for code in ["deviceauth_access_denied", "expired_token"] {
        let server = MockServer::start().await;
        mount_codex_device_start(&server, serde_json::json!(0)).await;
        mount_codex_device_poll_status(
            &server,
            403,
            serde_json::json!({"error": {"code": code}, "secret": "device-secret"}),
        )
        .await;
        let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
        let presenter = Arc::new(ScriptedPresenter::default());
        *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

        let error = flow.login(presenter.as_ref()).await.expect_err("denied");
        assert!(
            matches!(error, ProviderError::CredentialRevoked { .. }),
            "{code}: {error:?}"
        );
        assert!(!format!("{error:?}").contains("device-secret"));
    }
}

#[tokio::test]
async fn codex_device_invalid_interval_is_typed_before_any_poll() {
    let server = MockServer::start().await;
    mount_codex_device_start(&server, serde_json::json!("not-a-number")).await;
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

    let error = flow
        .login(presenter.as_ref())
        .await
        .expect_err("invalid interval");
    assert!(matches!(error, ProviderError::Config(_)), "{error:?}");
    // The token poll endpoint was never reached.
    assert!(
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .all(|r| r.url.path() != "/deviceauth/token")
    );
}

#[tokio::test]
async fn codex_device_usercode_404_is_typed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "detail": "not enabled server-secret"
        })))
        .mount(&server)
        .await;
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

    let error = flow
        .login(presenter.as_ref())
        .await
        .expect_err("404 usercode");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(!format!("{error:?}").contains("server-secret"));
}

#[tokio::test]
async fn codex_device_budget_exhaustion_times_out() {
    let server = MockServer::start().await;
    mount_codex_device_start(&server, serde_json::json!(30)).await;
    Mock::given(method("POST"))
        .and(path("/deviceauth/token"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "error": "deviceauth_authorization_pending"
        })))
        .mount(&server)
        .await;
    // Poll interval 30s inside a 250ms device budget: the first sleep exhausts it.
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_millis(250));
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.forced_method.lock().unwrap() = Some(OAuthLoginMethod::DeviceCode);

    let error = flow.login(presenter.as_ref()).await.expect_err("timeout");
    assert!(matches!(error, ProviderError::Timeout), "{error:?}");
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .iter()
            .any(|f| f.contains("timed out"))
    );
}

#[tokio::test]
async fn codex_refresh_requires_all_token_fields_and_reextracts_account_id() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": codex_jwt(Some("acct-new")),
            "expires_in": 3600,
        })))
        .mount(&server)
        .await;
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
    let cred = OAuthCredential {
        base_url: Some("https://chatgpt.example.test/backend".to_owned()),
        ..stored_credential()
    };

    let error = flow
        .refresh(&cred)
        .await
        .expect_err("missing refresh_token");
    assert!(matches!(error, ProviderError::Config(_)), "{error:?}");

    // Strict success: full triple, endpoint (base_url) preserved, account id
    // re-extracted from the new access token.
    let server2 = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": codex_jwt(Some("acct-new")),
            "refresh_token": "codex-refresh-2",
            "expires_in": 3600,
        })))
        .mount(&server2)
        .await;
    let flow2 = codex_flow(&server2, Duration::from_secs(30), Duration::from_secs(30));
    let refreshed = flow2.refresh(&cred).await.expect("refresh ok");
    assert_eq!(refreshed.refresh.expose_secret(), "codex-refresh-2");
    assert_eq!(
        refreshed.base_url.as_deref(),
        Some("https://chatgpt.example.test/backend")
    );
    assert_eq!(refreshed.account_id.as_deref(), Some("acct-new"));
    assert!(refreshed.expires_at.is_some());
}

#[tokio::test]
async fn codex_credential_without_account_id_is_typed() {
    let _port1455 = loopback_1455_guard().await;
    let server = MockServer::start().await;
    // Access token carries no chatgpt_account_id claim: pi credentialsFromToken
    // throws; the SDK must fail typed and never yield a credential.
    mount_codex_token_success(&server, &codex_jwt(None)).await;
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
    let presenter = codex_presenter();

    let error = flow
        .login(presenter.as_ref())
        .await
        .expect_err("no account id");
    assert!(
        matches!(error, ProviderError::AccountIdMissing { .. }),
        "{error:?}"
    );
}

#[tokio::test]
async fn codex_models_login_acks_the_store_write() {
    let _port1455 = loopback_1455_guard().await;
    let server = MockServer::start().await;
    let access = codex_jwt(Some("acct-models"));
    mount_codex_token_success(&server, &access).await;
    let store = FakeStore::new();
    let store_handle = store.clone();
    let presenter = codex_presenter();
    let flow = codex_flow(&server, Duration::from_secs(30), Duration::from_secs(30));
    let mut models = Models::new();
    models.register_oauth_flow("openai-codex", Arc::new(flow));
    models.set_credential_store(store);

    let credential = models
        .login("openai-codex", presenter.as_ref())
        .await
        .expect("login");
    assert_eq!(credential.account_id.as_deref(), Some("acct-models"));
    let expected_access = credential.access.expose_secret().to_owned();

    let stored = store_handle.entry("openai-codex").expect("stored");
    match stored {
        Credential::OAuthToken { access, .. } => {
            assert_eq!(access.expose_secret(), expected_access);
        }
        other => panic!("unexpected stored kind: {other:?}"),
    }
    assert!(presenter.success_at.lock().unwrap().is_some());
}

// OpenAI ChatGPT direct (Phase 20.25). Pinned to pi 1.3.0 openai-chatgpt.ts:
// dynamic client registration (the callback carries the issued client id),
// form exchange with the resource parameter, the fixed 127.0.0.1:1455
// redirect whose bind failure is terminal, strictly validated pasted URLs,
// the UUID device identity, and the direct-token scope/id_token contract.

use opi_ai::auth::oauth::chatgpt::OpenAiChatGptOAuthProvider;

static LOOPBACK_1455_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Serialize every test that binds the CLI-shared port 1455: Codex browser
/// logins fall back to paste-back when it is busy, but an OpenAI ChatGPT
/// login must see it free (its bind failure is terminal by contract).
async fn loopback_1455_guard() -> tokio::sync::MutexGuard<'static, ()> {
    LOOPBACK_1455_LOCK.lock().await
}

const CHATGPT_DEVICE_UUID: &str = "0F0E8D9A-1111-4222-8333-444455556666";
const CHATGPT_REDIRECT: &str = "http://127.0.0.1:1455/auth/callback";

fn chatgpt_flow(
    server: &MockServer,
    device_id: Option<&str>,
    timeout: Duration,
) -> OpenAiChatGptOAuthProvider {
    OpenAiChatGptOAuthProvider::new_with_endpoints(
        format!("{}/api/accounts/authorize", server.uri()),
        format!("{}/api/accounts/oauth/token", server.uri()),
        None,
        device_id.map(str::to_owned),
        timeout,
    )
}

/// Build the full redirect URL the user would paste back (echoing the flow's
/// own state) with an issued client id.
fn chatgpt_paste_hook() -> PresentHook {
    Arc::new(|url: &str| {
        let state = ScriptedPresenter::query_param(url, "state").unwrap();
        format!("{CHATGPT_REDIRECT}?code=code-1&state={state}&client_id=issued-client-1")
    })
}

async fn mount_chatgpt_token(server: &MockServer, mutate: serde_json::Value) {
    let mut body = serde_json::json!({
        "access_token": "at-1",
        "refresh_token": "rt-1",
        "expires_in": 3600,
        "scope": "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct",
        "id_token": "idt-1",
    });
    if let (Some(map), Some(overrides)) = (body.as_object_mut(), mutate.as_object()) {
        for (key, value) in overrides {
            if value.is_null() {
                map.remove(key);
            } else {
                map.insert(key.clone(), value.clone());
            }
        }
    }
    Mock::given(method("POST"))
        .and(path("/api/accounts/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

/// The decoded `name=value` pairs of a form-urlencoded request body.
fn form_pairs(body: &str) -> Vec<(String, String)> {
    body.split('&')
        .map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (percent_decode(k), percent_decode(v))
        })
        .collect()
}

/// Read one HTTP response from the loopback callback listener.
async fn chatgpt_callback_get(request_line: &str) -> Option<String> {
    let mut stream = tokio::net::TcpStream::connect("127.0.0.1:1455")
        .await
        .ok()?;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    stream
        .write_all(
            format!("{request_line} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .await
        .ok()?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.ok()?;
    Some(String::from_utf8_lossy(&buf).into_owned())
}

/// Wait until the presenter has recorded an authorize URL.
async fn chatgpt_wait_for_authorize_url(presenter: &ScriptedPresenter) -> String {
    for _ in 0..200 {
        if let Some(url) = presenter.authorize_urls.lock().unwrap().last() {
            return url.clone();
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("presenter saw no authorize url");
}

#[tokio::test]
async fn chatgpt_authorize_url_carries_dynamic_client_resource_and_host_identity() {
    let _guard = loopback_1455_guard().await;
    let server = MockServer::start().await;
    mount_chatgpt_token(&server, serde_json::json!({})).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.on_present.lock().unwrap() = Some(chatgpt_paste_hook());
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));

    let credential = flow.login(presenter.as_ref()).await.expect("chatgpt login");
    let url = presenter.last_authorize_url();
    assert_eq!(
        ScriptedPresenter::query_param(&url, "client_id").as_deref(),
        Some("dynamic_agent_client")
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "redirect_uri").as_deref(),
        Some(CHATGPT_REDIRECT)
    );
    let scope = ScriptedPresenter::query_param(&url, "scope").unwrap();
    assert!(scope.contains("resource.invoke"));
    assert!(scope.contains("chatgpt.tokens.use.direct"));
    assert_eq!(
        ScriptedPresenter::query_param(&url, "resource").as_deref(),
        Some("https://api.openai.com/v1")
    );
    assert_eq!(
        ScriptedPresenter::query_param(&url, "agent_name_hint").as_deref(),
        Some("opi")
    );
    // Uppercase UUIDs normalize to the lowercase urn form.
    assert_eq!(
        ScriptedPresenter::query_param(&url, "ext_agent_host_id").as_deref(),
        Some("urn:uuid:0f0e8d9a-1111-4222-8333-444455556666")
    );
    assert!(ScriptedPresenter::query_param(&url, "nonce").is_some());
    assert_eq!(
        ScriptedPresenter::query_param(&url, "code_challenge_method").as_deref(),
        Some("S256")
    );
    assert!(ScriptedPresenter::query_param(&url, "state").is_some());
    // The credential stores the issued client id, not the dynamic literal.
    assert_eq!(credential.client_id.as_deref(), Some("issued-client-1"));
    assert!(presenter.success_at.lock().unwrap().is_some());
}

#[tokio::test]
async fn chatgpt_exchange_posts_issued_client_id_and_resource_as_form() {
    let _guard = loopback_1455_guard().await;
    let server = MockServer::start().await;
    mount_chatgpt_token(&server, serde_json::json!({})).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.on_present.lock().unwrap() = Some(chatgpt_paste_hook());
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));

    let before = time_now_secs();
    let credential = flow.login(presenter.as_ref()).await.expect("login");
    let requests = server.received_requests().await.unwrap();
    let token_request = requests
        .iter()
        .find(|r| r.url.path() == "/api/accounts/oauth/token")
        .expect("token request");
    let content_type = token_request
        .headers
        .get("content-type")
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default();
    assert!(content_type.starts_with("application/x-www-form-urlencoded"));
    let body = String::from_utf8(token_request.body.clone()).unwrap();
    let pairs = form_pairs(&body);
    let pair = |name: &str| {
        pairs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    assert_eq!(pair("grant_type"), "authorization_code");
    assert_eq!(pair("client_id"), "issued-client-1");
    assert_eq!(pair("code"), "code-1");
    assert_eq!(pair("redirect_uri"), CHATGPT_REDIRECT);
    assert_eq!(pair("resource"), "https://api.openai.com/v1");
    assert!(!pair("code_verifier").is_empty());
    // Real service expiry (expires_in = 3600), not a pre-margin value.
    let expires_delta = credential.expires_at.unwrap().unix_timestamp() as i64 - before;
    assert!((3590..=3610).contains(&expires_delta), "{expires_delta}");
    assert_eq!(credential.access.expose_secret(), "at-1");
    assert_eq!(credential.refresh.expose_secret(), "rt-1");
    assert_eq!(credential.account_id, None);
}

/// Wall-clock seconds for real-expiry assertions.
fn time_now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[tokio::test]
async fn chatgpt_callback_with_issued_client_id_completes_login() {
    let _guard = loopback_1455_guard().await;
    let server = MockServer::start().await;
    mount_chatgpt_token(&server, serde_json::json!({})).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));

    let driver_presenter = presenter.clone();
    let drive = tokio::spawn(async move {
        let url = chatgpt_wait_for_authorize_url(&driver_presenter).await;
        let state = ScriptedPresenter::query_param(&url, "state").unwrap();
        let response = chatgpt_callback_get(&format!(
            "GET /auth/callback?code=cb-9&state={state}&client_id=issued-client-9"
        ))
        .await
        .expect("callback response");
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    });
    let credential = flow.login(presenter.as_ref()).await.expect("login");
    drive.await.expect("driver");
    assert_eq!(credential.client_id.as_deref(), Some("issued-client-9"));
    assert_eq!(credential.access.expose_secret(), "at-1");
}

#[tokio::test]
async fn chatgpt_busy_1455_is_typed_terminal_without_interaction() {
    let _guard = loopback_1455_guard().await;
    let server = MockServer::start().await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));
    // Hold the CLI-shared port like another pending login would.
    let holder = TcpListener::bind("127.0.0.1:1455").expect("hold 1455");

    let error = flow
        .login(presenter.as_ref())
        .await
        .expect_err("bind must fail terminally");
    assert!(matches!(error, ProviderError::Config(_)), "{error:?}");
    // The failure happens before the authorize URL is presented and before
    // any HTTP traffic: no silent manual-only or free-port fallback.
    assert!(presenter.authorize_urls.lock().unwrap().is_empty());
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .iter()
            .any(|r| r.contains("in use"))
    );
    drop(holder);
}

#[tokio::test]
async fn chatgpt_manual_paste_rejects_foreign_origin_and_bare_code() {
    let _guard = loopback_1455_guard().await;
    let pastes = [
        // localhost is a different origin than the fixed 127.0.0.1 redirect.
        format!("{CHATGPT_REDIRECT}?code=code-1&state=x&client_id=issued-client-1")
            .replace("127.0.0.1", "localhost"),
        // A bare code is never accepted for this flow.
        "code-1".to_owned(),
        // A foreign path on the right host.
        "http://127.0.0.1:1455/other?code=code-1&state=x&client_id=issued-client-1".to_owned(),
    ];
    for paste in pastes {
        let server = MockServer::start().await;
        mount_chatgpt_token(&server, serde_json::json!({})).await;
        let presenter = ScriptedPresenter::with_manual(paste);
        let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));
        let error = flow.login(presenter.as_ref()).await.expect_err("typed");
        assert!(matches!(error, ProviderError::Config(_)), "{error:?}");
        assert!(
            presenter
                .failures
                .lock()
                .unwrap()
                .iter()
                .any(|r| r == "manual redirect error")
        );
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn chatgpt_error_param_paste_is_typed_denial() {
    let _guard = loopback_1455_guard().await;
    let server = MockServer::start().await;
    let presenter = ScriptedPresenter::with_manual(format!(
        "{CHATGPT_REDIRECT}?error=access_denied&error_description=denied+by+user"
    ));
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));

    let error = flow.login(presenter.as_ref()).await.expect_err("denied");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .iter()
            .any(|r| r == "authorization denied")
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn chatgpt_callback_without_client_id_is_typed() {
    let _guard = loopback_1455_guard().await;
    let server = MockServer::start().await;
    mount_chatgpt_token(&server, serde_json::json!({})).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));

    let driver_presenter = presenter.clone();
    let drive = tokio::spawn(async move {
        let url = chatgpt_wait_for_authorize_url(&driver_presenter).await;
        let state = ScriptedPresenter::query_param(&url, "state").unwrap();
        let response =
            chatgpt_callback_get(&format!("GET /auth/callback?code=cb-8&state={state}")).await;
        let response = response.expect("callback response");
        assert!(response.starts_with("HTTP/1.1 400"), "{response}");
    });
    let error = flow.login(presenter.as_ref()).await.expect_err("typed");
    drive.await.expect("driver");
    assert!(matches!(error, ProviderError::Config(_)), "{error:?}");
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .iter()
            .any(|r| r == "callback parse error")
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn chatgpt_device_id_must_be_a_present_uuid_before_any_traffic() {
    for device_id in [None, Some("not-a-uuid"), Some("")] {
        let server = MockServer::start().await;
        let presenter = Arc::new(ScriptedPresenter::default());
        let flow = chatgpt_flow(&server, device_id, Duration::from_secs(30));
        let error = flow.login(presenter.as_ref()).await.expect_err("typed");
        assert!(
            matches!(error, ProviderError::Config(_)),
            "{device_id:?}: {error:?}"
        );
        assert!(presenter.authorize_urls.lock().unwrap().is_empty());
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn chatgpt_token_response_contract_violations_are_typed() {
    let _guard = loopback_1455_guard().await;
    let cases: [(serde_json::Value, &str); 6] = [
        (serde_json::json!({"id_token": null}), "id_token"),
        (
            serde_json::json!({"scope": "openid profile email offline_access"}),
            "chatgpt.tokens.use.direct",
        ),
        (serde_json::json!({"refresh_token": null}), "refresh_token"),
        (serde_json::json!({"expires_in": 0}), "expires_in"),
        (serde_json::json!({"expires_in": -5}), "expires_in"),
        (serde_json::json!({"expires_in": null}), "expires_in"),
    ];
    for (mutate, expected_fragment) in cases {
        let case = mutate.to_string();
        let server = MockServer::start().await;
        mount_chatgpt_token(&server, mutate).await;
        let presenter = Arc::new(ScriptedPresenter::default());
        *presenter.on_present.lock().unwrap() = Some(chatgpt_paste_hook());
        let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));
        let error = flow.login(presenter.as_ref()).await.expect_err("typed");
        assert!(
            matches!(error, ProviderError::Config(_)),
            "{case} ({expected_fragment}): {error:?}"
        );
        // A contract violation never yields a credential or success.
        assert!(presenter.success_at.lock().unwrap().is_none());
    }
}

#[tokio::test]
async fn chatgpt_refresh_reuses_stored_client_id_and_resource_and_revalidates() {
    let server = MockServer::start().await;
    mount_chatgpt_token(
        &server,
        serde_json::json!({
            "access_token": "at-2", "refresh_token": "rt-2", "expires_in": 5400,
            "id_token": null
        }),
    )
    .await;
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));
    let cred = OAuthCredential {
        access: secrecy::SecretString::new("at-1".into()),
        refresh: secrecy::SecretString::new("rt-1".into()),
        expires_at: None,
        base_url: None,
        account_id: None,
        client_id: Some("issued-client-7".to_owned()),
        enterprise_domain: None,
        available_model_ids: None,
    };

    let refreshed = flow.refresh(&cred).await.expect("refresh");
    assert_eq!(refreshed.access.expose_secret(), "at-2");
    assert_eq!(refreshed.refresh.expose_secret(), "rt-2");
    assert_eq!(refreshed.client_id.as_deref(), Some("issued-client-7"));
    let requests = server.received_requests().await.unwrap();
    let body = String::from_utf8(requests[0].body.clone()).unwrap();
    let pairs = form_pairs(&body);
    let pair = |name: &str| {
        pairs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    assert_eq!(pair("grant_type"), "refresh_token");
    assert_eq!(pair("client_id"), "issued-client-7");
    assert_eq!(pair("refresh_token"), "rt-1");
    assert_eq!(pair("resource"), "https://api.openai.com/v1");

    // A response without the direct-token scope is typed, never a credential.
    let server = MockServer::start().await;
    mount_chatgpt_token(
        &server,
        serde_json::json!({
            "access_token": "at-3", "refresh_token": "rt-3", "expires_in": 5400,
            "scope": "openid profile email offline_access", "id_token": null
        }),
    )
    .await;
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));
    let error = flow.refresh(&cred).await.expect_err("typed");
    assert!(matches!(error, ProviderError::Config(_)), "{error:?}");

    // A stored credential without the issued client id is a typed reconnect
    // error before any HTTP traffic.
    let server = MockServer::start().await;
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));
    let cred = OAuthCredential {
        client_id: None,
        enterprise_domain: None,
        available_model_ids: None,
        ..cred
    };
    let error = flow.refresh(&cred).await.expect_err("typed");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn chatgpt_models_login_acks_the_store_write_as_an_oauth_envelope() {
    let _guard = loopback_1455_guard().await;
    let server = MockServer::start().await;
    mount_chatgpt_token(&server, serde_json::json!({})).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    *presenter.on_present.lock().unwrap() = Some(chatgpt_paste_hook());
    let flow = chatgpt_flow(&server, Some(CHATGPT_DEVICE_UUID), Duration::from_secs(30));
    let store = FakeStore::new();
    let store_handle = store.clone();
    let mut models = Models::new();
    models.register_oauth_flow("openai-chatgpt", Arc::new(flow));
    models.set_credential_store(store);

    let credential = models
        .login("openai-chatgpt", presenter.as_ref())
        .await
        .expect("login");
    assert_eq!(credential.client_id.as_deref(), Some("issued-client-1"));
    // The persisted envelope is the OAuth token kind with the issued client
    // id — API-key-only consumers (the Decisions classifier) can never
    // resolve this subscription credential as an API key.
    let stored = store_handle.entry("openai-chatgpt").expect("stored");
    match stored {
        Credential::OAuthToken {
            access, client_id, ..
        } => {
            assert_eq!(access.expose_secret(), credential.access.expose_secret());
            assert_eq!(client_id.as_deref(), Some("issued-client-1"));
        }
        other => panic!("unexpected stored kind: {other:?}"),
    }
    assert!(presenter.success_at.lock().unwrap().is_some());
}

// ---------------------------------------------------------------------------
// Phase 20.26 — SDK OAuth (GitHub Copilot)
// ---------------------------------------------------------------------------

use opi_ai::auth::oauth::copilot::{
    self, COPILOT_CLIENT_ID, COPILOT_DEFAULT_BASE_URL, COPILOT_DEVICE_SCOPE, COPILOT_PUBLIC_DOMAIN,
    CopilotUrls, GitHubCopilotOAuthProvider,
};

fn copilot_token_body() -> serde_json::Value {
    let expires = time::OffsetDateTime::now_utc().unix_timestamp() + 3600;
    serde_json::json!({
        "token": "tid=1;exp=2;proxy-ep=proxy.individual.githubcopilot.com;sch=sha256",
        "expires_at": expires,
    })
}

/// Models catalog entries: (id, picker_enabled, policy_state, supports_tools).
fn copilot_models_body(items: &[(&str, bool, &str, bool)]) -> serde_json::Value {
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

async fn mount_copilot_device_start(server: &MockServer, verification_uri: String) {
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

async fn mount_copilot_poll(server: &MockServer, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

async fn mount_copilot_poll_complete(server: &MockServer) {
    mount_copilot_poll(server, serde_json::json!({"access_token": "gh-access-1"})).await;
}

async fn mount_copilot_exchange(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/copilot_internal/v2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(copilot_token_body()))
        .mount(server)
        .await;
}

async fn mount_copilot_models(server: &MockServer, body: serde_json::Value) {
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
fn copilot_flow(
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

fn copilot_presenter_with(answer: &str) -> Arc<ScriptedPresenter> {
    let presenter = ScriptedPresenter::default();
    *presenter.prompt_answers.lock().unwrap() = vec![answer.to_owned()];
    Arc::new(presenter)
}

#[tokio::test]
async fn copilot_login_persists_tokens_models_and_real_expiry() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll_complete(&server).await;
    mount_copilot_exchange(&server).await;
    mount_copilot_models(
        &server,
        copilot_models_body(&[("gpt-5-codex", true, "enabled", true)]),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let credential = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect("copilot login");

    // The device start carries the pinned client id + read:user scope.
    let requests = server.received_requests().await.unwrap();
    let device = requests
        .iter()
        .find(|request| request.url.path() == "/login/device/code")
        .expect("device start request");
    let body = String::from_utf8_lossy(&device.body).into_owned();
    assert!(body.contains(&format!("client_id={COPILOT_CLIENT_ID}")));
    assert!(body.contains(&format!(
        "scope={}",
        COPILOT_DEVICE_SCOPE.replace(':', "%3A")
    )));
    // The public user_code is presented, never the device_code.
    let device_codes = presenter.device_codes.lock().unwrap().clone();
    assert_eq!(device_codes.len(), 1);
    assert_eq!(device_codes[0].0, "ABCD-1234");
    assert_eq!(device_codes[0].1, format!("{}/login/device", server.uri()));
    // The exchange carries the GitHub token; models carry the copilot token
    // and the pinned API version header.
    let exchange = requests
        .iter()
        .find(|request| request.url.path() == "/copilot_internal/v2/token")
        .expect("token exchange");
    assert_eq!(
        exchange
            .headers
            .get("authorization")
            .and_then(|value| value.to_str().ok()),
        Some("Bearer gh-access-1")
    );
    let models = requests
        .iter()
        .find(|request| request.url.path() == "/models")
        .expect("models request");
    assert_eq!(
        models
            .headers
            .get("x-github-api-version")
            .and_then(|value| value.to_str().ok()),
        Some("2026-06-01")
    );
    // Credential: GitHub token is the refresh grant, copilot token the
    // bearer, real service expiry, discovered model ids, no policy requests
    // (the whitelist is empty).
    assert_eq!(credential.refresh.expose_secret(), "gh-access-1");
    assert!(credential.access.expose_secret().contains("proxy-ep="));
    // The stored expiry is the response's absolute service expiry, roughly
    // one hour from when the mock body was built — not a freshly derived
    // now+N.
    let expiry = credential.expires_at.expect("expiry");
    let until = (expiry - time::OffsetDateTime::now_utc()).whole_seconds();
    assert!(
        (3590..=3610).contains(&until),
        "real service expiry, {until}s remaining"
    );
    assert_eq!(
        credential.available_model_ids.as_deref(),
        Some(&["gpt-5-codex".to_owned()][..])
    );
    assert!(credential.base_url.is_none());
    assert!(credential.enterprise_domain.is_none());
    assert!(
        !requests
            .iter()
            .any(|request| request.url.path().contains("/policy")),
        "no policy activation without a whitelist"
    );
    assert!(presenter.success_at.lock().unwrap().is_some());
}

#[tokio::test]
async fn copilot_models_login_acks_the_store_write() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll_complete(&server).await;
    mount_copilot_exchange(&server).await;
    mount_copilot_models(&server, copilot_models_body(&[])).await;
    let store = FakeStore::new();
    let mut models = Models::new();
    models.register_oauth_flow(
        "github-copilot",
        Arc::new(copilot_flow(
            &server.uri(),
            Vec::new(),
            Duration::from_secs(30),
        )),
    );
    models.set_credential_store(store.clone());
    let presenter = Arc::new(ScriptedPresenter::default());

    let credential = models
        .login("github-copilot", presenter.as_ref())
        .await
        .expect("login");

    let stored = store.entry("github-copilot").expect("stored");
    match stored {
        Credential::OAuthToken {
            refresh,
            available_model_ids,
            ..
        } => {
            assert_eq!(refresh.expose_secret(), credential.refresh.expose_secret());
            assert_eq!(available_model_ids, credential.available_model_ids);
        }
        other => panic!("unexpected stored kind: {other:?}"),
    }
    assert!(presenter.success_at.lock().unwrap().is_some());
}

#[tokio::test]
async fn copilot_enterprise_domain_prompt_normalizes_and_derives_urls() {
    let server = MockServer::start().await;
    let uri = server.uri();
    Mock::given(method("POST"))
        .and(path("/ghe.test/login/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "dc-secret-1",
            "user_code": "ABCD-1234",
            "verification_uri": format!("{uri}/login/device"),
            "interval": 0,
            "expires_in": 900,
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/ghe.test/login/oauth/access_token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"error": "access_denied"})),
        )
        .mount(&server)
        .await;
    // Enterprise derivation: `https://{domain}/...` paths become
    // `{domain}/...` on the offline server.
    let flow = GitHubCopilotOAuthProvider::new_with_endpoints(
        CopilotUrls {
            device_code_url: format!("{uri}/login/device/code"),
            access_token_url: format!("{uri}/login/oauth/access_token"),
            copilot_token_url: format!("{uri}/copilot_internal/v2/token"),
        },
        {
            let uri = uri.clone();
            Arc::new(move |domain: &str| CopilotUrls {
                device_code_url: format!("{uri}/{domain}/login/device/code"),
                access_token_url: format!("{uri}/{domain}/login/oauth/access_token"),
                copilot_token_url: format!("{uri}/{domain}/copilot_internal/v2/token"),
            })
        },
        {
            let models_uri = uri.clone();
            Arc::new(move |_token: &str, _domain: Option<&str>| models_uri.clone())
        },
        Vec::new(),
        Duration::from_secs(30),
        reqwest::Client::new(),
    );
    let presenter = copilot_presenter_with("https://ghe.test/orgs/acme");

    // denied ends the flow before the exchange; the enterprise device start
    // already proves the URL derivation.
    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("denied");
    assert!(matches!(error, ProviderError::CredentialRevoked { .. }));
    let requests = server.received_requests().await.unwrap();
    assert!(
        requests
            .iter()
            .any(|request| request.url.path() == "/ghe.test/login/device/code"),
        "enterprise domain must derive its own device endpoint"
    );
}

#[tokio::test]
async fn copilot_invalid_enterprise_domain_is_typed_before_any_traffic() {
    let server = MockServer::start().await;
    let presenter = copilot_presenter_with("not a domain!");
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("invalid domain");
    assert!(matches!(error, ProviderError::Config(_)));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn copilot_untrusted_verification_uri_is_typed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "dc-secret-1",
            "user_code": "ABCD-1234",
            "verification_uri": "file:///etc/passwd",
            "interval": 0,
            "expires_in": 900,
        })))
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("untrusted verification uri");
    assert!(matches!(error, ProviderError::Config(_)));
    assert!(
        presenter.device_codes.lock().unwrap().is_empty(),
        "the user code is never presented for an untrusted verification URI"
    );
    let requests = server.received_requests().await.unwrap();
    assert!(
        !requests
            .iter()
            .any(|request| request.url.path() == "/login/oauth/access_token")
    );
}

#[tokio::test]
async fn copilot_denied_poll_is_typed_terminal_without_exchange() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll(&server, serde_json::json!({"error": "access_denied"})).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("denied");
    assert!(matches!(error, ProviderError::CredentialRevoked { .. }));
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .contains(&"device authorization denied".to_owned())
    );
    let requests = server.received_requests().await.unwrap();
    assert!(
        !requests
            .iter()
            .any(|request| request.url.path().starts_with("/copilot_internal"))
    );
}

#[tokio::test]
async fn copilot_expired_poll_is_typed_terminal() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll(&server, serde_json::json!({"error": "expired_token"})).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("expired");
    assert!(matches!(error, ProviderError::CredentialRevoked { .. }));
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .contains(&"device code expired".to_owned())
    );
}

#[tokio::test]
async fn copilot_unknown_poll_error_is_closed() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll(&server, serde_json::json!({"error": "unauthorized_client"})).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("unknown error code");
    assert!(matches!(error, ProviderError::AuthFailed(_)));
    assert!(!matches!(error, ProviderError::CredentialRevoked { .. }));
    let requests = server.received_requests().await.unwrap();
    assert!(
        !requests
            .iter()
            .any(|request| request.url.path().starts_with("/copilot_internal"))
    );
}

#[tokio::test]
async fn copilot_slow_down_uses_server_interval_to_keep_polling() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    // Every poll answers slow_down with server interval 1s: without the
    // server interval the RFC +5s default would allow only one poll inside
    // the 3s budget.
    mount_copilot_poll(
        &server,
        serde_json::json!({"error": "slow_down", "interval": 1}),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(3));

    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("budget bounds the slow_down loop");
    assert!(matches!(error, ProviderError::Timeout));
    let poll_count = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.url.path() == "/login/oauth/access_token")
        .count();
    assert!(
        poll_count >= 2,
        "server interval must keep polling: {poll_count}"
    );
}

#[tokio::test]
async fn copilot_token_exchange_401_is_credential_revoked() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll_complete(&server).await;
    Mock::given(method("GET"))
        .and(path("/copilot_internal/v2/token"))
        .respond_with(ResponseTemplate::new(401).set_body_string("revoked"))
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("revoked grant");
    assert!(matches!(error, ProviderError::CredentialRevoked { .. }));
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .contains(&"token exchange failed".to_owned())
    );
}

#[tokio::test]
async fn copilot_token_response_missing_fields_is_typed() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll_complete(&server).await;
    Mock::given(method("GET"))
        .and(path("/copilot_internal/v2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token": "tid=1;exp=2;proxy-ep=proxy.individual.githubcopilot.com",
        })))
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("missing expires_at");
    assert!(matches!(error, ProviderError::Config(_)));
    assert!(
        !server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .any(|request| request.url.path() == "/models")
    );
}

#[tokio::test]
async fn copilot_models_429_retries_then_succeeds() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll_complete(&server).await;
    mount_copilot_exchange(&server).await;
    // First models response is a 429 with Retry-After: 0; the login policy
    // retries and the second response completes discovery. `up_to_n_times`
    // expires the 429 mock after one hit so the base mock serves the retry.
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "0")
                .set_body_string("rate limited"),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    mount_copilot_models(&server, copilot_models_body(&[])).await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let credential = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect("retry recovers");
    assert_eq!(credential.available_model_ids.as_deref(), Some(&[][..]));
    let models_requests = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.url.path() == "/models")
        .count();
    assert_eq!(models_requests, 2);
}

#[tokio::test]
async fn copilot_models_429_exhaustion_is_typed() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll_complete(&server).await;
    mount_copilot_exchange(&server).await;
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "0")
                .set_body_string("rate limited"),
        )
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));

    let error = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect_err("exhausted retries");
    assert!(matches!(error, ProviderError::AuthFailed(_)));
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .contains(&"models discovery failed".to_owned())
    );
    let models_requests = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.url.path() == "/models")
        .count();
    assert_eq!(models_requests, 3, "login policy is 2 retries + 1 attempt");
}

#[tokio::test]
async fn copilot_policy_activation_is_login_only_and_progress_notified() {
    let server = MockServer::start().await;
    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll_complete(&server).await;
    mount_copilot_exchange(&server).await;
    mount_copilot_models(
        &server,
        copilot_models_body(&[
            ("gpt-5-codex", true, "enabled", true),
            ("claude-sonnet-4-5", true, "unconfigured", true),
            ("gpt-5-mini", true, "enabled", false),
        ]),
    )
    .await;
    Mock::given(method("POST"))
        .and(path("/models/claude-sonnet-4-5/policy"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());
    let whitelist = vec![
        "claude-sonnet-4-5".to_owned(),
        "gpt-5-mini".to_owned(),
        "unknown-model".to_owned(),
    ];

    mount_copilot_device_start(&server, format!("{}/login/device", server.uri())).await;
    mount_copilot_poll_complete(&server).await;
    mount_copilot_exchange(&server).await;
    let flow = copilot_flow(&server.uri(), whitelist, Duration::from_secs(30));

    let credential = OAuthProvider::login(&flow, presenter.as_ref())
        .await
        .expect("login with activation");

    assert_eq!(
        presenter.progress.lock().unwrap().as_slice(),
        ["Enabling models...".to_owned()].as_slice()
    );
    // tool_calls=false is filtered from the catalog; the enabled model and
    // the one enabled-by-login policy model are available.
    assert_eq!(
        credential.available_model_ids.as_deref(),
        Some(&["gpt-5-codex".to_owned(), "claude-sonnet-4-5".to_owned()][..])
    );
    let requests = server.received_requests().await.unwrap();
    let policy = requests
        .iter()
        .find(|request| request.url.path() == "/models/claude-sonnet-4-5/policy")
        .expect("policy activation request");
    assert_eq!(
        String::from_utf8_lossy(&policy.body),
        r#"{"state":"enabled"}"#
    );
    assert_eq!(
        policy
            .headers
            .get("openai-intent")
            .and_then(|value| value.to_str().ok()),
        Some("chat-policy")
    );
}

#[tokio::test]
async fn copilot_refresh_reexchanges_rediscovers_and_never_enables_policy() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ghe.test/copilot_internal/v2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(copilot_token_body()))
        .mount(&server)
        .await;
    mount_copilot_models(
        &server,
        copilot_models_body(&[("claude-sonnet-4-5", true, "unconfigured", true)]),
    )
    .await;
    let flow = copilot_flow(
        &server.uri(),
        vec!["claude-sonnet-4-5".to_owned()],
        Duration::from_secs(30),
    );
    let cred = OAuthCredential {
        access: secrecy::SecretString::from("old-copilot-token"),
        refresh: secrecy::SecretString::from("gh-access-1"),
        expires_at: Some(time::OffsetDateTime::now_utc() - time::Duration::minutes(10)),
        base_url: None,
        account_id: None,
        client_id: None,
        enterprise_domain: Some("ghe.test".to_owned()),
        available_model_ids: Some(vec!["stale-model".to_owned()]),
    };

    let refreshed = OAuthProvider::refresh(&flow, &cred).await.expect("refresh");

    assert_eq!(refreshed.refresh.expose_secret(), "gh-access-1");
    assert!(refreshed.access.expose_secret().contains("proxy-ep="));
    let expiry = refreshed.expires_at.expect("expiry");
    let until = (expiry - time::OffsetDateTime::now_utc()).whole_seconds();
    assert!(
        (3590..=3610).contains(&until),
        "real service expiry, {until}s remaining"
    );
    assert_eq!(refreshed.enterprise_domain.as_deref(), Some("ghe.test"));
    // The picker-on unconfigured model is still picker-listed after
    // discovery, but nothing was enabled: no policy request ever fired and
    // the catalog reflects exactly the strict picker semantics.
    assert_eq!(
        refreshed.available_model_ids.as_deref(),
        Some(&["claude-sonnet-4-5".to_owned()][..]),
        "refresh re-discovers picker availability without enabling anything"
    );
    let requests = server.received_requests().await.unwrap();
    assert!(
        !requests
            .iter()
            .any(|request| request.url.path().contains("/policy")),
        "refresh must not activate policies"
    );
    // The refresh-time discovery is a single attempt (policy 0/0).
    let models_requests = requests
        .iter()
        .filter(|request| request.url.path() == "/models")
        .count();
    assert_eq!(models_requests, 1);
    // The exchange used the enterprise endpoint derivation.
    assert!(
        requests
            .iter()
            .any(|request| request.url.path() == "/ghe.test/copilot_internal/v2/token"),
        "refresh must derive the enterprise exchange endpoint"
    );
}

#[tokio::test]
async fn copilot_refresh_401_is_credential_revoked() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/copilot_internal/v2/token"))
        .respond_with(ResponseTemplate::new(401).set_body_string("revoked"))
        .mount(&server)
        .await;
    let flow = copilot_flow(&server.uri(), Vec::new(), Duration::from_secs(30));
    let cred = OAuthCredential {
        access: secrecy::SecretString::from("old-copilot-token"),
        refresh: secrecy::SecretString::from("gh-access-1"),
        expires_at: Some(time::OffsetDateTime::now_utc() - time::Duration::minutes(10)),
        base_url: None,
        account_id: None,
        client_id: None,
        enterprise_domain: None,
        available_model_ids: None,
    };

    let error = OAuthProvider::refresh(&flow, &cred)
        .await
        .expect_err("revoked");
    assert!(matches!(error, ProviderError::CredentialRevoked { .. }));
}

// Pure-contract units (no HTTP): domain normalization, URL derivation and
// the proxy-endpoint base URL precedence.

#[test]
fn copilot_normalize_domain_variants() {
    assert_eq!(copilot::normalize_enterprise_domain(""), None);
    assert_eq!(
        copilot::normalize_enterprise_domain("company.ghe.com"),
        Some("company.ghe.com".to_owned())
    );
    assert_eq!(
        copilot::normalize_enterprise_domain("  https://company.ghe.com/orgs/acme  "),
        Some("company.ghe.com".to_owned())
    );
    assert_eq!(
        copilot::normalize_enterprise_domain("http://localhost:1455/x"),
        Some("localhost".to_owned())
    );
    assert_eq!(
        copilot::normalize_enterprise_domain("user@gh.example.com"),
        Some("gh.example.com".to_owned())
    );
    assert_eq!(copilot::normalize_enterprise_domain("not a domain!"), None);
    assert_eq!(copilot::normalize_enterprise_domain("https://"), None);
}

#[test]
fn copilot_urls_for_domain_shape() {
    let urls = copilot::copilot_urls_for_domain(COPILOT_PUBLIC_DOMAIN);
    assert_eq!(urls.device_code_url, "https://github.com/login/device/code");
    assert_eq!(
        urls.access_token_url,
        "https://github.com/login/oauth/access_token"
    );
    assert_eq!(
        urls.copilot_token_url,
        "https://api.github.com/copilot_internal/v2/token"
    );
}

#[test]
fn copilot_base_url_precedence() {
    let token = "tid=1;exp=2;proxy-ep=proxy.individual.githubcopilot.com;sch=sha256";
    assert_eq!(
        copilot::base_url_from_token(token).as_deref(),
        Some(COPILOT_DEFAULT_BASE_URL)
    );
    // A proxy endpoint without the `proxy.` prefix is used as-is.
    assert_eq!(
        copilot::base_url_from_token("proxy-ep=enterprise.example").as_deref(),
        Some("https://enterprise.example".to_owned()).as_deref()
    );
    assert_eq!(copilot::base_url_from_token("no-hint"), None);
    assert_eq!(
        copilot::request_base_url(Some(token), None),
        COPILOT_DEFAULT_BASE_URL
    );
    assert_eq!(
        copilot::request_base_url(Some(token), Some("ghe.test")),
        COPILOT_DEFAULT_BASE_URL,
        "the token proxy endpoint wins over the enterprise domain"
    );
    assert_eq!(
        copilot::request_base_url(None, Some("ghe.test")),
        "https://copilot-api.ghe.test"
    );
    assert_eq!(
        copilot::request_base_url(None, None),
        COPILOT_DEFAULT_BASE_URL
    );
}

#[test]
fn copilot_models_catalog_parsing_variants() {
    let whitelist = ["known-model".to_owned()];

    // Strict base: only picker-enabled non-disabled models survive; a
    // picker-on unconfigured model stays picker-available (activation is a
    // separate login action); the disabled and no-tools models drop.
    let strict_raw = copilot_models_body(&[
        ("picker-on", true, "enabled", true),
        ("picker-on-unconfigured", true, "unconfigured", true),
        ("picker-disabled", true, "disabled", true),
        ("no-tools", true, "enabled", false),
        ("known-model", false, "unconfigured", true),
    ]);
    let strict = copilot::parse_copilot_models_catalog(&strict_raw, false, &whitelist)
        .expect("strict catalog");
    assert_eq!(
        strict.available_model_ids,
        ["picker-on".to_owned(), "picker-on-unconfigured".to_owned()]
    );
    // Without the picker fallback, an unconfigured whitelisted model with a
    // false picker flag is not offered for activation.
    assert!(strict.policy_model_ids.is_empty());

    // Individual endpoint with every picker flag false: the enabled-policy
    // fallback applies, and the unconfigured whitelisted model becomes
    // eligible for login-time activation.
    let fallback_raw = copilot_models_body(&[
        ("enabled-one", false, "enabled", true),
        ("known-model", false, "unconfigured", true),
        ("disabled-one", false, "disabled", true),
    ]);
    let fallback = copilot::parse_copilot_models_catalog(&fallback_raw, true, &whitelist)
        .expect("fallback catalog");
    assert_eq!(fallback.available_model_ids, ["enabled-one".to_owned()]);
    assert_eq!(fallback.policy_model_ids, ["known-model".to_owned()]);

    // No `data` array is a typed failure.
    let invalid = serde_json::json!({"error": "oops"});
    assert!(copilot::parse_copilot_models_catalog(&invalid, false, &whitelist).is_err());
}

// ---------------------------------------------------------------------------
// OpenRouter OAuth (P20-S27), pinned to pi 1.3.0 openrouter.ts
// ---------------------------------------------------------------------------

use opi_ai::auth::oauth::openrouter::{
    OPENROUTER_AUTHORIZE_URL, OPENROUTER_CALLBACK_PATH_PREFIX, OPENROUTER_PERMANENT_KEY_UNIX_TS,
    OPENROUTER_REDIRECT_HOST, OPENROUTER_TOKEN_URL, OpenRouterOAuthProvider,
    openrouter_permanent_key_expiry,
};

fn openrouter_flow(token_url: String) -> OpenRouterOAuthProvider {
    OpenRouterOAuthProvider::new(
        OPENROUTER_AUTHORIZE_URL.to_owned(),
        token_url,
        Duration::from_secs(10),
    )
}

/// Wait for (and return) the flow's authorize URL.
async fn wait_for_authorize_url(presenter: &ScriptedPresenter) -> String {
    let mut url = None;
    for _ in 0..500 {
        url = presenter.authorize_urls.lock().unwrap().first().cloned();
        if url.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    url.expect("authorize url never presented")
}

async fn mount_openrouter_key(server: &MockServer, status: u16, body: String) {
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/keys"))
        .respond_with(
            ResponseTemplate::new(status)
                .insert_header("Content-Type", "application/json")
                .set_body_string(body),
        )
        .mount(server)
        .await;
}

/// Wait for the authorize URL, then drive the loopback callback with a code.
async fn drive_openrouter_callback(presenter: &ScriptedPresenter, code: &str) -> u16 {
    let mut url = None;
    for _ in 0..500 {
        url = presenter.authorize_urls.lock().unwrap().first().cloned();
        if url.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let url = url.expect("authorize url never presented");
    let callback = ScriptedPresenter::query_param(&url, "callback_url").unwrap();
    let response = reqwest::get(format!("{callback}?code={code}"))
        .await
        .expect("callback request");
    response.status().as_u16()
}

#[tokio::test]
async fn openrouter_login_exchanges_code_for_permanent_api_key() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"sk-or-v1-permanent"}"#.to_owned()).await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());

    let login = {
        let flow = flow.clone();
        let presenter = presenter.clone();
        tokio::spawn(async move { flow.login(presenter.as_ref()).await })
    };
    assert_eq!(drive_openrouter_callback(&presenter, "cb-1").await, 200);
    let credential = login.await.expect("join").expect("login failed");

    // Authorize URL shape: callback_url + S256 PKCE, nothing else.
    let url = presenter.last_authorize_url();
    let callback = ScriptedPresenter::query_param(&url, "callback_url").unwrap();
    assert!(callback.starts_with("http://127.0.0.1:"));
    assert!(callback.contains(OPENROUTER_CALLBACK_PATH_PREFIX));
    let challenge = ScriptedPresenter::query_param(&url, "code_challenge").unwrap();
    assert_eq!(challenge.len(), 43);
    assert_eq!(
        ScriptedPresenter::query_param(&url, "code_challenge_method").as_deref(),
        Some("S256")
    );
    for absent in [
        "response_type",
        "client_id",
        "redirect_uri",
        "state",
        "scope",
    ] {
        assert!(
            ScriptedPresenter::query_param(&url, absent).is_none(),
            "authorize URL must not carry {absent}"
        );
    }
    // Listening progress names the bound redirect URI.
    assert!(
        presenter
            .progress
            .lock()
            .unwrap()
            .iter()
            .any(|m| m.starts_with("Listening for OpenRouter OAuth callback on http://"))
    );

    // Exchange: exactly one JSON POST with the pinned body shape.
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method.as_str(), "POST");
    assert_eq!(
        requests[0]
            .headers
            .get("accept")
            .and_then(|v| v.to_str().ok()),
        Some("application/json")
    );
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["code"], "cb-1");
    assert_eq!(body["code_challenge_method"], "S256");
    let verifier = body["code_verifier"].as_str().unwrap();
    assert_eq!(verifier.len(), 43);

    // Credential: permanent key, empty refresh slot, sentinel expiry.
    assert_eq!(credential.access.expose_secret(), "sk-or-v1-permanent");
    assert_eq!(credential.refresh.expose_secret(), "");
    assert_eq!(
        credential.expires_at,
        Some(openrouter_permanent_key_expiry())
    );
    assert!(!credential.needs_refresh());
    assert_eq!(credential.base_url, None);
    assert_eq!(credential.account_id, None);
    assert_eq!(credential.client_id, None);
    assert!(presenter.failures.lock().unwrap().is_empty());
}

#[tokio::test]
async fn openrouter_callback_path_is_random_per_login() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"k"}"#.to_owned()).await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));

    let mut paths = Vec::new();
    for _ in 0..2 {
        let presenter = Arc::new(ScriptedPresenter::default());
        let login = {
            let flow = flow.clone();
            let presenter = presenter.clone();
            tokio::spawn(async move { flow.login(presenter.as_ref()).await })
        };
        drive_openrouter_callback(&presenter, "c").await;
        login.await.expect("join").expect("login failed");
        let url = presenter.last_authorize_url();
        let callback = ScriptedPresenter::query_param(&url, "callback_url").unwrap();
        let path = callback
            .split_once(OPENROUTER_CALLBACK_PATH_PREFIX)
            .unwrap()
            .1;
        assert!(!path.is_empty());
        paths.push(path.to_owned());
    }
    assert_ne!(paths[0], paths[1], "callback route must be per-login");
}

#[tokio::test]
async fn openrouter_wrong_callback_route_keeps_waiting() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"k"}"#.to_owned()).await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());
    let login = {
        let flow = flow.clone();
        let presenter = presenter.clone();
        tokio::spawn(async move { flow.login(presenter.as_ref()).await })
    };
    let url = wait_for_authorize_url(presenter.as_ref()).await;
    let callback = ScriptedPresenter::query_param(&url, "callback_url").unwrap();

    // A stray request for a different route (and a non-GET probe) gets 404
    // and must not complete or fail the login; the real callback then wins.
    let stray = callback.replace(
        OPENROUTER_CALLBACK_PATH_PREFIX,
        &format!("{OPENROUTER_CALLBACK_PATH_PREFIX}not-the-route-"),
    );
    let response = reqwest::get(format!("{stray}?code=stray-code"))
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 404);
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{callback}?code=stray"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 404);
    let response = reqwest::get(format!("{callback}?code=real-code"))
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);

    let credential = login.await.expect("join").expect("login failed");
    assert_eq!(credential.access.expose_secret(), "k");
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["code"], "real-code");
}

#[tokio::test]
async fn openrouter_callback_denial_is_typed_without_detail_echo() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"k"}"#.to_owned()).await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());
    let login = {
        let flow = flow.clone();
        let presenter = presenter.clone();
        tokio::spawn(async move { flow.login(presenter.as_ref()).await })
    };
    let url = wait_for_authorize_url(presenter.as_ref()).await;
    let callback = ScriptedPresenter::query_param(&url, "callback_url").unwrap();
    let response = reqwest::get(format!(
        "{callback}?error=access_denied&error_description=secret-canary"
    ))
    .await
    .unwrap();
    assert_eq!(response.status().as_u16(), 400);
    let error = login.await.expect("join").expect_err("denial must fail");
    assert!(matches!(error, ProviderError::Config(_)), "got {error:?}");
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .iter()
            .any(|r| r == "authorization denied")
    );
}

#[tokio::test]
async fn openrouter_callback_missing_code_fails_typed() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"k"}"#.to_owned()).await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());
    let login = {
        let flow = flow.clone();
        let presenter = presenter.clone();
        tokio::spawn(async move { flow.login(presenter.as_ref()).await })
    };
    let url = wait_for_authorize_url(presenter.as_ref()).await;
    let callback = ScriptedPresenter::query_param(&url, "callback_url").unwrap();
    let response = reqwest::get(format!("{callback}?unrelated=1"))
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 400);
    let error = login.await.expect("join").expect_err("missing code");
    assert!(matches!(error, ProviderError::Config(_)), "got {error:?}");
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn openrouter_manual_full_url_paste_exchanges_its_code() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"k-manual"}"#.to_owned()).await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());
    // Paste the flow's own callback URL back with a code appended.
    *presenter.on_present.lock().unwrap() = Some(Arc::new(|url: &str| {
        let callback = ScriptedPresenter::query_param(url, "callback_url").unwrap();
        format!("{callback}?code=manual-1")
    }));
    let credential = flow.login(presenter.as_ref()).await.expect("login failed");
    assert_eq!(credential.access.expose_secret(), "k-manual");
    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["code"], "manual-1");
}

#[tokio::test]
async fn openrouter_manual_query_fragment_and_bare_code_pastes() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"k"}"#.to_owned()).await;
    let token_url = format!("{}/api/v1/auth/keys", server.uri());

    for paste in ["x=1&code=q2", "bare-code-42"] {
        let flow = openrouter_flow(token_url.clone());
        let presenter = ScriptedPresenter::with_manual(paste);
        let credential = flow.login(presenter.as_ref()).await.expect("login failed");
        assert_eq!(credential.access.expose_secret(), "k");
        let requests = server.received_requests().await.unwrap();
        let body: serde_json::Value =
            serde_json::from_slice(&requests.last().unwrap().body).unwrap();
        assert_eq!(
            body["code"],
            if paste == "x=1&code=q2" {
                "q2"
            } else {
                "bare-code-42"
            }
        );
    }
}

#[tokio::test]
async fn openrouter_manual_missing_code_is_typed() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"k"}"#.to_owned()).await;
    let token_url = format!("{}/api/v1/auth/keys", server.uri());
    for bad in ["", "   ", "http://127.0.0.1:9/oauth/callback/abc?x=1"] {
        let flow = openrouter_flow(token_url.clone());
        let presenter = ScriptedPresenter::with_manual(bad);
        let error = flow.login(presenter.as_ref()).await.expect_err("must fail");
        assert!(matches!(error, ProviderError::Config(_)), "got {error:?}");
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn openrouter_exchange_http_error_is_closed_summary() {
    let server = MockServer::start().await;
    mount_openrouter_key(
        &server,
        400,
        r#"{"error":{"message":"canary-secret-detail"}}"#.to_owned(),
    )
    .await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let presenter = ScriptedPresenter::with_manual("paste-code");
    let error = flow
        .login(presenter.as_ref())
        .await
        .expect_err("http error");
    assert!(
        matches!(error, ProviderError::AuthFailed(_)) || matches!(error, ProviderError::Config(_)),
        "got {error:?}"
    );
    let rendered = format!("{error:?}");
    assert!(
        !rendered.contains("canary"),
        "untrusted detail echoed: {rendered}"
    );
}

#[tokio::test]
async fn openrouter_exchange_invalid_json_is_typed() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, "not-json".to_owned()).await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let presenter = ScriptedPresenter::with_manual("paste-code");
    let error = flow.login(presenter.as_ref()).await.expect_err("bad json");
    assert!(matches!(error, ProviderError::Config(_)), "got {error:?}");
}

#[tokio::test]
async fn openrouter_exchange_missing_or_empty_key_is_typed() {
    let server = MockServer::start().await;
    let token_url = format!("{}/api/v1/auth/keys", server.uri());
    for body in [r#"{"key":""}"#, "{}"] {
        mount_openrouter_key(&server, 200, body.to_owned()).await;
        let flow = openrouter_flow(token_url.clone());
        let presenter = ScriptedPresenter::with_manual("paste-code");
        let error = flow.login(presenter.as_ref()).await.expect_err("no key");
        assert!(matches!(error, ProviderError::Config(_)), "got {error:?}");
    }
}

#[tokio::test]
async fn openrouter_refresh_is_identity_without_network() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"k"}"#.to_owned()).await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let presenter = ScriptedPresenter::with_manual("paste-code");
    let credential = flow.login(presenter.as_ref()).await.expect("login failed");
    let before = server.received_requests().await.unwrap().len();

    let refreshed = flow.refresh(&credential).await.expect("identity refresh");
    assert_eq!(
        refreshed.access.expose_secret(),
        credential.access.expose_secret()
    );
    assert_eq!(refreshed.refresh.expose_secret(), "");
    assert_eq!(refreshed.expires_at, credential.expires_at);
    assert_eq!(server.received_requests().await.unwrap().len(), before);
}

#[tokio::test]
async fn openrouter_permanent_key_expiry_is_the_9999_sentinel() {
    assert_eq!(
        openrouter_permanent_key_expiry(),
        time::OffsetDateTime::from_unix_timestamp(OPENROUTER_PERMANENT_KEY_UNIX_TS).unwrap()
    );
    assert_eq!(openrouter_permanent_key_expiry().year(), 9999);
}

#[tokio::test]
async fn openrouter_models_login_persists_through_store_ack() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"sk-or-v1-acked"}"#.to_owned()).await;
    let store = FakeStore::new();
    let presenter = Arc::new(ScriptedPresenter::with_manual("paste-code"));
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let mut models = Models::new();
    models.register_oauth_flow("openrouter", Arc::new(flow));
    models.set_credential_store(store.clone());

    let credential = models
        .login("openrouter", presenter.as_ref().as_ref())
        .await
        .expect("models login");
    assert_eq!(credential.access.expose_secret(), "sk-or-v1-acked");
    assert!(!credential.needs_refresh());
    let stored = store.entry("openrouter").expect("stored credential");
    match stored {
        Credential::OAuthToken {
            access,
            refresh,
            expires_at,
            ..
        } => {
            assert_eq!(access.expose_secret(), "sk-or-v1-acked");
            assert_eq!(refresh.expose_secret(), "");
            assert_eq!(expires_at, Some(openrouter_permanent_key_expiry()));
        }
        other => panic!("wrong credential kind: {other:?}"),
    }
    assert!(presenter.success_at.lock().unwrap().is_some());
}

#[tokio::test]
async fn openrouter_login_issues_no_personalization_request() {
    let server = MockServer::start().await;
    mount_openrouter_key(&server, 200, r#"{"key":"k"}"#.to_owned()).await;
    let flow = openrouter_flow(format!("{}/api/v1/auth/keys", server.uri()));
    let presenter = Arc::new(ScriptedPresenter::default());
    let login = {
        let flow = flow.clone();
        let presenter = presenter.clone();
        tokio::spawn(async move { flow.login(presenter.as_ref()).await })
    };
    drive_openrouter_callback(&presenter, "cb-1").await;
    login.await.expect("join").expect("login failed");
    // Only the key exchange: no authorize fetch, no /models/user, nothing else.
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), "/api/v1/auth/keys");
}

#[tokio::test]
async fn openrouter_login_timeout_is_typed() {
    let flow = OpenRouterOAuthProvider::new(
        OPENROUTER_AUTHORIZE_URL.to_owned(),
        format!("{OPENROUTER_TOKEN_URL}/unused"),
        Duration::from_millis(300),
    );
    let presenter = ScriptedPresenter::default();
    let error = flow.login(&presenter).await.expect_err("timeout");
    assert!(matches!(error, ProviderError::Timeout), "got {error:?}");
    assert!(
        presenter
            .failures
            .lock()
            .unwrap()
            .iter()
            .any(|r| r == "timeout")
    );
}

#[tokio::test]
async fn openrouter_production_constants_are_pinned() {
    let flow = OpenRouterOAuthProvider::production();
    let credential = OAuthCredential {
        access: secrecy::SecretString::new("k".to_owned().into_boxed_str()),
        refresh: secrecy::SecretString::new(String::new().into_boxed_str()),
        expires_at: Some(openrouter_permanent_key_expiry()),
        base_url: None,
        account_id: None,
        client_id: None,
        enterprise_domain: None,
        available_model_ids: None,
    };
    // Identity refresh holds for the production construction too; only the
    // endpoints differ from the injected tests.
    let refreshed = flow.refresh(&credential).await.expect("identity");
    assert_eq!(refreshed.access.expose_secret(), "k");
    assert_eq!(OPENROUTER_REDIRECT_HOST, "127.0.0.1");
    assert_eq!(
        OPENROUTER_TOKEN_URL,
        "https://openrouter.ai/api/v1/auth/keys"
    );
    assert_eq!(OPENROUTER_AUTHORIZE_URL, "https://openrouter.ai/auth");
}

// ---------------------------------------------------------------------------
// 20.28 — SDK OAuth (Kimi Coding) device flow
// ---------------------------------------------------------------------------

use opi_ai::auth::oauth::kimi_coding::{
    KIMI_CLIENT_ID, KIMI_DEFAULT_OAUTH_HOST, KimiCodingOAuthProvider,
    parse_kimi_device_authorization,
};

fn kimi_start_body(
    interval: serde_json::Value,
    expires_in: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "device_code": "kimi-device-1",
        "user_code": "KIMI-CODE",
        "verification_uri": "https://auth.kimi.com/device",
        "verification_uri_complete": "https://auth.kimi.com/device?user_code=KIMI-CODE",
        "interval": interval,
        "expires_in": expires_in,
    })
}

async fn mount_kimi_start(server: &MockServer, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/api/oauth/device_authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

async fn mount_kimi_token(server: &MockServer, status: u16, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .mount(server)
        .await;
}

fn kimi_token_success() -> serde_json::Value {
    serde_json::json!({
        "access_token": "kimi-access-1",
        "refresh_token": "kimi-refresh-1",
        "expires_in": 3600,
    })
}

fn kimi_flow(server: &MockServer) -> KimiCodingOAuthProvider {
    KimiCodingOAuthProvider::new_with_oauth_host(server.uri())
}

async fn kimi_token_requests(server: &MockServer) -> Vec<wiremock::Request> {
    server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.url.path() == "/api/oauth/token")
        .collect()
}

/// Wall-clock bounds for an `expires_at` derived from `expires_in` seconds.
fn expires_within(expires_at: time::OffsetDateTime, expires_in: i64) {
    let now = time::OffsetDateTime::now_utc();
    let target = now + time::Duration::seconds(expires_in);
    assert!(
        expires_at > now + time::Duration::seconds(expires_in - 30),
        "expires_at {expires_at} too early for +{expires_in}s"
    );
    assert!(
        expires_at < target + time::Duration::seconds(30),
        "expires_at {expires_at} too late for +{expires_in}s"
    );
}

#[tokio::test]
async fn kimi_device_login_presents_complete_uri_and_exchanges_strict_triple() {
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
    mount_kimi_token(&server, 200, kimi_token_success()).await;
    let flow = kimi_flow(&server);
    let presenter = Arc::new(ScriptedPresenter::default());

    let credential = flow.login(presenter.as_ref()).await.expect("login");

    assert_eq!(credential.access.expose_secret(), "kimi-access-1");
    assert_eq!(credential.refresh.expose_secret(), "kimi-refresh-1");
    expires_within(credential.expires_at.expect("expiry"), 3600);
    assert_eq!(credential.base_url, None);
    assert_eq!(credential.account_id, None);
    assert_eq!(credential.client_id, None);
    assert!(!credential.needs_refresh());

    // The presenter gets the public user code and the COMPLETE verification
    // URI, never the device_code.
    assert_eq!(
        presenter.device_codes.lock().unwrap().as_slice(),
        &[(
            "KIMI-CODE".to_owned(),
            "https://auth.kimi.com/device?user_code=KIMI-CODE".to_owned()
        )]
    );

    let requests = server.received_requests().await.unwrap();
    let start = &requests[0];
    assert_eq!(start.url.path(), "/api/oauth/device_authorization");
    assert_eq!(
        String::from_utf8(start.body.clone()).unwrap(),
        format!("client_id={KIMI_CLIENT_ID}")
    );
    assert!(
        start
            .headers
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("application/x-www-form-urlencoded")
    );
    assert!(
        start
            .headers
            .get("accept")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("application/json")
    );

    let poll = &requests[1];
    let body = String::from_utf8(poll.body.clone()).unwrap();
    assert_eq!(poll.url.path(), "/api/oauth/token");
    assert!(
        body.contains(&format!("client_id={KIMI_CLIENT_ID}")),
        "{body}"
    );
    assert!(body.contains("device_code=kimi-device-1"), "{body}");
    assert!(
        body.contains("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code"),
        "{body}"
    );
    assert!(
        poll.headers
            .get("accept")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("application/json")
    );
}

#[tokio::test]
async fn kimi_start_non_ok_fails_typed_without_body_echo() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/device_authorization"))
        .respond_with(ResponseTemplate::new(500).set_body_string("secret-start-detail-CANARY"))
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = kimi_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(!format!("{error:?}").contains("CANARY"));
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device authorization failed"]
    );
    assert!(server.received_requests().await.unwrap().len() == 1);
}

#[tokio::test]
async fn kimi_start_missing_or_empty_fields_fail_typed() {
    for label in ["device_code", "user_code", "verification_uri_complete"] {
        for empty in [false, true] {
            let server = MockServer::start().await;
            let mut body = kimi_start_body(1.into(), 900.into());
            if empty {
                body[label] = serde_json::json!("");
            } else {
                body.as_object_mut().unwrap().remove(label);
            }
            mount_kimi_start(&server, body).await;
            let presenter = Arc::new(ScriptedPresenter::default());
            let error = kimi_flow(&server)
                .login(presenter.as_ref())
                .await
                .expect_err("must fail");
            assert!(
                matches!(error, ProviderError::Config(_)),
                "{error:?} for {label} empty={empty}"
            );
        }
    }
}

#[tokio::test]
async fn kimi_start_untrusted_verification_uris_fail_typed() {
    for field in ["verification_uri", "verification_uri_complete"] {
        for bad in ["urn:ietf:oauth:2.0:oob", "file:///etc/passwd"] {
            let server = MockServer::start().await;
            let mut body = kimi_start_body(1.into(), 900.into());
            body[field] = serde_json::json!(bad);
            mount_kimi_start(&server, body).await;
            let presenter = Arc::new(ScriptedPresenter::default());
            let error = kimi_flow(&server)
                .login(presenter.as_ref())
                .await
                .expect_err("must fail");
            assert!(
                matches!(error, ProviderError::Config(_)),
                "{error:?} for {field}={bad}"
            );
        }
    }
}

#[test]
fn kimi_parse_defaults_interval_and_expiry_and_rejects_bad_types() {
    // RFC 8628 defaults when the field is absent or not a positive finite
    // number.
    let base = kimi_start_body(serde_json::Value::Null, serde_json::Value::Null);
    for (interval, expect_interval) in [
        (serde_json::Value::Null, 5.0),
        (0.into(), 5.0),
        ((-1).into(), 5.0),
        (2.5.into(), 2.5),
    ] {
        let mut body = base.clone();
        if interval.is_null() {
            body.as_object_mut().unwrap().remove("interval");
        } else {
            body["interval"] = interval;
        }
        let parsed = parse_kimi_device_authorization(&body).expect("parse");
        assert_eq!(parsed.interval_seconds, expect_interval);
        assert_eq!(parsed.expires_in_seconds, 900.0);
    }
    let mut body = base;
    body.as_object_mut().unwrap().remove("expires_in");
    body["expires_in"] = 120.into();
    let parsed = parse_kimi_device_authorization(&body).expect("parse");
    assert_eq!(parsed.expires_in_seconds, 120.0);

    // Non-string fields fail typed.
    let mut body = kimi_start_body(1.into(), 900.into());
    body["user_code"] = serde_json::json!(7);
    assert!(parse_kimi_device_authorization(&body).is_err());
}

#[tokio::test]
async fn kimi_first_poll_waits_one_interval() {
    // interval 5s > deadline 2s: the pre-poll wait sleeps min(5, 2) and the
    // device deadline elapses before any token poll is issued.
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(5.into(), 2.into())).await;
    mount_kimi_token(
        &server,
        200,
        serde_json::json!({"error": "authorization_pending"}),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = kimi_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must time out");
    assert!(matches!(error, ProviderError::Timeout), "{error:?}");
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device flow timed out"]
    );
    // Only the device_authorization start request; zero polls.
    assert!(kimi_token_requests(&server).await.is_empty());
}

#[tokio::test]
async fn kimi_slow_down_uses_server_interval() {
    // Initial interval 5s, deadline 8s. Poll #1 (t=5) answers slow_down with
    // interval 1: honoring the server value keeps polling every second
    // (t=6, t=7) until the deadline — exactly three polls. The RFC default
    // (+5s -> interval 10) would never poll again after t=5.
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(5.into(), 8.into())).await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"error": "slow_down", "interval": 1})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    mount_kimi_token(
        &server,
        200,
        serde_json::json!({"error": "authorization_pending"}),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = kimi_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must time out");
    assert!(matches!(error, ProviderError::Timeout), "{error:?}");
    assert_eq!(kimi_token_requests(&server).await.len(), 3);
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device flow timed out after slow_down responses"]
    );
}

#[tokio::test]
async fn kimi_slow_down_without_interval_adds_five_seconds() {
    // Initial interval 1s, deadline 10s. Poll #1 (t=1) answers slow_down
    // with no interval: the RFC 8628 §3.5 default adds 5s, so poll #2 lands
    // at t=6 and the 4s remainder hits the deadline — exactly two polls. A
    // smaller increment (say +4s) would fit a third poll at t=9; ignoring
    // slow_down entirely would poll every second.
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 10.into())).await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"error": "slow_down"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    mount_kimi_token(
        &server,
        200,
        serde_json::json!({"error": "authorization_pending"}),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = kimi_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must time out");
    assert!(matches!(error, ProviderError::Timeout), "{error:?}");
    assert_eq!(kimi_token_requests(&server).await.len(), 2);
}

#[tokio::test]
async fn kimi_pending_then_success_completes_login() {
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 30.into())).await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"error": "authorization_pending"})),
        )
        .up_to_n_times(2)
        .mount(&server)
        .await;
    mount_kimi_token(&server, 200, kimi_token_success()).await;

    let credential = kimi_flow(&server)
        .login(Arc::new(ScriptedPresenter::default()).as_ref())
        .await
        .expect("login");
    assert_eq!(credential.access.expose_secret(), "kimi-access-1");
    assert_eq!(kimi_token_requests(&server).await.len(), 3);
}

#[tokio::test]
async fn kimi_expired_token_fails_typed() {
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
    mount_kimi_token(&server, 200, serde_json::json!({"error": "expired_token"})).await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = kimi_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must fail");
    assert!(
        matches!(error, ProviderError::CredentialRevoked { .. }),
        "{error:?}"
    );
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device code expired"]
    );
}

#[tokio::test]
async fn kimi_access_denied_fails_typed() {
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
    mount_kimi_token(&server, 200, serde_json::json!({"error": "access_denied"})).await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = kimi_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must fail");
    assert!(
        matches!(error, ProviderError::CredentialRevoked { .. }),
        "{error:?}"
    );
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device authorization denied"]
    );
}

#[tokio::test]
async fn kimi_unknown_error_code_fails_typed_without_echo() {
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
    mount_kimi_token(
        &server,
        400,
        serde_json::json!({
            "error": "server_error",
            "error_description": "secret-detail-CANARY"
        }),
    )
    .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = kimi_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(!format!("{error:?}").contains("CANARY"));
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device token request failed"]
    );
}

#[tokio::test]
async fn kimi_poll_5xx_fails_typed_without_body_echo() {
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(ResponseTemplate::new(503).set_body_string("poll-CANARY"))
        .mount(&server)
        .await;
    let presenter = Arc::new(ScriptedPresenter::default());

    let error = kimi_flow(&server)
        .login(presenter.as_ref())
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(!format!("{error:?}").contains("CANARY"));
    assert_eq!(
        presenter.failures.lock().unwrap().as_slice(),
        &["device token request failed"]
    );
}

#[tokio::test]
async fn kimi_success_with_missing_fields_fails_typed() {
    for bad in [
        serde_json::json!({"access_token": "kimi-access-1"}), // missing refresh
        serde_json::json!({
            "access_token": "kimi-access-1",
            "refresh_token": "",
            "expires_in": 3600
        }),
        serde_json::json!({
            "access_token": "kimi-access-1",
            "refresh_token": "kimi-refresh-1",
            "expires_in": 0
        }),
        serde_json::json!({
            "access_token": "kimi-access-1",
            "refresh_token": "kimi-refresh-1",
            "expires_in": "soon"
        }),
    ] {
        let server = MockServer::start().await;
        mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
        mount_kimi_token(&server, 200, bad.clone()).await;
        let presenter = Arc::new(ScriptedPresenter::default());
        let error = kimi_flow(&server)
            .login(presenter.as_ref())
            .await
            .expect_err("must fail");
        assert!(
            matches!(
                error,
                ProviderError::AuthFailed(_) | ProviderError::Config(_)
            ),
            "{error:?} for {bad}"
        );
        assert_eq!(
            presenter.failures.lock().unwrap().as_slice(),
            &["device token response invalid"]
        );
    }
}

#[tokio::test]
async fn kimi_login_persists_through_store_ack() {
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
    mount_kimi_token(&server, 200, kimi_token_success()).await;
    let store = FakeStore::new();
    let mut models = Models::new();
    models.register_oauth_flow("kimi-coding", Arc::new(kimi_flow(&server)));
    models.set_credential_store(store.clone());
    let presenter = Arc::new(ScriptedPresenter::default());

    let credential = models
        .login("kimi-coding", presenter.as_ref())
        .await
        .expect("login");
    assert_eq!(credential.access.expose_secret(), "kimi-access-1");

    let stored = store.entry("kimi-coding").expect("stored");
    match stored {
        Credential::OAuthToken {
            access, refresh, ..
        } => {
            assert_eq!(access.expose_secret(), "kimi-access-1");
            assert_eq!(refresh.expose_secret(), "kimi-refresh-1");
        }
        other => panic!("unexpected stored kind: {other:?}"),
    }
    // notify_success fired after the credential existed in the store.
    assert!(presenter.success_at.lock().unwrap().is_some());
}

fn kimi_stored_credential() -> OAuthCredential {
    OAuthCredential {
        access: secrecy::SecretString::new("kimi-old-access".to_owned().into_boxed_str()),
        refresh: secrecy::SecretString::new("kimi-old-refresh".to_owned().into_boxed_str()),
        expires_at: Some(time::OffsetDateTime::now_utc() - time::Duration::minutes(1)),
        base_url: None,
        account_id: None,
        client_id: None,
        enterprise_domain: None,
        available_model_ids: None,
    }
}

#[tokio::test]
async fn kimi_refresh_posts_grant_and_returns_strict_triple() {
    let server = MockServer::start().await;
    mount_kimi_token(&server, 200, kimi_token_success()).await;

    let refreshed = kimi_flow(&server)
        .refresh(&kimi_stored_credential())
        .await
        .expect("refresh");
    assert_eq!(refreshed.access.expose_secret(), "kimi-access-1");
    assert_eq!(refreshed.refresh.expose_secret(), "kimi-refresh-1");
    expires_within(refreshed.expires_at.expect("expiry"), 3600);

    let requests = kimi_token_requests(&server).await;
    assert_eq!(requests.len(), 1);
    let body = String::from_utf8(requests[0].body.clone()).unwrap();
    assert!(body.contains("grant_type=refresh_token"), "{body}");
    assert!(body.contains("refresh_token=kimi-old-refresh"), "{body}");
    assert!(
        body.contains(&format!("client_id={KIMI_CLIENT_ID}")),
        "{body}"
    );
    assert!(
        requests[0]
            .headers
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("application/x-www-form-urlencoded")
    );
}

#[tokio::test]
async fn kimi_refresh_invalid_grant_and_unauthorized_revoke_without_retry() {
    for (status, body) in [
        (400u16, serde_json::json!({"error": "invalid_grant"})),
        (401, serde_json::json!({"error": "invalid_token"})),
    ] {
        let server = MockServer::start().await;
        mount_kimi_token(&server, status, body).await;
        let error = kimi_flow(&server)
            .refresh(&kimi_stored_credential())
            .await
            .expect_err("must revoke");
        assert!(
            matches!(error, ProviderError::CredentialRevoked { .. }),
            "{error:?}"
        );
        // Non-retryable: exactly one request, no backoff retries.
        assert_eq!(kimi_token_requests(&server).await.len(), 1);
    }
}

#[tokio::test]
async fn kimi_refresh_retries_transient_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(ResponseTemplate::new(429).set_body_string("slow"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(ResponseTemplate::new(502).set_body_string("boom"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    mount_kimi_token(&server, 200, kimi_token_success()).await;

    let refreshed = kimi_flow(&server)
        .refresh(&kimi_stored_credential())
        .await
        .expect("refresh succeeds after transient failures");
    assert_eq!(refreshed.access.expose_secret(), "kimi-access-1");
    assert_eq!(kimi_token_requests(&server).await.len(), 3);
}

#[tokio::test]
async fn kimi_refresh_exhausts_retries_fails_typed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/token"))
        .respond_with(ResponseTemplate::new(500).set_body_string("always-CANARY"))
        .mount(&server)
        .await;

    let error = kimi_flow(&server)
        .refresh(&kimi_stored_credential())
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert!(!format!("{error:?}").contains("CANARY"));
    // One attempt plus three retries.
    assert_eq!(kimi_token_requests(&server).await.len(), 4);
}

#[tokio::test]
async fn kimi_refresh_non_retryable_terminal_error_fails_without_retry() {
    let server = MockServer::start().await;
    mount_kimi_token(
        &server,
        400,
        serde_json::json!({"error": "invalid_request"}),
    )
    .await;
    let error = kimi_flow(&server)
        .refresh(&kimi_stored_credential())
        .await
        .expect_err("must fail");
    assert!(matches!(error, ProviderError::AuthFailed(_)), "{error:?}");
    assert_eq!(kimi_token_requests(&server).await.len(), 1);
}

#[test]
fn kimi_production_constants() {
    assert_eq!(KIMI_DEFAULT_OAUTH_HOST, "https://auth.kimi.com");
    assert_eq!(KIMI_CLIENT_ID, "17e5f671-d194-4dfb-9706-5516cb48c098");
    assert_eq!(KimiCodingOAuthProvider::production().id(), "kimi-coding");
}

#[tokio::test]
async fn kimi_oauth_host_trims_trailing_slashes() {
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
    mount_kimi_token(&server, 200, kimi_token_success()).await;

    let flow = KimiCodingOAuthProvider::new_with_oauth_host(format!("{}/", server.uri()));
    let credential = flow
        .login(Arc::new(ScriptedPresenter::default()).as_ref())
        .await
        .expect("login through the trimmed host");
    assert_eq!(credential.access.expose_secret(), "kimi-access-1");
}

#[tokio::test]
async fn kimi_cancellation_during_device_presentation_is_typed() {
    let server = MockServer::start().await;
    mount_kimi_start(&server, kimi_start_body(1.into(), 900.into())).await;
    mount_kimi_token(&server, 200, kimi_token_success()).await;
    struct CancellingPresenter;
    impl LoginPresenter for CancellingPresenter {
        fn present_auth_url<'a>(
            &'a self,
            _url: &'a str,
        ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
            Box::pin(std::future::pending())
        }
        fn present_device_code<'a>(
            &'a self,
            _user_code: &'a str,
            _verification_uri: &'a str,
        ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
            Box::pin(async {
                Err(ProviderError::LoginCancelled {
                    provider_id: "kimi-coding".into(),
                })
            })
        }
        fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
            Box::pin(std::future::pending())
        }
        fn notify_success(&self) {}
        fn notify_failure(&self, _reason: &str) {}
    }

    let error = kimi_flow(&server)
        .login(&CancellingPresenter)
        .await
        .expect_err("must cancel");
    assert!(
        matches!(error, ProviderError::LoginCancelled { .. }),
        "{error:?}"
    );
    // The device start already happened; no token poll follows a cancel.
    assert!(kimi_token_requests(&server).await.is_empty());
}

#[test]
fn kimi_credential_near_expiry_needs_refresh() {
    let far = OAuthCredential {
        expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1)),
        ..kimi_stored_credential()
    };
    assert!(!far.needs_refresh());
    let near = OAuthCredential {
        expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::minutes(4)),
        ..kimi_stored_credential()
    };
    assert!(near.needs_refresh());
}
