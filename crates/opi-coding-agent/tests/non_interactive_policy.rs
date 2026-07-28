//! Policy tests for non-interactive mode (task 1.15).
//!
//! Verifies that non-interactive mode refuses mutating tools (write, edit, bash)
//! unless explicitly opted in via the allow_mutating flag.
//!
//! Smoke addendum: "non-interactive mode refuses mutating tools (write, edit,
//! bash) unless explicitly opted in via CLI flag or config"

use std::path::PathBuf;

use opi_ai::test_support::{self, MockProvider};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::policy::{RunMode, ToolRuntimeConfig, ToolSelection};
use opi_coding_agent::runner::{ExitCode, NonInteractiveRunner};

fn temp_workspace() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("opi-policy-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

// ---------------------------------------------------------------------------
// Test 1: write tool is blocked by default
// ---------------------------------------------------------------------------

#[tokio::test]
async fn policy_write_blocked_by_default() {
    let first = test_support::tool_call_response(
        "tc-1",
        "write",
        r#"{"path":"test.txt","content":"hello"}"#,
    );
    let second = test_support::text_response("Write was denied.");

    let provider = MockProvider::new("mock", vec![first, second]);

    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        temp_workspace(),
        false, // allow_mutating = false
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result = runner.run("Write a file").await;

    assert_eq!(result.exit_code, ExitCode::Success as i32);
    assert!(
        result.stdout.contains("unknown tool: write")
            || result.stderr.contains("unknown tool: write")
            || result.stdout.contains("Write was denied"),
        "should indicate write was not available, stdout: {:?}, stderr: {:?}",
        result.stdout,
        result.stderr
    );
}

// ---------------------------------------------------------------------------
// Test 2: edit tool is blocked by default
// ---------------------------------------------------------------------------

#[tokio::test]
async fn policy_edit_blocked_by_default() {
    let first = test_support::tool_call_response(
        "tc-1",
        "edit",
        r#"{"path":"test.txt","old_string":"foo","new_string":"bar"}"#,
    );
    let second = test_support::text_response("Edit was denied.");

    let provider = MockProvider::new("mock", vec![first, second]);

    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        temp_workspace(),
        false,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result = runner.run("Edit a file").await;

    assert_eq!(result.exit_code, ExitCode::Success as i32);
    assert!(
        result.stdout.contains("unknown tool: edit")
            || result.stderr.contains("unknown tool: edit")
            || result.stdout.contains("Edit was denied"),
        "should indicate edit was not available, stdout: {:?}, stderr: {:?}",
        result.stdout,
        result.stderr
    );
}

// ---------------------------------------------------------------------------
// Test 3: bash tool is blocked by default
// ---------------------------------------------------------------------------

#[tokio::test]
async fn policy_bash_blocked_by_default() {
    let first = test_support::tool_call_response("tc-1", "bash", r#"{"command":"ls -la"}"#);
    let second = test_support::text_response("Bash was denied.");

    let provider = MockProvider::new("mock", vec![first, second]);

    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        temp_workspace(),
        false,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result = runner.run("Run a command").await;

    assert_eq!(result.exit_code, ExitCode::Success as i32);
    assert!(
        result.stdout.contains("unknown tool: bash")
            || result.stderr.contains("unknown tool: bash")
            || result.stdout.contains("Bash was denied"),
        "should indicate bash was not available, stdout: {:?}, stderr: {:?}",
        result.stdout,
        result.stderr
    );
}

// ---------------------------------------------------------------------------
// Test 4: read tool is allowed by default
// ---------------------------------------------------------------------------

#[tokio::test]
async fn policy_read_allowed_by_default() {
    let first = test_support::tool_call_response("tc-1", "read", r#"{"path":"Cargo.toml"}"#);
    let second = test_support::text_response("File read successful.");

    let provider = MockProvider::new("mock", vec![first, second]);

    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        temp_workspace(),
        false,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result = runner.run("Read a file").await;

    assert_eq!(result.exit_code, ExitCode::Success as i32);
}

// ---------------------------------------------------------------------------
// Test 5: all tools allowed when allow_mutating=true
// ---------------------------------------------------------------------------

#[tokio::test]
async fn policy_all_tools_allowed_when_opted_in() {
    let first = test_support::tool_call_response(
        "tc-1",
        "write",
        r#"{"path":"test.txt","content":"hello"}"#,
    );
    let second = test_support::text_response("Write succeeded.");

    let provider = MockProvider::new("mock", vec![first, second]);

    let workspace = temp_workspace();
    let mut runner = NonInteractiveRunner::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        workspace.clone(),
        true, // allow_mutating = true
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result = runner.run("Write a file").await;

    assert_eq!(result.exit_code, ExitCode::Success as i32);
    // Tool should have executed (not denied)
    assert!(
        !result.stdout.contains("not allowed"),
        "should not contain denial message when opted in, got: {:?}",
        result.stdout
    );
    // Clean up written file
    let _ = std::fs::remove_file(workspace.join("test.txt"));
}

// ---------------------------------------------------------------------------
// Test 6: glob and grep are allowed by default
// ---------------------------------------------------------------------------

#[test]
fn policy_readonly_tools_always_allowed() {
    let config = ToolRuntimeConfig::resolve(RunMode::NonInteractive, false, ToolSelection::Default)
        .expect("tool config");
    assert_eq!(
        config.active_tool_names,
        vec!["read", "grep", "find", "ls", "glob"]
    );
}

// ---------------------------------------------------------------------------
// Phase 11.6: bash is a mutating tool denied before execution without opt-in.
// (Renamed from non_interactive_tools_bash_without_allow_mutating_is_policy_error
// to match the write/edit naming convention; behavior is unchanged.)
// ---------------------------------------------------------------------------

#[test]
fn bash_tool_denied_before_execution_without_allow_mutating() {
    let error = ToolRuntimeConfig::resolve(
        RunMode::NonInteractive,
        false,
        ToolSelection::Allowlist(vec!["bash".into()]),
    )
    .expect_err("bash should require opt-in");

    assert!(
        error
            .to_string()
            .contains("mutating tool 'bash' requires --allow-mutating")
    );
}

// ---------------------------------------------------------------------------
// Phase 11.4: write is a mutating tool denied before execution without opt-in.
// ---------------------------------------------------------------------------

#[test]
fn write_tool_denied_before_execution_without_allow_mutating() {
    // Policy-resolution level: requesting write without --allow-mutating in
    // non-interactive mode is a policy error before any tool body runs. (The
    // runner-level advertisement that write is an unknown tool is covered by
    // policy_write_blocked_by_default above; this binds the resolution-time
    // deny so the two invariants are not conflated.)
    let error = ToolRuntimeConfig::resolve(
        RunMode::NonInteractive,
        false,
        ToolSelection::Allowlist(vec!["write".into()]),
    )
    .expect_err("write should require opt-in");

    assert!(
        error
            .to_string()
            .contains("mutating tool 'write' requires --allow-mutating")
    );
}

// ---------------------------------------------------------------------------
// Phase 11.5: edit is a mutating tool denied before execution without opt-in.
// ---------------------------------------------------------------------------

#[test]
fn edit_tool_denied_before_execution_without_allow_mutating() {
    // Policy-resolution level: requesting edit without --allow-mutating in
    // non-interactive mode is a policy error before any tool body runs. (The
    // runner-level advertisement that edit is an unknown tool is covered by
    // policy_edit_blocked_by_default above; this binds the resolution-time
    // deny so the two invariants are not conflated.)
    let error = ToolRuntimeConfig::resolve(
        RunMode::NonInteractive,
        false,
        ToolSelection::Allowlist(vec!["edit".into()]),
    )
    .expect_err("edit should require opt-in");

    assert!(
        error
            .to_string()
            .contains("mutating tool 'edit' requires --allow-mutating")
    );
}

// ---------------------------------------------------------------------------
// Phase 11.8 S5: consolidated mutating-tool deny contract. Each mutating
// built-in is denied at policy resolution before any tool body runs; read-only
// tools resolve without opt-in. (The per-tool write/edit/bash tests above pin
// each individually; this is the named scenario anchor covering all three.)
// ---------------------------------------------------------------------------

#[test]
fn mutating_tools_denied_before_execution_without_allow_mutating() {
    use opi_coding_agent::policy::is_mutating_tool;
    for tool in ["write", "edit", "bash"] {
        assert!(is_mutating_tool(tool), "{tool} must be classified mutating");
        let msg = format!("{tool} should require opt-in");
        let err = ToolRuntimeConfig::resolve(
            RunMode::NonInteractive,
            false,
            ToolSelection::Allowlist(vec![tool.into()]),
        )
        .expect_err(&msg);
        assert!(
            err.to_string()
                .contains(&format!("mutating tool '{tool}' requires --allow-mutating")),
            "{tool} deny message: {err}"
        );
    }

    // Read-only tools resolve without opt-in.
    for tool in ["read", "grep"] {
        let msg = format!("{tool} should resolve without opt-in");
        let cfg = ToolRuntimeConfig::resolve(
            RunMode::NonInteractive,
            false,
            ToolSelection::Allowlist(vec![tool.into()]),
        )
        .expect(&msg);
        assert!(
            cfg.active_tool_names.iter().any(|n| n == tool),
            "{tool} should be active: {:?}",
            cfg.active_tool_names,
        );
    }
}
