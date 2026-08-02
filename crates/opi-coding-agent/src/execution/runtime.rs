//! Phase 16.8: the deep Execution Runtime assembly.
//!
//! [`ExecutionRuntime::build`] is the SOLE module that turns resolved execution
//! configuration, enabled named-package identities, the resolved User Policy,
//! and protocol hosting into a [`BashOperations`] selection. It decides between
//! the two architecture-mandated backends (design §Core Architecture):
//!
//! ```text
//! BashTool -> Arc<dyn BashOperations>
//!   +-- LocalBashOperations        (Minimal Runtime, returned directly)
//!   `-- RoutedBashOperations       (constructed only when routing requires it)
//!        +-- resolve_selection     (16.6 pure router)
//!        +-- local_ops             (LocalBashOperations, for `local` selections)
//!        +-- ProcessCommandAdapter (one per enabled external identity)
//!             `-- ExecutionProtocolHost (16.7)
//! ```
//!
//! # Branch contract (DoD)
//!
//! - **Minimal Runtime:** default-local routing + no enabled executable identity
//!   returns the injected `local_ops` directly. The store is never called and no
//!   router/eligibility/protocol/adapter state is constructed (proven transitively
//!   by pointer-identity with the injected `local_ops`). The already-resolved User
//!   Policy is still consulted for `local`, so an explicit `local = "deny"|"ask"`
//!   is honored exactly like any other adapter; reading the borrowed policy is not
//!   "constructing permission state".
//! - **Routed:** any other case constructs [`RoutedBashOperations`]. A selected
//!   external failure becomes a tool failure; **no failure path invokes another
//!   adapter** (no `local` fallback).
//!
//! # Substrate scope
//!
//! This task does NOT wire startup (16.9), add the bash-schema `backend` field
//! (16.9), or implement interactive `ask` prompting (16.9). Model-strategy
//! invocations therefore resolve with `adapter_not_selected` until 16.9 supplies
//! the per-invocation backend. The three types this module defines
//! ([`ExecutionRuntime`], [`RoutedBashOperations`], [`ProcessCommandAdapter`])
//! have zero production callers in 16.8 — they are test-driven substrate seams
//! exercised behaviorally via a mock [`IdentitySource`] and the 16.7 mock peer.

use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use opi_protocol::execution::v1::{Bounds, EnvInherit, NativeString, ProtocolId, WIRE_IDENTITY};

use crate::config::{ExecutionConfig, ExecutionRunMode, ExecutionStrategy, PermissionDecision};
use crate::package_activation::{ActivatedContribution, ActivationError, PackageActivationStore};
use crate::tool::{
    BashOpError, BashOperations, BashRequest, BashResult, LOCAL_BASH_OPERATION_DIAGNOSTIC,
    ToolDiagnostic,
};

use opi_tui::{PermissionChoice, PermissionSummary};

use super::failure::{ExecutionFailure, UnavailableDetail};
use super::permission::{
    InteractivePermissionBroker, LOCAL_ADAPTER_ID, PermissionManager, PermissionPolicy,
    run_mode_label,
};
use super::router::{Eligibility, EligibleAdapter, resolve_selection};
// The protocol-host entry types are re-exported by the parent `execution` module
// (16.7). `BackendLaunch`/`ExecutionRequest` carry borrowed lifetimes; the
// adapter owns its launch params locally and borrows them for the one `execute`.
use super::{BackendLaunch, CompletedOutcome, ExecutionProtocolHost, ExecutionRequest};

/// Stable diagnostic code for a backend-reported protocol diagnostic lifted into
/// a [`ToolDiagnostic`]. Backend diagnostics are message-only (redaction is the
/// backend's responsibility per the v1 wire); this code lets embedders match them.
const BACKEND_DIAGNOSTIC_CODE: &str = "opi.execution.backend_diagnostic";

/// Grace granted to the backend to report a terminal cleanup state. Mirrors
/// `execution::protocol_host::CLEANUP_REPORT_GRACE` (1500ms): the host sends
/// `cancel` at `deadline - CLEANUP_REPORT_GRACE`. Setting the host deadline to
/// `command_timeout + CLEANUP_REPORT_GRACE` aligns the host's cancel point with
/// the `execute.timeout_ms` sent to the backend, so the host never pre-empts a
/// command that still fits the configured timeout (audit FL25: no host/backend
/// timeout race). `protocol_host.rs` is owned by 16.7 (not edited here), so this
/// mirrors its constant rather than re-exporting it.
const CLEANUP_REPORT_GRACE: Duration = Duration::from_millis(1500);

// =========================================================================
// IdentitySource — injectable activation seam
// =========================================================================

/// Injectable pre-spawn revalidation seam over the named-package store.
///
/// Production is implemented by [`PackageActivationStore`] (forwards to its
/// synchronous `activate`). Tests inject a mock to drive [`ProcessCommandAdapter`]
/// without filesystem package fixtures, and a panic-on-call sentinel to prove
/// the Minimal-Runtime branch never touches the store. `activate` is
/// intentionally synchronous (it does blocking file I/O);
/// [`ProcessCommandAdapter::exec`] offloads it via `spawn_blocking`.
pub trait IdentitySource: Send + Sync {
    /// Resolve + revalidate the named package's executable contribution
    /// immediately before a process start. Returns metadata only (no spawn).
    fn activate(
        &self,
        name: &str,
        host_target: &str,
        host_opi_version: &str,
    ) -> Result<ActivatedContribution, ActivationError>;
}

impl IdentitySource for PackageActivationStore {
    fn activate(
        &self,
        name: &str,
        host_target: &str,
        host_opi_version: &str,
    ) -> Result<ActivatedContribution, ActivationError> {
        PackageActivationStore::activate(self, name, host_target, host_opi_version)
    }
}

// =========================================================================
// EnabledIdentity — the caller-resolved named identity input
// =========================================================================

/// One enabled named-package identity, resolved by the caller (16.9 startup).
///
/// Target compatibility is NOT carried here: it is hard-gated by the
/// contribution validator and re-gated by [`IdentitySource::activate`] at every
/// process start, so a mismatch yields `adapter_unavailable` (fail-closed),
/// never a wrong-target spawn. [`ExecutionRuntime::build`] therefore treats
/// every enabled identity as `available` and lets per-invocation activation be
/// the authoritative gate.
#[derive(Debug, Clone)]
pub struct EnabledIdentity {
    pub adapter_id: String,
    pub package_name: String,
}

impl Eligibility {
    /// Build the eligible-adapter set from the resolved enabled identities and
    /// permission policy. `local` is always a member (the built-in host
    /// backend); each enabled identity contributes one entry. This is the pure
    /// eligibility construction shared by [`ExecutionRuntime::build`] (Branch 2)
    /// and the 16.9 harness model-schema builder. It never queries the package
    /// store or spawns; `available` is true for every entry because
    /// per-invocation [`IdentitySource::activate`] is the authoritative
    /// availability gate.
    pub fn from_enabled(enabled: &[EnabledIdentity], policy: &PermissionPolicy) -> Self {
        let mut adapters = Vec::with_capacity(enabled.len() + 1);
        adapters.push(EligibleAdapter {
            id: LOCAL_ADAPTER_ID.to_string(),
            available: true,
            permission: policy.decision_for(LOCAL_ADAPTER_ID),
        });
        for identity in enabled {
            adapters.push(EligibleAdapter {
                id: identity.adapter_id.clone(),
                available: true,
                permission: policy.decision_for(&identity.adapter_id),
            });
        }
        Eligibility(adapters)
    }
}

// =========================================================================
// ExecutionRuntime — the sole assembly
// =========================================================================

/// The Execution Runtime assembly entry point.
pub struct ExecutionRuntime;

impl ExecutionRuntime {
    /// Assemble the [`BashOperations`] selection for one run.
    ///
    /// See the module docs for the branch contract and substrate scope.
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        config: &ExecutionConfig,
        mode: ExecutionRunMode,
        enabled: &[EnabledIdentity],
        policy: &PermissionPolicy,
        store: Arc<dyn IdentitySource>,
        local_ops: Arc<dyn BashOperations>,
        workspace_root: &Path,
        host_target: &str,
        host_opi_version: &str,
        manager: Arc<PermissionManager>,
        broker: Option<Arc<dyn InteractivePermissionBroker>>,
    ) -> Result<Arc<dyn BashOperations>, ExecutionFailure> {
        // --- Branch 1: Minimal Runtime ---
        // Default-local routing with no enabled external identity returns the
        // local backend directly. Nothing else is constructed; the store is not
        // called. The borrowed policy IS consulted so an explicit local
        // deny/ask is honored consistently with the routed branch.
        if enabled.is_empty() && is_default_local(config) {
            return match policy.decision_for(LOCAL_ADAPTER_ID) {
                PermissionDecision::Allow => Ok(local_ops),
                PermissionDecision::Deny => Err(ExecutionFailure::PolicyDenied {
                    adapter_id: LOCAL_ADAPTER_ID.to_string(),
                }),
                PermissionDecision::Ask => Err(ExecutionFailure::PermissionRequired {
                    adapter_id: LOCAL_ADAPTER_ID.to_string(),
                    mode,
                }),
            };
        }

        // --- Branch 2: routed assembly ---
        // Eligibility (local + each enabled identity) is built by the shared
        // pure constructor that the 16.9 harness model-schema builder also
        // calls; `available` is true for every entry (per-invocation activation
        // is the authoritative availability gate).
        let eligibility = Eligibility::from_enabled(enabled, policy);
        let mut adapters: HashMap<String, ProcessCommandAdapter> = HashMap::new();
        for identity in enabled {
            // Adapter-id uniqueness within eligibility: contribution validation
            // rejects reserved/colliding ids, so an enabled external never
            // duplicates `local` or another identity.
            let adapter = ProcessCommandAdapter {
                adapter_id: identity.adapter_id.clone(),
                package_name: identity.package_name.clone(),
                store: Arc::clone(&store),
                workspace_root: workspace_root.to_path_buf(),
                host_target: host_target.to_string(),
                host_opi_version: host_opi_version.to_string(),
            };
            adapters.insert(identity.adapter_id.clone(), adapter);
        }
        let routed = RoutedBashOperations {
            config: config.clone(),
            mode,
            eligibility,
            local_ops,
            adapters,
            manager,
            broker,
        };
        routed.assert_invariants();
        Ok(Arc::new(routed))
    }
}

/// The default Minimal-Runtime routing shape: `[execution] strategy = "fixed",
/// backend = "local"`.
fn is_default_local(config: &ExecutionConfig) -> bool {
    config.strategy == ExecutionStrategy::Fixed && config.backend == LOCAL_ADAPTER_ID
}

/// The host's end-to-end deadline for one external execution. The host sends
/// `cancel` at `deadline - CLEANUP_REPORT_GRACE`; passing `command_timeout +
/// CLEANUP_REPORT_GRACE` as the deadline aligns that cancel point with the
/// `execute.timeout_ms` (= `command_timeout`) sent to the backend, so the host
/// never pre-empts a command that still fits the configured timeout (audit FL25:
/// no host/backend race). Extracted as a pure helper so the deadline policy is
/// unit-testable across timeouts rather than buried inline in `exec`.
fn host_deadline(command_timeout: Duration) -> Duration {
    command_timeout + CLEANUP_REPORT_GRACE
}

// =========================================================================
// RoutedBashOperations
// =========================================================================

/// The routed [`BashOperations`] backend. Exists only when configured routing
/// may select a non-`local` adapter. Resolves one selection per invocation via
/// the pure 16.6 router and dispatches to the local backend or the matching
/// [`ProcessCommandAdapter`]. A selection or adapter failure propagates as a
/// tool failure; there is no fallback to `local` or any other adapter.
pub struct RoutedBashOperations {
    config: ExecutionConfig,
    mode: ExecutionRunMode,
    eligibility: Eligibility,
    local_ops: Arc<dyn BashOperations>,
    adapters: HashMap<String, ProcessCommandAdapter>,
    /// In-memory session grants shared with the harness (reset on in-process
    /// session switches). Checked before prompting and updated on allow-session.
    manager: Arc<PermissionManager>,
    /// Interactive `ask`-prompt seam. `None` is fail-closed: an interactive
    /// `ask` surfaces `permission_required` rather than dispatching or falling
    /// back to `local`. Headless modes install no broker.
    broker: Option<Arc<dyn InteractivePermissionBroker>>,
}

impl RoutedBashOperations {
    /// Invariant (audit FL11): every non-`local` eligibility entry has a matching
    /// adapter, so `adapters.get(selection.backend)` is total by construction.
    /// `resolve_selection` can only return a backend that is a member of the
    /// input eligibility (router.rs debug_assert), and `local` dispatches to
    /// `local_ops`, so a missing adapter is provably unreachable.
    fn assert_invariants(&self) {
        debug_assert!(
            self.eligibility
                .0
                .iter()
                .filter(|e| e.id != LOCAL_ADAPTER_ID)
                .all(|e| self.adapters.contains_key(&e.id)),
            "every non-local eligible adapter must have a ProcessCommandAdapter"
        );
    }
}

impl BashOperations for RoutedBashOperations {
    fn exec(
        &self,
        request: BashRequest,
    ) -> Pin<Box<dyn Future<Output = Result<BashResult, BashOpError>> + Send>> {
        // resolve_selection is pure and cheap; run it directly (no await), then
        // dispatch. The returned future borrows neither `&self` nor the inputs.
        // The model-supplied `backend` reaches the router here (Phase 16.9); it is
        // `None` under `fixed`/`rules` (where the router ignores it) and for the
        // local backend, and is only consulted under `strategy = "model"`.
        let selection = match resolve_selection(
            &self.config,
            self.mode,
            &self.eligibility,
            request.backend.as_deref(),
        ) {
            Ok(sel) => sel,
            Err(failure) => {
                // Phase 16.10 broker interception: a routed PermissionRequired
                // in Interactive mode with a broker installed is the ask-prompt
                // trigger. Grant dispatches DIRECTLY to the selected adapter —
                // never re-runs resolve_selection (the static policy is still
                // `ask`; the in-memory grant is the only escalation path, which
                // the pure router cannot observe — re-running it would re-fail
                // forever). Headless/no-broker permission_required and every
                // other failure pass through unchanged.
                if let ExecutionFailure::PermissionRequired {
                    ref adapter_id,
                    mode,
                } = failure
                    && mode == ExecutionRunMode::Interactive
                    && let Some(broker) = self.broker.clone()
                {
                    let manager = Arc::clone(&self.manager);
                    let adapters = self.adapters.clone();
                    let local_ops = Arc::clone(&self.local_ops);
                    let adapter_id = adapter_id.clone();
                    let mode = self.mode;
                    return Box::pin(async move {
                        resolve_ask_and_dispatch(
                            &manager, broker, adapters, local_ops, adapter_id, mode, request,
                        )
                        .await
                    });
                }
                let err = exec_failure_to_bash_op_error(failure);
                return Box::pin(async move { Err(err) });
            }
        };
        if selection.backend == LOCAL_ADAPTER_ID {
            let local_ops = Arc::clone(&self.local_ops);
            return Box::pin(async move { local_ops.exec(request).await });
        }
        // By construction (assert_invariants) the selected external has an
        // adapter. The defensive None arm is unreachable; map it to a named
        // error rather than unwrapping so a future invariant break cannot panic
        // a production run.
        match self.adapters.get(&selection.backend).cloned() {
            Some(adapter) => Box::pin(async move { adapter.exec(request).await }),
            None => Box::pin(async move {
                Err(BashOpError::Other {
                    message: format!(
                        "selected backend {:?} has no adapter (invariant violation)",
                        selection.backend
                    ),
                })
            }),
        }
    }
}

// =========================================================================
// Phase 16.10: interactive ask resolution + direct dispatch helpers
// =========================================================================

/// Resolve an interactive `ask` through the broker and dispatch directly to the
/// selected adapter. The static policy is still `ask`; the in-memory session
/// grant (or a fresh broker choice) is the ONLY escalation path, so this NEVER
/// re-runs `resolve_selection` (which would re-fail on the static ask policy).
async fn resolve_ask_and_dispatch(
    manager: &PermissionManager,
    broker: Arc<dyn InteractivePermissionBroker>,
    adapters: HashMap<String, ProcessCommandAdapter>,
    local_ops: Arc<dyn BashOperations>,
    adapter_id: String,
    mode: ExecutionRunMode,
    request: BashRequest,
) -> Result<BashResult, BashOpError> {
    // Session-grant short-circuit: an allow-for-session choice earlier this
    // session suppresses re-prompting. (BashTool runs Sequentially, so there is
    // no concurrent double-prompt on the same adapter.)
    if !manager.has_session_grant(&adapter_id) {
        let summary = permission_summary(&adapter_id, &adapters, mode);
        match broker.resolve_ask(summary).await {
            PermissionChoice::AllowSession => manager.grant_session(&adapter_id),
            // AllowOnce authorizes exactly this invocation; consumed at decision
            // time, independent of dispatch outcome (a crashed dispatch neither
            // burns nor double-spends it).
            PermissionChoice::AllowOnce => {}
            PermissionChoice::Deny => {
                return Err(exec_failure_to_bash_op_error(
                    ExecutionFailure::PermissionDenied { adapter_id },
                ));
            }
        }
    }
    dispatch_direct(&adapters, local_ops, &adapter_id, request).await
}

/// Build the redaction-safe [`PermissionSummary`] for a prompt. Carries ONLY
/// adapter id + package name + run-mode label — never command text, env, or
/// paths (the Phase 16 redaction invariant).
fn permission_summary(
    adapter_id: &str,
    adapters: &HashMap<String, ProcessCommandAdapter>,
    mode: ExecutionRunMode,
) -> PermissionSummary {
    let package_name = if adapter_id == LOCAL_ADAPTER_ID {
        String::new()
    } else {
        adapters
            .get(adapter_id)
            .map(|a| a.package_name.clone())
            .unwrap_or_default()
    };
    PermissionSummary {
        adapter_id: adapter_id.to_string(),
        package_name,
        run_mode_label: run_mode_label(mode).to_string(),
    }
}

/// Dispatch directly to the already-selected adapter (local or external). This
/// is the post-grant path: `resolve_selection` already picked `adapter_id`, so
/// re-running it is both unnecessary and wrong (it would re-fail on the static
/// ask policy). Mirrors the normal Ok-dispatch minus the router call.
async fn dispatch_direct(
    adapters: &HashMap<String, ProcessCommandAdapter>,
    local_ops: Arc<dyn BashOperations>,
    adapter_id: &str,
    request: BashRequest,
) -> Result<BashResult, BashOpError> {
    if adapter_id == LOCAL_ADAPTER_ID {
        return local_ops.exec(request).await;
    }
    match adapters.get(adapter_id).cloned() {
        Some(adapter) => adapter.exec(request).await,
        None => Err(BashOpError::Other {
            message: format!(
                "selected backend {adapter_id:?} has no adapter (invariant violation)"
            ),
        }),
    }
}

// =========================================================================
// ProcessCommandAdapter
// =========================================================================

/// The external-identity [`BashOperations`] adapter. One per enabled identity.
/// On every invocation it revalidates the named package (drift detection
/// "immediately before every process start"), then drives the one-shot 16.7
/// [`ExecutionProtocolHost`] and maps the outcome to a [`BashResult`]. There is
/// no `local` fallback: an activation or protocol failure becomes a tool
/// failure carrying the stable [`ExecutionFailure`] code.
///
/// Holds identity only (not launch params): the command/args/adapter_config are
/// re-fetched from each activation, so stale startup state can never drive a
/// spawn (TOCTOU-safe). Derives [`Clone`] so [`RoutedBashOperations::exec`] can
/// cheaply clone one adapter (Arc bump + owned small fields) into the dispatched
/// future.
#[derive(Clone)]
pub struct ProcessCommandAdapter {
    adapter_id: String,
    package_name: String,
    store: Arc<dyn IdentitySource>,
    workspace_root: PathBuf,
    host_target: String,
    host_opi_version: String,
}

impl BashOperations for ProcessCommandAdapter {
    fn exec(
        &self,
        request: BashRequest,
    ) -> Pin<Box<dyn Future<Output = Result<BashResult, BashOpError>> + Send>> {
        let store = Arc::clone(&self.store);
        let package_name = self.package_name.clone();
        let host_target = self.host_target.clone();
        let host_opi_version = self.host_opi_version.clone();
        let workspace_root = self.workspace_root.clone();
        let adapter_id = self.adapter_id.clone();
        Box::pin(async move {
            // 1. Pre-spawn revalidation. activate() is synchronous file I/O
            //    (trust-file + manifest + lock + hash recompute); offload it to
            //    the blocking pool so it never stalls the async worker.
            let activated = match tokio::task::spawn_blocking(move || {
                store.activate(&package_name, &host_target, &host_opi_version)
            })
            .await
            {
                Ok(Ok(a)) => a,
                Ok(Err(activation)) => {
                    return Err(exec_failure_to_bash_op_error(ExecutionFailure::from(
                        activation,
                    )));
                }
                Err(_) => {
                    return Err(exec_failure_to_bash_op_error(
                        ExecutionFailure::AdapterUnavailable {
                            adapter_id: Some(adapter_id.clone()),
                            detail: UnavailableDetail::Store,
                        },
                    ));
                }
            };
            // 2. Locate this adapter's contribution within the activated package.
            let contribution = match activated.validated.iter().find(|v| v.id == adapter_id) {
                Some(c) => c,
                None => {
                    return Err(exec_failure_to_bash_op_error(
                        ExecutionFailure::AdapterUnavailable {
                            adapter_id: Some(adapter_id.clone()),
                            detail: UnavailableDetail::Store,
                        },
                    ));
                }
            };
            // 3. Build the protocol request. `command` is the raw bash string
            //    (the host owns shell mapping); env additions are native-string
            //    keyed (infallible UTF-8 conversion). The host deadline is the
            //    command timeout plus the cleanup-report grace so the host's
            //    cancel point aligns with the backend's execute timeout.
            let env_additions: BTreeMap<NativeString, NativeString> = request
                .env
                .iter()
                .map(|(k, v)| (NativeString::from_utf8(k), NativeString::from_utf8(v)))
                .collect();
            let supported_protocols = vec![ProtocolId::new(WIRE_IDENTITY)];
            let deadline = host_deadline(request.timeout);
            let launch = BackendLaunch {
                program: &contribution.command,
                args: &contribution.args,
            };
            let protocol_request = ExecutionRequest {
                command: &request.command,
                workspace: &workspace_root,
                cwd: &request.cwd,
                timeout: request.timeout,
                deadline,
                env_inherit: EnvInherit::Inherit,
                env_additions: &env_additions,
                adapter_config: contribution.adapter_config.clone(),
                supported_protocols: &supported_protocols,
                signal: request.signal,
                bounds: Bounds::DEFAULT,
            };
            // 4. Drive the one-shot host and map the outcome.
            match ExecutionProtocolHost::execute(launch, protocol_request).await {
                Ok(outcome) => Ok(completed_outcome_to_bash_result(outcome)),
                Err(failure) => Err(exec_failure_to_bash_op_error(failure)),
            }
        })
    }
}

// =========================================================================
// Outcome + error mapping
// =========================================================================

/// Map a terminal in-band [`CompletedOutcome`] to a [`BashResult`]. Emits the
/// local `LOCAL_BASH_OPERATION_DIAGNOSTIC` operation-context diagnostic carrying
/// `exit_code`/`cancelled`/`timed_out` so `bash.rs`'s `lift_operation_context`
/// treats the routed backend as a transparent drop-in (no bash.rs edit on the
/// success path), then appends backend-reported diagnostics under
/// [`BACKEND_DIAGNOSTIC_CODE`].
fn completed_outcome_to_bash_result(outcome: CompletedOutcome) -> BashResult {
    let mut diagnostics: Vec<ToolDiagnostic> = Vec::with_capacity(outcome.diagnostics.len() + 1);
    diagnostics.push(operation_context_diagnostic(
        outcome.exit.map(|e| e as i32),
        outcome.cancelled,
        outcome.timed_out,
    ));
    for diagnostic in outcome.diagnostics {
        diagnostics.push(ToolDiagnostic {
            code: BACKEND_DIAGNOSTIC_CODE.to_string(),
            message: diagnostic.message,
            details: None,
        });
    }
    BashResult {
        stdout: outcome.stdout,
        stderr: outcome.stderr,
        exit_code: outcome.exit.map(|e| e as i32),
        signal: outcome.signal.map(|s| s as i32),
        diagnostics,
    }
}

/// Build the local operation-context [`ToolDiagnostic`] (mirrors
/// `operations::bash_operation_context_diagnostic` for the routed path).
/// `command_included` is always false (commands may carry secrets); `truncated`
/// is always false here because output bounding is the host's responsibility and
/// a truncated external stream surfaces as a protocol violation, not an
/// in-band result.
fn operation_context_diagnostic(
    exit_code: Option<i32>,
    cancelled: bool,
    timed_out: bool,
) -> ToolDiagnostic {
    let message = if cancelled {
        "command cancelled"
    } else if timed_out {
        "command timed out"
    } else {
        "command executed"
    };
    ToolDiagnostic {
        code: LOCAL_BASH_OPERATION_DIAGNOSTIC.to_string(),
        message: message.to_string(),
        details: Some(serde_json::json!({
            "exit_code": exit_code,
            "cancelled": cancelled,
            "timed_out": timed_out,
            "truncated": false,
            "command_included": false,
        })),
    }
}

/// Map a command-execute [`ExecutionFailure`] to a [`BashOpError`] at the
/// [`BashOperations`] boundary.
///
/// The stable code rides into the agent `ToolResult` via a [`ToolDiagnostic`]
/// whose `code` is the stable literal: `bash.rs`'s `append_backend_diagnostics`
/// lifts `error.diagnostics()` (every code except `LOCAL_BASH_OPERATION_DIAGNOSTIC`)
/// into the result, and `root_cause()` unwraps the [`BashOpError::BackendFailure`]
/// wrapper to its `Other` source for the user-facing message.
///
/// INVARIANT: no [`ExecutionFailure`] code equals
/// `LOCAL_BASH_OPERATION_DIAGNOSTIC` (`"opi.operations.bash.operation_context"`),
/// or `bash.rs`'s lift filter would drop it. The 14 stable execution codes never
/// collide with that marker; this is a cross-module coupling between this mapper
/// and `bash.rs`'s filter — a future `ExecutionFailure` variant must keep out of
/// that namespace.
fn exec_failure_to_bash_op_error(failure: ExecutionFailure) -> BashOpError {
    let code = failure.code();
    BashOpError::BackendFailure {
        source: Box::new(BashOpError::Other {
            message: code.to_string(),
        }),
        diagnostics: vec![ToolDiagnostic {
            code: code.to_string(),
            message: failure.to_string(),
            details: Some(serde_json::json!({
                "code": code,
                "remediation": failure.remediation(),
            })),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::permission::PermissionPolicy;
    use crate::tool::LocalBashOperations;
    use std::collections::BTreeMap;

    fn default_local_config() -> ExecutionConfig {
        ExecutionConfig::default()
    }

    #[test]
    fn is_default_local_recognizes_the_spec_default() {
        assert!(is_default_local(&default_local_config()));
        let mut c = default_local_config();
        c.backend = "opi-sandbox".into();
        assert!(!is_default_local(&c));
        let mut c = default_local_config();
        c.strategy = ExecutionStrategy::Rules;
        assert!(!is_default_local(&c));
    }

    #[test]
    fn host_deadline_aligns_host_cancel_with_backend_timeout() {
        // FL25: the host's cancel point is `deadline - CLEANUP_REPORT_GRACE`.
        // Driving the REAL `host_deadline` helper (not std-lib Duration Add)
        // across several command timeouts asserts the production invariant:
        // cancel_at == command_timeout, i.e. the host never pre-empts a command
        // that still fits the configured timeout.
        for timeout in [
            Duration::ZERO,
            Duration::from_secs(5),
            Duration::from_secs(30),
        ] {
            let deadline = host_deadline(timeout);
            let cancel_at = deadline
                .checked_sub(CLEANUP_REPORT_GRACE)
                .unwrap_or(Duration::ZERO);
            assert_eq!(
                cancel_at, timeout,
                "host cancel point must equal the backend execute timeout"
            );
            assert_eq!(deadline, timeout + CLEANUP_REPORT_GRACE);
        }
    }

    #[test]
    fn exec_failure_mapping_preserves_every_stable_code() {
        // The stable code rides both as the root-cause Other message AND as a
        // surviving BackendFailure diagnostic (the path bash.rs lifts).
        let cases: Vec<(&str, ExecutionFailure)> = vec![
            (
                "policy_denied",
                ExecutionFailure::PolicyDenied {
                    adapter_id: "opi-sandbox".into(),
                },
            ),
            (
                "permission_required",
                ExecutionFailure::PermissionRequired {
                    adapter_id: "opi-sandbox".into(),
                    mode: ExecutionRunMode::Rpc,
                },
            ),
            (
                "no_eligible_adapter",
                ExecutionFailure::NoEligibleAdapter {
                    strategy: ExecutionStrategy::Fixed,
                    mode: ExecutionRunMode::Interactive,
                },
            ),
            (
                "adapter_not_selected",
                ExecutionFailure::AdapterNotSelected {
                    requested: "ghost".into(),
                    strategy: ExecutionStrategy::Model,
                },
            ),
            (
                "adapter_unavailable",
                ExecutionFailure::AdapterUnavailable {
                    adapter_id: None,
                    detail: UnavailableDetail::Store,
                },
            ),
            ("protocol_violation", ExecutionFailure::ProtocolViolation),
            ("execution_failed", ExecutionFailure::ExecutionFailed),
            ("cleanup_unconfirmed", ExecutionFailure::CleanupUnconfirmed),
        ];
        for (expected, failure) in cases {
            let err = exec_failure_to_bash_op_error(failure);
            // Root cause is Other { message = code }.
            match err.root_cause() {
                BashOpError::Other { message } => assert_eq!(message, expected),
                other => panic!("root cause must be Other, got {other:?}"),
            }
            // The stable-code diagnostic survives (never equals the marker that
            // bash.rs filters out).
            assert_ne!(expected, LOCAL_BASH_OPERATION_DIAGNOSTIC);
            assert!(
                err.diagnostics().iter().any(|d| d.code == expected),
                "diagnostic with code {expected:?} must survive the mapping for {expected:?}"
            );
        }
    }

    #[test]
    fn minimal_runtime_with_local_deny_is_policy_denied() {
        // MF2: branch 1 consults the policy — an explicit local deny must fail
        // even with no enabled identity, and the store must never be touched.
        let mut decisions: BTreeMap<String, PermissionDecision> = BTreeMap::new();
        decisions.insert(LOCAL_ADAPTER_ID.to_string(), PermissionDecision::Deny);
        let policy = PermissionPolicy::from_map(decisions);
        let local_ops: Arc<dyn BashOperations> = Arc::new(LocalBashOperations::new());
        let store: Arc<dyn IdentitySource> = Arc::new(PanicStore);
        let err = match ExecutionRuntime::build(
            &default_local_config(),
            ExecutionRunMode::Interactive,
            &[],
            &policy,
            store,
            Arc::clone(&local_ops),
            Path::new("."),
            "x86_64-pc-windows-msvc",
            "0.8.0",
            Arc::new(PermissionManager::new()),
            None,
        ) {
            Err(e) => e,
            Ok(_) => panic!("expected policy_denied, but build returned Ok"),
        };
        assert_eq!(err.code(), "policy_denied");
    }

    /// A store that panics if activated (Minimal-Runtime / branch-1 sentinel).
    struct PanicStore;
    impl IdentitySource for PanicStore {
        fn activate(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<ActivatedContribution, ActivationError> {
            panic!("Minimal Runtime must not activate any package");
        }
    }

    // NOTE: behavioral branch proofs (Minimal-Runtime pointer-identity,
    // no-local-fallback, mock-peer-driven ProcessCommandAdapter end-to-end incl.
    // the bash.rs ToolResult code lift) live in the integration suite
    // `tests/execution_runtime.rs`.
}
