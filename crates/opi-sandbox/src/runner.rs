//! The sandbox runner: explicit-input request, owned run handle, lifecycle
//! events, and terminal result.
//!
//! [`SandboxRunner::run`] is a SYNCHRONOUS function: it validates the request,
//! creates the invocation-owned temp root, builds the command from EXPLICIT
//! inputs, calls the restriction seam, configures the process tree, and spawns —
//! attaching the [`TreeGuard`] before returning, with NO `.await` between spawn
//! and guard construction. That ordering invariant (Phase 16 task 16.11.1 audit
//! fold #1) is what guarantees a dropped run cannot release a spawned child with
//! no guard attached.
//!
//! Protocol callers split that same sequence: filesystem validation and
//! restriction preparation run on bounded blocking workers, while
//! the prepared spawn step is called only from the awaiting path.
//! Thus a non-abortable preparation worker that outlives its request owns no
//! spawn operation.
//!
//! [`SandboxRun`] is an owned [`Stream`] of [`SandboxEvent`]. A successful run's
//! first item is [`SandboxEvent::Started`] carrying the invocation-owned
//! temp-root path and the direct-child id, so a dropped-future test can OBSERVE
//! cleanup (fold #7). Launcher-based restrictions withhold that event until an
//! in-profile acknowledgement; rejection completes pre-start without emitting
//! `Started`.
//! Polling the stream drives the supervision to a single terminal
//! [`SandboxEvent::Completed`] (fold #9: single completion, no split handle).
//! Dropping an in-flight [`SandboxRun`] drops the owned child (`kill_on_drop`),
//! the tree guard, and the temp root, so the tree is killed and the temp root is
//! removed on EVERY terminal path (design `### L0 supervision`).

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll};
use std::time::Duration;

use futures_core::Stream;
use opi_protocol::execution::v1::EnvInherit;
use rand::RngCore;
use tokio::process::{Child, Command};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::policy::{
    ContractStatus, LauncherSpec, Mechanism, Restriction, RestrictionCtx, SandboxPolicy,
};
#[cfg(windows)]
use crate::process_tree::resume_child;
use crate::process_tree::{TerminationOutcome, TreeGuard, configure_tree};

/// Bounded per-stream output capture (1 MiB). Output beyond this cap is dropped
/// (the bound is enforced, not exceeded); the captured prefix is returned.
const OUTPUT_CAP: usize = 1024 * 1024;

/// Bounded grace for draining a terminated tree's still-open stdout/stderr pipes
/// (mirrors the Phase 16 task 16.2 `TERMINATED_PIPE_DRAIN_GRACE` invariant).
const PIPE_DRAIN_GRACE: Duration = Duration::from_millis(500);
/// Direct SDK calls expose an execution timeout rather than the protocol's
/// request-wide hard deadline. Once release is armed, compute one cleanup
/// cutoff from the former two sequential 500ms reap/drain allowances.
const DIRECT_CLEANUP_BUDGET: Duration = PIPE_DRAIN_GRACE.saturating_mul(2);

/// Absolute cutoffs for one run. Protocol callers provide the request-wide
/// cleanup deadline separately from the earlier execution cutoff; direct SDK
/// calls compute both once when explicit/auto release is armed and never
/// restart a relative timer.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RunDeadlines {
    start_by: Instant,
    cleanup: Instant,
    execution_timeout: Duration,
}

impl RunDeadlines {
    pub(crate) fn new(start_by: Instant, cleanup: Instant, execution_timeout: Duration) -> Self {
        Self {
            start_by,
            cleanup: cleanup.max(start_by),
            execution_timeout,
        }
    }

    pub(crate) fn start_by(self) -> Instant {
        self.start_by
    }

    pub(crate) fn cleanup(self) -> Instant {
        self.cleanup
    }

    pub(crate) fn execution_deadline_at(self, started: Instant) -> Instant {
        std::cmp::min(
            self.start_by,
            started
                .checked_add(self.execution_timeout)
                .unwrap_or(self.start_by),
        )
    }

    fn spawn_expired(self) -> bool {
        Instant::now() >= self.start_by
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RunDeadlinePlan {
    Fixed(RunDeadlines),
    OnRelease {
        execution_timeout: Duration,
        cleanup_budget: Duration,
    },
}

impl RunDeadlinePlan {
    fn arm_at(self, released: Instant) -> RunDeadlines {
        match self {
            Self::Fixed(deadlines) => deadlines,
            Self::OnRelease {
                execution_timeout,
                cleanup_budget,
            } => {
                let execution = released.checked_add(execution_timeout).unwrap_or(released);
                let cleanup = execution.checked_add(cleanup_budget).unwrap_or(execution);
                RunDeadlines::new(execution, cleanup, execution_timeout)
            }
        }
    }

    fn setup_expired(self, cancel: &CancellationToken) -> bool {
        cancel.is_cancelled() || matches!(self, Self::Fixed(deadlines) if deadlines.spawn_expired())
    }
}

/// Target standard-input policy for one sandboxed run. This is a LOCAL
/// invocation concern, not a protocol field: the `opi-protocol` `ExecutePayload`
/// carries no stdin (a protocol backend's own stdin is the host-to-backend JSONL
/// frame stream), so the SDK caller supplies this field — [`StdinPolicy::Null`]
/// for the protocol backend (16.12) and [`StdinPolicy::Inherit`] for the human
/// direct CLI (spec `### Human CLI`: "Direct `run` inherits terminal stdin by
/// default").
///
/// [`StdinPolicy::Null`] is the safe default: a backend that inherited its own
/// stdin would leak protocol frames to the target. There is deliberately NO
/// `Default` impl so a future caller cannot silently regress to dropping stdin
/// (Phase 16 task 16.11.2 audit fold: stdin-sdk-seam-c1a).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdinPolicy {
    /// The target's stdin is `/dev/null` (immediate EOF). The safe default and
    /// the protocol backend's policy.
    Null,
    /// The target inherits the caller process's stdin. The human direct-CLI
    /// policy.
    Inherit,
}

/// Explicit inputs for one sandboxed run. Mirrors the `opi-protocol`
/// `ExecutePayload` explicit-input set as ergonomic Rust types for the seven
/// ExecutePayload-aligned fields (program, args, workspace, cwd, timeout,
/// environment-inheritance policy, environment additions). There is NO
/// shell-string field: callers pass an explicit program and argument vector
/// (design `### State model`, `#Reuse outside Opi`; Phase 16 task 16.11.1 audit
/// fold #2). Two fields are LOCAL invocation concerns not carried by
/// `ExecutePayload`: [`StdinPolicy`] (the `stdin` field) and the cooperative
/// `cancel` token. The `stdin` field was added by task 16.11.2.
#[derive(Debug, Clone)]
pub struct SandboxRequest {
    /// The explicit program to execute (resolved by the caller; not a shell
    /// expression).
    pub program: PathBuf,
    /// The explicit argument vector.
    pub args: Vec<OsString>,
    /// The canonical workspace root.
    pub workspace: PathBuf,
    /// The working directory inside the workspace.
    pub cwd: PathBuf,
    /// The execution timeout. Must be non-zero.
    pub timeout: Duration,
    /// Environment-inheritance policy. `Clear` starts from an empty environment;
    /// `Inherit` keeps the host process environment. In both cases
    /// `env_additions` are applied on top.
    pub env_inherit: EnvInherit,
    /// Bounded environment additions applied after the inheritance policy.
    pub env_additions: BTreeMap<OsString, OsString>,
    /// Target standard-input policy. A LOCAL invocation concern (the protocol
    /// `ExecutePayload` carries no stdin); see [`StdinPolicy`].
    pub stdin: StdinPolicy,
    /// Optional cooperative cancellation token. When present, firing it resolves
    /// the run to [`SandboxOutcome::Cancelled`]. When absent, cancellation is
    /// exclusively future-drop (which observes no result).
    pub cancel: Option<CancellationToken>,
}

/// A request whose side-effect-free invariants and filesystem roots have been
/// validated. Keeping this token crate-private lets the protocol backend place
/// its `accepted` milestone after validation without duplicating runner logic.
pub(crate) struct ValidatedSandboxRequest {
    request: SandboxRequest,
    workspace: PathBuf,
    cwd: PathBuf,
}

pub(crate) struct StructurallyValidatedRequest {
    request: SandboxRequest,
}

pub(crate) struct PreparedSandboxRun {
    cmd: Command,
    temp: PreparedTemp,
    temp_root: PathBuf,
    release_gate: PathBuf,
    start_probe: Option<StartProbe>,
    cancel: CancellationToken,
    mechanism: Mechanism,
    contract: ContractStatus,
    faults: FaultInjection,
}

struct StartProbe {
    path: PathBuf,
    token: Vec<u8>,
}

impl StartProbe {
    fn new(release_gate: &Path) -> Result<Self, rand::Error> {
        let mut random = [0_u8; 32];
        rand::rngs::OsRng.try_fill_bytes(&mut random)?;
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut token = Vec::with_capacity(random.len() * 2);
        for byte in random {
            token.push(HEX[usize::from(byte >> 4)]);
            token.push(HEX[usize::from(byte & 0x0f)]);
        }
        Ok(Self {
            path: acknowledgement_path(release_gate),
            token,
        })
    }
}

struct PreparedTemp {
    temp: Option<tempfile::TempDir>,
    remove_delay: Duration,
    injected_failure: bool,
}

impl PreparedTemp {
    fn into_temp(mut self) -> tempfile::TempDir {
        self.temp.take().expect("prepared temp present")
    }

    fn close(mut self) -> bool {
        let temp = self.temp.take().expect("prepared temp present");
        if !self.remove_delay.is_zero() {
            std::thread::sleep(self.remove_delay);
        }
        temp.close().is_ok() && !self.injected_failure
    }
}

impl Drop for PreparedTemp {
    fn drop(&mut self) {
        let Some(temp) = self.temp.take() else {
            return;
        };
        if !self.remove_delay.is_zero() {
            std::thread::sleep(self.remove_delay);
        }
        let _ = temp.close();
    }
}

pub(crate) struct SpawnedSandboxRun {
    pub(crate) run: SandboxRun,
    pub(crate) expired: bool,
}

pub(crate) enum SpawnPreparedOutcome {
    Spawned(Box<SpawnedSandboxRun>),
    Expired(Box<PreparedSandboxRun>),
    Failed(SetupFailed),
}

pub(crate) enum StartConfirmationFailure {
    RestrictionSetup { cleanup: CleanupState },
    Deadline,
}

impl PreparedSandboxRun {
    fn setup_expired(&self, deadline_plan: RunDeadlinePlan) -> bool {
        deadline_plan.setup_expired(&self.cancel)
    }

    fn cleanup(self) -> bool {
        let Self { temp, .. } = self;
        temp.close()
    }
}

pub(crate) async fn cleanup_prepared_until(
    prepared: PreparedSandboxRun,
    deadline: Instant,
) -> bool {
    let cleanup = tokio::task::spawn_blocking(move || prepared.cleanup());
    matches!(
        tokio::time::timeout_at(deadline, cleanup).await,
        Ok(Ok(true))
    )
}

impl ValidatedSandboxRequest {
    pub(crate) fn setup_cancel_token(&self) -> CancellationToken {
        self.request.cancel.clone().unwrap_or_default()
    }
}

/// A redacted, closed reason that [`SandboxRunner::run`] failed before the run
/// could be started (Phase 16 task 16.11.1 audit fold: structured setup
/// failure).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SetupFailureReason {
    /// The request was malformed (zero timeout, empty workspace, etc.).
    #[error("invalid request")]
    InvalidRequest,
    /// The program could not be found.
    #[error("program not found")]
    ProgramNotFound,
    /// The restriction seam could not establish the requested contract.
    #[error("restriction setup failed")]
    RestrictionSetup,
    /// Spawning the child failed for a reason other than the program being
    /// absent.
    #[error("spawn failed")]
    SpawnFailed,
    /// The platform does not support the requested contract. (Emitted by later
    /// native tasks; reserved here for forward-compatibility.)
    #[error("unsupported platform")]
    UnsupportedPlatform,
}

/// A pre-spawn setup failure returned by [`SandboxRunner::run`]. The
/// invocation-owned temp root, if one was created, is removed before this is
/// returned (the error path is cleanup-non-vacuous).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("setup failed: {reason}")]
pub struct SetupFailed {
    /// The redacted reason setup failed.
    pub reason: SetupFailureReason,
}

/// Which output stream a [`SandboxEvent::Output`] chunk belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

/// The effective terminal status of a completed run. Unambiguous and structured
/// (design `#Failure and Diagnostics`); exit code and signal are never
/// conflated. Cleanup truth is carried separately by [`CleanupState`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxOutcome {
    /// The target exited with the given code (`None` if the code could not be
    /// determined).
    Exited {
        /// The exit code, if known.
        code: Option<i32>,
    },
    /// The target was terminated by a signal (Unix).
    Signaled {
        /// The signal number.
        signal: i32,
    },
    /// The deadline elapsed; the tree was terminated.
    TimedOut,
    /// A cooperative cancellation token fired; the tree was terminated.
    Cancelled,
}

/// Orthogonal cleanup state, mirroring the protocol `CompletedPayload.cleanup`.
/// Every observed tree-termination, child-reap, pipe-drain, and temp-removal
/// step contributes to this result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupState {
    /// The invocation-owned temp root and child tree were removed.
    Confirmed,
    /// One or more cleanup steps could not be confirmed.
    Unconfirmed,
}

/// The terminal result of one sandboxed run. Carries the structured outcome, the
/// orthogonal cleanup state, the bounded captured stdout/stderr, and the path of
/// the invocation-owned temp root that was removed.
#[derive(Debug, Clone)]
pub struct SandboxResult {
    /// The effective terminal outcome.
    pub outcome: SandboxOutcome,
    /// Whether invocation-owned cleanup completed.
    pub cleanup: CleanupState,
    /// Bounded captured standard output.
    pub stdout: Vec<u8>,
    /// Bounded captured standard error.
    pub stderr: Vec<u8>,
    /// Whether stdout exceeded the capture cap or could not be drained fully.
    pub stdout_truncated: bool,
    /// Whether stderr exceeded the capture cap or could not be drained fully.
    pub stderr_truncated: bool,
    /// The invocation-owned temp root, whether removed or left unconfirmed.
    pub temp_root: PathBuf,
}

/// Lifecycle events streamed by [`SandboxRun`]. A successfully established run
/// emits [`SandboxEvent::Started`] then a single terminal
/// [`SandboxEvent::Completed`]. A rejected launcher completes without a
/// `Started` event; [`SandboxEvent::Output`] and
/// [`SandboxEvent::Diagnostic`] are defined for incremental consumption (used by
/// the binary/protocol layer, task 16.11.2) and are not emitted by the library
/// stream itself (Phase 16 task 16.11.1 audit fold: enumerated variants, no
/// trailing placeholder).
#[derive(Debug, Clone)]
pub enum SandboxEvent {
    /// The target has started. Carries the invocation-owned temp-root path, the
    /// direct-child id, and the EFFECTIVE restriction that was applied — so a
    /// consumer (and a dropped-future test) can observe the run's identity and
    /// the honest effective contract.
    Started {
        /// The invocation-owned temp root, removed at terminal completion or
        /// drop.
        temp_root: PathBuf,
        /// The direct-child process id, if available.
        child_pid: Option<u32>,
        /// The mechanism that was applied.
        mechanism: Mechanism,
        /// The effective contract status after setup.
        contract: ContractStatus,
    },
    /// A captured output chunk. Emitted incrementally by the binary/protocol
    /// layer; not emitted by the library stream (output is buffered into the
    /// terminal [`SandboxResult`]).
    Output {
        /// Which stream this chunk belongs to.
        stream: OutputStream,
        /// The captured bytes.
        bytes: Vec<u8>,
    },
    /// A redacted diagnostic event.
    Diagnostic {
        /// The redacted diagnostic message.
        message: String,
    },
    /// Terminal: the run completed with this result. Emitted exactly once.
    Completed(SandboxResult),
}

/// The sandbox runner: a requested [`SandboxPolicy`] plus the [`Restriction`]
/// seam applied to every run. Construct with [`SandboxRunner::new`].
#[derive(Clone)]
pub struct SandboxRunner {
    policy: SandboxPolicy,
    restriction: Arc<dyn Restriction>,
    faults: FaultInjection,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct FaultInjection {
    pub(crate) attach: bool,
    pub(crate) terminate: bool,
    pub(crate) wait: bool,
    pub(crate) temp: bool,
    pub(crate) spawn_return_delay: Duration,
    pub(crate) temp_remove_delay: Duration,
    pub(crate) terminate_delay: Duration,
    pub(crate) validation_delay: Duration,
    pub(crate) prepared_delivery_delay: Duration,
    pub(crate) prepared_temp_remove_delay: Duration,
    pub(crate) prepared_temp_remove_fail: bool,
    #[cfg(test)]
    pub(crate) post_spawn_gate: Option<&'static PostSpawnGate>,
    #[cfg(test)]
    pub(crate) cancel_cleanup_gate: Option<&'static PostSpawnGate>,
}

#[cfg(test)]
#[derive(Debug)]
/// Coordinates cancellation after spawn + attach without relying on wall-clock
/// scheduling in parallel process-heavy tests.
pub(crate) struct PostSpawnGate {
    rendezvous: std::sync::Barrier,
}

#[cfg(test)]
impl PostSpawnGate {
    pub(crate) fn new() -> Self {
        Self {
            rendezvous: std::sync::Barrier::new(2),
        }
    }

    fn wait_in_runner(&self) {
        self.rendezvous.wait();
        self.rendezvous.wait();
    }

    pub(crate) fn cancel_after_spawn(&self, cancel: &CancellationToken) {
        self.rendezvous.wait();
        cancel.cancel();
        self.rendezvous.wait();
    }

    pub(crate) fn observe_before_cleanup(&self, observe: impl FnOnce()) {
        self.rendezvous.wait();
        observe();
        self.rendezvous.wait();
    }
}

impl SandboxRunner {
    /// Create a runner for `policy` that applies `restriction` to every run.
    pub fn new(policy: SandboxPolicy, restriction: Arc<dyn Restriction>) -> Self {
        Self {
            policy,
            restriction,
            faults: FaultInjection::default(),
        }
    }

    pub(crate) fn with_faults(mut self, faults: FaultInjection) -> Self {
        self.faults = faults;
        self
    }

    /// The configured policy.
    pub fn policy(&self) -> &SandboxPolicy {
        &self.policy
    }

    /// The configured restriction.
    pub fn restriction(&self) -> &dyn Restriction {
        self.restriction.as_ref()
    }

    /// Start one sandboxed run from EXPLICIT inputs.
    ///
    /// This is SYNCHRONOUS: it validates the request, creates the
    /// invocation-owned temp root, builds the command, calls the restriction
    /// seam, configures the process tree, spawns the child, and attaches the
    /// [`TreeGuard`] — all with NO `.await` between spawn and guard — then
    /// returns an owned [`SandboxRun`] whose polling drives the supervision and
    /// whose `Drop` drives cleanup. Requires a running tokio runtime on the
    /// calling thread (polling the stream uses tokio I/O and timers); panics at
    /// runtime if none is active.
    ///
    /// On any setup failure the temp root (if created) is removed before the
    /// `Err` is returned.
    pub fn run(&self, request: SandboxRequest) -> Result<SandboxRun, SetupFailed> {
        let timeout = request.timeout;
        let now = Instant::now();
        let execution = now.checked_add(timeout).ok_or_else(invalid_request)?;
        execution
            .checked_add(DIRECT_CLEANUP_BUDGET)
            .ok_or_else(invalid_request)?;
        let request = self.validate_request(request)?;
        let prepared = self.prepare_validated_until(request, None)?;
        let spawned = self.spawn_prepared(
            prepared,
            RunDeadlinePlan::OnRelease {
                execution_timeout: timeout,
                cleanup_budget: DIRECT_CLEANUP_BUDGET,
            },
        );
        match spawned {
            SpawnPreparedOutcome::Spawned(mut spawned) => {
                if spawned.expired {
                    spawned.run.keep_gated();
                }
                Ok(spawned.run)
            }
            SpawnPreparedOutcome::Expired(prepared) => {
                let _ = prepared.cleanup();
                Err(deadline_setup_failure())
            }
            SpawnPreparedOutcome::Failed(failed) => Err(failed),
        }
    }

    pub(crate) fn validate_request_shape(
        &self,
        request: SandboxRequest,
    ) -> Result<StructurallyValidatedRequest, SetupFailed> {
        if request.timeout.is_zero()
            || request.program.as_os_str().is_empty()
            || request.workspace.as_os_str().is_empty()
            || request.cwd.as_os_str().is_empty()
        {
            return Err(SetupFailed {
                reason: SetupFailureReason::InvalidRequest,
            });
        }
        if request
            .env_additions
            .keys()
            .any(|key| invalid_environment_key(key))
        {
            return Err(invalid_request());
        }
        #[cfg(windows)]
        if request.env_additions.keys().any(|key| {
            key.to_string_lossy()
                .to_ascii_uppercase()
                .starts_with("OPI_SANDBOX_")
        }) {
            return Err(SetupFailed {
                reason: SetupFailureReason::InvalidRequest,
            });
        }
        Ok(StructurallyValidatedRequest { request })
    }

    pub(crate) fn validate_request_filesystem(
        &self,
        validated: StructurallyValidatedRequest,
    ) -> Result<ValidatedSandboxRequest, SetupFailed> {
        let request = validated.request;
        if !self.faults.validation_delay.is_zero() {
            std::thread::sleep(self.faults.validation_delay);
        }
        let workspace = request
            .workspace
            .canonicalize()
            .map_err(|_| invalid_request())?;
        let cwd = request.cwd.canonicalize().map_err(|_| invalid_request())?;
        if !workspace.is_dir() || !cwd.is_dir() || !cwd.starts_with(&workspace) {
            return Err(SetupFailed {
                reason: SetupFailureReason::InvalidRequest,
            });
        }

        Ok(ValidatedSandboxRequest {
            request,
            workspace,
            cwd,
        })
    }

    /// Validate every request invariant that can be checked without creating
    /// an invocation root, installing restrictions, or spawning a process.
    pub(crate) fn validate_request(
        &self,
        request: SandboxRequest,
    ) -> Result<ValidatedSandboxRequest, SetupFailed> {
        let request = self.validate_request_shape(request)?;
        self.validate_request_filesystem(request)
    }

    /// Perform every blocking preparation step without spawning a process.
    /// This is the only runner operation allowed on a background setup worker.
    pub(crate) fn prepare_validated_until(
        &self,
        validated: ValidatedSandboxRequest,
        setup_deadline: Option<Instant>,
    ) -> Result<PreparedSandboxRun, SetupFailed> {
        let ValidatedSandboxRequest {
            request,
            workspace,
            cwd,
        } = validated;
        let setup_cancel = request.cancel.clone().unwrap_or_default();
        if setup_stopped(setup_deadline, &setup_cancel) {
            return Err(deadline_setup_failure());
        }
        let program = resolve_program(
            &request.program,
            &cwd,
            request.env_inherit,
            &request.env_additions,
        )
        .ok_or(SetupFailed {
            reason: SetupFailureReason::ProgramNotFound,
        })?;
        if setup_stopped(setup_deadline, &setup_cancel) {
            return Err(deadline_setup_failure());
        }
        // Create the invocation-owned temp root. Owned by `run` until it is moved
        // into the supervision future; on any error path below it drops and the
        // dir is removed.
        let temp = tempfile::TempDir::new().map_err(|_| SetupFailed {
            reason: SetupFailureReason::SpawnFailed,
        })?;
        let temp_root = temp.path().canonicalize().map_err(|_| SetupFailed {
            reason: SetupFailureReason::SpawnFailed,
        })?;
        let release_gate = temp_root.join("release.armed");
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&release_gate)
            .map_err(|_| SetupFailed {
                reason: SetupFailureReason::SpawnFailed,
            })?;

        let ctx = RestrictionCtx {
            workspace: &workspace,
            temp_root: &temp_root,
            network: self.policy.network,
            setup_deadline,
            setup_cancel: &setup_cancel,
        };

        let launcher = self.restriction.launcher(&ctx).map_err(|_| SetupFailed {
            reason: SetupFailureReason::RestrictionSetup,
        })?;
        if ctx.setup_cancelled() {
            return Err(deadline_setup_failure());
        }

        // Ask the restriction whether the target should be wrapped in a parent
        // program (macOS Seatbelt: `sandbox-exec -p <profile>`). The runner
        // builds the command AROUND the launcher so cwd/stdio/env/process-tree
        // config is then applied IDENTICALLY to the bare-program path.
        // std::process::Command exposes no stdio/kill_on_drop/env_clear getters
        // and no reprogram API, so a launcher cannot be installed later inside
        // `prepare` (a rebuild would drop the runner's piped stdio and env
        // policy) — the launcher spec must be computed before the command is
        // built. `NoRestriction`/Linux return `None` (default) and take the
        // bare path with a `prepare`-driven `pre_exec` hook unchanged.
        let launcher_present = launcher.is_some();
        let start_probe = launcher_present
            .then(|| StartProbe::new(&release_gate))
            .transpose()
            .map_err(|_| SetupFailed {
                reason: SetupFailureReason::RestrictionSetup,
            })?;
        let mut cmd = gated_command(
            &program,
            &request.args,
            &request.env_additions,
            &release_gate,
            start_probe.as_ref().map(|probe| probe.token.as_slice()),
            launcher,
        );
        cmd.current_dir(&cwd)
            .stdin(match request.stdin {
                StdinPolicy::Null => Stdio::null(),
                StdinPolicy::Inherit => Stdio::inherit(),
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        apply_env(
            &mut cmd,
            request.env_inherit,
            &request.env_additions,
            &temp_root,
        );
        #[cfg(windows)]
        apply_windows_bootstrap_env(&mut cmd, &release_gate, &program, &request.args);

        if ctx.setup_cancelled() {
            return Err(deadline_setup_failure());
        }
        let applied = self
            .restriction
            .prepare(&mut cmd, &ctx)
            .map_err(|_| SetupFailed {
                reason: SetupFailureReason::RestrictionSetup,
            })?;
        let contract_is_consistent = match (applied.mechanism, applied.contract) {
            (Mechanism::None, ContractStatus::Unrestricted) => !launcher_present,
            (Mechanism::Landlock | Mechanism::Seccomp, ContractStatus::Restricted) => {
                !launcher_present
            }
            (Mechanism::Seatbelt, ContractStatus::Restricted) => launcher_present,
            _ => false,
        };
        if !contract_is_consistent {
            return Err(SetupFailed {
                reason: SetupFailureReason::RestrictionSetup,
            });
        }

        // Restriction setup is cooperative and may be implemented by external
        // platform code. It must consume this run's original absolute budget.
        if ctx.setup_cancelled() {
            return Err(deadline_setup_failure());
        }

        configure_tree(&mut cmd);

        // A background worker may outlive its caller, so its final operation is
        // this deadline check and return of opaque prepared state. It never owns
        // or invokes the actual spawn operation.
        if ctx.setup_cancelled() {
            return Err(deadline_setup_failure());
        }

        let prepared = PreparedSandboxRun {
            cmd,
            temp: PreparedTemp {
                temp: Some(temp),
                remove_delay: self.faults.prepared_temp_remove_delay,
                injected_failure: self.faults.prepared_temp_remove_fail,
            },
            temp_root,
            release_gate,
            start_probe,
            cancel: request.cancel.unwrap_or_default(),
            mechanism: applied.mechanism,
            contract: applied.contract,
            faults: self.faults,
        };
        if !self.faults.prepared_delivery_delay.is_zero() {
            std::thread::sleep(self.faults.prepared_delivery_delay);
        }
        Ok(prepared)
    }

    /// Spawn and attach a prepared command on the caller's awaiting path. The
    /// child remains behind its release gate even if spawn returns after the
    /// fixed protocol cutoff.
    pub(crate) fn spawn_prepared(
        &self,
        prepared: PreparedSandboxRun,
        deadline_plan: RunDeadlinePlan,
    ) -> SpawnPreparedOutcome {
        if prepared.setup_expired(deadline_plan) {
            return SpawnPreparedOutcome::Expired(Box::new(prepared));
        }
        let PreparedSandboxRun {
            mut cmd,
            temp,
            temp_root,
            release_gate,
            start_probe,
            cancel,
            mechanism,
            contract,
            faults,
        } = prepared;
        let temp = temp.into_temp();
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return SpawnPreparedOutcome::Failed(SetupFailed {
                    reason: SetupFailureReason::ProgramNotFound,
                });
            }
            Err(_) => {
                return SpawnPreparedOutcome::Failed(SetupFailed {
                    reason: SetupFailureReason::SpawnFailed,
                });
            }
        };
        if !faults.spawn_return_delay.is_zero() {
            std::thread::sleep(faults.spawn_return_delay);
        }
        // Spawn and guard are in the same synchronous span: no `.await between`
        // them (Phase 16 task 16.11.1 audit fold #1).
        let child_pid = child.id();
        if faults.attach {
            let _ = child.start_kill();
            return SpawnPreparedOutcome::Failed(SetupFailed {
                reason: SetupFailureReason::SpawnFailed,
            });
        }
        let tree = match TreeGuard::attach_child(child_pid) {
            Ok(tree) => tree,
            Err(_) => {
                let _ = child.start_kill();
                return SpawnPreparedOutcome::Failed(SetupFailed {
                    reason: SetupFailureReason::SpawnFailed,
                });
            }
        };
        #[cfg(windows)]
        let mut tree = tree;
        #[cfg(windows)]
        if child_pid
            .ok_or(SetupFailed {
                reason: SetupFailureReason::SpawnFailed,
            })
            .and_then(|pid| {
                resume_child(pid).map_err(|_| SetupFailed {
                    reason: SetupFailureReason::SpawnFailed,
                })
            })
            .is_err()
        {
            let _ = tree.terminate();
            let _ = child.start_kill();
            return SpawnPreparedOutcome::Failed(SetupFailed {
                reason: SetupFailureReason::SpawnFailed,
            });
        }

        #[cfg(test)]
        if let Some(gate) = faults.post_spawn_gate {
            gate.wait_in_runner();
        }
        let expired = deadline_plan.setup_expired(&cancel);
        let deadline_cell = Arc::new(OnceLock::new());
        if let RunDeadlinePlan::Fixed(deadlines) = deadline_plan {
            let _ = deadline_cell.set(deadlines);
        }
        let inner = Box::pin(supervise(
            child,
            tree,
            temp,
            temp_root.clone(),
            Arc::clone(&deadline_cell),
            cancel.clone(),
            faults,
        ));

        let run = SandboxRun {
            started_emitted: false,
            completed: false,
            auto_release: true,
            temp_root,
            child_pid,
            mechanism,
            contract,
            release_gate: Some(release_gate),
            start_probe,
            start_probe_poll: None,
            start_probe_rejected: false,
            prestart_result: None,
            cancel,
            deadline_plan,
            deadline_cell,
            inner: Some(inner),
        };
        SpawnPreparedOutcome::Spawned(Box::new(SpawnedSandboxRun { run, expired }))
    }
}

fn setup_stopped(deadline: Option<Instant>, cancel: &CancellationToken) -> bool {
    cancel.is_cancelled() || deadline.is_some_and(|deadline| Instant::now() >= deadline)
}

fn invalid_request() -> SetupFailed {
    SetupFailed {
        reason: SetupFailureReason::InvalidRequest,
    }
}

fn deadline_setup_failure() -> SetupFailed {
    SetupFailed {
        reason: SetupFailureReason::SpawnFailed,
    }
}

#[cfg(unix)]
fn invalid_environment_key(key: &std::ffi::OsStr) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let bytes = key.as_bytes();
    bytes.is_empty() || bytes.contains(&b'=') || bytes.contains(&0)
}

#[cfg(windows)]
fn invalid_environment_key(key: &std::ffi::OsStr) -> bool {
    use std::os::windows::ffi::OsStrExt;
    let mut units = key.encode_wide();
    let Some(first) = units.next() else {
        return true;
    };
    first == b'=' as u16 || first == 0 || units.any(|unit| unit == b'=' as u16 || unit == 0)
}

#[cfg(not(any(unix, windows)))]
fn invalid_environment_key(key: &std::ffi::OsStr) -> bool {
    let key = key.to_string_lossy();
    key.is_empty() || key.contains('=') || key.contains('\0')
}

/// Owned handle to one in-flight run: a [`Stream`] of [`SandboxEvent`] whose
/// `Drop` kills the child tree and removes the invocation-owned temp root.
///
/// Poll it (via `futures_util::StreamExt::next`) to drive supervision. The first
/// item of an established run is [`SandboxEvent::Started`]; launcher rejection
/// instead yields a terminal [`SandboxEvent::Completed`] while the target is
/// still gated. Dropping the stream before completion drops the
/// owned supervision future, which drops the child (`kill_on_drop`), the
/// [`TreeGuard`], and the temp root — so the tree is killed and the temp root is
/// removed on every path (success, timeout, cancellation, error, dropped
/// future).
pub struct SandboxRun {
    started_emitted: bool,
    completed: bool,
    auto_release: bool,
    temp_root: PathBuf,
    child_pid: Option<u32>,
    mechanism: Mechanism,
    contract: ContractStatus,
    release_gate: Option<PathBuf>,
    start_probe: Option<StartProbe>,
    start_probe_poll: Option<Pin<Box<tokio::time::Sleep>>>,
    start_probe_rejected: bool,
    prestart_result: Option<SandboxResult>,
    cancel: CancellationToken,
    deadline_plan: RunDeadlinePlan,
    deadline_cell: Arc<OnceLock<RunDeadlines>>,
    inner: Option<Pin<Box<dyn std::future::Future<Output = SandboxResult> + Send>>>,
}

impl SandboxRun {
    /// The invocation-owned temp root that will be removed at terminal
    /// completion or drop.
    pub fn temp_root(&self) -> &Path {
        &self.temp_root
    }

    /// The direct-child process id, if available.
    pub fn child_pid(&self) -> Option<u32> {
        self.child_pid
    }

    /// Keep the target behind its release gate while cancellation drives the
    /// supervision future. Used by the protocol backend when its absolute
    /// cutoff expires after `Started` publication but before target release.
    pub(crate) fn keep_gated(&mut self) {
        self.auto_release = false;
    }

    fn arm_execution(&self) -> RunDeadlines {
        *self
            .deadline_cell
            .get_or_init(|| self.deadline_plan.arm_at(Instant::now()))
    }

    fn poll_start_confirmation(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), StartConfirmationFailure>> {
        if self.start_probe.is_none() {
            return Poll::Ready(Ok(()));
        }
        if let Some(result) = &self.prestart_result {
            return Poll::Ready(Err(start_confirmation_failure(
                result,
                self.start_probe_rejected,
            )));
        }
        if let Some(failure) = self.poll_prestart_completion(cx) {
            return Poll::Ready(Err(failure));
        }
        match verify_start_probe(self.start_probe.as_ref().expect("start probe present")) {
            ProbeStatus::Valid => {
                // Exit/rejection wins when the proof and child completion become
                // observable in the same turn. The bootstrap remains gated, so
                // a live accepted launcher must still be pending here.
                if let Some(failure) = self.poll_prestart_completion(cx) {
                    return Poll::Ready(Err(failure));
                }
                self.start_probe = None;
                self.start_probe_poll = None;
                return Poll::Ready(Ok(()));
            }
            ProbeStatus::Invalid => {
                self.start_probe_rejected = true;
                self.auto_release = false;
                self.cancel.cancel();
                if let Some(failure) = self.poll_prestart_completion(cx) {
                    return Poll::Ready(Err(failure));
                }
            }
            ProbeStatus::Missing => {}
        }
        let poll = self
            .start_probe_poll
            .get_or_insert_with(|| Box::pin(tokio::time::sleep(Duration::from_millis(5))));
        if poll.as_mut().poll(cx).is_ready() {
            self.start_probe_poll = None;
            cx.waker().wake_by_ref();
        }
        Poll::Pending
    }

    fn poll_prestart_completion(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Option<StartConfirmationFailure> {
        let result = match self
            .inner
            .as_mut()
            .expect("inner present before start confirmation")
            .as_mut()
            .poll(cx)
        {
            Poll::Pending => return None,
            Poll::Ready(result) => result,
        };
        self.inner = None;
        let failure = start_confirmation_failure(&result, self.start_probe_rejected);
        self.prestart_result = Some(result);
        Some(failure)
    }

    pub(crate) async fn confirm_start_until(
        &mut self,
        start_deadline: Instant,
        cleanup_deadline: Instant,
    ) -> Result<(), StartConfirmationFailure> {
        self.arm_execution();
        match tokio::time::timeout_at(
            start_deadline,
            std::future::poll_fn(|cx| self.poll_start_confirmation(cx)),
        )
        .await
        {
            Ok(result) => result,
            Err(_) if self.start_probe_rejected => tokio::time::timeout_at(
                cleanup_deadline,
                std::future::poll_fn(|cx| self.poll_start_confirmation(cx)),
            )
            .await
            .unwrap_or(Err(StartConfirmationFailure::RestrictionSetup {
                cleanup: CleanupState::Unconfirmed,
            })),
            Err(_) => Err(StartConfirmationFailure::Deadline),
        }
    }

    /// Release the real target after the caller has observed and published the
    /// [`SandboxEvent::Started`] contract. Idempotent.
    ///
    /// Cancellation and filesystem removal cannot form one cross-primitive
    /// transaction. The final cancellation observation immediately before
    /// `remove_file` is therefore the release linearization point: cancellation
    /// visible there wins and permanently keeps the gate; a cancellation that
    /// races after that point is ordered after release and supervision still
    /// terminates the target. Both explicit and automatic release use this one
    /// arbitration path.
    pub fn release(&mut self) -> io::Result<()> {
        if self.release_gate.is_none() {
            return Ok(());
        }
        let deadline_expired = Instant::now() >= self.arm_execution().start_by();
        let release_gate = self
            .release_gate
            .take()
            .expect("release gate checked above");
        if self.cancel.is_cancelled() {
            self.auto_release = false;
            self.release_gate = Some(release_gate);
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "execution cancelled before release",
            ));
        }
        if deadline_expired {
            self.auto_release = false;
            self.release_gate = Some(release_gate);
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "execution deadline elapsed before release",
            ));
        }
        match std::fs::remove_file(&release_gate) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => {
                self.release_gate = Some(release_gate);
                Err(error)
            }
        }
    }
}

impl Stream for SandboxRun {
    type Item = SandboxEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if !self.started_emitted {
            self.arm_execution();
            match self.poll_start_confirmation(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(_)) => {
                    let mut result = self
                        .prestart_result
                        .take()
                        .expect("failed start confirmation has a terminal result");
                    if !matches!(
                        result.outcome,
                        SandboxOutcome::TimedOut | SandboxOutcome::Cancelled
                    ) {
                        result.outcome = SandboxOutcome::Exited { code: Some(125) };
                    }
                    self.completed = true;
                    return Poll::Ready(Some(SandboxEvent::Completed(result)));
                }
            }
            self.started_emitted = true;
            return Poll::Ready(Some(SandboxEvent::Started {
                temp_root: self.temp_root.clone(),
                child_pid: self.child_pid,
                mechanism: self.mechanism,
                contract: self.contract,
            }));
        }
        if self.completed {
            return Poll::Ready(None);
        }
        if self.auto_release {
            let _ = self.release();
        }
        // Unpin: all fields are Unpin (Pin<Box<Future>> is Unpin). The scrutinee
        // borrow of `self.inner` ends before the `Ready` arm assigns it.
        match self
            .inner
            .as_mut()
            .expect("inner present until completion")
            .as_mut()
            .poll(cx)
        {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                self.completed = true;
                self.inner = None;
                Poll::Ready(Some(SandboxEvent::Completed(result)))
            }
        }
    }
}

fn acknowledgement_path(release_gate: &Path) -> PathBuf {
    let mut native = release_gate.as_os_str().to_os_string();
    native.push(".probe");
    PathBuf::from(native)
}

enum ProbeStatus {
    Missing,
    Valid,
    Invalid,
}

fn verify_start_probe(probe: &StartProbe) -> ProbeStatus {
    let metadata = match std::fs::symlink_metadata(&probe.path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return ProbeStatus::Missing,
        Err(_) => return ProbeStatus::Invalid,
    };
    if !metadata.file_type().is_file() {
        return ProbeStatus::Invalid;
    }
    let file = match open_probe_no_follow(&probe.path) {
        Ok(file) => file,
        Err(_) => return ProbeStatus::Invalid,
    };
    if !file
        .metadata()
        .is_ok_and(|metadata| metadata.file_type().is_file())
    {
        return ProbeStatus::Invalid;
    }
    let mut content = Vec::with_capacity(probe.token.len() + 1);
    if file
        .take((probe.token.len() + 1) as u64)
        .read_to_end(&mut content)
        .is_err()
    {
        return ProbeStatus::Invalid;
    }
    if content == probe.token {
        ProbeStatus::Valid
    } else {
        ProbeStatus::Invalid
    }
}

#[cfg(unix)]
fn open_probe_no_follow(path: &Path) -> io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;

    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)
}

#[cfg(windows)]
fn open_probe_no_follow(path: &Path) -> io::Result<std::fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;

    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
}

#[cfg(not(any(unix, windows)))]
fn open_probe_no_follow(path: &Path) -> io::Result<std::fs::File> {
    std::fs::OpenOptions::new().read(true).open(path)
}

fn start_confirmation_failure(
    result: &SandboxResult,
    probe_rejected: bool,
) -> StartConfirmationFailure {
    if !probe_rejected
        && matches!(
            result.outcome,
            SandboxOutcome::TimedOut | SandboxOutcome::Cancelled
        )
    {
        StartConfirmationFailure::Deadline
    } else {
        StartConfirmationFailure::RestrictionSetup {
            cleanup: result.cleanup,
        }
    }
}

// `SandboxRun` owns the supervision future, which owns the child/tree/temp
// guards. When `SandboxRun` drops before completion, `inner` (still `Some`)
// drops, dropping those guards: child `kill_on_drop`, `TreeGuard` terminate,
// temp-root removal. No explicit `Drop` body is needed beyond the field's own
// drop, but asserting the invariant here documents the contract.
// (When `inner` is already `None` — the run completed — cleanup already ran
// inside the supervision future before it returned.)

/// Apply the environment-inheritance policy and bounded additions to `cmd`.
fn apply_env(
    cmd: &mut Command,
    inherit: EnvInherit,
    additions: &BTreeMap<OsString, OsString>,
    temp_root: &Path,
) {
    if matches!(inherit, EnvInherit::Clear) {
        cmd.env_clear();
    }
    for (key, value) in additions {
        cmd.env(key, value);
    }
    cmd.env("TMPDIR", temp_root)
        .env("TMP", temp_root)
        .env("TEMP", temp_root);
}

#[cfg(windows)]
fn apply_windows_bootstrap_env(
    cmd: &mut Command,
    release_gate: &Path,
    program: &Path,
    args: &[OsString],
) {
    cmd.env("OPI_SANDBOX_RELEASE_GATE", release_gate)
        .env("OPI_SANDBOX_TARGET_PROGRAM", program)
        .env("OPI_SANDBOX_TARGET_ARG_COUNT", args.len().to_string())
        .env("OPI_SANDBOX_BACKEND_PID", std::process::id().to_string());
    for (index, argument) in args.iter().enumerate() {
        cmd.env(format!("OPI_SANDBOX_TARGET_ARG_{index}"), argument);
    }
}

/// Build a platform-native bootstrap that waits on the invocation-owned release
/// gate before it invokes the real target. A restriction launcher, when present,
/// remains the outermost process so it confines the bootstrap and target alike.
fn gated_command(
    program: &Path,
    args: &[OsString],
    env_additions: &BTreeMap<OsString, OsString>,
    release_gate: &Path,
    start_token: Option<&[u8]>,
    launcher: Option<LauncherSpec>,
) -> Command {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        // Keep bootstrap state in positional parameters. Assigning generic
        // shell variables would overwrite same-named exported request values.
        const SCRIPT: &str = r#"if [ -n "$1" ]; then
  (umask 077; set -C; printf '%s' "$1" > "$2.probe.tmp.$$") || exit 125
  /bin/ln "$2.probe.tmp.$$" "$2.probe" || { /bin/rm -f "$2.probe.tmp.$$"; exit 125; }
  /bin/rm -f "$2.probe.tmp.$$"
fi
shift
while [ -e "$1" ]; do
  kill -0 "$2" 2>&- || exit 125
  /bin/sleep 0.01
done
(
  while kill -0 "$2" 2>&- && kill -0 "$$" 2>&-; do
    /bin/sleep 0.05
  done
  if ! kill -0 "$2" 2>&-; then
    kill -KILL "-$$" 2>&-
  fi
) &
shift 2
if [ "$1" = restore-native-env ]; then
  shift
  exec /usr/bin/env -- "$@"
fi
[ "$1" = direct ] || exit 125
shift
exec "$@""#;

        let native_env = env_additions
            .iter()
            .filter(|(key, _)| {
                let bytes = key.as_os_str().as_bytes();
                !matches!(bytes.first(), Some(b'A'..=b'Z' | b'a'..=b'z' | b'_'))
                    || bytes[1..]
                        .iter()
                        .any(|byte| !byte.is_ascii_alphanumeric() && *byte != b'_')
            })
            .map(|(key, value)| {
                let mut assignment = key.as_os_str().as_bytes().to_vec();
                assignment.push(b'=');
                assignment.extend_from_slice(value.as_os_str().as_bytes());
                OsString::from_vec(assignment)
            })
            .collect::<Vec<_>>();

        let mut command = match launcher {
            Some(spec) => {
                let mut command = Command::new(spec.program);
                command.args(spec.prefix);
                command.arg("/bin/sh");
                command
            }
            None => Command::new("/bin/sh"),
        };
        command
            .arg("-c")
            .arg(SCRIPT)
            .arg("opi-sandbox-release-gate")
            .arg(OsString::from_vec(start_token.unwrap_or_default().to_vec()))
            .arg(release_gate)
            .arg(std::process::id().to_string());
        if native_env.is_empty() {
            command.arg("direct").arg(program).args(args);
        } else {
            // POSIX shells discard inherited environment names that are not
            // shell identifiers. Restore those byte-preserving additions with
            // `env`, then use a fixed native utility to remove command-name
            // ambiguity before it execs the target after `--`.
            command
                .arg("restore-native-env")
                .args(native_env)
                .args(["/usr/bin/nice", "-n", "0", "--"])
                .arg(program)
                .args(args);
        }
        command
    }
    #[cfg(windows)]
    {
        let _ = (env_additions, launcher, start_token);
        const SCRIPT: &str = r#"$gate = $env:OPI_SANDBOX_RELEASE_GATE
$program = $env:OPI_SANDBOX_TARGET_PROGRAM
$count = [int]$env:OPI_SANDBOX_TARGET_ARG_COUNT
$backendPid = [int]$env:OPI_SANDBOX_BACKEND_PID
$rest = @()
for ($i = 0; $i -lt $count; $i++) {
  $rest += [Environment]::GetEnvironmentVariable("OPI_SANDBOX_TARGET_ARG_$i")
}
while (Test-Path -LiteralPath $gate) {
  if ($null -eq (Get-Process -Id $backendPid -ErrorAction SilentlyContinue)) { exit 125 }
  Start-Sleep -Milliseconds 10
}
& $program @rest
exit $LASTEXITCODE"#;
        let mut command = Command::new("powershell");
        command
            .arg("-NoProfile")
            .arg("-Command")
            .arg(SCRIPT)
            .env("OPI_SANDBOX_RELEASE_GATE", release_gate)
            .env("OPI_SANDBOX_TARGET_PROGRAM", program)
            .env("OPI_SANDBOX_TARGET_ARG_COUNT", args.len().to_string())
            .env("OPI_SANDBOX_BACKEND_PID", std::process::id().to_string());
        for (index, argument) in args.iter().enumerate() {
            command.env(format!("OPI_SANDBOX_TARGET_ARG_{index}"), argument);
        }
        command
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (env_additions, release_gate, launcher, start_token);
        let mut command = Command::new(program);
        command.args(args);
        command
    }
}

fn resolve_program(
    program: &Path,
    cwd: &Path,
    inherit: EnvInherit,
    additions: &BTreeMap<OsString, OsString>,
) -> Option<PathBuf> {
    let has_path = program.is_absolute() || program.components().count() > 1;
    if has_path {
        let candidate = if program.is_absolute() {
            program.to_path_buf()
        } else {
            cwd.join(program)
        };
        return candidate.is_file().then_some(candidate);
    }

    #[cfg(windows)]
    {
        let direct = cwd.join(program);
        if direct.is_file() {
            return Some(direct);
        }
        let mut executable = program.as_os_str().to_os_string();
        executable.push(".exe");
        let direct_executable = cwd.join(executable);
        if direct_executable.is_file() {
            return Some(direct_executable);
        }
        let path = additions
            .iter()
            .find(|(key, _)| key.to_string_lossy().eq_ignore_ascii_case("PATH"))
            .map(|(_, value)| value.clone())
            .or_else(|| {
                matches!(inherit, EnvInherit::Inherit)
                    .then(|| std::env::var_os("PATH"))
                    .flatten()
            })?;
        std::env::split_paths(&path).find_map(|directory| {
            let base = if directory.as_os_str().is_empty() {
                cwd.to_path_buf()
            } else if directory.is_absolute() {
                directory
            } else {
                cwd.join(directory)
            };
            let candidate = base.join(program);
            if candidate.is_file() {
                return Some(candidate);
            }
            if program.extension().is_none() {
                let mut executable = program.as_os_str().to_os_string();
                executable.push(".exe");
                let candidate = base.join(executable);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
            None
        })
    }

    #[cfg(unix)]
    {
        let path = additions
            .get(std::ffi::OsStr::new("PATH"))
            .cloned()
            .or_else(|| {
                matches!(inherit, EnvInherit::Inherit)
                    .then(|| std::env::var_os("PATH"))
                    .flatten()
            })
            .unwrap_or_else(|| OsString::from("/usr/bin:/bin"));
        std::env::split_paths(&path).find_map(|directory| {
            let base = if directory.as_os_str().is_empty() {
                cwd.to_path_buf()
            } else if directory.is_absolute() {
                directory
            } else {
                cwd.join(directory)
            };
            let candidate = base.join(program);
            candidate.is_file().then_some(candidate)
        })
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = (cwd, inherit, additions);
        Some(program.to_path_buf())
    }
}

/// Map a raw exit status to the structured outcome. Signal termination (Unix)
/// is distinguished from a normal exit code.
fn status_to_outcome(status: std::process::ExitStatus) -> SandboxOutcome {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return SandboxOutcome::Signaled { signal };
        }
    }
    SandboxOutcome::Exited {
        code: status.code(),
    }
}

/// Supervise one spawned child through the bounded L0 lifecycle. Owns the child,
/// the tree guard, and the invocation-owned temp root for its whole body, so
/// dropping the future (a dropped run) terminates the tree and removes the temp
/// root on every path.
async fn supervise(
    mut child: Child,
    mut tree: TreeGuard,
    temp: tempfile::TempDir,
    temp_root: PathBuf,
    deadline_cell: Arc<OnceLock<RunDeadlines>>,
    cancel: CancellationToken,
    faults: FaultInjection,
) -> SandboxResult {
    let deadlines = *deadline_cell
        .get()
        .expect("execution deadline armed before supervision");
    let execution_deadline = deadlines.execution_deadline_at(Instant::now());
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let mut drain_out = CaptureTask::new(stdout, OUTPUT_CAP);
    let mut drain_err = CaptureTask::new(stderr, OUTPUT_CAP);
    let mut cleanup_confirmed = true;

    // Race wait / timeout / cancellation. On every branch the whole tree is
    // terminated; biased ordering (cancel > timeout > wait) classifies
    // simultaneous trips deterministically.
    let outcome = tokio::select! {
        biased;
        _ = cancel.cancelled() => {
            #[cfg(test)]
            if let Some(gate) = faults.cancel_cleanup_gate {
                gate.wait_in_runner();
            }
            cleanup_confirmed &= terminate_tree(&mut tree, faults.terminate);
            let _ = child.start_kill();
            cleanup_confirmed &= reap_child(&mut child, deadlines.cleanup).await;
            SandboxOutcome::Cancelled
        }
        _ = tokio::time::sleep_until(execution_deadline) => {
            if !faults.terminate_delay.is_zero() {
                std::thread::sleep(faults.terminate_delay);
            }
            cleanup_confirmed &= terminate_tree(&mut tree, faults.terminate);
            let _ = child.start_kill();
            cleanup_confirmed &= reap_child(&mut child, deadlines.cleanup).await;
            SandboxOutcome::TimedOut
        }
        status = child.wait() => match status {
            Ok(status) => {
                cleanup_confirmed &= !faults.wait;
                cleanup_confirmed &= terminate_tree(&mut tree, faults.terminate);
                status_to_outcome(status)
            }
            Err(_) => {
                cleanup_confirmed = false;
                cleanup_confirmed &= terminate_tree(&mut tree, faults.terminate);
                SandboxOutcome::Exited { code: None }
            }
        },
    };

    // Finish both drains under a bounded grace. On grace expiry the inner future
    // is dropped, which drops the CaptureTasks (aborting their tasks) and we
    // return what was captured so far (empty).
    match tokio::time::timeout_at(deadlines.cleanup, async {
        tokio::join!(drain_out.wait(), drain_err.wait())
    })
    .await
    {
        Ok((out_ok, err_ok)) => cleanup_confirmed &= out_ok && err_ok,
        Err(_) => {
            cleanup_confirmed = false;
            drain_out.abort_incomplete();
            drain_err.abort_incomplete();
        }
    }
    let out = drain_out.snapshot();
    let err = drain_err.snapshot();

    if !remove_temp_root_until(temp, deadlines.cleanup, faults.temp_remove_delay).await
        || faults.temp
    {
        cleanup_confirmed = false;
    }

    // Every observed termination/reap/drain/temp-removal step contributes to
    // the reported cleanup state. The remaining guards still provide a final
    // best-effort kill on drop when an earlier step failed.
    SandboxResult {
        outcome,
        cleanup: if cleanup_confirmed {
            CleanupState::Confirmed
        } else {
            CleanupState::Unconfirmed
        },
        stdout: out.bytes,
        stderr: err.bytes,
        stdout_truncated: out.truncated,
        stderr_truncated: err.truncated,
        temp_root,
    }
}

async fn remove_temp_root_until(
    temp: tempfile::TempDir,
    deadline: Instant,
    injected_delay: Duration,
) -> bool {
    let removal = tokio::task::spawn_blocking(move || {
        if !injected_delay.is_zero() {
            std::thread::sleep(injected_delay);
        }
        temp.close().is_ok()
    });
    matches!(
        tokio::time::timeout_at(deadline, removal).await,
        Ok(Ok(true))
    )
}

fn terminate_tree(tree: &mut TreeGuard, injected_failure: bool) -> bool {
    let confirmed = !matches!(tree.terminate(), TerminationOutcome::Failed(_));
    confirmed && !injected_failure
}

async fn reap_child(child: &mut Child, deadline: Instant) -> bool {
    matches!(
        tokio::time::timeout_at(deadline, child.wait()).await,
        Ok(Ok(_))
    )
}

/// Owned handle to one stream's drain task. The task reads the pipe into a
/// `Vec<u8>` bounded by `cap`; `finish` awaits the capture, and `Drop` aborts an
/// unfinished task so a descendant holding a pipe cannot outlive the run.
struct CaptureTask {
    handle: Option<tokio::task::JoinHandle<()>>,
    state: Arc<Mutex<CaptureState>>,
}

#[derive(Default)]
struct CaptureState {
    bytes: Vec<u8>,
    truncated: bool,
}

struct CaptureSnapshot {
    bytes: Vec<u8>,
    truncated: bool,
}

impl CaptureTask {
    /// Spawn the drain for `stream`, capturing up to `cap` bytes.
    fn new<R>(stream: Option<R>, cap: usize) -> Self
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
    {
        let state = Arc::new(Mutex::new(CaptureState::default()));
        let task_state = Arc::clone(&state);
        let handle = tokio::spawn(async move {
            if let Some(mut stream) = stream {
                use tokio::io::AsyncReadExt;
                let mut chunk = [0u8; 8192];
                loop {
                    match stream.read(&mut chunk).await {
                        Ok(0) => break,
                        Err(_) => {
                            lock_capture(&task_state).truncated = true;
                            break;
                        }
                        Ok(n) => {
                            let mut state = lock_capture(&task_state);
                            if state.bytes.len() < cap {
                                let take = std::cmp::min(n, cap - state.bytes.len());
                                state.bytes.extend_from_slice(&chunk[..take]);
                                state.truncated |= take < n;
                            } else {
                                state.truncated = true;
                            }
                        }
                    }
                }
            }
        });
        Self {
            handle: Some(handle),
            state,
        }
    }

    /// Await the capture while keeping ownership so a cancelled wait can abort.
    async fn wait(&mut self) -> bool {
        let Some(handle) = self.handle.as_mut() else {
            return true;
        };
        let completed = handle.await.is_ok();
        self.handle = None;
        completed
    }

    fn abort_incomplete(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
            lock_capture(&self.state).truncated = true;
        }
    }

    fn snapshot(&self) -> CaptureSnapshot {
        let state = lock_capture(&self.state);
        CaptureSnapshot {
            bytes: state.bytes.clone(),
            truncated: state.truncated,
        }
    }
}

impl Drop for CaptureTask {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

fn lock_capture(state: &Mutex<CaptureState>) -> std::sync::MutexGuard<'_, CaptureState> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_START_TOKEN: &[u8] = b"test-start-token-0123456789abcdef";

    #[cfg(unix)]
    struct PassThroughLauncher;

    #[cfg(unix)]
    impl Restriction for PassThroughLauncher {
        fn launcher(
            &self,
            _ctx: &RestrictionCtx<'_>,
        ) -> Result<Option<LauncherSpec>, crate::policy::RestrictionSetupError> {
            Ok(Some(LauncherSpec {
                program: PathBuf::from("/usr/bin/env"),
                prefix: Vec::new(),
            }))
        }

        fn prepare(
            &self,
            _cmd: &mut Command,
            _ctx: &RestrictionCtx<'_>,
        ) -> Result<crate::policy::AppliedRestriction, crate::policy::RestrictionSetupError>
        {
            Ok(crate::policy::AppliedRestriction {
                mechanism: Mechanism::Seatbelt,
                contract: ContractStatus::Restricted,
            })
        }
    }

    fn fake_seatbelt_run(script: &str) -> (SandboxRun, PathBuf, PathBuf) {
        let temp = tempfile::tempdir().expect("temp root");
        let temp_root = temp.path().to_path_buf();
        let release_gate = temp_root.join("release.armed");
        std::fs::write(&release_gate, b"").expect("create release gate");
        let probe = acknowledgement_path(&release_gate);
        let marker_root = tempfile::tempdir().expect("marker root").keep();
        let marker = marker_root.join("released");
        let mut cmd = if cfg!(windows) {
            let mut cmd = Command::new("powershell");
            cmd.args(["-NoProfile", "-Command", script]);
            cmd
        } else {
            let mut cmd = Command::new("sh");
            cmd.args(["-c", script]);
            cmd
        };
        cmd.env("OPI_TEST_GATE", &release_gate)
            .env("OPI_TEST_PROBE", &probe)
            .env(
                "OPI_TEST_TOKEN",
                std::str::from_utf8(TEST_START_TOKEN).expect("ASCII test token"),
            )
            .env("OPI_TEST_MARKER", &marker)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        configure_tree(&mut cmd);
        let child = cmd.spawn().expect("spawn fake launcher");
        let child_pid = child.id();
        let tree = TreeGuard::attach_child(child_pid).expect("attach fake launcher");
        #[cfg(windows)]
        let tree = {
            let tree = tree;
            resume_child(child_pid.expect("child pid")).expect("resume fake launcher");
            tree
        };
        let now = Instant::now();
        let deadlines = RunDeadlines::new(
            now + Duration::from_secs(3),
            now + Duration::from_secs(4),
            Duration::from_secs(3),
        );
        let deadline_cell = Arc::new(OnceLock::new());
        deadline_cell.set(deadlines).expect("set deadlines");
        let cancel = CancellationToken::new();
        let inner = Box::pin(supervise(
            child,
            tree,
            temp,
            temp_root.clone(),
            Arc::clone(&deadline_cell),
            cancel.clone(),
            FaultInjection::default(),
        ));
        let run = SandboxRun {
            started_emitted: false,
            completed: false,
            auto_release: true,
            temp_root,
            child_pid,
            mechanism: Mechanism::Seatbelt,
            contract: ContractStatus::Restricted,
            release_gate: Some(release_gate),
            start_probe: Some(StartProbe {
                path: probe.clone(),
                token: TEST_START_TOKEN.to_vec(),
            }),
            start_probe_poll: None,
            start_probe_rejected: false,
            prestart_result: None,
            cancel,
            deadline_plan: RunDeadlinePlan::Fixed(deadlines),
            deadline_cell,
            inner: Some(inner),
        };
        (run, probe, marker)
    }

    async fn next(run: &mut SandboxRun) -> Option<SandboxEvent> {
        std::future::poll_fn(|cx| Pin::new(&mut *run).poll_next(cx)).await
    }

    struct ProbeAndExitTogether {
        probe: PathBuf,
        token: Vec<u8>,
        first_poll: bool,
        result: Option<SandboxResult>,
    }

    impl std::future::Future for ProbeAndExitTogether {
        type Output = SandboxResult;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.first_poll {
                self.first_poll = false;
                std::fs::write(&self.probe, &self.token).expect("write simultaneous probe");
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            Poll::Ready(self.result.take().expect("one terminal result"))
        }
    }

    #[tokio::test]
    async fn launcher_exit_wins_when_probe_and_exit_become_ready_together() {
        let temp = tempfile::tempdir().expect("temp root");
        let temp_root = temp.path().to_path_buf();
        let release_gate = temp_root.join("release.armed");
        std::fs::write(&release_gate, b"").expect("create release gate");
        let probe = acknowledgement_path(&release_gate);
        let now = Instant::now();
        let deadlines = RunDeadlines::new(
            now + Duration::from_secs(3),
            now + Duration::from_secs(4),
            Duration::from_secs(3),
        );
        let deadline_cell = Arc::new(OnceLock::new());
        deadline_cell.set(deadlines).expect("set deadlines");
        let result = SandboxResult {
            outcome: SandboxOutcome::Exited { code: Some(68) },
            cleanup: CleanupState::Confirmed,
            stdout: Vec::new(),
            stderr: Vec::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            temp_root: temp_root.clone(),
        };
        let mut run = SandboxRun {
            started_emitted: false,
            completed: false,
            auto_release: true,
            temp_root,
            child_pid: None,
            mechanism: Mechanism::Seatbelt,
            contract: ContractStatus::Restricted,
            release_gate: Some(release_gate),
            start_probe: Some(StartProbe {
                path: probe.clone(),
                token: TEST_START_TOKEN.to_vec(),
            }),
            start_probe_poll: None,
            start_probe_rejected: false,
            prestart_result: None,
            cancel: CancellationToken::new(),
            deadline_plan: RunDeadlinePlan::Fixed(deadlines),
            deadline_cell,
            inner: Some(Box::pin(ProbeAndExitTogether {
                probe,
                token: TEST_START_TOKEN.to_vec(),
                first_poll: true,
                result: Some(result),
            })),
        };

        let event = next(&mut run).await;

        assert!(matches!(event, Some(SandboxEvent::Completed(_))));
    }

    #[tokio::test]
    async fn fake_profile_rejection_emits_no_started_event() {
        let (mut run, probe, marker) = fake_seatbelt_run("exit 65");

        let event = next(&mut run).await;

        assert!(matches!(event, Some(SandboxEvent::Completed(_))));
        assert!(!probe.exists(), "rejected profile emitted no proof");
        assert!(!marker.exists(), "rejected launcher never released target");
    }

    #[tokio::test]
    async fn fake_launcher_early_exit_emits_no_started_event() {
        let script = if cfg!(windows) {
            "Start-Sleep -Milliseconds 50; exit 66"
        } else {
            "sleep 0.05; exit 66"
        };
        let (mut run, probe, marker) = fake_seatbelt_run(script);

        let event = next(&mut run).await;

        assert!(matches!(event, Some(SandboxEvent::Completed(_))));
        assert!(!probe.exists(), "early launcher exit emitted no proof");
        assert!(
            !marker.exists(),
            "early launcher exit never released target"
        );
    }

    #[tokio::test]
    async fn fake_launcher_rejection_classifies_as_prestart_restriction_setup() {
        let (mut run, _probe, marker) = fake_seatbelt_run("exit 67");

        let failure = run
            .confirm_start_until(
                Instant::now() + Duration::from_secs(2),
                Instant::now() + Duration::from_secs(3),
            )
            .await
            .expect_err("missing acknowledgement is a pre-start failure");

        assert!(matches!(
            failure,
            StartConfirmationFailure::RestrictionSetup {
                cleanup: CleanupState::Confirmed
            }
        ));
        assert!(!marker.exists(), "restriction failure keeps target gated");
    }

    #[tokio::test]
    async fn forged_probe_content_is_rejected_immediately() {
        let script = if cfg!(windows) {
            "$tmp = \"$env:OPI_TEST_PROBE.tmp\"; Set-Content -NoNewline -LiteralPath $tmp -Value forged; Move-Item -LiteralPath $tmp -Destination $env:OPI_TEST_PROBE; while (Test-Path -LiteralPath $env:OPI_TEST_GATE) { Start-Sleep -Milliseconds 10 }"
        } else {
            "tmp=\"${OPI_TEST_PROBE}.tmp\"; printf forged > \"$tmp\"; ln \"$tmp\" \"$OPI_TEST_PROBE\"; rm -f \"$tmp\"; while [ -e \"$OPI_TEST_GATE\" ]; do sleep 0.01; done"
        };
        let (mut run, _probe, marker) = fake_seatbelt_run(script);

        let event = tokio::time::timeout(Duration::from_millis(500), next(&mut run))
            .await
            .expect("wrong probe content must fail without waiting for the deadline");

        assert!(matches!(event, Some(SandboxEvent::Completed(_))));
        assert!(!marker.exists(), "forged proof never releases the target");
    }

    #[tokio::test]
    async fn non_regular_probe_is_rejected_immediately() {
        let script = if cfg!(windows) {
            "New-Item -ItemType Directory -Path $env:OPI_TEST_PROBE | Out-Null; while (Test-Path -LiteralPath $env:OPI_TEST_GATE) { Start-Sleep -Milliseconds 10 }"
        } else {
            "mkdir \"$OPI_TEST_PROBE\"; while [ -e \"$OPI_TEST_GATE\" ]; do sleep 0.01; done"
        };
        let (mut run, _probe, marker) = fake_seatbelt_run(script);

        let event = tokio::time::timeout(Duration::from_millis(500), next(&mut run))
            .await
            .expect("non-regular probe must fail without waiting for the deadline");

        assert!(matches!(event, Some(SandboxEvent::Completed(_))));
        assert!(!marker.exists(), "non-regular proof never releases target");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn symlink_probe_is_rejected_immediately() {
        let script = "printf '%s' \"$OPI_TEST_TOKEN\" > \"${OPI_TEST_PROBE}.target\"; ln -s \"${OPI_TEST_PROBE}.target\" \"$OPI_TEST_PROBE\"; while [ -e \"$OPI_TEST_GATE\" ]; do sleep 0.01; done";
        let (mut run, _probe, marker) = fake_seatbelt_run(script);

        let event = tokio::time::timeout(Duration::from_millis(500), next(&mut run))
            .await
            .expect("symlink probe must fail without waiting for the deadline");

        assert!(matches!(event, Some(SandboxEvent::Completed(_))));
        assert!(!marker.exists(), "symlink proof never releases target");
    }

    #[tokio::test]
    async fn started_waits_for_in_profile_acknowledgement_before_release() {
        let script = if cfg!(windows) {
            "$tmp = \"$env:OPI_TEST_PROBE.tmp\"; Start-Sleep -Milliseconds 150; Set-Content -NoNewline -LiteralPath $tmp -Value $env:OPI_TEST_TOKEN; Move-Item -LiteralPath $tmp -Destination $env:OPI_TEST_PROBE; Remove-Item Env:OPI_TEST_TOKEN; while (Test-Path -LiteralPath $env:OPI_TEST_GATE) { Start-Sleep -Milliseconds 10 }; Set-Content -LiteralPath $env:OPI_TEST_MARKER -Value released"
        } else {
            "tmp=\"${OPI_TEST_PROBE}.tmp\"; sleep 0.15; printf '%s' \"$OPI_TEST_TOKEN\" > \"$tmp\"; ln \"$tmp\" \"$OPI_TEST_PROBE\"; rm -f \"$tmp\"; unset OPI_TEST_TOKEN; while [ -e \"$OPI_TEST_GATE\" ]; do sleep 0.01; done; printf released > \"$OPI_TEST_MARKER\""
        };
        let (mut run, probe, marker) = fake_seatbelt_run(script);

        assert!(
            tokio::time::timeout(Duration::from_millis(50), next(&mut run))
                .await
                .is_err(),
            "Started must remain pending before the in-profile proof"
        );
        assert!(matches!(
            next(&mut run).await,
            Some(SandboxEvent::Started { .. })
        ));
        assert_eq!(
            std::fs::read(&probe).expect("read proof"),
            TEST_START_TOKEN,
            "proof content must match the per-run token before Started"
        );
        assert!(
            !marker.exists(),
            "target stays gated until explicit release"
        );
        run.release().expect("release target");
        assert!(matches!(
            next(&mut run).await,
            Some(SandboxEvent::Completed(_))
        ));
        assert!(marker.exists(), "target ran only after release");
    }

    #[test]
    fn background_setup_path_has_no_spawn_capability() {
        let helper_source = include_str!("helper.rs");
        let runner_source = include_str!("runner.rs");
        let helper_production = helper_source
            .split("\n#[cfg(test)]\nmod tests")
            .next()
            .expect("helper production source");
        let runner_production = runner_source
            .split("\n#[cfg(test)]\nmod tests")
            .next()
            .expect("runner production source");
        let start_body = helper_production
            .split("pub(crate) async fn start(")
            .nth(1)
            .expect("helper start function");

        assert!(
            start_body.contains("spawn_blocking") && start_body.contains("prepare_validated_until"),
            "helper setup workers must call the preparation-only runner path"
        );
        assert!(
            !start_body.contains("run_validated_until") && !start_body.contains("cmd.spawn"),
            "a background setup worker must not own a path that can spawn"
        );
        assert!(
            runner_production.contains("pub(crate) fn spawn_prepared("),
            "actual spawn must be a distinct awaiting-path operation"
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_program_resolution_uses_request_path_case_insensitively() {
        let cwd = tempfile::tempdir().expect("cwd");
        let tools = tempfile::tempdir().expect("tools");
        let executable = tools.path().join("phase16-path-probe.exe");
        std::fs::write(&executable, b"fixture").expect("write fixture executable");
        let additions = [(
            OsString::from("Path"),
            tools.path().as_os_str().to_os_string(),
        )]
        .into_iter()
        .collect();

        assert_eq!(
            resolve_program(
                Path::new("phase16-path-probe"),
                cwd.path(),
                EnvInherit::Clear,
                &additions,
            ),
            Some(executable),
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_clear_environment_does_not_search_ambient_path() {
        let cwd = tempfile::tempdir().expect("cwd");
        assert_eq!(
            resolve_program(
                Path::new("cmd"),
                cwd.path(),
                EnvInherit::Clear,
                &BTreeMap::new(),
            ),
            None,
        );
    }

    fn request(program: PathBuf, args: Vec<OsString>) -> (SandboxRequest, tempfile::TempDir) {
        let workspace = tempfile::tempdir().expect("workspace");
        (
            SandboxRequest {
                program,
                args,
                workspace: workspace.path().to_path_buf(),
                cwd: workspace.path().to_path_buf(),
                timeout: Duration::from_secs(5),
                env_inherit: EnvInherit::Inherit,
                env_additions: BTreeMap::new(),
                stdin: StdinPolicy::Null,
                cancel: None,
            },
            workspace,
        )
    }

    fn exit_request() -> (SandboxRequest, tempfile::TempDir) {
        if cfg!(windows) {
            request(
                PathBuf::from("cmd"),
                vec![OsString::from("/C"), OsString::from("exit 0")],
            )
        } else {
            request(
                PathBuf::from("sh"),
                vec![OsString::from("-c"), OsString::from("exit 0")],
            )
        }
    }

    #[tokio::test]
    async fn direct_run_cancelled_after_spawn_stays_gated_and_cleans_up() {
        let marker_dir = tempfile::tempdir().expect("marker directory");
        let marker = marker_dir.path().join("must-not-exist");
        let (program, args) = if cfg!(windows) {
            (
                PathBuf::from("powershell"),
                vec![
                    OsString::from("-NoProfile"),
                    OsString::from("-Command"),
                    OsString::from(format!(
                        "Set-Content -LiteralPath '{}' -Value x",
                        marker.display()
                    )),
                ],
            )
        } else {
            (
                PathBuf::from("sh"),
                vec![
                    OsString::from("-c"),
                    OsString::from(format!("printf x > '{}'", marker.display())),
                ],
            )
        };
        let cancel = CancellationToken::new();
        let post_spawn_gate: &'static PostSpawnGate = Box::leak(Box::new(PostSpawnGate::new()));
        let cancel_after_spawn = cancel.clone();
        let gate_worker = std::thread::spawn(move || {
            post_spawn_gate.cancel_after_spawn(&cancel_after_spawn);
        });
        let (mut request, _workspace) = request(program, args);
        request.cancel = Some(cancel);
        let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(crate::NoRestriction))
            .with_faults(FaultInjection {
                post_spawn_gate: Some(post_spawn_gate),
                ..FaultInjection::default()
            });

        let mut run = runner
            .run(request)
            .expect("post-spawn cancellation returns a guarded run");
        gate_worker.join().expect("post-spawn cancellation worker");
        let temp_root = run.temp_root().to_path_buf();
        let result = tokio::time::timeout(Duration::from_secs(3), async {
            assert!(matches!(
                next(&mut run).await,
                Some(SandboxEvent::Started { .. })
            ));
            match next(&mut run).await {
                Some(SandboxEvent::Completed(result)) => result,
                other => panic!("expected cancelled completion, got {other:?}"),
            }
        })
        .await
        .expect("post-spawn cancellation cleanup is bounded");

        assert_eq!(result.outcome, SandboxOutcome::Cancelled);
        assert_eq!(result.cleanup, CleanupState::Confirmed);
        assert!(
            !marker.exists(),
            "cancelled target crossed its release gate"
        );
        assert!(!temp_root.exists(), "cancelled run removed its temp root");
    }

    #[tokio::test]
    async fn cancel_after_started_wins_before_auto_release() {
        let marker_dir = tempfile::tempdir().expect("marker directory");
        let marker = marker_dir.path().join("must-not-exist");
        let (program, args) = if cfg!(windows) {
            (
                PathBuf::from("powershell"),
                vec![
                    OsString::from("-NoProfile"),
                    OsString::from("-Command"),
                    OsString::from(format!(
                        "Set-Content -LiteralPath '{}' -Value x",
                        marker.display()
                    )),
                ],
            )
        } else {
            (
                PathBuf::from("sh"),
                vec![
                    OsString::from("-c"),
                    OsString::from(format!("printf x > '{}'", marker.display())),
                ],
            )
        };
        let cancel = CancellationToken::new();
        let cancel_cleanup_gate: &'static PostSpawnGate = Box::leak(Box::new(PostSpawnGate::new()));
        let (mut request, _workspace) = request(program, args);
        request.cancel = Some(cancel.clone());
        let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(crate::NoRestriction))
            .with_faults(FaultInjection {
                cancel_cleanup_gate: Some(cancel_cleanup_gate),
                ..FaultInjection::default()
            });
        let mut run = runner.run(request).expect("run starts behind its gate");
        assert!(matches!(
            next(&mut run).await,
            Some(SandboxEvent::Started { .. })
        ));

        let observed_marker = marker.clone();
        let observer = std::thread::spawn(move || {
            cancel_cleanup_gate.observe_before_cleanup(|| {
                let deadline = std::time::Instant::now() + Duration::from_millis(500);
                while !observed_marker.exists() && std::time::Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(5));
                }
            });
        });
        cancel.cancel();
        let result = tokio::time::timeout(Duration::from_secs(3), async {
            match next(&mut run).await {
                Some(SandboxEvent::Completed(result)) => result,
                other => panic!("expected cancelled completion, got {other:?}"),
            }
        })
        .await
        .expect("post-Started cancellation cleanup is bounded");
        observer.join().expect("cancellation cleanup observer");

        assert_eq!(result.outcome, SandboxOutcome::Cancelled);
        assert_eq!(result.cleanup, CleanupState::Confirmed);
        assert!(
            !marker.exists(),
            "cancelled target crossed its release gate"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unix_bootstrap_preserves_caller_token_environment_variables() {
        let (mut request, _workspace) = request(PathBuf::from("/usr/bin/env"), Vec::new());
        request.env_inherit = EnvInherit::Clear;
        for (key, value) in [
            ("token", "sentinel-token"),
            ("gate", "sentinel-gate"),
            ("backend", "sentinel-backend"),
            ("mode", "sentinel-mode"),
            ("leader", "sentinel-leader"),
            ("token_peer", "sentinel-peer"),
            ("PATH", "/usr/bin:/bin"),
        ] {
            request
                .env_additions
                .insert(OsString::from(key), OsString::from(value));
        }
        let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(crate::NoRestriction));
        let mut run = runner.run(request).expect("run starts");

        assert!(matches!(
            next(&mut run).await,
            Some(SandboxEvent::Started { .. })
        ));
        let result = match next(&mut run).await {
            Some(SandboxEvent::Completed(result)) => result,
            other => panic!("expected Completed, got {other:?}"),
        };

        assert_eq!(
            result.outcome,
            SandboxOutcome::Exited { code: Some(0) },
            "stderr: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let environment = String::from_utf8(result.stdout).expect("target environment is UTF-8");
        for (key, value) in [
            ("token", "sentinel-token"),
            ("gate", "sentinel-gate"),
            ("backend", "sentinel-backend"),
            ("mode", "sentinel-mode"),
            ("leader", "sentinel-leader"),
            ("token_peer", "sentinel-peer"),
        ] {
            assert!(
                environment
                    .lines()
                    .any(|line| line == format!("{key}={value}")),
                "caller environment entry {key} was changed: {environment:?}"
            );
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unix_acknowledgement_does_not_require_request_path() {
        let (mut request, _workspace) = request(PathBuf::from("/usr/bin/env"), Vec::new());
        request.env_inherit = EnvInherit::Clear;
        request
            .env_additions
            .insert(OsString::from("PATH"), OsString::new());
        let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(PassThroughLauncher));
        let mut run = runner.run(request).expect("run starts");

        assert!(matches!(
            next(&mut run).await,
            Some(SandboxEvent::Started { .. })
        ));
        let result = match next(&mut run).await {
            Some(SandboxEvent::Completed(result)) => result,
            other => panic!("expected Completed, got {other:?}"),
        };

        assert_eq!(
            result.outcome,
            SandboxOutcome::Exited { code: Some(0) },
            "stderr: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            String::from_utf8(result.stdout)
                .expect("target environment is UTF-8")
                .lines()
                .any(|line| line == "PATH="),
            "target must receive the caller's empty PATH"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unix_acknowledgement_ignores_hostile_request_path_utilities() {
        use std::os::unix::fs::PermissionsExt;

        let hostile = tempfile::tempdir().expect("hostile PATH directory");
        let marker_root = tempfile::tempdir().expect("marker directory");
        let marker = marker_root.path().join("hostile-utility-ran");
        for utility in ["ln", "rm", "sleep"] {
            let shim = hostile.path().join(utility);
            std::fs::write(
                &shim,
                format!(
                    "#!/bin/sh\nprintf '%s\\n' '{utility}' >> \"$OPI_TEST_HOSTILE_MARKER\"\nexec /bin/{utility} \"$@\"\n"
                ),
            )
            .expect("write hostile utility shim");
            std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755))
                .expect("make hostile utility shim executable");
        }
        let (mut request, _workspace) = request(PathBuf::from("/usr/bin/env"), Vec::new());
        request.env_inherit = EnvInherit::Clear;
        request.env_additions.insert(
            OsString::from("PATH"),
            hostile.path().as_os_str().to_os_string(),
        );
        request.env_additions.insert(
            OsString::from("OPI_TEST_HOSTILE_MARKER"),
            marker.as_os_str().to_os_string(),
        );
        let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(PassThroughLauncher));
        let mut run = runner.run(request).expect("run starts");

        assert!(matches!(
            next(&mut run).await,
            Some(SandboxEvent::Started { .. })
        ));
        assert!(
            !marker.exists(),
            "request-controlled utilities ran before Started"
        );
        run.release().expect("release target");
        let result = match next(&mut run).await {
            Some(SandboxEvent::Completed(result)) => result,
            other => panic!("expected Completed, got {other:?}"),
        };

        assert_eq!(result.outcome, SandboxOutcome::Exited { code: Some(0) });
        assert!(!marker.exists(), "request-controlled utilities ran");
        let environment = String::from_utf8(result.stdout).expect("target environment is UTF-8");
        let expected_path = format!("PATH={}", hostile.path().display());
        assert!(
            environment.lines().any(|line| line == expected_path),
            "target must receive its caller-provided PATH: {environment:?}"
        );
    }

    #[test]
    fn unix_bootstrap_has_no_out_of_profile_filesystem_redirection() {
        let source = include_str!("runner.rs");
        let script = source
            .split_once("const SCRIPT: &str = r#\"")
            .expect("Unix bootstrap start")
            .1
            .split_once("\"#;")
            .expect("Unix bootstrap end")
            .0;

        for line in script.lines().filter(|line| line.contains('>')) {
            assert!(
                line.contains("2>&-") || line.contains("> \"$2.probe.tmp.$$\""),
                "in-profile bootstrap redirects to a filesystem path outside the invocation root: {line}"
            );
        }
    }

    async fn complete_with_faults(faults: FaultInjection) -> SandboxResult {
        let (request, _workspace) = exit_request();
        let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(crate::NoRestriction))
            .with_faults(faults);
        let mut run = runner.run(request).expect("run starts");
        assert!(matches!(
            std::future::poll_fn(|cx| Pin::new(&mut run).poll_next(cx)).await,
            Some(SandboxEvent::Started { .. })
        ));
        match std::future::poll_fn(|cx| Pin::new(&mut run).poll_next(cx)).await {
            Some(SandboxEvent::Completed(result)) => result,
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn injected_attach_failure_refuses_before_target_release() {
        let marker_dir = tempfile::tempdir().expect("marker dir");
        let marker = marker_dir.path().join("must-not-exist");
        let (program, args) = if cfg!(windows) {
            (
                PathBuf::from("powershell"),
                vec![
                    OsString::from("-NoProfile"),
                    OsString::from("-Command"),
                    OsString::from(format!(
                        "Set-Content -LiteralPath '{}' -Value x",
                        marker.display()
                    )),
                ],
            )
        } else {
            (
                PathBuf::from("sh"),
                vec![
                    OsString::from("-c"),
                    OsString::from(format!("printf x > '{}'", marker.display())),
                ],
            )
        };
        let (request, _workspace) = request(program, args);
        let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(crate::NoRestriction))
            .with_faults(FaultInjection {
                attach: true,
                ..FaultInjection::default()
            });
        let failure = match runner.run(request) {
            Ok(run) => {
                drop(run);
                panic!("attach failure must refuse")
            }
            Err(failure) => failure,
        };
        assert_eq!(failure.reason, SetupFailureReason::SpawnFailed);
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert!(!marker.exists(), "target crossed a failed attach gate");
    }

    #[tokio::test]
    async fn injected_cleanup_failures_are_reported_unconfirmed() {
        for faults in [
            FaultInjection {
                terminate: true,
                ..FaultInjection::default()
            },
            FaultInjection {
                wait: true,
                ..FaultInjection::default()
            },
            FaultInjection {
                temp: true,
                ..FaultInjection::default()
            },
        ] {
            let result = complete_with_faults(faults).await;
            assert_eq!(result.cleanup, CleanupState::Unconfirmed);
        }
    }

    #[tokio::test]
    async fn delayed_temp_removal_is_bounded_by_the_hard_deadline() {
        let temp = tempfile::tempdir().expect("temp root");
        let temp_root = temp.path().to_path_buf();
        let deadline = Instant::now() + Duration::from_millis(50);

        let confirmed = tokio::time::timeout(
            Duration::from_millis(500),
            remove_temp_root_until(temp, deadline, Duration::from_secs(1)),
        )
        .await
        .expect("hard deadline bounds temp removal");

        assert!(!confirmed, "removal past the deadline is unconfirmed");
        tokio::time::timeout(Duration::from_secs(2), async {
            while temp_root.exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("detached remover eventually finishes");
    }

    #[tokio::test]
    async fn pre_spawn_expiry_does_not_implicitly_block_on_prepared_cleanup() {
        let (request, _workspace) = exit_request();
        let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(crate::NoRestriction))
            .with_faults(FaultInjection {
                prepared_temp_remove_delay: Duration::from_secs(1),
                ..FaultInjection::default()
            });
        let request = runner.validate_request(request).expect("valid request");
        let prepared = runner
            .prepare_validated_until(request, None)
            .expect("prepared run");
        let now = Instant::now();
        let deadlines = RunDeadlines::new(
            now - Duration::from_millis(1),
            now + Duration::from_secs(2),
            Duration::from_secs(5),
        );
        let wall_start = std::time::Instant::now();

        let prepared = match runner.spawn_prepared(prepared, RunDeadlinePlan::Fixed(deadlines)) {
            SpawnPreparedOutcome::Expired(prepared) => prepared,
            _ => panic!("expired preparation must be returned without spawning"),
        };
        assert!(
            wall_start.elapsed() < Duration::from_millis(400),
            "pre-spawn expiry implicitly waited for prepared cleanup"
        );
        let cleanup_start = std::time::Instant::now();
        let confirmed =
            cleanup_prepared_until(*prepared, Instant::now() + Duration::from_millis(50)).await;
        assert!(!confirmed, "late prepared cleanup must be unconfirmed");
        assert!(
            cleanup_start.elapsed() < Duration::from_millis(400),
            "prepared cleanup exceeded its hard deadline"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn release_is_idempotent_after_the_execution_deadline() {
        let (mut request, _workspace) = exit_request();
        request.timeout = Duration::from_millis(50);
        let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(crate::NoRestriction));
        let mut run = runner.run(request).expect("run starts gated");

        assert!(matches!(
            std::future::poll_fn(|cx| Pin::new(&mut run).poll_next(cx)).await,
            Some(SandboxEvent::Started { .. })
        ));
        run.release().expect("first release succeeds");
        tokio::time::advance(Duration::from_millis(51)).await;

        run.release()
            .expect("repeated release remains a successful no-op");
    }

    #[tokio::test(start_paused = true)]
    async fn unreleased_run_refuses_release_after_the_execution_deadline() {
        let (mut request, _workspace) = exit_request();
        request.timeout = Duration::from_millis(50);
        let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(crate::NoRestriction));
        let mut run = runner.run(request).expect("run starts gated");

        assert!(matches!(
            std::future::poll_fn(|cx| Pin::new(&mut run).poll_next(cx)).await,
            Some(SandboxEvent::Started { .. })
        ));
        tokio::time::advance(Duration::from_millis(51)).await;

        let error = run.release().expect_err("expired gate must stay closed");
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    }

    #[tokio::test(start_paused = true)]
    async fn expired_auto_release_keeps_the_target_behind_its_gate() {
        let marker_dir = tempfile::tempdir().expect("marker dir");
        let marker = marker_dir.path().join("must-not-exist");
        let (program, args) = if cfg!(windows) {
            (
                PathBuf::from("powershell"),
                vec![
                    OsString::from("-NoProfile"),
                    OsString::from("-Command"),
                    OsString::from(format!(
                        "Set-Content -LiteralPath '{}' -Value x",
                        marker.display()
                    )),
                ],
            )
        } else {
            (
                PathBuf::from("sh"),
                vec![
                    OsString::from("-c"),
                    OsString::from(format!("printf x > '{}'", marker.display())),
                ],
            )
        };
        let (mut request, _workspace) = request(program, args);
        request.timeout = Duration::from_millis(50);
        let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(crate::NoRestriction))
            .with_faults(FaultInjection {
                terminate_delay: Duration::from_millis(200),
                ..FaultInjection::default()
            });
        let mut run = runner.run(request).expect("run starts gated");

        assert!(matches!(
            std::future::poll_fn(|cx| Pin::new(&mut run).poll_next(cx)).await,
            Some(SandboxEvent::Started { .. })
        ));
        tokio::time::advance(Duration::from_millis(51)).await;
        let result = match std::future::poll_fn(|cx| Pin::new(&mut run).poll_next(cx)).await {
            Some(SandboxEvent::Completed(result)) => result,
            other => panic!("expected timed-out completion, got {other:?}"),
        };

        assert_eq!(result.outcome, SandboxOutcome::TimedOut);
        assert!(!marker.exists(), "expired auto-release crossed its gate");
    }
}
