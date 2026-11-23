//! Dynamic catalog and publication ownership for the [`Models`](crate::models::Models) facade.

use super::*;

use std::collections::HashMap;
use std::sync::Arc;

/// Dynamic catalog and publication owner behind the [`Models`] facade.
///
/// Store restoration, source registration, publisher retention and
/// publication generations form one consistency boundary for refresh rounds.
pub(crate) struct CatalogOwner {
    pub(crate) models_store: Option<crate::catalog::ModelsStore>,
    pub(crate) catalog_sources: Vec<std::sync::Arc<dyn crate::catalog::DynamicCatalogSource>>,
    pub(crate) authenticated_catalogs:
        HashMap<String, Arc<dyn crate::catalog::AuthenticatedCatalog>>,
    pub(crate) catalog_publishers: HashMap<String, crate::catalog::CatalogPublisher>,
    pub(crate) dynamic_catalogs: HashMap<String, crate::catalog::ModelsStoreEntry>,
    pub(crate) publication_generation: Arc<()>,
}

impl CatalogOwner {
    pub(crate) fn new() -> Self {
        Self {
            models_store: None,
            catalog_sources: Vec::new(),
            authenticated_catalogs: HashMap::new(),
            catalog_publishers: HashMap::new(),
            dynamic_catalogs: HashMap::new(),
            publication_generation: Arc::new(()),
        }
    }
}

impl Models {
    /// The compatibility metadata associated with a provider, if any.
    pub fn compat(&self, provider_id: &str) -> Option<&CompatMetadata> {
        self.compat.get(provider_id)
    }

    /// Refresh selected active provider owners, restoring their cached records
    /// before optional source work. `force` is interpreted only by each source.
    /// Successes publish through the existing store ACK; failed owners retain
    /// their prior records. Captured calls retain their frozen revisions.
    pub async fn refresh(
        &mut self,
        options: ModelsRefreshOptions,
    ) -> Result<RefreshReport, ModelsError> {
        self.refresh_with(options, &CancellationToken::new()).await
    }

    /// Refresh with cancellation. Selected cached records restore before source
    /// work; cancellation or supersession prevents a later fetched publication.
    pub async fn refresh_with(
        &mut self,
        options: ModelsRefreshOptions,
        token: &CancellationToken,
    ) -> Result<RefreshReport, ModelsError> {
        let mut report = RefreshReport::default();
        let selected: BTreeSet<String> = self
            .provider_ids()
            .into_iter()
            .filter(|id| {
                options
                    .providers
                    .as_ref()
                    .is_none_or(|wanted| wanted.contains(*id))
            })
            .map(str::to_owned)
            .collect();
        if token.is_cancelled() {
            report.status = RefreshStatus::Cancelled;
            return Ok(report);
        }
        if selected.is_empty() {
            if !options.allow_network {
                report.status = RefreshStatus::OfflineRestored;
            }
            return Ok(report);
        }
        let generation = self.catalog_owner.publication_generation.clone();
        // The complete persisted map remains the merge base even when this
        // instance restores only a subset or knows only some of its owners.
        let (guard, mut merged, mut publishers) =
            if let Some(store) = &self.catalog_owner.models_store {
                let (guard, snapshot, publishers) = store.begin_snapshot(Some(token.clone()));
                self.apply_selected_catalog_entries(&selected, &snapshot.catalogs, &publishers);
                for id in &selected {
                    if let Some(entry) = self.catalog_owner.dynamic_catalogs.get(id) {
                        report.entries.push(RefreshReportEntry {
                            id: id.clone(),
                            outcome: RefreshOutcome::Restored {
                                models: entry.models.len(),
                            },
                        });
                    }
                }
                (Some(guard), snapshot.catalogs, publishers)
            } else {
                (
                    None,
                    self.catalog_owner.dynamic_catalogs.clone(),
                    self.catalog_owner.catalog_publishers.clone(),
                )
            };
        if !options.allow_network {
            report.status = RefreshStatus::OfflineRestored;
            return Ok(report);
        }
        publishers.retain(|provider, _| merged.contains_key(provider));
        for id in &selected {
            let Some(provider) = self.registry.registry.get_provider(id) else {
                continue;
            };
            let context = crate::catalog::CatalogRefreshContext {
                allow_network: true,
                force: options.force,
                providers: BTreeSet::from([id.clone()]),
                cancel: token.clone(),
            };
            let authenticated = self.catalog_owner.authenticated_catalogs.get(id);
            let fetch = async {
                if let Some(catalog) = authenticated {
                    match self.get_auth(id, AuthResolutionOverrides::default()).await {
                        Ok(Some(auth)) => catalog.fetch(auth).await.map(Some),
                        Ok(None) => Ok(None),
                        Err(ModelsError::AuthResolution { error, .. }) => Err(*error),
                        Err(error) => Err(models_error_cause(error)),
                    }
                } else {
                    provider.refresh_models(context).await
                }
            };
            let result = tokio::select! {
                biased;
                _ = token.cancelled() => { report.status = RefreshStatus::Cancelled; return Ok(report); },
                result = fetch => result,
            };
            match result {
                Ok(Some(models)) => match validate_dynamic_catalog(id, &models).and_then(|()| {
                    if models
                        .iter()
                        .all(|model| self.chat_model_admitted(id, model))
                    {
                        Ok(())
                    } else {
                        Err(ProviderError::Config(
                            ProviderErrorSummary::attested_static(
                                "native catalog contains a model without a registered chat route",
                            ),
                        ))
                    }
                }) {
                    Ok(()) => {
                        report.entries.push(RefreshReportEntry {
                            id: id.clone(),
                            outcome: RefreshOutcome::Published {
                                models: models.len(),
                            },
                        });
                        publishers.insert(
                            id.clone(),
                            crate::catalog::CatalogPublisher::Native {
                                scope: authenticated.map(|catalog| catalog.scope().to_owned()),
                            },
                        );
                        merged.insert(
                            id.clone(),
                            crate::catalog::ModelsStoreEntry {
                                models: models
                                    .into_iter()
                                    .map(crate::catalog::CatalogRecord::from)
                                    .collect(),
                                checked_at: authenticated.map(|_| crate::time::now_ms()),
                                ..Default::default()
                            },
                        );
                    }
                    Err(error) => report.entries.push(RefreshReportEntry {
                        id: id.clone(),
                        outcome: RefreshOutcome::Failed(error),
                    }),
                },
                // An unconfigured authenticated route leaves its last-known
                // publication available for offline lookup.
                Ok(None) if authenticated.is_some() => {}
                Ok(None) => {
                    // A source's ownership is limited to its declared provider
                    // scope; unrelated sources cannot retain this snapshot.
                    let source_owned = matches!(publishers.get(id), Some(crate::catalog::CatalogPublisher::Source {id:source_id})
                        if self.catalog_owner.catalog_sources.iter().any(|source| source.id() == source_id && source.provider_ids().contains(id)));
                    if !source_owned {
                        merged.remove(id);
                        publishers.remove(id);
                    }
                    report.entries.push(RefreshReportEntry {
                        id: id.clone(),
                        outcome: RefreshOutcome::Published {
                            models: merged.get(id).map_or(0, |entry| entry.models.len()),
                        },
                    });
                }
                Err(error) => report.entries.push(RefreshReportEntry {
                    id: id.clone(),
                    outcome: RefreshOutcome::Failed(error),
                }),
            }
        }
        for source in &self.catalog_owner.catalog_sources {
            let source_selected: BTreeSet<String> = source
                .provider_ids()
                .iter()
                .filter(|id| selected.contains(*id))
                .cloned()
                .collect();
            if source_selected.is_empty() {
                continue;
            }
            let context = crate::catalog::CatalogRefreshContext {
                allow_network: true,
                force: options.force,
                providers: source_selected.clone(),
                cancel: token.clone(),
            };
            let result = tokio::select! {
                biased;
                _ = token.cancelled() => { report.status = RefreshStatus::Cancelled; return Ok(report); },
                result = source.fetch(context) => result,
            };
            match result {
                Ok(catalogs) => {
                    let error = catalogs
                        .keys()
                        .find(|id| !source.provider_ids().contains(id))
                        .map(|_| {
                            ProviderError::Config(ProviderErrorSummary::attested_static(
                                "catalog source returned an undeclared provider",
                            ))
                        })
                        .or_else(|| {
                            catalogs
                                .iter()
                                .filter(|(id, _)| source_selected.contains(*id))
                                .find_map(|(id, entry)| {
                                    self.validate_catalog_entry(id, entry).err()
                                })
                        });
                    if let Some(error) = error {
                        report.entries.push(RefreshReportEntry {
                            id: source.id().to_owned(),
                            outcome: RefreshOutcome::Failed(error),
                        });
                        continue;
                    }
                    let mut published = 0;
                    for (id, entry) in catalogs
                        .into_iter()
                        .filter(|(id, _)| source_selected.contains(id))
                    {
                        published += entry.models.len();
                        publishers.insert(
                            id.clone(),
                            crate::catalog::CatalogPublisher::Source {
                                id: source.id().to_owned(),
                            },
                        );
                        merged.insert(id, entry);
                    }
                    report.entries.push(RefreshReportEntry {
                        id: source.id().to_owned(),
                        outcome: RefreshOutcome::Published { models: published },
                    });
                }
                Err(error) => report.entries.push(RefreshReportEntry {
                    id: source.id().to_owned(),
                    outcome: RefreshOutcome::Failed(error),
                }),
            }
        }
        if token.is_cancelled() {
            report.status = RefreshStatus::Cancelled;
            return Ok(report);
        }
        if !Arc::ptr_eq(&generation, &self.catalog_owner.publication_generation) {
            report.status = RefreshStatus::Superseded;
            return Ok(report);
        }
        match guard {
            Some(guard) => match guard.commit_with_publishers(merged, publishers).await {
                Ok((commit, publishers)) => {
                    if token.is_cancelled() {
                        report.status = RefreshStatus::Cancelled;
                    } else if !Arc::ptr_eq(&generation, &self.catalog_owner.publication_generation)
                    {
                        report.status = RefreshStatus::Superseded;
                    } else {
                        self.apply_selected_catalog_entries(
                            &selected,
                            &commit.catalogs,
                            &publishers,
                        );
                    }
                }
                Err(crate::catalog::StoreCommitError::Superseded { .. }) => {
                    report.status = RefreshStatus::Superseded
                }
                Err(crate::catalog::StoreCommitError::Cancelled) => {
                    report.status = RefreshStatus::Cancelled
                }
                Err(crate::catalog::StoreCommitError::Store(error)) => {
                    return Err(ModelsError::CatalogStore(error));
                }
            },
            None => self.apply_selected_catalog_entries(&selected, &merged, &publishers),
        }
        Ok(report)
    }

    pub(crate) fn clear_catalog_entry(&mut self, provider: &str) {
        self.catalog_owner.dynamic_catalogs.remove(provider);
        self.catalog_owner.catalog_publishers.remove(provider);
        self.catalog_owner.authenticated_catalogs.remove(provider);
        self.catalog_owner.publication_generation = Arc::new(());
    }

    pub(crate) fn catalog_record_registered(
        &self,
        provider: &str,
        record: &crate::catalog::CatalogRecord,
    ) -> bool {
        match record {
            crate::catalog::CatalogRecord::Chat(_) => {
                self.registry.registry.get_provider(provider).is_some()
            }
            crate::catalog::CatalogRecord::Image(model) => {
                model.provider_id == provider
                    && self
                        .operation_owner
                        .image_apis
                        .contains_key(&(provider.into(), model.api.clone()))
            }
            crate::catalog::CatalogRecord::Classifier(model) => {
                model.provider_id == provider
                    && self
                        .operation_owner
                        .classifier_apis
                        .contains_key(&(provider.into(), model.api.clone()))
            }
        }
    }

    fn catalog_record_admitted(
        &self,
        provider: &str,
        record: &crate::catalog::CatalogRecord,
    ) -> bool {
        if !self.catalog_record_registered(provider, record) {
            return false;
        }
        match record {
            crate::catalog::CatalogRecord::Chat(model) => self.chat_model_admitted(provider, model),
            _ => true,
        }
    }

    fn chat_model_admitted(&self, provider: &str, model: &ModelInfo) -> bool {
        self.registry
            .registry
            .get_provider(provider)
            .is_some_and(|owner| {
                // This existing pre-auth gate checks the real mapped route graph
                // without credentials or seed-model membership.
                owner
                    .validate_simple_options(model, &SimpleOptions::default())
                    .is_ok()
            })
    }

    fn apply_selected_catalog_entries(
        &mut self,
        selected: &BTreeSet<String>,
        entries: &HashMap<String, crate::catalog::ModelsStoreEntry>,
        publishers: &HashMap<String, crate::catalog::CatalogPublisher>,
    ) {
        for id in selected {
            let matches_scope = match publishers.get(id) {
                Some(crate::catalog::CatalogPublisher::Native { scope: Some(scope) }) => self
                    .catalog_owner
                    .authenticated_catalogs
                    .get(id)
                    .is_some_and(|catalog| catalog.scope() == scope),
                _ => true,
            };
            if let Some(entry) = entries.get(id).filter(|_| matches_scope) {
                let mut entry = entry.clone();
                let acknowledged_empty = entry.models.is_empty();
                entry
                    .models
                    .retain(|record| self.catalog_record_admitted(id, record));
                if !acknowledged_empty && entry.models.is_empty() {
                    // A stale nonempty catalog for retired API owners is not an
                    // authoritative empty publication by the replacement owner.
                    self.catalog_owner.dynamic_catalogs.remove(id);
                    self.catalog_owner.catalog_publishers.remove(id);
                    continue;
                }
                self.catalog_owner
                    .dynamic_catalogs
                    .insert(id.clone(), entry);
                if let Some(publisher) = publishers.get(id) {
                    self.catalog_owner
                        .catalog_publishers
                        .insert(id.clone(), publisher.clone());
                } else {
                    self.catalog_owner.catalog_publishers.remove(id);
                }
            } else {
                self.catalog_owner.dynamic_catalogs.remove(id);
                self.catalog_owner.catalog_publishers.remove(id);
            }
        }
        let chat = self
            .catalog_owner
            .dynamic_catalogs
            .iter()
            .filter(|(id, _)| self.registry.registry.get_provider(id).is_some())
            .map(|(id, entry)| {
                (
                    id.clone(),
                    entry
                        .models
                        .iter()
                        .filter_map(|record| match record {
                            crate::catalog::CatalogRecord::Chat(model) => {
                                Some(model.as_ref().clone())
                            }
                            _ => None,
                        })
                        .collect(),
                )
            })
            .collect();
        self.registry.registry.replace_all_dynamic_catalogs(chat);
    }

    fn validate_catalog_entry(
        &self,
        provider: &str,
        entry: &crate::catalog::ModelsStoreEntry,
    ) -> Result<(), ProviderError> {
        let invalid =
            |message: String| ProviderError::Config(ProviderErrorSummary::sanitized(message));
        crate::catalog::wire::validate_provider_entry(provider, entry).map_err(invalid)?;
        if self.registry.registry.get_provider(provider).is_none()
            && !self
                .operation_owner
                .image_apis
                .keys()
                .any(|(owner, _)| owner == provider)
            && !self
                .operation_owner
                .classifier_apis
                .keys()
                .any(|(owner, _)| owner == provider)
        {
            return Err(invalid(format!(
                "catalog contains unknown provider '{provider}'"
            )));
        }
        for record in &entry.models {
            if !self.catalog_record_admitted(provider, record) {
                return Err(invalid(format!(
                    "catalog model '{}:{}' has no registered {} route",
                    provider,
                    record.id(),
                    record.operation()
                )));
            }
        }
        Ok(())
    }

    /// Attach a persisted dynamic-catalog store. Restores happen at the next
    /// refresh round; nothing is fetched or written here.
    pub fn set_models_store(&mut self, store: crate::catalog::ModelsStore) {
        self.catalog_owner.models_store = Some(store);
        self.catalog_owner.publication_generation = Arc::new(());
    }

    #[cfg(feature = "builtins")]
    pub(crate) fn register_authenticated_catalog(
        &mut self,
        provider: &str,
        catalog: Arc<dyn crate::catalog::AuthenticatedCatalog>,
    ) {
        self.catalog_owner
            .authenticated_catalogs
            .insert(provider.to_owned(), catalog);
        self.catalog_owner.publication_generation = Arc::new(());
    }

    /// Register a dynamic-catalog source fetched during network-enabled refresh
    /// rounds. Registered explicitly; the SDK installs no default source.
    pub fn register_catalog_source(
        &mut self,
        source: std::sync::Arc<dyn crate::catalog::DynamicCatalogSource>,
    ) {
        self.catalog_owner.catalog_sources.push(source);
        self.catalog_owner.publication_generation = Arc::new(());
    }
}
