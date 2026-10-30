//! ADC native credential families for Vertex generation auth (task 20.21.1).
//!
//! The pinned sources are `google-auth-library@10.6.2` (via pi 1.3.0's
//! `@google/genai@2.21.0` lock), `gaxios@7.1.4`, and `gcp-metadata@8.1.2`,
//! restored read-only during corpus research. opi owns the algorithm:
//!
//! * A `service_account` file signs an RS256 JWT (header `{"alg":"RS256"}`
//!   only; claims `iss`/`scope`/`aud`/`iat`/`exp = iat + 3600`) through the
//!   crate's RDF-23 signing owner and exchanges it at the **canonical**
//!   `https://oauth2.googleapis.com/token` endpoint
//!   (`grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer`); the file's
//!   `token_uri` never becomes a credential destination.
//! * An `authorized_user` file refreshes with all four form fields at the
//!   canonical endpoint; `quota_project_id` is kept separately and rides the
//!   generation request as `x-goog-user-project`.
//! * An `external_account_authorized_user` file refreshes with HTTP Basic
//!   client auth at its validated `token_url` or the default
//!   `https://sts.{universe_domain}/v1/oauthtoken` endpoint. A returned
//!   refresh token replaces the owner's copy in place; the ADC file is never
//!   rewritten by default (persistence would go through the store's mutation
//!   ACK contract).
//! * Token exchanges run the fixed Gaxios call configuration (POST admitted,
//!   3 extra retries, 2 no-response retries, statuses 1xx/408/429/5xx,
//!   delays 100/500/1500 ms) through the injectable transport — never the
//!   generation retry owner.
//!
//! File selection: an explicit `GOOGLE_APPLICATION_CREDENTIALS` path (scoped
//! or ambient through [`AuthContext`](crate::auth::AuthContext)) is terminal
//! on read/parse failure — it never falls back to another source. Without an
//! explicit path the portable well-known file (`%APPDATA%\gcloud\...` on
//! Windows, `$HOME/.config/gcloud/...` elsewhere) is consulted; a missing
//! well-known file means "not configured" and performs **zero** network work
//! (no metadata, BIOS, or gcloud discovery). The `external_account`
//! federation families are owned by the sibling `external`, `subject`, and
//! `executable` modules; `impersonated_service_account` composes its source
//! through the sibling `impersonation` module.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use secrecy::{ExposeSecret, SecretString};
use time::OffsetDateTime;

use crate::auth::{AdcCredentialSource, AuthContext};
use crate::credential::BoxAuthFuture;
use crate::http::{HttpClient, HttpClientBuilder, RedirectPolicy};
use crate::provider::ProviderErrorSummary;

use super::cache::{AdcCacheKey, AdcTokenCache};
use super::external::{self as external, ExternalAccountFile, resolve_external_account};
use super::impersonation::{self as impersonation, ImpersonatedFile};
use super::metadata::resolve_direct_metadata_token;

/// Canonical Google OAuth 2.0 token endpoint (hardcoded in the source's
/// gtoken/oauth2 paths; the service-account JWT `aud` is this same value).
pub const CANONICAL_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";

/// Default `external_account_authorized_user` refresh endpoint when the file
/// carries no `token_url`.
pub const DEFAULT_STS_OAUTHTOKEN_ENDPOINT: &str = "https://sts.googleapis.com/v1/oauthtoken";

/// The default Vertex scope `@google/genai` requests for ADC clients.
pub const CLOUD_PLATFORM_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";

/// Default `universe_domain` when the credentials file omits it.
pub const DEFAULT_UNIVERSE_DOMAIN: &str = "googleapis.com";

/// Fixed Gaxios retry budget for token exchanges: 3 extra retries on the
/// admitted method.
const GAXIOS_EXTRA_RETRIES: usize = 3;

/// Fixed Gaxios no-response (network-error) budget for token exchanges.
const GAXIOS_NO_RESPONSE_RETRIES: usize = 2;

/// Fixed first-retry delay (`retryDelay` default 100 ms, multiplier 2).
const GAXIOS_RETRY_DELAY_MS: u64 = 100;

/// JWT lifetime used by the source (`exp = iat + 3600`).
const JWT_LIFETIME_SECS: i64 = 3600;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Typed failure of one ADC native-family resolution. Carries no credential
/// material; [`AdcResolutionError::safe_summary`] is the redaction-safe text.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum AdcResolutionError {
    /// The caller cancelled an acquisition before any rotation started.
    #[error("ADC acquisition cancelled")]
    Cancelled,
    /// A source context could not be consulted.
    #[error("ADC source context unavailable")]
    ContextUnavailable,
    /// The source mutation did not produce a confirmed result.
    #[error("ADC source mutation outcome unknown")]
    MutationUnknown,
    /// The selected source was invalidated while its exchange was running.
    #[error("ADC source changed before the acquisition was acknowledged")]
    Superseded,
    /// The selected access token expired before it could serve this call.
    #[error("ADC access expired before acknowledgement")]
    Expired,
    /// Auto selection found no credentials file; the caller reports the
    /// credential as missing (never falls through to metadata discovery).
    #[error("no ADC credentials file is configured")]
    NotConfigured,
    /// The selected credentials file is not valid JSON. Terminal.
    #[error("ADC credentials file is not valid JSON")]
    MalformedJson {
        /// Path of the rejected file.
        path: String,
    },
    /// The JSON has no known `type` discriminator.
    #[error("ADC credentials file has an unknown type")]
    UnknownType {
        /// Raw (non-secret) `type` value found in the file.
        found: String,
    },
    /// A required field is missing for the selected family.
    #[error("ADC credentials file of type '{family}' is missing the '{field}' field")]
    MissingField {
        /// Family discriminator.
        family: &'static str,
        /// Missing field name.
        field: &'static str,
    },
    /// The file selects a family this owner does not admit yet
    /// (delivered by later tasks in the phase).
    #[error("ADC family '{family}' is not admitted in this owner yet")]
    FamilyNotAdmitted {
        /// Family discriminator.
        family: &'static str,
    },
    /// Reading the selected file failed.
    #[error("failed to read the selected ADC credentials file")]
    ReadFailed {
        /// Path that failed.
        path: String,
        /// Non-secret I/O reason.
        reason: String,
    },
    /// The file's `token_url` is not a usable absolute http(s) URL.
    #[error("ADC credentials file carries an invalid token_url")]
    InvalidTokenUrl,
    /// The private key was rejected by the upstream crypto profile
    /// (structure, encryption, size, or algorithm).
    #[error("ADC service-account private key was rejected by the crypto profile")]
    KeyRejected,
    /// The token exchange failed with a non-retryable or budget-exhausted
    /// HTTP status.
    #[error("ADC token exchange rejected with status {status} after {attempts} send(s)")]
    ExchangeRejected {
        /// Final HTTP status.
        status: u16,
        /// Sends performed (including the first).
        attempts: usize,
    },
    /// The exchange exhausted its no-response budget.
    #[error("ADC token exchange failed without a response after {attempts} send(s)")]
    ExchangeNetwork {
        /// Sends performed (including the first).
        attempts: usize,
    },
    /// The exchange timed out. The measured corpus pins a single send on
    /// timeout (no budget is derived from the no-response retry count).
    #[error("ADC token exchange timed out after {attempts} send(s)")]
    ExchangeTimeout {
        /// Sends performed (including the first).
        attempts: usize,
    },
    /// A token exchange redirect was refused or a source redirect bound failed.
    #[error("ADC token exchange refused a redirect")]
    Redirected,
    /// The metadata response's `Metadata-Flavor` header was absent or wrong.
    #[error("metadata service response has an incorrect Metadata-Flavor header")]
    MetadataFlavor,
    /// A 200 response did not carry the required token fields.
    #[error("token exchange response is missing the access token")]
    InvalidTokenResponse,
    /// The external-account credential source is structurally invalid
    /// (mutually exclusive keys, unsupported formats or versions, missing
    /// sub-keys). Terminal at parse time.
    #[error("ADC external_account credential_source is invalid: {reason}")]
    CredentialSourceInvalid {
        /// Stable non-secret reason.
        reason: &'static str,
    },
    /// `workforce_pool_user_project` was set for a non-workforce audience.
    #[error("workforce_pool_user_project should not be set for non-workforce pool credentials")]
    WorkforceProjectInvalid,
    /// A subject-token supplier failed (file read/parse, URL status or
    /// body, certificate material).
    #[error("the external-account subject token is unavailable: {reason}")]
    SubjectTokenUnavailable {
        /// Stable non-secret reason.
        reason: String,
    },
    /// The certificate source could not be located, read, or parsed.
    #[error("the certificate credential source is unavailable: {reason}")]
    CertificateSourceUnavailable {
        /// Stable non-secret reason.
        reason: &'static str,
    },
    /// An AWS subject leg failed (region or security credentials).
    #[error("the AWS subject credential source is unavailable: {reason}")]
    AwsSubjectUnavailable {
        /// Stable non-secret reason.
        reason: &'static str,
    },
    /// The executable admission gate is not set to `1`.
    #[error(
        "pluggable-auth executables must be explicitly allowed via GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES=1"
    )]
    ExecutablesNotAllowed,
    /// Spawning the executable failed.
    #[error("the executable could not be spawned")]
    ExecutableSpawnFailed,
    /// The selected helper's frozen environment or cwd could not be captured.
    #[error("the executable process context could not be captured")]
    ExecutableProcessContext,
    /// The helper's output pipe could not be collected.
    #[error("the executable output could not be collected")]
    ExecutableOutput,
    /// The caller stopped waiting and the owned helper was cancelled.
    #[error("the executable was cancelled")]
    ExecutableCancelled,
    /// The executable or its response reported a failure
    /// (non-zero exit or `success=false`).
    #[error("the executable failed with code {code}: {message}")]
    ExecutableFailed {
        /// Native exit code, or redacted untrusted helper response code.
        code: ProviderErrorSummary,
        /// Redacted helper output or response message; raw text is not retained.
        message: ProviderErrorSummary,
    },
    /// The executable exceeded its configured timeout; the process group
    /// was reaped.
    #[error("the executable failed to finish within the timeout specified")]
    ExecutableTimeout,
    /// The executable (or cache) response is structurally invalid.
    #[error("the executable response is invalid: {reason}")]
    ExecutableResponseInvalid {
        /// Stable non-secret reason.
        reason: &'static str,
    },
    /// A successful executable response is expired.
    #[error("the executable response is expired")]
    ExecutableResponseExpired,
    /// The impersonated-family configuration is structurally invalid
    /// (delegates, lifetime, or scopes shape).
    #[error("the impersonated_service_account file is invalid: {reason}")]
    ImpersonationConfigInvalid {
        /// Stable non-secret reason.
        reason: &'static str,
    },
    /// The `service_account_impersonation_url` carries no extractable
    /// target principal, or exceeds the 256-character bound.
    #[error("the service_account_impersonation_url carries no valid target principal")]
    ImpersonationTargetInvalid,
    /// An explicit `universe_domain` disagrees with the source credentials'
    /// universe (the source's credential-leak guard).
    #[error("the impersonated universe domain does not match the source credentials")]
    ImpersonationUniverseMismatch,
    /// The IAM `generateAccessToken` response is invalid (missing
    /// `accessToken` or a non-RFC3339 `expireTime`).
    #[error("the impersonation response is invalid: {reason}")]
    ImpersonationResponseInvalid {
        /// Stable non-secret reason.
        reason: &'static str,
    },
}

impl AdcResolutionError {
    /// Redaction-safe display text for provider-facing summaries.
    pub fn safe_summary(&self) -> String {
        self.to_string()
    }
}

// ---------------------------------------------------------------------------
// Injectable transport + clock
// ---------------------------------------------------------------------------

/// One ADC source HTTP request. Source legs are separate from generation
/// attempts; every ADC send runs through [`AdcSourceTransport`].
#[derive(Debug, Clone)]
pub struct AdcSourceRequest {
    /// HTTP method (`GET`/`POST`).
    pub method: String,
    /// Absolute request URL.
    pub url: String,
    /// Request headers (name, value) pairs.
    pub headers: Vec<(String, String)>,
    /// Optional exact request body.
    pub body: Option<String>,
    /// Optional per-request timeout.
    pub timeout: Option<Duration>,
}

/// One ADC source HTTP response.
#[derive(Debug, Clone)]
pub struct AdcSourceResponse {
    /// HTTP status code.
    pub status: u16,
    /// Response headers (name, value) pairs, repeated names included.
    pub headers: Vec<(String, String)>,
    /// Response body bytes.
    pub body: Vec<u8>,
}

/// Transport failure classes for ADC source legs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AdcSourceHttpError {
    /// No response (connection error, DNS, ...).
    #[error("network failure")]
    Network,
    /// The request timed out.
    #[error("timeout")]
    Timeout,
    /// A token redirect was refused or a source GET exceeded its redirect bound.
    #[error("redirect refused")]
    Redirect,
}

/// Injectable transport for ADC auth-source HTTP legs.
pub trait AdcSourceTransport: Send + Sync {
    /// Send one request and await its response. Dropping the returned future
    /// cancels the exchange.
    fn send(
        &self,
        request: AdcSourceRequest,
    ) -> BoxAuthFuture<'_, Result<AdcSourceResponse, AdcSourceHttpError>>;
}

/// Real HTTP transport backed by the controlled opi-ai client. Source GETs
/// follow the pinned twenty-redirect bound; token POST exchanges refuse
/// redirects. Cross-origin GET redirects drop sensitive headers.
pub struct ReqwestAdcTransport {
    client: HttpClient,
    get_client: HttpClient,
}

impl Default for ReqwestAdcTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl ReqwestAdcTransport {
    /// Build the transport with source-qualified defaults.
    pub fn new() -> Self {
        Self {
            client: HttpClientBuilder::new()
                .redirects(RedirectPolicy::None)
                .build()
                .expect("source transport construction should not fail with valid defaults"),
            get_client: HttpClientBuilder::new()
                .redirects(RedirectPolicy::Limited(20))
                .build()
                .expect("source transport construction should not fail with valid defaults"),
        }
    }

    /// Build a source transport with an explicit TLS configuration
    /// (certificate-source mTLS: the identity rides only the STS exchange).
    pub fn with_tls(
        options: crate::tls::TlsOptions,
    ) -> Result<Self, crate::http::HttpClientBuildError> {
        Ok(Self {
            client: HttpClientBuilder::new()
                .redirects(RedirectPolicy::None)
                .tls(options.clone())
                .build()?,
            get_client: HttpClientBuilder::new()
                .redirects(RedirectPolicy::Limited(20))
                .tls(options)
                .build()?,
        })
    }
}

impl AdcSourceTransport for ReqwestAdcTransport {
    fn send(
        &self,
        request: AdcSourceRequest,
    ) -> BoxAuthFuture<'_, Result<AdcSourceResponse, AdcSourceHttpError>> {
        Box::pin(async move {
            let method = reqwest::Method::from_bytes(request.method.as_bytes())
                .expect("ADC source methods are valid HTTP methods");
            let client = if method == reqwest::Method::GET {
                &self.get_client
            } else {
                &self.client
            };
            let mut builder = client.client().request(method, &request.url);
            for (name, value) in &request.headers {
                builder = builder.header(name, value);
            }
            if let Some(timeout) = request.timeout {
                builder = builder.timeout(timeout);
            }
            if let Some(body) = request.body.as_deref() {
                builder = builder.body(body.to_owned());
            }
            let response = builder.send().await.map_err(map_reqwest_error)?;
            let status = response.status().as_u16();
            if response.status().is_redirection() {
                return Err(AdcSourceHttpError::Redirect);
            }
            let headers = response
                .headers()
                .iter()
                .map(|(name, value)| {
                    (
                        name.as_str().to_owned(),
                        String::from_utf8_lossy(value.as_bytes()).into_owned(),
                    )
                })
                .collect();
            let body = response.bytes().await.map_err(map_reqwest_error)?.to_vec();
            Ok(AdcSourceResponse {
                status,
                headers,
                body,
            })
        })
    }
}

fn map_reqwest_error(error: reqwest::Error) -> AdcSourceHttpError {
    if error.is_timeout() {
        AdcSourceHttpError::Timeout
    } else if error.is_redirect() {
        AdcSourceHttpError::Redirect
    } else {
        AdcSourceHttpError::Network
    }
}

/// Injectable clock for expiry windows and JWT `iat`.
pub trait AdcClock: Send + Sync {
    /// Current wall time.
    fn now(&self) -> OffsetDateTime;
}

/// Production clock over [`OffsetDateTime::now_utc`].
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemAdcClock;

impl AdcClock for SystemAdcClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

/// Scriptable clock for deterministic fixtures. `set` moves the clock, so a
/// test can cross the eager-refresh window without sleeping.
#[derive(Debug)]
pub struct FixedAdcClock(Mutex<OffsetDateTime>);

impl FixedAdcClock {
    /// Freeze the clock at `time`.
    pub fn new(time: OffsetDateTime) -> Self {
        Self(Mutex::new(time))
    }

    /// Move the frozen clock.
    pub fn set(&self, time: OffsetDateTime) {
        *self.0.lock().expect("clock lock") = time;
    }
}

impl AdcClock for FixedAdcClock {
    fn now(&self) -> OffsetDateTime {
        *self.0.lock().expect("clock lock")
    }
}

// ---------------------------------------------------------------------------
// Selection + scoped environment
// ---------------------------------------------------------------------------

/// Explicit typed selection for the ADC chain. An explicit choice wins over
/// every probe without reading any further source; `Auto` is the only
/// selection that may consult the ambient context.
#[derive(Debug, Clone)]
pub enum AdcAuthSelection {
    /// `GOOGLE_APPLICATION_CREDENTIALS` (scoped, else ambient) → portable
    /// well-known file → not configured. Missing sources never trigger
    /// metadata or gcloud discovery.
    Auto,
    /// An explicitly selected credentials file. Read/parse failures are
    /// terminal for this resolution.
    CredentialsFile(PathBuf),
    /// The explicitly selected direct metadata source. `residency` freezes
    /// the GCP-residency input for the fixed per-request timeout
    /// (`Some(true)` → none, otherwise 3000 ms).
    DirectMetadata {
        /// Frozen residency input; `None` behaves as non-resident.
        residency: Option<bool>,
    },
}

/// Scoped values shadowing the ambient context for the ADC chain.
#[derive(Debug, Clone, Default)]
pub struct ScopedAdcEnv {
    /// Scoped `GOOGLE_APPLICATION_CREDENTIALS`.
    pub google_application_credentials: Option<String>,
}

impl ScopedAdcEnv {
    /// No scoped values: every key resolves from the ambient context.
    pub fn empty() -> Self {
        Self::default()
    }
}

// ---------------------------------------------------------------------------
// Resolved token
// ---------------------------------------------------------------------------

/// One resolved ADC access token with its non-secret source facts.
///
/// [`std::fmt::Debug`] redacts the token and any rotated refresh token.
#[derive(Clone)]
pub struct AdcResolvedToken {
    /// OAuth access token (Bearer for the Vertex generation route).
    pub access_token: SecretString,
    /// Source-reported expiry (`now + expires_in` seconds when known).
    pub expires_at: Option<OffsetDateTime>,
    /// Non-secret `quota_project_id` from the credentials file.
    pub quota_project: Option<String>,
    /// A replacement refresh token returned by this exchange
    /// (`external_account_authorized_user` only). The owner replaced its copy
    /// in place; the ADC file is never rewritten by default.
    pub rotated_refresh_token: Option<SecretString>,
    /// Typed non-secret source classification for provenance.
    pub source: AdcCredentialSource,
}

impl AdcResolvedToken {
    /// The stable wire principal this token acts as, when the source
    /// proves one (`Some` only for impersonation targets). A `None`
    /// source is **identity-unverified**: an automatic whole-call recovery
    /// must not silently reuse it across a source change — the input
    /// subject path staying the same does not prove the actual principal
    /// did.
    pub fn stable_identity(&self) -> Option<&str> {
        match &self.source {
            AdcCredentialSource::Impersonated {
                target_principal, ..
            } => Some(target_principal),
            _ => None,
        }
    }
}

impl std::fmt::Debug for AdcResolvedToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdcResolvedToken")
            .field("access_token", &"<redacted>")
            .field("expires_at", &self.expires_at)
            .field("quota_project", &self.quota_project)
            .field(
                "rotated_refresh_token",
                &self.rotated_refresh_token.as_ref().map(|_| "<redacted>"),
            )
            .field("source", &self.source)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// File selection and JSON dispatch
// ---------------------------------------------------------------------------

/// One parsed credentials file family.
#[derive(Clone)]
enum AdcFamily {
    /// An `external_account` federation file (task 20.21.2).
    External(ExternalAccountFile),
    /// An `impersonated_service_account` file (task 20.21.3).
    Impersonated(ImpersonatedFile),
    ServiceAccount {
        client_email: String,
        private_key_pem: String,
    },
    AuthorizedUser {
        client_id: String,
        client_secret: SecretString,
        refresh_token: SecretString,
        quota_project: Option<String>,
    },
    ExternalAccountAuthorizedUser {
        client_id: String,
        client_secret: SecretString,
        refresh_token: SecretString,
        token_url: Option<String>,
        universe_domain: Option<String>,
        quota_project: Option<String>,
    },
}

fn required_str(
    json: &serde_json::Value,
    field: &'static str,
    family: &'static str,
) -> Result<String, AdcResolutionError> {
    json.get(field)
        .and_then(|value| value.as_str())
        .map(str::to_owned)
        .filter(|value| !value.is_empty())
        .ok_or(AdcResolutionError::MissingField { family, field })
}

fn optional_str(json: &serde_json::Value, field: &str) -> Option<String> {
    json.get(field)
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn parse_adc_file(contents: &str, path: &str) -> Result<AdcFamily, AdcResolutionError> {
    let json: serde_json::Value =
        serde_json::from_str(contents).map_err(|_| AdcResolutionError::MalformedJson {
            path: path.to_owned(),
        })?;
    parse_adc_json(&json)
}

/// Dispatch one parsed credentials JSON object to its family. Shared by the
/// file reader and the recursive `source_credentials` embedded in
/// `impersonated_service_account` files (task 20.21.3).
fn parse_adc_json(json: &serde_json::Value) -> Result<AdcFamily, AdcResolutionError> {
    match json
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or("")
    {
        "service_account" => Ok(AdcFamily::ServiceAccount {
            client_email: required_str(json, "client_email", "service_account")?,
            private_key_pem: required_str(json, "private_key", "service_account")?,
        }),
        "authorized_user" => Ok(AdcFamily::AuthorizedUser {
            client_id: required_str(json, "client_id", "authorized_user")?,
            client_secret: SecretString::from(required_str(
                json,
                "client_secret",
                "authorized_user",
            )?),
            refresh_token: SecretString::from(required_str(
                json,
                "refresh_token",
                "authorized_user",
            )?),
            quota_project: optional_str(json, "quota_project_id"),
        }),
        "external_account_authorized_user" => Ok(AdcFamily::ExternalAccountAuthorizedUser {
            client_id: required_str(json, "client_id", "external_account_authorized_user")?,
            client_secret: SecretString::from(required_str(
                json,
                "client_secret",
                "external_account_authorized_user",
            )?),
            refresh_token: SecretString::from(required_str(
                json,
                "refresh_token",
                "external_account_authorized_user",
            )?),
            token_url: optional_str(json, "token_url"),
            universe_domain: optional_str(json, "universe_domain"),
            quota_project: optional_str(json, "quota_project_id"),
        }),
        "impersonated_service_account" => Ok(AdcFamily::Impersonated(
            impersonation::parse_impersonated(json)?,
        )),
        "external_account" => Ok(AdcFamily::External(external::parse_external_account(json)?)),
        other => Err(AdcResolutionError::UnknownType {
            found: other.to_owned(),
        }),
    }
}

/// Compute the portable well-known ADC file for one platform layout.
fn well_known_for(appdata: Option<&str>, home: Option<&str>, windows: bool) -> Option<PathBuf> {
    if windows {
        let appdata = appdata?;
        Some(
            Path::new(appdata)
                .join("gcloud")
                .join("application_default_credentials.json"),
        )
    } else {
        let home = home?;
        Some(
            Path::new(home)
                .join(".config")
                .join("gcloud")
                .join("application_default_credentials.json"),
        )
    }
}

/// The portable well-known ADC path for the current platform
/// (`%APPDATA%\gcloud\...` on Windows, `$HOME/.config/gcloud/...`
/// elsewhere). `None` when the platform home variable is absent.
pub fn well_known_adc_file(appdata: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
    well_known_for(appdata, home, cfg!(windows))
}

async fn ambient_env(
    ambient: &dyn AuthContext,
    name: &str,
) -> Result<Option<String>, AdcResolutionError> {
    match ambient.env(name) {
        Ok(value) => Ok(value),
        Err(_) => Ok(None),
    }
}

async fn read_credentials_file(path: &Path) -> Result<AdcFamily, AdcResolutionError> {
    let path_text = path.to_string_lossy().into_owned();
    let contents =
        tokio::fs::read_to_string(path)
            .await
            .map_err(|error| AdcResolutionError::ReadFailed {
                path: path_text.clone(),
                reason: error.to_string(),
            })?;
    parse_adc_file(&contents, &path_text)
}

// ---------------------------------------------------------------------------
// Fixed Gaxios exchange retry
// ---------------------------------------------------------------------------

/// Fixed delay before retry `attempt` (0-based): 100 ms first, then
/// `(2^attempt - 1)/2` seconds (500 ms, 1500 ms, ...). The upstream
/// total/max caps are `MAX_SAFE_INTEGER`, i.e. no cap.
fn gaxios_delay(attempt: usize) -> Duration {
    if attempt == 0 {
        Duration::from_millis(GAXIOS_RETRY_DELAY_MS)
    } else {
        let factor = (1u64 << attempt) - 1;
        Duration::from_millis(factor * 500)
    }
}

/// Whether an HTTP status is in the fixed Gaxios retry ranges
/// (1xx, 408, 429, 5xx).
fn gaxios_retry_status(status: u16) -> bool {
    (100..=199).contains(&status) || status == 408 || status == 429 || (500..=599).contains(&status)
}

pub(crate) struct GaxiosBudget {
    pub(crate) extra_retries: usize,
    pub(crate) no_response: usize,
}

pub(crate) const TOKEN_BUDGET: GaxiosBudget = GaxiosBudget {
    extra_retries: GAXIOS_EXTRA_RETRIES,
    no_response: GAXIOS_NO_RESPONSE_RETRIES,
};

/// Send one ADC source request under the fixed Gaxios retry configuration.
/// Timeouts are terminal single-send failures (the measured corpus pins one
/// send on timeout; no budget is derived from the no-response count).
pub(crate) async fn send_with_gaxios_retry(
    transport: &dyn AdcSourceTransport,
    request: AdcSourceRequest,
    budget: &GaxiosBudget,
) -> Result<(AdcSourceResponse, usize), AdcResolutionError> {
    let mut sends = 0usize;
    let mut retry_attempt = 0usize;
    loop {
        sends += 1;
        match transport.send(request.clone()).await {
            Err(AdcSourceHttpError::Timeout) => {
                return Err(AdcResolutionError::ExchangeTimeout { attempts: sends });
            }
            Err(AdcSourceHttpError::Redirect) => return Err(AdcResolutionError::Redirected),
            Err(AdcSourceHttpError::Network) => {
                if retry_attempt < budget.no_response && retry_attempt < budget.extra_retries {
                    let delay = gaxios_delay(retry_attempt);
                    retry_attempt += 1;
                    tokio::time::sleep(delay).await;
                    continue;
                }
                return Err(AdcResolutionError::ExchangeNetwork { attempts: sends });
            }
            Ok(response) => {
                if gaxios_retry_status(response.status) && retry_attempt < budget.extra_retries {
                    let delay = gaxios_delay(retry_attempt);
                    retry_attempt += 1;
                    tokio::time::sleep(delay).await;
                    continue;
                }
                return Ok((response, sends));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Token parsing
// ---------------------------------------------------------------------------

struct TokenResponse {
    access_token: String,
    expires_in: Option<i64>,
    refresh_token: Option<String>,
}

fn parse_token_response(response: &AdcSourceResponse) -> Result<TokenResponse, AdcResolutionError> {
    let json: serde_json::Value = serde_json::from_slice(&response.body)
        .map_err(|_| AdcResolutionError::InvalidTokenResponse)?;
    let access_token = json
        .get("access_token")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or(AdcResolutionError::InvalidTokenResponse)?
        .to_owned();
    let expires_in = json
        .get("expires_in")
        .and_then(|value| value.as_i64())
        .filter(|secs| *secs > 0);
    let refresh_token = json
        .get("refresh_token")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    Ok(TokenResponse {
        access_token,
        expires_in,
        refresh_token,
    })
}

// ---------------------------------------------------------------------------
// Family exchanges
// ---------------------------------------------------------------------------

fn urlencoded_form(pairs: &[(&str, String)]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in pairs {
        serializer.append_pair(key, value);
    }
    serializer.finish()
}

fn post_form(url: &str, headers: Vec<(String, String)>, body: String) -> AdcSourceRequest {
    let mut headers = headers;
    headers.push((
        "content-type".to_owned(),
        "application/x-www-form-urlencoded".to_owned(),
    ));
    AdcSourceRequest {
        method: "POST".to_owned(),
        url: url.to_owned(),
        headers,
        body: Some(body),
        timeout: None,
    }
}

fn base64_url_no_pad(bytes: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn base64_std(bytes: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Signs the service-account JWT through the crate's RDF-23 crypto owner and
/// returns the compact JWS. The header is alg-only; claims follow the pinned
/// shape (`iss`, `scope`, `aud`, `exp = iat + 3600`, `iat`).
fn sign_service_account_assertion(
    client_email: &str,
    private_key_pem: &str,
    scope: &str,
    audience: &str,
    iat: i64,
) -> Result<String, AdcResolutionError> {
    let header = br#"{"alg":"RS256"}"#;
    let claims = serde_json::json!({
        "iss": client_email,
        "scope": scope,
        "aud": audience,
        "exp": iat + JWT_LIFETIME_SECS,
        "iat": iat,
    });
    let claims_bytes = serde_json::to_vec(&claims).expect("claims serialize");
    let signing_input = format!(
        "{}.{}",
        base64_url_no_pad(header),
        base64_url_no_pad(&claims_bytes)
    );
    let key = crate::crypto::SigningOwner::aws_lc()
        .load_private_key_pem(private_key_pem.as_bytes())
        .map_err(|_| AdcResolutionError::KeyRejected)?;
    let signature = key
        .sign_rs256(signing_input.as_bytes())
        .map_err(|_| AdcResolutionError::KeyRejected)?;
    Ok(format!("{signing_input}.{}", base64_url_no_pad(&signature)))
}

fn exchange_rejected(status: u16, attempts: usize) -> AdcResolutionError {
    AdcResolutionError::ExchangeRejected { status, attempts }
}

#[allow(clippy::too_many_arguments)]
async fn exchange_family(
    family: &AdcFamily,
    refresh_override: Option<&SecretString>,
    transport: &dyn AdcSourceTransport,
    clock: &dyn AdcClock,
    token_endpoint: Option<&str>,
    universe_domain: Option<&str>,
    scopes: &[String],
    input: &AdcNativeInput<'_>,
) -> Result<AdcResolvedToken, AdcResolutionError> {
    let now = clock.now();
    let scope = scopes
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    match family {
        AdcFamily::External(file) => resolve_external_account(file, input, scopes).await,
        AdcFamily::Impersonated(file) => {
            // Boxed: the recursion (impersonated → source → impersonated …)
            // has no static depth bound by contract.
            Box::pin(impersonation::resolve_impersonated(
                file,
                input,
                scopes,
                refresh_override,
            ))
            .await
        }
        AdcFamily::ServiceAccount {
            client_email,
            private_key_pem,
        } => {
            let endpoint = token_endpoint.unwrap_or(CANONICAL_TOKEN_ENDPOINT);
            let iat = now.unix_timestamp();
            let assertion = sign_service_account_assertion(
                client_email,
                private_key_pem,
                &scope,
                endpoint,
                iat,
            )?;
            let body = urlencoded_form(&[
                (
                    "grant_type",
                    "urn:ietf:params:oauth:grant-type:jwt-bearer".to_owned(),
                ),
                ("assertion", assertion),
            ]);
            let (response, sends) = send_with_gaxios_retry(
                transport,
                post_form(endpoint, Vec::new(), body),
                &TOKEN_BUDGET,
            )
            .await?;
            if response.status != 200 {
                return Err(exchange_rejected(response.status, sends));
            }
            let token = parse_token_response(&response)?;
            Ok(AdcResolvedToken {
                access_token: SecretString::from(token.access_token),
                expires_at: token
                    .expires_in
                    .map(|secs| now + time::Duration::seconds(secs)),
                quota_project: None,
                rotated_refresh_token: None,
                source: AdcCredentialSource::ServiceAccount,
            })
        }
        AdcFamily::AuthorizedUser {
            client_id,
            client_secret,
            refresh_token,
            quota_project,
        } => {
            let endpoint = token_endpoint.unwrap_or(CANONICAL_TOKEN_ENDPOINT);
            let body = urlencoded_form(&[
                ("client_id", client_id.clone()),
                ("client_secret", client_secret.expose_secret().to_owned()),
                ("grant_type", "refresh_token".to_owned()),
                (
                    "refresh_token",
                    refresh_override
                        .unwrap_or(refresh_token)
                        .expose_secret()
                        .to_owned(),
                ),
            ]);
            let (response, sends) = send_with_gaxios_retry(
                transport,
                post_form(endpoint, Vec::new(), body),
                &TOKEN_BUDGET,
            )
            .await?;
            if response.status != 200 {
                return Err(exchange_rejected(response.status, sends));
            }
            let token = parse_token_response(&response)?;
            // The canonical refresh keeps the file's refresh token when the
            // response carries none; a returned token is a reported rotation.
            Ok(AdcResolvedToken {
                access_token: SecretString::from(token.access_token),
                expires_at: token
                    .expires_in
                    .map(|secs| now + time::Duration::seconds(secs)),
                quota_project: quota_project.clone(),
                rotated_refresh_token: token.refresh_token.map(SecretString::from),
                source: AdcCredentialSource::AuthorizedUser {
                    quota_project: quota_project.clone(),
                },
            })
        }
        AdcFamily::ExternalAccountAuthorizedUser {
            client_id,
            client_secret,
            refresh_token,
            token_url,
            universe_domain: file_universe,
            quota_project,
        } => {
            let endpoint = match token_url.as_deref() {
                Some(url) => {
                    let parsed = url::Url::parse(url)
                        .ok()
                        .filter(|url| matches!(url.scheme(), "http" | "https"))
                        .ok_or(AdcResolutionError::InvalidTokenUrl)?;
                    parsed.to_string()
                }
                None => format!(
                    "https://sts.{}/v1/oauthtoken",
                    file_universe
                        .as_deref()
                        .or(universe_domain)
                        .unwrap_or(DEFAULT_UNIVERSE_DOMAIN)
                ),
            };
            let effective_refresh = refresh_override.unwrap_or(refresh_token);
            let basic =
                base64_std(format!("{}:{}", client_id, client_secret.expose_secret()).as_bytes());
            let headers = vec![("authorization".to_owned(), format!("Basic {basic}"))];
            let body = urlencoded_form(&[
                ("grant_type", "refresh_token".to_owned()),
                (
                    "refresh_token",
                    effective_refresh.expose_secret().to_owned(),
                ),
            ]);
            let (response, sends) = send_with_gaxios_retry(
                transport,
                post_form(&endpoint, headers, body),
                &TOKEN_BUDGET,
            )
            .await?;
            if response.status != 200 {
                return Err(exchange_rejected(response.status, sends));
            }
            let token = parse_token_response(&response)?;
            let rotated = token.refresh_token.is_some();
            Ok(AdcResolvedToken {
                access_token: SecretString::from(token.access_token),
                expires_at: token
                    .expires_in
                    .map(|secs| now + time::Duration::seconds(secs)),
                quota_project: quota_project.clone(),
                rotated_refresh_token: token.refresh_token.map(SecretString::from),
                source: AdcCredentialSource::ExternalAccountAuthorizedUser {
                    quota_project: quota_project.clone(),
                    rotated,
                },
            })
        }
    }
}

/// Parse one embedded credentials JSON value (an impersonated file's
/// `source_credentials`) and exchange it through the same adapter — the
/// recursive composition entry with no added depth cap.
pub(crate) async fn exchange_family_for_value(
    json: &serde_json::Value,
    refresh_override: Option<&SecretString>,
    input: &AdcNativeInput<'_>,
    scopes: &[String],
) -> Result<AdcResolvedToken, AdcResolutionError> {
    let family = parse_adc_json(json)?;
    exchange_family(
        &family,
        refresh_override,
        input.transport,
        input.clock,
        input.token_endpoint,
        input.universe_domain,
        scopes,
        input,
    )
    .await
}

// ---------------------------------------------------------------------------
// Public resolution entry + owner
// ---------------------------------------------------------------------------

/// Input for one ADC native-chain resolution.
pub struct AdcNativeInput<'a> {
    /// Explicit typed selection (or [`AdcAuthSelection::Auto`]).
    pub selection: AdcAuthSelection,
    /// Scoped values shadowing the ambient context.
    pub scoped: &'a ScopedAdcEnv,
    /// Injectable ambient-source access. Never read for explicit selections.
    pub ambient: &'a dyn AuthContext,
    /// Injectable HTTP transport for token exchanges and metadata legs.
    pub transport: &'a dyn AdcSourceTransport,
    /// Injectable clock for `iat`/`exp` and expiry windows.
    pub clock: &'a dyn AdcClock,
    /// Canonical token-endpoint override (tests and pinned deployments).
    pub token_endpoint: Option<&'a str>,
    /// Universe domain override for the default STS endpoint.
    pub universe_domain: Option<&'a str>,
    /// Requested scopes (space-joined into the JWT `scope` / metadata
    /// `scopes` parameter). Defaults to [`CLOUD_PLATFORM_SCOPE`].
    pub scopes: &'a [String],
    /// Extra trust roots (PEM) for the certificate-source mTLS transport.
    /// `None` uses the pinned default roots; tests and pinned deployments
    /// can admit a loopback or private root for the STS leg only.
    pub certificate_extra_roots_pem: Option<&'a [u8]>,
    /// Canonical override for the IAM `generateAccessToken` leg (tests and
    /// pinned deployments); pins the impersonation exchanges of both the
    /// `impersonated_service_account` family and the external-account
    /// optional impersonation.
    pub impersonation_endpoint: Option<&'a str>,
}

impl<'a> AdcNativeInput<'a> {
    /// Build the input with the platform-default scope list.
    pub fn new(
        selection: AdcAuthSelection,
        scoped: &'a ScopedAdcEnv,
        ambient: &'a dyn AuthContext,
        transport: &'a dyn AdcSourceTransport,
        clock: &'a dyn AdcClock,
    ) -> Self {
        Self {
            selection,
            scoped,
            ambient,
            transport,
            clock,
            token_endpoint: None,
            universe_domain: None,
            scopes: &[],
            certificate_extra_roots_pem: None,
            impersonation_endpoint: None,
        }
    }

    /// Override the canonical token endpoint (tests and pinned deployments).
    pub fn with_token_endpoint(mut self, endpoint: Option<&'a str>) -> Self {
        self.token_endpoint = endpoint;
        self
    }

    /// Override the canonical impersonation endpoint (tests and pinned
    /// deployments).
    pub fn with_impersonation_endpoint(mut self, endpoint: Option<&'a str>) -> Self {
        self.impersonation_endpoint = endpoint;
        self
    }
}

fn default_scopes(scopes: &[String]) -> Vec<String> {
    if scopes.is_empty() {
        vec![CLOUD_PLATFORM_SCOPE.to_owned()]
    } else {
        scopes.to_vec()
    }
}

/// Resolve one ADC native credential.
///
/// `Ok(None)` means only one thing: the `Auto` selection found no configured
/// file (explicit env path absent, well-known file absent). Every configured
/// source that fails does so with a typed terminal error — a missing or
/// malformed selected file never falls through to metadata, another file, or
/// an API key.
pub async fn resolve_adc_native(
    input: &AdcNativeInput<'_>,
) -> Result<Option<AdcResolvedToken>, AdcResolutionError> {
    resolve_adc_native_with_refresh(input, None).await
}

/// [`resolve_adc_native`] with an owner-held replacement refresh token
/// (`external_account_authorized_user` rotation).
pub async fn resolve_adc_native_with_refresh(
    input: &AdcNativeInput<'_>,
    refresh_override: Option<&SecretString>,
) -> Result<Option<AdcResolvedToken>, AdcResolutionError> {
    let scopes = default_scopes(input.scopes);
    match &input.selection {
        AdcAuthSelection::DirectMetadata { residency } => {
            let token = resolve_direct_metadata_token(
                input.ambient,
                input.transport,
                input.clock,
                &scopes,
                *residency,
            )
            .await?;
            Ok(Some(token))
        }
        AdcAuthSelection::CredentialsFile(path) => {
            let family = read_credentials_file(path).await?;
            let token = exchange_family(
                &family,
                refresh_override,
                input.transport,
                input.clock,
                input.token_endpoint,
                input.universe_domain,
                &scopes,
                input,
            )
            .await?;
            Ok(Some(token))
        }
        AdcAuthSelection::Auto => {
            // Explicit environment path (scoped first, then ambient); a
            // present-but-invalid file is terminal.
            let explicit = match input.scoped.google_application_credentials.as_deref() {
                Some(path) => Some(path.to_owned()),
                None => ambient_env(input.ambient, "GOOGLE_APPLICATION_CREDENTIALS").await?,
            };
            let path = match explicit {
                Some(path) => PathBuf::from(path),
                None => {
                    // Portable well-known file; absence = not configured
                    // (zero network work, no discovery).
                    let appdata = ambient_env(input.ambient, "APPDATA").await?;
                    let home = ambient_env(input.ambient, "HOME").await?;
                    let Some(candidate) = well_known_adc_file(appdata.as_deref(), home.as_deref())
                    else {
                        return Ok(None);
                    };
                    match input.ambient.file_exists(&candidate.to_string_lossy()) {
                        Ok(true) => candidate,
                        Ok(false) => return Ok(None),
                        Err(_) => return Ok(None),
                    }
                }
            };
            let family = read_credentials_file(&path).await?;
            let token = exchange_family(
                &family,
                refresh_override,
                input.transport,
                input.clock,
                input.token_endpoint,
                input.universe_domain,
                &scopes,
                input,
            )
            .await?;
            Ok(Some(token))
        }
    }
}

/// Owner of one ADC native selection: the resolved token is cached across
/// logical calls under the pinned key grammar (config digest, scopes,
/// endpoints, universe) with single-flight acquisitions, the fixed
/// 5-minute eager window, short-TTL current-call service, and
/// late-publication fencing through [`AdcTokenCache::invalidate`]. A rotated
/// refresh token replaces the owner's copy in place (the ADC file is never
/// rewritten by default).
#[derive(Clone)]
pub struct AdcNativeOwner {
    selection: AdcAuthSelection,
    scoped: ScopedAdcEnv,
    ambient: Arc<dyn AuthContext>,
    transport: Arc<dyn AdcSourceTransport>,
    clock: Arc<dyn AdcClock>,
    token_endpoint: Option<String>,
    impersonation_endpoint: Option<String>,
    universe_domain: Option<String>,
    scopes: Vec<String>,
    cache: Arc<AdcTokenCache>,
    state: Arc<Mutex<AdcOwnerState>>,
    queue: Arc<crate::mutation::MutationQueue>,
    namespace: uuid::Uuid,
}

#[derive(Default)]
struct AdcOwnerState {
    generation: u64,
    completed: u64,
    refresh: std::collections::HashMap<AdcCacheKey, SecretString>,
    outcomes:
        std::collections::HashMap<AdcCacheKey, (u64, Result<AdcResolvedToken, AdcResolutionError>)>,
    keys: std::collections::HashSet<AdcCacheKey>,
}

struct AdcSnapshot {
    family: Option<AdcFamily>,
    key: AdcCacheKey,
    generation: u64,
    observed: u64,
    entered: u64,
    rotating: bool,
}

struct CancelAcquisition(tokio_util::sync::CancellationToken);
impl Drop for CancelAcquisition {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

fn family_can_rotate(family: &AdcFamily) -> bool {
    match family {
        AdcFamily::AuthorizedUser { .. } | AdcFamily::ExternalAccountAuthorizedUser { .. } => true,
        AdcFamily::Impersonated(file) => {
            parse_adc_json(&file.source_json).is_ok_and(|source| family_can_rotate(&source))
        }
        _ => false,
    }
}

impl AdcNativeOwner {
    /// Construct a lazy source owner. Its mutation receipts acknowledge memory,
    /// not the ADC file; the file is never rewritten implicitly.
    pub fn new(
        selection: AdcAuthSelection,
        transport: Arc<dyn AdcSourceTransport>,
        clock: Arc<dyn AdcClock>,
    ) -> Self {
        Self {
            selection,
            scoped: ScopedAdcEnv::default(),
            ambient: Arc::new(crate::auth::AmbientAuthContext),
            transport,
            clock,
            token_endpoint: None,
            impersonation_endpoint: None,
            universe_domain: None,
            scopes: Vec::new(),
            cache: Arc::new(AdcTokenCache::new()),
            state: Arc::new(Mutex::new(AdcOwnerState::default())),
            queue: Arc::new(crate::mutation::MutationQueue::new()),
            namespace: uuid::Uuid::new_v4(),
        }
    }

    /// Replace ambient source access without consulting the real environment.
    pub fn with_ambient(mut self, ambient: Arc<dyn AuthContext>) -> Self {
        self.ambient = ambient;
        self
    }
    /// Set the source-scoped environment.
    pub fn with_scoped(mut self, scoped: ScopedAdcEnv) -> Self {
        self.scoped = scoped;
        self
    }
    /// Pin the token exchange endpoint.
    pub fn with_token_endpoint(mut self, endpoint: Option<String>) -> Self {
        self.token_endpoint = endpoint;
        self
    }
    /// Pin the IAM exchange endpoint.
    pub fn with_impersonation_endpoint(mut self, endpoint: Option<String>) -> Self {
        self.impersonation_endpoint = endpoint;
        self
    }
    /// Set the selected universe.
    pub fn with_universe_domain(mut self, domain: Option<String>) -> Self {
        self.universe_domain = domain;
        self
    }
    /// Set the requested scopes.
    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }
    /// Share the store's mutation owner so its barrier observes rotations.
    pub fn with_mutation_queue(mut self, queue: Arc<crate::mutation::MutationQueue>) -> Self {
        self.queue = queue;
        self
    }
    /// Observe this source's admitted work without starting authentication.
    pub fn mutation_barrier(&self) -> crate::mutation::MutationBarrier {
        self.queue.barrier()
    }

    /// Check only the selected source's configuration and file presence.
    /// No credential file is read or parsed, no cache is refreshed, and no
    /// metadata or token endpoint is contacted. Explicit direct metadata is
    /// configured by selection, without claiming that the service is healthy.
    pub fn check_configured(&self) -> Result<bool, AdcResolutionError> {
        let env = |name| {
            self.ambient
                .env(name)
                .map_err(|_| AdcResolutionError::ContextUnavailable)
        };
        let path = match &self.selection {
            AdcAuthSelection::DirectMetadata { .. } => return Ok(true),
            AdcAuthSelection::CredentialsFile(path) => path.clone(),
            AdcAuthSelection::Auto => match match &self.scoped.google_application_credentials {
                Some(path) => Some(path.clone()),
                None => env("GOOGLE_APPLICATION_CREDENTIALS")?,
            } {
                Some(path) => PathBuf::from(path),
                None => {
                    let Some(path) =
                        well_known_adc_file(env("APPDATA")?.as_deref(), env("HOME")?.as_deref())
                    else {
                        return Ok(false);
                    };
                    path
                }
            },
        };
        self.ambient
            .file_exists(&path.to_string_lossy())
            .map_err(|_| AdcResolutionError::ContextUnavailable)
    }

    async fn snapshot(&self) -> Result<AdcSnapshot, AdcResolutionError> {
        let (generation, entered) = {
            let state = self.state.lock().expect("owner state lock");
            (state.generation, state.completed)
        };
        let env = |name| {
            self.ambient
                .env(name)
                .map_err(|_| AdcResolutionError::ContextUnavailable)
        };
        let path = match &self.selection {
            AdcAuthSelection::CredentialsFile(path) => Some(path.clone()),
            AdcAuthSelection::DirectMetadata { .. } => None,
            AdcAuthSelection::Auto => match match &self.scoped.google_application_credentials {
                Some(path) => Some(path.clone()),
                None => env("GOOGLE_APPLICATION_CREDENTIALS")?,
            } {
                Some(path) => Some(PathBuf::from(path)),
                None => {
                    let candidate =
                        well_known_adc_file(env("APPDATA")?.as_deref(), env("HOME")?.as_deref())
                            .ok_or(AdcResolutionError::NotConfigured)?;
                    if !self
                        .ambient
                        .file_exists(&candidate.to_string_lossy())
                        .map_err(|_| AdcResolutionError::ContextUnavailable)?
                    {
                        return Err(AdcResolutionError::NotConfigured);
                    }
                    Some(candidate)
                }
            },
        };
        let (config, family) = match path {
            Some(path) => {
                let raw = tokio::fs::read_to_string(&path).await.map_err(|_| {
                    AdcResolutionError::ReadFailed {
                        path: path.to_string_lossy().into_owned(),
                        reason: "selected file unavailable".into(),
                    }
                })?;
                let family = parse_adc_file(&raw, &path.to_string_lossy())?;
                (format!("{}\0{raw}", path.display()), Some(family))
            }
            None => (
                format!(
                    "{:?}:{:?}:{:?}",
                    self.selection,
                    env("GCE_METADATA_IP")?,
                    env("GCE_METADATA_HOST")?
                ),
                None,
            ),
        };
        let key = AdcCacheKey::from_parts(
            &config,
            None,
            &[],
            None,
            &self.scopes,
            None,
            None,
            self.token_endpoint.as_deref(),
            self.universe_domain.as_deref(),
            self.impersonation_endpoint.as_deref(),
        );
        self.state
            .lock()
            .expect("owner state lock")
            .keys
            .insert(key);
        let rotating = family.as_ref().is_some_and(family_can_rotate);
        Ok(AdcSnapshot {
            family,
            key,
            generation,
            observed: self.cache.version(key),
            entered,
            rotating,
        })
    }

    async fn acquire_snapshot(
        &self,
        snapshot: &AdcSnapshot,
        rotated: &std::sync::atomic::AtomicBool,
    ) -> Result<AdcResolvedToken, AdcResolutionError> {
        let recent = {
            let state = self.state.lock().expect("owner state lock");
            if state.generation != snapshot.generation {
                return Err(AdcResolutionError::Superseded);
            }
            state
                .outcomes
                .get(&snapshot.key)
                .filter(|(completed, _)| *completed > snapshot.entered)
                .map(|(_, result)| result.clone())
        };
        if let Some(result) = recent
            && match result.as_ref() {
                Err(_) => true,
                Ok(token) => token
                    .expires_at
                    .is_none_or(|expiry| self.clock.now() < expiry),
            }
        {
            return result;
        }
        let result = self
            .cache
            .resolve_observed(
                snapshot.key,
                self.clock.now(),
                snapshot.observed,
                || async {
                    let refresh = self
                        .state
                        .lock()
                        .expect("owner state lock")
                        .refresh
                        .get(&snapshot.key)
                        .cloned();
                    let input = AdcNativeInput {
                        selection: self.selection.clone(),
                        scoped: &self.scoped,
                        ambient: self.ambient.as_ref(),
                        transport: self.transport.as_ref(),
                        clock: self.clock.as_ref(),
                        token_endpoint: self.token_endpoint.as_deref(),
                        universe_domain: self.universe_domain.as_deref(),
                        scopes: &self.scopes,
                        certificate_extra_roots_pem: None,
                        impersonation_endpoint: self.impersonation_endpoint.as_deref(),
                    };
                    let token = if let Some(family) = &snapshot.family {
                        exchange_family(
                            family,
                            refresh.as_ref(),
                            input.transport,
                            input.clock,
                            input.token_endpoint,
                            input.universe_domain,
                            &default_scopes(&self.scopes),
                            &input,
                        )
                        .await?
                    } else {
                        resolve_adc_native_with_refresh(&input, refresh.as_ref())
                            .await?
                            .ok_or(AdcResolutionError::NotConfigured)?
                    };
                    if let Some(refresh) = &token.rotated_refresh_token {
                        let mut state = self.state.lock().expect("owner state lock");
                        // The key includes the captured file contents/client/source.
                        // Retain unique rotation material for that exact source even
                        // when its access publication was invalidated while in flight.
                        state.refresh.insert(snapshot.key, refresh.clone());
                        rotated.store(true, std::sync::atomic::Ordering::SeqCst);
                        if state.generation != snapshot.generation {
                            return Err(AdcResolutionError::Superseded);
                        }
                    }
                    Ok(token)
                },
            )
            .await;
        let mut state = self.state.lock().expect("owner state lock");
        state.completed += 1;
        if state.generation == snapshot.generation {
            let completed = state.completed;
            state
                .outcomes
                .insert(snapshot.key, (completed, result.clone()));
        }
        result
    }

    /// Resolve a captured source. Ordinary acquisition is cancelled with its
    /// waiter; once a potentially rotating exchange starts it remains owned
    /// until its memory ACK while the host Tokio runtime lives.
    pub async fn resolve_token(&self) -> Result<AdcResolvedToken, AdcResolutionError> {
        let snapshot = self.snapshot().await?;
        let cancelled = tokio_util::sync::CancellationToken::new();
        let _cancel = CancelAcquisition(cancelled.clone());
        let (send, receive) = tokio::sync::oneshot::channel();
        let owner = self.clone();
        let key = format!("adc-memory:{}:{:?}", self.namespace, snapshot.key);
        let handle = self.queue.submit(&key, Box::new(move || Box::pin(async move {
            let rotated=std::sync::atomic::AtomicBool::new(false);
            let result = if cancelled.is_cancelled() { Err(AdcResolutionError::Cancelled) }
            else if snapshot.rotating { owner.acquire_snapshot(&snapshot,&rotated).await }
            else { tokio::select! { biased; _ = cancelled.cancelled() => Err(AdcResolutionError::Cancelled), result = owner.acquire_snapshot(&snapshot,&rotated) => result } };
            let applied = match &result { _ if rotated.load(std::sync::atomic::Ordering::SeqCst) => Ok(crate::mutation::MutationApplied::Write), Ok(_) => Ok(crate::mutation::MutationApplied::Keep),
                Err(_) => Err(crate::mutation::MutationError::Operation(crate::provider::ProviderError::Config(ProviderErrorSummary::attested_static("ADC source acquisition failed")))) };
            let _ = send.send(result);
            applied
        })));
        let receipt = handle
            .await
            .map_err(|_| AdcResolutionError::MutationUnknown)?;
        if matches!(
            receipt.outcome,
            crate::mutation::MutationOutcome::Unknown { .. }
        ) {
            return Err(AdcResolutionError::MutationUnknown);
        }
        let token = receive
            .await
            .map_err(|_| AdcResolutionError::MutationUnknown)??;
        if token
            .expires_at
            .is_some_and(|expiry| self.clock.now() >= expiry)
        {
            return Err(AdcResolutionError::Expired);
        }
        Ok(token)
    }

    /// Invalidate access publications and fence old work. Confirmed same-source
    /// refresh material is retained; another file identity never borrows it.
    pub fn invalidate(&self) {
        let mut state = self.state.lock().expect("owner state lock");
        state.generation += 1;
        state.outcomes.clear();
        for key in &state.keys {
            self.cache.invalidate(*key);
        }
    }
}

impl crate::auth::AuthResolver for AdcNativeOwner {
    fn check_configuration(
        &self,
    ) -> Option<BoxAuthFuture<'_, Result<bool, crate::provider::ProviderError>>> {
        Some(Box::pin(async move {
            self.check_configured().map_err(adc_auth_error)
        }))
    }

    fn resolve(
        &self,
    ) -> BoxAuthFuture<'_, Result<crate::auth::ResolvedAuth, crate::provider::ProviderError>> {
        Box::pin(async move {
            let token = self.resolve_token().await.map_err(adc_auth_error)?;
            Ok(crate::auth::ResolvedAuth {
                env: Default::default(),
                scheme: crate::auth::AuthScheme::Bearer,
                secret: token.access_token,
                base_url: None,
                account_id: None,
                provenance: crate::auth::AuthProvenance {
                    source: crate::auth::AuthProvenanceSource::Adc {
                        source: token.source,
                    },
                    fallback: crate::auth::AuthFallback::NotAttempted,
                },
            })
        })
    }
}

fn adc_auth_error(error: AdcResolutionError) -> crate::provider::ProviderError {
    match error {
        AdcResolutionError::NotConfigured => crate::provider::ProviderError::CredentialNeeded {
            provider_id: "vertex".to_owned(),
        },
        other => crate::provider::ProviderError::Config(ProviderErrorSummary::from_untrusted(
            other.safe_summary(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn well_known_layouts_follow_the_platform_split() {
        let windows = well_known_for(Some("C:/Users/u/AppData/Roaming"), None, true);
        assert_eq!(
            windows.unwrap(),
            PathBuf::from("C:/Users/u/AppData/Roaming/gcloud/application_default_credentials.json")
        );
        assert!(well_known_for(None, Some("/home/u"), true).is_none());

        let unix = well_known_for(None, Some("/home/u"), false);
        assert_eq!(
            unix.unwrap(),
            PathBuf::from("/home/u/.config/gcloud/application_default_credentials.json")
        );
        assert!(well_known_for(Some("appdata"), None, false).is_none());
    }

    #[test]
    fn gaxios_delay_matches_the_fixed_formula() {
        assert_eq!(gaxios_delay(0), Duration::from_millis(100));
        assert_eq!(gaxios_delay(1), Duration::from_millis(500));
        assert_eq!(gaxios_delay(2), Duration::from_millis(1500));
        assert_eq!(gaxios_delay(3), Duration::from_millis(3500));
    }

    #[test]
    fn gaxios_status_ranges_are_exact() {
        for status in [100, 150, 199, 408, 429, 500, 503, 599] {
            assert!(gaxios_retry_status(status), "{status} retries");
        }
        for status in [200, 301, 400, 401, 403, 409] {
            assert!(!gaxios_retry_status(status), "{status} is terminal");
        }
    }
}
