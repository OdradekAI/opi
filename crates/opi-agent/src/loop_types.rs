//! Types for the agent loop.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use opi_ai::provider::{
    ProviderError, ProviderErrorCategory, ProviderErrorSummary, ThinkingConfig,
};
use opi_ai::{CollectionError, ProviderCollection, RegistryError};

use crate::authority::{ToolAuthorizer, ToolRegistry};
use crate::diagnostic_sink::DiagnosticSink;
use crate::evidence::{EvidenceHealth, EvidenceSink};
use crate::message::AgentMessage;

/// A provider failure after it crosses the Agent boundary.
///
/// The complete typed [`ProviderError`] is retained so variant-specific stable
/// codes and safe metadata survive unchanged. Any public text remains limited
/// to the closed or redacted summaries constructed by `opi-ai`.
#[derive(Debug)]
pub struct AgentProviderFailure {
    error: ProviderError,
}

/// Intrinsic reason a complete durable next-turn state cannot be stored.
///
/// This boundary intentionally excludes transformed request content, headers,
/// authentication, and provider I/O. Those remain validated by the provider
/// collection after request transforms have run.
#[derive(Debug, thiserror::Error)]
pub enum InvalidNextTurnReason {
    /// The selected canonical route is unknown or is not dispatchable.
    #[error(transparent)]
    Route(#[from] CollectionError),
    /// The raw selected identity differs from the canonical resolved route.
    #[error(
        "selected route '{selected_provider}:{selected_model}' is not canonical for resolved route '{resolved_provider}:{resolved_model}'"
    )]
    NonCanonicalModelSelection {
        selected_provider: String,
        selected_model: String,
        resolved_provider: String,
        resolved_model: String,
    },
    /// The resolved model metadata violates its intrinsic constraints.
    #[error("model '{model}' for provider '{provider}' has invalid constraints: {source}")]
    InvalidModelConstraints {
        provider: String,
        model: String,
        #[source]
        source: opi_ai::model_info::ModelInfoError,
    },
    /// The selected model cannot represent the requested thinking level.
    #[error(
        "model '{model}' for provider '{provider}' does not support thinking level '{level:?}'"
    )]
    UnsupportedThinking {
        provider: String,
        model: String,
        level: opi_ai::ThinkingLevel,
    },
    /// A floating-point inference scalar cannot be represented in JSON.
    #[error("temperature is not representable as a JSON number")]
    NonFiniteTemperature,
}

impl AgentProviderFailure {
    pub(crate) fn new(error: ProviderError) -> Self {
        Self { error }
    }

    /// Stable provider error category, independent of display text.
    pub fn category(&self) -> ProviderErrorCategory {
        self.error.category()
    }

    /// Exact stable diagnostic code for the retained provider variant.
    pub fn code(&self) -> &'static str {
        crate::diagnostic::Diagnostic::from(&self.error).code
    }

    /// Redaction-safe public summary carried by variants that own one.
    pub fn summary(&self) -> Option<&ProviderErrorSummary> {
        match &self.error {
            ProviderError::RequestFailed(summary)
            | ProviderError::StreamError(summary)
            | ProviderError::AuthFailed(summary)
            | ProviderError::Network(summary)
            | ProviderError::Config(summary)
            | ProviderError::ProviderSide(summary)
            | ProviderError::UnsupportedCapability(summary) => Some(summary),
            ProviderError::RateLimited { .. }
            | ProviderError::Timeout
            | ProviderError::CredentialNeeded { .. }
            | ProviderError::CredentialRevoked { .. }
            | ProviderError::AccountIdMissing { .. }
            | ProviderError::LoginCancelled { .. }
            | ProviderError::UnknownModel { .. }
            | ProviderError::MissingWireRoute { .. }
            | ProviderError::WireCompatMismatch { .. }
            | ProviderError::Cancelled => None,
        }
    }

    /// Retained provider failure with its safe typed metadata intact.
    pub fn provider_error(&self) -> &ProviderError {
        &self.error
    }
}

impl std::fmt::Display for AgentProviderFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(formatter)
    }
}

/// Errors that can occur during the agent loop.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("provider error: {0}")]
    Provider(AgentProviderFailure),
    #[error("invalid model spec: {spec}")]
    InvalidModelSpec { spec: String },
    #[error("unknown provider: {provider}")]
    UnknownProvider { provider: String },
    #[error("unknown model '{model}' for provider '{provider}'")]
    UnknownModel { provider: String, model: String },
    #[error("authentication failed: {0}")]
    AuthFailed(ProviderErrorSummary),
    #[error("credential needed for '{provider_id}': run /login {provider_id}")]
    CredentialNeeded { provider_id: String },
    #[error("credential revoked for '{provider_id}': login required")]
    CredentialRevoked { provider_id: String },
    /// The provider requires an account id that the stored credential does not
    /// carry (e.g. an OpenAI Codex token missing `chatgpt_account_id`). Kept
    /// distinct from [`AgentError::CredentialRevoked`] per the exit-remediation
    /// typed-failure contract: JSON/RPC/text modes emit the canonical provider
    /// id and a `/login <provider>` remediation.
    #[error("account id missing for '{provider_id}': run /login {provider_id}")]
    AccountIdMissing { provider_id: String },
    #[error("hook error: {0}")]
    Hook(String),
    #[error("cancelled")]
    Cancelled,
    /// An opaque armed-run capability belongs to another Agent or is no
    /// longer the latest generation armed by this Agent.
    #[error("armed run does not match this Agent's latest generation")]
    InvalidArmedRun,
    /// A tool reported a typed execution-boundary failure. Partial side effect
    /// and cleanup uncertainty remain distinguishable through the wrapped
    /// [`crate::tool::ToolError`] variant.
    #[error("tool error: {0}")]
    Tool(#[from] crate::tool::ToolError),
    #[error("max turns exceeded ({0})")]
    MaxTurnsExceeded(u32),
    /// Evidence capture setup failed before the run (fail-closed):
    /// the configured evidence sink could not be prepared. The run is aborted
    /// before its first provider/tool call so it never runs with incomplete
    /// evidence when capture was explicitly requested.
    #[error("evidence setup failed: {0}")]
    EvidenceSetup(String),
    /// Evidence capture could not be finalized after the run. Explicit capture
    /// is fail-visible: a durability/completeness failure is returned to the
    /// caller instead of being silently discarded after the model/tool work.
    #[error("evidence finalization failed: {0}")]
    EvidenceFinalization(String),
    /// A requested product session could not be reopened. The harness returns
    /// this before any new provider/tool turn starts instead of silently
    /// replacing the requested resume with a fresh sessionless run.
    #[error("session resume failed: {0}")]
    SessionResume(String),
    /// A completed turn could not be durably appended to its active session.
    #[error("session persistence failed: {0}")]
    SessionPersist(String),
    /// A provider route could not be prepared for a model call: the selection
    /// was unknown, ambiguous, undispatchable, or its authentication could not
    /// be resolved. Collection-owned preparation failures surface at this typed
    /// boundary rather than as a generic provider string.
    #[error("provider route not dispatchable for '{provider}'")]
    RouteNotDispatchable { provider: String },
    /// The request's canonical model identity disagreed with the route selected
    /// for the logical call, so dispatch was rejected before provider I/O.
    #[error(
        "request model '{request_model}' does not match resolved route '{route_provider}:{route_model}'"
    )]
    RequestRouteMismatch {
        request_model: String,
        route_provider: String,
        route_model: String,
    },
    /// A prior attempt made the prepared call's frozen credential terminal.
    #[error("credential failure terminated the prepared call for provider '{provider}'")]
    CredentialTerminated { provider: String },
    #[error("a prepared provider attempt is already active")]
    AttemptAlreadyActive,
    #[error("provider protocol failure: {detail}")]
    ProviderProtocol { detail: ProviderErrorSummary },
    /// A complete initial, idle replacement, or hook-produced state was
    /// rejected at the shared intrinsic validation boundary. The prior state,
    /// when present, is preserved unchanged.
    #[error("invalid next-turn candidate state: {0}")]
    InvalidNextTurnCandidate(InvalidNextTurnReason),
    /// A trusted tool registration was rejected (e.g. two registrations share a
    /// provider-visible name).
    #[error("invalid tool registration: {0}")]
    InvalidToolRegistration(String),
}

impl From<ProviderError> for AgentError {
    fn from(error: ProviderError) -> Self {
        AgentError::Provider(AgentProviderFailure::new(error))
    }
}

impl From<CollectionError> for AgentError {
    fn from(error: CollectionError) -> Self {
        match error {
            CollectionError::Registry(RegistryError::InvalidSpec(spec)) => {
                AgentError::InvalidModelSpec { spec }
            }
            CollectionError::Registry(RegistryError::UnknownProvider(provider)) => {
                AgentError::UnknownProvider { provider }
            }
            CollectionError::Registry(RegistryError::UnknownModel { provider, model }) => {
                AgentError::UnknownModel { provider, model }
            }
            CollectionError::Registry(_) => AgentError::Provider(AgentProviderFailure::new(
                ProviderError::Config(ProviderErrorSummary::redacted()),
            )),
            CollectionError::RouteNotDispatchable { provider } => {
                AgentError::RouteNotDispatchable { provider }
            }
            CollectionError::RequestRouteMismatch {
                request_model,
                route_provider,
                route_model,
            } => AgentError::RequestRouteMismatch {
                request_model,
                route_provider,
                route_model,
            },
            CollectionError::AttemptAlreadyActive => AgentError::AttemptAlreadyActive,
            CollectionError::CallCancelled => AgentError::Cancelled,
            CollectionError::CredentialTerminated { provider } => {
                AgentError::CredentialTerminated { provider }
            }
            CollectionError::Provider(error) => AgentError::from(error),
        }
    }
}

/// Canonical provider:model selection for one logical model call.
///
/// The selection is always the canonical `provider_id:model_id` pair resolved
/// by the provider collection; Agent Core has no alias registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSelection {
    /// Provider identifier owning the route.
    pub provider_id: String,
    /// Model identifier within the provider.
    pub model_id: String,
}

impl ModelSelection {
    /// Build a selection from its two canonical parts.
    pub fn new(provider_id: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            model_id: model_id.into(),
        }
    }

    /// Parse a canonical `provider:model` spec. Returns `None` for a bare model
    /// or missing provider; bare-input normalization is owned by the Reference
    /// Product, not by Agent Core state.
    pub fn parse_spec(spec: &str) -> Option<Self> {
        let (provider_id, model_id) = opi_ai::registry::parse_model_spec(spec).ok()?;
        Some(Self {
            provider_id: provider_id.to_owned(),
            model_id: model_id.to_owned(),
        })
    }

    /// Render the canonical `provider:model` spec consumed by
    /// [`ProviderCollection::prepare_call`].
    pub fn to_spec(&self) -> String {
        format!("{}:{}", self.provider_id, self.model_id)
    }
}

/// Per-request inference knobs owned by [`NextTurnState`]. These are mutable
/// next-turn state (a preparation may change them atomically with context and
/// model), distinct from the immutable loop-control knobs in
/// [`AgentLoopConfig`].
#[derive(Debug, Clone, Default)]
pub struct InferenceConfig {
    /// Thinking/reasoning configuration for extended thinking models.
    pub thinking: ThinkingConfig,
    /// Maximum output tokens per request.
    pub max_tokens: Option<u64>,
    /// Sampling temperature.
    pub temperature: Option<f64>,
}

/// The complete mutable next-request state owned durably by the Agent. A loop
/// run returns its final complete state to the Agent, which
/// stores it before the public operation settles. Replacement is always a
/// complete validated value, never a patch, merge, or append.
#[derive(Debug, Clone)]
pub struct NextTurnState {
    /// Complete conversation context.
    pub context: Vec<AgentMessage>,
    /// Canonical provider:model selection for the next call.
    pub model_selection: ModelSelection,
    /// Per-request inference configuration.
    pub inference: InferenceConfig,
}

impl NextTurnState {
    /// Build a new complete next-turn state.
    pub fn new(
        context: Vec<AgentMessage>,
        model_selection: ModelSelection,
        inference: InferenceConfig,
    ) -> Self {
        Self {
            context,
            model_selection,
            inference,
        }
    }
}

/// Validate one complete next-turn candidate without performing authentication
/// or provider I/O.
///
/// Construction, public idle replacement, and in-loop hook application cross
/// this same boundary before storing the state. Only intrinsic durable-state
/// facts are checked here; transformed request content and headers are checked
/// later by the provider collection before authentication and dispatch.
pub(crate) fn validate_next_turn_candidate(
    collection: &ProviderCollection,
    candidate: &NextTurnState,
) -> Result<(), AgentError> {
    let model_spec = candidate.model_selection.to_spec();

    collection
        .validate_dispatchable_route(&model_spec)
        .map_err(InvalidNextTurnReason::Route)
        .map_err(AgentError::InvalidNextTurnCandidate)?;

    let (provider, model) = collection
        .resolve(&model_spec)
        .map_err(CollectionError::Registry)
        .map_err(InvalidNextTurnReason::Route)
        .map_err(AgentError::InvalidNextTurnCandidate)?;
    if provider.id() != candidate.model_selection.provider_id
        || model.id != candidate.model_selection.model_id
    {
        return Err(AgentError::InvalidNextTurnCandidate(
            InvalidNextTurnReason::NonCanonicalModelSelection {
                selected_provider: candidate.model_selection.provider_id.clone(),
                selected_model: candidate.model_selection.model_id.clone(),
                resolved_provider: provider.id().to_owned(),
                resolved_model: model.id.clone(),
            },
        ));
    }
    model.validate().map_err(|source| {
        AgentError::InvalidNextTurnCandidate(InvalidNextTurnReason::InvalidModelConstraints {
            provider: provider.id().to_owned(),
            model: model.id.clone(),
            source,
        })
    })?;
    if candidate.inference.thinking.enabled
        && (!model.capabilities.supports_thinking
            || model
                .thinking_level_map
                .resolve(candidate.inference.thinking.level)
                .is_err())
    {
        return Err(AgentError::InvalidNextTurnCandidate(
            InvalidNextTurnReason::UnsupportedThinking {
                provider: provider.id().to_owned(),
                model: model.id.clone(),
                level: candidate.inference.thinking.level,
            },
        ));
    }

    if candidate
        .inference
        .temperature
        .is_some_and(|temperature| !temperature.is_finite())
    {
        return Err(AgentError::InvalidNextTurnCandidate(
            InvalidNextTurnReason::NonFiniteTemperature,
        ));
    }
    Ok(())
}

/// Input context for the agent loop.
pub struct AgentLoopContext {
    /// The dispatchable provider collection that owns route lookup, per-call
    /// auth preparation, and attempt dispatch. The loop prepares
    /// one call per turn and opens every retry attempt from that same prepared
    /// call.
    pub collection: Arc<ProviderCollection>,
    /// Immutable trusted-tool registry. The loop resolves every
    /// model-proposed tool name against this registry, projects provider-facing
    /// definitions from it, and executes only implementations reachable through
    /// a current authorization `Allow`.
    pub registry: Arc<ToolRegistry>,
    /// Mandatory trusted tool authorizer bound to the effective User Policy for
    /// the run. `None` is fail-closed: no tool executes without a current
    /// `Allow` (AUT-005).
    pub authorizer: Option<Arc<dyn ToolAuthorizer>>,
    /// Current versioned evidence health at run start. The loop advances its
    /// local value after sink failure, injects the current generation into each
    /// authorization request, and reauthorizes a stale `Allow` before launch.
    pub evidence_health: EvidenceHealth,
    /// Optional evidence sink binding the run's call-graph lifecycle. `None` is
    /// the capture-disabled default: typed identities still
    /// correlate authorization, but no evidence records are emitted. When
    /// `Some`, those identities also address correlated sink records.
    pub evidence_sink: Option<Arc<dyn EvidenceSink>>,
    /// The complete mutable state at the start of this run: conversation
    /// context, canonical model selection, and inference configuration. The
    /// loop atomically replaces this with the final complete state.
    pub state: NextTurnState,
    /// Optional immutable system prompt (run binding).
    pub system: Option<String>,
    /// Steering queue (high-priority user messages injected before next turn).
    pub steering_queue: Option<Arc<Mutex<VecDeque<String>>>>,
    /// Follow-up queue (messages injected when agent would otherwise stop).
    pub follow_up_queue: Option<Arc<Mutex<VecDeque<String>>>>,
    /// Optional sink receiving diagnostics emitted from runtime failure paths
    /// (retry, cancellation, provider/tool failures). `None` disables emission
    /// without changing any other runtime behavior.
    pub diagnostic_sink: Option<Arc<dyn DiagnosticSink>>,
    /// Opaque session identifier, set by the harness from the active
    /// SessionCoordinator. Propagated into every Request so providers can
    /// emit session-affinity headers.
    pub session_id: Option<String>,
}

/// Immutable loop-control configuration. Per-request inference knobs (thinking,
/// max tokens, temperature) live in [`InferenceConfig`] on [`NextTurnState`] so
/// a preparation can change them atomically with context and model.
#[derive(Debug, Clone)]
pub struct AgentLoopConfig {
    /// Maximum number of turns before stopping.
    pub max_turns: u32,
    /// Retry configuration for retryable provider errors.
    pub retry: Option<opi_ai::retry::RetryConfig>,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            max_turns: 50,
            retry: None,
        }
    }
}
