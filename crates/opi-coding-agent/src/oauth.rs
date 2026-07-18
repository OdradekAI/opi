//! OAuth provider registry + PKCE/device-code flows (Phase 14.2).
//!
//! Owns the concrete `OAuthProvider` implementations (Anthropic PKCE, GitHub
//! Copilot device-code, OpenAI Codex browser/device-code), the
//! `OAuthProviderRegistry`, and the production `TuiLoginPresenter`. All flow HTTP is mockable: authorize/
//! token endpoints are configurable so tests point them at a `wiremock` server,
//! and the `LoginPresenter` is an injected seam.
//!
//! # Secret handling
//!
//! Authorization codes, access/refresh tokens, and the device-code are secrets.
//! They are never passed to any `LoginPresenter` method (only the public
//! `user_code` and `verification_uri` are shown), never interpolated into
//! `notify_failure` reasons or `ProviderError` messages, and never written into
//! the loopback callback response. Token-endpoint error bodies are parsed down
//! to the OAuth `{error, error_description}` fields before any message is
//! built; the raw body (which could echo a submitted `code_verifier` or
//! `refresh_token`) is never surfaced. Token POSTs use a client with
//! `redirect::Policy::none()` so a 302 echo-redirect cannot leak the verifier.
//!
//! # Unstable
//!
//! Part of the **unstable 0.x extension substrate**. Breaking changes may occur
//! between minor versions without a major version bump.

use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::time::Duration;

use opi_ai::auth::{LoginPresenter, OAuthCredential, OAuthLoginMethod, OAuthProvider};
use opi_ai::credential::{BoxAuthFuture, CredentialStore, CredentialStoreError};
use opi_ai::provider::ProviderError;
use secrecy::{ExposeSecret, SecretString};
use time::OffsetDateTime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use sha2::{Digest, Sha256};

/// PKCE `code_verifier` length in raw bytes before base64url encoding. 48 bytes
/// encode to 64 URL-safe characters, within the RFC 7636 [43, 128] range.
const CODE_VERIFIER_BYTES: usize = 48;

/// Generate a cryptographically random PKCE `code_verifier` (43-128 char
/// URL-safe string) using the OS CSPRNG.
pub fn generate_code_verifier() -> String {
    let mut bytes = [0u8; CODE_VERIFIER_BYTES];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Compute the S256 PKCE `code_challenge` = base64url(sha256(verifier)) with no
/// padding, per RFC 7636.
pub fn code_challenge_s256(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

/// Generate a cryptographically random opaque `state` token for CSRF protection
/// in the authorization-code flow.
pub fn generate_state() -> String {
    let mut bytes = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

// ---------------------------------------------------------------------------
// Loopback binding (PKCE callback)
// ---------------------------------------------------------------------------

/// Bind a TCP listener on `127.0.0.1:0` (loopback ONLY, never `0.0.0.0`) for
/// the PKCE authorization-code callback. The ephemeral port is discovered by
/// the provider and embedded in the authorize URL's `redirect_uri`. Loopback
/// binding keeps the callback server off the LAN.
async fn bind_loopback() -> io::Result<TcpListener> {
    TcpListener::bind("127.0.0.1:0").await
}

// ---------------------------------------------------------------------------
// Shared PKCE authorization-code runner (Anthropic + Codex)
// ---------------------------------------------------------------------------

/// Configuration for one PKCE authorization-code login. Held by the thin
/// provider wrappers and passed by value into the shared runner.
struct PkceLoginConfig {
    authorize_url: String,
    token_url: String,
    client_id: String,
    authorize_params: Vec<(String, String)>,
    client: reqwest::Client,
    timeout: Duration,
}

/// Why the loopback callback arm failed. Carries a static discriminant so
/// `notify_failure` can receive a fixed reason string (never the secret code).
enum CallbackFail {
    StateMismatch,
    Parse,
    Io(io::Error),
}

/// The outcome of the 3-way `select!` race (callback vs manual-code vs timeout).
/// Distinct variants let `notify_failure` receive a fixed, non-secret reason.
enum LoginOutcome {
    Code(String),
    Timeout,
    StateMismatch,
    CallbackParse,
    CallbackIo(io::Error),
    Manual(ProviderError),
}

/// Token-endpoint JSON response. `refresh_token` and `expires_in` are optional
/// per RFC 6749; the caller validates presence per flow (login requires both,
/// refresh accepts absence).
#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

/// Run the PKCE authorization-code login: bind loopback, build the authorize
/// URL, race callback/manual-code/timeout, then exchange the code for tokens.
/// `base_url` is `None` for PKCE flows (Anthropic/Codex have no per-credential
/// base URL); the caller does not override it.
async fn run_pkce_login(
    config: PkceLoginConfig,
    presenter: &dyn LoginPresenter,
    finalize_credential: fn(OAuthCredential) -> Result<OAuthCredential, ProviderError>,
) -> Result<OAuthCredential, ProviderError> {
    let listener = bind_loopback()
        .await
        .map_err(|e| ProviderError::Config(format!("oauth loopback bind failed: {e}")))?;
    let port = listener
        .local_addr()
        .map_err(|e| ProviderError::Config(format!("oauth loopback local_addr failed: {e}")))?
        .port();
    let verifier = generate_code_verifier();
    let challenge = code_challenge_s256(&verifier);
    let state = generate_state();
    let redirect_uri = format!("http://127.0.0.1:{port}/");
    let mut authorize_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&code_challenge={}&code_challenge_method=S256&state={}",
        config.authorize_url,
        url_encode(&config.client_id),
        url_encode(&redirect_uri),
        challenge,
        state,
    );
    for (name, value) in &config.authorize_params {
        authorize_url.push('&');
        authorize_url.push_str(name);
        authorize_url.push('=');
        authorize_url.push_str(&url_encode(value));
    }

    presenter.present_auth_url(&authorize_url).await?;

    let outcome = tokio::select! {
        res = accept_one_callback(&listener, &state) => match res {
            Ok(code) => LoginOutcome::Code(code),
            Err(CallbackFail::StateMismatch) => LoginOutcome::StateMismatch,
            Err(CallbackFail::Parse) => LoginOutcome::CallbackParse,
            Err(CallbackFail::Io(e)) => LoginOutcome::CallbackIo(e),
        },
        res = presenter.await_manual_code() => match res {
            Ok(code) => LoginOutcome::Code(code),
            Err(e) => LoginOutcome::Manual(e),
        },
        _ = tokio::time::sleep(config.timeout) => LoginOutcome::Timeout,
    };
    // The listener is no longer needed: drop it so the port is freed and a
    // stale callback from a prior login cannot land on a still-bound socket.
    drop(listener);

    let code = match outcome {
        LoginOutcome::Code(code) => code,
        LoginOutcome::Timeout => {
            presenter.notify_failure("timeout");
            return Err(ProviderError::Timeout);
        }
        LoginOutcome::StateMismatch => {
            presenter.notify_failure("state mismatch");
            return Err(ProviderError::Config("oauth state mismatch".into()));
        }
        LoginOutcome::CallbackParse => {
            presenter.notify_failure("callback parse error");
            return Err(ProviderError::Config("oauth callback parse error".into()));
        }
        LoginOutcome::CallbackIo(e) => {
            presenter.notify_failure("callback IO error");
            return Err(ProviderError::Config(format!(
                "oauth callback IO error: {e}"
            )));
        }
        LoginOutcome::Manual(e) => {
            presenter.notify_failure("manual code error");
            return Err(e);
        }
    };

    // Token exchange runs OUTSIDE the select so a timeout firing mid-POST
    // cannot drop the POST future and consume the auth code irrecoverably.
    match exchange_authorization_code(&config, &code, &redirect_uri, &verifier)
        .await
        .and_then(finalize_credential)
    {
        Ok(cred) => {
            presenter.notify_success();
            Ok(cred)
        }
        Err(e) => {
            presenter.notify_failure("token exchange failed");
            Err(e)
        }
    }
}

fn accept_oauth_credential(credential: OAuthCredential) -> Result<OAuthCredential, ProviderError> {
    Ok(credential)
}

fn require_codex_account_id(
    mut credential: OAuthCredential,
) -> Result<OAuthCredential, ProviderError> {
    #[derive(serde::Deserialize)]
    struct CodexClaims {
        #[serde(rename = "https://api.openai.com/auth")]
        auth: Option<CodexAuthClaims>,
    }

    #[derive(serde::Deserialize)]
    struct CodexAuthClaims {
        chatgpt_account_id: Option<String>,
    }

    let account_id = credential
        .access
        .expose_secret()
        .split('.')
        .nth(1)
        .and_then(|payload| URL_SAFE_NO_PAD.decode(payload).ok())
        .and_then(|payload| serde_json::from_slice::<CodexClaims>(&payload).ok())
        .and_then(|claims| claims.auth)
        .and_then(|claims| claims.chatgpt_account_id)
        .map(|account_id| account_id.trim().to_owned())
        .filter(|account_id| !account_id.is_empty())
        .ok_or_else(|| ProviderError::AccountIdMissing {
            provider_id: "openai-codex".to_owned(),
        })?;
    credential.account_id = Some(account_id);
    Ok(credential)
}

/// Exchange an authorization code for tokens (PKCE login). Requires both
/// `refresh_token` and `expires_in` on login (hard error otherwise — never
/// produce a non-refreshable or always-refreshing credential). Returns a
/// credential with `base_url = None`; PKCE providers derive no base URL.
async fn exchange_authorization_code(
    config: &PkceLoginConfig,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> Result<OAuthCredential, ProviderError> {
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", config.client_id.as_str()),
        ("code_verifier", verifier),
    ];
    let resp = config
        .client
        .post(&config.token_url)
        .form(&params)
        .send()
        .await
        .map_err(|_| ProviderError::Network("token exchange failed".into()))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(token_endpoint_error(status, &body));
    }
    let token: TokenResponse = resp
        .json()
        .await
        .map_err(|e| ProviderError::Config(format!("token response parse failed: {e}")))?;
    let refresh = token
        .refresh_token
        .ok_or_else(|| ProviderError::Config("token response missing refresh_token".into()))?;
    let expires_in = token
        .expires_in
        .ok_or_else(|| ProviderError::Config("token response missing expires_in".into()))?;
    Ok(OAuthCredential {
        access: SecretString::new(token.access_token.into_boxed_str()),
        refresh: SecretString::new(refresh.into_boxed_str()),
        expires_at: Some(OffsetDateTime::now_utc() + time::Duration::seconds(expires_in)),
        base_url: None,
        account_id: None,
    })
}

/// Accept a single loopback callback, parse `code`+`state` from the query,
/// validate `state` against the expected CSRF token, and write a minimal HTTP
/// 200 response (no secret) before resolving. Single-accept: the listener is
/// dropped by the caller after this returns.
async fn accept_one_callback(
    listener: &TcpListener,
    expected_state: &str,
) -> Result<String, CallbackFail> {
    let (mut stream, _) = listener.accept().await.map_err(CallbackFail::Io)?;
    let mut buf: Vec<u8> = Vec::with_capacity(1024);
    let mut tmp = [0u8; 1024];
    loop {
        let n = stream.read(&mut tmp).await.map_err(CallbackFail::Io)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buf.len() > 8192 {
            return Err(CallbackFail::Parse);
        }
    }
    // Minimal 200 response with no secret. Written and flushed before resolving
    // so a racing manual-code-wins cancellation leaves a clean response.
    let body = "Login complete, you may close this window.";
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(resp.as_bytes())
        .await
        .map_err(CallbackFail::Io)?;
    stream.flush().await.map_err(CallbackFail::Io)?;

    let req = std::str::from_utf8(&buf).map_err(|_| CallbackFail::Parse)?;
    let path = req.split(' ').nth(1).ok_or(CallbackFail::Parse)?;
    let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");
    let mut code: Option<String> = None;
    let mut state: Option<String> = None;
    for pair in query.split('&') {
        if let Some(v) = pair.strip_prefix("code=") {
            code = Some(percent_decode(v));
        } else if let Some(v) = pair.strip_prefix("state=") {
            state = Some(percent_decode(v));
        }
    }
    let code = code.ok_or(CallbackFail::Parse)?;
    if state.as_deref() != Some(expected_state) {
        return Err(CallbackFail::StateMismatch);
    }
    Ok(code)
}

/// Refresh an OAuth credential by exchanging its refresh token. Used by the
/// PKCE providers (Anthropic, Codex). `base_url` is preserved verbatim (PKCE
/// providers have no per-credential base URL derivation). On 401/403 from the
/// token endpoint the credential is revoked (non-retryable, no auto-relogin).
/// A missing `refresh_token` in the response reuses the old one (RFC 6749 §6);
/// a missing `expires_in` yields `expires_at = None` (documented always-refresh).
async fn refresh_oauth_token(
    client: &reqwest::Client,
    token_url: &str,
    client_id: &str,
    cred: &OAuthCredential,
    provider_id: &str,
    base_url: Option<String>,
) -> Result<OAuthCredential, ProviderError> {
    let refresh_secret: &str = cred.refresh.expose_secret();
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_secret),
        ("client_id", client_id),
    ];
    let resp = client
        .post(token_url)
        .form(&params)
        .send()
        .await
        .map_err(|_| ProviderError::Network("token refresh failed".into()))?;
    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        // Drain the body without surfacing it; a revoked credential is a typed
        // non-retryable error with no token interpolation.
        let _ = resp.text().await;
        return Err(ProviderError::CredentialRevoked {
            provider_id: provider_id.to_owned(),
        });
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(token_endpoint_error(status, &body));
    }
    let token: TokenResponse = resp
        .json()
        .await
        .map_err(|e| ProviderError::Config(format!("refresh response parse failed: {e}")))?;
    let refresh = token
        .refresh_token
        .map(|s| SecretString::new(s.into_boxed_str()))
        .unwrap_or_else(|| cred.refresh.clone());
    let expires_at = token
        .expires_in
        .map(|secs| OffsetDateTime::now_utc() + time::Duration::seconds(secs));
    Ok(OAuthCredential {
        access: SecretString::new(token.access_token.into_boxed_str()),
        refresh,
        expires_at,
        base_url,
        account_id: cred.account_id.clone(),
    })
}

/// Build a non-retryable auth error from a token-endpoint non-2xx response.
/// Only the OAuth error code is surfaced: descriptions and raw response bodies
/// can echo the submitted authorization code, verifier, or refresh token.
fn token_endpoint_error(status: reqwest::StatusCode, body: &str) -> ProviderError {
    #[derive(serde::Deserialize)]
    struct OAuthError {
        #[serde(default)]
        error: Option<String>,
    }
    let parsed: Option<OAuthError> = serde_json::from_str(body).ok();
    let code = parsed
        .as_ref()
        .and_then(|e| e.error.as_deref())
        .unwrap_or("");
    let msg = match code.is_empty() {
        false => format!("token endpoint: {status} {code}"),
        true => format!("token endpoint: {status}"),
    };
    ProviderError::AuthFailed(msg)
}

/// Percent-encode a query value (RFC 3986 unreserved set kept). Used to build
/// the authorize URL so `redirect_uri`/`client_id` are correctly encoded.
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// Percent-decode a query value. Used to parse the callback's `code`/`state`.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2]))
        {
            out.push(h * 16 + l);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_default()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Build a reqwest client that does NOT follow redirects, so a 302
/// echo-redirect from a token endpoint cannot leak the `code_verifier`.
fn no_redirect_client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("valid reqwest client")
}

pub(crate) fn production_oauth_client() -> reqwest::Client {
    no_redirect_client()
}

#[derive(Clone)]
pub(crate) struct AnthropicOAuthEndpointConfig {
    authorize_url: String,
    token_url: String,
    client_id: String,
    login_timeout: Duration,
}

#[derive(Clone)]
pub(crate) struct CopilotOAuthEndpointConfig {
    device_authorization_url: String,
    token_url: String,
    copilot_token_url: String,
    client_id: String,
    scope: String,
    login_timeout: Duration,
}

#[derive(Clone)]
pub(crate) struct CodexOAuthEndpointConfig {
    authorize_url: String,
    token_url: String,
    device_user_code_url: String,
    device_token_url: String,
    device_verification_uri: String,
    device_redirect_uri: String,
    client_id: String,
    browser_timeout: Duration,
    device_timeout: Duration,
}

#[derive(Clone)]
pub(crate) struct OAuthEndpointConfig {
    anthropic: AnthropicOAuthEndpointConfig,
    copilot: CopilotOAuthEndpointConfig,
    codex: CodexOAuthEndpointConfig,
}

impl OAuthEndpointConfig {
    pub(crate) fn production() -> Self {
        const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);
        Self {
            anthropic: AnthropicOAuthEndpointConfig {
                authorize_url: "https://claude.ai/oauth/authorize".to_owned(),
                token_url: "https://platform.claude.com/v1/oauth/token".to_owned(),
                client_id: "9d1c250a-e61b-44d9-88ed-5944d1962f5e".to_owned(),
                login_timeout: LOGIN_TIMEOUT,
            },
            copilot: CopilotOAuthEndpointConfig {
                device_authorization_url: "https://github.com/login/device/code".to_owned(),
                token_url: "https://github.com/login/oauth/access_token".to_owned(),
                copilot_token_url: "https://api.github.com/copilot_internal/v2/token".to_owned(),
                client_id: "Iv1.b507a08c87ecfe98".to_owned(),
                scope: "read:user".to_owned(),
                login_timeout: LOGIN_TIMEOUT,
            },
            codex: CodexOAuthEndpointConfig {
                authorize_url: "https://auth.openai.com/oauth/authorize".to_owned(),
                token_url: "https://auth.openai.com/oauth/token".to_owned(),
                device_user_code_url: CODEX_DEVICE_USER_CODE_URL.to_owned(),
                device_token_url: CODEX_DEVICE_TOKEN_URL.to_owned(),
                device_verification_uri: CODEX_DEVICE_VERIFICATION_URI.to_owned(),
                device_redirect_uri: CODEX_DEVICE_REDIRECT_URI.to_owned(),
                client_id: "app_EMoamEEZ73f0CkXaXp7hrann".to_owned(),
                browser_timeout: LOGIN_TIMEOUT,
                device_timeout: CODEX_DEVICE_TIMEOUT,
            },
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn with_test_base_url(
        base_url: String,
        login_timeout: Duration,
        codex_device_timeout: Duration,
    ) -> Self {
        let base_url = base_url.trim_end_matches('/');
        Self {
            anthropic: AnthropicOAuthEndpointConfig {
                authorize_url: format!("{base_url}/anthropic/authorize"),
                token_url: format!("{base_url}/anthropic/token"),
                client_id: "9d1c250a-e61b-44d9-88ed-5944d1962f5e".to_owned(),
                login_timeout,
            },
            copilot: CopilotOAuthEndpointConfig {
                device_authorization_url: format!("{base_url}/copilot/device/code"),
                token_url: format!("{base_url}/copilot/oauth/token"),
                copilot_token_url: format!("{base_url}/copilot/token"),
                client_id: "Iv1.b507a08c87ecfe98".to_owned(),
                scope: "read:user".to_owned(),
                login_timeout,
            },
            codex: CodexOAuthEndpointConfig {
                authorize_url: format!("{base_url}/codex/authorize"),
                token_url: format!("{base_url}/codex/token"),
                device_user_code_url: format!("{base_url}/codex/device/usercode"),
                device_token_url: format!("{base_url}/codex/device/token"),
                device_verification_uri: format!("{base_url}/codex/device"),
                device_redirect_uri: CODEX_DEVICE_REDIRECT_URI.to_owned(),
                client_id: "app_EMoamEEZ73f0CkXaXp7hrann".to_owned(),
                browser_timeout: login_timeout,
                device_timeout: codex_device_timeout,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// AnthropicOAuthProvider (PKCE)
// ---------------------------------------------------------------------------

/// Anthropic OAuth provider using PKCE authorization-code with a `127.0.0.1`
/// loopback callback. Endpoints are configurable so tests point them at a
/// `wiremock` server. Bearer auth + the OAuth beta header are applied by the
/// factory (slice 5) via the compatibility profile, not here.
pub struct AnthropicOAuthProvider {
    authorize_url: String,
    token_url: String,
    client_id: String,
    client: reqwest::Client,
    timeout: Duration,
}

impl AnthropicOAuthProvider {
    fn with_services(endpoints: &AnthropicOAuthEndpointConfig, client: reqwest::Client) -> Self {
        Self {
            authorize_url: endpoints.authorize_url.clone(),
            token_url: endpoints.token_url.clone(),
            client_id: endpoints.client_id.clone(),
            client,
            timeout: endpoints.login_timeout,
        }
    }

    /// Construct with configurable endpoints and a login timeout. Builds a
    /// redirect-`none` HTTP client for the token exchange.
    pub fn new(
        authorize_url: String,
        token_url: String,
        client_id: String,
        timeout: Duration,
    ) -> Self {
        Self {
            authorize_url,
            token_url,
            client_id,
            client: no_redirect_client(),
            timeout,
        }
    }
}

impl OAuthProvider for AnthropicOAuthProvider {
    fn id(&self) -> &str {
        "anthropic"
    }

    fn login<'a>(
        &'a self,
        presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        let config = PkceLoginConfig {
            authorize_url: self.authorize_url.clone(),
            token_url: self.token_url.clone(),
            client_id: self.client_id.clone(),
            authorize_params: vec![
                ("code".into(), "true".into()),
                (
                    "scope".into(),
                    "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers user:file_upload".into(),
                ),
            ],
            client: self.client.clone(),
            timeout: self.timeout,
        };
        Box::pin(async move { run_pkce_login(config, presenter, accept_oauth_credential).await })
    }

    fn refresh<'a>(
        &'a self,
        cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        let token_url = self.token_url.clone();
        let client_id = self.client_id.clone();
        let client = self.client.clone();
        Box::pin(async move {
            refresh_oauth_token(
                &client,
                &token_url,
                &client_id,
                cred,
                "anthropic",
                cred.base_url.clone(),
            )
            .await
        })
    }
}

// ---------------------------------------------------------------------------
// CodexOAuthProvider (browser PKCE + device-code)
// ---------------------------------------------------------------------------

/// OpenAI Codex OAuth provider supporting browser PKCE with a `127.0.0.1`
/// loopback callback and OpenAI's device-code flow.
const CODEX_DEVICE_USER_CODE_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/usercode";
const CODEX_DEVICE_TOKEN_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/token";
const CODEX_DEVICE_VERIFICATION_URI: &str = "https://auth.openai.com/codex/device";
const CODEX_DEVICE_REDIRECT_URI: &str = "https://auth.openai.com/deviceauth/callback";
const CODEX_DEVICE_TIMEOUT: Duration = Duration::from_secs(15 * 60);

#[derive(serde::Deserialize)]
struct CodexDeviceAuthorization {
    device_auth_id: String,
    user_code: String,
    interval: serde_json::Value,
}

#[derive(serde::Deserialize)]
struct CodexDeviceToken {
    authorization_code: String,
    code_verifier: String,
}

enum CodexDevicePoll {
    Pending,
    SlowDown,
    Denied,
    Expired,
    Complete(CodexDeviceToken),
}

pub struct CodexOAuthProvider {
    authorize_url: String,
    token_url: String,
    device_user_code_url: String,
    device_token_url: String,
    device_verification_uri: String,
    device_redirect_uri: String,
    client_id: String,
    client: reqwest::Client,
    browser_timeout: Duration,
    device_timeout: Duration,
}

impl CodexOAuthProvider {
    fn with_services(endpoints: &CodexOAuthEndpointConfig, client: reqwest::Client) -> Self {
        Self {
            authorize_url: endpoints.authorize_url.clone(),
            token_url: endpoints.token_url.clone(),
            device_user_code_url: endpoints.device_user_code_url.clone(),
            device_token_url: endpoints.device_token_url.clone(),
            device_verification_uri: endpoints.device_verification_uri.clone(),
            device_redirect_uri: endpoints.device_redirect_uri.clone(),
            client_id: endpoints.client_id.clone(),
            client,
            browser_timeout: endpoints.browser_timeout,
            device_timeout: endpoints.device_timeout,
        }
    }

    /// Construct with configurable browser endpoints and a login timeout.
    ///
    /// Device-code login uses the production OpenAI endpoints and its fixed
    /// 15-minute budget.
    pub fn new(
        authorize_url: String,
        token_url: String,
        client_id: String,
        timeout: Duration,
    ) -> Self {
        Self {
            authorize_url,
            token_url,
            device_user_code_url: CODEX_DEVICE_USER_CODE_URL.to_owned(),
            device_token_url: CODEX_DEVICE_TOKEN_URL.to_owned(),
            device_verification_uri: CODEX_DEVICE_VERIFICATION_URI.to_owned(),
            device_redirect_uri: CODEX_DEVICE_REDIRECT_URI.to_owned(),
            client_id,
            client: no_redirect_client(),
            browser_timeout: timeout,
            device_timeout: CODEX_DEVICE_TIMEOUT,
        }
    }

    /// Construct with fully configurable endpoints for offline wire tests.
    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_device_endpoints(
        authorize_url: String,
        token_url: String,
        device_user_code_url: String,
        device_token_url: String,
        device_verification_uri: String,
        device_redirect_uri: String,
        client_id: String,
        browser_timeout: Duration,
        device_timeout: Duration,
    ) -> Self {
        Self {
            authorize_url,
            token_url,
            device_user_code_url,
            device_token_url,
            device_verification_uri,
            device_redirect_uri,
            client_id,
            client: no_redirect_client(),
            browser_timeout,
            device_timeout,
        }
    }
}

fn codex_device_interval(value: &serde_json::Value) -> Option<Duration> {
    let seconds = match value {
        serde_json::Value::Number(number) => number.as_u64(),
        serde_json::Value::String(text) => text.trim().parse::<u64>().ok(),
        _ => None,
    }?;
    Some(Duration::from_secs(seconds))
}

fn codex_device_error_code(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    match value.get("error")? {
        serde_json::Value::String(code) => Some(code.clone()),
        serde_json::Value::Object(error) => error.get("code")?.as_str().map(str::to_owned),
        _ => None,
    }
}

async fn poll_codex_device_token(
    client: &reqwest::Client,
    token_url: &str,
    device_auth_id: &str,
    user_code: &str,
) -> Result<CodexDevicePoll, ProviderError> {
    let response = client
        .post(token_url)
        .json(&serde_json::json!({
            "device_auth_id": device_auth_id,
            "user_code": user_code,
        }))
        .send()
        .await
        .map_err(|_| ProviderError::Network("device token poll failed".into()))?;
    let status = response.status();
    if status.is_success() {
        let token = response.json::<CodexDeviceToken>().await.map_err(|_| {
            ProviderError::Config("device token response missing required fields".into())
        })?;
        if token.authorization_code.is_empty() || token.code_verifier.is_empty() {
            return Err(ProviderError::Config(
                "device token response missing required fields".into(),
            ));
        }
        return Ok(CodexDevicePoll::Complete(token));
    }
    let body = response.text().await.unwrap_or_default();
    if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::NOT_FOUND {
        return Ok(CodexDevicePoll::Pending);
    }
    match codex_device_error_code(&body).as_deref() {
        Some("deviceauth_authorization_pending") | Some("authorization_pending") => {
            Ok(CodexDevicePoll::Pending)
        }
        Some("slow_down") => Ok(CodexDevicePoll::SlowDown),
        Some("access_denied") | Some("deviceauth_access_denied") => Ok(CodexDevicePoll::Denied),
        Some("expired_token") | Some("deviceauth_expired") => Ok(CodexDevicePoll::Expired),
        _ => Err(ProviderError::AuthFailed(format!(
            "device authorization failed ({status})"
        ))),
    }
}

async fn run_codex_device_login_flow(
    provider: &CodexOAuthProvider,
    presenter: &dyn LoginPresenter,
) -> Result<OAuthCredential, ProviderError> {
    let deadline = tokio::time::Instant::now() + provider.device_timeout;
    let response = tokio::select! {
        response = provider.client
            .post(&provider.device_user_code_url)
            .json(&serde_json::json!({"client_id": provider.client_id}))
            .send() => response.map_err(|_| {
                ProviderError::Network("device authorization request failed".into())
            })?,
        _ = tokio::time::sleep_until(deadline) => {
            presenter.notify_failure("device authorization timed out");
            return Err(ProviderError::Timeout);
        }
    };
    let status = response.status();
    if !status.is_success() {
        let _ = response.text().await;
        presenter.notify_failure("device authorization request failed");
        return Err(ProviderError::AuthFailed(format!(
            "device authorization request failed ({status})"
        )));
    }
    let device = response
        .json::<CodexDeviceAuthorization>()
        .await
        .map_err(|_| {
            ProviderError::Config("device authorization response missing required fields".into())
        })?;
    let mut interval = codex_device_interval(&device.interval).ok_or_else(|| {
        ProviderError::Config("device authorization response has invalid interval".into())
    })?;
    if device.device_auth_id.is_empty() || device.user_code.is_empty() {
        return Err(ProviderError::Config(
            "device authorization response missing required fields".into(),
        ));
    }

    tokio::select! {
        result = presenter.present_device_code(
            &device.user_code,
            &provider.device_verification_uri,
        ) => result?,
        _ = tokio::time::sleep_until(deadline) => {
            presenter.notify_failure("device authorization timed out");
            return Err(ProviderError::Timeout);
        }
    }

    let token = loop {
        let outcome = tokio::select! {
            outcome = poll_codex_device_token(
                &provider.client,
                &provider.device_token_url,
                &device.device_auth_id,
                &device.user_code,
            ) => outcome,
            _ = tokio::time::sleep_until(deadline) => {
                presenter.notify_failure("device authorization timed out");
                return Err(ProviderError::Timeout);
            }
        };
        match outcome {
            Ok(CodexDevicePoll::Complete(token)) => break token,
            Ok(CodexDevicePoll::Pending) => {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {}
                    _ = tokio::time::sleep_until(deadline) => {
                        presenter.notify_failure("device authorization timed out");
                        return Err(ProviderError::Timeout);
                    }
                }
            }
            Ok(CodexDevicePoll::SlowDown) => {
                interval += Duration::from_secs(5);
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {}
                    _ = tokio::time::sleep_until(deadline) => {
                        presenter.notify_failure("device authorization timed out");
                        return Err(ProviderError::Timeout);
                    }
                }
            }
            Ok(CodexDevicePoll::Denied) => {
                presenter.notify_failure("device authorization denied");
                return Err(ProviderError::CredentialRevoked {
                    provider_id: "openai-codex".to_owned(),
                });
            }
            Ok(CodexDevicePoll::Expired) => {
                presenter.notify_failure("device code expired");
                return Err(ProviderError::CredentialRevoked {
                    provider_id: "openai-codex".to_owned(),
                });
            }
            Err(error) => {
                presenter.notify_failure("device authorization failed");
                return Err(error);
            }
        }
    };

    let exchange_config = PkceLoginConfig {
        authorize_url: String::new(),
        token_url: provider.token_url.clone(),
        client_id: provider.client_id.clone(),
        authorize_params: Vec::new(),
        client: provider.client.clone(),
        timeout: provider.device_timeout,
    };
    let credential = tokio::select! {
        credential = exchange_authorization_code(
            &exchange_config,
            &token.authorization_code,
            &provider.device_redirect_uri,
            &token.code_verifier,
        ) => credential,
        _ = tokio::time::sleep_until(deadline) => {
            presenter.notify_failure("device authorization timed out");
            return Err(ProviderError::Timeout);
        }
    }
    .and_then(require_codex_account_id);
    match credential {
        Ok(credential) => {
            presenter.notify_success();
            Ok(credential)
        }
        Err(error) => {
            presenter.notify_failure("token exchange failed");
            Err(error)
        }
    }
}

async fn run_codex_device_login(
    provider: &CodexOAuthProvider,
    presenter: &dyn LoginPresenter,
) -> Result<OAuthCredential, ProviderError> {
    let flow = run_codex_device_login_flow(provider, presenter);
    let cancellation = presenter.await_login_cancelled();
    tokio::pin!(flow);
    tokio::pin!(cancellation);
    tokio::select! {
        biased;
        cancelled = &mut cancellation => {
            cancelled?;
            presenter.notify_failure("device authorization cancelled");
            Err(ProviderError::LoginCancelled {
                provider_id: "openai-codex".to_owned(),
            })
        }
        result = &mut flow => result,
    }
}

impl OAuthProvider for CodexOAuthProvider {
    fn id(&self) -> &str {
        "openai-codex"
    }

    fn login<'a>(
        &'a self,
        presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            let method = presenter
                .select_login_method(
                    "openai-codex",
                    &[OAuthLoginMethod::Browser, OAuthLoginMethod::DeviceCode],
                    OAuthLoginMethod::Browser,
                )
                .await?;
            match method {
                OAuthLoginMethod::Browser => {
                    let config = PkceLoginConfig {
                        authorize_url: self.authorize_url.clone(),
                        token_url: self.token_url.clone(),
                        client_id: self.client_id.clone(),
                        authorize_params: vec![
                            ("scope".into(), "openid profile email offline_access".into()),
                            ("id_token_add_organizations".into(), "true".into()),
                            ("codex_cli_simplified_flow".into(), "true".into()),
                            ("originator".into(), "opi".into()),
                        ],
                        client: self.client.clone(),
                        timeout: self.browser_timeout,
                    };
                    run_pkce_login(config, presenter, require_codex_account_id).await
                }
                OAuthLoginMethod::DeviceCode => run_codex_device_login(self, presenter).await,
            }
        })
    }

    fn refresh<'a>(
        &'a self,
        cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        let token_url = self.token_url.clone();
        let client_id = self.client_id.clone();
        let client = self.client.clone();
        Box::pin(async move {
            let credential = refresh_oauth_token(
                &client,
                &token_url,
                &client_id,
                cred,
                "openai-codex",
                cred.base_url.clone(),
            )
            .await?;
            require_codex_account_id(credential)
        })
    }
}

// ---------------------------------------------------------------------------
// CopilotOAuthProvider (GitHub device-code, then Copilot token exchange)
// ---------------------------------------------------------------------------

const COPILOT_USER_AGENT: &str = "GitHubCopilotChat/0.35.0";
const COPILOT_EDITOR_VERSION: &str = "vscode/1.107.0";
const COPILOT_PLUGIN_VERSION: &str = "copilot-chat/0.35.0";
const COPILOT_INTEGRATION_ID: &str = "vscode-chat";

/// Device-authorization response (RFC 8628). `device_code` is secret; only
/// `user_code` and `verification_uri` reach the presenter.
#[derive(serde::Deserialize)]
struct DeviceAuthorizationBody {
    device_code: String,
    user_code: String,
    verification_uri: String,
    #[serde(default)]
    interval: Option<u64>,
}

/// Device token-poll response. On success carries `access_token`; while the
/// user has not yet authorized, carries an OAuth `error` code instead.
#[derive(serde::Deserialize)]
struct DeviceTokenBody {
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

/// Outcome of one device token-poll iteration.
enum DevicePollOutcome {
    /// `authorization_pending`: sleep `interval`, retry.
    Pending,
    /// `slow_down`: increase `interval` by 5s (RFC 8628 §3.5), retry.
    SlowDown,
    /// `access_denied`: the user denied the request.
    Denied,
    /// `expired_token`: the device code expired.
    Expired,
    /// Success: the GitHub access token (long-lived; stored as the refresh
    /// credential so a later refresh re-exchanges it for a fresh Copilot token).
    Token(String),
}

/// Copilot token-exchange response. `expires_at` is an ABSOLUTE unix timestamp
/// (seconds), not a relative duration. `endpoints.api` is the per-credential
/// base URL (enterprise hosts); absent on some responses.
#[derive(serde::Deserialize)]
struct CopilotTokenBody {
    token: String,
    expires_at: i64,
    #[serde(default)]
    endpoints: Option<CopilotEndpoints>,
}

#[derive(serde::Deserialize)]
struct CopilotEndpoints {
    #[serde(default)]
    api: Option<String>,
}

/// Poll the device token endpoint once, classifying the response.
async fn poll_device_token(
    client: &reqwest::Client,
    token_url: &str,
    device_code: &str,
    client_id: &str,
) -> Result<DevicePollOutcome, ProviderError> {
    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("device_code", device_code),
        ("client_id", client_id),
    ];
    let resp = client
        .post(token_url)
        .header("accept", "application/json")
        .header("user-agent", COPILOT_USER_AGENT)
        .form(&params)
        .send()
        .await
        .map_err(|_| ProviderError::Network("device token poll failed".into()))?;
    // GitHub returns 200 with an `error` body for pending/denied/expired and
    // 200 with `access_token` on success; tolerate either status by parsing
    // the body. The device_code is never surfaced on any path here.
    let body: DeviceTokenBody = resp
        .json()
        .await
        .map_err(|e| ProviderError::Config(format!("device token response parse failed: {e}")))?;
    if let Some(token) = body.access_token {
        return Ok(DevicePollOutcome::Token(token));
    }
    match body.error.as_deref() {
        Some("authorization_pending") => Ok(DevicePollOutcome::Pending),
        Some("slow_down") => Ok(DevicePollOutcome::SlowDown),
        Some("access_denied") => Ok(DevicePollOutcome::Denied),
        Some("expired_token") => Ok(DevicePollOutcome::Expired),
        Some(other) => Err(ProviderError::Config(format!(
            "device authorization error: {other}"
        ))),
        None => Err(ProviderError::Config(
            "device token response has no access_token and no error".into(),
        )),
    }
}

/// GitHub Copilot OAuth provider using the device-code flow (RFC 8628), then
/// exchanging the GitHub token for a short-lived Copilot token. Unlike the PKCE
/// providers it NEVER calls `present_auth_url` or `await_manual_code`; the only
/// presenter call is `present_device_code(user_code, verification_uri)`. The
/// `device_code` is secret (it grants token issuance) and never leaves the poll
/// loop. `login` is bounded by `total_budget`; a hung poll or inter-poll sleep
/// yields `ProviderError::Timeout`.
pub struct CopilotOAuthProvider {
    device_authorization_url: String,
    token_url: String,
    copilot_token_url: String,
    client_id: String,
    scope: String,
    client: reqwest::Client,
    total_budget: Duration,
}

impl CopilotOAuthProvider {
    fn with_services(endpoints: &CopilotOAuthEndpointConfig, client: reqwest::Client) -> Self {
        Self {
            device_authorization_url: endpoints.device_authorization_url.clone(),
            token_url: endpoints.token_url.clone(),
            copilot_token_url: endpoints.copilot_token_url.clone(),
            client_id: endpoints.client_id.clone(),
            scope: endpoints.scope.clone(),
            client,
            total_budget: endpoints.login_timeout,
        }
    }

    /// Construct with configurable endpoints, the device-flow `scope`, and a
    /// total login budget (bounds polling + inter-poll sleeps).
    pub fn new(
        device_authorization_url: String,
        token_url: String,
        copilot_token_url: String,
        client_id: String,
        scope: String,
        total_budget: Duration,
    ) -> Self {
        Self {
            device_authorization_url,
            token_url,
            copilot_token_url,
            client_id,
            scope,
            client: no_redirect_client(),
            total_budget,
        }
    }

    /// Exchange a GitHub access token for a Copilot token (shared by login +
    /// refresh). `base_url` fallback preserves an existing base_url when the
    /// response omits `endpoints.api`. 401/403 -> CredentialRevoked.
    async fn exchange_copilot_token(
        client: &reqwest::Client,
        copilot_token_url: &str,
        github_token: &str,
        base_url_fallback: Option<String>,
    ) -> Result<(SecretString, Option<OffsetDateTime>, Option<String>), ProviderError> {
        let resp = client
            .get(copilot_token_url)
            .header("accept", "application/json")
            .header("user-agent", COPILOT_USER_AGENT)
            .header("Editor-Version", COPILOT_EDITOR_VERSION)
            .header("Editor-Plugin-Version", COPILOT_PLUGIN_VERSION)
            .header("Copilot-Integration-Id", COPILOT_INTEGRATION_ID)
            .bearer_auth(github_token)
            .send()
            .await
            .map_err(|_| ProviderError::Network("copilot token exchange failed".into()))?;
        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            let _ = resp.text().await;
            return Err(ProviderError::CredentialRevoked {
                provider_id: "github-copilot".to_owned(),
            });
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(token_endpoint_error(status, &body));
        }
        let body: CopilotTokenBody = resp.json().await.map_err(|e| {
            ProviderError::Config(format!("copilot token response parse failed: {e}"))
        })?;
        let expires_at = OffsetDateTime::from_unix_timestamp(body.expires_at).ok();
        let base_url = body.endpoints.and_then(|e| e.api).or(base_url_fallback);
        Ok((
            SecretString::new(body.token.into_boxed_str()),
            expires_at,
            base_url,
        ))
    }
}

impl OAuthProvider for CopilotOAuthProvider {
    fn id(&self) -> &str {
        "github-copilot"
    }

    fn login<'a>(
        &'a self,
        presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        let da_url = self.device_authorization_url.clone();
        let token_url = self.token_url.clone();
        let copilot_token_url = self.copilot_token_url.clone();
        let client_id = self.client_id.clone();
        let scope = self.scope.clone();
        let client = self.client.clone();
        let total_budget = self.total_budget;
        Box::pin(async move {
            // 1. Device-authorization request.
            let params = [("client_id", client_id.as_str()), ("scope", scope.as_str())];
            let resp = client
                .post(&da_url)
                .header("accept", "application/json")
                .header("user-agent", COPILOT_USER_AGENT)
                .form(&params)
                .send()
                .await
                .map_err(|_| {
                    ProviderError::Network("device authorization request failed".into())
                })?;
            let status = resp.status();
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                presenter.notify_failure("device authorization request failed");
                return Err(token_endpoint_error(status, &body));
            }
            let da: DeviceAuthorizationBody = resp.json().await.map_err(|e| {
                ProviderError::Config(format!("device authorization parse failed: {e}"))
            })?;
            // device_code is SECRET — local only, never passed to the presenter
            // or formatted into any error.
            let device_code = da.device_code;
            let user_code = da.user_code;
            let verification_uri = da.verification_uri;
            let mut interval = da
                .interval
                .map(Duration::from_secs)
                .unwrap_or_else(|| Duration::from_secs(5));

            // 2. Present the public user_code (NEVER the device_code).
            presenter
                .present_device_code(&user_code, &verification_uri)
                .await?;

            // 3. Poll until a token, a terminal error, or the total budget elapses.
            let deadline = tokio::time::Instant::now() + total_budget;
            let github_token = loop {
                let outcome = tokio::select! {
                    r = poll_device_token(&client, &token_url, &device_code, &client_id) => match r {
                        Ok(o) => o,
                        Err(e) => {
                            presenter.notify_failure("device authorization failed");
                            return Err(e);
                        }
                    },
                    _ = tokio::time::sleep_until(deadline) => {
                        presenter.notify_failure("device authorization timed out");
                        return Err(ProviderError::Timeout);
                    }
                };
                match outcome {
                    DevicePollOutcome::Pending | DevicePollOutcome::SlowDown => {
                        if matches!(outcome, DevicePollOutcome::SlowDown) {
                            // RFC 8628 §3.5: increase by exactly 5 seconds,
                            // persistently (not reset on the next pending).
                            interval += Duration::from_secs(5);
                        }
                        tokio::select! {
                            _ = tokio::time::sleep(interval) => {}
                            _ = tokio::time::sleep_until(deadline) => {
                                presenter.notify_failure("device authorization timed out");
                                return Err(ProviderError::Timeout);
                            }
                        }
                    }
                    DevicePollOutcome::Denied => {
                        presenter.notify_failure("device authorization denied");
                        return Err(ProviderError::CredentialRevoked {
                            provider_id: "github-copilot".to_owned(),
                        });
                    }
                    DevicePollOutcome::Expired => {
                        presenter.notify_failure("device code expired");
                        return Err(ProviderError::CredentialRevoked {
                            provider_id: "github-copilot".to_owned(),
                        });
                    }
                    DevicePollOutcome::Token(github_token) => break github_token,
                }
            };

            // 4. Exchange the GitHub token for a short-lived Copilot token.
            let (access, expires_at, base_url) = match Self::exchange_copilot_token(
                &client,
                &copilot_token_url,
                &github_token,
                None,
            )
            .await
            {
                Ok(triple) => triple,
                Err(e) => {
                    presenter.notify_failure("token exchange failed");
                    return Err(e);
                }
            };
            let cred = OAuthCredential {
                access,
                refresh: SecretString::new(github_token.into_boxed_str()),
                expires_at,
                base_url,
                account_id: None,
            };
            presenter.notify_success();
            Ok(cred)
        })
    }

    fn refresh<'a>(
        &'a self,
        cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        let copilot_token_url = self.copilot_token_url.clone();
        let client = self.client.clone();
        Box::pin(async move {
            let github_token: &str = cred.refresh.expose_secret();
            let (access, expires_at, base_url) = Self::exchange_copilot_token(
                &client,
                &copilot_token_url,
                github_token,
                cred.base_url.clone(),
            )
            .await?;
            Ok(OAuthCredential {
                access,
                refresh: cred.refresh.clone(),
                expires_at,
                base_url,
                account_id: None,
            })
        })
    }
}

// ---------------------------------------------------------------------------
// TuiLoginPresenter — production LoginPresenter
// ---------------------------------------------------------------------------

/// Production `LoginPresenter`. The presenter itself uses normal terminal IO:
/// `present_auth_url` prints the URL (no browser-open; headless/SSH parity),
/// `present_device_code` prints the public `user_code` + verification URI,
/// `await_manual_code` reads one line from stdin, and `notify_*` print a status
/// line. The interactive dispatcher suspends raw mode and the ratatui alternate
/// screen before invoking it, then restores both afterward. No method
/// logs access/refresh tokens, authorization codes, or device codes (only the
/// public `user_code` is shown via `present_device_code`).
pub struct TuiLoginPresenter;

impl TuiLoginPresenter {
    /// Construct the print-only presenter.
    pub fn new() -> Self {
        Self
    }
}

impl Default for TuiLoginPresenter {
    fn default() -> Self {
        Self::new()
    }
}

impl LoginPresenter for TuiLoginPresenter {
    fn select_login_method<'a>(
        &'a self,
        provider_id: &'a str,
        methods: &'a [OAuthLoginMethod],
        default: OAuthLoginMethod,
    ) -> BoxAuthFuture<'a, Result<OAuthLoginMethod, ProviderError>> {
        let provider_id = provider_id.to_owned();
        let methods = methods.to_vec();
        Box::pin(async move {
            if provider_id != "openai-codex"
                || methods != [OAuthLoginMethod::Browser, OAuthLoginMethod::DeviceCode]
                || default != OAuthLoginMethod::Browser
            {
                return Err(ProviderError::Config(format!(
                    "OAuth provider '{provider_id}' supplied unsupported login methods"
                )));
            }
            tokio::task::spawn_blocking(|| {
                println!("Select OpenAI Codex login method:");
                println!("  1. Browser login (default)");
                println!("  2. Device code login (headless)");
                print!("Choice [1]: ");
                let _ = std::io::Write::flush(&mut std::io::stdout());
                let mut line = String::new();
                std::io::stdin()
                    .read_line(&mut line)
                    .map_err(|e| ProviderError::Config(format!("stdin read failed: {e}")))?;
                match line.trim() {
                    "" | "1" => Ok(OAuthLoginMethod::Browser),
                    "2" => Ok(OAuthLoginMethod::DeviceCode),
                    "q" | "quit" | "cancel" => Err(ProviderError::LoginCancelled {
                        provider_id: "openai-codex".to_owned(),
                    }),
                    _ => Err(ProviderError::Config(
                        "invalid OpenAI Codex login method".into(),
                    )),
                }
            })
            .await
            .map_err(|e| ProviderError::Config(format!("login method join failed: {e}")))?
        })
    }

    fn present_auth_url<'a>(
        &'a self,
        url: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        let url = url.to_owned();
        Box::pin(async move {
            println!("Open this URL to authorize:\n{url}");
            Ok(())
        })
    }

    fn present_device_code<'a>(
        &'a self,
        user_code: &'a str,
        verification_uri: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        let user_code = user_code.to_owned();
        let verification_uri = verification_uri.to_owned();
        Box::pin(async move {
            println!("Go to {verification_uri} and enter the code: {user_code}");
            Ok(())
        })
    }

    fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
        Box::pin(async move {
            print!("Paste the authorization code: ");
            let _ = std::io::Write::flush(&mut std::io::stdout());
            // Blocking stdin read moved off the async executor. The interactive
            // dispatcher suspends raw/alternate-screen state around this
            // blocking line read so manual paste works in local and SSH terms.
            let line = tokio::task::spawn_blocking(|| {
                let mut line = String::new();
                std::io::stdin().read_line(&mut line).map(|_| line)
            })
            .await
            .map_err(|e| ProviderError::Config(format!("manual code join failed: {e}")))?
            .map_err(|e| ProviderError::Config(format!("stdin read failed: {e}")))?;
            Ok(line.trim().to_owned())
        })
    }

    fn await_login_cancelled<'a>(&'a self) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            tokio::signal::ctrl_c()
                .await
                .map_err(|_| ProviderError::Config("login cancellation signal failed".into()))
        })
    }

    fn notify_success(&self) {
        println!("Login successful.");
    }

    fn notify_failure(&self, reason: &str) {
        println!("Login failed: {reason}");
    }
}

// ---------------------------------------------------------------------------
// OAuthProviderRegistry — heterogeneous registry keyed by id
// ---------------------------------------------------------------------------

/// Errors from [`OAuthProviderRegistry::register`].
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    /// A provider with the same id is already registered. Registration never
    /// silently overwrites: a collision would route a stored refresh token to
    /// the wrong provider's token endpoint (a credential-exposure path).
    #[error("an OAuth provider with id `{id}` is already registered")]
    DuplicateId { id: String },
}

/// Heterogeneous registry of OAuth providers keyed by `id()`. Holds
/// `Arc<dyn OAuthProvider>` so PKCE and device-code providers coexist behind
/// one type. [`lookup`](Self::lookup) returns an owned `Arc` clone so it can be
/// moved into `AuthSource::Store` without borrowing the registry.
pub struct OAuthProviderRegistry {
    providers: HashMap<String, Arc<dyn OAuthProvider>>,
}

impl OAuthProviderRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register a provider. Returns `Err(DuplicateId)` if a provider with the
    /// same `id()` already exists; never silently overwrites.
    pub fn register(&mut self, provider: Arc<dyn OAuthProvider>) -> Result<(), RegistryError> {
        let id = provider.id().to_owned();
        if self.providers.contains_key(&id) {
            return Err(RegistryError::DuplicateId { id });
        }
        self.providers.insert(id, provider);
        Ok(())
    }

    /// Look up a provider by id (case-sensitive). Returns an owned `Arc` clone,
    /// or `None` if no provider with that id is registered.
    pub fn lookup(&self, id: &str) -> Option<Arc<dyn OAuthProvider>> {
        self.providers.get(id).cloned()
    }

    /// Sorted registered ids, for diagnostics. Non-secret.
    pub fn ids(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = self.providers.keys().map(|s| s.as_str()).collect();
        ids.sort_unstable();
        ids
    }

    pub(crate) fn registry_with_services(
        endpoints: &OAuthEndpointConfig,
        client: reqwest::Client,
    ) -> Self {
        let mut registry = Self::new();
        registry
            .register(Arc::new(AnthropicOAuthProvider::with_services(
                &endpoints.anthropic,
                client.clone(),
            )))
            .expect("anthropic OAuth provider id is unique in a fresh registry");
        registry
            .register(Arc::new(CodexOAuthProvider::with_services(
                &endpoints.codex,
                client.clone(),
            )))
            .expect("codex OAuth provider id is unique in a fresh registry");
        registry
            .register(Arc::new(CopilotOAuthProvider::with_services(
                &endpoints.copilot,
                client,
            )))
            .expect("copilot OAuth provider id is unique in a fresh registry");
        registry
    }

    /// Register the three production OAuth providers (Anthropic PKCE, GitHub
    /// Copilot device-code, OpenAI Codex browser/device-code) with their
    /// production endpoints and client ids. This is the single source of truth
    /// the provider factory and the `/login` command consult.
    ///
    /// The endpoint and client-id constants below are pinned to the reviewed
    /// `.repo/pi-0.80.6` OAuth profiles. Tests remain offline and never contact
    /// these production endpoints.
    pub fn registry_with_builtins() -> Self {
        Self::registry_with_services(
            &OAuthEndpointConfig::production(),
            production_oauth_client(),
        )
    }
}

impl Default for OAuthProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for OAuthProviderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Manual Debug: print only the sorted id list. Never recurse into
        // provider internals — a future SecretString field cached on a provider
        // would otherwise leak via {:?}.
        f.debug_struct("OAuthProviderRegistry")
            .field("ids", &self.ids())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// /login and /logout command helpers (Phase 14.2 slice 6)
// ---------------------------------------------------------------------------

/// Presenter adapter that delays the success notification until the acquired
/// credential has been persisted. Provider failures still reach the real
/// presenter immediately.
struct DeferredSuccessPresenter<'a> {
    inner: &'a dyn LoginPresenter,
}

impl LoginPresenter for DeferredSuccessPresenter<'_> {
    fn select_login_method<'a>(
        &'a self,
        provider_id: &'a str,
        methods: &'a [OAuthLoginMethod],
        default: OAuthLoginMethod,
    ) -> BoxAuthFuture<'a, Result<OAuthLoginMethod, ProviderError>> {
        self.inner
            .select_login_method(provider_id, methods, default)
    }

    fn present_auth_url<'a>(
        &'a self,
        url: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        self.inner.present_auth_url(url)
    }

    fn present_device_code<'a>(
        &'a self,
        user_code: &'a str,
        verification_uri: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        self.inner.present_device_code(user_code, verification_uri)
    }

    fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
        self.inner.await_manual_code()
    }

    fn await_login_cancelled<'a>(&'a self) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        self.inner.await_login_cancelled()
    }

    fn notify_success(&self) {}

    fn notify_failure(&self, reason: &str) {
        self.inner.notify_failure(reason);
    }
}

fn store_error_to_provider(error: CredentialStoreError) -> ProviderError {
    let message = match error {
        CredentialStoreError::BackendUnavailable { .. } => "credential store unavailable",
        CredentialStoreError::Backend { reason, .. } if reason.contains("credential lock") => {
            "credential store lock failed"
        }
        CredentialStoreError::Backend { .. } => "credential store backend failed",
        CredentialStoreError::MalformedEnvelope { .. }
        | CredentialStoreError::CorruptMarker { .. }
        | CredentialStoreError::UnknownEnvelope { .. }
        | CredentialStoreError::UnexpectedCredentialKind { .. } => {
            "credential store data is invalid"
        }
        _ => "credential store operation failed",
    };
    ProviderError::Config(message.to_owned())
}

/// Run the OAuth login flow for `provider_id`, writing the resulting
/// credential to `store` on success. `store` is the locked keychain store;
/// the write acquires the cross-process mutation lock. Called by the explicit
/// `/login <provider>` production dispatcher.
///
/// `presenter` is the UX seam: the production `TuiLoginPresenter` drives the
/// real TUI, while tests inject a `MockLoginPresenter` to avoid the real
/// loopback server and stdin read.
pub async fn login_oauth(
    provider_id: &str,
    registry: &OAuthProviderRegistry,
    store: &crate::credential_store::KeychainCredentialStore,
    presenter: &dyn LoginPresenter,
) -> Result<(), ProviderError> {
    let oauth = registry
        .lookup(provider_id)
        .ok_or_else(|| ProviderError::Config(format!("unknown OAuth provider: {provider_id}")))?;
    let deferred_presenter = DeferredSuccessPresenter { inner: presenter };
    let cred = oauth.login(&deferred_presenter).await?;
    let stored: opi_ai::credential::Credential = cred.into();
    store
        .write(provider_id, &stored)
        .await
        .map_err(store_error_to_provider)?;
    presenter.notify_success();
    Ok(())
}

/// Delete the stored credential for `provider_id`. Called by the interactive
/// `/logout` command. The store's `delete` acquires the cross-process mutation
/// lock; no credential is required to be logged out.
pub async fn logout_credential(
    provider_id: &str,
    store: &crate::credential_store::KeychainCredentialStore,
) -> Result<(), ProviderError> {
    store
        .delete(provider_id)
        .await
        .map_err(store_error_to_provider)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::bind_loopback;

    #[tokio::test]
    async fn bind_loopback_binds_loopback_address() {
        let listener = bind_loopback().await.expect("loopback bind");
        let addr = listener.local_addr().expect("local_addr");
        assert!(addr.ip().is_loopback(), "non-loopback bind: {addr}");
        assert_eq!(addr.ip(), std::net::Ipv4Addr::new(127, 0, 0, 1));
    }
}
