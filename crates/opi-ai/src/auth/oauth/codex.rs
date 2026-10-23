//! OpenAI Codex (ChatGPT OAuth) legacy flow, pinned to pi 1.3.0
//! `packages/ai/src/auth/oauth/openai-codex.ts`.
//!
//! Wire contract: form-urlencoded token exchange/refresh, the fixed
//! `http://localhost:1455/auth/callback` redirect URI whose port is shared
//! with the Codex CLI — a failed bind falls back to paste-back only, never a
//! free port — OpenAI's device-code flow (`deviceauth` usercode/token
//! endpoints with the 403/404 pending quirk), strictly required token
//! response fields, and the mandatory ChatGPT account id extracted from the
//! access-token JWT. Expiry is stored as the real service expiry; the single
//! five-minute near-expiry correction is
//! [`OAuthCredential::needs_refresh`](crate::auth::OAuthCredential::needs_refresh).

use std::time::Duration;

use crate::auth::oauth::{
    CallbackBindFallback, FlowBudget, PkceLoginConfig, RefreshFieldPolicy, TokenRequestEncoding,
    exchange_authorization_code, login_cancelled, no_redirect_client, provider_auth_error,
    provider_config_error, provider_network_error, refresh_oauth_token, run_pkce_login,
};
use crate::auth::{
    BoxAuthFuture, LoginPresenter, OAuthCredential, OAuthLoginMethod, OAuthProvider,
};
use crate::provider::ProviderError;
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use secrecy::ExposeSecret as _;

/// Production authorization endpoint (pi `AUTHORIZE_URL`).
pub const CODEX_AUTHORIZE_URL: &str = "https://auth.openai.com/oauth/authorize";
/// Production token endpoint (pi `TOKEN_URL`).
pub const CODEX_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
/// Production OAuth client id (pi `CLIENT_ID`).
pub const CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
/// Fixed browser redirect URI (pi `REDIRECT_URI`): the port is shared with
/// the Codex CLI. When it cannot be bound the login continues paste-back
/// only with this same URI — never a free port.
pub const CODEX_REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
/// Device usercode endpoint (pi `DEVICE_USER_CODE_URL`).
pub const CODEX_DEVICE_USER_CODE_URL: &str =
    "https://auth.openai.com/api/accounts/deviceauth/usercode";
/// Device token poll endpoint (pi `DEVICE_TOKEN_URL`).
pub const CODEX_DEVICE_TOKEN_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/token";
/// Verification URI shown next to the user code (pi
/// `DEVICE_VERIFICATION_URI`).
pub const CODEX_DEVICE_VERIFICATION_URI: &str = "https://auth.openai.com/codex/device";
/// Redirect URI for the device-code exchange (pi `DEVICE_REDIRECT_URI`).
pub const CODEX_DEVICE_REDIRECT_URI: &str = "https://auth.openai.com/deviceauth/callback";
/// Device-code total budget (pi `DEVICE_CODE_TIMEOUT_SECONDS`, 15 minutes).
pub const CODEX_DEVICE_TIMEOUT: Duration = Duration::from_secs(15 * 60);
/// Requested scopes (pi `SCOPE`).
pub const CODEX_SCOPES: &str = "openid profile email offline_access";
/// JWT claim path carrying the ChatGPT account id (pi `JWT_CLAIM_PATH`).
pub const CODEX_AUTH_CLAIM_PATH: &str = "https://api.openai.com/auth";

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

/// OpenAI Codex OAuth provider supporting browser PKCE with the fixed
/// `localhost:1455` loopback redirect and OpenAI's device-code flow.
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
    /// Construct with production endpoints, endpoints' default timeouts, and
    /// an injected HTTP client.
    pub fn with_http_client(client: reqwest::Client) -> Self {
        Self {
            authorize_url: CODEX_AUTHORIZE_URL.to_owned(),
            token_url: CODEX_TOKEN_URL.to_owned(),
            device_user_code_url: CODEX_DEVICE_USER_CODE_URL.to_owned(),
            device_token_url: CODEX_DEVICE_TOKEN_URL.to_owned(),
            device_verification_uri: CODEX_DEVICE_VERIFICATION_URI.to_owned(),
            device_redirect_uri: CODEX_DEVICE_REDIRECT_URI.to_owned(),
            client_id: CODEX_CLIENT_ID.to_owned(),
            client,
            browser_timeout: Duration::from_secs(300),
            device_timeout: CODEX_DEVICE_TIMEOUT,
        }
    }

    /// Construct with production endpoints and a redirect-disabled client.
    pub fn production() -> Self {
        Self::with_http_client(no_redirect_client())
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

/// Extract the ChatGPT account id from the access-token JWT (pi
/// `getAccountId` + `credentialsFromToken`): a token without a non-empty
/// `chatgpt_account_id` under the `https://api.openai.com/auth` claim is a
/// typed account-id failure, never a credential.
pub fn require_codex_account_id(
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
        .map_err(|_| provider_network_error("device token poll failed"))?;
    let status = response.status();
    if status.is_success() {
        let token = budget
            .wait(response.json::<CodexDeviceToken>())
            .await?
            .map_err(|_| provider_config_error("device token response missing required fields"))?;
        if token.authorization_code.is_empty() || token.code_verifier.is_empty() {
            return Err(provider_config_error(
                "device token response missing required fields",
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
    Err(provider_auth_error(format!(
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
                provider_network_error("device authorization request failed")
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
        return Err(provider_auth_error(format!(
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
                provider_config_error("device authorization response missing required fields")
            })?,
            Err(ProviderError::Timeout) => {
                presenter.notify_failure("device authorization timed out");
                return Err(ProviderError::Timeout);
            }
            Err(error) => return Err(error),
        }
    };
    let mut interval = codex_device_interval(&device.interval).ok_or_else(|| {
        provider_config_error("device authorization response has invalid interval")
    })?;
    if device.device_auth_id.is_empty() || device.user_code.is_empty() {
        return Err(provider_config_error(
            "device authorization response missing required fields",
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
        token_encoding: TokenRequestEncoding::Form,
        preferred_callback_port: None,
        callback_path: "/".to_owned(),
        redirect_host: "127.0.0.1".to_owned(),
        manual_only_redirect_uri: None,
        bind_fallback: CallbackBindFallback::FreePort,
    };
    let credential = exchange_authorization_code(
        &exchange_config,
        &token.authorization_code,
        "",
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
                            ("scope".into(), CODEX_SCOPES.into()),
                            ("id_token_add_organizations".into(), "true".into()),
                            ("codex_cli_simplified_flow".into(), "true".into()),
                            ("originator".into(), "opi".into()),
                        ],
                        client: self.client.clone(),
                        timeout: self.browser_timeout,
                        token_encoding: TokenRequestEncoding::Form,
                        // The 1455 port is shared with the Codex CLI (pi
                        // `REDIRECT_URI`): bind it or fall back to paste-back
                        // only with the same fixed URI — never a free port.
                        preferred_callback_port: Some(1455),
                        callback_path: "/auth/callback".to_owned(),
                        redirect_host: "localhost".to_owned(),
                        manual_only_redirect_uri: Some(CODEX_REDIRECT_URI.to_owned()),
                        bind_fallback: CallbackBindFallback::ManualOnly,
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
                OAuthLoginMethod::CopyCode => Err(provider_config_error(
                    "OAuth provider 'openai-codex' supplied an invalid login method",
                )),
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
                TokenRequestEncoding::Form,
                // pi `readTokenResponse` requires the full token triple on
                // refresh; a partial response is never a credential.
                RefreshFieldPolicy::Strict,
            )
            .await?;
            require_codex_account_id(credential)
        })
    }
}
