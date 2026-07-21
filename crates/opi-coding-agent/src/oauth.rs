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
//! the loopback callback response. Token-endpoint error codes are classified
//! through a closed protocol-code mapping before any message is built; the raw
//! body (which could echo a submitted `code_verifier` or `refresh_token`) is
//! never surfaced. Token POSTs use a client with
//! `redirect::Policy::none()` so a 302 echo-redirect cannot leak the verifier.
//!
//! # Unstable
//!
//! Part of the **unstable 0.x extension substrate**. Breaking changes may occur
//! between minor versions without a major version bump.

use std::borrow::Cow;
use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use opi_ai::auth::{LoginPresenter, OAuthCredential, OAuthLoginMethod, OAuthProvider};
use opi_ai::credential::{BoxAuthFuture, CredentialStore, CredentialStoreError};
use opi_ai::provider::ProviderError;
use secrecy::{ExposeSecret, SecretString};
use time::OffsetDateTime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::process::{Child, ChildStdout, Command};
use tokio_util::sync::CancellationToken;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use sha2::{Digest, Sha256};

/// PKCE `code_verifier` length in raw bytes before base64url encoding. 48 bytes
/// encode to 64 URL-safe characters, within the RFC 7636 [43, 128] range.
const CODE_VERIFIER_BYTES: usize = 48;

/// One absolute deadline shared by every stage of a login flow.
#[derive(Clone, Copy)]
struct FlowBudget {
    deadline: tokio::time::Instant,
}

impl FlowBudget {
    fn new(duration: Duration) -> Result<Self, ProviderError> {
        let deadline = tokio::time::Instant::now()
            .checked_add(duration)
            .ok_or_else(|| ProviderError::Config("OAuth login timeout is too large".into()))?;
        Ok(Self { deadline })
    }

    async fn wait<F: Future>(&self, future: F) -> Result<F::Output, ProviderError> {
        tokio::time::timeout_at(self.deadline, future)
            .await
            .map_err(|_| ProviderError::Timeout)
    }

    async fn elapsed(&self) {
        tokio::time::sleep_until(self.deadline).await;
    }
}

async fn within_optional_budget<F: Future>(
    budget: Option<FlowBudget>,
    future: F,
) -> Result<F::Output, ProviderError> {
    match budget {
        Some(budget) => budget.wait(future).await,
        None => Ok(future.await),
    }
}

fn login_cancelled(presenter: &dyn LoginPresenter, provider_id: &'static str) -> ProviderError {
    presenter.notify_failure("login cancelled");
    ProviderError::LoginCancelled {
        provider_id: provider_id.to_owned(),
    }
}

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
    provider_id: &'static str,
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
    Input(PkceInputError),
    Io(io::Error),
}

#[derive(Clone, Copy)]
enum PkceInputError {
    MalformedUrl,
    MalformedQueryEscape,
    InvalidUtf8,
    MissingCode,
    MissingState,
    DuplicateCode,
    DuplicateState,
    StateMismatch,
}

impl PkceInputError {
    fn provider_error(self) -> ProviderError {
        let message = match self {
            Self::MalformedUrl => "oauth redirect URL malformed",
            Self::MalformedQueryEscape => "oauth redirect query escape malformed",
            Self::InvalidUtf8 => "oauth redirect query is not valid UTF-8",
            Self::MissingCode => "oauth redirect missing code",
            Self::MissingState => "oauth redirect missing state",
            Self::DuplicateCode => "oauth redirect has duplicate code",
            Self::DuplicateState => "oauth redirect has duplicate state",
            Self::StateMismatch => "oauth state mismatch",
        };
        ProviderError::Config(message.to_owned())
    }
}

#[derive(Clone, Copy)]
enum PkceInputKind {
    Callback,
    Manual,
}

/// The outcome of the 3-way `select!` race (callback vs manual-code vs timeout).
/// Distinct variants let `notify_failure` receive a fixed, non-secret reason.
enum LoginOutcome {
    CallbackCode(String),
    ManualCode(String),
    Cancelled,
    Timeout,
    CallbackInput(PkceInputError),
    CallbackIo(io::Error),
    ManualInput(PkceInputError),
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
async fn run_pkce_login<'a>(
    config: PkceLoginConfig,
    presenter: &'a dyn LoginPresenter,
    finalize_credential: fn(OAuthCredential) -> Result<OAuthCredential, ProviderError>,
    cancellation: Option<BoxAuthFuture<'a, Result<(), ProviderError>>>,
    existing_budget: Option<FlowBudget>,
) -> Result<OAuthCredential, ProviderError> {
    let budget = match existing_budget {
        Some(budget) => budget,
        None => FlowBudget::new(config.timeout)?,
    };
    let listener = budget
        .wait(bind_loopback())
        .await
        .inspect_err(|error| {
            if matches!(error, ProviderError::Timeout) {
                presenter.notify_failure("timeout");
            }
        })?
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

    budget
        .wait(presenter.present_auth_url(&authorize_url))
        .await
        .inspect_err(|error| {
            if matches!(error, ProviderError::Timeout) {
                presenter.notify_failure("timeout");
            }
        })??;

    let mut cancellation = cancellation.unwrap_or_else(|| presenter.await_login_cancelled());
    let outcome = tokio::select! {
        biased;
        cancelled = cancellation.as_mut() => {
            cancelled?;
            LoginOutcome::Cancelled
        },
        res = accept_one_callback(&listener, &state) => match res {
            Ok(code) => LoginOutcome::CallbackCode(code),
            Err(CallbackFail::Input(error)) => LoginOutcome::CallbackInput(error),
            Err(CallbackFail::Io(e)) => LoginOutcome::CallbackIo(e),
        },
        res = presenter.await_manual_code() => match res {
            Ok(input) => match normalize_pkce_input(&input, &state, PkceInputKind::Manual) {
                Ok(code) => LoginOutcome::ManualCode(code),
                Err(error) => LoginOutcome::ManualInput(error),
            },
            Err(e) => LoginOutcome::Manual(e),
        },
        _ = budget.elapsed() => LoginOutcome::Timeout,
    };
    // The listener is no longer needed: drop it so the port is freed and a
    // stale callback from a prior login cannot land on a still-bound socket.
    drop(listener);
    if !matches!(
        outcome,
        LoginOutcome::ManualCode(_) | LoginOutcome::ManualInput(_) | LoginOutcome::Manual(_)
    ) && let Err(error) = presenter.cancel_manual_code().await
    {
        presenter.notify_failure("manual code cleanup failed");
        return Err(error);
    }

    let code = match outcome {
        LoginOutcome::CallbackCode(code) | LoginOutcome::ManualCode(code) => code,
        LoginOutcome::Cancelled => {
            return Err(login_cancelled(presenter, config.provider_id));
        }
        LoginOutcome::Timeout => {
            presenter.notify_failure("timeout");
            return Err(ProviderError::Timeout);
        }
        LoginOutcome::CallbackInput(error) => {
            presenter.notify_failure(match error {
                PkceInputError::StateMismatch => "state mismatch",
                _ => "callback parse error",
            });
            return Err(error.provider_error());
        }
        LoginOutcome::CallbackIo(e) => {
            presenter.notify_failure("callback IO error");
            return Err(ProviderError::Config(format!(
                "oauth callback IO error: {e}"
            )));
        }
        LoginOutcome::ManualInput(error) => {
            presenter.notify_failure("manual redirect error");
            return Err(error.provider_error());
        }
        LoginOutcome::Manual(e) => {
            presenter.notify_failure("manual code error");
            return Err(e);
        }
    };

    // Token exchange runs OUTSIDE the select so a timeout firing mid-POST
    // cannot drop the POST future and consume the auth code irrecoverably.
    match exchange_authorization_code(&config, &code, &redirect_uri, &verifier, budget)
        .await
        .and_then(finalize_credential)
    {
        Ok(cred) => {
            presenter.notify_success();
            Ok(cred)
        }
        Err(e) => {
            presenter.notify_failure(if matches!(e, ProviderError::Timeout) {
                "timeout"
            } else {
                "token exchange failed"
            });
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
    budget: FlowBudget,
) -> Result<OAuthCredential, ProviderError> {
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", config.client_id.as_str()),
        ("code_verifier", verifier),
    ];
    let resp = budget
        .wait(config.client.post(&config.token_url).form(&params).send())
        .await?
        .map_err(|_| ProviderError::Network("token exchange failed".into()))?;
    let status = resp.status();
    if !status.is_success() {
        let body = budget.wait(resp.text()).await?.unwrap_or_default();
        return Err(token_endpoint_error(status, &body));
    }
    let token: TokenResponse = budget
        .wait(resp.json())
        .await?
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
        expires_at: Some(
            OffsetDateTime::now_utc()
                .checked_add(time::Duration::seconds(expires_in))
                .ok_or_else(|| {
                    ProviderError::Config("token response expires_in out of range".into())
                })?,
        ),
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
            return Err(CallbackFail::Input(PkceInputError::MalformedUrl));
        }
    }
    // Parse and validate the request before telling the browser the login
    // succeeded (C-4.1). A valid callback gets 200 + "Login complete"; an
    // invalid or state-mismatched callback gets a fixed secret-free 400. A
    // response is written and flushed before resolving on this path so a racing
    // manual-code-wins cancellation leaves a clean response. (An earlier read
    // error or an oversize request aborts without a response.)
    let req =
        std::str::from_utf8(&buf).map_err(|_| CallbackFail::Input(PkceInputError::InvalidUtf8))?;
    let target = req
        .split(' ')
        .nth(1)
        .ok_or(CallbackFail::Input(PkceInputError::MalformedUrl))?;
    let code_result = normalize_pkce_input(target, expected_state, PkceInputKind::Callback);
    let (status_line, body) = match &code_result {
        Ok(_) => (
            "HTTP/1.1 200 OK",
            "Login complete, you may close this window.",
        ),
        Err(_) => (
            "HTTP/1.1 400 Bad Request",
            "Login failed, return to the terminal.",
        ),
    };
    let resp = format!(
        "{status_line}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(resp.as_bytes())
        .await
        .map_err(CallbackFail::Io)?;
    stream.flush().await.map_err(CallbackFail::Io)?;
    code_result.map_err(CallbackFail::Input)
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
        .map(|secs| {
            OffsetDateTime::now_utc()
                .checked_add(time::Duration::seconds(secs))
                .ok_or_else(|| ProviderError::Config("refresh expires_in out of range".into()))
        })
        .transpose()?;
    Ok(OAuthCredential {
        access: SecretString::new(token.access_token.into_boxed_str()),
        refresh,
        expires_at,
        base_url,
        account_id: cred.account_id.clone(),
    })
}

/// Return a fixed class for a recognized OAuth protocol error code, or one
/// fixed unknown class. Arbitrary server strings never cross this boundary.
fn oauth_error_class(code: &str) -> &'static str {
    match code {
        "invalid_request" => "invalid_request",
        "invalid_client" => "invalid_client",
        "invalid_grant" => "invalid_grant",
        "unauthorized_client" => "unauthorized_client",
        "unsupported_grant_type" => "unsupported_grant_type",
        "invalid_scope" => "invalid_scope",
        "authorization_pending" => "authorization_pending",
        "slow_down" => "slow_down",
        "access_denied" => "access_denied",
        "expired_token" => "expired_token",
        "invalid_token" => "invalid_token",
        "insufficient_scope" => "insufficient_scope",
        "server_error" => "server_error",
        "temporarily_unavailable" => "temporarily_unavailable",
        _ => "unknown_oauth_error",
    }
}

/// Build a non-retryable auth error from a token-endpoint non-2xx response.
/// Only a closed OAuth protocol error class is surfaced: descriptions, unknown
/// codes, and raw bodies can echo submitted authorization or credential data.
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
    let msg = format!("token endpoint: {status} {}", oauth_error_class(code));
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

/// Normalize either a raw manually pasted code or a redirect URL. Callback
/// input is always treated as a redirect target; manual input is treated as a
/// redirect URL only when it starts with an HTTP(S) scheme.
fn normalize_pkce_input(
    input: &str,
    expected_state: &str,
    kind: PkceInputKind,
) -> Result<String, PkceInputError> {
    if matches!(kind, PkceInputKind::Manual) && input.trim().is_empty() {
        return Err(PkceInputError::MissingCode);
    }
    let is_redirect = match kind {
        PkceInputKind::Callback => true,
        PkceInputKind::Manual => {
            input
                .get(..7)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
                || input
                    .get(..8)
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
        }
    };
    if !is_redirect {
        return Ok(input.to_owned());
    }

    let url_input = if matches!(kind, PkceInputKind::Callback) && input.starts_with('/') {
        Cow::Owned(format!("http://127.0.0.1{input}"))
    } else {
        Cow::Borrowed(input)
    };
    let url = reqwest::Url::parse(&url_input).map_err(|_| PkceInputError::MalformedUrl)?;
    let query = url.query().unwrap_or("");
    let mut code = None;
    let mut state = None;
    for pair in query.split('&') {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        match name {
            "code" if code.is_some() => return Err(PkceInputError::DuplicateCode),
            "code" => code = Some(value),
            "state" if state.is_some() => return Err(PkceInputError::DuplicateState),
            "state" => state = Some(value),
            _ => {}
        }
    }
    let code = code
        .filter(|value| !value.is_empty())
        .ok_or(PkceInputError::MissingCode)?;
    let state = state
        .filter(|value| !value.is_empty())
        .ok_or(PkceInputError::MissingState)?;
    let code = strict_percent_decode(code)?;
    let state = strict_percent_decode(state)?;
    if state != expected_state {
        return Err(PkceInputError::StateMismatch);
    }
    Ok(code)
}

/// Strictly percent-decode a query value, rejecting malformed escapes and
/// decoded byte sequences that are not UTF-8.
fn strict_percent_decode(s: &str) -> Result<String, PkceInputError> {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(PkceInputError::MalformedQueryEscape);
            }
            let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) else {
                return Err(PkceInputError::MalformedQueryEscape);
            };
            out.push(h * 16 + l);
            i += 3;
            continue;
        }
        if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).map_err(|_| PkceInputError::InvalidUtf8)
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
/// `wiremock` server. The factory supplies a lazy Bearer credential source;
/// `AnthropicProvider` applies the OAuth beta header for that auth scheme.
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
            provider_id: "anthropic",
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
        Box::pin(async move {
            run_pkce_login(config, presenter, accept_oauth_credential, None, None).await
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

/// Upper bound for OAuth 2.0 device-authorization polling intervals. RFC 8628
/// recommends 5-60s; a malformed or hostile response supplying an extreme
/// value (e.g. `u64::MAX`) must not overflow `tokio::time::sleep` or a later
/// slow-down increment (C-2.3).
const MAX_DEVICE_INTERVAL: Duration = Duration::from_secs(600);

fn codex_device_interval(value: &serde_json::Value) -> Option<Duration> {
    let seconds = match value {
        serde_json::Value::Number(number) => number.as_u64(),
        serde_json::Value::String(text) => text.trim().parse::<u64>().ok(),
        _ => None,
    }?;
    if seconds > MAX_DEVICE_INTERVAL.as_secs() {
        return None;
    }
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
    budget: FlowBudget,
) -> Result<CodexDevicePoll, ProviderError> {
    let response = budget
        .wait(
            client
                .post(token_url)
                .json(&serde_json::json!({
                    "device_auth_id": device_auth_id,
                    "user_code": user_code,
                }))
                .send(),
        )
        .await?
        .map_err(|_| ProviderError::Network("device token poll failed".into()))?;
    let status = response.status();
    if status.is_success() {
        let token = budget
            .wait(response.json::<CodexDeviceToken>())
            .await?
            .map_err(|_| {
                ProviderError::Config("device token response missing required fields".into())
            })?;
        if token.authorization_code.is_empty() || token.code_verifier.is_empty() {
            return Err(ProviderError::Config(
                "device token response missing required fields".into(),
            ));
        }
        return Ok(CodexDevicePoll::Complete(token));
    }
    let body = budget.wait(response.text()).await?.unwrap_or_default();
    // Classify the structured OAuth error code FIRST. A terminal code
    // (`access_denied`/`expired_token`) delivered on HTTP 403/404 must surface
    // as `Denied`/`Expired` instead of falling through to the status-based
    // `Pending` fallback below (which would otherwise hang ~15 min until the
    // outer device-flow timeout fires).
    match codex_device_error_code(&body).as_deref() {
        Some("deviceauth_authorization_pending") | Some("authorization_pending") => {
            return Ok(CodexDevicePoll::Pending);
        }
        Some("slow_down") => return Ok(CodexDevicePoll::SlowDown),
        Some("access_denied") | Some("deviceauth_access_denied") => {
            return Ok(CodexDevicePoll::Denied);
        }
        Some("expired_token") | Some("deviceauth_expired") => {
            return Ok(CodexDevicePoll::Expired);
        }
        _ => {}
    }
    // No recognized error code: apply the status-based OpenAI-quirk fallback
    // for pending-shape 403/404 bodies.
    if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::NOT_FOUND {
        return Ok(CodexDevicePoll::Pending);
    }
    Err(ProviderError::AuthFailed(format!(
        "device authorization failed ({status})"
    )))
}

async fn run_codex_device_login_flow(
    provider: &CodexOAuthProvider,
    presenter: &dyn LoginPresenter,
    budget: FlowBudget,
    cancellation: &mut BoxAuthFuture<'_, Result<(), ProviderError>>,
) -> Result<OAuthCredential, ProviderError> {
    let response = tokio::select! {
        biased;
        cancelled = cancellation.as_mut() => {
            cancelled?;
            return Err(login_cancelled(presenter, "openai-codex"));
        }
        response = budget.wait(
            provider.client
                .post(&provider.device_user_code_url)
                .json(&serde_json::json!({"client_id": provider.client_id}))
                .send()
        ) => match response {
            Ok(response) => response.map_err(|_| {
                ProviderError::Network("device authorization request failed".into())
            })?,
            Err(ProviderError::Timeout) => {
                presenter.notify_failure("device authorization timed out");
                return Err(ProviderError::Timeout);
            }
            Err(error) => return Err(error),
        }
    };
    let status = response.status();
    if !status.is_success() {
        let body = tokio::select! {
            biased;
            cancelled = cancellation.as_mut() => {
                cancelled?;
                return Err(login_cancelled(presenter, "openai-codex"));
            }
            body = budget.wait(response.text()) => match body {
                Ok(body) => body.unwrap_or_default(),
                Err(ProviderError::Timeout) => {
                    presenter.notify_failure("device authorization timed out");
                    return Err(ProviderError::Timeout);
                }
                Err(error) => return Err(error),
            }
        };
        drop(body);
        presenter.notify_failure("device authorization request failed");
        return Err(ProviderError::AuthFailed(format!(
            "device authorization request failed ({status})"
        )));
    }
    let device = tokio::select! {
        biased;
        cancelled = cancellation.as_mut() => {
            cancelled?;
            return Err(login_cancelled(presenter, "openai-codex"));
        }
        device = budget.wait(response.json::<CodexDeviceAuthorization>()) => match device {
            Ok(device) => device.map_err(|_| {
                ProviderError::Config(
                    "device authorization response missing required fields".into(),
                )
            })?,
            Err(ProviderError::Timeout) => {
                presenter.notify_failure("device authorization timed out");
                return Err(ProviderError::Timeout);
            }
            Err(error) => return Err(error),
        }
    };
    let mut interval = codex_device_interval(&device.interval).ok_or_else(|| {
        ProviderError::Config("device authorization response has invalid interval".into())
    })?;
    if device.device_auth_id.is_empty() || device.user_code.is_empty() {
        return Err(ProviderError::Config(
            "device authorization response missing required fields".into(),
        ));
    }

    tokio::select! {
        biased;
        cancelled = cancellation.as_mut() => {
            cancelled?;
            return Err(login_cancelled(presenter, "openai-codex"));
        }
        result = budget.wait(presenter.present_device_code(
                &device.user_code,
                &provider.device_verification_uri,
            )) => match result {
            Ok(result) => result?,
            Err(ProviderError::Timeout) => {
                presenter.notify_failure("device authorization timed out");
                return Err(ProviderError::Timeout);
            }
            Err(error) => return Err(error),
        }
    }

    let token = loop {
        let outcome = tokio::select! {
            biased;
            cancelled = cancellation.as_mut() => {
                cancelled?;
                return Err(login_cancelled(presenter, "openai-codex"));
            }
            outcome = poll_codex_device_token(
                &provider.client,
                &provider.device_token_url,
                &device.device_auth_id,
                &device.user_code,
                budget,
            ) => outcome,
        };
        match outcome {
            Ok(CodexDevicePoll::Complete(token)) => break token,
            Ok(CodexDevicePoll::Pending) => {
                tokio::select! {
                    biased;
                    cancelled = cancellation.as_mut() => {
                        cancelled?;
                        return Err(login_cancelled(presenter, "openai-codex"));
                    }
                    delay = budget.wait(tokio::time::sleep(interval)) => {
                        if let Err(ProviderError::Timeout) = delay {
                            presenter.notify_failure("device authorization timed out");
                            return Err(ProviderError::Timeout);
                        }
                    }
                }
            }
            Ok(CodexDevicePoll::SlowDown) => {
                interval = interval
                    .saturating_add(Duration::from_secs(5))
                    .min(MAX_DEVICE_INTERVAL);
                tokio::select! {
                    biased;
                    cancelled = cancellation.as_mut() => {
                        cancelled?;
                        return Err(login_cancelled(presenter, "openai-codex"));
                    }
                    delay = budget.wait(tokio::time::sleep(interval)) => {
                        if let Err(ProviderError::Timeout) = delay {
                            presenter.notify_failure("device authorization timed out");
                            return Err(ProviderError::Timeout);
                        }
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
            Err(ProviderError::Timeout) => {
                presenter.notify_failure("device authorization timed out");
                return Err(ProviderError::Timeout);
            }
            Err(error) => {
                presenter.notify_failure("device authorization failed");
                return Err(error);
            }
        }
    };

    let exchange_config = PkceLoginConfig {
        provider_id: "openai-codex",
        authorize_url: String::new(),
        token_url: provider.token_url.clone(),
        client_id: provider.client_id.clone(),
        authorize_params: Vec::new(),
        client: provider.client.clone(),
        timeout: provider.device_timeout,
    };
    let credential = exchange_authorization_code(
        &exchange_config,
        &token.authorization_code,
        &provider.device_redirect_uri,
        &token.code_verifier,
        budget,
    )
    .await
    .and_then(require_codex_account_id);
    match credential {
        Ok(credential) => {
            presenter.notify_success();
            Ok(credential)
        }
        Err(ProviderError::Timeout) => {
            presenter.notify_failure("device authorization timed out");
            Err(ProviderError::Timeout)
        }
        Err(error) => {
            presenter.notify_failure("token exchange failed");
            Err(error)
        }
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
            let browser_budget = FlowBudget::new(self.browser_timeout)?;
            let device_budget = FlowBudget::new(self.device_timeout)?;
            let selection_budget = if browser_budget.deadline >= device_budget.deadline {
                browser_budget
            } else {
                device_budget
            };
            let mut cancellation = presenter.await_login_cancelled();
            let method_result = tokio::select! {
                biased;
                cancelled = cancellation.as_mut() => {
                    match cancelled {
                        Ok(()) | Err(ProviderError::LoginCancelled { .. }) => {
                            return Err(login_cancelled(presenter, "openai-codex"));
                        }
                        Err(error) => return Err(error),
                    }
                }
                method = presenter.select_login_method(
                    "openai-codex",
                    &[OAuthLoginMethod::Browser, OAuthLoginMethod::DeviceCode],
                    OAuthLoginMethod::Browser,
                ) => method,
                _ = selection_budget.elapsed() => {
                    if let Err(error) = presenter.cancel_manual_code().await {
                        presenter.notify_failure("manual code cleanup failed");
                        return Err(error);
                    }
                    presenter.notify_failure("login method selection timed out");
                    return Err(ProviderError::Timeout);
                }
            };
            let method = match method_result {
                Ok(method) => method,
                Err(ProviderError::LoginCancelled { .. }) => {
                    return Err(login_cancelled(presenter, "openai-codex"));
                }
                Err(error) => return Err(error),
            };
            match method {
                OAuthLoginMethod::Browser => {
                    let config = PkceLoginConfig {
                        provider_id: "openai-codex",
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
                    run_pkce_login(
                        config,
                        presenter,
                        require_codex_account_id,
                        Some(cancellation),
                        Some(browser_budget),
                    )
                    .await
                }
                OAuthLoginMethod::DeviceCode => {
                    run_codex_device_login_flow(self, presenter, device_budget, &mut cancellation)
                        .await
                }
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
    budget: FlowBudget,
) -> Result<DevicePollOutcome, ProviderError> {
    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("device_code", device_code),
        ("client_id", client_id),
    ];
    let resp = budget
        .wait(
            client
                .post(token_url)
                .header("accept", "application/json")
                .header("user-agent", COPILOT_USER_AGENT)
                .form(&params)
                .send(),
        )
        .await?
        .map_err(|_| ProviderError::Network("device token poll failed".into()))?;
    // GitHub returns 200 with an `error` body for pending/denied/expired and
    // 200 with `access_token` on success; tolerate either status by parsing
    // the body. The device_code is never surfaced on any path here.
    let body: DeviceTokenBody = budget
        .wait(resp.json())
        .await?
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
            "device authorization error: {}",
            oauth_error_class(other)
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
        budget: Option<FlowBudget>,
    ) -> Result<(SecretString, Option<OffsetDateTime>, Option<String>), ProviderError> {
        let resp = within_optional_budget(
            budget,
            client
                .get(copilot_token_url)
                .header("accept", "application/json")
                .header("user-agent", COPILOT_USER_AGENT)
                .header("Editor-Version", COPILOT_EDITOR_VERSION)
                .header("Editor-Plugin-Version", COPILOT_PLUGIN_VERSION)
                .header("Copilot-Integration-Id", COPILOT_INTEGRATION_ID)
                .bearer_auth(github_token)
                .send(),
        )
        .await?
        .map_err(|_| ProviderError::Network("copilot token exchange failed".into()))?;
        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            let _ = within_optional_budget(budget, resp.text()).await?;
            return Err(ProviderError::CredentialRevoked {
                provider_id: "github-copilot".to_owned(),
            });
        }
        if !status.is_success() {
            let body = within_optional_budget(budget, resp.text())
                .await?
                .unwrap_or_default();
            return Err(token_endpoint_error(status, &body));
        }
        let body: CopilotTokenBody =
            within_optional_budget(budget, resp.json())
                .await?
                .map_err(|e| {
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
            let budget = FlowBudget::new(total_budget)?;
            let mut cancellation = presenter.await_login_cancelled();
            // 1. Device-authorization request.
            let params = [("client_id", client_id.as_str()), ("scope", scope.as_str())];
            let resp = tokio::select! {
                biased;
                cancelled = cancellation.as_mut() => {
                    cancelled?;
                    return Err(login_cancelled(presenter, "github-copilot"));
                }
                response = budget.wait(
                    client
                        .post(&da_url)
                        .header("accept", "application/json")
                        .header("user-agent", COPILOT_USER_AGENT)
                        .form(&params)
                        .send(),
                ) => match response {
                    Ok(response) => response,
                    Err(ProviderError::Timeout) => {
                        presenter.notify_failure("device authorization timed out");
                        return Err(ProviderError::Timeout);
                    }
                    Err(error) => return Err(error),
                }
            }
            .map_err(|_| ProviderError::Network("device authorization request failed".into()))?;
            let status = resp.status();
            if !status.is_success() {
                let body = tokio::select! {
                    biased;
                    cancelled = cancellation.as_mut() => {
                        cancelled?;
                        return Err(login_cancelled(presenter, "github-copilot"));
                    }
                    body = budget.wait(resp.text()) => match body {
                        Ok(body) => body.unwrap_or_default(),
                        Err(ProviderError::Timeout) => {
                            presenter.notify_failure("device authorization timed out");
                            return Err(ProviderError::Timeout);
                        }
                        Err(error) => return Err(error),
                    }
                };
                presenter.notify_failure("device authorization request failed");
                return Err(token_endpoint_error(status, &body));
            }
            let da: DeviceAuthorizationBody = tokio::select! {
                biased;
                cancelled = cancellation.as_mut() => {
                    cancelled?;
                    return Err(login_cancelled(presenter, "github-copilot"));
                }
                body = budget.wait(resp.json()) => match body {
                    Ok(body) => body.map_err(|e| {
                        ProviderError::Config(format!("device authorization parse failed: {e}"))
                    })?,
                    Err(ProviderError::Timeout) => {
                        presenter.notify_failure("device authorization timed out");
                        return Err(ProviderError::Timeout);
                    }
                    Err(error) => return Err(error),
                }
            };
            // device_code is SECRET — local only, never passed to the presenter
            // or formatted into any error.
            let device_code = da.device_code;
            let user_code = da.user_code;
            let verification_uri = da.verification_uri;
            let mut interval = da
                .interval
                .filter(|secs| *secs <= MAX_DEVICE_INTERVAL.as_secs())
                .map(Duration::from_secs)
                .unwrap_or_else(|| Duration::from_secs(5));

            // 2. Present the public user_code (NEVER the device_code).
            tokio::select! {
                biased;
                cancelled = cancellation.as_mut() => {
                    cancelled?;
                    return Err(login_cancelled(presenter, "github-copilot"));
                }
                presented = presenter.present_device_code(&user_code, &verification_uri) => {
                    presented?;
                }
                _ = budget.elapsed() => {
                    presenter.notify_failure("device authorization timed out");
                    return Err(ProviderError::Timeout);
                }
            }

            // 3. Poll until a token, a terminal error, or the total budget elapses.
            let github_token = loop {
                let outcome = tokio::select! {
                    biased;
                    cancelled = cancellation.as_mut() => {
                        cancelled?;
                        return Err(login_cancelled(presenter, "github-copilot"));
                    }
                    r = poll_device_token(&client, &token_url, &device_code, &client_id, budget) => match r {
                        Ok(o) => o,
                        Err(ProviderError::Timeout) => {
                            presenter.notify_failure("device authorization timed out");
                            return Err(ProviderError::Timeout);
                        }
                        Err(e) => {
                            presenter.notify_failure("device authorization failed");
                            return Err(e);
                        }
                    },
                    _ = budget.elapsed() => {
                        presenter.notify_failure("device authorization timed out");
                        return Err(ProviderError::Timeout);
                    }
                };
                match outcome {
                    DevicePollOutcome::Pending | DevicePollOutcome::SlowDown => {
                        if matches!(outcome, DevicePollOutcome::SlowDown) {
                            // RFC 8628 §3.5: increase by exactly 5 seconds,
                            // persistently (not reset on the next pending).
                            interval = interval
                                .saturating_add(Duration::from_secs(5))
                                .min(MAX_DEVICE_INTERVAL);
                        }
                        tokio::select! {
                            biased;
                            cancelled = cancellation.as_mut() => {
                                cancelled?;
                                return Err(login_cancelled(presenter, "github-copilot"));
                            }
                            _ = tokio::time::sleep(interval) => {}
                            _ = budget.elapsed() => {
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
                Some(budget),
            )
            .await
            {
                Ok(triple) => triple,
                Err(ProviderError::Timeout) => {
                    presenter.notify_failure("device authorization timed out");
                    return Err(ProviderError::Timeout);
                }
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
                None,
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

const MANUAL_INPUT_POISONED: &str = "manual input unavailable after process termination failure";

enum ManualProcessError {
    Io(io::Error),
    Unreaped(io::Error),
}

enum ManualReadError {
    Cancelled,
    Io(io::Error),
    Poisoned,
}

type ManualIoFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ManualProcessError>> + Send + 'a>>;

trait ManualLineProcess: Send {
    fn wait_for_line<'a>(&'a mut self) -> ManualIoFuture<'a, String>;
    fn terminate<'a>(&'a mut self) -> ManualIoFuture<'a, ()>;
}

trait ManualLineProcessSpawner: Send + Sync + 'static {
    fn spawn(&self) -> io::Result<Box<dyn ManualLineProcess>>;
}

struct TerminalManualLineProcessSpawner;

struct ChildManualLineProcess {
    child: Child,
    stdout: Option<ChildStdout>,
}

impl ManualLineProcess for ChildManualLineProcess {
    fn wait_for_line<'a>(&'a mut self) -> ManualIoFuture<'a, String> {
        Box::pin(async move {
            let mut stdout = self.stdout.take().ok_or_else(|| {
                ManualProcessError::Io(io::Error::other("manual input stdout unavailable"))
            })?;
            // Drain stdout concurrently with awaiting the child so a line
            // larger than the pipe capacity cannot deadlock the child on write
            // against the parent blocked on exit (C-4.2). A strict cap rejects
            // oversized input before the pipe fills.
            const MAX_MANUAL_LINE: usize = 8 * 1024;
            let mut line = Vec::with_capacity(512);
            let mut buf = [0u8; 1024];
            let drain = async move {
                loop {
                    let n = stdout
                        .read(&mut buf)
                        .await
                        .map_err(ManualProcessError::Io)?;
                    if n == 0 {
                        break;
                    }
                    line.extend_from_slice(&buf[..n]);
                    if line.len() > MAX_MANUAL_LINE {
                        return Err(ManualProcessError::Io(io::Error::other(
                            "manual input exceeds maximum length",
                        )));
                    }
                }
                String::from_utf8(line).map_err(|_| {
                    ManualProcessError::Io(io::Error::other("manual input was not valid UTF-8"))
                })
            };
            let (status, line_result) = tokio::join!(self.child.wait(), drain);
            let status = status.map_err(ManualProcessError::Unreaped)?;
            if !status.success() {
                return Err(ManualProcessError::Io(io::Error::other(
                    "manual input process failed",
                )));
            }
            line_result
        })
    }

    fn terminate<'a>(&'a mut self) -> ManualIoFuture<'a, ()> {
        Box::pin(async move {
            if self
                .child
                .try_wait()
                .map_err(ManualProcessError::Unreaped)?
                .is_none()
            {
                self.child
                    .start_kill()
                    .map_err(ManualProcessError::Unreaped)?;
            }
            self.child
                .wait()
                .await
                .map_err(ManualProcessError::Unreaped)?;
            self.stdout.take();
            Ok(())
        })
    }
}

#[cfg(windows)]
fn terminal_manual_line_command() -> Command {
    // The script is static: the pasted code travels only through inherited
    // stdin and captured stdout, never through argv or the environment.
    const SCRIPT: &str =
        "$line = [Console]::ReadLine(); if ($null -ne $line) { [Console]::Out.WriteLine($line) }";
    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        SCRIPT,
    ]);
    command
}

#[cfg(not(windows))]
fn terminal_manual_line_command() -> Command {
    // Keep the command static for the same reason as the Windows variant.
    const SCRIPT: &str = "IFS= read -r line; printf '%s\\n' \"$line\"";
    let mut command = Command::new("/bin/sh");
    command.args(["-c", SCRIPT]);
    command
}

impl ManualLineProcessSpawner for TerminalManualLineProcessSpawner {
    fn spawn(&self) -> io::Result<Box<dyn ManualLineProcess>> {
        let mut command = terminal_manual_line_command();
        let mut child = command
            .stdin(Stdio::inherit())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("manual input stdout unavailable"))?;
        Ok(Box::new(ChildManualLineProcess {
            child,
            stdout: Some(stdout),
        }))
    }
}

struct ManualReadState {
    cancellation: CancellationToken,
    finished: AtomicBool,
    finished_changed: tokio::sync::Notify,
}

impl ManualReadState {
    fn new() -> Self {
        Self {
            cancellation: CancellationToken::new(),
            finished: AtomicBool::new(false),
            finished_changed: tokio::sync::Notify::new(),
        }
    }

    fn cancel(&self) {
        self.cancellation.cancel();
    }

    fn finish(&self) {
        self.finished.store(true, Ordering::SeqCst);
        self.finished_changed.notify_waiters();
    }

    async fn wait_finished(&self) {
        loop {
            let changed = self.finished_changed.notified();
            if self.finished.load(Ordering::SeqCst) {
                break;
            }
            changed.await;
        }
    }
}

struct ManualReadGuard {
    state: Arc<ManualReadState>,
}

impl Drop for ManualReadGuard {
    fn drop(&mut self) {
        self.state.cancel();
    }
}

struct ManualInputBroker {
    spawner: Arc<dyn ManualLineProcessSpawner>,
    serializer: Arc<tokio::sync::Mutex<()>>,
    active: Arc<Mutex<Option<Arc<ManualReadState>>>>,
    poisoned: Arc<AtomicBool>,
}

impl ManualInputBroker {
    fn new<S: ManualLineProcessSpawner>(spawner: S) -> Arc<Self> {
        Self::with_serializer(spawner, MANUAL_INPUT_SERIALIZER.clone())
    }

    fn with_serializer<S: ManualLineProcessSpawner>(
        spawner: S,
        serializer: Arc<tokio::sync::Mutex<()>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            spawner: Arc::new(spawner),
            serializer,
            active: Arc::new(Mutex::new(None)),
            poisoned: Arc::new(AtomicBool::new(false)),
        })
    }

    async fn read_line(&self) -> Result<String, ProviderError> {
        if self.poisoned.load(Ordering::SeqCst) {
            return Err(ProviderError::Config(MANUAL_INPUT_POISONED.into()));
        }
        let state = Arc::new(ManualReadState::new());
        let guard = ManualReadGuard {
            state: state.clone(),
        };
        let spawner = self.spawner.clone();
        let serializer = self.serializer.clone();
        let active = self.active.clone();
        let poisoned = self.poisoned.clone();
        let worker_state = state.clone();
        let (result, receiver) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let serialization_guard = serializer.lock_owned().await;
            let process_result = if poisoned.load(Ordering::SeqCst) {
                Err(ManualReadError::Poisoned)
            } else {
                *active.lock().unwrap() = Some(worker_state.clone());
                if worker_state.cancellation.is_cancelled() {
                    Err(ManualReadError::Cancelled)
                } else {
                    match spawner.spawn() {
                        Ok(mut process) => {
                            tokio::select! {
                                biased;
                                _ = worker_state.cancellation.cancelled() => {
                                    match process.terminate().await {
                                        Ok(()) => Err(ManualReadError::Cancelled),
                                        Err(ManualProcessError::Io(error))
                                        | Err(ManualProcessError::Unreaped(error)) => {
                                            drop(error);
                                            poisoned.store(true, Ordering::SeqCst);
                                            Err(ManualReadError::Poisoned)
                                        }
                                    }
                                }
                                line = process.wait_for_line() => match line {
                                    Ok(line) => Ok(line),
                                    Err(ManualProcessError::Io(error)) => {
                                        Err(ManualReadError::Io(error))
                                    }
                                    Err(ManualProcessError::Unreaped(error)) => {
                                        drop(error);
                                        poisoned.store(true, Ordering::SeqCst);
                                        Err(ManualReadError::Poisoned)
                                    }
                                },
                            }
                        }
                        Err(error) => Err(ManualReadError::Io(error)),
                    }
                }
            };
            let mut current = active.lock().unwrap();
            if current
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, &worker_state))
            {
                *current = None;
            }
            drop(current);
            drop(serialization_guard);
            worker_state.finish();
            let _ = result.send(process_result);
        });
        let result = receiver
            .await
            .map_err(|_| ProviderError::Config("manual input broker stopped".into()))?
            .map_err(|error| match error {
                ManualReadError::Cancelled => {
                    ProviderError::Config("manual input cancelled".into())
                }
                ManualReadError::Io(error) => {
                    ProviderError::Config(format!("stdin read failed: {error}"))
                }
                ManualReadError::Poisoned => ProviderError::Config(MANUAL_INPUT_POISONED.into()),
            });
        drop(guard);
        result
    }

    async fn cancel_active_and_wait(&self) -> Result<(), ProviderError> {
        let active = self.active.lock().unwrap().clone();
        if let Some(active) = active {
            active.cancel();
            active.wait_finished().await;
        }
        if self.poisoned.load(Ordering::SeqCst) {
            Err(ProviderError::Config(MANUAL_INPUT_POISONED.into()))
        } else {
            Ok(())
        }
    }
}

static MANUAL_INPUT_SERIALIZER: LazyLock<Arc<tokio::sync::Mutex<()>>> =
    LazyLock::new(|| Arc::new(tokio::sync::Mutex::new(())));
static MANUAL_INPUT_BROKER: LazyLock<Arc<ManualInputBroker>> =
    LazyLock::new(|| ManualInputBroker::new(TerminalManualLineProcessSpawner));

/// Production `LoginPresenter`. The presenter itself uses normal terminal IO:
/// `present_auth_url` prints the URL (no browser-open; headless/SSH parity),
/// `present_device_code` prints the public `user_code` + verification URI,
/// `await_manual_code` reads one line from stdin, and `notify_*` print a status
/// line. The interactive dispatcher suspends raw mode and the ratatui alternate
/// screen before invoking it, then restores both afterward. No method
/// logs access/refresh tokens, authorization codes, or device codes (only the
/// public `user_code` is shown via `present_device_code`).
pub struct TuiLoginPresenter {
    manual_input: Arc<ManualInputBroker>,
}

impl TuiLoginPresenter {
    /// Construct the print-only presenter.
    pub fn new() -> Self {
        Self {
            manual_input: MANUAL_INPUT_BROKER.clone(),
        }
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
            println!("Select OpenAI Codex login method:");
            println!("  1. Browser login (default)");
            println!("  2. Device code login (headless)");
            print!("Choice [1]: ");
            let _ = std::io::Write::flush(&mut std::io::stdout());
            let line = self.manual_input.read_line().await?;
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
            let line = self.manual_input.read_line().await?;
            Ok(line.trim().to_owned())
        })
    }

    fn await_login_cancelled<'a>(&'a self) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move {
            tokio::signal::ctrl_c()
                .await
                .map_err(|_| ProviderError::Config("login cancellation signal failed".into()))?;
            self.manual_input.cancel_active_and_wait().await?;
            Ok(())
        })
    }

    fn cancel_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        Box::pin(async move { self.manual_input.cancel_active_and_wait().await })
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

    fn cancel_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        self.inner.cancel_manual_code()
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
    if let Err(error) = store.write(provider_id, &stored).await {
        presenter.notify_failure("credential store write failed");
        return Err(store_error_to_provider(error));
    }
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
    use std::io;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    use opi_ai::auth::OAuthProvider;
    use opi_ai::provider::ProviderError;

    use super::{
        AnthropicOAuthProvider, CodexOAuthProvider, ManualInputBroker, ManualIoFuture,
        ManualLineProcess, ManualLineProcessSpawner, ManualProcessError, TuiLoginPresenter,
        bind_loopback,
    };

    #[tokio::test]
    async fn bind_loopback_binds_loopback_address() {
        let listener = bind_loopback().await.expect("loopback bind");
        let addr = listener.local_addr().expect("local_addr");
        assert!(addr.ip().is_loopback(), "non-loopback bind: {addr}");
        assert_eq!(addr.ip(), std::net::Ipv4Addr::new(127, 0, 0, 1));
    }

    #[derive(Default)]
    struct FakeProcessState {
        spawns: AtomicUsize,
        active: Arc<AtomicUsize>,
        max_active: Arc<AtomicUsize>,
        first_terminated: AtomicBool,
        retry_started_before_termination: AtomicBool,
        termination_error: AtomicBool,
        spawned: tokio::sync::Notify,
    }

    struct FakeProcessSpawner {
        state: Arc<FakeProcessState>,
    }

    impl ManualLineProcessSpawner for FakeProcessSpawner {
        fn spawn(&self) -> io::Result<Box<dyn ManualLineProcess>> {
            let call = self.state.spawns.fetch_add(1, Ordering::SeqCst);
            if call > 0 && !self.state.first_terminated.load(Ordering::SeqCst) {
                self.state
                    .retry_started_before_termination
                    .store(true, Ordering::SeqCst);
            }
            let active = self.state.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.state.max_active.fetch_max(active, Ordering::SeqCst);
            self.state.spawned.notify_one();
            Ok(Box::new(FakeProcess {
                call,
                state: self.state.clone(),
                finished: false,
            }))
        }
    }

    struct FakeProcess {
        call: usize,
        state: Arc<FakeProcessState>,
        finished: bool,
    }

    impl FakeProcess {
        fn finish(&mut self) {
            if !self.finished {
                self.finished = true;
                self.state.active.fetch_sub(1, Ordering::SeqCst);
            }
        }
    }

    impl ManualLineProcess for FakeProcess {
        fn wait_for_line<'a>(&'a mut self) -> ManualIoFuture<'a, String> {
            Box::pin(async move {
                if self.call == 0 {
                    return std::future::pending::<Result<String, ManualProcessError>>().await;
                }
                self.finish();
                Ok("retry-code\n".to_owned())
            })
        }

        fn terminate<'a>(&'a mut self) -> ManualIoFuture<'a, ()> {
            Box::pin(async move {
                tokio::time::sleep(Duration::from_millis(100)).await;
                if self.state.termination_error.load(Ordering::SeqCst) {
                    return Err(ManualProcessError::Unreaped(io::Error::other(
                        "termination-secret-canary",
                    )));
                }
                self.finish();
                if self.call == 0 {
                    self.state.first_terminated.store(true, Ordering::SeqCst);
                }
                Ok(())
            })
        }
    }

    fn fake_broker(state: Arc<FakeProcessState>) -> Arc<ManualInputBroker> {
        ManualInputBroker::with_serializer(
            FakeProcessSpawner { state },
            Arc::new(tokio::sync::Mutex::new(())),
        )
    }

    #[tokio::test]
    async fn manual_input_cancel_then_retry_never_starts_a_competing_reader() {
        let state = Arc::new(FakeProcessState::default());
        let broker = fake_broker(state.clone());

        let spawned = state.spawned.notified();
        let first_broker = broker.clone();
        let first = tokio::spawn(async move { first_broker.read_line().await });
        spawned.await;
        let drop_started = Instant::now();
        first.abort();
        let _ = first.await;
        assert!(
            drop_started.elapsed() < Duration::from_millis(50),
            "dropping a manual input future blocked on reader termination"
        );

        let second = broker.read_line().await.unwrap();
        assert_eq!(second, "retry-code\n");
        assert!(state.first_terminated.load(Ordering::SeqCst));
        assert!(
            !state
                .retry_started_before_termination
                .load(Ordering::SeqCst)
        );
        assert_eq!(state.active.load(Ordering::SeqCst), 0);
        assert_eq!(state.max_active.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn manual_input_cancellation_waits_for_process_termination() {
        let state = Arc::new(FakeProcessState::default());
        let broker = fake_broker(state.clone());
        let spawned = state.spawned.notified();
        let read_broker = broker.clone();
        let read = tokio::spawn(async move { read_broker.read_line().await });
        spawned.await;

        let cancellation_started = Instant::now();
        broker.cancel_active_and_wait().await.unwrap();

        assert!(
            cancellation_started.elapsed() >= Duration::from_millis(100),
            "cancellation returned before process termination"
        );
        assert!(state.first_terminated.load(Ordering::SeqCst));
        assert_eq!(state.active.load(Ordering::SeqCst), 0);
        let error = read.await.unwrap().expect_err("cancelled read");
        assert!(matches!(error, super::ProviderError::Config(_)));
    }

    #[tokio::test]
    async fn queued_manual_read_cancellation_preserves_active_owner_and_retry_exclusivity() {
        let state = Arc::new(FakeProcessState::default());
        let broker = fake_broker(state.clone());
        let spawned = state.spawned.notified();
        let owner_broker = broker.clone();
        let owner = tokio::spawn(async move { owner_broker.read_line().await });
        spawned.await;

        let mut queued = Box::pin(broker.read_line());
        assert!(
            tokio::time::timeout(Duration::from_millis(20), queued.as_mut())
                .await
                .is_err(),
            "queued read unexpectedly completed"
        );
        drop(queued);

        tokio::time::timeout(Duration::from_millis(500), broker.cancel_active_and_wait())
            .await
            .expect("queued cancellation replaced the active owner and deadlocked")
            .expect("active owner cleanup");
        let owner_error = owner.await.unwrap().expect_err("active owner cancelled");
        assert!(matches!(owner_error, super::ProviderError::Config(_)));

        let retry = broker.read_line().await.expect("retry after owner reap");
        assert_eq!(retry, "retry-code\n");
        assert!(state.first_terminated.load(Ordering::SeqCst));
        assert_eq!(state.spawns.load(Ordering::SeqCst), 2);
        assert_eq!(state.active.load(Ordering::SeqCst), 0);
        assert_eq!(state.max_active.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn unreaped_manual_process_poisons_broker_without_leaking_or_spawning_again() {
        let state = Arc::new(FakeProcessState::default());
        state.termination_error.store(true, Ordering::SeqCst);
        let broker = fake_broker(state.clone());
        let spawned = state.spawned.notified();
        let read_broker = broker.clone();
        let read = tokio::spawn(async move { read_broker.read_line().await });
        spawned.await;

        let cleanup_error =
            tokio::time::timeout(Duration::from_millis(500), broker.cancel_active_and_wait())
                .await
                .expect("termination failure deadlocked cancellation")
                .expect_err("unreaped cleanup must poison");
        let first_error = read.await.unwrap().expect_err("termination failure");
        let retry_error = broker
            .read_line()
            .await
            .expect_err("poisoned broker must reject retries");

        for error in [&cleanup_error, &first_error, &retry_error] {
            assert!(matches!(
                error,
                super::ProviderError::Config(message)
                    if message == "manual input unavailable after process termination failure"
            ));
            assert!(!format!("{error:?} {error}").contains("termination-secret-canary"));
        }
        assert_eq!(state.spawns.load(Ordering::SeqCst), 1);
        assert_eq!(state.max_active.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn pkce_timeout_waits_for_manual_process_reap_before_returning() {
        let state = Arc::new(FakeProcessState::default());
        let broker = fake_broker(state.clone());
        let presenter = TuiLoginPresenter {
            manual_input: broker.clone(),
        };
        let provider = AnthropicOAuthProvider::new(
            "https://authorize.example/oauth/authorize".to_owned(),
            "http://127.0.0.1:1/oauth/token".to_owned(),
            "client-id".to_owned(),
            Duration::from_millis(50),
        );
        let started = Instant::now();

        let error = provider.login(&presenter).await.expect_err("flow timeout");

        assert!(matches!(error, ProviderError::Timeout), "{error:?}");
        assert!(
            started.elapsed() >= Duration::from_millis(140),
            "PKCE returned before delayed manual-process reap"
        );
        assert!(state.first_terminated.load(Ordering::SeqCst));
        assert_eq!(state.active.load(Ordering::SeqCst), 0);
        let retry = broker.read_line().await.expect("retry after flow cleanup");
        assert_eq!(retry, "retry-code\n");
        assert_eq!(state.max_active.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn codex_method_selection_timeout_waits_for_manual_process_reap_before_returning() {
        let state = Arc::new(FakeProcessState::default());
        let broker = fake_broker(state.clone());
        let presenter = TuiLoginPresenter {
            manual_input: broker.clone(),
        };
        let provider = CodexOAuthProvider::new_with_device_endpoints(
            "https://authorize.example/oauth/authorize".to_owned(),
            "http://127.0.0.1:1/oauth/token".to_owned(),
            "http://127.0.0.1:1/api/accounts/deviceauth/usercode".to_owned(),
            "http://127.0.0.1:1/api/accounts/deviceauth/token".to_owned(),
            "https://auth.example/device".to_owned(),
            "https://auth.example/device/callback".to_owned(),
            "client-id".to_owned(),
            Duration::from_millis(50),
            Duration::from_millis(50),
        );
        let started = Instant::now();

        let error = provider
            .login(&presenter)
            .await
            .expect_err("method-selection timeout");

        assert!(matches!(error, ProviderError::Timeout), "{error:?}");
        assert!(
            started.elapsed() >= Duration::from_millis(140),
            "Codex returned before delayed method-selection process reap"
        );
        assert!(state.first_terminated.load(Ordering::SeqCst));
        assert_eq!(state.active.load(Ordering::SeqCst), 0);
        let retry = broker
            .read_line()
            .await
            .expect("retry after method-selection cleanup");
        assert_eq!(retry, "retry-code\n");
        assert_eq!(state.max_active.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn production_manual_input_does_not_use_crossterm_events() {
        let source = include_str!("oauth.rs");
        let forbidden = ["crossterm", "::event"].concat();
        assert!(!source.contains(&forbidden));
    }
}
