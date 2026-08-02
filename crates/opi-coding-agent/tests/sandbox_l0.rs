//! L0 subprocess-tree lifecycle acceptance for the bash `command.execute` path
//! (Phase 15 task 15.4; re-framed for the Phase 16 policy-neutral supervision
//! seam in task 16.2).
//!
//! These prove `LocalBashOperations::exec` — which routes through the
//! policy-neutral `tool::supervision` module — and the adapter host terminate
//! the WHOLE subprocess tree, not just the direct child, on timeout,
//! cancellation, a clean shell exit, and a dropped exec future. Phase 16 task
//! 16.2 adds two acceptance behaviors: a clean direct-child exit kills
//! surviving background descendants, and a pipe-holding descendant cannot keep
//! exec pending beyond the bounded drain. The terminate-FAULT degrade paths
//! and the bounded-drain behavior are covered by the operations-inline
//! fault-seam unit tests (the exact grace value is an internal bound, not
//! regression-pinned); legacy environment-variable fault names are inert in
//! production code.
//!
//! # Platform split
//!
//! The marker-grandchild tests are platform-specific. The Windows variants use
//! a PowerShell grandchild that records its PID and sleeps; the Unix variants
//! background a `sleep`. On a Windows host only the Windows variants compile
//! and run; the Unix variants are `#[cfg(unix)]` and are verified by Linux CI
//! (ubuntu-latest; the six-target compile matrix is owned by 15.5.6). macOS
//! executes the identical Unix process-group path and is covered transitively
//! by the Linux runs.
//!
//! The structural Job-Object-flag and supervision-wiring tests run on every
//! supported host.

#![cfg(any(windows, unix))]

use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use opi_coding_agent::diagnostics::CODE_SANDBOX_DEGRADED;
use opi_coding_agent::tool::{BashOperations, BashRequest, LocalBashOperations};
use tokio_util::sync::CancellationToken;

const ENV_ATTACH_FAIL: &str = "OPI_TEST_L0_ATTACH_FAIL";
const ENV_TERMINATE_FAIL: &str = "OPI_TEST_L0_TERMINATE_FAIL";

static L0_LOCK_CELL: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
fn l0_lock() -> &'static tokio::sync::Mutex<()> {
    L0_LOCK_CELL.get_or_init(|| tokio::sync::Mutex::new(()))
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

fn quick_request(cwd: &Path) -> BashRequest {
    BashRequest {
        command: "echo hi".to_string(),
        cwd: cwd.to_path_buf(),
        timeout: Duration::from_secs(5),
        signal: CancellationToken::new(),
        env: vec![],
        backend: None,
    }
}

/// Scenario `phase15-l0-bash-tree-lifecycle`: in the default (off) sandbox
/// mode, timing out a bash command that has spawned a marker grandchild
/// terminates the whole subprocess tree.
#[cfg(windows)]
#[tokio::test]
async fn bash_l0_kills_process_tree_in_off_mode() {
    let _g = l0_lock().lock().await;
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
        backend: None,
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
        backend: None,
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

#[cfg(windows)]
#[tokio::test]
async fn dropped_exec_future_kills_process_tree() {
    let _g = l0_lock().lock().await;

    let dir = tempfile::tempdir().unwrap();

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
        backend: None,
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
    let dir = tempfile::tempdir().unwrap();
    let pidfile = dir.path().join("pid.txt");
    let ops = LocalBashOperations::new();
    let req = BashRequest {
        command: unix_grandchild_command(&pidfile),
        cwd: dir.path().to_path_buf(),
        timeout: Duration::from_millis(1500),
        signal: CancellationToken::new(),
        env: vec![],
        backend: None,
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
        backend: None,
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
async fn dropped_exec_future_kills_process_tree() {
    let _g = l0_lock().lock().await;
    let dir = tempfile::tempdir().unwrap();

    let pidfile = dir.path().join("pid.txt");
    let ops = LocalBashOperations::new();
    let req = BashRequest {
        command: unix_grandchild_command(&pidfile),
        cwd: dir.path().to_path_buf(),
        timeout: Duration::from_secs(60),
        signal: CancellationToken::new(),
        env: vec![],
        backend: None,
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
}

// =========================================================================
// Phase 16 L0 supervision seam (task 16.2)
// =========================================================================
//
// The policy-neutral `tool::supervision` module owns attach + the
// wait/timeout/cancel race + per-branch termination + bounded drain. These
// production-path tests prove the two NEW L0 behaviors through
// `LocalBashOperations::exec` (which routes through the seam): a clean
// direct-child exit kills surviving background descendants, and a
// pipe-holding descendant cannot keep exec pending beyond the bounded drain.
// The terminate-FAULT degrade path and the bounded-drain behavior are covered
// by the operations-inline fault-seam unit tests; the exact grace value is an
// internal bound, not regression-pinned.

#[cfg(windows)]
fn write_clean_exit_spawner(spawner: &Path, pidfile: &Path) {
    // The spawner backgrounds a long sleeper (the marker grandchild, which
    // inherits the stdout pipe), records its pid, and exits 0.
    let body = format!(
        "$p = Start-Process powershell -ArgumentList '-NoProfile','-Command','Start-Sleep -Seconds 60' -NoNewWindow -PassThru\n$p.Id | Set-Content -NoNewline '{}'\nexit 0\n",
        pidfile.to_string_lossy()
    );
    std::fs::write(spawner, body).unwrap();
}

#[cfg(unix)]
fn unix_clean_exit_command(pidfile: &Path) -> String {
    // Background a long sleeper (inherits the stdout pipe), record its pid, exit.
    format!(
        "sleep 60 & echo $! > '{}'; exit 0",
        pidfile.to_string_lossy()
    )
}

/// Phase 16 L0 acceptance: a clean direct-child exit terminates surviving
/// background descendants. The shell backgrounds a long-lived grandchild and
/// exits 0; the supervision seam must still reap the grandchild.
#[cfg(windows)]
#[tokio::test]
async fn clean_exit_kills_surviving_background_descendants() {
    let _g = l0_lock().lock().await;
    let dir = tempfile::tempdir().unwrap();
    let spawner = dir.path().join("spawn.ps1");
    let pidfile = dir.path().join("pid.txt");
    write_clean_exit_spawner(&spawner, &pidfile);
    let ops = LocalBashOperations::new();
    let req = BashRequest {
        command: format!(
            "powershell -NoProfile -ExecutionPolicy Bypass -File {}",
            spawner.to_string_lossy()
        ),
        cwd: dir.path().to_path_buf(),
        timeout: Duration::from_secs(10),
        signal: CancellationToken::new(),
        env: vec![],
        backend: None,
    };
    let result = ops.exec(req).await.unwrap();
    assert_eq!(
        result.exit_code,
        Some(0),
        "direct child should exit cleanly"
    );
    let pid = read_pid(&pidfile, 3000)
        .await
        .expect("grandchild should record its pid");
    let dead = await_process_dead_windows(pid, 4000).await;
    assert!(
        dead,
        "clean exit must terminate surviving descendant pid {pid}"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn clean_exit_kills_surviving_background_descendants() {
    let _g = l0_lock().lock().await;
    let dir = tempfile::tempdir().unwrap();
    let pidfile = dir.path().join("pid.txt");
    let ops = LocalBashOperations::new();
    let req = BashRequest {
        command: unix_clean_exit_command(&pidfile),
        cwd: dir.path().to_path_buf(),
        timeout: Duration::from_secs(10),
        signal: CancellationToken::new(),
        env: vec![],
        backend: None,
    };
    let result = ops.exec(req).await.unwrap();
    assert_eq!(
        result.exit_code,
        Some(0),
        "direct child should exit cleanly"
    );
    let pid = read_pid(&pidfile, 3000)
        .await
        .expect("grandchild should record its pid");
    let dead = await_process_dead_unix(pid, 4000).await;
    assert!(
        dead,
        "clean exit must terminate surviving descendant pid {pid}"
    );
}

/// Phase 16 L0 acceptance: a descendant holding an output pipe cannot keep
/// exec pending beyond the bounded drain. The grandchild holds the pipe for
/// ~60 s naturally; exec returns in well under that once the seam terminates
/// the tree. The bounded-drain behavior is proven under a terminate fault by
/// the operations-inline fault-seam unit tests; the exact grace value is an
/// internal bound, not pinned to 500 ms.
#[cfg(windows)]
#[tokio::test]
async fn pipe_holding_descendant_drains_within_bounded_grace() {
    let _g = l0_lock().lock().await;
    let dir = tempfile::tempdir().unwrap();
    let spawner = dir.path().join("spawn.ps1");
    let pidfile = dir.path().join("pid.txt");
    write_clean_exit_spawner(&spawner, &pidfile);
    let ops = LocalBashOperations::new();
    let req = BashRequest {
        command: format!(
            "powershell -NoProfile -ExecutionPolicy Bypass -File {}",
            spawner.to_string_lossy()
        ),
        cwd: dir.path().to_path_buf(),
        timeout: Duration::from_secs(10),
        signal: CancellationToken::new(),
        env: vec![],
        backend: None,
    };
    let start = std::time::Instant::now();
    let result = ops.exec(req).await.unwrap();
    let elapsed = start.elapsed();
    assert_eq!(result.exit_code, Some(0));
    assert!(
        elapsed < Duration::from_secs(3),
        "exec must return within the bounded drain, took {elapsed:?}"
    );
    let pid = read_pid(&pidfile, 3000).await.expect("grandchild pid");
    let _ = await_process_dead_windows(pid, 4000).await;
}

#[cfg(unix)]
#[tokio::test]
async fn pipe_holding_descendant_drains_within_bounded_grace() {
    let _g = l0_lock().lock().await;
    let dir = tempfile::tempdir().unwrap();
    let pidfile = dir.path().join("pid.txt");
    let ops = LocalBashOperations::new();
    let req = BashRequest {
        command: unix_clean_exit_command(&pidfile),
        cwd: dir.path().to_path_buf(),
        timeout: Duration::from_secs(10),
        signal: CancellationToken::new(),
        env: vec![],
        backend: None,
    };
    let start = std::time::Instant::now();
    let result = ops.exec(req).await.unwrap();
    let elapsed = start.elapsed();
    assert_eq!(result.exit_code, Some(0));
    assert!(
        elapsed < Duration::from_secs(3),
        "exec must return within the bounded drain, took {elapsed:?}"
    );
    let pid = read_pid(&pidfile, 3000).await.expect("grandchild pid");
    let _ = await_process_dead_unix(pid, 4000).await;
}

#[tokio::test]
async fn legacy_fault_environment_names_are_inert() {
    let _g = l0_lock().lock().await;
    unsafe {
        std::env::set_var(ENV_ATTACH_FAIL, "1");
        std::env::set_var(ENV_TERMINATE_FAIL, "1");
    }
    struct ClearLegacyFaultEnv;
    impl Drop for ClearLegacyFaultEnv {
        fn drop(&mut self) {
            unsafe {
                std::env::remove_var(ENV_ATTACH_FAIL);
                std::env::remove_var(ENV_TERMINATE_FAIL);
            }
        }
    }
    let _clear = ClearLegacyFaultEnv;
    let dir = tempfile::tempdir().unwrap();

    let result = LocalBashOperations::new()
        .exec(quick_request(dir.path()))
        .await
        .unwrap();

    assert_eq!(result.exit_code, Some(0));
    assert!(
        result
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != CODE_SANDBOX_DEGRADED),
        "legacy environment names must not activate L0 fault injection"
    );
}

// =========================================================================
// Adapter process-group contract (cross-platform structural proof)
// =========================================================================

/// Structural pin for the adapter's Unix process-group setup. The cross-host
/// behavioral proof that the retained TreeGuard reaps a marker grandchild
/// lives in `adapter_host.rs::adapter_l0_kills_subprocess_tree_on_host_drop`.
#[test]
fn adapter_process_group_contract() {
    let adapter_src = read_repo_file("crates/opi-coding-agent/src/adapter_host.rs");
    assert!(
        adapter_src.contains("cmd.process_group(0)"),
        "Unix adapter must keep its process_group(0) path intact (DoD: unchanged)"
    );
    assert!(
        adapter_src.contains("tree_attach(pid)"),
        "adapter must attach and retain a TreeGuard after process-group setup"
    );
}

/// Phase 16 structural pin: `LocalBashOperations::exec` delegates to the
/// policy-neutral `supervision` seam rather than an inlined wait/timeout/cancel
/// race. A future refactor cannot silently bypass it without breaking this.
#[test]
fn exec_routes_through_the_policy_neutral_supervision_seam() {
    let operations = read_repo_file("crates/opi-coding-agent/src/tool/operations.rs");
    assert!(
        operations.contains("super::supervision::supervise"),
        "LocalBashOperations::exec must delegate to the policy-neutral supervision seam"
    );
}
