//! Behavioral tests for the ls tool (task 3.9).
//!
//! Validates directory listing with bounded output, deterministic ordering,
//! hidden-file handling, path traversal rejection, and error cases.

mod common;

use std::fs;

use common::tool_result_text;
use opi_agent::diagnostic::code;
use opi_agent::tool::{ExecutionMode, Tool};
use opi_coding_agent::tool::{LsTool, MAX_NAV_RESULTS, MAX_NAV_VISITED_ENTRIES};
use serde_json::json;
use tokio_util::sync::CancellationToken;

// --- Basic listing ---

#[tokio::test]
async fn ls_tool_lists_directory_contents() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file1.txt"), "a").unwrap();
    fs::write(dir.path().join("file2.rs"), "b").unwrap();
    fs::create_dir(dir.path().join("subdir")).unwrap();

    let tool = LsTool::new(dir.path().to_path_buf());
    let result = tool
        .execute("c1", json!({ "path": "." }), CancellationToken::new(), None)
        .await
        .unwrap();

    assert!(
        !result.is_error,
        "unexpected error: {}",
        tool_result_text(&result)
    );
    let text = tool_result_text(&result);
    assert!(text.contains("file1.txt"), "should list file1.txt");
    assert!(text.contains("file2.rs"), "should list file2.rs");
    assert!(text.contains("subdir"), "should list subdir");
}

// --- Deterministic ordering ---

#[tokio::test]
async fn ls_tool_lists_in_deterministic_order() {
    let dir = tempfile::tempdir().unwrap();
    // Create files in reverse alphabetical order
    for name in &["c.txt", "a.txt", "b.txt"] {
        fs::write(dir.path().join(name), "").unwrap();
    }

    let tool = LsTool::new(dir.path().to_path_buf());
    let result = tool
        .execute("c2", json!({ "path": "." }), CancellationToken::new(), None)
        .await
        .unwrap();

    assert!(!result.is_error);
    let text = tool_result_text(&result);
    let pos_a = text.find("a.txt").expect("should contain a.txt");
    let pos_b = text.find("b.txt").expect("should contain b.txt");
    let pos_c = text.find("c.txt").expect("should contain c.txt");
    assert!(
        pos_a < pos_b && pos_b < pos_c,
        "entries should be in sorted order: a < b < c"
    );
}

// --- Hidden files ---

#[tokio::test]
async fn ls_tool_shows_hidden_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".hidden"), "secret").unwrap();
    fs::write(dir.path().join("visible.txt"), "data").unwrap();

    let tool = LsTool::new(dir.path().to_path_buf());
    let result = tool
        .execute("c3", json!({ "path": "." }), CancellationToken::new(), None)
        .await
        .unwrap();

    assert!(!result.is_error);
    let text = tool_result_text(&result);
    assert!(text.contains(".hidden"), "should list hidden files");
    assert!(text.contains("visible.txt"), "should list visible files");
}

// --- Subdirectory listing ---

#[tokio::test]
async fn ls_tool_lists_subdirectory() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("nested/deep")).unwrap();
    fs::write(dir.path().join("nested/a.txt"), "").unwrap();
    fs::write(dir.path().join("nested/b.txt"), "").unwrap();
    fs::write(dir.path().join("root.txt"), "").unwrap();

    let tool = LsTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c4",
            json!({ "path": "nested" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let text = tool_result_text(&result);
    assert!(text.contains("a.txt"), "should list files in subdirectory");
    assert!(text.contains("b.txt"), "should list files in subdirectory");
    assert!(
        !text.contains("root.txt"),
        "should not include files from parent directory"
    );
}

// --- Path traversal rejection ---

#[tokio::test]
async fn ls_tool_rejects_path_traversal() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("inside.txt"), "data").unwrap();

    let tool = LsTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c5",
            json!({ "path": "../" }),
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

// --- Nonexistent directory ---

#[tokio::test]
async fn ls_tool_nonexistent_path_is_error() {
    let dir = tempfile::tempdir().unwrap();

    let tool = LsTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c6",
            json!({ "path": "does_not_exist" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(
        result.is_error,
        "nonexistent path should be error: {}",
        tool_result_text(&result)
    );
}

// --- Bounded output: max_entries ---

#[tokio::test]
async fn ls_tool_truncates_at_max_entries() {
    let dir = tempfile::tempdir().unwrap();
    // Create more files than the limit
    for i in 0..20 {
        fs::write(dir.path().join(format!("file_{:02}.txt", i)), "").unwrap();
    }

    let tool = LsTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c7",
            json!({ "path": ".", "max_entries": 5 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let text = tool_result_text(&result);
    // Should have at most 5 entries plus possibly a truncation notice
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| !l.contains("truncated") && !l.is_empty())
        .collect();
    assert!(
        lines.len() <= 5,
        "should have at most 5 entries, got {}: {text}",
        lines.len()
    );
}

// --- Bounded output: max_depth (Phase 11.7: pins the WalkBuilder max_depth+1
//     mapping after ls switched off hand-rolled recursion) ---

#[tokio::test]
async fn ls_tool_respects_max_depth() {
    let dir = tempfile::tempdir().unwrap();
    // Depths relative to workspace root: a/ =1, a/a1.txt =2, a/b/b1.txt =3,
    // a/b/c/c1.txt =4. ls max_depth=N must include entries down to depth N+1
    // (WalkBuilder depth is inclusive with root=0, so ls uses max_depth(N+1)
    // and skips the depth-0 root).
    fs::create_dir_all(dir.path().join("a/b/c")).unwrap();
    fs::write(dir.path().join("a/a1.txt"), "").unwrap();
    fs::write(dir.path().join("a/b/b1.txt"), "").unwrap();
    fs::write(dir.path().join("a/b/c/c1.txt"), "").unwrap();
    let tool = LsTool::new(dir.path().to_path_buf());

    // max_depth=0: only immediate children of root (a/), nothing deeper.
    let t0 = tool_result_text(
        &tool
            .execute(
                "d0",
                json!({ "path": ".", "max_depth": 0 }),
                CancellationToken::new(),
                None,
            )
            .await
            .unwrap(),
    );
    assert!(t0.contains("a"), "depth 0 lists immediate child a/: {t0}");
    assert!(!t0.contains("a1.txt"), "depth 0 must not recurse: {t0}");
    assert!(!t0.contains("c1.txt"));

    // max_depth=1: a/, a/a1.txt, a/b/, but not b1.txt or c1.txt.
    let t1 = tool_result_text(
        &tool
            .execute(
                "d1",
                json!({ "path": ".", "max_depth": 1 }),
                CancellationToken::new(),
                None,
            )
            .await
            .unwrap(),
    );
    assert!(t1.contains("a1.txt"), "depth 1 includes a/a1.txt: {t1}");
    assert!(
        !t1.contains("b1.txt"),
        "depth 1 must exclude a/b/b1.txt: {t1}"
    );
    assert!(!t1.contains("c1.txt"));

    // max_depth=2: + a/b/b1.txt, a/b/c/, but not c1.txt.
    let t2 = tool_result_text(
        &tool
            .execute(
                "d2",
                json!({ "path": ".", "max_depth": 2 }),
                CancellationToken::new(),
                None,
            )
            .await
            .unwrap(),
    );
    assert!(t2.contains("b1.txt"), "depth 2 includes a/b/b1.txt: {t2}");
    assert!(
        !t2.contains("c1.txt"),
        "depth 2 must exclude a/b/c/c1.txt: {t2}"
    );

    // max_depth=3: + a/b/c/c1.txt.
    let t3 = tool_result_text(
        &tool
            .execute(
                "d3",
                json!({ "path": ".", "max_depth": 3 }),
                CancellationToken::new(),
                None,
            )
            .await
            .unwrap(),
    );
    assert!(t3.contains("c1.txt"), "depth 3 includes a/b/c/c1.txt: {t3}");
}

// --- Missing arguments ---

#[tokio::test]
async fn ls_tool_missing_path_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let tool = LsTool::new(dir.path().to_path_buf());
    let result = tool
        .execute("c9", json!({}), CancellationToken::new(), None)
        .await
        .unwrap();

    assert!(
        result.is_error,
        "missing path should be error: {}",
        tool_result_text(&result)
    );
}

// --- Tool definition ---

#[test]
fn ls_tool_has_valid_definition() {
    let tool = LsTool::new(std::path::PathBuf::from("."));
    let def = tool.definition();
    assert_eq!(def.name, "ls");
    assert!(!def.description.is_empty());
    assert!(def.input_schema.is_object());
}

#[test]
fn ls_tool_is_parallel() {
    let tool = LsTool::new(std::path::PathBuf::from("."));
    assert_eq!(tool.execution_mode(), ExecutionMode::Parallel);
}

// --- Details metadata ---

#[tokio::test]
async fn ls_tool_includes_details_metadata() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "").unwrap();
    fs::write(dir.path().join("b.txt"), "").unwrap();

    let tool = LsTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c10",
            json!({ "path": "." }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let details = result.details.as_ref().expect("should have details");
    assert!(
        details.get("entry_count").is_some(),
        "details should include entry_count"
    );
    assert_eq!(
        details.get("workspace_relation").and_then(|v| v.as_str()),
        Some("inside"),
        "ls details should include workspace_relation"
    );
}

// --- Gitignored entries ---

#[tokio::test]
async fn ls_tool_excludes_gitignored_entries() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".gitignore"), "target/\n").unwrap();
    fs::write(dir.path().join("visible.txt"), "data").unwrap();
    fs::create_dir_all(dir.path().join("target")).unwrap();
    fs::write(dir.path().join("target/secret.txt"), "build").unwrap();

    let tool = LsTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c11",
            json!({ "path": "." }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let text = tool_result_text(&result);
    assert!(
        text.contains("visible.txt"),
        "should list non-ignored files"
    );
    assert!(
        !text.contains("secret.txt"),
        "should not list gitignored files"
    );
}

// --- Entry type indication ---

#[tokio::test]
async fn ls_tool_distinguishes_dirs_and_files() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("mydir")).unwrap();
    fs::write(dir.path().join("myfile.txt"), "data").unwrap();

    let tool = LsTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c12",
            json!({ "path": "." }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let text = tool_result_text(&result);
    // Directories should be indicated (trailing / or explicit marker)
    assert!(
        text.contains("mydir") && (text.contains("mydir/") || text.contains("dir")),
        "directories should be distinguishable: {text}"
    );
}

// --- Truncation metadata correctness ---

#[tokio::test]
async fn ls_tool_truncation_shows_correct_omitted_count() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..20 {
        fs::write(dir.path().join(format!("file_{:02}.txt", i)), "").unwrap();
    }

    let tool = LsTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c13",
            json!({ "path": ".", "max_entries": 5 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let text = tool_result_text(&result);
    // Should say "15 entries omitted" (20 total - 5 shown = 15 omitted)
    assert!(
        text.contains("15 entries omitted"),
        "expected '15 entries omitted' in output, got: {text}"
    );
    // Details should have correct counts
    let details = result.details.as_ref().unwrap();
    assert_eq!(details["entry_count"], 5);
    assert_eq!(details["total_entries"], 20);
    assert_eq!(details["truncated"], true);
}

#[tokio::test]
async fn ls_reports_omitted_count_when_truncated() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..(MAX_NAV_RESULTS + 3) {
        fs::write(dir.path().join(format!("f{i:04}.txt")), "x").unwrap();
    }
    let tool = LsTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "ls-truncated",
            json!({ "path": "." }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(result.truncated);
    assert_eq!(result.details.as_ref().unwrap()["omitted_count"], json!(3));
}

#[tokio::test]
async fn ls_bounds_collection_before_walking_everything() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..(MAX_NAV_RESULTS + 20) {
        fs::write(dir.path().join(format!("f_{i:04}.txt")), "x").unwrap();
    }

    let ls = LsTool::new(dir.path().to_path_buf());
    let result = ls
        .execute(
            "ls-bound-1",
            json!({ "path": "." }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error, "{}", tool_result_text(&result));
    assert!(result.truncated);
    let details = result.details.as_ref().expect("details");
    assert_eq!(
        details
            .get("search_terminated_early")
            .and_then(|v| v.as_bool()),
        Some(false)
    );
    assert!(
        details
            .get("omitted_count")
            .and_then(|v| v.as_u64())
            .is_some_and(|count| count >= 20),
        "completed traversal should count omitted entries: {details}"
    );
    assert!(
        details
            .get("visited_entries")
            .and_then(|v| v.as_u64())
            .is_some_and(|count| count <= MAX_NAV_VISITED_ENTRIES as u64),
        "visited_entries should be bounded: {details}"
    );
}
// --- Filesystem error taxonomy (Phase 11.2) ---

#[tokio::test]
async fn filesystem_error_taxonomy_directory_failures() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file.txt"), "x").unwrap();
    let ls = LsTool::new(dir.path().to_path_buf());

    // NotFound: missing directory carries tool_path_not_found.
    let missing = ls
        .execute(
            "dtax-1",
            json!({ "path": "nope" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(missing.is_error);
    assert!(
        missing
            .diagnostics
            .iter()
            .any(|d| d.code == code::CODE_TOOL_PATH_NOT_FOUND),
        "missing directory should carry tool_path_not_found: {:?}",
        missing.diagnostics
    );

    // NotADirectory: listing a file carries tool_not_a_directory.
    let file_ls = ls
        .execute(
            "dtax-2",
            json!({ "path": "file.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(file_ls.is_error);
    assert!(
        file_ls
            .diagnostics
            .iter()
            .any(|d| d.code == code::CODE_TOOL_NOT_A_DIRECTORY),
        "listing a file should carry tool_not_a_directory: {:?}",
        file_ls.diagnostics
    );
    assert!(tool_result_text(&file_ls).contains("not a directory"));
}

#[tokio::test]
async fn unicode_directory_metadata_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let dirname = "unicode_dir_日本語";
    fs::create_dir(dir.path().join(dirname)).unwrap();
    fs::write(dir.path().join(dirname).join("f.txt"), "x").unwrap();
    let ls = LsTool::new(dir.path().to_path_buf());
    let result = ls
        .execute(
            "uni-d-1",
            json!({ "path": dirname }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!result.is_error, "{}", tool_result_text(&result));
    let details = result.details.as_ref().expect("ls details");
    assert_eq!(
        details.get("path").and_then(|v| v.as_str()),
        Some(dirname),
        "unicode directory name must round-trip in details.path"
    );
    assert!(tool_result_text(&result).contains("f.txt"));
    assert!(
        !tool_result_text(&result).contains('\u{FFFD}'),
        "no lossy U+FFFD in output: {}",
        tool_result_text(&result)
    );
}

#[cfg(unix)]
#[tokio::test]
async fn ls_tool_permission_denied_target_is_classified() {
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
    fs::write(locked.join("f.txt"), "x").unwrap();
    fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let ls = LsTool::new(dir.path().to_path_buf());
    let result = ls
        .execute(
            "pd-ls-1",
            json!({ "path": "locked" }),
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
        "unreadable target should carry tool_permission_denied: {:?}",
        result.diagnostics
    );
}

// --- Non-UTF-8 entry names (Phase 11.2, Unix-only) ---

#[cfg(unix)]
#[tokio::test]
async fn ls_tool_reports_unsupported_encoding_for_non_utf8_names() {
    use std::os::unix::ffi::OsStrExt;
    let dir = tempfile::tempdir().unwrap();
    // 0xFF is invalid UTF-8.
    let bad = std::ffi::OsStr::from_bytes(b"bad\xff.txt");
    fs::write(dir.path().join(bad), "x").unwrap();
    fs::write(dir.path().join("good.txt"), "y").unwrap();

    let ls = LsTool::new(dir.path().to_path_buf());
    let result = ls
        .execute(
            "uni-bad-1",
            json!({ "path": "." }),
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
    assert!(text.contains("good.txt"));
    assert!(
        !text.contains('\u{FFFD}'),
        "no lossy U+FFFD in output: {text}"
    );
}

// ---------------------------------------------------------------------------
// Phase 11.7: read-only navigation tools consistency (ls variants)
// ---------------------------------------------------------------------------

/// ls honors a NESTED `.gitignore` (the only rule for `secret.txt` lives in
/// `sub/`), matching grep/find/glob. Pre-11.7 ls hand-rolled a root-only
/// `GitignoreBuilder` and leaked such files; this fixture catches that bug.
#[tokio::test]
async fn nested_ignore_consistent_across_nav_tools() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".gitignore"), "unrelated_pattern_xyz\n").unwrap();
    fs::create_dir_all(dir.path().join("sub")).unwrap();
    fs::write(dir.path().join("sub/.gitignore"), "secret.txt\n").unwrap();
    fs::write(dir.path().join("sub/secret.txt"), "x").unwrap();
    fs::write(dir.path().join("sub/keep.txt"), "x").unwrap();

    let ls = LsTool::new(dir.path().to_path_buf());
    // max_depth must recurse into sub/ so the nested .gitignore is exercised
    // (ls defaults to a flat listing).
    let result = ls
        .execute(
            "ni-l1",
            json!({ "path": ".", "max_depth": 3 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    let text = tool_result_text(&result);
    assert!(!result.is_error, "{}", text);
    assert!(text.contains("keep.txt"));
    assert!(
        !text.contains("secret.txt"),
        "ls must honor nested .gitignore (was hand-rolled root-only): {text}"
    );
}

/// ls treats symlinks consistently with the other nav tools: a symlink pointing
/// outside the workspace is never traversed, so the outside file never leaks.
/// Pre-11.7 ls followed symlinks via path.is_dir() and diverged. Unix-only.
#[cfg(unix)]
#[tokio::test]
async fn nav_tools_symlink_behavior_reported() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("outside_secret.txt"), "x").unwrap();
    fs::write(dir.path().join("inside.txt"), "x").unwrap();
    let link = dir.path().join("link");
    if symlink(outside.path(), &link).is_err() {
        eprintln!("skipping ls symlink test; symlink creation failed");
        return;
    }

    let ls = LsTool::new(dir.path().to_path_buf());
    let result = ls
        .execute(
            "sl-l1",
            json!({ "path": "." }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    let text = tool_result_text(&result);
    assert!(!result.is_error, "{}", text);
    assert!(text.contains("inside.txt"));
    assert!(
        !text.contains("outside_secret"),
        "ls must not traverse the symlink (was diverging via path.is_dir): {text}"
    );
}

/// A zero-entry listing returns a clear non-empty message, never "".
#[tokio::test]
async fn ls_tool_empty_directory_returns_message() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("empty")).unwrap();

    let ls = LsTool::new(dir.path().to_path_buf());
    let result = ls
        .execute(
            "em-l1",
            json!({ "path": "empty" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    let text = tool_result_text(&result);
    assert!(!result.is_error);
    assert!(
        !text.is_empty(),
        "empty dir must return a non-empty message: {text:?}"
    );
    assert!(
        text.to_lowercase().contains("no entries"),
        "empty dir message: {text}"
    );
}

/// ls honors the CancellationToken mid-walk (pre-cancelled -> zero entries).
#[tokio::test]
async fn nav_tools_honour_cancellation_token() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..50 {
        fs::write(dir.path().join(format!("f_{i:02}.txt")), "x").unwrap();
    }
    let token = CancellationToken::new();
    token.cancel();
    let ls = LsTool::new(dir.path().to_path_buf());
    let result = ls
        .execute("cn-l1", json!({ "path": "." }), token, None)
        .await
        .unwrap();
    assert!(!result.is_error);
    let details = result.details.as_ref().expect("details");
    assert_eq!(
        details.get("cancelled").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        details.get("total_entries").and_then(|v| v.as_u64()),
        Some(0),
        "pre-cancelled walk must yield zero entries"
    );
}
