//! L0 process-tree supervision diagnostic contract.
//!
//! After Phase 16 task 16.16.1 removed the built-in native sandbox from core,
//! only the policy-neutral L0 process-tree supervision diagnostic remains. The
//! stable `&'static str` code identifies a process-tree supervision degradation
//! (an attach/terminate failure during L0 supervision) so embedders and tests
//! can match it by literal. The shared [`opi_agent::Diagnostic`] model is
//! constructed by value here; the opi-agent struct itself is unchanged. The
//! diagnostic carries a redacted `{ layer, reason }` payload in
//! `Diagnostic.details`: `layer` names the supervision layer (e.g.
//! `"unix-pgroup"`, `"windows-job"`) and `reason` is a short, curator-controlled
//! explanation. No command, environment variable, absolute path, or credential
//! is ever placed in the payload.

use opi_agent::diagnostic::{Diagnostic, Severity};

/// Closed, redaction-safe L0 process-tree supervision reason.
///
/// Every variant serializes to curator-controlled static text. Raw OS,
/// subprocess, command, environment, credential, and path data cannot cross the
/// public diagnostic-construction boundary.
///
/// (Phase 16 task 16.16.1 pruned the strict-sandbox variants when native
/// restriction left core; this enum now carries only the L0 supervision reasons
/// used by `tool::process_tree` and `tool::supervision`. The name is retained to
/// avoid churning the retained L0 `AttachError.reason` call sites; it is
/// L0-only vocabulary now.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxReason {
    MissingChildProcessId,
    ProcessTreeAttachFailed,
    ProcessTreeTerminationFailed,
}

impl SandboxReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingChildProcessId => "missing child process id",
            Self::ProcessTreeAttachFailed => "process-tree containment attach failed",
            Self::ProcessTreeTerminationFailed => "process-tree containment termination failed",
        }
    }
}

impl std::fmt::Display for SandboxReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Stable code for an L0 process-tree supervision degradation (an attach or
/// terminate failure during supervision). Supervision degradations are reported,
/// not fatal: the command still runs under the best-effort L0 baseline.
pub const CODE_PROCESS_TREE_DEGRADED: &str = "opi.process-tree.degraded";

/// Owning subsystem for L0 process-tree supervision diagnostics.
pub const SOURCE_PROCESS_TREE: &str = "process-tree";

/// Construct a redacted L0 process-tree-supervision-degraded diagnostic.
///
/// `layer` names the supervision layer (e.g. `"unix-pgroup"`); `reason` is a
/// short curator-controlled explanation. The payload is restricted to
/// `{ layer, reason }` and never carries command text, env vars, paths, or
/// secrets.
pub fn process_tree_degraded_diagnostic(layer: &'static str, reason: SandboxReason) -> Diagnostic {
    Diagnostic::new(
        Severity::Warning,
        CODE_PROCESS_TREE_DEGRADED,
        SOURCE_PROCESS_TREE,
        "process-tree supervision degraded",
    )
    .details(serde_json::json!({
        "layer": layer,
        "reason": reason.as_str(),
    }))
}
