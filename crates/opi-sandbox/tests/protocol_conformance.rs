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

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use opi_protocol::execution::v1::codec::decode_backend;
use opi_protocol::execution::v1::frames::ExecutePayload;
use opi_protocol::execution::v1::{
    BackendToHost, Bounds, CleanupState, EnvInherit, FailureCode, FailurePhase, HostToBackend,
    NativeString, RequestId, encode_line,
};

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
    let frame = HostToBackend::Execute(ExecutePayload {
        request_id: RequestId::new(rid.to_string()).unwrap(),
        program: native(program),
        args: args.iter().map(|value| native(value)).collect(),
        workspace: native(workspace),
        cwd: native(workspace),
        timeout_ms,
        env_inherit: EnvInherit::Inherit,
        env_additions: env_kvs
            .iter()
            .map(|(key, value)| (native(key), native(value)))
            .collect::<BTreeMap<_, _>>(),
    });
    encode_line(&frame, &Bounds::DEFAULT).unwrap()
}

#[cfg(unix)]
fn native(value: &str) -> NativeString {
    NativeString::from_bytes(value.as_bytes())
}

#[cfg(windows)]
fn native(value: &str) -> NativeString {
    use std::os::windows::ffi::OsStrExt;
    NativeString::from_bytes(
        std::ffi::OsStr::new(value)
            .encode_wide()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
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

fn large_output_target() -> (&'static str, Vec<String>) {
    let size = 1024 * 1024 + 4096;
    if cfg!(windows) {
        (
            "powershell",
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                format!(
                    "$b = New-Object byte[] {size}; [Console]::OpenStandardOutput().Write($b, 0, $b.Length)"
                ),
            ],
        )
    } else {
        (
            "sh",
            vec!["-c".to_string(), format!("head -c {size} /dev/zero")],
        )
    }
}

fn surviving_grandchild_target(pidfile: &std::path::Path) -> (&'static str, Vec<String>) {
    if cfg!(windows) {
        (
            "powershell",
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                format!(
                    "Start-Process powershell -ArgumentList '-NoProfile','-Command','Start-Sleep -Seconds 30' -PassThru | % {{ $_.Id | Out-File -Encoding ascii -FilePath '{}' }}; Start-Sleep -Seconds 30",
                    pidfile.display()
                ),
            ],
        )
    } else {
        (
            "sh",
            vec![
                "-c".to_string(),
                format!("sleep 30 & echo $! > '{}'; wait", pidfile.display()),
            ],
        )
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
    let open = Arc::new(AtomicBool::new(true));
    let reader = HeldOpenInput {
        input: Cursor::new(stdin.into_bytes()),
        open: open.clone(),
    };
    let result = run_drive_reader(Box::new(reader), supported).await;
    open.store(false, Ordering::Release);
    result
}

async fn run_drive_reader(stdin: Box<dyn Read + Send>, supported: bool) -> (i32, Vec<u8>) {
    let runner = SandboxRunner::new(SandboxPolicy::default(), Arc::new(NoRestriction));
    let mut out = Vec::new();
    let code = backend::drive(stdin, &mut out, Bounds::DEFAULT, supported, &[], &runner).await;
    (code, out)
}

struct BlockingAfterInput {
    input: Cursor<Vec<u8>>,
    release: std::sync::mpsc::Receiver<()>,
}

struct HeldOpenInput {
    input: Cursor<Vec<u8>>,
    open: Arc<AtomicBool>,
}

impl Read for HeldOpenInput {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let read = self.input.read(buffer)?;
        if read != 0 {
            return Ok(read);
        }
        while self.open.load(Ordering::Acquire) {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        Ok(0)
    }
}

struct FloodInput {
    initial: Vec<Vec<u8>>,
    next: usize,
    flood: Vec<u8>,
    reads: Arc<AtomicUsize>,
}

impl Read for FloodInput {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let line = if let Some(line) = self.initial.get(self.next) {
            self.next += 1;
            line
        } else {
            &self.flood
        };
        assert!(line.len() <= buffer.len());
        buffer[..line.len()].copy_from_slice(line);
        self.reads.fetch_add(1, Ordering::Relaxed);
        Ok(line.len())
    }
}

struct EofAfterFile {
    input: Cursor<Vec<u8>>,
    path: std::path::PathBuf,
}

impl Read for EofAfterFile {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let read = self.input.read(buffer)?;
        if read != 0 {
            return Ok(read);
        }
        for _ in 0..200 {
            if std::fs::read_to_string(&self.path)
                .ok()
                .and_then(|text| text.trim().parse::<u32>().ok())
                .is_some()
            {
                return Ok(0);
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(0)
    }
}

fn pid_alive(pid: u32) -> bool {
    if cfg!(windows) {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .output()
            .is_ok_and(|output| {
                String::from_utf8_lossy(&output.stdout).contains(&format!("\"{pid}\""))
            })
    } else {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .is_ok_and(|status| status.success())
    }
}

impl Read for BlockingAfterInput {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let read = self.input.read(buffer)?;
        if read != 0 {
            return Ok(read);
        }
        let _ = self.release.recv();
        Ok(0)
    }
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
    assert_eq!(ready.implementation.as_str(), "opi-sandbox");
    assert!(!ready.implementation_version.is_empty());
    assert_eq!(ready.target.as_str(), env!("OPI_SANDBOX_BUILD_TARGET"));
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

#[tokio::test]
async fn initialize_deadline_caps_execute_timeout() {
    let ws = workspace();
    let (program, args) = sleep_target();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 300, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &args, &ws, 30_000, &[])
    );
    let (code, out) =
        tokio::time::timeout(std::time::Duration::from_secs(3), run_drive(stdin, true))
            .await
            .expect("initialize deadline must cap the target timeout");
    assert_eq!(code, 0);
    let failed = failed_frame(&out);
    match failed.code {
        FailureCode::ExecutionTimedOut => assert_eq!(failed.phase, FailurePhase::Handshake),
        FailureCode::CleanupUnconfirmed => assert_eq!(failed.phase, FailurePhase::Execution),
        other => panic!("unexpected deadline failure: {other:?}"),
    }
}

#[tokio::test]
async fn initialize_deadline_expires_while_waiting_for_execute() {
    let input = format!(
        "{}\n",
        init_json("r1", 100, &["command-execution-jsonl-v1"])
    );
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let reader = BlockingAfterInput {
        input: Cursor::new(input.into_bytes()),
        release: release_rx,
    };
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        run_drive_reader(Box::new(reader), true),
    )
    .await;
    drop(release_tx);
    let (code, out) = result.expect("initialize deadline must bound execute wait");
    assert_eq!(code, 0);
    let failed = failed_frame(&out);
    assert_eq!(failed.code, FailureCode::ExecutionTimedOut);
    assert_eq!(failed.phase, FailurePhase::Handshake);
}

#[tokio::test]
async fn eof_after_execute_cancels_and_fails_protocol() {
    let ws = workspace();
    let (program, args) = sleep_target();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &args, &ws, 30_000, &[]),
    );
    let (code, out) = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        run_drive_reader(Box::new(Cursor::new(stdin.into_bytes())), true),
    )
    .await
    .expect("premature EOF must not wait for the command timeout");
    assert_eq!(code, 0);
    let failed = failed_frame(&out);
    assert_eq!(failed.code, FailureCode::ProtocolViolation);
    assert_eq!(failed.phase, FailurePhase::Execution);
}

#[tokio::test]
async fn eof_after_target_start_kills_descendant_tree() {
    let pid_dir = tempfile::tempdir().expect("pid dir");
    let pidfile = pid_dir.path().join("grandchild.pid");
    let ws = workspace();
    let (program, args) = surviving_grandchild_target(&pidfile);
    let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &arg_refs, &ws, 30_000, &[])
    );
    let reader = EofAfterFile {
        input: Cursor::new(stdin.into_bytes()),
        path: pidfile.clone(),
    };
    let (code, out) = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        run_drive_reader(Box::new(reader), true),
    )
    .await
    .expect("premature EOF must cancel and reap the target tree");
    assert_eq!(code, 0);
    let failed = failed_frame(&out);
    assert_eq!(failed.code, FailureCode::ProtocolViolation);
    let pid = std::fs::read_to_string(&pidfile)
        .expect("grandchild pidfile")
        .trim()
        .parse::<u32>()
        .expect("grandchild pid");
    for _ in 0..80 {
        if !pid_alive(pid) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("grandchild {pid} survived premature EOF cleanup");
}

#[tokio::test]
async fn bounded_input_channel_backpressures_a_flooding_host() {
    let ws = workspace();
    let (program, args) = sleep_target();
    let init = format!(
        "{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"])
    )
    .into_bytes();
    let execute = format!("{}\n", exec_json("r1", program, &args, &ws, 30_000, &[])).into_bytes();
    let reads = Arc::new(AtomicUsize::new(0));
    let reader = FloodInput {
        initial: vec![init],
        next: 0,
        flood: execute.clone(),
        reads: reads.clone(),
    };
    let (code, out) = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        run_drive_reader(Box::new(reader), true),
    )
    .await
    .expect("protocol violation must terminate a flooded exchange");
    assert_eq!(code, 0);
    assert_eq!(failed_frame(&out).code, FailureCode::ProtocolViolation);
    assert!(
        reads.load(Ordering::Relaxed) <= 16,
        "bounded channel must stop the reader near capacity; reads={}",
        reads.load(Ordering::Relaxed)
    );
}

#[tokio::test]
async fn queued_cancel_wins_a_simultaneous_fast_exit() {
    let ws = workspace();
    let (program, args) = echo_target();
    let stdin = format!(
        "{}\n{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &args, &ws, 30_000, &[]),
        cancel_json("r1", "canceled")
    );
    let (code, out) = run_drive(stdin, true).await;
    assert_eq!(code, 0);
    assert!(completed_frame(&out).cancelled);
}

#[tokio::test]
async fn output_truncation_is_visible_in_terminal_diagnostics() {
    let ws = workspace();
    let (program, args) = large_output_target();
    let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &arg_refs, &ws, 20_000, &[])
    );
    let (code, out) = run_drive(stdin, true).await;
    assert_eq!(code, 0);
    let completed = completed_frame(&out);
    assert!(
        completed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message == "stdout capture truncated")
    );
}

#[cfg(unix)]
#[tokio::test]
async fn non_utf8_argument_round_trips_through_protocol_backend() {
    use std::os::unix::ffi::OsStrExt;

    let workspace = tempfile::tempdir().expect("workspace");
    let invalid = vec![0xff, 0xfe, b'a'];
    let execute = HostToBackend::Execute(ExecutePayload {
        request_id: RequestId::new("r1".to_string()).unwrap(),
        program: NativeString::from_bytes(b"/usr/bin/printf"),
        args: vec![
            NativeString::from_bytes(b"%s"),
            NativeString::from_bytes(invalid.clone()),
        ],
        workspace: NativeString::from_bytes(workspace.path().as_os_str().as_bytes()),
        cwd: NativeString::from_bytes(workspace.path().as_os_str().as_bytes()),
        timeout_ms: 5_000,
        env_inherit: EnvInherit::Inherit,
        env_additions: BTreeMap::new(),
    });
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        encode_line(&execute, &Bounds::DEFAULT).unwrap()
    );
    let (code, out) = run_drive(stdin, true).await;
    assert_eq!(code, 0);
    let stdout = parse(&out)
        .into_iter()
        .filter_map(|frame| match frame {
            BackendToHost::Stdout(payload) => Some(payload.data.into_bytes()),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    assert_eq!(stdout, invalid);
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
