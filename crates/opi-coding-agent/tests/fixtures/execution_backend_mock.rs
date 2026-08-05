//! `command-execution-jsonl-v1` backend mock for the Phase 16.7 protocol-host
//! integration tests (`tests/execution_protocol_host.rs`).
//!
//! This is a `harness = false` `[[test]]` binary (see `Cargo.toml`); the host
//! test locates it via the deps-dir scan and selects a behavior with the
//! `OPI_PROTOCOL_MOCK_MODE` env var. It speaks the closed v1 frame set over
//! stdio using the typed `opi_protocol` frames (no hand-rolled JSON for the
//! happy paths; raw bytes are written only where a mode must produce a
//! protocol violation).
//!
//! Flow every mode starts with: read `initialize`, capture its host-generated
//! request id, then behave per mode.

use std::io::{BufRead, Write};

use opi_protocol::execution::v1::frames::{
    AcceptedPayload, CompletedPayload, FailedPayload, ReadyPayload, StartedPayload, StdoutPayload,
    TargetId,
};
use opi_protocol::execution::v1::{
    BackendToHost, Base64Bytes, CleanupState, Diagnostic, FailureCode, FailurePhase, HostToBackend,
    ImplementationId, ProtocolId, RequestId, WIRE_IDENTITY,
};

struct ReadyIdentity {
    implementation: String,
    version: String,
    target: String,
}

static READY_IDENTITY: std::sync::OnceLock<ReadyIdentity> = std::sync::OnceLock::new();

fn main() {
    // Mode (and an optional mode-specific param) come from ARGS first so the
    // host test can run many modes concurrently without racing a process-global
    // env var. Env is a fallback for manual invocation.
    let mut args = std::env::args().skip(1);
    let mode = args
        .next()
        .or_else(|| std::env::var("OPI_PROTOCOL_MOCK_MODE").ok())
        .unwrap_or_else(|| "happy_path".to_string());
    let remaining = args.collect::<Vec<_>>();
    let extra = remaining.first().cloned();
    let ready_identity = if mode == "happy_path" && remaining.len() >= 3 {
        ReadyIdentity {
            implementation: remaining[0].clone(),
            version: remaining[1].clone(),
            target: remaining[2].clone(),
        }
    } else {
        ReadyIdentity {
            implementation: "opi-sandbox".into(),
            version: "mock-1.0.0".into(),
            target: "mock-target".into(),
        }
    };
    READY_IDENTITY
        .set(ready_identity)
        .unwrap_or_else(|_| unreachable!("ready identity initialized once"));
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = std::io::BufReader::new(stdin.lock());
    let mut writer = stdout.lock();

    match mode.as_str() {
        "happy_path" => happy(&mut reader, &mut writer, b"hello\n"),
        "happy_binary" => happy(&mut reader, &mut writer, &[0xFF, 0x00, 0x42, b'\n']),
        "nonzero_exit" => terminal_exit(&mut reader, &mut writer, Some(2), None),
        "signal_in_band" => terminal_exit(&mut reader, &mut writer, None, Some(9)),
        "malformed_frame" => raw_after_started(&mut reader, &mut writer, b"{not valid json\n"),
        "oversized_frame" => oversized(&mut reader, &mut writer),
        "duplicate_accepted" => duplicate_accepted(&mut reader, &mut writer),
        "cross_request_id" => cross_request_id(&mut reader, &mut writer),
        "unknown_field" => unknown_field(&mut reader, &mut writer),
        "out_of_order" => out_of_order(&mut reader, &mut writer),
        "stdout_contamination" => raw_after_started(&mut reader, &mut writer, b"not json at all\n"),
        "premature_eof" => premature_eof(&mut reader, &mut writer),
        "crash_before_ready" => crash_before_ready(&mut reader),
        "crash_after_ready" => crash_after_ready(&mut reader, &mut writer),
        "protocol_incompatible" => protocol_incompatible(&mut reader, &mut writer),
        "ready_identity_mismatch" => ready_mismatch(&mut reader, &mut writer, "identity"),
        "ready_version_mismatch" => ready_mismatch(&mut reader, &mut writer, "version"),
        "ready_target_mismatch" => ready_mismatch(&mut reader, &mut writer, "target"),
        "slow_ready" => slow_ready(&mut reader, &mut writer),
        "terminal_extra_frame" => terminal_contamination(&mut reader, &mut writer, true),
        "terminal_extra_bytes" => terminal_contamination(&mut reader, &mut writer, false),
        "failed_terminal_extra_bytes" => failed_terminal_contamination(&mut reader, &mut writer),
        "terminal_diagnostic" => terminal_diagnostic(&mut reader, &mut writer),
        "hang_before_ready" => hang(&mut reader, &mut writer, HangPoint::BeforeReady),
        "hang_after_started" => hang(&mut reader, &mut writer, HangPoint::AfterStarted),
        // In-band Completed with timed_out=true and a clean exit code: the host
        // must NOT treat a timed-out frame as a success (no degraded success).
        "completed_timed_out" => completed_timed_out(&mut reader, &mut writer),
        // In-band Completed with cancelled=true and a clean exit code: the host
        // must NOT treat a cancelled frame as a success (no degraded success).
        "completed_cancelled" => completed_cancelled(&mut reader, &mut writer),
        "cancel_cleanup_unconfirmed" => cancel_cleanup_unconfirmed(&mut reader, &mut writer),
        "failed_pre_started" => failed(
            &mut reader,
            &mut writer,
            parse_failure_code(extra.as_deref().unwrap_or("failed")),
            FailPoint::PreStarted,
        ),
        "failed_post_started" => failed(
            &mut reader,
            &mut writer,
            parse_failure_code(extra.as_deref().unwrap_or("execution_failed")),
            FailPoint::PostStarted,
        ),
        "redact_canary" => redact_canary(
            &mut reader,
            extra.clone().unwrap_or_else(|| {
                std::env::var("OPI_REDACT_CANARY")
                    .unwrap_or_else(|_| "OPI_REDACT_CANARY_DEFAULT".into())
            }),
        ),
        "l0_grandchild" => l0_grandchild(
            &mut reader,
            &mut writer,
            extra
                .clone()
                .unwrap_or_else(|| std::env::var("OPI_L0_GC_PIDFILE").unwrap_or_default()),
        ),
        _ => std::process::exit(1),
    }
}

// --- helpers ---

fn read_host_frame(reader: &mut impl BufRead) -> Option<HostToBackend> {
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => return None,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                return serde_json::from_str(trimmed).ok();
            }
            Err(_) => return None,
        }
    }
}

/// Read `initialize` and return its request id (every mode starts here).
fn expect_initialize(reader: &mut impl BufRead) -> Option<RequestId> {
    match read_host_frame(reader)? {
        HostToBackend::Initialize(p) => Some(p.request_id),
        _ => None,
    }
}

fn send(writer: &mut impl Write, frame: &BackendToHost) {
    let json = serde_json::to_string(frame).unwrap();
    writer.write_all(json.as_bytes()).unwrap();
    writer.write_all(b"\n").unwrap();
    writer.flush().unwrap();
}

fn ready_frame(rid: &RequestId, protocol: &str) -> BackendToHost {
    let identity = READY_IDENTITY.get().expect("ready identity initialized");
    BackendToHost::Ready(ReadyPayload {
        request_id: rid.clone(),
        selected_protocol: ProtocolId::new(protocol),
        implementation: ImplementationId::new(identity.implementation.clone()).unwrap(),
        implementation_version: identity.version.clone(),
        target: TargetId::new(identity.target.clone()),
    })
}

fn custom_ready_frame(
    rid: &RequestId,
    implementation: &str,
    version: &str,
    target: &str,
) -> BackendToHost {
    BackendToHost::Ready(ReadyPayload {
        request_id: rid.clone(),
        selected_protocol: ProtocolId::new(WIRE_IDENTITY),
        implementation: ImplementationId::new(implementation).unwrap(),
        implementation_version: version.to_string(),
        target: TargetId::new(target),
    })
}

fn drain_until_eof(reader: &mut impl BufRead) {
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
    }
}

enum HangPoint {
    BeforeReady,
    AfterStarted,
}

enum FailPoint {
    PreStarted,
    PostStarted,
}

fn parse_failure_code(s: &str) -> FailureCode {
    use FailureCode::*;
    match s {
        "unavailable" => Unavailable,
        "failed" => Failed,
        "protocol_incompatible" => ProtocolIncompatible,
        "protocol_violation" => ProtocolViolation,
        "execution_failed" => ExecutionFailed,
        "execution_timed_out" => ExecutionTimedOut,
        "cleanup_unconfirmed" => CleanupUnconfirmed,
        _ => Failed,
    }
}

// --- modes ---

fn happy(reader: &mut impl BufRead, writer: &mut impl Write, out: &[u8]) {
    let rid = match expect_initialize(reader) {
        Some(r) => r,
        None => return,
    };
    send(writer, &ready_frame(&rid, WIRE_IDENTITY));
    // Consume execute.
    let _ = read_host_frame(reader);
    send(
        writer,
        &BackendToHost::Accepted(AcceptedPayload {
            request_id: rid.clone(),
        }),
    );
    send(
        writer,
        &BackendToHost::Started(StartedPayload {
            request_id: rid.clone(),
            placement: "host".into(),
            guarantee: "supervised".into(),
            policy: "none".into(),
            limitations: vec![],
        }),
    );
    send(
        writer,
        &BackendToHost::Stdout(StdoutPayload {
            request_id: rid.clone(),
            data: Base64Bytes::from_bytes(out),
        }),
    );
    send(
        writer,
        &BackendToHost::Completed(CompletedPayload {
            request_id: rid,
            exit: Some(0),
            signal: None,
            timed_out: false,
            cancelled: false,
            cleanup: CleanupState::Confirmed,
            diagnostics: vec![],
        }),
    );
    drain_until_eof(reader);
    std::process::exit(0);
}

/// Reports `Completed{timed_out: true, exit: Some(0)}` — a degraded-success
/// canary. The host must surface this as an error even though the exit code is
/// clean.
fn completed_timed_out(reader: &mut impl BufRead, writer: &mut impl Write) {
    completed_terminal(reader, writer, true, false);
}

/// Reports `Completed{cancelled: true, exit: Some(0)}` — the cancelled leg of
/// the no-degraded-success invariant. The host must surface this as an error
/// even though the exit code is clean.
fn completed_cancelled(reader: &mut impl BufRead, writer: &mut impl Write) {
    completed_terminal(reader, writer, false, true);
}

/// Shared in-band `Completed` reporter for a terminal state that carries a
/// clean exit code: `timed_out` and/or `cancelled` set.
fn completed_terminal(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    timed_out: bool,
    cancelled: bool,
) {
    let rid = match expect_initialize(reader) {
        Some(r) => r,
        None => return,
    };
    send(writer, &ready_frame(&rid, WIRE_IDENTITY));
    let _ = read_host_frame(reader);
    send(
        writer,
        &BackendToHost::Accepted(AcceptedPayload {
            request_id: rid.clone(),
        }),
    );
    send(
        writer,
        &BackendToHost::Started(StartedPayload {
            request_id: rid.clone(),
            placement: "host".into(),
            guarantee: "supervised".into(),
            policy: "none".into(),
            limitations: vec![],
        }),
    );
    send(
        writer,
        &BackendToHost::Completed(CompletedPayload {
            request_id: rid,
            exit: Some(0),
            signal: None,
            timed_out,
            cancelled,
            cleanup: CleanupState::Confirmed,
            diagnostics: vec![],
        }),
    );
    drain_until_eof(reader);
    std::process::exit(0);
}

fn terminal_exit(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    exit: Option<u32>,
    signal: Option<u32>,
) {
    let rid = match expect_initialize(reader) {
        Some(r) => r,
        None => return,
    };
    send(writer, &ready_frame(&rid, WIRE_IDENTITY));
    let _ = read_host_frame(reader);
    send(
        writer,
        &BackendToHost::Accepted(AcceptedPayload {
            request_id: rid.clone(),
        }),
    );
    send(
        writer,
        &BackendToHost::Started(StartedPayload {
            request_id: rid.clone(),
            placement: "host".into(),
            guarantee: "supervised".into(),
            policy: "none".into(),
            limitations: vec![],
        }),
    );
    send(
        writer,
        &BackendToHost::Completed(CompletedPayload {
            request_id: rid,
            exit,
            signal,
            timed_out: false,
            cancelled: false,
            cleanup: CleanupState::Confirmed,
            diagnostics: vec![],
        }),
    );
    drain_until_eof(reader);
    std::process::exit(0);
}

fn raw_after_started(reader: &mut impl BufRead, writer: &mut impl Write, bytes: &[u8]) {
    let rid = match expect_initialize(reader) {
        Some(r) => r,
        None => return,
    };
    send(writer, &ready_frame(&rid, WIRE_IDENTITY));
    let _ = read_host_frame(reader);
    send(
        writer,
        &BackendToHost::Accepted(AcceptedPayload {
            request_id: rid.clone(),
        }),
    );
    send(
        writer,
        &BackendToHost::Started(StartedPayload {
            request_id: rid,
            placement: "host".into(),
            guarantee: "supervised".into(),
            policy: "none".into(),
            limitations: vec![],
        }),
    );
    // Raw non-frame bytes on the protocol stdout -> host protocol_violation.
    writer.write_all(bytes).unwrap();
    writer.flush().unwrap();
    drain_until_eof(reader);
    std::process::exit(0);
}

fn oversized(reader: &mut impl BufRead, writer: &mut impl Write) {
    let rid = match expect_initialize(reader) {
        Some(r) => r,
        None => return,
    };
    send(writer, &ready_frame(&rid, WIRE_IDENTITY));
    let _ = read_host_frame(reader);
    send(
        writer,
        &BackendToHost::Accepted(AcceptedPayload {
            request_id: rid.clone(),
        }),
    );
    send(
        writer,
        &BackendToHost::Started(StartedPayload {
            request_id: rid.clone(),
            placement: "host".into(),
            guarantee: "supervised".into(),
            policy: "none".into(),
            limitations: vec![],
        }),
    );
    // A stdout chunk large enough to exceed any small test bound.
    let big = vec![b'A'; 8192];
    send(
        writer,
        &BackendToHost::Stdout(StdoutPayload {
            request_id: rid,
            data: Base64Bytes::from_bytes(big),
        }),
    );
    drain_until_eof(reader);
    std::process::exit(0);
}

fn duplicate_accepted(reader: &mut impl BufRead, writer: &mut impl Write) {
    let rid = match expect_initialize(reader) {
        Some(r) => r,
        None => return,
    };
    send(writer, &ready_frame(&rid, WIRE_IDENTITY));
    let _ = read_host_frame(reader);
    send(
        writer,
        &BackendToHost::Accepted(AcceptedPayload {
            request_id: rid.clone(),
        }),
    );
    // Second accepted in the same execution -> duplicate once-per-execution.
    send(
        writer,
        &BackendToHost::Accepted(AcceptedPayload { request_id: rid }),
    );
    drain_until_eof(reader);
    std::process::exit(0);
}

fn cross_request_id(reader: &mut impl BufRead, writer: &mut impl Write) {
    let rid = match expect_initialize(reader) {
        Some(r) => r,
        None => return,
    };
    send(writer, &ready_frame(&rid, WIRE_IDENTITY));
    let _ = read_host_frame(reader);
    send(
        writer,
        &BackendToHost::Accepted(AcceptedPayload { request_id: rid }),
    );
    // Started carries a WRONG request id -> cross-request id.
    send(
        writer,
        &BackendToHost::Started(StartedPayload {
            request_id: RequestId::new("evil-cross-id".to_string()).unwrap(),
            placement: "host".into(),
            guarantee: "supervised".into(),
            policy: "none".into(),
            limitations: vec![],
        }),
    );
    drain_until_eof(reader);
    std::process::exit(0);
}

fn unknown_field(reader: &mut impl BufRead, writer: &mut impl Write) {
    let rid = match expect_initialize(reader) {
        Some(r) => r,
        None => return,
    };
    send(writer, &ready_frame(&rid, WIRE_IDENTITY));
    let _ = read_host_frame(reader);
    // Raw accepted frame JSON with an unknown field -> deny_unknown_fields.
    let raw = format!(
        "{{\"type\":\"accepted\",\"payload\":{{\"request_id\":\"{}\",\"extra\":1}}}}\n",
        rid.as_str()
    );
    writer.write_all(raw.as_bytes()).unwrap();
    writer.flush().unwrap();
    drain_until_eof(reader);
    std::process::exit(0);
}

fn out_of_order(reader: &mut impl BufRead, writer: &mut impl Write) {
    let rid = match expect_initialize(reader) {
        Some(r) => r,
        None => return,
    };
    send(writer, &ready_frame(&rid, WIRE_IDENTITY));
    let _ = read_host_frame(reader);
    // Completed arrives before accepted/started -> out of order.
    send(
        writer,
        &BackendToHost::Completed(CompletedPayload {
            request_id: rid,
            exit: Some(0),
            signal: None,
            timed_out: false,
            cancelled: false,
            cleanup: CleanupState::Confirmed,
            diagnostics: vec![],
        }),
    );
    drain_until_eof(reader);
    std::process::exit(0);
}

fn failed(reader: &mut impl BufRead, writer: &mut impl Write, code: FailureCode, point: FailPoint) {
    let rid = match expect_initialize(reader) {
        Some(r) => r,
        None => return,
    };
    let phase = if matches!(point, FailPoint::PreStarted) {
        FailurePhase::Handshake
    } else {
        send(writer, &ready_frame(&rid, WIRE_IDENTITY));
        let _ = read_host_frame(reader);
        send(
            writer,
            &BackendToHost::Accepted(AcceptedPayload {
                request_id: rid.clone(),
            }),
        );
        send(
            writer,
            &BackendToHost::Started(StartedPayload {
                request_id: rid.clone(),
                placement: "host".into(),
                guarantee: "supervised".into(),
                policy: "none".into(),
                limitations: vec![],
            }),
        );
        FailurePhase::Execution
    };
    // Typed distress frame. Spec: before `started` the backend may terminate
    // with `unavailable`/`failed`; after started it may report execution
    // failure/timeout/cleanup/protocol distress.
    send(
        writer,
        &BackendToHost::Failed(FailedPayload {
            request_id: rid,
            code,
            phase,
            message: Some("backend failure at C:\\private\\tool with sk-proj-secret".into()),
            diagnostics: vec![Diagnostic {
                message: "detail C:\\private\\detail sk-proj-secret".into(),
            }],
        }),
    );
    drain_until_eof(reader);
    std::process::exit(0);
}

fn premature_eof(reader: &mut impl BufRead, _writer: &mut impl Write) {
    let rid = match expect_initialize(reader) {
        Some(r) => r,
        None => return,
    };
    send(_writer, &ready_frame(&rid, WIRE_IDENTITY));
    let _ = read_host_frame(reader);
    // Close stdout with no terminal frame -> premature EOF.
    std::process::exit(0);
}

fn crash_before_ready(reader: &mut impl BufRead) {
    let _ = expect_initialize(reader);
    std::process::exit(1);
}

fn crash_after_ready(reader: &mut impl BufRead, writer: &mut impl Write) {
    let rid = match expect_initialize(reader) {
        Some(r) => r,
        None => return,
    };
    send(writer, &ready_frame(&rid, WIRE_IDENTITY));
    let _ = read_host_frame(reader);
    std::process::exit(1);
}

fn protocol_incompatible(reader: &mut impl BufRead, writer: &mut impl Write) {
    let rid = match expect_initialize(reader) {
        Some(r) => r,
        None => return,
    };
    // ready selects a protocol NOT in the host's list.
    send(writer, &ready_frame(&rid, "command-execution-jsonl-v9"));
    drain_until_eof(reader);
    std::process::exit(0);
}

fn ready_mismatch(reader: &mut impl BufRead, writer: &mut impl Write, field: &str) {
    let Some(rid) = expect_initialize(reader) else {
        return;
    };
    let (implementation, version, target) = match field {
        "identity" => ("different-adapter", "mock-1.0.0", "mock-target"),
        "version" => ("opi-sandbox", "different-version", "mock-target"),
        "target" => ("opi-sandbox", "mock-1.0.0", "different-target"),
        _ => unreachable!(),
    };
    send(
        writer,
        &custom_ready_frame(&rid, implementation, version, target),
    );
    drain_until_eof(reader);
}

fn slow_ready(reader: &mut impl BufRead, writer: &mut impl Write) {
    let Some(rid) = expect_initialize(reader) else {
        return;
    };
    std::thread::sleep(std::time::Duration::from_millis(250));
    send(writer, &ready_frame(&rid, WIRE_IDENTITY));
    drain_until_eof(reader);
}

fn terminal_contamination(reader: &mut impl BufRead, writer: &mut impl Write, frame: bool) {
    let Some(rid) = expect_initialize(reader) else {
        return;
    };
    send(writer, &ready_frame(&rid, WIRE_IDENTITY));
    let _ = read_host_frame(reader);
    send(
        writer,
        &BackendToHost::Accepted(AcceptedPayload {
            request_id: rid.clone(),
        }),
    );
    send(
        writer,
        &BackendToHost::Started(StartedPayload {
            request_id: rid.clone(),
            placement: "host".into(),
            guarantee: "supervised".into(),
            policy: "none".into(),
            limitations: vec![],
        }),
    );
    send(
        writer,
        &BackendToHost::Completed(CompletedPayload {
            request_id: rid.clone(),
            exit: Some(0),
            signal: None,
            timed_out: false,
            cancelled: false,
            cleanup: CleanupState::Confirmed,
            diagnostics: vec![],
        }),
    );
    if frame {
        send(
            writer,
            &BackendToHost::Diagnostic(opi_protocol::execution::v1::frames::DiagnosticPayload {
                request_id: rid,
                message: "after terminal".into(),
            }),
        );
    } else {
        writer.write_all(b"x").unwrap();
        writer.flush().unwrap();
    }
    drain_until_eof(reader);
}

fn failed_terminal_contamination(reader: &mut impl BufRead, writer: &mut impl Write) {
    let Some(rid) = expect_initialize(reader) else {
        return;
    };
    send(
        writer,
        &BackendToHost::Failed(FailedPayload {
            request_id: rid,
            code: FailureCode::Unavailable,
            phase: FailurePhase::Handshake,
            message: Some("redacted failure detail".into()),
            diagnostics: vec![],
        }),
    );
    writer.write_all(b"x").unwrap();
    writer.flush().unwrap();
    drain_until_eof(reader);
}

fn terminal_diagnostic(reader: &mut impl BufRead, writer: &mut impl Write) {
    let Some(rid) = expect_initialize(reader) else {
        return;
    };
    send(writer, &ready_frame(&rid, WIRE_IDENTITY));
    let _ = read_host_frame(reader);
    send(
        writer,
        &BackendToHost::Accepted(AcceptedPayload {
            request_id: rid.clone(),
        }),
    );
    send(
        writer,
        &BackendToHost::Started(StartedPayload {
            request_id: rid.clone(),
            placement: "host".into(),
            guarantee: "supervised".into(),
            policy: "none".into(),
            limitations: vec![],
        }),
    );
    send(
        writer,
        &BackendToHost::Diagnostic(opi_protocol::execution::v1::frames::DiagnosticPayload {
            request_id: rid.clone(),
            message:
                "stream secret sk-proj-abcdefghijklmnopqrstuvwxyz123456 at C:\\private\\stream"
                    .into(),
        }),
    );
    send(
        writer,
        &BackendToHost::Completed(CompletedPayload {
            request_id: rid,
            exit: Some(0),
            signal: None,
            timed_out: false,
            cancelled: false,
            cleanup: CleanupState::Confirmed,
            diagnostics: vec![Diagnostic {
                message: "backend secret sk-proj-abcdefghijklmnopqrstuvwxyz123456 at C:\\private\\adapter".into(),
            }],
        }),
    );
    drain_until_eof(reader);
}

fn hang(reader: &mut impl BufRead, writer: &mut impl Write, point: HangPoint) {
    let rid = match expect_initialize(reader) {
        Some(r) => r,
        None => return,
    };
    if matches!(point, HangPoint::BeforeReady) {
        // Never send ready; the host's startup/handshake deadline fires.
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    send(writer, &ready_frame(&rid, WIRE_IDENTITY));
    let _ = read_host_frame(reader);
    send(
        writer,
        &BackendToHost::Accepted(AcceptedPayload {
            request_id: rid.clone(),
        }),
    );
    send(
        writer,
        &BackendToHost::Started(StartedPayload {
            request_id: rid,
            placement: "host".into(),
            guarantee: "supervised".into(),
            policy: "none".into(),
            limitations: vec![],
        }),
    );
    // Never complete; the host's execution deadline fires.
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn cancel_cleanup_unconfirmed(reader: &mut impl BufRead, writer: &mut impl Write) {
    let rid = match expect_initialize(reader) {
        Some(r) => r,
        None => return,
    };
    send(writer, &ready_frame(&rid, WIRE_IDENTITY));
    let _ = read_host_frame(reader);
    send(
        writer,
        &BackendToHost::Accepted(AcceptedPayload {
            request_id: rid.clone(),
        }),
    );
    send(
        writer,
        &BackendToHost::Started(StartedPayload {
            request_id: rid.clone(),
            placement: "host".into(),
            guarantee: "supervised".into(),
            policy: "none".into(),
            limitations: vec![],
        }),
    );
    // On receiving cancel, report unconfirmed cleanup -> CleanupUnconfirmed.
    while let Some(frame) = read_host_frame(reader) {
        if matches!(frame, HostToBackend::Cancel(_)) {
            send(
                writer,
                &BackendToHost::Completed(CompletedPayload {
                    request_id: rid,
                    exit: None,
                    signal: None,
                    timed_out: false,
                    cancelled: true,
                    cleanup: CleanupState::Unconfirmed,
                    diagnostics: vec![],
                }),
            );
            std::process::exit(0);
        }
    }
}

fn redact_canary(reader: &mut impl BufRead, canary: String) {
    let _ = expect_initialize(reader);
    // Write a canary to the backend PROCESS stderr (crash-evidence pipe). The
    // host must NEVER surface it; ExecutionFailure is payload-free.
    let mut stderr = std::io::stderr();
    let _ = writeln!(stderr, "{canary}");
    let _ = stderr.flush();
    std::process::exit(1);
}

/// Handshake to started, spawn a marker grandchild that records its pid, then
/// hang. The host's cancel/timeout/drop kills the whole tree (process group on
/// Unix, Job Object on Windows), which the test observes via the pidfile.
fn l0_grandchild(reader: &mut impl BufRead, writer: &mut impl Write, pidfile: String) {
    let rid = match expect_initialize(reader) {
        Some(r) => r,
        None => return,
    };
    send(writer, &ready_frame(&rid, WIRE_IDENTITY));
    let _ = read_host_frame(reader);
    send(
        writer,
        &BackendToHost::Accepted(AcceptedPayload {
            request_id: rid.clone(),
        }),
    );
    send(
        writer,
        &BackendToHost::Started(StartedPayload {
            request_id: rid,
            placement: "host".into(),
            guarantee: "supervised".into(),
            policy: "none".into(),
            limitations: vec![],
        }),
    );
    if !pidfile.is_empty() {
        #[cfg(windows)]
        {
            let script = format!(
                "$PID | Out-File -FilePath '{}' -NoNewline -Encoding ASCII\nStart-Sleep -Seconds 60\n",
                pidfile
            );
            let _ = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", &script])
                .spawn();
        }
        #[cfg(unix)]
        {
            let _ = std::process::Command::new("sh")
                .args(["-c", &format!("echo $$ > '{}'; sleep 60", pidfile)])
                .spawn();
        }
    }
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
