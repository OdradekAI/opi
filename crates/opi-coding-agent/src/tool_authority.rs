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

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use opi_agent::Tool;
use opi_agent::authority::{
    AuthorizationDecision, AuthorizationError, Capability, RegisteredTool, RegistrationId,
    ToolAuthorizationRequest, ToolAuthorizer, ToolOrigin,
};
use opi_agent::evidence::CapabilityClass;
use opi_agent::extension::CollectedExtensionTool;
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use crate::config::{ExecutionConfig, ExecutionRunMode, PermissionDecision};
use crate::execution::permission::{
    InteractivePermissionBroker, PermissionManager, PermissionPolicy,
};
use crate::execution::router::{CandidateDecision, Eligibility, resolve_candidate};
use opi_tui::{PermissionChoice, PermissionSummary};

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

/// Convert extension-owned tool contributions into immutable registrations
/// without inferring trust from their provider-visible names. Reference
/// Product assembly may then apply its exact extension-capability permission
/// filter; Phase 17 defines no implicit permission, so the default product
/// projection excludes these registrations.
pub fn register_extension_tools(tools: Vec<CollectedExtensionTool>) -> Vec<RegisteredTool> {
    tools
        .into_iter()
        .map(|collected| {
            let (extension_id, tool) = collected.into_parts();
            let definition = tool.definition();
            let name = definition.name.clone();
            RegisteredTool::new(
                RegistrationId::new(format!("extension:{extension_id}:{name}")),
                name.clone(),
                ToolOrigin::Extension {
                    extension_id: extension_id.clone(),
                },
                Capability::Extension { extension_id, name },
                definition,
                Arc::from(tool),
            )
        })
        .collect()
}

/// Assemble the Reference Product's trusted registrations without laundering
/// extension names through the built-in capability table. Phase 17 defines no
/// product permission language for extension capabilities, so every extension
/// registration is intentionally excluded after its registry-owned origin has
/// been established.
pub fn register_product_tools(
    builtin_tools: Vec<Box<dyn Tool>>,
    extension_tools: Vec<CollectedExtensionTool>,
) -> Vec<RegisteredTool> {
    let builtin_registrations = register_builtin_tools(builtin_tools);
    let extension_registrations = register_extension_tools(extension_tools);
    debug_assert!(
        extension_registrations
            .iter()
            .all(|registration| matches!(registration.origin, ToolOrigin::Extension { .. }))
    );
    drop(extension_registrations);
    builtin_registrations
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
    mutating_allowed: bool,
    /// Phase 17.7: whether complete evidence is required. Closed mapping: absent
    /// capture is `false` (no-op Minimal Runtime); explicit capture (CLI
    /// `--trace`, SDK embedder, RPC recording) is `true`. Under
    /// required-complete-evidence, an incomplete health generation fails closed
    /// at authorization (P17-EVD-009).
    complete_evidence_required: bool,
    path_scope_digest: String,
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
            mutating_allowed,
            complete_evidence_required,
            path_scope_digest,
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

/// Product-owned command permission scope carried from the authorizer to the
/// `bash` implementation. It binds the final Allow to one reached adapter, the
/// immutable workspace/path scope, and the one supported operation without
/// containing command text or raw paths.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct CommandPermissionScope {
    version: u8,
    adapter_id: String,
    workspace_scope_digest: String,
    operation: String,
}

impl CommandPermissionScope {
    fn new(adapter_id: String, workspace_scope_digest: String) -> Self {
        Self {
            version: 1,
            adapter_id,
            workspace_scope_digest,
            operation: "execute".to_owned(),
        }
    }

    fn render(&self) -> String {
        serde_json::to_string(self).expect("command permission scope is serializable")
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        serde_json::from_str(value).ok()
    }

    pub(crate) fn adapter_id(&self) -> &str {
        &self.adapter_id
    }

    pub(crate) fn covers_workspace(&self, workspace_scope_digest: &str) -> bool {
        self.version == 1
            && self.operation == "execute"
            && self.workspace_scope_digest == workspace_scope_digest
    }
}

/// Trusted, immutable command-routing facts used by the product authorizer.
/// The same resolved config and eligibility catalog are supplied to execution,
/// so authorization binds the adapter selected from the final validated args.
#[derive(Clone)]
pub struct CommandAuthorizationContext {
    config: ExecutionConfig,
    mode: ExecutionRunMode,
    eligibility: Eligibility,
    manager: Option<Arc<PermissionManager>>,
    broker: Option<Arc<dyn InteractivePermissionBroker>>,
    workspace_scope_digest: String,
    package_names: BTreeMap<String, String>,
}

impl CommandAuthorizationContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: ExecutionConfig,
        mode: ExecutionRunMode,
        eligibility: Eligibility,
        manager: Option<Arc<PermissionManager>>,
        broker: Option<Arc<dyn InteractivePermissionBroker>>,
        workspace_scope_digest: String,
        package_names: BTreeMap<String, String>,
    ) -> Self {
        Self {
            config,
            mode,
            eligibility,
            manager,
            broker,
            workspace_scope_digest,
            package_names,
        }
    }

    async fn authorize(
        &self,
        arguments: &serde_json::Value,
        cancel: CancellationToken,
    ) -> Result<CommandAuthorization, AuthorizationError> {
        let model_backend = arguments.get("backend").and_then(serde_json::Value::as_str);
        let candidate =
            match resolve_candidate(&self.config, self.mode, &self.eligibility, model_backend) {
                CandidateDecision::Allowed(candidate) => {
                    return Ok(self.allow(candidate.backend, "policy"));
                }
                CandidateDecision::Ask(candidate) => candidate,
                CandidateDecision::Refused(failure) => {
                    return Ok(CommandAuthorization::Deny {
                        stable_code: failure.code().to_owned(),
                        redacted_reason: "execution permission denied before tool launch"
                            .to_owned(),
                    });
                }
            };

        if self.mode != ExecutionRunMode::Interactive {
            return Ok(CommandAuthorization::Deny {
                stable_code: "permission_required".to_owned(),
                redacted_reason: "interactive approval is unavailable in this run mode".to_owned(),
            });
        }
        if self
            .manager
            .as_deref()
            .is_some_and(|manager| manager.has_session_grant(&candidate.backend))
        {
            return Ok(self.allow(candidate.backend, "session"));
        }
        let Some(broker) = self.broker.as_ref() else {
            return Ok(CommandAuthorization::Deny {
                stable_code: "permission_required".to_owned(),
                redacted_reason: "interactive approval is required before tool execution"
                    .to_owned(),
            });
        };
        let summary = PermissionSummary {
            adapter_id: candidate.backend.clone(),
            package_name: self
                .package_names
                .get(&candidate.backend)
                .cloned()
                .unwrap_or_default(),
            run_mode_label: "interactive".to_owned(),
        };
        let choice = tokio::select! {
            _ = cancel.cancelled() => {
                return Err(AuthorizationError::Unavailable("authorization cancelled".to_owned()));
            }
            choice = broker.resolve_ask(summary) => choice,
        };
        match choice {
            PermissionChoice::AllowSession => {
                let Some(manager) = self.manager.as_deref() else {
                    return Err(AuthorizationError::Unavailable(
                        "session permission state is unavailable".to_owned(),
                    ));
                };
                manager.grant_session(&candidate.backend);
                Ok(self.allow(candidate.backend, "session"))
            }
            PermissionChoice::AllowOnce => Ok(self.allow(candidate.backend, "invocation")),
            PermissionChoice::Deny => Ok(CommandAuthorization::Deny {
                stable_code: "permission_denied".to_owned(),
                redacted_reason: "interactive approval was denied".to_owned(),
            }),
        }
    }

    fn allow(&self, adapter_id: String, source: &str) -> CommandAuthorization {
        let scope =
            CommandPermissionScope::new(adapter_id.clone(), self.workspace_scope_digest.clone());
        CommandAuthorization::Allow {
            permission_ref: format!("command.execute:adapter:{adapter_id}:{source}"),
            permission_scope: scope.render(),
        }
    }
}

enum CommandAuthorization {
    Allow {
        permission_ref: String,
        permission_scope: String,
    },
    Deny {
        stable_code: String,
        redacted_reason: String,
    },
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
    command: Option<CommandAuthorizationContext>,
}

impl ProductToolAuthorizer {
    /// Bind the authorizer to an immutable policy and the optional live
    /// session-grant manager (used to honor an `ask`-granted local adapter).
    pub fn new(
        policy: Arc<EffectiveUserPolicy>,
        command: Option<CommandAuthorizationContext>,
    ) -> Self {
        Self { policy, command }
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
        let command = self.command.clone();
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
            let permission = match &request.capability {
                Capability::Builtin(CapabilityClass::WorkspaceRead) => Some((
                    request.capability.as_identity(),
                    request.capability.as_identity(),
                )),
                Capability::Builtin(CapabilityClass::WorkspaceWrite) if policy.mutating_allowed => {
                    Some((
                        request.capability.as_identity(),
                        format!(
                            "workspace.write:workspace:{}:operation:mutate",
                            policy.path_scope_digest
                        ),
                    ))
                }
                Capability::Builtin(CapabilityClass::CommandExecute) => {
                    let Some(command) = command else {
                        return Ok(AuthorizationDecision::Deny {
                            stable_code: "permission_unavailable".to_owned(),
                            redacted_reason: "command authorization context is unavailable"
                                .to_owned(),
                        });
                    };
                    match command.authorize(&request.arguments, _cancel).await? {
                        CommandAuthorization::Allow {
                            permission_ref,
                            permission_scope,
                        } => Some((permission_ref, permission_scope)),
                        CommandAuthorization::Deny {
                            stable_code,
                            redacted_reason,
                        } => {
                            return Ok(AuthorizationDecision::Deny {
                                stable_code,
                                redacted_reason,
                            });
                        }
                    }
                }
                Capability::Builtin(CapabilityClass::WorkspaceWrite)
                | Capability::Extension { .. } => None,
                // Capability is non_exhaustive; a future capability class is
                // not permitted by the current product policy (fail-closed).
                _ => None,
            };
            if let Some((permission_ref, permission_scope)) = permission {
                Ok(AuthorizationDecision::Allow {
                    policy_ref: policy.digest().to_owned(),
                    permission_ref,
                    permission_scope,
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
