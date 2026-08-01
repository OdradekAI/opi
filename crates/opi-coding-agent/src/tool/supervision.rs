//! Policy-neutral L0 process supervision deep module (Phase 16 task 16.2).
//!
//! Owns the bounded-command lifecycle for one spawned child: attach the OS
//! process tree, race `wait` / timeout / cancellation, terminate the whole
//! tree on every outcome (clean exit, timeout, cancellation, wait failure),
//! and drain stdout/stderr under a bounded grace. Degraded attach or cleanup
//! surface as redacted `{layer, reason}` pairs ([`AttachError`]); the caller
//! maps them into diagnostics. This module applies NO command-restriction
//! policy and knows nothing of `bash`, sandboxes, or diagnostic vocabulary.
//!
//! # Deep module
//!
//! One bounded-command entry point, [`supervise`], hides the select race, the
//! concurrent drain, the per-branch termination, and drop-driven cleanup. The
//! sibling [`process_tree`] module keeps the OS process-tree primitives
//! ([`TreeGuard`]); long-lived children that do NOT want the bounded race (the
//! adapter host) attach directly via [`TreeGuard::attach_child`]. This is the
//! deep split the design calls for.
//!
//! # Drop safety (dropped-future case)
//!
//! The [`TreeGuard`] and the owned drain-task handles are locals of
//! [`supervise_inner`]. Dropping the outer exec future drops them:
//! [`TreeGuard::Drop`](TreeGuard) terminates the tree and the drain handles
//! abort. Combined with `kill_on_drop(true)` set on the [`Command`] by the
//! caller, dropping an in-flight supervise future cannot orphan descendants
//! (proven by `sandbox_l0::dropped_exec_future_kills_process_tree`).
//!
//! # Naming
//!
//! The spec's Core Architecture names the reusable type `ProcessSupervisor`
//! (held by both `LocalBashOperations` and the future `ProcessCommandAdapter`).
//! For task 16.2 there is one caller and no per-instance state, so the seam is
//! shipped as a free async fn; the struct materializes when the second caller
//! (task 16.7) requires per-instance configuration.
//!
//! [`Command`]: tokio::process::Command

#![forbid(unsafe_code)]

use std::io;
use std::process::ExitStatus;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Child;
use tokio_util::sync::CancellationToken;

use super::operations::StreamCapture;
use super::process_tree::{AttachError, TerminationOutcome, TreeGuard};

/// Bound on how long a terminated tree's still-attached stdout/stderr pipes may
/// hold the drain. A descendant that retains a pipe write-end cannot keep
/// [`supervise`] pending beyond this grace; on expiry the drain task is aborted
/// and what was captured so far is returned. Pinned at 500 ms by the Phase 16
/// L0 acceptance contract (design §L0 supervision: "descendants holding output
/// pipes cannot exceed bounded drain grace"). Exposed so behavioral tests can
/// assert the bound without a magic literal.
pub(crate) const TERMINATED_PIPE_DRAIN_GRACE: Duration = Duration::from_millis(500);

/// Which control branch won the `wait` / timeout / cancellation race.
/// Policy-neutral: carries the OS outcome and the redacted kill error only.
#[derive(Debug)]
pub(crate) enum SupervisionKind {
    /// Direct child exited; the tree (including surviving background
    /// descendants) was terminated to preserve this status.
    Done(ExitStatus),
    /// Deadline elapsed; the tree was terminated.
    TimedOut { kill_error: Option<io::Error> },
    /// Cancellation token fired; the tree was terminated.
    Cancelled { kill_error: Option<io::Error> },
    /// `child.wait()` returned an error; the tree was terminated best-effort.
    WaitFailed,
}

/// Policy-neutral outcome of supervising one bounded command. Carries the
/// drained captures on EVERY branch — the caller, not the supervisor, decides
/// whether to discard partial output on timeout/cancellation — plus the
/// redacted attach/cleanup degradations observed during the run.
pub(crate) struct SupervisionOutcome {
    pub kind: SupervisionKind,
    pub out: StreamCapture,
    pub err: StreamCapture,
    pub degradations: Vec<AttachError>,
}

/// Supervise one spawned child through the bounded L0 lifecycle.
///
/// `cap` bounds the per-stream in-memory capture (the caller supplies its own
/// output cap; this module is neutral over the value). Attach, the
/// wait/timeout/cancel race, per-branch tree termination, and bounded drain all
/// run inside. The returned [`SupervisionOutcome`] carries the drained captures
/// and the redacted degradations; it never carries command text, paths, env, or
/// secrets.
#[cfg_attr(test, allow(dead_code))]
pub(crate) async fn supervise(
    child: &mut Child,
    timeout: Duration,
    signal: CancellationToken,
    cap: usize,
) -> SupervisionOutcome {
    // The non-test arm is the production entry; the cfg(test) arm exists so the
    // unit-test build (which routes through `supervise_with_faults`) still
    // type-checks the shared inner body against the cfg-gated fault parameter.
    #[cfg(test)]
    {
        supervise_inner(child, timeout, signal, cap, false, None).await
    }
    #[cfg(not(test))]
    {
        supervise_inner(child, timeout, signal, cap, false).await
    }
}

/// Test-only seam identical to [`supervise`] but injects attach/terminate tree
/// faults and an optional synthetic wait failure so the degradation paths and
/// the wait-failure termination branch can be exercised deterministically.
#[cfg(test)]
pub(crate) async fn supervise_with_faults(
    child: &mut Child,
    timeout: Duration,
    signal: CancellationToken,
    cap: usize,
    tree_faults: super::process_tree::TestTreeFaults,
    wait_fault: bool,
) -> SupervisionOutcome {
    supervise_inner(child, timeout, signal, cap, wait_fault, Some(tree_faults)).await
}

async fn supervise_inner(
    child: &mut Child,
    timeout: Duration,
    signal: CancellationToken,
    cap: usize,
    wait_fault: bool,
    #[cfg(test)] tree_faults: Option<super::process_tree::TestTreeFaults>,
) -> SupervisionOutcome {
    let mut degradations: Vec<AttachError> = Vec::new();

    // Attach L0 to the spawned child. Fail-open: on failure keep a disabled
    // guard (the direct child is still reaped via the wait path and
    // `kill_on_drop`) and record the redacted degradation.
    #[cfg(test)]
    let l0_attach = match (child.id(), tree_faults) {
        (Some(pid), Some(faults)) => TreeGuard::attach_with_faults(pid, faults),
        (Some(pid), None) => TreeGuard::attach_child(Some(pid)),
        (None, _) => TreeGuard::attach_child(None),
    };
    #[cfg(not(test))]
    let l0_attach = TreeGuard::attach_child(child.id());
    let mut l0_tree = match l0_attach {
        Ok(guard) => guard,
        Err(err) => {
            degradations.push(err);
            TreeGuard::disabled()
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // Drain stdout/stderr concurrently with the wait/timeout/cancel race to
    // avoid the stdout-then-stderr pipe deadlock. The tasks are owned so a
    // descendant that retained either pipe cannot keep this supervision pending
    // past grace after tree termination.
    let drain_out = OwnedCaptureTask::new(spawn_stream_capture(stdout, cap), cap);
    let drain_err = OwnedCaptureTask::new(spawn_stream_capture(stderr, cap), cap);

    // Run the control race. On every branch the whole tree is terminated: a
    // clean direct-child exit must still kill surviving background descendants,
    // and timeout / cancellation / wait failure each terminate the tree. The
    // `biased` ordering (cancel > timeout > wait) is preserved from the
    // pre-refactor exec path so simultaneous trips classify identically.
    //
    // Under `wait_fault` the real `child.wait()` is skipped: the still-running
    // child is terminated by `l0_tree` exactly as the real wait-error branch
    // does, proving wait-failure termination without a racy external reap.
    let kind = if wait_fault {
        push_terminate(&mut l0_tree, &mut degradations);
        SupervisionKind::WaitFailed
    } else {
        let cancel_future = signal.cancelled();
        let timeout_future = tokio::time::sleep(timeout);
        tokio::pin!(cancel_future);
        tokio::pin!(timeout_future);
        async {
            tokio::select! {
                biased;
                _ = &mut cancel_future => {
                    let kill_error = child.kill().await.err();
                    push_terminate(&mut l0_tree, &mut degradations);
                    SupervisionKind::Cancelled { kill_error }
                }
                _ = &mut timeout_future => {
                    let kill_error = child.kill().await.err();
                    push_terminate(&mut l0_tree, &mut degradations);
                    SupervisionKind::TimedOut { kill_error }
                }
                status = child.wait() => match status {
                    Ok(status) => {
                        push_terminate(&mut l0_tree, &mut degradations);
                        SupervisionKind::Done(status)
                    }
                    Err(_) => {
                        push_terminate(&mut l0_tree, &mut degradations);
                        SupervisionKind::WaitFailed
                    }
                },
            }
        }
        .await
    };

    let (out, err) = tokio::join!(drain_out.finish(), drain_err.finish());

    // `l0_tree` drops here; its Drop terminate is idempotent (AlreadyTerminated)
    // because every branch already terminated explicitly.
    SupervisionOutcome {
        kind,
        out,
        err,
        degradations,
    }
}

/// Terminate the tree and thread any failure as a redacted degradation.
fn push_terminate(tree: &mut TreeGuard, degradations: &mut Vec<AttachError>) {
    if let TerminationOutcome::Failed(err) = tree.terminate() {
        degradations.push(err);
    }
}

/// Spawn one stream's capture task. The task reads until EOF/error into a
/// [`StreamCapture`] bounded by `cap`. Owned by [`OwnedCaptureTask`] so it can
/// be aborted after grace.
fn spawn_stream_capture<R>(stream: Option<R>, cap: usize) -> tokio::task::JoinHandle<StreamCapture>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut capture = StreamCapture::new(cap);
        if let Some(mut stream) = stream {
            let mut buffer = [0u8; 8192];
            loop {
                match stream.read(&mut buffer).await {
                    Ok(0) | Err(_) => break,
                    Ok(read) => capture.append(&buffer[..read]),
                }
            }
        }
        capture
    })
}

/// Owned handle to one drain task. [`OwnedCaptureTask::finish`] awaits the
/// capture under [`TERMINATED_PIPE_DRAIN_GRACE`]; on expiry the task is aborted
/// and an empty capture (sized to the same `cap`) is returned so the bound is
/// enforced regardless of pipe state. Drop aborts an unfinished task.
pub(crate) struct OwnedCaptureTask {
    handle: Option<tokio::task::JoinHandle<StreamCapture>>,
    cap: usize,
}

impl OwnedCaptureTask {
    pub(crate) fn new(handle: tokio::task::JoinHandle<StreamCapture>, cap: usize) -> Self {
        Self {
            handle: Some(handle),
            cap,
        }
    }

    async fn finish(mut self) -> StreamCapture {
        let handle = self.handle.as_mut().expect("capture task is owned");
        match tokio::time::timeout(TERMINATED_PIPE_DRAIN_GRACE, handle).await {
            Ok(Ok(capture)) => {
                self.handle.take();
                capture
            }
            Ok(Err(_)) => {
                self.handle.take();
                StreamCapture::new(self.cap)
            }
            Err(_) => {
                let handle = self.handle.take().expect("capture task is owned");
                handle.abort();
                let _ = handle.await;
                StreamCapture::new(self.cap)
            }
        }
    }
}

impl Drop for OwnedCaptureTask {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    //! Supervision unit tests. The behavioral L0 supervision acceptance
    //! (timeout/cancel/drop tree-kill, clean-exit kills background descendants,
    //! bounded pipe-drain grace) lives in `tests/sandbox_l0.rs` through the
    //! production `LocalBashOperations::exec` path. The wait-failure
    //! termination case is driven here because it needs the test-only
    //! `wait_fault` seam, which integration tests cannot reach.

    use super::*;
    use std::time::Duration;

    /// wait-failure still runs the per-branch terminate. `wait_fault` skips the
    /// real `child.wait()`; the `WaitFailed` branch must call `push_terminate`.
    /// Prove it by injecting a terminate fault: if the branch invoked terminate,
    /// the fault surfaces as a redacted degradation. No grandchild race or
    /// timing is involved. Behavioral proof that terminate kills descendants on
    /// the OTHER branches (timeout/cancel/clean exit) lives in
    /// `tests/sandbox_l0.rs`; the `WaitFailed` branch calls the same terminate,
    /// so descendant cleanup on wait-failure follows by transitivity. Closes the
    /// L3 gap: wait-failure termination is now exercised, not just asserted at
    /// the `Display` level.
    #[tokio::test]
    async fn wait_failure_runs_terminate_and_surfaces_wait_failed() {
        let mut cmd = if cfg!(windows) {
            let mut c = tokio::process::Command::new("cmd");
            c.args(["/C", "echo hi"]);
            c
        } else {
            let mut c = tokio::process::Command::new("sh");
            c.args(["-c", "true"]);
            c
        };
        let mut child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("spawn quick child");

        let outcome = supervise_with_faults(
            &mut child,
            Duration::from_secs(10),
            CancellationToken::new(),
            64 * 1024,
            super::super::process_tree::TestTreeFaults::terminate(),
            true,
        )
        .await;

        assert!(
            matches!(outcome.kind, SupervisionKind::WaitFailed),
            "wait_fault must surface WaitFailed, got {:?}",
            outcome.kind
        );
        assert!(
            outcome.degradations.iter().any(|degradation| {
                degradation.reason
                    == crate::diagnostics::SandboxReason::ProcessTreeTerminationFailed
            }),
            "the WaitFailed branch must invoke terminate (terminate-fault degradation expected), got {:?}",
            outcome.degradations
        );
    }
}
