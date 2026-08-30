//! Crate-private external-process supervision (Phase 18 task 18.4).
//!
//! [`ProcessSupervisor`] is the one shared state machine the Phase 18 Agent and
//! benchmark adapters use to run external processes. It accepts only structured
//! inputs — argv, cwd, an exact environment projection, output caps, a timeout,
//! and a cancellation token — and settles, on every path, the child exit state,
//! bounded stdout/stderr captures, and cleanup evidence for the terminated
//! process tree. No OS primitive (pid, signal, handle) crosses this module; the
//! future `agent::process` and `benchmark::process` adapters consume only the
//! typed outcome.
//!
//! # Environment projection
//!
//! The provided map is the child's ENTIRE environment (`env_clear`), matching
//! the resolved experiment's exact environment projection: deterministic,
//! reproducible, and free of ambient leakage.
//!
//! # Failure behavior
//!
//! Spawn failures settle as [`ExitState::FailedToSpawn`] with a redacted static
//! reason token (never command text, arguments, environment values, or paths).
//! Timeout and cancellation terminate the whole descendant tree via
//! [`tree`]'s OS layer and report [`CleanupEvidence`]; a termination whose
//! emptiness cannot be confirmed is reported unverified rather than silently
//! claimed clean.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

mod tree;

/// Structured, shell-free spawn request for one supervised external process.
#[derive(Debug, Clone)]
pub(crate) struct SpawnSpec {
    /// Exact argv, including argv[0]. Never interpreted as a shell string.
    pub argv: Vec<OsString>,
    /// Working directory for the child. `None` inherits the supervisor's cwd.
    pub cwd: Option<PathBuf>,
    /// The child's ENTIRE environment (applied over a cleared base).
    pub env: BTreeMap<OsString, OsString>,
    /// Maximum number of stdout bytes retained; anything beyond is drained and
    /// discarded with `truncated = true`.
    pub stdout_cap: usize,
    /// Maximum number of stderr bytes retained; anything beyond is drained and
    /// discarded with `truncated = true`.
    pub stderr_cap: usize,
    /// Wall-clock budget for the whole run. Elapsing terminates the tree and
    /// settles the outcome as [`ExitState::TimedOut`].
    pub timeout: Duration,
}

/// Redacted reason a supervised process never started. Static tokens only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpawnReason {
    /// The executable path does not exist.
    NotFound,
    /// The executable or cwd could not be used due to permissions.
    PermissionDenied,
    /// The requested working directory does not exist.
    BadCwd,
    /// Any other spawn failure (e.g. resource exhaustion).
    SpawnFailed,
}

/// Settled exit state of one supervised run. Timeout and cancellation are
/// first-class states, not errors: the supervisor, not the child, decided the
/// run was over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExitState {
    /// The child exited by itself with this code. `None` from the platform
    /// (e.g. a Unix signal death) settles as `-1`: signal detail is not part
    /// of this seam's contract.
    Exited { code: i32 },
    /// The process never started; see the redacted [`SpawnReason`].
    FailedToSpawn { reason: SpawnReason },
    /// The timeout elapsed and the tree was terminated.
    TimedOut,
    /// The cancellation token fired and the tree was terminated.
    Cancelled,
}

/// Bounded stream capture: the retained prefix plus whether draining discarded
/// anything beyond [`SpawnSpec::stdout_cap`] / [`SpawnSpec::stderr_cap`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct OutputCapture {
    /// Retained prefix bytes (at most the configured cap).
    pub bytes: Vec<u8>,
    /// Whether the stream produced more bytes than the cap retained.
    pub truncated: bool,
}

/// Observed cleanup outcome for the terminated process tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CleanupEvidence {
    /// Nothing had to be terminated: the child settled by itself.
    NotRequired,
    /// The tree was terminated and its emptiness was observed within the
    /// post-termination grace window.
    TreeTerminated { layer: &'static str, verified: bool },
    /// Termination failed at the OS layer; cleanup must be reported
    /// unconfirmed.
    TreeTerminationFailed { layer: &'static str },
}

/// Settled outcome of one supervised run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SupervisedOutcome {
    /// How the run ended.
    pub exit: ExitState,
    /// Bounded stdout capture.
    pub stdout: OutputCapture,
    /// Bounded stderr capture.
    pub stderr: OutputCapture,
    /// What termination cleanup was observed, if any was needed.
    pub cleanup: CleanupEvidence,
}

/// The crate-private supervision state machine.
///
/// One value-free type: all state lives in the future returned by [`run`], so
/// a supervisor cannot be shared mid-run and every terminal path reports its
/// own cleanup evidence.
pub(crate) struct ProcessSupervisor;

impl ProcessSupervisor {
    /// Run `spec` to settlement under `timeout` and external `cancel`.
    ///
    /// Never panics and never returns an `Err`: every failure mode — spawn
    /// failure, timeout, cancellation, termination failure — is a settled
    /// outcome with redacted diagnostics, so callers cannot mistake an
    /// unsettled run for a completed one.
    pub(crate) async fn run(spec: &SpawnSpec, cancel: &CancellationToken) -> SupervisedOutcome {
        // An empty argv can never spawn; settle it as a redacted typed failure
        // instead of indexing into it.
        let Some(program) = spec.argv.first() else {
            return SupervisedOutcome {
                exit: ExitState::FailedToSpawn {
                    reason: SpawnReason::SpawnFailed,
                },
                stdout: OutputCapture::default(),
                stderr: OutputCapture::default(),
                cleanup: CleanupEvidence::NotRequired,
            };
        };

        let mut cmd = tokio::process::Command::new(program);
        cmd.args(&spec.argv[1..])
            .env_clear()
            .envs(&spec.env)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        if let Some(cwd) = &spec.cwd {
            cmd.current_dir(cwd);
        }
        tree::configure(&mut cmd);

        // A missing/forbidden cwd must not masquerade as a missing binary:
        // check it before spawn so the redacted reason is exact.
        if let Some(cwd) = &spec.cwd
            && let Err(error) = std::fs::metadata(cwd)
        {
            return SupervisedOutcome {
                exit: ExitState::FailedToSpawn {
                    reason: match error.kind() {
                        std::io::ErrorKind::NotFound => SpawnReason::BadCwd,
                        std::io::ErrorKind::PermissionDenied => SpawnReason::PermissionDenied,
                        _ => SpawnReason::SpawnFailed,
                    },
                },
                stdout: OutputCapture::default(),
                stderr: OutputCapture::default(),
                cleanup: CleanupEvidence::NotRequired,
            };
        }

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(error) => {
                return SupervisedOutcome {
                    exit: ExitState::FailedToSpawn {
                        reason: map_spawn_error(error),
                    },
                    stdout: OutputCapture::default(),
                    stderr: OutputCapture::default(),
                    cleanup: CleanupEvidence::NotRequired,
                };
            }
        };
        let mut guard = tree::TreeGuard::attach(child.id());
        let stdout_reader = child
            .stdout
            .take()
            .map(|s| bounded_reader(s, spec.stdout_cap));
        let stderr_reader = child
            .stderr
            .take()
            .map(|s| bounded_reader(s, spec.stderr_cap));

        // Settle decision. Biased so a naturally-exited child wins over a
        // cancellation that fired at the same instant: the exit is a fact, the
        // cancel is a request that arrived too late to matter.
        let timeout_sleep = tokio::time::sleep(spec.timeout);
        tokio::pin!(timeout_sleep);
        let decided = tokio::select! {
            biased;
            status = child.wait() => ExitState::Exited {
                code: status.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1),
            },
            _ = cancel.cancelled() => ExitState::Cancelled,
            _ = &mut timeout_sleep => ExitState::TimedOut,
        };

        match decided {
            ExitState::Exited { code } => SupervisedOutcome {
                exit: ExitState::Exited { code },
                // Readers may be held open by descendants that inherited the
                // pipes: settle them within a bounded EOF grace, then keep the
                // prefix captured so far instead of blocking forever.
                stdout: settle_reader(stdout_reader, EOF_GRACE).await,
                stderr: settle_reader(stderr_reader, EOF_GRACE).await,
                cleanup: CleanupEvidence::NotRequired,
            },
            kill_decision => {
                // The supervisor decided: terminate the tree first so held
                // pipes close and streams settle, reap the direct child (it
                // remains a group member until waited), then observe
                // emptiness for the cleanup evidence.
                let cleanup = if guard.terminate() {
                    let _ = tokio::time::timeout(REAP_GRACE, child.wait()).await;
                    let verified = guard.verify_terminated(VERIFY_WINDOW).await;
                    CleanupEvidence::TreeTerminated {
                        layer: tree::LAYER,
                        verified,
                    }
                } else {
                    CleanupEvidence::TreeTerminationFailed { layer: tree::LAYER }
                };
                // kill_on_drop remains the backstop if even the reap is refused.
                SupervisedOutcome {
                    exit: kill_decision,
                    stdout: settle_reader(stdout_reader, EOF_GRACE).await,
                    stderr: settle_reader(stderr_reader, EOF_GRACE).await,
                    cleanup,
                }
            }
        }
    }
}

/// Map a raw spawn [`std::io::Error`] to its redacted reason token. The
/// original error (and any command text it may carry) is dropped: only the
/// static token survives into the settled outcome.
fn map_spawn_error(error: std::io::Error) -> SpawnReason {
    match error.kind() {
        std::io::ErrorKind::NotFound => SpawnReason::NotFound,
        std::io::ErrorKind::PermissionDenied => SpawnReason::PermissionDenied,
        _ => SpawnReason::SpawnFailed,
    }
}

/// Bounded EOF grace after a natural child exit.
const EOF_GRACE: Duration = Duration::from_secs(2);
/// Bounded grace for reaping the direct child after tree termination.
const REAP_GRACE: Duration = Duration::from_secs(5);
/// Bounded window for observing tree emptiness after termination.
const VERIFY_WINDOW: Duration = Duration::from_secs(2);

/// One in-flight bounded stream reader: a shared accumulator plus the task
/// draining it. The accumulator holds the prefix captured so far, so aborting
/// the task on grace expiry never loses what was already read.
struct StreamReader {
    capture: std::sync::Arc<std::sync::Mutex<OutputCapture>>,
    task: tokio::task::JoinHandle<()>,
}

/// Read `stream` to EOF, retaining at most `cap` prefix bytes in the shared
/// accumulator and discarding the rest so a chatty child never blocks on a
/// full pipe.
fn bounded_reader<R: tokio::io::AsyncRead + Unpin + Send + 'static>(
    mut stream: R,
    cap: usize,
) -> StreamReader {
    let capture = std::sync::Arc::new(std::sync::Mutex::new(OutputCapture::default()));
    let sink = std::sync::Arc::clone(&capture);
    let task = tokio::spawn(async move {
        let mut chunk = [0u8; 8192];
        loop {
            let n = match tokio::io::AsyncReadExt::read(&mut stream, &mut chunk).await {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            let mut guard = sink.lock().unwrap_or_else(|p| p.into_inner());
            if guard.bytes.len() < cap {
                let room = cap - guard.bytes.len();
                let take = n.min(room);
                guard.bytes.extend_from_slice(&chunk[..take]);
                if take < n {
                    guard.truncated = true;
                }
            } else if n > 0 {
                guard.truncated = true;
            }
        }
    });
    StreamReader { capture, task }
}

/// Settle a reader: await EOF within `grace`; on expiry abort the drain and
/// snapshot the prefix captured so far.
async fn settle_reader(reader: Option<StreamReader>, grace: Duration) -> OutputCapture {
    let Some(reader) = reader else {
        return OutputCapture::default();
    };
    if tokio::time::timeout(grace, reader.task).await.is_err() {
        // The drain is abandoned mid-stream (a descendant still holds the
        // pipe); whatever arrived so far is the settled capture.
    }
    reader
        .capture
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    fn shell_spec(script: &str, timeout: Duration) -> SpawnSpec {
        SpawnSpec {
            argv: vec!["/bin/sh".into(), "-c".into(), script.into()],
            cwd: None,
            env: BTreeMap::from([("EVAL_PROBE".into(), "42".into())]),
            stdout_cap: 4096,
            stderr_cap: 4096,
            timeout,
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn supervised_output_beyond_cap_is_truncated_and_drained_to_eof() {
        // 200_000 bytes against a 4_096 cap: the child can only exit 0 if the
        // supervisor keeps draining past the cap instead of closing the pipe.
        let mut spec = shell_spec("/usr/bin/head -c 200000 /dev/zero", Duration::from_secs(20));
        spec.timeout = Duration::from_secs(20);
        let outcome = ProcessSupervisor::run(&spec, &CancellationToken::new()).await;

        assert_eq!(outcome.exit, ExitState::Exited { code: 0 });
        assert!(outcome.stdout.truncated);
        assert_eq!(outcome.stdout.bytes.len(), 4096);
        assert!(outcome.stdout.bytes.iter().all(|&b| b == 0));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn supervised_timeout_terminates_process_group_with_verified_cleanup() {
        let start = std::time::Instant::now();
        // One background descendant plus the child itself, both asleep far
        // beyond the timeout and the kill grace window.
        let spec = shell_spec(
            "/bin/sleep 15 & exec /bin/sleep 15",
            Duration::from_millis(200),
        );
        let outcome = ProcessSupervisor::run(&spec, &CancellationToken::new()).await;

        assert_eq!(outcome.exit, ExitState::TimedOut);
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "settlement took {}s",
            start.elapsed().as_secs()
        );
        assert_eq!(
            outcome.cleanup,
            CleanupEvidence::TreeTerminated {
                layer: "unix-pgroup",
                verified: true
            }
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn supervised_timeout_kills_background_descendants_that_outlive_the_child() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("growth");
        let mut spec = shell_spec(
            "(while :; do printf x >> \"$EVAL_MARKER\"; done) & exec /bin/sleep 15",
            Duration::from_millis(300),
        );
        spec.env.insert("EVAL_MARKER".into(), marker.clone().into());
        let outcome = ProcessSupervisor::run(&spec, &CancellationToken::new()).await;

        assert_eq!(outcome.exit, ExitState::TimedOut);
        let observed = std::fs::metadata(&marker)
            .map(|m| m.len())
            .expect("descendant must have produced evidence before the kill");
        assert!(observed > 0);
        // The background descendant is killed with the group: after grace, the
        // marker must stop growing.
        tokio::time::sleep(Duration::from_millis(750)).await;
        let settled = std::fs::metadata(&marker)
            .map(|m| m.len())
            .unwrap_or(observed);
        assert_eq!(
            settled, observed,
            "descendant kept writing after group termination"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn supervised_cancellation_settles_as_cancelled_with_verified_cleanup() {
        let token = CancellationToken::new();
        let cancel_later = {
            let token = token.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(150)).await;
                token.cancel();
            })
        };
        let start = std::time::Instant::now();
        let spec = shell_spec("exec /bin/sleep 15", Duration::from_secs(20));
        let outcome = ProcessSupervisor::run(&spec, &token).await;
        cancel_later.abort();

        assert_eq!(outcome.exit, ExitState::Cancelled);
        assert!(start.elapsed() < Duration::from_secs(5));
        assert_eq!(
            outcome.cleanup,
            CleanupEvidence::TreeTerminated {
                layer: "unix-pgroup",
                verified: true
            }
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn supervised_cancel_racing_natural_exit_never_corrupts_settlement() {
        // Both futures ready at once: either settlement is legal, but the run
        // must terminate promptly with a consistent (state, cleanup) pair.
        let token = CancellationToken::new();
        token.cancel();
        let spec = shell_spec("/bin/true", Duration::from_secs(20));
        let outcome = tokio::time::timeout(
            Duration::from_secs(5),
            ProcessSupervisor::run(&spec, &token),
        )
        .await
        .expect("racing cancel must not hang settlement");

        match outcome.exit {
            ExitState::Exited { code: 0 } => {
                assert_eq!(outcome.cleanup, CleanupEvidence::NotRequired)
            }
            ExitState::Cancelled => assert!(matches!(
                outcome.cleanup,
                CleanupEvidence::TreeTerminated { verified: true, .. }
            )),
            other => panic!("unexpected settlement {other:?}"),
        }
    }

    #[tokio::test]
    async fn supervised_spawn_failures_settle_as_redacted_typed_reasons() {
        let dir = tempfile::tempdir().unwrap();
        let noexec = dir.path().join("opi-eval-probe-noexec");
        std::fs::write(&noexec, b"#!nonexec\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&noexec, std::fs::Permissions::from_mode(0o644)).unwrap();
        }

        let base = shell_spec("/bin/true", Duration::from_secs(20));
        let cases = [
            (
                {
                    let mut s = base.clone();
                    s.argv = vec!["/nonexistent/opi-eval-probe-binary".into()];
                    s
                },
                SpawnReason::NotFound,
            ),
            (
                {
                    let mut s = base.clone();
                    s.cwd = Some(PathBuf::from("/nonexistent/opi-eval-probe-cwd"));
                    s
                },
                SpawnReason::BadCwd,
            ),
            (
                {
                    let mut s = base.clone();
                    s.argv = vec![noexec.clone().into()];
                    s
                },
                #[cfg(unix)]
                SpawnReason::PermissionDenied,
                #[cfg(not(unix))]
                SpawnReason::SpawnFailed,
            ),
        ];

        for (spec, reason) in &cases {
            let outcome = ProcessSupervisor::run(spec, &CancellationToken::new()).await;
            assert_eq!(
                outcome.exit,
                ExitState::FailedToSpawn { reason: *reason },
                "argv0={:?} cwd={:?}",
                spec.argv[0],
                spec.cwd
            );
            assert_eq!(outcome.cleanup, CleanupEvidence::NotRequired);
            assert_eq!(outcome.stdout, OutputCapture::default());
            assert_eq!(outcome.stderr, OutputCapture::default());
        }

        // Redaction: the settled outcome never echoes the failing argv or cwd.
        let outcome = ProcessSupervisor::run(&cases[0].0, &CancellationToken::new()).await;
        let rendered = format!("{outcome:?}");
        assert!(!rendered.contains("probe-binary"));
        assert!(!rendered.contains("probe-cwd"));
        assert!(!rendered.contains("noexec"));
    }

    #[tokio::test]
    async fn supervised_empty_argv_settles_failed_to_spawn_without_panicking() {
        let spec = SpawnSpec {
            argv: vec![],
            cwd: None,
            env: BTreeMap::new(),
            stdout_cap: 16,
            stderr_cap: 16,
            timeout: Duration::from_secs(5),
        };
        let outcome = tokio::time::timeout(
            Duration::from_secs(5),
            ProcessSupervisor::run(&spec, &CancellationToken::new()),
        )
        .await
        .expect("empty argv must settle, not hang or panic");

        assert_eq!(
            outcome.exit,
            ExitState::FailedToSpawn {
                reason: SpawnReason::SpawnFailed,
            }
        );
        assert_eq!(outcome.cleanup, CleanupEvidence::NotRequired);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn supervised_run_settles_exit_cwd_exact_env_and_captured_streams() {
        let dir = tempfile::tempdir().unwrap();
        let make_spec = |argv: Vec<OsString>| SpawnSpec {
            argv,
            cwd: Some(dir.path().to_path_buf()),
            env: BTreeMap::from([("EVAL_PROBE".into(), "42".into())]),
            stdout_cap: 4096,
            stderr_cap: 4096,
            timeout: Duration::from_secs(20),
        };

        // argv[0] is /usr/bin/env itself: its output is EXACTLY the environ the
        // supervisor projected (no shell that could add its own PWD etc.).
        let env_probe = make_spec(vec!["/usr/bin/env".into()]);
        let outcome = ProcessSupervisor::run(&env_probe, &CancellationToken::new()).await;
        assert_eq!(outcome.exit, ExitState::Exited { code: 0 });
        assert_eq!(outcome.cleanup, CleanupEvidence::NotRequired);
        assert!(!outcome.stdout.truncated);
        assert_eq!(outcome.stdout.bytes, b"EVAL_PROBE=42\n");
        assert_eq!(outcome.stderr, OutputCapture::default());

        // cwd is honored by a second supervised run.
        let cwd_probe = make_spec(vec!["/bin/pwd".into(), "-P".into()]);
        let outcome = ProcessSupervisor::run(&cwd_probe, &CancellationToken::new()).await;
        assert_eq!(outcome.exit, ExitState::Exited { code: 0 });
        let stdout = String::from_utf8(outcome.stdout.bytes).unwrap();
        assert_eq!(
            PathBuf::from(stdout.trim_end()),
            dir.path().canonicalize().unwrap()
        );

        // A failing child settles as a plain non-zero exit with captured stderr.
        let missing = dir.path().join("definitely-missing-file");
        let fail_probe = make_spec(vec!["/bin/cat".into(), missing.clone().into()]);
        let outcome = ProcessSupervisor::run(&fail_probe, &CancellationToken::new()).await;
        assert_eq!(outcome.exit, ExitState::Exited { code: 1 });
        assert!(!outcome.stderr.bytes.is_empty());
        assert_eq!(outcome.stdout.bytes, b"");
    }
}
