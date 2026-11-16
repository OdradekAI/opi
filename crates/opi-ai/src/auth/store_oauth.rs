//! Store-owned OAuth resolution shared by SDK routes and product adapters.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{
    AuthResolver, LoginPresenter, OAuthCredential, OAuthLoginMethod, OAuthProvider, ResolvedAuth,
};
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

/// Typed failure of a store-acknowledged login.
#[derive(Debug)]
pub enum LoginStoreError {
    /// The OAuth flow itself failed.
    Flow(ProviderError),
    /// The store rejected or failed the write (no success was reported).
    Store(CredentialStoreError),
    /// The mutation's outcome is unknown; the write may or may not have
    /// landed. Success is never reported for an unconfirmed outcome.
    Unknown {
        /// Redacted, non-secret loss detail.
        detail: String,
    },
}

/// Presenter wrapper that suppresses the flow's own success notification so
/// success is only presented after the store ACK (V-55: no success before
/// the ACK returns).
struct DeferredSuccessPresenter<'a> {
    inner: &'a dyn LoginPresenter,
}

impl LoginPresenter for DeferredSuccessPresenter<'_> {
    fn select_login_method<'a>(
        &'a self,
        provider_id: &'a str,
        methods: &'a [OAuthLoginMethod],
        default: OAuthLoginMethod,
    ) -> BoxAuthFuture<'a, Result<OAuthLoginMethod, ProviderError>> {
        self.inner
            .select_login_method(provider_id, methods, default)
    }

    fn present_auth_url<'a>(
        &'a self,
        url: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        self.inner.present_auth_url(url)
    }

    fn present_device_code<'a>(
        &'a self,
        user_code: &'a str,
        verification_uri: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        self.inner.present_device_code(user_code, verification_uri)
    }

    fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
        self.inner.await_manual_code()
    }

    fn await_login_cancelled<'a>(&'a self) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        self.inner.await_login_cancelled()
    }

    fn cancel_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        self.inner.cancel_manual_code()
    }

    fn prompt_text<'a>(
        &'a self,
        message: &'a str,
        placeholder: &'a str,
    ) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
        self.inner.prompt_text(message, placeholder)
    }

    fn notify_progress(&self, message: &str) {
        self.inner.notify_progress(message);
    }

    fn notify_success(&self) {
        // Deferred: the SDK reports success only after the store ACK.
    }

    fn notify_failure(&self, reason: &str) {
        self.inner.notify_failure(reason);
    }
}

/// Run one OAuth login and persist the resulting credential through the
/// store-owned serialized mutation queue, reporting success only after the
/// awaited ACK. Login serializes with logout/replace/refresh on the same
/// store; no automatic browser, keychain, environment fallback, or remote
/// revocation happens here.
pub async fn login_to_store(
    provider_id: &str,
    flow: &dyn OAuthProvider,
    store: &dyn CredentialStore,
    presenter: &dyn LoginPresenter,
) -> Result<OAuthCredential, LoginStoreError> {
    let deferred = DeferredSuccessPresenter { inner: presenter };
    let cred = flow.login(&deferred).await.map_err(LoginStoreError::Flow)?;
    // Persist through the store-owned serialized mutation queue; the awaited
    // receipt is the ACK before success is reported.
    let receipt = match store
        .modify(
            provider_id,
            crate::mutation::write_mutation(cred.clone().into()),
        )
        .await
    {
        Ok(receipt) => receipt,
        Err(error) => {
            presenter.notify_failure("credential store write failed");
            return Err(LoginStoreError::Store(error));
        }
    };
    match receipt.outcome {
        MutationOutcome::Written { .. } | MutationOutcome::Unchanged { .. } => {
            presenter.notify_success();
            Ok(cred)
        }
        MutationOutcome::Failed { error } => match &*error {
            MutationError::Store(store_error) => {
                presenter.notify_failure("credential store write failed");
                Err(LoginStoreError::Store(store_error.clone()))
            }
            _ => {
                presenter.notify_failure("credential store write failed");
                Err(LoginStoreError::Store(CredentialStoreError::Backend {
                    provider: provider_id.to_owned(),
                    reason: "credential mutation failed".to_owned(),
                }))
            }
        },
        MutationOutcome::Unknown { detail } => {
            presenter.notify_failure("credential store write outcome unknown");
            Err(LoginStoreError::Unknown { detail })
        }
        MutationOutcome::Deleted { .. } => {
            presenter.notify_failure("credential store write failed");
            Err(LoginStoreError::Store(CredentialStoreError::Backend {
                provider: provider_id.to_owned(),
                reason: "unexpected delete outcome for login".to_owned(),
            }))
        }
    }
}
