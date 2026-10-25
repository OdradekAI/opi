//! Radius gateway OAuth flow, pinned to pi 1.3.0
//! `packages/ai/src/auth/oauth/radius.ts` (sdk-contracts row 147).
//!
//! Radius is a `pi-messages` gateway: every OAuth client API lives on the
//! configured gateway (`/v1/oauth` discovery, `/v1/oauth/device`,
//! `/v1/oauth/token`) and only the interactive browser authorization
//! endpoint is discovered. Model-catalog loading is owned by the Radius
//! provider (task 20.51), not this flow. The login method selection mirrors
//! pi's prompt through [`LoginPresenter::select_login_method`] with browser
//! as the recommended default; the device mode never runs discovery first.
//!
//! Browser mode is the shared PKCE skeleton (`run_pkce_login`) with the
//! corpus's fixed loopback callback `http://127.0.0.1:1456/oauth/callback`,
//! a required bind (a busy port fails the login directly — pi starts no
//! paste-back-only fallback), the fixed client `pi-gateway`, scope
//! `gateway offline_access`, and the `handoff=url` authorize parameter; the
//! browser flow waits only for the callback and never starts a manual
//! input prompt. Device mode reuses the
//! shared RFC 8628 polling machine: wait one interval before the first
//! poll, a 1-second floor, `slow_down` honoring a server-provided interval
//! or adding 5 seconds, and a device-deadline typed timeout; unlike xAI the
//! verification URI passes through verbatim (the corpus applies no scheme
//! validation) and denial has the single RFC spelling. Token responses
//! require the full access/refresh/expiry triple on every grant. Device
//! expiry must be positive finite; browser and refresh use the shared
//! integer-expiry validation without an added positivity rule. Refresh is
//! `refresh_oauth_token` in
//! [`RefreshFieldPolicy::Strict`] shape — no RFC 6749 §6 refresh-token
//! preservation (that tolerance is xAI's sdk-contracts row 146 fixture) —
//! and 401/403 is a revoked session for relogin via the shared helper.
//!
//! The stored expiry is the real service lifetime: pi's 60-second write-time
//! skew (`TOKEN_EXPIRY_SKEW_MS`) is not reproduced; the single five-minute
//! near-expiry correction stays in
//! [`OAuthCredential::needs_refresh`](crate::auth::OAuthCredential::needs_refresh),
//! consistent with the OAuth family. Failure bodies are never echoed
//! (pi's discovery and token errors concatenate response text); statuses
//! and fixed OAuth error classes are.

use std::time::Duration;

use crate::auth::oauth::{
    AuthorizeStyle, CallbackBindFallback, PkceLoginConfig, RefreshFieldPolicy,
    TokenRequestEncoding, login_cancelled, provider_auth_error, provider_config_error,
    refresh_oauth_token, run_pkce_login_callback_only, token_endpoint_error,
};
use crate::auth::{
    BoxAuthFuture, LoginPresenter, OAuthCredential, OAuthLoginMethod, OAuthProvider,
};
use crate::provider::ProviderError;
use secrecy::SecretString;

/// Default gateway (pi `DEFAULT_RADIUS_GATEWAY`).
pub const RADIUS_DEFAULT_GATEWAY: &str = "https://radius.pi.dev";
/// Fixed OAuth client id (pi `OAUTH_CLIENT_ID`).
pub const RADIUS_OAUTH_CLIENT_ID: &str = "pi-gateway";
/// Requested OAuth scope (pi `OAUTH_SCOPE`).
pub const RADIUS_OAUTH_SCOPE: &str = "gateway offline_access";
/// Fixed loopback callback port (pi `CALLBACK_PORT`).
pub const RADIUS_CALLBACK_PORT: u16 = 1456;
/// Loopback callback host (pi `CALLBACK_HOST`).
pub const RADIUS_CALLBACK_HOST: &str = "127.0.0.1";
/// Loopback callback path (pi `CALLBACK_PATH`).
pub const RADIUS_CALLBACK_PATH: &str = "/oauth/callback";
/// Per-request deadline for discovery and device requests (pi 30s `fetch`
/// budget under the login `AbortSignal`).
pub const RADIUS_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Poll interval default when the response omits `interval` (RFC 8628 §3.2).
pub const RADIUS_DEFAULT_POLL_INTERVAL_SECONDS: f64 = 5.0;
/// Lower bound for the poll interval (shared machine floor).
pub const RADIUS_MINIMUM_POLL_INTERVAL: Duration = Duration::from_secs(1);
/// `slow_down` increment when the server sends no new interval (RFC 8628 §3.5).
pub const RADIUS_SLOW_DOWN_INCREMENT: Duration = Duration::from_secs(5);
/// Total browser-login budget (family default; pi uses the shared
/// interaction signal).
pub const RADIUS_BROWSER_LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

const DEVICE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
const PROVIDER_ID: &str = "radius";

/// Normalize a gateway URL the way pi `normalizeRadiusGatewayUrl` does:
/// default a missing scheme to `https://` and strip trailing slashes.
fn normalize_gateway(value: &str) -> String {
    let with_scheme = if value.starts_with("http://") || value.starts_with("https://") {
        value.to_owned()
    } else {
        format!("https://{value}")
    };
    with_scheme.trim_end_matches('/').to_owned()
}

/// Parsed device authorization response (pi `DeviceAuthorizationResponse`).
///
/// Stricter than the corpus's truthy field check (deviation pre-annotated):
/// strings must be non-empty and `expires_in` positive finite; the
/// verification URI passes through verbatim — pi applies no scheme
/// validation here, unlike xAI's https-only URIs.
#[derive(Debug, Clone, PartialEq)]
pub struct RadiusDeviceAuthorization {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval_seconds: f64,
    pub expires_in_seconds: f64,
}

fn positive_finite(value: &serde_json::Value) -> Option<f64> {
    value.as_f64().filter(|v| v.is_finite() && *v > 0.0)
}

fn invalid_device_response() -> ProviderError {
    provider_config_error("invalid device authorization response")
}

/// Parse and validate a device authorization response: `device_code`,
/// `user_code`, `verification_uri` (verbatim, no scheme check), and a
/// positive finite `expires_in` are required; `interval` falls back to the
/// 5-second default when absent or not a positive finite number
/// (RFC 8628 §3.2).
pub fn parse_radius_device_authorization(
    body: &serde_json::Value,
) -> Result<RadiusDeviceAuthorization, ProviderError> {
    let string_field = |name: &str| -> Option<String> {
        body.get(name)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let (Some(device_code), Some(user_code), Some(verification_uri), Some(expires_in_seconds)) = (
        string_field("device_code"),
        string_field("user_code"),
        string_field("verification_uri"),
        body.get("expires_in").and_then(positive_finite),
    ) else {
        return Err(invalid_device_response());
    };
    Ok(RadiusDeviceAuthorization {
        device_code,
        user_code,
        verification_uri,
        interval_seconds: body
            .get("interval")
            .and_then(positive_finite)
            .unwrap_or(RADIUS_DEFAULT_POLL_INTERVAL_SECONDS),
        expires_in_seconds,
    })
}

/// One device-token poll outcome (pi's poll result statuses).
enum RadiusPoll {
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

/// Terminal poll failures with their user-facing reason.
enum PollError {
    /// A success body was unusable.
    InvalidResponse(ProviderError),
    /// Transport, ≥500, or an unrecognized OAuth error code.
    RequestFailed(ProviderError),
    /// `expired_token`.
    Expired,
    /// `access_denied` (the single RFC spelling; no xAI variant).
    Denied,
}

/// Parse a success token response (pi `requestOAuthToken`'s read of the
/// full triple): `access_token` and `refresh_token` must be non-empty and
/// `expires_in` positive finite — on every grant, login and refresh alike.
fn parse_token_response(body: &serde_json::Value) -> Result<RadiusPoll, ProviderError> {
    let string_field = |name: &str| -> Option<String> {
        body.get(name)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let (Some(access), Some(refresh), Some(expires_in)) = (
        string_field("access_token"),
        string_field("refresh_token"),
        body.get("expires_in").and_then(positive_finite),
    ) else {
        return Err(provider_config_error(
            "device token response missing fields",
        ));
    };
    Ok(RadiusPoll::Complete {
        access,
        refresh,
        expires_in,
    })
}

/// Radius OAuth provider: gateway-fixed browser PKCE or RFC 8628 device
/// flow plus a strict single-pass refresh.
pub struct RadiusOAuthProvider {
    id: String,
    name: String,
    gateway: String,
    client: reqwest::Client,
}

impl RadiusOAuthProvider {
    /// Production default gateway.
    pub fn production() -> Self {
        Self::new_with_gateway(RADIUS_DEFAULT_GATEWAY)
    }

    /// Production gateway with an injected HTTP client.
    pub fn with_http_client(client: reqwest::Client) -> Self {
        Self {
            id: PROVIDER_ID.to_owned(),
            name: "Radius".to_owned(),
            gateway: RADIUS_DEFAULT_GATEWAY.to_owned(),
            client,
        }
    }

    /// Injected gateway (tests and alternate deployments); the URL is
    /// normalized and then fixed for the provider's lifetime (pi pins
    /// gateway/id/name at construction; requests never rewrite them).
    pub fn new_with_gateway(gateway: impl Into<String>) -> Self {
        Self {
            id: PROVIDER_ID.to_owned(),
            name: "Radius".to_owned(),
            gateway: normalize_gateway(&gateway.into()),
            client: reqwest::Client::new(),
        }
    }

    /// Set the flow's own provider id and display name, retaining its fixed
    /// gateway. Callers may independently choose their Models registration key.
    pub fn with_identity(mut self, id: impl Into<String>, name: impl Into<String>) -> Self {
        self.id = id.into();
        self.name = name.into();
        self
    }

    fn identity_error(&self, error: ProviderError) -> ProviderError {
        match error {
            ProviderError::LoginCancelled { .. } => ProviderError::LoginCancelled {
                provider_id: self.id.clone(),
            },
            ProviderError::CredentialRevoked { .. } => ProviderError::CredentialRevoked {
                provider_id: self.id.clone(),
            },
            other => other,
        }
    }

    fn discovery_url(&self) -> String {
        format!("{}/v1/oauth", self.gateway)
    }

    fn device_url(&self) -> String {
        format!("{}/v1/oauth/device", self.gateway)
    }

    fn token_url(&self) -> String {
        format!("{}/v1/oauth/token", self.gateway)
    }

    /// Fetch the browser authorization endpoint (pi `loadRadiusOAuthDiscovery`):
    /// a GET on the gateway's `/v1/oauth`; non-ok responses and a missing or
    /// non-string `authorizationEndpoint` fail typed (no scheme validation —
    /// pi hands the discovered endpoint to the browser verbatim).
    async fn load_discovery(&self) -> Result<String, ProviderError> {
        let send = self
            .client
            .get(self.discovery_url())
            .header("accept", "application/json")
            .send();
        let response = tokio::time::timeout(RADIUS_REQUEST_TIMEOUT, send)
            .await
            .map_err(|_| ProviderError::Timeout)?
            .map_err(|e| {
                crate::auth::oauth::provider_network_error(format!("discovery request failed: {e}"))
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(provider_auth_error(format!(
                "radius oauth discovery failed ({status})"
            )));
        }
        let body: serde_json::Value =
            tokio::time::timeout(RADIUS_REQUEST_TIMEOUT, response.json::<serde_json::Value>())
                .await
                .map_err(|_| ProviderError::Timeout)?
                .unwrap_or(serde_json::Value::Null);
        body.get("authorizationEndpoint")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| provider_config_error("invalid radius oauth discovery response"))
    }

    /// POST a form body and read the JSON response under the 30-second
    /// per-request deadline; an unparseable body becomes `Value::Null` so
    /// callers classify by status first.
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
        let response = tokio::time::timeout(RADIUS_REQUEST_TIMEOUT, send)
            .await
            .map_err(|_| ProviderError::Timeout)?
            .map_err(|e| {
                crate::auth::oauth::provider_network_error(format!("request failed: {e}"))
            })?;
        let status = response.status();
        let body =
            tokio::time::timeout(RADIUS_REQUEST_TIMEOUT, response.json::<serde_json::Value>())
                .await
                .map_err(|_| ProviderError::Timeout)?
                .unwrap_or(serde_json::Value::Null);
        Ok((status, body))
    }

    /// Start device authorization (pi `requestDeviceAuthorization`): one
    /// form POST with the fixed client id and scope; a non-ok or malformed
    /// response fails typed without echoing the body.
    async fn request_device_authorization(
        &self,
        presenter: &dyn LoginPresenter,
    ) -> Result<RadiusDeviceAuthorization, ProviderError> {
        let (status, body) = self
            .post_form_json(
                &self.device_url(),
                &[
                    ("client_id", RADIUS_OAUTH_CLIENT_ID),
                    ("scope", RADIUS_OAUTH_SCOPE),
                ],
            )
            .await
            .inspect_err(|_| presenter.notify_failure("device authorization failed"))?;
        if !status.is_success() {
            presenter.notify_failure("device authorization failed");
            return Err(provider_auth_error(format!(
                "device authorization failed ({status})"
            )));
        }
        parse_radius_device_authorization(&body)
            .inspect_err(|_| presenter.notify_failure("device authorization response invalid"))
    }

    /// Poll the token endpoint once (pi's poll closure): a success body is
    /// parsed first (the full triple), then non-ok responses classify
    /// through the RFC 8628 error codes.
    async fn poll_device_token(&self, device_code: &str) -> Result<RadiusPoll, PollError> {
        let (status, body) = self
            .post_form_json(
                &self.token_url(),
                &[
                    ("client_id", RADIUS_OAUTH_CLIENT_ID),
                    ("device_code", device_code),
                    ("grant_type", DEVICE_GRANT_TYPE),
                ],
            )
            .await
            .map_err(PollError::RequestFailed)?;
        if status.is_success() {
            return parse_token_response(&body).map_err(PollError::InvalidResponse);
        }
        match body.get("error").and_then(|v| v.as_str()) {
            Some("authorization_pending") => Ok(RadiusPoll::Pending),
            Some("slow_down") => Ok(RadiusPoll::SlowDown {
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

    /// Browser login: discovery then the shared PKCE skeleton with the
    /// corpus's fixed callback and authorize parameters.
    async fn login_browser(
        &self,
        presenter: &dyn LoginPresenter,
    ) -> Result<OAuthCredential, ProviderError> {
        let authorize_url = self
            .load_discovery()
            .await
            .inspect_err(|_| presenter.notify_failure("radius oauth discovery failed"))?;
        let config = PkceLoginConfig {
            provider_id: PROVIDER_ID,
            authorize_url,
            token_url: self.token_url(),
            client_id: RADIUS_OAUTH_CLIENT_ID.to_owned(),
            authorize_params: vec![
                ("scope".to_owned(), RADIUS_OAUTH_SCOPE.to_owned()),
                ("handoff".to_owned(), "url".to_owned()),
            ],
            client: self.client.clone(),
            timeout: RADIUS_BROWSER_LOGIN_TIMEOUT,
            token_encoding: TokenRequestEncoding::Form,
            preferred_callback_port: Some(RADIUS_CALLBACK_PORT),
            callback_path: RADIUS_CALLBACK_PATH.to_owned(),
            redirect_host: RADIUS_CALLBACK_HOST.to_owned(),
            // Required bind: pi fails the login when the fixed port is held;
            // there is no paste-back-only fallback for this flow.
            manual_only_redirect_uri: None,
            bind_fallback: CallbackBindFallback::Required,
            authorize_style: AuthorizeStyle::Standard,
            listening_progress: None,
        };
        run_pkce_login_callback_only(
            config,
            presenter,
            Ok,
            Some(presenter.await_login_cancelled()),
            None,
        )
        .await
    }

    /// Device login: authorization, presenter notification, then the shared
    /// polling machine under the device deadline.
    async fn login_device_code(
        &self,
        presenter: &dyn LoginPresenter,
    ) -> Result<OAuthCredential, ProviderError> {
        let mut cancellation = presenter.await_login_cancelled();
        let device = tokio::select! {
            biased;
            cancelled = cancellation.as_mut() => {
                cancelled?;
                return Err(login_cancelled(presenter, PROVIDER_ID));
            }
            device = self.request_device_authorization(presenter) => device?,
        };

        // Present the public user code with the verification URI (the seam
        // carries no interval/expiry notification — host UX).
        tokio::select! {
            biased;
            cancelled = cancellation.as_mut() => {
                cancelled?;
                return Err(login_cancelled(presenter, PROVIDER_ID));
            }
            presented = presenter.present_device_code(&device.user_code, &device.verification_uri) => {
                presented?;
            }
        }

        let mut interval = super::checked_poll_duration(device.interval_seconds)?
            .max(RADIUS_MINIMUM_POLL_INTERVAL);
        let deadline = super::checked_device_deadline(device.expires_in_seconds)?;
        tokio::select! {
            biased;
            cancelled = cancellation.as_mut() => {
                cancelled?;
                return Err(login_cancelled(presenter, PROVIDER_ID));
            }
            _ = tokio::time::sleep_until(deadline.min(super::next_poll_deadline(deadline, interval))) => {}
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
                Ok(RadiusPoll::Complete {
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
                Ok(RadiusPoll::Pending) => {}
                Ok(RadiusPoll::SlowDown { interval_seconds }) => {
                    saw_slow_down = true;
                    interval = match interval_seconds {
                        Some(seconds) => {
                            super::checked_poll_duration(seconds)?.max(RADIUS_MINIMUM_POLL_INTERVAL)
                        }
                        None => interval
                            .checked_add(RADIUS_SLOW_DOWN_INCREMENT)
                            .ok_or_else(|| {
                                provider_config_error("OAuth polling interval is out of range")
                            })?,
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
    }
}

/// pi's two deadline messages; the typed error stays `Timeout`.
fn device_flow_timeout(presenter: &dyn LoginPresenter, saw_slow_down: bool) -> ProviderError {
    if saw_slow_down {
        presenter.notify_failure("device flow timed out after slow_down responses");
    } else {
        presenter.notify_failure("device flow timed out");
    }
    ProviderError::Timeout
}

impl OAuthProvider for RadiusOAuthProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn to_auth(&self, cred: &OAuthCredential) -> crate::auth::ResolvedAuth {
        // The configured gateway owns routing for fresh and refreshed tokens;
        // credential metadata never supplies a Radius endpoint.
        crate::auth::ResolvedAuth {
            scheme: crate::auth::AuthScheme::Bearer,
            secret: cred.access.clone(),
            base_url: None,
            account_id: cred.account_id.clone(),
            provenance: crate::auth::AuthProvenance {
                source: crate::auth::AuthProvenanceSource::OAuth {
                    kind: self.id().to_owned(),
                },
                fallback: crate::auth::AuthFallback::NotAttempted,
            },
        }
    }

    fn login<'a>(
        &'a self,
        presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            // pi's select prompt, mirrored through the presenter seam with
            // browser as the recommended default.
            let method = presenter
                .select_login_method(
                    self.id(),
                    &[OAuthLoginMethod::Browser, OAuthLoginMethod::DeviceCode],
                    OAuthLoginMethod::Browser,
                )
                .await
                .map_err(|error| self.identity_error(error))?;
            match method {
                OAuthLoginMethod::Browser => self.login_browser(presenter).await,
                OAuthLoginMethod::DeviceCode => self.login_device_code(presenter).await,
                other => Err(provider_config_error(format!(
                    "OAuth provider 'radius' supplied an invalid default login method: {other:?}"
                ))),
            }
            .map_err(|error| self.identity_error(error))
        })
    }

    fn refresh<'a>(
        &'a self,
        cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            // pi `requestOAuthToken` refresh grant: the full triple is
            // required (no §6 preservation), one pass. The gateway stays
            // provider-owned construction state, never credential state.
            refresh_oauth_token(
                &self.client,
                &self.token_url(),
                RADIUS_OAUTH_CLIENT_ID,
                cred,
                self.id(),
                None,
                TokenRequestEncoding::Form,
                RefreshFieldPolicy::Strict,
            )
            .await
        })
    }
}
