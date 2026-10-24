//! Meta (Muse subscription) OAuth device flow, pinned to pi 1.3.0
//! `packages/ai/src/auth/oauth/meta.ts` (sdk-contracts row 145).
//!
//! RFC 8628 device authorization grant against `https://auth.meta.com`
//! (form requests, JSON responses). Meta splits identity from API access:
//! the identity token from the device flow is not accepted for inference,
//! so it is exchanged for a Model API key via the Muse Code key-mint
//! endpoint (`POST https://api.meta.ai/muse-code/key`, empty JSON body,
//! `x-api-version: 1.0.0`). Minted keys live about a day; the identity
//! token is stored as `refresh` and the minted key as `access`, so the
//! standard scheduler re-mints the key when it expires with no bespoke
//! renewal machinery. The identity token itself is not renewable
//! (auth.meta.com answers a refresh_token grant with 404), so a 401/403
//! from mint means the session is dead and the user must sign in again.
//! Every request carries pi's 30-second deadline; the polling loop follows
//! pi's shared `device-code.ts` state machine: wait one interval before
//! the first poll, a 1-second interval floor, `slow_down` honoring a
//! server-provided interval or adding 5 seconds (RFC 8628 §3.5), and a
//! device deadline from `expires_in` (default 15 minutes). Failure bodies
//! are never echoed; statuses and fixed OAuth error classes are. The one
//! corpus exception is the setup `action_url` on a keyless mint response:
//! a trusted http(s) URL reaches the presenter reason (sdk-contracts row
//! 145), never the raw body.

use std::time::Duration;

use crate::auth::oauth::{login_cancelled, provider_config_error, token_endpoint_error};
use crate::auth::{BoxAuthFuture, LoginPresenter, OAuthCredential, OAuthProvider};
use crate::provider::ProviderError;
use secrecy::{ExposeSecret, SecretString};
use time::OffsetDateTime;

/// Fixed OAuth client id (pi `CLIENT_ID`, the Muse Code CLI id).
pub const META_CLIENT_ID: &str = "1031625952748946";
/// Production device-flow host (pi `AUTH_HOST`).
pub const META_AUTH_HOST: &str = "https://auth.meta.com";
/// Production key-mint endpoint (pi `API_KEY_MINT_URL`).
pub const META_API_KEY_MINT_URL: &str = "https://api.meta.ai/muse-code/key";
/// Device-flow deadline default when the response omits `expires_in`
/// (pi's shared `device-code.ts` default, 15 minutes).
pub const META_DEVICE_CODE_TIMEOUT_SECONDS: f64 = 15.0 * 60.0;
/// Poll interval default when the response omits `interval` (RFC 8628 §3.2).
pub const META_DEFAULT_POLL_INTERVAL_SECONDS: f64 = 5.0;
/// Per-request deadline (pi `REQUEST_TIMEOUT_MS`).
pub const META_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Lower bound for the poll interval (pi `MINIMUM_INTERVAL_MS`).
pub const META_MINIMUM_POLL_INTERVAL: Duration = Duration::from_secs(1);
/// `slow_down` increment when the server sends no new interval (RFC 8628 §3.5).
pub const META_SLOW_DOWN_INCREMENT: Duration = Duration::from_secs(5);
/// Client-side Model API key lifetime (pi `API_KEY_LIFETIME_MS`, 24 hours).
pub const META_API_KEY_LIFETIME_SECONDS: f64 = 24.0 * 60.0 * 60.0;
/// Mint request API version header (pi `x-api-version`).
pub const META_API_VERSION: &str = "1.0.0";

const DEVICE_AUTHORIZATION_PATH: &str = "/oidc/device/authorization/";
const DEVICE_TOKEN_PATH: &str = "/oidc/device/token/";
const DEVICE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
const PROVIDER_ID: &str = "meta";

/// Parsed device authorization response (pi `DeviceAuthorization`). The
/// `verification_uri` is the presenter-facing URI: the complete variant
/// when trusted, otherwise the plain one.
#[derive(Debug, Clone, PartialEq)]
pub struct MetaDeviceAuthorization {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
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

/// Parse and validate a device authorization response: `device_code` and
/// `user_code` must be non-empty strings, the verification URI is the
/// trusted `verification_uri_complete` falling back to the trusted
/// `verification_uri` (pi coalesces at the trust level), and
/// `interval`/`expires_in` fall back to their defaults when absent or not
/// positive finite numbers.
pub fn parse_meta_device_authorization(
    body: &serde_json::Value,
) -> Result<MetaDeviceAuthorization, ProviderError> {
    let string_field = |name: &str| -> Option<String> {
        body.get(name)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let device_code = string_field("device_code");
    let user_code = string_field("user_code");
    let verification_uri = string_field("verification_uri").filter(|uri| trusted_http_url(uri));
    let verification_uri_complete =
        string_field("verification_uri_complete").filter(|uri| trusted_http_url(uri));
    let (device_code, user_code, verification_uri) = match (
        device_code,
        user_code,
        verification_uri_complete.or(verification_uri),
    ) {
        (Some(d), Some(u), Some(v)) => (d, u, v),
        _ => return Err(invalid_device_response()),
    };
    Ok(MetaDeviceAuthorization {
        device_code,
        user_code,
        verification_uri,
        interval_seconds: body
            .get("interval")
            .and_then(positive_finite)
            .unwrap_or(META_DEFAULT_POLL_INTERVAL_SECONDS),
        expires_in_seconds: body
            .get("expires_in")
            .and_then(positive_finite)
            .unwrap_or(META_DEVICE_CODE_TIMEOUT_SECONDS),
    })
}

fn invalid_device_response() -> ProviderError {
    provider_config_error("invalid device authorization response")
}

/// One device-token poll outcome (pi's poll result statuses). Only the
/// identity `access_token` is read; this wire has no refresh token or
/// expiry fields.
enum MetaPoll {
    Complete { identity_token: String },
    Pending,
    SlowDown { interval_seconds: Option<f64> },
}

/// Terminal poll failures, kept distinct so the login loop can notify the
/// matching user-facing reason while the typed error stays closed.
enum PollError {
    /// The request failed (transport, ≥500, or an unrecognized/absent OAuth
    /// error code classified through the fixed-class table).
    RequestFailed(ProviderError),
    /// `expired_token`: the device grant expired; the user must restart
    /// login (non-retryable revoked grant).
    Expired,
    /// `access_denied`.
    Denied,
}

/// Meta OAuth provider: RFC 8628 identity flow plus Model API key mint and
/// same-identity re-mint refresh.
pub struct MetaOAuthProvider {
    auth_host: String,
    mint_url: String,
    client: reqwest::Client,
}

impl MetaOAuthProvider {
    /// Production endpoints.
    pub fn production() -> Self {
        Self::new_with_endpoints(META_AUTH_HOST, META_API_KEY_MINT_URL)
    }

    /// Production endpoints with an injected HTTP client.
    pub fn with_http_client(client: reqwest::Client) -> Self {
        Self {
            auth_host: META_AUTH_HOST.to_owned(),
            mint_url: META_API_KEY_MINT_URL.to_owned(),
            client,
        }
    }

    /// Fully injected construction (tests wire both hosts to fixtures).
    /// Trailing slashes on the auth host are trimmed.
    pub fn new_with_endpoints(auth_host: impl Into<String>, mint_url: impl Into<String>) -> Self {
        Self {
            auth_host: auth_host.into().trim_end_matches('/').to_owned(),
            mint_url: mint_url.into(),
            client: reqwest::Client::new(),
        }
    }

    fn device_authorization_url(&self) -> String {
        format!("{}{DEVICE_AUTHORIZATION_PATH}", self.auth_host)
    }

    fn device_token_url(&self) -> String {
        format!("{}{DEVICE_TOKEN_PATH}", self.auth_host)
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
        let response = tokio::time::timeout(META_REQUEST_TIMEOUT, send)
            .await
            .map_err(|_| ProviderError::Timeout)?
            .map_err(|e| {
                crate::auth::oauth::provider_network_error(format!("request failed: {e}"))
            })?;
        let status = response.status();
        let body = tokio::time::timeout(META_REQUEST_TIMEOUT, response.json::<serde_json::Value>())
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
    ) -> Result<MetaDeviceAuthorization, ProviderError> {
        let (status, body) = self
            .post_form_json(
                &self.device_authorization_url(),
                &[("client_id", META_CLIENT_ID)],
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
        parse_meta_device_authorization(&body)
            .map_err(|e| fail_start_response_invalid(presenter, e))
    }

    /// Poll the token endpoint once and classify (pi `pollForIdentityToken`):
    /// a success body with a non-empty `access_token` (the identity token)
    /// completes; otherwise the RFC 8628 error code in the body — read
    /// regardless of the status, exactly as pi switches after the token
    /// check — maps to its outcome, and an unknown or absent code fails
    /// typed by status. Body detail is never echoed.
    async fn poll_device_token(&self, device_code: &str) -> Result<MetaPoll, PollError> {
        let (status, body) = self
            .post_form_json(
                &self.device_token_url(),
                &[
                    ("client_id", META_CLIENT_ID),
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
        let identity_token = body
            .get("access_token")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());
        if let Some(token) = identity_token.filter(|_| status.is_success()) {
            return Ok(MetaPoll::Complete {
                identity_token: token.to_owned(),
            });
        }
        match body.get("error").and_then(|v| v.as_str()) {
            Some("authorization_pending") => Ok(MetaPoll::Pending),
            Some("slow_down") => Ok(MetaPoll::SlowDown {
                interval_seconds: body.get("interval").and_then(positive_finite),
            }),
            Some("expired_token") => Err(PollError::Expired),
            Some("access_denied") => Err(PollError::Denied),
            _ => Err(PollError::RequestFailed(token_endpoint_error(
                status,
                &body.to_string(),
            ))),
        }
    }

    /// Exchange an identity token for a Model API key (pi `mintApiKey`).
    /// A 401/403 is a dead session (non-renewable identity token): typed
    /// revoked so the host asks for a fresh login, never an account
    /// fallback. A keyless success may carry a trusted http(s) setup
    /// `action_url` that reaches the presenter reason.
    async fn mint_api_key(
        &self,
        presenter: Option<&dyn LoginPresenter>,
        identity_token: &SecretString,
    ) -> Result<OAuthCredential, ProviderError> {
        let send = self
            .client
            .post(&self.mint_url)
            .header("accept", "application/json")
            .header(
                "authorization",
                format!("Bearer {}", identity_token.expose_secret()),
            )
            .header("x-api-version", META_API_VERSION)
            .json(&serde_json::json!({}))
            .send();
        let response = tokio::time::timeout(META_REQUEST_TIMEOUT, send)
            .await
            .map_err(|_| ProviderError::Timeout)?
            .map_err(|e| {
                crate::auth::oauth::provider_network_error(format!("mint request failed: {e}"))
            })?;
        let status = response.status();
        let body = tokio::time::timeout(META_REQUEST_TIMEOUT, response.json::<serde_json::Value>())
            .await
            .map_err(|_| ProviderError::Timeout)?
            .unwrap_or(serde_json::Value::Null);
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            if let Some(presenter) = presenter {
                presenter.notify_failure("meta session expired; sign in again");
            }
            return Err(ProviderError::CredentialRevoked {
                provider_id: PROVIDER_ID.to_owned(),
            });
        }
        if !status.is_success() {
            if let Some(presenter) = presenter {
                presenter.notify_failure("model api key mint failed");
            }
            return Err(crate::auth::oauth::provider_auth_error(format!(
                "model api key mint failed ({status})"
            )));
        }
        let api_key = body
            .get("api_key")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());
        let api_key = match api_key {
            Some(key) => key.to_owned(),
            None => {
                // pi keeps a trusted setup action_url for the user; an
                // untrusted value is dropped, and the raw body never
                // crosses the boundary.
                let mut reason = "model api key response invalid".to_owned();
                if let Some(url) = body
                    .get("action_url")
                    .and_then(|v| v.as_str())
                    .filter(|url| trusted_http_url(url))
                {
                    reason.push_str("; complete setup at ");
                    reason.push_str(url);
                }
                if let Some(presenter) = presenter {
                    presenter.notify_failure(&reason);
                }
                return Err(provider_config_error(
                    "model api key mint response missing api_key",
                ));
            }
        };
        let expires_at = OffsetDateTime::now_utc()
            .checked_add(time::Duration::seconds_f64(META_API_KEY_LIFETIME_SECONDS))
            .ok_or_else(|| provider_config_error("model api key lifetime overflow"))?;
        Ok(OAuthCredential {
            access: SecretString::new(api_key.into_boxed_str()),
            // The identity token is the renewable secret: refresh re-mints.
            refresh: SecretString::new(identity_token.expose_secret().to_owned().into_boxed_str()),
            expires_at: Some(expires_at),
            base_url: None,
            account_id: None,
            client_id: None,
            enterprise_domain: None,
            available_model_ids: None,
        })
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

impl OAuthProvider for MetaOAuthProvider {
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

            // Present the public user code with the trusted verification URI
            // (complete preferred), never the device_code.
            tokio::select! {
                biased;
                cancelled = cancellation.as_mut() => {
                    cancelled?;
                    return Err(login_cancelled(presenter, PROVIDER_ID));
                }
                presented = presenter.present_device_code(
                    &device.user_code,
                    &device.verification_uri,
                ) => {
                    presented?;
                }
            }

            // pi `pollOAuthDeviceCodeFlow`: wait one interval before the
            // first poll, then poll under the device deadline.
            let mut interval =
                Duration::from_secs_f64(device.interval_seconds).max(META_MINIMUM_POLL_INTERVAL);
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
                let identity_token = match outcome {
                    Ok(MetaPoll::Complete { identity_token }) => identity_token,
                    Ok(MetaPoll::Pending) => {
                        sleep_poll_gap(presenter, &mut cancellation, interval, deadline).await?;
                        continue;
                    }
                    Ok(MetaPoll::SlowDown { interval_seconds }) => {
                        saw_slow_down = true;
                        interval = match interval_seconds {
                            Some(seconds) => {
                                Duration::from_secs_f64(seconds).max(META_MINIMUM_POLL_INTERVAL)
                            }
                            None => interval + META_SLOW_DOWN_INCREMENT,
                        };
                        sleep_poll_gap(presenter, &mut cancellation, interval, deadline).await?;
                        continue;
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
                    Err(PollError::RequestFailed(error)) => {
                        presenter.notify_failure("device token request failed");
                        return Err(error);
                    }
                };

                // Identity in hand: pi notifies progress before minting.
                presenter.notify_progress("Enabling Meta Model API access...");
                return self
                    .mint_api_key(
                        Some(presenter),
                        &SecretString::new(identity_token.into_boxed_str()),
                    )
                    .await;
            }
        })
    }

    fn refresh<'a>(
        &'a self,
        cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            // pi's Meta refresh is a re-mint with the stored identity token:
            // no refresh_token grant (auth.meta.com has none), no retry
            // loop, and 401/403 stays the relogin classification.
            self.mint_api_key(None, &cred.refresh).await
        })
    }
}

/// Sleep until the next poll (bounded by the device deadline) while staying
/// cancellation-biased.
async fn sleep_poll_gap(
    presenter: &dyn LoginPresenter,
    cancellation: &mut crate::auth::BoxAuthFuture<'_, Result<(), ProviderError>>,
    interval: Duration,
    deadline: tokio::time::Instant,
) -> Result<(), ProviderError> {
    let next = tokio::time::Instant::now() + interval;
    tokio::select! {
        biased;
        cancelled = cancellation.as_mut() => {
            cancelled?;
            return Err(login_cancelled(presenter, PROVIDER_ID));
        }
        _ = tokio::time::sleep_until(next.min(deadline)) => {}
    }
    Ok(())
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
