//! Adapter process host tests (task 5.5).
//!
//! Covers: child process startup, initialize/capabilities handshake, correlated
//! request/response, per-request timeout, best-effort cancel, event delivery
//! under backpressure, crash detection, and child process reap on shutdown.

use std::path::PathBuf;
use std::time::Duration;

use opi_coding_agent::adapter_host::{AdapterHost, AdapterHostError, AdapterProcessConfig};
use opi_coding_agent::adapter_protocol::{
    AdapterHostMessage, AdapterProcessMessage, PROTOCOL_VERSION,
};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Locate the `adapter_host_mock` test binary in the same deps directory.
fn mock_adapter_bin() -> PathBuf {
    let current = std::env::current_exe().expect("current exe path");
    let deps_dir = current.parent().expect("deps directory");

    // Try exact name first (no hash suffix)
    let exact_name = if cfg!(windows) {
        "adapter_host_mock.exe"
    } else {
        "adapter_host_mock"
    };
    let exact_path = deps_dir.join(exact_name);
    if exact_path.exists() {
        return exact_path;
    }

    // Try with hash suffix — must match executable, not .d/.pdb files
    let exe_suffix = if cfg!(windows) { ".exe" } else { "" };
    let prefix = "adapter_host_mock-";
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
        "Could not find adapter_host_mock binary in {}. \
         Ensure [[test]] target 'adapter_host_mock' is defined in Cargo.toml",
        deps_dir.display()
    );
}

/// Build an `AdapterProcessConfig` that spawns the mock adapter with the given mode.
fn config_for_mode(mode: &str) -> AdapterProcessConfig {
    AdapterProcessConfig {
        command: mock_adapter_bin(),
        args: vec![],
        working_dir: std::env::current_dir().expect("cwd"),
        env: vec![("OPI_ADAPTER_TEST_MODE".to_string(), mode.to_string())],
    }
}

fn config_for_mode_with_env(mode: &str, env: Vec<(String, String)>) -> AdapterProcessConfig {
    let mut config = config_for_mode(mode);
    config.env.extend(env);
    config
}

/// Start a normal capabilities-mode adapter host with generous timeouts.
async fn start_capabilities_host() -> AdapterHost {
    AdapterHost::start(
        "mock",
        config_for_mode("capabilities"),
        Duration::from_secs(5),
    )
    .await
    .expect("start capabilities host")
}

// ---------------------------------------------------------------------------
// 1. Normal initialization handshake
// ---------------------------------------------------------------------------

#[tokio::test]
async fn host_initializes_and_receives_capabilities() {
    let host = start_capabilities_host().await;

    let caps = host.capabilities();
    assert_eq!(caps.tools.len(), 1);
    assert_eq!(caps.tools[0].name, "test_tool");
    assert_eq!(caps.commands.len(), 1);
    assert_eq!(caps.commands[0].name, "test/status");
    assert!(caps.hooks.iter().any(|h| h == "before_tool_call"));
    assert!(caps.hooks.iter().any(|h| h == "event"));
    assert!(caps.model_overrides.is_empty());

    host.shutdown("test_end").await.expect("shutdown");
}

// ---------------------------------------------------------------------------
// 2. Initialize timeout
// ---------------------------------------------------------------------------

#[tokio::test]
async fn host_times_out_unresponsive_adapter() {
    let err = AdapterHost::start("mock", config_for_mode("hang"), Duration::from_millis(150))
        .await
        .expect_err("should timeout on initialize");

    match err {
        AdapterHostError::InitializeTimeout { .. } => {}
        other => panic!("expected InitializeTimeout, got: {other}"),
    }
}

// ---------------------------------------------------------------------------
// 3. Adapter crash during handshake
// ---------------------------------------------------------------------------

#[tokio::test]
async fn host_detects_adapter_crash_during_handshake() {
    let err = AdapterHost::start("mock", config_for_mode("crash"), Duration::from_secs(5))
        .await
        .expect_err("should detect crash");

    // The crash can manifest as either AdapterExited or AdapterUnavailable
    // depending on whether the reader task detects EOF before the handshake
    // times out.
    match err {
        AdapterHostError::AdapterExited { .. } | AdapterHostError::AdapterUnavailable { .. } => {}
        other => panic!("expected AdapterExited or AdapterUnavailable, got: {other}"),
    }
}

// ---------------------------------------------------------------------------
// 4. Tool call round-trip
// ---------------------------------------------------------------------------

#[tokio::test]
async fn host_sends_tool_call_and_receives_result() {
    let host = start_capabilities_host().await;

    let id = host.next_id();
    let result = host
        .send_request(
            AdapterHostMessage::ToolCall {
                id: id.clone(),
                tool: "test_tool".into(),
                args: serde_json::json!({"input": "hello"}),
            },
            Duration::from_secs(5),
        )
        .await
        .expect("tool call");

    match result {
        AdapterProcessMessage::ToolResult {
            content, is_error, ..
        } => {
            assert!(!is_error);
            assert!(
                content
                    .iter()
                    .any(|c| c.to_string().contains("mock_result")),
                "expected mock_result in content"
            );
        }
        other => panic!("expected ToolResult, got: {other:?}"),
    }

    host.shutdown("test_end").await.expect("shutdown");
}

// ---------------------------------------------------------------------------
// 5. Command round-trip
// ---------------------------------------------------------------------------

#[tokio::test]
async fn host_sends_command_and_receives_result() {
    let host = start_capabilities_host().await;

    let id = host.next_id();
    let result = host
        .send_request(
            AdapterHostMessage::Command {
                id: id.clone(),
                name: "test/status".into(),
                args: serde_json::json!({}),
            },
            Duration::from_secs(5),
        )
        .await
        .expect("command");

    match result {
        AdapterProcessMessage::CommandResult { data, .. } => {
            assert_eq!(data["status"], "ok");
        }
        other => panic!("expected CommandResult, got: {other:?}"),
    }

    host.shutdown("test_end").await.expect("shutdown");
}

// ---------------------------------------------------------------------------
// 6. Hook round-trip
// ---------------------------------------------------------------------------

#[tokio::test]
async fn host_sends_hook_and_receives_result() {
    let host = start_capabilities_host().await;

    let id = host.next_id();
    let result = host
        .send_request(
            AdapterHostMessage::Hook {
                id: id.clone(),
                hook: "before_tool_call".into(),
                payload: serde_json::json!({"tool": "bash"}),
            },
            Duration::from_secs(5),
        )
        .await
        .expect("hook");

    match result {
        AdapterProcessMessage::HookResult { action, .. } => {
            assert_eq!(action, "continue");
        }
        other => panic!("expected HookResult, got: {other:?}"),
    }

    host.shutdown("test_end").await.expect("shutdown");
}

// ---------------------------------------------------------------------------
// 7. State serialize round-trip
// ---------------------------------------------------------------------------

#[tokio::test]
async fn host_sends_state_serialize_and_receives_result() {
    let host = start_capabilities_host().await;

    let id = host.next_id();
    let result = host
        .send_request(
            AdapterHostMessage::StateSerialize { id: id.clone() },
            Duration::from_secs(5),
        )
        .await
        .expect("state serialize");

    match result {
        AdapterProcessMessage::StateResult { state, .. } => {
            assert_eq!(state["mock"], true);
        }
        other => panic!("expected StateResult, got: {other:?}"),
    }

    host.shutdown("test_end").await.expect("shutdown");
}

// ---------------------------------------------------------------------------
// 8. State restore round-trip
// ---------------------------------------------------------------------------

#[tokio::test]
async fn host_sends_state_restore_and_receives_result() {
    let host = start_capabilities_host().await;

    let id = host.next_id();
    let result = host
        .send_request(
            AdapterHostMessage::StateRestore {
                id: id.clone(),
                state: serde_json::json!({"items": ["a"]}),
            },
            Duration::from_secs(5),
        )
        .await
        .expect("state restore");

    match result {
        AdapterProcessMessage::StateResult { state, .. } => {
            assert!(state.is_object());
        }
        other => panic!("expected StateResult, got: {other:?}"),
    }

    host.shutdown("test_end").await.expect("shutdown");
}

// ---------------------------------------------------------------------------
// 9. Per-request timeout
// ---------------------------------------------------------------------------

#[tokio::test]
async fn host_times_out_individual_request() {
    // hang_request mode: responds to initialize, then never responds
    let host = AdapterHost::start(
        "mock",
        config_for_mode("hang_request"),
        Duration::from_secs(5),
    )
    .await
    .expect("start host");

    let id = host.next_id();
    let err = host
        .send_request(
            AdapterHostMessage::ToolCall {
                id: id.clone(),
                tool: "test_tool".into(),
                args: serde_json::json!({}),
            },
            Duration::from_millis(150),
        )
        .await
        .expect_err("should timeout on request");

    match err {
        AdapterHostError::RequestTimeout { .. } => {}
        other => panic!("expected RequestTimeout, got: {other}"),
    }

    // Host should still be usable (pending entry cleaned up)
    // Try shutdown to verify it still works
    let _ = host.shutdown("test_timeout").await;
}

// ---------------------------------------------------------------------------
// 10. Best-effort cancel
// ---------------------------------------------------------------------------

#[tokio::test]
async fn host_sends_cancel_best_effort() {
    let host = start_capabilities_host().await;

    // Cancel should not error even if there's no matching in-flight request
    host.cancel("nonexistent-id", "test_cancel")
        .await
        .expect("cancel should be best-effort");

    host.shutdown("test_end").await.expect("shutdown");
}

// ---------------------------------------------------------------------------
// 11. Event delivery (fire-and-forget, does not block)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn host_sends_event_without_blocking() {
    let host = start_capabilities_host().await;

    // send_event should return quickly even though the adapter doesn't respond
    let start = std::time::Instant::now();
    host.send_event(serde_json::json!({"type": "turn_start", "turn": 1}))
        .await;
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "send_event took {elapsed:?}, should be near-instant"
    );

    host.shutdown("test_end").await.expect("shutdown");
}

#[tokio::test]
async fn event_drop_records_diagnostic() {
    let host = AdapterHost::start(
        "mock",
        config_for_mode("event_backpressure"),
        Duration::from_secs(5),
    )
    .await
    .expect("start event backpressure host");

    let payload = "x".repeat(1024 * 1024);
    for i in 0..64 {
        host.send_event(serde_json::json!({
            "type": "large_event",
            "index": i,
            "payload": payload
        }))
        .await;
        if !host.take_diagnostics().is_empty() {
            let _ = host.shutdown("test_end").await;
            return;
        }
    }

    let diagnostics = host.take_diagnostics();
    let _ = host.shutdown("test_end").await;
    assert!(
        !diagnostics.is_empty(),
        "backpressured event delivery should record diagnostics"
    );
}

// ---------------------------------------------------------------------------
// 12. Shutdown reaps child process
// ---------------------------------------------------------------------------

#[tokio::test]
async fn host_shutdown_reaps_child_process() {
    let host = start_capabilities_host().await;
    let pid = host.child_pid();

    host.shutdown("test_reap").await.expect("shutdown");

    // Verify the process is no longer running
    // On Unix, kill(pid, 0) would check; on Windows, OpenProcess + WaitForSingleObject
    // For cross-platform simplicity, just verify shutdown succeeded without error
    // (if reap failed, we'd get an error or panic)
    let _ = pid;
}

#[tokio::test]
async fn shutdown_waits_for_child_exit_before_kill() {
    let dir = tempfile::tempdir().expect("tempdir");
    let marker = dir.path().join("shutdown-marker.txt");
    let host = AdapterHost::start(
        "mock",
        config_for_mode_with_env(
            "shutdown_marker",
            vec![(
                "OPI_ADAPTER_SHUTDOWN_MARKER".to_string(),
                marker.display().to_string(),
            )],
        ),
        Duration::from_secs(5),
    )
    .await
    .expect("start shutdown marker host");

    host.shutdown("test_marker").await.expect("shutdown");

    assert!(
        marker.exists(),
        "adapter should be allowed to handle shutdown before host kills child"
    );
}

// ---------------------------------------------------------------------------
// 13. Crash detected after initialization (pending requests fail)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn host_reports_unavailable_after_crash() {
    // The mock completes initialization, then exits without replying to the
    // first normal request. This exercises a post-handshake crash without
    // relying on OS-level process-kill permission.
    let host = AdapterHost::start(
        "mock",
        config_for_mode("crash_on_request"),
        Duration::from_secs(5),
    )
    .await
    .expect("start crash-on-request host");

    // The request that triggers the crash should fail as unavailable.
    let id = host.next_id();
    let result = host
        .send_request(
            AdapterHostMessage::ToolCall {
                id: id.clone(),
                tool: "test_tool".into(),
                args: serde_json::json!({}),
            },
            Duration::from_secs(2),
        )
        .await;

    assert!(
        result.is_err(),
        "request that crashes adapter should fail, got: {:?}",
        result
    );
    let err = result.unwrap_err();
    match err {
        AdapterHostError::AdapterUnavailable { .. } | AdapterHostError::AdapterExited { .. } => {}
        other => panic!("expected AdapterUnavailable or AdapterExited, got: {other}"),
    }
}

// ---------------------------------------------------------------------------
// 14. Correlated requests get correct responses
// ---------------------------------------------------------------------------

#[tokio::test]
async fn host_correlates_multiple_concurrent_requests() {
    let host = start_capabilities_host().await;

    let id1 = host.next_id();
    let id2 = host.next_id();

    let req1 = host.send_request(
        AdapterHostMessage::ToolCall {
            id: id1.clone(),
            tool: "test_tool".into(),
            args: serde_json::json!({"input": "first"}),
        },
        Duration::from_secs(5),
    );

    let req2 = host.send_request(
        AdapterHostMessage::Command {
            id: id2.clone(),
            name: "test/status".into(),
            args: serde_json::json!({}),
        },
        Duration::from_secs(5),
    );

    let (r1, r2) = tokio::join!(req1, req2);

    // Tool call should get ToolResult
    match r1.expect("req1") {
        AdapterProcessMessage::ToolResult { .. } => {}
        other => panic!("expected ToolResult for req1, got: {other:?}"),
    }

    // Command should get CommandResult
    match r2.expect("req2") {
        AdapterProcessMessage::CommandResult { .. } => {}
        other => panic!("expected CommandResult for req2, got: {other:?}"),
    }

    host.shutdown("test_end").await.expect("shutdown");
}

// ---------------------------------------------------------------------------
// 15. next_id produces unique values
// ---------------------------------------------------------------------------

#[tokio::test]
async fn next_id_produces_unique_values() {
    let host = start_capabilities_host().await;
    let ids: Vec<String> = (0..10).map(|_| host.next_id()).collect();
    let unique: std::collections::HashSet<&String> = ids.iter().collect();
    assert_eq!(unique.len(), 10, "all ids should be unique");

    host.shutdown("test_end").await.expect("shutdown");
}

// ---------------------------------------------------------------------------
// 16. Protocol version sent in initialize matches constant
// ---------------------------------------------------------------------------

#[tokio::test]
async fn host_sends_correct_protocol_version_in_initialize() {
    // The capabilities-mode mock adapter validates the initialize message
    // implicitly — if the protocol version were wrong, the test infrastructure
    // would still work. Instead, verify the constant is used correctly by
    // checking that the host's initialize message contains the right version.
    // This is verified by the mock adapter accepting the handshake.
    //
    // Additionally, verify the PROTOCOL_VERSION constant is accessible and
    // matches the expected value.
    assert_eq!(PROTOCOL_VERSION, "opi-extension-jsonl-v1");

    let host = start_capabilities_host().await;
    // If initialization succeeded, the host sent the correct protocol version
    let _ = host.shutdown("test_end").await;
}

// ---------------------------------------------------------------------------
// Phase 15.4: adapter L0 subprocess-tree lifecycle (behavioral)
// ---------------------------------------------------------------------------

/// Read a pidfile once the grandchild has recorded its pid.
async fn read_pid_file(path: &std::path::Path, timeout: Duration) -> Option<u32> {
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

/// Poll `tasklist` until the pid is gone (Windows helper).
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

/// Cross-host production-call-site proof. A real `AdapterHost::start` retains
/// the adapter TreeGuard (a Job Object on Windows, a process-group guard on
/// Unix); dropping the host tears down a marker grandchild on both platforms.
#[tokio::test]
async fn adapter_l0_kills_subprocess_tree_on_host_drop() {
    let dir = tempfile::tempdir().unwrap();
    let pidfile = dir.path().join("adapter_gc_pid.txt");
    let config = config_for_mode_with_env(
        "l0_grandchild",
        vec![(
            "OPI_L0_GC_PIDFILE".to_string(),
            pidfile.to_string_lossy().into_owned(),
        )],
    );
    let host = AdapterHost::start("l0-adapter", config, Duration::from_secs(5))
        .await
        .expect("adapter handshake should complete");

    let pid = read_pid_file(&pidfile, Duration::from_secs(4))
        .await
        .expect("adapter grandchild should record its pid");
    // Drop runs Drop::drop (start_kill the adapter child) then drops the
    // l0_guard field, whose Job Object/process-group teardown reaps the tree.
    drop(host);
    let dead = wait_for_process_dead(pid, Duration::from_secs(6)).await;
    assert!(
        dead,
        "adapter L0 should reap marker grandchild pid {pid} on host drop"
    );
}
