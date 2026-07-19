//! Provider collection, catalog, and redacted auth-status facade.
//!
//! [`ProviderCollection`] is the higher-level facade above [`ProviderRegistry`]
//! that owns provider and model lookup, the provider-side auth contract,
//! OpenAI-compatible compatibility metadata, stream/complete dispatch, and
//! redacted missing/invalid auth diagnostics. It wraps a registry (D1) rather
//! than replacing it, so existing provider paths and the documented unstable
//! registration API keep working.
//!
//! # Auth descriptor, resolver, and store split
//!
//! [`AuthDescriptor`] carries redaction-safe collection metadata and supports
//! non-live dispatch gates. Concrete routes use [`crate::AuthResolver`] to
//! resolve live credentials for each stream, while [`crate::CredentialStore`]
//! owns persisted credential IO and redacted availability probes. OAuth flows
//! populate or refresh stored credentials outside this collection; the
//! collection does not perform login.
//!
//! # Complete-dispatch decision
//!
//! The current [`Provider`] trait is streaming-only.
//! Rather than adding a second trait method (which would touch every provider
//! adapter), complete dispatch is implemented by draining the stream returned
//! by [`Provider::stream`] to its terminal event. This keeps the decision
//! compatible with the existing streaming contract.
//!
//! # Unstable
//!
//! This surface is part of the **unstable 0.x extension substrate**. Breaking
//! changes may occur between minor versions without a major version bump.

use std::collections::HashMap;

use futures_util::StreamExt;

use crate::credential::CredentialSource;
use crate::message::AssistantMessage;
use crate::provider::{EventStream, ModelInfo, Provider, ProviderError, Request};
use crate::registry::{ModelCapabilities, ProviderRegistry, RegistrationError, RegistryError};
use crate::stream::{AssistantStreamEvent, StopReason};

// ---------------------------------------------------------------------------
// SecretKey — redacted credential value
// ---------------------------------------------------------------------------

/// An API key value that never reveals itself in debug/display output.
///
/// When the collection stores a raw credential, the value is always rendered
/// as `<redacted>` when formatted. Callers that need the raw value use
/// [`SecretKey::as_str`].
#[derive(Clone)]
pub struct SecretKey(String);

impl SecretKey {
    /// Wrap a raw credential value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Access the raw value programmatically.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the key is non-empty.
    pub fn is_present(&self) -> bool {
        !self.0.trim().is_empty()
    }
}

impl std::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}

impl std::fmt::Display for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}

// ---------------------------------------------------------------------------
// Auth contract
// ---------------------------------------------------------------------------

/// Provider-owned, redacted auth metadata for collection diagnostics and gates.
///
/// Describes how a provider's credential is sourced without leaking the secret
/// itself. Current variants cover raw static API keys, env-described API keys,
/// already-resolved credentials whose non-secret source can be named, and
/// secret-free references to credential-store entries. Live per-stream
/// resolution is owned by [`crate::AuthResolver`], and persisted credential IO
/// is owned by [`crate::CredentialStore`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum AuthDescriptor {
    /// A static API key value held by the collection. The key itself is
    /// redacted in all diagnostics; only its presence or absence is reported.
    StaticApiKey {
        /// Redacted at debug/display time; never surfaced in diagnostics.
        value: SecretKey,
    },
    /// An API key resolved from an environment variable at dispatch time.
    EnvApiKey {
        /// Name of the environment variable (e.g. `ANTHROPIC_API_KEY`).
        env_var: String,
    },
    /// Credentials have already been resolved by provider-specific logic.
    ///
    /// `source` is a non-secret label such as `env OPENAI_API_KEY` or
    /// `aws credential chain`; it is safe to show in diagnostics.
    Resolved {
        /// Non-secret credential source label.
        source: String,
    },
    /// A credential sourced from the OS keychain via the credential store.
    ///
    /// Secret-free: `key` is the store account key (usually the provider id)
    /// and `display_source` is a non-secret label shown in diagnostics. The
    /// descriptor itself cannot perform IO, so the redacted
    /// [`CredentialSource`] probe state is injected separately via
    /// [`ProviderCollection::set_probe`] and consulted by
    /// [`ProviderCollection::dispatch_stream`].
    StoreCredential {
        /// Store account key (typically the provider id).
        key: String,
        /// Non-secret display label (e.g. `keychain opi:anthropic`).
        display_source: String,
    },
}

impl AuthDescriptor {
    /// Resolve the descriptor to a redacted [`AuthStatus`] at dispatch time.
    ///
    /// The returned `source` text names the reason (for example the env var
    /// name) but never contains a credential value.
    pub fn resolve(&self) -> AuthStatus {
        match self {
            AuthDescriptor::StaticApiKey { value } => {
                if value.is_present() {
                    AuthStatus::Configured
                } else {
                    AuthStatus::Missing {
                        source: "static api key is empty".to_owned(),
                    }
                }
            }
            AuthDescriptor::EnvApiKey { env_var } => match std::env::var(env_var) {
                Ok(value) if !value.trim().is_empty() => AuthStatus::Configured,
                Ok(_) => AuthStatus::Missing {
                    source: format!("env var {env_var} is set but empty"),
                },
                Err(std::env::VarError::NotPresent) => AuthStatus::Missing {
                    source: format!("env var {env_var} is not set"),
                },
                Err(std::env::VarError::NotUnicode(_)) => AuthStatus::Missing {
                    source: format!("env var {env_var} is not valid unicode"),
                },
            },
            AuthDescriptor::Resolved { source } => {
                if source.trim().is_empty() {
                    AuthStatus::Missing {
                        source: "resolved auth source is empty".to_owned(),
                    }
                } else {
                    AuthStatus::Configured
                }
            }
            // Not authoritative: the secret-free descriptor cannot perform IO,
            // so the redacted probe state is injected via set_probe and gated in
            // dispatch_stream. Returning Configured here keeps auth_status()
            // non-blocking for the variant; dispatch_stream never consults
            // auth_status() for StoreCredential providers (see its hoisted gate).
            AuthDescriptor::StoreCredential { .. } => AuthStatus::Configured,
        }
    }
}

/// Resolution of an [`AuthDescriptor`] at a point in time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthStatus {
    /// Credential is present and non-empty.
    Configured,
    /// No credential found. `source` names the origin (e.g. env var name)
    /// without leaking any value.
    Missing { source: String },
}

// ---------------------------------------------------------------------------
// OpenAI-compatible compatibility metadata
// ---------------------------------------------------------------------------

/// Collection-level home for OpenAI-compatible profile flags.
///
/// Profile compatibility flags live alongside model metadata in the
/// collection instead of being scattered across factory call sites.
#[derive(Debug, Clone, Default)]
pub struct CompatMetadata {
    /// Whether the provider speaks an OpenAI-compatible Chat Completions API.
    pub openai_compatible: bool,
    /// Free-form profile label (e.g. `"openrouter"`, `"mistral"`) for
    /// diagnostics.
    pub profile: Option<String>,
}

// ---------------------------------------------------------------------------
// Completion result
// ---------------------------------------------------------------------------

/// Result of draining a provider stream to completion.
///
/// See the [module docs](self) for the complete-dispatch decision.
#[derive(Debug, Clone)]
pub enum CompletedRequest {
    /// Stream terminated with [`AssistantStreamEvent::Done`].
    Done {
        reason: StopReason,
        message: AssistantMessage,
    },
    /// Stream terminated with [`AssistantStreamEvent::Error`].
    Error {
        reason: StopReason,
        message: AssistantMessage,
    },
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Error type for provider collection operations.
#[derive(Debug, thiserror::Error)]
pub enum CollectionError {
    /// A registry lookup failed.
    #[error(transparent)]
    Registry(#[from] RegistryError),
    /// Dispatch was rejected because auth is not configured for the provider.
    ///
    /// `source` is redacted and never carries a credential value.
    #[error("auth not configured for provider '{provider}': {detail}")]
    AuthNotConfigured {
        /// Provider id whose auth is missing.
        provider: String,
        /// Redacted description of the missing auth source.
        detail: String,
    },
    /// A provider stream failed while draining to completion.
    #[error(transparent)]
    Provider(#[from] ProviderError),
}

// ---------------------------------------------------------------------------
// ProviderCollection
// ---------------------------------------------------------------------------

/// A collection of providers/models that owns provider+model lookup, auth
/// resolution, compatibility metadata, and stream/complete dispatch.
///
/// Wraps a [`ProviderRegistry`] and layers the auth/collection contract on top
/// so provider paths share one lookup surface.
pub struct ProviderCollection {
    registry: ProviderRegistry,
    auth: HashMap<String, AuthDescriptor>,
    compat: HashMap<String, CompatMetadata>,
    /// Redacted, precomputed probe state for [`AuthDescriptor::StoreCredential`]
    /// providers. The secret-free descriptor cannot perform IO, so the async
    /// outer command path probes the store and injects the result here;
    /// [`ProviderCollection::dispatch_stream`] consults it.
    probed: HashMap<String, CredentialSource>,
}

impl ProviderCollection {
    /// Construct an empty collection.
    pub fn new() -> Self {
        Self {
            registry: ProviderRegistry::new(),
            auth: HashMap::new(),
            compat: HashMap::new(),
            probed: HashMap::new(),
        }
    }

    /// Wrap an existing registry to layer collection semantics onto providers
    /// constructed by an outer factory.
    ///
    /// Pre-registered providers have no auth descriptor until one is attached,
    /// so [`ProviderCollection::auth_status`] returns `None` for them and
    /// dispatch is not auth-gated.
    pub fn from_registry(registry: ProviderRegistry) -> Self {
        Self {
            registry,
            auth: HashMap::new(),
            compat: HashMap::new(),
            probed: HashMap::new(),
        }
    }

    /// Register a provider with its auth descriptor and compatibility metadata.
    ///
    /// Replaces any existing entry with the same provider id.
    ///
    /// # Errors
    ///
    /// Propagates [`RegistrationError::EmptyProviderId`] from the registry.
    pub fn register(
        &mut self,
        provider: Box<dyn Provider>,
        auth: AuthDescriptor,
        compat: CompatMetadata,
    ) -> Result<(), RegistrationError> {
        let id = provider.id().to_owned();
        self.registry.register_provider(provider)?;
        self.auth.insert(id.clone(), auth);
        self.compat.insert(id, compat);
        Ok(())
    }

    /// Access the underlying registry (for `--list-models`, overrides, etc.).
    pub fn registry(&self) -> &ProviderRegistry {
        &self.registry
    }

    /// Return sorted registered provider ids.
    pub fn provider_ids(&self) -> Vec<&str> {
        self.registry.provider_ids()
    }

    /// Resolve a `provider:model` spec into provider reference + model info.
    pub fn resolve(&self, spec: &str) -> Result<(&dyn Provider, &ModelInfo), RegistryError> {
        self.registry.resolve(spec)
    }

    /// Query capabilities for a `provider:model` spec.
    pub fn capabilities(&self, spec: &str) -> Result<ModelCapabilities, RegistryError> {
        self.registry.capabilities(spec)
    }

    /// The auth descriptor associated with a provider, if any.
    pub fn auth_descriptor(&self, provider_id: &str) -> Option<&AuthDescriptor> {
        self.auth.get(provider_id)
    }

    /// Resolve the current redacted auth status for a provider, if the
    /// collection owns an auth descriptor for it.
    ///
    /// Not authoritative for [`AuthDescriptor::StoreCredential`]: that variant
    /// always resolves to [`AuthStatus::Configured`] here because the
    /// secret-free descriptor cannot perform IO. The redacted probe state
    /// injected via [`Self::set_probe`] is consulted separately in
    /// [`Self::dispatch_stream`].
    pub fn auth_status(&self, provider_id: &str) -> Option<AuthStatus> {
        self.auth.get(provider_id).map(AuthDescriptor::resolve)
    }

    /// Inject the redacted probe state for a [`AuthDescriptor::StoreCredential`]
    /// provider. The async outer command path (doctor / `--list-models`) probes
    /// the store and passes the resulting [`CredentialSource`] here so the
    /// secret-free descriptor does not have to perform IO.
    pub fn set_probe(&mut self, provider_id: &str, source: CredentialSource) {
        self.probed.insert(provider_id.to_owned(), source);
    }

    /// The redacted probe state previously injected for a StoreCredential
    /// provider, if any.
    pub fn probe(&self, provider_id: &str) -> Option<&CredentialSource> {
        self.probed.get(provider_id)
    }

    /// The compatibility metadata associated with a provider, if any.
    pub fn compat(&self, provider_id: &str) -> Option<&CompatMetadata> {
        self.compat.get(provider_id)
    }

    /// Resolve a spec, validate its auth, and return a provider stream.
    ///
    /// Dispatch is auth-gated only for providers the collection owns an auth
    /// descriptor for. A [`AuthStatus::Missing`] descriptor yields a redacted
    /// [`CollectionError::AuthNotConfigured`] before the provider is touched.
    ///
    /// [`AuthDescriptor::StoreCredential`] providers are gated on the injected
    /// [`CredentialSource`] (see [`Self::set_probe`]): `Absent` rejects, while
    /// `Present` and `BackendUnavailable` proceed. `BackendUnavailable`
    /// proceeds intentionally — keychain-required enforcement is the
    /// responsibility of the live per-stream resolver, not this deliberately
    /// non-live status gate.
    /// A StoreCredential provider with no injected probe is treated as
    /// non-gated, matching `from_registry`'s "no descriptor" semantics.
    pub fn dispatch_stream(
        &self,
        spec: &str,
        request: Request,
    ) -> Result<EventStream, CollectionError> {
        let (provider, _) = self.registry.resolve(spec)?;
        let id = provider.id();
        if let Some(AuthDescriptor::StoreCredential { display_source, .. }) = self.auth.get(id) {
            // Secret-free descriptor: consult the injected probe, not resolve().
            if matches!(self.probed.get(id), Some(CredentialSource::Absent)) {
                return Err(CollectionError::AuthNotConfigured {
                    provider: id.to_owned(),
                    detail: display_source.clone(),
                });
            }
            // Present | BackendUnavailable | no probe injected -> proceed.
        } else if let Some(AuthStatus::Missing { source }) = self.auth_status(id) {
            return Err(CollectionError::AuthNotConfigured {
                provider: id.to_owned(),
                detail: source,
            });
        }
        Ok(provider.stream(request))
    }

    /// Drain a provider stream to its terminal event.
    ///
    /// This is the explicit complete-dispatch decision: complete dispatch is
    /// built on top of the streaming [`Provider`] trait rather than a separate
    /// trait method. See the [module docs](self).
    ///
    /// Auth gating is identical to [`ProviderCollection::dispatch_stream`].
    pub async fn dispatch_complete(
        &self,
        spec: &str,
        request: Request,
    ) -> Result<CompletedRequest, CollectionError> {
        let stream = self.dispatch_stream(spec, request)?;
        Ok(drain_to_completion(stream).await?)
    }

    /// Refresh provider-side model catalogs.
    ///
    /// Calls [`Provider::refresh_models`] on every registered provider in
    /// deterministic (sorted) id order. Collects all results: if every
    /// provider succeeds, atomically replaces the registry-owned dynamic
    /// catalogs. If **any** provider returns an error, the last-known
    /// catalogs are left unchanged and the first error is returned.
    ///
    /// Refresh results are replace-all snapshots: `Ok(Some(models))` replaces
    /// that provider's prior dynamic catalog, while `Ok(None)` clears any prior
    /// dynamic snapshot and exposes the provider's built-in models. Repeated
    /// refreshes replace rather than append.
    pub async fn refresh(&mut self) -> Result<(), CollectionError> {
        let ids = self.registry.provider_ids();
        let ids: Vec<String> = ids.into_iter().map(|s| s.to_owned()).collect();

        // Collect all results first (no mutation until every provider succeeds).
        let mut new_catalogs: HashMap<String, Vec<ModelInfo>> = HashMap::new();
        for id in &ids {
            let provider = match self.registry.get_provider(id) {
                Some(p) => p,
                None => continue,
            };
            match provider.refresh_models().await {
                Ok(Some(models)) => {
                    new_catalogs.insert(id.clone(), models);
                }
                Ok(None) => {
                    // No dynamic snapshot for this provider in the replacement.
                }
                Err(err) => {
                    // Atomic rollback: leave last-known catalogs unchanged.
                    return Err(CollectionError::Provider(err));
                }
            }
        }

        // All succeeded — atomically replace.
        self.registry.replace_all_dynamic_catalogs(new_catalogs);
        Ok(())
    }
}

impl Default for ProviderCollection {
    fn default() -> Self {
        Self::new()
    }
}

/// Drain an event stream until it yields a terminal event or errors.
///
/// A stream that ends without a terminal event is treated as a stream error.
async fn drain_to_completion(mut stream: EventStream) -> Result<CompletedRequest, ProviderError> {
    while let Some(item) = stream.next().await {
        match item {
            Ok(AssistantStreamEvent::Done { reason, message }) => {
                return Ok(CompletedRequest::Done { reason, message });
            }
            Ok(AssistantStreamEvent::Error { reason, message }) => {
                return Ok(CompletedRequest::Error { reason, message });
            }
            Ok(_) => continue,
            Err(error) => return Err(error),
        }
    }
    Err(ProviderError::StreamError(
        "stream ended without a terminal event".to_owned(),
    ))
}
