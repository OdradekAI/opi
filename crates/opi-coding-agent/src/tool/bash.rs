use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use opi_agent::diagnostic::code::CODE_TOOL_EXECUTION_FAILED;
use opi_agent::tool::{ExecutionMode, Tool, ToolDiagnostic, ToolError, ToolResult, result};
use opi_ai::message::{OutputContent, ToolDef};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use super::{BashOpError, BashOperations, BashRequest, BashResult};
use super::{LOCAL_BASH_OPERATION_DIAGNOSTIC, LocalBashOperations, MAX_BASH_OUTPUT_BYTES};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BashArgs {
    /// Command to execute.
    pub command: String,
    /// Timeout in seconds (optional, defaults to 30).
    pub timeout_secs: Option<u64>,
}

/// Bash tool. A thin caller over the injected [`BashOperations`] backend
/// (Phase 15 T5 Operations seam). Command construction, spawn, bounded stream
/// capture, the timeout/cancel/`wait` race, and exit/signal extraction live in
/// `LocalBashOperations::exec`; this tool maps the returned [`BashResult`] into
/// the agent `ToolResult`, rebuilding the exact pre-15.2 details/diagnostic
/// shape so existing behavior is preserved byte-for-byte.
pub struct BashTool {
    workspace_root: PathBuf,
    ops: Arc<dyn BashOperations>,
    schema: serde_json::Value,
}

impl BashTool {
    /// Convenience constructor that self-injects the local default backend.
    /// Production wiring goes through [`Self::new_with_ops`] via
    /// `CodingHarness::build_tools`, which constructs the local default itself.
    pub fn new(workspace_root: PathBuf) -> Self {
        Self::new_with_ops(workspace_root, Arc::new(LocalBashOperations::new()))
    }

    /// Primary constructor: inject an explicit [`BashOperations`] backend so a
    /// mock (or a future remote/T4-sandbox backend) can be wired in.
    pub fn new_with_ops(workspace_root: PathBuf, ops: Arc<dyn BashOperations>) -> Self {
        let schema = schemars::schema_for!(BashArgs);
        Self {
            workspace_root,
            ops,
            schema: serde_json::to_value(&schema).unwrap_or_default(),
        }
    }
}

impl Tool for BashTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "bash".into(),
            description: "Execute a shell command with timeout and streamed output.".into(),
            input_schema: self.schema.clone(),
        }
    }

    fn execute(
        &self,
        _call_id: &str,
        arguments: serde_json::Value,
        signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let args: BashArgs = match serde_json::from_value(arguments) {
            Ok(a) => a,
            Err(e) => {
                return Box::pin(async move {
                    Ok(result::err(vec![OutputContent::Text {
                        text: format!("invalid arguments: {e}"),
                    }]))
                });
            }
        };
        let timeout = Duration::from_secs(args.timeout_secs.unwrap_or(30));
        let command = args.command;
        let cwd = self.workspace_root.clone();
        let workspace_root = self.workspace_root.clone();
        let ops = self.ops.clone();
        Box::pin(async move {
            let shell = if cfg!(windows) { "cmd" } else { "sh" };
            let request = BashRequest {
                command: command.clone(),
                cwd: cwd.clone(),
                timeout,
                signal,
                env: Vec::new(),
            };
            let backend = match ops.exec(request).await {
                Ok(r) => r,
                Err(BashOpError::SpawnFailed { message }) => {
                    return Ok(result::err(vec![OutputContent::Text {
                        text: format!("failed to spawn command: {message}"),
                    }]));
                }
                Err(BashOpError::WaitFailed { .. }) => {
                    return Ok(wait_failed_result(&workspace_root, &command, &cwd, shell));
                }
                Err(BashOpError::SandboxUnavailable { message }) => {
                    // Phase 15.5.1 fail-closed: require=true + an unavailable
                    // layer. The backend refused to spawn, so surface the redacted
                    // layer summary (no command/env/paths) as an error result.
                    return Ok(result::err(vec![OutputContent::Text {
                        text: format!("sandbox required but unavailable: {message}"),
                    }]));
                }
                Err(BashOpError::Other { message }) => {
                    return Ok(result::err(vec![OutputContent::Text {
                        text: format!("bash backend error: {message}"),
                    }]));
                }
            };

            // Lift the operation-context flags the backend carried in-band.
            let (cancelled, timed_out, truncated, full_output, kill_error) =
                lift_operation_context(&backend);
            let exit_code = backend.exit_code;

            // Content text mirrors the pre-15.2 shape: timeout/cancellation
            // report the cause (the backend discards the killed child's pipes,
            // so there is no captured output); a clean exit reports the merged
            // stdout-then-stderr preview, re-capped at MAX_BASH_OUTPUT_BYTES.
            let text = if cancelled {
                "command cancelled".to_string()
            } else if timed_out {
                "command timed out".to_string()
            } else {
                let mut merged: Vec<u8> =
                    Vec::with_capacity(backend.stdout.len() + backend.stderr.len());
                merged.extend_from_slice(&backend.stdout);
                merged.extend_from_slice(&backend.stderr);
                let cap = MAX_BASH_OUTPUT_BYTES.min(merged.len());
                String::from_utf8_lossy(&merged[..cap]).into_owned()
            };

            let details = with_env_policy(result::bash_operation_metadata(
                &workspace_root,
                &command,
                &cwd,
                shell,
                exit_code,
                timed_out,
                cancelled,
                truncated,
                full_output,
            ));
            let is_error = exit_code != Some(0);
            Ok(bash_result(
                vec![OutputContent::Text { text }],
                details,
                is_error,
                truncated,
                exit_code,
                cancelled,
                timed_out,
                kill_error,
            ))
        })
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Sequential
    }
}

/// Lift the operation-context flags the backend carried in
/// [`BashResult::diagnostics`] under [`LOCAL_BASH_OPERATION_DIAGNOSTIC`].
/// Returns `(cancelled, timed_out, truncated, full_output, kill_error)`. Defaults
/// to `false`/`None` if the backend omitted the context diagnostic.
fn lift_operation_context(result: &BashResult) -> (bool, bool, bool, Option<&str>, Option<&str>) {
    let details = result
        .diagnostics
        .iter()
        .find(|d| d.code == LOCAL_BASH_OPERATION_DIAGNOSTIC)
        .and_then(|d| d.details.as_ref());
    let flag = |key: &str| {
        details
            .and_then(|d| d.get(key))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    };
    let opt_str = |key: &str| details.and_then(|d| d.get(key)).and_then(|v| v.as_str());
    (
        flag("cancelled"),
        flag("timed_out"),
        flag("truncated"),
        opt_str("full_output"),
        opt_str("kill_error"),
    )
}

/// Assemble a bash tool result from the shared success builder, then override
/// `is_error` (nonzero exit) and `truncated` (output cap). Mirrors the Phase
/// 11.1 bash pattern: nonzero-exit keeps the success-shape result with details
/// present (the stable operation-metadata contract); only `is_error` flips.
///
/// On an error result a [`ToolDiagnostic`] carrying the operation context
/// (exit_code/cancelled/timed_out/truncated) is pushed so the agent loop (Phase
/// 11.8 / S1) lifts it into a Phase 7 Diagnostic + trace.
#[allow(clippy::too_many_arguments)] // threads the failure discriminators alongside the result builder inputs
fn bash_result(
    content: Vec<OutputContent>,
    details: Value,
    is_error: bool,
    truncated: bool,
    exit_code: Option<i32>,
    cancelled: bool,
    timed_out: bool,
    kill_error: Option<&str>,
) -> ToolResult {
    let mut tool_result = result::ok(content, details);
    tool_result.is_error = is_error;
    tool_result.truncated = truncated;
    if is_error {
        tool_result.diagnostics.push(bash_operation_diagnostic(
            exit_code, cancelled, timed_out, truncated, kill_error,
        ));
    }
    tool_result
}

fn wait_failed_result(workspace_root: &Path, command: &str, cwd: &Path, shell: &str) -> ToolResult {
    let details = with_env_policy(result::bash_operation_metadata(
        workspace_root,
        command,
        cwd,
        shell,
        None,
        false,
        false,
        false,
        None,
    ));
    let mut result = bash_result(
        vec![OutputContent::Text {
            text: "failed to wait for process".to_string(),
        }],
        details,
        true,
        false,
        None,
        false,
        false,
        None,
    );
    if let Some(diagnostic) = result.diagnostics.first_mut() {
        diagnostic.message = "failed to wait for process".to_string();
    }
    result
}

/// Build the bash operation-failure [`ToolDiagnostic`] carrying the stable
/// operation context the agent loop lifts into a Phase 7 Diagnostic +
/// DiagnosticLinked trace (Phase 11.8 / S1). Bash failures have no 11.2
/// filesystem cause, so the code is the generic [`CODE_TOOL_EXECUTION_FAILED`];
/// the per-cause detail lives in `context`. Raw command text is intentionally
/// excluded because commands can contain secrets.
fn bash_operation_diagnostic(
    exit_code: Option<i32>,
    cancelled: bool,
    timed_out: bool,
    truncated: bool,
    kill_error: Option<&str>,
) -> ToolDiagnostic {
    let message = if cancelled {
        "command cancelled"
    } else if timed_out {
        "command timed out"
    } else {
        "command exited non-zero"
    };
    let mut context = json!({
        "exit_code": exit_code,
        "cancelled": cancelled,
        "timed_out": timed_out,
        "truncated": truncated,
        "command_included": false,
    });
    if let Some(kill_error) = kill_error {
        context["kill_error"] = json!(kill_error);
    }
    ToolDiagnostic {
        code: CODE_TOOL_EXECUTION_FAILED.to_string(),
        message: message.to_string(),
        context,
    }
}

/// Inject the environment-handling policy token into bash operation metadata.
///
/// `details.env = { "inheritance": "inherited", "values_included": false }`.
/// `values_included: false` is the machine-checkable invariant that no inherited
/// environment values are dumped into details/diagnostics (the secret no-leak
/// test asserts it). Bash-local; intentionally NOT promoted into the shared
/// `bash_operation_metadata` builder in opi-agent.
fn with_env_policy(mut details: Value) -> Value {
    details["env"] = json!({ "inheritance": "inherited", "values_included": false });
    details
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_failed_result_carries_operation_metadata_and_diagnostic() {
        let workspace = PathBuf::from("D:/workspace");
        let cwd = workspace.clone();
        let result = wait_failed_result(&workspace, "echo SECRET_IN_COMMAND", &cwd, "cmd");

        assert!(result.is_error);
        assert!(!result.truncated);

        let details = result.details.as_ref().expect("details");
        assert_eq!(details["command"], "echo SECRET_IN_COMMAND");
        assert_eq!(details["exit_code"], serde_json::Value::Null);
        assert_eq!(details["timed_out"], false);
        assert_eq!(details["cancelled"], false);
        assert_eq!(details["truncated"], false);
        assert_eq!(
            details["env"],
            serde_json::json!({
                "inheritance": "inherited",
                "values_included": false
            })
        );

        assert_eq!(result.diagnostics.len(), 1);
        let diagnostic = &result.diagnostics[0];
        assert_eq!(diagnostic.code, CODE_TOOL_EXECUTION_FAILED);
        assert_eq!(diagnostic.message, "failed to wait for process");
        assert_eq!(diagnostic.context["exit_code"], serde_json::Value::Null);
        assert_eq!(diagnostic.context["timed_out"], false);
        assert_eq!(diagnostic.context["cancelled"], false);
        assert_eq!(diagnostic.context["truncated"], false);
        assert_eq!(diagnostic.context["command_included"], false);
        assert!(
            diagnostic.context.get("command").is_none(),
            "diagnostic context must not carry raw command text"
        );
    }
}
