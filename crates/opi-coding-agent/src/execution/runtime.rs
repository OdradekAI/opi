//! Phase 16.8: the deep Execution Runtime assembly.
//!
//! [`ExecutionRuntime::build`] turns routed execution configuration, enabled
//! named-package identities, the resolved User Policy, and protocol hosting
//! into a [`BashOperations`] selection. The production harness classifies
//! fixed-local allow before constructing extended execution state; this entry
//! point preserves the same direct-local result for embedders and tests. It
//! decides between the two architecture-mandated backends:
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
//! - **Minimal Runtime:** resolved fixed-local routing with effective
//!   `local = "allow"` returns the injected `local_ops` directly. Enabled but
//!   unselected external identities do not change that result. The store is
//!   never called and no router/eligibility/protocol/adapter state is
//!   constructed. Explicit `local = "deny"|"ask"` settings are outside this
//!   branch; interactive `ask` uses the routed permission broker, while
//!   headless `ask` is refused at build time with `permission_required`.
//! - **Routed:** any other case constructs [`RoutedBashOperations`]. A selected
//!   external failure becomes a tool failure; **no failure path invokes another
//!   adapter** (no `local` fallback).
//!
//! The routed branch owns eligibility, selection, interactive permission, and
//! external protocol adapters. The fixed-local allow branch owns none of that
//! state; headless fixed-local ask is rejected during harness construction.

use std::collections::{BTreeMap, HashSet};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use opi_protocol::execution::v1::{Bounds, EnvInherit, NativeString, ProtocolId, WIRE_IDENTITY};

use crate::config::{ExecutionConfig, ExecutionRunMode, ExecutionStrategy, PermissionDecision};
use crate::package_activation::{ActivatedContribution, ActivationError, PackageActivationStore};
use crate::tool::{
    BashExecutionContract, BashOpError, BashOperationContext, BashOperations, BashRequest,
    BashResult, ToolDiagnostic, finalize_complete_bash_output,
};

use opi_tui::{PermissionChoice, PermissionSummary};

use super::CLEANUP_REPORT_GRACE;
use super::failure::{ExecutionFailure, UnavailableDetail};
use super::permission::{
    InteractivePermissionBroker, LOCAL_ADAPTER_ID, PermissionManager, PermissionPolicy,
    run_mode_label,
};
use super::router::{
    CandidateDecision, CandidateSelection, Eligibility, EligibleAdapter, resolve_candidate,
};
// The protocol-host entry types are re-exported by the parent `execution` module
// (16.7). `BackendLaunch`/`ExecutionRequest` carry borrowed lifetimes; the
// adapter owns its launch params locally and borrows them for the one `execute`.
use super::{BackendLaunch, CompletedOutcome, ExecutionProtocolHost, ExecutionRequest};

#[cfg(test)]
pub(crate) mod construction_probe {
    use std::cell::RefCell;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    pub(crate) struct Counts {
        permission_managers: AtomicUsize,
        brokers: AtomicUsize,
        routers: AtomicUsize,
        protocol_states: AtomicUsize,
    }

    impl Counts {
        pub(crate) fn permission_managers(&self) -> usize {
            self.permission_managers.load(Ordering::SeqCst)
        }

        pub(crate) fn brokers(&self) -> usize {
            self.brokers.load(Ordering::SeqCst)
        }

        pub(crate) fn routers(&self) -> usize {
            self.routers.load(Ordering::SeqCst)
        }

        pub(crate) fn protocol_states(&self) -> usize {
            self.protocol_states.load(Ordering::SeqCst)
        }
    }

    thread_local! {
        static ACTIVE: RefCell<Option<Arc<Counts>>> = const { RefCell::new(None) };
    }

    pub(crate) struct Guard {
        previous: Option<Arc<Counts>>,
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            ACTIVE.with(|active| {
                active.replace(self.previous.take());
            });
        }
    }

    pub(crate) fn install() -> (Arc<Counts>, Guard) {
        let counts = Arc::new(Counts::default());
        let previous = ACTIVE.with(|active| active.replace(Some(Arc::clone(&counts))));
        (counts, Guard { previous })
    }

    fn with_counts(f: impl FnOnce(&Counts)) {
        ACTIVE.with(|active| {
            if let Some(counts) = active.borrow().as_ref() {
                f(counts);
            }
        });
    }

    pub(crate) fn permission_manager_constructed() {
        with_counts(|counts| {
            counts.permission_managers.fetch_add(1, Ordering::SeqCst);
        });
    }

    pub(crate) fn broker_constructed() {
        with_counts(|counts| {
            counts.brokers.fetch_add(1, Ordering::SeqCst);
        });
    }

    pub(crate) fn router_constructed() {
        with_counts(|counts| {
            counts.routers.fetch_add(1, Ordering::SeqCst);
        });
    }

    pub(crate) fn protocol_state_constructed() {
        with_counts(|counts| {
            counts.protocol_states.fetch_add(1, Ordering::SeqCst);
        });
    }
}

/// Stable diagnostic code for a backend-reported protocol diagnostic lifted into
/// a [`ToolDiagnostic`]. Backend diagnostics are message-only (redaction is the
/// backend's responsibility per the v1 wire); this code lets embedders match them.
const BACKEND_DIAGNOSTIC_CODE: &str = "opi.execution.backend_diagnostic";

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
    /// immediately before a process start. Returns metadata plus immutable
    /// validated executable launch material (no spawn).
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

/// Pure classification of the resolved fixed-local permission boundary.
/// Harness startup and runtime assembly share this plan so early lazy-state
/// decisions cannot drift from [`ExecutionRuntime::build`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionPlan {
    DirectLocal,
    PolicyDenied,
    HeadlessAskRefused,
    InteractiveAskRouted,
    GeneralRouted,
}

impl ExecutionPlan {
    pub(crate) fn refusal(self, mode: ExecutionRunMode) -> Option<ExecutionFailure> {
        match self {
            Self::PolicyDenied => Some(ExecutionFailure::PolicyDenied {
                adapter_id: LOCAL_ADAPTER_ID.to_string(),
            }),
            Self::HeadlessAskRefused => Some(ExecutionFailure::PermissionRequired {
                adapter_id: LOCAL_ADAPTER_ID.to_string(),
                mode,
            }),
            Self::DirectLocal | Self::InteractiveAskRouted | Self::GeneralRouted => None,
        }
    }
}

pub(crate) fn execution_plan(
    config: &ExecutionConfig,
    mode: ExecutionRunMode,
    policy: &PermissionPolicy,
) -> ExecutionPlan {
    if !is_default_local(config) {
        return ExecutionPlan::GeneralRouted;
    }

    match policy.decision_for(LOCAL_ADAPTER_ID) {
        PermissionDecision::Allow => ExecutionPlan::DirectLocal,
        PermissionDecision::Deny => ExecutionPlan::PolicyDenied,
        PermissionDecision::Ask if mode == ExecutionRunMode::Interactive => {
            ExecutionPlan::InteractiveAskRouted
        }
        PermissionDecision::Ask => ExecutionPlan::HeadlessAskRefused,
    }
}

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
        let plan = execution_plan(config, mode, policy);
        if let Some(failure) = plan.refusal(mode) {
            return Err(failure);
        }
        match plan {
            // --- Branch 1: Minimal Runtime ---
            // Resolved default fixed-local routing with effective local allow
            // returns the local backend directly. Enabled external identities
            // are unselected and require no router/adapter state.
            ExecutionPlan::DirectLocal => return Ok(local_ops),
            // Interactive fixed-local ask needs the routed permission broker.
            // A direct embedder that omits the broker remains fail-closed.
            ExecutionPlan::InteractiveAskRouted if broker.is_none() => {
                return Err(ExecutionFailure::PermissionRequired {
                    adapter_id: LOCAL_ADAPTER_ID.to_string(),
                    mode,
                });
            }
            ExecutionPlan::InteractiveAskRouted | ExecutionPlan::GeneralRouted => {}
            ExecutionPlan::PolicyDenied | ExecutionPlan::HeadlessAskRefused => {
                unreachable!("refused execution plans returned above")
            }
        }

        // --- Branch 2: routed assembly ---
        // Eligibility (local + each enabled identity) is built by the shared
        // pure constructor that the 16.9 harness model-schema builder also
        // calls; `available` is true for every entry (per-invocation activation
        // is the authoritative availability gate).
        #[cfg(test)]
        construction_probe::router_constructed();
        let mut seen = HashSet::with_capacity(enabled.len() + 1);
        seen.insert(LOCAL_ADAPTER_ID.to_string());
        for identity in enabled {
            if !seen.insert(identity.adapter_id.clone()) {
                return Err(ExecutionFailure::AdapterUnavailable {
                    adapter_id: Some(identity.adapter_id.clone()),
                    detail: UnavailableDetail::Collision,
                });
            }
        }
        let eligibility = Eligibility::from_enabled(enabled, policy);
        let mut dispatches = Vec::with_capacity(enabled.len() + 1);
        dispatches.push(DispatchTarget::Local(local_ops));
        for identity in enabled {
            // Adapter-id uniqueness within eligibility: contribution validation
            // rejects reserved/colliding ids, so an enabled external never
            // duplicates `local` or another identity.
            #[cfg(test)]
            construction_probe::protocol_state_constructed();
            let adapter = ProcessCommandAdapter {
                adapter_id: identity.adapter_id.clone(),
                package_name: identity.package_name.clone(),
                store: Arc::clone(&store),
                workspace_root: workspace_root.to_path_buf(),
                host_target: host_target.to_string(),
                host_opi_version: host_opi_version.to_string(),
            };
            dispatches.push(DispatchTarget::External(adapter));
        }
        let routed = RoutedBashOperations {
            config: config.clone(),
            mode,
            eligibility,
            dispatches,
            manager,
            broker,
        };
        Ok(Arc::new(routed))
    }
}

/// The resolved fixed-local routing shape: `[execution] strategy = "fixed",
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
fn host_deadline(command_timeout: Duration) -> Result<Duration, ExecutionFailure> {
    command_timeout
        .checked_add(CLEANUP_REPORT_GRACE)
        .ok_or(ExecutionFailure::ExecutionFailed)
}

// =========================================================================
// RoutedBashOperations
// =========================================================================

/// The routed [`BashOperations`] backend. Constructed when routing may select a
/// non-`local` adapter, or when explicit interactive fixed-local `ask` requires
/// broker mediation before local dispatch. Resolves one selection per invocation
/// via the pure 16.6 router and dispatches to the local backend or the matching
/// [`ProcessCommandAdapter`]. A selection or adapter failure propagates as a
/// tool failure; there is no fallback to `local` or any other adapter.
pub struct RoutedBashOperations {
    config: ExecutionConfig,
    mode: ExecutionRunMode,
    eligibility: Eligibility,
    /// Concrete targets in the same construction-validated order as
    /// `eligibility`. A router candidate index therefore resolves totally.
    dispatches: Vec<DispatchTarget>,
    /// In-memory session grants shared with the harness (reset on in-process
    /// session switches). Checked before prompting and updated on allow-session.
    manager: Arc<PermissionManager>,
    /// Interactive `ask`-prompt seam. `None` is fail-closed: an interactive
    /// `ask` surfaces `permission_required` rather than dispatching or falling
    /// back to `local`. Headless modes install no broker.
    broker: Option<Arc<dyn InteractivePermissionBroker>>,
}

impl RoutedBashOperations {
    fn selected_dispatch(&self, candidate: CandidateSelection) -> SelectedDispatch {
        debug_assert!(
            candidate.index < self.dispatches.len()
                && self.eligibility.0[candidate.index].id == candidate.backend,
            "router candidate must index the construction-validated dispatch catalog"
        );
        SelectedDispatch {
            adapter_id: candidate.backend,
            mode: candidate.mode,
            target: self.dispatches[candidate.index].clone(),
        }
    }
}

impl BashOperations for RoutedBashOperations {
    fn exec(
        &self,
        request: BashRequest,
    ) -> Pin<Box<dyn Future<Output = Result<BashResult, BashOpError>> + Send>> {
        // Candidate resolution is pure and cheap. Its index selects a concrete
        // target from the catalog assembled with the same eligibility order.
        let selection = match resolve_candidate(
            &self.config,
            self.mode,
            &self.eligibility,
            request.backend.as_deref(),
        ) {
            CandidateDecision::Allowed(candidate) => candidate,
            CandidateDecision::Ask(candidate) => {
                let selected = self.selected_dispatch(candidate);
                if selected.mode == ExecutionRunMode::Interactive
                    && let Some(broker) = self.broker.clone()
                {
                    let manager = Arc::clone(&self.manager);
                    return Box::pin(async move {
                        resolve_ask_and_dispatch(&manager, broker, selected, request).await
                    });
                }
                let failure = ExecutionFailure::PermissionRequired {
                    adapter_id: selected.adapter_id,
                    mode: selected.mode,
                };
                let err = exec_failure_to_bash_op_error(failure);
                return Box::pin(async move { Err(err) });
            }
            CandidateDecision::Refused(failure) => {
                let err = exec_failure_to_bash_op_error(failure);
                return Box::pin(async move { Err(err) });
            }
        };
        let selected = self.selected_dispatch(selection);
        Box::pin(async move { selected.dispatch(request).await })
    }
}

// =========================================================================
// Phase 16.10: interactive ask resolution + direct dispatch helpers
// =========================================================================

/// Resolve an interactive `ask` through the broker and dispatch directly to the
/// selected adapter. The static policy is still `ask`; the in-memory session
/// grant (or a fresh broker choice) is the ONLY escalation path, so this never
/// re-runs routing (which would re-fail on the static ask policy).
async fn resolve_ask_and_dispatch(
    manager: &PermissionManager,
    broker: Arc<dyn InteractivePermissionBroker>,
    selected: SelectedDispatch,
    request: BashRequest,
) -> Result<BashResult, BashOpError> {
    // Session-grant short-circuit: an allow-for-session choice earlier this
    // session suppresses re-prompting. (BashTool runs Sequentially, so there is
    // no concurrent double-prompt on the same adapter.)
    if !manager.has_session_grant(&selected.adapter_id) {
        let summary = selected.permission_summary();
        match broker.resolve_ask(summary).await {
            PermissionChoice::AllowSession => manager.grant_session(&selected.adapter_id),
            // AllowOnce authorizes exactly this invocation; consumed at decision
            // time, independent of dispatch outcome (a crashed dispatch neither
            // burns nor double-spends it).
            PermissionChoice::AllowOnce => {}
            PermissionChoice::Deny => {
                return Err(exec_failure_to_bash_op_error(
                    ExecutionFailure::PermissionDenied {
                        adapter_id: selected.adapter_id,
                    },
                ));
            }
        }
    }
    selected.dispatch(request).await
}

/// One concrete dispatch target retained from routing through permission.
#[derive(Clone)]
enum DispatchTarget {
    Local(Arc<dyn BashOperations>),
    External(ProcessCommandAdapter),
}

#[derive(Clone)]
struct SelectedDispatch {
    adapter_id: String,
    mode: ExecutionRunMode,
    target: DispatchTarget,
}

impl SelectedDispatch {
    fn permission_summary(&self) -> PermissionSummary {
        let package_name = match &self.target {
            DispatchTarget::Local(_) => String::new(),
            DispatchTarget::External(adapter) => adapter.package_name.clone(),
        };
        PermissionSummary {
            adapter_id: self.adapter_id.clone(),
            package_name,
            run_mode_label: run_mode_label(self.mode).to_string(),
        }
    }

    async fn dispatch(self, request: BashRequest) -> Result<BashResult, BashOpError> {
        match self.target {
            DispatchTarget::Local(local) => local.exec(request).await,
            DispatchTarget::External(adapter) => adapter.exec(request).await,
        }
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
            let supported_protocols =
                vec![ProtocolId::new(WIRE_IDENTITY).expect("v1 wire identity is non-empty")];
            let deadline = host_deadline(request.timeout).map_err(exec_failure_to_bash_op_error)?;
            let launch_path = contribution.bound_launch_path();
            let launch = BackendLaunch {
                program: &launch_path,
                args: &contribution.args,
                validated_executable: &contribution.executable,
            };
            let protocol_request = ExecutionRequest {
                command: &request.command,
                workspace: &workspace_root,
                cwd: &request.cwd,
                timeout: request.timeout,
                deadline,
                handshake_timeout: Duration::from_millis(contribution.handshake_timeout_ms),
                expected_implementation: &contribution.id,
                expected_implementation_version: &contribution.lock.package_version,
                expected_target: &contribution.target,
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
                Err(failure) => Err(protocol_failure_to_bash_op_error(failure)),
            }
        })
    }
}

// =========================================================================
// Outcome + error mapping
// =========================================================================

/// Map a terminal in-band [`CompletedOutcome`] to a [`BashResult`], applying the
/// same preview/full-output policy as local execution.
fn completed_outcome_to_bash_result(outcome: CompletedOutcome) -> BashResult {
    let output = finalize_complete_bash_output(outcome.stdout, outcome.stderr);
    let diagnostics = outcome
        .diagnostics
        .into_iter()
        .map(|diagnostic| ToolDiagnostic {
            code: BACKEND_DIAGNOSTIC_CODE.to_string(),
            message: diagnostic.message,
            details: None,
        })
        .collect();
    BashResult {
        stdout: output.stdout,
        stderr: output.stderr,
        context: BashOperationContext {
            exit_code: outcome.exit.map(|exit| exit as i32),
            signal: outcome.signal.map(|signal| signal as i32),
            cancelled: outcome.cancelled,
            timed_out: outcome.timed_out,
            truncated: output.truncated,
            full_output: output.full_output,
            kill_error: None,
            contract: BashExecutionContract {
                placement: outcome.started.placement,
                guarantee: outcome.started.guarantee,
                adapter_id: Some(outcome.ready.implementation.as_str().to_string()),
                implementation_version: Some(outcome.ready.implementation_version),
                target: Some(outcome.ready.target.as_str().to_string()),
                protocol: Some(outcome.ready.selected_protocol.as_str().to_string()),
                policy: Some(outcome.started.policy),
                limitations: outcome.started.limitations,
            },
        },
        diagnostics,
    }
}

/// Map a command-execute [`ExecutionFailure`] to a [`BashOpError`] at the
/// [`BashOperations`] boundary.
///
/// The stable code rides into the agent `ToolResult` via a [`ToolDiagnostic`]
/// whose `code` is the stable literal: `bash.rs`'s
/// `append_backend_diagnostics` lifts `error.diagnostics()` into the result,
/// and `root_cause()` unwraps the [`BashOpError::BackendFailure`] wrapper to its
/// `Other` source for the user-facing message.
fn exec_failure_to_bash_op_error(failure: ExecutionFailure) -> BashOpError {
    let code = failure.code();
    let mut details = serde_json::json!({
        "code": code,
        "remediation": failure.remediation(),
    });
    if let Some(adapter_id) = failure.adapter_id() {
        details["adapter_id"] = serde_json::json!(adapter_id);
    }
    BashOpError::BackendFailure {
        source: Box::new(BashOpError::Other {
            message: code.to_string(),
        }),
        diagnostics: vec![ToolDiagnostic {
            code: code.to_string(),
            message: failure.to_string(),
            details: Some(details),
        }],
    }
}

fn protocol_failure_to_bash_op_error(failure: super::ExecutionProtocolFailure) -> BashOpError {
    let mut error = exec_failure_to_bash_op_error(failure.failure);
    let BashOpError::BackendFailure { diagnostics, .. } = &mut error else {
        unreachable!("execution failure mapping always returns BackendFailure");
    };
    diagnostics.extend(
        failure
            .diagnostics
            .into_iter()
            .map(|diagnostic| ToolDiagnostic {
                code: BACKEND_DIAGNOSTIC_CODE.to_string(),
                message: diagnostic.message,
                details: None,
            }),
    );
    error
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
    fn execution_plan_classifies_all_fixed_local_permission_paths() {
        assert_eq!(
            execution_plan(
                &default_local_config(),
                ExecutionRunMode::Interactive,
                &PermissionPolicy::empty(),
            ),
            ExecutionPlan::DirectLocal,
        );

        let policy = |decision| {
            let mut decisions = BTreeMap::new();
            decisions.insert(LOCAL_ADAPTER_ID.to_string(), decision);
            PermissionPolicy::from_map(decisions)
        };
        assert_eq!(
            execution_plan(
                &default_local_config(),
                ExecutionRunMode::Interactive,
                &policy(PermissionDecision::Deny),
            ),
            ExecutionPlan::PolicyDenied,
        );
        assert_eq!(
            execution_plan(
                &default_local_config(),
                ExecutionRunMode::NonInteractive,
                &policy(PermissionDecision::Ask),
            ),
            ExecutionPlan::HeadlessAskRefused,
        );
        assert_eq!(
            execution_plan(
                &default_local_config(),
                ExecutionRunMode::Rpc,
                &policy(PermissionDecision::Ask),
            ),
            ExecutionPlan::HeadlessAskRefused,
        );
        assert_eq!(
            execution_plan(
                &default_local_config(),
                ExecutionRunMode::Interactive,
                &policy(PermissionDecision::Ask),
            ),
            ExecutionPlan::InteractiveAskRouted,
        );

        let mut routed = default_local_config();
        routed.strategy = ExecutionStrategy::Model;
        assert_eq!(
            execution_plan(
                &routed,
                ExecutionRunMode::Interactive,
                &PermissionPolicy::empty(),
            ),
            ExecutionPlan::GeneralRouted,
        );
    }

    #[test]
    fn external_adapter_protocol_state_is_counted_during_runtime_construction() {
        let (counts, _probe) = construction_probe::install();
        let mut config = default_local_config();
        config.backend = "opi-sandbox".to_string();
        let mut decisions = BTreeMap::new();
        decisions.insert("opi-sandbox".to_string(), PermissionDecision::Allow);
        let policy = PermissionPolicy::from_map(decisions);
        let local_ops: Arc<dyn BashOperations> = Arc::new(LocalBashOperations::new());

        ExecutionRuntime::build(
            &config,
            ExecutionRunMode::Interactive,
            &[EnabledIdentity {
                adapter_id: "opi-sandbox".to_string(),
                package_name: "mock-pkg".to_string(),
            }],
            &policy,
            Arc::new(PanicStore),
            local_ops,
            Path::new("."),
            "x86_64-pc-windows-msvc",
            "0.8.0",
            Arc::new(PermissionManager::new()),
            None,
        )
        .expect("fixed external allow constructs the routed runtime");

        assert_eq!(counts.protocol_states(), 1);
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
            let deadline = host_deadline(timeout).expect("bounded timeout");
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
    fn host_deadline_rejects_overflow() {
        assert!(host_deadline(Duration::MAX).is_err());
    }

    #[test]
    fn routed_operation_context_carries_signal() {
        let result = completed_outcome_to_bash_result(CompletedOutcome {
            ready: crate::execution::ReadyReport {
                selected_protocol: ProtocolId::new(WIRE_IDENTITY).unwrap(),
                implementation: opi_protocol::execution::v1::ImplementationId::new("opi-sandbox")
                    .unwrap(),
                implementation_version: "1.0.0".to_string(),
                target: opi_protocol::execution::v1::TargetId::new("test-target"),
            },
            started: crate::execution::StartedReport::default(),
            exit: None,
            signal: Some(9),
            timed_out: false,
            cancelled: false,
            cleanup: opi_protocol::execution::v1::CleanupState::Confirmed,
            stdout: Vec::new(),
            stderr: Vec::new(),
            diagnostics: Vec::new(),
        });

        assert_eq!(result.context.signal, Some(9));
    }

    #[test]
    fn routed_output_cap_plus_one_is_truncated_and_byte_recoverable() {
        let mut complete = vec![b'a'; crate::tool::MAX_BASH_OUTPUT_BYTES];
        complete.push(b'z');
        let result = completed_outcome_to_bash_result(CompletedOutcome {
            ready: crate::execution::ReadyReport {
                selected_protocol: ProtocolId::new(WIRE_IDENTITY).unwrap(),
                implementation: opi_protocol::execution::v1::ImplementationId::new("opi-sandbox")
                    .unwrap(),
                implementation_version: "1.0.0".to_string(),
                target: opi_protocol::execution::v1::TargetId::new("test-target"),
            },
            started: crate::execution::StartedReport {
                placement: "host".to_string(),
                guarantee: "restricted".to_string(),
                policy: "strict".to_string(),
                limitations: Vec::new(),
            },
            exit: Some(0),
            signal: None,
            timed_out: false,
            cancelled: false,
            cleanup: opi_protocol::execution::v1::CleanupState::Confirmed,
            stdout: complete.clone(),
            stderr: Vec::new(),
            diagnostics: Vec::new(),
        });

        assert_eq!(result.stdout.len(), crate::tool::MAX_BASH_OUTPUT_BYTES);
        assert!(result.context.truncated);
        let path = result
            .context
            .full_output
            .expect("routed truncation must retain complete bytes");
        assert_eq!(std::fs::read(&path).unwrap(), complete);
        std::fs::remove_file(path).unwrap();
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
            assert!(
                err.diagnostics().iter().any(|d| d.code == expected),
                "diagnostic with code {expected:?} must survive the mapping for {expected:?}"
            );
        }
    }

    #[test]
    fn minimal_runtime_with_local_deny_is_policy_denied() {
        // An explicit local deny is outside Branch 1 and fails during fixed-local
        // preflight, even with no enabled identity. The store stays untouched.
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
