use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use opi_agent::tool::Tool;
use opi_coding_agent::diagnostics::CODE_SANDBOX_DEGRADED;
use opi_coding_agent::tool::{
    BashOpError, BashOperations, BashRequest, BashResult, BashTool,
    LOCAL_BASH_OPERATION_DIAGNOSTIC, ToolDiagnostic as BackendDiagnostic,
};
use serde_json::json;
use tokio_util::sync::CancellationToken;

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

fn operation_context(exit_code: i32) -> BackendDiagnostic {
    BackendDiagnostic {
        code: LOCAL_BASH_OPERATION_DIAGNOSTIC.to_string(),
        message: "command executed".to_string(),
        details: Some(json!({
            "exit_code": exit_code,
            "cancelled": false,
            "timed_out": false,
            "truncated": false,
            "command_included": false,
        })),
    }
}

fn degraded_backend_diagnostic() -> BackendDiagnostic {
    BackendDiagnostic {
        code: CODE_SANDBOX_DEGRADED.to_string(),
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
        json!({ "command": "ignored" }),
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

#[tokio::test]
async fn bash_tool_preserves_backend_diagnostic_on_success() {
    let result = execute_with(BashResult {
        stdout: b"ok".to_vec(),
        stderr: Vec::new(),
        exit_code: Some(0),
        signal: None,
        diagnostics: vec![operation_context(0), degraded_backend_diagnostic()],
    })
    .await;

    assert!(!result.is_error);
    assert_eq!(result.diagnostics.len(), 1);
    let diagnostic = &result.diagnostics[0];
    assert_eq!(diagnostic.code, CODE_SANDBOX_DEGRADED);
    assert_eq!(diagnostic.context["layer"], "test-tree");
    assert_eq!(diagnostic.context["reason"], "attach unavailable");
}

#[tokio::test]
async fn bash_tool_preserves_backend_diagnostic_on_nonzero_exit() {
    let result = execute_with(BashResult {
        stdout: Vec::new(),
        stderr: b"failed".to_vec(),
        exit_code: Some(7),
        signal: None,
        diagnostics: vec![operation_context(7), degraded_backend_diagnostic()],
    })
    .await;

    assert!(result.is_error);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == CODE_SANDBOX_DEGRADED)
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
    assert_eq!(result.diagnostics[0].code, CODE_SANDBOX_DEGRADED);
    assert_eq!(result.diagnostics[0].context["layer"], "test-tree");
}
