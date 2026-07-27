//! Safety hooks tests for task 3.8.
//!
//! Validates hook-mediated confirm/deny for mutating tools in both interactive
//! and non-interactive modes, JSON mode policy events, and session audit records.

use std::fs;
use std::sync::Mutex;

use opi_agent::hooks::{AgentHooks, BeforeToolCallContext, BeforeToolCallResult};
use opi_ai::test_support::{MockProvider, text_response, tool_call_response};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::harness::{CodingHarness, InteractiveCodingHooks};
use opi_coding_agent::policy::ToolSelection;
use opi_coding_agent::runner::NonInteractiveRunner;

// Session dir tests must serialize (OPI_SESSIONS_DIR env mutation).
static SESSION_LOCK: Mutex<()> = Mutex::new(());

fn session_lock() -> std::sync::MutexGuard<'static, ()> {
    match SESSION_LOCK.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

async fn with_session_dir<F, Fut, R>(f: F) -> R
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = R>,
{
    let dir = tempfile::tempdir().expect("session temp dir");
    // SAFETY: test-only env var mutation, guarded by SESSION_LOCK.
    unsafe {
        std::env::set_var("OPI_SESSIONS_DIR", dir.path());
    }
    let result = f().await;
    // SAFETY: same as above.
    unsafe {
        std::env::remove_var("OPI_SESSIONS_DIR");
    }
    result
}

// --- Helpers ---

fn create_temp_workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    fs::create_dir_all(dir.path().join(".git")).expect("failed to create .git");
    dir
}

fn make_before_ctx(tool_name: &str) -> BeforeToolCallContext {
    BeforeToolCallContext {
        tool_call_id: "test-call-id".into(),
        tool_name: tool_name.into(),
        args: serde_json::json!({}),
        messages: vec![],
    }
}

// --- InteractiveCodingHooks unit tests ---

#[tokio::test]
async fn interactive_allows_read_only_tools() {
    let hooks = InteractiveCodingHooks::new(false);
    for tool in &["read", "glob", "grep"] {
        let result = hooks.before_tool_call(make_before_ctx(tool)).await;
        assert!(
            matches!(result, BeforeToolCallResult::Allow),
            "read-only tool '{tool}' should be allowed when mutating denied"
        );
    }
}

#[tokio::test]
async fn interactive_allows_mutating_tools() {
    let hooks = InteractiveCodingHooks::new(false);
    for tool in &["write", "edit", "bash"] {
        let result = hooks.before_tool_call(make_before_ctx(tool)).await;
        assert!(
            matches!(result, BeforeToolCallResult::Allow),
            "interactive hook should pass through mutating tool '{tool}'"
        );
    }
}

#[tokio::test]
async fn interactive_allows_all_when_mutating_allowed() {
    let hooks = InteractiveCodingHooks::new(true);
    for tool in &["read", "write", "edit", "bash", "glob", "grep"] {
        let result = hooks.before_tool_call(make_before_ctx(tool)).await;
        assert!(
            matches!(result, BeforeToolCallResult::Allow),
            "tool '{tool}' should be allowed when allow_mutating=true"
        );
    }
}

// --- Non-interactive hook tests via runner ---

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn non_interactive_denies_mutating_by_default() {
    let _lock = session_lock();
    with_session_dir(|| async {
        let workspace = create_temp_workspace();
        // Mock returns a tool call for "write", then a text response
        let mock = MockProvider::new(
            "mock",
            vec![
                tool_call_response("tc-1", "write", r#"{"path":"test.txt","content":"hi"}"#),
                text_response("done"),
            ],
        );

        let mut runner = NonInteractiveRunner::new(
            Box::new(mock),
            "mock:mock-model".into(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            false, // allow_mutating = false
            None,
            vec![],
            opi_coding_agent::project_trust::TrustDecision::Trusted,
        );

        let result = runner.run("test prompt").await;
        // The unavailable tool result is sent back to the LLM, which then
        // produces the "done" text response.
        assert_eq!(
            result.exit_code, 0,
            "Should succeed after unavailable-tool follow-up"
        );
        assert!(
            result.stdout.contains("done"),
            "Should contain the follow-up text response"
        );
    })
    .await
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn non_interactive_allows_mutating_when_flag_set() {
    let _lock = session_lock();
    with_session_dir(|| async {
        let workspace = create_temp_workspace();
        let mock = MockProvider::new(
            "mock",
            vec![
                tool_call_response("tc-1", "write", r#"{"path":"test.txt","content":"hi"}"#),
                text_response("done"),
            ],
        );

        let mut runner = NonInteractiveRunner::new(
            Box::new(mock),
            "mock:mock-model".into(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            true, // allow_mutating = true
            None,
            vec![],
            opi_coding_agent::project_trust::TrustDecision::Trusted,
        );

        let result = runner.run("test prompt").await;
        assert_eq!(result.exit_code, 0);
    })
    .await
}

// --- E2E: tool denial in JSON mode produces denial event ---

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn e2e_json_mode_tool_denial() {
    let _lock = session_lock();
    with_session_dir(|| async {
        let workspace = create_temp_workspace();
        let mock = MockProvider::new(
            "mock",
            vec![
                tool_call_response("tc-1", "bash", r#"{"command":"echo hi"}"#),
                text_response("done"),
            ],
        );

        let mut runner = NonInteractiveRunner::new(
            Box::new(mock),
            "mock:mock-model".into(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            false, // allow_mutating = false -> bash denied
            None,
            vec![],
            opi_coding_agent::project_trust::TrustDecision::Trusted,
        );

        let result = runner.run_json("test prompt").await;

        let output = result.stdout;
        // The unavailable tool should produce a tool_result event with is_error=true.
        assert!(
            output.contains("is_error") || output.contains("unknown tool: bash"),
            "JSON output should contain unavailable tool information: {output}"
        );
    })
    .await
}

// --- Session audit: tool denial recorded in session entries ---

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn session_audit_tool_denial() {
    let _lock = session_lock();
    with_session_dir(|| async {
        let workspace = create_temp_workspace();
        let mock = MockProvider::new(
            "mock",
            vec![
                tool_call_response("tc-1", "write", r#"{"path":"test.txt","content":"hi"}"#),
                text_response("done"),
            ],
        );

        let mut runner = NonInteractiveRunner::new(
            Box::new(mock),
            "mock:mock-model".into(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            false, // deny mutating
            None,
            vec![],
            opi_coding_agent::project_trust::TrustDecision::Trusted,
        );

        let result = runner.run("test prompt").await;
        assert_eq!(
            result.exit_code, 0,
            "Should succeed after unavailable-tool follow-up"
        );

        // The session should exist and contain the unavailable tool result.
        let session = runner.session().expect("session should exist");
        let session_path = session.session_path();

        // Read entries from the session file
        let (_header, entries) = opi_agent::session::SessionReader::read_all(session_path)
            .expect("session should be readable");

        let has_unavailable_tool = entries.iter().any(|e| {
            let json = serde_json::to_string(e).unwrap_or_default();
            json.contains("unknown tool: write")
        });
        assert!(
            has_unavailable_tool,
            "Session entries should contain unavailable tool audit record"
        );
    })
    .await
}

// --- Tool selection + hook interaction: allowlisted mutating tools are visible interactively ---

#[tokio::test]
async fn tool_selection_allowlist_includes_mutating_tool_interactively() {
    let workspace = create_temp_workspace();
    // Interactive policy accepts mutating allowlists without an extra hook denial.
    let mock = MockProvider::new(
        "mock",
        vec![
            tool_call_response("tc-1", "write", r#"{"path":"test.txt","content":"hi"}"#),
            text_response("done"),
        ],
    );

    let harness = CodingHarness::new_with_selection(
        Box::new(mock),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        ToolSelection::Allowlist(vec!["write".into(), "read".into()]),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    // The system prompt should contain write (allowlisted)
    let system = harness.system_prompt();
    assert!(
        system.contains("- write:"),
        "Allowlist should include write in tools"
    );
    assert!(
        !system.contains("- bash:"),
        "Allowlist should exclude bash from tools"
    );
}
