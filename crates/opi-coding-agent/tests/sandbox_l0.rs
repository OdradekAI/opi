//! L0 subprocess-tree lifecycle tests for Phase 15 task 15.4.
//!
//! These prove `LocalBashOperations::exec` (and the adapter host on Windows)
//! terminate the WHOLE subprocess tree — not just the direct child — on
//! timeout, cancellation, and a dropped exec future, and that assignment /
//! termination failures emit a stable `CODE_SANDBOX_DEGRADED` diagnostic.
//!
//! # Platform split
//!
//! The marker-grandchild tests are platform-specific. The Windows variants use
//! a PowerShell grandchild that records its PID and sleep; the Unix variants
//! background a `sleep`. On a Windows host only the Windows variants compile
//! and run; the Unix variants are `#[cfg(unix)]` and are verified by Linux CI
//! (ubuntu-latest; the six-target compile matrix is owned by 15.5.6).
//!
//! The injected-failure and structural Job-Object-flag tests are exercised on
//! every supported host.

#![cfg(any(windows, unix))]

use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use opi_coding_agent::diagnostics::CODE_SANDBOX_DEGRADED;
use opi_coding_agent::tool::{BashOperations, BashRequest, LocalBashOperations};
use tokio_util::sync::CancellationToken;

// Env vars that force L0 failures inside `process_tree` (read live). The
// injected-failure tests set them; the marker tests require them CLEAR, so all
// L0-touching tests in this binary are serialized under `L0_LOCK`.
const ENV_ATTACH_FAIL: &str = "OPI_TEST_L0_ATTACH_FAIL";
const ENV_TERMINATE_FAIL: &str = "OPI_TEST_L0_TERMINATE_FAIL";

static L0_LOCK_CELL: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
fn l0_lock() -> &'static tokio::sync::Mutex<()> {
    L0_LOCK_CELL.get_or_init(|| tokio::sync::Mutex::new(()))
}

fn clear_l0_env() {
    unsafe {
        std::env::remove_var(ENV_ATTACH_FAIL);
        std::env::remove_var(ENV_TERMINATE_FAIL);
    }
}

/// Read a repo file relative to the package root (matches the phase-11 doc-guard convention).
fn read_repo_file(relative: &str) -> String {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir.join("../..").join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Poll a pidfile until the grandchild records its PID.
async fn read_pid(path: &Path, timeout_ms: u64) -> Option<u32> {
    let start = std::time::Instant::now();
    loop {
        if let Ok(s) = std::fs::read_to_string(path)
            && let Ok(pid) = s.trim().parse::<u32>()
        {
            return Some(pid);
        }
        if start.elapsed() >= Duration::from_millis(timeout_ms) {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

// =========================================================================
// Windows: PowerShell grandchild detected via `tasklist`
// =========================================================================

#[cfg(windows)]
fn write_grandchild_script(script: &Path, pidfile: &Path) {
    // PowerShell writes its own PID, then sleeps. It is the GRANDCHILD: opi
    // spawns `cmd /C powershell -File ...`, so cmd is the bash child and
    // powershell is its child (the marker grandchild we must prove is killed).
    let body = format!(
        "$PID | Out-File -FilePath '{}' -NoNewline -Encoding ASCII\nStart-Sleep -Seconds 60\n",
        pidfile.to_string_lossy()
    );
    std::fs::write(script, body).unwrap();
}

#[cfg(windows)]
fn bash_command_for_script(script: &Path) -> String {
    // No embedded quotes: the bash backend wraps this in `cmd /C <command>`,
    // and Rust's MSVC-style arg quoting + cmd's own quote stripping mangle an
    // embedded `\"`. tempdir paths contain no spaces, so the bare path is safe.
    format!(
        "powershell -NoProfile -ExecutionPolicy Bypass -File {}",
        script.to_string_lossy()
    )
}

#[cfg(windows)]
fn process_alive(pid: u32) -> bool {
    let out = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output();
    match out {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout);
            s.contains(&pid.to_string()) && !s.contains("No tasks")
        }
        Err(_) => false,
    }
}

#[cfg(windows)]
async fn await_process_dead_windows(pid: u32, timeout_ms: u64) -> bool {
    let start = std::time::Instant::now();
    loop {
        if !process_alive(pid) {
            return true;
        }
        if start.elapsed() >= Duration::from_millis(timeout_ms) {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(windows)]
fn quick_request(cwd: &Path) -> BashRequest {
    BashRequest {
        command: "echo hi".to_string(),
        cwd: cwd.to_path_buf(),
        timeout: Duration::from_secs(5),
        signal: CancellationToken::new(),
        env: vec![],
    }
}

#[cfg(windows)]
fn long_request(cwd: &Path) -> BashRequest {
    BashRequest {
        command: "ping -n 30 127.0.0.1 >nul".to_string(),
        cwd: cwd.to_path_buf(),
        timeout: Duration::from_millis(800),
        signal: CancellationToken::new(),
        env: vec![],
    }
}

/// Scenario `phase15-l0-bash-tree-lifecycle`: in the default (off) sandbox
/// mode, timing out a bash command that has spawned a marker grandchild
/// terminates the whole subprocess tree.
#[cfg(windows)]
#[tokio::test]
async fn bash_l0_kills_process_tree_in_off_mode() {
    let _g = l0_lock().lock().await;
    clear_l0_env();
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("gc.ps1");
    let pidfile = dir.path().join("pid.txt");
    write_grandchild_script(&script, &pidfile);

    let ops = LocalBashOperations::new();
    let req = BashRequest {
        command: bash_command_for_script(&script),
        cwd: dir.path().to_path_buf(),
        timeout: Duration::from_millis(1500),
        signal: CancellationToken::new(),
        env: vec![],
    };
    let result = ops.exec(req).await.unwrap();
    assert!(
        result.exit_code.is_none(),
        "exec should time out, got exit {:?}",
        result.exit_code
    );

    let pid = read_pid(&pidfile, 2000)
        .await
        .expect("grandchild should record its pid");
    let dead = await_process_dead_windows(pid, 4000).await;
    assert!(dead, "L0 should kill grandchild pid {pid} on timeout");
}

#[cfg(windows)]
#[tokio::test]
async fn bash_l0_kills_process_tree_on_cancel() {
    let _g = l0_lock().lock().await;
    clear_l0_env();
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("gc.ps1");
    let pidfile = dir.path().join("pid.txt");
    write_grandchild_script(&script, &pidfile);

    let token = CancellationToken::new();
    let ops = LocalBashOperations::new();
    let req = BashRequest {
        command: bash_command_for_script(&script),
        cwd: dir.path().to_path_buf(),
        timeout: Duration::from_secs(60),
        signal: token.clone(),
        env: vec![],
    };
    let handle = tokio::spawn(async move { ops.exec(req).await });
    // Let the grandchild spawn, then cancel.
    tokio::time::sleep(Duration::from_millis(1200)).await;
    token.cancel();
    let result = handle.await.unwrap().unwrap();
    assert!(
        result.exit_code.is_none(),
        "cancelled exec must not report an exit code"
    );

    let pid = read_pid(&pidfile, 2000)
        .await
        .expect("grandchild should record its pid");
    let dead = await_process_dead_windows(pid, 4000).await;
    assert!(dead, "L0 should kill grandchild pid {pid} on cancellation");
}

/// DoD gate `dropped_exec_and_injected_failures`: (A) dropping an in-flight
/// exec future removes the tree; (B) an injected attach failure emits a
/// `CODE_SANDBOX_DEGRADED` diagnostic with the redacted `{layer, reason}`;
/// (C) an injected termination failure on timeout does the same.
#[cfg(windows)]
#[tokio::test]
async fn dropped_exec_and_injected_failures() {
    let _g = l0_lock().lock().await;

    let dir = tempfile::tempdir().unwrap();

    // --- (A) dropped exec future tears down the tree ----------------------
    clear_l0_env();
    let script = dir.path().join("gc.ps1");
    let pidfile = dir.path().join("pid.txt");
    write_grandchild_script(&script, &pidfile);
    let ops = LocalBashOperations::new();
    let req = BashRequest {
        command: bash_command_for_script(&script),
        cwd: dir.path().to_path_buf(),
        timeout: Duration::from_secs(60),
        signal: CancellationToken::new(),
        env: vec![],
    };
    let outcome = tokio::select! {
        _ = tokio::time::sleep(Duration::from_millis(1500)) => "dropped",
        r = ops.exec(req) => { let _ = r; "completed" }
    };
    assert_eq!(
        outcome, "dropped",
        "exec should not complete before the future is dropped"
    );
    let pid = read_pid(&pidfile, 2000)
        .await
        .expect("grandchild should record its pid");
    let dead = await_process_dead_windows(pid, 4000).await;
    assert!(
        dead,
        "dropping the exec future must kill grandchild pid {pid}"
    );

    // --- (B) injected attach failure -> degraded diagnostic ---------------
    clear_l0_env();
    unsafe {
        std::env::set_var(ENV_ATTACH_FAIL, "1");
    }
    let res = LocalBashOperations::new()
        .exec(quick_request(dir.path()))
        .await
        .unwrap();
    unsafe {
        std::env::remove_var(ENV_ATTACH_FAIL);
    }
    let diag = res
        .diagnostics
        .iter()
        .find(|d| d.code == CODE_SANDBOX_DEGRADED)
        .expect("attach failure must emit CODE_SANDBOX_DEGRADED");
    let layer = diag
        .details
        .as_ref()
        .and_then(|v| v.get("layer"))
        .and_then(|v| v.as_str());
    let reason = diag
        .details
        .as_ref()
        .and_then(|v| v.get("reason"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert_eq!(layer, Some("windows-job"), "degraded diagnostic layer");
    assert!(reason.contains("injected"), "degraded reason: {reason}");

    // --- (C) injected terminate failure on timeout -> degraded diagnostic -
    clear_l0_env();
    unsafe {
        std::env::set_var(ENV_TERMINATE_FAIL, "1");
    }
    let res = LocalBashOperations::new()
        .exec(long_request(dir.path()))
        .await
        .unwrap();
    unsafe {
        std::env::remove_var(ENV_TERMINATE_FAIL);
    }
    assert!(res.exit_code.is_none(), "long request should time out");
    assert!(
        res.diagnostics
            .iter()
            .any(|d| d.code == CODE_SANDBOX_DEGRADED),
        "terminate failure must emit CODE_SANDBOX_DEGRADED"
    );
}

/// Scenario `phase15-l0-windows-adapter-assignment`: the L0 Job Object is
/// configured kill-on-close with no breakaway, and assigning a child to a
/// `TreeGuard` then terminating it reaps the child well before its natural
/// exit (behavioral proof of kill-on-close, no elevation). The bash and adapter
/// PRODUCTION call sites are proven end-to-end by `bash_l0_kills_process_tree_*`
/// (LocalBashOperations::exec) and `adapter_l0_kills_subprocess_tree_on_host_drop`
/// (AdapterHost::start).
#[cfg(windows)]
#[tokio::test]
async fn windows_bash_and_adapter_use_kill_on_close_job() {
    // Behavioral: a long-lived child (~30s) assigned to a TreeGuard is reaped
    // promptly on terminate, proving kill-on-close works (not natural exit).
    let mut child = std::process::Command::new("cmd")
        .args(["/C", "ping -n 30 127.0.0.1 >nul"])
        .spawn()
        .expect("spawn helper child");
    let pid = child.id();
    {
        let mut guard = opi_coding_agent::tool::TreeGuard::attach(pid)
            .expect("job creation + assignment must succeed without elevation");
        guard.terminate();
    }
    let dead = await_process_dead_windows(pid, 3000).await;
    assert!(
        dead,
        "TreeGuard::terminate must reap the child via kill-on-close well before its ~30s natural exit"
    );
    let _ = child.wait();

    // Structural: the Job Object carries kill-on-close and omits breakaway.
    let pt = read_repo_file("crates/opi-coding-agent/src/tool/process_tree.rs");
    assert!(
        pt.contains("JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE"),
        "Job Object must be configured kill-on-close"
    );
    assert!(
        !pt.contains("JOB_OBJECT_LIMIT_BREAKAWAY_OK"),
        "Job Object must NOT allow breakaway"
    );
}

// =========================================================================
// Unix mirrors (compile out on a Windows host; verified by Linux CI)
// =========================================================================

#[cfg(unix)]
async fn await_process_dead_unix(pid: u32, timeout_ms: u64) -> bool {
    let start = std::time::Instant::now();
    loop {
        let alive = std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !alive {
            return true;
        }
        if start.elapsed() >= Duration::from_millis(timeout_ms) {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(unix)]
fn unix_grandchild_command(pidfile: &Path) -> String {
    // `sh -c`: background a long sleep (the marker grandchild), record its pid,
    // and keep the shell waiting so the exec does not return immediately.
    format!("sleep 60 & echo $! > '{}'; wait", pidfile.to_string_lossy())
}

#[cfg(unix)]
#[tokio::test]
async fn bash_l0_kills_process_tree_in_off_mode() {
    let _g = l0_lock().lock().await;
    clear_l0_env();
    let dir = tempfile::tempdir().unwrap();
    let pidfile = dir.path().join("pid.txt");
    let ops = LocalBashOperations::new();
    let req = BashRequest {
        command: unix_grandchild_command(&pidfile),
        cwd: dir.path().to_path_buf(),
        timeout: Duration::from_millis(1500),
        signal: CancellationToken::new(),
        env: vec![],
    };
    let result = ops.exec(req).await.unwrap();
    assert!(result.exit_code.is_none(), "exec should time out");
    let pid = read_pid(&pidfile, 2000)
        .await
        .expect("grandchild should record its pid");
    let dead = await_process_dead_unix(pid, 4000).await;
    assert!(dead, "L0 should kill unix grandchild pid {pid} on timeout");
}

#[cfg(unix)]
#[tokio::test]
async fn bash_l0_kills_process_tree_on_cancel() {
    let _g = l0_lock().lock().await;
    clear_l0_env();
    let dir = tempfile::tempdir().unwrap();
    let pidfile = dir.path().join("pid.txt");
    let token = CancellationToken::new();
    let ops = LocalBashOperations::new();
    let req = BashRequest {
        command: unix_grandchild_command(&pidfile),
        cwd: dir.path().to_path_buf(),
        timeout: Duration::from_secs(60),
        signal: token.clone(),
        env: vec![],
    };
    let handle = tokio::spawn(async move { ops.exec(req).await });
    tokio::time::sleep(Duration::from_millis(1200)).await;
    token.cancel();
    let result = handle.await.unwrap().unwrap();
    assert!(
        result.exit_code.is_none(),
        "cancelled exec must not report an exit code"
    );
    let pid = read_pid(&pidfile, 2000)
        .await
        .expect("grandchild should record its pid");
    let dead = await_process_dead_unix(pid, 4000).await;
    assert!(
        dead,
        "L0 should kill unix grandchild pid {pid} on cancellation"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn dropped_exec_and_injected_failures() {
    let _g = l0_lock().lock().await;
    let dir = tempfile::tempdir().unwrap();

    // (A) dropped exec future tears down the tree.
    clear_l0_env();
    let pidfile = dir.path().join("pid.txt");
    let ops = LocalBashOperations::new();
    let req = BashRequest {
        command: unix_grandchild_command(&pidfile),
        cwd: dir.path().to_path_buf(),
        timeout: Duration::from_secs(60),
        signal: CancellationToken::new(),
        env: vec![],
    };
    let outcome = tokio::select! {
        _ = tokio::time::sleep(Duration::from_millis(1500)) => "dropped",
        r = ops.exec(req) => { let _ = r; "completed" }
    };
    assert_eq!(outcome, "dropped");
    let pid = read_pid(&pidfile, 2000)
        .await
        .expect("grandchild should record its pid");
    let dead = await_process_dead_unix(pid, 4000).await;
    assert!(
        dead,
        "dropping the exec future must kill unix grandchild pid {pid}"
    );

    // (B) injected attach failure -> degraded diagnostic.
    clear_l0_env();
    unsafe {
        std::env::set_var(ENV_ATTACH_FAIL, "1");
    }
    let res = LocalBashOperations::new()
        .exec(BashRequest {
            command: "echo hi".to_string(),
            cwd: dir.path().to_path_buf(),
            timeout: Duration::from_secs(5),
            signal: CancellationToken::new(),
            env: vec![],
        })
        .await
        .unwrap();
    unsafe {
        std::env::remove_var(ENV_ATTACH_FAIL);
    }
    let diag = res
        .diagnostics
        .iter()
        .find(|d| d.code == CODE_SANDBOX_DEGRADED)
        .expect("attach failure must emit CODE_SANDBOX_DEGRADED");
    let layer = diag
        .details
        .as_ref()
        .and_then(|v| v.get("layer"))
        .and_then(|v| v.as_str());
    assert_eq!(layer, Some("unix-pgroup"));
}

// =========================================================================
// Adapter process-group contract (cross-platform structural proof)
// =========================================================================

/// Scenario `phase15-l0-windows-adapter-assignment` (STRUCTURAL Unix pin, NOT a
/// behavioral proof). The DoD deliberately does NOT add L0 tree-kill to the
/// Unix adapter ("the existing Unix adapter process_group(0) path remains
/// intact"), so this asserts only that the Unix spawn path is still wired the
/// same way. The Windows adapter's L0 Job-Object assignment is proven
/// behaviorally by `adapter_l0_kills_subprocess_tree_on_host_drop`
/// (adapter_host.rs), so it is not re-asserted here. A behavioral Unix mirror
/// is deferred to 15.5.6 (Linux CI): it would assert the Unix adapter does NOT
/// reap a marker grandchild on host-drop (the grandchild survives) — the
/// opposite of the Windows-only kill-on-close behavior — matching the DoD
/// scoping. Read this green check as "Unix adapter path unchanged," not as
/// evidence that the Unix adapter tree-kills.
#[test]
fn adapter_process_group_contract() {
    let adapter_src = read_repo_file("crates/opi-coding-agent/src/adapter_host.rs");
    assert!(
        adapter_src.contains("cmd.process_group(0)"),
        "Unix adapter must keep its process_group(0) path intact (DoD: unchanged)"
    );
}
