//! SDK OAuth login mechanism: shared PKCE authorization-code runner, JSON
//! token transport, and the store-acknowledged login entry.
//!
//! The runner is flow-mechanism only: HTTP goes through an injected
//! `reqwest::Client`, interaction through an injected
//! [`LoginPresenter`](crate::auth::LoginPresenter), and persistence through
//! an injected [`CredentialStore`](crate::credential::CredentialStore). No
//! browser is opened, no ambient
//! environment is consulted, and no keychain is read here.
//!
//! Wire shapes are pinned to pi 1.3.0
//! (`packages/ai/src/auth/oauth/{anthropic,pkce}.ts`): 32-byte base64url
//! PKCE verifiers with S256 challenges, a preferred callback port that falls
//! back to a free loopback port and finally to paste-back only, JSON token
//! requests with a 30-second per-request timeout, and real service expiry
//! (the single five-minute near-expiry correction lives in
//! [`OAuthCredential::needs_refresh`](crate::auth::OAuthCredential::needs_refresh),
//! never in a pre-subtracted stored expiry).

pub mod anthropic;
pub mod chatgpt;
pub mod codex;
pub mod copilot;

use std::borrow::Cow;
use std::future::Future;
use std::io;
use std::time::Duration;

use secrecy::{ExposeSecret, SecretString};
use time::OffsetDateTime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::auth::{LoginPresenter, OAuthCredential, OAuthLoginMethod, OAuthProvider};
use crate::credential::{BoxAuthFuture, CredentialStore, CredentialStoreError};
use crate::mutation::{MutationError, MutationOutcome};
use crate::provider::{ProviderError, ProviderErrorSummary};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use sha2::{Digest, Sha256};

/// PKCE verifier length in bytes (pi `pkce.ts`: 32 random bytes,
/// base64url-encoded without padding to a 43-character string).
pub const CODE_VERIFIER_BYTES: usize = 32;

/// Per-request timeout for JSON token requests (pi `anthropic.ts`
/// `postJson`: `AbortSignal.timeout(30_000)`).
const JSON_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

// ---------------------------------------------------------------------------
// Flow budget
// ---------------------------------------------------------------------------

/// Total login budget shared across one flow's interactions. Public so
/// multi-stage flows (browser-then-device) can thread one deadline through
/// the shared runner.
#[derive(Clone, Copy)]
pub struct FlowBudget {
    /// Absolute deadline for every budgeted interaction.
    pub deadline: tokio::time::Instant,
}

impl FlowBudget {
    pub fn new(duration: Duration) -> Result<Self, ProviderError> {
        let deadline = tokio::time::Instant::now()
            .checked_add(duration)
            .ok_or_else(|| provider_config_error("OAuth login timeout is too large"))?;
        Ok(Self { deadline })
    }

    pub async fn wait<F: Future>(&self, future: F) -> Result<F::Output, ProviderError> {
        tokio::time::timeout_at(self.deadline, future)
            .await
            .map_err(|_| ProviderError::Timeout)
    }

    pub async fn elapsed(&self) {
        tokio::time::sleep_until(self.deadline).await;
    }
}

pub fn login_cancelled(presenter: &dyn LoginPresenter, provider_id: &'static str) -> ProviderError {
    presenter.notify_failure("login cancelled");
    ProviderError::LoginCancelled {
        provider_id: provider_id.to_owned(),
    }
}

// ---------------------------------------------------------------------------
// PKCE primitives (pi pkce.ts)
// ---------------------------------------------------------------------------

/// Generate a cryptographically random PKCE `code_verifier` (43-128 char
/// URL-safe string) using the OS CSPRNG.
pub fn generate_code_verifier() -> String {
    let mut bytes = [0u8; CODE_VERIFIER_BYTES];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Compute the S256 PKCE `code_challenge` = base64url(sha256(verifier)) with
/// no padding, per RFC 7636.
pub fn code_challenge_s256(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

/// Generate a cryptographically random opaque `state` token for CSRF
/// protection in the authorization-code flow.
pub fn generate_state() -> String {
    let mut bytes = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

// ---------------------------------------------------------------------------
// Shared PKCE authorization-code runner
// ---------------------------------------------------------------------------

/// How the token endpoint receives its request body. Pi 1.3.0 sends JSON
/// (`anthropic.ts` `postJson`); flows not yet migrated keep form encoding
/// until their own cutover.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenRequestEncoding {
    /// `Content-Type`/`Accept: application/json` with a JSON body and a
    /// 30-second per-request timeout.
    Json,
    /// Classic `application/x-www-form-urlencoded` body.
    Form,
}

/// Configuration for one PKCE authorization-code login. Held by the provider
/// wrappers and passed by value into the shared runner.
#[derive(Clone)]
pub struct PkceLoginConfig {
    /// Stable provider id used in typed errors.
    pub provider_id: &'static str,
    /// Authorization endpoint shown to the user.
    pub authorize_url: String,
    /// Token endpoint exchanged against.
    pub token_url: String,
    /// OAuth client id.
    pub client_id: String,
    /// Extra authorize-URL query parameters (scope, provider flags).
    pub authorize_params: Vec<(String, String)>,
    /// HTTP client (redirects disabled by the constructors).
    pub client: reqwest::Client,
    /// Total login budget.
    pub timeout: Duration,
    /// Token-request body encoding.
    pub token_encoding: TokenRequestEncoding,
    /// Preferred callback port; falls back to a free loopback port when it
    /// cannot be bound, and finally to paste-back only.
    pub preferred_callback_port: Option<u16>,
    /// Path the callback server answers (pi anthropic: `/callback`).
    pub callback_path: String,
    /// Host named in the redirect URI (pi anthropic: `localhost`).
    pub redirect_host: String,
    /// Redirect URI used when no callback server could be bound at all
    /// (paste-back only). `None` keeps bind failure a typed error for flows
    /// without a manual-only fallback.
    pub manual_only_redirect_uri: Option<String>,
    /// What a failed bind of the preferred callback port does. Anthropic
    /// retries a free loopback port (pi `startOAuthCallbackServer` port-0
    /// fallback); Codex legacy goes straight to paste-back with the fixed
    /// redirect URI (pi `.catch(() => undefined)`).
    pub bind_fallback: CallbackBindFallback,
}

/// Behavior when the preferred callback port cannot be bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallbackBindFallback {
    /// Retry on a free loopback port (Anthropic ladder).
    FreePort,
    /// No server: paste-back only, with the flow's fixed redirect URI
    /// (Codex legacy: bind 1455 or paste).
    ManualOnly,
    /// The callback server must bind the preferred port; failure is a typed
    /// terminal error (OpenAI ChatGPT direct: port 1455 is shared with other
    /// sessions and the Codex CLI, and a foreign holder would reject the
    /// callback with a state mismatch — pi fails with a clear error instead).
    Required,
}

/// Why the loopback callback arm failed. Carries a static discriminant so
/// `notify_failure` can receive a fixed reason string (never the secret code).
enum CallbackFail {
    Parse(ParsedInputError),
    Io(io::Error),
}

#[derive(Clone, Copy, Debug)]
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
        provider_config_error(message)
    }
}

#[derive(Clone, Copy)]
pub(crate) enum PkceInputKind {
    Callback,
    Manual,
}

/// The outcome of the 3-way `select!` race (callback vs manual-code vs
/// timeout). Distinct variants let `notify_failure` receive a fixed,
/// non-secret reason.
enum LoginOutcome {
    CallbackCode(AuthzInput),
    ManualCode(AuthzInput),
    Cancelled,
    Timeout,
    CallbackParse(ParsedInputError),
    CallbackIo(io::Error),
    ManualParse(ParsedInputError),
    Manual(ProviderError),
}

/// A parsed authorization redirect input: the code plus any flow-specific
/// identity the redirect carried (OpenAI ChatGPT direct returns the issued
/// client id alongside the code).
pub(crate) struct AuthzInput {
    pub code: String,
    pub client_id: Option<String>,
}

/// A parse failure carrying the typed provider error plus a fixed,
/// secret-free `notify_failure` reason.
pub(crate) struct ParsedInputError {
    pub error: ProviderError,
    pub reason: &'static str,
}

/// Flow hook: parse a callback target or manual paste into an
/// [`AuthzInput`] validated against the expected CSRF state.
pub(crate) type InputParser = fn(&str, &str, PkceInputKind) -> Result<AuthzInput, ParsedInputError>;

/// Flow hook: exchange a parsed input for tokens. Receives the login config,
/// the input (code plus issued client id when the flow registers clients
/// dynamically), the CSRF state, the redirect URI used at authorization, the
/// PKCE verifier, and the shared login budget.
pub(crate) type ExchangeHook<'a> = &'a (
        dyn for<'x> Fn(
    &'x PkceLoginConfig,
    &'x AuthzInput,
    &'x str,
    &'x str,
    &'x str,
    FlowBudget,
) -> BoxAuthFuture<'x, Result<OAuthCredential, ProviderError>>
            + Send
            + Sync
    );

/// Base input parser shared by the fixed-client flows: the redirect URL must
/// carry `code` + matching `state` (paste-back accepts the documented
/// code-only formats); no flow identity rides the redirect.
fn base_parse_input(
    input: &str,
    expected_state: &str,
    kind: PkceInputKind,
) -> Result<AuthzInput, ParsedInputError> {
    let reason = |error: &PkceInputError| -> &'static str {
        match (kind, error) {
            (PkceInputKind::Callback, PkceInputError::StateMismatch) => "state mismatch",
            (PkceInputKind::Callback, _) => "callback parse error",
            (PkceInputKind::Manual, _) => "manual redirect error",
        }
    };
    match normalize_pkce_input(input, expected_state, kind) {
        Ok(code) => Ok(AuthzInput {
            code,
            client_id: None,
        }),
        Err(error) => Err(ParsedInputError {
            error: error.provider_error(),
            reason: reason(&error),
        }),
    }
}

fn callback_input_error(error: PkceInputError) -> ParsedInputError {
    ParsedInputError {
        error: error.provider_error(),
        reason: match error {
            PkceInputError::StateMismatch => "state mismatch",
            _ => "callback parse error",
        },
    }
}

/// Token-endpoint JSON response. `refresh_token` and `expires_in` are
/// optional per RFC 6749; the caller validates presence per flow (login
/// requires both, refresh accepts absence).
#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

/// Bind the callback listener per the flow's fallback policy: the preferred
/// port first, then `FreePort` retries a free loopback port (both failing is
/// the typed bind error), while `ManualOnly` returns `Ok(None)` so the login
/// continues paste-back only with the fixed redirect URI. Flows with no
/// preferred port treat a free-port bind failure as the typed error they
/// always were.
async fn bind_callback_listener(
    preferred: Option<u16>,
    fallback: CallbackBindFallback,
) -> io::Result<Option<TcpListener>> {
    if let Some(port) = preferred
        && let Ok(listener) = TcpListener::bind(("127.0.0.1", port)).await
    {
        return Ok(Some(listener));
    }
    if fallback == CallbackBindFallback::ManualOnly {
        return Ok(None);
    }
    if fallback == CallbackBindFallback::Required {
        // Surface the underlying bind error: the caller maps it to the
        // flow-pinned port-in-use message.
        let port = preferred.expect("Required fallback needs a preferred port");
        return Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            format!("port {port}"),
        ));
    }
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    Ok(Some(listener))
}

/// The redirect URI for a bound callback port, or the flow's fixed
/// paste-back-only URI when no server could be bound.
fn redirect_uri_for(config: &PkceLoginConfig, listener: Option<&TcpListener>) -> Option<String> {
    match listener {
        Some(listener) => listener.local_addr().ok().map(|addr| {
            format!(
                "http://{}:{}{}",
                config.redirect_host,
                addr.port(),
                config.callback_path
            )
        }),
        None => config.manual_only_redirect_uri.clone(),
    }
}

/// Run the PKCE authorization-code login: bind the callback ladder, build the
/// authorize URL, race callback/manual-code/timeout, then exchange the code
/// for tokens. `base_url` is `None` for PKCE flows (no per-credential base
/// URL); the caller does not override it.
pub async fn run_pkce_login<'a>(
    config: PkceLoginConfig,
    presenter: &'a dyn LoginPresenter,
    finalize_credential: fn(OAuthCredential) -> Result<OAuthCredential, ProviderError>,
    cancellation: Option<BoxAuthFuture<'a, Result<(), ProviderError>>>,
    existing_budget: Option<FlowBudget>,
) -> Result<OAuthCredential, ProviderError> {
    run_pkce_login_with(
        config,
        presenter,
        base_parse_input,
        &base_exchange,
        finalize_credential,
        cancellation,
        existing_budget,
    )
    .await
}

/// Base exchange hook for fixed-client flows: the shared
/// [`exchange_authorization_code`] against the login config.
fn base_exchange<'x>(
    config: &'x PkceLoginConfig,
    input: &'x AuthzInput,
    state: &'x str,
    redirect_uri: &'x str,
    verifier: &'x str,
    budget: FlowBudget,
) -> BoxAuthFuture<'x, Result<OAuthCredential, ProviderError>> {
    Box::pin(exchange_authorization_code(
        config,
        &input.code,
        state,
        redirect_uri,
        verifier,
        budget,
    ))
}

/// `run_pkce_login` with flow-supplied input parsing and token exchange.
/// OpenAI ChatGPT direct uses this: its callback and paste-back inputs carry
/// the issued client id (dynamic client registration), its manual input is a
/// strictly validated full redirect URL, and its token exchange posts the
/// issued client id and a `resource` parameter. The race skeleton, budget,
/// cancellation plumbing, and presenter notifications stay shared.
pub(crate) async fn run_pkce_login_with<'a>(
    config: PkceLoginConfig,
    presenter: &'a dyn LoginPresenter,
    parse_input: InputParser,
    exchange: ExchangeHook<'a>,
    finalize_credential: fn(OAuthCredential) -> Result<OAuthCredential, ProviderError>,
    cancellation: Option<BoxAuthFuture<'a, Result<(), ProviderError>>>,
    existing_budget: Option<FlowBudget>,
) -> Result<OAuthCredential, ProviderError> {
    let budget = match existing_budget {
        Some(budget) => budget,
        None => FlowBudget::new(config.timeout)?,
    };
    let listener = budget
        .wait(bind_callback_listener(
            config.preferred_callback_port,
            config.bind_fallback,
        ))
        .await
        .inspect_err(|error| {
            if matches!(error, ProviderError::Timeout) {
                presenter.notify_failure("timeout");
            }
        })?
        .map_err(|e| {
            if config.bind_fallback == CallbackBindFallback::Required {
                // Local crate-owned diagnostics for the CLI-shared port.
                presenter.notify_failure("callback port in use");
                ProviderError::Config(ProviderErrorSummary::attested_static(
                    "oauth callback port 1455 is in use: an unfinished login in another \
                     session or the Codex CLI may hold it; cancel that login and try again",
                ))
            } else {
                provider_config_error(format!("oauth loopback bind failed: {e}"))
            }
        })?;
    let Some(redirect_uri) = redirect_uri_for(&config, listener.as_ref()) else {
        return Err(provider_config_error(
            "oauth flow has no redirect uri and no manual fallback",
        ));
    };
    let verifier = generate_code_verifier();
    let challenge = code_challenge_s256(&verifier);
    let state = generate_state();
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
        res = accept_one_callback(listener.as_ref(), &state, parse_input) => match res {
            Ok(input) => LoginOutcome::CallbackCode(input),
            Err(CallbackFail::Parse(error)) => LoginOutcome::CallbackParse(error),
            Err(CallbackFail::Io(e)) => LoginOutcome::CallbackIo(e),
        },
        res = presenter.await_manual_code() => match res {
            Ok(input) => match parse_input(&input, &state, PkceInputKind::Manual) {
                Ok(input) => LoginOutcome::ManualCode(input),
                Err(error) => LoginOutcome::ManualParse(error),
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
        LoginOutcome::ManualCode(_) | LoginOutcome::ManualParse(_) | LoginOutcome::Manual(_)
    ) && let Err(error) = presenter.cancel_manual_code().await
    {
        presenter.notify_failure("manual code cleanup failed");
        return Err(error);
    }

    let input = match outcome {
        LoginOutcome::CallbackCode(input) | LoginOutcome::ManualCode(input) => input,
        LoginOutcome::Cancelled => {
            return Err(login_cancelled(presenter, config.provider_id));
        }
        LoginOutcome::Timeout => {
            presenter.notify_failure("timeout");
            return Err(ProviderError::Timeout);
        }
        LoginOutcome::CallbackParse(error) => {
            presenter.notify_failure(error.reason);
            return Err(error.error);
        }
        LoginOutcome::CallbackIo(e) => {
            presenter.notify_failure("callback IO error");
            return Err(provider_config_error(format!(
                "oauth callback IO error: {e}"
            )));
        }
        LoginOutcome::ManualParse(error) => {
            presenter.notify_failure(error.reason);
            return Err(error.error);
        }
        LoginOutcome::Manual(e) => {
            presenter.notify_failure("manual code error");
            return Err(e);
        }
    };

    // Token exchange runs OUTSIDE the select so a timeout firing mid-POST
    // cannot drop the POST future and consume the auth code irrecoverably.
    match exchange(&config, &input, &state, &redirect_uri, &verifier, budget)
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

/// Accept a single loopback callback, parse `code`+`state` from the query,
/// validate `state` against the expected CSRF token, and write a minimal HTTP
/// 200 response (no secret) before resolving. Single-accept: the listener is
/// dropped by the caller after this returns.
async fn accept_one_callback(
    listener: Option<&TcpListener>,
    expected_state: &str,
    parse_input: InputParser,
) -> Result<AuthzInput, CallbackFail> {
    let Some(listener) = listener else {
        // No callback server could be bound: this arm never wins.
        return std::future::pending().await;
    };
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
            return Err(CallbackFail::Parse(callback_input_error(
                PkceInputError::MalformedUrl,
            )));
        }
    }
    // Parse and validate the request before telling the browser the login
    // succeeded. A valid callback gets 200 + "Login complete"; an invalid or
    // state-mismatched callback gets a fixed secret-free 400. A response is
    // written and flushed before resolving on this path so a racing
    // manual-code-wins cancellation leaves a clean response. (An earlier read
    // error or an oversize request aborts without a response.)
    let req = std::str::from_utf8(&buf)
        .map_err(|_| CallbackFail::Parse(callback_input_error(PkceInputError::InvalidUtf8)))?;
    let target = req
        .split(' ')
        .nth(1)
        .ok_or_else(|| CallbackFail::Parse(callback_input_error(PkceInputError::MalformedUrl)))?;
    let code_result = parse_input(target, expected_state, PkceInputKind::Callback);
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
    code_result.map_err(CallbackFail::Parse)
}

/// Exchange an authorization code for tokens (PKCE login). Requires both
/// `refresh_token` and `expires_in` on login (hard error otherwise — never
/// produce a non-refreshable or always-refreshing credential). Returns a
/// credential with `base_url = None`; PKCE providers derive no base URL.
pub async fn exchange_authorization_code(
    config: &PkceLoginConfig,
    code: &str,
    state: &str,
    redirect_uri: &str,
    verifier: &str,
    budget: FlowBudget,
) -> Result<OAuthCredential, ProviderError> {
    let resp = match config.token_encoding {
        TokenRequestEncoding::Json => {
            let body = serde_json::json!({
                "grant_type": "authorization_code",
                "client_id": config.client_id,
                "code": code,
                "state": state,
                "redirect_uri": redirect_uri,
                "code_verifier": verifier,
            });
            let send = config
                .client
                .post(&config.token_url)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .json(&body)
                .send();
            // The total budget bounds the whole login; the 30-second
            // per-request deadline is the inner limit.
            match budget.wait(json_request_deadline(send)).await {
                Err(error) => return Err(error),
                Ok(Err(error)) => return Err(error),
                Ok(Ok(Err(_))) => return Err(provider_network_error("token exchange failed")),
                Ok(Ok(Ok(response))) => response,
            }
        }
        TokenRequestEncoding::Form => {
            let params = [
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", redirect_uri),
                ("client_id", config.client_id.as_str()),
                ("code_verifier", verifier),
            ];
            let send = config.client.post(&config.token_url).form(&params).send();
            unwrap_response(budget.wait(send).await, "token exchange failed").await?
        }
    };
    let status = resp.status();
    if !status.is_success() {
        let body = budget.wait(resp.text()).await?.unwrap_or_default();
        return Err(token_endpoint_error(status, &body));
    }
    let token: TokenResponse = budget
        .wait(resp.json())
        .await?
        .map_err(|e| provider_config_error(format!("token response parse failed: {e}")))?;
    let refresh = token
        .refresh_token
        .ok_or_else(|| provider_config_error("token response missing refresh_token"))?;
    let expires_in = token
        .expires_in
        .ok_or_else(|| provider_config_error("token response missing expires_in"))?;
    Ok(OAuthCredential {
        access: SecretString::new(token.access_token.into_boxed_str()),
        refresh: SecretString::new(refresh.into_boxed_str()),
        expires_at: Some(
            OffsetDateTime::now_utc()
                .checked_add(time::Duration::seconds(expires_in))
                .ok_or_else(|| provider_config_error("token response expires_in out of range"))?,
        ),
        base_url: None,
        account_id: None,
        enterprise_domain: None,
        available_model_ids: None,
        client_id: None,
    })
}

/// Field policy for a refresh response. Lenient keeps the product's RFC
/// 6749 §6 tolerance (a missing `refresh_token` reuses the old one, a
/// missing `expires_in` yields `expires_at = None`); Strict is pi
/// `readTokenResponse`: `access_token`, `refresh_token`, and `expires_in`
/// are all required (Codex legacy).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshFieldPolicy {
    /// Reuse the old refresh token / keep `expires_at = None` when absent.
    Lenient,
    /// Require the full token triple; anything missing is a typed error.
    Strict,
}

/// Refresh an OAuth credential by exchanging its refresh token. On 401/403
/// from the token endpoint the credential is revoked (non-retryable, no
/// auto-relogin). Under `Lenient`, a missing `refresh_token` in the response
/// reuses the old one (RFC 6749 §6) and a missing `expires_in` yields
/// `expires_at = None` (documented always-refresh); `Strict` fails typed on
/// any missing field. The stored expiry is always the real service
/// expiry: the single five-minute near-expiry correction is
/// [`OAuthCredential::needs_refresh`](crate::auth::OAuthCredential::needs_refresh).
#[allow(clippy::too_many_arguments)] // flat scalar refresh inputs; a params struct would not deepen the seam
pub async fn refresh_oauth_token(
    client: &reqwest::Client,
    token_url: &str,
    client_id: &str,
    cred: &OAuthCredential,
    provider_id: &str,
    base_url: Option<String>,
    encoding: TokenRequestEncoding,
    policy: RefreshFieldPolicy,
) -> Result<OAuthCredential, ProviderError> {
    let refresh_secret: &str = cred.refresh.expose_secret();
    let resp = match encoding {
        TokenRequestEncoding::Json => {
            let body = serde_json::json!({
                "grant_type": "refresh_token",
                "client_id": client_id,
                "refresh_token": refresh_secret,
            });
            let send = client
                .post(token_url)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .json(&body)
                .send();
            unwrap_response(json_request_deadline(send).await, "token refresh failed").await?
        }
        TokenRequestEncoding::Form => {
            let params = [
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_secret),
                ("client_id", client_id),
            ];
            let send = client.post(token_url).form(&params).send();
            unwrap_response(Ok(send.await), "token refresh failed").await?
        }
    };
    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        // Drain the body without surfacing it; a revoked credential is a
        // typed non-retryable error with no token interpolation.
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
        .map_err(|e| provider_config_error(format!("refresh response parse failed: {e}")))?;
    let refresh = match token.refresh_token {
        Some(s) => SecretString::new(s.into_boxed_str()),
        None => match policy {
            RefreshFieldPolicy::Lenient => cred.refresh.clone(),
            RefreshFieldPolicy::Strict => {
                return Err(provider_config_error(
                    "refresh response missing refresh_token",
                ));
            }
        },
    };
    let expires_at = match (token.expires_in, policy) {
        (Some(secs), _) => Some(
            OffsetDateTime::now_utc()
                .checked_add(time::Duration::seconds(secs))
                .ok_or_else(|| provider_config_error("refresh expires_in out of range"))?,
        ),
        (None, RefreshFieldPolicy::Lenient) => None,
        (None, RefreshFieldPolicy::Strict) => {
            return Err(provider_config_error("refresh response missing expires_in"));
        }
    };
    Ok(OAuthCredential {
        access: SecretString::new(token.access_token.into_boxed_str()),
        refresh,
        expires_at,
        base_url,
        account_id: cred.account_id.clone(),
        client_id: cred.client_id.clone(),
        enterprise_domain: cred.enterprise_domain.clone(),
        available_model_ids: cred.available_model_ids.clone(),
    })
}

/// Return a fixed class for a recognized OAuth protocol error code, or one
/// fixed unknown class. Arbitrary server strings never cross this boundary.
pub fn oauth_error_class(code: &str) -> &'static str {
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
/// Only a closed OAuth protocol error class is surfaced: descriptions,
/// unknown codes, and raw bodies can echo submitted authorization or
/// credential data.
pub fn token_endpoint_error(status: reqwest::StatusCode, body: &str) -> ProviderError {
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
    provider_auth_error(msg)
}

/// Percent-encode a query value (RFC 3986 unreserved set kept). Used to
/// build the authorize URL so `redirect_uri`/`client_id` are correctly
/// encoded.
pub(crate) fn url_encode(s: &str) -> String {
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

/// Normalize either a raw pasted code, a pasted redirect URL, a `code#state`
/// paste, or a `code=...&state=...` query paste (pi
/// `parseAuthorizationInput`). Callback input is always treated as a redirect
/// target. Manual pastes may omit `state`, which then means "the expected
/// state" exactly like pi's `parsed.state ?? verifier`.
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
        return parse_manual_paste(input, expected_state);
    }

    let url_input = if matches!(kind, PkceInputKind::Callback) && input.starts_with('/') {
        Cow::Owned(format!("http://127.0.0.1{input}"))
    } else {
        Cow::Borrowed(input)
    };
    let url = reqwest::Url::parse(&url_input).map_err(|_| PkceInputError::MalformedUrl)?;
    let query = url.query().unwrap_or("");
    // A pasted redirect URL must carry the state parameter; only the
    // copy-code paste formats may omit it.
    parse_code_state_query(query, expected_state, true)
}

/// Parse a `code`/`state` query string. Callback input requires both
/// parameters; manual input may omit `state` (it defaults to the expected
/// one).
fn parse_code_state_query(
    query: &str,
    expected_state: &str,
    require_state: bool,
) -> Result<String, PkceInputError> {
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
    let state = match state {
        Some(state) if !state.is_empty() => Some(state),
        _ if require_state => return Err(PkceInputError::MissingState),
        _ => None,
    };
    let code = strict_percent_decode(code)?;
    if let Some(state) = state {
        let state = strict_percent_decode(state)?;
        if state != expected_state {
            return Err(PkceInputError::StateMismatch);
        }
    }
    Ok(code)
}

/// Parse a non-URL manual paste: `code#state`, `code=...&state=...`, or a raw
/// code (pi `parseAuthorizationInput` minus the URL branch).
fn parse_manual_paste(input: &str, expected_state: &str) -> Result<String, PkceInputError> {
    if let Some((code, state)) = input.split_once('#') {
        if state.is_empty() {
            return Err(PkceInputError::MissingCode);
        }
        let state =
            strict_percent_decode(state).map_err(|_| PkceInputError::MalformedQueryEscape)?;
        if state != expected_state {
            return Err(PkceInputError::StateMismatch);
        }
        let code = strict_percent_decode(code).unwrap_or_else(|_| code.to_owned());
        if code.is_empty() {
            return Err(PkceInputError::MissingCode);
        }
        return Ok(code);
    }
    if input.contains("code=") {
        return parse_code_state_query(input, expected_state, false);
    }
    Ok(input.to_owned())
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

/// Run a future under an optional total budget. `None` runs it unbudgeted
/// (the codex device poll drives its own deadlines).
pub async fn within_optional_budget<F: Future>(
    budget: Option<FlowBudget>,
    future: F,
) -> Result<F::Output, ProviderError> {
    match budget {
        Some(budget) => budget.wait(future).await,
        None => Ok(future.await),
    }
}

/// Wrap one token-endpoint send in the 30-second per-request deadline pi
/// applies to JSON requests.
pub(crate) async fn json_request_deadline<F: Future>(send: F) -> Result<F::Output, ProviderError> {
    tokio::time::timeout(JSON_REQUEST_TIMEOUT, send)
        .await
        .map_err(|_| ProviderError::Timeout)
}

/// Flatten the timeout/transport response stack into one typed transport
/// failure with a fixed non-secret reason.
pub(crate) async fn unwrap_response(
    outer: Result<Result<reqwest::Response, reqwest::Error>, ProviderError>,
    message: &str,
) -> Result<reqwest::Response, ProviderError> {
    match outer {
        Err(error) => Err(error),
        Ok(Err(_)) => Err(provider_network_error(message)),
        Ok(Ok(response)) => Ok(response),
    }
}

/// Build a reqwest client that does NOT follow redirects, so a 302
/// echo-redirect from a token endpoint cannot leak the `code_verifier`.
pub fn no_redirect_client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("valid reqwest client")
}

/// Production HTTP client for OAuth flows (redirects disabled).
pub fn production_oauth_client() -> reqwest::Client {
    no_redirect_client()
}

// ---------------------------------------------------------------------------
// Store-acknowledged login (SDK)
// ---------------------------------------------------------------------------

/// Typed failure of a store-acknowledged login.
#[derive(Debug)]
pub enum LoginStoreError {
    /// The OAuth flow itself failed.
    Flow(ProviderError),
    /// The store rejected or failed the write (no success was reported).
    Store(CredentialStoreError),
    /// The mutation's outcome is unknown; the write may or may not have
    /// landed. Success is never reported for an unconfirmed outcome.
    Unknown {
        /// Redacted, non-secret loss detail.
        detail: String,
    },
}

/// Presenter wrapper that suppresses the flow's own success notification so
/// success is only presented after the store ACK (V-55: no success before
/// the ACK returns).
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

    fn cancel_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        self.inner.cancel_manual_code()
    }

    fn notify_success(&self) {
        // Deferred: the SDK reports success only after the store ACK.
    }

    fn notify_failure(&self, reason: &str) {
        self.inner.notify_failure(reason);
    }
}

/// Run one OAuth login and persist the resulting credential through the
/// store-owned serialized mutation queue, reporting success only after the
/// awaited ACK. Login serializes with logout/replace/refresh on the same
/// store; no automatic browser, keychain, environment fallback, or remote
/// revocation happens here.
pub async fn login_to_store(
    provider_id: &str,
    flow: &dyn OAuthProvider,
    store: &dyn CredentialStore,
    presenter: &dyn LoginPresenter,
) -> Result<OAuthCredential, LoginStoreError> {
    let deferred = DeferredSuccessPresenter { inner: presenter };
    let cred = flow.login(&deferred).await.map_err(LoginStoreError::Flow)?;
    // Persist through the store-owned serialized mutation queue; the awaited
    // receipt is the ACK before success is reported.
    let receipt = match store
        .modify(
            provider_id,
            crate::mutation::write_mutation(cred.clone().into()),
        )
        .await
    {
        Ok(receipt) => receipt,
        Err(error) => {
            presenter.notify_failure("credential store write failed");
            return Err(LoginStoreError::Store(error));
        }
    };
    match receipt.outcome {
        MutationOutcome::Written { .. } | MutationOutcome::Unchanged { .. } => {
            presenter.notify_success();
            Ok(cred)
        }
        MutationOutcome::Failed { error } => match &*error {
            MutationError::Store(store_error) => {
                presenter.notify_failure("credential store write failed");
                Err(LoginStoreError::Store(store_error.clone()))
            }
            _ => {
                presenter.notify_failure("credential store write failed");
                Err(LoginStoreError::Store(CredentialStoreError::Backend {
                    provider: provider_id.to_owned(),
                    reason: "credential mutation failed".to_owned(),
                }))
            }
        },
        MutationOutcome::Unknown { detail } => {
            presenter.notify_failure("credential store write outcome unknown");
            Err(LoginStoreError::Unknown { detail })
        }
        MutationOutcome::Deleted { .. } => {
            presenter.notify_failure("credential store write failed");
            Err(LoginStoreError::Store(CredentialStoreError::Backend {
                provider: provider_id.to_owned(),
                reason: "unexpected delete outcome for login".to_owned(),
            }))
        }
    }
}

// ---------------------------------------------------------------------------
// Local error constructors (match the product protocol's closed summaries)
// ---------------------------------------------------------------------------

/// Token-endpoint and flow failures keep the product protocol's fully
/// redacted summaries: arbitrary upstream text (including closed OAuth
/// error classes) never reaches `Display`, `Debug`, or diagnostics.
pub(crate) fn provider_config_error(message: impl AsRef<str>) -> ProviderError {
    ProviderError::Config(ProviderErrorSummary::from_untrusted(message))
}

pub(crate) fn provider_network_error(message: impl AsRef<str>) -> ProviderError {
    ProviderError::Network(ProviderErrorSummary::from_untrusted(message))
}

pub(crate) fn provider_auth_error(message: impl AsRef<str>) -> ProviderError {
    ProviderError::AuthFailed(ProviderErrorSummary::from_untrusted(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paste_with_hash_state_parses_and_validates() {
        assert_eq!(
            parse_manual_paste("code-1#expected", "expected").unwrap(),
            "code-1"
        );
        assert!(matches!(
            parse_manual_paste("code-1#other", "expected"),
            Err(PkceInputError::StateMismatch)
        ));
        assert_eq!(
            parse_manual_paste("raw-code", "expected").unwrap(),
            "raw-code"
        );
        assert_eq!(
            parse_manual_paste("code=a&state=expected", "expected").unwrap(),
            "a"
        );
    }

    #[test]
    fn redirect_uri_for_reports_the_manual_only_fallback() {
        let config = PkceLoginConfig {
            provider_id: "anthropic",
            authorize_url: String::new(),
            token_url: String::new(),
            client_id: String::new(),
            authorize_params: Vec::new(),
            client: no_redirect_client(),
            timeout: Duration::from_secs(1),
            token_encoding: TokenRequestEncoding::Json,
            preferred_callback_port: Some(53692),
            callback_path: "/callback".to_owned(),
            redirect_host: "localhost".to_owned(),
            manual_only_redirect_uri: Some("http://localhost:53692/callback".to_owned()),
            bind_fallback: CallbackBindFallback::FreePort,
        };
        assert_eq!(
            redirect_uri_for(&config, None).unwrap(),
            "http://localhost:53692/callback"
        );
    }
}
