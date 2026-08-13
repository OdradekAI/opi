//! Types for the agent loop (S6.1, S8.2, Phase 17.2).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use opi_ai::ProviderCollection;
use opi_ai::provider::ThinkingConfig;

use crate::authority::{ToolAuthorizer, ToolRegistry};
use crate::diagnostic_sink::DiagnosticSink;
use crate::evidence::{EvidenceHealth, EvidenceSink};
use crate::message::AgentMessage;

/// Errors that can occur during the agent loop.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("authentication failed: {0}")]
    AuthFailed(String),
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
    #[error("tool error: {0}")]
    Tool(String),
    #[error("hook error: {0}")]
    Hook(String),
    #[error("cancelled")]
    Cancelled,
    #[error("max turns exceeded ({0})")]
    MaxTurnsExceeded(u32),
    /// Evidence capture setup failed before the run (fail-closed, Phase 17.7):
    /// the configured evidence sink could not be prepared. The run is aborted
    /// before its first provider/tool call so it never runs with incomplete
    /// evidence when capture was explicitly requested (P17-EVD-007).
    #[error("evidence setup failed: {0}")]
    EvidenceSetup(String),
    /// A provider route could not be prepared for a model call: the selection
    /// was unknown, ambiguous, undispatchable, or its authentication could not
    /// be resolved. Phase 17.2 surfaces collection-owned preparation failures
    /// at this typed boundary rather than as a generic provider string.
    #[error("provider route not dispatchable for '{provider}': {detail}")]
    RouteNotDispatchable {
        /// Provider id (or requested selection) whose route failed.
        provider: String,
        /// Redacted, non-secret reason the route could not be prepared.
        detail: String,
    },
    /// A `prepare_next_turn` candidate was rejected because it was not a valid
    /// complete state (e.g. its model selection does not resolve to a route in
    /// the provider collection). The prior state is preserved unchanged.
    #[error("invalid next-turn candidate state: {0}")]
    InvalidNextTurnCandidate(String),
    /// A trusted tool registration was rejected (e.g. two registrations share a
    /// provider-visible name). Phase 17.4.
    #[error("invalid tool registration: {0}")]
    InvalidToolRegistration(String),
}

/// Canonical provider:model selection for one logical model call.
///
/// Phase 17 does not add an alias registry; the selection is always the
/// canonical `provider_id:model_id` pair resolved by the provider collection.
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
    /// Product (Phase 17.5), not by Agent Core state.
    pub fn parse_spec(spec: &str) -> Option<Self> {
        let (provider_id, model_id) = spec.split_once(':')?;
        let provider_id = provider_id.trim();
        let model_id = model_id.trim();
        if provider_id.is_empty() || model_id.is_empty() {
            return None;
        }
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

/// The complete mutable next-request state owned durably by the Agent (Phase
/// 17.2). A loop run returns its final complete state to the Agent, which
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

/// Input context for the agent loop.
pub struct AgentLoopContext {
    /// The dispatchable provider collection that owns route lookup, per-call
    /// auth preparation, and attempt dispatch (Phase 17.2). The loop prepares
    /// one call per turn and opens every retry attempt from that same prepared
    /// call.
    pub collection: Arc<ProviderCollection>,
    /// Immutable trusted-tool registry (Phase 17.4). The loop resolves every
    /// model-proposed tool name against this registry, projects provider-facing
    /// definitions from it, and executes only implementations reachable through
    /// a current authorization `Allow`.
    pub registry: Arc<ToolRegistry>,
    /// Mandatory trusted tool authorizer bound to the effective User Policy for
    /// the run. `None` is fail-closed: no tool executes without a current
    /// `Allow` (AUT-005).
    pub authorizer: Option<Arc<dyn ToolAuthorizer>>,
    /// Current versioned evidence-health snapshot for the run (Phase 17.4). The
    /// loop injects a copy into each authorization request and verifies an
    /// `Allow`'s generation still matches before execution. In 17.4 this is a
    /// run-start snapshot; evidence-failure-driven advancement and live reads
    /// arrive in 17.6/17.7.
    pub evidence_health: EvidenceHealth,
    /// Optional evidence sink binding the run's call-graph lifecycle (Phase
    /// 17.6). `None` is the capture-disabled no-op default: no identities are
    /// minted and no records are emitted, so execution behavior is unchanged.
    /// When `Some`, the loop allocates stable run/turn/call identities before
    /// emitting correlated records through this sink.
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
