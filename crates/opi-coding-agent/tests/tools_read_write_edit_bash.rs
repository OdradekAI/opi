//! Behavioral tests for read, write, edit, bash tools (task 1.9).
//!
//! DoD: "temp-dir tests cover success, failure, timeout/cancellation,
//!       cwd/env reporting, and minimal confirmation policy"

use std::time::Duration;

use opi_agent::diagnostic::code;
use opi_agent::tool::{ExecutionMode, Tool, ToolResult};
use opi_coding_agent::tool::{
    BashTool, EditTool, MAX_BASH_OUTPUT_BYTES, MAX_READ_OUTPUT_BYTES, PathPolicy, ReadTool,
    WriteTool,
};
use serde_json::json;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn tool_result_text(result: &ToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| match c {
            opi_ai::message::OutputContent::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn details_string<'a>(details: &'a serde_json::Value, key: &str) -> &'a str {
    details
        .get(key)
        .and_then(|value| value.as_str())
        .unwrap_or_else(|| panic!("details should include string key '{key}'"))
}

// ---------------------------------------------------------------------------
// ReadTool tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn read_tool_reads_file_content() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("hello.txt");
    std::fs::write(&file_path, "Hello, world!").unwrap();

    let tool = ReadTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c1",
            json!({ "path": "hello.txt" }),
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
    assert!(
        text.contains("Hello, world!"),
        "should contain file content"
    );
}

#[tokio::test]
async fn read_tool_reads_with_line_range() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("lines.txt");
    std::fs::write(&file_path, "line1\nline2\nline3\nline4\nline5").unwrap();

    let tool = ReadTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c2",
            json!({ "path": "lines.txt", "offset": 2, "limit": 2 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let text = tool_result_text(&result);
    assert!(text.contains("line2"), "should contain line2");
    assert!(text.contains("line3"), "should contain line3");
    assert!(!text.contains("line1"), "should not contain line1");
    assert!(!text.contains("line4"), "should not contain line4");
}

#[tokio::test]
async fn read_tool_file_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let tool = ReadTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c3",
            json!({ "path": "nonexistent.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(result.is_error, "should be error for missing file");
}

#[tokio::test]
async fn read_tool_is_parallel() {
    let tool = ReadTool::new(std::path::PathBuf::from("."));
    assert_eq!(tool.execution_mode(), ExecutionMode::Parallel);
}

#[tokio::test]
async fn read_tool_reports_workspace_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("inside.txt");
    std::fs::write(&file_path, "data").unwrap();

    let tool = ReadTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c4",
            json!({ "path": "inside.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    let text = tool_result_text(&result);
    assert!(
        text.contains("inside.txt"),
        "result should reference the file path"
    );

    let details = result.details.expect("should have details");
    assert_eq!(
        details
            .get("workspace_relation")
            .and_then(|value| value.as_str()),
        Some("inside")
    );
    assert!(
        !details
            .as_object()
            .map(|o| o.contains_key("inside_workspace"))
            .unwrap_or(false),
        "inside_workspace key must be superseded by workspace_relation"
    );
    assert!(details_string(&details, "resolved_path").ends_with("inside.txt"));
}

#[tokio::test]
async fn read_tool_allow_outside_policy_reads_absolute_outside_path() {
    let workspace = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("outside.txt");
    std::fs::write(&outside_file, "outside data").unwrap();

    let tool = ReadTool::new_with_policy(
        workspace.path().to_path_buf(),
        PathPolicy::AllowOutsideWorkspace,
    );
    let result = tool
        .execute(
            "outside-read-allow",
            json!({ "path": outside_file }),
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
    assert!(tool_result_text(&result).contains("outside data"));
    let details = result.details.expect("should have details");
    assert_eq!(
        details
            .get("workspace_relation")
            .and_then(|value| value.as_str()),
        Some("outside")
    );
    let resolved = details_string(&details, "resolved_path");
    assert!(
        !resolved.contains(r"\\?\"),
        "resolved_path must not leak the Windows verbatim prefix: {resolved}"
    );
    // The stripped display path must still resolve to the same file.
    assert_eq!(
        std::fs::canonicalize(resolved).unwrap(),
        std::fs::canonicalize(&outside_file).unwrap()
    );
}

#[tokio::test]
async fn read_tool_reports_line_ending_metadata_for_crlf() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("crlf-read.txt"),
        b"first\r\nsecond\r\nthird\r\n",
    )
    .unwrap();

    let result = ReadTool::new(dir.path().to_path_buf())
        .execute(
            "read-crlf",
            json!({ "path": "crlf-read.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error, "{}", tool_result_text(&result));
    let details = result.details.as_ref().expect("details");
    assert_eq!(
        details.get("line_ending").and_then(|v| v.as_str()),
        Some("crlf")
    );
    let text = tool_result_text(&result);
    assert!(
        text.contains("first\r\nsecond\r\nthird\r\n"),
        "read output must preserve CRLF bytes in the returned body: {text:?}"
    );
}

#[tokio::test]
async fn read_tool_caps_single_line_output_by_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let body = "A".repeat(70 * 1024);
    std::fs::write(dir.path().join("one-line.txt"), &body).unwrap();

    let result = ReadTool::new(dir.path().to_path_buf())
        .execute(
            "read-byte-cap",
            json!({ "path": "one-line.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error, "{}", tool_result_text(&result));
    assert!(
        result.truncated,
        "single oversized line must be byte-capped"
    );
    let details = result.details.as_ref().expect("details");
    assert_eq!(
        details.get("truncation_reason").and_then(|v| v.as_str()),
        Some("byte_cap")
    );
    let text = tool_result_text(&result);
    assert!(
        text.len() < body.len(),
        "returned text must be smaller than the source body"
    );
    assert!(
        text.contains("output truncated by byte cap"),
        "byte-cap marker missing: {text:?}"
    );
}

// ---------------------------------------------------------------------------
// WriteTool tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn write_tool_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    let tool = WriteTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c5",
            json!({ "path": "new.txt", "content": "Hello!" }),
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
    let written = std::fs::read_to_string(dir.path().join("new.txt")).unwrap();
    assert_eq!(written, "Hello!");
}

#[tokio::test]
async fn write_tool_replaces_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("existing.txt");
    std::fs::write(&file_path, "old content").unwrap();

    let tool = WriteTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c6",
            json!({ "path": "existing.txt", "content": "new content" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    let written = std::fs::read_to_string(file_path).unwrap();
    assert_eq!(written, "new content");
}

#[tokio::test]
async fn write_tool_is_sequential() {
    let tool = WriteTool::new(std::path::PathBuf::from("."));
    assert_eq!(tool.execution_mode(), ExecutionMode::Sequential);
}

#[tokio::test]
async fn write_tool_safety_context_in_details() {
    let dir = tempfile::tempdir().unwrap();
    let tool = WriteTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c7",
            json!({ "path": "safe.txt", "content": "data" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    // Safety boundary: details should include workspace root and path
    let details = result.details.expect("should have details");
    assert!(
        details.get("workspace_root").is_some(),
        "details should include workspace_root"
    );
    assert!(details.get("path").is_some(), "details should include path");
    assert_eq!(
        details
            .get("workspace_relation")
            .and_then(|value| value.as_str()),
        Some("inside")
    );
    assert!(
        !details
            .as_object()
            .map(|o| o.contains_key("inside_workspace"))
            .unwrap_or(false),
        "inside_workspace key must be superseded by workspace_relation"
    );
    assert!(details_string(&details, "resolved_path").ends_with("safe.txt"));
}

// ---------------------------------------------------------------------------
// WriteTool hardening (Phase 11.4): create/overwrite audit, atomic temp+rename,
// CRLF/final-newline preservation, NUL/binary rejection, parent-dir context.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn write_tool_reports_create_then_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let tool = WriteTool::new(dir.path().to_path_buf());

    // First write -> created; no before/after audit keys.
    let created = tool
        .execute(
            "w-audit-1",
            json!({ "path": "audit.txt", "content": "first" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!created.is_error, "{}", tool_result_text(&created));
    let cd = created.details.as_ref().expect("created details");
    assert_eq!(details_string(cd, "action"), "created");
    assert_eq!(cd.get("bytes_written").and_then(|v| v.as_u64()), Some(5));
    assert!(
        cd.get("bytes_before").is_none(),
        "created must not carry bytes_before"
    );
    assert!(
        cd.get("size_delta").is_none(),
        "created must not carry size_delta"
    );

    // Second write -> overwritten, with before/after audit.
    let overwritten = tool
        .execute(
            "w-audit-2",
            json!({ "path": "audit.txt", "content": "second!" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!overwritten.is_error, "{}", tool_result_text(&overwritten));
    let od = overwritten.details.as_ref().expect("overwrite details");
    assert_eq!(details_string(od, "action"), "overwritten");
    assert_eq!(od.get("bytes_written").and_then(|v| v.as_u64()), Some(7));
    assert_eq!(od.get("bytes_before").and_then(|v| v.as_u64()), Some(5));
    assert_eq!(od.get("size_delta").and_then(|v| v.as_i64()), Some(2));
}

#[tokio::test]
async fn write_tool_emits_size_or_diff_audit() {
    let dir = tempfile::tempdir().unwrap();
    let tool = WriteTool::new(dir.path().to_path_buf());
    let target = dir.path().join("delta.txt");
    std::fs::write(&target, b"0123456789").unwrap(); // 10 bytes

    // Equal-length overwrite: bytes_written > 0 but size_delta == 0 proves the
    // delta is computed from before/after sizes, not echoed from bytes_written.
    let r = tool
        .execute(
            "w-delta",
            json!({ "path": "delta.txt", "content": "abcdefghij" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!r.is_error);
    let d = r.details.as_ref().expect("details");
    assert_eq!(d.get("bytes_written").and_then(|v| v.as_u64()), Some(10));
    assert_eq!(d.get("bytes_before").and_then(|v| v.as_u64()), Some(10));
    assert_eq!(d.get("size_delta").and_then(|v| v.as_i64()), Some(0));
    // On-disk size must equal bytes_written (byte count, cross-platform).
    assert_eq!(std::fs::metadata(&target).unwrap().len(), 10);

    // Negative delta: overwrite with a smaller payload.
    let smaller = tool
        .execute(
            "w-delta-sm",
            json!({ "path": "delta.txt", "content": "xy" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    let sd = smaller.details.as_ref().expect("details");
    assert_eq!(sd.get("bytes_before").and_then(|v| v.as_u64()), Some(10));
    assert_eq!(sd.get("bytes_written").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(sd.get("size_delta").and_then(|v| v.as_i64()), Some(-8));
    assert_eq!(std::fs::metadata(&target).unwrap().len(), 2);
}

#[tokio::test]
async fn write_tool_preserves_line_endings() {
    // Raw byte comparison: no CRLF translation on Windows, no added/removed
    // trailing newline. Exercises the full JSON-string round-trip.
    let cases: &[(&str, &[u8])] = &[
        ("crlf.txt", b"line1\r\nline2\r\n"),
        ("lf.txt", b"line1\nline2\n"),
        ("no-trailing.txt", b"no trailing newline"),
        ("trailing.txt", b"trailing newline\n"),
        ("multibyte.txt", "héllo 世界\n".as_bytes()),
    ];
    let dir = tempfile::tempdir().unwrap();
    let tool = WriteTool::new(dir.path().to_path_buf());
    for (name, bytes) in cases {
        let content = String::from_utf8(bytes.to_vec()).unwrap();
        let r = tool
            .execute(
                "w-le",
                json!({ "path": name, "content": content }),
                CancellationToken::new(),
                None,
            )
            .await
            .unwrap();
        assert!(!r.is_error, "{name}: {}", tool_result_text(&r));
        // bytes_written is a BYTE count (multibyte case: 14 bytes, 9 chars).
        let d = r.details.as_ref().expect("details");
        assert_eq!(
            d.get("bytes_written").and_then(|v| v.as_u64()),
            Some(bytes.len() as u64),
            "{name}: bytes_written must be byte length"
        );
        let on_disk = std::fs::read(dir.path().join(name)).unwrap();
        assert_eq!(on_disk, *bytes, "{name}: bytes not preserved verbatim");
    }
}

#[tokio::test]
async fn write_tool_content_policy_for_nul_or_binary_like_input() {
    // "binary-like" is operationally defined as a NUL byte for a UTF-8 JSON
    // string (the only non-text signal); other control bytes are valid text.
    let dir = tempfile::tempdir().unwrap();
    let tool = WriteTool::new(dir.path().to_path_buf());

    let r = tool
        .execute(
            "w-nul",
            json!({ "path": "bin.txt", "content": "abc\0def" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(r.is_error, "NUL content must be rejected");
    assert_eq!(r.diagnostics.len(), 1, "exactly one diagnostic");
    assert_eq!(r.diagnostics[0].code, code::CODE_TOOL_UNSUPPORTED_ENCODING);
    assert!(
        !dir.path().join("bin.txt").exists(),
        "rejected write must not create a file"
    );

    // A rejected NUL write into a not-yet-existing nested path must NOT create
    // parent directories as a side effect (the check runs before any IO).
    let nested = tool
        .execute(
            "w-nul-nest",
            json!({ "path": "nested/dir/bin.txt", "content": "x\0y" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(nested.is_error);
    assert!(
        !dir.path().join("nested").exists(),
        "rejected NUL write must not create parent dirs"
    );
}

#[tokio::test]
async fn write_tool_atomic_no_partial_write() {
    let dir = tempfile::tempdir().unwrap();
    let tool = WriteTool::new(dir.path().to_path_buf());
    let target = dir.path().join("atomic.txt");

    // Success path: after a large overwrite, no temp sibling leaks and the
    // target holds the full new content (never a truncated mix).
    std::fs::write(&target, b"old").unwrap();
    let big = "X".repeat(4096);
    let r = tool
        .execute(
            "w-atomic-ok",
            json!({ "path": "atomic.txt", "content": big }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!r.is_error);
    assert_eq!(std::fs::read(&target).unwrap(), big.as_bytes());
    let leaked: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".opi-write-tmp"))
        .collect();
    assert!(
        leaked.is_empty(),
        "temp file leaked after successful write: {leaked:?}"
    );

    // Error path: an overwrite whose new content is rejected (NUL) must leave
    // the PRIOR content byte-for-byte intact (no partial/truncated mix).
    let r2 = tool
        .execute(
            "w-atomic-rej",
            json!({ "path": "atomic.txt", "content": "partial\0junk" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(r2.is_error);
    assert_eq!(
        std::fs::read(&target).unwrap(),
        big.as_bytes(),
        "prior content must be intact after rejected overwrite"
    );
}

#[tokio::test]
async fn write_tool_parent_directory_error_has_context() {
    // A parent path component that is an existing regular file -> NotADirectory
    // with directory context, classified deterministically (no ErrorKind match).
    let dir = tempfile::tempdir().unwrap();
    let file_parent = dir.path().join("regularfile");
    std::fs::write(&file_parent, b"i am a file").unwrap();

    let tool = WriteTool::new(dir.path().to_path_buf());
    let r = tool
        .execute(
            "w-parent",
            json!({ "path": "regularfile/child.txt", "content": "x" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(r.is_error, "writing under a file parent must fail");
    assert_eq!(r.diagnostics.len(), 1, "exactly one diagnostic");
    assert_eq!(r.diagnostics[0].code, code::CODE_TOOL_NOT_A_DIRECTORY);
    let ctx = &r.diagnostics[0].context;
    assert!(
        ctx.get("path").is_some(),
        "NotADirectory diagnostic must carry path context"
    );
}

#[tokio::test]
async fn write_tool_existing_directory_returns_not_a_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("already-dir")).unwrap();

    let result = WriteTool::new(dir.path().to_path_buf())
        .execute(
            "write-dir",
            json!({ "path": "already-dir", "content": "replacement" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(result.is_error, "writing to a directory must fail");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == code::CODE_TOOL_NOT_A_FILE),
        "directory target should carry tool_not_a_file: {:?}",
        result.diagnostics
    );
    assert!(
        result.details.is_none(),
        "filesystem failure details should stay in diagnostics"
    );
}

// ---------------------------------------------------------------------------
// EditTool tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn edit_tool_exact_string_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("edit.txt");
    std::fs::write(&file_path, "Hello, world!").unwrap();

    let tool = EditTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c8",
            json!({
                "path": "edit.txt",
                "old_string": "world",
                "new_string": "Rust"
            }),
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
    let content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "Hello, Rust!");
}

#[tokio::test]
async fn edit_tool_old_string_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("edit2.txt");
    std::fs::write(&file_path, "Hello").unwrap();

    let tool = EditTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c9",
            json!({
                "path": "edit2.txt",
                "old_string": "not present",
                "new_string": "replacement"
            }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(result.is_error, "should be error when old_string not found");
}

#[tokio::test]
async fn edit_tool_is_sequential() {
    let tool = EditTool::new(std::path::PathBuf::from("."));
    assert_eq!(tool.execution_mode(), ExecutionMode::Sequential);
}

#[tokio::test]
async fn edit_tool_safety_context_in_details() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("safe_edit.txt");
    std::fs::write(&file_path, "content").unwrap();

    let tool = EditTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c10",
            json!({
                "path": "safe_edit.txt",
                "old_string": "content",
                "new_string": "changed"
            }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    let details = result.details.expect("should have details");
    assert!(details.get("workspace_root").is_some());
    assert!(details.get("path").is_some());
    assert_eq!(
        details
            .get("workspace_relation")
            .and_then(|value| value.as_str()),
        Some("inside")
    );
    assert!(
        !details
            .as_object()
            .map(|o| o.contains_key("inside_workspace"))
            .unwrap_or(false),
        "inside_workspace key must be superseded by workspace_relation"
    );
    assert!(details_string(&details, "resolved_path").ends_with("safe_edit.txt"));
    assert!(details.get("before").is_some());
    assert!(details.get("after").is_some());
}

// ---------------------------------------------------------------------------
// Edit tool hardening (Phase 11.5)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn edit_crlf_preservation() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("crlf.txt");
    std::fs::write(&file_path, b"alpha\r\nbeta\r\ngamma\r\n").unwrap();

    let result = EditTool::new(dir.path().to_path_buf())
        .execute(
            "ecrlf",
            json!({ "path": "crlf.txt", "old_string": "beta", "new_string": "BETA" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!result.is_error, "edit: {}", tool_result_text(&result));

    // Raw bytes: only "beta"->"BETA" changes; every CRLF is preserved exactly.
    let after = std::fs::read(&file_path).unwrap();
    assert_eq!(after, b"alpha\r\nBETA\r\ngamma\r\n");
}

#[tokio::test]
async fn edit_final_newline_preservation() {
    let dir = tempfile::tempdir().unwrap();

    // File WITH a trailing newline keeps it.
    let with_nl = dir.path().join("with_nl.txt");
    std::fs::write(&with_nl, "foo bar\n").unwrap();
    let r = EditTool::new(dir.path().to_path_buf())
        .execute(
            "enl1",
            json!({ "path": "with_nl.txt", "old_string": "foo", "new_string": "FOO" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!r.is_error, "{}", tool_result_text(&r));
    assert_eq!(std::fs::read(&with_nl).unwrap(), b"FOO bar\n");

    // File WITHOUT a trailing newline stays without one.
    let no_nl = dir.path().join("no_nl.txt");
    std::fs::write(&no_nl, "foo bar").unwrap();
    let r = EditTool::new(dir.path().to_path_buf())
        .execute(
            "enl2",
            json!({ "path": "no_nl.txt", "old_string": "foo", "new_string": "FOO" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!r.is_error, "{}", tool_result_text(&r));
    assert_eq!(std::fs::read(&no_nl).unwrap(), b"FOO bar");
}

#[tokio::test]
async fn edit_not_found_diagnostic_richness() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("rich.txt");
    std::fs::write(&file_path, "line one\nline two\n").unwrap();

    let result = EditTool::new(dir.path().to_path_buf())
        .execute(
            "enf",
            json!({ "path": "rich.txt", "old_string": "absent token", "new_string": "x" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(result.is_error, "should be error when old_string not found");
    assert!(
        result.details.is_none(),
        "edit-semantic errors omit details (cause is in diagnostics.context)"
    );
    let text = tool_result_text(&result);
    assert!(
        text.contains("rich.txt"),
        "message must name the file: {text}"
    );
    assert!(
        text.contains("absent token"),
        "message must name the attempted old_string: {text}"
    );
    let diag = result.diagnostics.first().expect("diagnostic present");
    assert_eq!(diag.code, code::CODE_TOOL_EXECUTION_FAILED);
    let ctx = &diag.context;
    assert_eq!(ctx["occurrences"], 0);
    assert_eq!(ctx["old_string"], "absent token");
    assert_eq!(ctx["old_string_len"], 12);
    assert_eq!(ctx["file_bytes"], 18); // "line one\nline two\n"
    assert_eq!(ctx["line_count"], 2);
}

#[tokio::test]
async fn edit_multiple_match_behavior() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("multi.txt");
    std::fs::write(&file_path, "dup\ndup\ndup\n").unwrap();

    let result = EditTool::new(dir.path().to_path_buf())
        .execute(
            "emulti",
            json!({ "path": "multi.txt", "old_string": "dup", "new_string": "X" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(result.is_error, "non-unique old_string must be refused");
    assert!(
        result.details.is_none(),
        "edit-semantic errors omit details"
    );
    let diag = result.diagnostics.first().expect("diagnostic present");
    assert_eq!(diag.code, code::CODE_TOOL_EXECUTION_FAILED);
    let ctx = &diag.context;
    assert_eq!(ctx["occurrences"], 3);
    let offsets = ctx["sample_offsets"]
        .as_array()
        .expect("sample_offsets array");
    assert!(offsets.len() <= 3, "sample_offsets capped: {offsets:?}");
    let text = tool_result_text(&result);
    assert!(
        text.contains("not unique") && text.contains("3 occurrences"),
        "msg: {text}"
    );
    // File untouched on refusal.
    assert_eq!(std::fs::read(&file_path).unwrap(), b"dup\ndup\ndup\n");
}

#[tokio::test]
async fn edit_multiple_match_caps_sample_offsets() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("many.txt"), "zzzzzzzzzz").unwrap();

    let result = EditTool::new(dir.path().to_path_buf())
        .execute(
            "ecap",
            json!({ "path": "many.txt", "old_string": "z", "new_string": "q" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(result.is_error);
    let ctx = &result.diagnostics.first().expect("diagnostic").context;
    assert_eq!(ctx["occurrences"], 10);
    let offsets = ctx["sample_offsets"].as_array().expect("offsets");
    assert_eq!(offsets.len(), 3, "sample_offsets capped at 3: {offsets:?}");
}

#[tokio::test]
async fn edit_no_fuzzy_near_miss_fails() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("near.txt");
    std::fs::write(&file_path, "hello world\n").unwrap();
    let tool = EditTool::new(dir.path().to_path_buf());

    // (a) single-character substitution of a present string.
    let r = tool
        .execute(
            "e1",
            json!({ "path": "near.txt", "old_string": "hallo world", "new_string": "x" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(r.is_error, "single-char near-miss must fail (no fuzzy)");

    // (b) trailing-whitespace difference.
    let r = tool
        .execute(
            "e2",
            json!({ "path": "near.txt", "old_string": "hello world ", "new_string": "x" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(r.is_error, "whitespace near-miss must fail (no fuzzy)");

    // (c) CRLF-vs-LF difference: file uses LF after "world", old_string claims CRLF.
    let r = tool
        .execute(
            "e3",
            json!({ "path": "near.txt", "old_string": "hello world\r\n", "new_string": "x" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(r.is_error, "line-ending near-miss must fail (no fuzzy)");
}

#[tokio::test]
async fn edit_diff_preview_metadata_consistent() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("preview.txt"), "fn main() { hello }").unwrap();

    let result = EditTool::new(dir.path().to_path_buf())
        .execute(
            "eprev",
            json!({ "path": "preview.txt", "old_string": "hello", "new_string": "world" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!result.is_error, "{}", tool_result_text(&result));

    let details = result.details.as_ref().expect("edit details");
    assert_eq!(details["action"], "edited");
    assert_eq!(details["occurrences"], 1);
    // before/after must be STRING-valued: interactive.rs reads them via
    // as_str() to render the ratatui DiffView.
    assert!(details["before"].is_string(), "before must be string-typed");
    assert!(details["after"].is_string(), "after must be string-typed");
    assert_eq!(details["before"], "fn main() { hello }");
    assert_eq!(details["after"], "fn main() { world }");
    for key in [
        "workspace_root",
        "path",
        "resolved_path",
        "workspace_relation",
    ] {
        assert!(details.get(key).is_some(), "edit details missing {key}");
    }
    // Small file: the truncation flags must be ABSENT (only set when a preview
    // is capped), pinning the conditional so a small-file edit never reports
    // spurious truncation.
    assert!(
        details.get("before_truncated").is_none(),
        "before_truncated must be absent for a small file"
    );
    assert!(
        details.get("after_truncated").is_none(),
        "after_truncated must be absent for a small file"
    );
}

#[tokio::test]
async fn edit_rejects_empty_old_string() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("empty_old.txt");
    std::fs::write(&file_path, "something").unwrap();

    let result = EditTool::new(dir.path().to_path_buf())
        .execute(
            "ee",
            json!({ "path": "empty_old.txt", "old_string": "", "new_string": "x" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(result.is_error);
    let ctx = &result.diagnostics.first().expect("diagnostic").context;
    assert_eq!(ctx["old_string_len"], 0);
    let text = tool_result_text(&result);
    assert!(text.contains("must not be empty"), "msg: {text}");
    // No file side effect on a rejected edit.
    assert_eq!(std::fs::read(&file_path).unwrap(), b"something");
}

#[tokio::test]
async fn edit_rejects_noop_edit() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("noop.txt"), "abc").unwrap();

    let result = EditTool::new(dir.path().to_path_buf())
        .execute(
            "en",
            json!({ "path": "noop.txt", "old_string": "abc", "new_string": "abc" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(result.is_error);
    let text = tool_result_text(&result);
    assert!(text.contains("no-op"), "msg: {text}");
}

#[tokio::test]
async fn edit_rejects_oversized_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("huge.txt");
    let big = "a".repeat(1024 * 1024 + 1);
    std::fs::write(&file_path, &big).unwrap();

    let result = EditTool::new(dir.path().to_path_buf())
        .execute(
            "eo",
            json!({ "path": "huge.txt", "old_string": "a", "new_string": "b" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(result.is_error);
    let ctx = &result.diagnostics.first().expect("diagnostic").context;
    let file_bytes = ctx["file_bytes"].as_u64().expect("file_bytes");
    let limit_bytes = ctx["limit_bytes"].as_u64().expect("limit_bytes");
    assert_eq!(limit_bytes, 1024 * 1024);
    assert!(file_bytes > limit_bytes);
    let text = tool_result_text(&result);
    assert!(text.contains("exceeds"), "msg: {text}");
    // File untouched: the guard fires before any write.
    assert_eq!(std::fs::read(&file_path).unwrap(), big.as_bytes());
}

#[tokio::test]
async fn edit_accepts_file_at_size_limit() {
    // Boundary: a file of EXACTLY MAX_EDIT_FILE_BYTES (1 MiB) is accepted --
    // the guard is a strict `>`, so at-limit must pass. Pins `>` vs `>=`.
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("atlimit.txt");
    let body = "a".repeat(1024 * 1024 - "UNIQUE".len());
    let content = format!("{body}UNIQUE");
    assert_eq!(content.len(), 1024 * 1024, "fixture must be exactly 1 MiB");
    std::fs::write(&file_path, &content).unwrap();

    let result = EditTool::new(dir.path().to_path_buf())
        .execute(
            "elimit",
            json!({ "path": "atlimit.txt", "old_string": "UNIQUE", "new_string": "DONE" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(
        !result.is_error,
        "at-limit file must be editable: {}",
        tool_result_text(&result)
    );
    // The edit applied.
    assert!(
        std::fs::read_to_string(&file_path)
            .unwrap()
            .ends_with("DONE")
    );
}

#[tokio::test]
async fn edit_on_directory_returns_not_a_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("adir")).unwrap();

    let result = EditTool::new(dir.path().to_path_buf())
        .execute(
            "edir",
            json!({ "path": "adir", "old_string": "x", "new_string": "y" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(result.is_error);
    let diag = result.diagnostics.first().expect("diagnostic");
    assert_eq!(diag.code, code::CODE_TOOL_NOT_A_FILE);
}

#[tokio::test]
async fn edit_preserves_crlf_when_old_string_spans_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("span.txt");
    std::fs::write(&file_path, b"x\r\ny\r\n").unwrap();

    let result = EditTool::new(dir.path().to_path_buf())
        .execute(
            "espan",
            json!({ "path": "span.txt", "old_string": "x\r\n", "new_string": "Z\r\n" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!result.is_error, "{}", tool_result_text(&result));
    assert_eq!(std::fs::read(&file_path).unwrap(), b"Z\r\ny\r\n");
}

#[tokio::test]
async fn edit_preserves_leading_bom() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("bom.txt");
    let bom = b"\xEF\xBB\xBF";
    let mut content = bom.to_vec();
    content.extend_from_slice(b"hello world");
    std::fs::write(&file_path, &content).unwrap();

    let result = EditTool::new(dir.path().to_path_buf())
        .execute(
            "ebom",
            json!({ "path": "bom.txt", "old_string": "hello", "new_string": "HELLO" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!result.is_error, "{}", tool_result_text(&result));
    let after = std::fs::read(&file_path).unwrap();
    assert_eq!(&after[0..3], bom, "BOM bytes preserved at file start");
    assert_eq!(&after[3..], b"HELLO world");
}

#[tokio::test]
async fn edit_truncates_before_after_for_large_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("large_preview.txt");
    // 80 KiB filler + a unique marker: under the 1 MiB edit limit, over the
    // 64 KiB preview cap, with exactly one match so the edit succeeds.
    let filler = "x".repeat(80 * 1024);
    let content = format!("{filler}UNIQUE\n");
    std::fs::write(&file_path, &content).unwrap();

    let result = EditTool::new(dir.path().to_path_buf())
        .execute(
            "etrunc",
            json!({ "path": "large_preview.txt", "old_string": "UNIQUE", "new_string": "UNIQUE2" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!result.is_error, "{}", tool_result_text(&result));
    let details = result.details.as_ref().expect("details");
    assert_eq!(details["before_truncated"], true);
    assert_eq!(details["after_truncated"], true);
    let before = details["before"].as_str().expect("before string");
    let after = details["after"].as_str().expect("after string");
    assert!(
        before.len() <= 64 * 1024,
        "before preview bounded: {}",
        before.len()
    );
    assert!(
        after.len() <= 64 * 1024,
        "after preview bounded: {}",
        after.len()
    );
}

/// Phase 11.5 structural guard: EditTool applies edits atomically (sibling temp
/// file + rename, no partial writes), matching the 11.4 write standard. Phase
/// 15.2 moved the atomic-write recipe into the local `FileOperations` backend
/// (`operations.rs::atomic_write_bytes`); edit.rs now delegates read+write to
/// the injected backend, so the guard verifies the delegation plus the backend's
/// temp+rename mechanism.
#[test]
fn edit_uses_temp_and_rename_guard() {
    let root = phase11_workspace_root();
    let edit_src = std::fs::read_to_string(root.join("crates/opi-coding-agent/src/tool/edit.rs"))
        .expect("read edit.rs");
    let ops_src =
        std::fs::read_to_string(root.join("crates/opi-coding-agent/src/tool/operations.rs"))
            .expect("read operations.rs");
    let edit = phase11_strip_comments(&edit_src);
    let ops = phase11_strip_comments(&ops_src);
    assert!(
        edit.contains("ops.read_file") && edit.contains("ops.write_file"),
        "edit.rs must read and write through the injected FileOperations backend"
    );
    assert!(
        ops.contains("rename"),
        "operations.rs must rename a sibling temp file into place (atomic write)"
    );
    assert!(
        !edit.contains("fs::write(&file_path"),
        "edit.rs must not write the destination directly; the backend temp+renames"
    );
}

/// Phase 11.5 no-fuzzy pin: edit must use exact matching only. The behavioral
/// near-miss tests prove a near-miss fails; this guard is a regression lock
/// against future re-introduction of levenshtein/edit_distance/fuzzy logic.
#[test]
fn edit_no_fuzzy_symbols_guard() {
    let root = phase11_workspace_root();
    let src = std::fs::read_to_string(root.join("crates/opi-coding-agent/src/tool/edit.rs"))
        .expect("read edit.rs");
    let s = phase11_strip_comments(&src);
    for needle in ["levenshtein", "edit_distance", "fuzzy"] {
        assert!(
            !s.to_lowercase().contains(needle),
            "edit.rs must not use fuzzy-matching symbol '{needle}'"
        );
    }
    // Positive control: exact (non-fuzzy) match primitives are present.
    assert!(
        s.contains("matches(") || s.contains("match_indices("),
        "edit.rs uses exact match primitives"
    );
}

// ---------------------------------------------------------------------------
// Uniform tool-result contract (Phase 11.1)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn uniform_tool_result_details_contract() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.txt"), "hello world").unwrap();

    // read success: base contract + uniform path metadata
    let read = ReadTool::new(dir.path().to_path_buf())
        .execute(
            "u1",
            json!({ "path": "f.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!read.is_error, "read: {}", tool_result_text(&read));
    assert!(!read.truncated);
    assert!(read.diagnostics.is_empty());
    let rd = read.details.expect("read details");
    for key in [
        "workspace_root",
        "path",
        "resolved_path",
        "workspace_relation",
    ] {
        assert!(rd.get(key).is_some(), "read details missing {key}");
    }
    assert_eq!(
        rd.get("workspace_relation").and_then(|v| v.as_str()),
        Some("inside")
    );

    // write success: base contract + uniform path metadata
    let write = WriteTool::new(dir.path().to_path_buf())
        .execute(
            "u2",
            json!({ "path": "out.txt", "content": "x" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!write.is_error, "write: {}", tool_result_text(&write));
    assert!(!write.truncated);
    assert_eq!(
        write
            .details
            .as_ref()
            .and_then(|d| d.get("workspace_relation"))
            .and_then(|v| v.as_str()),
        Some("inside")
    );

    // edit success: base contract + uniform path metadata + before/after
    let edit = EditTool::new(dir.path().to_path_buf())
        .execute(
            "u3",
            json!({ "path": "f.txt", "old_string": "hello", "new_string": "HI" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!edit.is_error, "edit: {}", tool_result_text(&edit));
    let ed = edit.details.expect("edit details");
    assert_eq!(
        ed.get("workspace_relation").and_then(|v| v.as_str()),
        Some("inside")
    );
    assert!(ed.get("before").is_some());
    assert!(ed.get("after").is_some());

    // representative failure: base contract, no path metadata, not truncated
    let missing = ReadTool::new(dir.path().to_path_buf())
        .execute(
            "u4",
            json!({ "path": "nope.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(missing.is_error);
    assert!(!missing.truncated);
    assert!(
        missing
            .diagnostics
            .iter()
            .any(|d| d.code == code::CODE_TOOL_PATH_NOT_FOUND),
        "missing-file failure should carry tool_path_not_found: {:?}",
        missing.diagnostics
    );
    assert!(missing.details.is_none(), "failure details must stay None");
}

// ---------------------------------------------------------------------------
// Filesystem error taxonomy (Phase 11.2)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn filesystem_error_taxonomy_path_failures() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("file.txt"), "x").unwrap();
    std::fs::create_dir(dir.path().join("subdir")).unwrap();
    let read = ReadTool::new(dir.path().to_path_buf());

    // NotFound: missing file carries tool_path_not_found.
    let missing = read
        .execute(
            "tax-1",
            json!({ "path": "nope.txt" }),
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
        "missing file should carry tool_path_not_found: {:?}",
        missing.diagnostics
    );
    assert!(tool_result_text(&missing).contains("does not exist"));

    // NotAFile: reading a directory carries tool_not_a_file.
    let dir_read = read
        .execute(
            "tax-2",
            json!({ "path": "subdir" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(dir_read.is_error);
    assert!(
        dir_read
            .diagnostics
            .iter()
            .any(|d| d.code == code::CODE_TOOL_NOT_A_FILE),
        "reading a directory should carry tool_not_a_file: {:?}",
        dir_read.diagnostics
    );
    assert!(tool_result_text(&dir_read).contains("not a file"));

    // OutsideWorkspace: workspace-only read of an outside absolute path.
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("o.txt");
    std::fs::write(&outside_file, "x").unwrap();
    let abs = outside_file.to_string_lossy().to_string();
    let outside_read = read
        .execute(
            "tax-3",
            json!({ "path": abs }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(outside_read.is_error);
    assert!(
        outside_read
            .diagnostics
            .iter()
            .any(|d| d.code == code::CODE_TOOL_OUTSIDE_WORKSPACE),
        "outside-workspace read should carry tool_outside_workspace: {:?}",
        outside_read.diagnostics
    );
    assert!(tool_result_text(&outside_read).contains("outside the workspace"));
}

#[tokio::test]
async fn unicode_path_metadata_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let name = "café-日本語.txt";
    std::fs::write(dir.path().join(name), "x").unwrap();
    let read = ReadTool::new(dir.path().to_path_buf());
    let result = read
        .execute(
            "uni-1",
            json!({ "path": name }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!result.is_error, "{}", tool_result_text(&result));
    let details = result.details.expect("read details");
    let resolved = details_string(&details, "resolved_path");
    assert!(
        resolved.contains("café") && resolved.contains("日本語"),
        "unicode filename must round-trip in resolved_path: {resolved}"
    );
    assert!(
        !resolved.contains('\u{FFFD}'),
        "no lossy U+FFFD replacement: {resolved}"
    );
}

#[cfg(windows)]
#[tokio::test]
async fn windows_drive_prefix() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("f.txt"), "x").unwrap();
    // Feed the verbatim canonical form back to the tool and require the
    // user-facing resolved path to be clean (no \\?\ leak).
    let verbatim = std::fs::canonicalize(workspace.path().join("f.txt")).unwrap();
    let read = ReadTool::new_with_policy(
        workspace.path().to_path_buf(),
        PathPolicy::AllowOutsideWorkspace,
    );
    let result = read
        .execute(
            "win-1",
            json!({ "path": verbatim.to_string_lossy() }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!result.is_error, "{}", tool_result_text(&result));
    let details = result.details.expect("details");
    let resolved = details_string(&details, "resolved_path");
    assert!(
        !resolved.contains(r"\\?\"),
        "verbatim prefix must not leak into resolved_path: {resolved}"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn read_tool_permission_denied_is_classified() {
    unsafe extern "C" {
        fn getuid() -> u32;
    }
    if unsafe { getuid() } == 0 {
        eprintln!("skipping permission test under root");
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("locked.txt");
    std::fs::write(&file, "x").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();
    let read = ReadTool::new(dir.path().to_path_buf());
    let result = read
        .execute(
            "pd-1",
            json!({ "path": "locked.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    let _ = std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644));
    assert!(result.is_error);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == code::CODE_TOOL_PERMISSION_DENIED),
        "unreadable file should carry tool_permission_denied: {:?}",
        result.diagnostics
    );
    assert!(tool_result_text(&result).contains("permission denied"));
}

#[cfg(windows)]
#[tokio::test]
async fn windows_drive_prefix_forms() {
    let workspace = tempfile::tempdir().unwrap();
    let read = ReadTool::new(workspace.path().to_path_buf());
    // C:\foo (drive-absolute) is outside the workspace -> OutsideWorkspace, and
    // the message must not leak the verbatim prefix.
    let result = read
        .execute(
            "win-form-1",
            json!({ "path": "C:\\nonexistent-opi-11-2-probe.txt" }),
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
            .any(|d| d.code == code::CODE_TOOL_OUTSIDE_WORKSPACE),
        "drive-absolute outside path should be denied: {:?}",
        result.diagnostics
    );
    assert!(!tool_result_text(&result).contains(r"\\?\"));
    // C:foo (drive-relative) is handled deterministically without leaking the
    // verbatim prefix (treated as relative under the workspace root).
    let rel = read
        .execute(
            "win-form-2",
            json!({ "path": "C:nonexistent.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    if let Some(p) = rel
        .details
        .as_ref()
        .and_then(|d| d.get("resolved_path"))
        .and_then(|v| v.as_str())
    {
        assert!(!p.contains(r"\\?\"));
    }
}

// ---------------------------------------------------------------------------
// BashTool tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn bash_tool_runs_command() {
    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());
    let cmd = "echo hello";
    let result = tool
        .execute(
            "c11",
            json!({ "command": cmd }),
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
    assert!(text.contains("hello"), "output should contain 'hello'");
}

#[tokio::test]
async fn bash_tool_nonzero_exit_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());
    let cmd = if cfg!(windows) {
        "cmd /C exit 1"
    } else {
        "exit 1"
    };
    let result = tool
        .execute(
            "c12",
            json!({ "command": cmd }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(result.is_error, "non-zero exit should be error");
}

// ---------------------------------------------------------------------------
// Path boundary enforcement
// ---------------------------------------------------------------------------

#[tokio::test]
async fn write_tool_rejects_path_outside_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let tool = WriteTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "escape-1",
            json!({ "path": "../outside.txt", "content": "escaped" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(result.is_error, "should reject path outside workspace");
    let text = tool_result_text(&result);
    assert!(
        text.contains("outside the workspace"),
        "error should mention workspace boundary, got: {text}"
    );
}

#[tokio::test]
async fn write_tool_rejects_absolute_path_outside_workspace() {
    let workspace = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("outside-write.txt");

    let tool = WriteTool::new(workspace.path().to_path_buf());
    let result = tool
        .execute(
            "escape-absolute-write",
            json!({ "path": outside_file, "content": "escaped" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(result.is_error, "should reject path outside workspace");
    assert!(
        tool_result_text(&result).contains("outside the workspace"),
        "error should mention workspace boundary"
    );
    assert!(
        !outside_file.exists(),
        "write must not create files outside workspace"
    );
}

#[tokio::test]
async fn write_tool_normalizes_parent_segment_after_missing_component() {
    let workspace = tempfile::tempdir().unwrap();
    let tool = WriteTool::new(workspace.path().to_path_buf());
    let result = tool
        .execute(
            "normalize-missing-parent",
            json!({ "path": "missing/../target.txt", "content": "normalized" }),
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
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("target.txt")).unwrap(),
        "normalized"
    );
    assert!(
        !workspace.path().join("missing/target.txt").exists(),
        "path should normalize to workspace target, not missing/target"
    );
}

#[tokio::test]
async fn write_tool_rejects_parent_escape_after_missing_component() {
    let parent = tempfile::tempdir().unwrap();
    let workspace = parent.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let outside_file = parent.path().join("outside.txt");

    let tool = WriteTool::new(workspace);
    let result = tool
        .execute(
            "normalize-missing-parent-escape",
            json!({ "path": "missing/../../outside.txt", "content": "escaped" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(result.is_error, "should reject path outside workspace");
    assert!(
        tool_result_text(&result).contains("outside the workspace"),
        "error should mention workspace boundary"
    );
    assert!(
        !outside_file.exists(),
        "write must not create files outside workspace"
    );
}

#[tokio::test]
async fn edit_tool_rejects_path_outside_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let tool = EditTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "escape-2",
            json!({
                "path": "../escape.txt",
                "old_string": "x",
                "new_string": "y"
            }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(result.is_error, "should reject path outside workspace");
    let text = tool_result_text(&result);
    assert!(
        text.contains("outside the workspace"),
        "error should mention workspace boundary, got: {text}"
    );
}

#[tokio::test]
async fn edit_tool_rejects_absolute_path_outside_workspace() {
    let workspace = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("outside-edit.txt");
    std::fs::write(&outside_file, "before").unwrap();

    let tool = EditTool::new(workspace.path().to_path_buf());
    let result = tool
        .execute(
            "escape-absolute-edit",
            json!({
                "path": outside_file,
                "old_string": "before",
                "new_string": "after"
            }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(result.is_error, "should reject path outside workspace");
    assert!(
        tool_result_text(&result).contains("outside the workspace"),
        "error should mention workspace boundary"
    );
    assert_eq!(std::fs::read_to_string(&outside_file).unwrap(), "before");
}

#[tokio::test]
async fn read_tool_rejects_path_outside_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let tool = ReadTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "escape-3",
            json!({ "path": "../etc/passwd" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(result.is_error, "should reject path outside workspace");
    let text = tool_result_text(&result);
    assert!(
        text.contains("outside the workspace"),
        "error should mention workspace boundary, got: {text}"
    );
}

#[tokio::test]
async fn read_tool_workspace_policy_rejects_absolute_outside_path() {
    let workspace = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("outside-read.txt");
    std::fs::write(&outside_file, "outside").unwrap();

    let tool = ReadTool::new(workspace.path().to_path_buf());
    let result = tool
        .execute(
            "escape-absolute-read",
            json!({ "path": outside_file }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(result.is_error, "should reject path outside workspace");
    assert!(
        tool_result_text(&result).contains("outside the workspace"),
        "error should mention workspace boundary"
    );
}

#[tokio::test]
async fn bash_tool_timeout() {
    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());
    let cmd = if cfg!(windows) {
        "ping -n 30 127.0.0.1 >nul"
    } else {
        "sleep 30"
    };
    let result = tool
        .execute(
            "c13",
            json!({ "command": cmd, "timeout_secs": 1 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(result.is_error, "timeout should produce error");
    let text = tool_result_text(&result);
    assert!(
        text.to_lowercase().contains("timeout") || text.to_lowercase().contains("timed out"),
        "error should mention timeout: {text}"
    );
}

#[tokio::test]
async fn bash_tool_cancellation() {
    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());
    let token = CancellationToken::new();
    let cmd = if cfg!(windows) {
        "ping -n 30 127.0.0.1 >nul"
    } else {
        "sleep 30"
    };

    let handle = {
        let token = token.clone();
        tokio::spawn(async move {
            tool.execute(
                "c14",
                json!({ "command": cmd, "timeout_secs": 60 }),
                token,
                None,
            )
            .await
        })
    };

    // Give the process time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    token.cancel();

    let result = handle.await.unwrap().unwrap();
    assert!(result.is_error, "cancelled process should be error");
}

#[tokio::test]
async fn bash_cancel_timeout_details_contract() {
    let dir = tempfile::tempdir().unwrap();
    let long_cmd = if cfg!(windows) {
        "ping -n 30 127.0.0.1 >nul"
    } else {
        "sleep 30"
    };

    // Timeout branch: timed_out=true, cancelled=false, exit_code=null, shell present.
    let timed = BashTool::new(dir.path().to_path_buf())
        .execute(
            "b-timeout",
            json!({ "command": long_cmd, "timeout_secs": 1 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(timed.is_error);
    let td = timed.details.expect("timeout details");
    assert_eq!(
        td.get("timed_out").and_then(|v| v.as_bool()),
        Some(true),
        "timeout: timed_out must be true"
    );
    assert_eq!(
        td.get("cancelled").and_then(|v| v.as_bool()),
        Some(false),
        "timeout: cancelled must be false"
    );
    assert_eq!(
        td.get("exit_code"),
        Some(&serde_json::Value::Null),
        "timeout: exit_code must be null"
    );
    assert!(td.get("shell").is_some(), "timeout: shell must be present");
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
        assert!(
            td.get(key).is_some(),
            "timeout details missing stable key: {key}"
        );
    }

    // Cancellation branch: cancelled=true, timed_out=false, exit_code=null.
    let cancel_tool = BashTool::new(dir.path().to_path_buf());
    let token = CancellationToken::new();
    let handle = {
        let token = token.clone();
        tokio::spawn(async move {
            cancel_tool
                .execute(
                    "b-cancel",
                    json!({ "command": long_cmd, "timeout_secs": 60 }),
                    token,
                    None,
                )
                .await
        })
    };
    tokio::time::sleep(Duration::from_millis(150)).await;
    token.cancel();
    let cancelled = handle.await.unwrap().unwrap();
    assert!(cancelled.is_error);
    let cd = cancelled.details.expect("cancel details");
    assert_eq!(
        cd.get("cancelled").and_then(|v| v.as_bool()),
        Some(true),
        "cancel: cancelled must be true"
    );
    assert_eq!(
        cd.get("timed_out").and_then(|v| v.as_bool()),
        Some(false),
        "cancel: timed_out must be false"
    );
    assert_eq!(cd.get("exit_code"), Some(&serde_json::Value::Null));
    assert!(cd.get("shell").is_some(), "cancel: shell must be present");
}

#[tokio::test]
async fn bash_tool_cwd_reporting() {
    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());
    let cmd = if cfg!(windows) { "cd" } else { "pwd" };
    let result = tool
        .execute(
            "c15",
            json!({ "command": cmd }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error);
    // Check that details reports the correct cwd
    let details = result.details.expect("should have details");
    let cwd = details.get("cwd").unwrap().as_str().unwrap();
    assert!(
        cwd.contains(dir.path().file_name().unwrap().to_str().unwrap()),
        "details.cwd should contain temp dir name, got: {cwd}"
    );
}

#[tokio::test]
async fn bash_tool_is_sequential() {
    let tool = BashTool::new(std::path::PathBuf::from("."));
    assert_eq!(tool.execution_mode(), ExecutionMode::Sequential);
}

#[tokio::test]
async fn bash_tool_safety_context_in_details() {
    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "c16",
            json!({ "command": "echo test" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    let details = result.details.expect("should have details");
    assert!(
        details.get("command").is_some(),
        "details should include command"
    );
    assert!(details.get("cwd").is_some(), "details should include cwd");
}

#[tokio::test]
async fn bash_tool_env_inheritance_reporting() {
    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());
    let cmd = if cfg!(windows) { "set" } else { "env" };
    let result = tool
        .execute(
            "c17",
            json!({ "command": cmd }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    // Should be able to see inherited environment
    assert!(!result.is_error);
    let text = tool_result_text(&result);
    assert!(!text.is_empty(), "should have env output");
}

// ---------------------------------------------------------------------------
// Bash hardening (Phase 11.6)
// ---------------------------------------------------------------------------

/// Cross-platform generator for a stdout stream that safely exceeds the cap.
/// `extra` is added on top of the cap-derived line count so the test tracks the
/// `MAX_BASH_OUTPUT_BYTES` constant rather than a hard-coded byte total.
fn bash_oversized_stdout_command(extra_lines: usize) -> String {
    let n = (MAX_BASH_OUTPUT_BYTES / 13) + extra_lines;
    if cfg!(windows) {
        format!("FOR /L %i IN (1,1,{n}) DO @echo AAAAAAAAAAAA")
    } else {
        format!("yes AAAAAAAAAAAA | head -n {n}")
    }
}

/// Flatten a result's diagnostics into one searchable string (message + context
/// JSON). `ToolDiagnostic` is not `Serialize`, so this reads its pub fields;
/// the secret-leak test asserts the canary is absent from this text.
fn diagnostics_text(result: &ToolResult) -> String {
    result
        .diagnostics
        .iter()
        .flat_map(|d| {
            [
                d.message.clone(),
                serde_json::to_string(&d.context).unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[tokio::test]
async fn bash_failure_diagnostics_redact_command_canary() {
    const CANARY: &str = "OPI_BASH_COMMAND_SECRET_CANARY";

    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());
    let command = if cfg!(windows) {
        format!("echo {CANARY} && exit /B 1")
    } else {
        format!("echo {CANARY}; exit 1")
    };
    let result = tool
        .execute(
            "command-canary",
            json!({ "command": command, "timeout_secs": 30 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(result.is_error, "command should fail");
    assert!(
        tool_result_text(&result).contains(CANARY),
        "canary should remain in command output"
    );
    let diagnostics = diagnostics_text(&result);
    assert!(
        !diagnostics.contains(CANARY),
        "command canary must not leak into bash-owned diagnostics: {diagnostics}"
    );
}

#[tokio::test]
async fn bash_tool_output_truncation() {
    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "trunc-1",
            json!({ "command": bash_oversized_stdout_command(2000), "timeout_secs": 30 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!result.is_error, "{}", tool_result_text(&result));
    assert!(
        result.truncated,
        "ToolResult.truncated must be set when output exceeds the cap"
    );
    let details = result.details.as_ref().expect("details");
    assert_eq!(
        details.get("truncated").and_then(|v| v.as_bool()),
        Some(true),
        "details.truncated must mirror the top-level flag"
    );
    let full = details
        .get("full_output")
        .and_then(|v| v.as_str())
        .expect("details.full_output reference must be present when truncated");
    assert!(!full.is_empty(), "full_output path must be non-empty");
    assert!(full.ends_with(".log"), "full_output path: {full}");
    let file_size = std::fs::metadata(full)
        .map(|m| m.len())
        .unwrap_or_else(|e| panic!("full_output file must exist: {e}"));
    assert!(
        file_size as usize > MAX_BASH_OUTPUT_BYTES,
        "full_output file must hold the complete (>cap) output, got {file_size}"
    );
    let content = tool_result_text(&result);
    assert!(
        content.len() <= MAX_BASH_OUTPUT_BYTES + 4,
        "inline content must be capped near the limit, got {} bytes",
        content.len()
    );
    assert!(
        (content.len() as u64) < file_size,
        "inline content must be smaller than the full output"
    );
    // Stable operation metadata survives the truncation wiring.
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
        assert!(
            details.get(key).is_some(),
            "truncation details missing stable key: {key}"
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn bash_full_output_file_is_private_on_unix() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "private-full-output",
            json!({ "command": bash_oversized_stdout_command(2000), "timeout_secs": 30 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error, "{}", tool_result_text(&result));
    let full = result
        .details
        .as_ref()
        .and_then(|details| details.get("full_output"))
        .and_then(|value| value.as_str())
        .expect("details.full_output reference must be present when truncated");
    let mode = std::fs::metadata(full).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "full_output must not be group/world-readable");
}

#[tokio::test]
async fn bash_tool_timeout_reports_timed_out_not_cancelled() {
    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());
    let cmd = if cfg!(windows) {
        "ping -n 30 127.0.0.1 >nul"
    } else {
        "sleep 30"
    };
    let result = tool
        .execute(
            "to-2",
            json!({ "command": cmd, "timeout_secs": 1 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(result.is_error, "timeout must be an error");
    let details = result.details.as_ref().expect("details");
    assert_eq!(
        details.get("timed_out").and_then(|v| v.as_bool()),
        Some(true),
        "timeout: timed_out must be true"
    );
    assert_eq!(
        details.get("cancelled").and_then(|v| v.as_bool()),
        Some(false),
        "timeout: cancelled must be false"
    );
    assert!(
        details.get("full_output").is_none(),
        "timeout: must not expose a full_output reference"
    );
    // Phase 11.8 S1: the operation context is also carried as a tool-owned
    // ToolDiagnostic (timed_out=true) that the agent loop lifts into a Phase 7
    // Diagnostic + DiagnosticLinked trace.
    assert_eq!(
        result.diagnostics.len(),
        1,
        "timeout pushes exactly one operation diagnostic"
    );
    let tdiag = &result.diagnostics[0];
    assert_eq!(tdiag.code, code::CODE_TOOL_EXECUTION_FAILED);
    assert_eq!(
        tdiag.context.get("timed_out").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        tdiag.context.get("cancelled").and_then(|v| v.as_bool()),
        Some(false)
    );
}

#[tokio::test]
async fn bash_tool_cancellation_reports_cancelled_not_timed_out() {
    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());
    let token = CancellationToken::new();
    let cmd = if cfg!(windows) {
        "ping -n 30 127.0.0.1 >nul"
    } else {
        "sleep 30"
    };
    let handle = {
        let token = token.clone();
        tokio::spawn(async move {
            tool.execute(
                "can-2",
                json!({ "command": cmd, "timeout_secs": 60 }),
                token,
                None,
            )
            .await
        })
    };
    tokio::time::sleep(Duration::from_millis(150)).await;
    token.cancel();
    let result = handle.await.unwrap().unwrap();
    assert!(result.is_error, "cancellation must be an error");
    let details = result.details.as_ref().expect("details");
    assert_eq!(
        details.get("cancelled").and_then(|v| v.as_bool()),
        Some(true),
        "cancel: cancelled must be true"
    );
    assert_eq!(
        details.get("timed_out").and_then(|v| v.as_bool()),
        Some(false),
        "cancel: timed_out must be false"
    );
    assert!(
        details.get("full_output").is_none(),
        "cancel: must not expose a full_output reference"
    );
    // Phase 11.8 S1: the operation context is also carried as a tool-owned
    // ToolDiagnostic (cancelled=true) that the agent loop lifts into a Phase 7
    // Diagnostic + DiagnosticLinked trace.
    assert_eq!(
        result.diagnostics.len(),
        1,
        "cancellation pushes exactly one operation diagnostic"
    );
    let cdiag = &result.diagnostics[0];
    assert_eq!(cdiag.code, code::CODE_TOOL_EXECUTION_FAILED);
    assert_eq!(
        cdiag.context.get("cancelled").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        cdiag.context.get("timed_out").and_then(|v| v.as_bool()),
        Some(false)
    );
}

#[tokio::test]
async fn bash_tool_merged_stream_truncation() {
    // Neither stream alone exceeds the cap, but the MERGED stdout+stderr byte
    // count does, so truncated must be true. Proves merged (sum) accounting.
    // Do NOT assert preview content order (cross-platform line-ending variance).
    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());
    // Precondition: each stream individually stays under the cap (3000 lines *
    // 14 bytes worst-case CRLF = 42000 < cap), so truncation can ONLY come from
    // the merged sum. Enforced at compile time so a future fixture-count or cap
    // edit that breaks the SUM-accounting premise fails the build.
    const _: () = assert!(3000 * 14 < MAX_BASH_OUTPUT_BYTES);
    let cmd = if cfg!(windows) {
        // Parenthesize each FOR so cmd does NOT absorb the `&`-joined second
        // command into the first FOR's body (which would re-run it per iteration).
        "(FOR /L %i IN (1,1,3000) DO @echo AAAAAAAAAAAA) & (FOR /L %i IN (1,1,3000) DO @echo AAAAAAAAAAAA 1>&2)"
            .to_string()
    } else {
        "yes AAAAAAAAAAAA | head -n 3000; yes AAAAAAAAAAAA | head -n 3000 >&2".to_string()
    };
    let result = tool
        .execute(
            "merged-1",
            json!({ "command": cmd, "timeout_secs": 30 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(
        result.truncated,
        "merged stdout+stderr over the cap must set truncated; output: {}",
        tool_result_text(&result)
    );
    let details = result.details.as_ref().expect("details");
    assert_eq!(
        details.get("truncated").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert!(
        details
            .get("full_output")
            .and_then(|v| v.as_str())
            .is_some(),
        "merged truncation must expose a full_output reference"
    );
}

#[tokio::test]
async fn bash_tool_nonzero_exit_with_truncated_output() {
    // Load-bearing intersection: a command that writes >cap AND exits nonzero
    // must carry BOTH is_error=true (nonzero) AND truncated=true + full_output,
    // without the truncation wiring clobbering the success-shape details.
    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());
    let n = (MAX_BASH_OUTPUT_BYTES / 13) + 2000;
    let cmd = if cfg!(windows) {
        // Parenthesize the FOR so cmd runs it fully before `& exit /b 3`
        // (otherwise `& exit` is absorbed into the FOR body and aborts on iter 1).
        format!("(FOR /L %i IN (1,1,{n}) DO @echo AAAAAAAAAAAA) & exit /b 3")
    } else {
        format!("yes AAAAAAAAAAAA | head -n {n}; exit 3")
    };
    let result = tool
        .execute(
            "nz-trunc",
            json!({ "command": cmd, "timeout_secs": 30 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(result.is_error, "nonzero exit must be an error");
    assert!(
        result.truncated,
        "truncated flag must hold even on nonzero exit"
    );
    let details = result.details.as_ref().expect("details");
    assert_eq!(
        details.get("exit_code").and_then(|v| v.as_i64()),
        Some(3),
        "exit_code must be 3"
    );
    assert_eq!(
        details.get("timed_out").and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        details.get("cancelled").and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        details.get("truncated").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert!(
        details
            .get("full_output")
            .and_then(|v| v.as_str())
            .is_some(),
        "full_output must be present on nonzero+truncated"
    );
}

#[tokio::test]
async fn bash_tool_exit_code_and_cwd_capture() {
    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "exit-cwd",
            json!({ "command": "echo done" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!result.is_error, "{}", tool_result_text(&result));
    let details = result.details.as_ref().expect("details");
    assert_eq!(
        details.get("exit_code").and_then(|v| v.as_i64()),
        Some(0),
        "success exit_code must be 0"
    );
    let cwd = details
        .get("cwd")
        .and_then(|v| v.as_str())
        .expect("cwd present");
    assert!(
        cwd.contains(dir.path().file_name().unwrap().to_str().unwrap()),
        "cwd should be the workspace root, got: {cwd}"
    );
    assert!(
        details
            .get("workspace_root")
            .and_then(|v| v.as_str())
            .is_some(),
        "workspace_root present"
    );
}

#[tokio::test]
async fn bash_tool_env_policy_in_details() {
    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "env-pol",
            json!({ "command": "echo x" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    let env = result
        .details
        .as_ref()
        .expect("details")
        .get("env")
        .expect("env policy token present");
    assert_eq!(
        env.get("inheritance").and_then(|v| v.as_str()),
        Some("inherited")
    );
    assert_eq!(
        env.get("values_included").and_then(|v| v.as_bool()),
        Some(false),
        "env policy must declare values_included=false"
    );
    assert!(
        env.get("values").is_none(),
        "env policy token must not carry a values array"
    );
    let obj = env.as_object().expect("env is an object");
    assert_eq!(
        obj.len(),
        2,
        "env token must be exactly {{inheritance, values_included}}"
    );
}

/// Process-wide lock so the sentinel-env secret-no-leak test cannot race with
/// any other env-mutating test. CLAUDE.md requires env-mutating tests serialize.
static BASH_ENV_CANARY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Phase 11.6: the bash tool inherits the parent environment but must NEVER
/// dump inherited env VALUES into details/diagnostics. A sentinel secret set in
/// the parent env must be absent from details and diagnostics in both cases
/// (benign command, and a command that deliberately prints the env -- where the
/// canary appears in the command's own OUTPUT but still not in details/diagnostics).
#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn bash_tool_no_secret_leakage_in_diagnostics_and_env_reporting() {
    // Holds BASH_ENV_CANARY_LOCK (std::sync::Mutex) across `.await`. Safe because
    // #[tokio::test] runs on a current-thread runtime, so the guard never crosses
    // a thread boundary and cannot deadlock the runtime. The lock serializes env
    // mutation per CLAUDE.md; remove_var runs in the Drop guard on test exit.
    let _guard = BASH_ENV_CANARY_LOCK.lock().unwrap();
    const CANARY_NAME: &str = "OPI_BASH_SECRET_LEAK_CANARY";
    const CANARY_VALUE: &str = "opi-bash-ZQ9j2x-leak-sentinel-7c3f";
    // SAFETY: edition 2024 marks env mutation unsafe. We hold BASH_ENV_CANARY_LOCK
    // for the whole test (no other env-mutating test races), remove the var in the
    // Drop guard, and the canary is a synthetic value that never appears elsewhere.
    unsafe {
        std::env::set_var(CANARY_NAME, CANARY_VALUE);
    }
    struct EnvClean;
    impl Drop for EnvClean {
        fn drop(&mut self) {
            // SAFETY: same single-test lock; restores the env on test exit.
            unsafe {
                std::env::remove_var("OPI_BASH_SECRET_LEAK_CANARY");
            }
        }
    }
    let _clean = EnvClean;

    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());

    // Case 1: benign command that does NOT print env.
    let benign = tool
        .execute(
            "leak-1",
            json!({ "command": "echo hi" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    let benign_details = serde_json::to_string(benign.details.as_ref().unwrap()).unwrap();
    let benign_diags = diagnostics_text(&benign);
    assert!(
        !tool_result_text(&benign).contains(CANARY_VALUE),
        "benign: canary must not appear in command output"
    );
    assert!(
        !benign_details.contains(CANARY_VALUE),
        "benign: canary must not leak into details"
    );
    assert!(
        !benign_diags.contains(CANARY_VALUE),
        "benign: canary must not leak into diagnostics"
    );
    let env = benign.details.as_ref().unwrap().get("env").unwrap();
    assert_eq!(
        env.get("values_included").and_then(|v| v.as_bool()),
        Some(false)
    );

    // Case 2: command that deliberately prints the sentinel env var.
    let printcmd = if cfg!(windows) {
        format!("set {CANARY_NAME}")
    } else {
        format!("printenv {CANARY_NAME}")
    };
    let printed = tool
        .execute(
            "leak-2",
            json!({ "command": printcmd }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(
        tool_result_text(&printed).contains(CANARY_VALUE),
        "printenv: canary should appear in command output (command printed it)"
    );
    let printed_details = serde_json::to_string(printed.details.as_ref().unwrap()).unwrap();
    let printed_diags = diagnostics_text(&printed);
    assert!(
        !printed_details.contains(CANARY_VALUE),
        "printenv: canary must still not leak into details"
    );
    assert!(
        !printed_diags.contains(CANARY_VALUE),
        "printenv: canary must still not leak into diagnostics"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn bash_tool_unix_shell_selection() {
    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "shell-u",
            json!({ "command": "echo ok" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!result.is_error);
    let details = result.details.as_ref().expect("details");
    assert_eq!(
        details.get("shell").and_then(|v| v.as_str()),
        Some("sh"),
        "unix must select sh and report it in metadata"
    );
    assert!(tool_result_text(&result).contains("ok"));
}

#[cfg(windows)]
#[tokio::test]
async fn bash_tool_windows_shell_selection() {
    let dir = tempfile::tempdir().unwrap();
    let tool = BashTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "shell-w",
            json!({ "command": "echo ok" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!result.is_error, "{}", tool_result_text(&result));
    let details = result.details.as_ref().expect("details");
    assert_eq!(
        details.get("shell").and_then(|v| v.as_str()),
        Some("cmd"),
        "windows must select cmd and report it in metadata"
    );
    assert!(
        tool_result_text(&result).contains("ok"),
        "output: {}",
        tool_result_text(&result)
    );
}

#[test]
fn bash_tool_is_mutating_and_sequential() {
    use opi_coding_agent::policy::is_mutating_tool;
    let tool = BashTool::new(std::path::PathBuf::from("."));
    assert!(
        is_mutating_tool("bash"),
        "bash must be classified mutating (policy.rs)"
    );
    assert_eq!(tool.execution_mode(), ExecutionMode::Sequential);
}

/// Phase 11.6: bash must remain a single-shot command tool. Regression lock
/// against re-introducing persistent background shells / ptys / session pools.
/// Phase 15.2 moved the spawn into `LocalBashOperations::exec` (operations.rs),
/// so both the bash tool and the local backend are scanned for the forbidden
/// symbols, and the single-shot `tokio::process::Command` positive control now
/// lives in operations.rs.
#[test]
fn bash_tool_no_background_shell_symbols_guard() {
    let root = phase11_workspace_root();
    let bash_src = std::fs::read_to_string(root.join("crates/opi-coding-agent/src/tool/bash.rs"))
        .expect("read bash.rs");
    let ops_src =
        std::fs::read_to_string(root.join("crates/opi-coding-agent/src/tool/operations.rs"))
            .expect("read operations.rs");
    let bash = phase11_strip_comments(&bash_src);
    let ops = phase11_strip_comments(&ops_src);
    for needle in [
        "portable_pty",
        "portable-pty",
        "nushell",
        "session_pool",
        "background_shell",
        "BackgroundShell",
        "PtySession",
        "persistent_shell",
        "Lazy<",
        "OnceCell<",
        "static mut",
        "tokio::spawn",
    ] {
        assert!(
            !bash.contains(needle) && !ops.contains(needle),
            "bash must not use a background-shell / persistence symbol '{needle}'"
        );
    }
    // Positive control: the legitimate single-shot command primitive is present
    // in the local backend (proves the guard is not vacuously passing).
    assert!(
        ops.contains("tokio::process::Command"),
        "operations.rs (LocalBashOperations) must use tokio::process::Command for single-shot execution"
    );
}

// ---------------------------------------------------------------------------
// Symlink / junction escape regression tests
// ---------------------------------------------------------------------------

/// Helper: create a directory junction (Windows) or symlink (Unix) from
/// `link_path` pointing to `target`. Returns true if the link was created.
fn create_dir_link(link_path: &std::path::Path, target: &std::path::Path) -> bool {
    #[cfg(windows)]
    {
        // Use junction on Windows — no special privileges needed.
        let output = std::process::Command::new("cmd")
            .args([
                "/C",
                "mklink",
                "/J",
                &link_path.to_string_lossy(),
                &target.to_string_lossy(),
            ])
            .output();
        match output {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(target, link_path).is_ok()
    }
}

#[tokio::test]
async fn write_tool_rejects_symlink_escape_via_new_subpath() {
    let workspace = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();

    // workspace/link -> outside
    let link = workspace.path().join("link");
    if !create_dir_link(&link, outside.path()) {
        eprintln!("skipping: could not create directory link");
        return;
    }

    let tool = WriteTool::new(workspace.path().to_path_buf());
    let result = tool
        .execute(
            "symlink-escape-1",
            json!({ "path": "link/new/file.txt", "content": "escaped" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(
        result.is_error,
        "should reject symlink escape, got: {:?}",
        tool_result_text(&result)
    );
    let text = tool_result_text(&result);
    assert!(
        text.contains("outside the workspace"),
        "error should mention workspace boundary, got: {text}"
    );

    // Verify nothing was written outside
    assert!(
        !outside.path().join("new/file.txt").exists(),
        "file must not exist outside workspace"
    );
}

#[tokio::test]
async fn read_tool_rejects_symlink_escape_via_new_subpath() {
    let workspace = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();

    let link = workspace.path().join("link");
    if !create_dir_link(&link, outside.path()) {
        eprintln!("skipping: could not create directory link");
        return;
    }

    let tool = ReadTool::new(workspace.path().to_path_buf());
    let result = tool
        .execute(
            "symlink-escape-2",
            json!({ "path": "link/new/file.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(
        result.is_error,
        "should reject symlink escape, got: {:?}",
        tool_result_text(&result)
    );
    let text = tool_result_text(&result);
    assert!(
        text.contains("outside the workspace"),
        "error should mention workspace boundary, got: {text}"
    );
}

#[tokio::test]
async fn edit_tool_rejects_symlink_escape_via_new_subpath() {
    let workspace = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();

    let link = workspace.path().join("link");
    if !create_dir_link(&link, outside.path()) {
        eprintln!("skipping: could not create directory link");
        return;
    }

    let tool = EditTool::new(workspace.path().to_path_buf());
    let result = tool
        .execute(
            "symlink-escape-3",
            json!({
                "path": "link/new/file.txt",
                "old_string": "x",
                "new_string": "y"
            }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(
        result.is_error,
        "should reject symlink escape, got: {:?}",
        tool_result_text(&result)
    );
    let text = tool_result_text(&result);
    assert!(
        text.contains("outside the workspace"),
        "error should mention workspace boundary, got: {text}"
    );
}

// ---------------------------------------------------------------------------
// Phase 11.1 source-structure guard
// ---------------------------------------------------------------------------

fn phase11_workspace_root() -> std::path::PathBuf {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

/// Strip `//` line comments and `/* */` block comments (char-based, UTF-8 safe).
fn phase11_strip_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'/' && i + 1 < bytes.len() {
            if bytes[i + 1] == b'/' {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            } else if bytes[i + 1] == b'*' {
                let mut depth = 1;
                i += 2;
                while i < bytes.len() && depth > 0 {
                    if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                        depth += 1;
                        i += 2;
                    } else if bytes[i] == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                        depth -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                continue;
            }
        }
        if c == b'"' || c == b'\'' {
            let quote = c;
            out.push(c as char);
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    out.push(bytes[i] as char);
                    out.push(bytes[i + 1] as char);
                    i += 2;
                    continue;
                }
                out.push(bytes[i] as char);
                if bytes[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        out.push(c as char);
        i += 1;
    }
    out
}

/// Built-in tools must not hand-write `details: Some(..)`; every ToolResult is
/// built through `opi_agent::tool::result`. Scans the coding-agent tool sources
/// plus `opi_agent::tool` (the validation-error constructor), excluding
/// `result.rs` itself (the builder). Includes a vacuous-allowlist so the guard
/// cannot pass against an empty builder module.
#[test]
fn tool_result_details_use_shared_builders_guard() {
    let root = phase11_workspace_root();
    let mut scanned: Vec<std::path::PathBuf> = Vec::new();
    let tool_dir = root.join("crates/opi-coding-agent/src/tool");
    for entry in std::fs::read_dir(&tool_dir).expect("tool dir") {
        let p = entry.unwrap().path();
        // Phase 15.2: operations.rs defines a DISTINCT local ToolDiagnostic type
        // (details: Option<Value>) carried in BashResult; it is not the agent
        // ToolResult.details field this guard polices, so exclude it.
        let is_operations = p.file_name().is_some_and(|n| n == "operations.rs");
        if p.extension().is_some_and(|e| e == "rs") && !is_operations {
            scanned.push(p);
        }
    }
    scanned.push(root.join("crates/opi-agent/src/tool.rs"));

    let mut offenders: Vec<String> = Vec::new();
    for path in &scanned {
        let src = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let stripped = phase11_strip_comments(&src);
        if stripped.contains("details: Some(") || stripped.contains("details:Some(") {
            offenders.push(path.display().to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "these tool sources hand-write `details: Some(..)` instead of routing through          opi_agent::tool::result builders: {offenders:?}"
    );

    // Vacuous-allowlist: the builder module must actually define the stable keys.
    let builder = std::fs::read_to_string(root.join("crates/opi-agent/src/tool/result.rs"))
        .expect("result.rs");
    for key in [
        "workspace_root",
        "resolved_path",
        "workspace_relation",
        "command",
        "shell",
        "exit_code",
        "timed_out",
        "cancelled",
        "truncated",
    ] {
        assert!(
            builder.contains(key),
            "result.rs builder missing expected details key: {key}"
        );
    }
}

/// Phase 11.4 structural guard: WriteTool must persist atomically (sibling temp
/// file + rename, no partial writes), never a direct overwrite. Phase 15.2 moved
/// the atomic-write recipe into the local `FileOperations` backend
/// (`operations.rs::atomic_write_bytes`); write.rs delegates to the injected
/// backend, so the guard verifies the delegation plus the backend's temp+rename
/// mechanism. Behavioral no-partial/no-leak tests prove the end-to-end effect.
#[test]
fn write_uses_temp_and_rename_guard() {
    let root = phase11_workspace_root();
    let write_src = std::fs::read_to_string(root.join("crates/opi-coding-agent/src/tool/write.rs"))
        .expect("read write.rs");
    let ops_src =
        std::fs::read_to_string(root.join("crates/opi-coding-agent/src/tool/operations.rs"))
            .expect("read operations.rs");
    let write = phase11_strip_comments(&write_src);
    let ops = phase11_strip_comments(&ops_src);
    assert!(
        write.contains("ops.write_file"),
        "write.rs must delegate writing to the injected FileOperations backend"
    );
    assert!(
        ops.contains("rename"),
        "operations.rs must rename a sibling temp file into place (atomic write)"
    );
    assert!(
        !write.contains("fs::write(&file_path"),
        "write.rs must not write the final destination directly; the backend temp+renames"
    );
}

#[test]
fn write_and_edit_temp_files_have_drop_cleanup_guard() {
    let root = phase11_workspace_root();
    let helper = std::fs::read_to_string(root.join("crates/opi-coding-agent/src/tool/mod.rs"))
        .expect("read tool/mod.rs");
    let helper = phase11_strip_comments(&helper);
    assert!(
        helper.contains("struct TempFileGuard"),
        "tool/mod.rs must define a shared temp-file cleanup guard"
    );
    assert!(
        helper.contains("impl Drop for TempFileGuard"),
        "TempFileGuard must clean up on drop as a future-cancellation backstop"
    );
    assert!(
        helper.contains("std::fs::remove_file"),
        "TempFileGuard::drop must use sync remove_file because Drop cannot await"
    );

    // Phase 15.2: the atomic-write recipe (TempFileGuard + rename) moved from
    // the individual file tools into LocalFileOperations::atomic_write_bytes
    // (operations.rs). The shared guard (mod.rs) is unchanged; the backend is
    // where it is now armed, routed, cleaned up, and disarmed.
    let ops_src =
        std::fs::read_to_string(root.join("crates/opi-coding-agent/src/tool/operations.rs"))
            .expect("read operations.rs");
    let ops = phase11_strip_comments(&ops_src);
    assert!(
        ops.contains("TempFileGuard::new"),
        "operations.rs must arm the temp-file cleanup guard before writing"
    );
    assert!(
        ops.contains("guard.path()"),
        "operations.rs must route temp write/rename through the guard path"
    );
    assert!(
        ops.contains("guard.cleanup().await"),
        "operations.rs must eagerly clean up known error paths"
    );
    assert!(
        ops.contains("guard.disarm()"),
        "operations.rs must disarm the guard after successful rename"
    );
}

// ---------------------------------------------------------------------------
// Read tool hardening (Phase 11.3)
// ---------------------------------------------------------------------------

/// Phase 11.3 + 15.2 structural guard. Originally pinned that read.rs streamed
/// file bytes through a bounded read buffer. Phase 15.2 routes read through the
/// injected `FileOperations` backend (`ops.read_file`) and applies the line
/// window + byte cap on the assembled body at the tool layer. The local backend
/// reads the file fully (streaming reads are a deferred 15.1 residual); the tool
/// still caps OUTPUT at `MAX_READ_OUTPUT_BYTES`, so this guard now locks the
/// backend-delegation + output-cap contract rather than the in-tool streaming
/// loop.
#[test]
fn read_tool_routes_through_backend_and_caps_body() {
    let root = phase11_workspace_root();
    let src = std::fs::read_to_string(root.join("crates/opi-coding-agent/src/tool/read.rs"))
        .expect("read read.rs");
    let s = phase11_strip_comments(&src);
    assert!(
        s.contains("ops.read_file"),
        "read.rs must read through the injected FileOperations backend"
    );
    assert!(
        s.contains("MAX_READ_OUTPUT_BYTES"),
        "read.rs must cap the assembled body at MAX_READ_OUTPUT_BYTES"
    );
    assert!(
        !s.contains("tokio::fs::read(&file_path"),
        "read.rs must not buffer the file directly via tokio::fs; it goes through the backend"
    );
}

#[tokio::test]
async fn read_tool_line_ranges_are_stable() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("lines.txt");
    std::fs::write(&file_path, "line1\nline2\nline3\nline4\nline5").unwrap();
    let tool = ReadTool::new(dir.path().to_path_buf());

    // Full read: 1-based default offset, all five lines, not truncated.
    let r = tool
        .execute(
            "lr1",
            json!({ "path": "lines.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!r.is_error, "{}", tool_result_text(&r));
    let d = r.details.as_ref().expect("details");
    assert_eq!(d.get("line_count").and_then(|v| v.as_u64()), Some(5));
    assert_eq!(d.get("offset").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(d.get("limit").and_then(|v| v.as_u64()), Some(2000));
    assert_eq!(d.get("truncated").and_then(|v| v.as_bool()), Some(false));
    let text = tool_result_text(&r);
    for line in ["line1", "line2", "line3", "line4", "line5"] {
        assert!(text.contains(line), "expected {line} in body");
    }

    // offset=2, limit=2 selects lines 2-3 (1-based); lines 4-5 remain after the
    // window, so the result is truncated with omitted == 2.
    let r = tool
        .execute(
            "lr2",
            json!({ "path": "lines.txt", "offset": 2, "limit": 2 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!r.is_error);
    assert!(r.truncated);
    let text = tool_result_text(&r);
    assert!(text.contains("line2") && text.contains("line3"));
    assert!(!text.contains("line1") && !text.contains("line4"));
    let d = r.details.as_ref().expect("details");
    assert_eq!(d.get("offset").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(d.get("limit").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(d.get("line_count").and_then(|v| v.as_u64()), Some(5));
    assert_eq!(d.get("truncated").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(d.get("omitted").and_then(|v| v.as_u64()), Some(2));

    // offset past EOF is a clear non-error note, not a failure.
    let r = tool
        .execute(
            "lr3",
            json!({ "path": "lines.txt", "offset": 6 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!r.is_error, "{}", tool_result_text(&r));
    assert!(!r.truncated);
    let text = tool_result_text(&r);
    assert!(text.contains("past end of file") && text.contains("line_count 5"));
    let d = r.details.expect("details");
    assert_eq!(d.get("offset").and_then(|v| v.as_u64()), Some(6));
    assert_eq!(d.get("line_count").and_then(|v| v.as_u64()), Some(5));

    // Oversized limit clamps to available and is not truncated.
    let r = tool
        .execute(
            "lr4",
            json!({ "path": "lines.txt", "limit": 10 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!r.is_error);
    assert!(!r.truncated);
    let d = r.details.expect("details");
    assert_eq!(d.get("omitted").and_then(|v| v.as_u64()), Some(0));

    // offset=0 is invalid (1-based); it floors to 1, returns line1, and reports
    // offset=1 so the metadata matches the effective window.
    let r = tool
        .execute(
            "lr5",
            json!({ "path": "lines.txt", "offset": 0 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!r.is_error);
    assert!(tool_result_text(&r).contains("line1"));
    let d = r.details.expect("details");
    assert_eq!(d.get("offset").and_then(|v| v.as_u64()), Some(1));
}

#[tokio::test]
async fn read_tool_line_range_edge_cases() {
    let dir = tempfile::tempdir().unwrap();
    let tool = ReadTool::new(dir.path().to_path_buf());

    // limit: 0 on a non-empty file returns no lines and is truncated; the marker
    // is appended without a leading blank line.
    let three = dir.path().join("three.txt");
    std::fs::write(&three, "a\nb\nc").unwrap();
    let r = tool
        .execute(
            "ec1",
            json!({ "path": "three.txt", "limit": 0 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!r.is_error);
    assert!(r.truncated);
    let text = tool_result_text(&r);
    assert!(text.contains("... 3 lines omitted"));
    assert!(
        !text.contains("\n\n..."),
        "truncation marker must not follow a blank line"
    );
    let d = r.details.expect("details");
    assert_eq!(d.get("omitted").and_then(|v| v.as_u64()), Some(3));

    // Empty file: not an error, zero lines, not truncated, no past-EOF note.
    let empty = dir.path().join("empty.txt");
    std::fs::write(&empty, "").unwrap();
    let r = tool
        .execute(
            "ec2",
            json!({ "path": "empty.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!r.is_error);
    assert!(!r.truncated);
    let d = r.details.as_ref().expect("details");
    assert_eq!(d.get("line_count").and_then(|v| v.as_u64()), Some(0));
    assert_eq!(d.get("omitted").and_then(|v| v.as_u64()), Some(0));
    assert!(
        !tool_result_text(&r).contains("past end of file"),
        "empty file must not emit the past-EOF note"
    );

    // offset == total_lines returns the last line and is NOT out of range.
    let r = tool
        .execute(
            "ec3",
            json!({ "path": "three.txt", "offset": 3 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!r.is_error);
    assert!(!r.truncated);
    let d = r.details.as_ref().expect("details");
    assert_eq!(d.get("offset").and_then(|v| v.as_u64()), Some(3));
    assert_eq!(d.get("omitted").and_then(|v| v.as_u64()), Some(0));
    assert!(
        !tool_result_text(&r).contains("past end of file"),
        "offset at the last line is in range"
    );
}

#[tokio::test]
async fn read_tool_truncates_large_file_and_sets_truncated_flag() {
    let dir = tempfile::tempdir().unwrap();
    let big_path = dir.path().join("big.txt");
    // 2001 lines, no trailing newline.
    let big: String = (1..=2001u32)
        .map(|i| format!("L{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&big_path, &big).unwrap();
    let tool = ReadTool::new(dir.path().to_path_buf());

    // Default args cap at 2000 lines; 1 omitted; top-level truncated flag set.
    let r = tool
        .execute(
            "tr1",
            json!({ "path": "big.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!r.is_error, "{}", tool_result_text(&r));
    assert!(r.truncated, "top-level truncated flag must be set");
    let text = tool_result_text(&r);
    assert!(text.contains("L2000"), "line 2000 should be present");
    assert!(!text.contains("L2001"), "line 2001 must be omitted");
    assert!(
        text.contains("... 1 lines omitted"),
        "truncation marker missing: {text}"
    );
    let d = r.details.expect("details");
    assert_eq!(d.get("truncated").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(d.get("omitted").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(d.get("line_count").and_then(|v| v.as_u64()), Some(2001));
    assert_eq!(d.get("limit").and_then(|v| v.as_u64()), Some(2000));
    assert_eq!(d.get("offset").and_then(|v| v.as_u64()), Some(1));
    for key in [
        "workspace_root",
        "path",
        "resolved_path",
        "workspace_relation",
    ] {
        assert!(
            d.get(key).is_some(),
            "truncated result missing path key {key}"
        );
    }

    // Explicit limit=2001 is honored exactly (no default-cap reapply): not truncated.
    let r = tool
        .execute(
            "tr2",
            json!({ "path": "big.txt", "limit": 2001 }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!r.is_error);
    assert!(!r.truncated);
    let d = r.details.as_ref().expect("details");
    assert_eq!(d.get("omitted").and_then(|v| v.as_u64()), Some(0));
    assert!(tool_result_text(&r).contains("L2001"));

    // Boundary: exactly 2000 lines is not truncated under default args.
    let boundary_path = dir.path().join("boundary.txt");
    let boundary: String = (1..=2000u32)
        .map(|i| format!("L{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&boundary_path, &boundary).unwrap();
    let r = tool
        .execute(
            "tr3",
            json!({ "path": "boundary.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(!r.is_error);
    assert!(!r.truncated);
    let d = r.details.expect("details");
    assert_eq!(d.get("line_count").and_then(|v| v.as_u64()), Some(2000));
    assert_eq!(d.get("omitted").and_then(|v| v.as_u64()), Some(0));
}

#[tokio::test]
async fn read_tool_detects_binary_file_and_returns_diagnostic_context() {
    let dir = tempfile::tempdir().unwrap();
    let bin_path = dir.path().join("binary.bin");
    std::fs::write(&bin_path, b"hello\x00world").unwrap();
    let tool = ReadTool::new(dir.path().to_path_buf());

    let r = tool
        .execute(
            "bin1",
            json!({ "path": "binary.bin" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(r.is_error, "binary file must be reported as an error");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == code::CODE_TOOL_BINARY_FILE),
        "expected tool_binary_file diagnostic: {:?}",
        r.diagnostics
    );
    let text = tool_result_text(&r);
    assert!(
        text.contains("appears to be a binary file"),
        "binary message missing: {text}"
    );
    assert!(
        !text.contains("failed to read"),
        "must not collapse to the generic IO message"
    );
    assert!(r.details.is_none(), "error result must not carry details");
    assert!(!r.truncated);
}

#[tokio::test]
async fn read_tool_detects_nul_beyond_output_cap_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let bin_path = dir.path().join("late-nul.bin");
    let mut bytes = vec![b'a'; MAX_READ_OUTPUT_BYTES + 1];
    bytes.push(0);
    std::fs::write(&bin_path, bytes).unwrap();
    let tool = ReadTool::new(dir.path().to_path_buf());

    let r = tool
        .execute(
            "bin-late-nul",
            json!({ "path": "late-nul.bin" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(
        r.is_error,
        "NUL bytes past the output cap must still be reported as binary"
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == code::CODE_TOOL_BINARY_FILE),
        "expected tool_binary_file diagnostic: {:?}",
        r.diagnostics
    );
    assert!(r.details.is_none(), "binary error result must omit details");
}

#[tokio::test]
async fn read_file_rejects_invalid_utf8_with_unsupported_encoding() {
    let dir = tempfile::tempdir().unwrap();
    let bad_path = dir.path().join("bad_utf8.txt");
    // Valid UTF-8 prefix "abc" then invalid continuation bytes; no NUL byte.
    std::fs::write(&bad_path, b"abc\xff\xfe").unwrap();
    let tool = ReadTool::new(dir.path().to_path_buf());

    let r = tool
        .execute(
            "utf1",
            json!({ "path": "bad_utf8.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(r.is_error, "invalid UTF-8 must be reported as an error");
    let diag = r
        .diagnostics
        .iter()
        .find(|d| d.code == code::CODE_TOOL_UNSUPPORTED_ENCODING)
        .expect("tool_unsupported_encoding diagnostic");
    let text = tool_result_text(&r);
    assert!(
        text.contains("is not valid UTF-8"),
        "encoding message missing: {text}"
    );
    assert!(
        !text.contains("failed to read"),
        "must not collapse to the generic IO message"
    );
    assert!(
        !text.contains('\u{FFFD}'),
        "must not lossy-replace with U+FFFD"
    );
    let ctx = diag.context.as_object().expect("context object");
    assert!(
        ctx.get("path").and_then(|v| v.as_str()).is_some(),
        "context.path missing"
    );
    assert_eq!(
        ctx.get("byte_offset").and_then(|v| v.as_u64()),
        Some(3),
        "byte_offset should be the length of the valid UTF-8 prefix"
    );
    assert!(r.details.is_none());
    assert!(!r.truncated);
}

#[tokio::test]
async fn read_tool_returns_typed_diagnostic_context() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("file.txt"), "x").unwrap();
    std::fs::create_dir(dir.path().join("subdir")).unwrap();
    std::fs::write(dir.path().join("binary.bin"), b"hello\x00world").unwrap();
    std::fs::write(dir.path().join("bad_utf8.txt"), b"abc\xff\xfe").unwrap();
    let tool = ReadTool::new(dir.path().to_path_buf());

    // NotFound
    let r = tool
        .execute(
            "tc-notfound",
            json!({ "path": "nope.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(r.is_error);
    let d = r
        .diagnostics
        .iter()
        .find(|x| x.code == code::CODE_TOOL_PATH_NOT_FOUND)
        .expect("tool_path_not_found diagnostic");
    assert!(
        d.context
            .get("user_path")
            .and_then(|v| v.as_str())
            .is_some()
    );
    assert!(tool_result_text(&r).contains("does not exist"));
    assert!(r.details.is_none());

    // NotAFile
    let r = tool
        .execute(
            "tc-notafile",
            json!({ "path": "subdir" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(r.is_error);
    let d = r
        .diagnostics
        .iter()
        .find(|x| x.code == code::CODE_TOOL_NOT_A_FILE)
        .expect("tool_not_a_file diagnostic");
    assert!(d.context.get("path").and_then(|v| v.as_str()).is_some());
    assert!(tool_result_text(&r).contains("not a file"));

    // OutsideWorkspace (absolute outside path under WorkspaceOnly)
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("escape.txt");
    std::fs::write(&outside_file, "x").unwrap();
    let r = tool
        .execute(
            "tc-outside",
            json!({ "path": outside_file }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(r.is_error);
    let d = r
        .diagnostics
        .iter()
        .find(|x| x.code == code::CODE_TOOL_OUTSIDE_WORKSPACE)
        .expect("tool_outside_workspace diagnostic");
    assert!(
        d.context
            .get("user_path")
            .and_then(|v| v.as_str())
            .is_some()
    );
    assert!(
        d.context
            .get("symlink_traversed")
            .and_then(|v| v.as_bool())
            .is_some()
    );
    assert!(tool_result_text(&r).contains("outside the workspace"));

    // BinaryFile
    let r = tool
        .execute(
            "tc-binary",
            json!({ "path": "binary.bin" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(r.is_error);
    let d = r
        .diagnostics
        .iter()
        .find(|x| x.code == code::CODE_TOOL_BINARY_FILE)
        .expect("tool_binary_file diagnostic");
    assert!(d.context.get("path").and_then(|v| v.as_str()).is_some());

    // UnsupportedEncoding
    let r = tool
        .execute(
            "tc-encoding",
            json!({ "path": "bad_utf8.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
    assert!(r.is_error);
    let d = r
        .diagnostics
        .iter()
        .find(|x| x.code == code::CODE_TOOL_UNSUPPORTED_ENCODING)
        .expect("tool_unsupported_encoding diagnostic");
    assert!(d.context.get("path").and_then(|v| v.as_str()).is_some());
    assert_eq!(
        d.context.get("byte_offset").and_then(|v| v.as_u64()),
        Some(3)
    );
}

// ---------------------------------------------------------------------------
// Tool definition tests
// ---------------------------------------------------------------------------

#[test]
fn read_tool_has_valid_definition() {
    let tool = ReadTool::new(std::path::PathBuf::from("."));
    let def = tool.definition();
    assert_eq!(def.name, "read");
    assert!(!def.description.is_empty());
    assert!(def.input_schema.is_object());
}

#[test]
fn write_tool_has_valid_definition() {
    let tool = WriteTool::new(std::path::PathBuf::from("."));
    let def = tool.definition();
    assert_eq!(def.name, "write");
    assert!(!def.description.is_empty());
}

#[test]
fn edit_tool_has_valid_definition() {
    let tool = EditTool::new(std::path::PathBuf::from("."));
    let def = tool.definition();
    assert_eq!(def.name, "edit");
    assert!(!def.description.is_empty());
}

#[test]
fn bash_tool_has_valid_definition() {
    let tool = BashTool::new(std::path::PathBuf::from("."));
    let def = tool.definition();
    assert_eq!(def.name, "bash");
    assert!(!def.description.is_empty());
}

// ---------------------------------------------------------------------------
// Phase 11.8 S3: built-in tools ExecutionMode contract
// ---------------------------------------------------------------------------

/// Pin the `ExecutionMode` of every built-in tool so the agent loop's
/// `batch_is_sequential` classification (any Sequential tool -> serial batch)
/// holds. write/edit/bash are Sequential (mutating, side-effectful); the five
/// read-only navigation tools are Parallel. `execution_mode()` is a pure
/// declaration, so a dummy workspace root is sufficient.
#[test]
fn all_tools_execution_mode_contract() {
    use opi_coding_agent::tool::{FindTool, GlobTool, GrepTool, LsTool};
    let root = std::path::PathBuf::from(".");

    let sequential: [(&str, Box<dyn Tool>); 3] = [
        ("write", Box::new(WriteTool::new(root.clone()))),
        ("edit", Box::new(EditTool::new(root.clone()))),
        ("bash", Box::new(BashTool::new(root.clone()))),
    ];
    for (name, tool) in &sequential {
        assert_eq!(
            tool.execution_mode(),
            ExecutionMode::Sequential,
            "{name} must be Sequential"
        );
    }

    let parallel: [(&str, Box<dyn Tool>); 5] = [
        ("read", Box::new(ReadTool::new(root.clone()))),
        ("grep", Box::new(GrepTool::new(root.clone()))),
        ("find", Box::new(FindTool::new(root.clone()))),
        ("ls", Box::new(LsTool::new(root.clone()))),
        ("glob", Box::new(GlobTool::new(root.clone()))),
    ];
    for (name, tool) in &parallel {
        assert_eq!(
            tool.execution_mode(),
            ExecutionMode::Parallel,
            "{name} must be Parallel"
        );
    }

    // Sanity: exactly eight built-in tools (3 Sequential + 5 Parallel).
    assert_eq!(sequential.len(), 3);
    assert_eq!(parallel.len(), 5);
}

#[tokio::test]
async fn read_tool_result_text_uses_relative_path_for_inside_workspace() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("safe.txt"), "visible").unwrap();

    let tool = ReadTool::new(dir.path().to_path_buf());
    let result = tool
        .execute(
            "read-safe-path",
            json!({ "path": "safe.txt" }),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.is_error, "{}", tool_result_text(&result));
    let text = tool_result_text(&result);
    assert!(
        text.starts_with("safe.txt\n"),
        "read output should start with relative path, got {text:?}"
    );
    let root = dir.path().display().to_string();
    assert!(
        !text.contains(&root),
        "read text leaked workspace root: {text:?}"
    );
    let details_json = result
        .details
        .as_ref()
        .map(|d| serde_json::to_string(d).unwrap())
        .unwrap_or_default();
    assert!(
        !details_json.contains(&root),
        "read details leaked workspace root: {details_json:?}"
    );
}
