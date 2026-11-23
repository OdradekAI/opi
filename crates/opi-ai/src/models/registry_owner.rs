//! Registry and session-resource ownership for the [`Models`](crate::models::Models) facade.

use std::collections::HashMap;
use std::sync::Arc;

use super::ProviderRegistry;
use super::*;

/// Registry and resource-lifetime owner behind the public [`Models`] facade.
///
/// Provider lookup, registration serialization and session-resource
/// namespaces change together, so they stay behind one private owner instead
/// of being scattered across the facade's auth and catalog state.
pub(crate) struct RegistryOwner {
    pub(crate) registry: ProviderRegistry,
    pub(crate) resource_owners:
        HashMap<String, Arc<dyn crate::codex_resources::SessionResourceOwner>>,
    pub(crate) all_resource_owners:
        Arc<std::sync::Mutex<Vec<Arc<dyn crate::codex_resources::SessionResourceOwner>>>>,
    pub(crate) registration_gate: Arc<std::sync::Mutex<()>>,
}

impl RegistryOwner {
    pub(crate) fn new(registry: ProviderRegistry) -> Self {
        let resource_owners: HashMap<_, _> = registry
            .providers()
            .iter()
            .filter_map(|provider| {
                provider
                    .session_resources()
                    .map(|owner| (provider.id().to_owned(), owner.new_namespace()))
            })
            .collect();
        let all_resource_owners = Arc::new(std::sync::Mutex::new(
            resource_owners.values().cloned().collect(),
        ));
        Self {
            registry,
            resource_owners,
            all_resource_owners,
            registration_gate: Arc::default(),
        }
    }
}

impl Models {
    /// Sync lookup of one owned catalog record (`provider`, bare model id).
    ///
    /// Returns a clone of the resolved record across the registry's override,
    /// dynamic-catalog, and built-in layers. Custom chat API records stay
    /// absent: registering a [`crate::ChatApi`] never adds its record to any catalog
    /// read.
    pub fn get_model(&self, provider: &str, id: &str) -> Option<ModelInfo> {
        let spec = format!("{provider}:{id}");
        self.registry
            .registry
            .resolve(&spec)
            .ok()
            .map(|(_, model)| model.clone())
    }

    pub(crate) fn replace_resource_owner(
        &mut self,
        provider: &str,
        owner: Option<Arc<dyn crate::codex_resources::SessionResourceOwner>>,
    ) {
        self.registry.resource_owners.remove(provider);
        if let Some(owner) = owner {
            // Earlier revisions stay reachable by cleanup and owned calls.
            self.registry
                .all_resource_owners
                .lock()
                .unwrap()
                .push(owner.clone());
            self.registry
                .resource_owners
                .insert(provider.to_owned(), owner);
        }
    }

    /// Access the underlying registry (for `--list-models`, overrides, etc.).
    pub fn registry(&self) -> &ProviderRegistry {
        &self.registry.registry
    }

    /// Return sorted registered provider ids.
    pub fn provider_ids(&self) -> Vec<&str> {
        let mut ids: BTreeSet<&str> = self.registry.registry.provider_ids().into_iter().collect();
        ids.extend(
            self.operation_owner
                .chat_apis
                .values()
                .map(|entry| entry.provider_id.as_str()),
        );
        ids.extend(
            self.operation_owner
                .image_apis
                .values()
                .map(|entry| entry.provider_id.as_str()),
        );
        ids.extend(
            self.operation_owner
                .classifier_apis
                .values()
                .map(|entry| entry.provider_id.as_str()),
        );
        ids.extend(
            self.operation_owner
                .deferred_apis
                .values()
                .map(|entry| entry.provider_id.as_str()),
        );
        ids.into_iter().collect()
    }

    /// Remove one live model-provider's registrations without touching shared
    /// stores or cancelling calls that already captured an adapter revision.
    /// Unknown IDs leave every binding unchanged, including auth-only flows.
    pub fn delete_provider(&mut self, provider: &str) -> bool {
        if !self.provider_ids().contains(&provider) {
            return false;
        }
        self.registry.registry.remove_provider(provider);
        self.clear_catalog_entry(provider);
        self.registry.resource_owners.remove(provider);
        self.auth_owner.auth.remove(provider);
        self.compat.remove(provider);
        self.auth_owner.probed.remove(provider);
        self.auth_owner.resolvers.remove(provider);
        self.auth_owner.sources.remove(provider);
        self.auth_owner.oauth_routes.remove(provider);
        self.auth_owner.oauth_flows.remove(provider);
        #[cfg(feature = "builtins")]
        self.auth_owner.builtin_default_oauth.remove(provider);
        self.auth_owner.builtin_auth_factories.remove(provider);
        self.auth_owner.builtin_auth_checks.remove(provider);
        self.auth_owner.builtin_default_auth.remove(provider);
        self.operation_owner
            .chat_apis
            .retain(|_, entry| entry.provider_id != provider);
        self.operation_owner
            .image_apis
            .retain(|_, entry| entry.provider_id != provider);
        self.operation_owner
            .classifier_apis
            .retain(|_, entry| entry.provider_id != provider);
        self.operation_owner
            .deferred_apis
            .retain(|_, entry| entry.provider_id != provider);
        true
    }

    /// Remove all live model-provider registrations. Auth-only flows, shared
    /// stores and captured calls retain their existing ownership.
    pub fn clear_providers(&mut self) {
        let ids: Vec<_> = self.provider_ids().into_iter().map(str::to_owned).collect();
        for id in ids {
            self.delete_provider(&id);
        }
    }

    /// Last-known chat records for the selected active provider, or all active
    /// providers. This performs no authentication or source work.
    pub fn get_models(&self, provider: Option<&str>) -> Vec<ModelInfo> {
        self.registry
            .registry
            .all_models()
            .into_iter()
            .filter(|(owner, _)| {
                provider.is_none_or(|wanted| wanted == *owner)
                    && self.registry.registry.get_provider(owner).is_some()
            })
            .map(|(_, model)| model.clone())
            .collect()
    }

    /// Read one last-known record of exactly the requested operation.
    pub fn get_model_of_type(
        &self,
        operation: crate::catalog::CatalogOperation,
        provider: &str,
        id: &str,
    ) -> Option<crate::catalog::CatalogRecord> {
        self.get_models_of_type(operation, Some(provider))
            .into_iter()
            .find(|record| record.id() == id)
    }

    /// Read last-known operation-qualified records without source work. A
    /// present empty dynamic catalog is authoritative and suppresses fallback.
    pub fn get_models_of_type(
        &self,
        operation: crate::catalog::CatalogOperation,
        provider: Option<&str>,
    ) -> Vec<crate::catalog::CatalogRecord> {
        use crate::catalog::{CatalogOperation, CatalogRecord};
        if operation == CatalogOperation::Chat {
            return self
                .get_models(provider)
                .into_iter()
                .map(CatalogRecord::from)
                .collect();
        }
        let mut records = Vec::new();
        for owner in self
            .provider_ids()
            .into_iter()
            .filter(|owner| provider.is_none_or(|wanted| wanted == *owner))
        {
            if let Some(entry) = self.catalog_owner.dynamic_catalogs.get(owner) {
                records.extend(
                    entry
                        .models
                        .iter()
                        .filter(|record| {
                            record.operation() == operation
                                && self.catalog_record_registered(owner, record)
                        })
                        .cloned(),
                );
            } else {
                match operation {
                    CatalogOperation::Image => {
                        for ((id, api), registration) in &self.operation_owner.image_apis {
                            if id == owner {
                                records.extend(
                                    registration
                                        .records
                                        .iter()
                                        .filter(|record| {
                                            record.provider_id == owner && &record.api == api
                                        })
                                        .cloned()
                                        .map(CatalogRecord::Image),
                                );
                            }
                        }
                    }
                    CatalogOperation::Classifier => {
                        for ((id, api), registration) in &self.operation_owner.classifier_apis {
                            if id == owner {
                                records.extend(
                                    registration
                                        .records
                                        .iter()
                                        .filter(|record| {
                                            record.provider_id == owner && &record.api == api
                                        })
                                        .cloned()
                                        .map(CatalogRecord::Classifier),
                                );
                            }
                        }
                    }
                    CatalogOperation::Chat => unreachable!(),
                }
            }
        }
        records
    }

    /// Read all three operation kinds; equal bare IDs in different operations
    /// remain separate records.
    pub fn get_all_models(&self, provider: Option<&str>) -> Vec<crate::catalog::CatalogRecord> {
        use crate::catalog::CatalogOperation;
        [
            CatalogOperation::Chat,
            CatalogOperation::Image,
            CatalogOperation::Classifier,
        ]
        .into_iter()
        .flat_map(|operation| self.get_models_of_type(operation, provider))
        .collect()
    }

    /// Check configuration for the requested operation without resolving
    /// generation credentials or refreshing builtin sources.
    pub async fn get_available_of_type(
        &self,
        operation: crate::catalog::CatalogOperation,
        provider: Option<&str>,
    ) -> Result<Vec<crate::catalog::CatalogRecord>, ModelsError> {
        use crate::catalog::{CatalogOperation, CatalogRecord};
        match operation {
            CatalogOperation::Chat => Ok(self
                .get_available(provider)
                .await?
                .into_iter()
                .map(|(_, model)| CatalogRecord::from(model))
                .collect()),
            CatalogOperation::Classifier => Ok(self
                .get_available_classifiers(provider)
                .await?
                .into_iter()
                .map(CatalogRecord::Classifier)
                .collect()),
            CatalogOperation::Image => {
                let mut records = Vec::new();
                let mut builtin_status = HashMap::new();
                for ((owner, api), registration) in &self.operation_owner.image_apis {
                    if provider.is_some_and(|wanted| wanted != owner) {
                        continue;
                    }
                    let available = if let Some(check) = registration
                        .builtin_auth
                        .then(|| self.auth_owner.builtin_auth_checks.get(owner))
                        .flatten()
                    {
                        if let Some(available) = builtin_status.get(owner) {
                            *available
                        } else {
                            let available = check()
                                .await
                                .map_err(|error| ModelsError::AuthResolution {
                                    provider: owner.clone(),
                                    error: Box::new(error),
                                })?
                                .is_some_and(|status| !status.responses_only);
                            builtin_status.insert(owner.clone(), available);
                            available
                        }
                    } else {
                        match registration.resolver.check_configuration() {
                            Some(check) => {
                                check.await.map_err(|error| ModelsError::AuthResolution {
                                    provider: owner.clone(),
                                    error: Box::new(error),
                                })?
                            }
                            None => true,
                        }
                    };
                    if available {
                        records.extend(self.get_models_of_type(CatalogOperation::Image, Some(owner)).into_iter().filter(|record| matches!(record, CatalogRecord::Image(model) if &model.api == api)));
                    }
                }
                Ok(records)
            }
        }
    }

    /// Check all operation-specific registration configurations.
    pub async fn get_all_available(
        &self,
        provider: Option<&str>,
    ) -> Result<Vec<crate::catalog::CatalogRecord>, ModelsError> {
        use crate::catalog::CatalogOperation;
        let mut records = Vec::new();
        for operation in [
            CatalogOperation::Chat,
            CatalogOperation::Image,
            CatalogOperation::Classifier,
        ] {
            records.extend(self.get_available_of_type(operation, provider).await?);
        }
        Ok(records)
    }

    /// Resolve a `provider:model` spec into provider reference + model info.
    pub fn resolve(&self, spec: &str) -> Result<(&dyn Provider, &ModelInfo), RegistryError> {
        self.registry.registry.resolve(spec)
    }

    /// Validate that `spec` resolves to a provider/model route backed by a live
    /// authentication resolver, without performing authentication I/O.
    pub fn validate_dispatchable_route(&self, spec: &str) -> Result<(), ModelsError> {
        let (provider, _) = self.registry.registry.resolve(spec)?;
        let provider_id = provider.id();
        if !self.auth_owner.resolvers.contains_key(provider_id) {
            return Err(ModelsError::RouteNotDispatchable {
                provider: provider_id.to_owned(),
            });
        }
        Ok(())
    }

    /// Query capabilities for a `provider:model` spec.
    pub fn capabilities(&self, spec: &str) -> Result<ModelCapabilities, RegistryError> {
        self.registry.registry.capabilities(spec)
    }

    /// Cancel and await the session-scoped connection resources of every
    /// registered resource-owning provider (task 20.17).
    ///
    /// - `Some(nonempty id)` cancels only the generations and pooled
    ///   connections registered under that exact session (an unmatched id is
    ///   a no-op);
    /// - `None` cancels everything, including sessionless generations;
    /// - `Some("")` fails as
    ///   [`CleanupError::EmptySessionId`](crate::codex_resources::CleanupError::EmptySessionId)
    ///   with zero effects.
    ///
    /// The returned barrier waits for the cancelled calls' local owned
    /// exits and the bounded connection closes; one owner's failure never
    /// aborts the remaining release — unconfirmed effects are aggregated
    /// into the returned report. Caller parent tokens, external cancel
    /// owners, credentials and catalogs are never touched.
    pub fn cleanup_session_resources(
        &self,
        session: Option<&str>,
    ) -> impl Future<
        Output = Result<
            crate::codex_resources::CleanupReport,
            crate::codex_resources::CleanupError,
        >,
    >
    + Send
    + 'static
    + use<> {
        let session = session.map(str::to_owned);
        let resource_owners = self.registry.all_resource_owners.clone();
        let registration_gate = self.registry.registration_gate.clone();
        async move {
            if session.as_deref() == Some("") {
                return Err(crate::codex_resources::CleanupError::EmptySessionId);
            }
            // Every owner's cutoff is established on this Future's first poll,
            // before awaiting any one owner's releases.
            let handles: Vec<crate::codex_resources::CleanupHandle> = {
                // This lock also gates every Models first-poll registration.
                // All namespace cutoffs therefore precede any later generation,
                // even when calls are first-polled concurrently on other threads.
                let _registration = registration_gate.lock().unwrap();
                let owners = resource_owners.lock().unwrap();
                owners
                    .iter()
                    .map(|owner| owner.cleanup_session_resources(session.as_deref()))
                    .collect()
            };
            let mut report = crate::codex_resources::CleanupReport::default();
            for handle in handles {
                match handle.resolve().await {
                    Ok(effects) => report.merge(effects),
                    Err(crate::codex_resources::CleanupError::Incomplete { report: effects }) => {
                        report.merge(effects)
                    }
                    Err(crate::codex_resources::CleanupError::EmptySessionId) => {
                        report
                            .unconfirmed_effects
                            .push("a resource owner rejected the cleanup selector".into());
                    }
                }
            }
            if report.unconfirmed_effects.is_empty() {
                Ok(report)
            } else {
                Err(crate::codex_resources::CleanupError::Incomplete { report })
            }
        }
    }
}
