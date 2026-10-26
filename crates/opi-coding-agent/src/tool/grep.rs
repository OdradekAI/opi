use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use opi_agent::diagnostic::{FsToolError, code};
use opi_agent::tool::{ExecutionMode, Tool, ToolDiagnostic, ToolError, ToolResult, result};
use opi_ai::message::{OutputContent, ToolDef};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GrepArgs {
    /// Regex pattern to search for.
    pub pattern: String,
}

pub struct GrepTool {
    workspace_root: PathBuf,
    schema: serde_json::Value,
}

impl GrepTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        let schema = schemars::schema_for!(GrepArgs);
        Self {
            workspace_root,
            schema: serde_json::to_value(&schema).unwrap_or_default(),
        }
    }
}

impl Tool for GrepTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "grep".into(),
            description: "Gitignore-aware regex search over file contents. \
                Matches are sorted by relative path then line number and capped \
                at a fixed limit."
                .into(),
            input_schema: self.schema.clone(),
            constrained_sampling: None,
        }
    }

    fn execute(
        &self,
        _call_id: &str,
        arguments: serde_json::Value,
        signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let args: GrepArgs = match serde_json::from_value(arguments) {
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
        let pattern = args.pattern;
        Box::pin(async move {
            let re = match regex::Regex::new(&pattern) {
                Ok(r) => r,
                Err(e) => {
                    return Ok(result::err(vec![OutputContent::Text {
                        text: format!("invalid regex pattern: {e}"),
                    }]));
                }
            };

            // (relative path, line number, formatted line). Sorting by
            // (relative path, line number) gives the Phase 11.7 lexicographic
            // relative-path order with intra-file line order preserved (a
            // plain string sort would reorder same-file lines by line text).
            let mut matches: Vec<(String, usize, String)> = Vec::new();
            let mut files_oversized_skipped = 0usize;
            let mut files_skipped_non_utf8 = 0usize;
            let mut files_skipped_unreadable = 0usize;
            let mut files_skipped_permission_denied = 0usize;
            let mut total_read_bytes = 0u64;
            let mut visited_entries = 0usize;
            let mut search_terminated_early = false;
            let mut cancelled = false;
            let walker = super::nav_walk_builder(&workspace_root).build();

            for entry in walker.flatten() {
                if signal.is_cancelled() {
                    cancelled = true;
                    break;
                }
                if visited_entries >= super::MAX_NAV_VISITED_ENTRIES {
                    search_terminated_early = true;
                    break;
                }
                visited_entries += 1;
                if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                    continue;
                }
                let path = entry.path();
                let relative_os = path.strip_prefix(&workspace_root).unwrap_or(path);
                let Some(relative) = relative_os.to_str() else {
                    files_skipped_non_utf8 += 1;
                    continue;
                };

                let meta = match std::fs::metadata(path) {
                    Ok(meta) => meta,
                    Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                        files_skipped_permission_denied += 1;
                        continue;
                    }
                    Err(_) => {
                        files_skipped_unreadable += 1;
                        continue;
                    }
                };
                if meta.len() > super::MAX_NAV_FILE_BYTES {
                    files_oversized_skipped += 1;
                    continue;
                }
                if total_read_bytes.saturating_add(meta.len()) > super::MAX_GREP_TOTAL_READ_BYTES {
                    search_terminated_early = true;
                    break;
                }
                let content = match std::fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                        files_skipped_permission_denied += 1;
                        continue;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
                        files_skipped_non_utf8 += 1;
                        continue;
                    }
                    Err(_) => {
                        files_skipped_unreadable += 1;
                        continue;
                    }
                };
                total_read_bytes = total_read_bytes.saturating_add(meta.len());
                for (index, line) in content.lines().enumerate() {
                    if re.is_match(line) {
                        matches.push((
                            relative.to_owned(),
                            index + 1,
                            format!("{relative}: {line}"),
                        ));
                        if matches.len() > super::MAX_NAV_RESULTS {
                            search_terminated_early = true;
                            break;
                        }
                    }
                }
                if search_terminated_early {
                    break;
                }
            }

            matches.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
            let mut lines: Vec<String> = matches
                .into_iter()
                .map(|(_, _, formatted)| formatted)
                .collect();
            let total = lines.len();
            let mut truncated = total > super::MAX_NAV_RESULTS;
            let mut omitted_count = total.saturating_sub(super::MAX_NAV_RESULTS);
            if search_terminated_early {
                truncated = true;
                omitted_count = omitted_count.max(1);
            }
            if lines.len() > super::MAX_NAV_RESULTS {
                lines.truncate(super::MAX_NAV_RESULTS);
            }
            let text = if lines.is_empty() {
                if search_terminated_early {
                    format!(
                        "search terminated before completing; results are incomplete for pattern: {pattern}"
                    )
                } else {
                    format!("no matches for pattern: {pattern}")
                }
            } else {
                lines.join("\n")
            };

            let details = serde_json::json!({
                "workspace_root": workspace_root.to_string_lossy(),
                "pattern": pattern,
                "match_count": total,
                "workspace_relation": result::WorkspaceRelation::Inside,
                "truncated": truncated,
                "omitted_count": omitted_count,
                "cancelled": cancelled,
                "visited_entries": visited_entries,
                "search_terminated_early": search_terminated_early,
                "files_oversized_skipped": files_oversized_skipped,
                "files_skipped_non_utf8": files_skipped_non_utf8,
                "files_skipped_unreadable": files_skipped_unreadable,
                "files_skipped_permission_denied": files_skipped_permission_denied,
            });
            let mut tool_result = result::ok(vec![OutputContent::Text { text }], details);
            tool_result.truncated = truncated;
            if files_skipped_non_utf8 > 0 {
                tool_result.diagnostics.push(
                    FsToolError::UnsupportedEncoding {
                        omitted_count: files_skipped_non_utf8,
                    }
                    .to_diagnostic(),
                );
            }
            if files_skipped_permission_denied > 0 {
                tool_result.diagnostics.push(ToolDiagnostic {
                    code: code::CODE_TOOL_PERMISSION_DENIED.to_string(),
                    message: format!(
                        "{files_skipped_permission_denied} files skipped due to permission denied"
                    ),
                    context: serde_json::json!({
                        "omitted_count": files_skipped_permission_denied
                    }),
                });
            }
            Ok(tool_result)
        })
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Parallel
    }
}
