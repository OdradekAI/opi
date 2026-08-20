//! Tool calling abstraction (S8.2).

pub mod result;

use std::future::Future;
use std::pin::Pin;

use opi_ai::message::{OutputContent, ToolDef};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::evidence::{
    PermissionReference, PermissionScope, PolicyReference, ScopedGrantReference,
};

/// Callback for progress updates during tool execution.
pub type UpdateCallback = Box<dyn Fn(serde_json::Value) + Send + Sync>;

/// Verified opaque authorization facts forwarded from the trusted authorizer
/// to the selected tool implementation. Tools that do not need product-owned
/// permission binding may use the default [`Tool::execute_authorized`]
/// implementation, which delegates to [`Tool::execute`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolExecutionAuthorization {
    /// Opaque effective-policy reference.
    pub policy_ref: PolicyReference,
    /// Opaque product-owned permission reference.
    pub permission_ref: PermissionReference,
    /// Opaque product-owned permission scope.
    pub permission_scope: PermissionScope,
    /// Separately versioned scoped grant used by this authorization, if any.
    pub scoped_grant_ref: Option<ScopedGrantReference>,
}

/// Tool trait — each concrete tool implements this.
pub trait Tool: Send + Sync {
    /// Return the tool's definition (name, description, JSON Schema for input).
    fn definition(&self) -> ToolDef;

    /// Execute the tool with validated arguments.
    fn execute(
        &self,
        call_id: &str,
        arguments: serde_json::Value,
        signal: CancellationToken,
        on_update: Option<UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>>;

    /// Execute with the exact verified authorization that crossed the Agent's
    /// freshness gate. Product tools may override this to bind dispatch to the
    /// reached adapter or another product-owned scope; the default preserves
    /// the ordinary tool contract.
    fn execute_authorized(
        &self,
        call_id: &str,
        arguments: serde_json::Value,
        _authorization: ToolExecutionAuthorization,
        signal: CancellationToken,
        on_update: Option<UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        self.execute(call_id, arguments, signal, on_update)
    }

    /// Whether this tool must run sequentially.
    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Parallel
    }
}

/// Owned, lightweight diagnostic entry carried on a [`ToolResult`].
///
/// Deliberately not coupled to [`crate::diagnostic`] so `tool.rs` keeps its
/// zero-internal-dependency layering. The agent-loop boundary lifts each entry
/// into a [`Diagnostic`](crate::diagnostic::Diagnostic). The entry also serializes into the
/// agent-facing `AgentEvent::ToolExecutionEnd.diagnostics` wire field (NDJSON /
/// RPC) so headless consumers see per-cause context. It is deliberately NOT
/// added to the provider-facing `ToolResultMessage`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDiagnostic {
    /// Stable snake_case code (forward-compatible with `CODE_TOOL_*` constants).
    pub code: String,
    /// Human-readable cause description.
    pub message: String,
    /// Structured per-cause payload; becomes `Diagnostic::details` at the
    /// agent-loop boundary.
    pub context: serde_json::Value,
}

/// Result of a tool execution.
#[derive(Clone)]
pub struct ToolResult {
    pub content: Vec<OutputContent>,
    pub details: Option<serde_json::Value>,
    pub is_error: bool,
    pub terminate: bool,
    /// Whether `content` was truncated (large file, capped output, partial walk).
    pub truncated: bool,
    /// Tool-owned structured failure context; lifted into diagnostics by the
    /// agent loop.
    pub diagnostics: Vec<ToolDiagnostic>,
}

impl ToolResult {
    /// Create an error tool result from a validation error.
    pub fn from_validation_error(err: crate::validation::ValidationError) -> Self {
        let message = err.to_string();
        Self {
            content: vec![OutputContent::Text { text: message }],
            details: None,
            is_error: true,
            terminate: false,
            truncated: false,
            diagnostics: Vec::new(),
        }
    }
}

/// Errors from tool execution.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
    #[error("cancelled")]
    Cancelled,
    /// The tool failed after an external side effect may have occurred.
    #[error("partial side effect: {0}")]
    PartialSideEffect(String),
    /// The tool could not confirm cleanup of an external side effect.
    #[error("cleanup unknown: {0}")]
    CleanupUnknown(String),
}

/// Whether a tool runs sequentially or in parallel with others.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    Sequential,
    Parallel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_result_carries_truncated_and_diagnostics() {
        let result = ToolResult {
            content: Vec::new(),
            details: None,
            is_error: false,
            terminate: false,
            truncated: true,
            diagnostics: vec![ToolDiagnostic {
                code: "test_code".to_string(),
                message: "test message".to_string(),
                context: serde_json::json!({ "k": "v" }),
            }],
        };
        assert!(result.truncated);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code, "test_code");
        assert_eq!(result.diagnostics[0].message, "test message");
    }
}
