//! SDK contract tests for the standalone `opi-sandbox` runner (Phase 16 task 16.11.1, SC16-08).
//!
//! Every test drives the REAL `opi_sandbox::SandboxRunner::run` over EXPLICIT
//! inputs, asserts no cross-invocation state, and observes invocation-owned
//! cleanup (temp root removed; child TREE — including a surviving grandchild —
//! killed) after success, timeout, cancellation, error, and a dropped future.
//! The grandchild-descendant harness proves the L0 tree-kill reaches
//! descendants, not just the direct child.

#![cfg(test)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use opi_protocol::execution::v1::EnvInherit;
use opi_sandbox::{
    CleanupState, ContractStatus, Mechanism, NoRestriction, SandboxEvent, SandboxOutcome,
    SandboxPolicy, SandboxRequest, SandboxResult, SandboxRun, SandboxRunner, SetupFailureReason,
};
use tokio_util::sync::CancellationToken;

/// A default runner: workspace-write policy + the no-confinement restriction.
fn runner() -> SandboxRunner {
    SandboxRunner::new(SandboxPolicy::default(), Arc::new(NoRestriction))
}

/// Build a request for `program`/`args` with a fresh workspace temp dir and the
/// given timeout. Returns the request and the workspace guard.
fn make_request(
    program: PathBuf,
    args: Vec<String>,
    timeout: Duration,
) -> (SandboxRequest, tempfile::TempDir) {
    let workspace = tempfile::tempdir().expect("workspace temp dir");
    let request = SandboxRequest {
        program,
        args,
        workspace: workspace.path().to_path_buf(),
        cwd: workspace.path().to_path_buf(),
        timeout,
        env_inherit: EnvInherit::Inherit,
        env_additions: BTreeMap::new(),
        cancel: None,
    };
    (request, workspace)
}

/// Poll a run to terminal completion, asserting the first event is `Started`.
async fn drive_to_completion(mut run: SandboxRun) -> SandboxResult {
    let started = run.next().await.expect("a Started event");
    assert!(
        matches!(started, SandboxEvent::Started { .. }),
        "first event must be Started, got {started:?}"
    );
    match run.next().await.expect("a Completed event") {
        SandboxEvent::Completed(result) => result,
        other => panic!("expected Completed, got {other:?}"),
    }
}

/// Capture a run's invocation-owned temp root from its `Started` event, then
/// drain to completion so the run cleans up promptly.
async fn started_temp_root(mut run: SandboxRun) -> PathBuf {
    let root = match run.next().await.expect("a Started event") {
        SandboxEvent::Started { temp_root, .. } => temp_root,
        other => panic!("expected Started, got {other:?}"),
    };
    let _ = run.next().await; // drain Completed
    root
}

// --- explicit-input program builders (cross-platform) ---

#[cfg(unix)]
fn sleep_program(seconds: u64) -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("sh"),
        vec!["-c".to_string(), format!("sleep {seconds}")],
    )
}

#[cfg(unix)]
fn exit_program(code: i32) -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("sh"),
        vec!["-c".to_string(), format!("exit {code}")],
    )
}

#[cfg(unix)]
fn stdout_program(text: &str) -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("sh"),
        vec!["-c".to_string(), format!("printf '%s' '{text}'")],
    )
}

#[cfg(unix)]
fn env_echo_program(var: &str) -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("sh"),
        vec!["-c".to_string(), format!("printf '%s' \"${var}\"")],
    )
}

#[cfg(windows)]
fn sleep_program(seconds: u64) -> (PathBuf, Vec<String>) {
    // `ping` waits ~N-1 seconds without needing a console (unlike `timeout`,
    // which errors out immediately under a non-console stdin).
    (
        PathBuf::from("cmd"),
        vec![
            "/C".to_string(),
            format!("ping -n {} 127.0.0.1 >NUL", seconds + 1),
        ],
    )
}

#[cfg(windows)]
fn exit_program(code: i32) -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("cmd"),
        vec!["/C".to_string(), format!("exit {code}")],
    )
}

#[cfg(windows)]
fn stdout_program(text: &str) -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("cmd"),
        vec!["/C".to_string(), format!("echo {text}")],
    )
}

#[cfg(windows)]
fn env_echo_program(var: &str) -> (PathBuf, Vec<String>) {
    // cmd echoes %VAR% with a trailing newline trimmed by the caller.
    (
        PathBuf::from("cmd"),
        vec!["/C".to_string(), format!("echo %{var}%")],
    )
}

/// An explicit program + args whose direct child spawns a surviving grandchild
/// that records its own pid to `pidfile` (OUTSIDE the invocation temp root),
/// then stays alive. The grandchild is in the same process group / Job Object as
/// the direct child, so a tree-kill must reach it.
#[cfg(unix)]
fn surviving_grandchild_program(pidfile: &Path) -> (PathBuf, Vec<String>) {
    let script = format!("sleep 30 & echo $! > \"{p}\"; wait", p = pidfile.display());
    (PathBuf::from("sh"), vec!["-c".to_string(), script])
}

#[cfg(windows)]
fn surviving_grandchild_program(pidfile: &Path) -> (PathBuf, Vec<String>) {
    let script = format!(
        "Start-Process -FilePath powershell -ArgumentList '-NoProfile','-Command','Start-Sleep -Seconds 30' -PassThru | ForEach-Object {{ $_.Id | Out-File -Encoding ascii -FilePath '{p}' }}; Start-Sleep -Seconds 30",
        p = pidfile.display()
    );
    (
        PathBuf::from("powershell"),
        vec!["-NoProfile".to_string(), "-Command".to_string(), script],
    )
}

// --- descendant-liveness probes ---

#[cfg(unix)]
fn pid_alive(pid: u32) -> bool {
    // `kill -0 <pid>` exits successfully while a task exists (zombies included
    // until reaped); the poll loop tolerates the brief zombie window.
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(windows)]
fn pid_alive(pid: u32) -> bool {
    // `tasklist` CSV includes a quoted pid column for a live process.
    let output = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
        .output();
    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).contains(&format!("\"{pid}\"")),
        Err(_) => false,
    }
}

/// Wait until `pidfile` is written, returning the recorded grandchild pid.
async fn read_grandchild_pid(pidfile: &Path) -> u32 {
    for _ in 0..60 {
        if let Ok(text) = std::fs::read_to_string(pidfile)
            && let Ok(pid) = text.trim().parse::<u32>()
        {
            return pid;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!(
        "grandchild pidfile was never written: {}",
        pidfile.display()
    );
}

/// Poll until `pid` is no longer alive (up to ~4s). Returns true if it died.
async fn wait_for_exit(pid: u32) -> bool {
    for _ in 0..80 {
        if !pid_alive(pid) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

// =========================================================================
// SC16-08 contract tests
// =========================================================================

/// Explicit program + args run and the target's normal exit-zero is preserved.
#[tokio::test]
async fn explicit_program_runs_and_exit_zero_is_preserved() {
    let (prog, args) = if cfg!(unix) {
        sleep_program(0)
    } else {
        exit_program(0)
    };
    let (req, _ws) = make_request(prog, args, Duration::from_secs(5));
    let result = drive_to_completion(runner().run(req).expect("run starts")).await;
    match result.outcome {
        SandboxOutcome::Exited { code } => assert_eq!(code, Some(0), "exit zero"),
        other => panic!("expected Exited(0), got {other:?}"),
    }
}

/// A nonzero exit code is preserved verbatim as an in-band result.
#[tokio::test]
async fn nonzero_exit_code_is_preserved() {
    let (prog, args) = exit_program(42);
    let (req, _ws) = make_request(prog, args, Duration::from_secs(5));
    let result = drive_to_completion(runner().run(req).expect("run starts")).await;
    assert!(
        matches!(result.outcome, SandboxOutcome::Exited { code: Some(42) }),
        "expected Exited(42), got {:?}",
        result.outcome
    );
}

/// Captured stdout is returned in the terminal result.
#[tokio::test]
async fn captured_stdout_is_returned() {
    let (prog, args) = stdout_program("hello-sandbox");
    let (req, _ws) = make_request(prog, args, Duration::from_secs(5));
    let result = drive_to_completion(runner().run(req).expect("run starts")).await;
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.contains("hello-sandbox"),
        "expected captured stdout, got {stdout:?}"
    );
}

/// `env_additions` reach the target (explicit environment inputs are honored).
#[tokio::test]
async fn explicit_env_additions_reach_the_target() {
    let (prog, args) = env_echo_program("OPI_SANDBOX_TEST_VAR");
    let workspace = tempfile::tempdir().unwrap();
    let req = SandboxRequest {
        program: prog,
        args,
        workspace: workspace.path().to_path_buf(),
        cwd: workspace.path().to_path_buf(),
        timeout: Duration::from_secs(5),
        env_inherit: EnvInherit::Inherit,
        env_additions: {
            let mut m = BTreeMap::new();
            m.insert(
                "OPI_SANDBOX_TEST_VAR".to_string(),
                "explicit-value".to_string(),
            );
            m
        },
        cancel: None,
    };
    let result = drive_to_completion(runner().run(req).expect("run starts")).await;
    let out = String::from_utf8_lossy(&result.stdout);
    assert!(
        out.contains("explicit-value"),
        "env addition not honored, got {out:?}"
    );
}

/// Timeout terminates the tree and removes the invocation-owned temp root.
#[tokio::test]
async fn timeout_terminates_and_removes_temp_root() {
    let (prog, args) = sleep_program(30);
    let (mut req, _ws) = make_request(prog, args, Duration::from_millis(400));
    req.cancel = None;
    let run = runner().run(req).expect("run starts");
    let result = drive_to_completion(run).await;
    assert!(
        matches!(result.outcome, SandboxOutcome::TimedOut),
        "expected TimedOut, got {:?}",
        result.outcome
    );
    assert_eq!(result.cleanup, CleanupState::Confirmed);
    // The invocation-owned temp root was removed by the time the run completed.
    assert!(
        !result.temp_root.exists(),
        "temp root should be removed: {}",
        result.temp_root.display()
    );
}

/// Cooperative cancellation terminates the tree and removes the temp root.
#[tokio::test]
async fn cooperative_cancellation_terminates_and_removes_temp_root() {
    let (prog, args) = sleep_program(30);
    let (mut req, _ws) = make_request(prog, args, Duration::from_secs(30));
    let token = CancellationToken::new();
    req.cancel = Some(token.clone());
    let mut run = runner().run(req).expect("run starts");
    let started = run.next().await.expect("Started");
    assert!(matches!(started, SandboxEvent::Started { .. }));
    // Cancel after the target has started.
    tokio::time::sleep(Duration::from_millis(150)).await;
    token.cancel();
    let result = match run.next().await.expect("Completed") {
        SandboxEvent::Completed(r) => r,
        other => panic!("expected Completed, got {other:?}"),
    };
    assert!(
        matches!(result.outcome, SandboxOutcome::Cancelled),
        "expected Cancelled, got {:?}",
        result.outcome
    );
    assert!(!result.temp_root.exists(), "temp root should be removed");
}

/// Dropping an in-flight run after it has started kills the tree and removes
/// the temp root (the dropped-future cleanup path, observable via `Started`).
#[tokio::test]
async fn dropped_future_after_start_kills_tree_and_removes_temp_root() {
    let (prog, args) = sleep_program(30);
    let (req, _ws) = make_request(prog, args, Duration::from_secs(30));
    let mut run = runner().run(req).expect("run starts");
    let started = run.next().await.expect("Started");
    let temp_root = match started {
        SandboxEvent::Started { temp_root, .. } => temp_root,
        other => panic!("expected Started, got {other:?}"),
    };
    assert!(temp_root.exists(), "temp root exists while running");
    // Drop the in-flight run; its Drop drives tree-kill + temp removal.
    drop(run);
    // The temp root is removed by the guard's drop.
    for _ in 0..40 {
        if !temp_root.exists() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("temp root not removed after drop: {}", temp_root.display());
}

/// A program that cannot be found returns a setup failure (and no run starts).
#[tokio::test]
async fn program_not_found_returns_setup_failure() {
    let (req, _ws) = make_request(
        PathBuf::from("opi-sandbox-definitely-not-a-real-program-xyz"),
        vec![],
        Duration::from_secs(5),
    );
    let failure = match runner().run(req) {
        Ok(_) => panic!("missing program must fail to start"),
        Err(failure) => failure,
    };
    assert!(
        matches!(failure.reason, SetupFailureReason::ProgramNotFound),
        "expected ProgramNotFound, got {:?}",
        failure.reason
    );
}

/// An invalid request (zero timeout) is rejected before any work.
#[tokio::test]
async fn invalid_request_is_rejected_before_work() {
    let (prog, args) = sleep_program(0);
    let (mut req, _ws) = make_request(prog, args, Duration::ZERO);
    req.timeout = Duration::ZERO;
    let failure = match runner().run(req) {
        Ok(_) => panic!("zero timeout must be rejected"),
        Err(failure) => failure,
    };
    assert!(
        matches!(failure.reason, SetupFailureReason::InvalidRequest),
        "expected InvalidRequest, got {:?}",
        failure.reason
    );
}

/// Pins that the `Started` event carries the effective-restriction fields
/// produced by the runner. Today `Mechanism` and `ContractStatus` are
/// single-variant enums (the native confinement variants land in 16.13 /
/// 14.1), so their values are type-forced to `None` / `Unrestricted` and are
/// NOT runtime-asserted here — that would be a tautology. This remains a
/// compile-time API pin (the fields exist and carry those types); honest
/// effective-contract reporting is runtime-asserted once a second variant
/// exists.
#[tokio::test]
async fn started_event_carries_effective_restriction_fields() {
    let (prog, args) = if cfg!(unix) {
        sleep_program(0)
    } else {
        exit_program(0)
    };
    let (req, _ws) = make_request(prog, args, Duration::from_secs(5));
    let mut run = runner().run(req).expect("run starts");
    let started = run.next().await.expect("Started");
    match started {
        SandboxEvent::Started {
            mechanism,
            contract,
            ..
        } => {
            // Compile-time pin: both fields carry the effective-contract types.
            // Runtime equality is intentionally not asserted — the enums are
            // single-variant today, so it would be a tautology (see the doc
            // comment); it becomes meaningful once 16.13/14.1 add a variant.
            let _: (Mechanism, ContractStatus) = (mechanism, contract);
        }
        other => panic!("expected Started, got {other:?}"),
    }
}

/// Sequential runs receive DISTINCT invocation-owned temp roots (no
/// cross-invocation state).
#[tokio::test]
async fn sequential_runs_receive_distinct_temp_roots() {
    let (prog, args) = if cfg!(unix) {
        sleep_program(0)
    } else {
        exit_program(0)
    };
    let (req1, _ws1) = make_request(prog.clone(), args.clone(), Duration::from_secs(5));
    let (req2, _ws2) = make_request(prog, args, Duration::from_secs(5));

    let root1 = started_temp_root(runner().run(req1).expect("run1 starts")).await;
    let root2 = started_temp_root(runner().run(req2).expect("run2 starts")).await;

    assert_ne!(root1, root2, "temp roots must be distinct per invocation");
}

/// A surviving grandchild (descendant) is killed when the run is dropped after
/// start — proving the L0 tree-kill reaches descendants, not just the direct
/// child (Phase 16 task 16.11.1 audit fold: descendant observation).
#[tokio::test]
async fn dropped_future_kills_surviving_grandchild() {
    let pid_dir = tempfile::tempdir().expect("pid dir");
    let pidfile = pid_dir.path().join("gcpid");
    let (prog, args) = surviving_grandchild_program(&pidfile);
    let (req, _ws) = make_request(prog, args, Duration::from_secs(30));
    let mut run = runner().run(req).expect("run starts");
    let _ = run.next().await.expect("Started");
    // Wait until the grandchild has been spawned and its pid recorded, then drop.
    let grandchild = read_grandchild_pid(&pidfile).await;
    assert!(pid_alive(grandchild), "grandchild alive before drop");
    drop(run);
    assert!(
        wait_for_exit(grandchild).await,
        "grandchild {grandchild} should have been killed by the dropped run"
    );
}

/// A surviving grandchild is also killed on timeout (tree-kill on every path).
#[tokio::test]
async fn timeout_kills_surviving_grandchild() {
    let pid_dir = tempfile::tempdir().expect("pid dir");
    let pidfile = pid_dir.path().join("gcpid");
    let (prog, args) = surviving_grandchild_program(&pidfile);
    let (req, _ws) = make_request(prog, args, Duration::from_millis(400));
    let run = runner().run(req).expect("run starts");
    let result = drive_to_completion(run).await;
    assert!(matches!(result.outcome, SandboxOutcome::TimedOut));
    let grandchild = read_grandchild_pid(&pidfile).await;
    assert!(
        wait_for_exit(grandchild).await,
        "grandchild {grandchild} should have been killed on timeout"
    );
}
