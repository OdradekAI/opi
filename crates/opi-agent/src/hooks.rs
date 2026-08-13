//! Hook trait for agent loop customization (S8.2).

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
/// Phase 17.4 (AUT-006): this is a non-authoritative observation hook. It may
/// deny a tool call, but it is **not** an authority grant; the former `Allow`
/// variant is renamed `Continue` so no hook result can be mistaken for an
/// authorization decision. The authoritative decision is owned by the
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
    /// Replace the tool result entirely (field replacement, no deep merge).
    Replace(crate::tool::ToolResult),
}

/// Context provided to the should_stop_after_turn hook.
///
/// Phase 17.2: stop evaluation runs after the next-turn state has been
/// atomically applied, so this observes the applied complete state (not the
/// pre-preparation buffer).
#[derive(Clone)]
pub struct ShouldStopAfterTurnContext {
    pub state: NextTurnState,
    pub tool_results: Vec<opi_ai::message::ToolResultMessage>,
}

/// Context provided to the prepare_next_turn hook.
///
/// Phase 17.2: receives a snapshot of the current complete state plus the
/// completed turn outcome (tool results and any tool-driven terminate flag).
/// Returns a complete replacement state, `None` to retain the current state,
/// or an error/cancellation that leaves the prior state intact.
#[derive(Clone)]
pub struct PrepareNextTurnContext {
    pub state: NextTurnState,
    pub tool_results: Vec<opi_ai::message::ToolResultMessage>,
    pub terminate: bool,
    pub turn: u32,
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
    /// Phase 17.2: this runs before `should_stop_after_turn` observes state.
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
}
