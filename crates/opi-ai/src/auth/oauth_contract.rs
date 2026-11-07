//! OAuth credentials, flow and presentation contracts.
//! The existing refresh predicate reads the clock only when called.

use super::credential::{BoxAuthFuture, Credential};
use super::{AuthFallback, AuthProvenance, AuthProvenanceSource, AuthScheme, ResolvedAuth};
use crate::errors::{ProviderError, ProviderErrorSummary};
use secrecy::SecretString;
use time::OffsetDateTime;

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
    /// OAuth client id the token was issued to. Set by flows with dynamic
    /// client registration (OpenAI ChatGPT direct: the callback returns the
    /// issued id and refresh must reuse it); `None` for flows with a fixed
    /// client id. Non-secret.
    pub client_id: Option<String>,
    /// GitHub Enterprise domain the flow authenticated against (GitHub
    /// Copilot; `None` means the public `github.com` deployment). Non-secret.
    pub enterprise_domain: Option<String>,
    /// Model ids the account can use, as discovered from the provider's
    /// models metadata (GitHub Copilot: picker/policy-filtered ids unioned
    /// with the models enabled at login). Non-secret.
    pub available_model_ids: Option<Vec<String>>,
    /// Validated scopes issued with this credential, when the flow reports them.
    pub scopes: Option<Vec<String>>,
}

impl std::fmt::Debug for OAuthCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthCredential")
            .field("access", &"<redacted>")
            .field("refresh", &"<redacted>")
            .field("expires_at", &self.expires_at)
            .field("base_url", &self.base_url)
            .field("account_id", &self.account_id)
            .field("client_id", &self.client_id)
            .field("enterprise_domain", &self.enterprise_domain)
            .field("available_model_ids", &self.available_model_ids)
            .field("scopes", &self.scopes)
            .finish()
    }
}

impl OAuthCredential {
    /// Whether the access token should be refreshed before use: it has no known
    /// expiry, or it expires within the 5-minute refresh skew. The skew is fixed
    /// here so every store-owned resolver applies the same window once.
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
            client_id: o.client_id,
            enterprise_domain: o.enterprise_domain,
            available_model_ids: o.available_model_ids,
            scopes: o.scopes,
        }
    }
}

/// Login methods offered by an OAuth provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthLoginMethod {
    /// Browser authorization-code flow with a loopback callback and a
    /// paste-back fallback.
    Browser,
    /// Device-code flow: the user visits a verification URI and types a
    /// public code; no paste-back happens on this machine.
    DeviceCode,
    /// Copy-code flow: the provider's web page shows a one-time code the
    /// user pastes back (optionally `code#state`); no loopback callback.
    CopyCode,
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

    /// Display name of this authentication method. Reading metadata performs
    /// no authentication or source I/O. Custom implementations default to id.
    fn name(&self) -> &str {
        self.id()
    }

    /// Whether this method is backed by a provider subscription. `None`
    /// preserves an unspecified source value rather than guessing `false`.
    fn is_subscription(&self) -> Option<bool> {
        None
    }

    /// Optional provider-supplied label for selecting this login method.
    fn login_label(&self) -> Option<&str> {
        None
    }

    /// Convert this flow's acknowledged credential to its request identity.
    /// Provider-specific endpoint derivation belongs to the flow; no I/O occurs.
    fn to_auth(&self, cred: &OAuthCredential) -> ResolvedAuth {
        ResolvedAuth {
            env: Default::default(),
            scheme: AuthScheme::Bearer,
            secret: cred.access.clone(),
            base_url: cred.base_url.clone(),
            account_id: cred.account_id.clone(),
            provenance: AuthProvenance {
                source: AuthProvenanceSource::OAuth {
                    kind: self.id().to_owned(),
                },
                fallback: AuthFallback::NotAttempted,
            },
        }
    }

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

    /// Ask the user one free-text question during login (GitHub Copilot:
    /// the GitHub Enterprise URL/domain, blank for `github.com`).
    ///
    /// The default returns an empty string so presenters without text input
    /// keep each provider's public default (for Copilot, `github.com`); the
    /// flow still type-rejects an invalid non-empty answer. Only public
    /// display text travels through `message`/`placeholder` — never secrets.
    fn prompt_text<'a>(
        &'a self,
        message: &'a str,
        placeholder: &'a str,
    ) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
        Box::pin(async move {
            let _ = (message, placeholder);
            Ok(String::new())
        })
    }

    /// Report non-terminal progress during a login (GitHub Copilot:
    /// "Enabling models..."). Purely informational; the default is a no-op.
    fn notify_progress(&self, message: &str) {
        let _ = message;
    }

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
