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
//! [`SandboxRun`] is an owned [`Stream`] of [`SandboxEvent`]. Its first item is
//! [`SandboxEvent::Started`] carrying the invocation-owned temp-root path and the
//! direct-child id, so a dropped-future test can OBSERVE cleanup (fold #7).
//! Polling the stream drives the supervision to a single terminal
//! [`SandboxEvent::Completed`] (fold #9: single completion, no split handle).
//! Dropping an in-flight [`SandboxRun`] drops the owned child (`kill_on_drop`),
//! the tree guard, and the temp root, so the tree is killed and the temp root is
//! removed on EVERY terminal path (design `### L0 supervision`).

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use futures_core::Stream;
use opi_protocol::execution::v1::EnvInherit;
use tokio::process::{Child, Command};
use tokio_util::sync::CancellationToken;

use crate::policy::{ContractStatus, Mechanism, Restriction, SandboxPolicy};
use crate::process_tree::{TreeGuard, configure_tree};

/// Bounded per-stream output capture (1 MiB). Output beyond this cap is dropped
/// (the bound is enforced, not exceeded); the captured prefix is returned.
const OUTPUT_CAP: usize = 1024 * 1024;

/// Bounded grace for draining a terminated tree's still-open stdout/stderr pipes
/// (mirrors the Phase 16 task 16.2 `TERMINATED_PIPE_DRAIN_GRACE` invariant).
const PIPE_DRAIN_GRACE: Duration = Duration::from_millis(500);

/// Explicit inputs for one sandboxed run. Mirrors the `opi-protocol`
/// `ExecutePayload` explicit-input set as ergonomic Rust types. There is NO
/// shell-string field and NO target-stdin field: callers pass an explicit
/// program and argument vector (design `### State model`, `#Reuse outside Opi`;
/// Phase 16 task 16.11.1 audit fold #2).
#[derive(Debug, Clone)]
pub struct SandboxRequest {
    /// The explicit program to execute (resolved by the caller; not a shell
    /// expression).
    pub program: PathBuf,
    /// The explicit argument vector.
    pub args: Vec<String>,
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
    pub env_additions: BTreeMap<String, String>,
    /// Optional cooperative cancellation token. When present, firing it resolves
    /// the run to [`SandboxOutcome::Cancelled`]. When absent, cancellation is
    /// exclusively future-drop (which observes no result).
    pub cancel: Option<CancellationToken>,
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
/// conflated. There is NO `CleanupUnconfirmed` variant: cleanup-unconfirmed is a
/// REMOTE destination concept owned by the protocol/binary layer (16.11.2), and
/// local SDK cleanup is deterministic (Phase 16 task 16.11.1 audit folds #3/#10).
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
/// Local SDK cleanup is deterministic, so the library always reports
/// [`CleanupState::Confirmed`]; [`CleanupState::Unconfirmed`] is reserved for
/// the remote-destination case surfaced by the binary/protocol layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupState {
    /// The invocation-owned temp root and child tree were removed.
    Confirmed,
    /// Destination cleanup could not be confirmed (remote case; not emitted by
    /// this library).
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
    /// The invocation-owned temp root that was removed at terminal completion.
    pub temp_root: PathBuf,
}

/// Lifecycle events streamed by [`SandboxRun`]. This library emits
/// [`SandboxEvent::Started`] (first poll) then a single terminal
/// [`SandboxEvent::Completed`]; [`SandboxEvent::Output`] and
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
}

impl SandboxRunner {
    /// Create a runner for `policy` that applies `restriction` to every run.
    pub fn new(policy: SandboxPolicy, restriction: Arc<dyn Restriction>) -> Self {
        Self {
            policy,
            restriction,
        }
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
        if request.timeout.is_zero() || request.workspace.as_os_str().is_empty() {
            return Err(SetupFailed {
                reason: SetupFailureReason::InvalidRequest,
            });
        }
        // Create the invocation-owned temp root. Owned by `run` until it is moved
        // into the supervision future; on any error path below it drops and the
        // dir is removed.
        let temp = tempfile::TempDir::new().map_err(|_| SetupFailed {
            reason: SetupFailureReason::SpawnFailed,
        })?;
        let temp_root = temp.path().to_path_buf();

        let mut cmd = Command::new(&request.program);
        cmd.args(&request.args)
            .current_dir(&request.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        apply_env(&mut cmd, request.env_inherit, &request.env_additions);

        let applied = self
            .restriction
            .prepare(&mut cmd)
            .map_err(|_| SetupFailed {
                reason: SetupFailureReason::RestrictionSetup,
            })?;

        configure_tree(&mut cmd);

        let child = match cmd.spawn() {
            Ok(child) => child,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(SetupFailed {
                    reason: SetupFailureReason::ProgramNotFound,
                });
            }
            Err(_) => {
                return Err(SetupFailed {
                    reason: SetupFailureReason::SpawnFailed,
                });
            }
        };
        // Spawn and guard are in the same synchronous span: no `.await between`
        // them (Phase 16 task 16.11.1 audit fold #1).
        let child_pid = child.id();
        let tree = TreeGuard::attach_child(child_pid).unwrap_or_else(|_| TreeGuard::disabled());

        let cancel = request.cancel.unwrap_or_default();
        let mechanism = applied.mechanism;
        let contract = applied.contract;
        let inner = Box::pin(supervise(child, tree, temp, request.timeout, cancel));

        Ok(SandboxRun {
            started_emitted: false,
            completed: false,
            temp_root,
            child_pid,
            mechanism,
            contract,
            inner: Some(inner),
        })
    }
}

/// Owned handle to one in-flight run: a [`Stream`] of [`SandboxEvent`] whose
/// `Drop` kills the child tree and removes the invocation-owned temp root.
///
/// Poll it (via `futures_util::StreamExt::next`) to drive supervision. The first
/// item is [`SandboxEvent::Started`]; the terminal item is a single
/// [`SandboxEvent::Completed`]. Dropping the stream before completion drops the
/// owned supervision future, which drops the child (`kill_on_drop`), the
/// [`TreeGuard`], and the temp root — so the tree is killed and the temp root is
/// removed on every path (success, timeout, cancellation, error, dropped
/// future).
pub struct SandboxRun {
    started_emitted: bool,
    completed: bool,
    temp_root: PathBuf,
    child_pid: Option<u32>,
    mechanism: Mechanism,
    contract: ContractStatus,
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
}

impl Stream for SandboxRun {
    type Item = SandboxEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if !self.started_emitted {
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

// `SandboxRun` owns the supervision future, which owns the child/tree/temp
// guards. When `SandboxRun` drops before completion, `inner` (still `Some`)
// drops, dropping those guards: child `kill_on_drop`, `TreeGuard` terminate,
// temp-root removal. No explicit `Drop` body is needed beyond the field's own
// drop, but asserting the invariant here documents the contract.
// (When `inner` is already `None` — the run completed — cleanup already ran
// inside the supervision future before it returned.)

/// Apply the environment-inheritance policy and bounded additions to `cmd`.
fn apply_env(cmd: &mut Command, inherit: EnvInherit, additions: &BTreeMap<String, String>) {
    if matches!(inherit, EnvInherit::Clear) {
        cmd.env_clear();
    }
    for (key, value) in additions {
        cmd.env(key, value);
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
    timeout: Duration,
    cancel: CancellationToken,
) -> SandboxResult {
    let temp_root = temp.path().to_path_buf();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let drain_out = CaptureTask::new(stdout, OUTPUT_CAP);
    let drain_err = CaptureTask::new(stderr, OUTPUT_CAP);

    // Race wait / timeout / cancellation. On every branch the whole tree is
    // terminated; biased ordering (cancel > timeout > wait) classifies
    // simultaneous trips deterministically.
    let outcome = tokio::select! {
        biased;
        _ = cancel.cancelled() => {
            let _ = child.kill().await;
            let _ = tree.terminate();
            SandboxOutcome::Cancelled
        }
        _ = tokio::time::sleep(timeout) => {
            let _ = child.kill().await;
            let _ = tree.terminate();
            SandboxOutcome::TimedOut
        }
        status = child.wait() => match status {
            Ok(status) => {
                let _ = tree.terminate();
                status_to_outcome(status)
            }
            Err(_) => {
                let _ = tree.terminate();
                SandboxOutcome::Exited { code: None }
            }
        },
    };

    // Finish both drains under a bounded grace. On grace expiry the inner future
    // is dropped, which drops the CaptureTasks (aborting their tasks) and we
    // return what was captured so far (empty).
    let (out, err) = match tokio::time::timeout(PIPE_DRAIN_GRACE, async {
        tokio::join!(drain_out.finish(), drain_err.finish())
    })
    .await
    {
        Ok((out, err)) => (out, err),
        Err(_) => (Vec::new(), Vec::new()),
    };

    // `temp`, `tree`, and `child` drop here (locals) in reverse order: temp-root
    // removal, idempotent tree terminate, child kill_on_drop. Cleanup is
    // deterministic, so the result reports Confirmed.
    SandboxResult {
        outcome,
        cleanup: CleanupState::Confirmed,
        stdout: out,
        stderr: err,
        temp_root,
    }
}

/// Owned handle to one stream's drain task. The task reads the pipe into a
/// `Vec<u8>` bounded by `cap`; `finish` awaits the capture, and `Drop` aborts an
/// unfinished task so a descendant holding a pipe cannot outlive the run.
struct CaptureTask {
    handle: Option<tokio::task::JoinHandle<Vec<u8>>>,
}

impl CaptureTask {
    /// Spawn the drain for `stream`, capturing up to `cap` bytes.
    fn new<R>(stream: Option<R>, cap: usize) -> Self
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
    {
        let handle = tokio::spawn(async move {
            let mut buf = Vec::new();
            if let Some(mut stream) = stream {
                use tokio::io::AsyncReadExt;
                let mut chunk = [0u8; 8192];
                loop {
                    match stream.read(&mut chunk).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if buf.len() < cap {
                                let take = std::cmp::min(n, cap - buf.len());
                                buf.extend_from_slice(&chunk[..take]);
                            }
                        }
                    }
                }
            }
            buf
        });
        Self {
            handle: Some(handle),
        }
    }

    /// Await the capture. Consumes the handle so `Drop` will not double-abort.
    async fn finish(mut self) -> Vec<u8> {
        match self.handle.take() {
            Some(handle) => handle.await.unwrap_or_default(),
            None => Vec::new(),
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
