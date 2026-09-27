//! Hook trait for agent loop customization.

use std::future::Future;
use std::pin::Pin;

use tokio_util::sync::CancellationToken;

use crate::loop_types::{AgentError, NextTurnState};
use crate::message::AgentMessage;

/// Context provided before a tool call is executed.
pub struct BeforeToolCallContext {
    pub tool_call_id: String,
    pub tool_name: String,
    pub args: serde_json::Value,
    pub messages: Vec<AgentMessage>,
}

/// Result of the before_tool_call hook.
///
/// This is a non-authoritative observation hook. It may deny a tool call, but
/// it is **not** an authority grant; the former `Allow` variant is renamed
/// `Continue` so no hook result can be mistaken for an authorization decision.
/// The authoritative decision is owned by the
/// [`ToolAuthorizer`](crate::authority::ToolAuthorizer).
#[non_exhaustive]
pub enum BeforeToolCallResult {
    /// Continue to schema validation and trusted authorization (not a grant).
    Continue,
    /// Reject the tool call with an error message.
    Deny { reason: String },
}

/// Context provided after a tool call has been executed.
pub struct AfterToolCallContext {
    pub tool_call_id: String,
    pub tool_name: String,
    pub result: crate::tool::ToolResult,
}

/// Result of the after_tool_call hook.
#[non_exhaustive]
pub enum AfterToolCallResult {
    /// Keep the original tool result unchanged.
    Keep,
    /// Replace the emitted and persisted presentation result entirely (field
    /// replacement, no deep merge).
    ///
    /// Tool evidence retains the original lower-boundary execution outcome;
    /// replacement cannot rewrite what actually executed.
    Replace(crate::tool::ToolResult),
}

/// Context provided to the should_stop_after_turn hook.
///
/// Stop evaluation observes the completed turn before queue polling or
/// next-turn preparation. Returning true ends the run without preparing.
#[derive(Clone)]
pub struct ShouldStopAfterTurnContext {
    pub state: NextTurnState,
    pub tool_results: Vec<opi_ai::message::ToolResultMessage>,
}

/// Context provided to the prepare_next_turn hook.
///
/// Receives a snapshot of the current complete state plus the completed turn
/// outcome (tool results and any tool-driven terminate flag). Returns a
/// complete replacement state, `None` to retain the current state, or an
/// error/cancellation that leaves the prior state intact.
#[derive(Clone)]
pub struct PrepareNextTurnContext {
    pub state: NextTurnState,
    pub tool_results: Vec<opi_ai::message::ToolResultMessage>,
    pub terminate: bool,
    /// Number of completed turns in this loop invocation, starting at one.
    /// Resets for each invocation; this is not a budget or a session total.
    pub turn: u64,
}

/// Complete initial state, including input selected for the first dispatch.
#[derive(Clone)]
pub struct InitialTurnContext {
    pub state: NextTurnState,
}

/// External persistence reported by a settled preparation commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparationCommit {
    /// No external state changed; ordinary run rollback remains unchanged.
    NoExternalCommit,
    /// The validated candidate was committed to external durable state.
    ExternalStateCommitted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparationCompactionIdentity {
    pub run: crate::evidence::RunId,
    pub turn: crate::evidence::TurnId,
    pub call: crate::evidence::CallId,
    pub trigger: crate::evidence::CompactionTrigger,
}

/// Opaque start token retained across preparation and its commit callback.
#[must_use = "a started compaction must receive a terminal outcome"]
#[derive(Debug)]
pub struct PendingPreparationCompaction {
    pub(crate) identity: PreparationCompactionIdentity,
}

/// Borrowed access to the active run's preparation evidence lifecycle.
///
/// The allocator and health remain owned by Core. Operations use the same run
/// and turn as the preparation and cannot outlive its awaited callback.
pub struct PreparationEvidence<'a> {
    pub(crate) events: &'a crate::event::AgentEventSink,
    pub(crate) sink: &'a Option<std::sync::Arc<dyn crate::evidence::EvidenceSink>>,
    pub(crate) identities: &'a mut crate::evidence::IdentityAllocator,
    pub(crate) health: &'a mut crate::evidence::EvidenceHealth,
    pub(crate) turn: crate::evidence::TurnId,
    pub(crate) pending: &'a mut Vec<PreparationCompactionIdentity>,
    pub(crate) uncertain_outcome: &'a mut Option<crate::evidence::TerminalOutcome>,
}

/// Hook trait for customizing agent loop behavior.
///
/// Default implementations are no-ops or basic conversions.
pub trait AgentHooks: Send + Sync {
    /// Convert agent messages to provider-facing messages.
    fn convert_to_llm(
        &self,
        messages: &[AgentMessage],
    ) -> Result<Vec<opi_ai::message::Message>, AgentError>;

    /// Transform messages before conversion to LLM format.
    fn transform_context(
        &self,
        messages: Vec<AgentMessage>,
        _signal: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<AgentMessage>, AgentError>> + Send>> {
        Box::pin(async { Ok(messages) })
    }

    /// Decide whether the loop should stop after this turn.
    fn should_stop_after_turn(
        &self,
        _ctx: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        Box::pin(async { false })
    }

    /// Hook called before a tool is executed.
    fn before_tool_call(
        &self,
        _ctx: BeforeToolCallContext,
    ) -> Pin<Box<dyn Future<Output = BeforeToolCallResult> + Send>> {
        Box::pin(async { BeforeToolCallResult::Continue })
    }

    /// Hook called after a tool has been executed.
    fn after_tool_call(
        &self,
        _ctx: AfterToolCallContext,
    ) -> Pin<Box<dyn Future<Output = AfterToolCallResult> + Send>> {
        Box::pin(async { AfterToolCallResult::Keep })
    }

    /// Prepare the complete next-turn state before it is atomically applied.
    ///
    /// Receives a snapshot of the current complete state plus the completed
    /// turn outcome. Return `Ok(Some(replacement))` to replace the state with a
    /// validated complete value, `Ok(None)` to retain the current state, or
    /// `Err(_)` to leave the prior state intact and terminate the transition.
    /// Runs only after the completed-turn stop decision permits continuation
    /// and tool work or queued input warrants another provider call. Returning
    /// a candidate cannot itself create a turn. Queued input is appended after
    /// the replacement applies; on failure it remains available for a later run.
    fn prepare_next_turn(
        &self,
        _ctx: PrepareNextTurnContext,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Option<crate::loop_types::NextTurnState>, AgentError>>
                + Send,
        >,
    > {
        Box::pin(async { Ok(None) })
    }

    /// Prepare the initial state before transformation and provider dispatch.
    /// Selected queue messages must remain an unchanged suffix of a replacement.
    fn prepare_initial_turn<'a>(
        &'a self,
        _ctx: InitialTurnContext,
        _cancel: CancellationToken,
        _evidence: PreparationEvidence<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send + 'a>> {
        Box::pin(async { Ok(None) })
    }

    /// Prepare an already-warranted next turn with active run evidence.
    /// The default preserves the existing preparation hook contract.
    fn prepare_next_turn_transaction<'a>(
        &'a self,
        ctx: PrepareNextTurnContext,
        _cancel: CancellationToken,
        _evidence: PreparationEvidence<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<NextTurnState>, AgentError>> + Send + 'a>> {
        self.prepare_next_turn(ctx)
    }

    /// Validate product-owned constraints after intrinsic Core validation.
    /// No external state may be committed here.
    fn validate_prepared_state(&self, _candidate: &NextTurnState) -> Result<(), AgentError> {
        Ok(())
    }

    /// Whether this hook owns external persistence for preparation commits.
    /// Wrappers forward this fact; independent commit owners must reject an
    /// overlapping transaction during validation, before either owner writes.
    fn owns_preparation_commit(&self) -> bool {
        false
    }

    /// Settle external persistence for a fully validated candidate.
    ///
    /// Core does not drop this future when cancellation arrives. An `Ok` result
    /// always installs the candidate before cancellation is observed; reporting
    /// an external commit retains that baseline on later ordinary failure.
    /// Once durable persistence completes, return `ExternalStateCommitted` even
    /// if subsequent evidence emission fails; Core installs the candidate and
    /// withholds dispatch on incomplete evidence. `Err` reports no completed
    /// external commit, with any partial effect recorded separately in evidence.
    fn commit_prepared_state<'a>(
        &'a self,
        _candidate: NextTurnState,
        _cancel: CancellationToken,
        _evidence: PreparationEvidence<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<PreparationCommit, AgentError>> + Send + 'a>> {
        Box::pin(async { Ok(PreparationCommit::NoExternalCommit) })
    }
}
