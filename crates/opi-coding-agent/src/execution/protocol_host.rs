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

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio_util::sync::CancellationToken;

// Re-exported at the `execution::v1` root.
use opi_protocol::execution::v1::{
    BackendToHost, Bounds, CancelReason, CleanupState, Diagnostic, EnvInherit, FailureCode,
    FailurePhase, HostToBackend, ImplementationId, NativeString, ProtocolId, RequestId, Session,
    TargetId, WIRE_IDENTITY,
};
// NOT re-exported at the root -> addressed by module path.
use opi_protocol::execution::v1::codec::{
    CappedLineAccumulator, LineAccumulatorEvent, encode_host,
};
use opi_protocol::execution::v1::frames::{
    CancelPayload, CompletedPayload, ExecutePayload, FailedPayload, InitializePayload,
    StartedPayload,
};

#[cfg(windows)]
use crate::tool::process_tree::resume_child;
use crate::tool::process_tree::{TerminationOutcome, TreeGuard, configure_tree};

use super::CLEANUP_REPORT_GRACE;
use super::failure::ExecutionFailure;

/// Monotonic request-id counter (host-generated ids; no RNG required).
static REQUEST_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

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

/// Maximum number of diagnostic entries retained from one backend invocation.
/// The byte side of the diagnostic budget is supplied by
/// [`Bounds::max_cumulative_output`]; this independent count cap also bounds a
/// flood of empty diagnostic messages.
const MAX_DIAGNOSTIC_ENTRIES: usize = 128;

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
#[derive(Debug, thiserror::Error)]
#[error("{failure}")]
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
        if tokio::time::Instant::now() >= handshake_deadline {
            return Err(ExecutionFailure::ProtocolViolation.into());
        }
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
        let stdin = child.stdin.take().expect("piped stdin present");
        let stdout = child.stdout.take().expect("piped stdout present");
        let stderr = child.stderr.take().expect("piped stderr present");

        // --- concurrent bounded stderr drain (crash evidence only; never surfaced) ---
        let stderr_handle = tokio::spawn(drain_stderr(stderr));
        let reader = CappedReader::new(stdout, bounds.max_line_size);
        let mut active = ActiveProtocol::new(
            child,
            guard,
            stdin,
            reader,
            stderr_handle,
            hard_deadline,
            bounds,
        );
        if tokio::time::Instant::now() >= handshake_deadline {
            return terminate_and_fail(active, ExecutionFailure::ProtocolViolation).await;
        }

        // accumulated state for the eventual outcome
        let redactor = BackendTextRedactor::for_invocation(&launch, &request, child_pid);
        let mut accumulation = ProtocolAccumulation::new(bounds.max_cumulative_output, redactor);
        let mut state = HostState::new(HostPhase::AwaitingReady);

        // --- initialize (seed the session with the HOST id by observing it first) ---
        let init = HostToBackend::Initialize(InitializePayload {
            request_id: request_id.clone(),
            deadline_ms: u64::try_from(remaining_until(hard_deadline).as_millis())
                .unwrap_or(u64::MAX),
            adapter_config: request.adapter_config.clone(),
            supported_protocols: request.supported_protocols.to_vec(),
        });
        if session.observe_host(&init).is_err() {
            return terminate_and_fail(active, ExecutionFailure::ProtocolViolation).await;
        }
        if write_frame(active.stdin_mut(), bounds, &init, handshake_deadline)
            .await
            .is_err()
        {
            return terminate_failed_transmission(active).await;
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
            active.reader_mut(),
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
                    return finalize_terminal(terminal, active, placeholder_ready(), accumulation)
                        .await;
                }
                Err(e) => {
                    return terminate_and_fail(active, e).await;
                }
            },
            FrameSel::Canceled(reason) => {
                return finish_with_cancel(
                    active,
                    &mut session,
                    &mut state,
                    &request_id,
                    reason,
                    placeholder_ready(),
                    accumulation,
                )
                .await;
            }
            FrameSel::Eof | FrameSel::Codec(_) => {
                return terminate_and_fail(active, ExecutionFailure::ProtocolViolation).await;
            }
        };
        if !request
            .supported_protocols
            .iter()
            .any(|p| p == &ready.selected_protocol)
        {
            return terminate_and_fail(active, ExecutionFailure::ProtocolIncompatible).await;
        }
        if ready.implementation.as_str() != request.expected_implementation
            || ready.implementation_version != request.expected_implementation_version
            || ready.target.as_str() != request.expected_target
        {
            return terminate_and_fail(active, ExecutionFailure::ProtocolIncompatible).await;
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
        if session.observe_host(&exec_frame).is_err() {
            return terminate_and_fail(active, ExecutionFailure::ProtocolViolation).await;
        }
        if write_frame(active.stdin_mut(), bounds, &exec_frame, cancel_at)
            .await
            .is_err()
        {
            return terminate_failed_transmission(active).await;
        }

        // --- main frame loop (host-side transition ordering + accumulation) ---
        loop {
            match read_frame_select(
                active.reader_mut(),
                &mut session,
                &request.signal,
                cancel_at,
            )
            .await
            {
                FrameSel::Canceled(reason) => {
                    return finish_with_cancel(
                        active,
                        &mut session,
                        &mut state,
                        &request_id,
                        reason,
                        ready_report,
                        accumulation,
                    )
                    .await;
                }
                FrameSel::Eof => {
                    return terminate_and_fail(active, ExecutionFailure::ProtocolViolation).await;
                }
                FrameSel::Codec(e) => {
                    return terminate_and_fail(active, e).await;
                }
                FrameSel::Frame(frame) => match transition(&mut state, &frame) {
                    Ok(Action::Continue) => match frame {
                        BackendToHost::Accepted(_) => {}
                        BackendToHost::Started(p) => {
                            accumulation.started = accumulation.diagnostics.redact_started(p);
                        }
                        BackendToHost::Stdout(p) => {
                            accumulation.stdout.extend_from_slice(p.data.as_bytes())
                        }
                        BackendToHost::Stderr(p) => {
                            accumulation.stderr.extend_from_slice(p.data.as_bytes())
                        }
                        BackendToHost::Diagnostic(p) => {
                            if accumulation
                                .diagnostics
                                .push_backend(Diagnostic { message: p.message })
                                .is_err()
                            {
                                return terminate_and_fail(
                                    active,
                                    ExecutionFailure::ProtocolViolation,
                                )
                                .await;
                            }
                        }
                        _ => {}
                    },
                    Ok(Action::Terminal(terminal)) => {
                        return finalize_terminal(terminal, active, ready_report, accumulation)
                            .await;
                    }
                    Err(e) => {
                        return terminate_and_fail(active, e).await;
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

#[derive(Default)]
struct BackendTextRedactor {
    exact_values: Vec<String>,
}

impl BackendTextRedactor {
    fn new(values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut exact_values = values
            .into_iter()
            .map(Into::into)
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        exact_values.sort_by_key(|value| std::cmp::Reverse(value.len()));
        exact_values.dedup();
        Self { exact_values }
    }

    fn for_invocation(
        launch: &BackendLaunch<'_>,
        request: &ExecutionRequest<'_>,
        child_pid: Option<u32>,
    ) -> Self {
        let mut values = vec![
            request.command.to_string(),
            request.workspace.to_string_lossy().into_owned(),
            request.cwd.to_string_lossy().into_owned(),
            native_path(request.workspace).to_wire_string(),
            native_path(request.cwd).to_wire_string(),
            launch.program.to_string_lossy().into_owned(),
        ];
        values.extend(launch.args.iter().cloned());
        for value in request.env_additions.values() {
            values.push(value.to_wire_string());
        }
        collect_json_strings(&request.adapter_config, &mut values);
        if let Some(pid) = child_pid {
            values.push(pid.to_string());
        }
        Self::new(values)
    }

    fn redact(&self, value: &str) -> String {
        let exact_redacted = self
            .exact_values
            .iter()
            .fold(value.to_string(), |text, exact| {
                text.replace(exact, "[REDACTED]")
            });
        opi_agent::diagnostic::redact_text(
            &exact_redacted,
            opi_agent::diagnostic::RedactionMode::Summary,
        )
    }

    fn redact_started(&self, payload: StartedPayload) -> StartedReport {
        StartedReport {
            placement: self.redact(&payload.placement),
            guarantee: self.redact(&payload.guarantee),
            policy: self.redact(&payload.policy),
            limitations: payload
                .limitations
                .iter()
                .map(|limitation| self.redact(limitation))
                .collect(),
        }
    }
}

fn collect_json_strings(value: &serde_json::Value, values: &mut Vec<String>) {
    match value {
        serde_json::Value::String(value) => values.push(value.clone()),
        serde_json::Value::Array(items) => {
            for item in items {
                collect_json_strings(item, values);
            }
        }
        serde_json::Value::Object(fields) => {
            for value in fields.values() {
                collect_json_strings(value, values);
            }
        }
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {}
    }
}

/// Host-owned diagnostic budget. At most [`MAX_DIAGNOSTIC_ENTRIES`] (128)
/// entries and `Bounds::max_cumulative_output` unredacted message bytes are
/// accepted across streaming plus terminal diagnostics. This byte budget is
/// separate from [`Session`]'s decoded stdout/stderr cumulative-output counter;
/// the bounds field is reused only as the configured ceiling.
struct DiagnosticAccumulator {
    entries: Vec<Diagnostic>,
    cumulative_bytes: usize,
    max_cumulative_bytes: usize,
    redactor: BackendTextRedactor,
}

impl DiagnosticAccumulator {
    #[cfg(test)]
    fn new(max_cumulative_bytes: usize) -> Self {
        Self::with_redactor(max_cumulative_bytes, BackendTextRedactor::default())
    }

    fn with_redactor(max_cumulative_bytes: usize, redactor: BackendTextRedactor) -> Self {
        Self {
            entries: Vec::new(),
            cumulative_bytes: 0,
            max_cumulative_bytes,
            redactor,
        }
    }

    fn redact_started(&self, payload: StartedPayload) -> StartedReport {
        self.redactor.redact_started(payload)
    }

    fn push_backend(&mut self, diagnostic: Diagnostic) -> Result<(), ExecutionFailure> {
        self.extend_backend(std::iter::once(diagnostic))
    }

    fn extend_backend(
        &mut self,
        diagnostics: impl IntoIterator<Item = Diagnostic>,
    ) -> Result<(), ExecutionFailure> {
        let incoming = diagnostics.into_iter().collect::<Vec<_>>();
        let next_count = self
            .entries
            .len()
            .checked_add(incoming.len())
            .ok_or(ExecutionFailure::ProtocolViolation)?;
        if next_count > MAX_DIAGNOSTIC_ENTRIES {
            return Err(ExecutionFailure::ProtocolViolation);
        }
        let incoming_bytes = incoming.iter().try_fold(0usize, |total, diagnostic| {
            total.checked_add(diagnostic.message.len())
        });
        let next_bytes = incoming_bytes
            .and_then(|bytes| self.cumulative_bytes.checked_add(bytes))
            .ok_or(ExecutionFailure::ProtocolViolation)?;
        if next_bytes > self.max_cumulative_bytes {
            return Err(ExecutionFailure::ProtocolViolation);
        }
        self.entries
            .extend(incoming.into_iter().map(|_| Diagnostic {
                message: "backend reported a diagnostic".to_string(),
            }));
        self.cumulative_bytes = next_bytes;
        Ok(())
    }

    fn entries(&self) -> &[Diagnostic] {
        &self.entries
    }

    fn into_entries(self) -> Vec<Diagnostic> {
        self.entries
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
        (HostPhase::AwaitingStarted, Started(p)) if valid_started_contract(p) => {
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
        ) if valid_failed_for_phase(state.phase, p)
            && (!state.cancelling || state.phase == HostPhase::Draining) =>
        {
            state.phase = HostPhase::Terminal;
            Ok(Action::Terminal(Terminal::Failed(p.clone())))
        }
        _ => Err(ExecutionFailure::ProtocolViolation),
    }
}

/// Validate the documented `FailureCode`/`FailurePhase` pair against whether
/// the target has crossed the Started publication gate.
fn valid_failed_for_phase(host_phase: HostPhase, payload: &FailedPayload) -> bool {
    match host_phase {
        HostPhase::AwaitingReady | HostPhase::AwaitingAccepted | HostPhase::AwaitingStarted => {
            matches!(
                (payload.code, payload.phase),
                (
                    FailureCode::Unavailable
                        | FailureCode::Failed
                        | FailureCode::ProtocolIncompatible
                        | FailureCode::ProtocolViolation
                        | FailureCode::ExecutionTimedOut,
                    FailurePhase::Handshake
                ) | (FailureCode::CleanupUnconfirmed, FailurePhase::Cleanup)
            )
        }
        HostPhase::Draining => matches!(
            (payload.code, payload.phase),
            (
                FailureCode::ProtocolViolation
                    | FailureCode::ExecutionFailed
                    | FailureCode::ExecutionTimedOut,
                FailurePhase::Execution
            ) | (FailureCode::CleanupUnconfirmed, FailurePhase::Cleanup)
        ),
        HostPhase::Terminal => false,
    }
}

fn valid_started_contract(payload: &opi_protocol::execution::v1::frames::StartedPayload) -> bool {
    [
        payload.placement.as_str(),
        payload.guarantee.as_str(),
        payload.policy.as_str(),
    ]
    .into_iter()
    .all(|field| !field.trim().is_empty())
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
async fn write_frame<W: AsyncWrite + Unpin>(
    stdin: &mut W,
    bounds: Bounds,
    frame: &HostToBackend,
    deadline: tokio::time::Instant,
) -> Result<(), ()> {
    if tokio::time::Instant::now() >= deadline {
        return Err(());
    }
    let line = encode_host(frame, &bounds).map_err(|_| ())?;
    if tokio::time::Instant::now() >= deadline {
        return Err(());
    }
    let line = line + "\n";
    let write = async {
        stdin.write_all(line.as_bytes()).await?;
        stdin.flush().await?;
        Ok::<(), std::io::Error>(())
    };
    let write_deadline = std::cmp::min(
        deadline,
        tokio::time::Instant::now()
            .checked_add(WRITE_TIMEOUT)
            .unwrap_or(deadline),
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

/// Sole owner of the live backend lifecycle. Moving this value into a terminal
/// path makes stdin closure, tree termination, child reap, and stderr-drain
/// accounting a single-consumption operation.
struct ActiveProtocol {
    child: Child,
    guard: TreeGuard,
    stdin: Option<ChildStdin>,
    reader: CappedReader<ChildStdout>,
    stderr_handle: Option<tokio::task::JoinHandle<Vec<u8>>>,
    hard_deadline: tokio::time::Instant,
    bounds: Bounds,
}

impl ActiveProtocol {
    fn new(
        child: Child,
        guard: TreeGuard,
        stdin: ChildStdin,
        reader: CappedReader<ChildStdout>,
        stderr_handle: tokio::task::JoinHandle<Vec<u8>>,
        hard_deadline: tokio::time::Instant,
        bounds: Bounds,
    ) -> Self {
        Self {
            child,
            guard,
            stdin: Some(stdin),
            reader,
            stderr_handle: Some(stderr_handle),
            hard_deadline,
            bounds,
        }
    }

    fn stdin_mut(&mut self) -> &mut ChildStdin {
        self.stdin.as_mut().expect("active protocol stdin is open")
    }

    fn close_stdin(&mut self) {
        self.stdin.take();
    }

    fn reader_mut(&mut self) -> &mut CappedReader<ChildStdout> {
        &mut self.reader
    }

    async fn terminate_and_finish(mut self) -> TeardownConfirmation {
        self.close_stdin();
        let tree_terminated = !matches!(self.guard.terminate(), TerminationOutcome::Failed(_));
        finish_teardown(
            self.child,
            self.stderr_handle
                .take()
                .expect("active protocol stderr task is owned"),
            self.hard_deadline,
            tree_terminated,
        )
        .await
    }
}

struct ProtocolAccumulation {
    started: StartedReport,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    diagnostics: DiagnosticAccumulator,
}

impl ProtocolAccumulation {
    fn new(max_diagnostic_bytes: usize, redactor: BackendTextRedactor) -> Self {
        Self {
            started: StartedReport::default(),
            stdout: Vec::new(),
            stderr: Vec::new(),
            diagnostics: DiagnosticAccumulator::with_redactor(max_diagnostic_bytes, redactor),
        }
    }
}

/// Handle a terminal frame: Completed -> close stdin / drain / reap / Ok-or-
/// CleanupUnconfirmed; Failed -> the mapped code. Closing stdin after a terminal
/// frame lets the backend finish and exit (spec: after completed the host closes
/// protocol stdin and requires a successful backend exit).
async fn finalize_terminal(
    terminal: Terminal,
    mut active: ActiveProtocol,
    ready: ReadyReport,
    mut accumulation: ProtocolAccumulation,
) -> Result<CompletedOutcome, ExecutionProtocolFailure> {
    // After a terminal frame the host closes stdin (no further host frames).
    // ChildStdin is unbuffered (writes go straight to the OS pipe), so dropping
    // it cleanly closes the write end without losing flushed frames.
    active.close_stdin();
    match &terminal {
        Terminal::Completed(p) => {
            if accumulation
                .diagnostics
                .extend_backend(p.diagnostics.iter().cloned())
                .is_err()
            {
                let teardown = active.terminate_and_finish().await;
                return Err(failure_after_teardown(
                    ExecutionFailure::ProtocolViolation,
                    accumulation.diagnostics.into_entries(),
                    &teardown,
                ));
            }
        }
        Terminal::Failed(p) => {
            let terminal_diagnostics = p
                .message
                .iter()
                .cloned()
                .map(|message| Diagnostic { message })
                .chain(p.diagnostics.iter().cloned());
            if accumulation
                .diagnostics
                .extend_backend(terminal_diagnostics)
                .is_err()
            {
                let teardown = active.terminate_and_finish().await;
                return Err(failure_after_teardown(
                    ExecutionFailure::ProtocolViolation,
                    accumulation.diagnostics.into_entries(),
                    &teardown,
                ));
            }
        }
    }
    for diagnostic in accumulation.diagnostics.entries() {
        tracing::debug!(target: "execution_backend_diagnostic", message = %diagnostic.message);
    }

    // Every terminal frame must be followed immediately by clean EOF and a
    // successful backend exit. Extra bytes are a protocol violation regardless
    // of whether the terminal was `completed` or `failed`.
    let reap_deadline = grace_deadline(active.hard_deadline);
    if let Err(failure) = require_clean_eof(active.reader_mut(), reap_deadline).await {
        let teardown = active.terminate_and_finish().await;
        return Err(failure_after_teardown(
            failure,
            accumulation.diagnostics.into_entries(),
            &teardown,
        ));
    }
    let reap_result = reap_child(&mut active.child, reap_deadline).await;
    let teardown = active.terminate_and_finish().await;
    match reap_result {
        Some(0) => {}
        Some(_) => {
            return Err(failure_after_teardown(
                ExecutionFailure::ProtocolViolation,
                accumulation.diagnostics.into_entries(),
                &teardown,
            ));
        }
        None => {
            return Err(failure_after_teardown(
                ExecutionFailure::CleanupUnconfirmed,
                accumulation.diagnostics.into_entries(),
                &teardown,
            ));
        }
    }

    match terminal {
        Terminal::Failed(p) => Err(failure_after_teardown(
            map_failure_code(&p, ready.implementation.as_str()),
            accumulation.diagnostics.into_entries(),
            &teardown,
        )),
        Terminal::Completed(p)
            if p.cleanup == CleanupState::Unconfirmed || !teardown.confirmed() =>
        {
            Err(failure_after_teardown(
                ExecutionFailure::CleanupUnconfirmed,
                accumulation.diagnostics.into_entries(),
                &teardown,
            ))
        }
        Terminal::Completed(p) => Ok(CompletedOutcome {
            ready,
            started: accumulation.started,
            exit: p.exit,
            signal: p.signal,
            timed_out: p.timed_out,
            cancelled: p.cancelled,
            cleanup: p.cleanup,
            stdout: accumulation.stdout,
            stderr: accumulation.stderr,
            diagnostics: accumulation.diagnostics.into_entries(),
        }),
    }
}

/// Cancel path: send `cancel(reason)`, grant grace for a terminal frame, then
/// terminate. A `Completed{cleanup:Confirmed}` arriving in grace is an Ok result
/// (the cancel raced with completion); anything else -> CleanupUnconfirmed (or
/// the mapped Failed code).
async fn finish_with_cancel(
    mut active: ActiveProtocol,
    session: &mut Session,
    state: &mut HostState,
    request_id: &RequestId,
    reason: CancelReason,
    ready: ReadyReport,
    mut accumulation: ProtocolAccumulation,
) -> Result<CompletedOutcome, ExecutionProtocolFailure> {
    state.begin_cancel();
    let cancel = HostToBackend::Cancel(CancelPayload {
        request_id: request_id.clone(),
        reason,
    });
    let _ = session.observe_host(&cancel);
    let grace_end = grace_deadline(active.hard_deadline);
    let bounds = active.bounds;
    let _ = write_frame(active.stdin_mut(), bounds, &cancel, grace_end).await;
    let outcome: Result<Option<Terminal>, ExecutionFailure> =
        tokio::time::timeout_at(grace_end, async {
            loop {
                if tokio::time::Instant::now() >= grace_end {
                    return Ok(None);
                }
                match active.reader_mut().read_line().await {
                    Ok(None) => return Ok(None),
                    Ok(Some(line)) => match session.feed_backend_line(&line) {
                        Ok(frame) => match transition(state, &frame)? {
                            Action::Continue => match frame {
                                BackendToHost::Started(p) => {
                                    accumulation.started =
                                        accumulation.diagnostics.redact_started(p);
                                }
                                BackendToHost::Stdout(p) => {
                                    accumulation.stdout.extend_from_slice(p.data.as_bytes());
                                }
                                BackendToHost::Stderr(p) => {
                                    accumulation.stderr.extend_from_slice(p.data.as_bytes());
                                }
                                BackendToHost::Diagnostic(p) => {
                                    accumulation
                                        .diagnostics
                                        .push_backend(Diagnostic { message: p.message })?;
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
            match reason {
                CancelReason::Deadline => {
                    p.timed_out = true;
                    p.cancelled = false;
                }
                CancelReason::Canceled => {
                    p.timed_out = false;
                    p.cancelled = true;
                }
            }
            finalize_terminal(Terminal::Completed(p), active, ready, accumulation).await
        }
        Ok(Some(Terminal::Failed(p))) => {
            finalize_terminal(Terminal::Failed(p), active, ready, accumulation).await
        }
        Ok(None) => {
            let teardown = active.terminate_and_finish().await;
            Err(failure_after_teardown(
                ExecutionFailure::CleanupUnconfirmed,
                accumulation.diagnostics.into_entries(),
                &teardown,
            ))
        }
        Err(failure) => {
            let teardown = active.terminate_and_finish().await;
            Err(failure_after_teardown(
                failure,
                accumulation.diagnostics.into_entries(),
                &teardown,
            ))
        }
    }
}

/// Terminate the tree guard + reap the child + await the stderr drain. Used on
/// every non-terminal failure path before returning the failure code.
async fn terminate_and_fail(
    active: ActiveProtocol,
    code: ExecutionFailure,
) -> Result<CompletedOutcome, ExecutionProtocolFailure> {
    let teardown = active.terminate_and_finish().await;
    Err(failure_after_teardown(code, Vec::new(), &teardown))
}

/// A deadline-expired or otherwise incomplete host frame cannot be followed by
/// more protocol traffic: `write_all` cancellation may have left a partial
/// JSON line in the pipe. Close stdin and terminate locally. Preserve the
/// transmission's protocol-violation classification only when L0 termination,
/// child reap, and stderr drain all confirm inside the original hard deadline.
async fn terminate_failed_transmission(
    active: ActiveProtocol,
) -> Result<CompletedOutcome, ExecutionProtocolFailure> {
    let teardown = active.terminate_and_finish().await;
    Err(failure_after_teardown(
        ExecutionFailure::ProtocolViolation,
        Vec::new(),
        &teardown,
    ))
}

#[cfg(test)]
fn failed_transmission_failure(
    tree_confirmed: bool,
    child_reaped: bool,
    stderr_drained: bool,
) -> ExecutionFailure {
    TeardownConfirmation {
        tree_terminated: tree_confirmed,
        child_reaped,
        stderr_drained,
    }
    .classify(ExecutionFailure::ProtocolViolation)
}

struct TeardownConfirmation {
    tree_terminated: bool,
    child_reaped: bool,
    stderr_drained: bool,
}

impl TeardownConfirmation {
    fn confirmed(&self) -> bool {
        self.tree_terminated && self.child_reaped && self.stderr_drained
    }

    fn classify(&self, original: ExecutionFailure) -> ExecutionFailure {
        if self.confirmed() {
            original
        } else {
            ExecutionFailure::CleanupUnconfirmed
        }
    }
}

fn failure_after_teardown(
    original: ExecutionFailure,
    mut diagnostics: Vec<Diagnostic>,
    teardown: &TeardownConfirmation,
) -> ExecutionProtocolFailure {
    let original_code = original.code();
    let failure = teardown.classify(original);
    if !teardown.confirmed() && original_code != ExecutionFailure::CleanupUnconfirmed.code() {
        diagnostics.push(Diagnostic {
            message: format!("original_failure={original_code}"),
        });
    }
    ExecutionProtocolFailure::with_diagnostics(failure, diagnostics)
}

async fn finish_teardown(
    mut child: Child,
    stderr_handle: tokio::task::JoinHandle<Vec<u8>>,
    hard_deadline: tokio::time::Instant,
    tree_terminated: bool,
) -> TeardownConfirmation {
    // Best-effort reap so kill_on_drop/terminate are accounted; do not hang.
    let teardown_deadline = grace_deadline(hard_deadline);
    let child_reaped = matches!(
        tokio::time::timeout_at(teardown_deadline, child.wait()).await,
        Ok(Ok(_))
    );
    let stderr_drained = matches!(
        tokio::time::timeout_at(teardown_deadline, stderr_handle).await,
        Ok(Ok(_))
    );
    TeardownConfirmation {
        tree_terminated,
        child_reaped,
        stderr_drained,
    }
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
    line: Vec<u8>,
    accumulator: CappedLineAccumulator,
}

impl<R: AsyncRead + Unpin> CappedReader<R> {
    fn new(reader: R, cap: usize) -> Self {
        Self {
            inner: BufReader::new(reader),
            line: Vec::new(),
            accumulator: CappedLineAccumulator::new(cap),
        }
    }

    /// Read one JSONL line (without trailing newline; one trailing CR stripped).
    /// Returns `Ok(None)` at clean EOF, `Ok(Some(line))` with the bytes, or
    /// `Err` if a non-newline byte is seen while `line.len() >= cap`.
    async fn read_line(&mut self) -> Result<Option<Vec<u8>>, ReadErr> {
        let mut byte = [0_u8; 1];
        loop {
            match self.inner.read(&mut byte).await.map_err(|_| ReadErr::Io)? {
                0 => {
                    return if !self
                        .accumulator
                        .finish_eof(&mut self.line)
                        .map_err(|_| ReadErr::Oversized)?
                    {
                        Ok(None)
                    } else {
                        Ok(Some(std::mem::take(&mut self.line)))
                    };
                }
                _ => {
                    if self
                        .accumulator
                        .push_byte(byte[0], &mut self.line)
                        .map_err(|_| ReadErr::Oversized)?
                        == LineAccumulatorEvent::Complete
                    {
                        return Ok(Some(std::mem::take(&mut self.line)));
                    }
                }
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
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use opi_protocol::execution::v1::frames::{AcceptedPayload, StdoutPayload};
    use opi_protocol::execution::v1::{Base64Bytes, FailurePhase};
    use tokio::io::AsyncWrite;

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

    struct PendingWriter;

    impl AsyncWrite for PendingWriter {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            Poll::Pending
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Pending
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    fn initialize_frame() -> HostToBackend {
        HostToBackend::Initialize(InitializePayload {
            request_id: rid(),
            deadline_ms: 1,
            adapter_config: serde_json::json!({}),
            supported_protocols: vec![ProtocolId::new(WIRE_IDENTITY).unwrap()],
        })
    }

    fn execute_frame() -> HostToBackend {
        HostToBackend::Execute(ExecutePayload {
            request_id: rid(),
            program: NativeString::from_utf8("sh"),
            args: vec![NativeString::from_utf8("-c")],
            workspace: NativeString::from_utf8("workspace"),
            cwd: NativeString::from_utf8("workspace"),
            timeout_ms: 1,
            env_inherit: EnvInherit::Clear,
            env_additions: BTreeMap::new(),
        })
    }

    async fn read_capped(input: &[u8], cap: usize) -> Result<Option<Vec<u8>>, ReadErr> {
        let (mut writer, reader) = tokio::io::duplex(input.len().max(1));
        writer.write_all(input).await.expect("write capped input");
        writer.shutdown().await.expect("close capped input");
        CappedReader::new(reader, cap).read_line().await
    }

    #[tokio::test]
    async fn capped_reader_matches_canonical_lf_and_crlf_boundaries() {
        assert_eq!(
            read_capped(b"abcd\n", 4).await.unwrap(),
            Some(b"abcd".to_vec())
        );
        assert_eq!(
            read_capped(b"abcd\r\n", 4).await.unwrap(),
            Some(b"abcd".to_vec())
        );
        assert!(matches!(
            read_capped(b"abcde\n", 4).await,
            Err(ReadErr::Oversized)
        ));
        assert_eq!(
            read_capped(b"ab\rc\n", 4).await.unwrap(),
            Some(b"ab\rc".to_vec())
        );
        assert_eq!(
            read_capped(b"abcd\r", 5).await.unwrap(),
            Some(b"abcd\r".to_vec())
        );
    }

    #[tokio::test(start_paused = true)]
    async fn pending_initialize_write_failure_requires_fully_confirmed_local_teardown() {
        let start = tokio::time::Instant::now();
        let handshake_deadline = start + Duration::from_millis(7);
        let result = write_frame(
            &mut PendingWriter,
            Bounds::DEFAULT,
            &initialize_frame(),
            handshake_deadline,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(tokio::time::Instant::now(), handshake_deadline);
        assert!(matches!(
            failed_transmission_failure(true, true, true),
            ExecutionFailure::ProtocolViolation
        ));
        for (tree_confirmed, child_reaped, stderr_drained) in [
            (false, true, true),
            (true, false, true),
            (true, true, false),
        ] {
            assert!(matches!(
                failed_transmission_failure(tree_confirmed, child_reaped, stderr_drained),
                ExecutionFailure::CleanupUnconfirmed
            ));
        }
    }

    #[test]
    fn every_original_failure_yields_to_unconfirmed_teardown() {
        for teardown in [
            TeardownConfirmation {
                tree_terminated: false,
                child_reaped: true,
                stderr_drained: true,
            },
            TeardownConfirmation {
                tree_terminated: true,
                child_reaped: false,
                stderr_drained: true,
            },
            TeardownConfirmation {
                tree_terminated: true,
                child_reaped: true,
                stderr_drained: false,
            },
        ] {
            assert!(matches!(
                teardown.classify(ExecutionFailure::ProtocolViolation),
                ExecutionFailure::CleanupUnconfirmed
            ));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn pending_execute_write_stops_at_host_cancellation_cutoff() {
        let start = tokio::time::Instant::now();
        let cancel_at = start + Duration::from_millis(13);
        let result = write_frame(
            &mut PendingWriter,
            Bounds::DEFAULT,
            &execute_frame(),
            cancel_at,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(tokio::time::Instant::now(), cancel_at);
    }

    #[test]
    fn diagnostics_count_cap_accepts_exact_boundary_and_rejects_one_more() {
        let mut diagnostics = DiagnosticAccumulator::new(usize::MAX);
        diagnostics
            .extend_backend((0..MAX_DIAGNOSTIC_ENTRIES).map(|_| Diagnostic {
                message: String::new(),
            }))
            .unwrap();
        assert_eq!(diagnostics.entries().len(), MAX_DIAGNOSTIC_ENTRIES);
        assert!(
            diagnostics
                .push_backend(Diagnostic {
                    message: String::new(),
                })
                .is_err()
        );
    }

    #[test]
    fn diagnostics_byte_budget_accepts_exact_boundary_and_rejects_one_more() {
        let mut diagnostics = DiagnosticAccumulator::new(8);
        diagnostics
            .extend_backend([
                Diagnostic {
                    message: "123".into(),
                },
                Diagnostic {
                    message: "45678".into(),
                },
            ])
            .unwrap();
        assert_eq!(diagnostics.cumulative_bytes, 8);
        assert!(
            diagnostics
                .push_backend(Diagnostic {
                    message: "9".into(),
                })
                .is_err()
        );
    }

    #[test]
    fn terminal_diagnostic_batch_shares_stream_count_and_byte_budgets() {
        let mut diagnostics = DiagnosticAccumulator::new(8);
        diagnostics
            .push_backend(Diagnostic {
                message: "12".into(),
            })
            .unwrap();
        diagnostics
            .extend_backend([
                Diagnostic {
                    message: "345".into(),
                },
                Diagnostic {
                    message: "678".into(),
                },
            ])
            .unwrap();
        assert_eq!(diagnostics.entries().len(), 3);
        assert_eq!(diagnostics.cumulative_bytes, 8);
    }

    #[test]
    fn backend_diagnostic_text_is_replaced_by_a_host_owned_summary() {
        let canary = "plain-non-pattern-command-canary";
        let mut diagnostics = DiagnosticAccumulator::new(usize::MAX);
        diagnostics
            .push_backend(Diagnostic {
                message: format!("adapter echoed {canary}"),
            })
            .unwrap();

        assert_eq!(
            diagnostics.entries()[0].message,
            "backend reported a diagnostic"
        );
        assert!(!diagnostics.entries()[0].message.contains(canary));
    }

    #[test]
    fn effective_contract_redacts_exact_invocation_values() {
        let canaries = [
            "plain-command-canary",
            "plain-env-canary",
            "C:\\private\\workspace-canary",
            "424242",
        ];
        let redactor = BackendTextRedactor::new(canaries.iter().copied());
        let report = redactor.redact_started(StartedPayload {
            request_id: rid(),
            placement: format!("host {}", canaries[0]),
            guarantee: format!("restricted {}", canaries[1]),
            policy: format!("policy {}", canaries[2]),
            limitations: vec![format!("target pid {}", canaries[3])],
        });
        let public = serde_json::json!({
            "placement": report.placement,
            "guarantee": report.guarantee,
            "policy": report.policy,
            "limitations": report.limitations,
        })
        .to_string();

        for canary in canaries {
            assert!(
                !public.contains(canary),
                "contract leaked {canary:?}: {public}"
            );
        }
        assert!(public.contains("host"));
        assert!(public.contains("restricted"));
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
