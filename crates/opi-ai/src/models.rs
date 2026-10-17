//! Single `Models` front door: catalog, redacted auth status, and chat calls.
//!
//! [`Models`] consolidates provider/model lookup, the provider-side auth
//! contract, OpenAI-compatible compatibility metadata, and every chat-call
//! entry into one owner (AIR-045). Ordinary calls — native
//! [`stream`](Models::stream)/[`complete`](Models::complete), neutral
//! [`stream_simple`](Models::stream_simple)/[`complete_simple`](Models::complete_simple),
//! auth-injected [`stream_direct`](Models::stream_direct), and advanced
//! [`prepare_call`](Models::prepare_call) — share the sole request compiler,
//! auth resolution, attempt seam, and `ChatStream`
//! collector; there is no parallel facade or router.
//!
//! # Auth descriptor, resolver, and store split
//!
//! [`AuthDescriptor`] carries redaction-safe metadata and supports
//! non-live dispatch gates. Concrete routes use [`crate::AuthResolver`] to
//! resolve live credentials once per logical call, while
//! [`crate::CredentialStore`] owns persisted credential IO and redacted
//! availability probes. OAuth flows populate or refresh stored credentials
//! outside this driver; the driver does not perform login.
//!
//! # Typed custom chat APIs
//!
//! Custom chat adapters register as an object-safe [`ChatApi<O>`] with one
//! fixed options type `O` and an exact [`ApiId`]. Registration installs a
//! catalog-invisible provider route (the record never appears in catalog
//! reads) plus the typed dispatch entry. A call created through any entry
//! freezes the caller's complete [`ChatModelRecord`], the captured adapter,
//! and the owned input at construction; authentication starts on the first
//! poll. Later registration changes never mutate an already-created call.
//!
//! # Unstable
//!
//! This surface is part of the **unstable 0.x extension substrate**. Breaking
//! changes may occur between minor versions without a major version bump.

use std::any::{Any, TypeId};
use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::future::BoxFuture;
use futures_util::{Stream, stream};
use tokio_util::sync::CancellationToken;

use crate::auth::{AuthFallback, AuthProvenance, AuthProvenanceSource, AuthResolver, ResolvedAuth};
use crate::credential::CredentialSource;
use crate::message::AssistantMessage;
use crate::provider::{
    EventStream, ModelInfo, Provider, ProviderError, ProviderErrorSummary, Request,
};
use crate::registry::{ModelCapabilities, ProviderRegistry, RegistrationError, RegistryError};
use crate::stream::{AssistantStreamEvent, CallFailure, ChatStream};

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
/// [`Models::prepare_call`]; persisted credential IO is owned by
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
    /// [`Models::set_probe`] and read back via
    /// [`Models::probe`] for listing/auth-status display.
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
// Errors
// ---------------------------------------------------------------------------

/// Error type for provider collection operations.
#[derive(Debug, thiserror::Error)]
pub enum ModelsError {
    /// A registry lookup failed.
    #[error(transparent)]
    Registry(#[from] RegistryError),
    /// The resolved provider has no dispatchable route: it was registered
    /// without a live [`AuthResolver`], so
    /// [`Models::prepare_call`] cannot prepare authentication for it.
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
// Models
// ---------------------------------------------------------------------------

/// A single driver of providers/models that owns provider+model lookup, auth
/// resolution, compatibility metadata, and every chat-call entry.
///
/// Wraps a [`ProviderRegistry`] and layers the auth/call contract on top so
/// every entry — native, simple, direct, and advanced — shares one state
/// machine. See the [module-level documentation](self) for the full seam
/// description.
pub struct Models {
    registry: ProviderRegistry,
    auth: HashMap<String, AuthDescriptor>,
    compat: HashMap<String, CompatMetadata>,
    /// Redacted, precomputed probe state for [`AuthDescriptor::StoreCredential`]
    /// providers. The secret-free descriptor cannot perform IO, so the async
    /// outer command path probes the store and injects the result here;
    /// [`Models::probe`] exposes it for listing/auth-status display.
    probed: HashMap<String, CredentialSource>,
    /// Per-route live auth resolvers. A route
    /// is dispatchable only when it has a resolver here; legacy metadata-only
    /// registration leaves this absent.
    resolvers: HashMap<String, Arc<dyn AuthResolver>>,
    /// Per-route non-secret source classification used to build auth provenance.
    sources: HashMap<String, AuthProvenanceSource>,
    /// Typed custom chat API registrations keyed by exact [`ApiId`]. Entries
    /// are captured (cloned `Arc`s) at call creation, so later registration
    /// changes never mutate an existing call.
    chat_apis: BTreeMap<ApiId, ChatApiRegistration>,
}

impl Models {
    /// Construct an empty driver. Registers no provider and performs no
    /// env/auth/store/external IO.
    pub fn new() -> Self {
        Self {
            registry: ProviderRegistry::new(),
            auth: HashMap::new(),
            compat: HashMap::new(),
            probed: HashMap::new(),
            resolvers: HashMap::new(),
            sources: HashMap::new(),
            chat_apis: BTreeMap::new(),
        }
    }

    /// Wrap an existing registry to layer the driver semantics onto providers
    /// constructed by an outer factory.
    ///
    /// Pre-registered providers are lookup-only until replaced by
    /// [`register_route`](Self::register_route) or
    /// [`register_chat_api`](Self::register_chat_api):
    /// [`Models::auth_status`] returns `None` and
    /// [`prepare_call`](Self::prepare_call) returns
    /// [`ModelsError::RouteNotDispatchable`].
    pub fn from_registry(registry: ProviderRegistry) -> Self {
        Self {
            registry,
            auth: HashMap::new(),
            compat: HashMap::new(),
            probed: HashMap::new(),
            resolvers: HashMap::new(),
            sources: HashMap::new(),
            chat_apis: BTreeMap::new(),
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

    /// Register one typed custom chat adapter (AIR-045).
    ///
    /// The adapter is keyed by its exact [`ApiId`] with its fixed options type
    /// `O`. Registration installs a **catalog-invisible** provider route (the
    /// adapter's complete record never appears in `get_model`/`all_models`)
    /// plus the route's [`AuthResolver`], so every entry — native, simple,
    /// direct — reaches the same frozen binding and collector. Registering
    /// again with the same ApiId replaces the entry (compatible upsert);
    /// options created before the upsert keep working when the options type is
    /// unchanged and fail a typed pre-auth check when it is not.
    ///
    /// # Errors
    ///
    /// - [`RegistrationError::EmptyApiId`] if the ApiId is empty.
    /// - [`RegistrationError::EmptyProviderId`] if the provider id is empty.
    pub fn register_chat_api<O: Send + 'static>(
        &mut self,
        api: Arc<dyn ChatApi<O>>,
        resolver: Arc<dyn AuthResolver>,
        source: AuthProvenanceSource,
    ) -> Result<ChatApiHandle<O>, RegistrationError> {
        let api_id = api.api_id().clone();
        if api_id.as_str().trim().is_empty() {
            return Err(RegistrationError::EmptyApiId);
        }
        let provider_id = api.provider_id().to_owned();
        if provider_id.trim().is_empty() {
            return Err(RegistrationError::EmptyProviderId);
        }
        let dispatch: Arc<dyn ChatApiDispatch> = Arc::new(TypedChatApi { api });
        // Catalog-invisible route: a Provider with no models keeps the adapter
        // dispatchable through provider-route entries without listing it.
        self.registry
            .register_provider(Box::new(ChatApiRouteProvider {
                provider_id: provider_id.clone(),
                dispatch: Arc::clone(&dispatch),
            }))?;
        self.auth.remove(&provider_id);
        self.probed.remove(&provider_id);
        self.resolvers.insert(provider_id.clone(), resolver);
        self.sources.insert(provider_id.clone(), source);
        self.compat.remove(&provider_id);
        self.chat_apis.insert(
            api_id.clone(),
            ChatApiRegistration {
                provider_id,
                options_type: TypeId::of::<O>(),
                dispatch,
            },
        );
        Ok(ChatApiHandle {
            api_id,
            _options: PhantomData,
        })
    }

    /// Sync lookup of one owned catalog record (`provider`, bare model id).
    ///
    /// Returns a clone of the resolved record across the registry's override,
    /// dynamic-catalog, and built-in layers. Custom chat API records stay
    /// absent: registering a [`ChatApi`] never adds its record to any catalog
    /// read.
    pub fn get_model(&self, provider: &str, id: &str) -> Option<ModelInfo> {
        let spec = format!("{provider}:{id}");
        self.registry
            .resolve(&spec)
            .ok()
            .map(|(_, model)| model.clone())
    }

    fn route_auth_parts(
        &self,
        provider_id: &str,
    ) -> (Option<Arc<dyn AuthResolver>>, AuthProvenanceSource) {
        let resolver = self.resolvers.get(provider_id).cloned();
        let source = self
            .sources
            .get(provider_id)
            .cloned()
            .unwrap_or(AuthProvenanceSource::Static);
        (resolver, source)
    }

    fn chat_api_binding<O: Send + 'static>(
        &self,
        record: &ChatModelRecord,
        options: &NativeOptions<O>,
        auth_plan: AuthPlan,
    ) -> FrozenChatBinding {
        let entry = self
            .chat_apis
            .get(options.api_id())
            .map(|registration| ChatApiEntry {
                provider_id: registration.provider_id.clone(),
                options_type: registration.options_type,
                dispatch: Arc::clone(&registration.dispatch),
            });
        FrozenChatBinding {
            record: record.clone(),
            route: FrozenRoute::ChatApi {
                api_id: options.api_id().clone(),
                entry,
            },
            auth_plan,
        }
    }

    fn provider_binding(&self, record: &ChatModelRecord) -> FrozenChatBinding {
        let provider = self.registry.get_provider_arc(record.provider_id());
        FrozenChatBinding {
            record: record.clone(),
            route: FrozenRoute::Provider {
                provider_id: record.provider_id().to_owned(),
                provider,
            },
            auth_plan: AuthPlan::default_for(self, record.provider_id()),
        }
    }

    /// Native entry: start one chat call with the adapter's typed options.
    ///
    /// Construction captures the owned input, the caller's complete record,
    /// and the registered adapter behind the options' exact [`ApiId`] — no
    /// auth or external IO happens until the first poll. Type/provider/
    /// registration problems surface as one typed [`CallFailure`] before auth.
    pub fn stream<O: Send + 'static>(
        &self,
        record: ChatModelRecord,
        request: Request,
        options: NativeOptions<O>,
    ) -> ChatCall {
        let auth_plan = AuthPlan::default_for(self, record.provider_id());
        let binding = self.chat_api_binding(&record, &options, auth_plan);
        ChatCall::new(binding, request, options.into_call_options())
    }

    /// Native entry driven to the single final outcome.
    pub async fn complete<O: Send + 'static>(
        &self,
        record: ChatModelRecord,
        request: Request,
        options: NativeOptions<O>,
    ) -> Result<AssistantMessage, CallFailure> {
        self.stream(record, request, options).result().await
    }

    /// Neutral entry: start one chat call with common options only.
    ///
    /// Dispatches through the record's provider route (a registered
    /// [`ChatApi`] serves the route through its `simple_options` mapping).
    /// `Some` fields in `options` override the corresponding request fields
    /// before the shared compiler runs; everything else matches the native
    /// entry's frozen-binding and lazy-auth contract.
    pub fn stream_simple(
        &self,
        record: ChatModelRecord,
        mut request: Request,
        options: SimpleOptions,
    ) -> ChatCall {
        options.apply(&mut request);
        let binding = self.provider_binding(&record);
        ChatCall::new(binding, request, CallOptions::Simple)
    }

    /// Neutral entry driven to the single final outcome.
    pub async fn complete_simple(
        &self,
        record: ChatModelRecord,
        request: Request,
        options: SimpleOptions,
    ) -> Result<AssistantMessage, CallFailure> {
        self.stream_simple(record, request, options).result().await
    }

    /// Direct entry: one chat call on a registered adapter with injected,
    /// already-resolved authentication.
    ///
    /// The driver never consults its own resolvers for this call; the injected
    /// [`ResolvedAuth`] is frozen at construction exactly like the rest of the
    /// binding.
    pub fn stream_direct<O: Send + 'static>(
        &self,
        record: ChatModelRecord,
        request: Request,
        options: NativeOptions<O>,
        auth: ResolvedAuth,
    ) -> ChatCall {
        let binding = self.chat_api_binding(&record, &options, AuthPlan::Injected(Box::new(auth)));
        ChatCall::new(binding, request, options.into_call_options())
    }

    /// Direct entry driven to the single final outcome.
    pub async fn complete_direct<O: Send + 'static>(
        &self,
        record: ChatModelRecord,
        request: Request,
        options: NativeOptions<O>,
        auth: ResolvedAuth,
    ) -> Result<AssistantMessage, CallFailure> {
        self.stream_direct(record, request, options, auth)
            .result()
            .await
    }

    /// Resolve one canonical `provider:model` route, validate the request,
    /// resolve authentication once, and freeze an opaque
    /// [`PreparedProviderCall`] (advanced entry).
    ///
    /// Sequential attempts from the prepared call reuse the same frozen route,
    /// request, and authentication without repeating preparation. Route lookup,
    /// capability/wire validation, and auth preparation each precede
    /// model-request dispatch in that order; a failure at any step returns a
    /// typed error without selecting another provider, model, wire, credential
    /// policy, or local implementation. Ordinary entries reuse this exact
    /// compiler and auth resolution through [`ChatCall`].
    /// An omitted output limit resolves to the selected registry model's output
    /// capacity before authentication. An enabled thinking budget must fit
    /// below that default limit. Implicit and explicit ceilings are then clamped
    /// to remaining context with 4,096 safety tokens (minimum one output token).
    /// Thinking that no longer fits is reduced to leave 1,024 answer tokens
    /// where possible. The caller's request is not mutated.
    pub async fn prepare_call(
        &self,
        spec: &str,
        mut request: Request,
    ) -> Result<PreparedProviderCall, ModelsError> {
        let (provider_ref, model) = self.registry.resolve(spec)?;
        let provider_id = provider_ref.id().to_owned();
        let model_id = model.id.clone();
        let wire_api = model.wire_api;
        let canonical_request_model = format!("{provider_id}:{model_id}");
        if request.model != canonical_request_model {
            return Err(ModelsError::RequestRouteMismatch {
                request_model: request.model.clone(),
                route_provider: provider_id,
                route_model: model_id,
            });
        }
        compile_prepared_request(&provider_id, model, &mut request)?;

        let resolver = self
            .resolvers
            .get(&provider_id)
            .ok_or_else(|| ModelsError::RouteNotDispatchable {
                provider: provider_id.clone(),
            })?
            .clone();
        let source = self
            .sources
            .get(&provider_id)
            .cloned()
            .unwrap_or(AuthProvenanceSource::Static);

        let auth = resolve_route_auth(&resolver, source, &request.cancel).await?;
        let provider = self
            .registry
            .get_provider_arc(&provider_id)
            .ok_or_else(|| {
                ModelsError::Registry(RegistryError::UnknownProvider(provider_id.clone()))
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
    pub fn validate_dispatchable_route(&self, spec: &str) -> Result<(), ModelsError> {
        let (provider, _) = self.registry.resolve(spec)?;
        let provider_id = provider.id();
        if !self.resolvers.contains_key(provider_id) {
            return Err(ModelsError::RouteNotDispatchable {
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
    pub async fn refresh(&mut self) -> Result<(), ModelsError> {
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
                            return Err(ModelsError::Provider(ProviderError::Config(
                                ProviderErrorSummary::sanitized(format!(
                                    "dynamic catalog for provider '{id}' contains a model with an empty id"
                                )),
                            )));
                        }
                        if !seen.insert(model.id.as_str()) {
                            return Err(ModelsError::Provider(ProviderError::Config(
                                ProviderErrorSummary::sanitized(format!(
                                    "dynamic catalog for provider '{id}' has duplicate model id '{}'",
                                    model.id
                                )),
                            )));
                        }
                        if let Err(source) = model.validate() {
                            return Err(ModelsError::Provider(ProviderError::Config(
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
                    first_error.get_or_insert(ModelsError::Provider(err));
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

impl Default for Models {
    fn default() -> Self {
        Self::new()
    }
}

/// Drain an event stream to the single final outcome of the logical call.
///
/// A general stream-drain utility sharing the public seam with
/// [`ChatStream::result`](crate::stream::ChatStream::result): drives any
/// [`EventStream`] (for example the one returned by
/// [`PreparedProviderCall::start_attempt`]) through the same terminal
/// absorption, permanent-EOF and failure rules. `Done` terminals complete
/// `Ok` with the canonical message; every failure (in-band error terminal,
/// transport/protocol error, or a stream that ends without a terminal) drains
/// to exactly one `Err(CallFailure)` carrying the preserved partial and a
/// typed, redacted cause.
pub async fn drain_to_completion(
    stream: EventStream,
) -> Result<AssistantMessage, crate::stream::CallFailure> {
    crate::stream::ChatStream::new(stream).result().await
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
    /// - [`ModelsError::CallCancelled`] if the shared cancellation token has
    ///   been cancelled.
    /// - [`ModelsError::CredentialTerminated`] if a prior attempt rejected
    ///   the frozen credential.
    /// - [`ModelsError::AttemptAlreadyActive`] if a prior attempt stream is
    ///   still active.
    pub fn start_attempt(&self) -> Result<EventStream, ModelsError> {
        if self.request.cancel.is_cancelled() {
            return Err(ModelsError::CallCancelled);
        }
        if self.credential_terminal.load(Ordering::Acquire) {
            return Err(ModelsError::CredentialTerminated {
                provider: self.route.provider_id.clone(),
            });
        }
        if self.active.swap(true, Ordering::AcqRel) {
            return Err(ModelsError::AttemptAlreadyActive);
        }
        // Recheck after acquiring the slot so a concurrent terminal stream
        // cannot release it between the first check and this attempt.
        if self.credential_terminal.load(Ordering::Acquire) {
            self.active.store(false, Ordering::Release);
            return Err(ModelsError::CredentialTerminated {
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

// ---------------------------------------------------------------------------
// Shared compiler and auth resolution (the sole state machine)
// ---------------------------------------------------------------------------

/// The one request compiler shared by advanced preparation and every ordinary
/// entry: default output limit, context clamp, thinking resolution, and
/// capability/wire validation, in that order, without mutating the caller's
/// original `Request` (the compiler receives an owned copy).
fn compile_prepared_request(
    provider_id: &str,
    model: &ModelInfo,
    request: &mut Request,
) -> Result<(), ProviderError> {
    if request.max_tokens.is_none() {
        let limit = model.capabilities.max_output_tokens;
        if request.thinking.enabled
            && request
                .thinking
                .budget_tokens
                .is_some_and(|budget| budget >= limit)
        {
            return Err(ProviderError::UnsupportedCapability(
                ProviderErrorSummary::sanitized(
                    "thinking budget must be below the resolved model's default output limit",
                ),
            ));
        }
        request.max_tokens = Some(limit);
    }
    let context_limit = model.capabilities.context_window;
    if context_limit > 0 {
        let occupied = crate::context::estimate_context_tokens(
            request.system.as_deref(),
            &request.messages,
            &request.tools,
            &request.model,
        )
        .tokens;
        let available = context_limit
            .saturating_sub(occupied)
            .saturating_sub(4_096)
            .max(1);
        request.max_tokens = request.max_tokens.map(|limit| limit.min(available));
    }
    if request.thinking.enabled {
        let numeric_anthropic = matches!(
            &model.compat,
            crate::WireCompat::AnthropicMessages(compat) if !compat.force_adaptive_thinking
        );
        if numeric_anthropic && request.thinking.budget_tokens.is_none() {
            request.thinking.budget_tokens = Some(match request.thinking.level {
                crate::ThinkingLevel::Minimal => 1_024,
                crate::ThinkingLevel::Low => 2_048,
                crate::ThinkingLevel::Medium => 8_192,
                crate::ThinkingLevel::High
                | crate::ThinkingLevel::XHigh
                | crate::ThinkingLevel::Max => 16_384,
                // Preserve the adapter's fallback for legacy callers that
                // enable thinking without selecting a level or budget.
                crate::ThinkingLevel::None => 10_000,
            });
        }
        if let (Some(budget), Some(limit)) = (request.thinking.budget_tokens, request.max_tokens) {
            request.thinking.budget_tokens = Some(budget.min(limit.saturating_sub(1_024)));
        }
        if numeric_anthropic
            && request
                .thinking
                .budget_tokens
                .is_some_and(|budget| budget < 1_024)
        {
            return Err(ProviderError::UnsupportedCapability(
                ProviderErrorSummary::sanitized(
                    "response limit cannot fit the minimum Anthropic thinking budget and answer room",
                ),
            ));
        }
    }
    crate::provider::validate_request_for_model(provider_id, Some(model), request)
}

/// The one auth resolution shared by advanced preparation and every ordinary
/// entry: cancellation has priority over resolution, and a resolver that
/// reported no provenance decision inherits the registered route source.
async fn resolve_route_auth(
    resolver: &Arc<dyn AuthResolver>,
    source: AuthProvenanceSource,
    cancel: &CancellationToken,
) -> Result<ResolvedAuth, ModelsError> {
    if cancel.is_cancelled() {
        return Err(ModelsError::CallCancelled);
    }
    let mut auth = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(ModelsError::CallCancelled),
        resolved = resolver.resolve() => resolved?,
    };
    // Static/legacy resolvers return the default and rely on route assembly
    // for their non-secret source classification. A resolver that made a
    // real source/fallback decision reports it directly and must not have
    // that truthful result overwritten by route defaults.
    if auth.provenance == AuthProvenance::default() {
        auth.provenance = AuthProvenance {
            source,
            fallback: AuthFallback::NotAttempted,
        };
    }
    Ok(auth)
}

/// Map a preparation failure onto the typed cause a [`CallFailure`] carries
/// when the same step fails inside a lazy [`ChatCall`].
fn models_error_cause(error: ModelsError) -> ProviderError {
    match error {
        ModelsError::CallCancelled => ProviderError::Cancelled,
        ModelsError::RouteNotDispatchable { provider } => {
            ProviderError::Config(ProviderErrorSummary::sanitized(format!(
                "provider '{provider}' has no dispatchable route"
            )))
        }
        ModelsError::Provider(error) => error,
        ModelsError::Registry(error) => {
            ProviderError::Config(ProviderErrorSummary::sanitized(error.to_string()))
        }
        other => ProviderError::Config(ProviderErrorSummary::sanitized(other.to_string())),
    }
}

fn setup_failure(message: String) -> CallFailure {
    CallFailure::from_partial(
        None,
        ProviderError::Config(ProviderErrorSummary::sanitized(message)),
    )
}

// ---------------------------------------------------------------------------
// Typed custom chat API registration (AIR-045)
// ---------------------------------------------------------------------------

/// Exact, caller-chosen chat API identity.
///
/// Preserves the exact string identity (no normalization, length, or grammar
/// rules); two instances agree on an ApiId only when the strings are equal.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ApiId(String);

impl ApiId {
    /// Wrap an API identity string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The exact identity string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The complete chat record one ordinary call binds to: provider id, bare
/// model id, and the full [`ModelInfo`] (exact wire, capabilities, compat).
///
/// A record does not need catalog membership: calls dispatch on the record's
/// provider/wire exactly as the caller supplied it, and same-ID catalog data
/// never replaces a frozen record.
#[derive(Debug, Clone)]
pub struct ChatModelRecord {
    provider_id: String,
    info: ModelInfo,
}

impl ChatModelRecord {
    /// Build one complete record.
    pub fn new(provider_id: impl Into<String>, info: ModelInfo) -> Self {
        Self {
            provider_id: provider_id.into(),
            info,
        }
    }

    /// Owning provider id of the record.
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    /// Bare model id (no provider prefix).
    pub fn model_id(&self) -> &str {
        &self.info.id
    }

    /// Exact wire API of the record.
    pub fn wire_api(&self) -> crate::model_info::WireApi {
        self.info.wire_api
    }

    /// The full model metadata.
    pub fn info(&self) -> &ModelInfo {
        &self.info
    }

    fn spec(&self) -> String {
        format!("{}:{}", self.provider_id, self.info.id)
    }
}

/// Neutral options accepted by [`Models::stream_simple`] /
/// [`Models::complete_simple`].
///
/// `Some` fields override the corresponding request fields before the shared
/// compiler runs; `None` leaves the request untouched.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SimpleOptions {
    /// Sampling temperature override.
    pub temperature: Option<f64>,
    /// Output-limit override.
    pub max_tokens: Option<u64>,
    /// Per-call timeout override.
    pub timeout: Option<Duration>,
    /// Stop-sequence override (applied when non-empty).
    pub stop_sequences: Vec<String>,
}

impl SimpleOptions {
    fn apply(&self, request: &mut Request) {
        if self.temperature.is_some() {
            request.temperature = self.temperature;
        }
        if self.max_tokens.is_some() {
            request.max_tokens = self.max_tokens;
        }
        if self.timeout.is_some() {
            request.timeout = self.timeout;
        }
        if !self.stop_sequences.is_empty() {
            request.stop_sequences = self.stop_sequences.clone();
        }
    }
}

/// One custom chat adapter with its fixed, typed native options type `O`.
///
/// Object-safe: registrations hold `Arc<dyn ChatApi<O>>`. The adapter owns
/// its exact [`ApiId`], its provider route identity, its complete record, and
/// the neutral-to-native mapping used by the simple entry.
pub trait ChatApi<O: Send + 'static>: Send + Sync {
    /// The exact API identity this adapter serves.
    fn api_id(&self) -> &ApiId;

    /// The provider route this adapter owns.
    fn provider_id(&self) -> &str;

    /// The adapter's complete record. Serves documentation and callers that
    /// need a starting record; it never enters catalog reads.
    fn complete_record(&self) -> ModelInfo;

    /// The adapter's neutral options mapping used by [`Models::stream_simple`].
    fn simple_options(&self) -> O;

    /// Dispatch one prepared request with resolved authentication. This is
    /// the same prepared-auth seam [`Provider::stream_prepared`] uses; the
    /// adapter consumes the supplied [`ResolvedAuth`] directly and never
    /// consults a resolver.
    fn stream_prepared(&self, request: Request, options: O, auth: ResolvedAuth) -> EventStream;
}

/// Handle to one registered [`ChatApi`]; the only way to create its typed
/// [`NativeOptions`].
///
/// The handle carries no instance capability: its options stay valid for any
/// [`Models`] with a compatible registration (same exact [`ApiId`], same
/// options type), and a compatible registration upsert never invalidates
/// them.
pub struct ChatApiHandle<O: Send + 'static> {
    api_id: ApiId,
    _options: PhantomData<fn() -> O>,
}

impl<O: Send + 'static> ChatApiHandle<O> {
    /// The exact API identity of the registration that produced this handle.
    pub fn api_id(&self) -> &ApiId {
        &self.api_id
    }

    /// Wrap one options value, tagging it with the registration's API/type
    /// identity so portability and mismatch checks stay truthful.
    pub fn options(&self, value: O) -> NativeOptions<O> {
        NativeOptions {
            api_id: self.api_id.clone(),
            type_id: TypeId::of::<O>(),
            value,
        }
    }
}

/// Typed native options created by a [`ChatApiHandle`].
///
/// Carries the API/type tag of its creating registration, not an instance
/// capability: the same value may cross [`Models`] instances whose
/// registrations share the exact [`ApiId`] and options type.
pub struct NativeOptions<O: Send + 'static> {
    api_id: ApiId,
    type_id: TypeId,
    value: O,
}

impl<O: Send + 'static> NativeOptions<O> {
    /// The exact API identity tagged at creation.
    pub fn api_id(&self) -> &ApiId {
        &self.api_id
    }

    fn into_call_options(self) -> CallOptions {
        CallOptions::Native {
            type_id: self.type_id,
            value: Box::new(self.value),
        }
    }
}

impl<O: Send + 'static + Clone> Clone for NativeOptions<O> {
    fn clone(&self) -> Self {
        Self {
            api_id: self.api_id.clone(),
            type_id: self.type_id,
            value: self.value.clone(),
        }
    }
}

impl<O: Send + 'static> std::fmt::Debug for NativeOptions<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeOptions")
            .field("api_id", &self.api_id)
            .finish()
    }
}

/// Type-erased storage for one registered adapter.
struct ChatApiRegistration {
    provider_id: String,
    options_type: TypeId,
    dispatch: Arc<dyn ChatApiDispatch>,
}

/// The registration snapshot captured by a call at creation.
struct ChatApiEntry {
    provider_id: String,
    options_type: TypeId,
    dispatch: Arc<dyn ChatApiDispatch>,
}

/// Object-safe dispatch over one fixed options type.
trait ChatApiDispatch: Send + Sync {
    fn simple_options_erased(&self) -> Box<dyn Any + Send>;
    fn stream_prepared_erased(
        &self,
        request: Request,
        options: Box<dyn Any + Send>,
        auth: ResolvedAuth,
    ) -> EventStream;
}

struct TypedChatApi<O: Send + 'static> {
    api: Arc<dyn ChatApi<O>>,
}

impl<O: Send + 'static> ChatApiDispatch for TypedChatApi<O> {
    fn simple_options_erased(&self) -> Box<dyn Any + Send> {
        Box::new(self.api.simple_options())
    }

    fn stream_prepared_erased(
        &self,
        request: Request,
        options: Box<dyn Any + Send>,
        auth: ResolvedAuth,
    ) -> EventStream {
        match options.downcast::<O>() {
            Ok(options) => self.api.stream_prepared(request, *options, auth),
            Err(_) => Box::pin(stream::once(async {
                Err(ProviderError::Config(
                    ProviderErrorSummary::attested_static(
                        "native options type does not match the registered chat api",
                    ),
                ))
            })),
        }
    }
}

/// Catalog-invisible provider route serving one registered adapter through
/// provider-route entries (the simple entry dispatches here and the adapter's
/// `simple_options` mapping supplies its typed options).
struct ChatApiRouteProvider {
    provider_id: String,
    dispatch: Arc<dyn ChatApiDispatch>,
}

impl Provider for ChatApiRouteProvider {
    fn id(&self) -> &str {
        &self.provider_id
    }

    fn models(&self) -> &[ModelInfo] {
        // Catalog-invisible by contract: no records are listed or resolvable.
        &[]
    }

    fn stream_prepared(&self, request: Request, auth: ResolvedAuth) -> EventStream {
        let options = self.dispatch.simple_options_erased();
        self.dispatch.stream_prepared_erased(request, options, auth)
    }
}

// ---------------------------------------------------------------------------
// ChatCall — lazy owner of one ordinary Models entry
// ---------------------------------------------------------------------------

/// The frozen binding captured at call creation (AIR-045).
struct FrozenChatBinding {
    record: ChatModelRecord,
    route: FrozenRoute,
    auth_plan: AuthPlan,
}

enum FrozenRoute {
    ChatApi {
        api_id: ApiId,
        entry: Option<ChatApiEntry>,
    },
    Provider {
        provider_id: String,
        provider: Option<Arc<dyn Provider>>,
    },
}

enum AuthPlan {
    Route {
        provider_id: String,
        resolver: Option<Arc<dyn AuthResolver>>,
        source: AuthProvenanceSource,
    },
    Injected(Box<ResolvedAuth>),
}

impl AuthPlan {
    fn default_for(models: &Models, provider_id: &str) -> Self {
        let (resolver, source) = models.route_auth_parts(provider_id);
        Self::Route {
            provider_id: provider_id.to_owned(),
            resolver,
            source,
        }
    }
}

enum CallOptions {
    Simple,
    Native {
        type_id: TypeId,
        value: Box<dyn Any + Send>,
    },
}

/// One ordinary [`Models`] chat call: native, simple, or direct.
///
/// Construction captures the owned input, the caller's complete record, the
/// registered adapter/provider, and the auth plan — all `Send + 'static` —
/// without any auth or external IO. Authentication starts on the first
/// `poll_next`/`next().await`/[`ChatCall::result`]. Items mirror
/// [`ChatStream`]: events with their immutable event-time partials and exactly
/// one `Err(CallFailure)` for any failure, including typed pre-auth failures
/// (unregistered ApiId, options type/provider mismatch, non-dispatchable
/// route, cancellation) which never reach authentication. The first terminal
/// is absorbed and EOF is permanent, exactly like the shared collector.
pub struct ChatCall {
    state: ChatCallState,
    terminal: Option<Result<AssistantMessage, CallFailure>>,
}

/// The owned setup payload captured at construction.
struct ChatCallSetup {
    binding: FrozenChatBinding,
    request: Request,
    options: CallOptions,
}

enum ChatCallState {
    NotStarted(Box<ChatCallSetup>),
    Starting(BoxFuture<'static, Result<ChatStream, CallFailure>>),
    Active(Box<ChatStream>),
    Done,
}

impl ChatCall {
    fn new(binding: FrozenChatBinding, request: Request, options: CallOptions) -> Self {
        Self {
            state: ChatCallState::NotStarted(Box::new(ChatCallSetup {
                binding,
                request,
                options,
            })),
            terminal: None,
        }
    }

    /// Consume the handle and actively drive the call to the single final
    /// outcome (the first poll, and therefore auth, starts here if it had not
    /// happened yet).
    pub async fn result(mut self) -> Result<AssistantMessage, CallFailure> {
        use futures_util::StreamExt as _;
        while self.terminal.is_none() {
            if self.next().await.is_none() {
                break;
            }
        }
        match self.terminal.take() {
            Some(outcome) => outcome,
            None => Err(CallFailure::from_partial(
                None,
                ProviderError::StreamError(ProviderErrorSummary::attested_static(
                    "chat call ended without a terminal event",
                )),
            )),
        }
    }
}

impl Stream for ChatCall {
    type Item = Result<AssistantStreamEvent, CallFailure>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            if let ChatCallState::Active(stream) = &mut this.state {
                return match Stream::poll_next(Pin::new(stream), cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(Some(Ok(event))) => {
                        if let AssistantStreamEvent::Done { message, .. } = &event {
                            this.terminal = Some(Ok(message.clone()));
                        }
                        Poll::Ready(Some(Ok(event)))
                    }
                    Poll::Ready(Some(Err(failure))) => {
                        this.terminal = Some(Err(failure.clone()));
                        Poll::Ready(Some(Err(failure)))
                    }
                    Poll::Ready(None) => {
                        this.state = ChatCallState::Done;
                        Poll::Ready(None)
                    }
                };
            }
            if matches!(this.state, ChatCallState::Done) {
                return Poll::Ready(None);
            }
            match std::mem::replace(&mut this.state, ChatCallState::Done) {
                ChatCallState::NotStarted(setup) => {
                    // First poll: setup (and therefore auth) starts here.
                    let ChatCallSetup {
                        binding,
                        request,
                        options,
                    } = *setup;
                    this.state = ChatCallState::Starting(Box::pin(materialize_chat_call(
                        binding, request, options,
                    )));
                }
                ChatCallState::Starting(mut future) => {
                    match std::future::Future::poll(future.as_mut(), cx) {
                        Poll::Pending => {
                            this.state = ChatCallState::Starting(future);
                            return Poll::Pending;
                        }
                        Poll::Ready(Ok(stream)) => {
                            this.state = ChatCallState::Active(Box::new(stream));
                        }
                        Poll::Ready(Err(failure)) => {
                            this.terminal = Some(Err(failure.clone()));
                            this.state = ChatCallState::Done;
                            return Poll::Ready(Some(Err(failure)));
                        }
                    }
                }
                ChatCallState::Active(_) | ChatCallState::Done => {
                    unreachable!("active and done states are handled above")
                }
            }
        }
    }
}

/// Setup for one lazy call: canonical identity from the frozen record,
/// pre-auth route/type checks, the shared compiler, the shared auth
/// resolution (or the injected auth), and one dispatch through the captured
/// adapter/provider into the shared [`ChatStream`] collector.
async fn materialize_chat_call(
    binding: FrozenChatBinding,
    mut request: Request,
    options: CallOptions,
) -> Result<ChatStream, CallFailure> {
    // Canonical model identity comes from the frozen complete record.
    request.model = binding.record.spec();

    let event_stream = match binding.route {
        FrozenRoute::ChatApi { api_id, entry } => {
            let Some(entry) = entry else {
                return Err(setup_failure(format!(
                    "chat api '{}' is not registered",
                    api_id.as_str()
                )));
            };
            if entry.provider_id != binding.record.provider_id {
                return Err(setup_failure(format!(
                    "record provider '{}' does not match the registered chat api provider '{}'",
                    binding.record.provider_id(),
                    entry.provider_id
                )));
            }
            let CallOptions::Native { type_id, value } = options else {
                unreachable!("simple entries dispatch through provider routes")
            };
            if type_id != entry.options_type {
                return Err(setup_failure(format!(
                    "native options type does not match the registered chat api '{}'",
                    api_id.as_str()
                )));
            }
            compile_prepared_request(
                binding.record.provider_id(),
                binding.record.info(),
                &mut request,
            )
            .map_err(|error| CallFailure::from_partial(None, error))?;
            let auth = resolve_binding_auth(&binding.auth_plan, &request.cancel).await?;
            entry.dispatch.stream_prepared_erased(request, value, auth)
        }
        FrozenRoute::Provider {
            provider_id,
            provider,
        } => {
            let Some(provider) = provider else {
                return Err(setup_failure(format!("unknown provider: {provider_id}")));
            };
            compile_prepared_request(
                binding.record.provider_id(),
                binding.record.info(),
                &mut request,
            )
            .map_err(|error| CallFailure::from_partial(None, error))?;
            let auth = resolve_binding_auth(&binding.auth_plan, &request.cancel).await?;
            provider.stream_prepared(request, auth)
        }
    };
    Ok(ChatStream::new(event_stream))
}

async fn resolve_binding_auth(
    plan: &AuthPlan,
    cancel: &CancellationToken,
) -> Result<ResolvedAuth, CallFailure> {
    match plan {
        AuthPlan::Injected(auth) => Ok((**auth).clone()),
        AuthPlan::Route {
            provider_id,
            resolver: None,
            ..
        } => Err(setup_failure(format!(
            "provider '{provider_id}' has no dispatchable route"
        ))),
        AuthPlan::Route {
            resolver: Some(resolver),
            source,
            ..
        } => resolve_route_auth(resolver, source.clone(), cancel)
            .await
            .map_err(models_error_cause)
            .map_err(|cause| CallFailure::from_partial(None, cause)),
    }
}
