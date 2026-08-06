//! Task 16.7 acceptance: the production `ExecutionProtocolHost` drives the
//! closed `command-execution-jsonl-v1` state machine against the
//! `execution_backend_mock` stdio peer.
//!
//! Feature-gated (`#![cfg(feature = "execution-backend-test-fixture")]`): the
//! heavy subprocess suite is opt-in; the mock `[[test]]` target itself builds
//! under `--all-targets` regardless. Run with:
//!   `cargo test -p opi-coding-agent --features execution-backend-test-fixture
//!    --test execution_backend_mock --no-run` first (builds the mock peer), then
//!   `cargo test -p opi-coding-agent --features execution-backend-test-fixture
//!    --test execution_protocol_host`.

#![cfg(feature = "execution-backend-test-fixture")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use opi_coding_agent::execution::{
    BackendLaunch, CompletedOutcome, ExecutionProtocolFailure, ExecutionProtocolHost,
    ExecutionRequest,
};
use opi_protocol::execution::v1::{
    Bounds, CleanupState, EnvInherit, NativeString, ProtocolId, WIRE_IDENTITY,
};
use tokio_util::sync::CancellationToken;

// Suspended-process creation and ToolHelp thread enumeration are intentionally
// fail-closed on Windows. Limit fixture launch fan-out so the test harness does
// not consume the one-second handshake budget in scheduler contention.
#[cfg(windows)]
static WINDOWS_PROTOCOL_CONCURRENCY: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

/// Locate the `execution_backend_mock` test binary in the same deps directory
/// (mirrors `adapter_host.rs::mock_adapter_bin`).
fn mock_bin() -> PathBuf {
    let current = std::env::current_exe().expect("current exe path");
    let deps_dir = current.parent().expect("deps directory");
    let exact_name = if cfg!(windows) {
        "execution_backend_mock.exe"
    } else {
        "execution_backend_mock"
    };
    let exact_path = deps_dir.join(exact_name);
    if exact_path.exists() {
        return exact_path;
    }
    let exe_suffix = if cfg!(windows) { ".exe" } else { "" };
    let prefix = "execution_backend_mock-";
    if let Ok(entries) = std::fs::read_dir(deps_dir) {
        let newest = entries
            .flatten()
            .filter(|entry| {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                name.starts_with(prefix) && name.ends_with(exe_suffix) && !name.ends_with(".d")
            })
            .max_by_key(|entry| entry.metadata().and_then(|meta| meta.modified()).ok());
        if let Some(entry) = newest {
            return entry.path();
        }
    }
    panic!(
        "Could not find execution_backend_mock binary in {}. Build it first with \
         `cargo test --features execution-backend-test-fixture --test execution_backend_mock --no-run`",
        deps_dir.display()
    );
}

fn supported_protocols() -> Vec<ProtocolId> {
    vec![ProtocolId::new(WIRE_IDENTITY).expect("v1 wire identity is non-empty")]
}

/// Drive the host against the mock selected by `mode_args` (first arg = mode).
async fn run(
    mode_args: &[&str],
    bounds: Bounds,
    deadline: Duration,
) -> Result<CompletedOutcome, ExecutionProtocolFailure> {
    run_with(mode_args, bounds, deadline, CancellationToken::new()).await
}

async fn run_with(
    mode_args: &[&str],
    bounds: Bounds,
    deadline: Duration,
    signal: CancellationToken,
) -> Result<CompletedOutcome, ExecutionProtocolFailure> {
    run_with_handshake(mode_args, bounds, deadline, Duration::from_secs(1), signal).await
}

async fn run_with_handshake(
    mode_args: &[&str],
    bounds: Bounds,
    deadline: Duration,
    handshake_timeout: Duration,
    signal: CancellationToken,
) -> Result<CompletedOutcome, ExecutionProtocolFailure> {
    #[cfg(windows)]
    let _permit = WINDOWS_PROTOCOL_CONCURRENCY
        .acquire()
        .await
        .expect("protocol fixture semaphore remains open");
    let bin = mock_bin();
    let owned: Vec<String> = mode_args.iter().map(|s| (*s).to_string()).collect();
    let workspace = std::env::current_dir().expect("cwd");
    let empty = BTreeMap::<NativeString, NativeString>::new();
    let supported = supported_protocols();
    let executable = std::fs::File::open(&bin).expect("open validated mock");
    let launch = BackendLaunch {
        program: &bin,
        args: &owned,
        validated_executable: &executable,
    };
    let request = ExecutionRequest {
        command: "echo hi",
        workspace: &workspace,
        cwd: &workspace,
        timeout: deadline,
        deadline,
        handshake_timeout,
        expected_implementation: "opi-sandbox",
        expected_implementation_version: "mock-1.0.0",
        expected_target: "mock-target",
        env_inherit: EnvInherit::Inherit,
        env_additions: &empty,
        adapter_config: serde_json::json!({}),
        supported_protocols: &supported,
        signal,
        bounds,
    };
    ExecutionProtocolHost::execute(launch, request).await
}

fn assert_code(err: ExecutionProtocolFailure, expected: &str) {
    assert_eq!(
        err.code(),
        expected,
        "expected {expected:?}, got {} ({err})",
        err.code()
    );
}

// ---------------------------------------------------------------------------
// Happy path + in-band results
// ---------------------------------------------------------------------------

#[tokio::test]
async fn happy_path_full_ordering_and_output() {
    let outcome = run(&["happy_path"], Bounds::DEFAULT, Duration::from_secs(5))
        .await
        .expect("happy path completes");
    assert_eq!(outcome.ready.selected_protocol.as_str(), WIRE_IDENTITY);
    assert_eq!(outcome.ready.implementation.as_str(), "opi-sandbox");
    assert_eq!(outcome.ready.implementation_version, "mock-1.0.0");
    assert_eq!(outcome.ready.target.as_str(), "mock-target");
    assert_eq!(outcome.started.placement, "host");
    assert_eq!(outcome.started.guarantee, "supervised");
    assert_eq!(outcome.stdout, b"hello\n");
    assert_eq!(outcome.exit, Some(0));
    assert_eq!(outcome.signal, None);
    assert_eq!(outcome.cleanup, CleanupState::Confirmed);
}

#[tokio::test]
async fn binary_stdout_round_trips() {
    let outcome = run(&["happy_binary"], Bounds::DEFAULT, Duration::from_secs(5))
        .await
        .expect("binary happy path completes");
    assert_eq!(outcome.stdout, &[0xFF, 0x00, 0x42, b'\n']);
}

#[tokio::test]
async fn nonzero_exit_is_in_band_ok() {
    // DoD: nonzero exit remains an in-band result.
    let outcome = run(&["nonzero_exit"], Bounds::DEFAULT, Duration::from_secs(5))
        .await
        .expect("nonzero exit is Ok");
    assert_eq!(outcome.exit, Some(2));
    assert_eq!(outcome.cleanup, CleanupState::Confirmed);
}

#[tokio::test]
async fn signal_decodes_in_band_ok() {
    // Pure decode of Completed{signal}: cross-platform (no host signal kill).
    let outcome = run(&["signal_in_band"], Bounds::DEFAULT, Duration::from_secs(5))
        .await
        .expect("signal is in-band Ok");
    assert_eq!(outcome.signal, Some(9));
}

// ---------------------------------------------------------------------------
// Protocol violations (each rejection surface distinct)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn malformed_frame_is_protocol_violation() {
    assert_code(
        run(
            &["malformed_frame"],
            Bounds::DEFAULT,
            Duration::from_secs(5),
        )
        .await
        .unwrap_err(),
        "protocol_violation",
    );
}

#[tokio::test]
async fn oversized_frame_is_protocol_violation() {
    // Small bounds so the 8 KiB mock chunk exceeds max_line_size.
    let small = Bounds {
        max_line_size: 256,
        max_decoded_chunk_size: 64,
        max_configuration_size: 64,
        max_diagnostics_size: 64,
        max_cumulative_output: 1024,
    };
    assert_code(
        run(&["oversized_frame"], small, Duration::from_secs(5))
            .await
            .unwrap_err(),
        "protocol_violation",
    );
}

#[tokio::test]
async fn duplicate_once_per_execution_is_protocol_violation() {
    assert_code(
        run(
            &["duplicate_accepted"],
            Bounds::DEFAULT,
            Duration::from_secs(5),
        )
        .await
        .unwrap_err(),
        "protocol_violation",
    );
}

#[tokio::test]
async fn cross_request_id_is_protocol_violation() {
    assert_code(
        run(
            &["cross_request_id"],
            Bounds::DEFAULT,
            Duration::from_secs(5),
        )
        .await
        .unwrap_err(),
        "protocol_violation",
    );
}

#[tokio::test]
async fn unknown_required_field_is_protocol_violation() {
    assert_code(
        run(&["unknown_field"], Bounds::DEFAULT, Duration::from_secs(5))
            .await
            .unwrap_err(),
        "protocol_violation",
    );
}

#[tokio::test]
async fn out_of_order_frame_is_protocol_violation() {
    assert_code(
        run(&["out_of_order"], Bounds::DEFAULT, Duration::from_secs(5))
            .await
            .unwrap_err(),
        "protocol_violation",
    );
}

#[tokio::test]
async fn stdout_contamination_is_protocol_violation() {
    assert_code(
        run(
            &["stdout_contamination"],
            Bounds::DEFAULT,
            Duration::from_secs(5),
        )
        .await
        .unwrap_err(),
        "protocol_violation",
    );
}

#[tokio::test]
async fn premature_eof_is_protocol_violation() {
    assert_code(
        run(&["premature_eof"], Bounds::DEFAULT, Duration::from_secs(5))
            .await
            .unwrap_err(),
        "protocol_violation",
    );
}

#[tokio::test]
async fn crash_before_ready_is_protocol_violation() {
    assert_code(
        run(
            &["crash_before_ready"],
            Bounds::DEFAULT,
            Duration::from_secs(5),
        )
        .await
        .unwrap_err(),
        "protocol_violation",
    );
}

#[tokio::test]
async fn crash_after_ready_is_protocol_violation() {
    assert_code(
        run(
            &["crash_after_ready"],
            Bounds::DEFAULT,
            Duration::from_secs(5),
        )
        .await
        .unwrap_err(),
        "protocol_violation",
    );
}

#[tokio::test]
async fn protocol_incompatible_ready_mismatch() {
    assert_code(
        run(
            &["protocol_incompatible"],
            Bounds::DEFAULT,
            Duration::from_secs(5),
        )
        .await
        .unwrap_err(),
        "protocol_incompatible",
    );
}

#[tokio::test]
async fn ready_identity_version_and_target_must_match_lock() {
    for mode in [
        "ready_identity_mismatch",
        "ready_version_mismatch",
        "ready_target_mismatch",
    ] {
        let err = run(&[mode], Bounds::DEFAULT, Duration::from_secs(3))
            .await
            .expect_err("ready mismatch must fail closed");
        assert_code(err, "protocol_incompatible");
    }
}

#[tokio::test]
async fn late_ready_after_handshake_timeout_is_protocol_violation() {
    #[cfg(not(windows))]
    let started = std::time::Instant::now();
    let err = run_with_handshake(
        &["slow_ready"],
        Bounds::DEFAULT,
        Duration::from_secs(5),
        Duration::from_millis(50),
        CancellationToken::new(),
    )
    .await
    .expect_err("slow ready must exceed configured handshake timeout");
    assert_code(err, "protocol_violation");
    #[cfg(not(windows))]
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[tokio::test]
async fn terminal_requires_immediate_clean_eof() {
    for mode in [
        "terminal_extra_frame",
        "terminal_extra_bytes",
        "failed_terminal_extra_bytes",
    ] {
        let err = run(&[mode], Bounds::DEFAULT, Duration::from_secs(3))
            .await
            .expect_err("terminal contamination must fail closed");
        assert_eq!(
            err.code(),
            "protocol_violation",
            "{mode} must reject bytes after its terminal frame: {err}"
        );
    }
}

#[tokio::test]
async fn terminal_diagnostics_are_merged_and_host_redacted() {
    let outcome = run(
        &["terminal_diagnostic"],
        Bounds::DEFAULT,
        Duration::from_secs(3),
    )
    .await
    .expect("terminal diagnostic path");
    assert_eq!(outcome.diagnostics.len(), 2);
    for diagnostic in &outcome.diagnostics {
        let message = &diagnostic.message;
        assert!(!message.contains("sk-proj-"), "secret leaked: {message}");
        assert!(!message.contains("C:\\private"), "path leaked: {message}");
    }
}

// ---------------------------------------------------------------------------
// Backend distress: typed BackendToHost::Failed frames drive the real execute()
// Failed-terminal arm and every branch of map_failure_code on the production
// path (one test per wire FailureCode).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn failed_unavailable_pre_started_is_adapter_unavailable() {
    let error = run(
        &["failed_pre_started", "unavailable"],
        Bounds::DEFAULT,
        Duration::from_secs(5),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), "adapter_unavailable");
    match &error.failure {
        opi_coding_agent::execution::ExecutionFailure::AdapterUnavailable {
            adapter_id, ..
        } => assert_eq!(adapter_id.as_deref(), Some("opi-sandbox")),
        other => panic!("expected adapter unavailable, got {other:?}"),
    }
    assert!(error.remediation().contains("opi-sandbox"));
}

#[tokio::test]
async fn failed_generic_pre_started_is_execution_failed() {
    assert_code(
        run(
            &["failed_pre_started", "failed"],
            Bounds::DEFAULT,
            Duration::from_secs(5),
        )
        .await
        .unwrap_err(),
        "execution_failed",
    );
}

#[tokio::test]
async fn failed_execution_post_started_is_execution_failed() {
    assert_code(
        run(
            &["failed_post_started", "execution_failed"],
            Bounds::DEFAULT,
            Duration::from_secs(5),
        )
        .await
        .unwrap_err(),
        "execution_failed",
    );
}

#[tokio::test]
async fn failed_timed_out_is_execution_timed_out() {
    assert_code(
        run(
            &["failed_post_started", "execution_timed_out"],
            Bounds::DEFAULT,
            Duration::from_secs(5),
        )
        .await
        .unwrap_err(),
        "execution_timed_out",
    );
}

#[tokio::test]
async fn failed_cleanup_unconfirmed_is_cleanup_unconfirmed() {
    assert_code(
        run(
            &["failed_post_started", "cleanup_unconfirmed"],
            Bounds::DEFAULT,
            Duration::from_secs(5),
        )
        .await
        .unwrap_err(),
        "cleanup_unconfirmed",
    );
}

#[tokio::test]
async fn failed_protocol_incompatible_is_protocol_incompatible() {
    assert_code(
        run(
            &["failed_post_started", "protocol_incompatible"],
            Bounds::DEFAULT,
            Duration::from_secs(5),
        )
        .await
        .unwrap_err(),
        "protocol_incompatible",
    );
}

#[tokio::test]
async fn failed_protocol_violation_is_protocol_violation() {
    assert_code(
        run(
            &["failed_post_started", "protocol_violation"],
            Bounds::DEFAULT,
            Duration::from_secs(5),
        )
        .await
        .unwrap_err(),
        "protocol_violation",
    );
}

// ---------------------------------------------------------------------------
// Deadline / cancel / cleanup
// ---------------------------------------------------------------------------

async fn wait_for_started_marker(path: &Path, timeout: Duration) -> bool {
    let start = std::time::Instant::now();
    loop {
        if path.exists() {
            return true;
        }
        if start.elapsed() >= timeout {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

async fn cancel_after_started(mode: &str) -> Result<CompletedOutcome, ExecutionProtocolFailure> {
    let dir = tempfile::tempdir().expect("started marker directory");
    let marker = dir.path().join("started");
    let marker_arg = marker.to_string_lossy().into_owned();
    let mode_args = [mode, marker_arg.as_str()];
    let signal = CancellationToken::new();
    let ctrl = signal.clone();
    let future = run_with(&mode_args, Bounds::DEFAULT, Duration::from_secs(30), signal);
    tokio::pin!(future);
    tokio::select! {
        reached = wait_for_started_marker(&marker, Duration::from_secs(5)) => {
            assert!(reached, "{mode} must reach started before cancellation");
        }
        result = &mut future => {
            panic!("{mode} completed before reaching started: {result:?}");
        }
    }
    ctrl.cancel();
    future.await
}

#[tokio::test]
async fn hang_before_ready_deadline_is_cleanup_unconfirmed() {
    // deadline 2s -> cancel_at 0.5s; backend never ready -> grace -> kill.
    assert_code(
        run(
            &["hang_before_ready"],
            Bounds::DEFAULT,
            Duration::from_millis(2000),
        )
        .await
        .unwrap_err(),
        "cleanup_unconfirmed",
    );
}

#[tokio::test]
async fn hang_after_started_deadline_is_cleanup_unconfirmed() {
    // deadline 2.5s -> cancel_at 1s (backend already started); grace -> kill.
    #[cfg(not(windows))]
    let started = std::time::Instant::now();
    assert_code(
        run(
            &["hang_after_started"],
            Bounds::DEFAULT,
            Duration::from_millis(2500),
        )
        .await
        .unwrap_err(),
        "cleanup_unconfirmed",
    );
    #[cfg(not(windows))]
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "cancel, cleanup, drain, and reap must share the 2.5s invocation deadline"
    );
}

#[tokio::test]
async fn cancellation_rejects_completed_before_each_required_milestone() {
    for mode in [
        "cancel_completed_pre_ready",
        "cancel_completed_pre_accepted",
        "cancel_completed_pre_started",
    ] {
        let err = run(&[mode], Bounds::DEFAULT, Duration::from_secs(3))
            .await
            .expect_err("completed before the current milestone must fail closed");
        assert_eq!(
            err.code(),
            "protocol_violation",
            "{mode} must remain out of order during cancellation: {err}"
        );
    }
}

#[tokio::test]
async fn cancellation_rejects_failed_before_each_required_milestone() {
    for mode in [
        "cancel_failed_pre_ready",
        "cancel_failed_pre_accepted",
        "cancel_failed_pre_started",
    ] {
        let err = run(&[mode], Bounds::DEFAULT, Duration::from_secs(3))
            .await
            .expect_err("failed before the current milestone must fail closed");
        assert_eq!(
            err.code(),
            "protocol_violation",
            "{mode} must remain out of order during cancellation: {err}"
        );
    }
}

#[tokio::test]
async fn cancellation_pre_ready_rejects_subsequent_negotiation_sequence() {
    let err = run(
        &["cancel_sequence_pre_ready"],
        Bounds::DEFAULT,
        Duration::from_secs(3),
    )
    .await
    .expect_err("cancellation before ready must close negotiation");
    assert_eq!(
        err.code(),
        "protocol_violation",
        "post-cancel ready must not advance to a placeholder-backed success: {err}"
    );
}

#[tokio::test]
async fn failed_terminal_diagnostics_are_merged_and_host_redacted() {
    let error = run(
        &["failed_post_started", "execution_failed"],
        Bounds::DEFAULT,
        Duration::from_secs(5),
    )
    .await
    .expect_err("failed terminal must fail the invocation");
    assert_eq!(error.diagnostics.len(), 2);
    for diagnostic in &error.diagnostics {
        assert!(!diagnostic.message.contains("sk-proj-"));
        assert!(!diagnostic.message.contains("C:\\private"));
    }
}

#[tokio::test]
async fn external_cancel_is_cleanup_unconfirmed() {
    assert_code(
        cancel_after_started("hang_after_started")
            .await
            .unwrap_err(),
        "cleanup_unconfirmed",
    );
}

#[tokio::test]
async fn cancel_confirmed_cleanup_after_started_is_in_band_canceled() {
    let outcome = cancel_after_started("cancel_cleanup_confirmed")
        .await
        .expect("post-start cancellation with confirmed cleanup is in-band");
    assert!(outcome.cancelled);
    assert_eq!(outcome.cleanup, CleanupState::Confirmed);
}

#[tokio::test]
async fn cancel_unconfirmed_cleanup_reports_cleanup_unconfirmed() {
    assert_code(
        cancel_after_started("cancel_cleanup_unconfirmed")
            .await
            .unwrap_err(),
        "cleanup_unconfirmed",
    );
}

// ---------------------------------------------------------------------------
// Redaction
// ---------------------------------------------------------------------------

#[tokio::test]
async fn backend_stderr_canary_never_surfaces() {
    // Backend PROCESS stderr (crash-evidence pipe) carries a unique canary; the
    // failure envelope is payload-free and must never leak it.
    let canary = format!("OPI_REDACT_CANARY_{}", std::process::id());
    let err = run(
        &["redact_canary", &canary],
        Bounds::DEFAULT,
        Duration::from_secs(5),
    )
    .await
    .unwrap_err();
    assert_eq!(err.code(), "protocol_violation");
    let display = format!("{err}");
    let debug = format!("{err:?}");
    assert!(
        !display.contains(&canary),
        "canary leaked into Display: {display}"
    );
    assert!(
        !debug.contains(&canary),
        "canary leaked into Debug: {debug}"
    );
}

// ---------------------------------------------------------------------------
// L0 tree kill on dropped future (cancel/timeout/drop follow the same kill path)
// ---------------------------------------------------------------------------

async fn read_pid_file(path: &Path, timeout: Duration) -> Option<u32> {
    let start = std::time::Instant::now();
    loop {
        if let Ok(s) = std::fs::read_to_string(path)
            && let Ok(pid) = s.trim().parse::<u32>()
        {
            return Some(pid);
        }
        if start.elapsed() >= timeout {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[cfg(unix)]
async fn wait_for_process_dead(pid: u32, timeout: Duration) -> bool {
    let start = std::time::Instant::now();
    loop {
        let alive = std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if !alive {
            return true;
        }
        if start.elapsed() >= timeout {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(windows)]
async fn wait_for_process_dead(pid: u32, timeout: Duration) -> bool {
    let start = std::time::Instant::now();
    loop {
        let alive = match std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
        {
            Ok(o) => {
                let s = String::from_utf8_lossy(&o.stdout);
                s.contains(&pid.to_string()) && !s.contains("No tasks")
            }
            Err(_) => false,
        };
        if !alive {
            return true;
        }
        if start.elapsed() >= timeout {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Cancel/timeout/dropped-future cleanup all follow the SAME local kill path
/// (`TreeGuard::terminate`: process group on Unix, `TerminateJobObject` on the
/// Windows Job). This drives the cancel path against the `l0_grandchild` mock
/// (which leaves a marker grandchild alive) and asserts the whole tree is
/// reaped. Dropping the execution future invokes the same guard's
/// `Drop -> terminate`; cancel exercises that terminate call explicitly and
/// deterministically on both platforms.
#[tokio::test]
async fn tree_kill_reaps_backend_grandchild() {
    let dir = tempfile::tempdir().unwrap();
    let pidfile = dir.path().join("gc_pid.txt");
    let pid_str = pidfile.to_string_lossy().into_owned();
    let bin = mock_bin();
    let owned = vec!["l0_grandchild".to_string(), pid_str];
    let workspace = std::env::current_dir().expect("cwd");
    let empty = BTreeMap::<NativeString, NativeString>::new();
    let supported = supported_protocols();
    let signal = CancellationToken::new();
    let ctrl = signal.clone();
    let executable = std::fs::File::open(&bin).expect("open validated mock");
    let launch = BackendLaunch {
        program: &bin,
        args: &owned,
        validated_executable: &executable,
    };
    let request = ExecutionRequest {
        command: "echo hi",
        workspace: &workspace,
        cwd: &workspace,
        timeout: Duration::from_secs(30),
        deadline: Duration::from_secs(30),
        handshake_timeout: Duration::from_secs(1),
        expected_implementation: "opi-sandbox",
        expected_implementation_version: "mock-1.0.0",
        expected_target: "mock-target",
        env_inherit: EnvInherit::Inherit,
        env_additions: &empty,
        adapter_config: serde_json::json!({}),
        supported_protocols: &supported,
        signal,
        bounds: Bounds::DEFAULT,
    };
    let fut = ExecutionProtocolHost::execute(launch, request);
    tokio::pin!(fut);
    // Poll past spawn + attach + handshake: the grandchild records its pid.
    let gc_pid = tokio::select! {
        pid = read_pid_file(&pidfile, Duration::from_secs(5)) => {
            pid.expect("grandchild should record its pid")
        }
        r = &mut fut => panic!("future completed ({r:?}) before grandchild recorded pid"),
    };
    // Cancel drives finish_with_cancel -> guard.terminate() (the same kill path
    // Drop uses). Poll to completion; it must return CleanupUnconfirmed.
    ctrl.cancel();
    let err = (&mut fut).await.unwrap_err();
    assert_eq!(err.code(), "cleanup_unconfirmed");
    let dead = wait_for_process_dead(gc_pid, Duration::from_secs(8)).await;
    assert!(
        dead,
        "cancel/terminate should reap the backend grandchild pid {gc_pid}"
    );
}

// ---------------------------------------------------------------------------
// Structural guarantees (16.6 lesson 16: do not fake-falsify)
// ---------------------------------------------------------------------------

/// Structural: the host is a pure external-backend driver — it never references
/// `LocalBashOperations` (the only place a `local` fallback could be wired in).
/// That `ExecutionFailure` has no `Local`/fallback variant is already proven by
/// 16.6's `code_values_are_the_14_stable_literals` (all 14 codes, none `local`);
/// this guard closes the host-source side. (16.6 lesson 16: structural claim
/// covered by a real source guard, not a fake-falsified runtime check.)
#[test]
fn no_local_fallback_exists() {
    let src = include_str!("../src/execution/protocol_host.rs");
    assert!(
        !src.contains("LocalBashOperations"),
        "protocol host must not invoke the local backend"
    );
}
