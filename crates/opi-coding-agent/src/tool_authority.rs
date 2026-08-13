//! Phase 17.4 Reference Product trusted-tool authorization.
//!
//! Owns the immutable digest-addressed [`EffectiveUserPolicy`], the fixed
//! built-in capability map, and the [`ProductToolAuthorizer`] that reuses the
//! existing `command.execute` permission policy. Built-in Reference Product
//! tools enter the Agent as [`RegisteredTool`]s with their fixed capability;
//! extension/embedder tools without an exact existing capability permission are
//! **excluded** (fail-closed — Phase 17.4 creates no implicit allow rule and no
//! new permission language).
//!
//! Boundary: the [`ProductToolAuthorizer`] decision derives ONLY from the
//! immutable policy + the capability + the current evidence-health snapshot
//! (AUT-003/004). Model content (`arguments`) is never consulted for the
//! permission decision. Full arg-driven adapter binding for `command.execute`
//! stays in the routed bash backend; the authorizer adds the fail-closed
//! immutable-policy gate in front of it.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use opi_agent::Tool;
use opi_agent::authority::{
    AuthorizationDecision, AuthorizationError, Capability, RegisteredTool, RegistrationId,
    ToolAuthorizationRequest, ToolAuthorizer, ToolOrigin,
};
use opi_agent::evidence::CapabilityClass;
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use crate::config::{ExecutionRunMode, PermissionDecision};
use crate::execution::permission::{LOCAL_ADAPTER_ID, PermissionManager, PermissionPolicy};

/// The fixed built-in capability map (spec lines 419-423). Returns `None` for
/// names that are not built-in Reference Product tools.
pub fn builtin_capability(name: &str) -> Option<Capability> {
    match name {
        "read" | "grep" | "find" | "ls" | "glob" => {
            Some(Capability::Builtin(CapabilityClass::WorkspaceRead))
        }
        "write" | "edit" => Some(Capability::Builtin(CapabilityClass::WorkspaceWrite)),
        "bash" => Some(Capability::Builtin(CapabilityClass::CommandExecute)),
        _ => None,
    }
}

/// Register the built-in Reference Product tools as trusted registrations with
/// their fixed capability and a `Builtin` origin. Non-built-in tools (extension
/// or embedder tools without an exact existing capability permission) are
/// dropped: Phase 17.4 excludes them rather than implicitly allowing them.
pub fn register_builtin_tools(tools: Vec<Box<dyn Tool>>) -> Vec<RegisteredTool> {
    tools
        .into_iter()
        .filter_map(|tool| {
            let name = tool.definition().name.clone();
            let capability = builtin_capability(&name)?;
            Some(RegisteredTool::new(
                RegistrationId::new(format!("builtin:{name}")),
                name,
                ToolOrigin::Builtin,
                capability,
                tool.definition(),
                Arc::from(tool),
            ))
        })
        .collect()
}

fn decision_str(decision: PermissionDecision) -> &'static str {
    match decision {
        PermissionDecision::Allow => "allow",
        PermissionDecision::Ask => "ask",
        PermissionDecision::Deny => "deny",
    }
}

/// sha256 hex digest of an arbitrary stable input, for digest-addressing policy
/// facts whose own types are not canonical-serialization types.
pub fn digest_of(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

/// Immutable, digest-addressed effective user policy for the run (spec lines
/// 433-441). Assembled from facts the product already resolves: run mode,
/// active-tool selection, mutating opt-in, `command.execute` adapter permission
/// rules, whether complete evidence is required, project trust, and package
/// artifact/trust/activation state. Live session-scoped grants
/// ([`PermissionManager`]) are separately versioned and do NOT mutate this
/// digest (spec lines 437-439).
#[derive(Debug, Clone)]
pub struct EffectiveUserPolicy {
    run_mode: ExecutionRunMode,
    mutating_allowed: bool,
    command_execute_permission: PermissionPolicy,
    /// Phase 17.7: whether complete evidence is required. Closed mapping: absent
    /// capture is `false` (no-op Minimal Runtime); explicit capture (CLI
    /// `--trace`, SDK embedder, RPC recording) is `true`. Under
    /// required-complete-evidence, an incomplete health generation fails closed
    /// at authorization (P17-EVD-009).
    complete_evidence_required: bool,
    digest: String,
}

impl EffectiveUserPolicy {
    /// Assemble the immutable policy snapshot and compute its digest. The digest
    /// is a stable sha256 over a canonical rendering of the snapshotted facts
    /// (sorted tool names + sorted adapter decisions); a grant or run-state
    /// change that does not touch these facts leaves the digest unchanged.
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        run_mode: ExecutionRunMode,
        mut active_tool_names: Vec<String>,
        mutating_allowed: bool,
        command_execute_permission: PermissionPolicy,
        complete_evidence_required: bool,
        project_trust_digest: impl Into<String>,
        package_activation_digest: impl Into<String>,
        path_scope_digest: impl Into<String>,
    ) -> Self {
        active_tool_names.sort();
        let project_trust_digest = project_trust_digest.into();
        let package_activation_digest = package_activation_digest.into();
        let path_scope_digest = path_scope_digest.into();
        let digest = {
            let mut hasher = Sha256::new();
            hasher.update(run_mode_label(run_mode).as_bytes());
            for name in &active_tool_names {
                hasher.update(b"\nactive:");
                hasher.update(name.as_bytes());
            }
            hasher.update(b"\nmutating:");
            hasher.update(if mutating_allowed { b"1" } else { b"0" });
            for (id, decision) in command_execute_permission.canonical_entries() {
                hasher.update(b"\nperm:");
                hasher.update(id.as_bytes());
                hasher.update(b"=");
                hasher.update(decision_str(decision).as_bytes());
            }
            hasher.update(b"\nevidence:");
            hasher.update(if complete_evidence_required {
                b"1"
            } else {
                b"0"
            });
            hasher.update(b"\nproject:");
            hasher.update(project_trust_digest.as_bytes());
            hasher.update(b"\npackage:");
            hasher.update(package_activation_digest.as_bytes());
            hasher.update(b"\npath-scope:");
            hasher.update(path_scope_digest.as_bytes());
            hex::encode(hasher.finalize())
        };
        // The digest-only facts (active-tool selection, project trust, package
        // activation) are folded into `digest` above and not stored separately.
        // `complete_evidence_required` is also folded into the digest but IS
        // stored, because the authorization fail-closed rule (P17-EVD-009) reads
        // it at decision time.
        let _ = active_tool_names;
        Self {
            run_mode,
            mutating_allowed,
            command_execute_permission,
            complete_evidence_required,
            digest,
        }
    }

    /// The content-addressed digest of the immutable policy.
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// Whether complete evidence is required for the run (Phase 17.7).
    pub fn complete_evidence_required(&self) -> bool {
        self.complete_evidence_required
    }
}

fn run_mode_label(mode: ExecutionRunMode) -> &'static str {
    match mode {
        ExecutionRunMode::Interactive => "interactive",
        ExecutionRunMode::NonInteractive => "non-interactive",
        ExecutionRunMode::Rpc => "rpc",
    }
}

/// Product-owned trusted authorizer bound to an [`EffectiveUserPolicy`] for the
/// run. Reuses the existing `command.execute` permission policy for the local
/// adapter; never derives permission from model content.
pub struct ProductToolAuthorizer {
    policy: Arc<EffectiveUserPolicy>,
    permission_manager: Option<Arc<PermissionManager>>,
}

impl ProductToolAuthorizer {
    /// Bind the authorizer to an immutable policy and the optional live
    /// session-grant manager (used to honor an `ask`-granted local adapter).
    pub fn new(
        policy: Arc<EffectiveUserPolicy>,
        permission_manager: Option<Arc<PermissionManager>>,
    ) -> Self {
        Self {
            policy,
            permission_manager,
        }
    }
}

impl ToolAuthorizer for ProductToolAuthorizer {
    fn authorize(
        &self,
        request: ToolAuthorizationRequest,
        _cancel: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<AuthorizationDecision, AuthorizationError>> + Send>>
    {
        let policy = self.policy.clone();
        let manager = self.permission_manager.clone();
        Box::pin(async move {
            // P17-EVD-009: under required-complete-evidence, an incomplete health
            // generation fails closed. Unlaunched side effects are denied here;
            // in-flight effects already crossed the launch boundary and retain
            // their actual outcome (the loop's parallel launch boundary).
            if policy.complete_evidence_required() && !request.evidence_health.is_healthy() {
                return Ok(AuthorizationDecision::Deny {
                    stable_code: "evidence_incomplete".to_owned(),
                    redacted_reason: "complete evidence is required and evidence is incomplete"
                        .to_owned(),
                });
            }
            // AUT-003/004: the decision derives only from the immutable policy +
            // the capability + the current health snapshot. The validated
            // `request.arguments` are intentionally NOT consulted for permission.
            let allowed = match &request.capability {
                Capability::Builtin(CapabilityClass::WorkspaceRead) => true,
                Capability::Builtin(CapabilityClass::WorkspaceWrite) => policy.mutating_allowed,
                Capability::Builtin(CapabilityClass::CommandExecute) => {
                    command_execute_allowed(&policy, manager.as_deref())
                }
                Capability::Extension { .. } => false,
                // Capability is non_exhaustive; a future capability class is
                // not permitted by the current product policy (fail-closed).
                _ => false,
            };
            if allowed {
                Ok(AuthorizationDecision::Allow {
                    policy_ref: policy.digest().to_owned(),
                    permission_ref: request.capability.as_identity(),
                    permission_scope: request.capability.as_identity(),
                    registration_id: request.registration_id.clone(),
                    capability: request.capability.clone(),
                    evidence_health_generation: request.evidence_health.generation(),
                })
            } else {
                Ok(AuthorizationDecision::Deny {
                    stable_code: "policy_denied".to_owned(),
                    redacted_reason: "effective user policy denied the capability".to_owned(),
                })
            }
        })
    }
}

fn command_execute_allowed(
    policy: &EffectiveUserPolicy,
    manager: Option<&PermissionManager>,
) -> bool {
    // Reuse the existing command.execute permission policy for the local
    // adapter: Allow -> proceed; Deny -> fail closed; Ask -> a live session
    // grant covers it, otherwise interactive runs defer the prompt to the routed
    // bash backend (Tool::execute) and headless runs fail closed.
    match policy
        .command_execute_permission
        .decision_for(LOCAL_ADAPTER_ID)
    {
        PermissionDecision::Allow => true,
        PermissionDecision::Deny => false,
        PermissionDecision::Ask => {
            if manager.is_some_and(|m| m.has_session_grant(LOCAL_ADAPTER_ID)) {
                true
            } else {
                matches!(policy.run_mode, ExecutionRunMode::Interactive)
            }
        }
    }
}
