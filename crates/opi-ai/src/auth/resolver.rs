//! Per-call authentication resolution contract and fixed-value adapter.

use super::credential::BoxAuthFuture;
use super::{AuthProvenance, AuthScheme, ResolvedAuth};
use crate::errors::ProviderError;
use secrecy::SecretString;

/// Object-safe per-logical-call auth resolver.
///
/// [`crate::Models::prepare_call`] invokes the selected route's
/// resolver once and freezes the resulting [`ResolvedAuth`] for that prepared
/// logical call. Each permitted sequential retry receives the same prepared
/// auth through [`crate::Provider::stream_prepared`] without invoking the
/// resolver again. The resolver may read the keychain, perform a locked
/// refresh, or return a baked key; providers remain unaware of the source
/// mechanism.
pub trait AuthResolver: Send + Sync {
    /// Check this source's configuration without resolving or refreshing it.
    ///
    /// Sources may inspect configuration, store metadata or file existence;
    /// this must not exchange tokens or parse credential-file contents. `None`
    /// keeps an opaque resolver's registered source-classification check.
    fn check_configuration(&self) -> Option<BoxAuthFuture<'_, Result<bool, ProviderError>>> {
        None
    }

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
                env: Default::default(),
                scheme,
                secret,
                base_url: None,
                account_id: None,
                provenance: AuthProvenance::default(),
            })
        })
    }
}
