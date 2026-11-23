//! Authentication and source-resolution ownership for the [`Models`](crate::models::Models) facade.

use std::collections::HashMap;
use std::sync::Arc;

use super::*;
use super::{AuthDescriptor, BuiltinAuthCheck, BuiltinAuthFactory};
use crate::auth::{AuthProvenanceSource, AuthResolver};
use crate::credential::{CredentialSource, CredentialStore};

/// Authentication and source-resolution owner behind [`Models`].
///
/// Resolver maps, OAuth bindings, credential-store state and builtin auth
/// factories share lifecycle and source-ownership rules, so the facade keeps
/// them behind one private owner.
pub(crate) struct AuthOwner {
    pub(crate) auth: HashMap<String, AuthDescriptor>,
    pub(crate) probed: HashMap<String, CredentialSource>,
    pub(crate) resolvers: HashMap<String, Arc<dyn AuthResolver>>,
    pub(crate) sources: HashMap<String, AuthProvenanceSource>,
    pub(crate) oauth_flows: HashMap<String, std::sync::Arc<dyn crate::auth::OAuthProvider>>,
    pub(crate) oauth_routes: std::collections::HashSet<String>,
    #[cfg(feature = "builtins")]
    pub(crate) builtin_default_oauth: std::collections::HashSet<String>,
    pub(crate) builtin_auth_factories: HashMap<String, BuiltinAuthFactory>,
    pub(crate) builtin_auth_checks: HashMap<String, BuiltinAuthCheck>,
    pub(crate) builtin_default_auth: std::collections::HashSet<String>,
    pub(crate) auth_context: std::sync::Arc<dyn crate::auth::AuthContext>,
    pub(crate) credential_store: Option<std::sync::Arc<dyn CredentialStore>>,
    pub(crate) credential_store_explicit: bool,
}

impl AuthOwner {
    pub(crate) fn new() -> Self {
        Self {
            auth: HashMap::new(),
            probed: HashMap::new(),
            resolvers: HashMap::new(),
            sources: HashMap::new(),
            oauth_flows: HashMap::new(),
            oauth_routes: std::collections::HashSet::new(),
            #[cfg(feature = "builtins")]
            builtin_default_oauth: std::collections::HashSet::new(),
            builtin_auth_factories: HashMap::new(),
            builtin_auth_checks: HashMap::new(),
            builtin_default_auth: std::collections::HashSet::new(),
            auth_context: std::sync::Arc::new(crate::auth::AmbientAuthContext),
            credential_store: Some(Arc::new(crate::InMemoryCredentialStore::new())),
            credential_store_explicit: false,
        }
    }
}

impl Models {
    pub(crate) fn route_auth_parts(
        &self,
        provider_id: &str,
    ) -> (Option<Arc<dyn AuthResolver>>, AuthProvenanceSource) {
        let resolver = self.auth_owner.resolvers.get(provider_id).cloned();
        let source = self
            .auth_owner
            .sources
            .get(provider_id)
            .cloned()
            .unwrap_or(AuthProvenanceSource::Static);
        (resolver, source)
    }

    /// The auth descriptor associated with a provider, if any.
    pub fn auth_descriptor(&self, provider_id: &str) -> Option<&AuthDescriptor> {
        self.auth_owner.auth.get(provider_id)
    }

    /// Inject the ambient-source access used by the auth management
    /// operations ([`check_auth`](Self::check_auth),
    /// [`get_auth`](Self::get_auth), [`get_available`](Self::get_available)).
    ///
    /// An injected source **replaces** ambient access entirely: after this
    /// call the collection never consults the process environment or
    /// filesystem through [`AmbientAuthContext`](crate::auth::AmbientAuthContext).
    /// Injecting has no other effect; registration and static lookups perform
    /// zero source access either way.
    pub fn set_auth_context(&mut self, context: std::sync::Arc<dyn crate::auth::AuthContext>) {
        self.auth_owner.auth_context = context;
    }

    /// Replace the default memory store backing store-sourced auth checks,
    /// resolution, and [`login`](Self::login).
    ///
    /// The explicitly selected store is authoritative, including absence and
    /// errors; configuration checks do not fall back to probe metadata. Stored
    /// credentials are not copied from the replaced store. Construction,
    /// registration and static lookup perform no external store access.
    pub fn set_credential_store(&mut self, store: std::sync::Arc<dyn CredentialStore>) {
        for id in &self.auth_owner.oauth_routes {
            if let Some(flow) = self.auth_owner.oauth_flows.get(id) {
                self.auth_owner.resolvers.insert(
                    id.clone(),
                    Arc::new(crate::auth::store_oauth::StoreOAuthResolver::new(
                        store.clone(),
                        id.clone(),
                        flow.clone(),
                    )),
                );
            }
        }
        self.auth_owner.credential_store = Some(store);
        self.auth_owner.credential_store_explicit = true;
        let providers: Vec<_> = self
            .auth_owner
            .builtin_auth_factories
            .keys()
            .cloned()
            .collect();
        for id in providers {
            self.rebind_builtin_auth(&id);
        }
    }

    #[cfg(feature = "builtins")]
    pub(crate) fn bind_builtin_oauth_flow(
        &mut self,
        provider: &str,
        default: impl FnOnce() -> Option<Arc<dyn crate::auth::OAuthProvider>>,
    ) -> Option<Arc<dyn crate::auth::OAuthProvider>> {
        if !self.auth_owner.builtin_default_oauth.contains(provider)
            && let Some(flow) = self.auth_owner.oauth_flows.get(provider).cloned()
        {
            self.register_oauth_flow(provider, flow.clone());
            return Some(flow);
        }
        let flow = default()?;
        self.register_oauth_flow(provider, flow.clone());
        self.auth_owner
            .builtin_default_oauth
            .insert(provider.to_owned());
        Some(flow)
    }

    #[cfg(feature = "builtins")]
    pub(crate) fn register_builtin_auth_factory(
        &mut self,
        provider: &str,
        factory: BuiltinAuthFactory,
    ) {
        self.auth_owner
            .builtin_auth_factories
            .insert(provider.to_owned(), factory);
        self.rebind_builtin_auth(provider);
    }

    #[cfg(feature = "builtins")]
    pub(crate) fn mark_builtin_default_auth(&mut self, provider: &str, only_if_unbound: bool) {
        if !only_if_unbound
            || (!self.auth_owner.resolvers.contains_key(provider)
                && !self.auth_owner.auth.contains_key(provider))
        {
            self.auth_owner
                .builtin_default_auth
                .insert(provider.to_owned());
        }
    }

    #[cfg(feature = "builtins")]
    pub(crate) fn mark_builtin_image_auth(&mut self, provider: &str, api: &ApiId) {
        if let Some(registration) = self
            .operation_owner
            .image_apis
            .get_mut(&(provider.into(), api.clone()))
        {
            registration.builtin_auth = true;
        }
    }

    #[cfg(feature = "builtins")]
    pub(crate) fn mark_builtin_classifier_auth(&mut self, provider: &str, api: &ApiId) {
        if let Some(registration) = self
            .operation_owner
            .classifier_apis
            .get_mut(&(provider.into(), api.clone()))
        {
            registration.builtin_auth = true;
        }
    }

    fn rebind_builtin_auth(&mut self, provider: &str) {
        let Some(factory) = self.auth_owner.builtin_auth_factories.get(provider) else {
            return;
        };
        let bindings = factory(
            self.auth_owner.credential_store.clone(),
            self.auth_owner.oauth_flows.get(provider).cloned(),
        );
        if self.auth_owner.builtin_default_auth.contains(provider) {
            self.auth_owner
                .resolvers
                .insert(provider.to_owned(), bindings.chat.clone());
            self.auth_owner.auth.remove(provider);
            self.auth_owner
                .sources
                .insert(provider.to_owned(), AuthProvenanceSource::Static);
        }
        for registration in self
            .operation_owner
            .image_apis
            .values_mut()
            .filter(|entry| entry.provider_id == provider && entry.builtin_auth)
        {
            registration.resolver = bindings.chat.clone();
        }
        for registration in self
            .operation_owner
            .classifier_apis
            .values_mut()
            .filter(|entry| entry.provider_id == provider && entry.builtin_auth)
        {
            registration.resolver = bindings.classifier.clone();
        }
        self.auth_owner
            .builtin_auth_checks
            .insert(provider.to_owned(), bindings.check);
    }

    /// Register the SDK OAuth flow serving [`login`](Self::login) for one
    /// provider. Registering classifies the provider as store-backed OAuth
    /// for [`check_auth`](Self::check_auth) (`OAuth` kind, no refresh); it
    /// performs no network, store, or environment access.
    pub fn register_oauth_flow(
        &mut self,
        provider_id: &str,
        flow: std::sync::Arc<dyn crate::auth::OAuthProvider>,
    ) {
        #[cfg(feature = "builtins")]
        self.auth_owner.builtin_default_oauth.remove(provider_id);
        if self.auth_owner.builtin_default_auth.contains(provider_id)
            || self.auth_owner.oauth_routes.contains(provider_id)
            || (!self.auth_owner.resolvers.contains_key(provider_id)
                && !self.auth_owner.auth.contains_key(provider_id))
        {
            self.auth_owner.sources.insert(
                provider_id.to_owned(),
                AuthProvenanceSource::OAuth {
                    kind: provider_id.to_owned(),
                },
            );
        }
        self.auth_owner
            .oauth_flows
            .insert(provider_id.to_owned(), flow.clone());
        if self.auth_owner.oauth_routes.contains(provider_id)
            && let Some(store) = &self.auth_owner.credential_store
        {
            self.auth_owner.resolvers.insert(
                provider_id.to_owned(),
                Arc::new(crate::auth::store_oauth::StoreOAuthResolver::new(
                    store.clone(),
                    provider_id,
                    flow,
                )),
            );
        }
        self.rebind_builtin_auth(provider_id);
    }

    /// Bind an SDK OAuth flow to a dispatchable provider and the selected store.
    /// Login, refresh, ACK and request authentication share that store. This
    /// performs no source access. The constructor-owned memory store is used
    /// unless the caller explicitly replaces it.
    pub fn register_oauth_route(
        &mut self,
        provider: Box<dyn Provider>,
        flow: Arc<dyn crate::auth::OAuthProvider>,
        compat: CompatMetadata,
    ) -> Result<(), ModelsError> {
        let id = provider.id().to_owned();
        let store = self.auth_owner.credential_store.clone().ok_or_else(|| {
            ModelsError::CredentialStore(CredentialStoreError::Backend {
                provider: id.clone(),
                reason: "no credential store attached".to_owned(),
            })
        })?;
        let resolver = Arc::new(crate::auth::store_oauth::StoreOAuthResolver::new(
            store,
            id.clone(),
            flow.clone(),
        ));
        self.register_route(
            provider,
            resolver,
            AuthProvenanceSource::OAuth {
                kind: flow.id().to_owned(),
            },
            compat,
        )?;
        self.register_oauth_flow(&id, flow);
        self.auth_owner.oauth_routes.insert(id);
        Ok(())
    }

    /// Run the provider's registered OAuth login and store the resulting
    /// credential through the store-owned serialized mutation queue,
    /// acknowledging exactly that local write: success is reported only
    /// after the awaited store ACK, a failed write surfaces typed, and an
    /// unknown outcome stays honest as [`ModelsError::MutationOutcomeUnknown`].
    ///
    /// No automatic browser launch, keychain read, environment fallback, or
    /// remote revocation happens: interaction goes through `presenter`, HTTP
    /// through the flow's injected client, and the local write through the
    /// selected [`CredentialStore`] (constructor-owned memory by default).
    /// Calling `login` without a registered flow fails typed before interaction.
    pub async fn login(
        &self,
        provider_id: &str,
        presenter: &dyn crate::auth::LoginPresenter,
    ) -> Result<crate::auth::OAuthCredential, ModelsError> {
        let flow = self
            .auth_owner
            .oauth_flows
            .get(provider_id)
            .ok_or_else(|| ModelsError::UnknownOAuthFlow {
                provider: provider_id.to_owned(),
            })?;
        let store = self.auth_owner.credential_store.clone().ok_or_else(|| {
            ModelsError::CredentialStore(CredentialStoreError::Backend {
                provider: provider_id.to_owned(),
                reason: "no credential store attached".to_owned(),
            })
        })?;
        match crate::auth::store_oauth::login_to_store(
            provider_id,
            flow.as_ref(),
            store.as_ref(),
            presenter,
        )
        .await
        {
            Ok(credential) => Ok(credential),
            Err(crate::auth::store_oauth::LoginStoreError::Flow(error)) => {
                Err(ModelsError::AuthResolution {
                    provider: provider_id.to_owned(),
                    error: Box::new(error),
                })
            }
            Err(crate::auth::store_oauth::LoginStoreError::Store(error)) => {
                Err(ModelsError::CredentialStore(error))
            }
            Err(crate::auth::store_oauth::LoginStoreError::Unknown { detail }) => {
                Err(ModelsError::MutationOutcomeUnknown {
                    provider: provider_id.to_owned(),
                    detail,
                })
            }
        }
    }

    /// The ambient-source access this collection consults on demand.
    pub fn auth_context(&self) -> &std::sync::Arc<dyn crate::auth::AuthContext> {
        &self.auth_owner.auth_context
    }

    /// Check whether one provider has complete source configuration, on
    /// demand, without refreshing anything.
    ///
    /// Returns `Ok(Some(check))` when the selected source is configured — an
    /// expired stored OAuth credential is reported configured with kind
    /// [`AuthCheckKind::OAuth`] and no refresh attempt — and `Ok(None)` when
    /// the provider is unknown or unconfigured. Source failures are typed
    /// [`ModelsError`] values (non-unicode env bytes, unreachable credential
    /// backend, store error); they are never folded into `Ok(None)`.
    ///
    /// Descriptor-sourced providers check through the injected
    /// [`AuthContext`](crate::auth::AuthContext) (or the live store for
    /// [`AuthDescriptor::StoreCredential`]). Actual credentials take priority;
    /// only empty constructor-owned memory may fall back to the redacted probe
    /// snapshot injected via [`set_probe`](Self::set_probe). Explicit-store
    /// absence/errors remain authoritative. Route providers first use a declared
    /// configuration checker when present, then their non-secret source
    /// classification. Checking never resolves or refreshes authentication.
    pub async fn check_auth(&self, provider_id: &str) -> Result<Option<AuthCheck>, ModelsError> {
        if self.auth_owner.builtin_default_auth.contains(provider_id)
            && let Some(check) = self.auth_owner.builtin_auth_checks.get(provider_id)
        {
            return check()
                .await
                .map(|status| status.map(|status| status.check))
                .map_err(|error| ModelsError::AuthResolution {
                    provider: provider_id.to_owned(),
                    error: Box::new(error),
                });
        }
        if let Some(descriptor) = self.auth_owner.auth.get(provider_id) {
            return self.check_descriptor(provider_id, descriptor).await;
        }
        if !self.check_route_configuration(provider_id).await? {
            return Ok(None);
        }
        match self.auth_owner.sources.get(provider_id) {
            Some(source) => self.check_source(provider_id, source).await,
            None => Ok(None),
        }
    }

    /// Models of every provider whose source configuration checks out.
    ///
    /// Each entry is `(provider_id, model)` in registry order (sorted provider
    /// ids, provider catalog order within). `provider_id` restricts the check
    /// to one provider. Providers without any auth facts (pure registry
    /// providers registered behind [`from_registry`](Self::from_registry))
    /// are never available through this path.
    /// OAuth presence and account model metadata use one credential snapshot,
    /// so concurrent logout or replacement cannot combine different accounts.
    pub async fn get_available(
        &self,
        provider_id: Option<&str>,
    ) -> Result<Vec<(String, ModelInfo)>, ModelsError> {
        let ids: Vec<String> = self
            .registry
            .registry
            .provider_ids()
            .into_iter()
            .filter(|id| provider_id.is_none_or(|wanted| wanted == *id))
            .map(str::to_owned)
            .collect();
        let mut available = Vec::new();
        let mut account_models = std::collections::HashMap::new();
        let mut responses_only = std::collections::HashSet::new();
        for id in ids {
            if self.auth_owner.builtin_default_auth.contains(&id)
                && let Some(check) = self.auth_owner.builtin_auth_checks.get(&id)
            {
                if let Some(status) =
                    check().await.map_err(|error| ModelsError::AuthResolution {
                        provider: id.clone(),
                        error: Box::new(error),
                    })?
                {
                    if let Some(models) = status.available_model_ids {
                        account_models.insert(id.clone(), models);
                    }
                    if status.responses_only {
                        responses_only.insert(id.clone());
                    }
                    available.push(id);
                }
                continue;
            }
            if !self.auth_owner.auth.contains_key(&id)
                && matches!(
                    self.auth_owner.sources.get(&id),
                    Some(AuthProvenanceSource::OAuth { .. })
                )
                && let Some(store) = &self.auth_owner.credential_store
            {
                if !self.check_route_configuration(&id).await? {
                    continue;
                }
                let Some(credential) = store.read(&id).await? else {
                    continue;
                };
                if let Credential::OAuthToken {
                    available_model_ids: Some(ids),
                    ..
                } = credential
                {
                    account_models.insert(id.clone(), ids);
                }
                available.push(id);
            } else if self.check_auth(&id).await?.is_some() {
                available.push(id);
            }
        }
        let mut entries = Vec::new();
        for (provider, model) in self.registry.registry.all_models() {
            let allowed_after_auth = self
                .registry
                .registry
                .get_provider_arc(provider)
                .is_none_or(|route| {
                    route.model_allowed_after_auth(model, responses_only.contains(provider))
                });
            if available.iter().any(|id| id == provider)
                && allowed_after_auth
                && account_models
                    .get(provider)
                    .is_none_or(|ids| ids.iter().any(|id| id == &model.id))
            {
                entries.push((provider.to_owned(), model.clone()));
            }
        }
        Ok(entries)
    }

    async fn check_route_configuration(&self, provider_id: &str) -> Result<bool, ModelsError> {
        let Some(check) = self
            .auth_owner
            .resolvers
            .get(provider_id)
            .and_then(|resolver| resolver.check_configuration())
        else {
            return Ok(true);
        };
        check.await.map_err(|error| ModelsError::AuthResolution {
            provider: provider_id.to_owned(),
            error: Box::new(error),
        })
    }

    /// Resolve provider auth on demand for management/status use.
    ///
    /// An explicit [`AuthResolutionOverrides::api_key`] wins over every
    /// configured source: no environment, store, or resolver access happens.
    /// Otherwise the selected source resolves: a registered route invokes its
    /// live [`AuthResolver`] exactly once, and a descriptor-sourced provider
    /// resolves through the injected [`AuthContext`](crate::auth::AuthContext)
    /// or attached store. `Ok(None)` means unknown or unconfigured (a
    /// [`ProviderError::CredentialNeeded`] from a route maps here); any other
    /// selected-source failure propagates as [`ModelsError::AuthResolution`]
    /// and **no fallback source is tried**. Stored OAuth credentials resolve
    /// without refresh here; refresh belongs to the live route/flow owners.
    pub async fn get_auth(
        &self,
        provider_id: &str,
        overrides: AuthResolutionOverrides,
    ) -> Result<Option<ResolvedAuth>, ModelsError> {
        let explicit_choices = [
            overrides.api_key.is_some(),
            overrides.bearer.is_some(),
            overrides.aws_sigv4.is_some(),
        ]
        .iter()
        .filter(|set| **set)
        .count();
        if explicit_choices > 1 {
            return Err(ModelsError::ConflictingAuthOverrides {
                provider: provider_id.to_owned(),
            });
        }
        if let Some(api_key) = overrides.api_key {
            return Ok(Some(ResolvedAuth {
                env: Default::default(),
                scheme: AuthScheme::ApiKey,
                secret: api_key,
                base_url: None,
                account_id: None,
                provenance: AuthProvenance {
                    source: AuthProvenanceSource::Explicit,
                    fallback: AuthFallback::NotAttempted,
                },
            }));
        }
        if let Some(bearer) = overrides.bearer {
            return Ok(Some(ResolvedAuth {
                env: Default::default(),
                scheme: AuthScheme::Bearer,
                secret: bearer,
                base_url: None,
                account_id: None,
                provenance: AuthProvenance {
                    source: AuthProvenanceSource::Explicit,
                    fallback: AuthFallback::NotAttempted,
                },
            }));
        }
        if let Some(credentials) = overrides.aws_sigv4 {
            return Ok(Some(ResolvedAuth::aws_sigv4(
                credentials,
                AuthProvenance {
                    source: AuthProvenanceSource::Explicit,
                    fallback: AuthFallback::NotAttempted,
                },
            )));
        }
        if let Some(resolver) = self.auth_owner.resolvers.get(provider_id) {
            return match resolver.resolve().await {
                Ok(resolved) => Ok(Some(resolved)),
                Err(ProviderError::CredentialNeeded { .. }) => Ok(None),
                Err(error) => Err(ModelsError::AuthResolution {
                    provider: provider_id.to_owned(),
                    error: Box::new(error),
                }),
            };
        }
        let Some(descriptor) = self.auth_owner.auth.get(provider_id) else {
            return Ok(None);
        };
        match descriptor {
            AuthDescriptor::StaticApiKey { value } => {
                Ok(value.is_present().then(|| ResolvedAuth {
                    env: Default::default(),
                    scheme: AuthScheme::ApiKey,
                    secret: secrecy::SecretString::from(value.as_str().to_owned()),
                    base_url: None,
                    account_id: None,
                    provenance: AuthProvenance::default(),
                }))
            }
            AuthDescriptor::EnvApiKey { env_var } => {
                match self.auth_owner.auth_context.env(env_var)? {
                    Some(value) => Ok(Some(ResolvedAuth {
                        env: Default::default(),
                        scheme: AuthScheme::ApiKey,
                        secret: secrecy::SecretString::from(value),
                        base_url: None,
                        account_id: None,
                        provenance: AuthProvenance {
                            source: AuthProvenanceSource::Environment {
                                name: env_var.clone(),
                            },
                            fallback: AuthFallback::NotAttempted,
                        },
                    })),
                    None => Ok(None),
                }
            }
            // Label-only descriptor: the secret is owned by a route/flow that
            // is not part of this collection.
            AuthDescriptor::Resolved { .. } => Ok(None),
            AuthDescriptor::StoreCredential { key, .. } => {
                self.resolve_from_store(provider_id, key.as_str()).await
            }
        }
    }

    /// Delete the stored credential for one provider, acknowledging only that
    /// local deletion.
    ///
    /// The acknowledgement means exactly "the given store confirmed the entry
    /// is gone": no environment variable is unset, no remote session is
    /// revoked, and no other provider's entry is touched. Deleting an absent
    /// entry acknowledges successfully. Store failures propagate as typed
    /// [`ModelsError::CredentialStore`] errors.
    /// Logout `provider_id`: submit one serialized store-owned deletion and
    /// acknowledge exactly that local mutation. No environment variable is
    /// unset, no remote session is revoked, and other providers' entries are
    /// untouched. A store failure propagates typed; a lost mutation window
    /// surfaces as [`ModelsError::MutationOutcomeUnknown`].
    pub async fn logout(
        &self,
        store: &dyn CredentialStore,
        provider_id: &str,
    ) -> Result<(), ModelsError> {
        let receipt = store
            .modify(provider_id, crate::mutation::delete_mutation())
            .await?;
        match receipt.outcome {
            MutationOutcome::Deleted { .. } => Ok(()),
            MutationOutcome::Failed { error } => match &*error {
                MutationError::Store(store_error) => {
                    Err(ModelsError::CredentialStore(store_error.clone()))
                }
                MutationError::Operation(_) => Err(ModelsError::CredentialStore(
                    CredentialStoreError::Backend {
                        provider: provider_id.to_owned(),
                        reason: "delete mutation failed".to_owned(),
                    },
                )),
            },
            MutationOutcome::Unknown { detail } => Err(ModelsError::MutationOutcomeUnknown {
                provider: provider_id.to_owned(),
                detail,
            }),
            MutationOutcome::Written { .. } | MutationOutcome::Unchanged { .. } => Err(
                ModelsError::CredentialStore(CredentialStoreError::Backend {
                    provider: provider_id.to_owned(),
                    reason: "unexpected delete mutation outcome".to_owned(),
                }),
            ),
        }
    }

    /// Synchronously capture the selected store's mutation cutoff. Every
    /// `Models` instance sharing the same store observes the same
    /// generations. Constructors provide an independent local memory barrier;
    /// explicitly sharing a store also shares its cutoff and receipts.
    pub fn mutation_barrier(&self) -> Option<crate::mutation::MutationBarrier> {
        self.auth_owner
            .credential_store
            .as_ref()
            .map(|store| store.mutation_barrier())
    }

    /// Descriptor-path source check for [`check_auth`].
    async fn check_descriptor(
        &self,
        provider_id: &str,
        descriptor: &AuthDescriptor,
    ) -> Result<Option<AuthCheck>, ModelsError> {
        match descriptor {
            AuthDescriptor::StaticApiKey { value } => Ok(value.is_present().then(|| AuthCheck {
                source: Some("static api key".to_owned()),
                kind: AuthCheckKind::ApiKey,
            })),
            AuthDescriptor::EnvApiKey { env_var } => Ok(self
                .auth_owner
                .auth_context
                .env(env_var)?
                .map(|_| AuthCheck {
                    source: Some(env_var.clone()),
                    kind: AuthCheckKind::ApiKey,
                })),
            AuthDescriptor::Resolved { source } => {
                Ok((!source.trim().is_empty()).then(|| AuthCheck {
                    source: Some(source.clone()),
                    kind: AuthCheckKind::ApiKey,
                }))
            }
            AuthDescriptor::StoreCredential { key, .. } => {
                self.check_store_credential(provider_id, key, AuthCheckKind::ApiKey)
                    .await
            }
        }
    }

    /// Route-path source check for [`check_auth`].
    async fn check_source(
        &self,
        provider_id: &str,
        source: &AuthProvenanceSource,
    ) -> Result<Option<AuthCheck>, ModelsError> {
        match source {
            AuthProvenanceSource::Static => Ok(Some(AuthCheck {
                source: Some("static".to_owned()),
                kind: AuthCheckKind::ApiKey,
            })),
            AuthProvenanceSource::Environment { name } => {
                Ok(self.auth_owner.auth_context.env(name)?.map(|_| AuthCheck {
                    source: Some(name.clone()),
                    kind: AuthCheckKind::ApiKey,
                }))
            }
            // A route's provenance label names the store kind, not the store
            // account key; store-backed routes key entries by provider id.
            AuthProvenanceSource::CredentialStore { .. } | AuthProvenanceSource::OAuth { .. } => {
                let kind = if matches!(source, AuthProvenanceSource::OAuth { .. }) {
                    AuthCheckKind::OAuth
                } else {
                    AuthCheckKind::ApiKey
                };
                self.check_store_credential(provider_id, provider_id, kind)
                    .await
            }
            AuthProvenanceSource::AwsSigV4 { .. } => Ok(Some(AuthCheck {
                source: Some("aws credential chain".to_owned()),
                kind: AuthCheckKind::ApiKey,
            })),
            AuthProvenanceSource::Adc { .. } => Ok(Some(AuthCheck {
                source: Some("google application default credentials".to_owned()),
                kind: AuthCheckKind::ApiKey,
            })),
            AuthProvenanceSource::Explicit => Ok(Some(AuthCheck {
                source: Some("explicit per-call credential".to_owned()),
                kind: AuthCheckKind::ApiKey,
            })),
        }
    }

    /// Store-backed check: a live store read classifies the credential kind
    /// (an expired OAuth credential still reports configured, never
    /// refreshed). Only empty constructor-owned memory may consult injected
    /// probe metadata; explicit-store absence/errors remain authoritative.
    /// Backend-unavailable snapshots surface as typed errors.
    async fn check_store_credential(
        &self,
        provider_id: &str,
        key: &str,
        fallback_kind: AuthCheckKind,
    ) -> Result<Option<AuthCheck>, ModelsError> {
        if let Some(store) = self.auth_owner.credential_store.as_ref() {
            match store.read(key).await? {
                Some(Credential::ApiKey(_)) => {
                    return Ok(Some(AuthCheck {
                        source: Some("credential store".to_owned()),
                        kind: AuthCheckKind::ApiKey,
                    }));
                }
                Some(Credential::OAuthToken { .. }) => {
                    return Ok(Some(AuthCheck {
                        source: Some("OAuth".to_owned()),
                        kind: AuthCheckKind::OAuth,
                    }));
                }
                None if self.auth_owner.credential_store_explicit => return Ok(None),
                None => {}
            }
        }
        match self.auth_owner.probed.get(provider_id) {
            Some(CredentialSource::Present { label }) => Ok(Some(AuthCheck {
                source: Some(label.clone()),
                kind: fallback_kind,
            })),
            Some(CredentialSource::Absent) | None => Ok(None),
            Some(CredentialSource::BackendUnavailable { reason }) => Err(
                ModelsError::CredentialStore(CredentialStoreError::BackendUnavailable {
                    provider: provider_id.to_owned(),
                    reason: reason.clone(),
                }),
            ),
        }
    }

    /// Store-backed resolution for descriptor-sourced providers: returns the
    /// stored credential without refresh (refresh is route/flow-owned).
    async fn resolve_from_store(
        &self,
        _provider_id: &str,
        key: &str,
    ) -> Result<Option<ResolvedAuth>, ModelsError> {
        let Some(store) = self.auth_owner.credential_store.as_ref() else {
            return Ok(None);
        };
        match store.read(key).await? {
            Some(Credential::ApiKey(value)) => Ok(Some(ResolvedAuth {
                env: value.env().clone(),
                scheme: AuthScheme::ApiKey,
                secret: value
                    .key()
                    .cloned()
                    .ok_or_else(|| ProviderError::CredentialNeeded {
                        provider_id: _provider_id.to_owned(),
                    })?,
                base_url: None,
                account_id: None,
                provenance: AuthProvenance {
                    source: AuthProvenanceSource::CredentialStore {
                        kind: "credential store".to_owned(),
                    },
                    fallback: AuthFallback::NotAttempted,
                },
            })),
            Some(Credential::OAuthToken {
                access,
                base_url,
                account_id,
                ..
            }) => Ok(Some(ResolvedAuth {
                env: Default::default(),
                scheme: AuthScheme::Bearer,
                secret: access,
                base_url,
                account_id,
                provenance: AuthProvenance {
                    source: AuthProvenanceSource::CredentialStore {
                        kind: "credential store".to_owned(),
                    },
                    fallback: AuthFallback::NotAttempted,
                },
            })),
            None => Ok(None),
        }
    }

    /// Inject the redacted probe state for a [`AuthDescriptor::StoreCredential`]
    /// provider. The async outer command path (doctor / `--list-models`) probes
    /// the store and passes the resulting [`CredentialSource`] here so the
    /// secret-free descriptor does not have to perform IO.
    pub fn set_probe(&mut self, provider_id: &str, source: CredentialSource) {
        self.auth_owner
            .probed
            .insert(provider_id.to_owned(), source);
    }

    /// The redacted probe state previously injected for a StoreCredential
    /// provider, if any.
    pub fn probe(&self, provider_id: &str) -> Option<&CredentialSource> {
        self.auth_owner.probed.get(provider_id)
    }
}
