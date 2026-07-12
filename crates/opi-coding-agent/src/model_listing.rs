//! Model listing helpers backed by ProviderRegistry.

/// Entry for --list-models output.
pub struct ModelEntry {
    pub provider_id: String,
    pub model_id: String,
    pub display_name: String,
}

/// Convert a provider registry into display rows for model listing.
pub fn model_entries_from_registry(registry: &opi_ai::ProviderRegistry) -> Vec<ModelEntry> {
    registry
        .all_models()
        .into_iter()
        .map(|(provider_id, model)| ModelEntry {
            provider_id: provider_id.to_owned(),
            model_id: model.id.clone(),
            display_name: model.display_name.clone(),
        })
        .collect()
}
