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
use std::io::Write;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures_core::Stream;
use opi_protocol::execution::v1::codec::{CodecError, encode_backend};
use opi_protocol::execution::v1::frames::{
    AcceptedPayload, CompletedPayload, Diagnostic, DiagnosticPayload, FailedPayload,
    InitializePayload, ReadyPayload, StderrPayload, StdoutPayload,
};
use opi_protocol::execution::v1::{
    BackendToHost, Base64Bytes, Bounds, CleanupState as WireCleanup, FailureCode, FailurePhase,
    HostToBackend, ImplementationId, ProtocolId, RequestId, Session, TargetId, WIRE_IDENTITY,
    select,
};
use tokio::io::{AsyncRead, AsyncReadExt};
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

fn emit_post_start_failure(
    stdout: &mut dyn Write,
    bounds: Bounds,
    seed_id: &Option<RequestId>,
    failure: PostStartFailure,
) -> i32 {
    let (code, phase) = classify_post_start_failure(failure);
    emit_failed_or_silent(stdout, bounds, seed_id, code, phase)
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
pub async fn drive(
    stdin: Pin<Box<dyn AsyncRead + Send>>,
    stdout: &mut dyn Write,
    bounds: Bounds,
    supported: bool,
    limitations: &[String],
    restriction: Arc<dyn Restriction>,
) -> i32 {
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

async fn drive_with_faults(
    stdin: Pin<Box<dyn AsyncRead + Send>>,
    stdout: &mut dyn Write,
    bounds: Bounds,
    supported: bool,
    limitations: &[String],
    restriction: Arc<dyn Restriction>,
    faults: FaultInjection,
) -> i32 {
    let exchange_started = tokio::time::Instant::now();
    let mut session = match Session::new(bounds) {
        Ok(s) => s,
        Err(_) => return EXIT_NO_TERMINAL,
    };
    let mut reader = AsyncLineReader::new(stdin, bounds);

    let mut seed_id: Option<RequestId> = None;

    // --- read initialize (establishes the seed request id) ---
    let init = match tokio::time::timeout(
        INITIALIZE_WAIT_TIMEOUT,
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
                return fail_or_silent(stdout, bounds, &seed_id, FailureCode::ProtocolViolation);
            }
            HostIn::Eof | HostIn::Error => return EXIT_NO_TERMINAL,
        },
    };
    let Some(deadline) = exchange_started.checked_add(Duration::from_millis(init.deadline_ms))
    else {
        return emit_failed_or_silent(
            stdout,
            bounds,
            &seed_id,
            FailureCode::ProtocolViolation,
            FailurePhase::Handshake,
        );
    };
    if tokio::time::Instant::now() >= deadline {
        return emit_failed_or_silent(
            stdout,
            bounds,
            &seed_id,
            FailureCode::ExecutionTimedOut,
            FailurePhase::Handshake,
        );
    }
    let policy = parse_adapter_policy(&init);
    if tokio::time::Instant::now() >= deadline {
        return emit_failed_or_silent(
            stdout,
            bounds,
            &seed_id,
            FailureCode::ExecutionTimedOut,
            FailurePhase::Handshake,
        );
    }
    let Some(policy) = policy else {
        return emit_failed_or_silent(
            stdout,
            bounds,
            &seed_id,
            FailureCode::ProtocolViolation,
            FailurePhase::Handshake,
        );
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
            return emit_failed_or_silent(
                stdout,
                bounds,
                &seed_id,
                FailureCode::ProtocolIncompatible,
                FailurePhase::Handshake,
            );
        }
    };
    let ready = BackendToHost::Ready(ReadyPayload {
        request_id: seed_id.clone().expect("seed established by initialize"),
        selected_protocol: selected,
        implementation: ImplementationId::new("opi-sandbox").expect("static identity is non-empty"),
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        target: TargetId::new(env!("OPI_SANDBOX_BUILD_TARGET")),
    });
    if !emit_frame(stdout, bounds, &ready) {
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
            return emit_failed_or_silent(
                stdout,
                bounds,
                &seed_id,
                FailureCode::ExecutionTimedOut,
                FailurePhase::Handshake,
            );
        }
        Ok(frame) => match frame {
            HostIn::Frame(HostToBackend::Execute(p)) => p,
            HostIn::Frame(_) | HostIn::Eof | HostIn::Error => {
                return fail_or_silent(stdout, bounds, &seed_id, FailureCode::ProtocolViolation);
            }
        },
    };
    // Build and validate every side-effect-free request invariant before
    // admission. Restriction setup and process spawning remain after Accepted.
    let cancel = CancellationToken::new();
    let request = match helper::build_request(&exec, cancel.clone()) {
        Ok(request) => request,
        Err(code) => {
            return emit_failed_or_silent(stdout, bounds, &seed_id, code, FailurePhase::Handshake);
        }
    };
    let request = match helper::validate_request_until(&runner, request, deadline).await {
        Ok(request) => request,
        Err(code) => {
            return emit_failed_or_silent(stdout, bounds, &seed_id, code, FailurePhase::Handshake);
        }
    };
    let cleanup_cutoff = deadline
        .checked_sub(CLEANUP_RESERVE)
        .unwrap_or(exchange_started);
    if tokio::time::Instant::now() >= cleanup_cutoff {
        return emit_failed_or_silent(
            stdout,
            bounds,
            &seed_id,
            FailureCode::ExecutionTimedOut,
            FailurePhase::Handshake,
        );
    }
    let deadlines = RunDeadlines::new(
        cleanup_cutoff,
        deadline,
        Duration::from_millis(exec.timeout_ms),
    );
    if !emit_frame(
        stdout,
        bounds,
        &BackendToHost::Accepted(AcceptedPayload {
            request_id: seed_id.clone().expect("seed present"),
        }),
    ) {
        return EXIT_NO_TERMINAL;
    }

    // --- helper start gate (atomic: setup all-or-nothing) ---
    let start_outcome = helper::start(supported, &runner, request, deadlines).await;
    let mut run = match start_outcome {
        StartOutcome::Ready { run } => run,
        StartOutcome::Refused { code } => {
            return emit_failed_or_silent(stdout, bounds, &seed_id, code, FailurePhase::Handshake);
        }
        StartOutcome::Expired { mut run } => {
            cancel.cancel();
            run.keep_gated();
            let cleanup = drain_cancelled_run(&mut run, deadline).await;
            if cleanup.is_none_or(|result| result.cleanup != CleanupState::Confirmed) {
                return emit_post_start_failure(
                    stdout,
                    bounds,
                    &seed_id,
                    PostStartFailure::CleanupUnconfirmed,
                );
            }
            return emit_failed_or_silent(
                stdout,
                bounds,
                &seed_id,
                FailureCode::ExecutionTimedOut,
                FailurePhase::Handshake,
            );
        }
        StartOutcome::CleanupUnconfirmed => {
            return emit_post_start_failure(
                stdout,
                bounds,
                &seed_id,
                PostStartFailure::CleanupUnconfirmed,
            );
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
                    return emit_post_start_failure(
                        stdout,
                        bounds,
                        &seed_id,
                        PostStartFailure::CleanupUnconfirmed,
                    );
                }
                return emit_failed_or_silent(
                    stdout,
                    bounds,
                    &seed_id,
                    FailureCode::ExecutionTimedOut,
                    FailurePhase::Handshake,
                );
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
                    return emit_failed_or_silent(
                        stdout,
                        bounds,
                        &seed_id,
                        FailureCode::Failed,
                        FailurePhase::Handshake,
                    );
                }
            },
        };
    if tokio::time::Instant::now() >= execution_deadline {
        cancel.cancel();
        run.keep_gated();
        let cleanup = drain_cancelled_run(&mut run, deadline).await;
        if cleanup.is_none_or(|result| result.cleanup != CleanupState::Confirmed) {
            return emit_post_start_failure(
                stdout,
                bounds,
                &seed_id,
                PostStartFailure::CleanupUnconfirmed,
            );
        }
        return emit_failed_or_silent(
            stdout,
            bounds,
            &seed_id,
            FailureCode::ExecutionTimedOut,
            FailurePhase::Handshake,
        );
    }
    let started = BackendToHost::Started(helper::started_payload(
        &id,
        mechanism,
        contract,
        limitations,
    ));
    if !emit_frame(stdout, bounds, &started) {
        return EXIT_NO_TERMINAL;
    }
    // `started` is a publication gate, not permission to reset time. Recheck
    // the original execution cutoff immediately before releasing the target.
    if tokio::time::Instant::now() >= execution_deadline {
        cancel.cancel();
        run.keep_gated();
        let cleanup = drain_cancelled_run(&mut run, deadline).await;
        if cleanup.is_none_or(|result| result.cleanup != CleanupState::Confirmed) {
            return emit_post_start_failure(
                stdout,
                bounds,
                &seed_id,
                PostStartFailure::CleanupUnconfirmed,
            );
        }
        return emit_failed_or_silent(
            stdout,
            bounds,
            &seed_id,
            FailureCode::ExecutionTimedOut,
            FailurePhase::Execution,
        );
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
            return emit_post_start_failure(
                stdout,
                bounds,
                &seed_id,
                PostStartFailure::CleanupUnconfirmed,
            );
        }
        return emit_post_start_failure(stdout, bounds, &seed_id, PostStartFailure::Release);
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
                        return emit_post_start_failure(
                            stdout,
                            bounds,
                            &seed_id,
                            PostStartFailure::CleanupUnconfirmed,
                        );
                    }
                    return emit_failed_or_silent(
                        stdout, bounds, &seed_id,
                        FailureCode::ProtocolViolation, FailurePhase::Execution,
                    );
                }
            },
            _ = tokio::time::sleep_until(execution_deadline), if !cancel_requested => {
                cancel.cancel();
                let Some(mut result) = drain_cancelled_run(&mut run, deadline).await else {
                    return emit_post_start_failure(
                        stdout,
                        bounds,
                        &seed_id,
                        PostStartFailure::CleanupUnconfirmed,
                    );
                };
                if result.cleanup != CleanupState::Confirmed {
                    return emit_post_start_failure(
                        stdout,
                        bounds,
                        &seed_id,
                        PostStartFailure::CleanupUnconfirmed,
                    );
                }
                result.outcome = SandboxOutcome::TimedOut;
                break result;
            },
            _ = tokio::time::sleep_until(deadline) => {
                cancel.cancel();
                return emit_post_start_failure(
                    stdout,
                    bounds,
                    &seed_id,
                    PostStartFailure::CleanupUnconfirmed,
                );
            },
            ev = next_event(&mut run) => match ev {
                Some(SandboxEvent::Output { stream, bytes }) => {
                    if !emit_output_event(stdout, bounds, &id, stream, &bytes) {
                        return EXIT_NO_TERMINAL;
                    }
                }
                Some(SandboxEvent::Diagnostic { message }) => {
                    if !emit_frame(
                        stdout,
                        bounds,
                        &BackendToHost::Diagnostic(DiagnosticPayload {
                            request_id: id.clone(),
                            message,
                        }),
                    ) {
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
                    return emit_post_start_failure(
                        stdout,
                        bounds,
                        &seed_id,
                        PostStartFailure::StreamEnded,
                    );
                }
            },
        }
    };

    // --- output has already been relayed incrementally; emit completed ---
    let completed = BackendToHost::Completed(completed_payload(&id, &result));
    if !emit_frame(stdout, bounds, &completed) {
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
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
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

/// Encode + write + flush one backend frame. Returns false on encode failure
/// (oversized line) or write/flush failure (broken pipe).
fn emit_frame(stdout: &mut dyn Write, bounds: Bounds, frame: &BackendToHost) -> bool {
    let Ok(line) = encode_backend(frame, &bounds) else {
        return false;
    };
    write_all_nl_flush(stdout, line.as_bytes())
}

/// Emit a `failed` frame if a seed id exists (reporting the violation to the
/// host); otherwise exit silent. Always returns the appropriate exit code.
fn emit_failed_or_silent(
    stdout: &mut dyn Write,
    bounds: Bounds,
    seed_id: &Option<RequestId>,
    code: FailureCode,
    phase: FailurePhase,
) -> i32 {
    match seed_id {
        Some(id) => {
            if emit_failed(stdout, bounds, id, code, phase) {
                EXIT_OK
            } else {
                EXIT_NO_TERMINAL
            }
        }
        None => EXIT_NO_TERMINAL,
    }
}

/// Convenience for the common "ProtocolViolation, phase by target_started" case.
fn fail_or_silent(
    stdout: &mut dyn Write,
    bounds: Bounds,
    seed_id: &Option<RequestId>,
    code: FailureCode,
) -> i32 {
    // Callers of this helper are always pre-start (target_started == false).
    emit_failed_or_silent(stdout, bounds, seed_id, code, FailurePhase::Handshake)
}

/// Encode + write + flush a `failed` frame (redacted: no message, no
/// diagnostics). Returns false on emit failure.
fn emit_failed(
    stdout: &mut dyn Write,
    bounds: Bounds,
    id: &RequestId,
    code: FailureCode,
    phase: FailurePhase,
) -> bool {
    emit_frame(
        stdout,
        bounds,
        &BackendToHost::Failed(FailedPayload {
            request_id: id.clone(),
            code,
            phase,
            message: None,
            diagnostics: vec![],
        }),
    )
}

/// Emit captured target bytes as base64 `Stdout` (`is_stdout`) / `Stderr`
/// frames, chunked to `max_decoded_chunk_size` so each encoded line fits
/// `max_line_size`. Empty data emits zero frames. Returns false on emit failure.
fn emit_output(
    stdout: &mut dyn Write,
    bounds: Bounds,
    id: &RequestId,
    data: &[u8],
    is_stdout: bool,
) -> bool {
    let chunk = bounds.max_decoded_chunk_size.max(1);
    for piece in data.chunks(chunk) {
        let frame = if is_stdout {
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
        if !emit_frame(stdout, bounds, &frame) {
            return false;
        }
    }
    true
}

fn emit_output_event(
    stdout: &mut dyn Write,
    bounds: Bounds,
    id: &RequestId,
    stream: OutputStream,
    data: &[u8],
) -> bool {
    emit_output(
        stdout,
        bounds,
        id,
        data,
        matches!(stream, OutputStream::Stdout),
    )
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

/// Write `bytes` + a newline, then flush. Returns false on any I/O error.
fn write_all_nl_flush(stdout: &mut dyn Write, bytes: &[u8]) -> bool {
    let res = (|| -> std::io::Result<()> {
        stdout.write_all(bytes)?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
        Ok(())
    })();
    res.is_ok()
}

/// Cancellation-safe bounded async JSONL reader. Partial bytes live in this
/// object, so dropping an in-progress `read_line` future never loses data and
/// dropping the reader releases its owned input directly.
struct AsyncLineReader {
    inner: tokio::io::BufReader<Pin<Box<dyn AsyncRead + Send>>>,
    max_line_size: usize,
    line: Vec<u8>,
    pending_cr: bool,
}

impl AsyncLineReader {
    fn new(reader: Pin<Box<dyn AsyncRead + Send>>, bounds: Bounds) -> Self {
        Self {
            inner: tokio::io::BufReader::new(reader),
            max_line_size: bounds.max_line_size,
            line: Vec::new(),
            pending_cr: false,
        }
    }

    async fn read_line(&mut self) -> Result<Option<Vec<u8>>, CodecError> {
        let mut byte = [0u8; 1];
        loop {
            match self.inner.read(&mut byte).await? {
                0 => {
                    if self.pending_cr {
                        self.push_byte(b'\r')?;
                        self.pending_cr = false;
                    }
                    return if self.line.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(std::mem::take(&mut self.line)))
                    };
                }
                _ if byte[0] == b'\n' => {
                    self.pending_cr = false;
                    return Ok(Some(std::mem::take(&mut self.line)));
                }
                _ => {
                    if self.pending_cr {
                        self.push_byte(b'\r')?;
                        self.pending_cr = false;
                    }
                    if byte[0] == b'\r' {
                        self.pending_cr = true;
                    } else {
                        self.push_byte(byte[0])?;
                    }
                }
            }
        }
    }

    fn push_byte(&mut self, byte: u8) -> Result<(), CodecError> {
        if self.line.len() >= self.max_line_size {
            return Err(CodecError::OversizedLine {
                max_line_size: self.max_line_size,
            });
        }
        self.line.push(byte);
        Ok(())
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
        assert_eq!(
            emit_post_start_failure(&mut out, Bounds::DEFAULT, &request_id, failure,),
            EXIT_OK
        );
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
        assert_eq!(
            emit_post_start_failure(&mut out, Bounds::DEFAULT, &request_id, failure,),
            EXIT_OK
        );
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
        assert_eq!(
            emit_post_start_failure(
                &mut out,
                Bounds::DEFAULT,
                &request_id,
                PostStartFailure::CleanupUnconfirmed,
            ),
            EXIT_OK
        );
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
