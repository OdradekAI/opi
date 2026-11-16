//! Resolved authentication values and non-secret provenance.

#[cfg(any(feature = "builtins", feature = "provider-protocol-common"))]
use crate::errors::{ProviderError, ProviderErrorSummary};
use secrecy::SecretString;
use time::OffsetDateTime;

/// Secret-bearing AWS credentials prepared for one SigV4 logical call.
///
/// All credential members use [`SecretString`] so clones remain zeroizing and
/// diagnostics cannot reveal access keys, secret keys, or session tokens.
#[derive(Clone)]
pub struct AwsSigV4Credentials {
    /// AWS access key identifier. Treated as credential material and redacted.
    pub access_key_id: SecretString,
    /// AWS secret access key. Redacted in all diagnostics.
    pub secret_access_key: SecretString,
    /// Optional temporary-credential session token. Redacted in diagnostics.
    pub session_token: Option<SecretString>,
    /// Non-secret AWS region used in the SigV4 credential scope.
    pub region: String,
    /// Source-reported credential expiry, when the selected source supplied
    /// one (for example a `credential_process` `Expiration` field). Non-secret;
    /// the resolver records it for lifecycle ownership and never extends it.
    pub expires_at: Option<OffsetDateTime>,
}

impl AwsSigV4Credentials {
    /// Build static (non-expiring) credentials with an explicit region.
    pub fn new(
        access_key_id: SecretString,
        secret_access_key: SecretString,
        region: impl Into<String>,
    ) -> Self {
        Self {
            access_key_id,
            secret_access_key,
            session_token: None,
            region: region.into(),
            expires_at: None,
        }
    }

    /// Attach an optional temporary-credential session token.
    pub fn with_session_token(mut self, token: Option<SecretString>) -> Self {
        self.session_token = token;
        self
    }

    /// Attach the source-reported expiry.
    pub fn with_expires_at(mut self, expires_at: Option<OffsetDateTime>) -> Self {
        self.expires_at = expires_at;
        self
    }
}

impl std::fmt::Debug for AwsSigV4Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AwsSigV4Credentials")
            .field("access_key_id", &"<redacted>")
            .field("secret_access_key", &"<redacted>")
            .field(
                "session_token",
                &self.session_token.as_ref().map(|_| "<redacted>"),
            )
            .field("region", &self.region)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// How a concrete provider attaches prepared authentication at its HTTP boundary.
///
/// Equality is attachment-scheme identity only. In particular, two
/// [`AuthScheme::AwsSigV4`] values compare equal without comparing credential
/// members; secret equality is deliberately not part of this diagnostic and
/// configuration-facing type's contract.
#[derive(Clone)]
pub enum AuthScheme {
    /// API-key auth (e.g. Anthropic `x-api-key`).
    ApiKey,
    /// Bearer auth (`Authorization: Bearer <token>`).
    Bearer,
    /// Selected Anthropic workload identity. The adapter exchanges the
    /// configuration in [`ResolvedAuth::env`] only when issuing a request;
    /// source selection and availability checks perform no token exchange.
    AnthropicFederation,
    /// AWS Signature Version 4 using the complete prepared credential bundle.
    AwsSigV4(AwsSigV4Credentials),
}

impl PartialEq for AuthScheme {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::ApiKey, Self::ApiKey)
                | (Self::Bearer, Self::Bearer)
                | (Self::AnthropicFederation, Self::AnthropicFederation)
                | (Self::AwsSigV4(_), Self::AwsSigV4(_))
        )
    }
}

impl Eq for AuthScheme {}

impl std::fmt::Debug for AuthScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiKey => f.write_str("ApiKey"),
            Self::Bearer => f.write_str("Bearer"),
            Self::AnthropicFederation => f.write_str("AnthropicFederation"),
            Self::AwsSigV4(credentials) => f.debug_tuple("AwsSigV4").field(credentials).finish(),
        }
    }
}

/// Route-level handling for provider 401/403 responses.
///
/// This is intentionally independent of [`AuthScheme`]: Bearer syntax alone
/// does not imply that opi owns a refreshable credential lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthInvalidPolicy {
    /// The route is backed by opi-managed credentials and rejection invalidates
    /// the current credential.
    CredentialManaged,
    /// The route uses a fixed/static credential. Rejection is a bodyless auth
    /// failure and does not enter credential lifecycle handling.
    Static,
}

impl AuthInvalidPolicy {
    #[cfg(any(feature = "builtins", feature = "provider-protocol-common"))]
    pub(crate) fn error(self, provider_id: &str) -> ProviderError {
        match self {
            Self::CredentialManaged => ProviderError::CredentialRevoked {
                provider_id: provider_id.to_owned(),
            },
            Self::Static => {
                ProviderError::AuthFailed(ProviderErrorSummary::authentication_rejected())
            }
        }
    }
}

/// The auth scheme and secret a provider needs to issue one HTTP request.
///
/// Carries only what the provider's HTTP boundary consumes; the secret is a
/// [`SecretString`] exposed only via [`secrecy::ExposeSecret`] at the provider
/// boundary. [`Debug`](std::fmt::Debug) redacts the secret. The non-secret
/// [`AuthProvenance`] is carried beside the secret so callers and evidence can
/// distinguish auth sources without seeing the secret. Resolvers that select
/// among real sources attach their decision directly; the collection fills in
/// the registered route source only when a resolver returns the default.
#[derive(Clone)]
pub struct ResolvedAuth {
    /// How the provider attaches the secret to the HTTP request.
    pub scheme: AuthScheme,
    /// The secret value. Redacted in all diagnostics.
    pub secret: SecretString,
    /// Provider-specific endpoint resolved with the credential.
    pub base_url: Option<String>,
    /// Provider account identity, when required by a concrete wire.
    pub account_id: Option<String>,
    /// Owned provider-scoped configuration selected with this credential.
    /// Values are secret-bearing and never appear in diagnostics. Preparation
    /// consumes this snapshot rather than rereading a potentially changed store.
    pub env: std::collections::BTreeMap<String, SecretString>,
    /// Non-secret source classification plus fallback decision. Resolver-owned
    /// source selection is retained; [`crate::Models`] supplies the
    /// registered route source only for the default value.
    pub provenance: AuthProvenance,
}

impl ResolvedAuth {
    /// Build one prepared AWS SigV4 authentication result.
    ///
    /// SigV4 carries its complete credential bundle in the closed
    /// [`AuthScheme::AwsSigV4`] variant. The single-secret slot is empty and is
    /// consumed only by API-key and Bearer variants.
    pub fn aws_sigv4(credentials: AwsSigV4Credentials, provenance: AuthProvenance) -> Self {
        Self {
            scheme: AuthScheme::AwsSigV4(credentials),
            secret: SecretString::from(""),
            base_url: None,
            account_id: None,
            env: Default::default(),
            provenance,
        }
    }
}

impl std::fmt::Debug for ResolvedAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Manual impl so a future secrecy version that changes SecretString's
        // Debug cannot leak the secret here. Provenance is non-secret and
        // visible (design: only the secret is redacted).
        f.debug_struct("ResolvedAuth")
            .field("scheme", &self.scheme)
            .field("secret", &"<redacted>")
            .field("base_url", &self.base_url)
            .field("account_id", &self.account_id)
            .field("env_keys", &self.env.keys().collect::<Vec<_>>())
            .field("provenance", &self.provenance)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// AuthProvenance — non-secret source classification
// ---------------------------------------------------------------------------

/// Non-secret classification of where a resolved credential originated.
///
/// Carried beside the secret-bearing [`ResolvedAuth`] on a prepared call's
/// redacted route so callers and evidence can distinguish auth sources without
/// ever seeing the secret. No secret value, raw environment value, token, or
/// credential-store payload enters this type: `Environment` names the variable,
/// `CredentialStore` and `OAuth` name non-secret provider/store labels.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuthProvenanceSource {
    /// A static credential baked into provider construction.
    #[default]
    Static,
    /// A caller-supplied per-call credential that replaced every configured
    /// source for one resolution (see
    /// [`crate::Models::get_auth`] overrides). No
    /// environment, store, or resolver access happened for that resolution.
    Explicit,
    /// A credential read from a named environment variable. `name` is the
    /// variable name (e.g. `ANTHROPIC_API_KEY`), never its resolved value.
    Environment {
        /// Non-secret environment variable name.
        name: String,
    },
    /// A credential read from a credential store (e.g. OS keychain). `kind` is
    /// a non-secret store label, never the stored payload.
    CredentialStore {
        /// Non-secret store kind label.
        kind: String,
    },
    /// A credential obtained via an OAuth flow. `kind` is a non-secret provider
    /// label (e.g. `github-copilot`), never a token.
    OAuth {
        /// Non-secret OAuth provider label.
        kind: String,
    },
    /// AWS SigV4 credential-chain source selected for the prepared call.
    AwsSigV4 {
        /// Typed non-secret source reported by the Bedrock resolver.
        source: AwsCredentialSource,
    },
    /// ADC (Google Application Default Credentials) native-chain source
    /// selected for the prepared Vertex call.
    Adc {
        /// Typed non-secret source reported by the ADC resolver.
        source: AdcCredentialSource,
    },
}

/// Non-secret source selected by the local AWS credential resolver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AwsCredentialSource {
    /// The resolver input supplied the complete credential pair directly.
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
    /// Standard AWS environment variables supplied the credential pair.
    Environment,
    /// Scoped (injected per-call/per-credential) environment values supplied
    /// the complete pair. Distinct from ambient [`Self::Environment`]: an
    /// attempted-but-partial scoped pair fails closed and never borrows
    /// ambient halves.
    ScopedEnvironment,
    /// Static credentials came from an AWS shared credentials profile.
    ProfileFile,
    /// Static credentials came from an AWS shared config profile.
    ConfigFile,
    /// An AWS shared config profile's `credential_process` supplied credentials.
    CredentialProcess,
    /// A profile's `role_arn` exchange at STS supplied temporary
    /// credentials signed by a resolved source profile or
    /// `credential_source` provider.
    AssumeRole {
        /// Owning shared-config profile name.
        profile: String,
    },
    /// A web-identity token file exchange at STS supplied temporary
    /// credentials without source SigV4.
    WebIdentity {
        /// Where the selected token file/role came from.
        route: WebIdentityRoute,
    },
    /// The SSO source family (explicit init fields or a shared-config
    /// profile plus its `sso-session` section) supplied credentials.
    Sso {
        /// Owning shared-config profile name; `None` for the explicit
        /// init segment.
        profile: Option<String>,
    },
    /// A login session (`login_session` profile key) supplied credentials
    /// through the SignIn refresh with the owned ES256 DPoP proof.
    LoginSession {
        /// Trimmed session name.
        session: String,
    },
    /// The ECS container metadata provider supplied credentials; `legacy`
    /// distinguishes the one-send legacy provider from the modern fixed
    /// budget provider.
    EcsContainer {
        /// Whether the legacy provider produced the bundle.
        legacy: bool,
    },
    /// The EC2 instance metadata (IMDS) provider supplied credentials.
    Ec2InstanceMetadata,
    /// The explicit no-real-credential proxy-placeholder mode attached fixed
    /// nonsecret placeholder SigV4 credentials on a caller-supplied
    /// nonstandard endpoint. Zero real credential sources were read.
    ProxyPlaceholder,
}

/// Route a web-identity selection took.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebIdentityRoute {
    /// The shared-config profile's `web_identity_token_file`.
    Profile(String),
    /// The environment-driven chain segment
    /// (`AWS_WEB_IDENTITY_TOKEN_FILE`/`AWS_ROLE_ARN`).
    Environment,
}

/// Non-secret source selected by the ADC (Google Application Default
/// Credentials) native chain for a prepared Vertex call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdcCredentialSource {
    /// A `service_account` credentials file exchanged its RS256 JWT
    /// assertion for an OAuth access token at the canonical token endpoint.
    ServiceAccount,
    /// An `authorized_user` credentials file refreshed its token at the
    /// canonical Google token endpoint. `quota_project` carries the file's
    /// non-secret `quota_project_id` (sent as `x-goog-user-project`).
    AuthorizedUser {
        /// Non-secret quota project from the credentials file.
        quota_project: Option<String>,
    },
    /// An `external_account_authorized_user` credentials file refreshed its
    /// token at its validated `token_url` (or the default STS oauthtoken
    /// endpoint) with HTTP Basic client authentication. `rotated` reports
    /// that the exchange returned a replacement refresh token; the owner
    /// replaced it in place and never rewrites the ADC file by default.
    ExternalAccountAuthorizedUser {
        /// Non-secret quota project from the credentials file.
        quota_project: Option<String>,
        /// Whether the latest refresh returned a new refresh token.
        rotated: bool,
    },
    /// The explicitly selected direct metadata source served the token from
    /// the scoped instance metadata endpoint.
    DirectMetadata,
    /// An `external_account` credentials file federated a subject token
    /// through the RFC8693 STS exchange. `supplier` classifies the
    /// credential source; `workforce_user_project` mirrors the file's
    /// non-secret workforce field.
    ExternalAccount {
        /// Credential-source classification (non-secret).
        supplier: ExternalAccountSupplier,
        /// Non-secret `workforce_pool_user_project`, when set.
        workforce_user_project: Option<String>,
    },
    /// An impersonated access token from IAM `generateAccessToken`
    /// (task 20.21.3): either an `impersonated_service_account` file or an
    /// `external_account` file with `service_account_impersonation_url`.
    /// `source` keeps the resolved source identity and `target_principal`
    /// the impersonated one — the two never collapse; `delegates` is the
    /// delegation-chain length, and `quota_project` carries the
    /// impersonated file's non-secret `quota_project_id`.
    Impersonated {
        /// The resolved source credential identity.
        source: Box<AdcCredentialSource>,
        /// The impersonated target service-account email.
        target_principal: String,
        /// Number of delegates in the chain.
        delegates: usize,
        /// Non-secret quota project from the impersonated file.
        quota_project: Option<String>,
    },
}

/// Credential-source classification for the ADC `external_account` family
/// (task 20.21.2). Non-secret provenance only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalAccountSupplier {
    /// File-sourced subject token.
    File,
    /// URL-sourced subject token.
    Url,
    /// Certificate source with an mTLS STS leg.
    Certificate,
    /// AWS (aws1) subject credentials.
    Aws,
    /// Executable (pluggable) supplier.
    Executable,
}

/// Whether auth preparation used an explicitly allowed fallback.
///
/// An environment fallback is permitted only where the reviewed product auth
/// policy allows it; that decision is retained here as a closed, typed value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuthFallback {
    /// No fallback was attempted; the primary source resolved the credential.
    #[default]
    NotAttempted,
    /// An explicitly allowed fallback resolved the credential by moving from
    /// `from` to `to`. `reason` is a stable non-secret diagnostic.
    Used {
        /// The source that was attempted first.
        from: AuthProvenanceSource,
        /// The source that resolved the credential.
        to: AuthProvenanceSource,
        /// Stable non-secret reason for the fallback.
        reason: String,
    },
}

/// Closed, non-secret provenance carried beside resolved authentication.
///
/// Returned by the selected route's resolver during collection-owned call
/// preparation. The secret itself stays in [`ResolvedAuth`] and never enters
/// this value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthProvenance {
    /// Where the credential originated.
    pub source: AuthProvenanceSource,
    /// Whether an allowed fallback was used during preparation.
    pub fallback: AuthFallback,
}
