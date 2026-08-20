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
//! resolve live credentials once per prepared logical call, while
//! [`crate::CredentialStore`] owns persisted credential IO and redacted
//! availability probes. OAuth flows populate or refresh stored credentials
//! outside this collection; the collection does not perform login.
//!
//! # Complete-dispatch decision
//!
//! The current [`Provider`] trait is streaming-only.
//! Rather than adding a second trait method (which would touch every provider
//! adapter), complete dispatch is implemented by draining the stream returned
//! by [`Provider::stream_prepared`] to its terminal event. This keeps the
//! decision compatible with the existing streaming contract.
//!
//! # Unstable
//!
//! This surface is part of the **unstable 0.x extension substrate**. Breaking
//! changes may occur between minor versions without a major version bump.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};

use futures_util::{Stream, StreamExt};

use crate::auth::{AuthFallback, AuthProvenance, AuthProvenanceSource, AuthResolver, ResolvedAuth};
use crate::credential::CredentialSource;
use crate::message::AssistantMessage;
use crate::provider::{
    EventStream, ModelInfo, Provider, ProviderError, ProviderErrorSummary, Request,
};
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
/// secret-free references to credential-store entries. Live per-call
/// resolution is owned by [`crate::AuthResolver`] and coordinated by
/// [`ProviderCollection::prepare_call`]; persisted credential IO is owned by
/// [`crate::CredentialStore`].
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
    /// [`ProviderCollection::set_probe`] and read back via
    /// [`ProviderCollection::probe`] for listing/auth-status display.
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
            // so the redacted probe state is injected via set_probe and read
            // back via probe() for listing/auth-status display. Returning
            // Configured here keeps auth_status() non-blocking for the variant.
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
    /// The resolved provider has no dispatchable route: it was registered
    /// without a live [`AuthResolver`], so
    /// [`ProviderCollection::prepare_call`] cannot prepare authentication for it.
    #[error("provider '{provider}' has no dispatchable route")]
    RouteNotDispatchable {
        /// Provider id whose route is not dispatchable.
        provider: String,
    },
    /// The request identifies a different provider/model than the resolved
    /// route, so dispatch would make the frozen route facts untruthful.
    #[error(
        "request model '{request_model}' does not match resolved route '{route_provider}:{route_model}'"
    )]
    RequestRouteMismatch {
        /// Model identity supplied by the request.
        request_model: String,
        /// Provider id resolved from the route specification.
        route_provider: String,
        /// Model id resolved from the route specification.
        route_model: String,
    },
    /// [`start_attempt`](PreparedProviderCall::start_attempt) was called while a
    /// previous attempt stream is still active. At most one attempt may be
    /// active for a prepared call; a sequential retry must wait until the prior
    /// attempt's stream reaches a terminal event.
    #[error("an attempt is already active for this prepared call")]
    AttemptAlreadyActive,
    /// The prepared call's shared cancellation token was cancelled, which
    /// terminates the logical call and forbids any further attempt.
    #[error("the prepared call was cancelled")]
    CallCancelled,
    /// A prior attempt reported that the frozen credential is unavailable or
    /// revoked, so the prepared call cannot safely dispatch it again.
    #[error("credential failure terminated the prepared call for provider '{provider}'")]
    CredentialTerminated {
        /// Provider whose frozen credential entered the terminal state.
        provider: String,
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
    /// [`ProviderCollection::probe`] exposes it for listing/auth-status display.
    probed: HashMap<String, CredentialSource>,
    /// Per-route live auth resolvers. A route
    /// is dispatchable via [`prepare_call`](Self::prepare_call) only when it has
    /// a resolver here; legacy metadata-only registration leaves this absent.
    resolvers: HashMap<String, Arc<dyn AuthResolver>>,
    /// Per-route non-secret source classification used to build auth provenance.
    sources: HashMap<String, AuthProvenanceSource>,
}

impl ProviderCollection {
    /// Construct an empty collection.
    pub fn new() -> Self {
        Self {
            registry: ProviderRegistry::new(),
            auth: HashMap::new(),
            compat: HashMap::new(),
            probed: HashMap::new(),
            resolvers: HashMap::new(),
            sources: HashMap::new(),
        }
    }

    /// Wrap an existing registry to layer collection semantics onto providers
    /// constructed by an outer factory.
    ///
    /// Pre-registered providers are lookup-only until replaced by
    /// [`register_route`](Self::register_route):
    /// [`ProviderCollection::auth_status`] returns `None` and
    /// [`prepare_call`](Self::prepare_call) returns
    /// [`CollectionError::RouteNotDispatchable`].
    pub fn from_registry(registry: ProviderRegistry) -> Self {
        Self {
            registry,
            auth: HashMap::new(),
            compat: HashMap::new(),
            probed: HashMap::new(),
            resolvers: HashMap::new(),
            sources: HashMap::new(),
        }
    }

    /// Register a lookup-only provider with its redacted auth descriptor and
    /// compatibility metadata.
    ///
    /// Replaces any existing entry with the same provider id and removes its
    /// dispatch resolver. Use [`register_route`](Self::register_route) for a
    /// route that may start model attempts.
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
        self.resolvers.remove(&id);
        self.sources.remove(&id);
        self.probed.remove(&id);
        self.auth.insert(id.clone(), auth);
        self.compat.insert(id, compat);
        Ok(())
    }

    /// Register a dispatchable route: a concrete provider plus its per-request
    /// [`AuthResolver`] and non-secret source classification.
    ///
    /// The provider is registered in the underlying registry; the resolver and
    /// source classification are retained so
    /// [`prepare_call`](Self::prepare_call) can resolve and freeze authentication
    /// for the route. Replaces any existing route with the same provider id.
    ///
    /// # Errors
    ///
    /// Propagates [`RegistrationError::EmptyProviderId`] from the registry.
    pub fn register_route(
        &mut self,
        provider: Box<dyn Provider>,
        resolver: Arc<dyn AuthResolver>,
        source: AuthProvenanceSource,
        compat: CompatMetadata,
    ) -> Result<(), RegistrationError> {
        let id = provider.id().to_owned();
        self.registry.register_provider(provider)?;
        self.auth.remove(&id);
        self.probed.remove(&id);
        self.resolvers.insert(id.clone(), resolver);
        self.sources.insert(id.clone(), source);
        self.compat.insert(id, compat);
        Ok(())
    }

    /// Resolve one canonical `provider:model` route, validate the request,
    /// resolve authentication once, and freeze an opaque
    /// [`PreparedProviderCall`].
    ///
    /// Sequential attempts from the prepared call reuse the same frozen route,
    /// request, and authentication without repeating preparation. Route lookup,
    /// capability/wire validation, and auth preparation each precede
    /// model-request dispatch in that order; a failure at any step returns a
    /// typed error without selecting another provider, model, wire, credential
    /// policy, or local implementation.
    pub async fn prepare_call(
        &self,
        spec: &str,
        request: Request,
    ) -> Result<PreparedProviderCall, CollectionError> {
        let (provider_ref, model) = self.registry.resolve(spec)?;
        let provider_id = provider_ref.id().to_owned();
        let model_id = model.id.clone();
        let wire_api = model.wire_api;
        let canonical_request_model = format!("{provider_id}:{model_id}");
        if request.model != canonical_request_model {
            return Err(CollectionError::RequestRouteMismatch {
                request_model: request.model.clone(),
                route_provider: provider_id,
                route_model: model_id,
            });
        }
        crate::provider::validate_request_for_model(&provider_id, Some(model), &request)?;

        let resolver = self
            .resolvers
            .get(&provider_id)
            .ok_or_else(|| CollectionError::RouteNotDispatchable {
                provider: provider_id.clone(),
            })?
            .clone();
        let source = self
            .sources
            .get(&provider_id)
            .cloned()
            .unwrap_or(AuthProvenanceSource::Static);

        if request.cancel.is_cancelled() {
            return Err(CollectionError::CallCancelled);
        }
        let mut auth = tokio::select! {
            biased;
            _ = request.cancel.cancelled() => return Err(CollectionError::CallCancelled),
            resolved = resolver.resolve() => resolved?,
        };
        // Static/legacy resolvers return the default and rely on route assembly
        // for their non-secret source classification. A resolver that made a
        // real source/fallback decision reports it directly and must not have
        // that truthful result overwritten by collection defaults.
        if auth.provenance == AuthProvenance::default() {
            auth.provenance = AuthProvenance {
                source,
                fallback: AuthFallback::NotAttempted,
            };
        }
        let provider = self
            .registry
            .get_provider_arc(&provider_id)
            .ok_or_else(|| {
                CollectionError::Registry(RegistryError::UnknownProvider(provider_id.clone()))
            })?;
        let route = PreparedRoute {
            provider_id,
            model_id,
            wire_api,
        };
        Ok(PreparedProviderCall {
            provider,
            request,
            auth,
            route,
            active: Arc::new(AtomicBool::new(false)),
            credential_terminal: Arc::new(AtomicBool::new(false)),
        })
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

    /// Validate that `spec` resolves to a provider/model route backed by a live
    /// authentication resolver, without performing authentication I/O.
    pub fn validate_dispatchable_route(&self, spec: &str) -> Result<(), CollectionError> {
        let (provider, _) = self.registry.resolve(spec)?;
        let provider_id = provider.id();
        if !self.resolvers.contains_key(provider_id) {
            return Err(CollectionError::RouteNotDispatchable {
                provider: provider_id.to_owned(),
            });
        }
        Ok(())
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
    /// injected via [`Self::set_probe`] is read back via [`Self::probe`] for
    /// listing/auth-status display.
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
        let mut first_error = None;
        for id in &ids {
            let provider = match self.registry.get_provider(id) {
                Some(p) => p,
                None => continue,
            };
            match provider.refresh_models().await {
                Ok(Some(models)) => {
                    // Validate the candidate before it can replace the live
                    // catalog: a malformed or duplicate refresh must preserve
                    // the last-known catalog (mirrors register_model checks).
                    let mut seen = std::collections::HashSet::new();
                    let validation = models.iter().try_for_each(|model| {
                        if model.id.is_empty() {
                            return Err(CollectionError::Provider(ProviderError::Config(
                                ProviderErrorSummary::sanitized(format!(
                                    "dynamic catalog for provider '{id}' contains a model with an empty id"
                                )),
                            )));
                        }
                        if !seen.insert(model.id.as_str()) {
                            return Err(CollectionError::Provider(ProviderError::Config(
                                ProviderErrorSummary::sanitized(format!(
                                    "dynamic catalog for provider '{id}' has duplicate model id '{}'",
                                    model.id
                                )),
                            )));
                        }
                        if let Err(source) = model.validate() {
                            return Err(CollectionError::Provider(ProviderError::Config(
                                ProviderErrorSummary::sanitized(format!(
                                    "dynamic catalog for provider '{id}' has invalid model '{}': {source}",
                                    model.id
                                )),
                            )));
                        }
                        Ok(())
                    });
                    match validation {
                        Ok(()) => {
                            new_catalogs.insert(id.clone(), models);
                        }
                        Err(error) => {
                            first_error.get_or_insert(error);
                        }
                    }
                }
                Ok(None) => {
                    // No dynamic snapshot for this provider in the replacement.
                }
                Err(err) => {
                    first_error.get_or_insert(CollectionError::Provider(err));
                }
            }
        }

        if let Some(error) = first_error {
            return Err(error);
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
/// A general stream-drain utility: drive any [`EventStream`] (for example the
/// one returned by [`PreparedProviderCall::start_attempt`]) until it produces a
/// terminal event. A stream that ends without a terminal event is treated as a
/// stream error.
pub async fn drain_to_completion(
    mut stream: EventStream,
) -> Result<CompletedRequest, ProviderError> {
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
        ProviderErrorSummary::attested_static("stream ended without a terminal event"),
    ))
}

// ---------------------------------------------------------------------------
// Prepared call
// ---------------------------------------------------------------------------

/// Immutable, redacted facts about a resolved provider route.
///
/// Contains no secret material; the resolved secret and its non-secret
/// provenance both stay private to [`PreparedProviderCall`].
#[derive(Debug, Clone)]
pub struct PreparedRoute {
    /// Canonical provider id of the resolved route.
    pub provider_id: String,
    /// Canonical model id of the resolved route.
    pub model_id: String,
    /// Provider wire/API kind of the resolved route.
    pub wire_api: crate::model_info::WireApi,
}

/// An opaque prepared provider call.
///
/// Privately freezes the resolved provider, the request, and the secret-bearing
/// resolved authentication. [`start_attempt`](Self::start_attempt) reuses the
/// frozen route, request, and authentication for each permitted sequential
/// retry without repeating route/auth preparation. The secret never enters
/// [`PreparedRoute`], Agent-visible state, evidence, diagnostics, or
/// model-visible state.
pub struct PreparedProviderCall {
    provider: Arc<dyn Provider>,
    request: Request,
    auth: ResolvedAuth,
    route: PreparedRoute,
    /// At-most-one-active-attempt guard, cleared when an attempt stream reaches
    /// a terminal event or error.
    active: Arc<AtomicBool>,
    /// Set when an attempt reports that the frozen credential is unavailable
    /// or revoked; later attempts fail closed instead of redispatching it.
    credential_terminal: Arc<AtomicBool>,
}

struct AttemptStream {
    inner: EventStream,
    active: Arc<AtomicBool>,
    credential_terminal: Arc<AtomicBool>,
    released: bool,
}

impl AttemptStream {
    fn release(&mut self) {
        if !self.released {
            self.active.store(false, Ordering::Release);
            self.released = true;
        }
    }
}

impl Stream for AttemptStream {
    type Item = Result<AssistantStreamEvent, ProviderError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let item = this.inner.as_mut().poll_next(cx);
        if matches!(
            &item,
            Poll::Ready(Some(Err(
                ProviderError::CredentialNeeded { .. } | ProviderError::CredentialRevoked { .. }
            )))
        ) {
            this.credential_terminal.store(true, Ordering::Release);
        }
        let terminal = match &item {
            Poll::Ready(None) => true,
            Poll::Ready(Some(Err(_))) => true,
            Poll::Ready(Some(Ok(event))) => event.is_terminal(),
            Poll::Pending => false,
        };
        if terminal {
            this.release();
        }
        item
    }
}

impl Drop for AttemptStream {
    fn drop(&mut self) {
        self.release();
    }
}

impl std::fmt::Debug for PreparedProviderCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never surface the secret-bearing auth; only the redacted route facts.
        f.debug_struct("PreparedProviderCall")
            .field("route", &self.route)
            .field("auth", &"<redacted>")
            .finish()
    }
}

impl PreparedProviderCall {
    /// The immutable, redacted route facts for this prepared call.
    pub fn route(&self) -> &PreparedRoute {
        &self.route
    }

    /// The non-secret auth source classification plus fallback decision for the
    /// resolved authentication.
    ///
    /// Provenance is carried beside the secret on the private
    /// [`ResolvedAuth`]; this redacted accessor is the only public read path,
    /// so callers and evidence can distinguish auth sources without seeing the
    /// secret.
    pub fn auth_provenance(&self) -> &AuthProvenance {
        &self.auth.provenance
    }

    /// Start one attempt using the frozen request and authentication.
    ///
    /// Reuses the prepared route and authentication; does not repeat preparation.
    /// Every attempt shares the frozen [`Request::cancel`] token: cancelling it
    /// terminates the logical call and forbids any further attempt. At most one
    /// attempt stream may be active at a time; a sequential retry must wait until
    /// the prior attempt's stream reaches a terminal event or error, which
    /// releases the active slot. A credential-needed or credential-revoked
    /// stream error additionally terminates the prepared call and forbids retry.
    ///
    /// # Errors
    ///
    /// - [`CollectionError::CallCancelled`] if the shared cancellation token has
    ///   been cancelled.
    /// - [`CollectionError::CredentialTerminated`] if a prior attempt rejected
    ///   the frozen credential.
    /// - [`CollectionError::AttemptAlreadyActive`] if a prior attempt stream is
    ///   still active.
    pub fn start_attempt(&self) -> Result<EventStream, CollectionError> {
        if self.request.cancel.is_cancelled() {
            return Err(CollectionError::CallCancelled);
        }
        if self.credential_terminal.load(Ordering::Acquire) {
            return Err(CollectionError::CredentialTerminated {
                provider: self.route.provider_id.clone(),
            });
        }
        if self.active.swap(true, Ordering::AcqRel) {
            return Err(CollectionError::AttemptAlreadyActive);
        }
        // Recheck after acquiring the slot so a concurrent terminal stream
        // cannot release it between the first check and this attempt.
        if self.credential_terminal.load(Ordering::Acquire) {
            self.active.store(false, Ordering::Release);
            return Err(CollectionError::CredentialTerminated {
                provider: self.route.provider_id.clone(),
            });
        }
        let stream = self
            .provider
            .stream_prepared(self.request.clone(), self.auth.clone());
        Ok(Box::pin(AttemptStream {
            inner: stream,
            active: self.active.clone(),
            credential_terminal: self.credential_terminal.clone(),
            released: false,
        }))
    }
}
