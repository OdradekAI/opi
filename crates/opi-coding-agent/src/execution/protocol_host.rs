//! The one-shot `command.execute` protocol host (Phase 16.7).
//!
//! [`ExecutionProtocolHost::execute`] launches ONE supervised backend process
//! per request and drives the closed `command-execution-jsonl-v1` state machine
//! over its stdio: stdin carries host->backend frames, stdout carries
//! backend->host frames, and stderr is bounded out-of-band crash evidence only.
//!
//! It composes the Phase 16.2 policy-neutral `TreeGuard` (long-lived-child
//! path: attach on spawn, terminate on cancel/deadline/drop) and the Phase 16.3
//! `opi_protocol::execution::v1` codec. It does NOT implement `BashOperations`
//! (16.8 wires routing), does NOT touch startup (16.9), and has NO fallback to
//! `local` or `opi-extension-jsonl-v1`. There is no degraded-success state.
//!
//! See the Phase 16 design (`docs/superpowers/specs/2026-07-...-phase16-...md`):
//! State machine (§`opi-protocol`), Process and transport, Cancellation and
//! cleanup, Failure and Diagnostics.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio_util::sync::CancellationToken;

// Re-exported at the `execution::v1` root.
use opi_protocol::execution::v1::{
    BackendToHost, Bounds, CancelReason, CleanupState, Diagnostic, EnvInherit, FailureCode,
    HostToBackend, ImplementationId, NativeString, ProtocolId, RequestId, Session, TargetId,
    WIRE_IDENTITY,
};
// NOT re-exported at the root -> addressed by module path.
use opi_protocol::execution::v1::codec::encode_host;
use opi_protocol::execution::v1::frames::{
    CancelPayload, CompletedPayload, ExecutePayload, FailedPayload, InitializePayload,
};

#[cfg(windows)]
use crate::tool::process_tree::resume_child;
use crate::tool::process_tree::{TreeGuard, configure_tree};

use super::failure::ExecutionFailure;

/// Monotonic request-id counter (host-generated ids; no RNG required).
static REQUEST_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Grace granted to the backend to report a terminal cleanup state after the
/// host sends `cancel`, and to reap the backend process after `completed`.
/// Distinct from `supervision::TERMINATED_PIPE_DRAIN_GRACE` (the post-kill pipe
/// drain bound). Sourced from the Phase 16 design §Cancellation and cleanup.
const CLEANUP_REPORT_GRACE: Duration = Duration::from_millis(1500);

/// Per-write timeout for every host->backend stdin frame. Bounds a wedged
/// backend that is not draining its stdin (preventing a hung `execute`). After a
/// write timeout the host proceeds to cancel/grace/terminate; it never loops
/// back into a blocking write.
const WRITE_TIMEOUT: Duration = Duration::from_millis(500);

/// Bound on retained backend PROCESS stderr (crash evidence). The pipe is always
/// drained concurrently (to avoid deadlock); only the first `STDERR_CAP` bytes
/// are retained and they are NEVER surfaced into [`ExecutionFailure`] (redaction:
/// the envelope is payload-free). Mirror of `adapter_host.rs`'s tracing-only
/// stderr handling.
const STDERR_CAP: usize = 64 * 1024;

/// What to spawn as the backend (the locked executable). Command and
/// configuration travel in protocol frames, NEVER in these args.
pub struct BackendLaunch<'a> {
    pub program: &'a Path,
    pub args: &'a [String],
    /// Keeps the exact validated executable open until `spawn` returns. On
    /// Unix `program` addresses this descriptor; on Windows the handle denies
    /// replacement of the validated pathname.
    pub validated_executable: &'a std::fs::File,
}

/// One execution request. The HOST owns `map_shell_command`: `command` is the
/// raw bash shell string, mapped to an explicit platform shell program + arg
/// vector before the `execute` frame is sent.
pub struct ExecutionRequest<'a> {
    /// Raw bash shell string.
    pub command: &'a str,
    /// Canonical workspace root.
    pub workspace: &'a Path,
    /// Working directory inside the workspace.
    pub cwd: &'a Path,
    /// Execution timeout (-> `execute.timeout_ms`).
    pub timeout: Duration,
    /// ONE end-to-end deadline covering startup -> cleanup.
    pub deadline: Duration,
    /// Maximum time allowed for initialize/ready negotiation.
    pub handshake_timeout: Duration,
    /// Locked identity/version/target that `ready` must match exactly.
    pub expected_implementation: &'a str,
    pub expected_implementation_version: &'a str,
    pub expected_target: &'a str,
    /// Environment-inheritance policy.
    pub env_inherit: EnvInherit,
    /// Bounded environment additions (native keys/values).
    pub env_additions: &'a BTreeMap<NativeString, NativeString>,
    /// Bounded adapter configuration (sized by the codec on `initialize`).
    pub adapter_config: serde_json::Value,
    /// The host's ordered supported-protocol list (preference order).
    pub supported_protocols: &'a [ProtocolId],
    /// External cancellation token. Cancel -> `cancel(Canceled)`.
    pub signal: CancellationToken,
    /// Protocol bounds (codec line/message/config/diagnostics/cumulative caps).
    pub bounds: Bounds,
}

/// Negotiation result captured from `ready`.
#[derive(Debug, Clone)]
pub struct ReadyReport {
    pub selected_protocol: ProtocolId,
    pub implementation: ImplementationId,
    pub implementation_version: String,
    pub target: TargetId,
}

/// Effective contract captured from `started` (reported before target release).
#[derive(Debug, Clone, Default)]
pub struct StartedReport {
    pub placement: String,
    pub guarantee: String,
    pub policy: String,
    pub limitations: Vec<String>,
}

/// Terminal in-band result. Nonzero exit, signal, `timed_out`, and `cancelled`
/// are IN-BAND here (Ok), not failures. `cleanup` is always `Confirmed` on the
/// Ok path: a `Completed{cleanup:Unconfirmed}` maps to
/// [`ExecutionFailure::CleanupUnconfirmed`] (no degraded success).
#[derive(Debug, Clone)]
pub struct CompletedOutcome {
    pub ready: ReadyReport,
    pub started: StartedReport,
    pub exit: Option<u32>,
    pub signal: Option<u32>,
    pub timed_out: bool,
    pub cancelled: bool,
    pub cleanup: CleanupState,
    /// Decoded target stdout (from `Stdout` frames).
    pub stdout: Vec<u8>,
    /// Decoded target stderr (from `Stderr` frames).
    pub stderr: Vec<u8>,
    pub diagnostics: Vec<Diagnostic>,
}

/// A failed protocol execution plus any redacted in-band diagnostics emitted
/// before or on its terminal `failed` frame.
#[derive(Debug)]
pub struct ExecutionProtocolFailure {
    pub failure: ExecutionFailure,
    pub diagnostics: Vec<Diagnostic>,
}

impl ExecutionProtocolFailure {
    fn with_diagnostics(failure: ExecutionFailure, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            failure,
            diagnostics,
        }
    }

    pub fn code(&self) -> &'static str {
        self.failure.code()
    }

    pub fn remediation(&self) -> String {
        self.failure.remediation()
    }
}

impl From<ExecutionFailure> for ExecutionProtocolFailure {
    fn from(failure: ExecutionFailure) -> Self {
        Self::with_diagnostics(failure, Vec::new())
    }
}

impl std::fmt::Display for ExecutionProtocolFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.failure.fmt(f)
    }
}

impl std::error::Error for ExecutionProtocolFailure {}

/// The one-shot execution protocol host.
pub struct ExecutionProtocolHost;

impl ExecutionProtocolHost {
    /// Launch one supervised backend process and drive the protocol to a
    /// terminal frame. See the module docs for the determinism, no-degraded-
    /// success, and no-local-fallback guarantees.
    pub async fn execute(
        launch: BackendLaunch<'_>,
        request: ExecutionRequest<'_>,
    ) -> Result<CompletedOutcome, ExecutionProtocolFailure> {
        // The one invocation clock starts before backend spawn; every later
        // negotiation, execution, cancellation, drain, and reap window is
        // derived from this absolute deadline.
        let start = tokio::time::Instant::now();
        let hard_deadline = start
            .checked_add(request.deadline)
            .ok_or(ExecutionFailure::ExecutionFailed)?;
        let cancel_at = hard_deadline
            .checked_sub(CLEANUP_REPORT_GRACE)
            .unwrap_or(start);
        let handshake_deadline = std::cmp::min(
            cancel_at,
            start
                .checked_add(request.handshake_timeout)
                .unwrap_or(hard_deadline),
        );
        let request_id = RequestId::new(format!(
            "opi-exec-{}",
            REQUEST_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
        ))
        .expect("generated request id is non-empty");
        let bounds = request.bounds;
        let mut session = Session::new(bounds).map_err(|_| ExecutionFailure::ProtocolViolation)?;

        // --- spawn (no await between spawn and attach: closes the drop window) ---
        let mut cmd = tokio::process::Command::new(launch.program);
        let _validated_executable = launch.validated_executable;
        cmd.args(launch.args);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        configure_tree(&mut cmd);
        let mut child = cmd.spawn().map_err(|_| ExecutionFailure::ExecutionFailed)?;

        // --- attach the L0 tree guard (fail-closed: the kill guarantee is required) ---
        let child_pid = child.id();
        let guard = match TreeGuard::attach_child(child_pid) {
            Ok(guard) => guard,
            Err(_) => {
                let _ = child.start_kill();
                let _ = tokio::time::timeout_at(hard_deadline, child.wait()).await;
                return Err(ExecutionFailure::ExecutionFailed.into());
            }
        };
        #[cfg(windows)]
        let mut guard = guard;
        #[cfg(windows)]
        if child_pid
            .ok_or(ExecutionFailure::ExecutionFailed)
            .and_then(|pid| resume_child(pid).map_err(|_| ExecutionFailure::ExecutionFailed))
            .is_err()
        {
            let _ = guard.terminate();
            let _ = child.start_kill();
            let _ = tokio::time::timeout_at(hard_deadline, child.wait()).await;
            return Err(ExecutionFailure::ExecutionFailed.into());
        }
        let mut stdin = child.stdin.take().expect("piped stdin present");
        let stdout = child.stdout.take().expect("piped stdout present");
        let stderr = child.stderr.take().expect("piped stderr present");

        // --- concurrent bounded stderr drain (crash evidence only; never surfaced) ---
        let stderr_handle = tokio::spawn(drain_stderr(stderr));
        let mut reader = CappedReader::new(stdout, bounds.max_line_size);

        // accumulated state for the eventual outcome
        let mut started = StartedReport::default();
        let mut stdout_acc: Vec<u8> = Vec::new();
        let mut stderr_acc: Vec<u8> = Vec::new();
        let mut diagnostics: Vec<Diagnostic> = Vec::new();
        let mut state = HostState::new(HostPhase::AwaitingReady);

        // --- initialize (seed the session with the HOST id by observing it first) ---
        let init = HostToBackend::Initialize(InitializePayload {
            request_id: request_id.clone(),
            deadline_ms: u64::try_from(remaining_until(hard_deadline).as_millis())
                .unwrap_or(u64::MAX),
            adapter_config: request.adapter_config.clone(),
            supported_protocols: request.supported_protocols.to_vec(),
        });
        if session.observe_host(&init).is_err()
            || write_frame(&mut stdin, bounds, &init, hard_deadline)
                .await
                .is_err()
        {
            return terminate_and_fail(
                child,
                guard,
                stderr_handle,
                stdin,
                ExecutionFailure::ProtocolViolation,
                hard_deadline,
            )
            .await;
        }

        // --- ready (command not disclosed until ready validates) ---
        let placeholder_ready =
            || ReadyReport {
                selected_protocol: request.supported_protocols.first().cloned().unwrap_or_else(
                    || ProtocolId::new(WIRE_IDENTITY).expect("v1 wire identity is non-empty"),
                ),
                implementation: ImplementationId::new(request.expected_implementation)
                    .expect("validated selected adapter identity is non-empty"),
                implementation_version: String::new(),
                target: TargetId::new(""),
            };
        let ready = match read_frame_select(
            &mut reader,
            &mut session,
            &request.signal,
            handshake_deadline,
        )
        .await
        {
            FrameSel::Frame(frame) => match transition(&mut state, &frame) {
                Ok(Action::Continue) => match frame {
                    BackendToHost::Ready(p) => p,
                    _ => unreachable!("only ready advances the pre-ready state"),
                },
                Ok(Action::Terminal(terminal)) => {
                    return finalize_terminal(
                        terminal,
                        child,
                        guard,
                        stderr_handle,
                        stdin,
                        placeholder_ready(),
                        started,
                        stdout_acc,
                        stderr_acc,
                        diagnostics,
                        hard_deadline,
                        &mut reader,
                    )
                    .await;
                }
                Err(e) => {
                    return terminate_and_fail(
                        child,
                        guard,
                        stderr_handle,
                        stdin,
                        e,
                        hard_deadline,
                    )
                    .await;
                }
            },
            FrameSel::Canceled(reason) => {
                return finish_with_cancel(
                    stdin,
                    &mut reader,
                    &mut session,
                    &mut state,
                    child,
                    guard,
                    stderr_handle,
                    bounds,
                    &request_id,
                    hard_deadline,
                    reason,
                    placeholder_ready(),
                    started,
                    stdout_acc,
                    stderr_acc,
                    diagnostics,
                )
                .await;
            }
            FrameSel::Eof | FrameSel::Codec(_) => {
                return terminate_and_fail(
                    child,
                    guard,
                    stderr_handle,
                    stdin,
                    ExecutionFailure::ProtocolViolation,
                    hard_deadline,
                )
                .await;
            }
        };
        if !request
            .supported_protocols
            .iter()
            .any(|p| p == &ready.selected_protocol)
        {
            return terminate_and_fail(
                child,
                guard,
                stderr_handle,
                stdin,
                ExecutionFailure::ProtocolIncompatible,
                hard_deadline,
            )
            .await;
        }
        if ready.implementation.as_str() != request.expected_implementation
            || ready.implementation_version != request.expected_implementation_version
            || ready.target.as_str() != request.expected_target
        {
            return terminate_and_fail(
                child,
                guard,
                stderr_handle,
                stdin,
                ExecutionFailure::ProtocolIncompatible,
                hard_deadline,
            )
            .await;
        }
        let ready_report = ReadyReport {
            selected_protocol: ready.selected_protocol.clone(),
            implementation: ready.implementation.clone(),
            implementation_version: ready.implementation_version.clone(),
            target: ready.target.clone(),
        };

        // --- execute (map the bash shell string host-side) ---
        let (program, args) = map_shell_command(request.command);
        let exec_frame = HostToBackend::Execute(ExecutePayload {
            request_id: request_id.clone(),
            program,
            args,
            workspace: native_path(request.workspace),
            cwd: native_path(request.cwd),
            timeout_ms: u64::try_from(request.timeout.min(remaining_until(cancel_at)).as_millis())
                .unwrap_or(u64::MAX),
            env_inherit: request.env_inherit,
            env_additions: request.env_additions.clone(),
        });
        if session.observe_host(&exec_frame).is_err()
            || write_frame(&mut stdin, bounds, &exec_frame, hard_deadline)
                .await
                .is_err()
        {
            return terminate_and_fail(
                child,
                guard,
                stderr_handle,
                stdin,
                ExecutionFailure::ProtocolViolation,
                hard_deadline,
            )
            .await;
        }

        // --- main frame loop (host-side transition ordering + accumulation) ---
        loop {
            match read_frame_select(&mut reader, &mut session, &request.signal, cancel_at).await {
                FrameSel::Canceled(reason) => {
                    return finish_with_cancel(
                        stdin,
                        &mut reader,
                        &mut session,
                        &mut state,
                        child,
                        guard,
                        stderr_handle,
                        bounds,
                        &request_id,
                        hard_deadline,
                        reason,
                        ready_report,
                        started,
                        stdout_acc,
                        stderr_acc,
                        diagnostics,
                    )
                    .await;
                }
                FrameSel::Eof => {
                    return terminate_and_fail(
                        child,
                        guard,
                        stderr_handle,
                        stdin,
                        ExecutionFailure::ProtocolViolation,
                        hard_deadline,
                    )
                    .await;
                }
                FrameSel::Codec(e) => {
                    return terminate_and_fail(
                        child,
                        guard,
                        stderr_handle,
                        stdin,
                        e,
                        hard_deadline,
                    )
                    .await;
                }
                FrameSel::Frame(frame) => match transition(&mut state, &frame) {
                    Ok(Action::Continue) => match frame {
                        BackendToHost::Accepted(_) => {}
                        BackendToHost::Started(p) => {
                            started = StartedReport {
                                placement: p.placement.clone(),
                                guarantee: p.guarantee.clone(),
                                policy: p.policy.clone(),
                                limitations: p.limitations.clone(),
                            };
                        }
                        BackendToHost::Stdout(p) => stdout_acc.extend_from_slice(p.data.as_bytes()),
                        BackendToHost::Stderr(p) => stderr_acc.extend_from_slice(p.data.as_bytes()),
                        BackendToHost::Diagnostic(p) => diagnostics
                            .push(redact_backend_diagnostic(Diagnostic { message: p.message })),
                        _ => {}
                    },
                    Ok(Action::Terminal(terminal)) => {
                        return finalize_terminal(
                            terminal,
                            child,
                            guard,
                            stderr_handle,
                            stdin,
                            ready_report,
                            started,
                            stdout_acc,
                            stderr_acc,
                            diagnostics,
                            hard_deadline,
                            &mut reader,
                        )
                        .await;
                    }
                    Err(e) => {
                        return terminate_and_fail(
                            child,
                            guard,
                            stderr_handle,
                            stdin,
                            e,
                            hard_deadline,
                        )
                        .await;
                    }
                },
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Shell mapping (host-owned; pub(crate) — 16.8 must not reach past the host)
// ---------------------------------------------------------------------------

/// Map a bash shell string to an explicit platform shell program + arg vector,
/// matching `tool::operations::build_bash_command`: Unix `sh -c <command>`,
/// Windows `cmd /C <command>`. The host sends these over the wire as the
/// `execute` program/args; the backend target spawns them.
pub(crate) fn map_shell_command(command: &str) -> (NativeString, Vec<NativeString>) {
    if cfg!(windows) {
        (
            NativeString::from_utf8("cmd"),
            vec![
                NativeString::from_utf8("/C"),
                NativeString::from_utf8(command),
            ],
        )
    } else {
        (
            NativeString::from_utf8("sh"),
            vec![
                NativeString::from_utf8("-c"),
                NativeString::from_utf8(command),
            ],
        )
    }
}

fn native_path(p: &Path) -> NativeString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt as _;
        NativeString::from_bytes(p.as_os_str().as_bytes())
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt as _;
        let bytes = p
            .as_os_str()
            .encode_wide()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        NativeString::from_bytes(bytes)
    }
}

fn redact_backend_diagnostic(diagnostic: Diagnostic) -> Diagnostic {
    Diagnostic {
        message: opi_agent::diagnostic::redact_text(
            &diagnostic.message,
            opi_agent::diagnostic::RedactionMode::Summary,
        ),
    }
}

// ---------------------------------------------------------------------------
// Host-side state machine (Session deliberately does NOT enforce ordering)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HostPhase {
    AwaitingReady,
    AwaitingAccepted,
    AwaitingStarted,
    Draining,
    #[allow(dead_code)]
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HostState {
    phase: HostPhase,
    cancelling: bool,
}

impl HostState {
    const fn new(phase: HostPhase) -> Self {
        Self {
            phase,
            cancelling: false,
        }
    }

    fn begin_cancel(&mut self) {
        self.cancelling = true;
    }
}

#[derive(Debug)]
enum Action {
    Continue,
    Terminal(Terminal),
}

#[derive(Debug)]
enum Terminal {
    /// A `Completed` frame (cleanup state inspected by the caller).
    Completed(CompletedPayload),
    /// A `Failed` frame; mapping is deferred until terminal diagnostics and
    /// clean EOF have been enforced.
    Failed(FailedPayload),
}

/// Validate `frame` against the host state machine and advance state.
/// `Failed` is legal pre-started during normal flow, but once cancellation
/// begins a terminal is legal only after the required milestones reach
/// `started`.
fn transition(state: &mut HostState, frame: &BackendToHost) -> Result<Action, ExecutionFailure> {
    use BackendToHost::*;
    match (state.phase, frame) {
        (HostPhase::AwaitingReady, Ready(_)) if !state.cancelling => {
            state.phase = HostPhase::AwaitingAccepted;
            Ok(Action::Continue)
        }
        (HostPhase::AwaitingAccepted, Accepted(_)) => {
            state.phase = HostPhase::AwaitingStarted;
            Ok(Action::Continue)
        }
        (HostPhase::AwaitingStarted, Started(_)) => {
            state.phase = HostPhase::Draining;
            Ok(Action::Continue)
        }
        (HostPhase::Draining, Stdout(_) | Stderr(_) | Diagnostic(_)) => Ok(Action::Continue),
        (HostPhase::Draining, Completed(p)) => {
            state.phase = HostPhase::Terminal;
            Ok(Action::Terminal(Terminal::Completed(p.clone())))
        }
        // Failed is legal in any pre-terminal state during the normal flow.
        // Once cancellation begins, a pre-started failure cannot bypass the
        // ready -> accepted -> started milestones.
        (
            HostPhase::AwaitingReady
            | HostPhase::AwaitingAccepted
            | HostPhase::AwaitingStarted
            | HostPhase::Draining,
            Failed(p),
        ) if !state.cancelling || state.phase == HostPhase::Draining => {
            state.phase = HostPhase::Terminal;
            Ok(Action::Terminal(Terminal::Failed(p.clone())))
        }
        _ => Err(ExecutionFailure::ProtocolViolation),
    }
}

/// Map a wire `FailureCode` (closed 7-code set) to the architecture envelope.
/// Redacted: drops the optional `message` and diagnostics detail (F7).
fn map_failure_code(p: &FailedPayload, selected_adapter_id: &str) -> ExecutionFailure {
    match p.code {
        FailureCode::ProtocolIncompatible => ExecutionFailure::ProtocolIncompatible,
        FailureCode::ProtocolViolation => ExecutionFailure::ProtocolViolation,
        FailureCode::ExecutionTimedOut => ExecutionFailure::ExecutionTimedOut,
        FailureCode::CleanupUnconfirmed => ExecutionFailure::CleanupUnconfirmed,
        FailureCode::Unavailable => ExecutionFailure::AdapterUnavailable {
            adapter_id: Some(selected_adapter_id.to_string()),
            detail: super::failure::UnavailableDetail::Handshake,
        },
        FailureCode::Failed | FailureCode::ExecutionFailed => ExecutionFailure::ExecutionFailed,
    }
}

// ---------------------------------------------------------------------------
// Frame select (read with cancel + deadline), capped reader, write, drain, reap
// ---------------------------------------------------------------------------

enum FrameSel {
    Frame(BackendToHost),
    Eof,
    Codec(ExecutionFailure),
    Canceled(CancelReason),
}

async fn read_frame_select(
    reader: &mut CappedReader<ChildStdout>,
    session: &mut Session,
    signal: &CancellationToken,
    cancel_at: tokio::time::Instant,
) -> FrameSel {
    tokio::select! {
        biased;
        _ = signal.cancelled() => FrameSel::Canceled(CancelReason::Canceled),
        _ = tokio::time::sleep_until(cancel_at) => FrameSel::Canceled(CancelReason::Deadline),
        r = reader.read_line() => match r {
            Ok(None) => FrameSel::Eof,
            Ok(Some(line)) => match session.feed_backend_line(&line) {
                Ok(f) => FrameSel::Frame(f),
                Err(_) => FrameSel::Codec(ExecutionFailure::ProtocolViolation),
            },
            Err(_) => FrameSel::Codec(ExecutionFailure::ProtocolViolation),
        },
    }
}

/// Encode + write one host frame, timeout-bounded. On any failure (encode,
/// timeout, I/O) returns Err; the caller proceeds to terminate.
async fn write_frame(
    stdin: &mut ChildStdin,
    bounds: Bounds,
    frame: &HostToBackend,
    hard_deadline: tokio::time::Instant,
) -> Result<(), ()> {
    let line = encode_host(frame, &bounds).map_err(|_| ())?;
    let line = line + "\n";
    let write = async {
        stdin.write_all(line.as_bytes()).await?;
        stdin.flush().await?;
        Ok::<(), std::io::Error>(())
    };
    let write_deadline = std::cmp::min(
        hard_deadline,
        tokio::time::Instant::now()
            .checked_add(WRITE_TIMEOUT)
            .unwrap_or(hard_deadline),
    );
    match tokio::time::timeout_at(write_deadline, write).await {
        Ok(Ok(())) => Ok(()),
        _ => Err(()),
    }
}

/// Drain backend PROCESS stderr concurrently for the whole execution into a
/// bounded buffer, emitting each chunk to `tracing::debug!` (redaction-safe
/// sink). Returns the retained (capped) bytes; they are never surfaced into the
/// failure envelope. The pipe is always drained so a chatty backend cannot
/// deadlock the stdout read loop.
async fn drain_stderr(mut stderr: tokio::process::ChildStderr) -> Vec<u8> {
    let mut acc: Vec<u8> = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match stderr.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                let keep = (STDERR_CAP.saturating_sub(acc.len())).min(n);
                if keep > 0 {
                    acc.extend_from_slice(&buf[..keep]);
                }
                tracing::debug!(target: "execution_backend_stderr", "backend stderr drained");
            }
            Err(_) => break,
        }
    }
    acc
}

/// Read-and-discard stdout raw bytes until EOF or `deadline` (post-terminal pipe
/// drain so the backend can flush and exit). Bounded so a wedged backend cannot
/// hang the reap.
async fn require_clean_eof(
    reader: &mut CappedReader<ChildStdout>,
    deadline: tokio::time::Instant,
) -> Result<(), ExecutionFailure> {
    let mut byte = [0u8; 1];
    match tokio::time::timeout_at(deadline, reader.read_raw(&mut byte)).await {
        Ok(Ok(0)) => Ok(()),
        Ok(Ok(_)) | Ok(Err(_)) => Err(ExecutionFailure::ProtocolViolation),
        Err(_) => Err(ExecutionFailure::CleanupUnconfirmed),
    }
}

/// Reap the backend process within `grace`. Returns the exit code on clean
/// exit, or `None` on grace expiry (caller terminates + classifies).
async fn reap_child(child: &mut Child, deadline: tokio::time::Instant) -> Option<i32> {
    match tokio::time::timeout_at(deadline, child.wait()).await {
        Ok(Ok(status)) => Some(status.code().unwrap_or(-1)),
        _ => None,
    }
}

fn remaining_until(deadline: tokio::time::Instant) -> Duration {
    deadline.saturating_duration_since(tokio::time::Instant::now())
}

fn grace_deadline(hard_deadline: tokio::time::Instant) -> tokio::time::Instant {
    std::cmp::min(
        hard_deadline,
        tokio::time::Instant::now()
            .checked_add(CLEANUP_REPORT_GRACE)
            .unwrap_or(hard_deadline),
    )
}

// ---------------------------------------------------------------------------
// Terminal finalization + cancel path + teardown
// ---------------------------------------------------------------------------

/// Handle a terminal frame: Completed -> close stdin / drain / reap / Ok-or-
/// CleanupUnconfirmed; Failed -> the mapped code. Closing stdin after a terminal
/// frame lets the backend finish and exit (spec: after completed the host closes
/// protocol stdin and requires a successful backend exit).
#[allow(clippy::too_many_arguments)]
async fn finalize_terminal(
    terminal: Terminal,
    mut child: Child,
    mut guard: TreeGuard,
    stderr_handle: tokio::task::JoinHandle<Vec<u8>>,
    stdin: ChildStdin,
    ready: ReadyReport,
    started: StartedReport,
    stdout_acc: Vec<u8>,
    stderr_acc: Vec<u8>,
    mut diagnostics: Vec<Diagnostic>,
    hard_deadline: tokio::time::Instant,
    reader: &mut CappedReader<ChildStdout>,
) -> Result<CompletedOutcome, ExecutionProtocolFailure> {
    // After a terminal frame the host closes stdin (no further host frames).
    // ChildStdin is unbuffered (writes go straight to the OS pipe), so dropping
    // it cleanly closes the write end without losing flushed frames.
    drop(stdin);
    match &terminal {
        Terminal::Completed(p) => {
            diagnostics.extend(p.diagnostics.iter().cloned().map(redact_backend_diagnostic));
        }
        Terminal::Failed(p) => {
            diagnostics.extend(
                p.message
                    .iter()
                    .cloned()
                    .map(|message| redact_backend_diagnostic(Diagnostic { message })),
            );
            diagnostics.extend(p.diagnostics.iter().cloned().map(redact_backend_diagnostic));
        }
    }
    for diagnostic in &diagnostics {
        tracing::debug!(target: "execution_backend_diagnostic", message = %diagnostic.message);
    }

    // Every terminal frame must be followed immediately by clean EOF and a
    // successful backend exit. Extra bytes are a protocol violation regardless
    // of whether the terminal was `completed` or `failed`.
    let reap_deadline = grace_deadline(hard_deadline);
    if let Err(failure) = require_clean_eof(reader, reap_deadline).await {
        let _ = guard.terminate();
        finish_teardown(child, stderr_handle, hard_deadline).await;
        return Err(ExecutionProtocolFailure::with_diagnostics(
            failure,
            diagnostics,
        ));
    }
    match reap_child(&mut child, reap_deadline).await {
        Some(0) => finish_teardown(child, stderr_handle, hard_deadline).await,
        Some(_) => {
            let _ = guard.terminate();
            finish_teardown(child, stderr_handle, hard_deadline).await;
            return Err(ExecutionProtocolFailure::with_diagnostics(
                ExecutionFailure::ProtocolViolation,
                diagnostics,
            ));
        }
        None => {
            let _ = guard.terminate();
            finish_teardown(child, stderr_handle, hard_deadline).await;
            return Err(ExecutionProtocolFailure::with_diagnostics(
                ExecutionFailure::CleanupUnconfirmed,
                diagnostics,
            ));
        }
    }

    match terminal {
        Terminal::Failed(p) => Err(ExecutionProtocolFailure::with_diagnostics(
            map_failure_code(&p, ready.implementation.as_str()),
            diagnostics,
        )),
        Terminal::Completed(p) if p.cleanup == CleanupState::Unconfirmed => {
            Err(ExecutionProtocolFailure::with_diagnostics(
                ExecutionFailure::CleanupUnconfirmed,
                diagnostics,
            ))
        }
        Terminal::Completed(p) => Ok(CompletedOutcome {
            ready,
            started,
            exit: p.exit,
            signal: p.signal,
            timed_out: p.timed_out,
            cancelled: p.cancelled,
            cleanup: p.cleanup,
            stdout: stdout_acc,
            stderr: stderr_acc,
            diagnostics,
        }),
    }
}

/// Cancel path: send `cancel(reason)`, grant grace for a terminal frame, then
/// terminate. A `Completed{cleanup:Confirmed}` arriving in grace is an Ok result
/// (the cancel raced with completion); anything else -> CleanupUnconfirmed (or
/// the mapped Failed code).
#[allow(clippy::too_many_arguments)]
async fn finish_with_cancel(
    mut stdin: ChildStdin,
    reader: &mut CappedReader<ChildStdout>,
    session: &mut Session,
    state: &mut HostState,
    child: Child,
    mut guard: TreeGuard,
    stderr_handle: tokio::task::JoinHandle<Vec<u8>>,
    bounds: Bounds,
    request_id: &RequestId,
    hard_deadline: tokio::time::Instant,
    reason: CancelReason,
    ready: ReadyReport,
    mut started: StartedReport,
    mut stdout_acc: Vec<u8>,
    mut stderr_acc: Vec<u8>,
    mut diagnostics: Vec<Diagnostic>,
) -> Result<CompletedOutcome, ExecutionProtocolFailure> {
    state.begin_cancel();
    let cancel = HostToBackend::Cancel(CancelPayload {
        request_id: request_id.clone(),
        reason,
    });
    let _ = session.observe_host(&cancel);
    let grace_end = grace_deadline(hard_deadline);
    let _ = write_frame(&mut stdin, bounds, &cancel, grace_end).await;
    let outcome: Result<Option<Terminal>, ExecutionFailure> =
        tokio::time::timeout_at(grace_end, async {
            loop {
                if tokio::time::Instant::now() >= grace_end {
                    return Ok(None);
                }
                match reader.read_line().await {
                    Ok(None) => return Ok(None),
                    Ok(Some(line)) => match session.feed_backend_line(&line) {
                        Ok(frame) => match transition(state, &frame)? {
                            Action::Continue => match frame {
                                BackendToHost::Started(p) => {
                                    started = StartedReport {
                                        placement: p.placement,
                                        guarantee: p.guarantee,
                                        policy: p.policy,
                                        limitations: p.limitations,
                                    };
                                }
                                BackendToHost::Stdout(p) => {
                                    stdout_acc.extend_from_slice(p.data.as_bytes());
                                }
                                BackendToHost::Stderr(p) => {
                                    stderr_acc.extend_from_slice(p.data.as_bytes());
                                }
                                BackendToHost::Diagnostic(p) => {
                                    diagnostics.push(redact_backend_diagnostic(Diagnostic {
                                        message: p.message,
                                    }))
                                }
                                _ => {}
                            },
                            Action::Terminal(terminal) => return Ok(Some(terminal)),
                        },
                        Err(_) => return Err(ExecutionFailure::ProtocolViolation),
                    },
                    Err(_) => return Err(ExecutionFailure::ProtocolViolation),
                }
            }
        })
        .await
        .unwrap_or(Ok(None));

    match outcome {
        Ok(Some(Terminal::Completed(mut p))) => {
            p.cancelled = true;
            finalize_terminal(
                Terminal::Completed(p),
                child,
                guard,
                stderr_handle,
                stdin,
                ready,
                started,
                stdout_acc,
                stderr_acc,
                diagnostics,
                hard_deadline,
                reader,
            )
            .await
        }
        Ok(Some(Terminal::Failed(p))) => {
            finalize_terminal(
                Terminal::Failed(p),
                child,
                guard,
                stderr_handle,
                stdin,
                ready,
                started,
                stdout_acc,
                stderr_acc,
                diagnostics,
                hard_deadline,
                reader,
            )
            .await
        }
        Ok(None) => {
            drop(stdin);
            let _ = guard.terminate();
            finish_teardown(child, stderr_handle, hard_deadline).await;
            Err(ExecutionProtocolFailure::with_diagnostics(
                ExecutionFailure::CleanupUnconfirmed,
                diagnostics,
            ))
        }
        Err(failure) => {
            drop(stdin);
            let _ = guard.terminate();
            finish_teardown(child, stderr_handle, hard_deadline).await;
            Err(ExecutionProtocolFailure::with_diagnostics(
                failure,
                diagnostics,
            ))
        }
    }
}

/// Terminate the tree guard + reap the child + await the stderr drain. Used on
/// every non-terminal failure path before returning the failure code.
async fn terminate_and_fail(
    child: Child,
    mut guard: TreeGuard,
    stderr_handle: tokio::task::JoinHandle<Vec<u8>>,
    stdin: ChildStdin,
    code: ExecutionFailure,
    hard_deadline: tokio::time::Instant,
) -> Result<CompletedOutcome, ExecutionProtocolFailure> {
    drop(stdin);
    let _ = guard.terminate();
    finish_teardown(child, stderr_handle, hard_deadline).await;
    Err(code.into())
}

async fn finish_teardown(
    mut child: Child,
    stderr_handle: tokio::task::JoinHandle<Vec<u8>>,
    hard_deadline: tokio::time::Instant,
) {
    // Best-effort reap so kill_on_drop/terminate are accounted; do not hang.
    let teardown_deadline = grace_deadline(hard_deadline);
    let _ = tokio::time::timeout_at(teardown_deadline, child.wait()).await;
    let _ = tokio::time::timeout_at(teardown_deadline, stderr_handle).await;
}

// ---------------------------------------------------------------------------
// Capped async line reader (mirrors opi_protocol::LineReader's cap-before-
// materialize contract; LineReader is sync R: Read and cannot run on ChildStdout)
// ---------------------------------------------------------------------------

/// Buffered async line reader that rejects an oversized line BEFORE
/// materializing it (memory O(1) in line length), mirroring
/// `opi_protocol::execution::v1::codec::LineReader::read_line` (cap, strip one
/// trailing CR, Ok(None) at clean EOF). Does NOT use `read_until`/`split` (those
/// materialize the full line before any cap check). Cap = `Bounds.max_line_size`.
struct CappedReader<R: AsyncRead + Unpin> {
    inner: BufReader<R>,
    cap: usize,
    line: Vec<u8>,
}

impl<R: AsyncRead + Unpin> CappedReader<R> {
    fn new(reader: R, cap: usize) -> Self {
        Self {
            inner: BufReader::new(reader),
            cap,
            line: Vec::new(),
        }
    }

    /// Read one JSONL line (without trailing newline; one trailing CR stripped).
    /// Returns `Ok(None)` at clean EOF, `Ok(Some(line))` with the bytes, or
    /// `Err` if a non-newline byte is seen while `line.len() >= cap`.
    async fn read_line(&mut self) -> Result<Option<Vec<u8>>, ReadErr> {
        loop {
            let (newline_found, oversize, consumed) = {
                let buf = self.inner.fill_buf().await.map_err(|_| ReadErr::Io)?;
                if buf.is_empty() {
                    return if self.line.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(std::mem::take(&mut self.line)))
                    };
                }
                let mut newline_found = false;
                let mut oversize = false;
                let mut consumed = 0;
                for &byte in buf.iter() {
                    if byte == b'\n' {
                        newline_found = true;
                        consumed += 1;
                        break;
                    }
                    if self.line.len() >= self.cap {
                        oversize = true;
                        break;
                    }
                    self.line.push(byte);
                    consumed += 1;
                }
                (newline_found, oversize, consumed)
            };
            self.inner.consume(consumed);
            if oversize {
                return Err(ReadErr::Oversized);
            }
            if newline_found {
                if self.line.last() == Some(&b'\r') {
                    self.line.pop();
                }
                return Ok(Some(std::mem::take(&mut self.line)));
            }
        }
    }

    /// Raw byte read for post-terminal pipe draining (no line/cap semantics).
    async fn read_raw(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf).await
    }
}

#[derive(Debug)]
enum ReadErr {
    Io,
    Oversized,
}

// ===========================================================================
// Tests (host-level unit tests; the subprocess-driven SC16-06a suite lives in
// tests/execution_protocol_host.rs under the execution-backend-test-fixture
// feature).
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use opi_protocol::execution::v1::frames::{AcceptedPayload, StdoutPayload};
    use opi_protocol::execution::v1::{Base64Bytes, FailurePhase};

    #[cfg(unix)]
    #[test]
    fn map_shell_command_unix() {
        let (program, args) = map_shell_command("echo hi");
        assert_eq!(program.as_bytes(), b"sh");
        assert_eq!(
            args.iter().map(|a| a.as_bytes()).collect::<Vec<_>>(),
            [b"-c".as_ref(), b"echo hi"]
        );
    }

    #[cfg(windows)]
    #[test]
    fn map_shell_command_windows() {
        let (program, args) = map_shell_command("echo hi");
        assert_eq!(program.as_bytes(), b"cmd");
        assert_eq!(
            args.iter().map(|a| a.as_bytes()).collect::<Vec<_>>(),
            [b"/C".as_ref(), b"echo hi"]
        );
    }

    #[cfg(unix)]
    #[test]
    fn native_path_preserves_non_utf8_bytes() {
        use std::os::unix::ffi::OsStringExt as _;

        let bytes = b"/tmp/opi-\xff".to_vec();
        let path = std::path::PathBuf::from(std::ffi::OsString::from_vec(bytes.clone()));
        assert_eq!(native_path(&path).as_bytes(), bytes);
    }

    #[cfg(windows)]
    #[test]
    fn native_path_preserves_unpaired_wide_units() {
        use std::os::windows::ffi::OsStringExt as _;

        let units = [b'C' as u16, b':' as u16, b'\\' as u16, 0xD800, 0xDC00];
        let path = std::path::PathBuf::from(std::ffi::OsString::from_wide(&units));
        let expected = units
            .into_iter()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        assert_eq!(native_path(&path).as_bytes(), expected);
    }

    fn rid() -> RequestId {
        RequestId::new("r".into()).unwrap()
    }

    #[test]
    fn completed_before_started_is_protocol_violation() {
        let mut state = HostState::new(HostPhase::AwaitingStarted);
        let completed = BackendToHost::Completed(CompletedPayload {
            request_id: rid(),
            exit: Some(0),
            signal: None,
            timed_out: false,
            cancelled: false,
            cleanup: CleanupState::Confirmed,
            diagnostics: vec![],
        });
        assert!(matches!(
            transition(&mut state, &completed),
            Err(ExecutionFailure::ProtocolViolation)
        ));
    }

    #[test]
    fn stdout_before_started_is_protocol_violation() {
        let mut state = HostState::new(HostPhase::AwaitingStarted);
        let stdout = BackendToHost::Stdout(StdoutPayload {
            request_id: rid(),
            data: Base64Bytes::from_bytes(b"x"),
        });
        assert!(matches!(
            transition(&mut state, &stdout),
            Err(ExecutionFailure::ProtocolViolation)
        ));
    }

    #[test]
    fn failed_before_started_is_accepted_distress() {
        let mut state = HostState::new(HostPhase::AwaitingStarted);
        let failed = BackendToHost::Failed(FailedPayload {
            request_id: rid(),
            code: FailureCode::Unavailable,
            phase: FailurePhase::Handshake,
            message: None,
            diagnostics: vec![],
        });
        match transition(&mut state, &failed) {
            Ok(Action::Terminal(Terminal::Failed(payload)))
                if payload.code == FailureCode::Unavailable => {}
            other => panic!("expected AdapterUnavailable distress, got {other:?}"),
        }
    }

    #[test]
    fn accepted_advances_to_awaiting_started() {
        let mut state = HostState::new(HostPhase::AwaitingAccepted);
        let accepted = BackendToHost::Accepted(AcceptedPayload { request_id: rid() });
        assert!(matches!(
            transition(&mut state, &accepted),
            Ok(Action::Continue)
        ));
        assert_eq!(state.phase, HostPhase::AwaitingStarted);
    }
}
