//! The production `opi-sandbox backend --stdio` entry point (Phase 16 task
//! 16.12): the BACKEND half of the `command-execution-jsonl-v1` state machine.
//!
//! This is the mirror of the Phase 16.7 host
//! (`opi_coding_agent::execution::protocol_host::ExecutionProtocolHost`): where
//! the host launches a backend and reads backend->host frames, the backend reads
//! host->backend frames from its stdin and writes backend->host frames to its
//! stdout. One backend process accepts exactly one execution. After a terminal
//! frame (`completed` or `failed`) the backend flushes and exits 0; the target's
//! own exit code is IN-BAND in `completed`, never the backend's process exit.
//!
//! # State machine
//!
//! ```text
//! read initialize  -> negotiate (select) -> emit ready
//! read execute     -> validate -> emit accepted
//! helper::start    -> Refused{code} -> emit failed{Handshake}; or
//!                  -> Expired{run} -> keep gated + cancel + drain; or
//!                  -> CleanupUnconfirmed -> emit failed{Cleanup}; or
//!                  -> Ready{run} -> poll Started -> emit+flush started
//! drain            -> select { cancel frame -> fire token ; run -> Completed }
//!                  -> emit Stdout/Stderr chunks -> emit completed
//! (terminal)       -> flush -> exit 0
//! ```
//!
//! Every emitted frame echoes the single host-generated request id captured from
//! `initialize`. Protocol stdin is reserved (host->backend JSONL) and never
//! inherited by the target (`crate::helper::build_request` pins
//! [`crate::StdinPolicy::Null`]); protocol stdout carries ONLY `encode_backend`
//! lines (target stdout/stderr are base64 `Stdout`/`Stderr` frames).
//!
//! # Platform posture
//!
//! Portable conformance (the full success state machine) is driven by
//! [`drive`] with an INJECTED restriction + `supported = true`. The real
//! executable ([`run`]) uses `crate::platform::current`: supported native
//! Linux and macOS postures can execute with their platform restriction, while
//! unsupported postures refuse at the pre-start platform gate with
//! `failed{Unavailable, Handshake}`.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures_core::Stream;
use opi_protocol::execution::v1::codec::{
    CappedLineAccumulator, CodecError, LineAccumulatorEvent, encode_backend,
};
use opi_protocol::execution::v1::frames::{
    AcceptedPayload, CompletedPayload, Diagnostic, DiagnosticPayload, FailedPayload,
    InitializePayload, ReadyPayload, StderrPayload, StdoutPayload,
};
use opi_protocol::execution::v1::{
    BackendToHost, Base64Bytes, Bounds, CleanupState as WireCleanup, FailureCode, FailurePhase,
    HostToBackend, ImplementationId, ProtocolId, RequestId, Session, TargetId, WIRE_IDENTITY,
    select,
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

use crate::helper::{self, StartOutcome};
use crate::platform;
use crate::policy::{NetworkPolicy, NoRestriction, Profile, Restriction, SandboxPolicy};
use crate::runner::{
    CleanupState, FaultInjection, OutputStream, RunDeadlines, SandboxEvent, SandboxOutcome,
    SandboxRunner,
};

/// Backend exit after a clean protocol exchange (a terminal frame was emitted +
/// flushed). The target's own exit is in-band in `completed`.
const EXIT_OK: i32 = 0;
/// Backend exit when no terminal frame could be emitted (the very first frame
/// was malformed/oversized so no request id was established, or the stdout pipe
/// broke). The host classifies unexpected exit / EOF as a protocol violation.
const EXIT_NO_TERMINAL: i32 = 1;

/// The host must send `initialize` immediately after spawning the one-shot
/// backend. No request id exists before that frame, so expiry is a silent,
/// nonzero process failure rather than a fabricated request-scoped terminal.
const INITIALIZE_WAIT_TIMEOUT: Duration = Duration::from_secs(5);

/// Portion of the hard request budget held back for target cancellation,
/// process-tree reap, pipe drain, and invocation-root removal. The execute
/// timeout can select an earlier cutoff, but cleanup never extends the hard
/// Initialize deadline.
const CLEANUP_RESERVE: Duration = Duration::from_millis(500);

#[derive(Clone, Copy)]
enum PostStartFailure {
    Release,
    StreamEnded,
    CleanupUnconfirmed,
}

fn classify_post_start_failure(failure: PostStartFailure) -> (FailureCode, FailurePhase) {
    match failure {
        PostStartFailure::Release | PostStartFailure::StreamEnded => {
            (FailureCode::ExecutionFailed, FailurePhase::Execution)
        }
        PostStartFailure::CleanupUnconfirmed => {
            (FailureCode::CleanupUnconfirmed, FailurePhase::Cleanup)
        }
    }
}

/// Drive one backend exchange over `stdin`/`stdout` with an INJECTED
/// restriction and platform posture. This is the pure testable core: production
/// [`run`] applies `platform::current`, while the standalone binary owns the
/// native-stdin bridge; portable conformance tests inject `supported = true`,
/// empty limitations, and a [`NoRestriction`] to exercise the full success
/// state machine. The runner is constructed only after
/// `initialize.adapter_config` has been validated and mapped to a
/// [`SandboxPolicy`].
///
/// `stdin` is an owned asynchronous reader, so cancellation drops the input
/// resource without leaving a blocking worker behind. `stdout` is borrowed for
/// the whole exchange and flushed after every emitted frame. Returns `EXIT_OK`
/// (0) after a terminal frame, or `EXIT_NO_TERMINAL` (1) if none could be
/// emitted.
pub async fn drive<W>(
    stdin: Pin<Box<dyn AsyncRead + Send>>,
    stdout: &mut W,
    bounds: Bounds,
    supported: bool,
    limitations: &[String],
    restriction: Arc<dyn Restriction>,
) -> i32
where
    W: AsyncWrite + Unpin + ?Sized,
{
    drive_with_faults(
        stdin,
        stdout,
        bounds,
        supported,
        limitations,
        restriction,
        FaultInjection::default(),
    )
    .await
}

async fn drive_with_faults<W>(
    stdin: Pin<Box<dyn AsyncRead + Send>>,
    stdout: &mut W,
    bounds: Bounds,
    supported: bool,
    limitations: &[String],
    restriction: Arc<dyn Restriction>,
    faults: FaultInjection,
) -> i32
where
    W: AsyncWrite + Unpin + ?Sized,
{
    let exchange_started = tokio::time::Instant::now();
    let initialize_deadline = exchange_started + INITIALIZE_WAIT_TIMEOUT;
    let mut session = match Session::new(bounds) {
        Ok(s) => s,
        Err(_) => return EXIT_NO_TERMINAL,
    };
    let mut reader = AsyncLineReader::new(stdin, bounds);
    let mut output = FrameOutput::new(stdout, bounds, initialize_deadline);

    let mut seed_id: Option<RequestId> = None;

    // --- read initialize (establishes the seed request id) ---
    let init = match tokio::time::timeout_at(
        initialize_deadline,
        recv_host_frame(&mut reader, &mut session, &mut seed_id),
    )
    .await
    {
        Err(_) => return EXIT_NO_TERMINAL,
        Ok(frame) => match frame {
            HostIn::Frame(HostToBackend::Initialize(p)) => p,
            HostIn::Frame(_) => {
                // First frame was not initialize: a protocol violation. If a seed id
                // somehow exists, report it; otherwise the host classifies the
                // silence. (initialize is the only frame that seeds, so this is
                // usually EXIT_NO_TERMINAL.)
                return output
                    .fail_or_silent(&seed_id, FailureCode::ProtocolViolation)
                    .await;
            }
            HostIn::Eof | HostIn::Error => return EXIT_NO_TERMINAL,
        },
    };
    let Some(deadline) = exchange_started.checked_add(Duration::from_millis(init.deadline_ms))
    else {
        return output
            .emit_failed_or_silent(
                &seed_id,
                FailureCode::ProtocolViolation,
                FailurePhase::Handshake,
            )
            .await;
    };
    output.set_deadline(deadline);
    if tokio::time::Instant::now() >= deadline {
        return output
            .emit_failed_or_silent(
                &seed_id,
                FailureCode::ExecutionTimedOut,
                FailurePhase::Handshake,
            )
            .await;
    }
    let policy = parse_adapter_policy(&init);
    if tokio::time::Instant::now() >= deadline {
        return output
            .emit_failed_or_silent(
                &seed_id,
                FailureCode::ExecutionTimedOut,
                FailurePhase::Handshake,
            )
            .await;
    }
    let Some(policy) = policy else {
        return output
            .emit_failed_or_silent(
                &seed_id,
                FailureCode::ProtocolViolation,
                FailurePhase::Handshake,
            )
            .await;
    };
    let runner = SandboxRunner::new(policy, restriction).with_faults(faults);

    // --- negotiate (first-match by host preference) ---
    let backend_supported: BTreeSet<ProtocolId> =
        [ProtocolId::new(WIRE_IDENTITY).expect("v1 wire identity is non-empty")]
            .into_iter()
            .collect();
    let selected = match select(&init.supported_protocols, &backend_supported) {
        Ok(p) => p,
        Err(_) => {
            return output
                .emit_failed_or_silent(
                    &seed_id,
                    FailureCode::ProtocolIncompatible,
                    FailurePhase::Handshake,
                )
                .await;
        }
    };
    let ready = BackendToHost::Ready(ReadyPayload {
        request_id: seed_id.clone().expect("seed established by initialize"),
        selected_protocol: selected,
        implementation: ImplementationId::new("opi-sandbox").expect("static identity is non-empty"),
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        target: TargetId::new(env!("OPI_SANDBOX_BUILD_TARGET")),
    });
    if !output.emit_frame(&ready).await {
        return EXIT_NO_TERMINAL;
    }

    // --- read execute ---
    let exec = match tokio::time::timeout_at(
        deadline,
        recv_host_frame(&mut reader, &mut session, &mut seed_id),
    )
    .await
    {
        Err(_) => {
            return output
                .emit_failed_or_silent(
                    &seed_id,
                    FailureCode::ExecutionTimedOut,
                    FailurePhase::Handshake,
                )
                .await;
        }
        Ok(frame) => match frame {
            HostIn::Frame(HostToBackend::Execute(p)) => p,
            HostIn::Frame(_) | HostIn::Eof | HostIn::Error => {
                return output
                    .fail_or_silent(&seed_id, FailureCode::ProtocolViolation)
                    .await;
            }
        },
    };
    // Build and validate every side-effect-free request invariant before
    // admission. Restriction setup and process spawning remain after Accepted.
    let cancel = CancellationToken::new();
    let request = match helper::build_request(&exec, cancel.clone()) {
        Ok(request) => request,
        Err(code) => {
            return output
                .emit_failed_or_silent(&seed_id, code, FailurePhase::Handshake)
                .await;
        }
    };
    let request = match helper::validate_request_until(&runner, request, deadline).await {
        Ok(request) => request,
        Err(code) => {
            return output
                .emit_failed_or_silent(&seed_id, code, FailurePhase::Handshake)
                .await;
        }
    };
    let cleanup_cutoff = deadline
        .checked_sub(CLEANUP_RESERVE)
        .unwrap_or(exchange_started);
    if tokio::time::Instant::now() >= cleanup_cutoff {
        return output
            .emit_failed_or_silent(
                &seed_id,
                FailureCode::ExecutionTimedOut,
                FailurePhase::Handshake,
            )
            .await;
    }
    let deadlines = RunDeadlines::new(
        cleanup_cutoff,
        deadline,
        Duration::from_millis(exec.timeout_ms),
    );
    if !output
        .emit_frame(&BackendToHost::Accepted(AcceptedPayload {
            request_id: seed_id.clone().expect("seed present"),
        }))
        .await
    {
        return EXIT_NO_TERMINAL;
    }

    // --- helper start gate (atomic: setup all-or-nothing) ---
    let start_outcome = helper::start(supported, &runner, request, deadlines).await;
    let mut run = match start_outcome {
        StartOutcome::Ready { run } => run,
        StartOutcome::Refused { code } => {
            return output
                .emit_failed_or_silent(&seed_id, code, FailurePhase::Handshake)
                .await;
        }
        StartOutcome::Expired { mut run } => {
            cancel.cancel();
            run.keep_gated();
            let cleanup = drain_cancelled_run(&mut run, deadline).await;
            if cleanup.is_none_or(|result| result.cleanup != CleanupState::Confirmed) {
                return output
                    .emit_post_start_failure(&seed_id, PostStartFailure::CleanupUnconfirmed)
                    .await;
            }
            return output
                .emit_failed_or_silent(
                    &seed_id,
                    FailureCode::ExecutionTimedOut,
                    FailurePhase::Handshake,
                )
                .await;
        }
        StartOutcome::CleanupUnconfirmed => {
            return output
                .emit_post_start_failure(&seed_id, PostStartFailure::CleanupUnconfirmed)
                .await;
        }
    };
    // The execute timeout starts once setup has established the gated run. It
    // is converted to an absolute cutoff exactly once here and is still capped
    // by the request-wide cleanup cutoff, so setup has consumed the hard
    // Initialize budget without consuming target runtime.
    let execution_deadline = deadlines.execution_deadline_at(tokio::time::Instant::now());

    // --- poll Started, build + flush the started frame (output-relay gate) ---
    let id = seed_id.clone().expect("seed present");
    let (mechanism, contract) =
        match tokio::time::timeout_at(execution_deadline, next_event(&mut run)).await {
            Err(_) => {
                cancel.cancel();
                run.keep_gated();
                let cleanup = drain_cancelled_run(&mut run, deadline).await;
                if cleanup.is_none_or(|result| result.cleanup != CleanupState::Confirmed) {
                    return output
                        .emit_post_start_failure(&seed_id, PostStartFailure::CleanupUnconfirmed)
                        .await;
                }
                return output
                    .emit_failed_or_silent(
                        &seed_id,
                        FailureCode::ExecutionTimedOut,
                        FailurePhase::Handshake,
                    )
                    .await;
            }
            Ok(event) => match event {
                Some(SandboxEvent::Started {
                    mechanism,
                    contract,
                    ..
                }) => (mechanism, contract),
                Some(_) | None => {
                    // The stream produced no Started event: setup did not establish a
                    // started contract. Treat as a pre-start failure.
                    return output
                        .emit_failed_or_silent(
                            &seed_id,
                            FailureCode::Failed,
                            FailurePhase::Handshake,
                        )
                        .await;
                }
            },
        };
    if tokio::time::Instant::now() >= execution_deadline {
        cancel.cancel();
        run.keep_gated();
        let cleanup = drain_cancelled_run(&mut run, deadline).await;
        if cleanup.is_none_or(|result| result.cleanup != CleanupState::Confirmed) {
            return output
                .emit_post_start_failure(&seed_id, PostStartFailure::CleanupUnconfirmed)
                .await;
        }
        return output
            .emit_failed_or_silent(
                &seed_id,
                FailureCode::ExecutionTimedOut,
                FailurePhase::Handshake,
            )
            .await;
    }
    let started = BackendToHost::Started(helper::started_payload(
        &id,
        mechanism,
        contract,
        limitations,
    ));
    if !output.emit_frame(&started).await {
        return EXIT_NO_TERMINAL;
    }
    // `started` is a publication gate, not permission to reset time. Recheck
    // the original execution cutoff immediately before releasing the target.
    if tokio::time::Instant::now() >= execution_deadline {
        cancel.cancel();
        run.keep_gated();
        let cleanup = drain_cancelled_run(&mut run, deadline).await;
        if cleanup.is_none_or(|result| result.cleanup != CleanupState::Confirmed) {
            return output
                .emit_post_start_failure(&seed_id, PostStartFailure::CleanupUnconfirmed)
                .await;
        }
        return output
            .emit_failed_or_silent(
                &seed_id,
                FailureCode::ExecutionTimedOut,
                FailurePhase::Execution,
            )
            .await;
    }
    // The real target remains behind the runner's release gate until the
    // `started` frame has been written and flushed.
    if run.release().is_err() {
        cancel.cancel();
        run.keep_gated();
        if drain_cancelled_run(&mut run, deadline)
            .await
            .is_none_or(|result| result.cleanup != CleanupState::Confirmed)
        {
            return output
                .emit_post_start_failure(&seed_id, PostStartFailure::CleanupUnconfirmed)
                .await;
        }
        return output
            .emit_post_start_failure(&seed_id, PostStartFailure::Release)
            .await;
    }

    // --- drain: host input has deterministic precedence over deadline and
    // completion. Thus an already-buffered cancel wins a simultaneous exit.
    let mut cancel_requested = false;
    let result = loop {
        tokio::select! {
            biased;
            frame = recv_host_frame(&mut reader, &mut session, &mut seed_id), if !cancel_requested => match frame {
                HostIn::Frame(HostToBackend::Cancel(_)) => {
                        cancel_requested = true;
                        cancel.cancel();
                }
                HostIn::Frame(_) | HostIn::Eof | HostIn::Error => {
                    cancel.cancel();
                    if drain_cancelled_run(&mut run, deadline)
                        .await
                        .is_none_or(|result| result.cleanup != CleanupState::Confirmed)
                    {
                        return output
                            .emit_post_start_failure(
                                &seed_id,
                                PostStartFailure::CleanupUnconfirmed,
                            )
                            .await;
                    }
                    return output
                        .emit_failed_or_silent(
                            &seed_id,
                            FailureCode::ProtocolViolation,
                            FailurePhase::Execution,
                        )
                        .await;
                }
            },
            _ = tokio::time::sleep_until(execution_deadline), if !cancel_requested => {
                cancel.cancel();
                let Some(mut result) = drain_cancelled_run(&mut run, deadline).await else {
                    return output
                        .emit_post_start_failure(&seed_id, PostStartFailure::CleanupUnconfirmed)
                        .await;
                };
                if result.cleanup != CleanupState::Confirmed {
                    return output
                        .emit_post_start_failure(&seed_id, PostStartFailure::CleanupUnconfirmed)
                        .await;
                }
                result.outcome = SandboxOutcome::TimedOut;
                break result;
            },
            _ = tokio::time::sleep_until(deadline) => {
                cancel.cancel();
                return output
                    .emit_post_start_failure(&seed_id, PostStartFailure::CleanupUnconfirmed)
                    .await;
            },
            ev = next_event(&mut run) => match ev {
                Some(SandboxEvent::Output { stream, bytes }) => {
                    if !output.emit_output_event(&id, stream, &bytes).await {
                        return EXIT_NO_TERMINAL;
                    }
                }
                Some(SandboxEvent::Diagnostic { message }) => {
                    if !output
                        .emit_frame(&BackendToHost::Diagnostic(DiagnosticPayload {
                            request_id: id.clone(),
                            message,
                        }))
                        .await
                    {
                        return EXIT_NO_TERMINAL;
                    }
                }
                Some(SandboxEvent::Completed(mut result)) => {
                    if cancel_requested {
                        result.outcome = SandboxOutcome::Cancelled;
                    }
                    break result;
                }
                Some(SandboxEvent::Started { .. }) => continue,
                None => {
                    return output
                        .emit_post_start_failure(&seed_id, PostStartFailure::StreamEnded)
                        .await;
                }
            },
        }
    };

    // --- output has already been relayed incrementally; emit completed ---
    let completed = BackendToHost::Completed(completed_payload(&id, &result));
    if !output.emit_frame(&completed).await {
        return EXIT_NO_TERMINAL;
    }

    // Terminal frame emitted + flushed: exit 0. The host closes stdin and reaps
    // expecting exit 0; the backend does NOT read another host frame.
    EXIT_OK
}

/// Drive one exchange with the live platform posture. The standalone binary
/// owns the process-only native-stdin bridge and supplies it here; reusable
/// library callers can supply any owned asynchronous reader without workers.
/// Supported Linux/macOS postures execute with their native restriction;
/// unsupported postures refuse before target start.
pub async fn run(stdin: Pin<Box<dyn AsyncRead + Send>>) -> i32 {
    let posture = platform::current();
    let restriction = posture
        .restriction
        .clone()
        .unwrap_or_else(|| Arc::new(NoRestriction));
    let mut stdout = tokio::io::stdout();
    drive(
        stdin,
        &mut stdout,
        Bounds::DEFAULT,
        posture.supported,
        &posture.limitations,
        restriction,
    )
    .await
}

/// Parse the bounded opaque wire value as the backend's closed adapter
/// configuration. Missing fields retain the standalone SDK defaults, so `{}`
/// is `workspace-write` + `deny`. No aliases or extension keys are accepted.
fn parse_adapter_policy(init: &InitializePayload) -> Option<SandboxPolicy> {
    let config = init.adapter_config.as_object()?;
    if config
        .keys()
        .any(|key| key != "profile" && key != "network")
    {
        return None;
    }

    let profile = match config.get("profile") {
        None => Profile::default(),
        Some(value) => match value.as_str() {
            Some("workspace-write") => Profile::WorkspaceWrite,
            _ => return None,
        },
    };
    let network = match config.get("network") {
        None => NetworkPolicy::default(),
        Some(value) => match value.as_str() {
            Some("deny") => NetworkPolicy::Deny,
            Some("allow") => NetworkPolicy::Allow,
            _ => return None,
        },
    };

    Some(SandboxPolicy::new(profile, network))
}

// ---------------------------------------------------------------------------
// Frame I/O, mapping, and the blocking stdin reader
// ---------------------------------------------------------------------------

/// Result of reading + session-checking one host line.
enum HostIn {
    /// A decoded host frame (already observed by the session).
    Frame(HostToBackend),
    /// Clean EOF before a frame.
    Eof,
    /// A read/codec/session error.
    Error,
}

/// Read one host frame from the channel, decode + observe it (seeding
/// `seed_id` from the first frame's request id). Returns [`HostIn::Error`] on
/// any codec or session invariant violation (the caller decides whether a seed
/// id exists to echo in a `failed` frame).
async fn recv_host_frame(
    reader: &mut AsyncLineReader,
    session: &mut Session,
    seed_id: &mut Option<RequestId>,
) -> HostIn {
    match reader.read_line().await {
        Ok(Some(line)) => match session.feed_host_line(&line) {
            Ok(frame) => {
                if seed_id.is_none() {
                    *seed_id = Some(frame.request_id().clone());
                }
                HostIn::Frame(frame)
            }
            Err(_) => HostIn::Error,
        },
        Ok(None) => HostIn::Eof,
        Err(_) => HostIn::Error,
    }
}

/// Ordered, deadline-aware backend frame output.
struct FrameOutput<'a, W: ?Sized> {
    stdout: &'a mut W,
    bounds: Bounds,
    deadline: tokio::time::Instant,
}

impl<W> FrameOutput<'_, W>
where
    W: AsyncWrite + Unpin + ?Sized,
{
    fn new(stdout: &mut W, bounds: Bounds, deadline: tokio::time::Instant) -> FrameOutput<'_, W> {
        FrameOutput {
            stdout,
            bounds,
            deadline,
        }
    }

    fn set_deadline(&mut self, deadline: tokio::time::Instant) {
        self.deadline = deadline;
    }

    /// Encode, write, and flush one backend frame before the request deadline.
    async fn emit_frame(&mut self, frame: &BackendToHost) -> bool {
        let Ok(line) = encode_backend(frame, &self.bounds) else {
            return false;
        };
        tokio::time::timeout_at(self.deadline, async {
            self.stdout.write_all(line.as_bytes()).await?;
            self.stdout.write_all(b"\n").await?;
            self.stdout.flush().await
        })
        .await
        .is_ok_and(|result| result.is_ok())
    }

    /// Emit a `failed` frame if a seed id exists; otherwise exit silent.
    async fn emit_failed_or_silent(
        &mut self,
        seed_id: &Option<RequestId>,
        code: FailureCode,
        phase: FailurePhase,
    ) -> i32 {
        match seed_id {
            Some(id) if self.emit_failed(id, code, phase).await => EXIT_OK,
            Some(_) | None => EXIT_NO_TERMINAL,
        }
    }

    async fn fail_or_silent(&mut self, seed_id: &Option<RequestId>, code: FailureCode) -> i32 {
        self.emit_failed_or_silent(seed_id, code, FailurePhase::Handshake)
            .await
    }

    async fn emit_failed(
        &mut self,
        id: &RequestId,
        code: FailureCode,
        phase: FailurePhase,
    ) -> bool {
        self.emit_frame(&BackendToHost::Failed(FailedPayload {
            request_id: id.clone(),
            code,
            phase,
            message: None,
            diagnostics: vec![],
        }))
        .await
    }

    async fn emit_post_start_failure(
        &mut self,
        seed_id: &Option<RequestId>,
        failure: PostStartFailure,
    ) -> i32 {
        let (code, phase) = classify_post_start_failure(failure);
        self.emit_failed_or_silent(seed_id, code, phase).await
    }

    /// Emit captured target bytes as bounded base64 output frames.
    async fn emit_output_event(
        &mut self,
        id: &RequestId,
        stream: OutputStream,
        data: &[u8],
    ) -> bool {
        let chunk = self.bounds.max_decoded_chunk_size.max(1);
        for piece in data.chunks(chunk) {
            let frame = if matches!(stream, OutputStream::Stdout) {
                BackendToHost::Stdout(StdoutPayload {
                    request_id: id.clone(),
                    data: Base64Bytes::from_bytes(piece),
                })
            } else {
                BackendToHost::Stderr(StderrPayload {
                    request_id: id.clone(),
                    data: Base64Bytes::from_bytes(piece),
                })
            };
            if !self.emit_frame(&frame).await {
                return false;
            }
        }
        true
    }
}

/// Map a terminal [`crate::SandboxResult`] to the wire `completed` payload.
/// Nonzero exit and signal are IN-BAND (not failures). Exit codes are narrowed
/// to the 0..=255 process range; local SDK cleanup is always `Confirmed`.
fn completed_payload(id: &RequestId, result: &crate::runner::SandboxResult) -> CompletedPayload {
    let (exit, signal, timed_out, cancelled) = match result.outcome {
        SandboxOutcome::Exited { code } => (code.map(|c| (c & 0xFF) as u32), None, false, false),
        SandboxOutcome::Signaled { signal } => (None, Some(signal as u32), false, false),
        SandboxOutcome::TimedOut => (None, None, true, false),
        SandboxOutcome::Cancelled => (None, None, false, true),
    };
    CompletedPayload {
        request_id: id.clone(),
        exit,
        signal,
        timed_out,
        cancelled,
        cleanup: map_cleanup(result.cleanup),
        diagnostics: [
            result.stdout_truncated.then(|| Diagnostic {
                message: "stdout capture truncated".to_string(),
            }),
            result.stderr_truncated.then(|| Diagnostic {
                message: "stderr capture truncated".to_string(),
            }),
        ]
        .into_iter()
        .flatten()
        .collect(),
    }
}

fn map_cleanup(c: CleanupState) -> WireCleanup {
    match c {
        CleanupState::Confirmed => WireCleanup::Confirmed,
        CleanupState::Unconfirmed => WireCleanup::Unconfirmed,
    }
}

/// Poll one event from the run stream without a `futures-util` dependency (the
/// crate depends only on `futures_core` for the [`Stream`] trait).
async fn next_event<S>(run: &mut S) -> Option<SandboxEvent>
where
    S: Stream<Item = SandboxEvent> + Unpin,
{
    std::future::poll_fn(|cx| Pin::new(&mut *run).poll_next(cx)).await
}

async fn next_completed<S>(run: &mut S) -> Option<crate::runner::SandboxResult>
where
    S: Stream<Item = SandboxEvent> + Unpin,
{
    loop {
        match next_event(run).await {
            Some(SandboxEvent::Completed(result)) => return Some(result),
            Some(_) => {}
            None => return None,
        }
    }
}

async fn drain_cancelled_run<S>(
    run: &mut S,
    deadline: tokio::time::Instant,
) -> Option<crate::runner::SandboxResult>
where
    S: Stream<Item = SandboxEvent> + Unpin,
{
    tokio::time::timeout_at(deadline, next_completed(run))
        .await
        .ok()
        .flatten()
}

/// Cancellation-safe bounded async JSONL reader. Partial bytes live in this
/// object, so dropping an in-progress `read_line` future never loses data and
/// dropping the reader releases its owned input directly.
struct AsyncLineReader {
    inner: tokio::io::BufReader<Pin<Box<dyn AsyncRead + Send>>>,
    line: Vec<u8>,
    accumulator: CappedLineAccumulator,
}

impl AsyncLineReader {
    fn new(reader: Pin<Box<dyn AsyncRead + Send>>, bounds: Bounds) -> Self {
        Self {
            inner: tokio::io::BufReader::new(reader),
            line: Vec::new(),
            accumulator: CappedLineAccumulator::new(bounds.max_line_size),
        }
    }

    async fn read_line(&mut self) -> Result<Option<Vec<u8>>, CodecError> {
        let mut byte = [0u8; 1];
        loop {
            match self.inner.read(&mut byte).await? {
                0 => {
                    return if !self.accumulator.finish_eof(&mut self.line)? {
                        Ok(None)
                    } else {
                        Ok(Some(std::mem::take(&mut self.line)))
                    };
                }
                _ if self.accumulator.push_byte(byte[0], &mut self.line)?
                    == LineAccumulatorEvent::Complete =>
                {
                    return Ok(Some(std::mem::take(&mut self.line)));
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opi_protocol::execution::v1::EnvInherit;
    use tokio::io::AsyncWriteExt;

    #[cfg(unix)]
    fn test_native(value: &std::ffi::OsStr) -> opi_protocol::execution::v1::NativeString {
        use std::os::unix::ffi::OsStrExt;
        opi_protocol::execution::v1::NativeString::from_bytes(value.as_bytes())
    }

    #[cfg(windows)]
    fn test_native(value: &std::ffi::OsStr) -> opi_protocol::execution::v1::NativeString {
        use std::os::windows::ffi::OsStrExt;
        opi_protocol::execution::v1::NativeString::from_bytes(
            value
                .encode_wide()
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<_>>(),
        )
    }

    #[tokio::test]
    async fn delayed_filesystem_validation_fails_before_accepted() {
        let workspace = tempfile::tempdir().expect("workspace");
        let marker_dir = tempfile::tempdir().expect("marker dir");
        let marker = marker_dir.path().join("must-not-exist");
        let request_id = RequestId::new("r1".to_string()).expect("request id");
        let initialize = HostToBackend::Initialize(InitializePayload {
            request_id: request_id.clone(),
            deadline_ms: 100,
            adapter_config: serde_json::json!({}),
            supported_protocols: vec![ProtocolId::new(WIRE_IDENTITY).expect("protocol id")],
        });
        let execute = HostToBackend::Execute(opi_protocol::execution::v1::frames::ExecutePayload {
            request_id,
            program: test_native(std::ffi::OsStr::new("target-must-not-run")),
            args: Vec::new(),
            workspace: test_native(workspace.path().as_os_str()),
            cwd: test_native(workspace.path().as_os_str()),
            timeout_ms: 1_000,
            env_inherit: EnvInherit::Inherit,
            env_additions: Default::default(),
        });
        let input = format!(
            "{}\n{}\n",
            opi_protocol::execution::v1::encode_line(&initialize, &Bounds::DEFAULT)
                .expect("initialize line"),
            opi_protocol::execution::v1::encode_line(&execute, &Bounds::DEFAULT)
                .expect("execute line"),
        )
        .into_bytes();
        let (mut host, reader) = tokio::io::duplex(input.len());
        host.write_all(&input).await.expect("write request");
        let mut out = Vec::new();

        let code = tokio::time::timeout(
            Duration::from_millis(500),
            drive_with_faults(
                Box::pin(reader),
                &mut out,
                Bounds::DEFAULT,
                true,
                &[],
                Arc::new(NoRestriction),
                FaultInjection {
                    validation_delay: Duration::from_millis(300),
                    ..FaultInjection::default()
                },
            ),
        )
        .await
        .expect("initialize deadline bounds validation");
        drop(host);

        assert_eq!(code, EXIT_OK);
        let frames = out
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| {
                opi_protocol::execution::v1::codec::decode_backend(line)
                    .expect("valid backend frame")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            frames.iter().map(BackendToHost::kind).collect::<Vec<_>>(),
            vec!["ready", "failed"]
        );
        let BackendToHost::Failed(failed) = &frames[1] else {
            panic!("expected terminal failed frame")
        };
        assert_eq!(failed.code, FailureCode::ExecutionTimedOut);
        assert_eq!(failed.phase, FailurePhase::Handshake);
        assert!(!marker.exists(), "validation timeout mutated target state");
    }

    #[tokio::test]
    async fn ended_execution_stream_emits_execution_failed_in_execution_phase() {
        let mut run = futures_util::stream::empty::<SandboxEvent>();
        let failure = match next_event(&mut run).await {
            None => PostStartFailure::StreamEnded,
            Some(event) => panic!("expected ended stream, got {event:?}"),
        };

        let mut out = Vec::new();
        let request_id = Some(RequestId::new("r1".to_string()).expect("valid request id"));
        {
            let mut output = FrameOutput::new(
                &mut out,
                Bounds::DEFAULT,
                tokio::time::Instant::now() + Duration::from_secs(1),
            );
            assert_eq!(
                output.emit_post_start_failure(&request_id, failure).await,
                EXIT_OK
            );
        }
        let line = out
            .strip_suffix(b"\n")
            .expect("one newline-terminated frame");
        let BackendToHost::Failed(failed) =
            opi_protocol::execution::v1::codec::decode_backend(line).expect("valid failed frame")
        else {
            panic!("expected failed frame")
        };
        assert_eq!(failed.code, FailureCode::ExecutionFailed);
        assert_eq!(failed.phase, FailurePhase::Execution);
    }

    #[tokio::test]
    async fn drain_rejects_completed_run_with_unconfirmed_cleanup() {
        let result = crate::runner::SandboxResult {
            outcome: SandboxOutcome::Cancelled,
            cleanup: CleanupState::Unconfirmed,
            stdout: Vec::new(),
            stderr: Vec::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            temp_root: std::path::PathBuf::from("injected-temp-root"),
        };
        let mut run = futures_util::stream::iter([SandboxEvent::Completed(result)]);

        let failure = if drain_cancelled_run(
            &mut run,
            tokio::time::Instant::now() + Duration::from_secs(1),
        )
        .await
        .is_some_and(|result| result.cleanup == CleanupState::Confirmed)
        {
            panic!("unconfirmed cleanup must not count as a successful drain")
        } else {
            PostStartFailure::CleanupUnconfirmed
        };

        let mut out = Vec::new();
        let request_id = Some(RequestId::new("r1".to_string()).expect("valid request id"));
        {
            let mut output = FrameOutput::new(
                &mut out,
                Bounds::DEFAULT,
                tokio::time::Instant::now() + Duration::from_secs(1),
            );
            assert_eq!(
                output.emit_post_start_failure(&request_id, failure).await,
                EXIT_OK
            );
        }
        let line = out
            .strip_suffix(b"\n")
            .expect("one newline-terminated frame");
        let BackendToHost::Failed(failed) =
            opi_protocol::execution::v1::codec::decode_backend(line).expect("valid failed frame")
        else {
            panic!("expected failed frame")
        };
        assert_eq!(failed.code, FailureCode::CleanupUnconfirmed);
        assert_eq!(failed.phase, FailurePhase::Cleanup);
    }

    #[tokio::test(start_paused = true)]
    async fn stuck_cleanup_stops_at_the_original_deadline_and_reports_unconfirmed() {
        let mut run = futures_util::stream::pending::<SandboxEvent>();
        let started = tokio::time::Instant::now();
        let deadline = started + Duration::from_millis(100);
        let drain = drain_cancelled_run(&mut run, deadline);
        tokio::pin!(drain);

        tokio::time::advance(Duration::from_millis(99)).await;
        assert!(
            tokio::time::timeout(Duration::ZERO, &mut drain)
                .await
                .is_err(),
            "cleanup must remain pending before the absolute deadline"
        );
        tokio::time::advance(Duration::from_millis(1)).await;
        assert!(drain.await.is_none());
        assert_eq!(tokio::time::Instant::now(), deadline);

        let mut out = Vec::new();
        let request_id = Some(RequestId::new("r1".to_string()).expect("valid request id"));
        {
            let mut output = FrameOutput::new(
                &mut out,
                Bounds::DEFAULT,
                tokio::time::Instant::now() + Duration::from_secs(1),
            );
            assert_eq!(
                output
                    .emit_post_start_failure(&request_id, PostStartFailure::CleanupUnconfirmed)
                    .await,
                EXIT_OK
            );
        }
        let line = out
            .strip_suffix(b"\n")
            .expect("one newline-terminated frame");
        let BackendToHost::Failed(failed) =
            opi_protocol::execution::v1::codec::decode_backend(line).expect("valid failed frame")
        else {
            panic!("expected failed frame")
        };
        assert_eq!(failed.code, FailureCode::CleanupUnconfirmed);
        assert_eq!(failed.phase, FailurePhase::Cleanup);
    }

    #[test]
    fn empty_adapter_config_maps_to_the_exact_default_policy() {
        let init = InitializePayload {
            request_id: RequestId::new("r1".to_string()).expect("valid request id"),
            deadline_ms: 1_000,
            adapter_config: serde_json::json!({}),
            supported_protocols: vec![
                ProtocolId::new(WIRE_IDENTITY).expect("valid protocol identity"),
            ],
        };

        assert_eq!(
            parse_adapter_policy(&init),
            Some(SandboxPolicy::new(
                Profile::WorkspaceWrite,
                NetworkPolicy::Deny,
            ))
        );
    }
}
