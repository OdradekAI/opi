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

/// The schema source for the `bash` tool input. This is the byte-stable
/// pre-extension contract: `schemars::schema_for!(BashArgs)` produces the
/// default schema, which Phase 16.9 keeps identical whether or not model routing
/// is configured. The model-routing `backend` enum is added to a COPY of this
/// schema by the harness when `strategy = "model"` — never by altering this
/// type — so `fixed`/`rules`/default schemas never carry the field.
///
/// Only the derived `JsonSchema` is consumed (by `default_bash_schema`); the
/// fields are never instantiated because deserialization goes through
/// `BashCallArgs`. The dead-code allow is intentional for that schema-only role.
#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BashArgs {
    /// Command to execute.
    pub command: String,
    /// Timeout in seconds (optional, defaults to 30).
    pub timeout_secs: Option<u64>,
}

/// The deserialization target for a `bash` invocation. Carries the optional
/// model-supplied `backend` (Phase 16.9) that `BashArgs` intentionally omits
/// so the default schema stays byte-stable. `backend` reaches the router only
/// under `strategy = "model"`; `fixed`/`rules` ignore it.
#[derive(Debug, Deserialize)]
pub struct BashCallArgs {
    pub command: String,
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub backend: Option<String>,
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

    /// Primary constructor: inject an explicit [`BashOperations`] backend and
    /// derive the default byte-stable schema from `BashArgs`.
    pub fn new_with_ops(workspace_root: PathBuf, ops: Arc<dyn BashOperations>) -> Self {
        Self::new_with_ops_and_schema(workspace_root, ops, default_bash_schema())
    }

    /// Phase 16.9: inject an explicit backend AND a precomputed input schema.
    /// Production (`CodingHarness::build_tools`) passes the resolved dynamic
    /// schema (the default, or the default plus the model-routing `backend`
    /// enum under `strategy = "model"`); tests pass [`default_bash_schema`].
    pub fn new_with_ops_and_schema(
        workspace_root: PathBuf,
        ops: Arc<dyn BashOperations>,
        schema: serde_json::Value,
    ) -> Self {
        Self {
            workspace_root,
            ops,
            schema,
        }
    }
}

/// The byte-stable default `bash` input schema, computed fresh from `BashArgs`.
/// The Minimal-Runtime / `fixed` / `rules` schemas are this value byte-for-byte;
/// the byte-equality acceptance check compares a tool's injected schema against a
/// fresh call here so a regression (a schemars bump, an accidental `BashArgs`
/// edit, or a wrong-strategy injection) fails loud.
pub fn default_bash_schema() -> serde_json::Value {
    let schema = schemars::schema_for!(BashArgs);
    serde_json::to_value(&schema).unwrap_or_default()
}

/// Add the required bounded `backend` field to a copy of the default bash schema
/// for `strategy = "model"` (Phase 16.9). Each candidate is a `oneOf` variant
/// `{const, title, description}` so it carries its own approval hint: an `ask`
/// candidate is visible with a description that it requires interactive
/// approval, and a `deny` candidate is absent (filtered upstream). `oneOf` (not a
/// flat `enum`) is required because JSON-Schema enums cannot attach per-value
/// descriptions, which the design mandates (§Model routing). The field is
/// required and `additionalProperties = false` keeps it bounded. This is the
/// ONLY divergence from `default_bash_schema`; `fixed`/`rules`/default schemas
/// never carry the field.
pub fn with_model_backend_enum(
    mut schema: serde_json::Value,
    candidates: &[(&str, bool)],
) -> serde_json::Value {
    let one_of: Vec<serde_json::Value> = candidates
        .iter()
        .map(|(id, requires_approval)| {
            let description = if *requires_approval {
                "Execution adapter; selecting it requires interactive approval before it runs."
            } else {
                "Execution adapter."
            };
            serde_json::json!({
                "const": id,
                "title": id,
                "description": description,
            })
        })
        .collect();
    if let Some(obj) = schema.as_object_mut() {
        if let Some(properties) = obj.get_mut("properties").and_then(|p| p.as_object_mut()) {
            properties.insert(
                "backend".to_string(),
                serde_json::json!({
                    "oneOf": one_of,
                    "description": "Backend adapter for this command. One of the eligible, non-denied execution adapters; required under the model strategy."
                }),
            );
        }
        if let Some(required) = obj.get_mut("required").and_then(|r| r.as_array_mut()) {
            required.push(serde_json::json!("backend"));
        }
        obj.insert("additionalProperties".to_string(), serde_json::json!(false));
    }
    schema
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
        let args: BashCallArgs = match serde_json::from_value(arguments) {
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
        let backend = args.backend;
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
                backend,
            };
            let backend = match ops.exec(request).await {
                Ok(r) => r,
                Err(error) => {
                    return Ok(backend_error_result(
                        &workspace_root,
                        &command,
                        &cwd,
                        shell,
                        &error,
                    ));
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
            let mut result = bash_result(
                vec![OutputContent::Text { text }],
                details,
                is_error,
                truncated,
                exit_code,
                cancelled,
                timed_out,
                kill_error,
            );
            append_backend_diagnostics(&mut result, &backend.diagnostics);
            Ok(result)
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

/// Preserve every backend diagnostic except the private operation-context
/// carrier, which this wrapper consumes when rebuilding the stable bash
/// details and execution-failure diagnostic.
fn append_backend_diagnostics(
    result: &mut ToolResult,
    diagnostics: &[super::operations::ToolDiagnostic],
) {
    result.diagnostics.extend(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code != LOCAL_BASH_OPERATION_DIAGNOSTIC)
            .map(|diagnostic| ToolDiagnostic {
                code: diagnostic.code.clone(),
                message: diagnostic.message.clone(),
                context: diagnostic.details.clone().unwrap_or_else(|| json!({})),
            }),
    );
}

fn backend_error_result(
    workspace_root: &Path,
    command: &str,
    cwd: &Path,
    shell: &str,
    error: &BashOpError,
) -> ToolResult {
    let mut result = match error.root_cause() {
        BashOpError::SpawnFailed { message } => result::err(vec![OutputContent::Text {
            text: format!("failed to spawn command: {message}"),
        }]),
        BashOpError::WaitFailed { .. } => wait_failed_result(workspace_root, command, cwd, shell),
        BashOpError::Other { message } => result::err(vec![OutputContent::Text {
            text: format!("bash backend error: {message}"),
        }]),
        BashOpError::BackendFailure { .. } => unreachable!("root_cause removes wrappers"),
    };
    append_backend_diagnostics(&mut result, error.diagnostics());
    result
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
