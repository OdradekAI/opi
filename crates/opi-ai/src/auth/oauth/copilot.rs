//! GitHub Copilot OAuth flow, pinned to pi 1.3.0
//! `packages/ai/src/auth/oauth/github-copilot.ts`.
//!
//! The flow is a GitHub device-code authorization (RFC 8628) against
//! `github.com` or an explicit GitHub Enterprise domain, followed by the
//! Copilot token exchange (`api.<domain>/copilot_internal/v2/token`) that
//! mints a short-lived Copilot token, a models-metadata discovery that
//! yields the account's `availableModelIds`, and — during login only —
//! best-effort policy activation for models the account has not configured
//! yet. Refresh re-exchanges the stored GitHub token and re-discovers the
//! models metadata; it never activates policies.
//!
//! The credential stores the GitHub access token in the refresh slot and the
//! Copilot token in the access slot, exactly like pi: the GitHub token is the
//! durable grant, the Copilot token is the per-request bearer. The request
//! base URL is derived per request from the Copilot token's `proxy-ep=` hint
//! (fallback: enterprise domain, then the individual default), never stored.

use std::sync::Arc;
use std::time::Duration;

use secrecy::{ExposeSecret, SecretString};
use time::OffsetDateTime;

use crate::auth::{LoginPresenter, OAuthCredential, OAuthProvider};

use super::{FlowBudget, login_cancelled, provider_auth_error, provider_config_error};
use crate::BoxAuthFuture;
use crate::provider::ProviderError;

/// Public GitHub deployment domain (pi `loginGitHubCopilot` default).
pub const COPILOT_PUBLIC_DOMAIN: &str = "github.com";
/// Default Copilot API base (pi `getGitHubCopilotBaseUrl` fallback).
pub const COPILOT_DEFAULT_BASE_URL: &str = "https://api.individual.githubcopilot.com";
/// Fixed GitHub OAuth app client id (pi `CLIENT_ID`, stored base64-decoded).
pub const COPILOT_CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";
/// Device-code scope (pi `startDeviceFlow`).
pub const COPILOT_DEVICE_SCOPE: &str = "read:user";
/// User-Agent pinned by the Copilot API surface (pi `COPILOT_HEADERS`).
pub const COPILOT_USER_AGENT: &str = "GitHubCopilotChat/0.35.0";
/// Editor version header (pi `COPILOT_HEADERS`).
pub const COPILOT_EDITOR_VERSION: &str = "vscode/1.107.0";
/// Editor plugin version header (pi `COPILOT_HEADERS`).
pub const COPILOT_PLUGIN_VERSION: &str = "copilot-chat/0.35.0";
/// Integration id header (pi `COPILOT_HEADERS`).
pub const COPILOT_INTEGRATION_ID: &str = "vscode-chat";
/// GitHub API version header sent with models/policy requests
/// (pi `COPILOT_API_VERSION`).
pub const COPILOT_API_VERSION: &str = "2026-06-01";

/// Total login budget (pi has none; the product retains the 300s cap from
/// the shared Phase 20 budget decision).
pub const COPILOT_TOTAL_BUDGET: Duration = Duration::from_secs(300);
/// Per-request deadline for models/policy requests
/// (pi `AbortSignal.timeout(5000)`).
const COPILOT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
/// Upper bound on the device-flow lifetime derived from `expires_in`;
/// without a server value the total budget alone bounds the flow.
const MAX_DEVICE_LIFETIME: Duration = Duration::from_secs(900);
/// Lower bound for device polling intervals (pi `MINIMUM_INTERVAL_MS`).
const MIN_DEVICE_INTERVAL: Duration = Duration::from_secs(1);
/// Upper bound for device polling intervals; a hostile server value must
/// not overflow the sleep (RFC 8628 recommends 5-60s).
const MAX_DEVICE_INTERVAL: Duration = Duration::from_secs(60);
/// RFC 8628 §3.5 `slow_down` increment when the server sends no new
/// interval (pi `SLOW_DOWN_INTERVAL_INCREMENT_MS`).
const SLOW_DOWN_INCREMENT: Duration = Duration::from_secs(5);
/// Default poll interval when the device response omits one
/// (pi `DEFAULT_POLL_INTERVAL_SECONDS`, RFC 8628 §3.2).
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Retry policy for the models discovery / policy activation requests (pi
/// `fetchWithRateLimitRetry` call sites).
#[derive(Clone, Copy)]
pub struct CopilotRetryPolicy {
    /// Maximum 429 retries after the first response.
    pub max_retries: u32,
    /// Total elapsed-time bound for the retry loop; zero disables it.
    pub max_elapsed: Duration,
}

impl CopilotRetryPolicy {
    /// Login-time discovery policy (pi: `maxRetries: 2, maxElapsedMs: 5000`).
    pub const LOGIN: Self = Self {
        max_retries: 2,
        max_elapsed: Duration::from_millis(5000),
    };
    /// Refresh-time discovery policy (pi: `maxRetries: 0, maxElapsedMs: 0` —
    /// one attempt, no retry).
    pub const REFRESH: Self = Self {
        max_retries: 0,
        max_elapsed: Duration::ZERO,
    };
}

/// Injectable models/policy base URL derivation for one (copilot token,
/// enterprise domain) pair.
pub type ModelsBaseUrlFn = Arc<dyn Fn(&str, Option<&str>) -> String + Send + Sync>;

/// Injectable enterprise-domain endpoint derivation.
pub type EnterpriseUrlsFn = Arc<dyn Fn(&str) -> CopilotUrls + Send + Sync>;

/// The three flow endpoints pi `getUrls(domain)` derives from one domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CopilotUrls {
    /// Device-authorization endpoint (`https://{domain}/login/device/code`).
    pub device_code_url: String,
    /// Device-token polling endpoint
    /// (`https://{domain}/login/oauth/access_token`).
    pub access_token_url: String,
    /// Copilot token exchange
    /// (`https://api.{domain}/copilot_internal/v2/token`).
    pub copilot_token_url: String,
}

/// Production derivation (pi `getUrls`): all three endpoints share the
/// domain, the Copilot exchange lives under `api.{domain}`.
pub fn copilot_urls_for_domain(domain: &str) -> CopilotUrls {
    CopilotUrls {
        device_code_url: format!("https://{domain}/login/device/code"),
        access_token_url: format!("https://{domain}/login/oauth/access_token"),
        copilot_token_url: format!("https://api.{domain}/copilot_internal/v2/token"),
    }
}

/// Normalize a user-entered GitHub Enterprise URL/domain to a bare hostname
/// (pi `normalizeDomain`): scheme-agnostic, hostname only — userinfo, port,
/// path, query and fragment are dropped. Empty input is `None` (the public
/// deployment); unparseable input is also `None`, which the login flow
/// rejects typed before any HTTP traffic.
pub fn normalize_enterprise_domain(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let rest = if let Some(after) = trimmed.strip_prefix("https://") {
        after
    } else if let Some(after) = trimmed.strip_prefix("http://") {
        after
    } else {
        trimmed
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    // Drop userinfo (`user@host`).
    let host_port = authority.rsplit('@').next().unwrap_or_default();
    // Drop the port; a bracketed IPv6 literal keeps its brackets (pi
    // `url.hostname` keeps them too).
    let host = if host_port.starts_with('[') {
        let end = host_port.find(']')?;
        &host_port[..=end]
    } else {
        host_port.split(':').next().unwrap_or_default()
    };
    if host.is_empty() || host.contains([' ', '\t', '\n', '\r', '\\']) {
        return None;
    }
    Some(host.to_owned())
}

/// Extract the API base URL from a Copilot token's `proxy-ep=` hint (pi
/// `getBaseUrlFromToken`): `proxy.<host>` becomes `https://api.<host>`.
/// Returns `None` when the token carries no usable hint.
pub fn base_url_from_token(token: &str) -> Option<String> {
    let start = token.find("proxy-ep=")? + "proxy-ep=".len();
    let host = token[start..].split(';').next().unwrap_or_default().trim();
    if host.is_empty() {
        return None;
    }
    let api_host = host
        .strip_prefix("proxy.")
        .map(|rest| format!("api.{rest}"))
        .unwrap_or_else(|| host.to_owned());
    Some(format!("https://{api_host}"))
}

/// Per-request base URL (pi `getGitHubCopilotBaseUrl` / `toAuth`): the
/// token's proxy endpoint wins, then the enterprise domain, then the
/// individual default.
pub fn request_base_url(token: Option<&str>, enterprise_domain: Option<&str>) -> String {
    if let Some(url) = token.and_then(base_url_from_token) {
        return url;
    }
    if let Some(domain) = enterprise_domain {
        return format!("https://copilot-api.{domain}");
    }
    COPILOT_DEFAULT_BASE_URL.to_owned()
}

/// Copilot headers shared by the token exchange, models discovery and
/// policy activation requests (pi `COPILOT_HEADERS`).
fn copilot_headers(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder
        .header("accept", "application/json")
        .header("user-agent", COPILOT_USER_AGENT)
        .header("Editor-Version", COPILOT_EDITOR_VERSION)
        .header("Editor-Plugin-Version", COPILOT_PLUGIN_VERSION)
        .header("Copilot-Integration-Id", COPILOT_INTEGRATION_ID)
}

/// Device-poll outcomes (pi `pollForGitHubAccessToken` mapped onto the
/// retained typed terminal contract).
enum DevicePoll {
    Pending,
    SlowDown(Option<f64>),
    Denied,
    Expired,
    Token(String),
}

/// Parsed models catalog (pi `parseGitHubCopilotModelCatalog`).
#[derive(Debug, Default)]
pub struct CopilotModelsCatalog {
    /// Account-usable model ids (picker/policy filtered, or the
    /// enabled-policy fallback on the individual endpoint).
    pub available_model_ids: Vec<String>,
    /// Models that are known to the static catalog but not yet configured
    /// for the account; login offers to enable exactly these.
    pub policy_model_ids: Vec<String>,
}

/// Parse the `/models` response. `allow_policy_fallback` mirrors pi: only
/// the individual endpoint may fall back from all-false picker flags to
/// explicit enabled policies. The static-catalog whitelist gates which
/// unconfigured models login may enable.
pub fn parse_copilot_models_catalog(
    raw: &serde_json::Value,
    allow_policy_fallback: bool,
    static_catalog: &[String],
) -> Result<CopilotModelsCatalog, ProviderError> {
    let data = raw
        .get("data")
        .and_then(|value| value.as_array())
        .ok_or_else(|| provider_config_error("invalid Copilot models response"))?;

    // (id, picker_enabled, policy_state)
    let mut account_models: Vec<(String, bool, Option<&str>)> = Vec::new();
    for item in data {
        let Some(id) = item.get("id").and_then(|value| value.as_str()) else {
            continue;
        };
        // pi filters models that explicitly declare no tool-call support.
        if item
            .pointer("/capabilities/supports/tool_calls")
            .and_then(|value| value.as_bool())
            == Some(false)
        {
            continue;
        }
        let picker_enabled = item
            .get("model_picker_enabled")
            .and_then(|value| value.as_bool())
            == Some(true);
        let policy_state = item
            .pointer("/policy/state")
            .and_then(|value| value.as_str());
        account_models.push((id.to_owned(), picker_enabled, policy_state));
    }

    let picker_model_ids: Vec<String> = account_models
        .iter()
        .filter(|(_, picker, state)| *picker && *state != Some("disabled"))
        .map(|(id, _, _)| id.clone())
        .collect();
    let use_policy_fallback = allow_policy_fallback && picker_model_ids.is_empty();
    let available_model_ids = if !picker_model_ids.is_empty() || !allow_policy_fallback {
        picker_model_ids
    } else {
        account_models
            .iter()
            .filter(|(_, _, state)| *state == Some("enabled"))
            .map(|(id, _, _)| id.clone())
            .collect()
    };
    let policy_model_ids: Vec<String> = account_models
        .iter()
        .filter(|(id, picker, state)| {
            *state == Some("unconfigured")
                && static_catalog.iter().any(|known| known == id)
                && (*picker || use_policy_fallback)
        })
        .map(|(id, _, _)| id.clone())
        .collect();
    Ok(CopilotModelsCatalog {
        available_model_ids,
        policy_model_ids,
    })
}

/// GitHub Copilot OAuth provider (device-code flow + Copilot token exchange
/// + models discovery + login-only policy activation).
///
/// Endpoint derivation is injectable: production derives all URLs from the
/// authenticated domain over `https` exactly like pi `getUrls`; hosts and
/// tests inject their own mapping to offline endpoints. The static-catalog
/// whitelist for policy activation is supplied by the host (opi-ai bundles
/// no Copilot model catalog); the default is empty, which disables
/// activation without changing discovery.
pub struct GitHubCopilotOAuthProvider {
    client: reqwest::Client,
    total_budget: Duration,
    /// Endpoints for the public deployment (or an injected default).
    default_urls: CopilotUrls,
    /// Endpoints for an authenticated enterprise domain.
    urls_for_enterprise: EnterpriseUrlsFn,
    /// Models/policy base URL for one (token, domain) pair.
    models_base_url: ModelsBaseUrlFn,
    policy_whitelist: Arc<[String]>,
}

impl GitHubCopilotOAuthProvider {
    /// Production provider: `github.com` endpoints, per-request
    /// proxy-endpoint derivation, no bundled policy whitelist.
    pub fn production() -> Self {
        Self::with_http_client(super::production_oauth_client())
    }

    /// Production endpoints with an injected HTTP client.
    pub fn with_http_client(client: reqwest::Client) -> Self {
        Self {
            client,
            total_budget: COPILOT_TOTAL_BUDGET,
            default_urls: copilot_urls_for_domain(COPILOT_PUBLIC_DOMAIN),
            urls_for_enterprise: Arc::new(copilot_urls_for_domain),
            models_base_url: Arc::new(|token, domain| request_base_url(Some(token), domain)),
            policy_whitelist: Arc::from(Vec::new()),
        }
    }

    /// Fully injected provider: explicit public-deployment endpoints, an
    /// enterprise-domain endpoint mapping, a models-base derivation and a
    /// policy whitelist (hosts inject their static catalog ids).
    pub fn new_with_endpoints(
        default_urls: CopilotUrls,
        urls_for_enterprise: EnterpriseUrlsFn,
        models_base_url: ModelsBaseUrlFn,
        policy_whitelist: Vec<String>,
        total_budget: Duration,
        client: reqwest::Client,
    ) -> Self {
        Self {
            client,
            total_budget,
            default_urls,
            urls_for_enterprise,
            models_base_url,
            policy_whitelist: Arc::from(policy_whitelist),
        }
    }

    /// Replace the policy whitelist (the host's static catalog ids).
    pub fn with_policy_whitelist(mut self, ids: Vec<String>) -> Self {
        self.policy_whitelist = Arc::from(ids);
        self
    }

    fn urls_for_domain(&self, enterprise_domain: Option<&str>) -> CopilotUrls {
        match enterprise_domain {
            Some(domain) => (self.urls_for_enterprise)(domain),
            None => self.default_urls.clone(),
        }
    }

    /// Ask the user for the GitHub Enterprise URL/domain (pi
    /// `loginGitHubCopilot`'s first interaction). Blank means the public
    /// deployment; a non-empty unparseable answer is a typed failure before
    /// any HTTP traffic.
    async fn resolve_enterprise_domain(
        presenter: &dyn LoginPresenter,
        cancellation: &mut BoxAuthFuture<'_, Result<(), ProviderError>>,
        budget: FlowBudget,
    ) -> Result<Option<String>, ProviderError> {
        let input = tokio::select! {
            biased;
            cancelled = cancellation.as_mut() => {
                cancelled?;
                return Err(login_cancelled(presenter, "github-copilot"));
            }
            input = presenter.prompt_text(
                "GitHub Enterprise URL/domain (blank for github.com)",
                "company.ghe.com",
            ) => input?,
            _ = budget.elapsed() => {
                return Err(ProviderError::Timeout);
            }
        };
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        normalize_enterprise_domain(trimmed)
            .map(Some)
            .ok_or_else(|| provider_config_error("invalid GitHub Enterprise URL/domain"))
    }

    /// Start the device flow (pi `startDeviceFlow`): POST the client id and
    /// scope, strictly validate the response, and require an http(s)
    /// verification URI so a hostile server cannot aim `open` at a local
    /// executable.
    async fn start_device_flow(
        client: &reqwest::Client,
        urls: &CopilotUrls,
        budget: FlowBudget,
    ) -> Result<DeviceCodeBody, ProviderError> {
        let response = budget
            .wait(
                client
                    .post(&urls.device_code_url)
                    .header("accept", "application/json")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .header("user-agent", COPILOT_USER_AGENT)
                    .body(format!(
                        "client_id={}&scope={}",
                        super::url_encode(COPILOT_CLIENT_ID),
                        super::url_encode(COPILOT_DEVICE_SCOPE),
                    ))
                    .send(),
            )
            .await
            .map_err(|_| ProviderError::Timeout)?
            .map_err(|_| super::provider_network_error("device authorization request failed"))?;
        let status = response.status();
        if !status.is_success() {
            return Err(provider_auth_error(format!(
                "device authorization request failed ({status})"
            )));
        }
        let body: serde_json::Value = budget
            .wait(response.json())
            .await
            .map_err(|_| ProviderError::Timeout)?
            .map_err(|_| provider_config_error("invalid device code response"))?;
        let string_field = |name: &str| -> Option<String> {
            body.get(name)
                .and_then(|value| value.as_str())
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        };
        let device_code = string_field("device_code").ok_or(invalid_device_fields())?;
        let user_code = string_field("user_code").ok_or(invalid_device_fields())?;
        let verification_uri = string_field("verification_uri").ok_or(invalid_device_fields())?;
        let interval = body
            .get("interval")
            .and_then(|value| value.as_f64())
            .filter(|value| value.is_finite() && *value >= 0.0);
        let expires_in = body
            .get("expires_in")
            .and_then(|value| value.as_f64())
            .filter(|value| value.is_finite() && *value > 0.0)
            .ok_or(invalid_device_fields())?;
        // The verification URI is opened in the user's browser; it must be a
        // real http(s) URL, never a local executable path.
        if let Some((scheme, rest)) = verification_uri.split_once("://")
            && (scheme == "http" || scheme == "https")
            && !rest.is_empty()
        {
            return Ok(DeviceCodeBody {
                device_code,
                user_code,
                verification_uri,
                interval,
                expires_in,
            });
        }
        Err(provider_config_error(
            "untrusted verification_uri in device code response",
        ))
    }

    /// Poll the device-token endpoint once and classify (pi
    /// `pollForGitHubAccessToken`: pending/slow_down stay non-terminal;
    /// `access_denied`/`expired_token` are the known terminal outcomes under
    /// the retained Rust safety contract; anything else fails typed with the
    /// provider's error code never echoed).
    async fn poll_device_token(
        client: &reqwest::Client,
        urls: &CopilotUrls,
        device_code: &str,
        budget: FlowBudget,
    ) -> Result<DevicePoll, ProviderError> {
        let response = budget
            .wait(
                client
                    .post(&urls.access_token_url)
                    .header("accept", "application/json")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .header("user-agent", COPILOT_USER_AGENT)
                    .body(format!(
                        "client_id={}&device_code={}&grant_type={}",
                        super::url_encode(COPILOT_CLIENT_ID),
                        super::url_encode(device_code),
                        super::url_encode("urn:ietf:params:oauth:grant-type:device_code"),
                    ))
                    .send(),
            )
            .await
            .map_err(|_| ProviderError::Timeout)?
            .map_err(|_| super::provider_network_error("device token poll failed"))?;
        let status = response.status();
        if !status.is_success() {
            return Err(provider_auth_error(format!(
                "device authorization failed ({status})"
            )));
        }
        let body: serde_json::Value = budget
            .wait(response.json())
            .await
            .map_err(|_| ProviderError::Timeout)?
            .map_err(|_| provider_config_error("invalid device token response"))?;
        if let Some(token) = body
            .get("access_token")
            .and_then(|value| value.as_str())
            .filter(|token| !token.is_empty())
        {
            return Ok(DevicePoll::Token(token.to_owned()));
        }
        let error = body
            .get("error")
            .and_then(|value| value.as_str())
            .ok_or_else(|| provider_config_error("invalid device token response"))?;
        match error {
            "authorization_pending" => Ok(DevicePoll::Pending),
            "slow_down" => Ok(DevicePoll::SlowDown(
                body.get("interval")
                    .and_then(|value| value.as_f64())
                    .filter(|value| value.is_finite() && *value > 0.0),
            )),
            "access_denied" => Ok(DevicePoll::Denied),
            "expired_token" => Ok(DevicePoll::Expired),
            _ => Err(provider_auth_error("device authorization failed")),
        }
    }

    /// Exchange the GitHub access token for a short-lived Copilot token (pi
    /// `refreshGitHubCopilotAccessToken`). 401/403 is a revoked grant; the
    /// response body is never echoed. Returns the raw token string so the
    /// caller can derive the request base URL from its proxy endpoint.
    async fn exchange_copilot_token(
        client: &reqwest::Client,
        urls: &CopilotUrls,
        github_token: &str,
        budget: FlowBudget,
    ) -> Result<(String, OffsetDateTime), ProviderError> {
        let response = budget
            .wait(
                copilot_headers(client.get(&urls.copilot_token_url))
                    .bearer_auth(github_token)
                    .send(),
            )
            .await
            .map_err(|_| ProviderError::Timeout)?
            .map_err(|_| super::provider_network_error("copilot token exchange failed"))?;
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(ProviderError::CredentialRevoked {
                provider_id: "github-copilot".to_owned(),
            });
        }
        if !status.is_success() {
            return Err(provider_auth_error(format!(
                "copilot token exchange failed ({status})"
            )));
        }
        let body: serde_json::Value = budget
            .wait(response.json())
            .await
            .map_err(|_| ProviderError::Timeout)?
            .map_err(|_| provider_config_error("invalid Copilot token response"))?;
        let token = body
            .get("token")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| provider_config_error("invalid Copilot token response fields"))?;
        let expires_at = body
            .get("expires_at")
            .and_then(|value| value.as_f64())
            .filter(|value| value.is_finite() && *value > 0.0)
            .ok_or_else(|| provider_config_error("invalid Copilot token response fields"))?;
        let expires_at = OffsetDateTime::from_unix_timestamp(expires_at.floor() as i64)
            .map_err(|_| provider_config_error("invalid Copilot token response fields"))?;
        Ok((token.to_owned(), expires_at))
    }

    /// One rate-limited models/policy request with bounded retries (pi
    /// `fetchWithRateLimitRetry`): 429 responses retry honoring
    /// `Retry-After` (seconds or HTTP-date) with an exponential fallback,
    /// bounded by the retry policy's elapsed budget.
    async fn request_with_retry(
        request: reqwest::RequestBuilder,
        policy: CopilotRetryPolicy,
    ) -> Result<reqwest::Response, ProviderError> {
        let retry_deadline = (policy.max_elapsed > Duration::ZERO)
            .then(|| tokio::time::Instant::now() + policy.max_elapsed);
        for retry in 0..=policy.max_retries {
            let response = request
                .try_clone()
                .expect("copilot retry request is cloneable")
                .timeout(COPILOT_REQUEST_TIMEOUT)
                .send()
                .await
                .map_err(|_| super::provider_network_error("copilot models request failed"))?;
            if response.status() != reqwest::StatusCode::TOO_MANY_REQUESTS
                || retry == policy.max_retries
            {
                return Ok(response);
            }
            let delay = match Self::retry_after_delay(response.headers()) {
                Some(delay) => delay,
                // Unparseable/absent Retry-After: pi returns the 429
                // response to the caller.
                None => return Ok(response),
            };
            if let Some(retry_deadline) = retry_deadline {
                let remaining =
                    retry_deadline.saturating_duration_since(tokio::time::Instant::now());
                if delay >= remaining {
                    return Ok(response);
                }
            }
            tokio::time::sleep(delay).await;
        }
        unreachable!("the retry loop returns within max_retries")
    }

    /// Parse `Retry-After` into a delay: seconds (pi `Number.parseFloat`)
    /// or an HTTP-date (pi `Date.parse`), floored at zero and capped at one
    /// hour. `None` when the header is absent or unparseable.
    fn retry_after_delay(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
        let raw = headers.get("retry-after")?.to_str().ok()?.trim().to_owned();
        if raw.is_empty() {
            return None;
        }
        if let Ok(seconds) = raw.parse::<f64>() {
            if seconds.is_finite() && seconds >= 0.0 {
                return Some(Duration::from_secs_f64(seconds.min(3600.0)));
            }
            return None;
        }
        let parsed =
            OffsetDateTime::parse(&raw, &time::format_description::well_known::Rfc2822).ok()?;
        let delta = (parsed - OffsetDateTime::now_utc())
            .as_seconds_f64()
            .max(0.0);
        Some(Duration::from_secs_f64(delta.min(3600.0)))
    }

    /// Discover the account's models metadata (pi `fetchGitHubCopilotModels`).
    /// The policy fallback is enabled only on the individual endpoint, so it
    /// is derived here from the resolved base URL.
    async fn fetch_models(
        client: &reqwest::Client,
        base_url: &str,
        copilot_token: &str,
        policy: CopilotRetryPolicy,
        static_catalog: &[String],
    ) -> Result<CopilotModelsCatalog, ProviderError> {
        let response = Self::request_with_retry(
            copilot_headers(client.get(format!("{base_url}/models")))
                .header("authorization", format!("Bearer {copilot_token}"))
                .header("X-GitHub-Api-Version", COPILOT_API_VERSION),
            policy,
        )
        .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(provider_auth_error(format!(
                "copilot models request failed ({status})"
            )));
        }
        let raw: serde_json::Value = response
            .json()
            .await
            .map_err(|_| provider_config_error("invalid Copilot models response"))?;
        parse_copilot_models_catalog(&raw, base_url == COPILOT_DEFAULT_BASE_URL, static_catalog)
    }

    /// Enable one unconfigured model (pi `enableGitHubCopilotModel`): a 429
    /// that survives its retries is a typed failure; any other non-ok
    /// answer is a best-effort skip (`Ok(false)`).
    async fn enable_model(
        client: &reqwest::Client,
        base_url: &str,
        copilot_token: &str,
        model_id: &str,
    ) -> Result<bool, ProviderError> {
        let response = Self::request_with_retry(
            copilot_headers(
                client
                    .post(format!("{base_url}/models/{model_id}/policy"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {copilot_token}"))
                    .header("openai-intent", "chat-policy")
                    .header("x-interaction-type", "chat-policy")
                    .body(r#"{"state":"enabled"}"#.to_owned()),
            ),
            CopilotRetryPolicy::LOGIN,
        )
        .await?;
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(provider_auth_error(
                "copilot policy activation rate limited",
            ));
        }
        Ok(response.status().is_success())
    }
}

fn invalid_device_fields() -> ProviderError {
    provider_config_error("invalid device code response fields")
}

/// Device-authorization response fields (pi `DeviceCodeResponse`). The
/// `device_code` is secret (it grants token issuance) and never leaves this
/// module or reaches any presenter method.
struct DeviceCodeBody {
    device_code: String,
    user_code: String,
    verification_uri: String,
    interval: Option<f64>,
    expires_in: f64,
}

impl OAuthProvider for GitHubCopilotOAuthProvider {
    fn to_auth(&self, cred: &OAuthCredential) -> crate::auth::ResolvedAuth {
        crate::auth::ResolvedAuth {
            scheme: crate::auth::AuthScheme::Bearer,
            secret: cred.access.clone(),
            base_url: cred.base_url.clone().or_else(|| {
                Some(request_base_url(
                    Some(cred.access.expose_secret()),
                    cred.enterprise_domain.as_deref(),
                ))
            }),
            account_id: cred.account_id.clone(),
            provenance: crate::auth::AuthProvenance {
                source: crate::auth::AuthProvenanceSource::OAuth {
                    kind: self.id().to_owned(),
                },
                fallback: crate::auth::AuthFallback::NotAttempted,
            },
        }
    }

    fn id(&self) -> &str {
        "github-copilot"
    }

    fn name(&self) -> &str {
        "GitHub Copilot"
    }

    fn is_subscription(&self) -> Option<bool> {
        Some(true)
    }

    fn login<'a>(
        &'a self,
        presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            let budget = FlowBudget::new(self.total_budget)?;
            let mut cancellation = presenter.await_login_cancelled();

            let enterprise_domain =
                Self::resolve_enterprise_domain(presenter, &mut cancellation, budget).await?;
            let urls = self.urls_for_domain(enterprise_domain.as_deref());

            // 1. Device-authorization request. Cancellation is raced through
            //    the whole request including the response body, so a user
            //    cancel preempts a stalled server before any user code is
            //    presented.
            let device = {
                let outcome = tokio::select! {
                    biased;
                    cancelled = cancellation.as_mut() => {
                        cancelled?;
                        return Err(login_cancelled(presenter, "github-copilot"));
                    }
                    outcome = Self::start_device_flow(&self.client, &urls, budget) => outcome,
                };
                match outcome {
                    Ok(device) => device,
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
            let mut interval = device
                .interval
                .map(|seconds| {
                    Duration::from_secs_f64(seconds.clamp(
                        MIN_DEVICE_INTERVAL.as_secs_f64(),
                        MAX_DEVICE_INTERVAL.as_secs_f64(),
                    ))
                })
                .unwrap_or(DEFAULT_POLL_INTERVAL);
            let device_deadline = tokio::time::Instant::now()
                + Duration::from_secs_f64(
                    device
                        .expires_in
                        .clamp(0.0, MAX_DEVICE_LIFETIME.as_secs_f64()),
                );

            // 2. Present the public user_code (never the device_code).
            tokio::select! {
                biased;
                cancelled = cancellation.as_mut() => {
                    cancelled?;
                    return Err(login_cancelled(presenter, "github-copilot"));
                }
                presented = presenter.present_device_code(
                    &device.user_code,
                    &device.verification_uri,
                ) => {
                    presented?;
                }
                _ = budget.elapsed() => {
                    presenter.notify_failure("device authorization timed out");
                    return Err(ProviderError::Timeout);
                }
            }

            // 3. Poll until the GitHub token, a terminal outcome, or a
            //    deadline. Like pi, the first poll waits one interval.
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
            let github_token = loop {
                if tokio::time::Instant::now() >= device_deadline {
                    presenter.notify_failure("device code expired");
                    return Err(ProviderError::CredentialRevoked {
                        provider_id: "github-copilot".to_owned(),
                    });
                }
                let outcome = tokio::select! {
                    biased;
                    cancelled = cancellation.as_mut() => {
                        cancelled?;
                        return Err(login_cancelled(presenter, "github-copilot"));
                    }
                    outcome = Self::poll_device_token(
                        &self.client,
                        &urls,
                        &device.device_code,
                        budget,
                    ) => outcome,
                    _ = budget.elapsed() => {
                        presenter.notify_failure("device authorization timed out");
                        return Err(ProviderError::Timeout);
                    }
                };
                match outcome {
                    Ok(DevicePoll::Token(token)) => break token,
                    Ok(DevicePoll::Pending) => {}
                    Ok(DevicePoll::SlowDown(server_interval)) => {
                        interval = match server_interval {
                            Some(seconds) => Duration::from_secs_f64(seconds.clamp(
                                MIN_DEVICE_INTERVAL.as_secs_f64(),
                                MAX_DEVICE_INTERVAL.as_secs_f64(),
                            )),
                            None => (interval + SLOW_DOWN_INCREMENT).min(MAX_DEVICE_INTERVAL),
                        };
                    }
                    Ok(DevicePoll::Denied) => {
                        presenter.notify_failure("device authorization denied");
                        return Err(ProviderError::CredentialRevoked {
                            provider_id: "github-copilot".to_owned(),
                        });
                    }
                    Ok(DevicePoll::Expired) => {
                        presenter.notify_failure("device code expired");
                        return Err(ProviderError::CredentialRevoked {
                            provider_id: "github-copilot".to_owned(),
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
            };

            // 4. Exchange the GitHub token for a short-lived Copilot token.
            //    Once the GitHub token is acquired, cancellation no longer
            //    preempts the flow: the bounded exchange runs to completion
            //    so the acquired grant is never burned by a late cancel
            //    (retained Rust safety contract).
            let (copilot_token, expires_at) = match Self::exchange_copilot_token(
                &self.client,
                &urls,
                &github_token,
                budget,
            )
            .await
            {
                Ok(pair) => pair,
                Err(ProviderError::Timeout) => {
                    presenter.notify_failure("device authorization timed out");
                    return Err(ProviderError::Timeout);
                }
                Err(error) => {
                    presenter.notify_failure("token exchange failed");
                    return Err(error);
                }
            };

            // 5. Discover the account's models metadata.
            let base_url = (self.models_base_url)(&copilot_token, enterprise_domain.as_deref());
            let catalog = match Self::fetch_models(
                &self.client,
                &base_url,
                &copilot_token,
                CopilotRetryPolicy::LOGIN,
                &self.policy_whitelist,
            )
            .await
            {
                Ok(catalog) => catalog,
                Err(ProviderError::Timeout) => {
                    presenter.notify_failure("device authorization timed out");
                    return Err(ProviderError::Timeout);
                }
                Err(error) => {
                    presenter.notify_failure("models discovery failed");
                    return Err(error);
                }
            };

            // 6. Login-only policy activation for unconfigured models.
            let mut enabled_model_ids: Vec<String> = Vec::new();
            if !catalog.policy_model_ids.is_empty() {
                presenter.notify_progress("Enabling models...");
                for model_id in &catalog.policy_model_ids {
                    match Self::enable_model(&self.client, &base_url, &copilot_token, model_id)
                        .await
                    {
                        Ok(true) => enabled_model_ids.push(model_id.clone()),
                        Ok(false) => {}
                        Err(error) => {
                            presenter.notify_failure("model policy activation failed");
                            return Err(error);
                        }
                    }
                }
            }

            // 7. Credential: the GitHub token is the durable refresh grant,
            //    the Copilot token the per-request bearer. The stored expiry
            //    is the real service expiry; the single five-minute
            //    near-expiry correction lives in `needs_refresh`.
            let mut available_model_ids = catalog.available_model_ids;
            for id in enabled_model_ids {
                if !available_model_ids.contains(&id) {
                    available_model_ids.push(id);
                }
            }
            presenter.notify_success();
            Ok(OAuthCredential {
                access: SecretString::new(copilot_token.into_boxed_str()),
                refresh: SecretString::new(github_token.into_boxed_str()),
                expires_at: Some(expires_at),
                base_url: None,
                account_id: None,
                client_id: None,
                scopes: None,
                enterprise_domain: enterprise_domain.clone(),
                available_model_ids: Some(available_model_ids),
            })
        })
    }

    fn refresh<'a>(
        &'a self,
        cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            // pi `copilotEnterpriseDomain`: the stored domain is
            // re-normalized; an invalid stored value falls back to the
            // public deployment.
            let enterprise_domain = cred
                .enterprise_domain
                .as_deref()
                .and_then(normalize_enterprise_domain);
            let urls = self.urls_for_domain(enterprise_domain.as_deref());
            let github_token = cred.refresh.expose_secret();
            let budget = FlowBudget::new(self.total_budget)?;
            let (copilot_token, expires_at) =
                Self::exchange_copilot_token(&self.client, &urls, github_token, budget).await?;
            // Refresh re-discovers the models metadata with no retries and
            // never activates policies: enabling models is a login-only
            // policy action, not a refresh side effect.
            let base_url = (self.models_base_url)(&copilot_token, enterprise_domain.as_deref());
            let catalog = Self::fetch_models(
                &self.client,
                &base_url,
                &copilot_token,
                CopilotRetryPolicy::REFRESH,
                &self.policy_whitelist,
            )
            .await?;
            Ok(OAuthCredential {
                access: SecretString::new(copilot_token.into_boxed_str()),
                refresh: cred.refresh.clone(),
                expires_at: Some(expires_at),
                base_url: cred.base_url.clone(),
                account_id: cred.account_id.clone(),
                client_id: cred.client_id.clone(),
                scopes: cred.scopes.clone(),
                enterprise_domain: cred.enterprise_domain.clone(),
                available_model_ids: Some(catalog.available_model_ids),
            })
        })
    }
}
