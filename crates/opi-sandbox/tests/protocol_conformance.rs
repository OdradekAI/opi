//! Portable protocol conformance for the `opi-sandbox` backend --stdio state
//! machine (Phase 16 task 16.12).
//!
//! These tests drive [`opi_sandbox::backend::drive`] with an INJECTED
//! [`SandboxRunner`] (`NoRestriction`) + `supported = true`, feeding host frames
//! as JSONL over an injected stdin (`std::io::Cursor`) and capturing the backend's
//! stdout frames. They prove the full success ordering AND every bounded terminal
//! invalid path required by the DoD. The REAL-binary negotiation + unsupported
//! pre-start path is exercised by `tests/backend_protocol_smoke.rs` via the
//! Python fixture client.
//!
//! Host input is written as literal JSONL (the protocol's own `decode_backend`
//! parses the backend output, so no serde_json dev-dep is needed). Tempdir paths
//! are forward-slashed so they need no JSON escaping on Windows.

#![forbid(unsafe_code)]

use std::io::Cursor;
use std::sync::Arc;

use opi_protocol::execution::v1::codec::decode_backend;
use opi_protocol::execution::v1::{BackendToHost, Bounds, CleanupState, FailureCode, FailurePhase};

use opi_sandbox::{NoRestriction, SandboxPolicy, SandboxRunner, backend};

/// Build a host `initialize` JSONL line.
fn init_json(rid: &str, deadline_ms: u64, protocols: &[&str]) -> String {
    let protos = protocols
        .iter()
        .map(|p| format!("\"{p}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"type":"initialize","payload":{{"request_id":"{rid}","deadline_ms":{deadline_ms},"adapter_config":{{}},"supported_protocols":[{protos}]}}}}"#
    )
}

/// Build a host `execute` JSONL line. `workspace` must be forward-slashed.
fn exec_json(
    rid: &str,
    program: &str,
    args: &[&str],
    workspace: &str,
    timeout_ms: u64,
    env_kvs: &[(&str, &str)],
) -> String {
    let args_j = args
        .iter()
        .map(|a| format!("\"{a}\""))
        .collect::<Vec<_>>()
        .join(",");
    let env_j = env_kvs
        .iter()
        .map(|(k, v)| format!("\"{k}\":\"{v}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"type":"execute","payload":{{"request_id":"{rid}","program":"{program}","args":[{args_j}],"workspace":"{workspace}","cwd":"{workspace}","timeout_ms":{timeout_ms},"env_inherit":"inherit","env_additions":{{{env_j}}}}}}}"#
    )
}

/// Build a host `cancel` JSONL line.
fn cancel_json(rid: &str, reason: &str) -> String {
    format!(r#"{{"type":"cancel","payload":{{"request_id":"{rid}","reason":"{reason}"}}}}"#)
}

/// A platform echo target: outputs "hi" and exits 0.
fn echo_target() -> (&'static str, Vec<&'static str>) {
    if cfg!(windows) {
        ("cmd", vec!["/C", "echo hi"])
    } else {
        ("sh", vec!["-c", "echo hi"])
    }
}

/// A platform long-sleep target: does not finish within the test window.
fn sleep_target() -> (&'static str, Vec<&'static str>) {
    if cfg!(windows) {
        ("cmd", vec!["/C", "ping -n 30 -w 1000 127.0.0.1"])
    } else {
        ("sh", vec!["-c", "sleep 30"])
    }
}

/// A forward-slashed tempdir workspace path (no JSON escaping needed).
fn workspace() -> String {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path().to_string_lossy().replace('\\', "/").to_string();
    // Leak keep the dir alive for the run (the target writes nothing here).
    std::mem::forget(tmp);
    ws
}

/// Drive the backend with an injected NoRestriction runner and return (exit, stdout).
async fn run_drive(stdin: String, supported: bool) -> (i32, Vec<u8>) {
    let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(NoRestriction));
    let mut out = Vec::new();
    let code = backend::drive(
        Box::new(Cursor::new(stdin.into_bytes())),
        &mut out,
        Bounds::DEFAULT,
        supported,
        &[],
        &runner,
    )
    .await;
    (code, out)
}

/// Decode the captured stdout into ordered backend frames.
fn parse(out: &[u8]) -> Vec<BackendToHost> {
    let mut frames = Vec::new();
    for line in out.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        frames.push(decode_backend(line).expect("captured line is a valid backend frame"));
    }
    frames
}

fn kinds(frames: &[BackendToHost]) -> Vec<&'static str> {
    frames.iter().map(|f| f.kind()).collect()
}

/// Parse the captured stdout and return the (owned) failed payload.
fn failed_frame(out: &[u8]) -> opi_protocol::execution::v1::frames::FailedPayload {
    parse(out)
        .into_iter()
        .find_map(|f| match f {
            BackendToHost::Failed(p) => Some(p),
            _ => None,
        })
        .expect("failed frame")
}

/// Parse the captured stdout and return the (owned) completed payload.
fn completed_frame(out: &[u8]) -> opi_protocol::execution::v1::frames::CompletedPayload {
    parse(out)
        .into_iter()
        .find_map(|f| match f {
            BackendToHost::Completed(p) => Some(p),
            _ => None,
        })
        .expect("completed frame")
}

/// SUCCESS: the full initialize->ready->execute->accepted->started->stdout->
/// completed ordering, with honest started vocabulary, echoed request id,
/// delivered target output, and an in-band exit 0.
#[tokio::test]
async fn success_full_state_machine() {
    let ws = workspace();
    let (program, args) = echo_target();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &args, &ws, 10_000, &[])
    );
    let (code, out) = run_drive(stdin, true).await;
    assert_eq!(code, 0, "clean exchange exits 0");
    let frames = parse(&out);

    // Ordering: ready, accepted, started, (stdout), completed.
    let order = kinds(&frames);
    let i_ready = order
        .iter()
        .position(|k| *k == "ready")
        .expect("ready frame");
    let i_accepted = order
        .iter()
        .position(|k| *k == "accepted")
        .expect("accepted");
    let i_started = order.iter().position(|k| *k == "started").expect("started");
    let i_completed = order
        .iter()
        .position(|k| *k == "completed")
        .expect("completed");
    assert!(i_ready < i_accepted);
    assert!(i_accepted < i_started);
    assert!(i_started < i_completed);

    // ready negotiation + non-empty identity/target.
    let BackendToHost::Ready(ready) = &frames[i_ready] else {
        unreachable!()
    };
    assert_eq!(
        ready.selected_protocol.as_str(),
        "command-execution-jsonl-v1"
    );
    assert!(!ready.implementation_version.is_empty());
    assert!(!ready.target.as_str().is_empty());

    // started: honest 16.12 vocabulary (L0 only; never restricted/isolated).
    let BackendToHost::Started(started) = &frames[i_started] else {
        unreachable!()
    };
    assert_eq!(started.placement, "host");
    assert_eq!(started.guarantee, "supervised");
    assert_eq!(started.policy, "unrestricted");

    // Exactly one terminal frame (completed), no second terminal frame.
    let terminals = order
        .iter()
        .filter(|k| **k == "completed" || **k == "failed")
        .count();
    assert_eq!(terminals, 1, "exactly one terminal frame");

    // completed: in-band exit 0, cleanup confirmed.
    let BackendToHost::Completed(completed) = &frames[i_completed] else {
        unreachable!()
    };
    assert_eq!(completed.exit, Some(0));
    assert!(!completed.timed_out);
    assert!(!completed.cancelled);
    assert_eq!(completed.cleanup, CleanupState::Confirmed);

    // Target output was delivered as a base64 stdout frame between started and
    // completed.
    let stdout_data: Vec<u8> = frames[i_started..i_completed]
        .iter()
        .filter_map(|f| match f {
            BackendToHost::Stdout(p) => Some(p.data.as_bytes().to_vec()),
            _ => None,
        })
        .flatten()
        .collect();
    let text = String::from_utf8_lossy(&stdout_data);
    assert!(text.contains("hi"), "target output delivered: {text:?}");

    // Every frame echoes the host request id.
    for f in &frames {
        assert_eq!(f.request_id().as_str(), "r1", "request id echoed");
    }
}

/// Cancel during drain resolves to completed{cancelled:true}.
#[tokio::test]
async fn cancel_during_drain_completes_cancelled() {
    let ws = workspace();
    let (program, args) = sleep_target();
    let stdin = format!(
        "{}\n{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &args, &ws, 30_000, &[]),
        cancel_json("r1", "canceled"),
    );
    let (code, out) = run_drive(stdin, true).await;
    assert_eq!(code, 0);
    let completed = completed_frame(&out);
    assert!(completed.cancelled, "cancelled flag set");
    assert!(!completed.timed_out);
}

/// SDK timeout (execute.timeout_ms) resolves to completed{timed_out:true}.
#[tokio::test]
async fn timeout_completes_timed_out() {
    let ws = workspace();
    let (program, args) = sleep_target();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &args, &ws, 100, &[]),
    );
    let (code, out) = run_drive(stdin, true).await;
    assert_eq!(code, 0);
    let completed = completed_frame(&out);
    assert!(completed.timed_out, "timed_out flag set");
}

/// Unsupported platform refuses at the gate: failed{Unavailable, Handshake}.
#[tokio::test]
async fn unsupported_platform_refuses_unavailable_handshake() {
    let ws = workspace();
    let (program, args) = echo_target();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &args, &ws, 10_000, &[]),
    );
    // supported = false: the real-binary 16.12 path.
    let (code, out) = run_drive(stdin, false).await;
    assert_eq!(code, 0, "clean distress exits 0");
    let frames = parse(&out);
    // ready (negotiation) precedes the pre-start failed.
    assert!(frames.iter().any(|f| f.kind() == "ready"));
    let failed = frames
        .iter()
        .find_map(|f| match f {
            BackendToHost::Failed(p) => Some(p),
            _ => None,
        })
        .expect("failed frame");
    assert_eq!(failed.code, FailureCode::Unavailable);
    assert_eq!(failed.phase, FailurePhase::Handshake);
}

/// A missing target program is a pre-start failure: failed{Failed, Handshake}.
#[tokio::test]
async fn program_not_found_failed_handshake() {
    let ws = workspace();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        exec_json("r1", "no-such-program-xyz", &[], &ws, 10_000, &[]),
    );
    let (code, out) = run_drive(stdin, true).await;
    assert_eq!(code, 0);
    let failed = failed_frame(&out);
    assert_eq!(failed.code, FailureCode::Failed);
    assert_eq!(failed.phase, FailurePhase::Handshake);
}

/// A zero-timeout execute is semantically invalid: failed{ProtocolViolation,
/// Handshake}.
#[tokio::test]
async fn zero_timeout_is_protocol_violation_handshake() {
    let ws = workspace();
    let (program, args) = echo_target();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &args, &ws, 0, &[]),
    );
    let (code, out) = run_drive(stdin, true).await;
    assert_eq!(code, 0);
    let failed = failed_frame(&out);
    assert_eq!(failed.code, FailureCode::ProtocolViolation);
    assert_eq!(failed.phase, FailurePhase::Handshake);
}

/// execute before initialize is out of order: failed{ProtocolViolation, Handshake}.
#[tokio::test]
async fn execute_before_initialize_is_protocol_violation() {
    let ws = workspace();
    let (program, args) = echo_target();
    let stdin = format!("{}\n", exec_json("r1", program, &args, &ws, 10_000, &[]),);
    let (code, out) = run_drive(stdin, true).await;
    assert_eq!(code, 0);
    let failed = failed_frame(&out);
    assert_eq!(failed.code, FailureCode::ProtocolViolation);
    assert_eq!(failed.phase, FailurePhase::Handshake);
}

/// A duplicate execute during drain is a post-start violation:
/// failed{ProtocolViolation, Execution}.
#[tokio::test]
async fn duplicate_execute_is_protocol_violation_execution() {
    let ws = workspace();
    let (program, args) = sleep_target();
    let stdin = format!(
        "{}\n{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &args, &ws, 30_000, &[]),
        exec_json("r1", program, &args, &ws, 30_000, &[]),
    );
    let (code, out) = run_drive(stdin, true).await;
    assert_eq!(code, 0);
    let failed = failed_frame(&out);
    assert_eq!(failed.code, FailureCode::ProtocolViolation);
    assert_eq!(failed.phase, FailurePhase::Execution);
}

/// A cross-request id on execute is rejected: failed{ProtocolViolation, Handshake}.
#[tokio::test]
async fn cross_request_id_is_protocol_violation() {
    let ws = workspace();
    let (program, args) = echo_target();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        exec_json("r2", program, &args, &ws, 10_000, &[]),
    );
    let (code, out) = run_drive(stdin, true).await;
    assert_eq!(code, 0);
    let failed = failed_frame(&out);
    assert_eq!(failed.code, FailureCode::ProtocolViolation);
    assert_eq!(failed.phase, FailurePhase::Handshake);
}

/// No protocol in common: failed{ProtocolIncompatible, Handshake}.
#[tokio::test]
async fn negotiation_no_overlap_is_protocol_incompatible() {
    let ws = workspace();
    let (program, args) = echo_target();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["some-other-protocol"]),
        exec_json("r1", program, &args, &ws, 10_000, &[]),
    );
    let (code, out) = run_drive(stdin, true).await;
    assert_eq!(code, 0);
    let failed = failed_frame(&out);
    assert_eq!(failed.code, FailureCode::ProtocolIncompatible);
    assert_eq!(failed.phase, FailurePhase::Handshake);
}

/// A malformed FIRST frame leaves no seed id: the backend writes nothing and
/// exits nonzero (the host classifies EOF/unexpected-exit).
#[tokio::test]
async fn malformed_first_frame_is_silent_nonzero() {
    let stdin = "this is not json\n".to_string();
    let (code, out) = run_drive(stdin, true).await;
    assert_eq!(code, 1, "no terminal frame could be emitted");
    assert!(out.is_empty(), "nothing written to stdout");
}

/// A first JSONL line exceeding Bounds.max_line_size is rejected by the capped
/// reader before parse (no seed id established) -> EXIT_NO_TERMINAL, no stdout.
/// Proves the oversized-line terminal path at the backend seam.
#[tokio::test]
async fn oversized_first_line_is_rejected_silent() {
    let oversized = "A".repeat(Bounds::DEFAULT.max_line_size + 1);
    let (code, out) = run_drive(oversized, true).await;
    assert_eq!(
        code, 1,
        "oversized line: no terminal frame could be emitted"
    );
    assert!(out.is_empty(), "nothing written to stdout");
}

/// Redaction: a failed frame carries no command/env/program payload.
#[tokio::test]
async fn failed_frame_is_redacted() {
    let ws = workspace();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        exec_json(
            "r1",
            "no-such-program-xyz",
            &[],
            &ws,
            10_000,
            &[("CANARY_ENV", "secret-value")],
        ),
    );
    let (_code, out) = run_drive(stdin, true).await;
    let raw = String::from_utf8_lossy(&out);
    assert!(
        !raw.contains("CANARY_ENV") && !raw.contains("secret-value"),
        "failed path must not leak env payload: {raw:?}"
    );
    let failed = failed_frame(&out);
    assert_eq!(failed.message, None, "message redacted");
    assert!(failed.diagnostics.is_empty(), "diagnostics redacted");
}
