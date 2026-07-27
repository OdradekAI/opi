//! AGENTS.md / CLAUDE.md context file discovery (task 3.7).
//!
//! Discovers context files by walking from the current working directory up
//! to the git root, then optionally checking a global config directory.
//! Deterministic precedence: nearest directory first, then ancestors, then
//! global. Per-directory order: AGENTS.md before CLAUDE.md. OPI.md is NOT
//! loaded (ADR-020).

use std::path::{Path, PathBuf};

/// Maximum size for a single context file (128 KB).
const MAX_CONTEXT_FILE_SIZE: u64 = 128 * 1024;

/// Context file names to look for, in per-directory order.
const CONTEXT_FILE_NAMES: &[&str] = &["AGENTS.md", "CLAUDE.md"];

/// Result of context file discovery.
pub struct ContextFiles {
    /// Concatenated content with directory headings and separators.
    pub content: String,
    /// Number of files successfully loaded.
    pub files_loaded: usize,
}

/// Discover and concatenate AGENTS.md and CLAUDE.md context files.
///
/// Walks from `cwd` upward to the git root (detected by `.git` presence) or
/// filesystem root, then checks `global_config_dir` if provided. Returns
/// concatenated content with per-file headings. Equivalent to
/// [`discover_context_files_with_trust`] with `include_project = true`.
pub fn discover_context_files(cwd: &Path, global_config_dir: Option<&Path>) -> ContextFiles {
    discover_context_files_with_trust(cwd, global_config_dir, true)
}

/// Trust-aware context discovery (task 15.7).
///
/// When `include_project` is `false` (an untrusted project), the workspace
/// ancestor walk is skipped and only the user-global `global_config_dir` is
/// consulted, so an untrusted project's `AGENTS.md`/`CLAUDE.md` are not
/// auto-injected into the system prompt (a prompt-injection channel). The files
/// remain readable via the `read` tool, which is ungated.
pub fn discover_context_files_with_trust(
    cwd: &Path,
    global_config_dir: Option<&Path>,
    include_project: bool,
) -> ContextFiles {
    let mut parts: Vec<String> = Vec::new();

    // Walk from cwd upward to git root (inclusive) or filesystem root.
    if include_project {
        for dir in project_candidate_directories(cwd) {
            load_dir_context(&dir, &mut parts);
        }
    }

    // Check global config dir last.
    if let Some(global_dir) = global_config_dir {
        load_dir_context(global_dir, &mut parts);
    }

    if parts.is_empty() {
        return ContextFiles {
            content: String::new(),
            files_loaded: 0,
        };
    }

    ContextFiles {
        content: parts.join("\n\n"),
        files_loaded: parts.len(),
    }
}

/// Return the exact ordered project directories consulted for local context.
///
/// The walk starts at `cwd`, stops after the nearest ancestor containing a
/// `.git` marker, and otherwise reaches the filesystem root. Project trust
/// detection uses this same list so every context file eligible for prompt
/// injection is represented in the pre-trust resource check.
pub(crate) fn project_candidate_directories(cwd: &Path) -> Vec<PathBuf> {
    let stop_at = find_git_root(cwd);
    let mut candidates = Vec::new();
    let mut current = Some(cwd);
    while let Some(dir) = current {
        candidates.push(dir.to_path_buf());
        if stop_at.as_deref() == Some(dir) {
            break;
        }
        current = dir.parent();
    }
    candidates
}

fn load_dir_context(dir: &Path, parts: &mut Vec<String>) {
    for name in CONTEXT_FILE_NAMES {
        let path = dir.join(name);
        if let Some(content) = read_context_file(&path) {
            parts.push(format!("--- {name} ---\n{content}"));
        }
    }
}

fn read_context_file(path: &Path) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;

    // Skip files exceeding the size limit
    if metadata.len() > MAX_CONTEXT_FILE_SIZE {
        return None;
    }

    let content = std::fs::read_to_string(path).ok()?;

    // Skip empty files
    if content.trim().is_empty() {
        return None;
    }

    Some(content)
}

/// Find the git root by walking upward looking for `.git`.
fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}
