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
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use opi_protocol::execution::v1::EnvInherit;
use opi_sandbox::{
    AppliedRestriction, CleanupState, ContractStatus, Mechanism, NoRestriction, Restriction,
    SandboxEvent, SandboxOutcome, SandboxPolicy, SandboxRequest, SandboxResult, SandboxRun,
    SandboxRunner, SetupFailureReason, StdinPolicy,
};
use tokio_util::sync::CancellationToken;

/// A default runner: workspace-write policy + the no-confinement restriction.
fn runner() -> SandboxRunner {
    SandboxRunner::new(SandboxPolicy::default(), Arc::new(NoRestriction))
}

struct InconsistentRestriction;

impl Restriction for InconsistentRestriction {
    fn prepare(
        &self,
        _cmd: &mut tokio::process::Command,
        _ctx: &opi_sandbox::policy::RestrictionCtx<'_>,
    ) -> Result<AppliedRestriction, opi_sandbox::policy::RestrictionSetupError> {
        Ok(AppliedRestriction {
            mechanism: Mechanism::None,
            contract: ContractStatus::Restricted,
        })
    }
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
        args: args.into_iter().map(OsString::from).collect(),
        workspace: workspace.path().to_path_buf(),
        cwd: workspace.path().to_path_buf(),
        timeout,
        env_inherit: EnvInherit::Inherit,
        env_additions: BTreeMap::new(),
        stdin: StdinPolicy::Null,
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
fn signal_self_program() -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("sh"),
        vec!["-c".to_string(), "kill -TERM $$".to_string()],
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

#[cfg(unix)]
fn temp_env_program() -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("sh"),
        vec![
            "-c".to_string(),
            "printf '%s|%s|%s' \"$TMPDIR\" \"$TMP\" \"$TEMP\"".to_string(),
        ],
    )
}

#[cfg(unix)]
fn marker_program(marker: &Path) -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("sh"),
        vec![
            "-c".to_string(),
            format!("printf started > '{}'", marker.display()),
        ],
    )
}

#[cfg(unix)]
fn large_stdout_program(size: usize) -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("sh"),
        vec!["-c".to_string(), format!("head -c {size} /dev/zero")],
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

#[cfg(windows)]
fn temp_env_program() -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("powershell"),
        vec![
            "-NoProfile".to_string(),
            "-Command".to_string(),
            "Write-Output ([string]::Join([char]124,@($env:TMPDIR,$env:TMP,$env:TEMP)))"
                .to_string(),
        ],
    )
}

#[cfg(windows)]
fn marker_program(marker: &Path) -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("powershell"),
        vec![
            "-NoProfile".to_string(),
            "-Command".to_string(),
            format!(
                "Set-Content -NoNewline -LiteralPath '{}' -Value started",
                marker.display()
            ),
        ],
    )
}

#[cfg(windows)]
fn large_stdout_program(size: usize) -> (PathBuf, Vec<String>) {
    (
        PathBuf::from("powershell"),
        vec![
            "-NoProfile".to_string(),
            "-Command".to_string(),
            format!(
                "$bytes = New-Object byte[] {size}; [Console]::OpenStandardOutput().Write($bytes, 0, $bytes.Length)"
            ),
        ],
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
    for _ in 0..200 {
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

#[cfg(unix)]
#[tokio::test]
async fn signal_termination_is_structured_not_an_exit_code() {
    let (prog, args) = signal_self_program();
    let (req, _ws) = make_request(prog, args, Duration::from_secs(5));
    let result = drive_to_completion(runner().run(req).expect("run starts")).await;
    assert_eq!(result.outcome, SandboxOutcome::Signaled { signal: 15 });
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
    let (prog, args) = env_echo_program("OPI_EXPLICIT_TEST_VAR");
    let workspace = tempfile::tempdir().unwrap();
    let req = SandboxRequest {
        program: prog,
        args: args.into_iter().map(OsString::from).collect(),
        workspace: workspace.path().to_path_buf(),
        cwd: workspace.path().to_path_buf(),
        timeout: Duration::from_secs(5),
        env_inherit: EnvInherit::Inherit,
        env_additions: {
            let mut m = BTreeMap::new();
            m.insert(
                OsString::from("OPI_EXPLICIT_TEST_VAR"),
                OsString::from("explicit-value"),
            );
            m
        },
        stdin: StdinPolicy::Null,
        cancel: None,
    };
    let result = drive_to_completion(runner().run(req).expect("run starts")).await;
    let out = String::from_utf8_lossy(&result.stdout);
    assert!(
        out.contains("explicit-value"),
        "env addition not honored, got {out:?}"
    );
}

#[cfg(windows)]
#[tokio::test]
async fn windows_rejects_bootstrap_environment_overrides() {
    let (prog, args) = exit_program(0);
    let (mut req, _workspace) = make_request(prog, args, Duration::from_secs(5));
    req.env_additions.insert(
        OsString::from("opi_sandbox_release_gate"),
        OsString::from("caller-controlled"),
    );
    let failure = match runner().run(req) {
        Ok(_) => panic!("bootstrap namespace must be reserved case-insensitively"),
        Err(failure) => failure,
    };
    assert_eq!(failure.reason, SetupFailureReason::InvalidRequest);
}

#[cfg(unix)]
#[tokio::test]
async fn non_utf8_argv_and_environment_round_trip_through_sdk() {
    use std::os::unix::ffi::OsStringExt;

    let invalid_arg = vec![0xff, 0xfe, b'a'];
    let (mut arg_request, _arg_workspace) = make_request(
        PathBuf::from("/usr/bin/printf"),
        Vec::new(),
        Duration::from_secs(5),
    );
    arg_request.args = vec![
        OsString::from("%s"),
        OsString::from_vec(invalid_arg.clone()),
    ];
    let arg_result = drive_to_completion(runner().run(arg_request).expect("argv run starts")).await;
    assert_eq!(arg_result.stdout, invalid_arg);

    let key = vec![b'K', 0xff];
    let value = vec![b'V', 0x80];
    let (mut env_request, _env_workspace) = make_request(
        PathBuf::from("/usr/bin/env"),
        Vec::new(),
        Duration::from_secs(5),
    );
    env_request.env_additions.insert(
        OsString::from_vec(key.clone()),
        OsString::from_vec(value.clone()),
    );
    let env_result = drive_to_completion(runner().run(env_request).expect("env run starts")).await;
    let mut expected = key;
    expected.push(b'=');
    expected.extend(value);
    assert!(
        env_result
            .stdout
            .split(|byte| *byte == b'\n')
            .any(|line| line == expected),
        "native environment entry was not preserved"
    );
}

#[tokio::test]
async fn output_over_capture_cap_retains_prefix_and_reports_truncation() {
    const CAP: usize = 1024 * 1024;
    let (prog, args) = large_stdout_program(CAP + 4096);
    let (req, _ws) = make_request(prog, args, Duration::from_secs(10));
    let result = drive_to_completion(runner().run(req).expect("run starts")).await;
    assert_eq!(result.stdout.len(), CAP);
    assert!(result.stdout_truncated, "truncation must be observable");
    assert!(!result.stderr_truncated);
}

#[tokio::test]
async fn invocation_temp_root_is_exported_through_standard_temp_variables() {
    let (prog, args) = temp_env_program();
    let (req, _ws) = make_request(prog, args, Duration::from_secs(5));
    let mut run = runner().run(req).expect("run starts");
    let temp_root = match run.next().await.expect("Started") {
        SandboxEvent::Started { temp_root, .. } => temp_root,
        other => panic!("expected Started, got {other:?}"),
    };
    let result = match run.next().await.expect("Completed") {
        SandboxEvent::Completed(result) => result,
        other => panic!("expected Completed, got {other:?}"),
    };
    let output = String::from_utf8_lossy(&result.stdout);
    let values: Vec<PathBuf> = output.trim().split('|').map(PathBuf::from).collect();
    assert_eq!(
        values,
        vec![temp_root.clone(), temp_root.clone(), temp_root],
        "outcome={:?}, stderr={:?}",
        result.outcome,
        String::from_utf8_lossy(&result.stderr)
    );
}

#[tokio::test]
async fn cwd_outside_workspace_is_rejected_before_spawn() {
    let (prog, args) = exit_program(0);
    let (mut req, _ws) = make_request(prog, args, Duration::from_secs(5));
    let outside = tempfile::tempdir().expect("outside cwd");
    req.cwd = outside.path().to_path_buf();
    let failure = match runner().run(req) {
        Ok(run) => {
            drop(run);
            panic!("outside cwd must fail before spawn")
        }
        Err(failure) => failure,
    };
    assert_eq!(failure.reason, SetupFailureReason::InvalidRequest);
}

#[tokio::test]
async fn inconsistent_effective_contract_is_rejected_before_spawn() {
    let marker_dir = tempfile::tempdir().expect("marker dir");
    let marker = marker_dir.path().join("must-not-exist.marker");
    let (prog, args) = marker_program(&marker);
    let (req, _ws) = make_request(prog, args, Duration::from_secs(5));
    let inconsistent =
        SandboxRunner::new(SandboxPolicy::default(), Arc::new(InconsistentRestriction));
    let failure = match inconsistent.run(req) {
        Ok(run) => {
            drop(run);
            panic!("inconsistent contract must fail before spawn")
        }
        Err(failure) => failure,
    };
    assert_eq!(failure.reason, SetupFailureReason::RestrictionSetup);
    assert!(!marker.exists(), "target ran despite inconsistent contract");
}

#[tokio::test]
async fn target_cannot_act_until_started_has_been_observed() {
    let marker_dir = tempfile::tempdir().expect("marker dir");
    let marker = marker_dir.path().join("started.marker");
    let (prog, args) = marker_program(&marker);
    let (req, _ws) = make_request(prog, args, Duration::from_secs(5));
    let mut run = runner().run(req).expect("run starts behind gate");

    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(!marker.exists(), "target acted before Started was observed");

    assert!(matches!(
        run.next().await,
        Some(SandboxEvent::Started { .. })
    ));
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        !marker.exists(),
        "target acted before the consumer advanced beyond Started"
    );

    let result = match run.next().await.expect("Completed") {
        SandboxEvent::Completed(result) => result,
        other => panic!("expected Completed, got {other:?}"),
    };
    assert!(
        matches!(result.outcome, SandboxOutcome::Exited { code: Some(0) }),
        "unexpected target outcome: {:?}; stderr={:?}",
        result.outcome,
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(marker.exists(), "target did not run after release");
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

/// The no-restriction runner reports its exact effective mechanism and
/// contract at runtime.
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
            assert_eq!(mechanism, Mechanism::None);
            assert_eq!(contract, ContractStatus::Unrestricted);
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
    run.release().expect("release target after Started");
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
    let (req, _ws) = make_request(prog, args, Duration::from_secs(8));
    let run = runner().run(req).expect("run starts");
    let result = drive_to_completion(run).await;
    assert!(matches!(result.outcome, SandboxOutcome::TimedOut));
    let grandchild = read_grandchild_pid(&pidfile).await;
    assert!(
        wait_for_exit(grandchild).await,
        "grandchild {grandchild} should have been killed on timeout"
    );
}

/// Abruptly terminating the process that owns a run still kills the target
/// tree: Windows relies on Job-Object kill-on-close, while Unix uses the gated
/// bootstrap's parent-death watchdog.
#[tokio::test]
async fn hard_kill_of_run_owner_kills_target_tree() {
    const HELPER: &str = "OPI_SANDBOX_TEST_OWNER_HELPER";
    const PIDFILE: &str = "OPI_SANDBOX_TEST_OWNER_PIDFILE";

    if std::env::var_os(HELPER).is_some() {
        let pidfile = PathBuf::from(std::env::var_os(PIDFILE).expect("helper pidfile"));
        let (prog, args) = surviving_grandchild_program(&pidfile);
        let (req, _workspace) = make_request(prog, args, Duration::from_secs(60));
        let mut run = runner().run(req).expect("helper run starts");
        assert!(matches!(
            run.next().await,
            Some(SandboxEvent::Started { .. })
        ));
        run.release().expect("release helper target");
        std::future::pending::<()>().await;
        unreachable!();
    }

    let pid_dir = tempfile::tempdir().expect("pid dir");
    let pidfile = pid_dir.path().join("owner-grandchild.pid");
    let mut owner = std::process::Command::new(std::env::current_exe().expect("test binary"))
        .args([
            "--exact",
            "hard_kill_of_run_owner_kills_target_tree",
            "--nocapture",
        ])
        .env(HELPER, "1")
        .env(PIDFILE, &pidfile)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn run owner helper");
    let grandchild = tokio::time::timeout(Duration::from_secs(15), read_grandchild_pid(&pidfile))
        .await
        .unwrap_or_else(|_| {
            let _ = owner.kill();
            panic!("owner helper target did not start")
        });
    assert!(pid_alive(grandchild));
    owner.kill().expect("hard-kill run owner");
    let _ = owner.wait();
    assert!(
        wait_for_exit(grandchild).await,
        "grandchild {grandchild} survived abrupt owner termination"
    );
}
