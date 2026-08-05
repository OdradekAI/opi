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
//! read execute     -> emit accepted
//! helper::start    -> Refused{code} -> emit failed{Handshake}; or
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
//! # 16.12 scope (honest)
//!
//! Portable conformance (the full success state machine) is driven by
//! [`drive`] with an INJECTED runner + `supported = true`; the REAL executable
//! ([`run`]) negotiates then refuses at the platform gate
//! (`failed{Unavailable, Handshake}`) because `crate::platform::current` is
//! unsupported on every platform this phase. Successful NATIVE run is owned by
//! 16.13 / 16.14.1.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures_core::Stream;
use opi_protocol::execution::v1::codec::{LineReader, encode_backend};
use opi_protocol::execution::v1::frames::{
    AcceptedPayload, CompletedPayload, Diagnostic, FailedPayload, ReadyPayload, StderrPayload,
    StdoutPayload,
};
use opi_protocol::execution::v1::{
    BackendToHost, Base64Bytes, Bounds, CleanupState as WireCleanup, FailureCode, FailurePhase,
    HostToBackend, ImplementationId, ProtocolId, RequestId, Session, TargetId, WIRE_IDENTITY,
    select,
};
use tokio_util::sync::CancellationToken;

use crate::helper::{self, StartOutcome};
use crate::platform;
use crate::policy::{NoRestriction, SandboxPolicy};
use crate::runner::{CleanupState, SandboxEvent, SandboxOutcome, SandboxRun, SandboxRunner};

/// Backend exit after a clean protocol exchange (a terminal frame was emitted +
/// flushed). The target's own exit is in-band in `completed`.
const EXIT_OK: i32 = 0;
/// Backend exit when no terminal frame could be emitted (the very first frame
/// was malformed/oversized so no request id was established, or the stdout pipe
/// broke). The host classifies unexpected exit / EOF as a protocol violation.
const EXIT_NO_TERMINAL: i32 = 1;

/// A small bounded queue prevents an input-flooding host from growing backend
/// memory without bound. Dropping the receiver after the terminal frame also
/// releases a reader blocked on backpressure.
const INPUT_CHANNEL_CAPACITY: usize = 8;

/// One unit pushed from the blocking stdin reader to the async driver.
enum InputLine {
    /// A capped JSONL line (no trailing newline).
    Line(Vec<u8>),
    /// Clean EOF (no more bytes).
    Eof,
    /// A read or codec error (oversized line / I/O).
    Error,
}

/// Drive one backend exchange over `stdin`/`stdout` with an INJECTED runner and
/// platform posture. This is the pure testable core: production [`run`] wires
/// `std::io::stdin()` / `stdout()` and `platform::current`; portable
/// conformance tests inject `supported = true`, empty limitations, and a
/// [`NoRestriction`] runner to exercise the full success state machine.
///
/// `stdin` is owned + `Send` so the blocking reader can run on a dedicated
/// thread; `stdout` is borrowed for the whole exchange and flushed after every
/// emitted frame. Returns `EXIT_OK` (0) after a terminal frame, or
/// `EXIT_NO_TERMINAL` (1) if none could be emitted.
pub async fn drive(
    stdin: Box<dyn Read + Send>,
    stdout: &mut dyn Write,
    bounds: Bounds,
    supported: bool,
    limitations: &[String],
    runner: &SandboxRunner,
) -> i32 {
    let exchange_started = tokio::time::Instant::now();
    let mut session = match Session::new(bounds) {
        Ok(s) => s,
        Err(_) => return EXIT_NO_TERMINAL,
    };
    // Bridge the sync LineReader to the async driver: a blocking reader thread
    // owns stdin and feeds capped lines through a bounded channel. This lets
    // the drain loop `select!` between a host `cancel` frame and the run poll
    // (the opi-protocol LineReader is sync `R: Read` and cannot live in a select).
    let (tx, mut rx) = tokio::sync::mpsc::channel::<InputLine>(INPUT_CHANNEL_CAPACITY);
    let _reader = tokio::task::spawn_blocking(move || run_reader(stdin, bounds, tx));

    let mut seed_id: Option<RequestId> = None;

    // --- read initialize (establishes the seed request id) ---
    let init = match recv_host_frame(&mut rx, &mut session, &mut seed_id).await {
        HostIn::Frame(HostToBackend::Initialize(p)) => p,
        HostIn::Frame(_) => {
            // First frame was not initialize: a protocol violation. If a seed id
            // somehow exists, report it; otherwise the host classifies the
            // silence. (initialize is the only frame that seeds, so this is
            // usually EXIT_NO_TERMINAL.)
            return fail_or_silent(stdout, bounds, &seed_id, FailureCode::ProtocolViolation);
        }
        HostIn::Eof | HostIn::Error => return EXIT_NO_TERMINAL,
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

    // --- negotiate (first-match by host preference) ---
    let backend_supported: BTreeSet<ProtocolId> =
        [ProtocolId::new(WIRE_IDENTITY)].into_iter().collect();
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
        recv_host_frame(&mut rx, &mut session, &mut seed_id),
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
    let cancel = CancellationToken::new();
    let mut request = match helper::build_request(&exec, cancel.clone()) {
        Ok(request) => request,
        Err(code) => {
            return emit_failed_or_silent(stdout, bounds, &seed_id, code, FailurePhase::Handshake);
        }
    };
    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
    if remaining.is_zero() {
        return emit_failed_or_silent(
            stdout,
            bounds,
            &seed_id,
            FailureCode::ExecutionTimedOut,
            FailurePhase::Handshake,
        );
    }
    request.timeout = request.timeout.min(remaining);
    let mut run = match helper::start(supported, runner, request) {
        StartOutcome::Ready { run } => run,
        StartOutcome::Refused { code } => {
            return emit_failed_or_silent(stdout, bounds, &seed_id, code, FailurePhase::Handshake);
        }
    };

    // --- poll Started, build + flush the started frame (output-relay gate) ---
    let id = seed_id.clone().expect("seed present");
    let (mechanism, contract) = match next_event(&mut run).await {
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
    let started = BackendToHost::Started(helper::started_payload(
        &id,
        mechanism,
        contract,
        limitations,
    ));
    if !emit_frame(stdout, bounds, &started) {
        return EXIT_NO_TERMINAL;
    }
    // The real target remains behind the runner's release gate until the
    // `started` frame has been written and flushed.
    if run.release().is_err() {
        cancel.cancel();
        if !drain_cancelled_run(&mut run, deadline).await {
            return emit_failed_or_silent(
                stdout,
                bounds,
                &seed_id,
                FailureCode::CleanupUnconfirmed,
                FailurePhase::Execution,
            );
        }
        return emit_failed_or_silent(
            stdout,
            bounds,
            &seed_id,
            FailureCode::ExecutionFailed,
            FailurePhase::Execution,
        );
    }

    // --- drain: host input has deterministic precedence over deadline and
    // completion. Thus an already-buffered cancel wins a simultaneous exit.
    let mut cancel_requested = false;
    let result = loop {
        tokio::select! {
            biased;
            line = rx.recv(), if !cancel_requested => match line {
                Some(InputLine::Line(b)) => match session.feed_host_line(&b) {
                    Ok(HostToBackend::Cancel(_)) => {
                        cancel_requested = true;
                        cancel.cancel();
                    }
                    Ok(_) | Err(_) => {
                        cancel.cancel();
                        if !drain_cancelled_run(&mut run, deadline).await {
                            return emit_failed_or_silent(
                                stdout, bounds, &seed_id,
                                FailureCode::CleanupUnconfirmed, FailurePhase::Execution,
                            );
                        }
                        return emit_failed_or_silent(
                            stdout, bounds, &seed_id,
                            FailureCode::ProtocolViolation, FailurePhase::Execution,
                        );
                    }
                },
                Some(InputLine::Eof) | Some(InputLine::Error) | None => {
                    cancel.cancel();
                    if !drain_cancelled_run(&mut run, deadline).await {
                        return emit_failed_or_silent(
                            stdout, bounds, &seed_id,
                            FailureCode::CleanupUnconfirmed, FailurePhase::Execution,
                        );
                    }
                    return emit_failed_or_silent(
                        stdout, bounds, &seed_id,
                        FailureCode::ProtocolViolation, FailurePhase::Execution,
                    );
                }
            },
            _ = tokio::time::sleep_until(deadline) => {
                cancel.cancel();
                drop(run);
                return emit_failed_or_silent(
                    stdout, bounds, &seed_id,
                    FailureCode::CleanupUnconfirmed, FailurePhase::Execution,
                );
            },
            ev = next_event(&mut run) => match ev {
                Some(SandboxEvent::Completed(mut result)) => {
                    if cancel_requested {
                        result.outcome = SandboxOutcome::Cancelled;
                    }
                    break result;
                }
                Some(_) => continue,
                None => {
                    return emit_failed_or_silent(
                        stdout, bounds, &seed_id,
                        FailureCode::ExecutionFailed, FailurePhase::Execution,
                    );
                }
            },
        }
    };

    // Stop protocol input immediately after reaching a terminal result. This
    // drops queued/future frames and unblocks the bounded reader channel.
    rx.close();

    // --- emit captured stdout/stderr as base64 chunks, then completed ---
    if !emit_output(stdout, bounds, &id, &result.stdout, true) {
        return EXIT_NO_TERMINAL;
    }
    if !emit_output(stdout, bounds, &id, &result.stderr, false) {
        return EXIT_NO_TERMINAL;
    }
    let completed = BackendToHost::Completed(completed_payload(&id, &result));
    if !emit_frame(stdout, bounds, &completed) {
        return EXIT_NO_TERMINAL;
    }

    // Terminal frame emitted + flushed: exit 0. The host closes stdin and reaps
    // expecting exit 0; the backend does NOT read another host frame.
    EXIT_OK
}

/// Production entry point: wire process stdio + the live platform posture and
/// drive one exchange. In 16.12 `platform::current` is unsupported on every
/// platform, so production negotiates then refuses at the gate
/// (`failed{Unavailable, Handshake}`); the injected-runner tests exercise
/// successful `started` -> `completed`.
pub async fn run() -> i32 {
    let posture = platform::current();
    // The runner is constructed even on the unsupported path (with the neutral
    // placeholder restriction); it is never invoked there because the helper
    // gate refuses before `runner.run`.
    let runner = SandboxRunner::new(
        SandboxPolicy::default(),
        posture
            .restriction
            .clone()
            .unwrap_or_else(|| Arc::new(NoRestriction)),
    );
    let stdin: Box<dyn Read + Send> = Box::new(std::io::stdin());
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    drive(
        stdin,
        &mut stdout,
        Bounds::DEFAULT,
        posture.supported,
        &posture.limitations,
        &runner,
    )
    .await
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
    rx: &mut tokio::sync::mpsc::Receiver<InputLine>,
    session: &mut Session,
    seed_id: &mut Option<RequestId>,
) -> HostIn {
    match rx.recv().await {
        Some(InputLine::Line(b)) => match session.feed_host_line(&b) {
            Ok(frame) => {
                if seed_id.is_none() {
                    *seed_id = Some(frame.request_id().clone());
                }
                HostIn::Frame(frame)
            }
            Err(_) => HostIn::Error,
        },
        Some(InputLine::Eof) => HostIn::Eof,
        Some(InputLine::Error) | None => HostIn::Error,
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
async fn next_event(run: &mut SandboxRun) -> Option<SandboxEvent> {
    std::future::poll_fn(|cx| Pin::new(&mut *run).poll_next(cx)).await
}

async fn next_completed(run: &mut SandboxRun) -> Option<crate::runner::SandboxResult> {
    loop {
        match next_event(run).await {
            Some(SandboxEvent::Completed(result)) => return Some(result),
            Some(_) => {}
            None => return None,
        }
    }
}

async fn drain_cancelled_run(run: &mut SandboxRun, deadline: tokio::time::Instant) -> bool {
    tokio::time::timeout_at(deadline, next_completed(run))
        .await
        .is_ok_and(|result| result.is_some())
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

/// The blocking stdin reader: cap lines at `bounds.max_line_size` and feed them
/// to the channel; send a terminal `Eof` / `Error` and return at end.
fn run_reader(
    mut stdin: Box<dyn Read + Send>,
    bounds: Bounds,
    tx: tokio::sync::mpsc::Sender<InputLine>,
) {
    let mut reader = LineReader::new(stdin.as_mut(), bounds);
    let mut buf = Vec::new();
    loop {
        match reader.read_line(&mut buf) {
            Ok(true) => {
                if tx
                    .blocking_send(InputLine::Line(std::mem::take(&mut buf)))
                    .is_err()
                {
                    return;
                }
            }
            Ok(false) => {
                let _ = tx.blocking_send(InputLine::Eof);
                return;
            }
            Err(_) => {
                let _ = tx.blocking_send(InputLine::Error);
                return;
            }
        }
    }
}
