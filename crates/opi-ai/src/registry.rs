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
//!   returns [`RegistrationError::EmptyModelId`].
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

use crate::provider::{ModelInfo, Provider};

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
}

/// Capabilities of a resolved model.
///
/// This type is `#[non_exhaustive]` so external consumers must construct it
/// through [`ModelCapabilities::new`] or the builder-style setters.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ModelCapabilities {
    pub context_window: u64,
    pub max_output_tokens: u64,
    pub supports_images: bool,
    pub supports_streaming: bool,
    pub supports_thinking: bool,
    /// True when the provider supports Anthropic-style `cache_control` markers
    /// on content blocks and tool definitions.
    pub supports_cache_control: bool,
    /// True when the provider/model pair supports one-hour
    /// `cache_control { ttl: "1h" }` markers.
    pub supports_long_cache_retention: bool,
}

impl ModelCapabilities {
    /// Create capabilities with required sizing fields. All boolean flags
    /// default to `false`; use builder methods to enable them.
    pub fn new(context_window: u64, max_output_tokens: u64) -> Self {
        Self {
            context_window,
            max_output_tokens,
            supports_images: false,
            supports_streaming: false,
            supports_thinking: false,
            supports_cache_control: false,
            supports_long_cache_retention: false,
        }
    }
    pub fn with_images(mut self, v: bool) -> Self {
        self.supports_images = v;
        self
    }
    pub fn with_streaming(mut self, v: bool) -> Self {
        self.supports_streaming = v;
        self
    }
    pub fn with_thinking(mut self, v: bool) -> Self {
        self.supports_thinking = v;
        self
    }
    pub fn with_cache_control(mut self, v: bool) -> Self {
        self.supports_cache_control = v;
        self
    }
    pub fn with_long_cache_retention(mut self, v: bool) -> Self {
        self.supports_long_cache_retention = v;
        self
    }
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self::new(0, 0)
    }
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
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            model_overrides: HashMap::new(),
        }
    }

    /// Register a custom provider. Replaces any existing provider with the same
    /// id.
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
        self.providers.push(provider);
        Ok(())
    }

    /// Backward-compatible alias: register a provider without validation.
    /// Prefer [`register_provider`](Self::register_provider) for new code.
    pub fn register(&mut self, provider: Box<dyn Provider>) {
        let id = provider.id().to_owned();
        self.providers.retain(|p| p.id() != id);
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
    /// Checks the override layer first, then falls back to the provider's own
    /// model list.
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
    /// Each entry is `(provider_id, &ModelInfo)`. When a model override
    /// shadows a provider's built-in model (same provider id and model id),
    /// the override entry replaces the built-in entry so consumers see a
    /// deduplicated view consistent with [`resolve`](Self::resolve).
    ///
    /// Useful for `--list-models` style enumeration.
    pub fn all_models(&self) -> Vec<(&str, &ModelInfo)> {
        let mut result = Vec::new();

        // Models from registered providers, skipping any that are shadowed
        // by an override.
        for provider in &self.providers {
            for model in provider.models() {
                let key = (provider.id().to_owned(), model.id.clone());
                if self.model_overrides.contains_key(&key) {
                    continue; // override will be added below
                }
                result.push((provider.id(), model));
            }
        }

        // Override models (supplement or shadow provider models). HashMap
        // iteration is intentionally normalized so list-models/pickers stay
        // deterministic once overrides are present.
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
