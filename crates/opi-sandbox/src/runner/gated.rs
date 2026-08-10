//! Gated spawn, start confirmation, target release, and run-stream state.

use super::supervision::{SupervisionControl, supervise};
use super::*;

pub(super) struct StartProbe {
    pub(super) path: PathBuf,
    pub(super) token: Vec<u8>,
}

impl StartProbe {
    pub(super) fn new(release_gate: &Path) -> Result<Self, rand::Error> {
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

impl SandboxRunner {
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
            owner_death_cleanup,
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
        let (event_tx, event_rx) = mpsc::channel(OUTPUT_EVENT_CAPACITY);
        let inner = Box::pin(supervise(
            child,
            tree,
            temp,
            owner_death_cleanup,
            temp_root.clone(),
            SupervisionControl {
                deadline_cell: Arc::clone(&deadline_cell),
                cancel: cancel.clone(),
                faults,
                event_tx,
            },
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
            event_rx,
            terminal_result: None,
            inner: Some(inner),
        };
        SpawnPreparedOutcome::Spawned(Box::new(SpawnedSandboxRun { run, expired }))
    }
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
        loop {
            // Output/diagnostic events always precede completion. Once the
            // supervision future is ready, retain its result until every
            // already-sent channel item has been observed and all senders have
            // closed; this prevents a final chunk racing the terminal event.
            match Pin::new(&mut self.event_rx).poll_recv(cx) {
                Poll::Ready(Some(event)) => return Poll::Ready(Some(event)),
                Poll::Ready(None) if self.terminal_result.is_some() => {
                    self.completed = true;
                    return Poll::Ready(Some(SandboxEvent::Completed(
                        self.terminal_result
                            .take()
                            .expect("terminal result checked above"),
                    )));
                }
                Poll::Ready(None) | Poll::Pending => {}
            }

            let Some(inner) = self.inner.as_mut() else {
                return Poll::Pending;
            };
            match inner.as_mut().poll(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(result) => {
                    self.inner = None;
                    self.terminal_result = Some(result);
                }
            }
        }
    }
}

pub(super) fn acknowledgement_path(release_gate: &Path) -> PathBuf {
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

/// Build a platform-native bootstrap that waits on the invocation-owned release
/// gate before it invokes the real target. A restriction launcher, when present,
/// remains the outermost process so it confines the bootstrap and target alike.
pub(super) fn gated_command(
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
