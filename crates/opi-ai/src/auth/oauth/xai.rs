//! xAI (Grok/X subscription) OAuth device flow, pinned to pi 1.3.0
//! `packages/ai/src/auth/oauth/xai.ts` (sdk-contracts row 146).
//!
//! RFC 8628 device authorization grant against `https://auth.x.ai` with
//! form-encoded requests and JSON responses; the access token authenticates
//! inference as a plain API key (`toAuth = apiKey`). Verification URIs are
//! https-only (pi `validateVerificationUri`): an untrusted `verification_uri`
//! fails login, and an untrusted `verification_uri_complete` fails too — pi
//! validates before choosing `complete ?? plain`, it never coalesces onto the
//! plain URI. The device authorization requires a positive finite
//! `expires_in` (unlike Meta's default); `interval` falls back to the shared
//! 5-second default. Token responses require non-empty `access_token` and,
//! on login, `refresh_token`; `expires_in` defaults to 3600 seconds. Refresh
//! is a single pass (pi `refreshXaiToken` has no retry loop); a response
//! omitting `refresh_token` preserves the stored one (RFC 6749 §6, the
//! sdk-contracts row 146 fixture), and 401/403 is a dead credential for
//! relogin. Every request carries pi's 30-second deadline; the polling loop
//! follows pi's shared `device-code.ts` state machine: wait one interval
//! before the first poll, a 1-second interval floor, `slow_down` honoring a
//! server-provided interval or adding 5 seconds (RFC 8628 §3.5), and a device
//! deadline from `expires_in`. Failure bodies are never echoed; statuses and
//! fixed OAuth error classes are.

use std::time::Duration;

use crate::auth::oauth::{login_cancelled, provider_config_error, token_endpoint_error};
use crate::auth::{BoxAuthFuture, LoginPresenter, OAuthCredential, OAuthProvider};
use crate::provider::ProviderError;
use secrecy::{ExposeSecret, SecretString};

/// Fixed OAuth client id (pi `XAI_CLIENT_ID`).
pub const XAI_CLIENT_ID: &str = "b1a00492-073a-47ea-816f-4c329264a828";
/// Requested OAuth scope (pi `XAI_SCOPE`).
pub const XAI_SCOPE: &str = "openid profile email offline_access grok-cli:access api:access";
/// Device authorization endpoint (pi `XAI_DEVICE_CODE_URL`).
pub const XAI_DEVICE_CODE_URL: &str = "https://auth.x.ai/oauth2/device/code";
/// Token endpoint (pi `XAI_TOKEN_URL`).
pub const XAI_TOKEN_URL: &str = "https://auth.x.ai/oauth2/token";
/// Token lifetime default when a response omits `expires_in`
/// (pi `DEFAULT_TOKEN_LIFETIME_SECONDS`).
pub const XAI_DEFAULT_TOKEN_LIFETIME_SECONDS: f64 = 3600.0;
/// Poll interval default when the response omits `interval` (RFC 8628 §3.2).
pub const XAI_DEFAULT_POLL_INTERVAL_SECONDS: f64 = 5.0;
/// Per-request deadline (pi `REQUEST_TIMEOUT_MS`).
pub const XAI_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Lower bound for the poll interval (pi `MINIMUM_INTERVAL_MS`).
pub const XAI_MINIMUM_POLL_INTERVAL: Duration = Duration::from_secs(1);
/// `slow_down` increment when the server sends no new interval (RFC 8628 §3.5).
pub const XAI_SLOW_DOWN_INCREMENT: Duration = Duration::from_secs(5);

const DEVICE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
const PROVIDER_ID: &str = "xai";

/// Parsed device authorization response (pi `XaiDeviceCode`).
#[derive(Debug, Clone, PartialEq)]
pub struct XaiDeviceAuthorization {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub interval_seconds: f64,
    pub expires_in_seconds: f64,
}

/// The verification URI is opened in the user's browser; only https URLs
/// are trusted (pi `validateVerificationUri`).
fn trusted_https_url(value: &str) -> bool {
    match value.split_once("://") {
        Some((scheme, rest)) => scheme == "https" && !rest.is_empty(),
        None => false,
    }
}

fn positive_finite(value: &serde_json::Value) -> Option<f64> {
    value.as_f64().filter(|v| v.is_finite() && *v > 0.0)
}

fn invalid_device_response() -> ProviderError {
    provider_config_error("invalid device authorization response")
}

/// Parse and validate a device authorization response: `device_code`,
/// `user_code`, `verification_uri` (https only), and a positive finite
/// `expires_in` are required; a non-empty `verification_uri_complete` must
/// also be https; `interval` falls back to the 5-second default when absent
/// or not a positive finite number (RFC 8628 §3.2).
pub fn parse_xai_device_authorization(
    body: &serde_json::Value,
) -> Result<XaiDeviceAuthorization, ProviderError> {
    let string_field = |name: &str| -> Option<String> {
        body.get(name)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let device_code = string_field("device_code");
    let user_code = string_field("user_code");
    let verification_uri = string_field("verification_uri");
    let (device_code, user_code, verification_uri) =
        match (device_code, user_code, verification_uri) {
            (Some(d), Some(u), Some(v)) => (d, u, v),
            _ => return Err(invalid_device_response()),
        };
    if !trusted_https_url(&verification_uri) {
        return Err(provider_config_error(
            "untrusted verification uri in device authorization response",
        ));
    }
    // An untrusted complete URI fails rather than coalescing onto the plain
    // one: pi validates each candidate before choosing `complete ?? plain`.
    let verification_uri_complete = string_field("verification_uri_complete");
    if let Some(complete) = verification_uri_complete.as_deref()
        && !trusted_https_url(complete)
    {
        return Err(provider_config_error(
            "untrusted verification uri in device authorization response",
        ));
    }
    let expires_in_seconds = body
        .get("expires_in")
        .and_then(positive_finite)
        .ok_or_else(invalid_device_response)?;
    Ok(XaiDeviceAuthorization {
        device_code,
        user_code,
        verification_uri,
        verification_uri_complete,
        interval_seconds: body
            .get("interval")
            .and_then(positive_finite)
            .unwrap_or(XAI_DEFAULT_POLL_INTERVAL_SECONDS),
        expires_in_seconds,
    })
}

/// One device-token poll outcome (pi's poll result statuses).
enum XaiPoll {
    Complete {
        access: String,
        refresh: String,
        expires_in: f64,
    },
    Pending,
    SlowDown {
        interval_seconds: Option<f64>,
    },
}

/// Parse a success token response (pi `credentialsFromTokenResponse`):
/// `access_token` must be a non-empty string; `refresh_token` follows the
/// corpus ternary — absent with a previous token preserves it (RFC 6749 §6,
/// the sdk-contracts row 146 fixture), anything else (missing on login,
/// empty, or non-string) is a typed failure; `expires_in` defaults to 3600
/// seconds when absent and must be positive finite when present.
fn parse_token_response(
    body: &serde_json::Value,
    previous_refresh: Option<&str>,
) -> Result<XaiPoll, ProviderError> {
    let access = body
        .get("access_token")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(invalid_token_response)?;
    let refresh = if body.get("refresh_token").is_none() {
        previous_refresh
            .map(str::to_owned)
            .ok_or_else(invalid_token_response)?
    } else {
        body.get("refresh_token")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .ok_or_else(invalid_token_response)?
    };
    let expires_in = body
        .get("expires_in")
        .map(|v| positive_finite(v).ok_or_else(invalid_token_response))
        .transpose()?
        .unwrap_or(XAI_DEFAULT_TOKEN_LIFETIME_SECONDS);
    Ok(XaiPoll::Complete {
        access: access.to_owned(),
        refresh,
        expires_in,
    })
}

fn invalid_token_response() -> ProviderError {
    provider_config_error("device token response missing fields")
}

/// xAI OAuth provider: RFC 8628 device flow plus a single-pass
/// refresh-grant rotation.
pub struct XaiOAuthProvider {
    device_code_url: String,
    token_url: String,
    client: reqwest::Client,
}

/// Terminal poll failures, kept distinct so the login loop can notify the
/// matching user-facing reason while the typed error stays closed.
enum PollError {
    /// The token endpoint answered but the success body was unusable.
    InvalidResponse(ProviderError),
    /// The request failed (transport, ≥500, or an unrecognized OAuth error
    /// code classified through the fixed-class table).
    RequestFailed(ProviderError),
    /// `expired_token`: the device grant expired; the user must restart
    /// login (non-retryable revoked grant).
    Expired,
    /// `access_denied` or xAI's `authorization_denied` spelling.
    Denied,
}

impl XaiOAuthProvider {
    /// Production endpoints (`https://auth.x.ai`).
    pub fn production() -> Self {
        Self::new_with_endpoints(XAI_DEVICE_CODE_URL, XAI_TOKEN_URL)
    }

    /// Production endpoints with an injected HTTP client.
    pub fn with_http_client(client: reqwest::Client) -> Self {
        Self {
            device_code_url: XAI_DEVICE_CODE_URL.to_owned(),
            token_url: XAI_TOKEN_URL.to_owned(),
            client,
        }
    }

    /// Fully injected construction for tests and alternate deployments.
    pub fn new_with_endpoints(
        device_code_url: impl Into<String>,
        token_url: impl Into<String>,
    ) -> Self {
        Self {
            device_code_url: device_code_url.into(),
            token_url: token_url.into(),
            client: reqwest::Client::new(),
        }
    }

    /// POST a form body and read the JSON response under pi's 30-second
    /// per-request deadline. Transport failures are network-typed; an
    /// unparseable body becomes `Value::Null` so callers classify by status
    /// first (pi `readJson` yields null and the status check wins).
    async fn post_form_json(
        &self,
        url: &str,
        form: &[(&str, &str)],
    ) -> Result<(reqwest::StatusCode, serde_json::Value), ProviderError> {
        let send = self
            .client
            .post(url)
            .header("accept", "application/json")
            .form(form)
            .send();
        let response = tokio::time::timeout(XAI_REQUEST_TIMEOUT, send)
            .await
            .map_err(|_| ProviderError::Timeout)?
            .map_err(|e| {
                crate::auth::oauth::provider_network_error(format!("request failed: {e}"))
            })?;
        let status = response.status();
        let body = tokio::time::timeout(XAI_REQUEST_TIMEOUT, response.json::<serde_json::Value>())
            .await
            .map_err(|_| ProviderError::Timeout)?
            .unwrap_or(serde_json::Value::Null);
        Ok((status, body))
    }

    /// Start device authorization (pi `requestDeviceCode`): one form POST
    /// with the fixed client id, scope, and `referrer=pi`; non-ok or
    /// malformed responses fail typed without echoing the body.
    async fn start_device_authorization(
        &self,
        presenter: &dyn LoginPresenter,
    ) -> Result<XaiDeviceAuthorization, ProviderError> {
        let (status, body) = self
            .post_form_json(
                &self.device_code_url,
                &[
                    ("client_id", XAI_CLIENT_ID),
                    ("scope", XAI_SCOPE),
                    ("referrer", "pi"),
                ],
            )
            .await
            .map_err(|e| fail_start(presenter, e))?;
        if !status.is_success() {
            return Err(fail_start(
                presenter,
                crate::auth::oauth::provider_auth_error(format!(
                    "device authorization failed ({status})"
                )),
            ));
        }
        parse_xai_device_authorization(&body).map_err(|e| fail_start_response_invalid(presenter, e))
    }

    /// Poll the token endpoint once and classify (pi `pollForTokens`): a
    /// success body is parsed first (missing fields are typed), then the
    /// RFC 8628 error codes map to their outcomes with xAI's
    /// `authorization_denied` spelling accepted beside `access_denied`.
    async fn poll_device_token(
        &self,
        device_code: &str,
        previous_refresh: Option<&str>,
    ) -> Result<XaiPoll, PollError> {
        let (status, body) = self
            .post_form_json(
                &self.token_url,
                &[
                    ("client_id", XAI_CLIENT_ID),
                    ("device_code", device_code),
                    ("grant_type", DEVICE_GRANT_TYPE),
                ],
            )
            .await
            .map_err(PollError::RequestFailed)?;
        if status.is_success() {
            return parse_token_response(&body, previous_refresh)
                .map_err(PollError::InvalidResponse);
        }
        match body.get("error").and_then(|v| v.as_str()) {
            Some("authorization_pending") => Ok(XaiPoll::Pending),
            Some("slow_down") => Ok(XaiPoll::SlowDown {
                interval_seconds: body.get("interval").and_then(positive_finite),
            }),
            Some("expired_token") => Err(PollError::Expired),
            Some("access_denied") | Some("authorization_denied") => Err(PollError::Denied),
            _ => {
                // Status plus a fixed OAuth error class; the body detail
                // never crosses the boundary.
                Err(PollError::RequestFailed(token_endpoint_error(
                    status,
                    &body.to_string(),
                )))
            }
        }
    }
}

fn fail_start(presenter: &dyn LoginPresenter, error: ProviderError) -> ProviderError {
    presenter.notify_failure("device authorization failed");
    error
}

fn fail_start_response_invalid(
    presenter: &dyn LoginPresenter,
    error: ProviderError,
) -> ProviderError {
    presenter.notify_failure("device authorization response invalid");
    error
}

impl OAuthProvider for XaiOAuthProvider {
    fn id(&self) -> &str {
        PROVIDER_ID
    }

    fn name(&self) -> &str {
        "xAI (Grok/X subscription)"
    }

    fn is_subscription(&self) -> Option<bool> {
        Some(true)
    }

    fn login_label(&self) -> Option<&str> {
        Some("Sign in with SuperGrok or X Premium")
    }

    fn login<'a>(
        &'a self,
        presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            let mut cancellation = presenter.await_login_cancelled();
            let device = tokio::select! {
                biased;
                cancelled = cancellation.as_mut() => {
                    cancelled?;
                    return Err(login_cancelled(presenter, PROVIDER_ID));
                }
                device = self.start_device_authorization(presenter) => device?,
            };

            // Present the public user code with the COMPLETE verification
            // URI when present (pi notifies `verificationUriComplete ??
            // verificationUri`), never the device_code.
            let presented_uri = device
                .verification_uri_complete
                .clone()
                .unwrap_or_else(|| device.verification_uri.clone());
            tokio::select! {
                biased;
                cancelled = cancellation.as_mut() => {
                    cancelled?;
                    return Err(login_cancelled(presenter, PROVIDER_ID));
                }
                presented = presenter.present_device_code(
                    &device.user_code,
                    &presented_uri,
                ) => {
                    presented?;
                }
            }

            // pi `pollOAuthDeviceCodeFlow`: wait one interval before the
            // first poll, then poll under the device deadline.
            let mut interval = super::checked_poll_duration(device.interval_seconds)?
                .max(XAI_MINIMUM_POLL_INTERVAL);
            let deadline = super::checked_device_deadline(device.expires_in_seconds)?;
            tokio::select! {
                biased;
                cancelled = cancellation.as_mut() => {
                    cancelled?;
                    return Err(login_cancelled(presenter, PROVIDER_ID));
                }
                _ = tokio::time::sleep_until(deadline.min(
                    super::next_poll_deadline(deadline, interval),
                )) => {}
            }

            let mut saw_slow_down = false;
            loop {
                if tokio::time::Instant::now() >= deadline {
                    return Err(device_flow_timeout(presenter, saw_slow_down));
                }
                let outcome = tokio::select! {
                    biased;
                    cancelled = cancellation.as_mut() => {
                        cancelled?;
                        return Err(login_cancelled(presenter, PROVIDER_ID));
                    }
                    outcome = self.poll_device_token(&device.device_code, None) => outcome,
                };
                match outcome {
                    Ok(XaiPoll::Complete {
                        access,
                        refresh,
                        expires_in,
                    }) => {
                        let expires_at = super::checked_token_expiry(expires_in)?;
                        return Ok(OAuthCredential {
                            access: SecretString::new(access.into_boxed_str()),
                            refresh: SecretString::new(refresh.into_boxed_str()),
                            expires_at: Some(expires_at),
                            base_url: None,
                            account_id: None,
                            client_id: None,
                            scopes: None,
                            enterprise_domain: None,
                            available_model_ids: None,
                        });
                    }
                    Ok(XaiPoll::Pending) => {}
                    Ok(XaiPoll::SlowDown { interval_seconds }) => {
                        saw_slow_down = true;
                        interval =
                            match interval_seconds {
                                Some(seconds) => super::checked_poll_duration(seconds)?
                                    .max(XAI_MINIMUM_POLL_INTERVAL),
                                None => interval.checked_add(XAI_SLOW_DOWN_INCREMENT).ok_or_else(
                                    || {
                                        provider_config_error(
                                            "OAuth polling interval is out of range",
                                        )
                                    },
                                )?,
                            };
                    }
                    Err(PollError::Expired) => {
                        presenter.notify_failure("device code expired");
                        return Err(ProviderError::CredentialRevoked {
                            provider_id: PROVIDER_ID.to_owned(),
                        });
                    }
                    Err(PollError::Denied) => {
                        presenter.notify_failure("device authorization denied");
                        return Err(ProviderError::CredentialRevoked {
                            provider_id: PROVIDER_ID.to_owned(),
                        });
                    }
                    Err(PollError::InvalidResponse(error)) => {
                        presenter.notify_failure("device token response invalid");
                        return Err(error);
                    }
                    Err(PollError::RequestFailed(error)) => {
                        presenter.notify_failure("device token request failed");
                        return Err(error);
                    }
                }
                let next = super::next_poll_deadline(deadline, interval);
                tokio::select! {
                    biased;
                    cancelled = cancellation.as_mut() => {
                        cancelled?;
                        return Err(login_cancelled(presenter, PROVIDER_ID));
                    }
                    _ = tokio::time::sleep_until(next.min(deadline)) => {}
                }
            }
        })
    }

    fn refresh<'a>(
        &'a self,
        cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            // pi `refreshXaiToken`: a single pass with the refresh grant; a
            // missing `refresh_token` in the response preserves the stored
            // one, and 401/403 is a dead credential.
            let form = [
                ("client_id", XAI_CLIENT_ID),
                ("grant_type", "refresh_token"),
                ("refresh_token", cred.refresh.expose_secret()),
            ];
            let (status, body) = self.post_form_json(&self.token_url, &form).await?;
            if status == reqwest::StatusCode::UNAUTHORIZED
                || status == reqwest::StatusCode::FORBIDDEN
            {
                return Err(ProviderError::CredentialRevoked {
                    provider_id: PROVIDER_ID.to_owned(),
                });
            }
            if !status.is_success() {
                return Err(token_endpoint_error(status, &body.to_string()));
            }
            match parse_token_response(&body, Some(cred.refresh.expose_secret()))? {
                XaiPoll::Complete {
                    access,
                    refresh,
                    expires_in,
                } => {
                    let expires_at = super::checked_token_expiry(expires_in)?;
                    Ok(OAuthCredential {
                        access: SecretString::new(access.into_boxed_str()),
                        refresh: SecretString::new(refresh.into_boxed_str()),
                        expires_at: Some(expires_at),
                        base_url: cred.base_url.clone(),
                        account_id: cred.account_id.clone(),
                        client_id: cred.client_id.clone(),
                        scopes: cred.scopes.clone(),
                        enterprise_domain: cred.enterprise_domain.clone(),
                        available_model_ids: cred.available_model_ids.clone(),
                    })
                }
                _ => unreachable!("parse_token_response only completes"),
            }
        })
    }
}

/// pi's two deadline messages: the plain device-flow timeout and the
/// slow-down variant (its WSL clock-drift wording is host UX; the typed
/// error stays `Timeout`).
fn device_flow_timeout(presenter: &dyn LoginPresenter, saw_slow_down: bool) -> ProviderError {
    if saw_slow_down {
        presenter.notify_failure("device flow timed out after slow_down responses");
    } else {
        presenter.notify_failure("device flow timed out");
    }
    ProviderError::Timeout
}
