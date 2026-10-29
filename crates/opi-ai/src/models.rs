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
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::future::BoxFuture;
use futures_util::{Stream, StreamExt, stream};
use tokio_util::sync::CancellationToken;

use crate::auth::{
    AuthFallback, AuthProvenance, AuthProvenanceSource, AuthResolver, AuthScheme,
    AwsSigV4Credentials, ResolvedAuth,
};
use crate::credential::{Credential, CredentialSource, CredentialStore, CredentialStoreError};
use crate::message::AssistantMessage;
use crate::mutation::{MutationError, MutationOutcome};
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
    /// [`Models::set_probe`] and consulted by
    /// [`Models::check_auth`] when no live store is attached.
    StoreCredential {
        /// Store account key (typically the provider id).
        key: String,
        /// Non-secret display label (e.g. `keychain opi:anthropic`).
        display_source: String,
    },
}

/// Non-secret result of an on-demand source-configuration check.
///
/// Produced by [`Models::check_auth`] and [`Models::get_available`]. `source`
/// names where the configured credential comes from (for example an
/// environment-variable name or `"OAuth"`) without ever carrying a credential
/// value. `None` from those operations means the provider is unknown or has no
/// complete configuration; source *failures* are typed
/// [`ModelsError`] values, never an empty or silently-missing result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthCheck {
    /// Non-secret source label for status UI (e.g. `ANTHROPIC_API_KEY`,
    /// `"OAuth"`, `"credential store"`).
    pub source: Option<String>,
    /// Which credential family the configured source supplies.
    pub kind: AuthCheckKind,
}

/// Credential family reported by [`AuthCheck`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthCheckKind {
    /// A static or environment-sourced API key.
    ApiKey,
    /// A stored OAuth credential (reported configured even when its access
    /// token is expired: checks never refresh).
    OAuth,
}

/// Explicit per-call resolution overrides for
/// [`Models::get_auth`](crate::Models::get_auth).
///
/// An explicit override wins over every configured source: no environment,
/// store, or resolver access happens for that call. At most one override may
/// be set — the schemes are mutually exclusive attachment choices, so
/// multiple set overrides fail as [`ModelsError::ConflictingAuthOverrides`]
/// instead of silently preferring one.
#[derive(Debug, Clone, Default)]
pub struct AuthResolutionOverrides {
    /// Caller-supplied API key that replaces all configured sources.
    pub api_key: Option<secrecy::SecretString>,
    /// Caller-supplied Bearer token that replaces all configured sources
    /// (for example a Bedrock API key).
    pub bearer: Option<secrecy::SecretString>,
    /// Caller-supplied typed AWS SigV4 credential bundle that replaces all
    /// configured sources.
    pub aws_sigv4: Option<AwsSigV4Credentials>,
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
#[non_exhaustive]
pub enum ModelsError {
    /// Registration of a dispatchable provider failed.
    #[error(transparent)]
    Registration(#[from] RegistrationError),
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
    /// [`start_attempt`](PreparedProviderCall::start_attempt) was called
    /// before the admitted retry's readiness deadline. The caller must keep
    /// waiting (cancellably) for the [`RetryAdmission`](crate::retry::RetryAdmission) `delay_ms` it read; no
    /// dispatch happened.
    #[error("the admitted retry is not ready for another {retry_ready_in_ms}ms")]
    RetryNotReady {
        /// Milliseconds remaining before the next attempt start is ready.
        retry_ready_in_ms: u64,
    },
    /// The prepared call is sealed (post-handoff, unknown-dispatch, exhausted
    /// budget, or a terminal outcome) and no further attempt may be
    /// dispatched on it. A new logical call must re-prepare.
    #[error("the prepared call is sealed ({})", seal.as_str())]
    CallSealed {
        /// Which sealed state the call reached.
        seal: crate::retry::AcquisitionSeal,
    },
    /// A provider stream failed while draining to completion.
    #[error(transparent)]
    Provider(#[from] ProviderError),
    /// Ambient source access failed while checking or resolving
    /// authentication. The failure is typed: an unreadable source is never
    /// reported as "unconfigured".
    #[error(transparent)]
    AuthContext(#[from] crate::auth::AuthContextError),
    /// A credential-store operation failed while checking, resolving, or
    /// deleting authentication. Store failures stay distinct from missing
    /// entries and never trigger a silent fallback.
    #[error(transparent)]
    CredentialStore(#[from] CredentialStoreError),
    /// The selected auth source failed while resolving a credential. The
    /// failure propagates: no other source is tried for that call.
    #[error("auth resolution failed for '{provider}': {error}")]
    AuthResolution {
        /// Provider whose selected source failed.
        provider: String,
        /// Typed failure reported by the selected source.
        error: Box<ProviderError>,
    },
    /// More than one mutually exclusive explicit auth override was supplied
    /// for one resolution. Exactly one explicit choice may be selected; the
    /// conflicting request is rejected before any source access.
    #[error(
        "conflicting explicit auth overrides for '{provider}': at most one of api_key, bearer, aws_sigv4 may be set"
    )]
    ConflictingAuthOverrides {
        /// Provider whose per-call overrides conflicted.
        provider: String,
    },
    /// A serialized store mutation settled with an unknown outcome (the
    /// worker was lost mid-mutation). The effect may or may not have landed;
    /// callers must not claim the previous state still holds.
    #[error("mutation outcome unknown for provider '{provider}': {detail}")]
    MutationOutcomeUnknown {
        /// Provider key the mutation targeted.
        provider: String,
        /// Redacted, non-secret loss detail.
        detail: String,
    },
    /// [`login`](Models::login) was asked to run a provider that has no
    /// registered OAuth flow.
    #[error("no OAuth flow registered for provider '{provider}'")]
    UnknownOAuthFlow {
        /// Provider whose flow was requested.
        provider: String,
    },
    /// A dynamic-catalog store operation failed while restoring or
    /// committing a revision.
    #[error(transparent)]
    CatalogStore(#[from] crate::catalog::ModelsStoreError),
}

/// Outcome of one refresh round, reported per provider or source id.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum RefreshOutcome {
    /// The entry's complete catalog was published in the new revision
    /// (`models: 0` means the provider fell back to its built-in models).
    Published {
        /// Number of models in the published snapshot.
        models: usize,
    },
    /// The entry failed; its prior data (if any) is retained. The error is
    /// redacted by construction.
    Failed(ProviderError),
    /// The entry was restored from the persisted store during the
    /// restore-before-network step.
    Restored {
        /// Number of models in the restored snapshot.
        models: usize,
    },
}

impl RefreshOutcome {
    /// Whether this outcome is a reported failure.
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failed(_))
    }
}

/// Terminal status of a refresh round.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum RefreshStatus {
    /// A new revision was fetched and published.
    #[default]
    Published,
    /// Offline round: the store cache was restored, no network work ran.
    OfflineRestored,
    /// A newer revision landed first; this round was dropped without
    /// publishing.
    Superseded,
    /// The refresh was cancelled; nothing was persisted or published.
    Cancelled,
}

/// One row of a [`RefreshReport`].
#[derive(Clone, Debug, PartialEq)]
pub struct RefreshReportEntry {
    /// Provider id or catalog-source id.
    pub id: String,
    /// What happened to that entry.
    pub outcome: RefreshOutcome,
}

/// Per-entry report of one refresh round.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RefreshReport {
    status: RefreshStatus,
    entries: Vec<RefreshReportEntry>,
}

impl RefreshReport {
    /// Terminal status of the round.
    pub fn status(&self) -> RefreshStatus {
        self.status
    }

    /// Every provider/source outcome, in evaluation order.
    pub fn entries(&self) -> &[RefreshReportEntry] {
        &self.entries
    }

    /// Whether the round reported no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Find the report row for `id` (provider id or catalog-source id).
    pub fn find(&self, id: &str) -> Option<&RefreshReportEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }
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
    resource_owners: HashMap<String, Arc<dyn crate::codex_resources::SessionResourceOwner>>,
    all_resource_owners:
        Arc<std::sync::Mutex<Vec<Arc<dyn crate::codex_resources::SessionResourceOwner>>>>,
    registration_gate: Arc<std::sync::Mutex<()>>,
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
    /// Registered SDK OAuth flows keyed by provider id; the login command
    /// dispatches here and persists through the attached credential store.
    oauth_flows: HashMap<String, std::sync::Arc<dyn crate::auth::OAuthProvider>>,
    /// Routes whose resolver is generated from the attached OAuth store.
    oauth_routes: std::collections::HashSet<String>,
    /// Typed custom chat API registrations keyed by exact [`ApiId`]. Entries
    /// are captured (cloned `Arc`s) at call creation, so later registration
    /// changes never mutate an existing call.
    chat_apis: BTreeMap<ApiId, ChatApiRegistration>,
    /// Injectable ambient-source access consulted on demand by the auth
    /// management operations. Defaults to the lazy
    /// [`AmbientAuthContext`](crate::auth::AmbientAuthContext); construction
    /// and registration never touch it.
    auth_context: std::sync::Arc<dyn crate::auth::AuthContext>,
    /// Optional credential store backing the auth management operations
    /// (`check_auth`/`get_auth`/`logout` on store-backed sources). Absent by
    /// construction-time design: no store, env, or network access happens
    /// until the owner attaches one.
    credential_store: Option<std::sync::Arc<dyn CredentialStore>>,
    /// Optional persisted dynamic-catalog store. Attached explicitly via
    /// [`Models::set_models_store`]; the SDK never creates one or fetches a
    /// remote catalog by default.
    models_store: Option<crate::catalog::ModelsStore>,
    /// Explicitly registered dynamic-catalog sources, fetched only during
    /// `refresh(true)` rounds in registration order.
    catalog_sources: Vec<std::sync::Arc<dyn crate::catalog::DynamicCatalogSource>>,
    /// Publisher ownership from the same acknowledged revision as each
    /// dynamic catalog; unrelated sources cannot retain a native snapshot.
    catalog_publishers: HashMap<String, crate::catalog::store::CatalogPublisher>,
    /// Whether the attached store's snapshot has already been restored into
    /// the registry during a refresh round.
    store_restored: bool,
}

impl Models {
    /// Construct an empty driver. Registers no provider and performs no
    /// env/auth/store/external IO.
    pub fn new() -> Self {
        Self {
            registry: ProviderRegistry::new(),
            resource_owners: HashMap::new(),
            all_resource_owners: Arc::default(),
            registration_gate: Arc::default(),
            auth: HashMap::new(),
            compat: HashMap::new(),
            probed: HashMap::new(),
            resolvers: HashMap::new(),
            sources: HashMap::new(),
            oauth_flows: HashMap::new(),
            oauth_routes: std::collections::HashSet::new(),
            chat_apis: BTreeMap::new(),
            auth_context: std::sync::Arc::new(crate::auth::AmbientAuthContext),
            credential_store: None,
            models_store: None,
            catalog_sources: Vec::new(),
            catalog_publishers: HashMap::new(),
            store_restored: false,
        }
    }

    /// Wrap an existing registry to layer the driver semantics onto providers
    /// constructed by an outer factory.
    ///
    /// Pre-registered providers are lookup-only until replaced by
    /// [`register_route`](Self::register_route) or
    /// [`register_chat_api`](Self::register_chat_api):
    /// [`Models::check_auth`] returns `Ok(None)` and
    /// [`prepare_call`](Self::prepare_call) returns
    /// [`ModelsError::RouteNotDispatchable`].
    pub fn from_registry(registry: ProviderRegistry) -> Self {
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
            auth: HashMap::new(),
            compat: HashMap::new(),
            probed: HashMap::new(),
            resolvers: HashMap::new(),
            sources: HashMap::new(),
            oauth_flows: HashMap::new(),
            oauth_routes: std::collections::HashSet::new(),
            chat_apis: BTreeMap::new(),
            auth_context: std::sync::Arc::new(crate::auth::AmbientAuthContext),
            credential_store: None,
            models_store: None,
            catalog_sources: Vec::new(),
            catalog_publishers: HashMap::new(),
            store_restored: false,
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
        let resource_owner = provider
            .session_resources()
            .map(|owner| owner.new_namespace());
        self.registry.register_provider(provider)?;
        self.replace_resource_owner(&id, resource_owner);
        self.resolvers.remove(&id);
        self.oauth_routes.remove(&id);
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
        let resource_owner = provider
            .session_resources()
            .map(|owner| owner.new_namespace());
        self.registry.register_provider(provider)?;
        self.replace_resource_owner(&id, resource_owner);
        self.oauth_routes.remove(&id);
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
        let resource_owner = api.session_resources().map(|owner| owner.new_namespace());
        let dispatch: Arc<dyn ChatApiDispatch> = Arc::new(TypedChatApi { api });
        // Catalog-invisible route: a Provider with no models keeps the adapter
        // dispatchable through provider-route entries without listing it.
        self.registry
            .register_provider(Box::new(ChatApiRouteProvider {
                provider_id: provider_id.clone(),
                dispatch: Arc::clone(&dispatch),
            }))?;
        self.replace_resource_owner(&provider_id, resource_owner.clone());
        self.auth.remove(&provider_id);
        self.oauth_routes.remove(&provider_id);
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
                resource_owner,
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

    fn replace_resource_owner(
        &mut self,
        provider: &str,
        owner: Option<Arc<dyn crate::codex_resources::SessionResourceOwner>>,
    ) {
        self.resource_owners.remove(provider);
        if let Some(owner) = owner {
            // Earlier revisions stay reachable by cleanup and owned calls.
            self.all_resource_owners.lock().unwrap().push(owner.clone());
            self.resource_owners.insert(provider.to_owned(), owner);
        }
    }

    fn chat_api_binding<O: Send + 'static>(
        &self,
        record: &ChatModelRecord,
        options: &NativeOptions<O>,
        auth_plan: AuthPlan,
    ) -> FrozenChatBinding {
        let registration = self.chat_apis.get(options.api_id());
        let resource_owner = registration.and_then(|entry| entry.resource_owner.clone());
        let entry = registration.map(|registration| ChatApiEntry {
            provider_id: registration.provider_id.clone(),
            options_type: registration.options_type,
            dispatch: Arc::clone(&registration.dispatch),
        });
        FrozenChatBinding {
            record: record.clone(),
            registration_gate: self.registration_gate.clone(),
            resource_owner,
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
            registration_gate: self.registration_gate.clone(),
            resource_owner: self.resource_owners.get(record.provider_id()).cloned(),
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

    /// Native entry driven to the single final outcome. Construction freezes
    /// the owned call; authentication starts when the returned future is polled.
    pub fn complete<O: Send + 'static>(
        &self,
        record: ChatModelRecord,
        request: Request,
        options: NativeOptions<O>,
    ) -> impl Future<Output = Result<AssistantMessage, CallFailure>> + Send + 'static + use<O> {
        self.stream(record, request, options).result()
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
        // Neutral entry: clamp the requested thinking level to the nearest
        // supported level (Pi streamSimple) before the shared compiler's
        // per-wire thinking preflight; native entries retain exact levels and
        // fail typed preflight instead.
        if request.thinking.enabled
            && let Some(level) = record
                .info()
                .thinking_level_map
                .nearest_supported(request.thinking.level)
        {
            request.thinking.level = level;
        }
        let binding = self.provider_binding(&record);
        ChatCall::new(binding, request, CallOptions::Simple(options))
    }

    /// Neutral entry driven to the single final outcome. Construction freezes
    /// the owned call; authentication starts when the returned future is polled.
    pub fn complete_simple(
        &self,
        record: ChatModelRecord,
        request: Request,
        options: SimpleOptions,
    ) -> impl Future<Output = Result<AssistantMessage, CallFailure>> + Send + 'static + use<> {
        self.stream_simple(record, request, options).result()
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

    /// Direct entry driven to the single final outcome. Construction freezes
    /// the owned call and injected authentication before returning the future.
    pub fn complete_direct<O: Send + 'static>(
        &self,
        record: ChatModelRecord,
        request: Request,
        options: NativeOptions<O>,
        auth: ResolvedAuth,
    ) -> impl Future<Output = Result<AssistantMessage, CallFailure>> + Send + 'static + use<O> {
        self.stream_direct(record, request, options, auth).result()
    }

    /// Resolve one canonical `provider:model` route, validate the request,
    /// resolve authentication once, and freeze an opaque
    /// [`PreparedProviderCall`] (advanced entry) with the default acquisition
    /// policy (zero extra sends).
    ///
    /// Equivalent to
    /// [`prepare_call_with_policy`](Self::prepare_call_with_policy) with
    /// [`AcquisitionPolicy::default`](crate::retry::AcquisitionPolicy::default).
    pub async fn prepare_call(
        &self,
        spec: &str,
        request: Request,
    ) -> Result<PreparedProviderCall, ModelsError> {
        // Ordinary entries freeze the provider's protocol-default
        // acquisition policy (helper/Codex wires default to zero extra
        // sends; Bedrock freezes its resolved Smithy configuration).
        let (provider_ref, _model) = self.registry.resolve(spec)?;
        let policy = provider_ref.acquisition_policy_default();
        self.prepare_call_with_policy(spec, request, policy).await
    }

    /// Resolve one canonical `provider:model` route, validate the request,
    /// resolve authentication once, and freeze an opaque
    /// [`PreparedProviderCall`] (advanced entry) with an explicit per-call
    /// acquisition policy.
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
    pub async fn prepare_call_with_policy(
        &self,
        spec: &str,
        mut request: Request,
        policy: crate::retry::AcquisitionPolicy,
    ) -> Result<PreparedProviderCall, ModelsError> {
        let (provider_ref, model) = self.registry.resolve(spec)?;
        request.callbacks = request.callbacks.for_call();
        let provider_id = provider_ref.id().to_owned();
        let model_id = model.id.clone();
        let wire_api = model.wire_api;
        let resource_owner = self.resource_owners.get(&provider_id).cloned();
        let resources = {
            let _registration = self.registration_gate.lock().unwrap();
            resource_owner
                .as_ref()
                .and_then(|owner| owner.register_call(&request))
        };
        request.cancel = resources
            .as_ref()
            .map_or_else(|| request.cancel.child_token(), |lease| lease.token());
        if request.cancel.is_cancelled() {
            return Err(ModelsError::CallCancelled);
        }
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
            request: Box::new(request),
            auth,
            route,
            credential_terminal: Arc::new(AtomicBool::new(false)),
            acquisition: Arc::new(crate::retry::AcquisitionCore::new(policy)),
            invocation_id: uuid::Uuid::new_v4().to_string(),
            resources,
            resource_owner,
            selected_model: Some(Box::new(model.clone())),
            simple_options: SimpleOptions::default(),
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
        self.auth_context = context;
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
        let resource_owners = self.all_resource_owners.clone();
        let registration_gate = self.registration_gate.clone();
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

    /// Attach the credential store backing store-sourced auth checks,
    /// resolution, and [`logout`](Self::logout).
    ///
    /// Absent by default: constructing, registering, and statically looking up
    /// providers never touches a store. Store-backed checks without a live
    /// store consult the injected probe snapshot instead
    /// ([`set_probe`](Self::set_probe)).
    pub fn set_credential_store(&mut self, store: std::sync::Arc<dyn CredentialStore>) {
        for id in &self.oauth_routes {
            if let Some(flow) = self.oauth_flows.get(id) {
                self.resolvers.insert(
                    id.clone(),
                    Arc::new(crate::auth::store_oauth::StoreOAuthResolver::new(
                        store.clone(),
                        id.clone(),
                        flow.clone(),
                    )),
                );
            }
        }
        self.credential_store = Some(store);
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
        self.sources.insert(
            provider_id.to_owned(),
            AuthProvenanceSource::OAuth {
                kind: provider_id.to_owned(),
            },
        );
        self.oauth_flows
            .insert(provider_id.to_owned(), flow.clone());
        if self.oauth_routes.contains(provider_id)
            && let Some(store) = &self.credential_store
        {
            self.resolvers.insert(
                provider_id.to_owned(),
                Arc::new(crate::auth::store_oauth::StoreOAuthResolver::new(
                    store.clone(),
                    provider_id,
                    flow,
                )),
            );
        }
    }

    /// Bind an SDK OAuth flow to a dispatchable provider and the attached store.
    /// Login, refresh, ACK and request authentication share that store. This
    /// performs no source access; attach the store before registering the route.
    pub fn register_oauth_route(
        &mut self,
        provider: Box<dyn Provider>,
        flow: Arc<dyn crate::auth::OAuthProvider>,
        compat: CompatMetadata,
    ) -> Result<(), ModelsError> {
        let id = provider.id().to_owned();
        let store = self.credential_store.clone().ok_or_else(|| {
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
        self.oauth_routes.insert(id);
        Ok(())
    }

    /// Run the provider's registered OAuth login and persist the resulting
    /// credential through the store-owned serialized mutation queue,
    /// acknowledging exactly that local write: success is reported only
    /// after the awaited store ACK, a failed write surfaces typed, and an
    /// unknown outcome stays honest as [`ModelsError::MutationOutcomeUnknown`].
    ///
    /// No automatic browser launch, keychain read, environment fallback, or
    /// remote revocation happens: interaction goes through `presenter`, HTTP
    /// through the flow's injected client, persistence through the attached
    /// [`CredentialStore`]. Calling `login` for a provider without a
    /// registered flow or without an attached store is a typed error before
    /// any interaction.
    pub async fn login(
        &self,
        provider_id: &str,
        presenter: &dyn crate::auth::LoginPresenter,
    ) -> Result<crate::auth::OAuthCredential, ModelsError> {
        let flow =
            self.oauth_flows
                .get(provider_id)
                .ok_or_else(|| ModelsError::UnknownOAuthFlow {
                    provider: provider_id.to_owned(),
                })?;
        let store = self.credential_store.clone().ok_or_else(|| {
            ModelsError::CredentialStore(CredentialStoreError::Backend {
                provider: provider_id.to_owned(),
                reason: "no credential store attached".to_owned(),
            })
        })?;
        match crate::auth::oauth::login_to_store(
            provider_id,
            flow.as_ref(),
            store.as_ref(),
            presenter,
        )
        .await
        {
            Ok(credential) => Ok(credential),
            Err(crate::auth::oauth::LoginStoreError::Flow(error)) => {
                Err(ModelsError::AuthResolution {
                    provider: provider_id.to_owned(),
                    error: Box::new(error),
                })
            }
            Err(crate::auth::oauth::LoginStoreError::Store(error)) => {
                Err(ModelsError::CredentialStore(error))
            }
            Err(crate::auth::oauth::LoginStoreError::Unknown { detail }) => {
                Err(ModelsError::MutationOutcomeUnknown {
                    provider: provider_id.to_owned(),
                    detail,
                })
            }
        }
    }

    /// The ambient-source access this collection consults on demand.
    pub fn auth_context(&self) -> &std::sync::Arc<dyn crate::auth::AuthContext> {
        &self.auth_context
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
    /// [`AuthDescriptor::StoreCredential`]); store-backed providers without a
    /// live store fall back to the redacted probe snapshot injected via
    /// [`set_probe`](Self::set_probe). Route providers first use a declared
    /// configuration checker when present, then their non-secret source
    /// classification. Checking never resolves or refreshes authentication.
    pub async fn check_auth(&self, provider_id: &str) -> Result<Option<AuthCheck>, ModelsError> {
        if let Some(descriptor) = self.auth.get(provider_id) {
            return self.check_descriptor(provider_id, descriptor).await;
        }
        if !self.check_route_configuration(provider_id).await? {
            return Ok(None);
        }
        match self.sources.get(provider_id) {
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
            .provider_ids()
            .into_iter()
            .filter(|id| provider_id.is_none_or(|wanted| wanted == *id))
            .map(str::to_owned)
            .collect();
        let mut available = Vec::new();
        let mut account_models = std::collections::HashMap::new();
        for id in ids {
            if !self.auth.contains_key(&id)
                && matches!(
                    self.sources.get(&id),
                    Some(AuthProvenanceSource::OAuth { .. })
                )
                && let Some(store) = &self.credential_store
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
        for (provider, model) in self.registry.all_models() {
            if available.iter().any(|id| id == provider)
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
        if let Some(resolver) = self.resolvers.get(provider_id) {
            return match resolver.resolve().await {
                Ok(resolved) => Ok(Some(resolved)),
                Err(ProviderError::CredentialNeeded { .. }) => Ok(None),
                Err(error) => Err(ModelsError::AuthResolution {
                    provider: provider_id.to_owned(),
                    error: Box::new(error),
                }),
            };
        }
        let Some(descriptor) = self.auth.get(provider_id) else {
            return Ok(None);
        };
        match descriptor {
            AuthDescriptor::StaticApiKey { value } => {
                Ok(value.is_present().then(|| ResolvedAuth {
                    scheme: AuthScheme::ApiKey,
                    secret: secrecy::SecretString::from(value.as_str().to_owned()),
                    base_url: None,
                    account_id: None,
                    provenance: AuthProvenance::default(),
                }))
            }
            AuthDescriptor::EnvApiKey { env_var } => match self.auth_context.env(env_var)? {
                Some(value) => Ok(Some(ResolvedAuth {
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
            },
            // Label-only descriptor: the secret is owned by a route/flow that
            // is not part of this collection.
            AuthDescriptor::Resolved { .. } => Ok(None),
            AuthDescriptor::StoreCredential { key, .. } => {
                self.resolve_from_store(provider_id, key).await
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

    /// Synchronously capture the attached store's mutation cutoff. Every
    /// `Models` instance sharing the same store observes the same
    /// generations. `None` when no credential store is attached.
    pub fn mutation_barrier(&self) -> Option<crate::mutation::MutationBarrier> {
        self.credential_store
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
            AuthDescriptor::EnvApiKey { env_var } => {
                Ok(self.auth_context.env(env_var)?.map(|_| AuthCheck {
                    source: Some(env_var.clone()),
                    kind: AuthCheckKind::ApiKey,
                }))
            }
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
                Ok(self.auth_context.env(name)?.map(|_| AuthCheck {
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
    /// refreshed); without a live store the injected probe snapshot decides,
    /// with backend-unavailable snapshots surfacing as typed errors.
    async fn check_store_credential(
        &self,
        provider_id: &str,
        key: &str,
        fallback_kind: AuthCheckKind,
    ) -> Result<Option<AuthCheck>, ModelsError> {
        if let Some(store) = self.credential_store.as_ref() {
            return match store.read(key).await? {
                Some(Credential::ApiKey(_)) => Ok(Some(AuthCheck {
                    source: Some("credential store".to_owned()),
                    kind: AuthCheckKind::ApiKey,
                })),
                Some(Credential::OAuthToken { .. }) => Ok(Some(AuthCheck {
                    source: Some("OAuth".to_owned()),
                    kind: AuthCheckKind::OAuth,
                })),
                None => Ok(None),
            };
        }
        match self.probed.get(provider_id) {
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
        let Some(store) = self.credential_store.as_ref() else {
            return Ok(None);
        };
        match store.read(key).await? {
            Some(Credential::ApiKey(value)) => Ok(Some(ResolvedAuth {
                scheme: AuthScheme::ApiKey,
                secret: value,
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
    /// Refresh provider-side model catalogs.
    ///
    /// `allow_network` selects the two refresh modes:
    ///
    /// - `false` performs **no OAuth/network work at all**: an attached
    ///   [`crate::catalog::ModelsStore`] is restored into the registry and
    ///   the report carries [`RefreshOutcome::Restored`] entries with status
    ///   [`RefreshStatus::OfflineRestored`].
    /// - `true` first restores any pending store snapshot, then fetches every
    ///   registered provider (deterministic sorted id order) and every
    ///   registered catalog source (registration order). Outcomes are
    ///   **per-provider**: successes publish, failures retain that
    ///   provider's prior data and report a redacted error without erasing
    ///   other providers' catalogs. `Ok(None)` clears that provider's dynamic
    ///   snapshot back to its built-in models.
    ///
    /// When a store is attached, publication goes through the store's single
    /// revision-commit owner: the committed snapshot is persisted first and
    /// exactly the acknowledged map is applied to memory, so a superseded or
    /// cancelled refresh can never publish late. Already-prepared calls keep
    /// their frozen bindings across refreshes.
    ///
    /// The SDK registers no source and attaches no store by default: there
    /// is no pi.dev or default startup fetch policy here.
    pub async fn refresh(&mut self, allow_network: bool) -> Result<RefreshReport, ModelsError> {
        self.refresh_with(allow_network, &tokio_util::sync::CancellationToken::new())
            .await
    }

    /// [`refresh`](Self::refresh) observing `token`: cancellation observed
    /// before publication stops fetching and leaves the current publication
    /// intact, with report status [`RefreshStatus::Cancelled`]. Without a
    /// store, the final token check begins the synchronous memory commit;
    /// later cancellation cannot roll back that completed publication.
    pub async fn refresh_with(
        &mut self,
        allow_network: bool,
        token: &tokio_util::sync::CancellationToken,
    ) -> Result<RefreshReport, ModelsError> {
        let mut report = RefreshReport {
            status: RefreshStatus::Published,
            entries: Vec::new(),
        };

        // Restore the persisted cache before any network work.
        if !self.store_restored {
            if let Some(store) = &self.models_store {
                let (snapshot, publishers) = store.snapshot_with_publishers();
                for (id, models) in &snapshot.catalogs {
                    report.entries.push(RefreshReportEntry {
                        id: id.clone(),
                        outcome: RefreshOutcome::Restored {
                            models: models.len(),
                        },
                    });
                }
                self.registry
                    .replace_all_dynamic_catalogs(snapshot.catalogs);
                self.catalog_publishers = publishers;
            }
            self.store_restored = true;
        }

        if !allow_network {
            report.status = RefreshStatus::OfflineRestored;
            return Ok(report);
        }

        let guard = self
            .models_store
            .as_ref()
            .map(|store| store.begin_with(token.clone()));

        // Merge over the current publication: failures retain prior data.
        let mut merged = self.registry.dynamic_catalogs();
        let mut publishers = self.catalog_publishers.clone();
        publishers.retain(|provider, _| merged.contains_key(provider));
        let mut cancelled = false;

        let mut ids: Vec<String> = self
            .registry
            .provider_ids()
            .into_iter()
            .map(str::to_owned)
            .collect();
        ids.sort();
        for id in ids {
            let Some(provider) = self.registry.get_provider(&id) else {
                continue;
            };
            let outcome = tokio::select! {
                biased;
                _ = token.cancelled() => None,
                result = provider.refresh_models() => Some(result),
            };
            let result = match outcome {
                Some(result) => result,
                None => {
                    cancelled = true;
                    break;
                }
            };
            match result {
                Ok(Some(models)) => match validate_dynamic_catalog(&id, &models) {
                    Ok(()) => {
                        report.entries.push(RefreshReportEntry {
                            id: id.clone(),
                            outcome: RefreshOutcome::Published {
                                models: models.len(),
                            },
                        });
                        publishers
                            .insert(id.clone(), crate::catalog::store::CatalogPublisher::Native);
                        merged.insert(id, models);
                    }
                    Err(error) => report.entries.push(RefreshReportEntry {
                        id: id.clone(),
                        outcome: RefreshOutcome::Failed(error),
                    }),
                },
                Ok(None) => {
                    // Native absence cannot erase a confirmed source-owned
                    // publication while that source remains registered. A
                    // different provider's source confers no such ownership.
                    let source_owned = matches!(publishers.get(&id),
                        Some(crate::catalog::store::CatalogPublisher::Source { id: source_id })
                        if self.catalog_sources.iter().any(|source| source.id() == source_id));
                    if !source_owned {
                        merged.remove(&id);
                        publishers.remove(&id);
                    }
                    report.entries.push(RefreshReportEntry {
                        id: id.clone(),
                        outcome: RefreshOutcome::Published {
                            models: merged.get(&id).map_or(0, Vec::len),
                        },
                    });
                }
                Err(error) => report.entries.push(RefreshReportEntry {
                    id: id.clone(),
                    outcome: RefreshOutcome::Failed(error),
                }),
            }
        }

        if !cancelled {
            for source in &self.catalog_sources {
                let outcome = tokio::select! {
                    biased;
                    _ = token.cancelled() => None,
                    result = source.fetch(token.clone()) => Some(result),
                };
                let result = match outcome {
                    Some(result) => result,
                    None => {
                        cancelled = true;
                        break;
                    }
                };
                match result {
                    Ok(catalogs) => {
                        let mut published = 0_usize;
                        let mut source_error = None;
                        for (provider_id, models) in catalogs {
                            if self.registry.get_provider(&provider_id).is_none() {
                                source_error.get_or_insert(ProviderError::Config(
                                    ProviderErrorSummary::sanitized(format!(
                                        "catalog source '{}' published unknown provider '{}'",
                                        source.id(),
                                        provider_id
                                    )),
                                ));
                                continue;
                            }
                            match validate_dynamic_catalog(&provider_id, &models) {
                                Ok(()) => {
                                    published += models.len();
                                    publishers.insert(
                                        provider_id.clone(),
                                        crate::catalog::store::CatalogPublisher::Source {
                                            id: source.id().to_owned(),
                                        },
                                    );
                                    merged.insert(provider_id, models);
                                }
                                Err(error) => {
                                    source_error.get_or_insert(error);
                                }
                            }
                        }
                        match source_error {
                            None => report.entries.push(RefreshReportEntry {
                                id: source.id().to_owned(),
                                outcome: RefreshOutcome::Published { models: published },
                            }),
                            Some(error) => report.entries.push(RefreshReportEntry {
                                id: source.id().to_owned(),
                                outcome: RefreshOutcome::Failed(error),
                            }),
                        }
                    }
                    Err(error) => report.entries.push(RefreshReportEntry {
                        id: source.id().to_owned(),
                        outcome: RefreshOutcome::Failed(error),
                    }),
                }
            }
        }

        if cancelled {
            report.status = RefreshStatus::Cancelled;
            return Ok(report);
        }

        match guard {
            Some(guard) => match guard.commit_with_publishers(merged, publishers).await {
                Ok((commit, publishers)) => {
                    self.registry.replace_all_dynamic_catalogs(commit.catalogs);
                    self.catalog_publishers = publishers;
                }
                Err(crate::catalog::StoreCommitError::Superseded { .. }) => {
                    report.status = RefreshStatus::Superseded;
                }
                Err(crate::catalog::StoreCommitError::Cancelled) => {
                    report.status = RefreshStatus::Cancelled;
                }
                Err(crate::catalog::StoreCommitError::Store(error)) => {
                    return Err(ModelsError::CatalogStore(error));
                }
            },
            None => {
                // A fetch can return success in the same poll that an external
                // caller cancels. Recheck at the synchronous memory commit
                // boundary, after every fetch and before replacing any state.
                if token.is_cancelled() {
                    report.status = RefreshStatus::Cancelled;
                    return Ok(report);
                }
                self.registry.replace_all_dynamic_catalogs(merged);
                self.catalog_publishers = publishers;
            }
        }
        Ok(report)
    }

    /// Attach a persisted dynamic-catalog store. Restores happen at the next
    /// refresh round; nothing is fetched or written here.
    pub fn set_models_store(&mut self, store: crate::catalog::ModelsStore) {
        self.models_store = Some(store);
        self.store_restored = false;
    }

    /// Register a dynamic-catalog source fetched during `refresh(true)`
    /// rounds. Registered explicitly; the SDK installs no default source.
    pub fn register_catalog_source(
        &mut self,
        source: std::sync::Arc<dyn crate::catalog::DynamicCatalogSource>,
    ) {
        self.catalog_sources.push(source);
    }
}

/// Validate a candidate dynamic catalog before it can replace the live
/// publication: a malformed or duplicate refresh must preserve the
/// last-known catalog (mirrors `register_model` checks).
fn validate_dynamic_catalog(provider_id: &str, models: &[ModelInfo]) -> Result<(), ProviderError> {
    let config = |message: String| ProviderError::Config(ProviderErrorSummary::sanitized(message));
    let mut seen = std::collections::HashSet::new();
    for model in models {
        if model.id.is_empty() {
            return Err(config(format!(
                "dynamic catalog for provider '{provider_id}' contains a model with an empty id"
            )));
        }
        if !seen.insert(model.id.as_str()) {
            return Err(config(format!(
                "dynamic catalog for provider '{provider_id}' has duplicate model id '{}'",
                model.id
            )));
        }
        if let Err(source) = model.validate() {
            return Err(config(format!(
                "dynamic catalog for provider '{provider_id}' has invalid model '{}': {source}",
                model.id
            )));
        }
    }
    Ok(())
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
/// resolved authentication. `start_attempt` reuses the
/// An admitted strict-downgrade variant (task 20.11): exactly one rebuilt
/// payload variant after a source-qualified Anthropic strict-limit
/// rejection, with its machine-readable reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrictVariantAdmission {
    /// The recorded downgrade reason (e.g. `strict_tool_limits_exceeded`).
    pub reason: &'static str,
}

/// frozen route, request, and authentication for each permitted sequential
/// retry without repeating route/auth preparation. The secret never enters
/// [`PreparedRoute`], Agent-visible state, evidence, diagnostics, or
/// model-visible state.
pub struct PreparedProviderCall {
    provider: Arc<dyn Provider>,
    request: Box<Request>,
    auth: ResolvedAuth,
    route: PreparedRoute,
    /// Set when an attempt reports that the frozen credential is unavailable
    /// or revoked; later attempts fail closed instead of redispatching it.
    credential_terminal: Arc<AtomicBool>,
    /// The acquisition-retry owner: per-variant send counters, retry
    /// admission, readiness deadline, and sealed states.
    acquisition: Arc<crate::retry::AcquisitionCore>,
    /// The logical call's stable retry-invocation identifier (the wire
    /// `amz-sdk-invocation-id`; task 20.20).
    invocation_id: String,
    resources: Option<crate::codex_resources::CallResourceLease>,
    resource_owner: Option<Arc<dyn crate::codex_resources::SessionResourceOwner>>,
    // Retain the complete record without inflating callers' inline async state.
    selected_model: Option<Box<ModelInfo>>,
    simple_options: SimpleOptions,
}

struct AttemptStream {
    inner: EventStream,
    cancel: CancellationToken,
    credential_terminal: Arc<AtomicBool>,
    acquisition: Arc<crate::retry::AcquisitionCore>,
    released: bool,
    /// Whether this attempt was ever polled (dispatched). Distinguishes a
    /// lease released before send from a dropped dispatch with unknown
    /// effect.
    polled: bool,
    _resource_owner: Option<Arc<dyn crate::codex_resources::SessionResourceOwner>>,
}

impl AttemptStream {
    fn release(&mut self) {
        if !self.released {
            // A released attempt must never poll its old producer or mutate
            // the acquisition state after a subsequent attempt starts.
            self.inner = Box::pin(stream::empty());
            self.acquisition.release_attempt();
            self.released = true;
        }
    }
}

impl Stream for AttemptStream {
    type Item = Result<AssistantStreamEvent, ProviderError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.released {
            return Poll::Ready(None);
        }
        if !this.polled {
            if this.cancel.is_cancelled() {
                this.acquisition.note_failure(&ProviderError::Cancelled);
                this.release();
                return Poll::Ready(Some(Err(ProviderError::Cancelled)));
            }
            this.polled = true;
            this.acquisition.note_dispatch();
        }
        let item = this.inner.as_mut().poll_next(cx);
        match &item {
            Poll::Ready(Some(Err(cause))) => {
                if matches!(
                    cause,
                    ProviderError::CredentialNeeded { .. }
                        | ProviderError::CredentialRevoked { .. }
                ) {
                    this.credential_terminal.store(true, Ordering::Release);
                }
                this.acquisition.note_failure(cause);
            }
            Poll::Ready(Some(Ok(_))) => {
                // The first delivered stream item is the streaming-response
                // handoff; it seals acquisition retry for this logical call.
                this.acquisition.note_handoff();
            }
            Poll::Ready(None) => {
                this.acquisition.note_finished();
            }
            Poll::Pending => {}
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
        if !self.released {
            if self.polled {
                // Dispatched but dropped without an observed closed outcome:
                // the effect is unknown and further scheduling is refused.
                self.acquisition.note_unknown_dispatch();
            }
            self.release();
        }
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

    /// The SDK-owned admission for the next attempt, if the last observed
    /// failure was retryable and the frozen per-variant budget still has
    /// room.
    ///
    /// The caller reads the [`RetryAdmission`](crate::retry::RetryAdmission), waits `delay_ms`
    /// (cancellably, through its own token — the shared
    /// [`Request::cancel`] token also forbids any further attempt once
    /// cancelled), and then calls [`start_attempt`](Self::start_attempt)
    /// again. Starting before the deadline returns
    /// [`ModelsError::RetryNotReady`] with zero dispatch.
    pub fn retry_admission(&self) -> Option<crate::retry::RetryAdmission> {
        self.acquisition.admission()
    }

    /// The sealed state, if this prepared call no longer admits acquisition
    /// attempts (post-handoff, unknown-dispatch, exhausted budget, or a
    /// terminal outcome).
    pub fn acquisition_seal(&self) -> Option<crate::retry::AcquisitionSeal> {
        self.acquisition.seal()
    }

    /// The pending strict-downgrade variant admission, if the last observed
    /// failure was a source-qualified Anthropic strict-limit rejection on
    /// the first variant (task 20.11, V-34/V-36).
    ///
    /// The caller dispatches it immediately through
    /// [`start_attempt`](Self::start_attempt) — no readiness wait: the
    /// rebuilt variant removes only prefer-strict, re-runs the payload
    /// callback under its own freeze slot, and keeps the frozen
    /// headers/auth/model. At most one exists per logical call and nothing
    /// resets the counters to gain another.
    pub fn strict_variant_admission(&self) -> Option<StrictVariantAdmission> {
        self.acquisition
            .strict_variant_admission()
            .map(|reason| StrictVariantAdmission { reason })
    }

    /// Start one attempt using the frozen request and authentication.
    ///
    /// Reserves the active lease immediately. The adapter and its producer
    /// are constructed on the stream's first poll; dropping an unpolled
    /// lease sends nothing and leaves it available for a later start.
    ///
    /// Reuses the prepared route and authentication; does not repeat preparation.
    /// Every attempt shares the frozen [`Request::cancel`] token: cancelling it
    /// terminates the logical call and forbids any further attempt. At most one
    /// attempt stream may be active at a time; a sequential retry must wait until
    /// the prior attempt's stream reaches a terminal event or error, which
    /// releases the active slot. A credential-needed or credential-revoked
    /// stream error additionally terminates the prepared call and forbids retry.
    /// The acquisition owner refuses dispatch while the call is sealed
    /// (post-handoff, unknown-dispatch, exhausted budget, or a terminal
    /// outcome) and before an admitted retry's readiness deadline.
    ///
    /// # Errors
    ///
    /// - [`ModelsError::CallCancelled`] if the shared cancellation token has
    ///   been cancelled.
    /// - [`ModelsError::CredentialTerminated`] if a prior attempt rejected
    ///   the frozen credential.
    /// - [`ModelsError::CallSealed`] if the acquisition owner sealed the call.
    /// - [`ModelsError::RetryNotReady`] if the admitted retry's wait has not
    ///   elapsed yet.
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
        if let Err(error) = self.acquisition.check_start() {
            return Err(match error {
                crate::retry::RetryStartError::AlreadyActive => ModelsError::AttemptAlreadyActive,
                crate::retry::RetryStartError::Sealed(seal) => ModelsError::CallSealed { seal },
                crate::retry::RetryStartError::NotReady { retry_ready_in_ms } => {
                    ModelsError::RetryNotReady { retry_ready_in_ms }
                }
            });
        }
        if self.request.cancel.is_cancelled() {
            self.acquisition.release_attempt();
            return Err(ModelsError::CallCancelled);
        }
        // Recheck revocation after atomically reserving the execution lease.
        if self.credential_terminal.load(Ordering::Acquire) {
            self.acquisition.release_attempt();
            return Err(ModelsError::CredentialTerminated {
                provider: self.route.provider_id.clone(),
            });
        }
        // Adapters may eagerly spawn their producer in `stream_prepared`.
        // Defer that invocation too, so an unpolled lease has no effects.
        let provider = self.provider.clone();
        let mut request = (*self.request).clone();
        let auth = self.auth.clone();
        let attempt = self.acquisition.attempt_context(&self.invocation_id);
        request.callbacks = request.callbacks.with_attempt(attempt.clone());
        let resources = self.resources.clone();
        let selected_model = self.selected_model.clone();
        let simple_options = self.simple_options.clone();
        let stream = Box::pin(
            stream::once(async move {
                match selected_model {
                    Some(model) => provider.stream_model_resource_attempt(
                        *model,
                        simple_options,
                        request,
                        auth,
                        attempt,
                        resources,
                    ),
                    None => provider.stream_resource_attempt(request, auth, attempt, resources),
                }
            })
            .flatten(),
        );
        Ok(Box::pin(AttemptStream {
            inner: stream,
            cancel: self.request.cancel.clone(),
            credential_terminal: self.credential_terminal.clone(),
            acquisition: self.acquisition.clone(),
            released: false,
            polled: false,
            _resource_owner: self.resource_owner.clone(),
        }))
    }
}

/// Drive ordinary and direct calls with the same per-call acquisition owner
/// used by advanced preparation. Admitted retries consume their existing
/// counters; only the source-qualified strict admission creates a new variant.
pub(crate) fn drive_resolved_call(
    provider: Arc<dyn Provider>,
    request: Request,
    auth: ResolvedAuth,
) -> EventStream {
    if let Some(attempt) = request.callbacks.inherited_attempt() {
        return provider.stream_attempt(request, auth, attempt);
    }
    // Direct adapters are lazy too. Retain their registration in the public
    // stream, rather than only in a producer that can finish before delivery.
    Box::pin(
        stream::once(async move {
            let mut request = request;
            request.callbacks = request.callbacks.for_call();
            let resources = provider
                .session_resources()
                .and_then(|owner| owner.register_call(&request));
            drive_resolved_call_registered(
                provider,
                request,
                auth,
                resources,
                None,
                None,
                SimpleOptions::default(),
            )
        })
        .flatten(),
    )
}

fn drive_resolved_call_registered(
    provider: Arc<dyn Provider>,
    mut request: Request,
    auth: ResolvedAuth,
    resources: Option<crate::codex_resources::CallResourceLease>,
    resource_owner: Option<Arc<dyn crate::codex_resources::SessionResourceOwner>>,
    selected_model: Option<ModelInfo>,
    simple_options: SimpleOptions,
) -> EventStream {
    request.callbacks = request.callbacks.ensure_call_scope();
    request.cancel = resources
        .as_ref()
        .map_or_else(|| request.cancel.child_token(), |lease| lease.token());
    let route = PreparedRoute {
        provider_id: provider.id().to_owned(),
        model_id: request
            .model
            .split_once(':')
            .map_or(request.model.as_str(), |(_, id)| id)
            .to_owned(),
        wire_api: provider
            .models()
            .iter()
            .find(|model| {
                model.id
                    == request
                        .model
                        .split_once(':')
                        .map_or(request.model.as_str(), |(_, id)| id)
            })
            .map_or(crate::model_info::WireApi::OpenAiCompletions, |model| {
                model.wire_api
            }),
    };
    let policy = provider.acquisition_policy_default();
    let prepared = PreparedProviderCall {
        provider,
        request: Box::new(request),
        auth,
        route,
        credential_terminal: Arc::new(AtomicBool::new(false)),
        acquisition: Arc::new(crate::retry::AcquisitionCore::new(policy)),
        invocation_id: uuid::Uuid::new_v4().to_string(),
        resources,
        resource_owner,
        selected_model: selected_model.map(Box::new),
        simple_options,
    };
    Box::pin(stream::unfold(
        (prepared, None::<EventStream>, false),
        |(prepared, mut current, finished)| async move {
            use futures_util::StreamExt;
            if finished {
                return None;
            }
            loop {
                if current.is_none() {
                    current = match prepared.start_attempt() {
                        Ok(attempt) => Some(attempt),
                        Err(error) => {
                            return Some((Err(models_error_cause(error)), (prepared, None, true)));
                        }
                    };
                }
                match current.as_mut().expect("attempt initialized").next().await {
                    Some(Err(error)) => {
                        if prepared.strict_variant_admission().is_some() {
                            current = None;
                            continue;
                        }
                        if let Some(admission) = prepared.retry_admission() {
                            tokio::select! {
                                biased;
                                _ = prepared.request.cancel.cancelled() => {
                                    return Some((Err(ProviderError::Cancelled), (prepared, None, true)));
                                }
                                _ = tokio::time::sleep(Duration::from_millis(admission.delay_ms)) => {}
                            }
                            current = None;
                            continue;
                        }
                        return Some((Err(error), (prepared, None, true)));
                    }
                    Some(Ok(event)) => {
                        let terminal = event.is_terminal();
                        return Some((Ok(event), (prepared, current, terminal)));
                    }
                    None => return None,
                }
            }
        },
    ))
}

// ---------------------------------------------------------------------------
// Shared compiler and auth resolution (the sole state machine)
// ---------------------------------------------------------------------------

/// Request preparation for neutral calls and the advanced entry: default
/// output limit, context clamp, thinking resolution, and
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
        // Canonical system/tool declarations carry chronology and timestamp
        // facts needed to validate an anchor and estimate its tail. Passing
        // replayed current values separately would lose those facts or count
        // the same declaration twice.
        let occupied =
            crate::context::estimate_context_tokens(None, request.context.messages(), &[]).tokens;
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
        // Task 20.42: budget-based Claude models on the bedrock wire share
        // the numeric neutral policy (pi streamSimple's adjust/clamp path);
        // adaptive models keep the level flowing to `output_config.effort`.
        let numeric_bedrock = model.wire_api == crate::WireApi::BedrockConverseStream
            && !crate::bedrock::supports_adaptive_thinking(&model.id, &model.display_name);
        if (numeric_anthropic || numeric_bedrock) && request.thinking.budget_tokens.is_none() {
            request.thinking.budget_tokens = Some(match request.thinking.level {
                crate::ThinkingLevel::Minimal => 1_024,
                crate::ThinkingLevel::Low => 2_048,
                crate::ThinkingLevel::Medium => 8_192,
                crate::ThinkingLevel::High
                | crate::ThinkingLevel::XHigh
                | crate::ThinkingLevel::Max => 16_384,
                // Legacy callers that enable thinking without selecting a
                // level or budget take the source native default (task
                // 20.39, pi `thinkingBudgetTokens || 1024`).
                crate::ThinkingLevel::None => 1_024,
            });
        }
        if let (Some(budget), Some(limit)) = (request.thinking.budget_tokens, request.max_tokens) {
            request.thinking.budget_tokens = Some(budget.min(limit.saturating_sub(1_024)));
        }
        // Task 20.39 (annex row 165): no typed preflight for sub-1024
        // numeric budgets — the wire assembly applies the source
        // `|| 1024` semantics (a zero budget sends 1024; nonzero budgets
        // pass through).
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
    /// Explicit tool selection on an adapter that admits this neutral option.
    pub tool_choice: Option<SimpleToolChoice>,
    /// Sampling temperature override.
    pub temperature: Option<f64>,
    /// Output-limit override.
    pub max_tokens: Option<u64>,
    /// Per-call timeout override.
    pub timeout: Option<Duration>,
    /// Stop-sequence override (applied when non-empty).
    pub stop_sequences: Vec<String>,
}

/// Neutral tool selection. Adapter-specific forced-tool forms remain native.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimpleToolChoice {
    Auto,
    None,
}

impl SimpleOptions {
    pub(crate) fn validate_default(&self) -> Result<(), ProviderError> {
        if self.tool_choice.is_some() {
            return Err(ProviderError::UnsupportedCapability(
                ProviderErrorSummary::attested_static(
                    "this adapter does not support the neutral tool selection option",
                ),
            ));
        }
        Ok(())
    }
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
    /// Validate explicit neutral options before auth. Adapters opt in only to
    /// options their simple mapping consumes; defaults reject new wire inputs.
    fn validate_simple_options(
        &self,
        _model: &ModelInfo,
        options: &SimpleOptions,
    ) -> Result<(), ProviderError> {
        options.validate_default()
    }
    /// Validate the fields consumed by this API's native entry before auth.
    /// Native adapters that replace neutral fields validate their own options
    /// and retain the common structural/model checks.
    fn validate_native_request(
        &self,
        model: &ModelInfo,
        request: &Request,
        _options: &O,
    ) -> Result<(), ProviderError> {
        crate::provider::validate_request_for_model(self.provider_id(), Some(model), request)
    }

    /// Dispatch with the caller's frozen complete record. The default keeps
    /// existing custom adapters source-compatible; metadata-aware adapters
    /// consume this record rather than their construction-time exemplar.
    fn stream_model_resource_prepared(
        &self,
        _model: ModelInfo,
        request: Request,
        options: O,
        auth: ResolvedAuth,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        self.stream_resource_prepared(request, options, auth, resources)
    }

    /// Map the neutral options consumed by this registered API.
    fn simple_options_for(&self, _options: &SimpleOptions) -> O {
        self.simple_options()
    }

    /// Instance-owned connection resources, when this API has a transport pool.
    fn session_resources(&self) -> Option<&dyn crate::codex_resources::SessionResourceOwner> {
        None
    }

    /// Consume a generation registered before authentication. Resource-owning
    /// APIs forward it to their real adapter; other APIs use the ordinary seam.
    fn stream_resource_prepared(
        &self,
        request: Request,
        options: O,
        auth: ResolvedAuth,
        _resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        self.stream_prepared(request, options, auth)
    }
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
    // The namespace follows this exact adapter revision, independently of a
    // later provider-route replacement under the same provider id.
    resource_owner: Option<Arc<dyn crate::codex_resources::SessionResourceOwner>>,
}

/// The registration snapshot captured by a call at creation.
struct ChatApiEntry {
    provider_id: String,
    options_type: TypeId,
    dispatch: Arc<dyn ChatApiDispatch>,
}

/// Object-safe dispatch over one fixed options type.
trait ChatApiDispatch: Send + Sync {
    fn validate_simple_erased(
        &self,
        model: &ModelInfo,
        options: &SimpleOptions,
    ) -> Result<(), ProviderError>;
    fn simple_options_erased(&self, options: &SimpleOptions) -> Box<dyn Any + Send>;
    fn validate_native_erased(
        &self,
        model: &ModelInfo,
        request: &Request,
        options: &(dyn Any + Send),
    ) -> Result<(), ProviderError>;
    fn stream_prepared_erased(
        &self,
        model: ModelInfo,
        request: Request,
        options: Box<dyn Any + Send>,
        auth: ResolvedAuth,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream;
}

struct TypedChatApi<O: Send + 'static> {
    api: Arc<dyn ChatApi<O>>,
}

impl<O: Send + 'static> ChatApiDispatch for TypedChatApi<O> {
    fn validate_simple_erased(
        &self,
        model: &ModelInfo,
        options: &SimpleOptions,
    ) -> Result<(), ProviderError> {
        self.api.validate_simple_options(model, options)
    }
    fn simple_options_erased(&self, options: &SimpleOptions) -> Box<dyn Any + Send> {
        Box::new(self.api.simple_options_for(options))
    }

    fn validate_native_erased(
        &self,
        model: &ModelInfo,
        request: &Request,
        options: &(dyn Any + Send),
    ) -> Result<(), ProviderError> {
        let options = options.downcast_ref::<O>().ok_or_else(|| {
            ProviderError::Config(ProviderErrorSummary::attested_static(
                "native options type does not match the registered chat api",
            ))
        })?;
        self.api.validate_native_request(model, request, options)
    }

    fn stream_prepared_erased(
        &self,
        model: ModelInfo,
        request: Request,
        options: Box<dyn Any + Send>,
        auth: ResolvedAuth,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        match options.downcast::<O>() {
            Ok(options) => self
                .api
                .stream_model_resource_prepared(model, request, *options, auth, resources),
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
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        options: &SimpleOptions,
    ) -> Result<(), ProviderError> {
        self.dispatch.validate_simple_erased(model, options)
    }
    fn id(&self) -> &str {
        &self.provider_id
    }

    fn models(&self) -> &[ModelInfo] {
        // Catalog-invisible by contract: no records are listed or resolvable.
        &[]
    }

    fn stream_prepared(&self, _request: Request, _auth: ResolvedAuth) -> EventStream {
        Box::pin(stream::once(async move {
            Err(ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "registered chat APIs require a complete model record through Models",
                ),
            ))
        }))
    }

    fn stream_model_resource_attempt(
        &self,
        model: ModelInfo,
        simple: SimpleOptions,
        request: Request,
        auth: ResolvedAuth,
        _attempt: crate::provider::AttemptContext,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        let options = self.dispatch.simple_options_erased(&simple);
        self.dispatch
            .stream_prepared_erased(model, request, options, auth, resources)
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
    resource_owner: Option<Arc<dyn crate::codex_resources::SessionResourceOwner>>,
    registration_gate: Arc<std::sync::Mutex<()>>,
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
    Simple(SimpleOptions),
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

struct ResourceApiStream {
    inner: EventStream,
    _resources: Option<crate::codex_resources::CallResourceLease>,
    _owner: Option<Arc<dyn crate::codex_resources::SessionResourceOwner>>,
}

impl Stream for ResourceApiStream {
    type Item = Result<AssistantStreamEvent, ProviderError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().inner.as_mut().poll_next(cx)
    }
}

enum ChatCallState {
    NotStarted(Box<ChatCallSetup>),
    Starting(BoxFuture<'static, Result<ChatStream, CallFailure>>),
    Active(Box<ChatStream>),
    Done,
}

impl ChatCall {
    fn new(binding: FrozenChatBinding, mut request: Request, options: CallOptions) -> Self {
        request.callbacks = request.callbacks.for_call();
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
    let resources = {
        let _registration = binding.registration_gate.lock().unwrap();
        binding
            .resource_owner
            .as_ref()
            .and_then(|owner| owner.register_call(&request))
    };
    request.cancel = resources
        .as_ref()
        .map_or_else(|| request.cancel.child_token(), |lease| lease.token());
    if request.cancel.is_cancelled() {
        return Err(CallFailure::from_partial(None, ProviderError::Cancelled));
    }

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
            entry
                .dispatch
                .validate_native_erased(binding.record.info(), &request, value.as_ref())
                .map_err(|error| CallFailure::from_partial(None, error))?;
            let auth = resolve_binding_auth(&binding.auth_plan, &request.cancel).await?;
            let retained_resources = resources.clone();
            let stream = entry.dispatch.stream_prepared_erased(
                binding.record.info().clone(),
                request,
                value,
                auth,
                resources,
            );
            Box::pin(ResourceApiStream {
                inner: stream,
                _resources: retained_resources,
                _owner: binding.resource_owner,
            }) as EventStream
        }
        FrozenRoute::Provider {
            provider_id,
            provider,
        } => {
            let Some(provider) = provider else {
                return Err(setup_failure(format!("unknown provider: {provider_id}")));
            };
            let CallOptions::Simple(simple_options) = options else {
                unreachable!("native entries dispatch through chat APIs")
            };
            provider
                .validate_simple_options(binding.record.info(), &simple_options)
                .map_err(|error| CallFailure::from_partial(None, error))?;
            compile_prepared_request(
                binding.record.provider_id(),
                binding.record.info(),
                &mut request,
            )
            .map_err(|error| CallFailure::from_partial(None, error))?;
            let auth = resolve_binding_auth(&binding.auth_plan, &request.cancel).await?;
            drive_resolved_call_registered(
                provider,
                request,
                auth,
                resources,
                binding.resource_owner,
                Some(binding.record.info().clone()),
                simple_options,
            )
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
