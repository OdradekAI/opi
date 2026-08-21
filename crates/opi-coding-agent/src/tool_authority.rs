//! Reference Product trusted-tool authorization.
//!
//! Owns the immutable digest-addressed [`EffectiveUserPolicy`], the fixed
//! built-in capability map, and the [`ProductToolAuthorizer`] that reuses the
//! existing `command.execute` permission policy. Built-in Reference Product
//! tools enter the Agent as [`RegisteredTool`]s with their fixed capability;
//! extension/embedder tools without an exact existing capability permission are
//! **excluded**: the product creates no implicit allow rule or new permission
//! language.
//!
//! Boundary: the [`ProductToolAuthorizer`] decision derives ONLY from the
//! immutable policy + the capability + the current evidence-health snapshot
//! (AUT-003/004). For `command.execute`, the validated arguments select the
//! adapter binding inside this trusted boundary; no permission fact derives
//! from argument content. The routed bash backend still owns arg-driven
//! adapter execution behind the authorizer's fail-closed immutable-policy
//! gate.

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use opi_agent::Tool;
use opi_agent::authority::{
    AuthorizationDecision, AuthorizationError, CapabilityIdentity, RegisteredTool, RegistrationId,
    ToolAuthorizationRequest, ToolAuthorizer, ToolOrigin,
};
use opi_agent::evidence::{
    PermissionReference, PermissionScope, PolicyReference, ScopedGrantReference,
};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use crate::config::{ExecutionConfig, ExecutionRunMode, PermissionDecision};
use crate::execution::permission::{
    InteractivePermissionBroker, PermissionManager, PermissionPolicy,
};
use crate::execution::router::{CandidateDecision, Eligibility, resolve_candidate};
use opi_tui::{PermissionChoice, PermissionSummary};

/// Product-owned workspace-read capability.
pub static WORKSPACE_READ_CAPABILITY: std::sync::LazyLock<CapabilityIdentity> =
    std::sync::LazyLock::new(|| {
        CapabilityIdentity::new("opi.workspace.read").expect("valid product capability")
    });
/// Product-owned workspace-write capability.
pub static WORKSPACE_WRITE_CAPABILITY: std::sync::LazyLock<CapabilityIdentity> =
    std::sync::LazyLock::new(|| {
        CapabilityIdentity::new("opi.workspace.write").expect("valid product capability")
    });
/// Product-owned command-execution capability.
pub static COMMAND_EXECUTE_CAPABILITY: std::sync::LazyLock<CapabilityIdentity> =
    std::sync::LazyLock::new(|| {
        CapabilityIdentity::new("opi.command.execute").expect("valid product capability")
    });

/// The fixed built-in capability map for Reference Product tools (the
/// trusted-tool-registration capability table). Returns `None` for
/// names that are not built-in Reference Product tools.
pub fn builtin_capability(name: &str) -> Option<CapabilityIdentity> {
    match name {
        "read" | "grep" | "find" | "ls" | "glob" => Some(WORKSPACE_READ_CAPABILITY.clone()),
        "write" | "edit" => Some(WORKSPACE_WRITE_CAPABILITY.clone()),
        "bash" => Some(COMMAND_EXECUTE_CAPABILITY.clone()),
        _ => None,
    }
}

/// Register the built-in Reference Product tools as trusted registrations with
/// their fixed capability and a `Builtin` origin. Non-built-in tools (extension
/// or embedder tools without an exact existing capability permission) are
/// dropped rather than implicitly allowed.
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

/// Assemble the Reference Product's trusted registrations without laundering
/// extension names through the built-in capability table. The product defines
/// no permission language for extension capabilities, so extension tool
/// contributions are excluded before registration and never materialize a
/// registration that the product would immediately discard.
pub fn register_product_tools(builtin_tools: Vec<Box<dyn Tool>>) -> Vec<RegisteredTool> {
    register_builtin_tools(builtin_tools)
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

/// Immutable, digest-addressed effective user policy for the run (the
/// effective-product-policy contract). Assembled from facts the product
/// already resolves: run mode, active-tool selection, mutating opt-in,
/// `command.execute` adapter permission rules, whether complete evidence is
/// required, project trust, and package artifact/trust/activation state. Live
/// session-scoped grants ([`PermissionManager`]) are separately versioned and
/// do NOT mutate this digest.
#[derive(Debug, Clone)]
pub struct EffectiveUserPolicy {
    mutating_allowed: bool,
    /// Whether complete evidence is required. Closed mapping: absent
    /// capture is `false` (no-op Minimal Runtime); explicit capture (CLI
    /// `--trace`, SDK embedder, RPC recording) is `true`. Under
    /// required-complete-evidence, an incomplete health generation fails closed
    /// at authorization.
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
        // stored, because the authorization fail-closed rule reads
        // it at decision time.
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

    /// Whether complete evidence is required for the run.
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
            permission_ref: PermissionReference::new(format!(
                "command.execute:adapter:{adapter_id}:{source}"
            ))
            .expect("product permission reference is valid"),
            permission_scope: PermissionScope::new(scope.render())
                .expect("serialized product scope is valid"),
            scoped_grant_ref: (source == "session").then(|| {
                ScopedGrantReference::new(format!("command.execute:adapter:{adapter_id}:session"))
                    .expect("product scoped grant reference is valid")
            }),
        }
    }
}

enum CommandAuthorization {
    Allow {
        permission_ref: PermissionReference,
        permission_scope: PermissionScope,
        scoped_grant_ref: Option<ScopedGrantReference>,
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
            // Under required-complete-evidence, an incomplete health
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
            // the capability + the current health snapshot. The `command.execute`
            // arm below passes the validated `request.arguments` to the trusted
            // command context so the adapter binding matches the arguments the
            // tool will run with; no permission fact derives from argument
            // content.
            let permission = if request.capability == *WORKSPACE_READ_CAPABILITY {
                Some((
                    PermissionReference::new("opi.workspace.read")
                        .expect("valid product permission"),
                    PermissionScope::new("workspace:read").expect("valid product scope"),
                    None,
                ))
            } else if request.capability == *WORKSPACE_WRITE_CAPABILITY && policy.mutating_allowed {
                Some((
                    PermissionReference::new("opi.workspace.write")
                        .expect("valid product permission"),
                    PermissionScope::new(format!(
                        "workspace.write:workspace:{}:operation:mutate",
                        policy.path_scope_digest
                    ))
                    .expect("valid product scope"),
                    None,
                ))
            } else if request.capability == *COMMAND_EXECUTE_CAPABILITY {
                let Some(command) = command else {
                    return Ok(AuthorizationDecision::Deny {
                        stable_code: "permission_unavailable".to_owned(),
                        redacted_reason: "command authorization context is unavailable".to_owned(),
                    });
                };
                match command.authorize(&request.arguments, _cancel).await? {
                    CommandAuthorization::Allow {
                        permission_ref,
                        permission_scope,
                        scoped_grant_ref,
                    } => Some((permission_ref, permission_scope, scoped_grant_ref)),
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
            } else {
                None
            };
            if let Some((permission_ref, permission_scope, scoped_grant_ref)) = permission {
                Ok(AuthorizationDecision::Allow {
                    policy_ref: PolicyReference::new(policy.digest())
                        .expect("policy digest is a valid opaque reference"),
                    permission_ref,
                    permission_scope,
                    scoped_grant_ref,
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
