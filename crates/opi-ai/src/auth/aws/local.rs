//! Local AWS credential-source chain for Bedrock.
//!
//! One coordinator resolves one selected-source identity per call from the
//! completed local families: explicit typed selections, scoped/ambient
//! environment pairs and bearer tokens, the shared-config INI static
//! overlay, and `credential_process`. Each step yields an exact
//! Continue/Selected/Terminal outcome per the verified locked-Node source
//! corpus:
//!
//! - **Selected** — the step resolved a complete credential bundle; no lower
//!   source runs.
//! - **Continue** — the source was unconfigured or failed a continue-eligible
//!   discovery check (missing ambient pair, absent profile, unreadable or
//!   junk profile file, missing/blank or failed `credential_process`). The
//!   chain advances; it never borrows another family's halves.
//! - **Terminal** — a selected/scoped typed source is incomplete, or the
//!   local families are exhausted. These fail closed as
//!   [`CredentialResolutionError`](self).
//!
//! Source order (local slice of the full chain): explicit typed selection →
//! bearer token (scoped then ambient) → environment pair (skipped when an
//! explicit/scoped profile governs) → INI static overlay (per-field
//! credentials/config merge) → `credential_process`. STS/web-identity/ECS/
//! IMDS/SSO families are later slices; their absence is reported as typed
//! exhaustion, never silently substituted.
//!
//! # Unstable
//!
//! Part of the **unstable 0.x extension substrate**. Breaking changes may
//! occur between minor versions without a major version bump.

use std::path::Path;

use secrecy::SecretString;

use crate::auth::aws::process::{
    CredentialProcessOutput, CredentialProcessRunner, CredentialProcessSpec,
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
}

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
    /// Explicit region option (highest region precedence).
    pub region: Option<&'a str>,
    /// Region of last resort when no source named one.
    pub default_region: &'a str,
}

pub(crate) const ENV_ACCESS_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
pub(crate) const ENV_SECRET_ACCESS_KEY: &str = "AWS_SECRET_ACCESS_KEY";
pub(crate) const ENV_SESSION_TOKEN: &str = "AWS_SESSION_TOKEN";
pub(crate) const ENV_PROFILE: &str = "AWS_PROFILE";
pub(crate) const ENV_REGION: &str = "AWS_REGION";
pub(crate) const ENV_DEFAULT_REGION: &str = "AWS_DEFAULT_REGION";
pub(crate) const ENV_BEARER_TOKEN_BEDROCK: &str = "AWS_BEARER_TOKEN_BEDROCK";

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

    resolve_ini_profile(input, &profile_name).await
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
            _ => {}
        }
    }

    (!props.is_empty()).then_some(props)
}

/// INI stage: merged static credentials, then the profile's
/// `credential_process`.
///
/// A partial merged static pair is continue-eligible (the fixed Node
/// `resolveProfileData` order falls through to the later families), as is
/// any `credential_process` failure or absence. Exhaustion of the local
/// families is the typed terminal outcome.
async fn resolve_ini_profile(
    input: &AwsLocalChainInput<'_>,
    profile_name: &str,
) -> Result<ResolvedAuth, CredentialResolutionError> {
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
    let merged = config_props
        .unwrap_or_default()
        .overlay(credentials_props.unwrap_or_default(), true);

    let region = resolve_region(input, merged.region.as_deref()).await;

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
            access_key_id: SecretString::from(merged.access_key_id.unwrap_or_default()),
            secret_access_key: SecretString::from(merged.secret_access_key.unwrap_or_default()),
            session_token: merged
                .session_token
                .filter(|token| !token.trim().is_empty())
                .map(SecretString::from),
            region,
            expires_at: None,
        };
        return Ok(ResolvedAuth::aws_sigv4(
            credentials,
            source_provenance(source),
        ));
    }

    if let Some(command) = merged
        .credential_process
        .as_deref()
        .filter(|command| !command.trim().is_empty())
    {
        let spec = CredentialProcessSpec::inherit(command);
        match input.process.run(&spec).await {
            Ok(output) => {
                return Ok(ResolvedAuth::aws_sigv4(
                    credentials_from_process(output, region),
                    source_provenance(CredentialSource::CredentialProcess),
                ));
            }
            // Every credential_process failure is continue-eligible in the
            // fixed Node corpus; the local slice ends here.
            Err(_) => return Err(CredentialResolutionError::Exhausted),
        }
    }

    Err(CredentialResolutionError::Exhausted)
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
