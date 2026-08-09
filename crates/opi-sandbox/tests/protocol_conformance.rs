//! Portable protocol conformance for the `opi-sandbox` backend --stdio state
//! machine (Phase 16 task 16.12).
//!
//! These tests drive [`opi_sandbox::backend::drive`] with an INJECTED
//! [`NoRestriction`] + `supported = true`, feeding host frames as JSONL over an
//! injected async stdin and capturing the backend's stdout frames.
//! They prove the full success ordering AND every bounded terminal invalid path
//! required by the DoD. The REAL-binary negotiation + unsupported pre-start path
//! is exercised by `tests/backend_protocol_smoke.rs` via the Python fixture
//! client.
//!
//! Host input is written as literal JSONL, while the protocol's own
//! `decode_backend` parses backend output. Tempdir paths are forward-slashed so
//! they need no JSON escaping on Windows.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::io::{Cursor, Write};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use opi_protocol::execution::v1::codec::decode_backend;
use opi_protocol::execution::v1::frames::ExecutePayload;
use opi_protocol::execution::v1::{
    BackendToHost, Bounds, CleanupState, EnvInherit, FailureCode, FailurePhase, HostToBackend,
    NativeString, RequestId, encode_line,
};

use opi_sandbox::policy::{RestrictionCtx, RestrictionSetupError};
use opi_sandbox::{AppliedRestriction, NetworkPolicy, NoRestriction, Restriction, backend};
use tokio::io::{AsyncRead, AsyncWriteExt, ReadBuf};

type TestInput = Pin<Box<dyn AsyncRead + Send>>;

/// Build a host `initialize` JSONL line.
fn init_json(rid: &str, deadline_ms: u64, protocols: &[&str]) -> String {
    init_json_with_config(rid, deadline_ms, "{}", protocols)
}

/// Build a host `initialize` JSONL line with an explicit adapter configuration.
fn init_json_with_config(
    rid: &str,
    deadline_ms: u64,
    adapter_config: &str,
    protocols: &[&str],
) -> String {
    let protos = protocols
        .iter()
        .map(|p| format!("\"{p}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"type":"initialize","payload":{{"request_id":"{rid}","deadline_ms":{deadline_ms},"adapter_config":{adapter_config},"supported_protocols":[{protos}]}}}}"#
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
    exec_json_with_cwd(
        rid, program, args, workspace, workspace, timeout_ms, env_kvs,
    )
}

fn exec_json_with_cwd(
    rid: &str,
    program: &str,
    args: &[&str],
    workspace: &str,
    cwd: &str,
    timeout_ms: u64,
    env_kvs: &[(&str, &str)],
) -> String {
    let frame = HostToBackend::Execute(ExecutePayload {
        request_id: RequestId::new(rid.to_string()).unwrap(),
        program: native(program),
        args: args.iter().map(|value| native(value)).collect(),
        workspace: native(workspace),
        cwd: native(cwd),
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

#[cfg(unix)]
fn native_nul() -> NativeString {
    NativeString::from_bytes([0])
}

#[cfg(windows)]
fn native_nul() -> NativeString {
    NativeString::from_bytes([0, 0])
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
    run_drive_with_restriction(stdin, supported, Arc::new(NoRestriction)).await
}

async fn run_drive_reader(stdin: TestInput, supported: bool) -> (i32, Vec<u8>) {
    run_drive_reader_with_restriction(stdin, supported, Arc::new(NoRestriction)).await
}

async fn run_drive_with_restriction(
    stdin: String,
    supported: bool,
    restriction: Arc<dyn Restriction>,
) -> (i32, Vec<u8>) {
    let bytes = stdin.into_bytes();
    let (mut host, reader) = tokio::io::duplex(bytes.len().max(1));
    host.write_all(&bytes).await.expect("write host frames");
    let result = run_drive_reader_with_restriction(Box::pin(reader), supported, restriction).await;
    drop(host);
    result
}

async fn run_drive_reader_with_restriction(
    stdin: TestInput,
    supported: bool,
    restriction: Arc<dyn Restriction>,
) -> (i32, Vec<u8>) {
    let mut out = Vec::new();
    let code = backend::drive(
        stdin,
        &mut out,
        Bounds::DEFAULT,
        supported,
        &[],
        restriction,
    )
    .await;
    (code, out)
}

struct RecordingRestriction {
    observed_networks: Arc<Mutex<Vec<NetworkPolicy>>>,
    setup_delay: Duration,
}

struct CooperativeRecordingRestriction {
    observed_networks: Arc<Mutex<Vec<NetworkPolicy>>>,
    setup_delay: Duration,
    stopped: Arc<AtomicBool>,
}

struct LatchingRestriction {
    observed_networks: Arc<Mutex<Vec<NetworkPolicy>>>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
    completed: std::sync::mpsc::Sender<()>,
}

struct DelayedFailingRestriction {
    prepare_count: Arc<AtomicUsize>,
    setup_delay: Duration,
}

struct ReleaseFailingRestriction;

struct DelayedFlushWriter {
    bytes: Vec<u8>,
    flushes: usize,
    delay_on_flush: usize,
    delay: Duration,
}

impl Write for DelayedFlushWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.flushes += 1;
        if self.flushes == self.delay_on_flush {
            std::thread::sleep(self.delay);
        }
        Ok(())
    }
}

impl Restriction for RecordingRestriction {
    fn prepare(
        &self,
        _cmd: &mut tokio::process::Command,
        ctx: &RestrictionCtx<'_>,
    ) -> Result<AppliedRestriction, RestrictionSetupError> {
        self.observed_networks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(ctx.network);
        std::thread::sleep(self.setup_delay);
        Ok(AppliedRestriction::none())
    }
}

impl Restriction for CooperativeRecordingRestriction {
    fn prepare(
        &self,
        _cmd: &mut tokio::process::Command,
        ctx: &RestrictionCtx<'_>,
    ) -> Result<AppliedRestriction, RestrictionSetupError> {
        self.observed_networks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(ctx.network);
        let finish = Instant::now() + self.setup_delay;
        while Instant::now() < finish {
            if ctx.setup_cancelled() {
                self.stopped.store(true, Ordering::Release);
                return Err(RestrictionSetupError::Failed("setup-cancelled"));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(AppliedRestriction::none())
    }
}

impl Restriction for LatchingRestriction {
    fn prepare(
        &self,
        _cmd: &mut tokio::process::Command,
        ctx: &RestrictionCtx<'_>,
    ) -> Result<AppliedRestriction, RestrictionSetupError> {
        self.observed_networks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(ctx.network);
        let _ = self
            .release
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .recv();
        let _ = self.completed.send(());
        Ok(AppliedRestriction::none())
    }
}

impl Restriction for DelayedFailingRestriction {
    fn prepare(
        &self,
        _cmd: &mut tokio::process::Command,
        _ctx: &RestrictionCtx<'_>,
    ) -> Result<AppliedRestriction, RestrictionSetupError> {
        self.prepare_count.fetch_add(1, Ordering::Relaxed);
        std::thread::sleep(self.setup_delay);
        Err(RestrictionSetupError::Failed("injected failure"))
    }
}

impl Restriction for ReleaseFailingRestriction {
    fn prepare(
        &self,
        _cmd: &mut tokio::process::Command,
        ctx: &RestrictionCtx<'_>,
    ) -> Result<AppliedRestriction, RestrictionSetupError> {
        let release_gate = ctx.temp_root.join("release.armed");
        std::fs::remove_file(&release_gate).expect("replace release gate file");
        std::fs::create_dir(&release_gate).expect("inject an unremovable release gate");
        Ok(AppliedRestriction::none())
    }
}

fn marker_target(marker: &std::path::Path) -> (&'static str, Vec<String>) {
    if cfg!(windows) {
        (
            "powershell",
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                format!(
                    "Set-Content -LiteralPath '{}' -Value started",
                    marker.display()
                ),
            ],
        )
    } else {
        (
            "sh",
            vec![
                "-c".to_string(),
                format!("printf started > '{}'", marker.display()),
            ],
        )
    }
}

fn recording_restriction(
    observed_networks: Arc<Mutex<Vec<NetworkPolicy>>>,
    setup_delay: Duration,
) -> Arc<dyn Restriction> {
    Arc::new(RecordingRestriction {
        observed_networks,
        setup_delay,
    })
}

fn cooperative_recording_restriction(
    observed_networks: Arc<Mutex<Vec<NetworkPolicy>>>,
    setup_delay: Duration,
    stopped: Arc<AtomicBool>,
) -> Arc<dyn Restriction> {
    Arc::new(CooperativeRecordingRestriction {
        observed_networks,
        setup_delay,
        stopped,
    })
}

fn delayed_failing_restriction(
    prepare_count: Arc<AtomicUsize>,
    setup_delay: Duration,
) -> Arc<dyn Restriction> {
    Arc::new(DelayedFailingRestriction {
        prepare_count,
        setup_delay,
    })
}

fn recorded_networks(observed_networks: &Mutex<Vec<NetworkPolicy>>) -> Vec<NetworkPolicy> {
    observed_networks
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

struct BlockingDropInput {
    dropped: Arc<AtomicBool>,
}

impl AsyncRead for BlockingDropInput {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Pending
    }
}

impl Drop for BlockingDropInput {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release);
    }
}

struct FloodInput {
    initial: Vec<Vec<u8>>,
    next: usize,
    flood: Vec<u8>,
    reads: Arc<AtomicUsize>,
}

impl AsyncRead for FloodInput {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let next = self.next;
        let line = if next < self.initial.len() {
            self.next += 1;
            &self.initial[next]
        } else {
            &self.flood
        };
        assert!(line.len() <= buffer.remaining());
        buffer.put_slice(line);
        self.reads.fetch_add(1, Ordering::Relaxed);
        Poll::Ready(Ok(()))
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

fn assert_pre_admission_protocol_failure(out: &[u8]) {
    assert_eq!(kinds(&parse(out)), vec!["ready", "failed"]);
    let failed = failed_frame(out);
    assert_eq!(failed.code, FailureCode::ProtocolViolation);
    assert_eq!(failed.phase, FailurePhase::Handshake);
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

#[tokio::test]
async fn adapter_config_network_deny_reaches_the_shared_runner() {
    let observed = Arc::new(Mutex::new(Vec::new()));
    let restriction = recording_restriction(observed.clone(), Duration::ZERO);
    let ws = workspace();
    let (program, args) = echo_target();
    let stdin = format!(
        "{}\n{}\n",
        init_json_with_config(
            "r1",
            30_000,
            r#"{"profile":"workspace-write","network":"deny"}"#,
            &["command-execution-jsonl-v1"],
        ),
        exec_json("r1", program, &args, &ws, 10_000, &[])
    );

    let (code, out) = run_drive_with_restriction(stdin, true, restriction).await;

    assert_eq!(code, 0);
    assert!(matches!(completed_frame(&out).exit, Some(0)));
    assert_eq!(recorded_networks(&observed), vec![NetworkPolicy::Deny]);
}

#[tokio::test]
async fn adapter_config_network_allow_reaches_the_shared_runner() {
    let observed = Arc::new(Mutex::new(Vec::new()));
    let restriction = recording_restriction(observed.clone(), Duration::ZERO);
    let ws = workspace();
    let (program, args) = echo_target();
    let stdin = format!(
        "{}\n{}\n",
        init_json_with_config(
            "r1",
            30_000,
            r#"{"profile":"workspace-write","network":"allow"}"#,
            &["command-execution-jsonl-v1"],
        ),
        exec_json("r1", program, &args, &ws, 10_000, &[])
    );

    let (code, out) = run_drive_with_restriction(stdin, true, restriction).await;

    assert_eq!(code, 0);
    assert!(matches!(completed_frame(&out).exit, Some(0)));
    assert_eq!(recorded_networks(&observed), vec![NetworkPolicy::Allow]);
}

#[tokio::test]
async fn empty_adapter_config_preserves_the_default_network_policy() {
    let observed = Arc::new(Mutex::new(Vec::new()));
    let restriction = recording_restriction(observed.clone(), Duration::ZERO);
    let ws = workspace();
    let (program, args) = echo_target();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &args, &ws, 10_000, &[])
    );

    let (code, out) = run_drive_with_restriction(stdin, true, restriction).await;

    assert_eq!(code, 0);
    assert!(matches!(completed_frame(&out).exit, Some(0)));
    assert_eq!(recorded_networks(&observed), vec![NetworkPolicy::Deny]);
}

#[tokio::test]
async fn invalid_adapter_config_value_is_rejected_before_target_start() {
    let marker_dir = tempfile::tempdir().expect("marker dir");
    let marker = marker_dir.path().join("must-not-exist");
    let ws = workspace();
    let (program, args) = marker_target(&marker);
    let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let stdin = format!(
        "{}\n{}\n",
        init_json_with_config(
            "r1",
            30_000,
            r#"{"profile":"workspace-write","network":"blocked"}"#,
            &["command-execution-jsonl-v1"],
        ),
        exec_json("r1", program, &arg_refs, &ws, 10_000, &[])
    );

    let (code, out) = run_drive(stdin, true).await;

    assert_eq!(code, 0);
    let failed = failed_frame(&out);
    assert_eq!(failed.code, FailureCode::ProtocolViolation);
    assert_eq!(failed.phase, FailurePhase::Handshake);
    assert!(!marker.exists(), "invalid config released the target");
}

#[tokio::test]
async fn unknown_adapter_config_field_is_rejected_before_target_start() {
    let marker_dir = tempfile::tempdir().expect("marker dir");
    let marker = marker_dir.path().join("must-not-exist");
    let ws = workspace();
    let (program, args) = marker_target(&marker);
    let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let stdin = format!(
        "{}\n{}\n",
        init_json_with_config(
            "r1",
            30_000,
            r#"{"profile":"workspace-write","network":"deny","extra":true}"#,
            &["command-execution-jsonl-v1"],
        ),
        exec_json("r1", program, &arg_refs, &ws, 10_000, &[])
    );

    let (code, out) = run_drive(stdin, true).await;

    assert_eq!(code, 0);
    let failed = failed_frame(&out);
    assert_eq!(failed.code, FailureCode::ProtocolViolation);
    assert_eq!(failed.phase, FailurePhase::Handshake);
    assert!(!marker.exists(), "unknown config field released the target");
}

#[tokio::test]
async fn non_object_adapter_config_is_rejected_before_target_start() {
    let marker_dir = tempfile::tempdir().expect("marker dir");
    let marker = marker_dir.path().join("must-not-exist");
    let ws = workspace();
    let (program, args) = marker_target(&marker);
    let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let stdin = format!(
        "{}\n{}\n",
        init_json_with_config("r1", 30_000, "null", &["command-execution-jsonl-v1"],),
        exec_json("r1", program, &arg_refs, &ws, 10_000, &[])
    );

    let (code, out) = run_drive(stdin, true).await;

    assert_eq!(code, 0);
    let failed = failed_frame(&out);
    assert_eq!(failed.code, FailureCode::ProtocolViolation);
    assert_eq!(failed.phase, FailurePhase::Handshake);
    assert!(!marker.exists(), "non-object config released the target");
}

#[tokio::test]
async fn initialize_deadline_expiry_during_setup_fails_closed() {
    let marker_dir = tempfile::tempdir().expect("marker dir");
    let marker = marker_dir.path().join("must-not-exist");
    let observed = Arc::new(Mutex::new(Vec::new()));
    let stopped = Arc::new(AtomicBool::new(false));
    let restriction = cooperative_recording_restriction(
        observed.clone(),
        Duration::from_millis(500),
        stopped.clone(),
    );
    let ws = workspace();
    let (program, args) = marker_target(&marker);
    let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 700, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &arg_refs, &ws, 10_000, &[])
    );

    let (code, out) = tokio::time::timeout(
        Duration::from_secs(2),
        run_drive_with_restriction(stdin, true, restriction),
    )
    .await
    .expect("setup deadline path must remain bounded");
    assert_eq!(code, 0);
    let failed = failed_frame(&out);
    assert_eq!(failed.code, FailureCode::ExecutionTimedOut);
    assert_eq!(failed.phase, FailurePhase::Handshake);
    assert_eq!(recorded_networks(&observed), vec![NetworkPolicy::Deny]);
    assert!(
        stopped.load(Ordering::Acquire),
        "cooperative setup did not observe its cutoff"
    );
    assert!(!marker.exists(), "expired setup released the target");
}

#[tokio::test]
async fn non_cooperative_setup_is_observed_only_until_the_hard_deadline() {
    let marker_dir = tempfile::tempdir().expect("marker dir");
    let marker = marker_dir.path().join("must-not-exist");
    let observed = Arc::new(Mutex::new(Vec::new()));
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let (completed_tx, completed_rx) = std::sync::mpsc::channel();
    let restriction = Arc::new(LatchingRestriction {
        observed_networks: observed.clone(),
        release: Mutex::new(release_rx),
        completed: completed_tx,
    });
    let ws = workspace();
    let (program, args) = marker_target(&marker);
    let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 700, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &arg_refs, &ws, 10_000, &[])
    );

    let (code, out) = tokio::time::timeout(
        Duration::from_secs(2),
        run_drive_with_restriction(stdin, true, restriction),
    )
    .await
    .expect("hard deadline must bound non-cooperative setup");
    assert_eq!(code, 0);
    assert_eq!(kinds(&parse(&out)), vec!["ready", "accepted", "failed"]);
    let failed = failed_frame(&out);
    assert_eq!(failed.code, FailureCode::CleanupUnconfirmed);
    assert_eq!(failed.phase, FailurePhase::Cleanup);
    assert_eq!(recorded_networks(&observed), vec![NetworkPolicy::Deny]);
    assert!(
        completed_rx.try_recv().is_err(),
        "non-cooperative setup finished before the backend hard deadline"
    );
    release_tx.send(()).expect("release setup worker");
    tokio::task::spawn_blocking(move || completed_rx.recv_timeout(Duration::from_secs(1)))
        .await
        .expect("join completion observer")
        .expect("setup worker completes after release");
    assert!(!marker.exists(), "expired setup released a late target");
}

#[tokio::test]
async fn initialize_deadline_wins_over_a_delayed_setup_refusal() {
    let marker_dir = tempfile::tempdir().expect("marker dir");
    let marker = marker_dir.path().join("must-not-exist");
    let prepare_count = Arc::new(AtomicUsize::new(0));
    let restriction =
        delayed_failing_restriction(prepare_count.clone(), Duration::from_millis(700));
    let ws = workspace();
    let (program, args) = marker_target(&marker);
    let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 1_000, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &arg_refs, &ws, 10_000, &[])
    );

    let (code, out) = tokio::time::timeout(
        Duration::from_secs(5),
        run_drive_with_restriction(stdin, true, restriction),
    )
    .await
    .expect("delayed setup refusal must remain bounded");

    assert_eq!(code, 0);
    assert_eq!(kinds(&parse(&out)), vec!["ready", "accepted", "failed"]);
    let failed = failed_frame(&out);
    assert_eq!(failed.code, FailureCode::ExecutionTimedOut);
    assert_eq!(failed.phase, FailurePhase::Handshake);
    assert_eq!(prepare_count.load(Ordering::Relaxed), 1);
    assert!(!marker.exists(), "refused setup released the target");
}

#[tokio::test]
async fn execution_cutoff_after_started_flush_never_releases_the_target() {
    let marker_dir = tempfile::tempdir().expect("marker dir");
    let marker = marker_dir.path().join("must-not-exist");
    let ws = workspace();
    let (program, args) = marker_target(&marker);
    let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let input = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &arg_refs, &ws, 100, &[])
    );
    let bytes = input.into_bytes();
    let (mut host, reader) = tokio::io::duplex(bytes.len().max(1));
    host.write_all(&bytes).await.expect("write host frames");
    let mut out = DelayedFlushWriter {
        bytes: Vec::new(),
        flushes: 0,
        delay_on_flush: 3,
        delay: Duration::from_millis(200),
    };

    let code = backend::drive(
        Box::pin(reader),
        &mut out,
        Bounds::DEFAULT,
        true,
        &[],
        Arc::new(NoRestriction),
    )
    .await;
    drop(host);

    assert_eq!(code, 0);
    assert_eq!(
        kinds(&parse(&out.bytes)),
        vec!["ready", "accepted", "started", "failed"]
    );
    let failed = failed_frame(&out.bytes);
    assert_eq!(failed.code, FailureCode::ExecutionTimedOut);
    assert_eq!(failed.phase, FailurePhase::Execution);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !marker.exists(),
        "target crossed the gate after its execution cutoff"
    );
}

#[tokio::test]
async fn injected_release_failure_is_execution_failed_in_execution_phase() {
    let ws = workspace();
    let (program, args) = echo_target();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &args, &ws, 10_000, &[])
    );

    let (code, out) =
        run_drive_with_restriction(stdin, true, Arc::new(ReleaseFailingRestriction)).await;

    assert_eq!(code, 0);
    let failed = failed_frame(&out);
    assert_eq!(failed.code, FailureCode::ExecutionFailed);
    assert_eq!(failed.phase, FailurePhase::Execution);
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
    assert_eq!(completed.cleanup, CleanupState::Confirmed);
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
    assert_eq!(completed.cleanup, CleanupState::Confirmed);
}

#[tokio::test]
async fn absolute_deadline_reserves_time_for_confirmed_timeout_cleanup() {
    let ws = workspace();
    let (program, args) = sleep_target();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 2_500, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &args, &ws, 30_000, &[])
    );
    let started = Instant::now();

    let (code, out) = tokio::time::timeout(Duration::from_secs(3), run_drive(stdin, true))
        .await
        .expect("the single request deadline must bound execution and cleanup");

    assert_eq!(code, 0);
    let completed = completed_frame(&out);
    assert!(completed.timed_out);
    assert!(!completed.cancelled);
    assert_eq!(completed.cleanup, CleanupState::Confirmed);
    assert!(
        started.elapsed() <= Duration::from_millis(2_650),
        "cleanup must not receive a fresh post-deadline grace: {:?}",
        started.elapsed()
    );
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
    // The runner timeout and backend absolute deadline are intentionally the
    // same instant. Scheduling can therefore observe expiry either before
    // Started (Handshake) or while dropping the started run (Cleanup).
    match failed.code {
        FailureCode::ExecutionTimedOut => assert_eq!(failed.phase, FailurePhase::Handshake),
        FailureCode::CleanupUnconfirmed => assert_eq!(failed.phase, FailurePhase::Cleanup),
        other => panic!("unexpected deadline failure: {other:?}"),
    }
}

#[tokio::test]
async fn initialize_deadline_expires_while_waiting_for_execute() {
    let input = format!(
        "{}\n",
        init_json("r1", 100, &["command-execution-jsonl-v1"])
    );
    let result =
        tokio::time::timeout(std::time::Duration::from_secs(2), run_drive(input, true)).await;
    let (code, out) = result.expect("initialize deadline must bound execute wait");
    assert_eq!(code, 0);
    let failed = failed_frame(&out);
    assert_eq!(failed.code, FailureCode::ExecutionTimedOut);
    assert_eq!(failed.phase, FailurePhase::Handshake);
}

#[tokio::test]
async fn initialize_watchdog_releases_owned_silent_input() {
    let dropped = Arc::new(AtomicBool::new(false));
    let reader = BlockingDropInput {
        dropped: dropped.clone(),
    };

    let (code, out) = tokio::time::timeout(
        Duration::from_secs(7),
        run_drive_reader(Box::pin(reader), true),
    )
    .await
    .expect("initialize watchdog must remain bounded");
    let dropped_at_return = dropped.load(Ordering::Acquire);

    assert_eq!(code, 1);
    assert!(out.is_empty());
    assert!(
        dropped_at_return,
        "drive returned while a reader worker still owned the silent input"
    );
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
        run_drive_reader(Box::pin(Cursor::new(stdin.into_bytes())), true),
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
    let bytes = stdin.into_bytes();
    let (mut host, reader) = tokio::io::duplex(bytes.len().max(1));
    host.write_all(&bytes).await.expect("write host frames");
    let mut drive = Box::pin(run_drive_reader(Box::pin(reader), true));
    let pid = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            tokio::select! {
                result = &mut drive => panic!("backend ended before target pid was visible: {result:?}"),
                _ = tokio::time::sleep(Duration::from_millis(50)) => {
                    if let Some(pid) = std::fs::read_to_string(&pidfile)
                        .ok()
                        .and_then(|text| text.trim().parse::<u32>().ok())
                    {
                        break pid;
                    }
                }
            }
        }
    })
    .await
    .expect("target pidfile");
    drop(host);
    let (code, out) = tokio::time::timeout(std::time::Duration::from_secs(5), &mut drive)
        .await
        .expect("premature EOF must cancel and reap the target tree");
    assert_eq!(code, 0);
    let failed = failed_frame(&out);
    assert_eq!(failed.code, FailureCode::ProtocolViolation);
    for _ in 0..80 {
        if !pid_alive(pid) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("grandchild {pid} survived premature EOF cleanup");
}

#[tokio::test]
async fn bounded_async_reader_stops_a_flooding_host() {
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
        run_drive_reader(Box::pin(reader), true),
    )
    .await
    .expect("protocol violation must terminate a flooded exchange");
    assert_eq!(code, 0);
    assert_eq!(failed_frame(&out).code, FailureCode::ProtocolViolation);
    assert!(
        reads.load(Ordering::Relaxed) <= 16,
        "bounded reader must stop near the violation; reads={}",
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
    assert_eq!(kinds(&frames), vec!["ready", "accepted", "failed"]);
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
    assert_eq!(
        kinds(&parse(&out)),
        vec!["ready", "accepted", "failed"],
        "execution setup failures remain post-admission"
    );
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
    assert_pre_admission_protocol_failure(&out);
}

#[tokio::test]
async fn nonexistent_workspace_is_rejected_before_admission() {
    let parent = tempfile::tempdir().expect("workspace parent");
    let missing = parent
        .path()
        .join("missing")
        .to_string_lossy()
        .replace('\\', "/");
    let (program, args) = echo_target();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &args, &missing, 10_000, &[]),
    );

    let (code, out) = run_drive(stdin, true).await;

    assert_eq!(code, 0);
    assert_pre_admission_protocol_failure(&out);
}

#[tokio::test]
async fn workspace_file_is_rejected_before_admission() {
    let parent = tempfile::tempdir().expect("workspace parent");
    let file = parent.path().join("not-a-directory");
    std::fs::write(&file, b"fixture").expect("workspace file");
    let file = file.to_string_lossy().replace('\\', "/");
    let (program, args) = echo_target();
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        exec_json("r1", program, &args, &file, 10_000, &[]),
    );

    let (code, out) = run_drive(stdin, true).await;

    assert_eq!(code, 0);
    assert_pre_admission_protocol_failure(&out);
}

#[tokio::test]
async fn invalid_cwd_is_rejected_before_admission() {
    let workspace = tempfile::tempdir().expect("workspace");
    let outside = tempfile::tempdir().expect("outside cwd");
    let missing = workspace.path().join("missing");
    let workspace = workspace.path().to_string_lossy().replace('\\', "/");
    let outside = outside.path().to_string_lossy().replace('\\', "/");
    let missing = missing.to_string_lossy().replace('\\', "/");
    let (program, args) = echo_target();

    for cwd in [&outside, &missing] {
        let stdin = format!(
            "{}\n{}\n",
            init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
            exec_json_with_cwd("r1", program, &args, &workspace, cwd, 10_000, &[]),
        );
        let (code, out) = run_drive(stdin, true).await;

        assert_eq!(code, 0);
        assert_pre_admission_protocol_failure(&out);
    }
}

#[cfg(windows)]
#[tokio::test]
async fn malformed_native_string_is_rejected_before_admission() {
    let ws = workspace();
    let execute = HostToBackend::Execute(ExecutePayload {
        request_id: RequestId::new("r1".to_string()).unwrap(),
        program: NativeString::from_bytes(*b"x"),
        args: Vec::new(),
        workspace: native(&ws),
        cwd: native(&ws),
        timeout_ms: 10_000,
        env_inherit: EnvInherit::Inherit,
        env_additions: BTreeMap::new(),
    });
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        encode_line(&execute, &Bounds::DEFAULT).unwrap(),
    );

    let (code, out) = run_drive(stdin, true).await;

    assert_eq!(code, 0);
    assert_pre_admission_protocol_failure(&out);
}

#[tokio::test]
async fn native_string_with_nul_is_rejected_before_admission() {
    let ws = workspace();
    let execute = HostToBackend::Execute(ExecutePayload {
        request_id: RequestId::new("r1".to_string()).unwrap(),
        program: native_nul(),
        args: Vec::new(),
        workspace: native(&ws),
        cwd: native(&ws),
        timeout_ms: 10_000,
        env_inherit: EnvInherit::Inherit,
        env_additions: BTreeMap::new(),
    });
    let stdin = format!(
        "{}\n{}\n",
        init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
        encode_line(&execute, &Bounds::DEFAULT).unwrap(),
    );

    let (code, out) = run_drive(stdin, true).await;

    assert_eq!(code, 0);
    assert_pre_admission_protocol_failure(&out);
}

#[tokio::test]
async fn invalid_environment_keys_are_rejected_before_admission() {
    let ws = workspace();
    let (program, args) = echo_target();

    for key in ["", "A=B"] {
        let execute = HostToBackend::Execute(ExecutePayload {
            request_id: RequestId::new("r1".to_string()).unwrap(),
            program: native(program),
            args: args.iter().map(|value| native(value)).collect(),
            workspace: native(&ws),
            cwd: native(&ws),
            timeout_ms: 10_000,
            env_inherit: EnvInherit::Inherit,
            env_additions: [(native(key), native("value"))].into_iter().collect(),
        });
        let stdin = format!(
            "{}\n{}\n",
            init_json("r1", 30_000, &["command-execution-jsonl-v1"]),
            encode_line(&execute, &Bounds::DEFAULT).unwrap(),
        );

        let (code, out) = run_drive(stdin, true).await;

        assert_eq!(code, 0);
        assert_pre_admission_protocol_failure(&out);
    }
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
