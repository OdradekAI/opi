//! Provider registry — resolves `provider:model` specs to provider + model info.
//!
//! Supports both built-in providers and custom providers registered at runtime
//! through [`ProviderRegistry::register_provider`]. Additional model metadata
//! can be layered onto existing providers via [`ProviderRegistry::register_model`].
//!
//! # Registration
//!
//! Custom providers implement the [`Provider`] trait
//! and are registered before agent startup. Provider breadth should arrive
//! through registration rather than core provider additions — the registry
//! is the single source of truth for provider and model resolution.
//!
//! Model overrides let you add fine-tuned or deployment-specific models to an
//! existing provider without implementing a new provider. Overrides take
//! precedence over the provider's own model list on name collision.
//!
//! # Capability Declaration
//!
//! Each model carries a [`ModelInfo`] struct that declares its capabilities:
//! context window size, max output tokens, image support, streaming support,
//! and thinking/reasoning support. These are queried via
//! [`ProviderRegistry::capabilities`] and used for request validation.
//!
//! Note: [`validate_request_capabilities`](crate::provider::validate_request_capabilities)
//! operates on a bare `&dyn Provider` reference and does not see the registry's
//! override layer. Callers that need override-aware capability checks should
//! use [`ProviderRegistry::capabilities`] instead.
//!
//! # Duplicate / Invalid Registration Behavior
//!
//! - **Providers**: registering a provider with the same id as an existing
//!   provider silently replaces it. An empty provider id returns
//!   [`RegistrationError::EmptyProviderId`].
//! - **Models**: registering a model override with the same `(provider_id,
//!   model_id)` pair as an existing override returns
//!   [`RegistrationError::DuplicateModel`]. Registering an override for a
//!   model that already exists in the provider's built-in list is allowed —
//!   the override shadows the built-in at resolve time. An empty model id
//!   returns [`RegistrationError::EmptyModelId`], and invalid metadata returns
//!   [`RegistrationError::InvalidModel`].
//!
//! # --list-models Integration
//!
//! [`ProviderRegistry::all_models`] returns all models across all providers
//! and the override layer in a deduplicated form (overrides replace built-ins
//! on collision). This method is designed for `--list-models` style
//! enumeration. Custom providers registered through extensions will appear
//! alongside built-in providers when the registry is used as the model source.
//!
//! # Streaming Contract
//!
//! Registered providers must implement [`Provider::stream`] returning an
//! [`EventStream`](crate::provider::EventStream). The registry does not
//! modify or wrap the stream — it passes the provider's stream through
//! directly on resolve. Extensions that provide custom providers must honor
//! the same streaming contract as built-in providers.
//!
//! # Unstable
//!
//! The registration API is part of the **unstable 0.x extension surface**.
//! Breaking changes may occur between minor versions without a major version
//! bump.

use std::collections::HashMap;

use crate::model_info::ModelInfoError;
use crate::provider::{ModelInfo, Provider};

pub use crate::model_info::ModelCapabilities;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error type for registry operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RegistryError {
    #[error("invalid model spec: {0}")]
    InvalidSpec(String),
    #[error("unknown provider: {0}")]
    UnknownProvider(String),
    #[error("unknown model '{model}' for provider '{provider}'")]
    UnknownModel { provider: String, model: String },
}

/// Error type for provider/model registration.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RegistrationError {
    /// Provider id is empty.
    #[error("provider id cannot be empty")]
    EmptyProviderId,
    /// Model id is empty.
    #[error("model id cannot be empty for provider '{provider}'")]
    EmptyModelId { provider: String },
    /// Model with the same id already exists for this provider.
    #[error("model '{model}' already registered for provider '{provider}'")]
    DuplicateModel { provider: String, model: String },
    /// Model metadata failed validation.
    #[error("invalid model '{model}' for provider '{provider}': {source}")]
    InvalidModel {
        provider: String,
        model: String,
        #[source]
        source: ModelInfoError,
    },
}

// ---------------------------------------------------------------------------
// ProviderRegistry
// ---------------------------------------------------------------------------

/// Registry of available providers, keyed by provider id.
///
/// Supports dynamic registration of custom providers and model overrides.
/// See the [module-level documentation](self) for registration semantics.
pub struct ProviderRegistry {
    providers: Vec<Box<dyn Provider>>,
    /// Supplementary model overrides keyed by `(provider_id, model_id)`.
    model_overrides: HashMap<(String, String), ModelInfo>,
    /// Per-provider dynamic catalogs populated by `Provider::refresh_models`.
    /// When present, the dynamic catalog replaces the provider's built-in
    /// model list for resolution and enumeration. Cleared and replaced
    /// atomically by `ProviderCollection::refresh`.
    dynamic_catalogs: HashMap<String, Vec<ModelInfo>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            model_overrides: HashMap::new(),
            dynamic_catalogs: HashMap::new(),
        }
    }

    /// Register a custom provider. Replaces any existing provider with the same
    /// id and clears that provider's prior dynamic catalog snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`RegistrationError::EmptyProviderId`] if the provider id is
    /// empty.
    pub fn register_provider(
        &mut self,
        provider: Box<dyn Provider>,
    ) -> Result<(), RegistrationError> {
        if provider.id().is_empty() {
            return Err(RegistrationError::EmptyProviderId);
        }
        let id = provider.id().to_owned();
        self.providers.retain(|p| p.id() != id);
        self.dynamic_catalogs.remove(&id);
        self.providers.push(provider);
        Ok(())
    }

    /// Backward-compatible alias: register a provider without id validation,
    /// clearing that provider's prior dynamic catalog snapshot.
    /// Prefer [`register_provider`](Self::register_provider) for new code.
    pub fn register(&mut self, provider: Box<dyn Provider>) {
        let id = provider.id().to_owned();
        self.providers.retain(|p| p.id() != id);
        self.dynamic_catalogs.remove(&id);
        self.providers.push(provider);
    }

    /// Register a model override for an existing or future provider.
    ///
    /// The model is stored in the registry's override layer and will be
    /// returned by [`resolve`](Self::resolve) when a matching spec is looked
    /// up. Override models take precedence over the provider's own model list.
    /// Registering an override for a model that already exists in the
    /// provider's own list is allowed — the override shadows the built-in.
    ///
    /// # Errors
    ///
    /// - [`RegistrationError::EmptyModelId`] if the model id is empty.
    /// - [`RegistrationError::InvalidModel`] if the model metadata is invalid.
    /// - [`RegistrationError::DuplicateModel`] if the same `(provider_id,
    ///   model_id)` pair already exists in the override layer.
    pub fn register_model(
        &mut self,
        provider_id: &str,
        model: ModelInfo,
    ) -> Result<(), RegistrationError> {
        if model.id.is_empty() {
            return Err(RegistrationError::EmptyModelId {
                provider: provider_id.to_owned(),
            });
        }
        let key = (provider_id.to_owned(), model.id.clone());

        // Check override layer for duplicates. Registering an override for a
        // model that already exists in the provider's own list is allowed —
        // the override takes precedence at resolve time.
        if self.model_overrides.contains_key(&key) {
            return Err(RegistrationError::DuplicateModel {
                provider: provider_id.to_owned(),
                model: model.id,
            });
        }

        model
            .validate()
            .map_err(|source| RegistrationError::InvalidModel {
                provider: provider_id.to_owned(),
                model: model.id.clone(),
                source,
            })?;
        self.model_overrides.insert(key, model);
        Ok(())
    }

    /// Return sorted list of registered provider ids.
    pub fn provider_ids(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = self.providers.iter().map(|p| p.id()).collect();
        ids.sort();
        ids
    }

    /// Resolve a `provider:model` spec into provider reference + model info.
    ///
    /// Layering (first match wins):
    /// 1. Model overrides (registered via [`register_model`](Self::register_model)).
    /// 2. Dynamic catalog (populated by [`Provider::refresh_models`]).
    /// 3. Provider built-in model list.
    pub fn resolve(&self, spec: &str) -> Result<(&dyn Provider, &ModelInfo), RegistryError> {
        let (provider_id, model_id) = split_spec(spec)?;
        let provider = self
            .providers
            .iter()
            .find(|p| p.id() == provider_id)
            .ok_or_else(|| RegistryError::UnknownProvider(provider_id.to_owned()))?;

        // Check override layer first.
        let key = (provider_id.to_owned(), model_id.to_owned());
        if let Some(model) = self.model_overrides.get(&key) {
            return Ok((provider.as_ref(), model));
        }

        // Check dynamic catalog (from refresh_models).
        if let Some(catalog) = self.dynamic_catalogs.get(provider_id) {
            if let Some(model) = catalog.iter().find(|m| m.id == model_id) {
                return Ok((provider.as_ref(), model));
            }
            // Dynamic catalog is authoritative when present — don't fall through
            // to built-in models.
            return Err(RegistryError::UnknownModel {
                provider: provider_id.to_owned(),
                model: model_id.to_owned(),
            });
        }

        // Fall back to provider's own models.
        let model = provider
            .models()
            .iter()
            .find(|m| m.id == model_id)
            .ok_or_else(|| RegistryError::UnknownModel {
                provider: provider_id.to_owned(),
                model: model_id.to_owned(),
            })?;
        Ok((provider.as_ref(), model))
    }

    /// Query capabilities for a `provider:model` spec.
    pub fn capabilities(&self, spec: &str) -> Result<ModelCapabilities, RegistryError> {
        let (_, model) = self.resolve(spec)?;
        Ok(model.capabilities)
    }

    /// Get a provider by id.
    pub fn get_provider(&self, id: &str) -> Option<&dyn Provider> {
        self.providers
            .iter()
            .find(|p| p.id() == id)
            .map(|p| p.as_ref())
    }

    /// Return all models across all providers and the override layer.
    ///
    /// Each entry is `(provider_id, &ModelInfo)`. Layering (first wins):
    /// 1. Model overrides shadow dynamic catalogs and built-in models.
    /// 2. Dynamic catalogs (from `refresh_models`) replace built-in models.
    /// 3. Provider built-in models are the base layer.
    ///
    /// Useful for `--list-models` style enumeration.
    pub fn all_models(&self) -> Vec<(&str, &ModelInfo)> {
        let mut result = Vec::new();

        // Collect overridden model keys so we can skip shadowed entries.
        let overridden: Vec<(&String, &String)> = self
            .model_overrides
            .keys()
            .map(|(pid, mid)| (pid, mid))
            .collect();

        for provider in &self.providers {
            let pid = provider.id();

            // If a dynamic catalog exists for this provider, use it instead of
            // built-in models.
            if let Some(catalog) = self.dynamic_catalogs.get(pid) {
                for model in catalog {
                    if overridden.contains(&(&pid.to_owned(), &model.id)) {
                        continue; // override will be added below
                    }
                    result.push((pid, model));
                }
            } else {
                for model in provider.models() {
                    if overridden.contains(&(&pid.to_owned(), &model.id)) {
                        continue; // override will be added below
                    }
                    result.push((pid, model));
                }
            }
        }

        // Override models (supplement or shadow). HashMap iteration is
        // intentionally normalized so list-models/pickers stay deterministic
        // once overrides are present.
        let mut overrides = self.model_overrides.iter().collect::<Vec<_>>();
        overrides.sort_by(|((provider_a, model_a), _), ((provider_b, model_b), _)| {
            provider_a
                .cmp(provider_b)
                .then_with(|| model_a.cmp(model_b))
        });
        for ((provider_id, _model_id), model) in overrides {
            result.push((provider_id.as_str(), model));
        }

        result
    }

    /// Replace the dynamic catalog for a single provider.
    ///
    /// Used by [`crate::ProviderCollection::refresh`] to atomically install refreshed
    /// catalogs after all providers succeed.
    pub fn set_dynamic_catalog(&mut self, provider_id: &str, models: Vec<ModelInfo>) {
        self.dynamic_catalogs.insert(provider_id.to_owned(), models);
    }

    /// Atomically replace all dynamic catalogs.
    ///
    /// Clears existing catalogs and installs `catalogs` as the new set.
    /// Used by [`crate::ProviderCollection::refresh`] for all-or-nothing replacement.
    pub fn replace_all_dynamic_catalogs(&mut self, catalogs: HashMap<String, Vec<ModelInfo>>) {
        self.dynamic_catalogs = catalogs;
    }

    /// Clear all dynamic catalogs (rollback on refresh error).
    pub fn clear_dynamic_catalogs(&mut self) {
        self.dynamic_catalogs.clear();
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Split a `provider:model` spec. Returns `InvalidSpec` if no colon or empty parts.
fn split_spec(spec: &str) -> Result<(&str, &str), RegistryError> {
    let Some((provider, model)) = spec.split_once(':') else {
        return Err(RegistryError::InvalidSpec(format!(
            "spec must be 'provider:model', got: {spec:?}"
        )));
    };
    if provider.is_empty() || model.is_empty() {
        return Err(RegistryError::InvalidSpec(format!(
            "spec must be 'provider:model', got: {spec:?}"
        )));
    }
    Ok((provider, model))
}
