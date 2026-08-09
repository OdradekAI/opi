use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use opi_agent::tool::Tool;
use opi_coding_agent::diagnostics::CODE_PROCESS_TREE_DEGRADED;
use opi_coding_agent::tool::{
    BashOpError, BashOperationContext, BashOperations, BashRequest, BashResult, BashTool,
    ToolDiagnostic as BackendDiagnostic,
};
use serde_json::json;
use tokio_util::sync::CancellationToken;

const INPUT_CANARY: &str = r#"SECRET_ENV=sk-proj-signal-canary C:\private\signal-canary"#;

struct DiagnosticBashOperations {
    result: Result<BashResult, BashOpError>,
}

impl BashOperations for DiagnosticBashOperations {
    fn exec(
        &self,
        _request: BashRequest,
    ) -> Pin<Box<dyn Future<Output = Result<BashResult, BashOpError>> + Send>> {
        let result = self.result.clone();
        Box::pin(async move { result })
    }
}

fn operation_context(exit_code: Option<i32>, signal: Option<i32>) -> BashOperationContext {
    BashOperationContext::local(exit_code, signal)
}

fn degraded_backend_diagnostic() -> BackendDiagnostic {
    BackendDiagnostic {
        code: CODE_PROCESS_TREE_DEGRADED.to_string(),
        message: "subprocess tree lifecycle degraded".to_string(),
        details: Some(json!({
            "layer": "test-tree",
            "reason": "attach unavailable",
        })),
    }
}

async fn execute_with(result: BashResult) -> opi_agent::tool::ToolResult {
    BashTool::new_with_ops(
        PathBuf::from("."),
        Arc::new(DiagnosticBashOperations { result: Ok(result) }),
    )
    .execute(
        "diagnostic-test",
        json!({ "command": INPUT_CANARY }),
        CancellationToken::new(),
        None,
    )
    .await
    .expect("tool execution")
}

async fn execute_error(error: BashOpError) -> opi_agent::tool::ToolResult {
    BashTool::new_with_ops(
        PathBuf::from("."),
        Arc::new(DiagnosticBashOperations { result: Err(error) }),
    )
    .execute(
        "diagnostic-error-test",
        json!({ "command": "ignored" }),
        CancellationToken::new(),
        None,
    )
    .await
    .expect("tool execution")
}

fn public_result_json(result: &opi_agent::tool::ToolResult) -> String {
    json!({
        "content": &result.content,
        "details": &result.details,
        "diagnostics": &result.diagnostics,
    })
    .to_string()
}

#[tokio::test]
async fn bash_tool_preserves_backend_diagnostic_on_success() {
    let result = execute_with(BashResult {
        stdout: b"ok".to_vec(),
        stderr: Vec::new(),
        context: operation_context(Some(0), None),
        diagnostics: vec![degraded_backend_diagnostic()],
    })
    .await;

    assert!(!result.is_error);
    assert_eq!(result.diagnostics.len(), 1);
    let diagnostic = &result.diagnostics[0];
    assert_eq!(diagnostic.code, CODE_PROCESS_TREE_DEGRADED);
    assert_eq!(diagnostic.context["layer"], "test-tree");
    assert_eq!(diagnostic.context["reason"], "attach unavailable");
}

#[tokio::test]
async fn bash_tool_preserves_backend_diagnostic_on_nonzero_exit() {
    let result = execute_with(BashResult {
        stdout: Vec::new(),
        stderr: b"failed".to_vec(),
        context: operation_context(Some(7), None),
        diagnostics: vec![degraded_backend_diagnostic()],
    })
    .await;

    assert!(result.is_error);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == CODE_PROCESS_TREE_DEGRADED)
    );
}

#[tokio::test]
async fn bash_tool_preserves_backend_diagnostic_on_backend_error() {
    let result = execute_error(BashOpError::BackendFailure {
        source: Box::new(BashOpError::Other {
            message: "backend unavailable".to_string(),
        }),
        diagnostics: vec![degraded_backend_diagnostic()],
    })
    .await;

    assert!(result.is_error);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, CODE_PROCESS_TREE_DEGRADED);
    assert_eq!(result.diagnostics[0].context["layer"], "test-tree");
}

#[tokio::test]
async fn bash_tool_renders_signal_specific_public_result() {
    let result = execute_with(BashResult {
        stdout: Vec::new(),
        stderr: Vec::new(),
        context: operation_context(None, Some(9)),
        diagnostics: Vec::new(),
    })
    .await;

    assert!(result.is_error);
    assert_eq!(result.details.as_ref().unwrap()["signal"], 9);
    assert_eq!(
        result.diagnostics[0].message,
        "command terminated by signal 9"
    );
    assert_eq!(result.diagnostics[0].context["signal"], 9);
    match &result.content[0] {
        opi_ai::message::OutputContent::Text { text } => {
            assert_eq!(text, "command terminated by signal 9")
        }
        other => panic!("expected text output, got {other:?}"),
    }
    assert!(!public_result_json(&result).contains(INPUT_CANARY));
}

#[tokio::test]
async fn bash_tool_treats_zero_exit_with_signal_as_signal_failure() {
    let result = execute_with(BashResult {
        stdout: Vec::new(),
        stderr: Vec::new(),
        context: operation_context(Some(0), Some(9)),
        diagnostics: Vec::new(),
    })
    .await;

    assert!(result.is_error, "a present signal cannot be successful");
    assert_eq!(result.details.as_ref().unwrap()["exit_code"], 0);
    assert_eq!(result.details.as_ref().unwrap()["signal"], 9);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        result.diagnostics[0].message,
        "command terminated by signal 9"
    );
    assert_eq!(result.diagnostics[0].context["exit_code"], 0);
    assert_eq!(result.diagnostics[0].context["signal"], 9);
}

#[cfg(unix)]
#[tokio::test]
async fn direct_local_signal_is_public_and_signal_specific() {
    let workspace = tempfile::tempdir().unwrap();
    let result = BashTool::new(workspace.path().to_path_buf())
        .execute(
            "direct-signal",
            json!({ "command": format!(": # {INPUT_CANARY}\nkill -TERM $$") }),
            CancellationToken::new(),
            None,
        )
        .await
        .expect("tool execution");

    assert!(result.is_error);
    assert_eq!(result.details.as_ref().unwrap()["signal"], 15);
    assert_eq!(
        result.diagnostics[0].message,
        "command terminated by signal 15"
    );
    assert_eq!(result.diagnostics[0].context["signal"], 15);
    match &result.content[0] {
        opi_ai::message::OutputContent::Text { text } => {
            assert_eq!(text, "command terminated by signal 15")
        }
        other => panic!("expected text output, got {other:?}"),
    }
    assert!(!public_result_json(&result).contains(INPUT_CANARY));
}
