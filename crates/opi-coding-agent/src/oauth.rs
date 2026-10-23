//! OAuth provider registry plus PKCE/device-code flows.
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

use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use opi_ai::auth::oauth::{
    FlowBudget, login_cancelled, no_redirect_client, oauth_error_class, token_endpoint_error,
    within_optional_budget,
};
// PKCE primitives moved to the SDK; re-exported here because the host test
// suites pin their RFC 7636 behavior through this path.
pub use opi_ai::auth::oauth::{code_challenge_s256, generate_code_verifier, generate_state};
// The Anthropic and Codex legacy flows are SDK-owned; re-exported for the
// registry and tests.
pub use opi_ai::auth::oauth::anthropic::AnthropicOAuthProvider;
pub use opi_ai::auth::oauth::codex::{
    CODEX_DEVICE_REDIRECT_URI, CODEX_DEVICE_TIMEOUT, CODEX_DEVICE_TOKEN_URL,
    CODEX_DEVICE_USER_CODE_URL, CODEX_DEVICE_VERIFICATION_URI, CodexOAuthProvider,
};
use opi_ai::auth::{LoginPresenter, OAuthCredential, OAuthLoginMethod, OAuthProvider};
use opi_ai::credential::{BoxAuthFuture, CredentialStore, CredentialStoreError};
use opi_ai::provider::ProviderError;
use secrecy::{ExposeSecret, SecretString};
use time::OffsetDateTime;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, ChildStdout, Command};
use tokio_util::sync::CancellationToken;

fn safe_provider_summary(message: impl AsRef<str>) -> opi_ai::provider::ProviderErrorSummary {
    opi_ai::provider::ProviderErrorSummary::from_untrusted(message.as_ref())
}

fn provider_config_error(message: impl AsRef<str>) -> ProviderError {
    ProviderError::Config(safe_provider_summary(message))
}

fn provider_network_error(message: impl AsRef<str>) -> ProviderError {
    ProviderError::Network(safe_provider_summary(message))
}

// FlowBudget, the PKCE primitives, the loopback bind ladder, the shared PKCE
// runner, callback/manual parsing, the token transport, and the Anthropic
// provider moved to the SDK (`opi_ai::auth::oauth`); the imports above carry
// the re-exports this host module still uses.

// Token exchange/refresh transport and all parsing helpers moved to the SDK
// (`opi_ai::auth::oauth`): JSON and form encodings, callback/manual
// normalization, strict percent-decoding, and the closed token-endpoint
// error classes.

pub(crate) fn production_oauth_client() -> reqwest::Client {
    opi_ai::auth::oauth::production_oauth_client()
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

// AnthropicOAuthProvider moved to the SDK (`opi_ai::auth::oauth::anthropic`)
// and is re-exported above; this host module keeps only host policy.

// ---------------------------------------------------------------------------
// CopilotOAuthProvider (GitHub device-code, then Copilot token exchange)
// ---------------------------------------------------------------------------

/// Upper bound for device-authorization polling intervals (RFC 8628
/// recommends 5-60s); a hostile value must not overflow `tokio::time::sleep`
/// or a slow-down increment.
const MAX_DEVICE_INTERVAL: Duration = Duration::from_secs(600);

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
        .map_err(|_| provider_network_error("device token poll failed"))?;
    // GitHub returns 200 with an `error` body for pending/denied/expired and
    // 200 with `access_token` on success; tolerate either status by parsing
    // the body. The device_code is never surfaced on any path here.
    let body: DeviceTokenBody = budget
        .wait(resp.json())
        .await?
        .map_err(|e| provider_config_error(format!("device token response parse failed: {e}")))?;
    if let Some(token) = body.access_token {
        return Ok(DevicePollOutcome::Token(token));
    }
    match body.error.as_deref() {
        Some("authorization_pending") => Ok(DevicePollOutcome::Pending),
        Some("slow_down") => Ok(DevicePollOutcome::SlowDown),
        Some("access_denied") => Ok(DevicePollOutcome::Denied),
        Some("expired_token") => Ok(DevicePollOutcome::Expired),
        Some(other) => Err(provider_config_error(format!(
            "device authorization error: {}",
            oauth_error_class(other)
        ))),
        None => Err(provider_config_error(
            "device token response has no access_token and no error",
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
        .map_err(|_| provider_network_error("copilot token exchange failed"))?;
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
                    provider_config_error(format!("copilot token response parse failed: {e}"))
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
            .map_err(|_| provider_network_error("device authorization request failed"))?;
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
                        provider_config_error(format!("device authorization parse failed: {e}"))
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
            return Err(provider_config_error(MANUAL_INPUT_POISONED));
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
            .map_err(|_| provider_config_error("manual input broker stopped"))?
            .map_err(|error| match error {
                ManualReadError::Cancelled => provider_config_error("manual input cancelled"),
                ManualReadError::Io(error) => {
                    provider_config_error(format!("stdin read failed: {error}"))
                }
                ManualReadError::Poisoned => provider_config_error(MANUAL_INPUT_POISONED),
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
            Err(provider_config_error(MANUAL_INPUT_POISONED))
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
            // pi login methods: Anthropic offers browser + copy-code, Codex
            // offers browser + device code. Anything else is a typed
            // configuration failure.
            let (provider_label, second) = match (provider_id.as_str(), methods.as_slice()) {
                ("anthropic", [OAuthLoginMethod::Browser, OAuthLoginMethod::CopyCode]) => {
                    ("Anthropic", "Copy code login (headless)")
                }
                ("openai-codex", [OAuthLoginMethod::Browser, OAuthLoginMethod::DeviceCode]) => {
                    ("OpenAI Codex", "Device code login (headless)")
                }
                _ => {
                    return Err(provider_config_error(format!(
                        "OAuth provider '{provider_id}' supplied unsupported login methods"
                    )));
                }
            };
            if default != OAuthLoginMethod::Browser {
                return Err(provider_config_error(format!(
                    "OAuth provider '{provider_id}' supplied an invalid default login method"
                )));
            }
            println!("Select {provider_label} login method:");
            println!("  1. Browser login (default)");
            println!("  2. {second}");
            print!("Choice [1]: ");
            let _ = std::io::Write::flush(&mut std::io::stdout());
            let line = self.manual_input.read_line().await?;
            match line.trim() {
                "" | "1" => Ok(OAuthLoginMethod::Browser),
                "2" => Ok(methods[1]),
                "q" | "quit" | "cancel" => Err(ProviderError::LoginCancelled {
                    provider_id: provider_id.to_owned(),
                }),
                _ => Err(provider_config_error(format!(
                    "invalid {provider_label} login method"
                ))),
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
                .map_err(|_| provider_config_error("login cancellation signal failed"))?;
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
            .register(Arc::new(AnthropicOAuthProvider::with_http_client(
                endpoints.anthropic.authorize_url.clone(),
                endpoints.anthropic.token_url.clone(),
                endpoints.anthropic.client_id.clone(),
                endpoints.anthropic.login_timeout,
                client.clone(),
            )))
            .expect("anthropic OAuth provider id is unique in a fresh registry");
        let codex = &endpoints.codex;
        registry
            .register(Arc::new(CodexOAuthProvider::new_with_device_endpoints(
                codex.authorize_url.clone(),
                codex.token_url.clone(),
                codex.device_user_code_url.clone(),
                codex.device_token_url.clone(),
                codex.device_verification_uri.clone(),
                codex.device_redirect_uri.clone(),
                codex.client_id.clone(),
                codex.browser_timeout,
                codex.device_timeout,
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
// /login and /logout command helpers
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
    provider_config_error(message)
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
        .ok_or_else(|| provider_config_error(format!("unknown OAuth provider: {provider_id}")))?;
    let deferred_presenter = DeferredSuccessPresenter { inner: presenter };
    let cred = oauth.login(&deferred_presenter).await?;
    let stored: opi_ai::credential::Credential = cred.into();
    // Persist through the store-owned serialized mutation queue so login
    // serializes with logout/replace/refresh on the same store; the awaited
    // receipt is the ACK before success is reported.
    let receipt = match store
        .modify(provider_id, opi_ai::write_mutation(stored))
        .await
    {
        Ok(receipt) => receipt,
        Err(error) => {
            presenter.notify_failure("credential store write failed");
            return Err(store_error_to_provider(error));
        }
    };
    match receipt.outcome {
        opi_ai::mutation::MutationOutcome::Written { .. } => {}
        opi_ai::mutation::MutationOutcome::Failed {
            error: ref mutation_error,
        } => match &**mutation_error {
            opi_ai::mutation::MutationError::Store(store_error) => {
                presenter.notify_failure("credential store write failed");
                return Err(store_error_to_provider(store_error.clone()));
            }
            _ => {
                presenter.notify_failure("credential store write failed");
                return Err(provider_config_error("credential mutation failed"));
            }
        },
        _ => {
            presenter.notify_failure("credential store write outcome unknown");
            return Err(provider_config_error(
                "credential store write outcome unknown",
            ));
        }
    }
    presenter.notify_success();
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
    };

    #[tokio::test]
    async fn bind_loopback_binds_loopback_address() {
        // The bind ladder (preferred 53692, then a free loopback port, then
        // paste-back) moved to the SDK with the Anthropic flow; the unit
        // guard that the callback socket stays on loopback moves with it.
        // Here we pin the SDK's free-port fallback shape from the host side.
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("loopback bind");
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
                    if message == "[REDACTED]"
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
