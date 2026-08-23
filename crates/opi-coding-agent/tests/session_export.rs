//! Phase 13.5 acceptance: local session export with redaction.
//!
//! DoD scenario `phase13-local-session-export-redacted`: the local export
//! command renders markdown and JSON from typed session reads / context
//! results, chooses active branch or full tree, applies Phase 7 redaction
//! plus tool-output / thinking omission flags, writes only the requested
//! output file, and leaves the source session byte-for-byte identical on
//! success and failure. No sharing / publishing path exists.

use std::path::PathBuf;

use opi_agent::message::AgentMessage;
use opi_agent::session::{
    BranchSummaryEntry, SessionEntry, SessionHeader, SessionInfoEntry, SessionReader, SessionWriter,
};
use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, Message, OutputContent, ToolCall,
    ToolResultMessage, UserMessage,
};
use opi_coding_agent::cli::{ExportFormat, ExportRedactMode};
use opi_coding_agent::session_cli::{ExportError, ExportOptions, ExportScope, export_session};

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

const TS: &str = "2026-07-05T00:00:00Z";

fn set_sessions_dir(dir: &std::path::Path) {
    // SAFETY: test-only env var mutation; tests acquire SESSION_MUTEX first.
    unsafe {
        std::env::set_var("OPI_SESSIONS_DIR", dir);
    }
}

fn clear_sessions_dir() {
    // SAFETY: test-only env var mutation; tests acquire SESSION_MUTEX first.
    unsafe {
        std::env::remove_var("OPI_SESSIONS_DIR");
    }
}

fn session_lock() -> std::sync::MutexGuard<'static, ()> {
    static SESSION_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
    SESSION_MUTEX
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn user_msg(id: &str, parent: Option<&str>, text: &str) -> SessionEntry {
    SessionEntry::Message(opi_agent::session::MessageEntry {
        id: id.into(),
        parent_id: parent.map(str::to_owned),
        timestamp: TS.into(),
        message: Message::User(UserMessage {
            content: vec![InputContent::Text { text: text.into() }],
            timestamp_ms: 0,
        }),
    })
}

fn assistant_msg(id: &str, parent: Option<&str>, text: &str) -> SessionEntry {
    SessionEntry::Message(opi_agent::session::MessageEntry {
        id: id.into(),
        parent_id: parent.map(str::to_owned),
        timestamp: TS.into(),
        message: Message::Assistant(AssistantMessage {
            content: vec![AssistantContent::Text { text: text.into() }],
            api: opi_ai::ApiKind::Anthropic,
            provider: "anthropic".into(),
            model: "claude-sonnet-4".into(),
            response_model: None,
            response_id: None,
            usage: Default::default(),
            stop_reason: opi_ai::stream::StopReason::Stop,
            error_message: None,
            timestamp_ms: 0,
        }),
    })
}

fn assistant_with_thinking(
    id: &str,
    parent: Option<&str>,
    thinking: &str,
    text: &str,
) -> SessionEntry {
    SessionEntry::Message(opi_agent::session::MessageEntry {
        id: id.into(),
        parent_id: parent.map(str::to_owned),
        timestamp: TS.into(),
        message: Message::Assistant(AssistantMessage {
            content: vec![
                AssistantContent::Thinking {
                    thinking: thinking.into(),
                },
                AssistantContent::Text { text: text.into() },
            ],
            api: opi_ai::ApiKind::Anthropic,
            provider: "anthropic".into(),
            model: "claude-sonnet-4".into(),
            response_model: None,
            response_id: None,
            usage: Default::default(),
            stop_reason: opi_ai::stream::StopReason::Stop,
            error_message: None,
            timestamp_ms: 0,
        }),
    })
}

fn assistant_with_tool_call(
    id: &str,
    parent: Option<&str>,
    call_id: &str,
    tool: &str,
    args: &str,
) -> SessionEntry {
    SessionEntry::Message(opi_agent::session::MessageEntry {
        id: id.into(),
        parent_id: parent.map(str::to_owned),
        timestamp: TS.into(),
        message: Message::Assistant(AssistantMessage {
            content: vec![AssistantContent::ToolCall {
                tool_call: ToolCall {
                    id: call_id.into(),
                    name: tool.into(),
                    arguments: args.into(),
                },
            }],
            api: opi_ai::ApiKind::Anthropic,
            provider: "anthropic".into(),
            model: "claude-sonnet-4".into(),
            response_model: None,
            response_id: None,
            usage: Default::default(),
            stop_reason: opi_ai::stream::StopReason::ToolUse,
            error_message: None,
            timestamp_ms: 0,
        }),
    })
}

fn tool_result_msg(
    id: &str,
    parent: Option<&str>,
    call_id: &str,
    tool: &str,
    output: &str,
) -> SessionEntry {
    SessionEntry::Message(opi_agent::session::MessageEntry {
        id: id.into(),
        parent_id: parent.map(str::to_owned),
        timestamp: TS.into(),
        message: Message::ToolResult(ToolResultMessage {
            tool_call_id: call_id.into(),
            tool_name: tool.into(),
            content: vec![OutputContent::Text {
                text: output.into(),
            }],
            details: None,
            is_error: false,
            truncated: false,
            timestamp_ms: 0,
        }),
    })
}

fn leaf(id: &str, parent: Option<&str>, entry_id: &str) -> SessionEntry {
    SessionEntry::Leaf(opi_agent::session::LeafEntry {
        id: id.into(),
        parent_id: parent.map(str::to_owned),
        timestamp: TS.into(),
        entry_id: entry_id.into(),
    })
}

fn branch_summary(id: &str, parent: Option<&str>, summary: &str) -> SessionEntry {
    SessionEntry::BranchSummary(BranchSummaryEntry {
        id: id.into(),
        parent_id: parent.map(str::to_owned),
        timestamp: TS.into(),
        summary: summary.into(),
    })
}

fn session_info(id: &str, parent: Option<&str>, name: &str) -> SessionEntry {
    SessionEntry::SessionInfo(SessionInfoEntry {
        id: id.into(),
        parent_id: parent.map(str::to_owned),
        timestamp: TS.into(),
        name: name.into(),
    })
}

/// Build a session JSONL on disk inside `dir` with the given entries and
/// return `(session_id, path)`. The active tip is set via a trailing Leaf
/// when `active_tip_entry_id` is `Some`.
fn write_session(
    dir: &std::path::Path,
    id: &str,
    entries: &[SessionEntry],
    active_tip_entry_id: Option<&str>,
) -> PathBuf {
    let path = dir.join(format!("{id}.jsonl"));
    let header = SessionHeader::new_for_test(id.into(), TS.into(), "/workspace".into(), None);
    let mut writer = SessionWriter::create(&path, header).expect("create session");
    for entry in entries {
        writer.append(entry).expect("append entry");
    }
    if let Some(tip) = active_tip_entry_id {
        writer
            .append(&leaf(&format!("leaf-{id}"), Some(tip), tip))
            .expect("append leaf");
    }
    drop(writer);
    path
}

fn read_bytes(path: &std::path::Path) -> Vec<u8> {
    std::fs::read(path).expect("read file bytes")
}

/// A canary Anthropic-shaped API key with >= 20 chars after the prefix so the
/// SecretRedactor pattern `sk-ant-[a-zA-Z0-9-]{20,}` matches it.
const KEY_CANARY: &str = "sk-ant-FAKE1234567890abcdefghijklmnop";

/// A POSIX absolute path canary matched by the Summary-mode ABSOLUTE_PATH_RE.
const PATH_CANARY: &str = "/tmp/secret-blob/data.dat";

// ---------------------------------------------------------------------------
// Slice 1: active-branch markdown export with redaction, source unchanged
// ---------------------------------------------------------------------------

#[test]
fn phase13_export_active_branch_markdown_redacts_and_preserves_source() {
    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());

    let entries = vec![
        user_msg("m1", None, &format!("My key is {KEY_CANARY}")),
        assistant_msg("m2", Some("m1"), &format!("Stored at {PATH_CANARY}")),
    ];
    let path = write_session(sessions.path(), "sess-md", &entries, Some("m2"));
    let before = read_bytes(&path);

    let output = out_dir.path().join("export.md");
    let options = ExportOptions {
        session_ref: "sess-md".into(),
        format: ExportFormat::Markdown,
        output: output.clone(),
        scope: ExportScope::ActiveBranch,
        include_tool_output: true,
        include_thinking: true,
        redact: ExportRedactMode::Summary,
    };
    export_session(&options).expect("markdown export succeeds");

    let exported = std::fs::read_to_string(&output).expect("read export");
    assert!(
        !exported.contains(KEY_CANARY),
        "API key must be redacted from markdown export: {exported}"
    );
    assert!(
        exported.contains("[REDACTED]"),
        "redaction marker expected in markdown export: {exported}"
    );
    assert!(
        !exported.contains(PATH_CANARY),
        "absolute path must be redacted in summary mode: {exported}"
    );

    // Active-branch content is still rendered.
    assert!(
        exported.contains("## ") || exported.contains("User") || exported.contains("Assistant"),
        "markdown should include role headings: {exported}"
    );

    let after = read_bytes(&path);
    assert_eq!(
        before, after,
        "source session bytes must be unchanged after a successful export"
    );

    clear_sessions_dir();
}

#[test]
fn phase13_export_verbose_preserves_paths_but_redacts_secrets() {
    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());

    let entries = vec![user_msg(
        "m1",
        None,
        &format!("path={PATH_CANARY} key={KEY_CANARY}"),
    )];
    let path = write_session(sessions.path(), "sess-verbose", &entries, Some("m1"));
    let before = read_bytes(&path);

    let output = out_dir.path().join("verbose.md");
    export_session(&ExportOptions {
        session_ref: "sess-verbose".into(),
        format: ExportFormat::Markdown,
        output: output.clone(),
        scope: ExportScope::ActiveBranch,
        include_tool_output: true,
        include_thinking: true,
        redact: ExportRedactMode::Verbose,
    })
    .expect("verbose export succeeds");

    let exported = std::fs::read_to_string(&output).unwrap();
    assert!(
        exported.contains(PATH_CANARY),
        "verbose export should preserve path evidence: {exported}"
    );
    assert!(
        !exported.contains(KEY_CANARY),
        "verbose export must still redact secrets: {exported}"
    );
    assert!(exported.contains("[REDACTED]"));
    assert_eq!(before, read_bytes(&path), "source session unchanged");

    clear_sessions_dir();
}

#[test]
fn phase13_export_renders_branch_summary_entry() {
    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());

    let entries = vec![
        user_msg("m1", None, "root"),
        branch_summary(
            "bs1",
            Some("m1"),
            &format!("parent branch carried {KEY_CANARY}"),
        ),
        assistant_msg("m2", Some("m1"), "active child"),
    ];
    write_session(sessions.path(), "sess-branch-summary", &entries, Some("m2"));

    let md_output = out_dir.path().join("branch-summary.md");
    export_session(&ExportOptions {
        session_ref: "sess-branch-summary".into(),
        format: ExportFormat::Markdown,
        output: md_output.clone(),
        scope: ExportScope::ActiveBranch,
        include_tool_output: true,
        include_thinking: true,
        redact: ExportRedactMode::Summary,
    })
    .expect("markdown export succeeds");
    let md = std::fs::read_to_string(&md_output).unwrap();
    assert!(md.contains("## Branch Summary"), "{md}");
    assert!(!md.contains(KEY_CANARY), "{md}");
    assert!(md.contains("[REDACTED]"), "{md}");

    let json_output = out_dir.path().join("branch-summary.json");
    export_session(&ExportOptions {
        session_ref: "sess-branch-summary".into(),
        format: ExportFormat::Json,
        output: json_output.clone(),
        scope: ExportScope::ActiveBranch,
        include_tool_output: true,
        include_thinking: true,
        redact: ExportRedactMode::Summary,
    })
    .expect("json export succeeds");
    let value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&json_output).unwrap()).unwrap();
    let branch_summary = value["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|msg| msg["role"] == "branch_summary")
        .expect("branch_summary message exported");
    assert_eq!(branch_summary["parent_session_id"], "");
    assert_eq!(branch_summary["entry_count"], 0);
    assert!(
        !branch_summary["summary"]
            .as_str()
            .unwrap_or_default()
            .contains(KEY_CANARY)
    );

    clear_sessions_dir();
}

#[test]
fn phase13_unicode_metadata_round_trips_through_context_and_export() {
    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());

    let unicode_name = "session-\u{4f1a}\u{8bdd}";
    let unicode_summary = "summary-\u{5206}\u{652f}-\u{2713}";
    let entries = vec![
        user_msg("m1", None, "root"),
        session_info("info1", Some("m1"), unicode_name),
        branch_summary("bs1", Some("m1"), unicode_summary),
        assistant_msg("m2", Some("m1"), "active child"),
    ];
    let path = write_session(sessions.path(), "sess-unicode", &entries, Some("m2"));

    let (_header, read_entries, recovery) = SessionReader::read_with_recovery(&path).unwrap();
    assert!(recovery.is_clean());
    let ctx = opi_agent::session_context::reconstruct_context(&read_entries, &recovery);
    assert_eq!(ctx.session_name.as_deref(), Some(unicode_name));
    assert!(ctx.messages.iter().any(|msg| {
        matches!(
            msg,
            AgentMessage::BranchSummary(summary) if summary.summary == unicode_summary
        )
    }));

    let output = out_dir.path().join("unicode.md");
    export_session(&ExportOptions {
        session_ref: "sess-unicode".into(),
        format: ExportFormat::Markdown,
        output: output.clone(),
        scope: ExportScope::ActiveBranch,
        include_tool_output: true,
        include_thinking: true,
        redact: ExportRedactMode::None,
    })
    .expect("unicode export succeeds");
    let md = std::fs::read_to_string(&output).unwrap();
    assert!(
        md.contains(unicode_summary),
        "unicode branch summary must survive export: {md}"
    );

    clear_sessions_dir();
}

// ---------------------------------------------------------------------------
// Slice 2: JSON renderer is deterministic and redacts
// ---------------------------------------------------------------------------

#[test]
fn phase13_export_active_branch_json_is_deterministic_and_redacted() {
    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());

    let entries = vec![
        user_msg("m1", None, &format!("My key is {KEY_CANARY}")),
        assistant_msg("m2", Some("m1"), &format!("Stored at {PATH_CANARY}")),
    ];
    let path = write_session(sessions.path(), "sess-json", &entries, Some("m2"));
    let before = read_bytes(&path);

    let mk = || ExportOptions {
        session_ref: "sess-json".into(),
        format: ExportFormat::Json,
        output: out_dir.path().join("export.json"),
        scope: ExportScope::ActiveBranch,
        include_tool_output: true,
        include_thinking: true,
        redact: ExportRedactMode::Summary,
    };
    export_session(&mk()).expect("json export 1");
    let first = std::fs::read_to_string(out_dir.path().join("export.json")).unwrap();
    export_session(&mk()).expect("json export 2");
    let second = std::fs::read_to_string(out_dir.path().join("export.json")).unwrap();
    assert_eq!(
        first, second,
        "JSON export must be deterministic for the same input"
    );

    assert!(
        !first.contains(KEY_CANARY),
        "API key must be redacted from JSON export: {first}"
    );
    // The exported payload must parse as JSON.
    let parsed: serde_json::Value = serde_json::from_str(&first).expect("export is valid JSON");
    assert!(parsed.is_object(), "export root should be an object");

    let after = read_bytes(&path);
    assert_eq!(before, after, "source bytes unchanged after json export");

    clear_sessions_dir();
}

// ---------------------------------------------------------------------------
// Slice 3: full-tree export includes sibling-branch messages
// ---------------------------------------------------------------------------

#[test]
fn phase13_export_full_tree_includes_sibling_branch_messages() {
    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());

    // Active branch: m1 -> m2 (tip = m2). Sibling branch: m1 -> m3.
    // Active-branch export would omit m3; full-tree must include it.
    let entries = vec![
        user_msg("m1", None, "trunk message ON_BRANCH_CANARY"),
        assistant_msg("m2", Some("m1"), "active branch reply"),
        assistant_msg("m3", Some("m1"), "sibling branch reply SIBLING_CANARY"),
    ];
    let path = write_session(sessions.path(), "sess-tree", &entries, Some("m2"));

    // Active-branch export: sibling omitted.
    let active_out = out_dir.path().join("active.md");
    export_session(&ExportOptions {
        session_ref: "sess-tree".into(),
        format: ExportFormat::Markdown,
        output: active_out.clone(),
        scope: ExportScope::ActiveBranch,
        include_tool_output: true,
        include_thinking: true,
        redact: ExportRedactMode::None,
    })
    .expect("active export");
    let active_md = std::fs::read_to_string(&active_out).unwrap();
    assert!(
        !active_md.contains("SIBLING_CANARY"),
        "active-branch export must omit sibling messages: {active_md}"
    );

    // Full-tree export: sibling present.
    let full_out = out_dir.path().join("full.md");
    export_session(&ExportOptions {
        session_ref: "sess-tree".into(),
        format: ExportFormat::Markdown,
        output: full_out.clone(),
        scope: ExportScope::FullTree,
        include_tool_output: true,
        include_thinking: true,
        redact: ExportRedactMode::None,
    })
    .expect("full-tree export");
    let full_md = std::fs::read_to_string(&full_out).unwrap();
    assert!(
        full_md.contains("SIBLING_CANARY"),
        "full-tree export must include sibling-branch messages: {full_md}"
    );
    assert!(
        full_md.contains("trunk message"),
        "full-tree export must include trunk messages: {full_md}"
    );

    let _ = read_bytes(&path); // source never written
    clear_sessions_dir();
}

// ---------------------------------------------------------------------------
// Slice 4a: exclude-tool-output / exclude-thinking flags
// ---------------------------------------------------------------------------

#[test]
fn phase13_export_excludes_tool_output_and_thinking_when_requested() {
    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());

    let entries = vec![
        user_msg("m1", None, "please check the file"),
        assistant_with_tool_call("m2", Some("m1"), "tc1", "bash", "{}"),
        tool_result_msg("m3", Some("m2"), "tc1", "bash", "TOOL_OUTPUT_CANARY"),
        assistant_with_thinking("m4", Some("m3"), "THINKING_CANARY", "all done"),
    ];
    write_session(sessions.path(), "sess-x", &entries, Some("m4"));

    // Both excluded.
    let out = out_dir.path().join("excluded.md");
    export_session(&ExportOptions {
        session_ref: "sess-x".into(),
        format: ExportFormat::Markdown,
        output: out.clone(),
        scope: ExportScope::ActiveBranch,
        include_tool_output: false,
        include_thinking: false,
        redact: ExportRedactMode::None,
    })
    .expect("exclude export");
    let md = std::fs::read_to_string(&out).unwrap();
    assert!(
        !md.contains("TOOL_OUTPUT_CANARY"),
        "tool output must be omitted: {md}"
    );
    assert!(
        !md.contains("THINKING_CANARY"),
        "thinking must be omitted: {md}"
    );

    // Both included.
    let out2 = out_dir.path().join("included.md");
    export_session(&ExportOptions {
        session_ref: "sess-x".into(),
        format: ExportFormat::Markdown,
        output: out2.clone(),
        scope: ExportScope::ActiveBranch,
        include_tool_output: true,
        include_thinking: true,
        redact: ExportRedactMode::None,
    })
    .expect("include export");
    let md2 = std::fs::read_to_string(&out2).unwrap();
    assert!(
        md2.contains("TOOL_OUTPUT_CANARY"),
        "tool output must be present when included: {md2}"
    );
    assert!(
        md2.contains("THINKING_CANARY"),
        "thinking must be present when included: {md2}"
    );

    clear_sessions_dir();
}

// ---------------------------------------------------------------------------
// Slice 4b: invalid output path fails AND preserves source
// ---------------------------------------------------------------------------

#[test]
fn phase13_export_invalid_output_path_fails_and_preserves_source() {
    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());

    let entries = vec![
        user_msg("m1", None, "hello"),
        assistant_msg("m2", Some("m1"), "world"),
    ];
    let path = write_session(sessions.path(), "sess-err", &entries, Some("m2"));
    let before = read_bytes(&path);

    // Output path inside a directory that does not exist -> write must fail.
    let bad_output = sessions.path().join("nonexistent-subdir").join("out.md");
    let result = export_session(&ExportOptions {
        session_ref: "sess-err".into(),
        format: ExportFormat::Markdown,
        output: bad_output.clone(),
        scope: ExportScope::ActiveBranch,
        include_tool_output: true,
        include_thinking: true,
        redact: ExportRedactMode::Summary,
    });

    assert!(
        matches!(result, Err(ExportError::Write(_))),
        "expected Write error for invalid output path, got {result:?}"
    );
    assert!(
        !bad_output.exists(),
        "no partial output file should be created on write failure"
    );

    let after = read_bytes(&path);
    assert_eq!(
        before, after,
        "source session bytes must be unchanged after a failed export"
    );

    clear_sessions_dir();
}

// ---------------------------------------------------------------------------
// Slice 4c: not-found session surfaces NotFound
// ---------------------------------------------------------------------------

#[test]
fn phase13_export_unknown_session_returns_not_found() {
    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());

    let result = export_session(&ExportOptions {
        session_ref: "does-not-exist".into(),
        format: ExportFormat::Markdown,
        output: out_dir.path().join("out.md"),
        scope: ExportScope::ActiveBranch,
        include_tool_output: true,
        include_thinking: true,
        redact: ExportRedactMode::Summary,
    });
    assert!(
        matches!(result, Err(ExportError::NotFound(_))),
        "expected NotFound for missing session, got {result:?}"
    );

    clear_sessions_dir();
}
