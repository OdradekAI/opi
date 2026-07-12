use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use opi_agent::diagnostic::{FsToolError, code};
use opi_agent::tool::{ExecutionMode, Tool, ToolDiagnostic, ToolError, ToolResult, result};
use opi_ai::message::{OutputContent, ToolDef};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use tokio_util::sync::CancellationToken;

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
    schema: serde_json::Value,
}

impl WriteTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        let schema = schemars::schema_for!(WriteArgs);
        Self {
            workspace_root,
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
            //    (tokio::fs::write truncates then writes, so a post-write stat is
            //    too late.) Existing directories are rejected before temp-file
            //    staging so they get the same typed NotAFile diagnostic as read/edit.
            let existing_meta = match tokio::fs::metadata(&file_path).await {
                Ok(meta) => Some(meta),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                    return Ok(super::fs_error_result(FsToolError::PermissionDenied {
                        path: file_path.clone(),
                    }));
                }
                Err(_) => None,
            };
            if existing_meta.as_ref().is_some_and(|meta| meta.is_dir()) {
                return Ok(super::fs_error_result(FsToolError::NotAFile {
                    path: file_path.clone(),
                }));
            }
            let existed_before = existing_meta.is_some();
            let bytes_before = existing_meta.as_ref().map(|m| m.len());

            // 3. Ensure the parent directory exists. create_dir_all failure is
            //    classified by an explicit probe rather than a platform-specific
            //    ErrorKind: a parent component that is an existing regular file
            //    is reported as NotADirectory deterministically.
            if let Some(parent) = file_path.parent()
                && let Err(e) = tokio::fs::create_dir_all(parent).await
            {
                if let Some(file_ancestor) = first_file_ancestor(parent) {
                    return Ok(super::fs_error_result(FsToolError::NotADirectory {
                        path: file_ancestor,
                    }));
                }
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    return Ok(super::fs_error_result(FsToolError::PermissionDenied {
                        path: parent.to_path_buf(),
                    }));
                }
                return Ok(result::err(vec![OutputContent::Text {
                    text: format!(
                        "failed to create directories for {}: {e}",
                        file_path.display()
                    ),
                }]));
            }

            // 4. Atomic write: stage the full content in a sibling temp file in
            //    the target directory, then rename into place. rename is atomic
            //    on the same filesystem and replaces the existing target, so an
            //    interrupted write leaves either the full new content or the
            //    prior content (never a partial/truncated mix). Every error path
            //    best-effort removes the temp file, with a Drop guard as a
            //    cancellation backstop. (fsync before rename is a durability
            //    concern, out of Phase 11 scope and matching the prior
            //    direct-write behavior.)
            let parent_dir = file_path.parent().unwrap_or(file_path.as_path());
            let file_name = file_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "file".to_string());
            let pid = std::process::id();
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let temp_path = parent_dir.join(format!(".{file_name}.opi-write-tmp-{pid}-{nanos}"));
            let mut temp_guard = super::TempFileGuard::new(temp_path);

            if let Err(e) = tokio::fs::write(temp_guard.path(), args.content.as_bytes()).await {
                temp_guard.cleanup().await;
                return Ok(result::err(vec![OutputContent::Text {
                    text: format!("failed to write {}: {e}", file_path.display()),
                }]));
            }
            if let Err(e) = tokio::fs::rename(temp_guard.path(), &file_path).await {
                temp_guard.cleanup().await;
                return Ok(result::err(vec![OutputContent::Text {
                    text: format!("failed to write {}: {e}", file_path.display()),
                }]));
            }
            temp_guard.disarm();

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

/// Walk from `start` upward, returning the first existing component (including
/// `start` itself) that is a regular file. This lets a parent-component-is-a-
/// file condition be classified as `NotADirectory` deterministically,
/// independent of the platform-specific `ErrorKind` that `create_dir_all`
/// surfaces (ENOTDIR on Linux, ERROR_DIRECTORY on Windows). Stops at the first
/// existing directory; returns `None` when only directories or missing
/// components are found.
fn first_file_ancestor(start: &Path) -> Option<PathBuf> {
    let mut current = start;
    loop {
        match std::fs::metadata(current) {
            Ok(meta) if meta.is_file() => return Some(current.to_path_buf()),
            Ok(_) => return None, // existing dir (or symlink-to-dir): ancestors are dirs
            Err(_) => current = current.parent()?,
        }
    }
}
