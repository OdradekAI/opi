//! Drives `tests/fixtures/protocol_client.py` against the REAL `opi-sandbox`
//! binary: proves protocol negotiation (initialize -> ready) and the structured
//! execute contract on the production `backend --stdio` executable, then a clean
//! backend exit. The fixture is OS-aware: on supported Linux (16.13) it asserts
//! the confined successful run (started{supervised, restricted} -> completed);
//! off-Linux it asserts the Phase 16.12 pre-start refusal
//! (failed{unavailable, handshake}).
//!
//! This mirrors the repo precedent (`opi-coding-agent/tests/artifact_audit_script.rs`)
//! of a dedicated Rust test invoking a Python fixture directly, so the
//! Python-free isolation smoke (`scripts/opi-sandbox-smoke.{sh,ps1}`) stays pure.

#![forbid(unsafe_code)]

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn python_command() -> &'static str {
    if cfg!(windows) { "python" } else { "python3" }
}

#[test]
fn backend_negotiation_and_execute_contract() {
    let binary = env!("CARGO_BIN_EXE_opi-sandbox");
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("protocol_client.py");
    let output = Command::new(python_command())
        .arg(&script)
        .arg(binary)
        .output()
        .expect("python must be installed to run the backend protocol smoke");
    assert!(
        output.status.success(),
        "protocol client failed against the real binary\n\
         --- stdout ---\n{}\n\
         --- stderr ---\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn silent_host_is_bounded_and_cannot_receive_a_request_scoped_frame() {
    let binary = env!("CARGO_BIN_EXE_opi-sandbox");
    let mut child = Command::new(binary)
        .args(["backend", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn real backend");
    let held_open_stdin = child.stdin.take().expect("piped stdin");
    let deadline = Instant::now() + Duration::from_secs(8);
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll backend") {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().expect("kill hung backend");
            let _ = child.wait();
            panic!("silent host left the backend blocked past its fixed watchdog");
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    drop(held_open_stdin);
    let output = child.wait_with_output().expect("collect backend output");

    assert!(
        !status.success(),
        "no request id means no clean terminal exchange"
    );
    assert!(
        output.stdout.is_empty(),
        "the backend must not fabricate a request-scoped frame without initialize"
    );
}
