use std::collections::BTreeMap;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opi_ai::auth::{LoginPresenter, OAuthLoginMethod};
use opi_ai::credential::{BoxAuthFuture, Credential, CredentialStore};
use opi_ai::provider::ProviderError;
use opi_ai::test_support::MockProvider;
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::credential_store::{
    BackendError, FakeKeyringBackend, KEYCHAIN_SERVICE, KeychainCredentialStore, KeyringBackend,
};
use opi_coding_agent::harness::CodingHarness;
use opi_coding_agent::interactive::{install_interactive_tui_test_driver, run_interactive_tui};
use opi_coding_agent::interactive_auth::{
    AuthCommandOutcome, AuthCommandServices, LoginTerminalControl, dispatch_auth_command,
};
use opi_coding_agent::oauth::{OAuthProviderRegistry, code_challenge_s256};
use opi_coding_agent::policy::ToolSelection;
use opi_coding_agent::runner::ExitCode;
use opi_tui::Keybindings;
use secrecy::ExposeSecret;
use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "common/phase14_auth_runtime.rs"]
mod phase14_auth_runtime;
use phase14_auth_runtime::{
    credential_runner, run_json_credential_capture, run_rpc_stdio_capture,
    run_text_credential_capture,
};

const SECRET_CANARY: &str = "AUTH-DO-NOT-LEAK";
const ANTHROPIC_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const COPILOT_CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";
const CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";

#[derive(Clone)]
struct OrderingBackend {
    inner: FakeKeyringBackend,
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl KeyringBackend for OrderingBackend {
    fn get(&self, service: &str, provider_id: &str) -> Result<Option<String>, BackendError> {
        self.inner.get(service, provider_id)
    }

    fn set(&self, service: &str, provider_id: &str, value: &str) -> Result<(), BackendError> {
        if service == KEYCHAIN_SERVICE {
            self.events.lock().unwrap().push("persist");
        }
        self.inner.set(service, provider_id, value)
    }

    fn delete(&self, service: &str, provider_id: &str) -> Result<(), BackendError> {
        self.inner.delete(service, provider_id)
    }
}

struct ConcretePresenter {
    method: Option<OAuthLoginMethod>,
    manual_code: Option<String>,
    fail_auth_url: bool,
    captured_urls: Mutex<Vec<String>>,
    captured_device_codes: Mutex<Vec<(String, String)>>,
    manual_calls: AtomicUsize,
    success_count: AtomicUsize,
    events: Arc<Mutex<Vec<&'static str>>>,
    failure_reasons: Mutex<Vec<String>>,
    cancel_on_auth_url: bool,
    cancel_on_device_present: bool,
    cancel_login: tokio::sync::Notify,
}

impl ConcretePresenter {
    fn browser(manual_code: &str, events: Arc<Mutex<Vec<&'static str>>>) -> Self {
        Self {
            method: Some(OAuthLoginMethod::Browser),
            manual_code: Some(manual_code.to_owned()),
            fail_auth_url: false,
            captured_urls: Mutex::new(Vec::new()),
            captured_device_codes: Mutex::new(Vec::new()),
            manual_calls: AtomicUsize::new(0),
            success_count: AtomicUsize::new(0),
            events,
            failure_reasons: Mutex::new(Vec::new()),
            cancel_on_auth_url: false,
            cancel_on_device_present: false,
            cancel_login: tokio::sync::Notify::new(),
        }
    }

    fn device(events: Arc<Mutex<Vec<&'static str>>>) -> Self {
        Self {
            method: Some(OAuthLoginMethod::DeviceCode),
            manual_code: None,
            fail_auth_url: false,
            captured_urls: Mutex::new(Vec::new()),
            captured_device_codes: Mutex::new(Vec::new()),
            manual_calls: AtomicUsize::new(0),
            success_count: AtomicUsize::new(0),
            events,
            failure_reasons: Mutex::new(Vec::new()),
            cancel_on_auth_url: false,
            cancel_on_device_present: false,
            cancel_login: tokio::sync::Notify::new(),
        }
    }
}

impl LoginPresenter for ConcretePresenter {
    fn select_login_method<'a>(
        &'a self,
        provider_id: &'a str,
        _methods: &'a [OAuthLoginMethod],
        _default: OAuthLoginMethod,
    ) -> BoxAuthFuture<'a, Result<OAuthLoginMethod, ProviderError>> {
        let method = self.method;
        let provider_id = provider_id.to_owned();
        Box::pin(async move { method.ok_or(ProviderError::LoginCancelled { provider_id }) })
    }

    fn present_auth_url<'a>(
        &'a self,
        url: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        self.captured_urls.lock().unwrap().push(url.to_owned());
        if self.cancel_on_auth_url {
            self.cancel_login.notify_one();
        }
        let fail = self.fail_auth_url;
        Box::pin(async move {
            if fail {
                Err(ProviderError::Config(SECRET_CANARY.to_owned()))
            } else {
                Ok(())
            }
        })
    }

    fn present_device_code<'a>(
        &'a self,
        user_code: &'a str,
        verification_uri: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        self.captured_device_codes
            .lock()
            .unwrap()
            .push((user_code.to_owned(), verification_uri.to_owned()));
        if self.cancel_on_device_present {
            self.cancel_login.notify_one();
        }
        Box::pin(async { Ok(()) })
    }

    fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
        self.manual_calls.fetch_add(1, Ordering::SeqCst);
        let code = self.manual_code.clone();
        Box::pin(async move {
            match code {
                Some(code) => Ok(code),
                None => std::future::pending::<Result<String, ProviderError>>().await,
            }
        })
    }

    fn await_login_cancelled<'a>(&'a self) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            self.cancel_login.notified().await;
            Ok(())
        })
    }

    fn notify_success(&self) {
        self.events.lock().unwrap().push("success");
        self.success_count.fetch_add(1, Ordering::SeqCst);
    }

    fn notify_failure(&self, reason: &str) {
        self.failure_reasons.lock().unwrap().push(reason.to_owned());
    }
}

fn concrete_store(
    root: &std::path::Path,
) -> (
    KeychainCredentialStore,
    FakeKeyringBackend,
    Arc<Mutex<Vec<&'static str>>>,
) {
    let backend = FakeKeyringBackend::new();
    let events = Arc::new(Mutex::new(Vec::new()));
    let store = KeychainCredentialStore::with_lock_timeout(
        Box::new(OrderingBackend {
            inner: backend.clone(),
            events: events.clone(),
        }),
        root.to_path_buf(),
        Duration::from_millis(250),
    );
    (store, backend, events)
}

fn concrete_services<'a>(
    store: &'a KeychainCredentialStore,
    presenter: &'a dyn LoginPresenter,
    server: &MockServer,
) -> AuthCommandServices<'a> {
    concrete_services_with_timeouts(
        store,
        presenter,
        server,
        Duration::from_secs(2),
        Duration::from_secs(2),
    )
}

fn concrete_services_with_timeouts<'a>(
    store: &'a KeychainCredentialStore,
    presenter: &'a dyn LoginPresenter,
    server: &MockServer,
    login_timeout: Duration,
    device_timeout: Duration,
) -> AuthCommandServices<'a> {
    AuthCommandServices::with_test_services(
        store,
        presenter,
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap(),
        server.uri(),
        login_timeout,
        device_timeout,
    )
}

fn codex_jwt(account_id: &str) -> String {
    use base64::Engine;
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&json!({
            "https://api.openai.com/auth": {
                "chatgpt_account_id": account_id,
            }
        }))
        .unwrap(),
    );
    format!("{header}.{payload}.synthetic-signature")
}

fn decoded_query_pairs(url: &str) -> BTreeMap<String, String> {
    let parsed = reqwest::Url::parse(url).expect("valid authorization URL");
    let pairs = parsed.query_pairs().into_owned().collect::<Vec<_>>();
    let result = pairs.iter().cloned().collect::<BTreeMap<_, _>>();
    assert_eq!(result.len(), pairs.len(), "query must not repeat keys");
    result
}

fn decoded_form_pairs(body: &[u8]) -> BTreeMap<String, String> {
    let body = std::str::from_utf8(body).expect("UTF-8 form body");
    let parsed =
        reqwest::Url::parse(&format!("http://form.invalid/?{body}")).expect("valid form body");
    let pairs = parsed.query_pairs().into_owned().collect::<Vec<_>>();
    let result = pairs.iter().cloned().collect::<BTreeMap<_, _>>();
    assert_eq!(result.len(), pairs.len(), "form body must not repeat keys");
    result
}

fn assert_url_safe_no_pad(value: &str, expected_len: usize) {
    assert_eq!(value.len(), expected_len);
    assert!(
        value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'),
        "expected unpadded base64url value: {value}"
    );
}

fn assert_loopback_redirect(redirect_uri: &str) {
    let redirect = reqwest::Url::parse(redirect_uri).expect("valid redirect URI");
    assert_eq!(redirect.scheme(), "http");
    assert_eq!(redirect.host_str(), Some("127.0.0.1"));
    assert!(redirect.port().is_some());
    assert_eq!(redirect.path(), "/");
    assert_eq!(redirect.query(), None);
    assert_eq!(redirect.fragment(), None);
}

fn assert_content_type(request: &wiremock::Request, expected: &str) {
    assert_eq!(
        request
            .headers
            .get("content-type")
            .expect("request content-type")
            .to_str()
            .unwrap(),
        expected
    );
}

async fn assert_stored_then_deleted(
    provider_id: &str,
    store: &KeychainCredentialStore,
    presenter: &ConcretePresenter,
    terminal: &mut RecordingTerminal,
    server: &MockServer,
) -> Credential {
    let outcome = dispatch_auth_command(
        &format!("/login {provider_id}"),
        terminal,
        concrete_services(store, presenter, server),
    )
    .await;
    assert_eq!(
        outcome,
        AuthCommandOutcome::LoggedIn {
            provider_id: provider_id.to_owned(),
        }
    );
    let stored = store
        .read(provider_id)
        .await
        .unwrap()
        .expect("concrete dispatch persisted an OAuth credential");
    let events = presenter.events.lock().unwrap().clone();
    assert_eq!(events, ["persist", "success"]);

    let outcome = dispatch_auth_command(
        &format!("/logout {provider_id}"),
        terminal,
        concrete_services(store, presenter, server),
    )
    .await;
    assert_eq!(
        outcome,
        AuthCommandOutcome::LoggedOut {
            provider_id: provider_id.to_owned(),
        }
    );
    assert!(store.read(provider_id).await.unwrap().is_none());
    stored
}

#[tokio::test]
async fn dispatcher_runs_concrete_anthropic_login_and_logout() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/anthropic/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "anthropic-access",
            "refresh_token": "anthropic-refresh",
            "expires_in": 3600
        })))
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let (store, _backend, events) = concrete_store(root.path());
    let presenter = ConcretePresenter::browser("anthropic-code", events);
    let mut terminal = RecordingTerminal::default();

    let stored =
        assert_stored_then_deleted("anthropic", &store, &presenter, &mut terminal, &server).await;

    let Credential::OAuthToken {
        access,
        refresh,
        base_url,
        account_id,
        ..
    } = stored
    else {
        panic!("Anthropic dispatcher stored the wrong credential kind");
    };
    assert_eq!(access.expose_secret(), "anthropic-access");
    assert_eq!(refresh.expose_secret(), "anthropic-refresh");
    assert_eq!(base_url, None);
    assert_eq!(account_id, None);
    let authorize_url = presenter.captured_urls.lock().unwrap()[0].clone();
    assert_eq!(
        authorize_url.split_once('?').unwrap().0,
        format!("{}/anthropic/authorize", server.uri())
    );
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method.as_str(), "POST");
    assert_eq!(requests[0].url.path(), "/anthropic/token");
    assert_content_type(&requests[0], "application/x-www-form-urlencoded");
    let token_form = decoded_form_pairs(&requests[0].body);
    let redirect_uri = token_form["redirect_uri"].clone();
    let verifier = token_form["code_verifier"].clone();
    assert_loopback_redirect(&redirect_uri);
    assert_url_safe_no_pad(&verifier, 64);
    assert_eq!(
        token_form,
        BTreeMap::from([
            ("client_id".to_owned(), ANTHROPIC_CLIENT_ID.to_owned()),
            ("code".to_owned(), "anthropic-code".to_owned()),
            ("code_verifier".to_owned(), verifier.clone()),
            ("grant_type".to_owned(), "authorization_code".to_owned()),
            ("redirect_uri".to_owned(), redirect_uri.clone()),
        ])
    );
    let authorize_query = decoded_query_pairs(&authorize_url);
    let state = authorize_query["state"].clone();
    let challenge = authorize_query["code_challenge"].clone();
    assert_url_safe_no_pad(&state, 32);
    assert_url_safe_no_pad(&challenge, 43);
    assert_eq!(challenge, code_challenge_s256(&verifier));
    assert_eq!(
        authorize_query,
        BTreeMap::from([
            ("client_id".to_owned(), ANTHROPIC_CLIENT_ID.to_owned()),
            ("code".to_owned(), "true".to_owned()),
            ("code_challenge".to_owned(), challenge),
            ("code_challenge_method".to_owned(), "S256".to_owned()),
            ("redirect_uri".to_owned(), redirect_uri),
            ("response_type".to_owned(), "code".to_owned()),
            (
                "scope".to_owned(),
                "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers user:file_upload".to_owned(),
            ),
            ("state".to_owned(), state),
        ])
    );
    assert_eq!(terminal.transitions, ["suspend", "resume"]);
}

#[tokio::test]
async fn dispatcher_runs_concrete_github_copilot_login_and_logout() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/copilot/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_code": "copilot-device-secret",
            "user_code": "COPILOT-PUBLIC",
            "verification_uri": format!("{}/copilot/verify", server.uri()),
            "interval": 0
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/copilot/oauth/token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"access_token":"github-token"})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/copilot/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "token": "copilot-access",
            "expires_at": time::OffsetDateTime::now_utc().unix_timestamp() + 3600,
            "endpoints": {"api":"https://api.githubcopilot.test"}
        })))
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let (store, _backend, events) = concrete_store(root.path());
    let presenter = ConcretePresenter::device(events);
    let mut terminal = RecordingTerminal::default();

    let stored =
        assert_stored_then_deleted("github-copilot", &store, &presenter, &mut terminal, &server)
            .await;

    let Credential::OAuthToken {
        access,
        refresh,
        base_url,
        ..
    } = stored
    else {
        panic!("Copilot dispatcher stored the wrong credential kind");
    };
    assert_eq!(access.expose_secret(), "copilot-access");
    assert_eq!(refresh.expose_secret(), "github-token");
    assert_eq!(base_url.as_deref(), Some("https://api.githubcopilot.test"));
    assert_eq!(
        presenter.captured_device_codes.lock().unwrap().as_slice(),
        &[(
            "COPILOT-PUBLIC".to_owned(),
            format!("{}/copilot/verify", server.uri()),
        )]
    );
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests
            .iter()
            .map(|request| (request.method.as_str(), request.url.path()))
            .collect::<Vec<_>>(),
        [
            ("POST", "/copilot/device/code"),
            ("POST", "/copilot/oauth/token"),
            ("GET", "/copilot/token"),
        ]
    );
    assert_eq!(
        decoded_form_pairs(&requests[0].body),
        BTreeMap::from([
            ("client_id".to_owned(), COPILOT_CLIENT_ID.to_owned()),
            ("scope".to_owned(), "read:user".to_owned()),
        ])
    );
    assert_eq!(
        decoded_form_pairs(&requests[1].body),
        BTreeMap::from([
            ("client_id".to_owned(), COPILOT_CLIENT_ID.to_owned()),
            ("device_code".to_owned(), "copilot-device-secret".to_owned(),),
            (
                "grant_type".to_owned(),
                "urn:ietf:params:oauth:grant-type:device_code".to_owned(),
            ),
        ])
    );
    for request in &requests[..2] {
        assert_content_type(request, "application/x-www-form-urlencoded");
        assert_eq!(
            request.headers.get("accept").unwrap().to_str().unwrap(),
            "application/json"
        );
        assert_eq!(
            request.headers.get("user-agent").unwrap().to_str().unwrap(),
            "GitHubCopilotChat/0.35.0"
        );
    }
    assert_eq!(
        requests[2]
            .headers
            .get("authorization")
            .unwrap()
            .to_str()
            .unwrap(),
        "Bearer github-token"
    );
    for (header, expected) in [
        ("accept", "application/json"),
        ("user-agent", "GitHubCopilotChat/0.35.0"),
        ("editor-version", "vscode/1.107.0"),
        ("editor-plugin-version", "copilot-chat/0.35.0"),
        ("copilot-integration-id", "vscode-chat"),
    ] {
        assert_eq!(
            requests[2]
                .headers
                .get(header)
                .unwrap_or_else(|| panic!("missing {header}"))
                .to_str()
                .unwrap(),
            expected
        );
    }
    assert_eq!(presenter.manual_calls.load(Ordering::SeqCst), 0);
    assert_eq!(terminal.transitions, ["suspend", "resume"]);
}

#[tokio::test]
async fn dispatcher_runs_concrete_openai_codex_browser_login_and_logout() {
    let server = MockServer::start().await;
    let access = codex_jwt("browser-account");
    Mock::given(method("POST"))
        .and(path("/codex/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": access,
            "refresh_token": "codex-browser-refresh",
            "expires_in": 3600
        })))
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let (store, _backend, events) = concrete_store(root.path());
    let presenter = ConcretePresenter::browser("codex-browser-code", events);
    let mut terminal = RecordingTerminal::default();

    let stored =
        assert_stored_then_deleted("openai-codex", &store, &presenter, &mut terminal, &server)
            .await;

    let Credential::OAuthToken {
        account_id,
        refresh,
        ..
    } = stored
    else {
        panic!("Codex dispatcher stored the wrong credential kind");
    };
    assert_eq!(account_id.as_deref(), Some("browser-account"));
    assert_eq!(refresh.expose_secret(), "codex-browser-refresh");
    let authorize_url = presenter.captured_urls.lock().unwrap()[0].clone();
    assert_eq!(
        authorize_url.split_once('?').unwrap().0,
        format!("{}/codex/authorize", server.uri())
    );
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method.as_str(), "POST");
    assert_eq!(requests[0].url.path(), "/codex/token");
    assert_content_type(&requests[0], "application/x-www-form-urlencoded");
    let token_form = decoded_form_pairs(&requests[0].body);
    let redirect_uri = token_form["redirect_uri"].clone();
    let verifier = token_form["code_verifier"].clone();
    assert_loopback_redirect(&redirect_uri);
    assert_url_safe_no_pad(&verifier, 64);
    assert_eq!(
        token_form,
        BTreeMap::from([
            ("client_id".to_owned(), CODEX_CLIENT_ID.to_owned()),
            ("code".to_owned(), "codex-browser-code".to_owned()),
            ("code_verifier".to_owned(), verifier.clone()),
            ("grant_type".to_owned(), "authorization_code".to_owned()),
            ("redirect_uri".to_owned(), redirect_uri.clone()),
        ])
    );
    let authorize_query = decoded_query_pairs(&authorize_url);
    let state = authorize_query["state"].clone();
    let challenge = authorize_query["code_challenge"].clone();
    assert_url_safe_no_pad(&state, 32);
    assert_url_safe_no_pad(&challenge, 43);
    assert_eq!(challenge, code_challenge_s256(&verifier));
    assert_eq!(
        authorize_query,
        BTreeMap::from([
            ("client_id".to_owned(), CODEX_CLIENT_ID.to_owned()),
            ("code_challenge".to_owned(), challenge),
            ("code_challenge_method".to_owned(), "S256".to_owned()),
            ("codex_cli_simplified_flow".to_owned(), "true".to_owned(),),
            ("id_token_add_organizations".to_owned(), "true".to_owned(),),
            ("originator".to_owned(), "opi".to_owned()),
            ("redirect_uri".to_owned(), redirect_uri),
            ("response_type".to_owned(), "code".to_owned()),
            (
                "scope".to_owned(),
                "openid profile email offline_access".to_owned(),
            ),
            ("state".to_owned(), state),
        ])
    );
    assert_eq!(terminal.transitions, ["suspend", "resume"]);
}

#[tokio::test]
async fn dispatcher_runs_concrete_openai_codex_device_login_and_logout() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/codex/device/usercode"))
        .and(body_json(json!({"client_id": CODEX_CLIENT_ID})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_auth_id": "codex-device-secret",
            "user_code": "CODEX-PUBLIC",
            "interval": 0
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/codex/device/token"))
        .and(body_json(json!({
            "device_auth_id": "codex-device-secret",
            "user_code": "CODEX-PUBLIC"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_code": "codex-device-authorization",
            "code_verifier": "codex-device-verifier"
        })))
        .mount(&server)
        .await;
    let access = codex_jwt("device-account");
    Mock::given(method("POST"))
        .and(path("/codex/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": access,
            "refresh_token": "codex-device-refresh",
            "expires_in": 3600
        })))
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let (store, _backend, events) = concrete_store(root.path());
    let presenter = ConcretePresenter::device(events);
    let mut terminal = RecordingTerminal::default();

    let stored =
        assert_stored_then_deleted("openai-codex", &store, &presenter, &mut terminal, &server)
            .await;

    let Credential::OAuthToken { account_id, .. } = stored else {
        panic!("Codex dispatcher stored the wrong credential kind");
    };
    assert_eq!(account_id.as_deref(), Some("device-account"));
    assert_eq!(
        presenter.captured_device_codes.lock().unwrap().as_slice(),
        &[(
            "CODEX-PUBLIC".to_owned(),
            format!("{}/codex/device", server.uri())
        )]
    );
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests
            .iter()
            .map(|request| (request.method.as_str(), request.url.path()))
            .collect::<Vec<_>>(),
        [
            ("POST", "/codex/device/usercode"),
            ("POST", "/codex/device/token"),
            ("POST", "/codex/token"),
        ]
    );
    assert_content_type(&requests[0], "application/json");
    assert_content_type(&requests[1], "application/json");
    assert_content_type(&requests[2], "application/x-www-form-urlencoded");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&requests[0].body).unwrap(),
        json!({"client_id": CODEX_CLIENT_ID})
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&requests[1].body).unwrap(),
        json!({
            "device_auth_id": "codex-device-secret",
            "user_code": "CODEX-PUBLIC",
        })
    );
    assert_eq!(
        decoded_form_pairs(&requests[2].body),
        BTreeMap::from([
            ("client_id".to_owned(), CODEX_CLIENT_ID.to_owned()),
            ("code".to_owned(), "codex-device-authorization".to_owned(),),
            (
                "code_verifier".to_owned(),
                "codex-device-verifier".to_owned(),
            ),
            ("grant_type".to_owned(), "authorization_code".to_owned()),
            (
                "redirect_uri".to_owned(),
                "https://auth.openai.com/deviceauth/callback".to_owned(),
            ),
        ])
    );
    assert_eq!(presenter.manual_calls.load(Ordering::SeqCst), 0);
    assert_eq!(terminal.transitions, ["suspend", "resume"]);
}

async fn mount_anthropic_token(server: &MockServer, status: u16) {
    Mock::given(method("POST"))
        .and(path("/anthropic/token"))
        .respond_with(
            ResponseTemplate::new(status).set_body_json(if status == 200 {
                json!({
                    "access_token": SECRET_CANARY,
                    "refresh_token": "anthropic-refresh",
                    "expires_in": 3600
                })
            } else {
                json!({
                    "error": "server_error",
                    "error_description": SECRET_CANARY
                })
            }),
        )
        .mount(server)
        .await;
}

fn assert_one_terminal_cycle(terminal: &RecordingTerminal) {
    assert_eq!(terminal.transitions, ["suspend", "resume"]);
}

#[tokio::test]
async fn dispatcher_restores_terminal_once_on_every_concrete_exit() {
    // Success.
    let server = MockServer::start().await;
    mount_anthropic_token(&server, 200).await;
    let root = tempfile::tempdir().unwrap();
    let (store, _backend, events) = concrete_store(root.path());
    let presenter = ConcretePresenter::browser("success-code", events);
    let mut terminal = RecordingTerminal::default();
    assert!(matches!(
        dispatch_auth_command(
            "/login anthropic",
            &mut terminal,
            concrete_services(&store, &presenter, &server),
        )
        .await,
        AuthCommandOutcome::LoggedIn { .. }
    ));
    assert_one_terminal_cycle(&terminal);

    // Provider failure.
    let server = MockServer::start().await;
    mount_anthropic_token(&server, 500).await;
    let root = tempfile::tempdir().unwrap();
    let (store, _backend, events) = concrete_store(root.path());
    let presenter = ConcretePresenter::browser("failure-code", events);
    let mut terminal = RecordingTerminal::default();
    assert!(matches!(
        dispatch_auth_command(
            "/login anthropic",
            &mut terminal,
            concrete_services(&store, &presenter, &server),
        )
        .await,
        AuthCommandOutcome::Failed { .. }
    ));
    assert_one_terminal_cycle(&terminal);

    // Login-method selection cancellation.
    let server = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let (store, _backend, events) = concrete_store(root.path());
    let presenter = ConcretePresenter {
        method: None,
        ..ConcretePresenter::browser("unused", events)
    };
    let mut terminal = RecordingTerminal::default();
    assert_eq!(
        dispatch_auth_command(
            "/login openai-codex",
            &mut terminal,
            concrete_services(&store, &presenter, &server),
        )
        .await,
        AuthCommandOutcome::Failed {
            message: "authentication cancelled for provider 'openai-codex'".to_owned(),
        }
    );
    assert_one_terminal_cycle(&terminal);

    // Active Device Code cancellation.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/codex/device/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_auth_id":"cancel-device",
            "user_code":"CANCEL-CODE",
            "interval":60
        })))
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let (store, _backend, events) = concrete_store(root.path());
    let presenter = ConcretePresenter {
        cancel_on_device_present: true,
        ..ConcretePresenter::device(events)
    };
    let mut terminal = RecordingTerminal::default();
    assert_eq!(
        dispatch_auth_command(
            "/login openai-codex",
            &mut terminal,
            concrete_services(&store, &presenter, &server),
        )
        .await,
        AuthCommandOutcome::Failed {
            message: "authentication cancelled for provider 'openai-codex'".to_owned(),
        }
    );
    let requests = server.received_requests().await.unwrap();
    assert!(
        requests
            .iter()
            .any(|request| request.url.path() == "/codex/device/usercode")
    );
    assert!(
        requests
            .iter()
            .all(|request| request.url.path() != "/codex/token")
    );
    assert!(store.read("openai-codex").await.unwrap().is_none());
    assert_one_terminal_cycle(&terminal);

    // Presenter failure.
    let server = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let (store, _backend, events) = concrete_store(root.path());
    let presenter = ConcretePresenter {
        fail_auth_url: true,
        ..ConcretePresenter::browser("presenter-code", events)
    };
    let mut terminal = RecordingTerminal::default();
    assert_eq!(
        dispatch_auth_command(
            "/login anthropic",
            &mut terminal,
            concrete_services(&store, &presenter, &server),
        )
        .await,
        AuthCommandOutcome::Failed {
            message: "authentication configuration failed".to_owned(),
        }
    );
    assert_one_terminal_cycle(&terminal);

    // OAuth timeout.
    let server = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let (store, _backend, events) = concrete_store(root.path());
    let presenter = ConcretePresenter {
        manual_code: None,
        ..ConcretePresenter::browser("unused", events)
    };
    let mut terminal = RecordingTerminal::default();
    assert_eq!(
        dispatch_auth_command(
            "/login anthropic",
            &mut terminal,
            concrete_services_with_timeouts(
                &store,
                &presenter,
                &server,
                Duration::from_millis(20),
                Duration::from_secs(2),
            ),
        )
        .await,
        AuthCommandOutcome::Failed {
            message: "authentication timed out".to_owned(),
        }
    );
    assert_one_terminal_cycle(&terminal);

    // Store failure after a concrete provider succeeds.
    let server = MockServer::start().await;
    mount_anthropic_token(&server, 200).await;
    let root = tempfile::tempdir().unwrap();
    let store = KeychainCredentialStore::with_lock_timeout(
        Box::new(FakeKeyringBackend::new().with_unavailable()),
        root.path().to_path_buf(),
        Duration::from_millis(50),
    );
    let events = Arc::new(Mutex::new(Vec::new()));
    let presenter = ConcretePresenter::browser("store-code", events);
    let mut terminal = RecordingTerminal::default();
    assert!(matches!(
        dispatch_auth_command(
            "/login anthropic",
            &mut terminal,
            concrete_services(&store, &presenter, &server),
        )
        .await,
        AuthCommandOutcome::Failed { .. }
    ));
    assert_one_terminal_cycle(&terminal);

    // Lock failure after a concrete provider succeeds.
    let server = MockServer::start().await;
    mount_anthropic_token(&server, 200).await;
    let root = tempfile::tempdir().unwrap();
    let store = test_store(root.path(), Duration::from_millis(20));
    let lock_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(root.path().join("credential.lock"))
        .unwrap();
    fs4::FileExt::lock(&lock_file).unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    let presenter = ConcretePresenter::browser("lock-code", events);
    let mut terminal = RecordingTerminal::default();
    assert!(matches!(
        dispatch_auth_command(
            "/login anthropic",
            &mut terminal,
            concrete_services(&store, &presenter, &server),
        )
        .await,
        AuthCommandOutcome::Failed { .. }
    ));
    fs4::FileExt::unlock(&lock_file).unwrap();
    assert_one_terminal_cycle(&terminal);

    // Dropping an in-flight concrete login.
    let server = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let (store, _backend, events) = concrete_store(root.path());
    let presenter = ConcretePresenter {
        manual_code: None,
        ..ConcretePresenter::browser("unused", events)
    };
    let mut terminal = RecordingTerminal::default();
    let mut login = Box::pin(dispatch_auth_command(
        "/login anthropic",
        &mut terminal,
        concrete_services_with_timeouts(
            &store,
            &presenter,
            &server,
            Duration::from_secs(60),
            Duration::from_secs(60),
        ),
    ));
    assert!(
        tokio::time::timeout(Duration::from_millis(20), login.as_mut())
            .await
            .is_err()
    );
    drop(login);
    assert_one_terminal_cycle(&terminal);
}

#[tokio::test]
async fn browser_and_copilot_cancellation_restore_terminal_without_persistence() {
    for provider_id in ["anthropic", "openai-codex"] {
        let server = MockServer::start().await;
        let root = tempfile::tempdir().unwrap();
        let (store, _backend, events) = concrete_store(root.path());
        let presenter = ConcretePresenter {
            cancel_on_auth_url: true,
            manual_code: None,
            ..ConcretePresenter::browser("unused", events)
        };
        let mut terminal = RecordingTerminal::default();

        let outcome = dispatch_auth_command(
            &format!("/login {provider_id}"),
            &mut terminal,
            concrete_services(&store, &presenter, &server),
        )
        .await;

        assert_eq!(
            outcome,
            AuthCommandOutcome::Failed {
                message: format!("authentication cancelled for provider '{provider_id}'"),
            }
        );
        assert_eq!(
            presenter.failure_reasons.lock().unwrap().as_slice(),
            &["login cancelled"]
        );
        assert!(store.read(provider_id).await.unwrap().is_none());
        assert!(
            server.received_requests().await.unwrap().is_empty(),
            "{provider_id} exchanged a code after cancellation"
        );
        assert_one_terminal_cycle(&terminal);
    }

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/copilot/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_code": "copilot-device",
            "user_code": "COPILOT-CODE",
            "verification_uri": "https://example.invalid/device",
            "interval": 60
        })))
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let (store, _backend, events) = concrete_store(root.path());
    let presenter = ConcretePresenter {
        cancel_on_device_present: true,
        ..ConcretePresenter::device(events)
    };
    let mut terminal = RecordingTerminal::default();

    let outcome = dispatch_auth_command(
        "/login github-copilot",
        &mut terminal,
        concrete_services(&store, &presenter, &server),
    )
    .await;

    assert_eq!(
        outcome,
        AuthCommandOutcome::Failed {
            message: "authentication cancelled for provider 'github-copilot'".to_owned(),
        }
    );
    assert_eq!(
        presenter.failure_reasons.lock().unwrap().as_slice(),
        &["login cancelled"]
    );
    assert!(store.read("github-copilot").await.unwrap().is_none());
    assert!(
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .all(|request| request.url.path() != "/copilot/token")
    );
    assert_one_terminal_cycle(&terminal);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dispatcher_store_and_lock_failures_are_typed_redacted_and_unsuccessful() {
    for lock_held in [false, true] {
        let server = MockServer::start().await;
        mount_anthropic_token(&server, 200).await;
        let root = tempfile::tempdir().unwrap();
        let backend = if lock_held {
            FakeKeyringBackend::new()
        } else {
            FakeKeyringBackend::new().with_unavailable()
        };
        let store = KeychainCredentialStore::with_lock_timeout(
            Box::new(backend),
            root.path().to_path_buf(),
            Duration::from_millis(25),
        );
        let lock_file = lock_held.then(|| {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .read(true)
                .write(true)
                .open(root.path().join("credential.lock"))
                .unwrap();
            fs4::FileExt::lock(&file).unwrap();
            file
        });
        let events = Arc::new(Mutex::new(Vec::new()));
        let presenter = ConcretePresenter::browser(SECRET_CANARY, events);
        let mut terminal = RecordingTerminal::default();

        let outcome = dispatch_auth_command(
            "/login anthropic",
            &mut terminal,
            concrete_services(&store, &presenter, &server),
        )
        .await;
        if let Some(file) = &lock_file {
            fs4::FileExt::unlock(file).unwrap();
        }

        let AuthCommandOutcome::Failed { message } = outcome else {
            panic!("store/lock failure reported success: {outcome:?}");
        };
        assert_eq!(message, "credential store operation failed");
        assert!(!message.contains(SECRET_CANARY));
        assert_eq!(presenter.success_count.load(Ordering::SeqCst), 0);
        assert!(store.read("anthropic").await.is_err() || lock_held);
        assert_one_terminal_cycle(&terminal);
    }
}

#[tokio::test]
async fn dispatcher_device_flows_never_call_manual_code() {
    let copilot_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/copilot/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_code": "copilot-device",
            "user_code": "COPILOT-CODE",
            "verification_uri": format!("{}/copilot/verify", copilot_server.uri()),
            "interval": 0
        })))
        .mount(&copilot_server)
        .await;
    Mock::given(method("POST"))
        .and(path("/copilot/oauth/token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"access_token":"github-token"})),
        )
        .mount(&copilot_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/copilot/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "token":"copilot-token",
            "expires_at":time::OffsetDateTime::now_utc().unix_timestamp() + 3600
        })))
        .mount(&copilot_server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let (store, _backend, events) = concrete_store(root.path());
    let presenter = ConcretePresenter::device(events);
    let mut terminal = RecordingTerminal::default();
    assert!(matches!(
        dispatch_auth_command(
            "/login github-copilot",
            &mut terminal,
            concrete_services(&store, &presenter, &copilot_server),
        )
        .await,
        AuthCommandOutcome::LoggedIn { .. }
    ));
    assert_eq!(presenter.manual_calls.load(Ordering::SeqCst), 0);

    let codex_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/codex/device/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_auth_id":"codex-device",
            "user_code":"CODEX-CODE",
            "interval":0
        })))
        .mount(&codex_server)
        .await;
    Mock::given(method("POST"))
        .and(path("/codex/device/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_code":"codex-authorization",
            "code_verifier":"codex-verifier"
        })))
        .mount(&codex_server)
        .await;
    Mock::given(method("POST"))
        .and(path("/codex/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token":codex_jwt("device-no-manual"),
            "refresh_token":"codex-refresh",
            "expires_in":3600
        })))
        .mount(&codex_server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let (store, _backend, events) = concrete_store(root.path());
    let presenter = ConcretePresenter::device(events);
    let mut terminal = RecordingTerminal::default();
    assert!(matches!(
        dispatch_auth_command(
            "/login openai-codex",
            &mut terminal,
            concrete_services(&store, &presenter, &codex_server),
        )
        .await,
        AuthCommandOutcome::LoggedIn { .. }
    ));
    assert_eq!(presenter.manual_calls.load(Ordering::SeqCst), 0);
}

#[derive(Default)]
struct RecordingTerminal {
    transitions: Vec<&'static str>,
    fail_suspend: bool,
    fail_resume: bool,
}

impl LoginTerminalControl for RecordingTerminal {
    fn suspend_for_login(&mut self) -> io::Result<()> {
        self.transitions.push("suspend");
        if self.fail_suspend {
            Err(io::Error::other(SECRET_CANARY))
        } else {
            Ok(())
        }
    }

    fn resume_after_login(&mut self) -> io::Result<()> {
        self.transitions.push("resume");
        if self.fail_resume {
            Err(io::Error::other(SECRET_CANARY))
        } else {
            Ok(())
        }
    }
}

fn test_store(root: &std::path::Path, timeout: Duration) -> KeychainCredentialStore {
    KeychainCredentialStore::with_lock_timeout(
        Box::new(FakeKeyringBackend::new()),
        root.to_path_buf(),
        timeout,
    )
}

#[tokio::test]
async fn interactive_auth_dispatcher_reports_terminal_restore_failures_once() {
    let server = MockServer::start().await;
    mount_anthropic_token(&server, 200).await;
    let root = tempfile::tempdir().unwrap();
    let (store, _backend, events) = concrete_store(root.path());
    let presenter = ConcretePresenter::browser("terminal-code", events);
    let mut terminal = RecordingTerminal {
        fail_suspend: true,
        fail_resume: true,
        ..Default::default()
    };

    let outcome = dispatch_auth_command(
        "/login anthropic",
        &mut terminal,
        concrete_services(&store, &presenter, &server),
    )
    .await;

    assert_eq!(
        outcome,
        AuthCommandOutcome::Failed {
            message: "terminal restore failed".to_owned(),
        }
    );
    assert_eq!(terminal.transitions, ["suspend", "resume"]);
    assert_eq!(presenter.success_count.load(Ordering::SeqCst), 0);

    let root = tempfile::tempdir().unwrap();
    let (store, _backend, events) = concrete_store(root.path());
    let presenter = ConcretePresenter::browser("terminal-code", events);
    let mut terminal = RecordingTerminal {
        fail_resume: true,
        ..Default::default()
    };

    let outcome = dispatch_auth_command(
        "/login anthropic",
        &mut terminal,
        concrete_services(&store, &presenter, &server),
    )
    .await;

    assert_eq!(
        outcome,
        AuthCommandOutcome::Failed {
            message: "terminal restore failed".to_owned(),
        }
    );
    assert_eq!(terminal.transitions, ["suspend", "resume"]);
    assert_eq!(presenter.success_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn interactive_auth_dispatcher_parses_the_reviewed_command_forms() {
    let server = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let (store, _backend, events) = concrete_store(root.path());
    let presenter = ConcretePresenter::browser("unused", events);
    let mut terminal = RecordingTerminal::default();
    let cases = [
        (
            "/login",
            AuthCommandOutcome::Usage("usage: /login <provider>".to_owned()),
        ),
        (
            "/logout",
            AuthCommandOutcome::Usage("usage: /logout <provider>".to_owned()),
        ),
        (
            "  /logout   anthropic  ",
            AuthCommandOutcome::LoggedOut {
                provider_id: "anthropic".to_owned(),
            },
        ),
        ("/model", AuthCommandOutcome::NotHandled),
        (
            "/login anthropic extra",
            AuthCommandOutcome::Failed {
                message: "authentication configuration failed".to_owned(),
            },
        ),
    ];

    for (input, expected) in cases {
        let outcome = dispatch_auth_command(
            input,
            &mut terminal,
            concrete_services(&store, &presenter, &server),
        )
        .await;
        assert_eq!(outcome, expected, "input: {input:?}");
    }
    assert_eq!(terminal.transitions, ["suspend", "resume"]);
}

#[tokio::test(flavor = "current_thread")]
async fn interactive_auth_help_is_discoverable_through_dispatcher() {
    let workspace = tempfile::tempdir().unwrap();
    let store = Arc::new(test_store(workspace.path(), Duration::from_secs(1)));
    let registry = OAuthProviderRegistry::registry_with_builtins();
    let provider = MockProvider::new("mock", Vec::new());
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .tool_selection(ToolSelection::Disabled)
    .build();
    harness.credential_store = Some(store);
    harness.oauth_registry = Some(registry);

    let driver = install_interactive_tui_test_driver(["/help", "exit"])
        .expect("the debug-only headless TUI driver installs once");
    run_interactive_tui(
        harness,
        "mock:mock-model".into(),
        "default",
        Keybindings::default(),
    )
    .await
    .expect("outer interactive entry point handles scripted auth input");
    let capture = driver.capture();
    let help = capture
        .system_messages
        .iter()
        .find(|message| message.contains("/login <provider>"))
        .expect("/help must render production command help through run_interactive_tui");

    for (command, description) in [
        (
            "/login <provider>",
            "authenticate and persist an OAuth credential",
        ),
        ("/logout <provider>", "delete the persisted credential"),
    ] {
        assert!(help.contains(command), "missing command {command}: {help}");
        assert!(
            help.contains(description),
            "missing description {description}: {help}"
        );
    }
    assert!(capture.terminal_transitions.is_empty());

    let json_runner = credential_runner(workspace.path());
    let json = run_json_credential_capture(json_runner).await;
    let text_runner = credential_runner(workspace.path());
    let text = run_text_credential_capture(text_runner).await;
    let rpc = run_rpc_stdio_capture("phase14_rpc_run_stdio_child");

    assert_eq!(json["exit_code"], ExitCode::AuthFailure as i32);
    let json_remediations: Vec<_> = json["stdout"]
        .as_str()
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .filter(|line| line["type"] == "CredentialNeeded")
        .collect();
    assert_eq!(
        json_remediations.len(),
        1,
        "NDJSON must emit exactly one CredentialNeeded event"
    );
    assert_eq!(json_remediations[0]["provider_id"], "anthropic");
    assert_eq!(json_remediations[0]["remediation"], "/login anthropic");
    assert_eq!(text["exit_code"], ExitCode::AuthFailure as i32);
    assert!(text["stderr"].as_str().unwrap().contains("anthropic"));
    assert!(
        text["stderr"]
            .as_str()
            .unwrap()
            .contains("/login anthropic")
    );
    let rpc_remediations: Vec<_> = rpc
        .iter()
        .filter(|line| line["type"] == "CredentialNeeded")
        .collect();
    assert_eq!(
        rpc_remediations.len(),
        1,
        "RPC must emit exactly one CredentialNeeded event"
    );
    let remediation = rpc_remediations[0];
    assert_eq!(remediation["provider_id"], "anthropic");
    assert_eq!(remediation["remediation"], "/login anthropic");

    if let Some(root) = std::env::var_os("OPI_TEST_ARTIFACT_DIR") {
        let protocol = std::path::PathBuf::from(root).join("protocol");
        std::fs::create_dir_all(&protocol).expect("create protocol artifact directory");
        std::fs::write(
            protocol.join("runtime-auth-help-tui.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "entry_point": "run_interactive_tui",
                "capture": capture,
            }))
            .unwrap(),
        )
        .unwrap();
        std::fs::write(
            protocol.join("runtime-auth-help-ndjson-metadata.json"),
            serde_json::to_vec_pretty(&json).unwrap(),
        )
        .unwrap();
        std::fs::write(
            protocol.join("run-runtime-auth-help.ndjson"),
            json["stdout"].as_str().unwrap(),
        )
        .unwrap();
        std::fs::write(
            protocol.join("runtime-auth-help-text.json"),
            serde_json::to_vec_pretty(&text).unwrap(),
        )
        .unwrap();
        std::fs::write(
            protocol.join("runtime-auth-help-rpc.jsonl"),
            rpc.iter()
                .map(serde_json::Value::to_string)
                .collect::<Vec<_>>()
                .join("\n")
                + "\n",
        )
        .unwrap();
    }
}

#[tokio::test]
#[ignore = "subprocess-only RPC stdio entry point"]
async fn phase14_rpc_run_stdio_child() {
    phase14_auth_runtime::run_rpc_stdio_child().await;
}
