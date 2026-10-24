//! Kimi Code (subscription) OAuth device flow, pinned to pi 1.3.0
//! `packages/ai/src/auth/oauth/kimi-coding.ts` (sdk-contracts row 144).
//!
//! RFC 8628 device authorization grant against `https://auth.kimi.com` with
//! form-encoded requests and JSON responses. The access token authenticates
//! requests to `https://api.kimi.com/coding` as an `Authorization: Bearer`
//! header (standard per-request auth; no flow-specific header plumbing).
//! Every request carries pi's 30-second deadline; the polling loop follows
//! pi's shared `device-code.ts` state machine: wait one interval before the
//! first poll, a 1-second interval floor, `slow_down` honoring a
//! server-provided interval or adding 5 seconds (RFC 8628 §3.5), and a
//! device deadline from `expires_in` (default 15 minutes). pi's
//! `KIMI_CODE_OAUTH_HOST`/`KIMI_OAUTH_HOST` environment override is host
//! policy: the SDK takes the OAuth host by construction. Failure bodies are
//! never echoed; statuses and fixed OAuth error classes are.

use std::time::Duration;

use crate::auth::oauth::{login_cancelled, provider_config_error, token_endpoint_error};
use crate::auth::{BoxAuthFuture, LoginPresenter, OAuthCredential, OAuthProvider};
use crate::provider::ProviderError;
use secrecy::{ExposeSecret, SecretString};
use time::OffsetDateTime;

/// Fixed OAuth client id (pi `CLIENT_ID`).
pub const KIMI_CLIENT_ID: &str = "17e5f671-d194-4dfb-9706-5516cb48c098";
/// Production OAuth host (pi `DEFAULT_OAUTH_HOST`).
pub const KIMI_DEFAULT_OAUTH_HOST: &str = "https://auth.kimi.com";
/// Device-flow deadline default when the response omits `expires_in`
/// (pi `DEVICE_CODE_TIMEOUT_SECONDS`, 15 minutes).
pub const KIMI_DEVICE_CODE_TIMEOUT_SECONDS: f64 = 15.0 * 60.0;
/// Poll interval default when the response omits `interval` (RFC 8628 §3.2).
pub const KIMI_DEFAULT_POLL_INTERVAL_SECONDS: f64 = 5.0;
/// Per-request deadline (pi `REQUEST_TIMEOUT_MS`).
pub const KIMI_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Lower bound for the poll interval (pi `MINIMUM_INTERVAL_MS`).
pub const KIMI_MINIMUM_POLL_INTERVAL: Duration = Duration::from_secs(1);
/// `slow_down` increment when the server sends no new interval (RFC 8628 §3.5).
pub const KIMI_SLOW_DOWN_INCREMENT: Duration = Duration::from_secs(5);
/// Additional refresh attempts after the first (pi `REFRESH_MAX_RETRIES`).
pub const KIMI_REFRESH_MAX_RETRIES: u32 = 3;

const DEVICE_AUTHORIZATION_PATH: &str = "/api/oauth/device_authorization";
const TOKEN_PATH: &str = "/api/oauth/token";
const DEVICE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
const PROVIDER_ID: &str = "kimi-coding";

/// Parsed device authorization response (pi `DeviceAuthorization`).
#[derive(Debug, Clone, PartialEq)]
pub struct KimiDeviceAuthorization {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub interval_seconds: f64,
    pub expires_in_seconds: f64,
}

/// The verification URI is opened in the user's browser; only http(s) URLs
/// are trusted (pi `trustedHttpUrl`).
fn trusted_http_url(value: &str) -> bool {
    match value.split_once("://") {
        Some((scheme, rest)) => (scheme == "http" || scheme == "https") && !rest.is_empty(),
        None => false,
    }
}

fn positive_finite(value: &serde_json::Value) -> Option<f64> {
    value.as_f64().filter(|v| v.is_finite() && *v > 0.0)
}

/// Parse and validate a device authorization response: every identifier must
/// be a non-empty string, both verification URIs must be http(s) URLs, and
/// `interval`/`expires_in` fall back to their RFC/pi defaults when absent or
/// not positive finite numbers.
pub fn parse_kimi_device_authorization(
    body: &serde_json::Value,
) -> Result<KimiDeviceAuthorization, ProviderError> {
    let string_field = |name: &str| -> Option<String> {
        body.get(name)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let device_code = string_field("device_code");
    let user_code = string_field("user_code");
    let verification_uri = string_field("verification_uri");
    let verification_uri_complete = string_field("verification_uri_complete");
    let (device_code, user_code, verification_uri, verification_uri_complete) = match (
        device_code,
        user_code,
        verification_uri,
        verification_uri_complete,
    ) {
        (Some(d), Some(u), Some(v), Some(c)) => (d, u, v, c),
        _ => return Err(invalid_device_response()),
    };
    if !trusted_http_url(&verification_uri) || !trusted_http_url(&verification_uri_complete) {
        return Err(provider_config_error(
            "untrusted verification uri in device authorization response",
        ));
    }
    Ok(KimiDeviceAuthorization {
        device_code,
        user_code,
        verification_uri,
        verification_uri_complete,
        interval_seconds: body
            .get("interval")
            .and_then(positive_finite)
            .unwrap_or(KIMI_DEFAULT_POLL_INTERVAL_SECONDS),
        expires_in_seconds: body
            .get("expires_in")
            .and_then(positive_finite)
            .unwrap_or(KIMI_DEVICE_CODE_TIMEOUT_SECONDS),
    })
}

fn invalid_device_response() -> ProviderError {
    provider_config_error("invalid device authorization response")
}

/// One device-token poll outcome (pi's poll result statuses).
enum KimiPoll {
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

#[derive(Debug, serde::Deserialize)]
struct StrictTokenTriple {
    access_token: String,
    refresh_token: String,
    expires_in: f64,
}

/// Parse the strict token triple: non-empty access and refresh tokens and a
/// positive finite `expires_in` (pi `parseTokenResponse`).
fn parse_strict_triple(body: &serde_json::Value) -> Result<KimiPoll, ProviderError> {
    let triple: StrictTokenTriple =
        serde_json::from_value(body.clone()).map_err(|_| invalid_token_response())?;
    if triple.access_token.is_empty()
        || triple.refresh_token.is_empty()
        || !(triple.expires_in.is_finite() && triple.expires_in > 0.0)
    {
        return Err(invalid_token_response());
    }
    Ok(KimiPoll::Complete {
        access: triple.access_token,
        refresh: triple.refresh_token,
        expires_in: triple.expires_in,
    })
}

fn invalid_token_response() -> ProviderError {
    provider_config_error("device token response missing fields")
}

/// Kimi Coding OAuth provider: RFC 8628 device flow plus refresh-grant
/// rotation with bounded transient retries.
pub struct KimiCodingOAuthProvider {
    oauth_host: String,
    client: reqwest::Client,
}

/// Terminal poll failures, kept distinct so the login loop can notify the
/// matching user-facing reason while the typed error stays closed.
enum PollError {
    /// The token endpoint answered but the body was unusable (strict-triple
    /// or JSON failures).
    InvalidResponse(ProviderError),
    /// The request failed (transport, ≥500, or an unrecognized OAuth error
    /// code classified through the fixed-class table).
    RequestFailed(ProviderError),
    /// `expired_token`: the device grant expired; the user must restart
    /// login (non-retryable revoked grant).
    Expired,
    /// `access_denied`.
    Denied,
}

impl KimiCodingOAuthProvider {
    /// Production endpoints (`https://auth.kimi.com`).
    pub fn production() -> Self {
        Self::new_with_oauth_host(KIMI_DEFAULT_OAUTH_HOST)
    }

    /// Production OAuth host with an injected HTTP client.
    pub fn with_http_client(client: reqwest::Client) -> Self {
        Self {
            oauth_host: KIMI_DEFAULT_OAUTH_HOST.to_owned(),
            client,
        }
    }

    /// Fully injected construction. Trailing slashes on the host are trimmed
    /// (pi `getOauthHost`).
    pub fn new_with_oauth_host(oauth_host: impl Into<String>) -> Self {
        Self {
            oauth_host: oauth_host.into().trim_end_matches('/').to_owned(),
            client: reqwest::Client::new(),
        }
    }

    fn device_authorization_url(&self) -> String {
        format!("{}{DEVICE_AUTHORIZATION_PATH}", self.oauth_host)
    }

    fn token_url(&self) -> String {
        format!("{}{TOKEN_PATH}", self.oauth_host)
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
        let response = tokio::time::timeout(KIMI_REQUEST_TIMEOUT, send)
            .await
            .map_err(|_| ProviderError::Timeout)?
            .map_err(|e| {
                crate::auth::oauth::provider_network_error(format!("request failed: {e}"))
            })?;
        let status = response.status();
        let body = tokio::time::timeout(KIMI_REQUEST_TIMEOUT, response.json::<serde_json::Value>())
            .await
            .map_err(|_| ProviderError::Timeout)?
            .unwrap_or(serde_json::Value::Null);
        Ok((status, body))
    }

    /// Start device authorization (pi `startDeviceAuthorization`): one form
    /// POST with the fixed client id; non-ok or malformed responses fail
    /// typed without echoing the body.
    async fn start_device_authorization(
        &self,
        presenter: &dyn LoginPresenter,
    ) -> Result<KimiDeviceAuthorization, ProviderError> {
        let (status, body) = self
            .post_form_json(
                &self.device_authorization_url(),
                &[("client_id", KIMI_CLIENT_ID)],
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
        parse_kimi_device_authorization(&body)
            .map_err(|e| fail_start_response_invalid(presenter, e))
    }

    /// Poll the token endpoint once and classify (pi `pollForToken`):
    /// ≥500 fails, a success body must be the strict triple, and the RFC
    /// 8628 error codes map to their terminal outcomes. Body detail is
    /// never echoed; statuses and fixed classes are.
    async fn poll_device_token(&self, device_code: &str) -> Result<KimiPoll, PollError> {
        let (status, body) = self
            .post_form_json(
                &self.token_url(),
                &[
                    ("client_id", KIMI_CLIENT_ID),
                    ("device_code", device_code),
                    ("grant_type", DEVICE_GRANT_TYPE),
                ],
            )
            .await
            .map_err(PollError::RequestFailed)?;
        if status.as_u16() >= 500 {
            return Err(PollError::RequestFailed(
                crate::auth::oauth::provider_auth_error(format!(
                    "device token request failed ({status})"
                )),
            ));
        }
        if status.is_success() && body.get("access_token").and_then(|v| v.as_str()).is_some() {
            return parse_strict_triple(&body).map_err(PollError::InvalidResponse);
        }
        match body.get("error").and_then(|v| v.as_str()) {
            Some("authorization_pending") => Ok(KimiPoll::Pending),
            Some("slow_down") => Ok(KimiPoll::SlowDown {
                interval_seconds: body.get("interval").and_then(positive_finite),
            }),
            Some("expired_token") => Err(PollError::Expired),
            Some("access_denied") => Err(PollError::Denied),
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

    /// One refresh attempt (pi `refreshToken` single pass): 401/403 or an
    /// `invalid_grant` error code is a dead credential (non-retryable);
    /// 429/5xx and transport failures are retryable; anything else is
    /// terminal-typed.
    async fn refresh_attempt(
        &self,
        cred: &OAuthCredential,
    ) -> Result<OAuthCredential, RefreshAttemptOutcome> {
        let form = [
            ("client_id", KIMI_CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", cred.refresh.expose_secret()),
        ];
        let (status, body) = match self.post_form_json(&self.token_url(), &form).await {
            Ok(pair) => pair,
            Err(ProviderError::Timeout | ProviderError::RequestFailed(_)) => {
                return Err(RefreshAttemptOutcome::Retryable);
            }
            Err(e) => return Err(RefreshAttemptOutcome::Terminal(e)),
        };
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(RefreshAttemptOutcome::Revoked);
        }
        if !status.is_success() {
            let error_code = body.get("error").and_then(|v| v.as_str()).unwrap_or("");
            if error_code == "invalid_grant" {
                return Err(RefreshAttemptOutcome::Revoked);
            }
            if status.as_u16() == 429 || status.as_u16() >= 500 {
                return Err(RefreshAttemptOutcome::Retryable);
            }
            return Err(RefreshAttemptOutcome::Terminal(token_endpoint_error(
                status,
                &body.to_string(),
            )));
        }
        match parse_strict_triple(&body) {
            Ok(KimiPoll::Complete {
                access,
                refresh,
                expires_in,
            }) => {
                let expires_at = OffsetDateTime::now_utc()
                    .checked_add(time::Duration::seconds_f64(expires_in))
                    .ok_or_else(|| RefreshAttemptOutcome::Terminal(invalid_token_response()));
                let expires_at = match expires_at {
                    Ok(at) => at,
                    Err(e) => return Err(e),
                };
                Ok(OAuthCredential {
                    access: SecretString::new(access.into_boxed_str()),
                    refresh: SecretString::new(refresh.into_boxed_str()),
                    expires_at: Some(expires_at),
                    base_url: cred.base_url.clone(),
                    account_id: cred.account_id.clone(),
                    client_id: cred.client_id.clone(),
                    enterprise_domain: cred.enterprise_domain.clone(),
                    available_model_ids: cred.available_model_ids.clone(),
                })
            }
            Ok(_) => unreachable!("parse_strict_triple only completes"),
            Err(e) => Err(RefreshAttemptOutcome::Terminal(e)),
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

enum RefreshAttemptOutcome {
    /// The stored credential is dead: the caller surfaces a typed revoked
    /// error, never a fallback account (pi 401/403/invalid_grant).
    Revoked,
    /// Transient (429/5xx/transport): retryable under the bounded policy.
    Retryable,
    /// Any other failure: typed, not retried.
    Terminal(ProviderError),
}

impl OAuthProvider for KimiCodingOAuthProvider {
    fn id(&self) -> &str {
        PROVIDER_ID
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
            // URI (pi notifies `verificationUriComplete`), never the
            // device_code.
            tokio::select! {
                biased;
                cancelled = cancellation.as_mut() => {
                    cancelled?;
                    return Err(login_cancelled(presenter, PROVIDER_ID));
                }
                presented = presenter.present_device_code(
                    &device.user_code,
                    &device.verification_uri_complete,
                ) => {
                    presented?;
                }
            }

            // pi `pollOAuthDeviceCodeFlow`: wait one interval before the
            // first poll, then poll under the device deadline.
            let mut interval =
                Duration::from_secs_f64(device.interval_seconds).max(KIMI_MINIMUM_POLL_INTERVAL);
            let deadline = tokio::time::Instant::now()
                + Duration::from_secs_f64(device.expires_in_seconds)
                    .clamp(Duration::ZERO, Duration::from_secs(u64::MAX / 2));
            tokio::select! {
                biased;
                cancelled = cancellation.as_mut() => {
                    cancelled?;
                    return Err(login_cancelled(presenter, PROVIDER_ID));
                }
                _ = tokio::time::sleep_until(deadline.min(
                    tokio::time::Instant::now() + interval,
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
                    outcome = self.poll_device_token(&device.device_code) => outcome,
                };
                match outcome {
                    Ok(KimiPoll::Complete {
                        access,
                        refresh,
                        expires_in,
                    }) => {
                        let expires_at = OffsetDateTime::now_utc()
                            .checked_add(time::Duration::seconds_f64(expires_in))
                            .ok_or_else(invalid_token_response)?;
                        return Ok(OAuthCredential {
                            access: SecretString::new(access.into_boxed_str()),
                            refresh: SecretString::new(refresh.into_boxed_str()),
                            expires_at: Some(expires_at),
                            base_url: None,
                            account_id: None,
                            client_id: None,
                            enterprise_domain: None,
                            available_model_ids: None,
                        });
                    }
                    Ok(KimiPoll::Pending) => {}
                    Ok(KimiPoll::SlowDown { interval_seconds }) => {
                        saw_slow_down = true;
                        interval = match interval_seconds {
                            Some(seconds) => {
                                Duration::from_secs_f64(seconds).max(KIMI_MINIMUM_POLL_INTERVAL)
                            }
                            None => interval + KIMI_SLOW_DOWN_INCREMENT,
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
                let next = tokio::time::Instant::now() + interval;
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
            // pi `refreshToken`: up to three additional attempts with 1s/2s/4s
            // backoff before each retry; transient failures exhaust to a
            // typed error, a dead credential never retries.
            for attempt in 0..=KIMI_REFRESH_MAX_RETRIES {
                if attempt > 0 {
                    tokio::time::sleep(Duration::from_secs(1 << (attempt - 1))).await;
                }
                match self.refresh_attempt(cred).await {
                    Ok(credential) => return Ok(credential),
                    Err(RefreshAttemptOutcome::Revoked) => {
                        return Err(ProviderError::CredentialRevoked {
                            provider_id: PROVIDER_ID.to_owned(),
                        });
                    }
                    Err(RefreshAttemptOutcome::Terminal(error)) => return Err(error),
                    Err(RefreshAttemptOutcome::Retryable) => {
                        if attempt == KIMI_REFRESH_MAX_RETRIES {
                            return Err(crate::auth::oauth::provider_auth_error(
                                "token refresh failed after retries",
                            ));
                        }
                    }
                }
            }
            unreachable!("the loop returns on its final iteration")
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
