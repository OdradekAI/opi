use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use opi_agent::diagnostic::{FsToolError, code};
use opi_agent::tool::{ExecutionMode, Tool, ToolDiagnostic, ToolError, ToolResult, result};
use opi_ai::message::{OutputContent, ToolDef};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use super::{FileOperations, FsOpError, LocalFileOperations};

/// Maximum byte size of a file edit will read and rewrite. Files larger than
/// this are rejected with a guardrail reason so the tool never buffers a
/// pathological file or emits a multi-megabyte before/after preview.
const MAX_EDIT_FILE_BYTES: u64 = 1024 * 1024; // 1 MiB

/// Maximum number of bytes of the pre-/post-edit file content embedded as the
/// `before`/`after` diff-preview strings. Keeps the RPC/NDJSON payload bounded
/// and the ratatui DiffView diff table affordable; the full file is still
/// written to disk. Beyond the cap the value is truncated on a UTF-8 char
/// boundary and a `before_truncated`/`after_truncated` flag is set.
const MAX_PREVIEW_BYTES: usize = 64 * 1024; // 64 KiB

/// Maximum number of characters of the attempted `old_string` rendered in a
/// failure message. The full length is always reported in
/// `details.old_string_len`. Truncation is char-based to stay UTF-8 safe.
const MAX_SNIPPET_CHARS: usize = 200;

/// Maximum number of match byte-offsets surfaced on an ambiguous
/// (multiple-match) refusal; the full count is always reported in
/// `details.occurrences`.
const MAX_SAMPLE_OFFSETS: usize = 3;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EditArgs {
    /// Relative path within workspace to edit.
    pub path: String,
    /// Exact, unique string to find in the file.
    ///
    /// Must be non-empty, must differ from `new_string`, and must occur exactly
    /// once. A not-unique, empty, or no-op `old_string` is rejected: the edit
    /// tool prefers clear failure over the silent first-occurrence-only
    /// replacement the prior implementation performed, and it never fuzzy-matches
    /// (a near-miss still fails). Bulk replace-all is intentionally not
    /// supported; disambiguate a repeated string by including more surrounding
    /// context in `old_string`. CRLF/LF and final-newline state of the file are
    /// preserved byte-for-byte because the file is decoded as UTF-8 and rewritten
    /// verbatim (Rust opens files in binary mode; no text-mode translation).
    pub old_string: String,
    /// Replacement string.
    pub new_string: String,
}

pub struct EditTool {
    workspace_root: PathBuf,
    ops: Arc<dyn FileOperations>,
    schema: serde_json::Value,
}

impl EditTool {
    /// Convenience constructor with the local filesystem backend. Production
    /// wiring injects via [`Self::new_with_ops`].
    pub fn new(workspace_root: PathBuf) -> Self {
        let ops = Arc::new(LocalFileOperations::new(workspace_root.clone()));
        Self::new_with_ops(workspace_root, ops)
    }

    /// Primary constructor with an explicit [`FileOperations`] backend (Phase 15
    /// T5 Operations seam). PathPolicy runs first; the backend receives the
    /// already-resolved path for stat, read, and the atomic temp+rename write.
    pub fn new_with_ops(workspace_root: PathBuf, ops: Arc<dyn FileOperations>) -> Self {
        let schema = schemars::schema_for!(EditArgs);
        Self {
            workspace_root,
            ops,
            schema: serde_json::to_value(&schema).unwrap_or_default(),
        }
    }
}

impl Tool for EditTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "edit".into(),
            description: "Replace a unique exact string in a file.".into(),
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
        let args: EditArgs = match serde_json::from_value(arguments) {
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
                // Path-resolution failures (OutsideWorkspace,
                // UnresolvedWorkspaceRoot) each carry a distinct CODE_TOOL_*
                // diagnostic via the taxonomy.
                return Box::pin(async move { Ok(super::fs_error_result(e)) });
            }
        };
        let workspace_relation = resolved_path.workspace_relation;
        let file_path = resolved_path.path;
        let workspace_root = self.workspace_root.clone();
        let path_for_display = args.path.clone();
        let ops = self.ops.clone();
        Box::pin(async move {
            // 1. Argument validations BEFORE any filesystem side effect, so a
            //    rejected edit touches no file. Empty old_string would make
            //    "exact match" meaningless (str::matches of "" is unbounded);
            //    old==new is a no-op write. Both are clear-failure refusals.
            if args.old_string.is_empty() {
                let context = json!({
                    "path": path_for_display,
                    "old_string_len": 0u64,
                });
                return Ok(edit_semantic_error(
                    format!("old_string must not be empty (editing {path_for_display})"),
                    context,
                ));
            }
            if args.old_string == args.new_string {
                let context = json!({
                    "path": path_for_display,
                    "old_string": snippet(&args.old_string),
                    "old_string_len": args.old_string.len(),
                });
                return Ok(edit_semantic_error(
                    format!(
                        "new_string equals old_string (editing {path_for_display}); edit is a no-op"
                    ),
                    context,
                ));
            }

            // 2. Taxonomy read path (mirrors read.rs). Stat first so the
            //    oversized guardrail rejects a huge file BEFORE it is buffered
            //    into memory, and so a directory is classified as NotAFile
            //    rather than read as bytes. Then NUL (binary) then UTF-8.
            let metadata = match ops.metadata(&file_path).await {
                Ok(m) => m,
                Err(FsOpError::NotFound { .. }) => {
                    return Ok(super::fs_error_result(FsToolError::NotFound {
                        user_path: path_for_display.clone(),
                        resolved_path: Some(file_path.clone()),
                    }));
                }
                Err(FsOpError::PermissionDenied { .. }) => {
                    return Ok(super::fs_error_result(FsToolError::PermissionDenied {
                        path: file_path.clone(),
                    }));
                }
                Err(e) => {
                    return Ok(result::err(vec![OutputContent::Text {
                        text: format!("failed to stat {}: {e}", file_path.display()),
                    }]));
                }
            };
            if metadata.is_dir {
                return Ok(super::fs_error_result(FsToolError::NotAFile {
                    path: file_path.clone(),
                }));
            }
            let file_bytes = metadata.len;
            if file_bytes > MAX_EDIT_FILE_BYTES {
                let context = json!({
                    "path": path_for_display,
                    "file_bytes": file_bytes,
                    "limit_bytes": MAX_EDIT_FILE_BYTES,
                });
                return Ok(edit_semantic_error(
                    format!(
                        "file '{path_for_display}' is {file_bytes} bytes which exceeds the {MAX_EDIT_FILE_BYTES}-byte edit limit; use a smaller unique old_string or the write tool"
                    ),
                    context,
                ));
            }

            let bytes = match ops.read_file(&file_path).await {
                Ok(b) => b,
                Err(FsOpError::NotFound { .. }) => {
                    return Ok(super::fs_error_result(FsToolError::NotFound {
                        user_path: path_for_display.clone(),
                        resolved_path: Some(file_path.clone()),
                    }));
                }
                Err(FsOpError::PermissionDenied { .. }) => {
                    return Ok(super::fs_error_result(FsToolError::PermissionDenied {
                        path: file_path.clone(),
                    }));
                }
                Err(FsOpError::NotAFile { .. }) => {
                    return Ok(super::fs_error_result(FsToolError::NotAFile {
                        path: file_path.clone(),
                    }));
                }
                Err(e) => {
                    return Ok(result::err(vec![OutputContent::Text {
                        text: format!("failed to read {}: {e}", file_path.display()),
                    }]));
                }
            };

            // NUL bytes are the binary-content heuristic; checked before UTF-8
            // so a file that is both NUL-bearing and invalid UTF-8 reports as
            // binary (the more accurate diagnosis), matching read.rs.
            if bytes.contains(&0u8) {
                return Ok(super::fs_error_result(FsToolError::BinaryFile {
                    path: file_path.clone(),
                }));
            }

            let content = match String::from_utf8(bytes) {
                Ok(s) => s,
                Err(e) => {
                    // File-content encoding failure is reported with the shared
                    // tool_unsupported_encoding code but a content-appropriate
                    // message: the FsToolError::UnsupportedEncoding variant is
                    // directory/entry-shaped, so the single-file case builds the
                    // diagnostic directly (mirrors read.rs). The agent loop
                    // lifts this into Phase 7 traces (task 11.8).
                    let byte_offset = e.utf8_error().valid_up_to();
                    let message = format!("'{}' is not valid UTF-8", file_path.display());
                    let mut unsupported = result::err(vec![OutputContent::Text {
                        text: message.clone(),
                    }]);
                    unsupported.diagnostics.push(ToolDiagnostic {
                        code: code::CODE_TOOL_UNSUPPORTED_ENCODING.to_string(),
                        message,
                        context: json!({
                            "path": file_path.display().to_string(),
                            "byte_offset": byte_offset,
                        }),
                    });
                    return Ok(unsupported);
                }
            };

            // 3. Exact-match counting. str::matches is exact (no fuzzy logic);
            //    a near-miss therefore falls through to the not-found branch.
            let occurrences = content.matches(&args.old_string).count();
            if occurrences == 0 {
                let context = json!({
                    "path": path_for_display,
                    "old_string": snippet(&args.old_string),
                    "old_string_len": args.old_string.len(),
                    "occurrences": 0u64,
                    "file_bytes": content.len(),
                    "line_count": content.lines().count(),
                });
                return Ok(edit_semantic_error(
                    format!(
                        "old_string not found in {path_for_display}; attempted to match: {}",
                        snippet_debug(&args.old_string)
                    ),
                    context,
                ));
            }
            if occurrences > 1 {
                // Not unique: refuse rather than silently replace the first
                // occurrence. Surface the full count plus a capped sample of
                // byte-offsets so the caller can disambiguate.
                let sample_offsets: Vec<usize> = content
                    .match_indices(&args.old_string)
                    .take(MAX_SAMPLE_OFFSETS)
                    .map(|(offset, _)| offset)
                    .collect();
                let context = json!({
                    "path": path_for_display,
                    "old_string": snippet(&args.old_string),
                    "old_string_len": args.old_string.len(),
                    "occurrences": occurrences,
                    "sample_offsets": sample_offsets,
                });
                return Ok(edit_semantic_error(
                    format!(
                        "old_string is not unique in {path_for_display}: found {occurrences} occurrences; add surrounding context to make it unique"
                    ),
                    context,
                ));
            }

            // 4. Exactly one match: apply the replacement.
            let before = content.clone();
            let new_content = content.replacen(&args.old_string, &args.new_string, 1);

            // 5. Atomic write via the backend. `LocalFileOperations` stages the
            //    new content in a sibling temp file then renames it into place,
            //    so an interrupted edit leaves either the full new content or the
            //    prior content (never a partial/truncated mix) — the same recipe
            //    the tool used to inline with `super::TempFileGuard`.
            if let Err(e) = ops.write_file(&file_path, new_content.as_bytes()).await {
                return Ok(result::err(vec![OutputContent::Text {
                    text: format!("failed to write {}: {e}", file_path.display()),
                }]));
            }

            // 6. Diff-preview metadata. before/after stay STRING-valued because
            //    interactive.rs reads them as strings to render the ratatui
            //    DiffView; they are byte-capped with truncated flags so a large
            //    (but under-limit) file cannot flood the RPC/NDJSON payload.
            let mut details = result::path_metadata(
                &workspace_root,
                &path_for_display,
                &file_path,
                workspace_relation,
            );
            details["action"] = json!("edited");
            details["occurrences"] = json!(1u64);
            let (before_preview, before_truncated) = truncate_preview(&before);
            let (after_preview, after_truncated) = truncate_preview(&new_content);
            details["before"] = json!(before_preview);
            details["after"] = json!(after_preview);
            if before_truncated {
                details["before_truncated"] = json!(true);
            }
            if after_truncated {
                details["after_truncated"] = json!(true);
            }

            Ok(result::ok(
                vec![OutputContent::Text {
                    text: format!("edited {path_for_display}"),
                }],
                details,
            ))
        })
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Sequential
    }
}

/// Build an edit-semantic error result: `is_error` with `details` left `None`
/// and the structured cause carried in a `tool_execution_failed` diagnostic's
/// `context`. Used for not-found, multiple-match, no-op, empty-`old_string`,
/// and oversized-file causes. These are edit-semantic causes (the file itself
/// is fine), so they do not map to an [`FsToolError`] variant. Routing the
/// cause through `diagnostics[].context` (rather than `details`) honors the
/// Phase 11.1/11.2 substrate invariant that error results omit `details` --
/// the same channel read/write/bash use -- while still surfacing the
/// multiple-match behavior the DoD requires (the message names the file,
/// old_string, and occurrence count; diagnostics lift into Phase 7 traces).
fn edit_semantic_error(message: String, context: Value) -> ToolResult {
    let mut res = result::err(vec![OutputContent::Text {
        text: message.clone(),
    }]);
    res.diagnostics.push(ToolDiagnostic {
        code: code::CODE_TOOL_EXECUTION_FAILED.to_string(),
        message,
        context,
    });
    res
}

/// Truncate `s` to at most [`MAX_SNIPPET_CHARS`] Unicode chars, appending an
/// ellipsis when truncated. Char-based so it never splits a multibyte codepoint.
fn snippet(s: &str) -> String {
    if s.chars().count() <= MAX_SNIPPET_CHARS {
        return s.to_string();
    }
    let head: String = s.chars().take(MAX_SNIPPET_CHARS).collect();
    format!("{head}...")
}

/// Like [`snippet`] but renders internal newlines/tabs as escape sequences so a
/// multiline `old_string` is legible inside a flat failure message.
fn snippet_debug(s: &str) -> String {
    snippet(s)
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Truncate a before/after preview string to [`MAX_PREVIEW_BYTES`] on a UTF-8
/// char boundary. Returns the (possibly truncated) value and whether truncation
/// occurred.
fn truncate_preview(s: &str) -> (String, bool) {
    if s.len() <= MAX_PREVIEW_BYTES {
        return (s.to_string(), false);
    }
    let mut end = MAX_PREVIEW_BYTES;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    (s[..end].to_string(), true)
}
