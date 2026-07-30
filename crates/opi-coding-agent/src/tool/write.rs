use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use opi_agent::diagnostic::{FsToolError, code};
use opi_agent::tool::{ExecutionMode, Tool, ToolDiagnostic, ToolError, ToolResult, result};
use opi_ai::message::{OutputContent, ToolDef};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use super::{FileOperations, FsOpError, LocalFileOperations};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WriteArgs {
    /// Relative path within workspace to write.
    pub path: String,
    /// Content to write.
    ///
    /// Carried as a UTF-8 JSON string, so arbitrary non-UTF-8 bytes are not
    /// representable at this boundary. "Binary-like" content is therefore
    /// defined operationally as the presence of a NUL byte (the conventional
    /// binary marker, matching the read-tool heuristic) and is rejected before
    /// any filesystem side effect. Bytes are otherwise written verbatim, so
    /// CRLF/LF and final-newline state round-trip exactly (Rust opens files in
    /// binary mode; no text-mode translation).
    pub content: String,
}

pub struct WriteTool {
    workspace_root: PathBuf,
    ops: Arc<dyn FileOperations>,
    schema: serde_json::Value,
}

impl WriteTool {
    /// Convenience constructor with the local filesystem backend. Production
    /// wiring injects via [`Self::new_with_ops`].
    pub fn new(workspace_root: PathBuf) -> Self {
        let ops = Arc::new(LocalFileOperations::new(workspace_root.clone()));
        Self::new_with_ops(workspace_root, ops)
    }

    /// Primary constructor with an explicit [`FileOperations`] backend (Phase 15
    /// T5 Operations seam). PathPolicy runs first; the backend receives the
    /// already-resolved path and performs the atomic temp+rename write.
    pub fn new_with_ops(workspace_root: PathBuf, ops: Arc<dyn FileOperations>) -> Self {
        let schema = schemars::schema_for!(WriteArgs);
        Self {
            workspace_root,
            ops,
            schema: serde_json::to_value(&schema).unwrap_or_default(),
        }
    }
}

impl Tool for WriteTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "write".into(),
            description: "Create or replace a file with the given content.".into(),
            input_schema: self.schema.clone(),
        }
    }

    fn execute(
        &self,
        _call_id: &str,
        arguments: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let args: WriteArgs = match serde_json::from_value(arguments) {
            Ok(a) => a,
            Err(e) => {
                return Box::pin(async move {
                    Ok(result::err(vec![OutputContent::Text {
                        text: format!("invalid arguments: {e}"),
                    }]))
                });
            }
        };
        let resolved_path = match super::resolve_tool_path(
            &self.workspace_root,
            &args.path,
            super::PathPolicy::WorkspaceOnly,
        ) {
            Ok(p) => p,
            Err(e) => {
                // Path-resolution failures (OutsideWorkspace, UnresolvedWorkspaceRoot)
                // each carry a distinct CODE_TOOL_* diagnostic via the taxonomy.
                return Box::pin(async move { Ok(super::fs_error_result(e)) });
            }
        };
        let workspace_relation = resolved_path.workspace_relation;
        let file_path = resolved_path.path;
        let workspace_root = self.workspace_root.clone();
        let path_for_display = args.path.clone();
        let ops = self.ops.clone();
        Box::pin(async move {
            let bytes_written = args.content.len();

            // 1. Reject NUL/binary-like content BEFORE any filesystem side effect,
            //    so a rejected write leaves no file and creates no parent dirs.
            //    Built directly with the shared tool_unsupported_encoding code
            //    (the FsToolError::UnsupportedEncoding variant is entry-shaped and
            //    reused by ls/find); the agent loop lifts this into Phase 7 traces.
            if args.content.contains('\0') {
                let message = format!(
                    "'{path_for_display}' contains a NUL byte and cannot be written as a text file"
                );
                let mut unsupported = result::err(vec![OutputContent::Text {
                    text: message.clone(),
                }]);
                unsupported.diagnostics.push(ToolDiagnostic {
                    code: code::CODE_TOOL_UNSUPPORTED_ENCODING.to_string(),
                    message,
                    context: json!({ "path": path_for_display }),
                });
                return Ok(unsupported);
            }

            // 2. Probe existence + prior size BEFORE writing so create vs
            //    overwrite is classified and a before/after audit is captured.
            //    Existing directories are rejected before any write so they get
            //    the same typed NotAFile diagnostic as read/edit.
            let existing_meta = match ops.metadata(&file_path).await {
                Ok(meta) => Some(meta),
                Err(FsOpError::NotFound { .. }) => None,
                Err(error) => {
                    return Ok(fs_op_error_result(
                        error,
                        &path_for_display,
                        &file_path,
                        "failed to inspect",
                    ));
                }
            };
            if existing_meta.as_ref().is_some_and(|meta| meta.is_dir) {
                return Ok(super::fs_error_result(FsToolError::NotAFile {
                    path: file_path.clone(),
                }));
            }
            let existed_before = existing_meta.is_some();
            let bytes_before = existing_meta.as_ref().map(|m| m.len);

            // 3. Ensure the parent directory exists. mkdir failure is classified
            //    by an explicit probe rather than a backend-specific error: a
            //    parent component that is an existing regular file is reported as
            //    NotADirectory deterministically.
            if let Some(parent) = file_path.parent()
                && let Err(e) = ops.mkdir(parent, true).await
            {
                match first_non_directory_ancestor(ops.as_ref(), parent).await {
                    Ok(Some(file_ancestor)) => {
                        return Ok(super::fs_error_result(FsToolError::NotADirectory {
                            path: file_ancestor,
                        }));
                    }
                    Ok(None) => {}
                    Err(probe_error) => {
                        return Ok(fs_op_error_result(
                            probe_error,
                            &path_for_display,
                            parent,
                            "failed to inspect parent directories",
                        ));
                    }
                }
                return Ok(fs_op_error_result(
                    e,
                    &path_for_display,
                    parent,
                    "failed to create parent directories",
                ));
            }

            // 4. Atomic write via the backend. `LocalFileOperations` stages the
            //    content in a sibling temp file then renames it into place, so an
            //    interrupted write leaves either the full new content or the
            //    prior content (never a partial/truncated mix) — the same recipe
            //    the tool used to inline with `super::TempFileGuard`.
            if let Err(e) = ops.write_file(&file_path, args.content.as_bytes()).await {
                return Ok(fs_op_error_result(
                    e,
                    &path_for_display,
                    &file_path,
                    "failed to write",
                ));
            }

            // 5. Audit details: action + bytes_written (always); before/after
            //    size audit on overwrite. size_delta is signed (smaller overwrite
            //    yields a negative delta).
            let action = if existed_before {
                "overwritten"
            } else {
                "created"
            };
            let mut details = result::path_metadata(
                &workspace_root,
                &path_for_display,
                &file_path,
                workspace_relation,
            );
            details["action"] = json!(action);
            details["bytes_written"] = json!(bytes_written);
            if existed_before && let Some(before) = bytes_before {
                details["bytes_before"] = json!(before);
                details["size_delta"] = json!((bytes_written as i64) - (before as i64));
            }

            let verb = if existed_before { "overwrote" } else { "wrote" };
            Ok(result::ok(
                vec![OutputContent::Text {
                    text: format!("{verb} {path_for_display}"),
                }],
                details,
            ))
        })
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Sequential
    }
}

fn fs_op_error_result(
    error: FsOpError,
    user_path: &str,
    fallback_path: &Path,
    operation: &str,
) -> ToolResult {
    let fs_error = match error {
        FsOpError::NotFound { path } => FsToolError::NotFound {
            user_path: user_path.to_string(),
            resolved_path: Some(path),
        },
        FsOpError::PermissionDenied { path } => FsToolError::PermissionDenied { path },
        FsOpError::NotAFile { path } => FsToolError::NotAFile { path },
        FsOpError::NotADirectory { path } => FsToolError::NotADirectory { path },
        FsOpError::Io { path, message } => {
            let message = format!("{operation} {}: {message}", fallback_path.display());
            let mut error_result = result::err(vec![OutputContent::Text {
                text: message.clone(),
            }]);
            error_result.diagnostics.push(ToolDiagnostic {
                code: code::CODE_TOOL_EXECUTION_FAILED.to_string(),
                message,
                context: json!({
                    "operation": operation,
                    "path": path.display().to_string(),
                }),
            });
            return error_result;
        }
    };
    super::fs_error_result(fs_error)
}

/// Walk from `start` upward through the injected backend, returning the first
/// existing component (including `start` itself) that is not a directory.
/// `NotFound`, `NotADirectory`, and unclassified metadata errors continue
/// upward because platform backends can report any of them for a descendant of
/// the actual file component. Typed permission failures remain visible.
async fn first_non_directory_ancestor(
    ops: &dyn FileOperations,
    start: &Path,
) -> Result<Option<PathBuf>, FsOpError> {
    let mut current = start;
    loop {
        match ops.metadata(current).await {
            Ok(meta) if meta.is_dir => return Ok(None),
            Ok(_) => return Ok(Some(current.to_path_buf())),
            Err(
                FsOpError::NotFound { .. } | FsOpError::NotADirectory { .. } | FsOpError::Io { .. },
            ) => {
                let Some(parent) = current.parent() else {
                    return Ok(None);
                };
                current = parent;
            }
            Err(error) => return Err(error),
        }
    }
}
