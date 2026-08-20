//! Shared HTTP client with connection pooling and proxy support (tasks 3.13, 3.12).
//!
//! Provides [`HttpClient`] wrapping `reqwest::Client` with tuned pool
//! defaults, proxy configuration, and [`HttpClientBuilder`] for custom
//! configuration. All providers should store `Arc<HttpClient>` to avoid
//! per-request client allocation.

use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::provider::{ProviderError, ProviderErrorSummary};

/// Default maximum idle connections per host in the connection pool.
const DEFAULT_POOL_MAX_IDLE_PER_HOST: usize = 10;

/// Default idle timeout for pooled connections.
const DEFAULT_POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(90);

const ERROR_BODY_CLASSIFICATION_MAX_BYTES: usize = 8 * 1024;
const ERROR_BODY_CLASSIFICATION_DEADLINE: Duration = Duration::from_secs(1);

/// Read the bounded response prefix needed for provider-specific error
/// classification.
///
/// Only Gemini-compatible HTTP 400 bodies use this path. The read retains at
/// most 8 KiB, completes within one second when the request has no timeout,
/// respects a stricter request timeout, and remains cancellation-aware. Errors
/// never include provider-controlled response text. A declared or observed
/// overflow returns `Ok(None)`, so callers skip embedded-code classification.
pub(crate) async fn read_bounded_error_body(
    mut response: reqwest::Response,
    cancel: &CancellationToken,
    request_timeout: Option<Duration>,
) -> Result<Option<Vec<u8>>, ProviderError> {
    let deadline = request_timeout
        .map(|timeout| timeout.min(ERROR_BODY_CLASSIFICATION_DEADLINE))
        .unwrap_or(ERROR_BODY_CLASSIFICATION_DEADLINE);
    let read = async {
        let content_length = response.content_length();
        if content_length.is_some_and(|length| {
            length > u64::try_from(ERROR_BODY_CLASSIFICATION_MAX_BYTES).unwrap_or(u64::MAX)
        }) {
            return Ok(None);
        }
        let mut body = Vec::with_capacity(ERROR_BODY_CLASSIFICATION_MAX_BYTES);
        while body.len() < ERROR_BODY_CLASSIFICATION_MAX_BYTES {
            let chunk = tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
                chunk = response.chunk() => chunk,
            }
            .map_err(|error| {
                if error.is_timeout() {
                    ProviderError::Timeout
                } else {
                    ProviderError::Network(ProviderErrorSummary::attested_static(
                        "provider error response could not be read",
                    ))
                }
            })?;
            let Some(chunk) = chunk else {
                break;
            };
            let remaining = ERROR_BODY_CLASSIFICATION_MAX_BYTES - body.len();
            if chunk.len() > remaining {
                return Ok(None);
            }
            body.extend_from_slice(&chunk);
        }
        if body.len() == ERROR_BODY_CLASSIFICATION_MAX_BYTES
            && content_length != Some(ERROR_BODY_CLASSIFICATION_MAX_BYTES as u64)
        {
            return Ok(None);
        }
        Ok(Some(body))
    };

    match tokio::time::timeout(deadline, read).await {
        Ok(result) => result,
        Err(_) if cancel.is_cancelled() => Err(ProviderError::Cancelled),
        Err(_) => Err(ProviderError::Timeout),
    }
}

/// Proxy configuration for an [`HttpClient`].
///
/// When `url` is `Some`, the client routes requests through the proxy.
/// `no_proxy` is a comma-separated list of host patterns that bypass the
/// proxy (e.g. `"localhost,*.internal"`).
#[derive(Debug, Clone, Default)]
pub struct ProxyConfig {
    /// Proxy URL (e.g. `http://proxy.example.com:8080`).
    pub url: Option<String>,
    /// Comma-separated host patterns to exclude from proxying.
    pub no_proxy: Option<String>,
}

impl ProxyConfig {
    fn normalize(&mut self) {
        if self.url.as_ref().is_some_and(|s| s.trim().is_empty()) {
            self.url = None;
        }
        if self.no_proxy.as_ref().is_some_and(|s| s.trim().is_empty()) {
            self.no_proxy = None;
        }
    }
}

/// Shared HTTP client with tuned connection-pool and proxy settings.
///
/// Wraps a `reqwest::Client` with sensible defaults for LLM provider use:
/// connection pooling enabled, limited idle connections per host, a
/// reasonable idle timeout, and optional proxy configuration. Designed to be
/// held as `Arc<HttpClient>` per provider or shared across providers.
#[derive(Debug)]
pub struct HttpClient {
    inner: reqwest::Client,
    max_idle_per_host: usize,
    idle_timeout: Duration,
    proxy_config: ProxyConfig,
}

impl HttpClient {
    /// Create a new client with default pool settings and no proxy.
    ///
    /// Defaults:
    /// - `pool_max_idle_per_host`: 10
    /// - `pool_idle_timeout`: 90 seconds
    /// - proxy: none
    pub fn new() -> Self {
        HttpClientBuilder::new()
            .build()
            .expect("HttpClient construction should not fail with valid defaults")
    }

    /// Access the underlying `reqwest::Client`.
    pub fn client(&self) -> &reqwest::Client {
        &self.inner
    }

    /// Return the pool configuration as `(max_idle_per_host, idle_timeout)`.
    pub fn pool_config(&self) -> (usize, Duration) {
        (self.max_idle_per_host, self.idle_timeout)
    }

    /// Return the resolved proxy configuration.
    pub fn proxy_config(&self) -> &ProxyConfig {
        &self.proxy_config
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for custom `HttpClient` instances.
pub struct HttpClientBuilder {
    max_idle_per_host: usize,
    idle_timeout: Duration,
    proxy_config: ProxyConfig,
}

impl HttpClientBuilder {
    /// Create a builder with default settings.
    pub fn new() -> Self {
        Self {
            max_idle_per_host: DEFAULT_POOL_MAX_IDLE_PER_HOST,
            idle_timeout: DEFAULT_POOL_IDLE_TIMEOUT,
            proxy_config: ProxyConfig::default(),
        }
    }

    /// Set the maximum number of idle connections per host.
    pub fn max_idle_per_host(mut self, n: usize) -> Self {
        self.max_idle_per_host = n;
        self
    }

    /// Set the idle timeout for pooled connections.
    pub fn idle_timeout(mut self, d: Duration) -> Self {
        self.idle_timeout = d;
        self
    }

    /// Set explicit proxy configuration.
    ///
    /// When set, this takes precedence over environment variable detection.
    /// An empty `url` is normalized to `None` (no proxy).
    pub fn proxy(mut self, config: ProxyConfig) -> Self {
        self.proxy_config = config;
        self.proxy_config.normalize();
        self
    }

    /// Build the `HttpClient`.
    ///
    /// Returns an error if the underlying `reqwest::Client` fails to
    /// construct (e.g. invalid TLS or proxy URL).
    pub fn build(self) -> Result<HttpClient, reqwest::Error> {
        let mut builder = reqwest::Client::builder()
            .pool_max_idle_per_host(self.max_idle_per_host)
            .pool_idle_timeout(Some(self.idle_timeout));

        if let Some(ref url) = self.proxy_config.url {
            let mut proxy = reqwest::Proxy::all(url)?;
            if let Some(ref np) = self.proxy_config.no_proxy {
                proxy = proxy.no_proxy(reqwest::NoProxy::from_string(np));
            }
            builder = builder.proxy(proxy);
        }

        let inner = builder.build()?;
        Ok(HttpClient {
            inner,
            max_idle_per_host: self.max_idle_per_host,
            idle_timeout: self.idle_timeout,
            proxy_config: self.proxy_config,
        })
    }
}

impl Default for HttpClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve proxy configuration from explicit values.
///
/// `https_proxy` takes precedence over `http_proxy` when both are set.
/// Empty strings are treated as `None`. This is the pure-logic core used by
/// [`proxy_from_env`] and config resolution.
pub fn resolve_proxy(
    http_proxy: Option<&str>,
    https_proxy: Option<&str>,
    no_proxy: Option<&str>,
) -> ProxyConfig {
    let url = https_proxy
        .and_then(|s| {
            if s.trim().is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        })
        .or_else(|| {
            http_proxy.and_then(|s| {
                if s.trim().is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            })
        });
    let np = no_proxy.and_then(|s| {
        if s.trim().is_empty() {
            None
        } else {
            Some(s.to_string())
        }
    });
    ProxyConfig { url, no_proxy: np }
}

/// Read an environment variable, preferring uppercase over lowercase.
fn env_var_case_insensitive(upper: &str, lower: &str) -> Option<String> {
    std::env::var(upper)
        .ok()
        .or_else(|| std::env::var(lower).ok())
}

/// Resolve proxy configuration from standard environment variables.
///
/// Checks both uppercase and lowercase variants of `HTTP_PROXY`,
/// `HTTPS_PROXY`, and `NO_PROXY`. Uppercase takes precedence when both
/// cases exist. `HTTPS_PROXY` takes precedence over `HTTP_PROXY`.
pub fn proxy_from_env() -> ProxyConfig {
    let https_proxy = env_var_case_insensitive("HTTPS_PROXY", "https_proxy");
    let http_proxy = env_var_case_insensitive("HTTP_PROXY", "http_proxy");
    let no_proxy = env_var_case_insensitive("NO_PROXY", "no_proxy");
    resolve_proxy(
        http_proxy.as_deref(),
        https_proxy.as_deref(),
        no_proxy.as_deref(),
    )
}

/// Redact credentials embedded in a proxy URL for safe display.
///
/// Converts `http://user:pass@host:port` to `http://***:***@host:port`.
/// URLs without credentials are returned unchanged.
pub fn redact_proxy_credentials(url: &str) -> String {
    if let Some(scheme_end) = url.find("://") {
        let after_scheme = &url[scheme_end + 3..];
        if let Some(at_pos) = after_scheme.find('@') {
            let credentials = &after_scheme[..at_pos];
            let host_part = &after_scheme[at_pos + 1..];
            if credentials.contains(':') {
                return format!("{}***:***@{}", &url[..scheme_end + 3], host_part);
            }
            // User without password
            return format!("{}***@{}", &url[..scheme_end + 3], host_part);
        }
    }
    url.to_string()
}

// ---------------------------------------------------------------------------
// Safe provider error body excerpts
// ---------------------------------------------------------------------------

/// Maximum number of characters retained in a provider error body excerpt.
const SAFE_EXCERPT_MAX_CHARS: usize = 256;

static SECRET_KEY_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
static BEARER_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
static CREDENTIALED_URL_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
static QUERY_SECRET_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();

fn secret_key_re() -> &'static regex::Regex {
    SECRET_KEY_RE.get_or_init(|| {
        regex::Regex::new(
            r"sk-[A-Za-z0-9-]{20,}|gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{82,}|AIza[0-9A-Za-z_-]{35,}|eyJ[A-Za-z0-9_-]{8,}\.eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}",
        )
        .expect("valid secret-key regex")
    })
}

fn bearer_re() -> &'static regex::Regex {
    BEARER_RE.get_or_init(|| {
        regex::Regex::new(r"(?i)bearer\s+[A-Za-z0-9._-]+").expect("valid bearer regex")
    })
}

fn credentialed_url_re() -> &'static regex::Regex {
    CREDENTIALED_URL_RE.get_or_init(|| {
        regex::Regex::new(r"(?P<scheme>[a-zA-Z][a-zA-Z0-9+.-]*://)[^/\s:@]+:[^/\s@]+@")
            .expect("valid credentialed-url regex")
    })
}

fn query_secret_re() -> &'static regex::Regex {
    QUERY_SECRET_RE.get_or_init(|| {
        regex::Regex::new(
            r"(?i)([?&](?:api[_-]?key|key|token|access[_-]?token|refresh[_-]?token|session[_-]?token|access[_-]?key[_-]?id|secret[_-]?access[_-]?key|secret|password|authorization|proxy[_-]?authorization)=)[^&#\s]+",
        )
        .expect("valid query-secret regex")
    })
}

/// Produce a redacted, length-capped excerpt of diagnostic text.
///
/// The constructor-enforced [`crate::provider::ProviderErrorSummary`] is the
/// producer boundary for public provider errors. This helper is a secondary
/// defense for locally-produced context and transport errors: it strips known
/// credential patterns (including credential-bearing URL query keys) and caps
/// the excerpt length.
pub fn safe_excerpt(body: &str) -> String {
    let scrubbed = secret_key_re().replace_all(body, "[REDACTED]").into_owned();
    let scrubbed = bearer_re()
        .replace_all(&scrubbed, "Bearer [REDACTED]")
        .into_owned();
    let scrubbed = credentialed_url_re()
        .replace_all(&scrubbed, "${scheme}[REDACTED]@")
        .into_owned();
    let scrubbed = query_secret_re()
        .replace_all(&scrubbed, "${1}[REDACTED]")
        .into_owned();
    if scrubbed.chars().count() > SAFE_EXCERPT_MAX_CHARS {
        let truncated: String = scrubbed.chars().take(SAFE_EXCERPT_MAX_CHARS).collect();
        format!("{truncated}…")
    } else {
        scrubbed
    }
}
