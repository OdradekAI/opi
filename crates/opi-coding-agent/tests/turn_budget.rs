//! Product-owned turn budgets do not impose a default Agent limit.

mod common;

use std::sync::{Arc, Mutex};

use opi_ai::provider::Request;
use opi_ai::test_support::{MockProvider, text_response, tool_call_response};
use opi_coding_agent::config::load_config_file;
use opi_coding_agent::policy::ToolSelection;
use opi_coding_agent::project_trust::TrustDecision;
use opi_coding_agent::runner::{ExitCode, NonInteractiveRunner};

fn runner(
    workspace: &std::path::Path,
    budget: Option<u32>,
    tool_turns: usize,
) -> (NonInteractiveRunner, Arc<Mutex<Vec<Request>>>) {
    std::fs::write(workspace.join("input.txt"), "fixture").unwrap();
    let path = workspace.join("config.toml");
    let budget = budget
        .map(|limit| format!("[defaults]\nmax_iterations = {limit}\n"))
        .unwrap_or_default();
    std::fs::write(&path, format!("{budget}[compaction]\nenabled = false\n")).unwrap();
    let config = load_config_file(&path).unwrap();
    let mut responses = (0..tool_turns)
        .map(|turn| tool_call_response(&format!("read-{turn}"), "read", r#"{"path":"input.txt"}"#))
        .collect::<Vec<_>>();
    responses.extend([text_response("complete"), text_response("next prompt")]);
    let provider = MockProvider::new("mock", responses);
    let calls = provider.call_log_handle();
    let runner = NonInteractiveRunner::new_with_resume(
        Box::new(provider),
        "mock:mock-model".into(),
        config,
        workspace.to_path_buf(),
        false,
        None,
        vec![],
        None,
        ToolSelection::Allowlist(vec!["read".into()]),
        TrustDecision::Trusted,
    )
    .unwrap();
    (runner, calls)
}

#[tokio::test]
async fn omitted_budget_completes_more_than_fifty_provider_turns() {
    let _guard = common::empty_user_config_dir();
    let workspace = tempfile::tempdir().unwrap();
    let (mut runner, calls) = runner(workspace.path(), None, 51);
    let result = runner.run_json("read repeatedly, then finish").await;
    assert_eq!(
        result.exit_code,
        ExitCode::Success as i32,
        "{}",
        result.stderr
    );
    assert_eq!(calls.lock().unwrap().len(), 52);
}

#[tokio::test]
async fn explicit_budget_fails_before_an_extra_request_and_resets_next_run() {
    let _guard = common::empty_user_config_dir();
    let workspace = tempfile::tempdir().unwrap();
    let (mut runner, calls) = runner(workspace.path(), Some(2), 2);
    let failed = runner.run_json("read repeatedly").await;
    assert_eq!(failed.exit_code, ExitCode::RuntimeFailure as i32);
    assert!(failed.stderr.contains("max turns exceeded (2)"));
    assert_eq!(calls.lock().unwrap().len(), 2);
    let succeeded = runner.run_json("finish this new prompt").await;
    assert_eq!(
        succeeded.exit_code,
        ExitCode::Success as i32,
        "{}",
        succeeded.stderr
    );
    assert_eq!(calls.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn natural_completion_at_explicit_budget_is_success() {
    let _guard = common::empty_user_config_dir();
    let workspace = tempfile::tempdir().unwrap();
    let (mut runner, calls) = runner(workspace.path(), Some(2), 1);
    let result = runner.run_json("read then finish").await;
    assert_eq!(
        result.exit_code,
        ExitCode::Success as i32,
        "{}",
        result.stderr
    );
    assert_eq!(calls.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn zero_budget_fails_without_a_provider_request() {
    let _guard = common::empty_user_config_dir();
    let workspace = tempfile::tempdir().unwrap();
    let (mut runner, calls) = runner(workspace.path(), Some(0), 0);
    let result = runner.run_json("must not dispatch").await;
    assert_eq!(calls.lock().unwrap().len(), 0);
    assert_eq!(result.exit_code, ExitCode::RuntimeFailure as i32);
    assert!(result.stderr.contains("max turns exceeded (0)"));
}
