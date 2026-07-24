//! Behavioral tests for the find tool (task 3.9).
//!
//! Validates gitignore-aware file discovery with pi-style argument naming,
//! path scoping, hidden-file handling, and error cases.

mod common;

use std::fs;

use common::{create_gitignore, tool_result_text};
use opi_agent::tool::{ExecutionMode, Tool};
use opi_coding_agent::tool::{FindTool, MAX_NAV_RESULTS, MAX_NAV_VISITED_ENTRIES};
use serde_json::json;
use tokio_util::sync::CancellationToken;

// --- Basic pattern matching ---

#[tokio::test]
async fn find_tool_matches_files_by_glob_pattern() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("foo.rs"), "fn main() {}").unwrap();
    fs::write(dir.path().join("bar.rs"), "fn bar() {}").unwrap();
    fs::write(dir.path().join("baz.txt"), "hello").unwrap();

    let tool = FindTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c1",
            json!({ "pattern": "**/*.rs" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(
        !result.is_error,
        "unexpected error: {}",
        tool_result_text(&result)
    );
    let text = tool_result_text(&result);
    assert!(text.contains("foo.rs"), "should find foo.rs");
    assert!(text.contains("bar.rs"), "should find bar.rs");
    assert!(!text.contains("baz.txt"), "should not find baz.txt");
}

#[tokio::test]
async fn find_tool_no_match_returns_message() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file.txt"), "data").unwrap();

    let tool = FindTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c2",
            json!({ "pattern": "*.nonexistent" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let text = tool_result_text(&result);
    assert!(
        !text.contains("file.txt"),
        "should not match unrelated files"
    );
    // Phase 11.7: a zero-match query returns a clear non-empty no-matches
    // message (never the empty string), locking find's parity with grep/glob.
    assert!(
        !text.is_empty(),
        "find zero-match must be non-empty: {text:?}"
    );
    assert!(
        text.to_lowercase().contains("no matches"),
        "find zero-match must name the cause: {text}"
    );
}

// --- Path scoping ---

#[tokio::test]
async fn find_tool_scopes_to_subdirectory() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::create_dir_all(dir.path().join("tests")).unwrap();
    fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(dir.path().join("tests/main.rs"), "#[test] fn t() {}").unwrap();

    let tool = FindTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c3",
            json!({ "pattern": "**/*.rs", "path": "src" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let text = tool_result_text(&result);
    assert!(
        text.contains("src/main.rs") || text.contains("main.rs"),
        "should find file in src subdir"
    );
    assert!(
        !text.contains("tests"),
        "should not find files outside scoped path"
    );
}

#[tokio::test]
async fn find_tool_rejects_path_traversal() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("inside.txt"), "data").unwrap();

    let tool = FindTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c4",
            json!({ "pattern": "*", "path": "../" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(
        result.is_error,
        "path traversal should be rejected: {}",
        tool_result_text(&result)
    );
    let text = tool_result_text(&result);
    assert!(
        text.contains("outside") || text.contains("workspace") || text.contains("invalid"),
        "error should explain path issue: {text}"
    );
}

// --- Gitignore awareness ---

#[tokio::test]
async fn find_tool_ignores_gitignored_dirs() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::create_dir_all(dir.path().join("target")).unwrap();
    fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(dir.path().join("target/build.rs"), "fn build() {}").unwrap();
    create_gitignore(dir.path(), "target/\n");

    let tool = FindTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c5",
            json!({ "pattern": "**/*.rs" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let text = tool_result_text(&result);
    assert!(text.contains("main.rs"), "should find src/main.rs");
    assert!(
        !text.contains("target"),
        "should not find files in gitignored target dir"
    );
}

// --- Hidden files ---

#[tokio::test]
async fn find_tool_finds_hidden_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".hidden.cfg"), "secret").unwrap();
    fs::write(dir.path().join("visible.txt"), "data").unwrap();

    let tool = FindTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c6",
            json!({ "pattern": "*" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let text = tool_result_text(&result);
    assert!(
        text.contains(".hidden.cfg"),
        "should find hidden files when not gitignored"
    );
}

// --- Invalid pattern ---

#[tokio::test]
async fn find_tool_invalid_glob_pattern_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let tool = FindTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c7",
            json!({ "pattern": "[invalid" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(
        result.is_error,
        "invalid glob pattern should be error: {}",
        tool_result_text(&result)
    );
    let text = tool_result_text(&result);
    assert!(
        text.to_lowercase().contains("pattern") || text.to_lowercase().contains("glob"),
        "error should mention pattern issue: {text}"
    );
}

// --- Invalid arguments ---

#[tokio::test]
async fn find_tool_missing_pattern_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let tool = FindTool::new(dir.path().to_path_buf());
    let result = tool
        .execute("c8", json!({}), CancellationToken::new(), None)
        .await
        .unwrap();

    assert!(
        result.is_error,
        "missing pattern should be error: {}",
        tool_result_text(&result)
    );
}

// --- Tool definition ---

#[test]
fn find_tool_has_valid_definition() {
    let tool = FindTool::new(std::path::PathBuf::from("."));
    let def = tool.definition();
    assert_eq!(def.name, "find");
    assert!(!def.description.is_empty());
    assert!(def.input_schema.is_object());
}

#[test]
fn find_tool_is_parallel() {
    let tool = FindTool::new(std::path::PathBuf::from("."));
    assert_eq!(tool.execution_mode(), ExecutionMode::Parallel);
}

// --- Details metadata ---

#[tokio::test]
async fn find_tool_includes_details_metadata() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.rs"), "").unwrap();
    fs::write(dir.path().join("b.rs"), "").unwrap();

    let tool = FindTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c9",
            json!({ "pattern": "*.rs" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let details = result.details.as_ref().expect("should have details");
    assert!(
        details.get("match_count").is_some(),
        "details should include match_count"
    );
    assert_eq!(
        details.get("workspace_relation").and_then(|v| v.as_str()),
        Some("inside"),
        "find details should include workspace_relation"
    );
}

// --- Non-UTF-8 entry names (Phase 11.2) ---
// macOS rejects non-UTF-8 filenames at creation (errno EILSEQ), so this path
// is gated to non-macOS Unix, whose byte-oriented filesystems allow such names.

#[cfg(all(unix, not(target_os = "macos")))]
#[tokio::test]
async fn find_tool_reports_unsupported_encoding_for_non_utf8_names() {
    use opi_agent::diagnostic::code;
    use std::os::unix::ffi::OsStrExt;
    let dir = tempfile::tempdir().unwrap();
    let bad = std::ffi::OsStr::from_bytes(b"bad\xff.rs");
    fs::write(dir.path().join(bad), "x").unwrap();
    fs::write(dir.path().join("good.rs"), "y").unwrap();

    let find = FindTool::new(dir.path().to_path_buf());
    let result = find
        .execute(
            "f-uni-bad-1",
            json!({ "pattern": "*.rs" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!result.is_error, "{}", tool_result_text(&result));
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == code::CODE_TOOL_UNSUPPORTED_ENCODING),
        "non-UTF-8 name should yield unsupported_encoding diagnostic: {:?}",
        result.diagnostics
    );
    let text = tool_result_text(&result);
    assert!(text.contains("good.rs"));
    assert!(
        !text.contains('\u{FFFD}'),
        "no lossy U+FFFD in output: {text}"
    );
}

// --- Filesystem error taxonomy: path validation (Phase 11.2) ---

#[tokio::test]
async fn find_tool_scope_path_not_found_is_error() {
    use opi_agent::diagnostic::code;
    let dir = tempfile::tempdir().unwrap();
    let find = FindTool::new(dir.path().to_path_buf());
    let result = find
        .execute(
            "f-nf-1",
            json!({ "pattern": "*.rs", "path": "missing_dir" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(result.is_error);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == code::CODE_TOOL_PATH_NOT_FOUND),
        "missing scope path should carry tool_path_not_found: {:?}",
        result.diagnostics
    );
}

#[tokio::test]
async fn find_tool_file_scope_is_not_a_directory() {
    use opi_agent::diagnostic::code;
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file.txt"), "x").unwrap();
    let find = FindTool::new(dir.path().to_path_buf());
    let result = find
        .execute(
            "f-nd-1",
            json!({ "pattern": "*.rs", "path": "file.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(result.is_error);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == code::CODE_TOOL_NOT_A_DIRECTORY),
        "file scope should carry tool_not_a_directory: {:?}",
        result.diagnostics
    );
}

#[cfg(unix)]
#[tokio::test]
async fn find_tool_permission_denied_scope_is_classified() {
    unsafe extern "C" {
        fn getuid() -> u32;
    }
    if unsafe { getuid() } == 0 {
        eprintln!("skipping permission test under root");
        return;
    }
    use opi_agent::diagnostic::code;
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let locked = dir.path().join("locked");
    fs::create_dir(&locked).unwrap();
    fs::write(locked.join("f.rs"), "x").unwrap();
    fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let find = FindTool::new(dir.path().to_path_buf());
    let result = find
        .execute(
            "f-pd-1",
            json!({ "pattern": "*.rs", "path": "locked" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    let _ = fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755));
    assert!(result.is_error);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == code::CODE_TOOL_PERMISSION_DENIED),
        "unreadable scope should carry tool_permission_denied: {:?}",
        result.diagnostics
    );
}

// ---------------------------------------------------------------------------
// Phase 11.7: read-only navigation tools consistency (find variants)
// ---------------------------------------------------------------------------

/// find honors a NESTED `.gitignore` (the only rule for `secret.txt` lives in
/// `sub/`), matching grep/glob/ls.
#[tokio::test]
async fn nested_ignore_consistent_across_nav_tools() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".gitignore"), "unrelated_pattern_xyz\n").unwrap();
    fs::create_dir_all(dir.path().join("sub")).unwrap();
    fs::write(dir.path().join("sub/.gitignore"), "secret.txt\n").unwrap();
    fs::write(dir.path().join("sub/secret.txt"), "x").unwrap();
    fs::write(dir.path().join("sub/keep.txt"), "x").unwrap();

    let find = FindTool::new(dir.path().to_path_buf())
        .execute(
            "ni-f1",
            json!({ "pattern": "sub/*.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    let text = tool_result_text(&find);
    assert!(!find.is_error, "{}", text);
    assert!(text.contains("keep.txt"));
    assert!(
        !text.contains("secret.txt"),
        "find must honor nested .gitignore: {text}"
    );
}

/// find emits relative paths in strict lexicographic order.
#[tokio::test]
async fn nav_tools_emit_sorted_results() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["c.rs", "a.rs", "b.rs"] {
        fs::write(dir.path().join(name), "x").unwrap();
    }
    let find = FindTool::new(dir.path().to_path_buf())
        .execute(
            "so-f1",
            json!({ "pattern": "*.rs" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!find.is_error);
    let text = tool_result_text(&find);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines,
        vec!["a.rs", "b.rs", "c.rs"],
        "find sorted relative: {text}"
    );
    for l in &lines {
        assert!(
            !l.starts_with('/') && !l.starts_with('\\') && !l.contains(':'),
            "find must emit relative paths only: {l}"
        );
    }
}

#[tokio::test]
async fn find_bounds_collection_before_walking_everything() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..(MAX_NAV_RESULTS + 20) {
        fs::write(dir.path().join(format!("f_{i:04}.txt")), "x").unwrap();
    }

    let find = FindTool::new(dir.path().to_path_buf())
        .execute(
            "find-bound-1",
            json!({ "pattern": "f_*.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!find.is_error, "{}", tool_result_text(&find));
    assert!(find.truncated);
    let details = find.details.as_ref().expect("details");
    assert_eq!(
        details
            .get("search_terminated_early")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert!(
        details
            .get("visited_entries")
            .and_then(|v| v.as_u64())
            .is_some_and(|count| count <= MAX_NAV_VISITED_ENTRIES as u64),
        "visited_entries should be bounded: {details}"
    );
}

#[tokio::test]
async fn find_early_termination_without_matches_is_not_reported_as_no_matches() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..(MAX_NAV_VISITED_ENTRIES + 1) {
        fs::write(dir.path().join(format!("f_{i:05}.txt")), "x").unwrap();
    }

    let find = FindTool::new(dir.path().to_path_buf())
        .execute(
            "find-early-no-match",
            json!({ "pattern": "*.rs" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!find.is_error, "{}", tool_result_text(&find));
    assert!(find.truncated);
    let text = tool_result_text(&find).to_lowercase();
    assert!(
        text.contains("terminated before completing"),
        "early termination must be visible in provider-facing text: {text}"
    );
    assert!(
        !text.contains("no matches"),
        "early termination is not a complete no-match result: {text}"
    );
}
/// find honors the CancellationToken mid-walk (pre-cancelled -> zero matches).
#[tokio::test]
async fn nav_tools_honour_cancellation_token() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..(MAX_NAV_RESULTS + 200) {
        fs::write(dir.path().join(format!("f_{i:04}.txt")), "x").unwrap();
    }
    let token = CancellationToken::new();
    token.cancel();
    let find = FindTool::new(dir.path().to_path_buf())
        .execute("cn-f1", json!({ "pattern": "f_*.txt" }), token, None)
        .await
        .unwrap();
    assert!(!find.is_error);
    let details = find.details.as_ref().expect("details");
    assert_eq!(
        details.get("cancelled").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        details.get("match_count").and_then(|v| v.as_u64()),
        Some(0),
        "pre-cancelled walk must match zero files"
    );
}
