//! Local AWS credential-source chain for Bedrock.
//!
//! One coordinator resolves one selected-source identity per call from the
//! completed local families: explicit typed selections, scoped/ambient
//! environment pairs and bearer tokens, the shared-config INI family
//! (assume-role/source-profile → static → web-identity →
//! `credential_process`, in the verified `resolveProfileData` order), the
//! environment-driven web-identity segment, and the remote ECS/IMDS segment.
//! Each step yields an exact Continue/Selected/Terminal outcome per the
//! verified locked-Node source corpus:
//!
//! - **Selected** — the step resolved a complete credential bundle; no lower
//!   source runs.
//! - **Continue** — the source was unconfigured or failed a continue-eligible
//!   discovery check (missing ambient pair, absent profile, unreadable or
//!   junk profile file, missing/blank or failed `credential_process`,
//!   ambiguous role configuration, profile cycles, unsupported
//!   `credential_source`, ECS/IMDS unavailability). The chain advances; it
//!   never borrows another family's halves and never falls from a selected
//!   ECS segment to IMDS.
//! - **Terminal** — a selected/scoped typed source is incomplete, a required
//!   MFA presenter is missing, the selected token file is unreadable, the
//!   configured container endpoint violates host/protocol rules, STS denied
//!   the exchange, or the families are exhausted. These fail closed as
//!   [`CredentialResolutionError`](self).
//!
//! Source order: explicit typed selection → bearer token (scoped then
//! ambient) → environment pair (skipped when an explicit/scoped profile
//! governs) → INI family → environment web identity → ECS (modern then
//! legacy) or IMDS. STS exchange sends and Bedrock generation sends are
//! counted separately by construction: they run on distinct injectable
//! seams. SSO/SSOOIDC/login families are a later slice; their absence is
//! reported as typed exhaustion, never silently substituted.
//!
//! # Unstable
//!
//! Part of the **unstable 0.x extension substrate**. Breaking changes may
//! occur between minor versions without a major version bump.

use std::collections::HashSet;
use std::path::Path;

use secrecy::SecretString;

use crate::auth::aws::cache::{AwsSourceCache, DpopJtiSource, SourceClock};
use crate::auth::aws::endpoint::sts_endpoint_url;
use crate::auth::aws::login::resolve_login_session;
use crate::auth::aws::metadata::{self, EcsInputs, ImdsInputs};
use crate::auth::aws::process::{
    CredentialProcessOutput, CredentialProcessRunner, CredentialProcessSpec,
};
use crate::auth::aws::role::{
    MfaTokenPresenter, RoleAssumption, StsExchange, StsExchangeError, assume_role,
};
use crate::auth::aws::source_http::AwsSourceTransport;
use crate::auth::aws::sso::{SsoProfileConfig, resolve_sso_source};
use crate::auth::aws::web_identity::{
    WebIdentityError, assume_role_with_web_identity, read_web_identity_token,
};
use crate::auth::{
    AuthContext, AuthFallback, AuthProvenance, AuthProvenanceSource, AuthScheme,
    AwsCredentialSource, AwsSigV4Credentials, ResolvedAuth,
};
use crate::provider::ProviderErrorSummary;

/// Typed, secret-free failures of local AWS credential resolution.
///
/// Error values deliberately omit file paths, process output, operating-system
/// diagnostics, and credential material so callers can surface them safely.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum CredentialResolutionError {
    #[error("no AWS credentials are available from the local sources")]
    Exhausted,
    #[error("the attempted {credential_source:?} AWS credential source is incomplete")]
    IncompleteSource { credential_source: CredentialSource },
    /// A selected source failed a closed, non-secret terminal rule. No lower
    /// source is consulted.
    #[error("the {credential_source:?} AWS credential source failed: {reason}")]
    Terminal {
        /// The selected source that failed.
        credential_source: CredentialSource,
        /// Typed, secret-free terminal reason.
        reason: TerminalReason,
    },
}

/// Typed, secret-free reasons a selected AWS source fails terminally.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TerminalReason {
    /// A profile requires MFA (`mfa_serial`) but no presenter was injected.
    /// opi never prompts interactively during resolution.
    #[error("MFA presentation is required but no presenter is available")]
    MfaPresenterMissing,
    /// The injected MFA presenter failed to produce a token code.
    #[error("MFA presentation failed")]
    MfaPresentationFailed,
    /// STS denied the exchange; status/code/request-id are retained.
    #[error("STS denied the exchange (status {}, code {}, request id {})",
        status.map(|s| s.to_string()).unwrap_or_else(|| "unknown".into()),
        code.as_deref().unwrap_or("unknown"),
        request_id.as_deref().unwrap_or("unknown"))]
    StsDenied {
        /// HTTP status code when observed.
        status: Option<u16>,
        /// Standard STS error code when present.
        code: Option<String>,
        /// Response request id when present.
        request_id: Option<String>,
    },
    /// The STS exchange failed at the transport level.
    #[error("the STS exchange failed at the transport level")]
    StsTransport,
    /// A 2xx STS response did not satisfy the fixed shape.
    #[error("the STS response was invalid")]
    InvalidStsResponse,
    /// The configured container endpoint violates host/protocol rules.
    #[error("the configured container endpoint violates host or protocol rules")]
    InvalidContainerEndpoint,
    /// The selected web-identity token file is unreadable or empty.
    #[error("the web identity token file is unreadable")]
    TokenFileUnreadable,
    /// A selected metadata source returned an invalid credential payload.
    #[error("the metadata source returned an invalid credential payload")]
    InvalidSourceResponse,
    /// An SSO selection was partially configured (fields present but
    /// incomplete). Never treated as "unconfigured".
    #[error("the SSO configuration is incomplete")]
    IncompleteSsoConfiguration,
    /// The SSO token is unreadable, invalid, or expired without a refresh
    /// path (legacy tokens never refresh; a fresh login is required).
    #[error("the SSO token is unreadable, invalid, or expired")]
    InvalidSsoToken,
    /// The SSO service denied an exchange; status/code/request-id are
    /// retained (OAuth error codes including slowdown/throttle).
    #[error("SSO denied the exchange (status {}, code {}, request id {})",
        status.map(|s| s.to_string()).unwrap_or_else(|| "unknown".into()),
        code.as_deref().unwrap_or("unknown"),
        request_id.as_deref().unwrap_or("unknown"))]
    SsoDenied {
        /// HTTP status code when observed.
        status: Option<u16>,
        /// Standard error code when present.
        code: Option<String>,
        /// Response request id when present.
        request_id: Option<String>,
    },
    /// A 2xx SSO response did not satisfy the fixed shape.
    #[error("the SSO response was invalid")]
    InvalidSsoResponse,
    /// The SSO exchange failed at the transport level.
    #[error("the SSO exchange failed at the transport level")]
    SsoTransport,
    /// A selected login session's cache is missing or unusable.
    #[error("the login session cache is missing or invalid")]
    LoginCacheInvalid,
    /// The owned ES256 DPoP proof could not be built or signed.
    #[error("the DPoP proof failed")]
    DpopFailed,
    /// SignIn denied the refresh with the terminal AccessDenied family.
    #[error("SignIn denied the refresh (status {}, code {})",
        status.map(|s| s.to_string()).unwrap_or_else(|| "unknown".into()),
        code.as_deref().unwrap_or("unknown"))]
    LoginDenied {
        /// HTTP status code when observed.
        status: Option<u16>,
        /// Error code when present.
        code: Option<String>,
    },
    /// A 2xx SignIn response did not satisfy the fixed shape.
    #[error("the SignIn response was invalid")]
    InvalidLoginResponse,
}

impl CredentialResolutionError {
    /// Convert this closed, non-secret reason into a provider-facing summary.
    /// Unlike arbitrary upstream text, these variants contain no paths,
    /// process output, operating-system diagnostics, or credential material.
    pub fn into_safe_provider_summary(self) -> ProviderErrorSummary {
        ProviderErrorSummary::sanitized(self.to_string())
    }
}

/// Local source family that produced a resolved credential bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialSource {
    /// The resolution input supplied the complete credential pair directly
    /// (explicit typed selection or configured static pair).
    ExplicitConfig,
    /// A configured access-key ID was paired with secrets loaded from named
    /// environment variables. The names are non-secret; no resolved value is
    /// retained in provenance.
    ConfiguredEnvironment {
        /// Environment-variable name configured for the secret access key.
        secret_access_key_env: String,
        /// Optional environment-variable name configured for a session token.
        session_token_env: Option<String>,
    },
    /// Standard AWS environment variables supplied the complete ambient pair.
    Environment,
    /// Scoped (injected per-call/per-credential) environment values supplied
    /// the complete pair.
    ScopedEnvironment,
    /// The merged shared-config profile's static credentials supplied the
    /// bundle; the identity anchor (access key id) came from the
    /// credentials file.
    ProfileFile,
    /// The merged shared-config profile's static credentials supplied the
    /// bundle; the identity anchor came from the config file.
    ConfigFile,
    /// The profile's `credential_process` supplied the bundle.
    CredentialProcess,
    /// The profile's `role_arn` exchange at STS supplied temporary
    /// credentials, signed by a resolved source profile or a
    /// `credential_source` provider.
    AssumeRole {
        /// Owning shared-config profile name.
        profile: String,
    },
    /// A web-identity token file exchange at STS supplied temporary
    /// credentials without source SigV4.
    WebIdentity {
        /// Where the selected token file/role came from.
        route: crate::auth::WebIdentityRoute,
    },
    /// The SSO source family (explicit init fields or a shared-config
    /// profile plus its `sso-session` section) supplied the bundle.
    Sso {
        /// Owning shared-config profile name; `None` for the explicit
        /// init segment.
        profile: Option<String>,
    },
    /// A login session (`login_session` profile key) supplied the bundle
    /// through the SignIn refresh with the owned ES256 DPoP proof.
    LoginSession {
        /// Trimmed session name.
        session: String,
    },
    /// The ECS container metadata provider supplied the bundle; `legacy`
    /// distinguishes the one-send legacy container provider from the modern
    /// fixed-budget provider.
    EcsContainer {
        /// Whether the legacy provider produced the bundle.
        legacy: bool,
    },
    /// The EC2 instance metadata (IMDS) provider supplied the bundle.
    Ec2InstanceMetadata,
}

/// Route a web-identity selection took (see
/// [`crate::auth::WebIdentityRoute`]).
pub use crate::auth::WebIdentityRoute;

impl From<CredentialSource> for AwsCredentialSource {
    fn from(source: CredentialSource) -> Self {
        match source {
            CredentialSource::ExplicitConfig => Self::ExplicitConfig,
            CredentialSource::ConfiguredEnvironment {
                secret_access_key_env,
                session_token_env,
            } => Self::ConfiguredEnvironment {
                secret_access_key_env,
                session_token_env,
            },
            CredentialSource::Environment => Self::Environment,
            CredentialSource::ScopedEnvironment => Self::ScopedEnvironment,
            CredentialSource::ProfileFile => Self::ProfileFile,
            CredentialSource::ConfigFile => Self::ConfigFile,
            CredentialSource::CredentialProcess => Self::CredentialProcess,
            CredentialSource::AssumeRole { profile } => Self::AssumeRole { profile },
            CredentialSource::WebIdentity { route } => Self::WebIdentity { route },
            CredentialSource::Sso { profile } => Self::Sso { profile },
            CredentialSource::LoginSession { session } => Self::LoginSession { session },
            CredentialSource::EcsContainer { legacy } => Self::EcsContainer { legacy },
            CredentialSource::Ec2InstanceMetadata => Self::Ec2InstanceMetadata,
        }
    }
}

/// Injected per-call/per-credential AWS environment values ("scoped" env).
///
/// Scoped values shadow their ambient counterparts key-by-key. An
/// attempted-but-partial scoped pair fails closed: it never borrows ambient
/// halves (retained opi protection; an explicit difference from Pi's
/// per-field mixing).
#[derive(Debug, Clone, Default)]
pub struct ScopedAwsEnv {
    /// Scoped `AWS_ACCESS_KEY_ID`.
    pub access_key_id: Option<SecretString>,
    /// Scoped `AWS_SECRET_ACCESS_KEY`.
    pub secret_access_key: Option<SecretString>,
    /// Scoped `AWS_SESSION_TOKEN`.
    pub session_token: Option<SecretString>,
    /// Scoped `AWS_PROFILE`; with an explicit selection it suppresses the
    /// environment pair.
    pub profile: Option<String>,
    /// Scoped `AWS_REGION`.
    pub region: Option<String>,
    /// Scoped `AWS_DEFAULT_REGION`.
    pub default_region: Option<String>,
    /// Scoped `AWS_BEARER_TOKEN_BEDROCK`.
    pub bearer_token_bedrock: Option<SecretString>,
    /// Scoped `AWS_WEB_IDENTITY_TOKEN_FILE`.
    pub web_identity_token_file: Option<String>,
    /// Scoped `AWS_ROLE_ARN`.
    pub role_arn: Option<String>,
    /// Scoped `AWS_ROLE_SESSION_NAME`.
    pub role_session_name: Option<String>,
    /// Scoped `AWS_CONTAINER_CREDENTIALS_RELATIVE_URI`.
    pub container_credentials_relative_uri: Option<String>,
    /// Scoped `AWS_CONTAINER_CREDENTIALS_FULL_URI`.
    pub container_credentials_full_uri: Option<String>,
    /// Scoped `AWS_CONTAINER_AUTHORIZATION_TOKEN`.
    pub container_authorization_token: Option<SecretString>,
    /// Scoped `AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE`.
    pub container_authorization_token_file: Option<String>,
    /// Scoped explicit-init SSO start URL (no ambient counterpart: the
    /// fixed chain treats these as init fields, not environment values).
    pub sso_start_url: Option<String>,
    /// Scoped explicit-init SSO account id.
    pub sso_account_id: Option<String>,
    /// Scoped explicit-init SSO region.
    pub sso_region: Option<String>,
    /// Scoped explicit-init SSO role name.
    pub sso_role_name: Option<String>,
    /// Scoped explicit-init modern SSO session name.
    pub sso_session: Option<String>,
    /// Scoped `AWS_EC2_METADATA_DISABLED` (parsed semantics).
    pub ec2_metadata_disabled: Option<bool>,
    /// Scoped `AWS_METADATA_SERVICE_ENDPOINT`.
    pub metadata_service_endpoint: Option<String>,
    /// Scoped `AWS_EC2_METADATA_SERVICE_ENDPOINT_MODE`.
    pub metadata_service_endpoint_mode: Option<String>,
}

impl ScopedAwsEnv {
    /// No scoped values: every key resolves from the ambient context.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Whether any scoped environment-pair key was supplied.
    fn pair_attempted(&self) -> bool {
        self.access_key_id.is_some()
            || self.secret_access_key.is_some()
            || self.session_token.is_some()
    }
}

/// One explicit typed auth selection for the chain.
///
/// The enum makes mutual exclusion structural: exactly one explicit choice
/// can be selected, and an explicit choice wins over every Auto probe
/// without reading any source.
#[derive(Debug, Clone)]
pub enum AwsAuthSelection {
    /// Resolve through the local source chain in verified order.
    Auto,
    /// Pre-resolved typed SigV4 credentials; zero source access.
    SigV4(AwsSigV4Credentials),
    /// Pre-resolved Bearer token; zero source access.
    Bearer(SecretString),
    /// Resolve through the INI/process stages for exactly this profile; the
    /// environment pair is skipped.
    Profile(String),
}

/// Input for one local-chain resolution.
pub struct AwsLocalChainInput<'a> {
    /// Explicit typed selection (or [`AwsAuthSelection::Auto`]).
    pub selection: AwsAuthSelection,
    /// Scoped environment values shadowing the ambient context.
    pub scoped: ScopedAwsEnv,
    /// Injectable ambient-source access. Never read for explicit selections.
    pub ambient: &'a dyn AuthContext,
    /// Optional injected shared credentials file path.
    pub credentials_file: Option<&'a Path>,
    /// Optional injected shared config file path.
    pub config_file: Option<&'a Path>,
    /// Injectable `credential_process` runner; called at most once per
    /// profile per resolution.
    pub process: &'a dyn CredentialProcessRunner,
    /// Injectable HTTP transport for STS/web-identity/ECS/IMDS exchanges.
    /// Every auth-source send runs here, separate from generation attempts.
    pub transport: &'a dyn AwsSourceTransport,
    /// Optional injected MFA presenter for profile-required MFA.
    pub mfa: Option<&'a dyn MfaTokenPresenter>,
    /// Explicit STS endpoint override (tests and pinned deployments).
    pub sts_endpoint: Option<&'a str>,
    /// Shared source cache (single-flight, refresh windows, mutation-owner
    /// registration). Required for the SSO/login families.
    pub source_cache: Option<&'a AwsSourceCache>,
    /// Injectable clock for expiry windows.
    pub clock: &'a dyn SourceClock,
    /// Injectable DPoP `jti` source.
    pub dpop_jti: &'a dyn DpopJtiSource,
    /// Optional injected standard SSO token-cache directory.
    pub sso_cache_dir: Option<&'a Path>,
    /// Optional injected standard login cache directory.
    pub login_cache_dir: Option<&'a Path>,
    /// SSO portal endpoint override.
    pub sso_endpoint: Option<&'a str>,
    /// SSO OIDC endpoint override.
    pub oidc_endpoint: Option<&'a str>,
    /// SignIn endpoint override. The default construction is a documented
    /// opi default (the corpus pins the wire path, not the host).
    pub signin_endpoint: Option<&'a str>,
    /// Explicit region option (highest region precedence).
    pub region: Option<&'a str>,
    /// Region of last resort when no source named one.
    pub default_region: &'a str,
}

impl<'a> AwsLocalChainInput<'a> {
    /// Attach the auth-source HTTP transport.
    pub fn with_source_transport(mut self, transport: &'a dyn AwsSourceTransport) -> Self {
        self.transport = transport;
        self
    }

    /// Attach the MFA presenter.
    pub fn with_mfa(mut self, mfa: Option<&'a dyn MfaTokenPresenter>) -> Self {
        self.mfa = mfa;
        self
    }

    /// Attach a shared source cache (SSO/login families).
    pub fn with_source_cache(mut self, cache: Option<&'a AwsSourceCache>) -> Self {
        self.source_cache = cache;
        self
    }

    /// Attach an injectable clock.
    pub fn with_clock(mut self, clock: &'a dyn SourceClock) -> Self {
        self.clock = clock;
        self
    }

    /// Attach the DPoP `jti` source.
    pub fn with_dpop_jti(mut self, jti: &'a dyn DpopJtiSource) -> Self {
        self.dpop_jti = jti;
        self
    }

    /// Attach the standard SSO token-cache directory.
    pub fn with_sso_cache_dir(mut self, dir: Option<&'a Path>) -> Self {
        self.sso_cache_dir = dir;
        self
    }

    /// Attach the standard login cache directory.
    pub fn with_login_cache_dir(mut self, dir: Option<&'a Path>) -> Self {
        self.login_cache_dir = dir;
        self
    }

    /// Pin the SignIn endpoint.
    pub fn with_signin_endpoint(mut self, endpoint: Option<&'a str>) -> Self {
        self.signin_endpoint = endpoint;
        self
    }

    /// Pin an explicit STS endpoint.
    pub fn with_sts_endpoint(mut self, endpoint: Option<&'a str>) -> Self {
        self.sts_endpoint = endpoint;
        self
    }
}

pub(crate) const ENV_ACCESS_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
pub(crate) const ENV_SECRET_ACCESS_KEY: &str = "AWS_SECRET_ACCESS_KEY";
pub(crate) const ENV_SESSION_TOKEN: &str = "AWS_SESSION_TOKEN";
pub(crate) const ENV_PROFILE: &str = "AWS_PROFILE";
pub(crate) const ENV_REGION: &str = "AWS_REGION";
pub(crate) const ENV_DEFAULT_REGION: &str = "AWS_DEFAULT_REGION";
pub(crate) const ENV_BEARER_TOKEN_BEDROCK: &str = "AWS_BEARER_TOKEN_BEDROCK";
pub(crate) const ENV_WEB_IDENTITY_TOKEN_FILE: &str = "AWS_WEB_IDENTITY_TOKEN_FILE";
pub(crate) const ENV_ROLE_ARN: &str = "AWS_ROLE_ARN";
pub(crate) const ENV_ROLE_SESSION_NAME: &str = "AWS_ROLE_SESSION_NAME";
pub(crate) const ENV_ECS_RELATIVE_URI: &str = "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI";
pub(crate) const ENV_ECS_FULL_URI: &str = "AWS_CONTAINER_CREDENTIALS_FULL_URI";
pub(crate) const ENV_ECS_AUTHORIZATION_TOKEN: &str = "AWS_CONTAINER_AUTHORIZATION_TOKEN";
pub(crate) const ENV_ECS_AUTHORIZATION_TOKEN_FILE: &str = "AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE";
pub(crate) const ENV_EC2_METADATA_DISABLED: &str = "AWS_EC2_METADATA_DISABLED";
pub(crate) const ENV_METADATA_SERVICE_ENDPOINT: &str = "AWS_METADATA_SERVICE_ENDPOINT";
pub(crate) const ENV_METADATA_SERVICE_ENDPOINT_MODE: &str =
    "AWS_EC2_METADATA_SERVICE_ENDPOINT_MODE";

/// Resolve one complete prepared Bedrock authentication result from the local
/// AWS source chain.
///
/// The selected source is retained as typed, non-secret provenance beside the
/// zeroizing credential bundle. Failure is typed and fail-closed: no lower
/// source is consulted after a terminal outcome.
pub async fn resolve_local_aws_auth(
    input: &AwsLocalChainInput<'_>,
) -> Result<ResolvedAuth, CredentialResolutionError> {
    match input.selection.clone() {
        AwsAuthSelection::SigV4(credentials) => {
            Ok(ResolvedAuth::aws_sigv4(credentials, explicit_provenance()))
        }
        AwsAuthSelection::Bearer(token) => {
            let region = resolve_region(input, None).await;
            Ok(bearer_auth(
                token,
                AuthProvenanceSource::Explicit,
                None,
                &region,
            ))
        }
        AwsAuthSelection::Profile(profile) => resolve_auto(input, Some(profile)).await,
        AwsAuthSelection::Auto => resolve_auto(input, None).await,
    }
}

fn explicit_provenance() -> AuthProvenance {
    AuthProvenance {
        source: AuthProvenanceSource::Explicit,
        fallback: AuthFallback::NotAttempted,
    }
}

fn source_provenance(source: CredentialSource) -> AuthProvenance {
    AuthProvenance {
        source: AuthProvenanceSource::AwsSigV4 {
            source: source.into(),
        },
        fallback: AuthFallback::NotAttempted,
    }
}

fn bearer_auth(
    token: SecretString,
    source: AuthProvenanceSource,
    base_url_override: Option<String>,
    region: &str,
) -> ResolvedAuth {
    ResolvedAuth {
        scheme: AuthScheme::Bearer,
        secret: token,
        base_url: base_url_override.or_else(|| Some(bedrock_runtime_url(region))),
        account_id: None,
        provenance: AuthProvenance {
            source,
            fallback: AuthFallback::NotAttempted,
        },
    }
}

pub(crate) fn bedrock_runtime_url(region: &str) -> String {
    format!("https://bedrock-runtime.{region}.amazonaws.com")
}

/// Build the explicit-init SSO selection from scoped fields (the fixed
/// chain treats these as init fields, not ambient environment values).
/// `Err(Terminal)` marks an incomplete selection.
async fn explicit_sso_config(
    input: &AwsLocalChainInput<'_>,
) -> Result<Option<SsoProfileConfig>, CredentialResolutionError> {
    let scoped = &input.scoped;
    if scoped.sso_start_url.is_none()
        && scoped.sso_account_id.is_none()
        && scoped.sso_region.is_none()
        && scoped.sso_role_name.is_none()
        && scoped.sso_session.is_none()
    {
        return Ok(None);
    }
    if input.source_cache.is_none() {
        return Err(CredentialResolutionError::Terminal {
            credential_source: CredentialSource::Sso { profile: None },
            reason: TerminalReason::IncompleteSsoConfiguration,
        });
    }
    build_sso_config(
        input,
        scoped.sso_session.as_deref(),
        scoped.sso_start_url.as_deref(),
        scoped.sso_region.as_deref(),
        scoped.sso_account_id.as_deref(),
        scoped.sso_role_name.as_deref(),
    )
    .await
    .map_err(|()| CredentialResolutionError::Terminal {
        credential_source: CredentialSource::Sso { profile: None },
        reason: TerminalReason::IncompleteSsoConfiguration,
    })
}

async fn resolve_auto(
    input: &AwsLocalChainInput<'_>,
    explicit_profile: Option<String>,
) -> Result<ResolvedAuth, CredentialResolutionError> {
    // Bearer (scoped then ambient) precedes every SigV4-family source, per
    // the verified adapter order.
    if let Some(auth) = resolve_bearer(input).await? {
        return Ok(auth);
    }

    // An explicit or scoped profile governs: the environment pair is skipped
    // entirely. Only an ambient profile does not suppress the pair.
    let governing_profile = explicit_profile.or_else(|| input.scoped.profile.clone());

    if governing_profile.is_none()
        && let Some(auth) = resolve_environment_pair(input).await?
    {
        return Ok(auth);
    }

    // Chain segment 2: explicit SSO (scoped init fields only; a partially
    // given selection is terminal). A governing profile skips this segment:
    // profile SSO is handled inside the INI family.
    if governing_profile.is_none()
        && let Some(sso) = explicit_sso_config(input).await?
    {
        let (credentials, _sso_region) = resolve_sso_source(input, &sso, None).await?;
        return Ok(ResolvedAuth::aws_sigv4(
            credentials,
            source_provenance(CredentialSource::Sso { profile: None }),
        ));
    }

    let profile_name = governing_profile
        .or_else(|| {
            input
                .ambient
                .env(ENV_PROFILE)
                .ok()
                .flatten()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| "default".to_owned());

    let (ini_outcome, profile_region) = resolve_ini_profile(input, &profile_name).await?;
    if let Some(auth) = ini_outcome {
        return Ok(auth);
    }

    // Chain segment 5: environment-driven web identity (explicit init
    // fields are not part of this coordinator; scoped values shadow the
    // ambient ones).
    if let Some(auth) = resolve_environment_web_identity(input, profile_region.as_deref()).await? {
        return Ok(auth);
    }

    // Chain segment 6: remote ECS or IMDS. A selected ECS segment never
    // falls through to IMDS.
    resolve_remote(input, profile_region.as_deref()).await
}

async fn resolve_bearer(
    input: &AwsLocalChainInput<'_>,
) -> Result<Option<ResolvedAuth>, CredentialResolutionError> {
    if let Some(token) = input.scoped.bearer_token_bedrock.clone() {
        let region = resolve_region(input, None).await;
        return Ok(Some(bearer_auth(
            token,
            AuthProvenanceSource::Environment {
                name: ENV_BEARER_TOKEN_BEDROCK.to_owned(),
            },
            None,
            &region,
        )));
    }
    if let Some(token) = ambient_non_blank(input, ENV_BEARER_TOKEN_BEDROCK) {
        let region = resolve_region(input, None).await;
        return Ok(Some(bearer_auth(
            SecretString::from(token),
            AuthProvenanceSource::Environment {
                name: ENV_BEARER_TOKEN_BEDROCK.to_owned(),
            },
            None,
            &region,
        )));
    }
    Ok(None)
}

/// Resolve the scoped/ambient environment pair step.
///
/// A partial scoped pair fails closed without borrowing ambient halves. An
/// incomplete or missing ambient pair is continue-eligible per the fixed
/// Node corpus.
async fn resolve_environment_pair(
    input: &AwsLocalChainInput<'_>,
) -> Result<Option<ResolvedAuth>, CredentialResolutionError> {
    if input.scoped.pair_attempted() {
        let complete = input
            .scoped
            .access_key_id
            .as_ref()
            .is_some_and(non_blank_secret)
            && input
                .scoped
                .secret_access_key
                .as_ref()
                .is_some_and(non_blank_secret);
        if !complete {
            return Err(CredentialResolutionError::IncompleteSource {
                credential_source: CredentialSource::ScopedEnvironment,
            });
        }
        let region = resolve_region(input, None).await;
        let credentials = AwsSigV4Credentials {
            access_key_id: input.scoped.access_key_id.clone().expect("checked above"),
            secret_access_key: input
                .scoped
                .secret_access_key
                .clone()
                .expect("checked above"),
            session_token: input.scoped.session_token.clone().filter(non_blank_secret),
            region,
            expires_at: None,
        };
        return Ok(Some(ResolvedAuth::aws_sigv4(
            credentials,
            source_provenance(CredentialSource::ScopedEnvironment),
        )));
    }
    let ambient_access = ambient_non_blank(input, ENV_ACCESS_KEY_ID);
    let ambient_secret = ambient_non_blank(input, ENV_SECRET_ACCESS_KEY);
    if let (Some(access), Some(secret)) = (ambient_access, ambient_secret) {
        let region = resolve_region(input, None).await;
        let credentials = AwsSigV4Credentials {
            access_key_id: SecretString::from(access),
            secret_access_key: SecretString::from(secret),
            session_token: ambient_non_blank(input, ENV_SESSION_TOKEN).map(SecretString::from),
            region,
            expires_at: None,
        };
        return Ok(Some(ResolvedAuth::aws_sigv4(
            credentials,
            source_provenance(CredentialSource::Environment),
        )));
    }
    // Partial or missing ambient pair: continue to the INI profile.
    Ok(None)
}

fn non_blank_secret(value: &SecretString) -> bool {
    !secrecy::ExposeSecret::expose_secret(value)
        .trim()
        .is_empty()
}

fn ambient_non_blank(input: &AwsLocalChainInput<'_>, name: &str) -> Option<String> {
    input
        .ambient
        .env(name)
        .ok()
        .flatten()
        .filter(|value| !value.trim().is_empty())
}

/// Region precedence: explicit option → scoped `AWS_REGION` → ambient
/// `AWS_REGION` → scoped `AWS_DEFAULT_REGION` → ambient `AWS_DEFAULT_REGION`
/// → merged profile region (when supplied) → declared default.
async fn resolve_region(input: &AwsLocalChainInput<'_>, profile_region: Option<&str>) -> String {
    let candidates = [
        input
            .region
            .map(str::to_owned)
            .filter(|value| !value.trim().is_empty()),
        input.scoped.region.clone(),
        ambient_non_blank(input, ENV_REGION),
        input.scoped.default_region.clone(),
        ambient_non_blank(input, ENV_DEFAULT_REGION),
        profile_region
            .map(str::to_owned)
            .filter(|value| !value.trim().is_empty()),
    ];
    candidates
        .into_iter()
        .flatten()
        .next()
        .unwrap_or_else(|| input.default_region.to_owned())
}

/// Per-field merged shared-config profile properties.
///
/// The credentials file overrides the config file field-by-field; the
/// credentials/debug-redacted shape keeps secrets out of diagnostics.
#[derive(Clone, Default)]
struct ProfileProperties {
    access_key_id: Option<String>,
    secret_access_key: Option<String>,
    session_token: Option<String>,
    region: Option<String>,
    credential_process: Option<String>,
    role_arn: Option<String>,
    role_session_name: Option<String>,
    external_id: Option<String>,
    duration_seconds: Option<String>,
    mfa_serial: Option<String>,
    web_identity_token_file: Option<String>,
    source_profile: Option<String>,
    credential_source: Option<String>,
    sso_session: Option<String>,
    sso_start_url: Option<String>,
    sso_account_id: Option<String>,
    sso_region: Option<String>,
    sso_role_name: Option<String>,
    login_session: Option<String>,
    /// Whether the access key id (identity anchor) came from the credentials
    /// file (`true`) or the config file (`false`).
    access_key_from_credentials_file: bool,
}

impl std::fmt::Debug for ProfileProperties {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProfileProperties")
            .field(
                "access_key_id",
                &self.access_key_id.as_ref().map(|_| "<redacted>"),
            )
            .field(
                "secret_access_key",
                &self.secret_access_key.as_ref().map(|_| "<redacted>"),
            )
            .field(
                "session_token",
                &self.session_token.as_ref().map(|_| "<redacted>"),
            )
            .field("region", &self.region)
            .field(
                "credential_process",
                &self.credential_process.as_ref().map(|_| "<redacted>"),
            )
            .field("role_arn", &self.role_arn)
            .field("role_session_name", &self.role_session_name)
            .field("external_id", &self.external_id)
            .field("duration_seconds", &self.duration_seconds)
            .field("mfa_serial", &self.mfa_serial)
            .field("web_identity_token_file", &self.web_identity_token_file)
            .field("source_profile", &self.source_profile)
            .field("credential_source", &self.credential_source)
            .field(
                "access_key_from_credentials_file",
                &self.access_key_from_credentials_file,
            )
            .finish()
    }
}

impl ProfileProperties {
    fn is_empty(&self) -> bool {
        self.access_key_id.is_none()
            && self.secret_access_key.is_none()
            && self.session_token.is_none()
            && self.region.is_none()
            && self.credential_process.is_none()
            && self.role_arn.is_none()
            && self.role_session_name.is_none()
            && self.external_id.is_none()
            && self.duration_seconds.is_none()
            && self.mfa_serial.is_none()
            && self.web_identity_token_file.is_none()
            && self.source_profile.is_none()
            && self.credential_source.is_none()
            && self.sso_session.is_none()
            && self.sso_start_url.is_none()
            && self.sso_account_id.is_none()
            && self.sso_region.is_none()
            && self.sso_role_name.is_none()
            && self.login_session.is_none()
    }

    /// Overlay `upper` onto `self` field-by-field; non-blank `upper` values
    /// win, and fields `upper` leaves blank or absent keep the `self` value.
    /// The identity-anchor flag tracks where the winning access key id
    /// came from.
    fn overlay(mut self, upper: Self, upper_is_credentials_file: bool) -> Self {
        fn merge(lower: &mut Option<String>, upper: Option<String>) {
            if let Some(value) = upper.filter(|value| !value.trim().is_empty()) {
                *lower = Some(value);
            }
        }
        if let Some(access) = upper
            .access_key_id
            .clone()
            .filter(|value| !value.trim().is_empty())
        {
            self.access_key_from_credentials_file = upper_is_credentials_file;
            self.access_key_id = Some(access);
        }
        merge(&mut self.secret_access_key, upper.secret_access_key);
        merge(&mut self.session_token, upper.session_token);
        merge(&mut self.region, upper.region);
        merge(&mut self.credential_process, upper.credential_process);
        merge(&mut self.role_arn, upper.role_arn);
        merge(&mut self.role_session_name, upper.role_session_name);
        merge(&mut self.external_id, upper.external_id);
        merge(&mut self.duration_seconds, upper.duration_seconds);
        merge(&mut self.mfa_serial, upper.mfa_serial);
        merge(
            &mut self.web_identity_token_file,
            upper.web_identity_token_file,
        );
        merge(&mut self.source_profile, upper.source_profile);
        merge(&mut self.credential_source, upper.credential_source);
        merge(&mut self.sso_session, upper.sso_session);
        merge(&mut self.sso_start_url, upper.sso_start_url);
        merge(&mut self.sso_account_id, upper.sso_account_id);
        merge(&mut self.sso_region, upper.sso_region);
        merge(&mut self.sso_role_name, upper.sso_role_name);
        merge(&mut self.login_session, upper.login_session);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfileFileKind {
    Credentials,
    Config,
}

/// Read one profile section from a shared file. Read failures are
/// continue-eligible: the file contributes no properties (`None`).
async fn read_profile_properties(
    path: &Path,
    profile_name: &str,
    kind: ProfileFileKind,
) -> Option<ProfileProperties> {
    let contents = tokio::fs::read_to_string(path).await.ok()?;
    parse_profile_properties(&contents, profile_name, kind)
}

/// Parse one profile section leniently: blank lines, comments, and
/// non-`key=value` lines are skipped (the fixed Node corpus swallows file
/// read/parse failures instead of failing resolution closed).
fn parse_profile_properties(
    contents: &str,
    profile_name: &str,
    kind: ProfileFileKind,
) -> Option<ProfileProperties> {
    let target_header = match kind {
        ProfileFileKind::Credentials => format!("[{profile_name}]"),
        ProfileFileKind::Config if profile_name == "default" => "[default]".to_string(),
        ProfileFileKind::Config => format!("[profile {profile_name}]"),
    };

    let mut in_target = false;
    let mut props = ProfileProperties::default();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            if in_target {
                break;
            }
            in_target = line == target_header;
            continue;
        }
        if !in_target {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().to_string();
        match key {
            "aws_access_key_id" => props.access_key_id = Some(value),
            "aws_secret_access_key" => props.secret_access_key = Some(value),
            "aws_session_token" => props.session_token = Some(value),
            "region" => props.region = Some(value),
            "credential_process" => props.credential_process = Some(value),
            "role_arn" => props.role_arn = Some(value),
            "role_session_name" => props.role_session_name = Some(value),
            "external_id" => props.external_id = Some(value),
            "duration_seconds" => props.duration_seconds = Some(value),
            "duration_seconds_long" => props.duration_seconds = Some(value),
            "mfa_serial" => props.mfa_serial = Some(value),
            "web_identity_token_file" => props.web_identity_token_file = Some(value),
            "source_profile" => props.source_profile = Some(value),
            "credential_source" => props.credential_source = Some(value),
            "sso_session" => props.sso_session = Some(value),
            "sso_start_url" => props.sso_start_url = Some(value),
            "sso_account_id" => props.sso_account_id = Some(value),
            "sso_region" => props.sso_region = Some(value),
            "sso_role_name" => props.sso_role_name = Some(value),
            "login_session" => props.login_session = Some(value),
            _ => {}
        }
    }

    (!props.is_empty()).then_some(props)
}

/// One `[sso-session NAME]` section from the shared config file.
#[derive(Debug, Clone, Default)]
struct SsoSessionData {
    start_url: Option<String>,
    region: Option<String>,
}

/// Read one `[sso-session NAME]` section from the config file. Missing
/// sections and unreadable files contribute no data (`None` distinguishes
/// "absent" from an empty section).
async fn read_sso_session_data(
    input: &AwsLocalChainInput<'_>,
    session: &str,
) -> Option<SsoSessionData> {
    let path = input.config_file?;
    let contents = tokio::fs::read_to_string(path).await.ok()?;
    let target_header = format!("[sso-session {session}]");
    let mut in_target = false;
    let mut data = SsoSessionData::default();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            if in_target {
                break;
            }
            in_target = line == target_header;
            continue;
        }
        if !in_target {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().to_string();
        match key.trim() {
            "sso_start_url" => data.start_url = Some(value),
            "sso_region" => data.region = Some(value),
            _ => {}
        }
    }
    Some(data)
}

/// Build the validated SSO config from merged profile properties (INI
/// family) or scoped init fields (explicit segment). `Ok(None)` means no
/// SSO fields were given (continue); `Err(())` marks an incomplete
/// selection (terminal).
async fn build_sso_config(
    input: &AwsLocalChainInput<'_>,
    session: Option<&str>,
    start_url: Option<&str>,
    sso_region: Option<&str>,
    account_id: Option<&str>,
    role_name: Option<&str>,
) -> Result<Option<SsoProfileConfig>, ()> {
    fn non_empty(value: Option<&str>) -> Option<&str> {
        value.filter(|v| !v.trim().is_empty())
    }
    let session = non_empty(session);
    let start_url = non_empty(start_url);
    let sso_region = non_empty(sso_region);
    let account_id = non_empty(account_id);
    let role_name = non_empty(role_name);
    if session.is_none()
        && start_url.is_none()
        && sso_region.is_none()
        && account_id.is_none()
        && role_name.is_none()
    {
        return Ok(None);
    }
    let (start_url, region) = if let Some(session) = session {
        // Modern selection: the session section supplies the start URL and
        // region unless the selection itself carries them (explicit init
        // fields win; a profile without a section still resolves when the
        // values are inline).
        let data = read_sso_session_data(input, session)
            .await
            .unwrap_or_default();
        let section_start = non_empty(data.start_url.as_deref());
        let section_region = non_empty(data.region.as_deref());
        (
            start_url.or(section_start).ok_or(())?.to_owned(),
            sso_region.or(section_region).ok_or(())?.to_owned(),
        )
    } else {
        (
            start_url.ok_or(())?.to_owned(),
            sso_region.ok_or(())?.to_owned(),
        )
    };
    Ok(Some(SsoProfileConfig {
        start_url,
        region,
        account_id: account_id.ok_or(())?.to_owned(),
        role_name: role_name.ok_or(())?.to_owned(),
        session: session.map(str::to_owned),
    }))
}

/// INI family in the verified `resolveProfileData` order:
/// assume-role/source-profile → static → web-identity → `credential_process`.
///
/// A partial merged static pair, ambiguous role configuration, profile
/// cycles, unsupported `credential_source`, and any `credential_process`
/// failure are continue-eligible. A required MFA without a presenter, an
/// unreadable selected token file, an invalid container endpoint, and STS
/// denials are terminal. `Ok((None, region))` means the family produced
/// nothing; the resolved region still guides later segments.
async fn resolve_ini_profile(
    input: &AwsLocalChainInput<'_>,
    profile_name: &str,
) -> Result<(Option<ResolvedAuth>, Option<String>), CredentialResolutionError> {
    let merged = merged_profile(input, profile_name).await;
    let region = resolve_region(input, merged.region.as_deref()).await;

    // Assume-role: role_arn plus exactly one of source_profile /
    // credential_source.
    let role_arn = merged
        .role_arn
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    let source_profile = merged
        .source_profile
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    let credential_source = merged
        .credential_source
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    if let Some(role_arn) = role_arn {
        match (source_profile, credential_source) {
            (Some(_), Some(_)) | (None, None) => {
                // Ambiguous or incomplete role configuration is
                // continue-eligible (the family advances).
            }
            (Some(source_name), None) => {
                let mut visited = HashSet::from([profile_name.to_owned()]);
                if let Some(auth) = resolve_assume_role_from_source_profile(
                    input,
                    profile_name,
                    role_arn,
                    source_name,
                    &merged,
                    &region,
                    &mut visited,
                )
                .await?
                {
                    return Ok((Some(auth), Some(region)));
                }
            }
            (None, Some(kind)) => {
                if let Some(auth) = resolve_assume_role_from_credential_source(
                    input,
                    profile_name,
                    role_arn,
                    kind,
                    &merged,
                    &region,
                )
                .await?
                {
                    return Ok((Some(auth), Some(region)));
                }
            }
        }
    }

    let static_complete = merged
        .access_key_id
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && merged
            .secret_access_key
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
    if static_complete {
        let source = if merged.access_key_from_credentials_file {
            CredentialSource::ProfileFile
        } else {
            CredentialSource::ConfigFile
        };
        let credentials = AwsSigV4Credentials {
            access_key_id: SecretString::from(merged.access_key_id.clone().unwrap_or_default()),
            secret_access_key: SecretString::from(
                merged.secret_access_key.clone().unwrap_or_default(),
            ),
            session_token: merged
                .session_token
                .clone()
                .filter(|token| !token.trim().is_empty())
                .map(SecretString::from),
            region: region.clone(),
            expires_at: None,
        };
        return Ok((
            Some(ResolvedAuth::aws_sigv4(
                credentials,
                source_provenance(source),
            )),
            Some(region),
        ));
    }

    // Web identity from the profile's token file. Failure of the selected
    // file or the exchange is terminal; the family never discovers another
    // identity.
    let token_file = merged
        .web_identity_token_file
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    if let (Some(role_arn), Some(token_file)) = (role_arn, token_file) {
        let auth = resolve_web_identity(
            input,
            std::path::Path::new(token_file),
            role_arn,
            merged.role_session_name.as_deref(),
            CredentialSource::WebIdentity {
                route: WebIdentityRoute::Profile(profile_name.to_owned()),
            },
            &region,
        )
        .await?;
        return Ok((Some(auth), Some(region)));
    }

    if let Some(command) = merged
        .credential_process
        .as_deref()
        .filter(|command| !command.trim().is_empty())
    {
        let spec = CredentialProcessSpec::inherit(command);
        if let Ok(output) = input.process.run(&spec).await {
            return Ok((
                Some(ResolvedAuth::aws_sigv4(
                    credentials_from_process(output, region.clone()),
                    source_provenance(CredentialSource::CredentialProcess),
                )),
                Some(region),
            ));
        }
        // Every credential_process failure is continue-eligible in the fixed
        // Node corpus.
    }

    // SSO from the profile (after process in the fixed INI order). Any SSO
    // field present makes the selection selected: an incomplete
    // configuration is terminal, never "unconfigured".
    if let Some(config) = build_sso_config(
        input,
        merged.sso_session.as_deref(),
        merged.sso_start_url.as_deref(),
        merged.sso_region.as_deref(),
        merged.sso_account_id.as_deref(),
        merged.sso_role_name.as_deref(),
    )
    .await
    .map_err(|()| CredentialResolutionError::Terminal {
        credential_source: CredentialSource::Sso {
            profile: Some(profile_name.to_owned()),
        },
        reason: TerminalReason::IncompleteSsoConfiguration,
    })? && input.source_cache.is_some()
    {
        let (credentials, _sso_region) =
            resolve_sso_source(input, &config, Some(profile_name)).await?;
        return Ok((
            Some(ResolvedAuth::aws_sigv4(
                credentials,
                source_provenance(CredentialSource::Sso {
                    profile: Some(profile_name.to_owned()),
                }),
            )),
            Some(region),
        ));
    }

    // login_session closes the INI family. A missing key continues; a
    // selected session's bad cache, DPoP failure, and the terminal
    // AccessDenied family stop; an expired-token refresh failure raises the
    // default continue-eligible error.
    if let Some(session) = merged
        .login_session
        .as_deref()
        .filter(|session| !session.trim().is_empty())
        && input.source_cache.is_some()
        && let Some(credentials) =
            resolve_login_session(input, session, profile_name, &region).await?
    {
        return Ok((
            Some(ResolvedAuth::aws_sigv4(
                credentials,
                source_provenance(CredentialSource::LoginSession {
                    session: session.trim().to_owned(),
                }),
            )),
            Some(region),
        ));
    }

    Ok((None, merged.region.clone().filter(|r| !r.trim().is_empty())))
}

/// Read and overlay one profile from both shared files.
async fn merged_profile(input: &AwsLocalChainInput<'_>, profile_name: &str) -> ProfileProperties {
    let credentials_props = match input.credentials_file {
        Some(path) => {
            read_profile_properties(path, profile_name, ProfileFileKind::Credentials).await
        }
        None => None,
    };
    let config_props = match input.config_file {
        Some(path) => read_profile_properties(path, profile_name, ProfileFileKind::Config).await,
        None => None,
    };
    // Per-field overlay: credentials file wins over config file.
    config_props
        .unwrap_or_default()
        .overlay(credentials_props.unwrap_or_default(), true)
}

/// Resolve one nested source profile chain: descend `source_profile` links
/// (each assuming its own role) until a static pair anchors the chain, then
/// fold the STS exchanges from the anchor outward. Cycles and unresolved
/// sources yield `Ok(None)` (continue-eligible, no STS send).
async fn resolve_source_profile_credentials(
    input: &AwsLocalChainInput<'_>,
    source_name: &str,
    region: &str,
    visited: &mut HashSet<String>,
) -> Result<Option<AwsSigV4Credentials>, CredentialResolutionError> {
    let mut merged = merged_profile(input, source_name).await;
    let mut chain: Vec<(String, RoleAssumption)> = Vec::new();
    let mut current = source_name.to_owned();
    loop {
        if !visited.insert(current.clone()) {
            return Ok(None);
        }
        if static_pair_complete(&merged) {
            break;
        }
        let role_arn = merged
            .role_arn
            .as_deref()
            .filter(|value| !value.trim().is_empty());
        let next_source = merged
            .source_profile
            .as_deref()
            .filter(|value| !value.trim().is_empty());
        let credential_source_blank = merged
            .credential_source
            .as_deref()
            .is_none_or(|value| value.trim().is_empty());
        match (role_arn, next_source) {
            (Some(role_arn), Some(next_source)) if credential_source_blank => {
                let profile = current.clone();
                chain.push((
                    profile,
                    RoleAssumption {
                        role_arn: role_arn.to_owned(),
                        role_session_name: merged.role_session_name.clone(),
                        external_id: merged.external_id.clone(),
                        duration_seconds: merged.duration_seconds.clone(),
                        mfa_serial: merged.mfa_serial.clone(),
                    },
                ));
                current = next_source.to_owned();
                merged = merged_profile(input, &current).await;
            }
            _ => return Ok(None),
        }
    }

    // Static anchor.
    let mut credentials = AwsSigV4Credentials {
        access_key_id: SecretString::from(merged.access_key_id.unwrap_or_default()),
        secret_access_key: SecretString::from(merged.secret_access_key.unwrap_or_default()),
        session_token: merged
            .session_token
            .filter(|token| !token.trim().is_empty())
            .map(SecretString::from),
        region: region.to_owned(),
        expires_at: None,
    };
    // Fold exchanges from the anchor outward (discovery order).
    let endpoint = sts_endpoint_url(region, input.sts_endpoint);
    for (profile, params) in chain {
        let token_code = mfa_token_code(input, &profile, &params)?;
        credentials = assume_role(
            input.transport,
            &credentials,
            &endpoint,
            &params,
            token_code.as_deref(),
        )
        .await
        .map_err(|error| CredentialResolutionError::Terminal {
            credential_source: CredentialSource::AssumeRole { profile },
            reason: sts_terminal_reason(error),
        })?
        .into_credentials(region);
    }
    Ok(Some(credentials))
}

fn static_pair_complete(merged: &ProfileProperties) -> bool {
    merged
        .access_key_id
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && merged
            .secret_access_key
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

/// Present the MFA token code for one profile's assumption, or produce the
/// typed terminal reason.
fn mfa_token_code(
    input: &AwsLocalChainInput<'_>,
    profile: &str,
    params: &RoleAssumption,
) -> Result<Option<String>, CredentialResolutionError> {
    match params
        .mfa_serial
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        Some(serial) => match input.mfa {
            Some(presenter) => presenter.token_code(serial).map(Some).map_err(|_| {
                CredentialResolutionError::Terminal {
                    credential_source: CredentialSource::AssumeRole {
                        profile: profile.to_owned(),
                    },
                    reason: TerminalReason::MfaPresentationFailed,
                }
            }),
            None => Err(CredentialResolutionError::Terminal {
                credential_source: CredentialSource::AssumeRole {
                    profile: profile.to_owned(),
                },
                reason: TerminalReason::MfaPresenterMissing,
            }),
        },
        None => Ok(None),
    }
}

/// Run the assume-role exchange for one profile whose source is another
/// profile. Returns `Ok(None)` when the source is unresolved
/// (continue-eligible).
async fn resolve_assume_role_from_source_profile(
    input: &AwsLocalChainInput<'_>,
    profile_name: &str,
    role_arn: &str,
    source_name: &str,
    merged: &ProfileProperties,
    region: &str,
    visited: &mut HashSet<String>,
) -> Result<Option<ResolvedAuth>, CredentialResolutionError> {
    let Some(source_credentials) =
        resolve_source_profile_credentials(input, source_name, region, visited).await?
    else {
        return Ok(None);
    };
    let exchange = run_assume_role(
        input,
        profile_name,
        role_arn,
        merged,
        region,
        &source_credentials,
    )
    .await?;
    Ok(Some(exchange))
}

/// Route assume-role through a `credential_source` provider. Provider
/// failures are continue-eligible (the family advances).
#[allow(clippy::too_many_arguments)]
async fn resolve_assume_role_from_credential_source(
    input: &AwsLocalChainInput<'_>,
    profile_name: &str,
    role_arn: &str,
    kind: &str,
    merged: &ProfileProperties,
    region: &str,
) -> Result<Option<ResolvedAuth>, CredentialResolutionError> {
    let source_credentials = match kind {
        "Environment" => {
            let access = ambient_non_blank(input, ENV_ACCESS_KEY_ID);
            let secret = ambient_non_blank(input, ENV_SECRET_ACCESS_KEY);
            match (access, secret) {
                (Some(access), Some(secret)) => AwsSigV4Credentials {
                    access_key_id: SecretString::from(access),
                    secret_access_key: SecretString::from(secret),
                    session_token: ambient_non_blank(input, ENV_SESSION_TOKEN)
                        .map(SecretString::from),
                    region: region.to_owned(),
                    expires_at: None,
                },
                _ => return Ok(None),
            }
        }
        "EcsContainer" => {
            let inputs = ecs_inputs(input);
            match metadata::resolve_ecs(input.transport, &inputs).await {
                Ok((credentials, _legacy)) => credentials.into_credentials(region),
                Err(_) => return Ok(None),
            }
        }
        "Ec2InstanceMetadata" => {
            let inputs = imds_inputs(input);
            match metadata::resolve_imds(input.transport, &inputs).await {
                Ok(credentials) => credentials.into_credentials(region),
                Err(_) => return Ok(None),
            }
        }
        // Unsupported credential_source values are continue-eligible.
        _ => return Ok(None),
    };
    let exchange = run_assume_role(
        input,
        profile_name,
        role_arn,
        merged,
        region,
        &source_credentials,
    )
    .await?;
    Ok(Some(exchange))
}

/// Present MFA (when required) and run the signed AssumeRole exchange.
/// Failures are terminal for the selected profile.
async fn run_assume_role(
    input: &AwsLocalChainInput<'_>,
    profile_name: &str,
    role_arn: &str,
    merged: &ProfileProperties,
    region: &str,
    source_credentials: &AwsSigV4Credentials,
) -> Result<ResolvedAuth, CredentialResolutionError> {
    let params = RoleAssumption {
        role_arn: role_arn.to_owned(),
        role_session_name: merged.role_session_name.clone(),
        external_id: merged.external_id.clone(),
        duration_seconds: merged.duration_seconds.clone(),
        mfa_serial: merged.mfa_serial.clone(),
    };
    let token_code = mfa_token_code(input, profile_name, &params)?;
    let endpoint = sts_endpoint_url(region, input.sts_endpoint);
    let outcome = assume_role(
        input.transport,
        source_credentials,
        &endpoint,
        &params,
        token_code.as_deref(),
    )
    .await;
    match outcome {
        Ok(exchange) => Ok(sts_exchange_auth(
            exchange,
            region,
            CredentialSource::AssumeRole {
                profile: profile_name.to_owned(),
            },
        )),
        Err(error) => Err(CredentialResolutionError::Terminal {
            credential_source: CredentialSource::AssumeRole {
                profile: profile_name.to_owned(),
            },
            reason: sts_terminal_reason(error),
        }),
    }
}

/// Shared web-identity resolution: read the selected file, exchange
/// anonymously, retain subject/audience/provider and account metadata.
async fn resolve_web_identity(
    input: &AwsLocalChainInput<'_>,
    token_file: &std::path::Path,
    role_arn: &str,
    role_session_name: Option<&str>,
    source: CredentialSource,
    region: &str,
) -> Result<ResolvedAuth, CredentialResolutionError> {
    let token = read_web_identity_token(token_file).await.map_err(|_| {
        CredentialResolutionError::Terminal {
            credential_source: source.clone(),
            reason: TerminalReason::TokenFileUnreadable,
        }
    })?;
    let endpoint = sts_endpoint_url(region, input.sts_endpoint);
    let outcome = assume_role_with_web_identity(
        input.transport,
        &endpoint,
        &token,
        role_arn,
        role_session_name,
        None,
    )
    .await;
    match outcome {
        Ok(exchange) => Ok(sts_exchange_auth(exchange, region, source)),
        Err(error) => Err(CredentialResolutionError::Terminal {
            credential_source: source,
            reason: web_identity_terminal_reason(error),
        }),
    }
}

/// Build the selected auth from one STS exchange (account from the assumed
/// role ARN when parseable).
fn sts_exchange_auth(
    exchange: StsExchange,
    region: &str,
    source: CredentialSource,
) -> ResolvedAuth {
    let account_id = exchange
        .assumed_role_user_arn
        .as_deref()
        .and_then(account_from_arn);
    ResolvedAuth {
        scheme: AuthScheme::AwsSigV4(exchange.into_credentials(region)),
        secret: SecretString::from(String::new()),
        base_url: None,
        account_id,
        provenance: source_provenance(source),
    }
}

/// Extract the account id from an ARN
/// (`arn:partition:service:region:account:resource`; the STS region slot is
/// empty).
fn account_from_arn(arn: &str) -> Option<String> {
    let mut parts = arn.split(':');
    let _arn = parts.next()?;
    let _partition = parts.next()?;
    let _service = parts.next()?;
    let _region = parts.next()?;
    let account = parts.next()?.trim();
    (!account.is_empty()).then(|| account.to_owned())
}

fn sts_terminal_reason(error: StsExchangeError) -> TerminalReason {
    match error {
        StsExchangeError::Transport => TerminalReason::StsTransport,
        StsExchangeError::Denied(crate::auth::aws::role::StsDenial {
            status,
            code,
            request_id,
        }) => TerminalReason::StsDenied {
            status: Some(status),
            code,
            request_id,
        },
        StsExchangeError::InvalidResponse => TerminalReason::InvalidStsResponse,
    }
}

fn web_identity_terminal_reason(error: WebIdentityError) -> TerminalReason {
    match error {
        WebIdentityError::TokenFileUnreadable => TerminalReason::TokenFileUnreadable,
        WebIdentityError::StsTransport => TerminalReason::StsTransport,
        WebIdentityError::StsDenied(crate::auth::aws::role::StsDenial {
            status,
            code,
            request_id,
        }) => TerminalReason::StsDenied {
            status: Some(status),
            code,
            request_id,
        },
        WebIdentityError::StsInvalid => TerminalReason::InvalidStsResponse,
    }
}

/// Chain segment 5: environment-driven web identity.
async fn resolve_environment_web_identity(
    input: &AwsLocalChainInput<'_>,
    profile_region: Option<&str>,
) -> Result<Option<ResolvedAuth>, CredentialResolutionError> {
    let token_file = input
        .scoped
        .web_identity_token_file
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .or_else(|| ambient_non_blank(input, ENV_WEB_IDENTITY_TOKEN_FILE));
    let role_arn = input
        .scoped
        .role_arn
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .or_else(|| ambient_non_blank(input, ENV_ROLE_ARN));
    let (Some(token_file), Some(role_arn)) = (token_file, role_arn) else {
        // Missing file or role configuration is continue-eligible.
        return Ok(None);
    };
    let session_name = input
        .scoped
        .role_session_name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .or_else(|| ambient_non_blank(input, ENV_ROLE_SESSION_NAME));
    let region = resolve_region(input, profile_region).await;
    let auth = resolve_web_identity(
        input,
        std::path::Path::new(&token_file),
        &role_arn,
        session_name.as_deref(),
        CredentialSource::WebIdentity {
            route: WebIdentityRoute::Environment,
        },
        &region,
    )
    .await?;
    Ok(Some(auth))
}

/// Chain segment 6: ECS (modern then legacy) or IMDS. A selected ECS
/// segment never falls through to IMDS.
async fn resolve_remote(
    input: &AwsLocalChainInput<'_>,
    profile_region: Option<&str>,
) -> Result<ResolvedAuth, CredentialResolutionError> {
    let region = resolve_region(input, profile_region).await;
    let ecs = ecs_inputs(input);
    if ecs.configured() {
        return match metadata::resolve_ecs(input.transport, &ecs).await {
            Ok((credentials, legacy)) => Ok(ResolvedAuth::aws_sigv4(
                credentials.into_credentials(&region),
                source_provenance(CredentialSource::EcsContainer { legacy }),
            )),
            Err(metadata::EcsError::InvalidEndpoint) => Err(CredentialResolutionError::Terminal {
                credential_source: CredentialSource::EcsContainer { legacy: false },
                reason: TerminalReason::InvalidContainerEndpoint,
            }),
            // ECS exhaustion ends the chain: no IMDS fallthrough.
            Err(metadata::EcsError::Unavailable | metadata::EcsError::InvalidResponse) => {
                Err(CredentialResolutionError::Exhausted)
            }
        };
    }

    // IMDS branch (disabled by env/explicit policy when set).
    if ec2_metadata_disabled(input) {
        return Err(CredentialResolutionError::Exhausted);
    }
    match metadata::resolve_imds(input.transport, &imds_inputs(input)).await {
        Ok(credentials) => Ok(ResolvedAuth::aws_sigv4(
            credentials.into_credentials(&region),
            source_provenance(CredentialSource::Ec2InstanceMetadata),
        )),
        Err(_) => Err(CredentialResolutionError::Exhausted),
    }
}

/// Collect ECS inputs with scoped values shadowing the ambient context.
fn ecs_inputs(input: &AwsLocalChainInput<'_>) -> EcsInputs {
    EcsInputs {
        relative_uri: input
            .scoped
            .container_credentials_relative_uri
            .clone()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| ambient_non_blank(input, ENV_ECS_RELATIVE_URI)),
        full_uri: input
            .scoped
            .container_credentials_full_uri
            .clone()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| ambient_non_blank(input, ENV_ECS_FULL_URI)),
        authorization_token: input
            .scoped
            .container_authorization_token
            .as_ref()
            .map(|token| secrecy::ExposeSecret::expose_secret(token).to_owned())
            .filter(|token| !token.trim().is_empty())
            .or_else(|| ambient_non_blank(input, ENV_ECS_AUTHORIZATION_TOKEN)),
        authorization_token_file: input
            .scoped
            .container_authorization_token_file
            .clone()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| ambient_non_blank(input, ENV_ECS_AUTHORIZATION_TOKEN_FILE)),
    }
}

/// Collect IMDS inputs with scoped values shadowing the ambient context.
fn imds_inputs(input: &AwsLocalChainInput<'_>) -> ImdsInputs {
    ImdsInputs {
        endpoint_override: input
            .scoped
            .metadata_service_endpoint
            .clone()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| ambient_non_blank(input, ENV_METADATA_SERVICE_ENDPOINT)),
        endpoint_mode: input
            .scoped
            .metadata_service_endpoint_mode
            .clone()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| ambient_non_blank(input, ENV_METADATA_SERVICE_ENDPOINT_MODE)),
        v1_disabled: ec2_metadata_disabled(input),
    }
}

fn ec2_metadata_disabled(input: &AwsLocalChainInput<'_>) -> bool {
    input.scoped.ec2_metadata_disabled.unwrap_or_else(|| {
        metadata::parse_metadata_disabled(
            ambient_non_blank(input, ENV_EC2_METADATA_DISABLED).as_deref(),
        )
    })
}

fn credentials_from_process(
    output: CredentialProcessOutput,
    region: String,
) -> AwsSigV4Credentials {
    AwsSigV4Credentials {
        access_key_id: output.access_key_id,
        secret_access_key: output.secret_access_key,
        session_token: output.session_token,
        region,
        expires_at: output.expiration,
    }
}
