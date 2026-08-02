mod bash;
mod edit;
mod find;
mod glob;
mod grep;
mod ls;
mod operations;
pub(crate) mod process_tree;
mod read;
pub(crate) mod supervision;
mod write;

pub use bash::{BashTool, default_bash_schema, with_model_backend_enum};
pub use edit::EditTool;
pub use find::FindTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use ls::LsTool;
pub use operations::{
    AccessMode, BashOpError, BashOperations, BashRequest, BashResult, FileOperations, FsOpError,
    LOCAL_BASH_OPERATION_DIAGNOSTIC, LocalBashOperations, LocalFileOperations,
    MAX_BASH_OUTPUT_BYTES, OpMetadata, ToolDiagnostic,
};
pub use process_tree::{AttachError, TerminationOutcome, TreeGuard, configure_tree};
pub use read::{MAX_READ_OUTPUT_BYTES, ReadTool};
pub use write::WriteTool;

use std::path::{Path, PathBuf};

use opi_agent::diagnostic::FsToolError;
use opi_agent::tool::result::WorkspaceRelation;
use opi_agent::tool::{ToolResult, result};
use opi_ai::message::OutputContent;

/// Path boundary policy for file tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPolicy {
    WorkspaceOnly,
    AllowOutsideWorkspace,
}

/// Resolved path metadata shared by file tools.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedToolPath {
    /// Canonical, verbatim-stripped absolute path used for filesystem operations
    /// and user-facing display.
    pub path: PathBuf,
    pub workspace_relation: WorkspaceRelation,
    /// `true` when resolution followed a symlink/junction (canonical form
    /// differs from the lexical path). Lets callers report traversal rather than
    /// silently collapsing it to a workspace-boundary result.
    pub symlink_traversed: bool,
}

/// Resolve a user-supplied file path for tool execution.
///
/// Relative paths are based on `workspace_root`; absolute paths are preserved.
/// A leading `@` is ignored for editor-style path mentions, and `~` expands
/// from HOME/USERPROFILE when available. The canonical path is stripped of the
/// Windows verbatim (`\\?\`) prefix so it does not leak into user-facing
/// metadata. Symlink/junction traversal is detected and reported via
/// [`ResolvedToolPath::symlink_traversed`]. Returns a typed [`FsToolError`] for
/// workspace-boundary and unresolved-root causes so each carries a distinct
/// `CODE_TOOL_*` diagnostic instead of a generic string.
pub fn resolve_tool_path(
    workspace_root: &Path,
    user_path: &str,
    policy: PathPolicy,
) -> Result<ResolvedToolPath, FsToolError> {
    let expanded = expand_user_path(user_path);
    let canonical_root = match std::fs::canonicalize(workspace_root) {
        Ok(root) => strip_verbatim_prefix(&root),
        Err(e) => {
            return Err(FsToolError::UnresolvedWorkspaceRoot {
                source: e.to_string(),
            });
        }
    };
    let lexical_abs = if expanded.is_absolute() {
        strip_verbatim_prefix(&expanded)
    } else {
        canonical_root.join(&expanded)
    };
    let lexical = normalize_path_components(&lexical_abs);
    let canonical = canonicalize_existing_or_nearest(&lexical_abs)
        .map(|c| strip_verbatim_prefix(&c))
        .map_err(|e| FsToolError::UnresolvedWorkspaceRoot { source: e })?;
    // A symlink/junction traversal makes the canonical path diverge from the
    // lexical path. On case-insensitive filesystems (Windows NTFS) a bare casing
    // difference would also diverge, so compare case-insensitively there to avoid
    // false-positive traversal reports; case-sensitive hosts use exact equality.
    let symlink_traversed = paths_diverge_indicating_traversal(&canonical, &lexical);
    let inside_workspace = canonical.starts_with(&canonical_root);

    if policy == PathPolicy::WorkspaceOnly && !inside_workspace {
        return Err(FsToolError::OutsideWorkspace {
            user_path: user_path.to_string(),
            symlink_traversed,
        });
    }

    Ok(ResolvedToolPath {
        path: canonical,
        workspace_relation: if inside_workspace {
            WorkspaceRelation::Inside
        } else {
            WorkspaceRelation::Outside
        },
        symlink_traversed,
    })
}

pub fn validate_workspace_path(
    workspace_root: &Path,
    user_path: &str,
) -> Result<PathBuf, FsToolError> {
    resolve_tool_path(workspace_root, user_path, PathPolicy::WorkspaceOnly).map(|r| r.path)
}

/// Strip the Windows verbatim (`\\?\`) prefix from a canonical path so it does
/// not leak into user-facing metadata. No-op on non-Windows hosts.
#[cfg(windows)]
fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path.to_path_buf()
    }
}

#[cfg(not(windows))]
fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    path.to_path_buf()
}

/// Decide whether canonical-vs-lexical divergence reflects an actual
/// symlink/junction traversal (true) rather than mere case-folding (false).
#[cfg(windows)]
fn paths_diverge_indicating_traversal(canonical: &Path, lexical: &Path) -> bool {
    canonical.to_string_lossy().to_lowercase() != lexical.to_string_lossy().to_lowercase()
}

#[cfg(not(windows))]
fn paths_diverge_indicating_traversal(canonical: &Path, lexical: &Path) -> bool {
    canonical != lexical
}

/// Build an error [`ToolResult`] for a filesystem/tool cause, carrying the
/// per-cause diagnostic so the agent loop can lift it into Phase 7 traces.
pub fn fs_error_result(error: FsToolError) -> ToolResult {
    let text = error.message();
    let mut result = result::err(vec![OutputContent::Text { text }]);
    result.diagnostics.push(error.to_diagnostic());
    result
}

/// Best-effort sibling temp-file cleanup guard for atomic write/edit paths.
///
/// Known error branches call [`TempFileGuard::cleanup`] so cleanup stays async.
/// The `Drop` fallback covers future cancellation or early-return paths between
/// staging and rename; Drop cannot await, so it uses synchronous removal.
pub(crate) struct TempFileGuard {
    path: PathBuf,
    armed: bool,
}

impl TempFileGuard {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) async fn cleanup(&mut self) {
        if self.armed {
            let _ = tokio::fs::remove_file(&self.path).await;
            self.armed = false;
        }
    }

    pub(crate) fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// Shared `ignore::WalkBuilder` configuration for the four read-only navigation
/// tools (grep/find/ls/glob), so ignore-file handling, hidden-file defaults,
/// and no-follow symlink behavior are identical across them (Phase 11.7).
///
/// `hidden(false)` keeps dotfiles visible (matching prior behavior);
/// `git_ignore(true)` honors nested `.gitignore` files (ls previously
/// hand-rolled a root-only matcher); `follow_links` stays at the builder
/// default of `false` so symlinks are reported as entries but never traversed,
/// uniformly across all four tools (the Phase 11.7 symlink-consistency fix).
pub(crate) fn nav_walk_builder(root: &std::path::Path) -> ignore::WalkBuilder {
    let mut builder = ignore::WalkBuilder::new(root);
    builder
        .hidden(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .add_custom_ignore_filename(".gitignore")
        .sort_by_file_path(|a, b| a.cmp(b));
    builder
}

/// Default cap on the number of results grep/find/glob return inline. Results
/// beyond this are dropped with `truncated` set and an `omitted_count` in
/// details. ls keeps its own `max_entries` override (`ls::DEFAULT_MAX_ENTRIES`);
/// the values match but the policies differ, so the constants are separate.
/// Exposed as `pub` so behavioral tests can build fixtures sized to the cap.
pub const MAX_NAV_RESULTS: usize = 200;

/// Upper bound on read-only navigation work before grep/find/glob/ls stop
/// walking. When this cap is hit, `search_terminated_early` is true and
/// `omitted_count` is a known lower bound rather than an exact total.
pub const MAX_NAV_VISITED_ENTRIES: usize = 10_000;

/// Upper bound on cumulative file bytes grep will read during one search.
pub const MAX_GREP_TOTAL_READ_BYTES: u64 = 8 * 1024 * 1024;

/// Upper bound on the size of a single file grep will read. Files whose
/// metadata length exceeds this are skipped (counted in
/// `details.files_oversized_skipped`) so one giant file cannot dominate memory
/// or time. find/glob/ls do not read file content and are bounded by the
/// result-count cap instead. Exposed as `pub` for behavioral tests.
pub const MAX_NAV_FILE_BYTES: u64 = 1 << 20; // 1 MiB

fn expand_user_path(user_path: &str) -> PathBuf {
    let path = user_path.strip_prefix('@').unwrap_or(user_path);
    if path == "~" {
        return home_dir().unwrap_or_else(|| PathBuf::from(path));
    }
    if let Some(rest) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\"))
        && let Some(home) = home_dir()
    {
        return home.join(rest);
    }
    PathBuf::from(path)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn canonicalize_existing_or_nearest(path: &Path) -> Result<PathBuf, String> {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return Ok(canonical);
    }

    // Path doesn't exist, so canonicalize the nearest existing ancestor.
    // Preserve the original lexical suffix relative to that ancestor, then
    // normalize after joining so `..` segments are applied consistently.
    let mut ancestor = path;
    while let Some(parent) = ancestor.parent() {
        if let Ok(canonical_ancestor) = std::fs::canonicalize(parent) {
            let suffix = path.strip_prefix(parent).unwrap_or_else(|_| Path::new(""));
            return Ok(normalize_path_components(&canonical_ancestor.join(suffix)));
        }
        ancestor = parent;
    }

    Ok(normalize_path_components(path))
}

/// Resolve `.` and `..` components without touching the filesystem.
fn normalize_path_components(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => {}
            c => normalized.push(c.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_user_path_strips_at_prefix() {
        let path = expand_user_path("@Cargo.toml");
        assert_eq!(path, PathBuf::from("Cargo.toml"));
    }

    #[test]
    fn normalize_path_components_removes_parent_segments() {
        let path = normalize_path_components(Path::new("/tmp/a/../b"));
        assert!(path.ends_with(Path::new("tmp").join("b")));
    }

    #[test]
    fn canonicalize_existing_or_nearest_normalizes_missing_suffix_parent_segments() {
        let workspace = tempfile::tempdir().unwrap();
        let path = workspace.path().join("missing/child/../../target.txt");
        let resolved = canonicalize_existing_or_nearest(&path).unwrap();

        assert_eq!(
            resolved,
            std::fs::canonicalize(workspace.path())
                .unwrap()
                .join("target.txt")
        );
    }

    use opi_agent::diagnostic::code;

    #[test]
    fn resolve_outside_workspace_returns_typed_outside_error() {
        let workspace = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let outside_file = outside.path().join("escape.txt");
        std::fs::write(&outside_file, "x").unwrap();
        let abs = outside_file.to_string_lossy().to_string();
        let err = resolve_tool_path(workspace.path(), &abs, PathPolicy::WorkspaceOnly)
            .expect_err("outside path must be denied");
        assert_eq!(err.code(), code::CODE_TOOL_OUTSIDE_WORKSPACE);
        assert!(err.message().contains("outside the workspace"));
    }

    #[test]
    fn resolve_unresolved_workspace_root_returns_typed_error() {
        let bogus = std::env::temp_dir().join("opi-11-2-nonexistent-root-zzz");
        let _ = std::fs::remove_dir_all(&bogus);
        let err = resolve_tool_path(&bogus, "anything.txt", PathPolicy::WorkspaceOnly)
            .expect_err("nonexistent root must fail");
        assert_eq!(err.code(), code::CODE_TOOL_UNRESOLVED_WORKSPACE_ROOT);
    }

    fn make_dir_link(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(dst, src)
        }
        #[cfg(windows)]
        {
            let status = std::process::Command::new("cmd")
                .args([
                    "/C",
                    "mklink",
                    "/J",
                    &src.to_string_lossy(),
                    &dst.to_string_lossy(),
                ])
                .status()?;
            status
                .success()
                .then_some(())
                .ok_or_else(|| std::io::Error::other("mklink /J failed"))
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (src, dst);
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "symlinks unsupported",
            ))
        }
    }

    #[test]
    fn resolve_reports_symlink_or_junction_traversal() {
        let workspace = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let link = workspace.path().join("link");
        if let Err(e) = make_dir_link(&link, outside.path()) {
            eprintln!("skipping symlink traversal test; link creation failed: {e}");
            return;
        }
        let resolved =
            resolve_tool_path(workspace.path(), "link", PathPolicy::AllowOutsideWorkspace)
                .expect("link should resolve");
        assert!(
            resolved.symlink_traversed,
            "symlink/junction traversal must be reported"
        );
        assert_eq!(resolved.workspace_relation, WorkspaceRelation::Outside);
    }

    #[test]
    fn resolve_outside_workspace_via_symlink_reports_traversal() {
        let workspace = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let link = workspace.path().join("link");
        if let Err(e) = make_dir_link(&link, outside.path()) {
            eprintln!("skipping symlink-escape test; link creation failed: {e}");
            return;
        }
        let err = resolve_tool_path(workspace.path(), "link", PathPolicy::WorkspaceOnly)
            .expect_err("escape should be denied");
        assert_eq!(err.code(), code::CODE_TOOL_OUTSIDE_WORKSPACE);
        let ctx = err.context();
        assert_eq!(
            ctx.get("symlink_traversed").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[cfg(windows)]
    #[test]
    fn resolve_strips_windows_verbatim_prefix() {
        let workspace = tempfile::tempdir().unwrap();
        std::fs::write(workspace.path().join("f.txt"), "x").unwrap();
        let resolved =
            resolve_tool_path(workspace.path(), "f.txt", PathPolicy::WorkspaceOnly).unwrap();
        let s = resolved.path.to_string_lossy().to_string();
        assert!(
            !s.contains(r"\\?\"),
            "verbatim prefix leaked into resolved path: {s}"
        );
    }
}
