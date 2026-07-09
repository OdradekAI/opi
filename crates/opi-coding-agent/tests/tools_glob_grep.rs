//! Behavioral tests for glob and grep tools (task 1.10).
//!
//! DoD: "tests cover ignored dirs and regex errors"

mod common;

use common::{create_gitignore, tool_result_text};
use opi_agent::diagnostic::code;
use opi_agent::tool::{ExecutionMode, Tool};
use opi_coding_agent::tool::{
    GlobTool, GrepTool, MAX_GREP_TOTAL_READ_BYTES, MAX_NAV_FILE_BYTES, MAX_NAV_RESULTS,
    MAX_NAV_VISITED_ENTRIES,
};
use serde_json::json;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// GlobTool tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn glob_tool_finds_files_by_pattern() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("foo.rs"), "fn main() {}").unwrap();
    std::fs::write(dir.path().join("bar.rs"), "fn bar() {}").unwrap();
    std::fs::write(dir.path().join("baz.txt"), "hello").unwrap();

    let tool = GlobTool::new(dir.path().to_path_buf());
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
async fn glob_tool_ignores_gitignored_dirs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::create_dir_all(dir.path().join("target")).unwrap();
    std::fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(dir.path().join("target/build.rs"), "fn build() {}").unwrap();
    create_gitignore(dir.path(), "target/\n");

    let tool = GlobTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c2",
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

#[tokio::test]
async fn glob_tool_empty_pattern_matches_nothing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("file.txt"), "data").unwrap();

    let tool = GlobTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c3",
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
}

#[tokio::test]
async fn glob_tool_is_parallel() {
    let tool = GlobTool::new(std::path::PathBuf::from("."));
    assert_eq!(tool.execution_mode(), ExecutionMode::Parallel);
}

// ---------------------------------------------------------------------------
// GrepTool tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn grep_tool_finds_matching_lines() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("code.rs"),
        "fn hello() {}\nfn world() {}\nfn goodbye() {}\n",
    )
    .unwrap();

    let tool = GrepTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c4",
            json!({ "pattern": "hello" }),
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
    assert!(text.contains("hello"), "should match 'hello'");
    assert!(
        !text.contains("goodbye"),
        "should not match unrelated lines"
    );
}

#[tokio::test]
async fn grep_tool_regex_pattern() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("data.txt"), "foo123\nbar456\nbaz\n").unwrap();

    let tool = GrepTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c5",
            json!({ "pattern": "[a-z]+\\d+" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let text = tool_result_text(&result);
    assert!(text.contains("foo123"), "should match foo123");
    assert!(text.contains("bar456"), "should match bar456");
    assert!(!text.contains("baz"), "should not match baz (no digits)");
}

#[tokio::test]
async fn grep_tool_invalid_regex_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let tool = GrepTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c6",
            json!({ "pattern": "[invalid" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(
        result.is_error,
        "invalid regex should be error: {}",
        tool_result_text(&result)
    );
    let text = tool_result_text(&result);
    assert!(
        text.to_lowercase().contains("regex") || text.to_lowercase().contains("pattern"),
        "error should mention regex issue: {text}"
    );
}

#[tokio::test]
async fn grep_tool_ignores_gitignored_dirs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::create_dir_all(dir.path().join("node_modules")).unwrap();
    std::fs::write(dir.path().join("src/main.rs"), "secret_token_here\n").unwrap();
    std::fs::write(
        dir.path().join("node_modules/pkg.js"),
        "secret_token_in_dep\n",
    )
    .unwrap();
    create_gitignore(dir.path(), "node_modules/\n");

    let tool = GrepTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c7",
            json!({ "pattern": "secret_token" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let text = tool_result_text(&result);
    assert!(text.contains("main.rs"), "should find match in src/main.rs");
    assert!(
        !text.contains("node_modules"),
        "should not search in gitignored node_modules"
    );
}

#[tokio::test]
async fn grep_tool_no_matches_is_not_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("file.txt"), "hello world\n").unwrap();

    let tool = GrepTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c8",
            json!({ "pattern": "nonexistent_pattern" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    // No matches is not an error; just empty results
    assert!(!result.is_error);
}

#[tokio::test]
async fn grep_tool_is_parallel() {
    let tool = GrepTool::new(std::path::PathBuf::from("."));
    assert_eq!(tool.execution_mode(), ExecutionMode::Parallel);
}

// ---------------------------------------------------------------------------
// Uniform tool-result contract (Phase 11.1)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn uniform_tool_result_details_contract() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.rs"),
        "fn a() {}
",
    )
    .unwrap();
    std::fs::write(dir.path().join("b.txt"), "x").unwrap();

    // grep success: base contract + workspace_relation parity (walks root -> inside)
    let grep = GrepTool::new(dir.path().to_path_buf())
        .execute(
            "u1",
            json!({ "pattern": "fn" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!grep.is_error, "grep: {}", tool_result_text(&grep));
    assert!(!grep.truncated);
    assert!(grep.diagnostics.is_empty());
    let gd = grep.details.expect("grep details");
    assert_eq!(
        gd.get("workspace_relation").and_then(|v| v.as_str()),
        Some("inside")
    );

    // glob success: base contract + workspace_relation parity
    let glob = GlobTool::new(dir.path().to_path_buf())
        .execute(
            "u2",
            json!({ "pattern": "*.rs" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!glob.is_error, "glob: {}", tool_result_text(&glob));
    assert!(!glob.truncated);
    let gld = glob.details.expect("glob details");
    assert_eq!(
        gld.get("workspace_relation").and_then(|v| v.as_str()),
        Some("inside")
    );

    // representative failure: invalid regex carries base contract, no details
    let bad = GrepTool::new(dir.path().to_path_buf())
        .execute(
            "u3",
            json!({ "pattern": "[invalid" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(bad.is_error);
    assert!(!bad.truncated);
    assert!(bad.details.is_none(), "grep failure details must stay None");
}

// ---------------------------------------------------------------------------
// Tool definition tests
// ---------------------------------------------------------------------------

#[test]
fn glob_tool_has_valid_definition() {
    let tool = GlobTool::new(std::path::PathBuf::from("."));
    let def = tool.definition();
    assert_eq!(def.name, "glob");
    assert!(!def.description.is_empty());
    assert!(def.input_schema.is_object());
}

#[test]
fn grep_tool_has_valid_definition() {
    let tool = GrepTool::new(std::path::PathBuf::from("."));
    let def = tool.definition();
    assert_eq!(def.name, "grep");
    assert!(!def.description.is_empty());
    assert!(def.input_schema.is_object());
}

// ---------------------------------------------------------------------------
// Phase 11.7: read-only navigation tools consistency
// ---------------------------------------------------------------------------

/// Nested `.gitignore` (the only rule ignoring `secret.txt` lives in `sub/`)
/// must be honored by both grep and glob. Pre-11.7 ls hand-rolled a root-only
/// matcher and leaked such files; this fixture genuinely fails on that bug.
#[tokio::test]
async fn nested_ignore_consistent_across_nav_tools() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".gitignore"), "unrelated_pattern_xyz\n").unwrap();
    std::fs::create_dir_all(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("sub/.gitignore"), "secret.txt\n").unwrap();
    std::fs::write(dir.path().join("sub/secret.txt"), "match here\n").unwrap();
    std::fs::write(dir.path().join("sub/keep.txt"), "match here\n").unwrap();

    let grep = GrepTool::new(dir.path().to_path_buf())
        .execute(
            "ni-1",
            json!({ "pattern": "match" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    let grep_text = tool_result_text(&grep);
    assert!(!grep.is_error, "{}", grep_text);
    assert!(grep_text.contains("keep.txt"));
    assert!(
        !grep_text.contains("secret.txt"),
        "grep must honor nested .gitignore: {grep_text}"
    );

    let glob = GlobTool::new(dir.path().to_path_buf())
        .execute(
            "ni-2",
            json!({ "pattern": "sub/*.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    let glob_text = tool_result_text(&glob);
    assert!(!glob.is_error, "{}", glob_text);
    assert!(glob_text.contains("keep.txt"));
    assert!(
        !glob_text.contains("secret.txt"),
        "glob must honor nested .gitignore: {glob_text}"
    );
}

/// Results are emitted in strict lexicographic relative-path order, and as
/// relative (not absolute) paths. For grep, intra-file order follows line
/// number, not line text.
#[tokio::test]
async fn nav_tools_emit_sorted_results() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["c.rs", "a.rs", "b.rs"] {
        std::fs::write(dir.path().join(name), "x\n").unwrap();
    }

    let glob = GlobTool::new(dir.path().to_path_buf())
        .execute(
            "so-1",
            json!({ "pattern": "*.rs" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!glob.is_error);
    let glob_text = tool_result_text(&glob);
    let lines: Vec<&str> = glob_text.lines().collect();
    assert_eq!(
        lines,
        vec!["a.rs", "b.rs", "c.rs"],
        "glob must emit sorted relative paths"
    );
    for l in &lines {
        assert!(
            !l.starts_with('/') && !l.starts_with('\\') && l.len() >= 2 && !l.contains(':'),
            "glob must emit relative paths only: {l}"
        );
    }

    // grep: a.rs has two matching lines where line-1 text sorts AFTER line-2
    // text; line-number order must win (zebra_line before alpha_line).
    std::fs::write(dir.path().join("a.rs"), "zebra_line\nalpha_line\n").unwrap();
    std::fs::write(dir.path().join("b.rs"), "x_line\n").unwrap();
    std::fs::write(dir.path().join("c.rs"), "x_line\n").unwrap();
    let grep = GrepTool::new(dir.path().to_path_buf())
        .execute(
            "so-2",
            json!({ "pattern": "line" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    let gtext = tool_result_text(&grep);
    let pos_a = gtext.find("a.rs").expect("a.rs");
    let pos_b = gtext.find("b.rs").expect("b.rs");
    let pos_c = gtext.find("c.rs").expect("c.rs");
    assert!(pos_a < pos_b && pos_b < pos_c, "path order: {gtext}");
    let pos_zebra = gtext.find("a.rs: zebra_line").expect("zebra line");
    let pos_alpha = gtext.find("a.rs: alpha_line").expect("alpha line");
    assert!(
        pos_zebra < pos_alpha,
        "intra-file order must be line number, not text: {gtext}"
    );
}

/// grep/glob cap large result sets at MAX_NAV_RESULTS with truncation flags and
/// an exact omitted_count; exactly at the cap nothing is truncated.
#[tokio::test]
async fn nav_tools_cap_large_result_sets() {
    let cap = MAX_NAV_RESULTS;
    let dir = tempfile::tempdir().unwrap();
    for i in 0..(cap + 5) {
        std::fs::write(dir.path().join(format!("f_{i:04}.txt")), "x").unwrap();
    }
    let glob = GlobTool::new(dir.path().to_path_buf())
        .execute(
            "cap-1",
            json!({ "pattern": "f_*.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!glob.is_error);
    assert!(glob.truncated, "result.truncated must be set over the cap");
    let details = glob.details.as_ref().expect("details");
    assert_eq!(
        details.get("truncated").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        details
            .get("search_terminated_early")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert!(
        details
            .get("omitted_count")
            .and_then(|v| v.as_u64())
            .is_some_and(|count| count >= 1),
        "early stop should report an omitted lower bound: {details}"
    );
    assert!(
        details
            .get("match_count")
            .and_then(|v| v.as_u64())
            .is_some_and(|count| count <= (cap + 1) as u64),
        "match_count should stop near the cap: {details}"
    );
    assert_eq!(
        tool_result_text(&glob).lines().count(),
        cap,
        "emitted lines must equal the cap"
    );

    // Exactly at cap: not truncated.
    let dir2 = tempfile::tempdir().unwrap();
    for i in 0..cap {
        std::fs::write(dir2.path().join(format!("f_{i:04}.txt")), "x").unwrap();
    }
    let glob2 = GlobTool::new(dir2.path().to_path_buf())
        .execute(
            "cap-2",
            json!({ "pattern": "f_*.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!glob2.is_error);
    assert!(!glob2.truncated, "exactly at cap must not truncate");
    let d2 = glob2.details.as_ref().expect("details");
    assert_eq!(d2.get("truncated").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(d2.get("omitted_count").and_then(|v| v.as_u64()), Some(0));
}

/// A zero-match query returns a clear non-empty no-matches message (never "").
#[tokio::test]
async fn nav_tools_empty_result_message() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("file.txt"), "hello\n").unwrap();

    let grep = GrepTool::new(dir.path().to_path_buf())
        .execute(
            "em-1",
            json!({ "pattern": "zzz_no_match" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!grep.is_error);
    let gt = tool_result_text(&grep);
    assert!(!gt.is_empty(), "grep zero-match must be non-empty: {gt:?}");
    assert!(gt.to_lowercase().contains("no matches"), "grep: {gt}");

    let glob = GlobTool::new(dir.path().to_path_buf())
        .execute(
            "em-2",
            json!({ "pattern": "*.nomatch" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!glob.is_error);
    let glt = tool_result_text(&glob);
    assert!(
        !glt.is_empty(),
        "glob zero-match must be non-empty: {glt:?}"
    );
    assert!(glt.to_lowercase().contains("no matches"), "glob: {glt}");
}

/// regex/glob parse errors are structured: is_error, details None, message
/// names the offending pattern kind.
#[tokio::test]
async fn nav_tools_parse_errors_are_structured() {
    let dir = tempfile::tempdir().unwrap();

    let grep = GrepTool::new(dir.path().to_path_buf())
        .execute(
            "pe-1",
            json!({ "pattern": "[invalid" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(grep.is_error);
    assert!(grep.details.is_none(), "parse error must keep details None");
    let gt = tool_result_text(&grep);
    assert!(
        gt.to_lowercase().contains("regex") || gt.to_lowercase().contains("pattern"),
        "grep parse error must name the cause: {gt}"
    );

    let glob = GlobTool::new(dir.path().to_path_buf())
        .execute(
            "pe-2",
            json!({ "pattern": "[invalid" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(glob.is_error);
    assert!(glob.details.is_none());
    let glt = tool_result_text(&glob);
    assert!(
        glt.to_lowercase().contains("glob") || glt.to_lowercase().contains("pattern"),
        "glob parse error must name the cause: {glt}"
    );
}

/// grep skips files over the size guard (counted in details) without reading
/// them, while still matching smaller files.
#[tokio::test]
async fn grep_tool_skips_oversized_file() {
    let dir = tempfile::tempdir().unwrap();
    // big.txt contains the pattern but exceeds the size guard; it must be
    // skipped (stat-first), not read.
    let mut big = String::from("match\n");
    big.push_str(&"a".repeat(MAX_NAV_FILE_BYTES as usize));
    std::fs::write(dir.path().join("big.txt"), big).unwrap();
    std::fs::write(dir.path().join("small.txt"), "match\n").unwrap();

    let grep = GrepTool::new(dir.path().to_path_buf())
        .execute(
            "os-1",
            json!({ "pattern": "match" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!grep.is_error);
    let text = tool_result_text(&grep);
    assert!(
        text.contains("small.txt"),
        "small file match must be present: {text}"
    );
    assert!(
        !text.contains("big.txt"),
        "oversized file must be skipped: {text}"
    );
    let details = grep.details.as_ref().expect("details");
    assert_eq!(
        details
            .get("files_oversized_skipped")
            .and_then(|v| v.as_u64()),
        Some(1),
        "oversized skip must be counted: {details}"
    );
}

#[tokio::test]
async fn grep_reports_non_utf8_content_skips() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("bad.txt"), [0xff, 0xfe, b'a']).unwrap();
    let tool = GrepTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "grep-non-utf8",
            json!({ "pattern": "a" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    assert_eq!(
        result.details.as_ref().unwrap()["files_skipped_non_utf8"],
        json!(1)
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == code::CODE_TOOL_UNSUPPORTED_ENCODING),
        "{:?}",
        result.diagnostics
    );
}

#[tokio::test]
async fn grep_bounds_match_collection_before_reading_everything() {
    let dir = tempfile::tempdir().unwrap();
    let mut body = String::new();
    for i in 0..(MAX_NAV_RESULTS + 20) {
        body.push_str(&format!("match {i}\n"));
    }
    std::fs::write(dir.path().join("many.txt"), body).unwrap();

    let grep = GrepTool::new(dir.path().to_path_buf())
        .execute(
            "grep-bound-1",
            json!({ "pattern": "match" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!grep.is_error, "{}", tool_result_text(&grep));
    assert!(grep.truncated);
    let details = grep.details.as_ref().expect("details");
    assert_eq!(
        details
            .get("search_terminated_early")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert!(
        details
            .get("match_count")
            .and_then(|v| v.as_u64())
            .is_some_and(|count| count <= (MAX_NAV_RESULTS + 1) as u64),
        "match_count should stop near the cap: {details}"
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
async fn grep_early_termination_without_matches_is_not_reported_as_no_matches() {
    let dir = tempfile::tempdir().unwrap();
    let chunk = "x".repeat((MAX_GREP_TOTAL_READ_BYTES / 8) as usize);
    for i in 0..9 {
        std::fs::write(dir.path().join(format!("chunk_{i}.txt")), &chunk).unwrap();
    }

    let grep = GrepTool::new(dir.path().to_path_buf())
        .execute(
            "grep-early-no-match",
            json!({ "pattern": "needle_not_present" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!grep.is_error, "{}", tool_result_text(&grep));
    assert!(grep.truncated);
    let text = tool_result_text(&grep).to_lowercase();
    assert!(
        text.contains("terminated before completing"),
        "early termination must be visible in provider-facing text: {text}"
    );
    assert!(
        !text.contains("no matches"),
        "early termination is not a complete no-match result: {text}"
    );
}

#[tokio::test]
async fn glob_early_termination_without_matches_is_not_reported_as_no_matches() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..(MAX_NAV_VISITED_ENTRIES + 1) {
        std::fs::write(dir.path().join(format!("f_{i:05}.txt")), "x").unwrap();
    }

    let glob = GlobTool::new(dir.path().to_path_buf())
        .execute(
            "glob-early-no-match",
            json!({ "pattern": "*.rs" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!glob.is_error, "{}", tool_result_text(&glob));
    assert!(glob.truncated);
    let text = tool_result_text(&glob).to_lowercase();
    assert!(
        text.contains("terminated before completing"),
        "early termination must be visible in provider-facing text: {text}"
    );
    assert!(
        !text.contains("no matches"),
        "early termination is not a complete no-match result: {text}"
    );
}

#[tokio::test]
async fn glob_bounds_collection_before_walking_everything() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..(MAX_NAV_RESULTS + 20) {
        std::fs::write(dir.path().join(format!("f_{i:04}.txt")), "x").unwrap();
    }

    let glob = GlobTool::new(dir.path().to_path_buf())
        .execute(
            "glob-bound-1",
            json!({ "pattern": "f_*.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!glob.is_error, "{}", tool_result_text(&glob));
    assert!(glob.truncated);
    let details = glob.details.as_ref().expect("details");
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
/// Symlink behavior is consistent: with follow_links(false), a symlink pointing
/// outside the workspace is never traversed, so the outside file never leaks.
/// (Pre-11.7 ls followed via path.is_dir(); grep/glob already did not follow.)
/// Unix-only: symlinks are well-defined here; the cross-tool consistency is
/// shared code, so the Windows job inherits it.
#[cfg(unix)]
#[tokio::test]
async fn nav_tools_symlink_behavior_reported() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("outside_secret.txt"), "leak\n").unwrap();
    std::fs::write(dir.path().join("inside.txt"), "leak\n").unwrap();
    let link = dir.path().join("link");
    if symlink(outside.path(), &link).is_err() {
        eprintln!("skipping symlink nav test; symlink creation failed");
        return;
    }

    let glob = GlobTool::new(dir.path().to_path_buf())
        .execute(
            "sl-1",
            json!({ "pattern": "**/*.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!glob.is_error);
    let gtext = tool_result_text(&glob);
    assert!(gtext.contains("inside.txt"));
    assert!(
        !gtext.contains("outside_secret"),
        "glob must not traverse the symlink: {gtext}"
    );

    let grep = GrepTool::new(dir.path().to_path_buf())
        .execute(
            "sl-2",
            json!({ "pattern": "leak" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!grep.is_error);
    let gtext2 = tool_result_text(&grep);
    assert!(gtext2.contains("inside.txt"));
    assert!(
        !gtext2.contains("outside_secret"),
        "grep must not read through the symlink: {gtext2}"
    );
}

/// Filenames are emitted in sorted order.
#[tokio::test]
async fn nav_tools_paths_sorted() {
    let dir = tempfile::tempdir().unwrap();
    for name in [
        "c_unicode_order.rs",
        "a_unicode_order.rs",
        "b_unicode_order.rs",
    ] {
        std::fs::write(dir.path().join(name), "x\n").unwrap();
    }
    let glob = GlobTool::new(dir.path().to_path_buf())
        .execute(
            "un-1",
            json!({ "pattern": "*.rs" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!glob.is_error);
    let glob_text = tool_result_text(&glob);
    let lines: Vec<&str> = glob_text.lines().collect();
    assert_eq!(
        lines,
        vec![
            "a_unicode_order.rs",
            "b_unicode_order.rs",
            "c_unicode_order.rs",
        ],
    );
}

/// The CancellationToken is polled mid-walk: a pre-cancelled token aborts the
/// walk before any result is emitted (deterministic; no timer). If the signal
/// were ignored, glob would enumerate and cap the full tree instead.
#[tokio::test]
async fn nav_tools_honour_cancellation_token() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..(MAX_NAV_RESULTS + 200) {
        std::fs::write(dir.path().join(format!("f_{i:04}.txt")), "x").unwrap();
    }
    let token = CancellationToken::new();
    token.cancel(); // pre-cancel: deterministic
    let glob = GlobTool::new(dir.path().to_path_buf())
        .execute("cn-1", json!({ "pattern": "f_*.txt" }), token, None)
        .await
        .unwrap();
    assert!(!glob.is_error, "cancellation is cooperative, not an error");
    let details = glob.details.as_ref().expect("details");
    assert_eq!(
        details.get("cancelled").and_then(|v| v.as_bool()),
        Some(true),
        "must report cancelled"
    );
    assert_eq!(
        details.get("match_count").and_then(|v| v.as_u64()),
        Some(0),
        "pre-cancelled walk must match zero files (signal polled, not ignored)"
    );
}

/// grep honors the CancellationToken mid-walk (parity with glob/find/ls; closes
/// the grep-specific cancellation gap). If the signal were ignored, grep would
/// read every fixture file and report a large match_count.
#[tokio::test]
async fn grep_tool_honours_cancellation_token() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..(MAX_NAV_RESULTS + 200) {
        std::fs::write(dir.path().join(format!("f_{i:04}.txt")), "match\n").unwrap();
    }
    let token = CancellationToken::new();
    token.cancel();
    let grep = GrepTool::new(dir.path().to_path_buf())
        .execute("cn-g1", json!({ "pattern": "match" }), token, None)
        .await
        .unwrap();
    assert!(!grep.is_error);
    let details = grep.details.as_ref().expect("details");
    assert_eq!(
        details.get("cancelled").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        details.get("match_count").and_then(|v| v.as_u64()),
        Some(0),
        "pre-cancelled grep walk must match zero files"
    );
}
