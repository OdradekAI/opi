//! Per-request auth-resolution contracts.
//!
//! IO-free types owned by [`crate`]. The concrete resolvers (`AuthSource`,
//! `OAuthProviderRegistry`, `TuiLoginPresenter`) live in `opi-coding-agent`;
//! `opi-ai` defines only the object-safe contracts so `ProviderCollection`
//! can resolve authentication once per prepared logical call and reuse it for
//! sequential retries without depending on a concrete backend or making
//! providers generic over the resolver.
//!
//! All async trait methods return [`BoxAuthFuture`] boxed futures, so
//! `AuthResolver`, `OAuthProvider`, and `LoginPresenter` are usable behind
//! `dyn` without an `async-trait` dependency.
//!
//! # Unstable
//!
//! Part of the **unstable 0.x extension substrate**. Breaking changes may
//! occur between minor versions without a major version bump.

use secrecy::SecretString;
use time::OffsetDateTime;

use crate::credential::{BoxAuthFuture, Credential};
use crate::provider::{ProviderError, ProviderErrorSummary};

// ---------------------------------------------------------------------------
// Resolved auth + resolver
// ---------------------------------------------------------------------------

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
    /// AWS Signature Version 4 using the complete prepared credential bundle.
    AwsSigV4(AwsSigV4Credentials),
}

impl PartialEq for AuthScheme {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::ApiKey, Self::ApiKey)
                | (Self::Bearer, Self::Bearer)
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
    /// Non-secret source classification plus fallback decision. Resolver-owned
    /// source selection is retained; [`crate::ProviderCollection`] supplies the
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
    /// Static credentials came from an AWS shared credentials profile.
    ProfileFile,
    /// Static credentials came from an AWS shared config profile.
    ConfigFile,
    /// An AWS shared config profile's `credential_process` supplied credentials.
    CredentialProcess,
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

/// Object-safe per-logical-call auth resolver.
///
/// [`crate::ProviderCollection::prepare_call`] invokes the selected route's
/// resolver once and freezes the resulting [`ResolvedAuth`] for that prepared
/// logical call. Each permitted sequential retry receives the same prepared
/// auth through [`crate::Provider::stream_prepared`] without invoking the
/// resolver again. The resolver may read the keychain, perform a locked
/// refresh, or return a baked key; providers remain unaware of the source
/// mechanism.
pub trait AuthResolver: Send + Sync {
    /// Resolve auth for one prepared logical call. Returning
    /// [`ProviderError::CredentialNeeded`] signals that no credential is
    /// available and the caller must obtain one (interactive login or a typed
    /// non-interactive diagnostic).
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>>;
}

/// No-IO auth resolver wrapping a fixed secret.
///
/// Existing direct `opi-ai` constructors wrap their fixed key in this resolver
/// so library users retain a small construction path without moving env or
/// keychain access into `opi-ai`.
#[derive(Clone)]
pub struct StaticAuthResolver {
    scheme: AuthScheme,
    secret: SecretString,
}

impl StaticAuthResolver {
    /// Build a resolver that always returns `scheme` + `secret`.
    pub fn new(scheme: AuthScheme, secret: SecretString) -> Self {
        Self { scheme, secret }
    }
}

impl AuthResolver for StaticAuthResolver {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        let scheme = self.scheme.clone();
        let secret = self.secret.clone();
        Box::pin(async move {
            Ok(ResolvedAuth {
                scheme,
                secret,
                base_url: None,
                account_id: None,
                provenance: AuthProvenance::default(),
            })
        })
    }
}

// ---------------------------------------------------------------------------
// OAuth credential + provider + presenter
// ---------------------------------------------------------------------------

/// A refreshable OAuth credential owned by the `OAuthProvider` layer.
///
/// Distinct from the persisted [`crate::Credential::OAuthToken`] envelope: this
/// is the OAuth-provider-facing value produced by `login`/`refresh` and
/// consumed by the per-request resolver. Every secret field is a
/// [`SecretString`]; [`Debug`](std::fmt::Debug) redacts `access` and `refresh`.
#[derive(Clone)]
pub struct OAuthCredential {
    /// Bearer/access token. Redacted in all diagnostics.
    pub access: SecretString,
    /// Refresh token. Redacted in all diagnostics.
    pub refresh: SecretString,
    /// Token expiry, if known.
    pub expires_at: Option<OffsetDateTime>,
    /// Provider-specific base URL preserved across refresh (e.g. Copilot
    /// enterprise). Non-secret.
    pub base_url: Option<String>,
    /// Provider account identity, when required by a concrete wire. Non-secret.
    pub account_id: Option<String>,
}

impl std::fmt::Debug for OAuthCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthCredential")
            .field("access", &"<redacted>")
            .field("refresh", &"<redacted>")
            .field("expires_at", &self.expires_at)
            .field("base_url", &self.base_url)
            .field("account_id", &self.account_id)
            .finish()
    }
}

impl OAuthCredential {
    /// Whether the access token should be refreshed before use: it has no known
    /// expiry, or it expires within the 5-minute refresh skew. The skew is fixed
    /// here so the concrete resolver (`opi-coding-agent`) does not need a `time`
    /// dependency just to compute it.
    pub fn needs_refresh(&self) -> bool {
        match self.expires_at {
            None => true,
            Some(exp) => time::OffsetDateTime::now_utc() + time::Duration::minutes(5) >= exp,
        }
    }
}

impl From<OAuthCredential> for Credential {
    fn from(o: OAuthCredential) -> Self {
        Credential::OAuthToken {
            access: o.access,
            refresh: o.refresh,
            expires_at: o.expires_at,
            base_url: o.base_url,
            account_id: o.account_id,
        }
    }
}

/// Login methods offered by an OAuth provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthLoginMethod {
    Browser,
    DeviceCode,
}

/// Object-safe OAuth provider contract.
///
/// `login` runs an authorization flow (PKCE or device-code) via a
/// [`LoginPresenter`] and returns the resulting credential. `refresh` exchanges
/// a refresh token for a new access token. Both return [`BoxAuthFuture`] so the
/// registry can hold `Arc<dyn OAuthProvider>` heterogeneously. Flow specifics
/// (PKCE callback server vs device-code polling) live inside each
/// implementation's `login`; the trait is flow-agnostic.
pub trait OAuthProvider: Send + Sync {
    /// Provider identifier (for example `anthropic`, `github-copilot`, or
    /// `openai-codex`).
    fn id(&self) -> &str;

    /// Run the authorization flow, returning a fresh credential on success.
    fn login<'a>(
        &'a self,
        presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>>;

    /// Refresh an existing credential, returning a new access token.
    fn refresh<'a>(
        &'a self,
        cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>>;
}

/// Object-safe login UX contract.
///
/// The production `TuiLoginPresenter` lives in `opi-coding-agent`. Browser
/// PKCE flows may race a loopback callback against
/// [`await_manual_code`](Self::await_manual_code). GitHub Copilot and OpenAI
/// Codex Device Code instead call
/// [`present_device_code`](Self::present_device_code), poll their provider,
/// and never await paste-back. RPC/JSON/non-interactive modes do not construct
/// a presenter; their credential-needed handling is a typed diagnostic path.
pub trait LoginPresenter: Send + Sync {
    /// Select one of the provider's supported login methods.
    ///
    /// The object-safe default is deterministic and preserves existing
    /// presenters: it returns `default` when that method is present.
    fn select_login_method<'a>(
        &'a self,
        provider_id: &'a str,
        methods: &'a [OAuthLoginMethod],
        default: OAuthLoginMethod,
    ) -> BoxAuthFuture<'a, Result<OAuthLoginMethod, ProviderError>> {
        Box::pin(async move {
            if methods.contains(&default) {
                Ok(default)
            } else {
                Err(ProviderError::Config(ProviderErrorSummary::sanitized(
                    format!(
                        "OAuth provider '{provider_id}' supplied an invalid default login method"
                    ),
                )))
            }
        })
    }

    /// Present an authorization URL to the user (open browser / display link).
    fn present_auth_url<'a>(&'a self, url: &'a str)
    -> BoxAuthFuture<'a, Result<(), ProviderError>>;

    /// Present the public `user_code` and the verification URI the user must
    /// visit. The `device_code` is a secret (it grants token issuance) and is
    /// never passed to any presenter method; only the public `user_code` the
    /// user types at `verification_uri` is shown. Presenter methods receive
    /// only public display values (`authorize_url`, `user_code`,
    /// `verification_uri`) — never access/refresh tokens, authorization codes,
    /// or device codes.
    fn present_device_code<'a>(
        &'a self,
        user_code: &'a str,
        verification_uri: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>>;

    /// Wait for the user to cancel an active login flow.
    ///
    /// The default stays pending so existing presenters and providers preserve
    /// their behavior. Providers that support active-flow cancellation race
    /// this single-shot future against their authorization flow.
    fn await_login_cancelled<'a>(&'a self) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        Box::pin(std::future::pending())
    }

    /// Await the user pasting a Browser-PKCE manual code or redirect URL.
    ///
    /// Device-code providers must not call this method; they present the public
    /// user code through [`present_device_code`](Self::present_device_code).
    fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>>;

    /// Cancel an abandoned manual-code read and wait until any external input
    /// reader has terminated.
    ///
    /// Presenters whose [`await_manual_code`](Self::await_manual_code) never
    /// spawns an external reader may retain this no-op default.
    fn cancel_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        Box::pin(async { Ok(()) })
    }

    /// Notify the user that login succeeded.
    fn notify_success(&self);

    /// Notify the user that login failed.
    fn notify_failure(&self, reason: &str);
}
