use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use opi_agent::diagnostic::FsToolError;
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult, result};
use opi_ai::message::{OutputContent, ToolDef};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

const DEFAULT_MAX_ENTRIES: usize = 200;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LsArgs {
    /// Directory path to list (relative to workspace root, use "." for root).
    pub path: String,
    /// Maximum number of entries to return. Defaults to 200.
    #[serde(default)]
    pub max_entries: Option<usize>,
    /// Maximum recursion depth. 0 lists only the specified directory, 1 includes
    /// immediate children and their types, etc. Defaults to 0 (flat listing).
    #[serde(default)]
    pub max_depth: Option<usize>,
}

pub struct LsTool {
    workspace_root: PathBuf,
    schema: serde_json::Value,
}

impl LsTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        let schema = schemars::schema_for!(LsArgs);
        Self {
            workspace_root,
            schema: serde_json::to_value(&schema).unwrap_or_default(),
        }
    }
}

impl Tool for LsTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "ls".into(),
            description: "List directory contents with bounded output. Entries \
                are sorted deterministically. Directories are indicated with a \
                trailing /. Honors nested .gitignore files like the other nav \
                tools."
                .into(),
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
        let args: LsArgs = match serde_json::from_value(arguments) {
            Ok(a) => a,
            Err(e) => {
                return Box::pin(async move {
                    Ok(result::err(vec![OutputContent::Text {
                        text: format!("invalid arguments: {e}"),
                    }]))
                });
            }
        };
        let workspace_root = self.workspace_root.clone();
        let max_entries = args.max_entries.unwrap_or(DEFAULT_MAX_ENTRIES);
        let max_depth = args.max_depth.unwrap_or(0);
        let path_arg = args.path;

        Box::pin(async move {
            // Resolve the target directory through the shared resolver so the
            // workspace relation is recovered uniformly ("." resolves to root).
            let resolved = match super::resolve_tool_path(
                &workspace_root,
                &path_arg,
                super::PathPolicy::WorkspaceOnly,
            ) {
                Ok(r) => r,
                Err(e) => {
                    return Ok(super::fs_error_result(e));
                }
            };
            let target = resolved.path;
            let workspace_relation = resolved.workspace_relation;

            if !target.exists() {
                return Ok(super::fs_error_result(FsToolError::NotFound {
                    user_path: path_arg.clone(),
                    resolved_path: Some(target.clone()),
                }));
            }

            if !target.is_dir() {
                return Ok(super::fs_error_result(FsToolError::NotADirectory {
                    path: target.clone(),
                }));
            }

            // Surface an unreadable target directory instead of silently listing
            // it as empty (WalkBuilder.flatten would otherwise swallow the
            // target-level EACCES). Nested-dir permission errors during the
            // walk are swallowed by flatten(), consistent with grep/find/glob.
            if let Err(e) = std::fs::read_dir(&target)
                && e.kind() == std::io::ErrorKind::PermissionDenied
            {
                return Ok(super::fs_error_result(FsToolError::PermissionDenied {
                    path: target.clone(),
                }));
            }

            // Walk via the shared ignore configuration so ls honors nested
            // .gitignore files and treats symlinks identically to grep/find/glob
            // (no follow). ignore::WalkBuilder depth is INCLUSIVE with the walk
            // root at depth 0; to reproduce the prior collect_entries(current=0,
            // max_depth) semantics — list the target's children and recurse
            // `max_depth` levels below them — cap the walker at `max_depth + 1`
            // and skip the root entry itself (depth 0).
            let walker = super::nav_walk_builder(&target)
                .max_depth(Some(max_depth + 1))
                .build();

            let mut entries: Vec<Entry> = Vec::new();
            let mut non_utf8 = 0usize;
            let mut visited_entries = 0usize;
            let mut total_entries = 0usize;
            let mut omitted = 0usize;
            let mut search_terminated_early = false;
            let mut cancelled = false;
            // Canonicalize the workspace root once so walked entry paths strip
            // correctly on macOS, where the tempdir lives under /var (a symlink
            // to /private/var): the walker is rooted at the canonical target, so
            // entries are canonical and would not strip against the raw root. The
            // raw-root fallback keeps it a no-op where raw and canonical coincide.
            let canonical_root =
                std::fs::canonicalize(&workspace_root).unwrap_or_else(|_| workspace_root.clone());
            for entry in walker.flatten() {
                // Honor the cancellation token mid-walk (sync poll; blocking
                // iterator). Cooperative: return partial results on cancel.
                if signal.is_cancelled() {
                    cancelled = true;
                    break;
                }
                if visited_entries >= super::MAX_NAV_VISITED_ENTRIES {
                    search_terminated_early = true;
                    break;
                }
                visited_entries += 1;
                if entry.depth() == 0 {
                    // The target directory itself; skip (we list its contents).
                    continue;
                }
                let path = entry.path();
                let relative_os = path
                    .strip_prefix(&canonical_root)
                    .or_else(|_| path.strip_prefix(&workspace_root))
                    .unwrap_or(path);
                let Some(relative) = relative_os.to_str() else {
                    // Non-UTF-8 entry name (Unix-only in practice): skip and
                    // report via an UnsupportedEncoding diagnostic instead of
                    // silent U+FFFD.
                    non_utf8 += 1;
                    continue;
                };
                total_entries += 1;
                if entries.len() >= max_entries {
                    omitted += 1;
                    continue;
                }
                // Use the entry's file_type (does NOT follow symlinks) so a
                // symlink-to-dir is not marked as a directory and not recursed
                // - consistent with grep/find/glob (Phase 11.7).
                let is_dir = entry.file_type().is_some_and(|ft| ft.is_dir());
                entries.push(Entry {
                    relative_path: relative.to_owned(),
                    is_dir,
                });
            }

            entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

            if search_terminated_early && entries.len() >= max_entries {
                omitted = omitted.max(1);
            }
            let truncated = omitted > 0 || search_terminated_early;

            let mut lines: Vec<String> = entries
                .iter()
                .map(|e| {
                    if e.is_dir {
                        format!("{}/", e.relative_path)
                    } else {
                        e.relative_path.clone()
                    }
                })
                .collect();

            if truncated {
                lines.push(format!("... (truncated, {} entries omitted)", omitted));
            }

            let text = if lines.is_empty() {
                // Non-empty no-entries message (Phase 11.7): never the empty
                // string for a zero-match listing.
                "no entries".to_string()
            } else {
                lines.join("\n")
            };

            let workspace_root_display: std::borrow::Cow<'_, str> =
                if matches!(workspace_relation, result::WorkspaceRelation::Inside) {
                    std::borrow::Cow::Borrowed(".")
                } else {
                    workspace_root.to_string_lossy()
                };
            let details = serde_json::json!({
                "workspace_root": workspace_root_display,
                "path": path_arg,
                "entry_count": entries.len(),
                "total_entries": total_entries,
                "truncated": truncated,
                "omitted_count": omitted,
                "workspace_relation": workspace_relation,
                "cancelled": cancelled,
                "visited_entries": visited_entries,
                "search_terminated_early": search_terminated_early,
            });

            let mut tool_result = result::ok(vec![OutputContent::Text { text }], details);
            tool_result.truncated = truncated;
            if non_utf8 > 0 {
                tool_result.diagnostics.push(
                    FsToolError::UnsupportedEncoding {
                        omitted_count: non_utf8,
                    }
                    .to_diagnostic(),
                );
            }
            Ok(tool_result)
        })
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Parallel
    }
}

struct Entry {
    relative_path: String,
    is_dir: bool,
}
