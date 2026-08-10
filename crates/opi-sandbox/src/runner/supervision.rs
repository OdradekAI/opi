//! Child supervision, output capture, and bounded terminal cleanup.

use super::preparation::OwnerDeathCleanup;
use super::*;

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
pub(super) async fn supervise(
    mut child: Child,
    mut tree: TreeGuard,
    temp: tempfile::TempDir,
    owner_death_cleanup: Option<OwnerDeathCleanup>,
    temp_root: PathBuf,
    control: SupervisionControl,
) -> SandboxResult {
    let SupervisionControl {
        deadline_cell,
        cancel,
        faults,
        event_tx,
    } = control;
    let deadlines = *deadline_cell
        .get()
        .expect("execution deadline armed before supervision");
    let execution_deadline = deadlines.execution_deadline_at(Instant::now());
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let mut drain_out =
        CaptureTask::new(stdout, OUTPUT_CAP, OutputStream::Stdout, event_tx.clone());
    let mut drain_err = CaptureTask::new(stderr, OUTPUT_CAP, OutputStream::Stderr, event_tx);
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
    cleanup_confirmed &=
        finish_owner_death_cleanup_until(owner_death_cleanup, deadlines.cleanup).await;

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

async fn finish_owner_death_cleanup_until(
    cleanup: Option<OwnerDeathCleanup>,
    deadline: Instant,
) -> bool {
    let Some(cleanup) = cleanup else {
        return true;
    };
    let finish = tokio::task::spawn_blocking(move || cleanup.finish());
    matches!(
        tokio::time::timeout_at(deadline, finish).await,
        Ok(Ok(true))
    )
}

pub(super) struct SupervisionControl {
    pub(super) deadline_cell: Arc<OnceLock<RunDeadlines>>,
    pub(super) cancel: CancellationToken,
    pub(super) faults: FaultInjection,
    pub(super) event_tx: mpsc::Sender<SandboxEvent>,
}

pub(super) async fn remove_temp_root_until(
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
    /// Spawn the drain for `stream`, retaining a preview of at most `cap`
    /// bytes while relaying every read chunk through the bounded event channel.
    fn new<R>(
        stream: Option<R>,
        cap: usize,
        output_stream: OutputStream,
        event_tx: mpsc::Sender<SandboxEvent>,
    ) -> Self
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
                            let _ = event_tx
                                .send(SandboxEvent::Diagnostic {
                                    message: match output_stream {
                                        OutputStream::Stdout => {
                                            "stdout stream read failed; terminal preview may be incomplete"
                                        }
                                        OutputStream::Stderr => {
                                            "stderr stream read failed; terminal preview may be incomplete"
                                        }
                                    }
                                    .to_string(),
                                })
                                .await;
                            break;
                        }
                        Ok(n) => {
                            {
                                let mut state = lock_capture(&task_state);
                                if state.bytes.len() < cap {
                                    let take = std::cmp::min(n, cap - state.bytes.len());
                                    state.bytes.extend_from_slice(&chunk[..take]);
                                    state.truncated |= take < n;
                                } else {
                                    state.truncated = true;
                                }
                            }
                            if event_tx
                                .send(SandboxEvent::Output {
                                    stream: output_stream,
                                    bytes: chunk[..n].to_vec(),
                                })
                                .await
                                .is_err()
                            {
                                break;
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
