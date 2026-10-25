//! Store-owned OAuth resolution shared by SDK routes and product adapters.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{AuthResolver, OAuthCredential, OAuthProvider, ResolvedAuth};
use crate::credential::{BoxAuthFuture, Credential, CredentialStore, CredentialStoreError};
use crate::mutation::{CredentialMutationOp, MutationDecision, MutationError, MutationOutcome};
use crate::provider::{ProviderError, ProviderErrorSummary};

/// A lazy OAuth route bound to one store key and flow. Construction does no I/O.
/// Refresh runs in the store's serialized mutation; its 15-second network bound
/// excludes lock waiting and persistence ACK. A started refresh survives a
/// dropped waiter while the store's runtime lives.
pub struct StoreOAuthResolver {
    store: Arc<dyn CredentialStore>,
    key: String,
    flow: Arc<dyn OAuthProvider>,
    network_timeout: Duration,
}

impl StoreOAuthResolver {
    /// Bind a flow to its provider key in `store`.
    pub fn new(
        store: Arc<dyn CredentialStore>,
        key: impl Into<String>,
        flow: Arc<dyn OAuthProvider>,
    ) -> Self {
        Self {
            store,
            key: key.into(),
            flow,
            network_timeout: Duration::from_secs(15),
        }
    }

    /// Override only the refresh exchange bound, for explicit host policy.
    pub fn with_refresh_timeout(mut self, timeout: Duration) -> Self {
        self.network_timeout = timeout;
        self
    }
}

struct CancelWait(tokio_util::sync::CancellationToken);
impl Drop for CancelWait {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

impl AuthResolver for StoreOAuthResolver {
    fn resolve(&self) -> BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async move {
            let cancel = tokio_util::sync::CancellationToken::new();
            let _waiting = CancelWait(cancel.clone());
            let result = Arc::new(Mutex::new(None));
            let committed = result.clone();
            let flow = self.flow.clone();
            let key = self.key.clone();
            let timeout = self.network_timeout;
            let op = CredentialMutationOp::Update(Box::new(move |view, current| {
                let flow = flow.clone();
                let committed = committed.clone();
                let key = key.clone();
                let cancel = cancel.clone();
                Box::pin(async move {
                    if cancel.is_cancelled() {
                        return Err(MutationError::Operation(ProviderError::Cancelled));
                    }
                    // Identity is selected exactly once, under the same exclusion
                    // as login/logout/replace/refresh, never from an earlier peek.
                    let current = match current {
                        Some(Credential::ApiKey(_)) => {
                            return Err(MutationError::Operation(ProviderError::Config(
                                ProviderErrorSummary::redacted(),
                            )));
                        }
                        Some(credential) => as_oauth(credential).expect("OAuth envelope"),
                        None => {
                            return Err(MutationError::Operation(
                                ProviderError::CredentialNeeded { provider_id: key },
                            ));
                        }
                    };
                    if !current.needs_refresh() {
                        *committed.lock().unwrap() = Some(current);
                        return Ok(MutationDecision::Keep);
                    }
                    // Cancellation after this point stops only the caller's wait.
                    let refreshed = tokio::time::timeout(timeout, flow.refresh(&current))
                        .await
                        .unwrap_or(Err(ProviderError::Timeout));
                    match refreshed {
                        Ok(refreshed) => {
                            *committed.lock().unwrap() = Some(refreshed.clone());
                            Ok(MutationDecision::Write(refreshed.into()))
                        }
                        Err(error) => {
                            // Some cross-process backends may reveal an already committed
                            // same-identity rotation even after our exchange failed.
                            if let Ok(Some(credential)) = view.read_current().await
                                && let Some(fresh) = as_oauth(&credential)
                                && current.account_id.is_some()
                                && fresh.account_id == current.account_id
                                && fresh.client_id == current.client_id
                                && fresh.enterprise_domain == current.enterprise_domain
                                && fresh.base_url == current.base_url
                                && !fresh.needs_refresh()
                            {
                                *committed.lock().unwrap() = Some(fresh);
                                return Ok(MutationDecision::Keep);
                            }
                            Err(MutationError::Operation(error))
                        }
                    }
                })
            }));
            let receipt = self
                .store
                .modify(&self.key, op)
                .await
                .map_err(store_error)?;
            match receipt.outcome {
                MutationOutcome::Written { .. } | MutationOutcome::Unchanged { .. } => result
                    .lock()
                    .unwrap()
                    .take()
                    .ok_or_else(|| ProviderError::CredentialNeeded {
                        provider_id: self.key.clone(),
                    })
                    .and_then(|credential| {
                        if credential
                            .expires_at
                            .is_some_and(|expiry| expiry <= time::OffsetDateTime::now_utc())
                        {
                            Err(ProviderError::AuthFailed(
                                ProviderErrorSummary::attested_static(
                                    "OAuth access expired before acknowledgement",
                                ),
                            ))
                        } else {
                            Ok(self.flow.to_auth(&credential))
                        }
                    }),
                MutationOutcome::Failed { error } => match &*error {
                    MutationError::Store(error) => Err(store_error(error.clone())),
                    MutationError::Operation(error) => Err(error.clone()),
                },
                MutationOutcome::Unknown { .. } => Err(ProviderError::AuthFailed(
                    ProviderErrorSummary::sanitized("OAuth mutation outcome unknown"),
                )),
                MutationOutcome::Deleted { .. } => Err(ProviderError::CredentialNeeded {
                    provider_id: self.key.clone(),
                }),
            }
        })
    }
}

/// Read the OAuth envelope without refreshing or exposing its secret fields.
pub(crate) fn as_oauth(credential: &Credential) -> Option<OAuthCredential> {
    match credential {
        Credential::OAuthToken {
            access,
            refresh,
            expires_at,
            base_url,
            account_id,
            client_id,
            enterprise_domain,
            available_model_ids,
            scopes,
        } => Some(OAuthCredential {
            access: access.clone(),
            refresh: refresh.clone(),
            expires_at: *expires_at,
            base_url: base_url.clone(),
            account_id: account_id.clone(),
            client_id: client_id.clone(),
            enterprise_domain: enterprise_domain.clone(),
            available_model_ids: available_model_ids.clone(),
            scopes: scopes.clone(),
        }),
        Credential::ApiKey(_) => None,
    }
}

fn store_error(error: CredentialStoreError) -> ProviderError {
    ProviderError::Config(ProviderErrorSummary::from_untrusted(error.to_string()))
}
