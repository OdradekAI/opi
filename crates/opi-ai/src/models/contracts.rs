//! Public contracts and result types for the [`Models`](crate::models::Models) facade.

use super::*;

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
    /// [`crate::CredentialSource`] probe state is injected separately via
    /// [`Models::set_probe`] and consulted by
    /// [`Models::check_auth`] when no live store is attached.
    StoreCredential {
        /// Store account key (typically the provider id).
        key: String,
        /// Non-secret display label (e.g. `keychain opi:anthropic`).
        display_source: String,
    },
}

pub(crate) struct BuiltinAuthStatus {
    pub check: AuthCheck,
    pub available_model_ids: Option<Vec<String>>,
    pub responses_only: bool,
    pub classifiers_allowed: bool,
}

pub(crate) type BuiltinAuthCheck = Arc<
    dyn Fn() -> crate::credential::BoxAuthFuture<
            'static,
            Result<Option<BuiltinAuthStatus>, ProviderError>,
        > + Send
        + Sync,
>;

pub(crate) struct BuiltinAuthBindings {
    pub chat: Arc<dyn AuthResolver>,
    pub classifier: Arc<dyn AuthResolver>,
    pub check: BuiltinAuthCheck,
}

pub(crate) type BuiltinAuthFactory = Arc<
    dyn Fn(
            Option<Arc<dyn CredentialStore>>,
            Option<Arc<dyn crate::auth::OAuthProvider>>,
        ) -> BuiltinAuthBindings
        + Send
        + Sync,
>;

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
    /// Operation-qualified image selection failed: the model is not a
    /// registered image model (a chat model or an unknown id).
    #[error("model '{provider}/{id}' is not an image model")]
    NotAnImageModel {
        /// Provider id of the requested model.
        provider: String,
        /// Bare model id of the requested model.
        id: String,
    },
    /// Operation-qualified classifier selection failed: the model is not a
    /// registered classifier model (a chat model or an unknown id).
    #[error("model '{provider}/{id}' is not a classifier model")]
    NotAClassifierModel {
        /// Provider id of the requested model.
        provider: String,
        /// Bare model id of the requested model.
        id: String,
    },
    /// A record append targeted an API identity with no classifier
    /// registration.
    #[error("no classifier API registered for '{api}'")]
    ClassifierApiNotRegistered {
        /// API identity the record carried.
        api: String,
    },
    /// An image record has no matching provider/API registration.
    #[error("no image API registered for '{api}'")]
    ImageApiNotRegistered {
        /// API identity the record carried.
        api: String,
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

impl ModelsError {
    /// Whether the logical call's own cancellation interrupted preparation or
    /// forbids another attempt. Authentication resolvers returning a cancellation
    /// error independently do not establish this call-cancellation origin.
    pub fn is_call_cancelled(&self) -> bool {
        matches!(self, Self::CallCancelled)
            || matches!(self, Self::Provider(error) if error.is_call_cancelled())
    }
}

/// Caller selection and source policy for an explicit model refresh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelsRefreshOptions {
    /// Permit source network work after restoring selected cached records.
    pub allow_network: bool,
    /// Selected provider IDs; `None` selects all active owners and an empty
    /// set selects none. Unknown IDs perform no source work.
    pub providers: Option<BTreeSet<String>>,
    /// Optional source-specific forced refresh, forwarded only with network work.
    pub force: Option<bool>,
}

impl Default for ModelsRefreshOptions {
    fn default() -> Self {
        Self {
            allow_network: true,
            providers: None,
            force: None,
        }
    }
}

/// Outcome of one refresh round, reported per provider or source id.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum RefreshOutcome {
    /// The entry's dynamic catalog was published in the new revision.
    Published {
        /// Number of models in the published snapshot. Zero can denote an
        /// authoritative empty catalog or the absence of a dynamic entry.
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
    pub(crate) status: RefreshStatus,
    pub(crate) entries: Vec<RefreshReportEntry>,
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
