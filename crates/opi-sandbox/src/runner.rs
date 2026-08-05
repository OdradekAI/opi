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
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use futures_core::Stream;
use opi_protocol::execution::v1::EnvInherit;
use tokio::process::{Child, Command};
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
    faults: FaultInjection,
}

#[derive(Debug, Clone, Copy, Default)]
struct FaultInjection {
    attach: bool,
    terminate: bool,
    wait: bool,
    temp: bool,
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

    #[cfg(test)]
    fn with_faults(mut self, faults: FaultInjection) -> Self {
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
        if request.timeout.is_zero() || request.workspace.as_os_str().is_empty() {
            return Err(SetupFailed {
                reason: SetupFailureReason::InvalidRequest,
            });
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
        let workspace = request.workspace.canonicalize().map_err(|_| SetupFailed {
            reason: SetupFailureReason::InvalidRequest,
        })?;
        let cwd = request.cwd.canonicalize().map_err(|_| SetupFailed {
            reason: SetupFailureReason::InvalidRequest,
        })?;
        if !cwd.starts_with(&workspace) {
            return Err(SetupFailed {
                reason: SetupFailureReason::InvalidRequest,
            });
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
        };

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
        let mut cmd = gated_command(
            &program,
            &request.args,
            &request.env_additions,
            &release_gate,
            self.restriction.launcher(&ctx),
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

        let applied = self
            .restriction
            .prepare(&mut cmd, &ctx)
            .map_err(|_| SetupFailed {
                reason: SetupFailureReason::RestrictionSetup,
            })?;
        let contract_is_consistent = matches!(
            (applied.mechanism, applied.contract),
            (Mechanism::None, ContractStatus::Unrestricted)
                | (
                    Mechanism::Landlock | Mechanism::Seccomp | Mechanism::Seatbelt,
                    ContractStatus::Restricted
                )
        );
        if !contract_is_consistent {
            return Err(SetupFailed {
                reason: SetupFailureReason::RestrictionSetup,
            });
        }

        configure_tree(&mut cmd);

        let mut child = match cmd.spawn() {
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
        if self.faults.attach {
            let _ = child.start_kill();
            return Err(SetupFailed {
                reason: SetupFailureReason::SpawnFailed,
            });
        }
        let tree = match TreeGuard::attach_child(child_pid) {
            Ok(tree) => tree,
            Err(_) => {
                let _ = child.start_kill();
                return Err(SetupFailed {
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
            return Err(SetupFailed {
                reason: SetupFailureReason::SpawnFailed,
            });
        }

        let cancel = request.cancel.unwrap_or_default();
        let mechanism = applied.mechanism;
        let contract = applied.contract;
        let inner = Box::pin(supervise(
            child,
            tree,
            temp,
            temp_root.clone(),
            request.timeout,
            cancel,
            self.faults,
        ));

        Ok(SandboxRun {
            started_emitted: false,
            completed: false,
            temp_root,
            child_pid,
            mechanism,
            contract,
            release_gate: Some(release_gate),
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
    release_gate: Option<PathBuf>,
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

    /// Release the real target after the caller has observed and published the
    /// [`SandboxEvent::Started`] contract. Idempotent.
    pub fn release(&mut self) -> io::Result<()> {
        let Some(release_gate) = self.release_gate.take() else {
            return Ok(());
        };
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
        let _ = self.release();
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
    launcher: Option<LauncherSpec>,
) -> Command {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        const SCRIPT: &str = r#"gate=$1
shift
backend=$PPID
exec 9>"${gate}.probe" || exit 125
while [ -e "$gate" ]; do
  kill -0 "$backend" 2>&9 || exit 125
  sleep 0.01
done
(
  leader=$$
  while kill -0 "$backend" 2>&9 && kill -0 "$leader" 2>&9; do
    sleep 0.05
  done
  if ! kill -0 "$backend" 2>&9; then
    kill -KILL "-$leader" 2>&9
  fi
) &
mode=$1
shift
if [ "$mode" = restore-native-env ]; then
  exec /usr/bin/env -- "$@"
fi
[ "$mode" = direct ] || exit 125
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
            .arg(release_gate);
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
        let _ = (env_additions, launcher);
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
        let _ = (env_additions, release_gate, launcher);
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
    timeout: Duration,
    cancel: CancellationToken,
    faults: FaultInjection,
) -> SandboxResult {
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
            cleanup_confirmed &= terminate_tree(&mut tree, faults.terminate);
            let _ = child.start_kill();
            cleanup_confirmed &= reap_child(&mut child).await;
            SandboxOutcome::Cancelled
        }
        _ = tokio::time::sleep(timeout) => {
            cleanup_confirmed &= terminate_tree(&mut tree, faults.terminate);
            let _ = child.start_kill();
            cleanup_confirmed &= reap_child(&mut child).await;
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
    match tokio::time::timeout(PIPE_DRAIN_GRACE, async {
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

    if temp.close().is_err() || faults.temp {
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

fn terminate_tree(tree: &mut TreeGuard, injected_failure: bool) -> bool {
    let confirmed = !matches!(tree.terminate(), TerminationOutcome::Failed(_));
    confirmed && !injected_failure
}

async fn reap_child(child: &mut Child) -> bool {
    matches!(
        tokio::time::timeout(PIPE_DRAIN_GRACE, child.wait()).await,
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
}
