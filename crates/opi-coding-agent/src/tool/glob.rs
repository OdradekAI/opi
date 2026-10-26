use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use opi_agent::diagnostic::FsToolError;
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult, result};
use opi_ai::message::{OutputContent, ToolDef};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GlobArgs {
    /// Glob pattern to search for (e.g. "**/*.rs").
    pub pattern: String,
}

pub struct GlobTool {
    workspace_root: PathBuf,
    schema: serde_json::Value,
}

impl GlobTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        let schema = schemars::schema_for!(GlobArgs);
        Self {
            workspace_root,
            schema: serde_json::to_value(&schema).unwrap_or_default(),
        }
    }
}

impl Tool for GlobTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "glob".into(),
            description: "Gitignore-aware file discovery by glob pattern. \
                Results are relative paths in lexicographic order, capped at a \
                fixed limit. An opi convenience (not pi-parity)."
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
        let args: GlobArgs = match serde_json::from_value(arguments) {
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
            let glob_matcher = match globset::Glob::new(&pattern) {
                Ok(g) => g.compile_matcher(),
                Err(e) => {
                    return Ok(result::err(vec![OutputContent::Text {
                        text: format!("invalid glob pattern: {e}"),
                    }]));
                }
            };

            let mut matched: Vec<String> = Vec::new();
            let mut non_utf8 = 0usize;
            let mut visited_entries = 0usize;
            let mut search_terminated_early = false;
            let mut cancelled = false;
            let walker = super::nav_walk_builder(&workspace_root).build();

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
                if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                    continue;
                }
                let path = entry.path();
                let relative = path.strip_prefix(&workspace_root).unwrap_or(path);
                if !(glob_matcher.is_match(relative) || glob_matcher.is_match(path)) {
                    continue;
                }
                let Some(rel_str) = relative.to_str() else {
                    // Non-UTF-8 entry name (Unix-only in practice): skip and
                    // report via UnsupportedEncoding, not U+FFFD (parity with find).
                    non_utf8 += 1;
                    continue;
                };
                matched.push(rel_str.to_owned());
                if matched.len() > super::MAX_NAV_RESULTS {
                    search_terminated_early = true;
                    break;
                }
            }

            matched.sort();
            let total = matched.len();
            let mut truncated = total > super::MAX_NAV_RESULTS;
            let mut omitted_count = total.saturating_sub(super::MAX_NAV_RESULTS);
            if search_terminated_early {
                truncated = true;
                omitted_count = omitted_count.max(1);
            }
            if matched.len() > super::MAX_NAV_RESULTS {
                matched.truncate(super::MAX_NAV_RESULTS);
            }
            let text = if matched.is_empty() {
                if search_terminated_early {
                    format!(
                        "search terminated before completing; results are incomplete for pattern: {pattern}"
                    )
                } else {
                    format!("no matches for pattern: {pattern}")
                }
            } else {
                matched.join("\n")
            };

            // glob walks the workspace root directly, so the relation is always `inside`.
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
