//! Shared [`ToolResult`] builders and metadata blocks (Phase 11.1).
//!
//! Every built-in tool routes its success and failure results through these
//! helpers so the base contract (`content`/`details`/`is_error`/`terminate`/
//! `truncated`/`diagnostics`) and the path-/operation-metadata shapes stay
//! uniform across tools and branches.

use std::path::Path;

use opi_ai::message::OutputContent;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{ToolDiagnostic, ToolResult};

/// Build a successful tool result carrying structured `details`.
///
/// `truncated` defaults to `false`; tools that cap output construct the result
/// via this helper, then set `truncated` (and the matching details key) before
/// returning.
pub fn ok(content: Vec<OutputContent>, details: Value) -> ToolResult {
    ToolResult {
        content,
        details: Some(details),
        is_error: false,
        terminate: false,
        truncated: false,
        diagnostics: Vec::<ToolDiagnostic>::new(),
    }
}

/// Build an error tool result. Per the Phase 11 Error Handling policy, failure
/// branches carry structured cause info through `diagnostics`, so `details`
/// stays `None` here.
pub fn err(content: Vec<OutputContent>) -> ToolResult {
    ToolResult {
        content,
        details: None,
        is_error: true,
        terminate: false,
        truncated: false,
        diagnostics: Vec::<ToolDiagnostic>::new(),
    }
}

/// Uniform path-metadata block emitted by path-addressed tools (read/write/
/// edit) when a concrete path applies.
///
/// Shape: `{ workspace_root, path (user-facing), resolved_path, workspace_relation }`.
///
/// For inside-workspace results the absolute root is relativized away so it
/// does not leak to the model or into public NDJSON `details`: `workspace_root`
/// collapses to `"."` and `resolved_path` is stripped to its workspace-relative
/// form. `resolved` arrives already canonicalized (from `resolve_tool_path`),
/// so the root is canonicalized before stripping — on macOS the tempdir lives
/// under `/var` (a symlink to `/private/var`), and the raw and canonical
/// prefixes otherwise diverge so `strip_prefix` would fail and the absolute
/// path would leak. On Linux/Windows without such a symlink this is a no-op.
pub fn path_metadata(
    workspace_root: &Path,
    user_path: &str,
    resolved: &Path,
    relation: WorkspaceRelation,
) -> Value {
    let mut value = json!({
        "path": user_path,
        "resolved_path": resolved.to_string_lossy(),
        "workspace_relation": relation,
    });
    match relation {
        WorkspaceRelation::Inside => {
            value["workspace_root"] = json!(".");
            let root_for_strip = std::fs::canonicalize(workspace_root)
                .unwrap_or_else(|_| workspace_root.to_path_buf());
            if let Ok(rel) = resolved.strip_prefix(&root_for_strip) {
                value["resolved_path"] = json!(rel.display().to_string());
            }
        }
        _ => {
            value["workspace_root"] = json!(workspace_root.to_string_lossy());
        }
    }
    value
}

/// Bash operation-metadata block. One stable key set across the success,
/// nonzero-exit, timeout, and cancellation branches. `full_output` is included
/// only when a cap applies (omitted by 11.1; added by 11.6).
///
/// The argument count matches the Phase 11 stable operation key set one-to-one;
/// collapsing fields would lose the uniform contract across the four branches.
#[allow(clippy::too_many_arguments)]
pub fn bash_operation_metadata(
    workspace_root: &Path,
    command: &str,
    cwd: &Path,
    shell: &str,
    exit_code: Option<i32>,
    timed_out: bool,
    cancelled: bool,
    truncated: bool,
    full_output: Option<&str>,
) -> Value {
    let mut value = json!({
        "workspace_root": workspace_root.to_string_lossy(),
        "command": command,
        "cwd": cwd.to_string_lossy(),
        "shell": shell,
        "exit_code": exit_code,
        "timed_out": timed_out,
        "cancelled": cancelled,
        "truncated": truncated,
    });
    if let Some(full) = full_output {
        value["full_output"] = json!(full);
    }
    value
}

/// Workspace relation vocabulary, derived from the Phase 11 Filesystem Tool
/// Policy prose ("inside the workspace, outside the workspace, or unresolved").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceRelation {
    Inside,
    Outside,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::ToolResult;
    use opi_ai::message::OutputContent;
    use std::path::Path;

    #[test]
    fn ok_builds_success_result_with_details() {
        let r: ToolResult = ok(
            vec![OutputContent::Text { text: "hi".into() }],
            serde_json::json!({"k":1}),
        );
        assert!(!r.is_error);
        assert!(!r.terminate);
        assert!(!r.truncated);
        assert!(r.diagnostics.is_empty());
        assert_eq!(r.details, Some(serde_json::json!({"k":1})));
    }

    #[test]
    fn err_builds_error_result_without_details() {
        let r = err(vec![OutputContent::Text {
            text: "boom".into(),
        }]);
        assert!(r.is_error);
        assert!(r.details.is_none());
        assert!(!r.truncated);
        assert!(r.diagnostics.is_empty());
    }

    #[test]
    fn path_metadata_has_uniform_shape() {
        let v = path_metadata(
            Path::new("/ws"),
            "src/a.rs",
            Path::new("/ws/src/a.rs"),
            WorkspaceRelation::Inside,
        );
        let o = v.as_object().expect("object");
        assert!(o.contains_key("workspace_root"));
        assert!(o.contains_key("path"));
        assert!(o.contains_key("resolved_path"));
        assert_eq!(
            o.get("workspace_relation").and_then(|x| x.as_str()),
            Some("inside")
        );
    }

    #[test]
    fn bash_operation_metadata_has_stable_key_set_without_full_output() {
        let v = bash_operation_metadata(
            Path::new("/ws"),
            "echo hi",
            Path::new("/ws"),
            "sh",
            Some(0),
            false,
            false,
            false,
            None,
        );
        let o = v.as_object().expect("object");
        for key in [
            "workspace_root",
            "command",
            "cwd",
            "shell",
            "exit_code",
            "timed_out",
            "cancelled",
            "truncated",
        ] {
            assert!(o.contains_key(key), "missing stable bash key: {key}");
        }
        assert!(!o.contains_key("full_output"));
    }

    #[test]
    fn bash_operation_metadata_includes_full_output_and_nulls_when_provided() {
        let v = bash_operation_metadata(
            Path::new("/ws"),
            "c",
            Path::new("/ws"),
            "sh",
            None,
            true,
            false,
            true,
            Some("/tmp/full"),
        );
        assert_eq!(
            v.get("full_output").and_then(|x| x.as_str()),
            Some("/tmp/full")
        );
        assert_eq!(v.get("exit_code"), Some(&serde_json::Value::Null));
        assert_eq!(v.get("timed_out").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(v.get("cancelled").and_then(|x| x.as_bool()), Some(false));
    }

    #[test]
    fn workspace_relation_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&WorkspaceRelation::Outside).unwrap(),
            "\"outside\""
        );
    }
}
