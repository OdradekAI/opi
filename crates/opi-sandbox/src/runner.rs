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

mod gated;
mod preparation;
mod supervision;
#[cfg(test)]
mod tests;

pub(crate) use gated::{SpawnPreparedOutcome, StartConfirmationFailure};
pub(crate) use preparation::cleanup_prepared_until;

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
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::policy::{
    ContractStatus, LauncherSpec, Mechanism, Restriction, RestrictionCtx, SandboxPolicy,
};
#[cfg(windows)]
use crate::process_tree::resume_child;
use crate::process_tree::{TerminationOutcome, TreeGuard, configure_tree};

/// Bounded per-stream terminal preview (1 MiB). Every byte is also relayed as
/// an incremental [`SandboxEvent::Output`] through a bounded channel.
const OUTPUT_CAP: usize = 1024 * 1024;
/// At most eight 8-KiB output chunks may wait between the pipe drains and the
/// consumer. A slow consumer therefore applies process-pipe backpressure while
/// keeping the runner's memory use bounded.
const OUTPUT_EVENT_CAPACITY: usize = 8;

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
    /// `env_additions` are applied next, then the invocation-owned `TMPDIR`,
    /// `TMP`, and `TEMP` aliases are set to the private temporary root. Callers
    /// cannot override those reserved aliases through `env_additions`.
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
    owner_death_cleanup: Option<preparation::OwnerDeathCleanup>,
    temp_root: PathBuf,
    release_gate: PathBuf,
    start_probe: Option<gated::StartProbe>,
    cancel: CancellationToken,
    mechanism: Mechanism,
    contract: ContractStatus,
    faults: FaultInjection,
}

struct PreparedTemp {
    temp: Option<tempfile::TempDir>,
    remove_delay: Duration,
    injected_failure: bool,
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
/// orthogonal cleanup state, bounded stdout/stderr previews, and the path of the
/// invocation-owned temp root that was removed. Complete output is delivered by
/// preceding [`SandboxEvent::Output`] events.
#[derive(Debug, Clone)]
pub struct SandboxResult {
    /// The effective terminal outcome.
    pub outcome: SandboxOutcome,
    /// Whether invocation-owned cleanup completed.
    pub cleanup: CleanupState,
    /// Bounded standard-output preview.
    pub stdout: Vec<u8>,
    /// Bounded standard-error preview.
    pub stderr: Vec<u8>,
    /// Whether stdout exceeded the capture cap or could not be drained fully.
    pub stdout_truncated: bool,
    /// Whether stderr exceeded the capture cap or could not be drained fully.
    pub stderr_truncated: bool,
    /// The invocation-owned temp root, whether removed or left unconfirmed.
    pub temp_root: PathBuf,
}

/// Lifecycle events streamed by [`SandboxRun`]. A successfully established run
/// emits [`SandboxEvent::Started`], incremental output and redacted diagnostics,
/// then a single terminal [`SandboxEvent::Completed`]. A rejected launcher
/// completes without a `Started` event.
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
    /// A target output chunk. Chunks are emitted incrementally with bounded
    /// backpressure and remain ordered within each output stream.
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
        let execution = now
            .checked_add(timeout)
            .ok_or_else(preparation::invalid_request)?;
        execution
            .checked_add(DIRECT_CLEANUP_BUDGET)
            .ok_or_else(preparation::invalid_request)?;
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
                Err(preparation::deadline_setup_failure())
            }
            SpawnPreparedOutcome::Failed(failed) => Err(failed),
        }
    }
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
    start_probe: Option<gated::StartProbe>,
    start_probe_poll: Option<Pin<Box<tokio::time::Sleep>>>,
    start_probe_rejected: bool,
    prestart_result: Option<SandboxResult>,
    cancel: CancellationToken,
    deadline_plan: RunDeadlinePlan,
    deadline_cell: Arc<OnceLock<RunDeadlines>>,
    event_rx: mpsc::Receiver<SandboxEvent>,
    terminal_result: Option<SandboxResult>,
    inner: Option<Pin<Box<dyn std::future::Future<Output = SandboxResult> + Send>>>,
}

// `SandboxRun` owns the supervision future, which owns the child/tree/temp
// guards. When `SandboxRun` drops before completion, `inner` (still `Some`)
// drops, dropping those guards: child `kill_on_drop`, `TreeGuard` terminate,
// temp-root removal. No explicit `Drop` body is needed beyond the field's own
// drop, but asserting the invariant here documents the contract.
// (When `inner` is already `None` — the run completed — cleanup already ran
// inside the supervision future before it returned.)
