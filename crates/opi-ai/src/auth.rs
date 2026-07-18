//! Per-request auth-resolution contracts (Phase 14.2).
//!
//! IO-free types owned by [`crate`]. The concrete resolvers (`AuthSource`,
//! `OAuthProviderRegistry`, `TuiLoginPresenter`) live in `opi-coding-agent`;
//! `opi-ai` defines only the object-safe contracts so providers can resolve
//! auth inside each returned stream without depending on a concrete backend
//! or becoming generic over the resolver.
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
use crate::provider::ProviderError;

// ---------------------------------------------------------------------------
// Resolved auth + resolver
// ---------------------------------------------------------------------------

/// How a concrete provider attaches the secret at its HTTP boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthScheme {
    /// API-key auth (e.g. Anthropic `x-api-key`).
    ApiKey,
    /// Bearer auth (`Authorization: Bearer <token>`).
    Bearer,
}

/// The auth scheme and secret a provider needs to issue one HTTP request.
///
/// Carries only what the provider's HTTP boundary consumes; the secret is a
/// [`SecretString`] exposed only via [`secrecy::ExposeSecret`] at the provider
/// boundary. [`Debug`](std::fmt::Debug) redacts the secret.
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
}

impl std::fmt::Debug for ResolvedAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Manual impl so a future secrecy version that changes SecretString's
        // Debug cannot leak the secret here.
        f.debug_struct("ResolvedAuth")
            .field("scheme", &self.scheme)
            .field("secret", &"<redacted>")
            .field("base_url", &self.base_url)
            .field("account_id", &self.account_id)
            .finish()
    }
}

/// Object-safe per-request auth resolver.
///
/// Each concrete provider holds an `Arc<dyn AuthResolver>` and calls
/// [`resolve`](Self::resolve) inside the stream returned by `Provider::stream`,
/// immediately before the HTTP request. The resolver may read the keychain,
/// perform a locked refresh, or return a baked key — the provider is unaware
/// of the mechanism.
pub trait AuthResolver: Send + Sync {
    /// Resolve the auth for the next request. Returning
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
        let scheme = self.scheme;
        let secret = self.secret.clone();
        Box::pin(async move {
            Ok(ResolvedAuth {
                scheme,
                secret,
                base_url: None,
                account_id: None,
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
}

impl std::fmt::Debug for OAuthCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthCredential")
            .field("access", &"<redacted>")
            .field("refresh", &"<redacted>")
            .field("expires_at", &self.expires_at)
            .field("base_url", &self.base_url)
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
        }
    }
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
/// The production `TuiLoginPresenter` lives in `opi-coding-agent`. Every OAuth
/// flow supports manual paste (headless/SSH/no-browser) via
/// [`await_manual_code`](Self::await_manual_code). RPC/JSON/non-interactive
/// modes do not construct a presenter; their credential-needed handling is a
/// typed diagnostic path, not an unused presenter implementation.
pub trait LoginPresenter: Send + Sync {
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

    /// Await the user pasting the manual code (headless fallback). Returns the
    /// pasted code.
    fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>>;

    /// Notify the user that login succeeded.
    fn notify_success(&self);

    /// Notify the user that login failed.
    fn notify_failure(&self, reason: &str);
}
