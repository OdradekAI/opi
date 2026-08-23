//! Session runtime integration tests (task 5).
//!
//! Tests the full lifecycle: harness creates a session, persists messages as
//! JSONL, and the session can be read back and reconstructed for resume.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::StreamExt;
use futures_util::stream;
use opi_agent::diagnostic::{SOURCE_SESSION, code};
use opi_agent::message::AgentMessage;
use opi_agent::session::{MessageEntry, SessionEntry, SessionHeader, SessionReader, SessionWriter};
use opi_agent::session_event::ThinkingLevel;
use opi_ai::message::{
    AssistantContent, InputContent, Message, OutputContent, ToolCall, ToolResultMessage,
    UserMessage,
};
use opi_ai::provider::{EventStream, Provider, ProviderError, Request};
use opi_ai::registry::ModelCapabilities;
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::test_support::{self, MockProvider};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::harness::CodingHarness;
use opi_coding_agent::session_coordinator::SessionCoordinator;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_header(id: &str, cwd: &str) -> SessionHeader {
    SessionHeader::new_for_test(id.into(), "2026-05-22T12:00:00Z".into(), cwd.into(), None)
}

fn test_message_entry(id: &str, text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        id: id.into(),
        parent_id: None,
        timestamp: "2026-05-22T12:00:01Z".into(),
        message: Message::User(UserMessage {
            content: vec![InputContent::Text { text: text.into() }],
            timestamp_ms: 0,
        }),
    })
}

/// Build a user Message entry chained to `parent` (Phase 13.3 fixture helper).
/// The opi-agent context builder walks the `parent_id` chain, so linear-chain
/// fixtures must set `parent_id` explicitly — rootless entries are treated as
/// siblings, not a linear chain.
fn chained_message_entry(id: &str, parent: &str, text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        id: id.into(),
        parent_id: Some(parent.into()),
        timestamp: "2026-05-22T12:00:01Z".into(),
        message: Message::User(UserMessage {
            content: vec![InputContent::Text { text: text.into() }],
            timestamp_ms: 0,
        }),
    })
}

fn message_entry_count(entries: &[SessionEntry]) -> usize {
    entries
        .iter()
        .filter(|entry| matches!(entry, SessionEntry::Message(_)))
        .count()
}

/// Set env var safely (edition 2024 requires unsafe for set_var).
fn set_sessions_dir(dir: &std::path::Path) {
    // SAFETY: test-only env var mutation; no other thread reads this var
    // concurrently during the test.
    unsafe {
        std::env::set_var("OPI_SESSIONS_DIR", dir);
    }
}

/// Remove env var safely (edition 2024 requires unsafe for remove_var).
fn clear_sessions_dir() {
    // SAFETY: test-only env var mutation; no other thread reads this var
    // concurrently during the test.
    unsafe {
        std::env::remove_var("OPI_SESSIONS_DIR");
    }
}

// ---------------------------------------------------------------------------
// SessionCoordinator tests
// ---------------------------------------------------------------------------

#[test]
fn session_coordinator_creates_jsonl_file() {
    let dir = tempfile::tempdir().unwrap();
    let coord = SessionCoordinator::new_for_test(
        dir.path(),
        "/test/cwd",
        opi_agent::compaction::CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    // Verify file exists
    let jsonl_files: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
        .collect();
    assert_eq!(jsonl_files.len(), 1, "should create exactly one JSONL file");

    // Verify header is readable
    let (header, entries) = SessionReader::read_all(&jsonl_files[0].path()).unwrap();
    assert_eq!(header.type_, "session");
    assert_eq!(header.cwd, "/test/cwd");
    assert_eq!(header.id, coord.session_id());
    assert!(entries.is_empty(), "fresh session should have no entries");
}

#[test]
fn session_coordinator_rapid_creations_use_distinct_paths() {
    let dir = tempfile::tempdir().unwrap();
    let start = Arc::new(std::sync::Barrier::new(16));
    let mut paths = (0..16)
        .map(|_| {
            let start = start.clone();
            let path = dir.path().to_path_buf();
            std::thread::spawn(move || {
                start.wait();
                SessionCoordinator::new_for_test(
                    &path,
                    "/test/cwd",
                    opi_agent::compaction::CompactionConfig::default(),
                    "anthropic:claude-sonnet-4",
                )
                .unwrap()
                .session_path()
                .to_path_buf()
            })
        })
        .collect::<Vec<_>>();

    let mut paths = paths
        .drain(..)
        .map(|thread| thread.join().expect("session creation thread panicked"))
        .collect::<Vec<_>>();

    paths.sort();
    paths.dedup();
    assert_eq!(paths.len(), 16, "each new session must own a distinct file");
}

#[test]
fn session_coordinator_persists_messages_on_turn_end() {
    let dir = tempfile::tempdir().unwrap();
    let mut coord = SessionCoordinator::new_for_test(
        dir.path(),
        "/test",
        opi_agent::compaction::CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    let messages = vec![
        AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "Hello".into(),
            }],
            timestamp_ms: 0,
        })),
        AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "World".into(),
            }],
            timestamp_ms: 0,
        })),
    ];

    coord
        .on_turn_end_simple(&messages, &opi_ai::stream::Usage::default())
        .unwrap();

    // Read back
    let jsonl_path = dir.path().join(format!("{}.jsonl", coord.session_id()));
    let (_header, entries) = SessionReader::read_all(&jsonl_path).unwrap();
    assert_eq!(
        message_entry_count(&entries),
        2,
        "should have two message entries"
    );
    assert!(
        matches!(entries.last(), Some(SessionEntry::Leaf(_))),
        "runtime should update the active branch leaf"
    );
}

#[test]
fn phase13_model_thinking_metadata_does_not_advance_leaf() {
    //! Phase 13.3 acceptance: idle set_model_validated/set_thinking_level
    //! appends must persist `model_change` and `thinking_level_change` entries
    //! parented to the current content tip WITHOUT appending a new Leaf or
    //! changing the active content tip. This is the coordinator-level half of
    //! DoD scenario `phase13-model-thinking-write-read-and-branch-summary-runtime`.

    use opi_agent::session::SessionEntry;
    use opi_agent::session_event::ThinkingLevel;

    let dir = tempfile::tempdir().unwrap();
    let mut coord = SessionCoordinator::new_for_test(
        dir.path(),
        "/test",
        opi_agent::compaction::CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    let messages = vec![AgentMessage::Llm(Message::User(UserMessage {
        content: vec![InputContent::Text {
            text: "Hello".into(),
        }],
        timestamp_ms: 0,
    }))];
    coord
        .on_turn_end_simple(&messages, &opi_ai::stream::Usage::default())
        .unwrap();

    let jsonl_path = dir.path().join(format!("{}.jsonl", coord.session_id()));
    let (_header, entries_before) = SessionReader::read_all(&jsonl_path).unwrap();

    // After the turn there is exactly one Leaf pointing at the message tip.
    let leaf_count_before = entries_before
        .iter()
        .filter(|e| matches!(e, SessionEntry::Leaf(_)))
        .count();
    assert_eq!(leaf_count_before, 1, "turn should append one Leaf");

    let tip_before = entries_before
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEntry::Leaf(l) => Some(l.entry_id.clone()),
            _ => None,
        })
        .expect("at least one Leaf");
    let msg_tip_id = entries_before
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEntry::Message(m) => Some(m.id.clone()),
            _ => None,
        })
        .expect("at least one Message");

    // Phase 13.3: typed metadata appends through the coordinator.
    coord
        .append_model_change(
            "anthropic:claude-opus-4".into(),
            opi_agent::session::ModelInputSource::Canonical,
        )
        .unwrap();
    coord
        .append_thinking_level_change(ThinkingLevel::High)
        .unwrap();

    let (_header, entries_after) = SessionReader::read_all(&jsonl_path).unwrap();

    // 1. The model_change and thinking_level_change entries were persisted,
    //    parented to the content tip (the message tip id).
    let model_change = entries_after.iter().find_map(|e| match e {
        SessionEntry::ModelChange(m) => Some(m.clone()),
        _ => None,
    });
    let thinking_change = entries_after.iter().find_map(|e| match e {
        SessionEntry::ThinkingLevelChange(t) => Some(t.clone()),
        _ => None,
    });
    let model_change = model_change.expect("model_change entry persisted");
    let thinking_change = thinking_change.expect("thinking_level_change entry persisted");
    assert_eq!(model_change.model, "anthropic:claude-opus-4");
    assert_eq!(thinking_change.level, ThinkingLevel::High);
    assert_eq!(
        model_change.parent_id.as_deref(),
        Some(msg_tip_id.as_str()),
        "model_change must be parented to the content tip"
    );
    assert_eq!(
        thinking_change.parent_id.as_deref(),
        Some(msg_tip_id.as_str()),
        "thinking_level_change must be parented to the content tip"
    );

    // 2. NO new Leaf was appended (metadata does not advance the content tip).
    let leaf_count_after = entries_after
        .iter()
        .filter(|e| matches!(e, SessionEntry::Leaf(_)))
        .count();
    assert_eq!(
        leaf_count_after, leaf_count_before,
        "metadata append must not add a Leaf"
    );

    // 3. The active content tip is unchanged.
    let tip_after = entries_after
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEntry::Leaf(l) => Some(l.entry_id.clone()),
            _ => None,
        })
        .expect("at least one Leaf");
    assert_eq!(
        tip_after, tip_before,
        "metadata append must not change the active content tip"
    );
}

#[test]
fn phase13_session_info_and_label_appends_are_durable_and_do_not_advance_leaf() {
    //! Phase 13.4 acceptance: idle `/name`, `/label`, and `/unlabel` appends
    //! through SessionCoordinator persist `session_info` and `label` entries
    //! parented to the current content tip WITHOUT appending a new Leaf or
    //! changing the active content tip. The in-memory accessors surface the
    //! latest name and the deduplicated, Add/Remove-applied label set.

    use opi_agent::session::{LabelAction, SessionEntry};

    let dir = tempfile::tempdir().unwrap();
    let mut coord = SessionCoordinator::new_for_test(
        dir.path(),
        "/test",
        opi_agent::compaction::CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    let messages = vec![AgentMessage::Llm(Message::User(UserMessage {
        content: vec![InputContent::Text {
            text: "Hello".into(),
        }],
        timestamp_ms: 0,
    }))];
    coord
        .on_turn_end_simple(&messages, &opi_ai::stream::Usage::default())
        .unwrap();

    let jsonl_path = dir.path().join(format!("{}.jsonl", coord.session_id()));
    let (_header, entries_before) = SessionReader::read_all(&jsonl_path).unwrap();
    let leaf_count_before = entries_before
        .iter()
        .filter(|e| matches!(e, SessionEntry::Leaf(_)))
        .count();
    assert_eq!(leaf_count_before, 1, "turn should append one Leaf");
    let msg_tip_id = entries_before
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEntry::Message(m) => Some(m.id.clone()),
            _ => None,
        })
        .expect("at least one Message");
    let tip_before = entries_before
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEntry::Leaf(l) => Some(l.entry_id.clone()),
            _ => None,
        })
        .expect("at least one Leaf");

    // Phase 13.4 typed metadata appends through the coordinator.
    coord.append_session_info("project-x".into()).unwrap();
    coord.append_label("bug".into(), LabelAction::Add).unwrap();
    coord
        .append_label("draft".into(), LabelAction::Add)
        .unwrap();
    coord
        .append_label("bug".into(), LabelAction::Remove)
        .unwrap();

    let (_header, entries_after) = SessionReader::read_all(&jsonl_path).unwrap();

    // 1. session_info entry persisted, parented to the content tip.
    let session_info = entries_after
        .iter()
        .find_map(|e| match e {
            SessionEntry::SessionInfo(s) => Some(s.clone()),
            _ => None,
        })
        .expect("session_info entry persisted");
    assert_eq!(session_info.name, "project-x");
    assert_eq!(
        session_info.parent_id.as_deref(),
        Some(msg_tip_id.as_str()),
        "session_info must be parented to the content tip"
    );

    // 2. label entries persisted with correct actions, parented to the tip.
    let label_entries: Vec<_> = entries_after
        .iter()
        .filter_map(|e| match e {
            SessionEntry::Label(l) => Some(l.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        label_entries.len(),
        3,
        "three label appends expected (bug add, draft add, bug remove)"
    );
    for label in &label_entries {
        assert_eq!(
            label.parent_id.as_deref(),
            Some(msg_tip_id.as_str()),
            "label entries must be parented to the content tip"
        );
    }

    // 3. NO new Leaf was appended and the active content tip is unchanged.
    let leaf_count_after = entries_after
        .iter()
        .filter(|e| matches!(e, SessionEntry::Leaf(_)))
        .count();
    assert_eq!(
        leaf_count_after, leaf_count_before,
        "metadata append must not add a Leaf"
    );
    let tip_after = entries_after
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEntry::Leaf(l) => Some(l.entry_id.clone()),
            _ => None,
        })
        .expect("at least one Leaf");
    assert_eq!(tip_after, tip_before, "metadata must not change the tip");

    // 4. In-memory accessors reflect the latest name + deduplicated label set
    //    (bug removed, draft kept). This is the live read path used by
    //    `/session info` and RPC `session_info`.
    assert_eq!(coord.name(), Some("project-x"));
    assert_eq!(coord.labels(), &["draft".to_string()]);
    assert_eq!(
        coord.active_branch_id(),
        Some(msg_tip_id.as_str()),
        "active_branch_id must report the content tip"
    );
}

#[test]
fn phase13_empty_string_name_and_label_are_preserved_by_coordinator_api() {
    use opi_agent::session::LabelAction;

    let dir = tempfile::tempdir().unwrap();
    let mut coord = SessionCoordinator::new_for_test(
        dir.path(),
        "/test",
        opi_agent::compaction::CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    coord.append_session_info(String::new()).unwrap();
    coord.append_label(String::new(), LabelAction::Add).unwrap();

    assert_eq!(coord.name(), Some(""));
    assert_eq!(coord.labels(), &["".to_string()]);

    let (_header, entries) = SessionReader::read_all(coord.session_path()).unwrap();
    let ctx = opi_agent::session_context::reconstruct_context(
        &entries,
        &opi_agent::session::CrashRecovery::default(),
    );
    assert_eq!(ctx.session_name.as_deref(), Some(""));
    assert_eq!(ctx.labels, vec!["".to_string()]);
}

#[test]
fn phase13_coordinator_open_existing_seeds_name_and_labels() {
    //! Phase 13.4 acceptance: reopening a session via `open_existing` seeds
    //! the in-memory name/labels view from the persisted active-chain entries
    //! so `/session info` and RPC `session_info` reflect prior metadata
    //! without a full reconstruct pass.

    use opi_agent::session::{LabelAction, SessionEntry};

    let dir = tempfile::tempdir().unwrap();
    let mut coord = SessionCoordinator::new_for_test(
        dir.path(),
        "/test",
        opi_agent::compaction::CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();
    coord.append_session_info("seeded-name".into()).unwrap();
    coord
        .append_label("ready".into(), LabelAction::Add)
        .unwrap();
    coord.append_label("wip".into(), LabelAction::Add).unwrap();
    coord
        .append_label("ready".into(), LabelAction::Remove)
        .unwrap();

    let path = coord.session_path().to_path_buf();
    let session_id = coord.session_id().to_string();
    // Read back the entries the way the harness resume path would.
    let (_header, entries) = SessionReader::read_all(&path).unwrap();
    drop(coord);

    let reopened = SessionCoordinator::open_existing(
        path,
        session_id,
        &entries,
        0,
        opi_agent::compaction::CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    assert_eq!(reopened.name(), Some("seeded-name"));
    assert_eq!(reopened.labels(), &["wip".to_string()]);

    // The coordinator seed accepts rootless metadata (the pre-first-turn
    // case where `/name` is issued before any message). `reconstruct_context`
    // walks the parent_id chain and does not surface rootless metadata, so we
    // cross-check against a direct scan of the persisted entries instead.
    let session_info_on_disk = entries.iter().find_map(|e| match e {
        SessionEntry::SessionInfo(s) => Some(s.name.clone()),
        _ => None,
    });
    assert_eq!(session_info_on_disk, Some("seeded-name".to_string()));
}

#[test]
fn phase13_coordinator_open_existing_seeds_metadata_parented_to_active_metadata_entry() {
    use opi_agent::session::{LabelAction, LabelEntry, LeafEntry, SessionInfoEntry};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("metadata-through-metadata.jsonl");
    let header = SessionHeader::new_for_test(
        "metadata-through-metadata".into(),
        "2026-07-06T00:00:00Z".into(),
        "/test".into(),
        None,
    );
    let mut writer = SessionWriter::create(&path, header).unwrap();
    writer.append(&test_message_entry("m1", "root")).unwrap();
    writer
        .append(&SessionEntry::SessionInfo(SessionInfoEntry {
            id: "info-1".into(),
            parent_id: Some("m1".into()),
            timestamp: "2026-07-06T00:00:01Z".into(),
            name: "metadata-parent".into(),
        }))
        .unwrap();
    writer
        .append(&SessionEntry::Label(LabelEntry {
            id: "label-1".into(),
            parent_id: Some("info-1".into()),
            timestamp: "2026-07-06T00:00:02Z".into(),
            label: "carried".into(),
            action: LabelAction::Add,
        }))
        .unwrap();
    writer
        .append(&chained_message_entry(
            "m2",
            "info-1",
            "child through metadata parent",
        ))
        .unwrap();
    writer
        .append(&SessionEntry::Leaf(LeafEntry {
            id: "leaf-1".into(),
            parent_id: Some("m2".into()),
            timestamp: "2026-07-06T00:00:03Z".into(),
            entry_id: "m2".into(),
        }))
        .unwrap();
    drop(writer);

    let (_header, entries) = SessionReader::read_all(&path).unwrap();
    let message_count = opi_agent::session_context::reconstruct_context(
        &entries,
        &opi_agent::session::CrashRecovery::default(),
    )
    .messages
    .len();
    let reopened = SessionCoordinator::open_existing(
        path,
        "metadata-through-metadata".into(),
        &entries,
        message_count,
        opi_agent::compaction::CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    assert_eq!(reopened.name(), Some("metadata-parent"));
    assert_eq!(
        reopened.labels(),
        &["carried".to_string()],
        "coordinator seed should include metadata whose parent is active metadata"
    );
}

#[test]
fn session_coordinator_redacts_tool_result_details_but_keeps_content() {
    let dir = tempfile::tempdir().unwrap();
    let mut coord = SessionCoordinator::new_for_test(
        dir.path(),
        "/test",
        opi_agent::compaction::CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    let messages = vec![AgentMessage::Llm(Message::ToolResult(ToolResultMessage {
        tool_call_id: "tc-redact".into(),
        tool_name: "bash".into(),
        content: vec![OutputContent::Text {
            text: "VISIBLE_TOOL_OUTPUT".into(),
        }],
        details: Some(serde_json::json!({
            "command": "echo OPI_SESSION_COMMAND_SECRET_CANARY",
            "cwd": "C:\\Users\\private\\repo",
            "exit_code": 1,
        })),
        is_error: true,
        truncated: false,
        timestamp_ms: 0,
    }))];

    coord
        .on_turn_end(&messages, &opi_ai::stream::Usage::default(), 0)
        .unwrap();

    let (_header, entries) = SessionReader::read_all(coord.session_path()).unwrap();
    let tool_result = entries
        .iter()
        .find_map(|entry| match entry {
            SessionEntry::Message(MessageEntry {
                message: Message::ToolResult(tool_result),
                ..
            }) => Some(tool_result),
            _ => None,
        })
        .expect("persisted tool result");

    assert_eq!(
        tool_result.content,
        vec![OutputContent::Text {
            text: "VISIBLE_TOOL_OUTPUT".into()
        }]
    );
    let details = tool_result.details.as_ref().expect("redacted details");
    assert_eq!(details["command"], "[REDACTED]");
    assert_eq!(details["cwd"], "[REDACTED]");
    assert!(
        !serde_json::to_string(&entries)
            .unwrap()
            .contains("OPI_SESSION_COMMAND_SECRET_CANARY")
    );
}

#[test]
fn session_coordinator_redacts_assistant_tool_call_arguments() {
    let dir = tempfile::tempdir().unwrap();
    let mut coord = SessionCoordinator::new_for_test(
        dir.path(),
        "/test",
        opi_agent::compaction::CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    let mut assistant = test_support::base_assistant();
    assistant.content.push(AssistantContent::ToolCall {
        tool_call: ToolCall {
            id: "tc-redact-args".into(),
            name: "bash".into(),
            arguments:
                r#"{"command":"echo OPI_SESSION_TOOL_ARGS_SECRET_CANARY","cwd":"C:\\Users\\private"}"#
                    .into(),
        },
    });

    let messages = vec![AgentMessage::Llm(Message::Assistant(assistant))];
    coord
        .on_turn_end(&messages, &opi_ai::stream::Usage::default(), 0)
        .unwrap();

    let (_header, entries) = SessionReader::read_all(coord.session_path()).unwrap();
    let rendered = serde_json::to_string(&entries).unwrap();
    assert!(
        !rendered.contains("OPI_SESSION_TOOL_ARGS_SECRET_CANARY"),
        "{rendered}"
    );
    assert!(rendered.contains("[REDACTED]"), "{rendered}");
}

#[test]
fn session_coordinator_accumulates_usage() {
    let dir = tempfile::tempdir().unwrap();
    let mut coord = SessionCoordinator::new_for_test(
        dir.path(),
        "/test",
        opi_agent::compaction::CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    let usage = opi_ai::stream::Usage::reported(100, 50, 0, 0, None, None);

    coord.on_turn_end_simple(&[], &usage).unwrap();
    assert_eq!(coord.usage().turn_count(), 1);
    assert_eq!(coord.usage().total_input_tokens(), 100);
    assert_eq!(coord.usage().total_output_tokens(), 50);

    coord.on_turn_end_simple(&[], &usage).unwrap();
    assert_eq!(coord.usage().turn_count(), 2);
    assert_eq!(coord.usage().total_input_tokens(), 200);
}

// ---------------------------------------------------------------------------
// Harness session wiring tests
// ---------------------------------------------------------------------------
// These tests mutate OPI_SESSIONS_DIR so they must run sequentially.

static SESSION_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Acquire the session-test lock, recovering from any prior poisoning.
fn session_lock() -> std::sync::MutexGuard<'static, ()> {
    match SESSION_TEST_LOCK.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn harness_creates_session_file_on_prompt() {
    let _lock = session_lock();
    let dir = tempfile::tempdir().unwrap();
    set_sessions_dir(dir.path());

    let response = test_support::text_response("Hello!");
    let provider = MockProvider::new("mock", vec![response]);

    let mut harness = CodingHarness::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result = harness.prompt("Hi").await.unwrap();
    assert!(!result.is_empty(), "should have messages");

    // Verify session was created
    let session = harness.session();
    assert!(session.is_some(), "harness should have an active session");

    // Verify JSONL file was written
    let jsonl_files: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
        .collect();
    assert_eq!(jsonl_files.len(), 1, "should create one session file");

    let (header, entries) = SessionReader::read_all(&jsonl_files[0].path()).unwrap();
    assert_eq!(header.type_, "session");
    assert!(!entries.is_empty(), "session should have persisted entries");

    clear_sessions_dir();
}

// ---------------------------------------------------------------------------
// Reconstruct context tests
// ---------------------------------------------------------------------------

#[test]
fn reconstruct_context_from_session_entries() {
    let dir = tempfile::tempdir().unwrap();
    let header = make_header("test-sess", "/repo");
    let path = dir.path().join("test-sess.jsonl");
    let mut writer = SessionWriter::create(&path, header).unwrap();

    writer.append(&test_message_entry("e1", "Hello")).unwrap();
    writer
        .append(&chained_message_entry("e2", "e1", "World"))
        .unwrap();

    let (_header, entries) = SessionReader::read_all(&path).unwrap();
    let messages = opi_coding_agent::session_cli::reconstruct_context(&entries);

    assert_eq!(messages.len(), 2);
    for msg in &messages {
        assert!(
            matches!(msg, AgentMessage::Llm(Message::User(_))),
            "should reconstruct LLM messages"
        );
    }
}

#[test]
fn reconstruct_context_includes_compaction_summaries() {
    use opi_agent::message::AgentMessage;
    use opi_agent::session::CompactionEntry;

    let dir = tempfile::tempdir().unwrap();
    let header = make_header("test-sess", "/repo");
    let path = dir.path().join("test-sess.jsonl");
    let mut writer = SessionWriter::create(&path, header).unwrap();

    // first_kept_entry_id points at e2 — e1 must be dropped, e2 kept.
    writer.append(&test_message_entry("e1", "Hello")).unwrap();
    writer
        .append(&chained_message_entry("e2", "e1", "World"))
        .unwrap();
    writer
        .append(&SessionEntry::Compaction(CompactionEntry {
            id: "c1".into(),
            parent_id: Some("e2".into()),
            timestamp: "2026-05-22T12:00:02Z".into(),
            summary: "compacted".into(),
            first_kept_entry_id: "e2".into(),
            tokens_before: 100,
            tokens_after: 50,
        }))
        .unwrap();
    writer
        .append(&chained_message_entry("e3", "c1", "Follow up"))
        .unwrap();

    let (_header, entries) = SessionReader::read_all(&path).unwrap();
    let messages = opi_coding_agent::session_cli::reconstruct_context(&entries);

    // Expected: [summary(c1), e2, e3]. The kept tail (e2) was already in
    // JSONL before the Compaction entry — runtime never re-emits kept
    // entries after the marker.
    assert_eq!(
        messages.len(),
        3,
        "compaction preserves kept tail from before the marker plus post-compaction entries"
    );
    assert!(
        matches!(&messages[0], AgentMessage::CompactionSummary(cs) if cs.summary == "compacted")
    );
    let text_at = |idx: usize| -> String {
        match &messages[idx] {
            AgentMessage::Llm(Message::User(u)) => match &u.content[0] {
                InputContent::Text { text } => text.clone(),
                _ => panic!("expected text content"),
            },
            _ => panic!("expected user llm message at {idx}"),
        }
    };
    assert_eq!(text_at(1), "World", "kept-tail entry e2 must survive");
    assert_eq!(
        text_at(2),
        "Follow up",
        "post-compaction entry e3 must survive"
    );
}

#[test]
fn reconstruct_context_missing_first_kept_id_emits_summary_and_keeps_prior() {
    // Defensive: a corrupt/forward-incompatible session whose first_kept_entry_id
    // does not match any chain entry must not crash. The opi-agent context
    // builder (Phase 13.2) documents this case: no truncation is performed,
    // but the summary is still emitted at the compaction entry's position,
    // preserving the prior chain rather than dropping it.
    use opi_agent::message::AgentMessage;
    use opi_agent::session::CompactionEntry;

    let dir = tempfile::tempdir().unwrap();
    let header = make_header("test-sess", "/repo");
    let path = dir.path().join("test-sess.jsonl");
    let mut writer = SessionWriter::create(&path, header).unwrap();

    writer.append(&test_message_entry("e1", "Hello")).unwrap();
    writer
        .append(&SessionEntry::Compaction(CompactionEntry {
            id: "c1".into(),
            parent_id: Some("e1".into()),
            timestamp: "2026-05-22T12:00:02Z".into(),
            summary: "compacted".into(),
            first_kept_entry_id: "missing".into(),
            tokens_before: 100,
            tokens_after: 50,
        }))
        .unwrap();
    writer
        .append(&chained_message_entry("e2", "c1", "Post"))
        .unwrap();

    let (_header, entries) = SessionReader::read_all(&path).unwrap();
    let messages = opi_coding_agent::session_cli::reconstruct_context(&entries);

    // [summary(c1), e1, e2]: summary at front, prior chain preserved.
    assert_eq!(messages.len(), 3);
    assert!(matches!(&messages[0], AgentMessage::CompactionSummary(_)));
    assert!(matches!(&messages[1], AgentMessage::Llm(_)));
    assert!(matches!(&messages[2], AgentMessage::Llm(_)));
}

// ---------------------------------------------------------------------------
// Full lifecycle test: write, read, verify
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn full_lifecycle_write_read_verify() {
    let _lock = session_lock();
    let dir = tempfile::tempdir().unwrap();
    set_sessions_dir(dir.path());

    // Create harness and run a prompt
    let response = test_support::text_response("I am an assistant");
    let provider = MockProvider::new("mock", vec![response]);

    let mut harness = CodingHarness::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let _result = harness.prompt("Who are you?").await.unwrap();

    // Get the session ID
    let session_id = harness.session().unwrap().session_id().to_owned();

    // Verify the session file can be read back
    let session_path = dir.path().join(format!("{session_id}.jsonl"));
    assert!(session_path.exists(), "session file should exist");

    let (header, entries) = SessionReader::read_all(&session_path).unwrap();
    assert_eq!(header.id, session_id);
    assert!(!entries.is_empty(), "should have persisted entries");

    // Verify reconstruct produces the right number of messages
    let reconstructed = opi_coding_agent::session_cli::reconstruct_context(&entries);
    assert!(
        !reconstructed.is_empty(),
        "reconstructed messages should not be empty"
    );

    clear_sessions_dir();
}

// ---------------------------------------------------------------------------
// Multi-turn session persistence test
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn multi_turn_session_persistence() {
    let _lock = session_lock();
    let dir = tempfile::tempdir().unwrap();
    set_sessions_dir(dir.path());

    let first = test_support::text_response("First response");
    let second = test_support::text_response("Second response");
    let provider = MockProvider::new("mock", vec![first, second]);

    let mut harness = CodingHarness::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let result1 = harness.prompt("Hello").await.unwrap();
    let result2 = harness.continue_("Tell me more").await.unwrap();

    assert!(result1.len() >= 2, "first turn should have messages");
    assert!(result2.len() >= 4, "second turn should have more messages");

    // Check usage accumulation
    let session = harness.session().unwrap();
    assert_eq!(session.usage().turn_count(), 2, "should track 2 turns");

    // Read back session file
    let session_id = session.session_id().to_owned();
    let session_path = dir.path().join(format!("{session_id}.jsonl"));
    let (_header, entries) = SessionReader::read_all(&session_path).unwrap();

    // Should have entries from both turns
    assert!(
        entries.len() >= 4,
        "should have entries from both turns, got {}",
        entries.len()
    );

    clear_sessions_dir();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn harness_forks_current_session_into_new_parented_session() {
    let _lock = session_lock();
    let dir = tempfile::tempdir().unwrap();
    set_sessions_dir(dir.path());

    let response = test_support::text_response("Fork me");
    let provider = MockProvider::new("mock", vec![response]);

    let mut harness = CodingHarness::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let _ = harness.prompt("Hello").await.unwrap();
    let source_session_id = harness.session().unwrap().session_id().to_owned();
    let source_path = dir.path().join(format!("{source_session_id}.jsonl"));
    let (_, source_entries) = SessionReader::read_all(&source_path).unwrap();

    let (forked_session_id, message_count) = harness.fork_current_session().unwrap();
    let source_content_entries = message_entry_count(&source_entries);

    assert_ne!(forked_session_id, source_session_id);
    assert_eq!(harness.session().unwrap().session_id(), forked_session_id);
    assert_eq!(message_count, source_content_entries);

    let forked_path = dir.path().join(format!("{forked_session_id}.jsonl"));
    let (forked_header, forked_entries) = SessionReader::read_all(&forked_path).unwrap();
    assert_eq!(
        forked_header.parent_session.as_deref(),
        Some(source_session_id.as_str())
    );
    // Phase 13.3: fork copies the active-chain content AND appends a fresh
    // Leaf at the tip, so the forked entry count is source content entries + 1
    // (the new Leaf). Compare message-entry counts for the content invariant.
    assert_eq!(
        message_entry_count(&forked_entries),
        source_content_entries,
        "fork preserves the active-chain content"
    );
    assert!(
        forked_entries
            .iter()
            .any(|e| matches!(e, SessionEntry::Leaf(_))),
        "fork writes a fresh Leaf at the active tip"
    );
    assert!(
        source_path.exists(),
        "source session must remain append-only"
    );

    clear_sessions_dir();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn phase14_session_affinity_tracks_new_resume_and_fork() {
    let _lock = session_lock();
    let workspace = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());
    let provider = MockProvider::new(
        "mock",
        vec![
            test_support::text_response("new"),
            test_support::text_response("resumed"),
            test_support::text_response("forked"),
        ],
    );
    let call_log = provider.call_log_handle();
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .build();

    let source_session_id = harness.session().unwrap().session_id().to_owned();
    harness.prompt("new session").await.unwrap();
    harness.resume_session_id(&source_session_id).unwrap();
    harness.prompt("resumed session").await.unwrap();
    let (forked_session_id, _) = harness.fork_current_session().unwrap();
    harness.prompt("forked session").await.unwrap();

    let calls = call_log.lock().unwrap();
    assert_eq!(calls.len(), 3);
    assert_eq!(
        calls[0].session_id.as_deref(),
        Some(source_session_id.as_str())
    );
    assert_eq!(
        calls[1].session_id.as_deref(),
        Some(source_session_id.as_str())
    );
    assert_eq!(
        calls[2].session_id.as_deref(),
        Some(forked_session_id.as_str())
    );
    drop(calls);
    clear_sessions_dir();
}

// ---------------------------------------------------------------------------
// Compaction tests
// ---------------------------------------------------------------------------

#[test]
fn compaction_shrinks_buffer_and_returns_summary_plus_kept() {
    use opi_agent::compaction::CompactionConfig;
    use opi_ai::message::AssistantContent;
    use opi_ai::stream::Usage;

    let dir = tempfile::tempdir().unwrap();
    let mut coord = SessionCoordinator::new_for_test(
        dir.path(),
        "/test",
        // Tiny threshold so compaction triggers immediately.
        CompactionConfig {
            enabled: true,
            threshold_tokens: 1,
        },
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    let user = |t: &str| {
        AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text { text: t.into() }],
            timestamp_ms: 0,
        }))
    };
    let assistant = |t: &str| {
        let mut a = test_support::base_assistant();
        a.content.push(AssistantContent::Text { text: t.into() });
        AgentMessage::Llm(Message::Assistant(a))
    };

    let messages: Vec<AgentMessage> = (0..8)
        .flat_map(|i| {
            vec![
                user(&format!("user message number {i} with extra padding text")),
                assistant(&format!(
                    "assistant response number {i} with extra padding text"
                )),
            ]
        })
        .collect();

    let usage = Usage::reported(100, 100, 0, 0, None, None);

    let out = coord
        .on_turn_end_simple(&messages, &usage)
        .unwrap()
        .expect("compaction should trigger above threshold");

    // After compaction the new buffer is [summary, ...kept_messages].
    assert!(matches!(
        &out.new_agent_messages[0],
        AgentMessage::CompactionSummary(_)
    ));
    assert!(
        out.new_agent_messages.len() < messages.len(),
        "compacted buffer must be smaller than input ({} vs {})",
        out.new_agent_messages.len(),
        messages.len()
    );
    // tokens_after should not exceed tokens_before — equality only happens
    // when nothing fit on the kept side (unusual for our 25% split).
    assert!(out.tokens_after <= out.tokens_before);
}

#[test]
fn compaction_engine_reads_pricing_and_reports_cost() {
    use opi_agent::compaction::CompactionConfig;
    use opi_ai::stream::Usage;

    let dir = tempfile::tempdir().unwrap();
    let mut coord = SessionCoordinator::new_for_test(
        dir.path(),
        "/test",
        CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    // Sonnet pricing: $3/Mtok input, $15/Mtok output
    let usage = Usage::reported(1_000_000, 500_000, 0, 0, None, None);
    coord.on_turn_end_simple(&[], &usage).unwrap();

    let cost = coord.cost_summary().expect("sonnet pricing should resolve");
    assert!((cost.input_cost - 3.0).abs() < 1e-6);
    assert!((cost.output_cost - 7.5).abs() < 1e-6);
    assert!((cost.total_cost() - 10.5).abs() < 1e-6);
}

#[test]
fn cost_summary_returns_none_for_unknown_model() {
    use opi_agent::compaction::CompactionConfig;

    let dir = tempfile::tempdir().unwrap();
    let coord = SessionCoordinator::new_for_test(
        dir.path(),
        "/test",
        CompactionConfig::default(),
        "future:unknown-model",
    )
    .unwrap();

    assert!(coord.cost_summary().is_none());
}

#[test]
fn cost_summary_returns_none_when_any_turn_has_unknown_usage() {
    use opi_agent::compaction::CompactionConfig;
    use opi_ai::stream::Usage;

    let dir = tempfile::tempdir().unwrap();
    let mut coord = SessionCoordinator::new_for_test(
        dir.path(),
        "/test",
        CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    coord
        .on_turn_end_simple(&[], &Usage::reported(100, 50, 0, 0, None, None))
        .unwrap();
    coord.on_turn_end_simple(&[], &Usage::unknown()).unwrap();

    assert!(coord.cost_summary().is_none());
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn harness_unprefixed_embedded_pricing_survives_switch_resume_and_fork() {
    use opi_ai::message::AssistantContent;
    use opi_ai::model_info::{ModelInfo, ModelPricing, WireApi};
    use opi_ai::stream::{Pricing, StopReason, Usage};

    fn priced_model(id: &str, input: f64, output: f64) -> ModelInfo {
        ModelInfo::new(
            id,
            id,
            WireApi::AnthropicMessages,
            ModelCapabilities::new(200_000, 8_192).with_streaming(true),
        )
        .with_pricing(
            ModelPricing::try_new(
                Pricing {
                    input_cost_per_mtok: input,
                    output_cost_per_mtok: output,
                    cache_read_cost_per_mtok: 0.0,
                    cache_write_cost_per_mtok: 0.0,
                },
                vec![],
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn assert_total_cost(harness: &CodingHarness, expected: f64, lifecycle: &str) {
        let cost = harness
            .session()
            .and_then(|session| session.cost_summary())
            .unwrap_or_else(|| panic!("{lifecycle}: embedded pricing was cleared"));
        assert!(
            (cost.total_cost() - expected).abs() < 1e-6,
            "{lifecycle}: expected {expected}, got {}",
            cost.total_cost()
        );
    }

    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());

    let mut assistant = test_support::base_assistant();
    assistant.content.push(AssistantContent::Text {
        text: "priced".into(),
    });
    assistant.usage = Usage::reported(1_000_000, 1_000_000, 0, 0, None, None);
    let response = vec![
        AssistantStreamEvent::Start {
            partial: test_support::base_assistant(),
        },
        AssistantStreamEvent::TextDelta {
            content_index: 0,
            delta: "priced".into(),
            partial: assistant.clone(),
        },
        AssistantStreamEvent::Done {
            reason: StopReason::Stop,
            message: assistant,
        },
    ];
    let provider = MockProvider::new_with_models(
        "anthropic",
        vec![
            priced_model("claude-sonnet-4", 7.0, 11.0),
            priced_model("claude-opus-4", 19.0, 23.0),
        ],
        vec![response],
    );
    let mut harness = CodingHarness::new(
        Box::new(provider),
        "claude-sonnet-4".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    assert_total_cost(&harness, 0.0, "new construction");
    harness.prompt("record priced usage").await.unwrap();
    assert_total_cost(&harness, 18.0, "new session usage");
    let source_session_id = harness.session().unwrap().session_id().to_owned();

    harness.set_model("claude-opus-4".into());
    assert_eq!(harness.model(), "claude-opus-4");
    assert_total_cost(&harness, 42.0, "unprefixed set_model");

    assert_eq!(
        harness
            .set_model_validated("claude-sonnet-4".into())
            .unwrap(),
        "anthropic:claude-sonnet-4"
    );
    assert_total_cost(&harness, 18.0, "unprefixed set_model_validated");
    let unknown = harness
        .set_model_validated("unknown-model".into())
        .unwrap_err();
    // A bare model resolves only through the unique-dispatchable-route proof:
    // no match is typed remediation, not a spec parse error.
    assert!(
        unknown.contains("bare model") && unknown.contains("matches no dispatchable route"),
        "{unknown}"
    );
    // A cross-provider switch to an UNREGISTERED provider still fails, now at
    // route resolution (unknown model/provider) rather than "cannot switch".
    let other_provider = harness
        .set_model_validated("openai:gpt-4o".into())
        .unwrap_err();
    assert!(other_provider.contains("unknown model"), "{other_provider}");
    assert_eq!(harness.model(), "claude-sonnet-4");
    assert_total_cost(&harness, 18.0, "rejected model changes");

    harness.resume_session_id(&source_session_id).unwrap();
    assert_eq!(harness.model(), "claude-sonnet-4");
    assert_total_cost(&harness, 18.0, "resume");

    harness.fork_current_session().unwrap();
    assert_eq!(harness.model(), "claude-sonnet-4");
    assert_total_cost(&harness, 18.0, "fork");

    clear_sessions_dir();
}

#[test]
fn embedded_model_pricing_overrides_legacy_fallback() {
    use opi_agent::compaction::CompactionConfig;
    use opi_ai::model_info::{ModelPricing, PricingTier};
    use opi_ai::stream::{Pricing, Usage};

    let dir = tempfile::tempdir().unwrap();
    let mut coord = SessionCoordinator::new_for_test(
        dir.path(),
        "/test",
        CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();
    coord.set_cost_model(
        "anthropic:claude-sonnet-4",
        Some(
            ModelPricing::try_new(
                Pricing {
                    input_cost_per_mtok: 1.0,
                    output_cost_per_mtok: 2.0,
                    cache_read_cost_per_mtok: 0.1,
                    cache_write_cost_per_mtok: 1.25,
                },
                vec![PricingTier {
                    input_tokens_above: 2_000_000,
                    pricing: Pricing {
                        input_cost_per_mtok: 2.0,
                        output_cost_per_mtok: 4.0,
                        cache_read_cost_per_mtok: 0.2,
                        cache_write_cost_per_mtok: 2.5,
                    },
                }],
            )
            .unwrap(),
        ),
    );
    coord
        .on_turn_end_simple(&[], &Usage::reported(1_000_000, 500_000, 0, 0, None, None))
        .unwrap();

    let cost = coord.cost_summary().unwrap();
    assert!((cost.input_cost - 1.0).abs() < 1e-6);
    assert!((cost.output_cost - 1.0).abs() < 1e-6);
}

#[test]
fn embedded_model_pricing_updates_on_model_switch_and_resume() {
    use opi_agent::compaction::CompactionConfig;
    use opi_ai::model_info::ModelPricing;
    use opi_ai::stream::{Pricing, Usage};

    let dir = tempfile::tempdir().unwrap();
    let mut coord = SessionCoordinator::new_for_test(
        dir.path(),
        "/test",
        CompactionConfig::default(),
        "future:first",
    )
    .unwrap();
    let embedded = ModelPricing::try_new(
        Pricing {
            input_cost_per_mtok: 7.0,
            output_cost_per_mtok: 11.0,
            cache_read_cost_per_mtok: 0.0,
            cache_write_cost_per_mtok: 0.0,
        },
        Vec::new(),
    )
    .unwrap();
    coord.set_cost_model("future:second", Some(embedded.clone()));
    let usage = Usage::reported(1_000_000, 1_000_000, 0, 0, None, None);
    let mut assistant = test_support::base_assistant();
    assistant.usage = usage.clone();
    coord
        .on_turn_end_simple(
            &[
                AgentMessage::Llm(Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "pricing".into(),
                    }],
                    timestamp_ms: 0,
                })),
                AgentMessage::Llm(Message::Assistant(assistant)),
            ],
            &usage,
        )
        .unwrap();
    let path = coord.session_path().to_path_buf();
    let session_id = coord.session_id().to_owned();
    drop(coord);
    let (_, entries) = SessionReader::read_all(&path).unwrap();

    let mut resumed = SessionCoordinator::open_existing(
        path,
        session_id,
        &entries,
        0,
        CompactionConfig::default(),
        "future:second",
    )
    .unwrap();
    resumed.set_cost_model("future:second", Some(embedded));
    let cost = resumed.cost_summary().unwrap();
    assert!((cost.total_cost() - 18.0).abs() < 1e-6);
}

#[test]
fn legacy_pricing_fallback_remains_for_unmigrated_model() {
    use opi_agent::compaction::CompactionConfig;
    use opi_ai::stream::Usage;

    let dir = tempfile::tempdir().unwrap();
    let mut coord = SessionCoordinator::new_for_test(
        dir.path(),
        "/test",
        CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();
    coord.set_cost_model("anthropic:claude-sonnet-4", None);
    coord
        .on_turn_end_simple(&[], &Usage::reported(1_000_000, 0, 0, 0, None, None))
        .unwrap();
    assert!((coord.cost_summary().unwrap().input_cost - 3.0).abs() < 1e-6);
}

#[test]
fn thinking_level_additions_round_trip_with_current_session_schema() {
    use opi_agent::session::FORMAT_VERSION;

    assert_eq!(FORMAT_VERSION, 2);
    for (level, expected) in [
        (ThinkingLevel::None, "\"none\""),
        (ThinkingLevel::Minimal, "\"minimal\""),
        (ThinkingLevel::XHigh, "\"xhigh\""),
        (ThinkingLevel::Max, "\"max\""),
    ] {
        let encoded = serde_json::to_string(&level).unwrap();
        assert_eq!(encoded, expected);
        assert_eq!(
            serde_json::from_str::<ThinkingLevel>(&encoded).unwrap(),
            level
        );
    }
}

// ---------------------------------------------------------------------------
// Resume: open_existing reuses the original session file
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Compaction events flow through harness subscribers
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn harness_emits_compaction_events_on_threshold() {
    use std::sync::Arc;

    let _lock = session_lock();
    let dir = tempfile::tempdir().unwrap();
    set_sessions_dir(dir.path());

    // Use threshold=0 so compaction triggers regardless of MockProvider usage
    // (which is zero by default).
    let mut config = OpiConfig::default();
    config.compaction.threshold_tokens = 0;

    let response = test_support::text_response("ok");
    let provider = MockProvider::new("mock", vec![response]);
    let mut harness = CodingHarness::new(
        Box::new(provider),
        "mock-model".into(),
        config,
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let starts = Arc::new(std::sync::Mutex::new(0u32));
    let ends = Arc::new(std::sync::Mutex::new(0u32));
    let starts_c = starts.clone();
    let ends_c = ends.clone();
    harness.subscribe(Box::new(move |event| match event {
        opi_agent::event::AgentEvent::CompactionStart { .. } => {
            *starts_c.lock().unwrap() += 1;
        }
        opi_agent::event::AgentEvent::CompactionEnd { .. } => {
            *ends_c.lock().unwrap() += 1;
        }
        _ => {}
    }));

    // Need at least two messages for compaction to be possible.
    harness.prompt("first prompt").await.unwrap();

    let s = *starts.lock().unwrap();
    let e = *ends.lock().unwrap();
    assert_eq!(s, e, "every CompactionStart should have a matching End");
    assert!(s >= 1, "expected at least one CompactionStart");

    clear_sessions_dir();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn threshold_compaction_is_counted_in_diagnostics() {
    let _lock = session_lock();
    let dir = tempfile::tempdir().unwrap();
    set_sessions_dir(dir.path());

    let mut config = OpiConfig::default();
    config.compaction.threshold_tokens = 0;

    let provider = MockProvider::new("mock", vec![test_support::text_response("ok")]);
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock-model".into(),
        config,
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .record_diagnostics(true)
    .build();

    harness.prompt("first prompt").await.unwrap();

    let counts = harness
        .diagnostic_counts()
        .expect("record_diagnostics installs a sink");
    assert!(
        counts.info >= 1,
        "successful compaction should be counted as an info diagnostic: {counts:?}"
    );

    clear_sessions_dir();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn manual_compaction_is_counted_in_diagnostics() {
    let _lock = session_lock();
    let dir = tempfile::tempdir().unwrap();
    set_sessions_dir(dir.path());

    let mut config = OpiConfig::default();
    config.compaction.threshold_tokens = u64::MAX;

    let provider = MockProvider::new("mock", vec![test_support::text_response("ok")]);
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock-model".into(),
        config,
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .record_diagnostics(true)
    .build();

    harness.prompt("first prompt").await.unwrap();
    let result = harness
        .compact(opi_agent::session_event::CompactionReason::Manual)
        .unwrap();
    assert!(result.is_some(), "manual compaction should produce output");

    let counts = harness
        .diagnostic_counts()
        .expect("record_diagnostics installs a sink");
    assert!(
        counts.info >= 1,
        "manual compaction should be counted as an info diagnostic: {counts:?}"
    );

    clear_sessions_dir();
}

#[test]
fn open_existing_appends_to_original_file() {
    use opi_agent::compaction::CompactionConfig;
    use opi_ai::stream::Usage;

    let dir = tempfile::tempdir().unwrap();

    // Create initial session and persist a turn.
    let (session_path, session_id) = {
        let mut coord = SessionCoordinator::new_for_test(
            dir.path(),
            "/repo",
            CompactionConfig::default(),
            "anthropic:claude-sonnet-4",
        )
        .unwrap();
        let path = coord.session_path().to_path_buf();
        let id = coord.session_id().to_owned();

        let msg = AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "first".into(),
            }],
            timestamp_ms: 0,
        }));
        coord.on_turn_end_simple(&[msg], &Usage::default()).unwrap();
        (path, id)
    };

    let (header_before, entries_before) =
        opi_agent::session::SessionReader::read_all(&session_path).unwrap();
    assert_eq!(message_entry_count(&entries_before), 1);
    assert!(
        matches!(entries_before.last(), Some(SessionEntry::Leaf(_))),
        "first turn should persist a leaf pointer"
    );

    // Open the same file and append another turn.
    let mut resumed = SessionCoordinator::open_existing(
        session_path.clone(),
        session_id.clone(),
        &entries_before,
        1, // one prior agent message
        CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    let msg = AgentMessage::Llm(Message::User(UserMessage {
        content: vec![InputContent::Text {
            text: "second".into(),
        }],
        timestamp_ms: 0,
    }));
    resumed
        .on_turn_end_simple(&[msg], &Usage::default())
        .unwrap();

    let (header_after, entries_after) =
        opi_agent::session::SessionReader::read_all(&session_path).unwrap();
    assert_eq!(header_after.id, header_before.id);
    assert_eq!(
        message_entry_count(&entries_after),
        2,
        "resumed session should grow by message entries, not start over"
    );
    assert!(
        matches!(entries_after.last(), Some(SessionEntry::Leaf(_))),
        "resumed turn should update the active branch leaf"
    );
}

#[test]
fn open_existing_preserves_kept_tail_after_compaction() {
    // The runtime writes the Compaction entry AFTER the kept-tail messages
    // were already persisted in earlier turns. Resuming must keep those tail
    // entries in the compaction buffer; otherwise future compactions / token
    // accounting see an empty post-summary buffer.
    use opi_agent::compaction::CompactionConfig;
    use opi_agent::session::{CompactionEntry, MessageEntry};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("resume.jsonl");
    let header = make_header("resume", "/repo");
    let mut writer = SessionWriter::create(&path, header).unwrap();

    // Two pre-compaction entries.
    writer
        .append(&SessionEntry::Message(MessageEntry {
            id: "msg-1".into(),
            parent_id: None,
            timestamp: "2026-05-22T12:00:00Z".into(),
            message: Message::User(UserMessage {
                content: vec![InputContent::Text { text: "old".into() }],
                timestamp_ms: 0,
            }),
        }))
        .unwrap();
    writer
        .append(&SessionEntry::Message(MessageEntry {
            id: "msg-2".into(),
            parent_id: None,
            timestamp: "2026-05-22T12:00:01Z".into(),
            message: Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "kept-tail".into(),
                }],
                timestamp_ms: 0,
            }),
        }))
        .unwrap();
    // Compaction: keep msg-2 as the first-kept entry. msg-1 is dropped.
    writer
        .append(&SessionEntry::Compaction(CompactionEntry {
            id: "cmp-1".into(),
            parent_id: None,
            timestamp: "2026-05-22T12:00:02Z".into(),
            summary: "old context summary".into(),
            first_kept_entry_id: "msg-2".into(),
            tokens_before: 100,
            tokens_after: 30,
        }))
        .unwrap();
    drop(writer);

    let (_header, entries) = SessionReader::read_all(&path).unwrap();
    let resumed = SessionCoordinator::open_existing(
        path.clone(),
        "resume".into(),
        &entries,
        2, // post-resume agent buffer has [summary, msg-2]
        CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    let kept = resumed.compaction_entries();
    assert_eq!(
        kept.len(),
        2,
        "entries should be [summary, msg-2] after resume with Compaction; got {} entries",
        kept.len(),
    );
    // First entry is the compaction summary.
    assert!(matches!(
        &kept[0].message,
        AgentMessage::CompactionSummary(_)
    ));
    // Second entry is the kept tail.
    assert_eq!(kept[1].id, "msg-2");
}

#[test]
fn resume_session_id_surfaces_recovery_diagnostics_in_resource_metadata() {
    let _lock = session_lock();
    let dir = tempfile::tempdir().unwrap();
    set_sessions_dir(dir.path());

    let path = dir.path().join("truncated-harness-resume.jsonl");
    let header = make_header("truncated-harness-resume", "/repo");
    let mut writer = SessionWriter::create(&path, header).unwrap();
    writer
        .append(&test_message_entry("msg-1", "hello"))
        .unwrap();
    drop(writer);

    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    write!(file, "{{\"type\":\"message\"").unwrap();

    let provider = MockProvider::new("mock", Vec::new());
    let mut harness = CodingHarness::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let message_count = harness
        .resume_session_id("truncated-harness-resume")
        .expect("resume should recover preceding entries");

    assert_eq!(message_count, 1);
    assert!(
        harness
            .resource_metadata()
            .diagnostics
            .iter()
            .any(|d| { d.code == code::CODE_SESSION_TRUNCATED_LINE && d.source == SOURCE_SESSION }),
        "interactive resume should retain recovery diagnostics in resource metadata: {:?}",
        harness.resource_metadata().diagnostics
    );

    clear_sessions_dir();
}

// ---------------------------------------------------------------------------
// Multi-assistant turn usage aggregation
// ---------------------------------------------------------------------------
//
// A single user prompt that triggers a tool call produces two assistant
// messages in one turn: the tool-call response, then the final response.
// Both carry their own `usage`. The cumulative session usage must include
// BOTH — not just the last one.

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn multi_assistant_turn_accumulates_all_assistant_usages() {
    use opi_ai::message::{AssistantContent, ToolCall};
    use opi_ai::stream::{AssistantStreamEvent, StopReason, Usage};

    let _lock = session_lock();
    let dir = tempfile::tempdir().unwrap();
    set_sessions_dir(dir.path());

    // Tool-call assistant: 100 in / 30 out.
    let tool_call = ToolCall {
        id: "tc-1".into(),
        name: "noop".into(),
        arguments: "{}".into(),
    };
    let mut tool_partial = test_support::base_assistant();
    tool_partial.content.push(AssistantContent::ToolCall {
        tool_call: tool_call.clone(),
    });
    tool_partial.usage = Usage::reported(100, 30, 0, 0, None, None);
    let tool_response = vec![
        AssistantStreamEvent::Start {
            partial: test_support::base_assistant(),
        },
        AssistantStreamEvent::ToolCallEnd {
            content_index: 0,
            tool_call,
            partial: tool_partial.clone(),
        },
        AssistantStreamEvent::Done {
            reason: StopReason::ToolUse,
            message: tool_partial,
        },
    ];

    // Final text assistant: 200 in / 50 out.
    let mut final_partial = test_support::base_assistant();
    final_partial.content.push(AssistantContent::Text {
        text: "done".into(),
    });
    final_partial.usage = Usage::reported(200, 50, 0, 0, None, None);
    let final_response = vec![
        AssistantStreamEvent::Start {
            partial: test_support::base_assistant(),
        },
        AssistantStreamEvent::TextDelta {
            content_index: 0,
            delta: "done".into(),
            partial: final_partial.clone(),
        },
        AssistantStreamEvent::Done {
            reason: StopReason::Stop,
            message: final_partial,
        },
    ];

    let provider = MockProvider::new("mock", vec![tool_response, final_response]);

    // Phase 17.4: the harness no longer accepts injected custom tools (only
    // built-in registrations run). The scripted "noop" tool-call below is not a
    // registered tool, so it resolves to an unknown-tool error result and the
    // turn still completes within one persist_turn — which is exactly what this
    // turn_count / token-summing assertion exercises.

    let mut harness = CodingHarness::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );

    let _ = harness.prompt("Use the tool").await.unwrap();

    let session = harness.session().expect("session should be active");
    let usage = session.usage();

    // The two assistant messages were emitted in the SAME user prompt, so
    // turn_count should be 1 (one persist_turn call) while tokens are the
    // sum of both assistants.
    assert_eq!(usage.turn_count(), 1, "one prompt should be one turn");
    assert_eq!(
        usage.total_input_tokens(),
        300,
        "input tokens must sum tool-call (100) + final (200)"
    );
    assert_eq!(
        usage.total_output_tokens(),
        80,
        "output tokens must sum tool-call (30) + final (50)"
    );

    clear_sessions_dir();
}

// ---------------------------------------------------------------------------
// Active-branch resume via Leaf entries
// ---------------------------------------------------------------------------

#[test]
fn reconstruct_context_follows_last_leaf_to_active_branch() {
    use opi_agent::message::AgentMessage;
    use opi_agent::session::LeafEntry;

    // A branched session: root e1 has two children e2a (branch A) and
    // e2b (branch B). Branch A has follow-up e3a; branch B has none. The
    // active branch is A (Leaf -> e3a). File-order replay would include
    // both branches; the active-branch walk must return only e1 -> e2a -> e3a.
    let dir = tempfile::tempdir().unwrap();
    let header = make_header("branch-sess", "/repo");
    let path = dir.path().join("branch-sess.jsonl");
    let mut writer = SessionWriter::create(&path, header).unwrap();

    let user = |id: &str, parent: Option<&str>, text: &str| {
        SessionEntry::Message(MessageEntry {
            id: id.into(),
            parent_id: parent.map(|s| s.into()),
            timestamp: "2026-05-23T12:00:00Z".into(),
            message: Message::User(UserMessage {
                content: vec![InputContent::Text { text: text.into() }],
                timestamp_ms: 0,
            }),
        })
    };

    writer.append(&user("e1", None, "root")).unwrap();
    writer.append(&user("e2a", Some("e1"), "branch-a")).unwrap();
    writer.append(&user("e2b", Some("e1"), "branch-b")).unwrap();
    writer
        .append(&user("e3a", Some("e2a"), "follow-a"))
        .unwrap();
    writer
        .append(&SessionEntry::Leaf(LeafEntry {
            id: "l1".into(),
            parent_id: Some("e3a".into()),
            timestamp: "2026-05-23T12:00:01Z".into(),
            entry_id: "e3a".into(),
        }))
        .unwrap();
    drop(writer);

    let (_h, entries) = SessionReader::read_all(&path).unwrap();
    let messages = opi_coding_agent::session_cli::reconstruct_context(&entries);

    // Expect exactly the active branch: e1, e2a, e3a (the sibling e2b is
    // dropped).
    assert_eq!(
        messages.len(),
        3,
        "active-branch walk must drop sibling branch e2b; got {} messages",
        messages.len()
    );
    let text_at = |idx: usize| -> String {
        match &messages[idx] {
            AgentMessage::Llm(Message::User(u)) => match &u.content[0] {
                InputContent::Text { text } => text.clone(),
                _ => panic!("expected text content"),
            },
            _ => panic!("expected user llm message at {idx}"),
        }
    };
    assert_eq!(text_at(0), "root");
    assert_eq!(text_at(1), "branch-a");
    assert_eq!(text_at(2), "follow-a");
}

#[test]
fn reconstruct_context_uses_last_leaf_when_multiple_present() {
    use opi_agent::message::AgentMessage;
    use opi_agent::session::LeafEntry;

    // Two leaf entries written in order. The branch pointer is mutable, so
    // the last leaf wins — newer activity supersedes older.
    let dir = tempfile::tempdir().unwrap();
    let header = make_header("two-leaf-sess", "/repo");
    let path = dir.path().join("two-leaf-sess.jsonl");
    let mut writer = SessionWriter::create(&path, header).unwrap();

    let user = |id: &str, parent: Option<&str>, text: &str| {
        SessionEntry::Message(MessageEntry {
            id: id.into(),
            parent_id: parent.map(|s| s.into()),
            timestamp: "2026-05-23T12:00:00Z".into(),
            message: Message::User(UserMessage {
                content: vec![InputContent::Text { text: text.into() }],
                timestamp_ms: 0,
            }),
        })
    };

    writer.append(&user("e1", None, "root")).unwrap();
    writer.append(&user("e2a", Some("e1"), "branch-a")).unwrap();
    writer.append(&user("e2b", Some("e1"), "branch-b")).unwrap();
    // Older leaf points at branch A.
    writer
        .append(&SessionEntry::Leaf(LeafEntry {
            id: "l-old".into(),
            parent_id: Some("e2a".into()),
            timestamp: "2026-05-23T12:00:01Z".into(),
            entry_id: "e2a".into(),
        }))
        .unwrap();
    // User switched to branch B; newer leaf wins.
    writer
        .append(&SessionEntry::Leaf(LeafEntry {
            id: "l-new".into(),
            parent_id: Some("e2b".into()),
            timestamp: "2026-05-23T12:00:02Z".into(),
            entry_id: "e2b".into(),
        }))
        .unwrap();
    drop(writer);

    let (_h, entries) = SessionReader::read_all(&path).unwrap();
    let messages = opi_coding_agent::session_cli::reconstruct_context(&entries);

    assert_eq!(
        messages.len(),
        2,
        "must follow newest leaf (-> e2b), not older one"
    );
    if let AgentMessage::Llm(Message::User(u)) = &messages[1]
        && let InputContent::Text { text } = &u.content[0]
    {
        assert_eq!(text, "branch-b");
    } else {
        panic!("expected branch-b at index 1");
    }
}

#[test]
fn session_coordinator_append_leaf_switches_active_branch() {
    use opi_agent::message::AgentMessage;
    use opi_agent::session::LeafEntry;

    let dir = tempfile::tempdir().unwrap();
    let header = make_header("append-leaf-sess", "/repo");
    let path = dir.path().join("append-leaf-sess.jsonl");
    let mut writer = SessionWriter::create(&path, header).unwrap();

    let user = |id: &str, parent: Option<&str>, text: &str| {
        SessionEntry::Message(MessageEntry {
            id: id.into(),
            parent_id: parent.map(|s| s.into()),
            timestamp: "2026-05-23T12:00:00Z".into(),
            message: Message::User(UserMessage {
                content: vec![InputContent::Text { text: text.into() }],
                timestamp_ms: 0,
            }),
        })
    };

    writer.append(&user("e1", None, "root")).unwrap();
    writer.append(&user("e2a", Some("e1"), "branch-a")).unwrap();
    writer.append(&user("e2b", Some("e1"), "branch-b")).unwrap();
    writer
        .append(&SessionEntry::Leaf(LeafEntry {
            id: "l-old".into(),
            parent_id: None,
            timestamp: "2026-05-23T12:00:01Z".into(),
            entry_id: "e2a".into(),
        }))
        .unwrap();
    drop(writer);

    let (_header, entries) = SessionReader::read_all(&path).unwrap();
    let mut coord = SessionCoordinator::open_existing(
        path.clone(),
        "append-leaf-sess".into(),
        &entries,
        2,
        opi_agent::compaction::CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    coord.append_leaf("e2b").unwrap();
    let (_header, entries) = SessionReader::read_all(&path).unwrap();
    let messages = opi_coding_agent::session_cli::reconstruct_context(&entries);

    assert_eq!(messages.len(), 2);
    if let AgentMessage::Llm(Message::User(u)) = &messages[1]
        && let InputContent::Text { text } = &u.content[0]
    {
        assert_eq!(text, "branch-b");
    } else {
        panic!("expected branch-b after appended leaf");
    }
}

#[test]
fn session_coordinator_links_turn_entries_and_updates_leaf_tip() {
    let dir = tempfile::tempdir().unwrap();
    let mut coord = SessionCoordinator::new_for_test(
        dir.path(),
        "/test",
        opi_agent::compaction::CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    let messages = vec![
        AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "first".into(),
            }],
            timestamp_ms: 0,
        })),
        AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "second".into(),
            }],
            timestamp_ms: 0,
        })),
    ];

    coord
        .on_turn_end_simple(&messages, &opi_ai::stream::Usage::default())
        .unwrap();

    let path = dir.path().join(format!("{}.jsonl", coord.session_id()));
    let (_header, entries) = SessionReader::read_all(&path).unwrap();
    assert_eq!(entries.len(), 3, "two messages plus a leaf pointer");

    let first = match &entries[0] {
        SessionEntry::Message(m) => m,
        other => panic!("expected first message entry, got {other:?}"),
    };
    let second = match &entries[1] {
        SessionEntry::Message(m) => m,
        other => panic!("expected second message entry, got {other:?}"),
    };
    assert_eq!(first.parent_id, None);
    assert_eq!(second.parent_id.as_deref(), Some(first.id.as_str()));

    let leaf = match &entries[2] {
        SessionEntry::Leaf(l) => l,
        other => panic!("expected leaf entry, got {other:?}"),
    };
    assert_eq!(leaf.entry_id, second.id);
    assert_eq!(leaf.parent_id.as_deref(), Some(second.id.as_str()));
}

#[test]
fn open_existing_appends_new_turn_under_active_leaf_tip() {
    let dir = tempfile::tempdir().unwrap();
    let header = make_header("branch-append-sess", "/repo");
    let path = dir.path().join("branch-append-sess.jsonl");
    let mut writer = SessionWriter::create(&path, header).unwrap();

    let user = |id: &str, parent: Option<&str>, text: &str| {
        SessionEntry::Message(MessageEntry {
            id: id.into(),
            parent_id: parent.map(|s| s.into()),
            timestamp: "2026-05-23T12:00:00Z".into(),
            message: Message::User(UserMessage {
                content: vec![InputContent::Text { text: text.into() }],
                timestamp_ms: 0,
            }),
        })
    };

    writer.append(&user("e1", None, "root")).unwrap();
    writer.append(&user("e2a", Some("e1"), "branch-a")).unwrap();
    writer.append(&user("e2b", Some("e1"), "branch-b")).unwrap();
    writer
        .append(&SessionEntry::Leaf(opi_agent::session::LeafEntry {
            id: "leaf-a".into(),
            parent_id: Some("e2a".into()),
            timestamp: "2026-05-23T12:00:01Z".into(),
            entry_id: "e2a".into(),
        }))
        .unwrap();
    drop(writer);

    let (_header, entries) = SessionReader::read_all(&path).unwrap();
    let mut coord = SessionCoordinator::open_existing(
        path.clone(),
        "branch-append-sess".into(),
        &entries,
        2,
        opi_agent::compaction::CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    coord
        .on_turn_end_simple(
            &[AgentMessage::Llm(Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "after-a".into(),
                }],
                timestamp_ms: 0,
            }))],
            &opi_ai::stream::Usage::default(),
        )
        .unwrap();

    let (_header, entries) = SessionReader::read_all(&path).unwrap();
    let appended = entries
        .iter()
        .find_map(|entry| match entry {
            SessionEntry::Message(m) => match &m.message {
                Message::User(u) => match &u.content[0] {
                    InputContent::Text { text } if text == "after-a" => Some(m),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        })
        .expect("new branch message should be persisted");
    assert_eq!(appended.parent_id.as_deref(), Some("e2a"));

    let last_leaf_tip = entries.iter().rev().find_map(|entry| match entry {
        SessionEntry::Leaf(l) => Some(l.entry_id.as_str()),
        _ => None,
    });
    assert_eq!(last_leaf_tip, Some(appended.id.as_str()));
}

#[test]
fn reconstruct_context_walks_chain_without_leaf() {
    // A linear session with a proper parent_id chain but no Leaf entries must
    // still reconstruct in chain (root->tip) order. The opi-agent context
    // builder walks the parent_id chain; a Leaf pointer is not required when
    // the chain is unambiguous.
    use opi_agent::message::AgentMessage;

    let dir = tempfile::tempdir().unwrap();
    let header = make_header("linear-sess", "/repo");
    let path = dir.path().join("linear-sess.jsonl");
    let mut writer = SessionWriter::create(&path, header).unwrap();

    writer.append(&test_message_entry("e1", "a")).unwrap();
    writer
        .append(&chained_message_entry("e2", "e1", "b"))
        .unwrap();
    writer
        .append(&chained_message_entry("e3", "e2", "c"))
        .unwrap();
    drop(writer);

    let (_h, entries) = SessionReader::read_all(&path).unwrap();
    let messages = opi_coding_agent::session_cli::reconstruct_context(&entries);

    assert_eq!(messages.len(), 3);
    let text_at = |idx: usize| -> String {
        match &messages[idx] {
            AgentMessage::Llm(Message::User(u)) => match &u.content[0] {
                InputContent::Text { text } => text.clone(),
                _ => panic!("expected text"),
            },
            _ => panic!("expected user message"),
        }
    };
    assert_eq!(text_at(0), "a");
    assert_eq!(text_at(1), "b");
    assert_eq!(text_at(2), "c");
}

#[test]
fn reconstruct_context_active_branch_with_compaction() {
    // Active-branch walk must still apply compaction semantics: messages
    // on the active branch that precede `first_kept_entry_id` are dropped
    // and replaced by the summary, kept tail is preserved.
    use opi_agent::message::AgentMessage;
    use opi_agent::session::{CompactionEntry, LeafEntry};

    let dir = tempfile::tempdir().unwrap();
    let header = make_header("branch-compact-sess", "/repo");
    let path = dir.path().join("branch-compact-sess.jsonl");
    let mut writer = SessionWriter::create(&path, header).unwrap();

    let user = |id: &str, parent: Option<&str>, text: &str| {
        SessionEntry::Message(MessageEntry {
            id: id.into(),
            parent_id: parent.map(|s| s.into()),
            timestamp: "2026-05-23T12:00:00Z".into(),
            message: Message::User(UserMessage {
                content: vec![InputContent::Text { text: text.into() }],
                timestamp_ms: 0,
            }),
        })
    };

    // Active chain: e1 -> e2 -> c1 (keeps e2) -> e3
    // Sibling: e2b (branch B) — must be dropped.
    writer.append(&user("e1", None, "old")).unwrap();
    writer.append(&user("e2", Some("e1"), "kept-tail")).unwrap();
    writer.append(&user("e2b", Some("e1"), "sibling")).unwrap();
    writer
        .append(&SessionEntry::Compaction(CompactionEntry {
            id: "c1".into(),
            parent_id: Some("e2".into()),
            timestamp: "2026-05-23T13:00:00Z".into(),
            summary: "summary".into(),
            first_kept_entry_id: "e2".into(),
            tokens_before: 100,
            tokens_after: 30,
        }))
        .unwrap();
    writer.append(&user("e3", Some("c1"), "post")).unwrap();
    writer
        .append(&SessionEntry::Leaf(LeafEntry {
            id: "l1".into(),
            parent_id: Some("e3".into()),
            timestamp: "2026-05-23T13:00:01Z".into(),
            entry_id: "e3".into(),
        }))
        .unwrap();
    drop(writer);

    let (_h, entries) = SessionReader::read_all(&path).unwrap();
    let messages = opi_coding_agent::session_cli::reconstruct_context(&entries);

    // Active chain after compaction: [summary, e2, e3]. Sibling e2b dropped.
    assert_eq!(messages.len(), 3, "expected summary + kept-tail + post");
    assert!(matches!(&messages[0], AgentMessage::CompactionSummary(cs) if cs.summary == "summary"));
    if let AgentMessage::Llm(Message::User(u)) = &messages[1]
        && let InputContent::Text { text } = &u.content[0]
    {
        assert_eq!(text, "kept-tail");
    } else {
        panic!("expected kept-tail at index 1");
    }
    if let AgentMessage::Llm(Message::User(u)) = &messages[2]
        && let InputContent::Text { text } = &u.content[0]
    {
        assert_eq!(text, "post");
    } else {
        panic!("expected post at index 2");
    }
}

// ---------------------------------------------------------------------------
// open_existing with branched LeafEntry — regression test
// ---------------------------------------------------------------------------

#[test]
fn open_existing_with_branched_leaf_excludes_sibling_from_compaction_buffer() {
    // Regression: open_existing() must follow the active branch (via
    // select_ordered_entries) when loading entries into the compaction buffer.
    // A naive file-order replay would include sibling branch messages.
    use opi_agent::compaction::CompactionConfig;
    use opi_agent::session::{LeafEntry, MessageEntry};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("branched.jsonl");
    let header = make_header("branched", "/repo");
    let mut writer = SessionWriter::create(&path, header).unwrap();

    // Root message (shared by both branches)
    writer
        .append(&SessionEntry::Message(MessageEntry {
            id: "e1".into(),
            parent_id: None,
            timestamp: "2026-05-22T12:00:00Z".into(),
            message: Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "root".into(),
                }],
                timestamp_ms: 0,
            }),
        }))
        .unwrap();

    // Branch A: e1 -> e2a -> e3a
    writer
        .append(&SessionEntry::Message(MessageEntry {
            id: "e2a".into(),
            parent_id: Some("e1".into()),
            timestamp: "2026-05-22T12:00:01Z".into(),
            message: Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "branch-a-msg".into(),
                }],
                timestamp_ms: 0,
            }),
        }))
        .unwrap();

    // Branch B: e1 -> e2b (sibling)
    writer
        .append(&SessionEntry::Message(MessageEntry {
            id: "e2b".into(),
            parent_id: Some("e1".into()),
            timestamp: "2026-05-22T12:00:02Z".into(),
            message: Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "branch-b-sibling".into(),
                }],
                timestamp_ms: 0,
            }),
        }))
        .unwrap();

    // Leaf points to e2a as the active branch tip.
    writer
        .append(&SessionEntry::Leaf(LeafEntry {
            id: "leaf-1".into(),
            parent_id: None,
            timestamp: "2026-05-22T12:00:03Z".into(),
            entry_id: "e2a".into(),
        }))
        .unwrap();

    drop(writer);

    let (_header, entries) = SessionReader::read_all(&path).unwrap();
    let resumed = SessionCoordinator::open_existing(
        path,
        "branched".into(),
        &entries,
        2, // agent has [root, branch-a-msg]
        CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    let buffer = resumed.compaction_entries();
    // Must contain root (e1) and branch A (e2a), but NOT sibling branch B (e2b).
    assert_eq!(
        buffer.len(),
        2,
        "expected 2 entries (root + branch A), got {} entries: {:?}",
        buffer.len(),
        buffer.iter().map(|e| &e.id).collect::<Vec<_>>()
    );
    assert_eq!(buffer[0].id, "e1", "first entry must be root");
    assert_eq!(buffer[1].id, "e2a", "second entry must be branch A");

    // Verify the sibling is definitely not present.
    let ids: Vec<&str> = buffer.iter().map(|e| e.id.as_str()).collect();
    assert!(
        !ids.contains(&"e2b"),
        "sibling branch B (e2b) must not appear in compaction buffer, got: {ids:?}"
    );
}

// ---------------------------------------------------------------------------
// open_existing replays usage from persisted assistant messages
// ---------------------------------------------------------------------------

#[test]
fn open_existing_replays_usage_from_assistant_messages() {
    use opi_agent::compaction::CompactionConfig;
    use opi_agent::session::MessageEntry;
    use opi_ai::message::{AssistantContent, Message};
    use opi_ai::stream::Usage;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("usage-replay.jsonl");
    let header = make_header("usage-replay", "/repo");
    let mut writer = SessionWriter::create(&path, header).unwrap();

    // Write a user message (no usage).
    writer
        .append(&SessionEntry::Message(MessageEntry {
            id: "msg-1".into(),
            parent_id: None,
            timestamp: "2026-05-22T12:00:00Z".into(),
            message: Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "hello".into(),
                }],
                timestamp_ms: 0,
            }),
        }))
        .unwrap();

    // Write two assistant messages with usage.
    let mut asst1 = test_support::base_assistant();
    asst1
        .content
        .push(AssistantContent::Text { text: "hi".into() });
    asst1.usage = Usage::reported(100, 50, 10, 5, None, None);
    writer
        .append(&SessionEntry::Message(MessageEntry {
            id: "msg-2".into(),
            parent_id: None,
            timestamp: "2026-05-22T12:00:01Z".into(),
            message: Message::Assistant(asst1),
        }))
        .unwrap();

    let mut asst2 = test_support::base_assistant();
    asst2.content.push(AssistantContent::Text {
        text: "world".into(),
    });
    asst2.usage = Usage::reported(200, 80, 20, 10, None, None);
    writer
        .append(&SessionEntry::Message(MessageEntry {
            id: "msg-3".into(),
            parent_id: None,
            timestamp: "2026-05-22T12:00:02Z".into(),
            message: Message::Assistant(asst2),
        }))
        .unwrap();
    drop(writer);

    let (_header, entries) = SessionReader::read_all(&path).unwrap();
    let resumed = SessionCoordinator::open_existing(
        path,
        "usage-replay".into(),
        &entries,
        3,
        CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    let usage = resumed.usage();
    assert_eq!(
        usage.turn_count(),
        1,
        "should count 1 user message as 1 turn, not 2 assistant messages"
    );
    assert_eq!(
        usage.total_input_tokens(),
        300,
        "input tokens must sum both assistants"
    );
    assert_eq!(
        usage.total_output_tokens(),
        130,
        "output tokens must sum both assistants"
    );
    assert_eq!(
        usage.total_cache_read_tokens(),
        30,
        "cache read tokens must sum both assistants"
    );
    assert_eq!(
        usage.total_cache_write_tokens(),
        15,
        "cache write tokens must sum both assistants"
    );
}

#[test]
fn phase14_usage_subsets_survive_session_resume() {
    use std::io::Write as _;

    use opi_agent::compaction::CompactionConfig;
    use opi_agent::session::FORMAT_VERSION;
    use opi_ai::message::AssistantContent;
    use opi_ai::model_info::{ModelInfo, WireApi};
    use opi_ai::stream::Usage;
    use opi_coding_agent::harness::ResumeInfo;

    let dir = tempfile::tempdir().unwrap();
    let mut coordinator = SessionCoordinator::new_for_test(
        dir.path(),
        "/repo",
        CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();
    let session_id = coordinator.session_id().to_owned();
    let session_path = coordinator.session_path().to_path_buf();

    let usage = Usage::reported(
        1_000_000,
        500_000,
        200_000,
        100_000,
        Some(25_000),
        Some(100_000),
    );
    let mut assistant = test_support::base_assistant();
    assistant.content.push(AssistantContent::Text {
        text: "persisted".into(),
    });
    assistant.usage = usage.clone();
    let messages = vec![
        AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "persist usage".into(),
            }],
            timestamp_ms: 0,
        })),
        AgentMessage::Llm(Message::Assistant(assistant)),
    ];
    coordinator.on_turn_end_simple(&messages, &usage).unwrap();

    let original_usage = coordinator.usage().clone();
    let original_cost = coordinator
        .cost_summary()
        .expect("reported usage has known model pricing");
    drop(coordinator);

    let raw = std::fs::read_to_string(&session_path).unwrap();
    if let Some(artifact_dir) = std::env::var_os("OPI_TEST_ARTIFACT_DIR") {
        let artifact_dir = std::path::PathBuf::from(artifact_dir);
        std::fs::create_dir_all(&artifact_dir).unwrap();
        std::fs::write(artifact_dir.join("phase14-session-resume.jsonl"), &raw).unwrap();
    }
    assert!(
        !raw.contains("\"cost\""),
        "calculated cost must not be persisted in session JSONL"
    );
    let (header, entries, recovery) = SessionReader::read_with_recovery(&session_path).unwrap();
    assert!(recovery.is_clean());
    assert_eq!(header.version, FORMAT_VERSION);
    assert_eq!(FORMAT_VERSION, 2, "durable session bindings use schema v2");

    let reconstructed = opi_agent::session_context::reconstruct_context(&entries, &recovery);
    let resumed_harness = CodingHarness::builder(
        Box::new(MockProvider::new_with_models(
            "anthropic",
            vec![ModelInfo::new(
                "claude-sonnet-4",
                "Claude Sonnet 4",
                WireApi::AnthropicMessages,
                ModelCapabilities::new(200_000, 8_192),
            )],
            Vec::new(),
        )),
        "anthropic:claude-sonnet-4".into(),
        OpiConfig::default(),
        dir.path().to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .initial_messages(reconstructed.messages)
    .resume(ResumeInfo {
        path: session_path,
        session_id,
        entries: entries.clone(),
        original_cwd: dir.path().to_path_buf(),
        diagnostics: reconstructed.diagnostics,
        recorded_model: reconstructed.model,
        recorded_thinking: reconstructed.thinking_level,
    })
    .build();
    let resumed = resumed_harness
        .session()
        .expect("fresh harness resumed the persisted session");
    assert_eq!(resumed.usage(), &original_usage);
    assert_eq!(resumed.usage().total_cache_write_1h_tokens(), Some(25_000));
    assert_eq!(resumed.usage().total_reasoning_tokens(), Some(100_000));
    assert_eq!(resumed.cost_summary(), Some(original_cost));

    let legacy_path = dir.path().join("legacy-missing-optional-usage.jsonl");
    let legacy_header = make_header("legacy-missing-optional-usage", "/repo");
    let legacy_assistant = entries
        .iter()
        .find(|entry| {
            matches!(
                entry,
                SessionEntry::Message(MessageEntry {
                    message: Message::Assistant(_),
                    ..
                })
            )
        })
        .expect("persisted assistant entry");
    let mut legacy_value = serde_json::to_value(legacy_assistant).unwrap();
    fn remove_optional_usage_fields(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                map.remove("cache_write_1h_tokens");
                map.remove("reasoning_tokens");
                for child in map.values_mut() {
                    remove_optional_usage_fields(child);
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    remove_optional_usage_fields(item);
                }
            }
            _ => {}
        }
    }
    remove_optional_usage_fields(&mut legacy_value);
    let legacy_writer = SessionWriter::create(&legacy_path, legacy_header).unwrap();
    drop(legacy_writer);
    let mut legacy_file = std::fs::OpenOptions::new()
        .append(true)
        .open(&legacy_path)
        .unwrap();
    let legacy_envelope = serde_json::json!({
        "type": "entry",
        "classification": "required",
        "entry": legacy_value,
    });
    writeln!(
        legacy_file,
        "{}",
        serde_json::to_string(&legacy_envelope).unwrap()
    )
    .unwrap();
    drop(legacy_file);

    let (legacy_header, legacy_entries) = SessionReader::read_all(&legacy_path).unwrap();
    assert_eq!(legacy_header.version, FORMAT_VERSION);
    let legacy_usage = legacy_entries
        .iter()
        .find_map(|entry| match entry {
            SessionEntry::Message(MessageEntry {
                message: Message::Assistant(assistant),
                ..
            }) => Some(&assistant.usage),
            _ => None,
        })
        .expect("legacy assistant usage deserialized");
    assert_eq!(legacy_usage.cache_write_1h_tokens, None);
    assert_eq!(legacy_usage.reasoning_tokens, None);
}

// ---------------------------------------------------------------------------
// Phase 8 task 8.6 — resumed session-recovery diagnostics reach the in-process sink
//
// resume_session_id loops record_harness_diagnostic over session.diagnostics
// (mirroring compaction), so recovery diagnostics from a truncated/corrupt
// session JSONL land in the in-process RecordingSink and are counted by
// diagnostic_counts() — not just in resource_metadata. This pins that wiring.
// ---------------------------------------------------------------------------

#[test]
fn open_existing_treats_legacy_nonzero_usage_as_reported_for_cost_summary() {
    use opi_agent::compaction::CompactionConfig;
    use opi_agent::session::MessageEntry;
    use opi_ai::message::{AssistantContent, Message};
    use opi_ai::stream::Usage;
    use std::io::Write;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy-usage-replay.jsonl");
    let header = make_header("legacy-usage-replay", "/repo");
    let mut writer = SessionWriter::create(&path, header).unwrap();

    writer
        .append(&SessionEntry::Message(MessageEntry {
            id: "msg-1".into(),
            parent_id: None,
            timestamp: "2026-05-22T12:00:00Z".into(),
            message: Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "hello".into(),
                }],
                timestamp_ms: 0,
            }),
        }))
        .unwrap();
    drop(writer);

    let mut assistant = test_support::base_assistant();
    assistant.content.push(AssistantContent::Text {
        text: "legacy".into(),
    });
    assistant.usage = Usage::reported(1_000_000, 500_000, 0, 0, None, None);
    let assistant_entry = SessionEntry::Message(MessageEntry {
        id: "msg-2".into(),
        parent_id: None,
        timestamp: "2026-05-22T12:00:01Z".into(),
        message: Message::Assistant(assistant),
    });
    let mut legacy_value = serde_json::to_value(&assistant_entry).unwrap();
    remove_reported_flags(&mut legacy_value);

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    let legacy_envelope = serde_json::json!({
        "type": "entry",
        "classification": "required",
        "entry": legacy_value,
    });
    writeln!(file, "{}", serde_json::to_string(&legacy_envelope).unwrap()).unwrap();
    drop(file);

    let (_header, entries) = SessionReader::read_all(&path).unwrap();
    let resumed = SessionCoordinator::open_existing(
        path,
        "legacy-usage-replay".into(),
        &entries,
        2,
        CompactionConfig::default(),
        "anthropic:claude-sonnet-4",
    )
    .unwrap();

    let cost = resumed
        .cost_summary()
        .expect("legacy nonzero usage without reported field should still price");
    assert!((cost.input_cost - 3.0).abs() < 1e-6);
    assert!((cost.output_cost - 7.5).abs() < 1e-6);
    assert!((cost.total_cost() - 10.5).abs() < 1e-6);
}

fn remove_reported_flags(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            map.remove("reported");
            for child in map.values_mut() {
                remove_reported_flags(child);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                remove_reported_flags(item);
            }
        }
        _ => {}
    }
}

#[test]
fn phase8_session_recovery_diagnostics_reach_in_process_sink() {
    let _lock = session_lock();
    let dir = tempfile::tempdir().unwrap();
    set_sessions_dir(dir.path());

    let path = dir.path().join("recovery-sink-resume.jsonl");
    let header = make_header("recovery-sink-resume", "/repo");
    let mut writer = SessionWriter::create(&path, header).unwrap();
    writer
        .append(&test_message_entry("msg-1", "hello"))
        .unwrap();
    drop(writer);

    // Append a truncated trailing line so resume yields TruncatedLine recovery.
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    write!(file, "{{\"type\":\"message\"").unwrap();

    let provider = MockProvider::new("mock", Vec::new());
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .record_diagnostics(true)
    .build();

    let message_count = harness
        .resume_session_id("recovery-sink-resume")
        .expect("resume should recover preceding entries");
    assert_eq!(message_count, 1);

    // The recovery diagnostic must be counted in the in-process sink, not just
    // retained in resource metadata. TruncatedLine is Severity::Warning.
    let counts = harness
        .diagnostic_counts()
        .expect("record_diagnostics installs a recording sink");
    assert!(
        counts.warning >= 1,
        "resumed session-recovery diagnostics should be counted in the in-process sink: {counts:?}"
    );

    // The matching diagnostic is present in the sink snapshot with the stable
    // session-recovery code and source.
    let saw_recovery = harness
        .resource_metadata()
        .diagnostics
        .iter()
        .any(|d| d.code == code::CODE_SESSION_TRUNCATED_LINE && d.source == SOURCE_SESSION);
    assert!(
        saw_recovery,
        "recovery diagnostic must also be retained in resource metadata"
    );

    clear_sessions_dir();
}

// ---------------------------------------------------------------------------
// Phase 8: cancellation persists only finalized state (task 8.4)
//
// A cancelled turn must not write partial assistant/tool/session state: the
// harness skips persistence for a turn whose agent run returns
// Err(AgentError::Cancelled), and SessionCoordinator only ever appends
// finalized AgentMessage::Llm entries. So after a completed turn 1 followed by
// a cancelled turn 2, the session JSONL contains only turn 1's finalized
// messages.
// ---------------------------------------------------------------------------

/// Provider whose first stream completes (so turn 1 finalizes and persists) and
/// whose second stream hangs mid-stream (so turn 2 can be cancelled before any
/// assistant message is finalized).
struct CompleteThenHangProvider {
    calls: Arc<Mutex<usize>>,
    second_stream_pending: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
}

impl CompleteThenHangProvider {
    fn new() -> (Self, tokio::sync::oneshot::Receiver<()>) {
        let (second_stream_pending, second_stream_ready) = tokio::sync::oneshot::channel();
        (
            Self {
                calls: Arc::new(Mutex::new(0)),
                second_stream_pending: Mutex::new(Some(second_stream_pending)),
            },
            second_stream_ready,
        )
    }
}

impl Provider for CompleteThenHangProvider {
    fn id(&self) -> &str {
        "mock"
    }
    fn models(&self) -> &[opi_ai::provider::ModelInfo] {
        static MODELS: std::sync::OnceLock<Vec<opi_ai::provider::ModelInfo>> =
            std::sync::OnceLock::new();
        MODELS
            .get_or_init(|| {
                vec![opi_ai::provider::ModelInfo::new(
                    "mock-model",
                    "Mock Model",
                    opi_ai::WireApi::OpenAiCompletions,
                    opi_ai::ModelCapabilities::new(100_000, 4_096),
                )]
            })
            .as_slice()
    }
    fn stream_prepared(&self, _request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        let mut count = self.calls.lock().unwrap();
        *count += 1;
        if *count == 1 {
            let events = test_support::text_response("first-response");
            Box::pin(stream::iter(events.into_iter().map(Ok::<_, ProviderError>)))
        } else {
            // Emit Start + a partial TextDelta, then hang. The Done event that
            // would finalize the assistant message never arrives.
            let pending = self.second_stream_pending.lock().unwrap().take();
            let mut partial = test_support::base_assistant();
            partial.content.push(AssistantContent::Text {
                text: "partial".into(),
            });
            Box::pin(
                stream::iter([
                    Ok::<_, ProviderError>(AssistantStreamEvent::Start {
                        partial: test_support::base_assistant(),
                    }),
                    Ok(AssistantStreamEvent::TextDelta {
                        content_index: 0,
                        delta: "partial".into(),
                        partial,
                    }),
                ])
                .chain(stream::once(async move {
                    if let Some(pending) = pending {
                        let _ = pending.send(());
                    }
                    std::future::pending::<Result<AssistantStreamEvent, ProviderError>>().await
                })),
            )
        }
    }
}

// The `session_lock()` guard serializes OPI_SESSIONS_DIR mutation across
// sibling tests and must be held for the whole test, including its awaits.
// There is no re-entrant acquisition, so the std Mutex is safe to hold across
// these await points.
#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase8_cancel_persists_only_finalized_state() {
    let _lock = session_lock();
    let dir = tempfile::tempdir().expect("tempdir");
    set_sessions_dir(dir.path());

    let (provider, second_stream_pending) = CompleteThenHangProvider::new();
    let mut harness = CodingHarness::new(
        Box::new(provider),
        "mock-model".into(),
        OpiConfig::default(),
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );
    // Turn 1 completes; the harness persists the finalized user + assistant
    // messages (and a leaf) to the session JSONL.
    let result1 = harness.prompt("first").await.unwrap();
    assert!(
        result1
            .iter()
            .any(|m| matches!(m, AgentMessage::Llm(Message::Assistant(_)))),
        "turn 1 produces a finalized assistant message"
    );

    // Turn 2 hangs mid-stream; cancel it. The harness returns Err(Cancelled)
    // and skips persistence for the cancelled turn.
    let token = harness.cancel_token();
    let mut handle = tokio::spawn(async move { harness.continue_("second").await });
    match tokio::time::timeout(Duration::from_secs(2), second_stream_pending).await {
        Ok(Ok(())) => {}
        result => {
            handle.abort();
            let _ = handle.await;
            clear_sessions_dir();
            panic!("second provider stream did not reach its pending tail: {result:?}");
        }
    }
    token.cancel();
    let result2 = match tokio::time::timeout(Duration::from_secs(2), &mut handle).await {
        Ok(result) => result.expect("continue task panicked"),
        Err(_) => {
            handle.abort();
            let _ = handle.await;
            clear_sessions_dir();
            panic!("cancelled turn did not finish within timeout");
        }
    };
    assert!(
        matches!(result2, Err(opi_agent::loop_types::AgentError::Cancelled)),
        "cancelled turn returns Err(Cancelled)"
    );

    // Read back the session JSONL.
    let jsonl_path = {
        let mut files: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
            .collect();
        assert_eq!(
            files.len(),
            1,
            "exactly one session JSONL exists: {files:?}"
        );
        files.remove(0)
    };
    let (_header, entries) = SessionReader::read_all(&jsonl_path).unwrap();

    // Only turn-1 finalized messages persist: the user prompt + the assistant
    // response. The cancelled turn 2 contributes nothing.
    assert_eq!(
        message_entry_count(&entries),
        2,
        "only the turn-1 finalized user + assistant messages persist: {entries:?}"
    );
    let mut leaked_texts: Vec<String> = Vec::new();
    for e in &entries {
        let SessionEntry::Message(MessageEntry { message, .. }) = e else {
            continue;
        };
        match message {
            Message::User(u) => {
                for c in &u.content {
                    if let InputContent::Text { text } = c {
                        leaked_texts.push(text.clone());
                    }
                }
            }
            Message::Assistant(a) => {
                for c in &a.content {
                    if let AssistantContent::Text { text } = c {
                        leaked_texts.push(text.clone());
                    }
                }
            }
            _ => {}
        }
    }
    assert!(
        !leaked_texts.iter().any(|t| t == "partial"),
        "no partial assistant content from the cancelled turn may be persisted: {leaked_texts:?}"
    );
    assert!(
        !leaked_texts.iter().any(|t| t == "second"),
        "the cancelled turn's user message must not be persisted: {leaked_texts:?}"
    );

    clear_sessions_dir();
}

// ---------------------------------------------------------------------------
// Phase 13.3: idle model/thinking metadata writes through the harness
// ---------------------------------------------------------------------------

/// Custom-model extension override so `set_model_validated("mock:custom-model")`
/// resolves against the same `mock` provider the harness was built with.
struct ModelOverrideExtension;

impl opi_agent::extension::Extension for ModelOverrideExtension {
    fn name(&self) -> &str {
        "model-override-extension"
    }

    fn model_overrides(&self) -> Vec<(String, opi_ai::provider::ModelInfo)> {
        vec![
            (
                "mock".into(),
                opi_ai::provider::ModelInfo::new(
                    "custom-model",
                    "Custom Model",
                    opi_ai::WireApi::OpenAiCompletions,
                    ModelCapabilities::new(100_000, 4_096)
                        .with_images(true)
                        .with_streaming(true),
                ),
            ),
            (
                "mock".into(),
                opi_ai::provider::ModelInfo::new(
                    "thinking-model",
                    "Thinking Model",
                    opi_ai::WireApi::OpenAiCompletions,
                    ModelCapabilities::new(100_000, 4_096)
                        .with_images(true)
                        .with_streaming(true)
                        .with_thinking(true),
                ),
            ),
        ]
    }
}

fn build_phase13_harness(workspace: &std::path::Path) -> CodingHarness {
    build_phase13_harness_with_model(workspace, "mock:mock-model")
}

fn build_phase13_harness_with_model(workspace: &std::path::Path, model: &str) -> CodingHarness {
    let mut registry = opi_agent::extension::ExtensionRegistry::new();
    registry.register(Box::new(ModelOverrideExtension)).unwrap();
    CodingHarness::builder(
        Box::new(MockProvider::new(
            "mock",
            vec![test_support::text_response("ok")],
        )),
        model.into(),
        OpiConfig::default(),
        workspace.to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .extension_registry(registry)
    .build()
}

fn delete_active_session_file(harness: &CodingHarness) -> std::path::PathBuf {
    let path = harness
        .session()
        .expect("session exists")
        .session_path()
        .to_path_buf();
    std::fs::remove_file(&path).expect("delete active session file");
    assert!(!path.exists(), "session path removed before metadata write");
    path
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn phase13_model_thinking_changes_persist_via_harness() {
    use opi_agent::session::SessionEntry;

    let _lock = session_lock();
    let workspace = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());
    let mut harness = build_phase13_harness(workspace.path());

    harness.prompt("seed turn").await.unwrap();

    // Capture the session file state right after the turn (one Leaf at the
    // content tip) so we can prove metadata writes do not advance it.
    let jsonl_path = {
        let session = harness.session().expect("session exists after prompt");
        let id = session.session_id().to_owned();
        sessions.path().join(format!("{id}.jsonl"))
    };
    let (_h, entries_after_prompt) = SessionReader::read_all(&jsonl_path).unwrap();
    let leaf_count_after_prompt = entries_after_prompt
        .iter()
        .filter(|e| matches!(e, SessionEntry::Leaf(_)))
        .count();

    // Idle metadata writes through the validated harness paths.
    harness
        .set_model_validated("mock:custom-model".into())
        .expect("same-provider model accepted");
    harness
        .set_thinking_level("off")
        .expect("off accepted on non-thinking model");

    let (_h, entries_after_meta) = SessionReader::read_all(&jsonl_path).unwrap();

    let model_change = entries_after_meta.iter().find_map(|e| match e {
        SessionEntry::ModelChange(m) => Some(m.clone()),
        _ => None,
    });
    let thinking_change = entries_after_meta.iter().find_map(|e| match e {
        SessionEntry::ThinkingLevelChange(t) => Some(t.clone()),
        _ => None,
    });
    let model_change = model_change.expect("model_change entry persisted");
    let thinking_change = thinking_change.expect("thinking_level_change entry persisted");
    assert_eq!(model_change.model, "mock:custom-model");
    assert_eq!(thinking_change.level, ThinkingLevel::None);

    // Metadata is parented to the content tip (the last Message entry id), not
    // to a new Leaf, and no new Leaf was appended.
    let content_tip = entries_after_prompt
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEntry::Message(m) => Some(m.id.clone()),
            _ => None,
        })
        .expect("a content tip exists");
    assert_eq!(
        model_change.parent_id.as_deref(),
        Some(content_tip.as_str())
    );
    assert_eq!(
        thinking_change.parent_id.as_deref(),
        Some(content_tip.as_str())
    );
    let leaf_count_after_meta = entries_after_meta
        .iter()
        .filter(|e| matches!(e, SessionEntry::Leaf(_)))
        .count();
    assert_eq!(
        leaf_count_after_meta, leaf_count_after_prompt,
        "metadata writes must not append a Leaf"
    );

    clear_sessions_dir();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn phase13_model_change_write_failure_leaves_runtime_model_unchanged() {
    let _lock = session_lock();
    let workspace = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());
    let mut harness = build_phase13_harness(workspace.path());

    let previous_model = harness.model().to_owned();
    let deleted_path = delete_active_session_file(&harness);

    let err = harness
        .set_model_validated("mock:custom-model".into())
        .expect_err("missing session path must fail before mutating model");

    assert!(
        err.contains("model change write failed"),
        "metadata write failure should be surfaced: {err}"
    );
    assert_eq!(
        harness.model(),
        previous_model,
        "failed metadata write must leave runtime model unchanged"
    );
    assert!(
        !deleted_path.exists(),
        "failed metadata write must not recreate the deleted session path"
    );

    clear_sessions_dir();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn phase13_thinking_change_write_failure_leaves_runtime_thinking_unchanged() {
    let _lock = session_lock();
    let workspace = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());
    let mut harness = build_phase13_harness_with_model(workspace.path(), "mock:thinking-model");

    let previous = harness.thinking_config();
    let deleted_path = delete_active_session_file(&harness);

    let err = match harness.set_thinking_level("low") {
        Ok(_) => panic!("missing session path must fail before mutating thinking config"),
        Err(err) => err,
    };

    assert!(
        err.contains("thinking level write failed"),
        "metadata write failure should be surfaced: {err}"
    );
    let current = harness.thinking_config();
    assert_eq!(
        current.enabled, previous.enabled,
        "failed metadata write must leave thinking enabled flag unchanged"
    );
    assert_eq!(
        current.budget_tokens, previous.budget_tokens,
        "failed metadata write must leave thinking budget unchanged"
    );
    assert!(
        !deleted_path.exists(),
        "failed metadata write must not recreate the deleted session path"
    );

    clear_sessions_dir();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn phase13_labels_and_session_name_do_not_enter_provider_context() {
    //! Phase 13.4 acceptance: `/label`, `/unlabel`, and `/name` appends through
    //! the harness persist `label` and `session_info` entries (UI-visible
    //! metadata) but the canary text never appears in any message sent to the
    //! provider. This is the harness-level half of DoD scenario
    //! `phase13-session-name-label-commands`.

    use opi_ai::message::{AssistantContent, InputContent, Message};
    use opi_ai::test_support;

    let _lock = session_lock();
    let workspace = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());

    let provider = MockProvider::new("mock", vec![test_support::text_response("ok-response")]);
    let call_log = provider.call_log_handle();
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .build();

    // Idle metadata writes through the harness validated paths.
    harness
        .set_session_name("SECRET_NAME_CANARY".into())
        .expect("session name appended");
    harness
        .add_label("SECRET_LABEL_CANARY".into())
        .expect("label appended");

    harness.prompt("hello").await.unwrap();

    // No request sent to the provider carries the canaries in any message
    // content. Labels and session_info are metadata only.
    let log = call_log.lock().unwrap();
    assert!(
        !log.is_empty(),
        "provider should have received at least one request"
    );
    for request in log.iter() {
        for message in &request.messages {
            match message {
                Message::User(u) => {
                    for c in &u.content {
                        if let InputContent::Text { text } = c {
                            assert!(
                                !text.contains("SECRET_LABEL_CANARY"),
                                "label text leaked into provider context: {text}"
                            );
                            assert!(
                                !text.contains("SECRET_NAME_CANARY"),
                                "session name leaked into provider context: {text}"
                            );
                        }
                    }
                }
                Message::Assistant(a) => {
                    for c in &a.content {
                        if let AssistantContent::Text { text } = c {
                            assert!(
                                !text.contains("SECRET_LABEL_CANARY"),
                                "label leaked via assistant content: {text}"
                            );
                            assert!(
                                !text.contains("SECRET_NAME_CANARY"),
                                "name leaked via assistant content: {text}"
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        if let Some(system) = &request.system {
            assert!(
                !system.contains("SECRET_LABEL_CANARY") && !system.contains("SECRET_NAME_CANARY"),
                "metadata leaked into system prompt: {system}"
            );
        }
    }

    clear_sessions_dir();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn phase13_session_metadata_aggregates_name_labels_branch_model_thinking() {
    //! Phase 13.4 acceptance: `CodingHarness::session_metadata()` returns the
    //! live aggregated view (name, labels, active branch, model, thinking)
    //! that `/session info` and RPC `session_info` surface to the user.

    let _lock = session_lock();
    let workspace = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());
    let mut harness = build_phase13_harness(workspace.path());
    harness.prompt("seed turn").await.unwrap();

    harness
        .set_session_name("demo-session".into())
        .expect("name appended");
    harness.add_label("bug".into()).expect("label add");
    harness.add_label("draft".into()).expect("label add");
    harness.remove_label("bug".into()).expect("label remove");

    let meta = harness
        .session_metadata()
        .expect("session_metadata returns Some when a session is active");
    assert_eq!(meta.name.as_deref(), Some("demo-session"));
    assert_eq!(meta.labels, vec!["draft".to_string()]);
    assert!(
        meta.active_branch.is_some(),
        "active_branch must report the content tip after a turn"
    );
    assert_eq!(meta.model, "mock:mock-model");
    // Mock model has no thinking; the aggregated config reflects that.
    assert!(!meta.thinking.enabled);

    clear_sessions_dir();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn phase13_session_metadata_survives_resume() {
    //! Phase 13.4 acceptance: session name + labels written through the
    //! harness idle paths survive a drop + resume cycle. This closes the
    //! write-then-read loop through production wiring (harness write ->
    //! SessionCoordinator append -> JSONL -> `resume_session_id` ->
    //! `open_existing` seed -> `session_metadata`).

    let _lock = session_lock();
    let workspace = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());

    // Write phase: seed a turn and write metadata through the harness.
    let session_id = {
        let mut harness_a = build_phase13_harness(workspace.path());
        harness_a.prompt("seed turn").await.unwrap();
        harness_a
            .set_session_name("persistent-name".into())
            .expect("name appended");
        harness_a.add_label("keep".into()).expect("label keep add");
        harness_a.add_label("drop".into()).expect("label drop add");
        harness_a
            .remove_label("drop".into())
            .expect("label drop remove");
        harness_a
            .session()
            .expect("session exists after prompt")
            .session_id()
            .to_owned()
    };
    // harness_a dropped here; its session file persists in `sessions`.

    // Read phase: a fresh harness resumes by id and observes the metadata.
    let mut harness_b = build_phase13_harness(workspace.path());
    harness_b
        .resume_session_id(&session_id)
        .expect("resume succeeds");
    let meta = harness_b
        .session_metadata()
        .expect("session_metadata after resume");
    assert_eq!(meta.name.as_deref(), Some("persistent-name"));
    assert_eq!(meta.labels, vec!["keep".to_string()]);
    assert!(
        meta.active_branch.is_some(),
        "resumed session reports an active branch"
    );

    clear_sessions_dir();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn phase13_harness_session_tree_accessor_matches_direct_reader_tree() {
    //! Phase 13 remediation: branch picker, branch resume, and RPC session_info
    //! should share one harness-level session-tree read path built on
    //! `SessionReader::read_with_recovery`, rather than each re-reading the
    //! JSONL with slightly different recovery behavior.

    let _lock = session_lock();
    let workspace = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());

    let mut harness = build_phase13_harness(workspace.path());
    harness.prompt("seed turn").await.unwrap();

    let (tree, recovery) = harness
        .session_tree()
        .expect("active harness exposes a session tree");
    assert!(recovery.is_clean(), "fresh session should read cleanly");

    let path = harness
        .session()
        .expect("session exists after prompt")
        .session_path()
        .to_path_buf();
    let (_header, entries, direct_recovery) =
        SessionReader::read_with_recovery(&path).expect("direct session read");
    assert!(direct_recovery.is_clean());
    let direct_tree = opi_agent::session_branch::SessionTree::from_entries(&entries);

    assert_eq!(tree.active_tip(), direct_tree.active_tip());
    assert_eq!(
        tree.active_branch_index(),
        direct_tree.active_branch_index()
    );
    assert_eq!(tree.branches().len(), direct_tree.branches().len());

    clear_sessions_dir();
}

/// Pre-populate a session file with a recorded `model_change` and
/// `thinking_level_change` parented to the content tip, plus a Leaf pointer.
fn write_phase13_recorded_session(
    dir: &std::path::Path,
    session_id: &str,
    recorded_model: &str,
    recorded_thinking: ThinkingLevel,
) -> std::path::PathBuf {
    use opi_agent::session::{LeafEntry, ModelChangeEntry, ThinkingLevelChangeEntry};

    let path = dir.join(format!("{session_id}.jsonl"));
    let header = SessionHeader::new_for_test(
        session_id.into(),
        "2026-07-05T12:00:00Z".into(),
        "/repo".into(),
        None,
    );
    let mut writer = SessionWriter::create(&path, header).unwrap();
    let user = SessionEntry::Message(MessageEntry {
        id: "msg-1".into(),
        parent_id: None,
        timestamp: "2026-07-05T12:00:01Z".into(),
        message: Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "seed".into(),
            }],
            timestamp_ms: 0,
        }),
    });
    writer.append(&user).unwrap();
    writer
        .append(&SessionEntry::ModelChange(ModelChangeEntry {
            id: "model-1".into(),
            parent_id: Some("msg-1".into()),
            timestamp: "2026-07-05T12:00:02Z".into(),
            model: recorded_model.into(),
            input_source: None,
        }))
        .unwrap();
    writer
        .append(&SessionEntry::ThinkingLevelChange(
            ThinkingLevelChangeEntry {
                id: "thinking-1".into(),
                parent_id: Some("msg-1".into()),
                timestamp: "2026-07-05T12:00:03Z".into(),
                level: recorded_thinking,
            },
        ))
        .unwrap();
    writer
        .append(&SessionEntry::Leaf(LeafEntry {
            id: "leaf-1".into(),
            parent_id: Some("msg-1".into()),
            timestamp: "2026-07-05T12:00:04Z".into(),
            entry_id: "msg-1".into(),
        }))
        .unwrap();
    path
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn phase13_resume_applies_recorded_model_and_thinking() {
    use opi_agent::session_event::ThinkingLevel;

    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());
    write_phase13_recorded_session(
        sessions.path(),
        "resume-compat",
        "mock:custom-model",
        ThinkingLevel::None,
    );

    let workspace = tempfile::tempdir().unwrap();
    let mut harness = build_phase13_harness(workspace.path());
    // Harness starts on mock:mock-model; resume should re-apply the recorded
    // mock:custom-model on the same provider. harness.model() returns the bare
    // model-id half.
    assert_eq!(harness.model(), "mock-model");
    harness
        .resume_session_id("resume-compat")
        .expect("resume succeeds");

    assert_eq!(
        harness.model(),
        "custom-model",
        "resume re-applies the recorded model_change when the provider matches"
    );
    let thinking = harness.thinking_config();
    assert!(
        !thinking.enabled,
        "resume re-applies the recorded thinking_level_change (None -> disabled)"
    );

    clear_sessions_dir();
}

#[test]
fn phase13_builder_resume_applies_recorded_model_cli_path() {
    //! Phase 13.3 acceptance (M1): the CLI `--resume <ID>` path builds the
    //! harness through `CodingHarnessBuilder::resume(ResumeInfo)`, which must
    //! also re-apply the recorded `model_change` — not just the interactive
    //! `resume_session_id` path. The recorded model/thinking are carried on
    //! `ResumeInfo` from `main.rs` and applied at the end of
    //! `new_with_build_options`.
    use opi_coding_agent::harness::ResumeInfo;

    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());

    // A minimal session file the builder can point at (path + entries). The
    // recorded metadata is supplied via ResumeInfo directly to exercise the
    // builder-application path independent of reconstruct_context.
    let session_path = sessions.path().join("cli-resume.jsonl");
    let header = SessionHeader::new_for_test(
        "cli-resume".into(),
        "2026-07-05T12:00:00Z".into(),
        workspace.path().display().to_string(),
        None,
    );
    let mut writer = SessionWriter::create(&session_path, header).unwrap();
    writer.append(&test_message_entry("m1", "seed")).unwrap();
    drop(writer);

    let mut registry = opi_agent::extension::ExtensionRegistry::new();
    registry.register(Box::new(ModelOverrideExtension)).unwrap();
    let harness = CodingHarness::builder(
        Box::new(MockProvider::new(
            "mock",
            vec![test_support::text_response("ok")],
        )),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .extension_registry(registry)
    .resume(ResumeInfo {
        path: session_path,
        session_id: "cli-resume".into(),
        entries: Vec::new(),
        original_cwd: workspace.path().to_path_buf(),
        diagnostics: Vec::new(),
        recorded_model: Some("mock:custom-model".into()),
        recorded_thinking: Some(ThinkingLevel::None),
    })
    .build();

    assert_eq!(
        harness.model(),
        "custom-model",
        "CLI --resume path must re-apply the recorded model_change"
    );
    assert!(
        !harness.thinking_config().enabled,
        "CLI --resume path must re-apply the recorded thinking_level_change"
    );

    clear_sessions_dir();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn phase13_resume_emits_diagnostic_for_incompatible_recorded_model() {
    use opi_agent::diagnostic::code::CODE_SESSION_RESUME_MODEL_INCOMPATIBLE;

    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());
    // Recorded model_change targets a different provider than the harness's
    // active `mock` provider.
    write_phase13_recorded_session(
        sessions.path(),
        "resume-incompat",
        "openai:gpt-4o",
        ThinkingLevel::None,
    );

    let workspace = tempfile::tempdir().unwrap();
    let mut harness = CodingHarness::builder(
        Box::new(MockProvider::new(
            "mock",
            vec![test_support::text_response("ok")],
        )),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .record_diagnostics(true)
    .build();

    harness
        .resume_session_id("resume-incompat")
        .expect("resume still succeeds despite the incompatible recording");

    assert_eq!(
        harness.model(),
        "mock-model",
        "CLI/config model is kept when the recorded model is incompatible"
    );
    let diags = harness.recorded_diagnostics();
    assert!(
        diags
            .iter()
            .any(|d| d.code == CODE_SESSION_RESUME_MODEL_INCOMPATIBLE),
        "expected a Phase 7 resume-model-incompatible diagnostic: {diags:?}"
    );

    clear_sessions_dir();
}

// ---------------------------------------------------------------------------
// Phase 13.3: resume and fork route through the opi-agent context builder
// ---------------------------------------------------------------------------

/// Build a session file whose active branch carries a `branch_summary` parented
/// to the content tip (per the 13.1 storage invariant, the summary does not
/// advance the tip — the next message's parent_id is the content tip, not the
/// summary id). Returns the session file path.
fn write_phase13_branch_summary_session(
    dir: &std::path::Path,
    session_id: &str,
) -> std::path::PathBuf {
    use opi_agent::session::{BranchSummaryEntry, LeafEntry};

    let ts = |s: &str| s.to_owned();
    let path = dir.join(format!("{session_id}.jsonl"));
    let header = SessionHeader::new_for_test(
        session_id.into(),
        ts("2026-07-05T12:00:00Z"),
        "/repo".into(),
        None,
    );
    let mut writer = SessionWriter::create(&path, header).unwrap();
    let user_at = |id: &str, parent: Option<&str>, text: &str| {
        SessionEntry::Message(MessageEntry {
            id: id.into(),
            parent_id: parent.map(str::to_owned),
            timestamp: ts("2026-07-05T12:00:01Z"),
            message: Message::User(UserMessage {
                content: vec![InputContent::Text { text: text.into() }],
                timestamp_ms: 0,
            }),
        })
    };
    writer.append(&user_at("m1", None, "root")).unwrap();
    writer
        .append(&SessionEntry::BranchSummary(BranchSummaryEntry {
            id: "bs1".into(),
            parent_id: Some("m1".into()),
            timestamp: ts("2026-07-05T12:00:02Z"),
            summary: "branch summary text".into(),
        }))
        .unwrap();
    // m2 is parented to m1 (the content tip), NOT bs1 — branch summaries are
    // metadata that do not advance the tip.
    writer.append(&user_at("m2", Some("m1"), "child")).unwrap();
    writer
        .append(&SessionEntry::Leaf(LeafEntry {
            id: "l1".into(),
            parent_id: Some("m2".into()),
            timestamp: ts("2026-07-05T12:00:03Z"),
            entry_id: "m2".into(),
        }))
        .unwrap();
    path
}

/// Fingerprint a message sequence as variant tags + a content probe, so resume
/// and fork outputs can be compared without exposing AgentMessage internals.
fn fingerprint_messages(messages: &[AgentMessage]) -> Vec<String> {
    messages
        .iter()
        .map(|m| match m {
            AgentMessage::Llm(Message::User(u)) => format!(
                "user:{}",
                u.content
                    .iter()
                    .find_map(|c| match c {
                        InputContent::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .unwrap_or_default()
            ),
            AgentMessage::Llm(Message::Assistant(_)) => "assistant".into(),
            AgentMessage::BranchSummary(b) => format!("branch_summary:{}", b.summary),
            AgentMessage::CompactionSummary(c) => format!("compaction:{}", c.summary),
            _ => "other".into(),
        })
        .collect()
}

#[test]
fn phase13_resume_and_fork_use_context_builder() {
    use opi_agent::session_context::reconstruct_context;

    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());
    let source_path = write_phase13_branch_summary_session(sessions.path(), "bs-session");

    // Resume reads the source; reconstruct via the opi-agent context API.
    let resumed =
        opi_coding_agent::session_cli::resume_session(sessions.path(), "bs-session").unwrap();
    let resumed_fp =
        fingerprint_messages(&reconstruct_context(&resumed.entries, &resumed.recovery).messages);

    // Fork copies the same reconstructed active branch that resume would use.
    let forked =
        opi_coding_agent::session_cli::fork_session(sessions.path(), "bs-session").unwrap();
    let forked_fp =
        fingerprint_messages(&reconstruct_context(&forked.entries, &forked.recovery).messages);

    // Both observe: root message, branch_summary injected after its parent,
    // then the child message. branch_summary enters LLM context at the
    // documented position.
    assert_eq!(
        resumed_fp,
        vec![
            "user:root".to_owned(),
            "branch_summary:branch summary text".to_owned(),
            "user:child".to_owned(),
        ],
        "resume must route through the opi-agent context builder"
    );
    assert_eq!(
        resumed_fp, forked_fp,
        "resume and fork must observe identical reconstructed context"
    );

    // Fork leaves the source session unchanged.
    let (_h, entries_after) = SessionReader::read_all(&source_path).unwrap();
    assert_eq!(
        entries_after.len(),
        4,
        "fork must not modify the source session (m1 + bs1 + m2 + leaf)"
    );

    clear_sessions_dir();
}

// ---------------------------------------------------------------------------
// Phase 17.8: resume / fork / branch reconstruction preserve the canonical
// route binding (P17-A13). A recorded route is re-applied from the chain, not
// guessed from the active provider or dropped on fork/branch switches.
// ---------------------------------------------------------------------------

fn phase17_route_model_info(id: &str) -> opi_ai::provider::ModelInfo {
    opi_ai::provider::ModelInfo::new(
        id,
        id,
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(100_000, 4_096),
    )
}

/// Write a two-branch session: root `msg-1` forks into branch A (`msg-2a`,
/// recording `mock:alpha-model`) and branch B (`msg-2b`, recording
/// `mock:beta-model`). A `Leaf` selects branch A as active. The mock provider
/// owns both models so both routes resolve.
fn write_two_branch_route_session(dir: &std::path::Path, session_id: &str) {
    use opi_agent::session::{LeafEntry, ModelChangeEntry};
    let path = dir.join(format!("{session_id}.jsonl"));
    let header = SessionHeader::new_for_test(
        session_id.into(),
        "2026-08-14T12:00:00Z".into(),
        "/repo".into(),
        None,
    );
    let mut writer = SessionWriter::create(&path, header).unwrap();
    let user = |id: &str, parent: Option<&str>, text: &str| {
        SessionEntry::Message(MessageEntry {
            id: id.into(),
            parent_id: parent.map(|p| p.to_owned()),
            timestamp: "2026-08-14T12:00:01Z".into(),
            message: Message::User(UserMessage {
                content: vec![InputContent::Text { text: text.into() }],
                timestamp_ms: 0,
            }),
        })
    };
    let model = |id: &str, parent: &str, spec: &str| {
        SessionEntry::ModelChange(ModelChangeEntry {
            id: id.into(),
            parent_id: Some(parent.into()),
            timestamp: "2026-08-14T12:00:02Z".into(),
            model: spec.into(),
            input_source: None,
        })
    };
    writer.append(&user("msg-1", None, "root")).unwrap();
    writer
        .append(&user("msg-2a", Some("msg-1"), "branch a"))
        .unwrap();
    writer
        .append(&model("model-a", "msg-2a", "mock:alpha-model"))
        .unwrap();
    writer
        .append(&user("msg-2b", Some("msg-1"), "branch b"))
        .unwrap();
    writer
        .append(&model("model-b", "msg-2b", "mock:beta-model"))
        .unwrap();
    writer
        .append(&SessionEntry::Leaf(LeafEntry {
            id: "leaf-a".into(),
            parent_id: Some("msg-2a".into()),
            timestamp: "2026-08-14T12:00:03Z".into(),
            entry_id: "msg-2a".into(),
        }))
        .unwrap();
}

#[test]
fn phase17_resume_fork_branch_preserve_canonical_route_binding() {
    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());
    write_two_branch_route_session(sessions.path(), "route-binding");

    let workspace = tempfile::tempdir().unwrap();
    let provider = MockProvider::new_with_models(
        "mock",
        vec![
            phase17_route_model_info("alpha-model"),
            phase17_route_model_info("beta-model"),
        ],
        Vec::new(),
    );
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:alpha-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .build();

    // Resume loads branch A, whose chain records mock:alpha-model.
    harness
        .resume_session_id("route-binding")
        .expect("resume succeeds");
    assert_eq!(
        harness.model_spec(),
        "mock:alpha-model",
        "resume re-applies the active branch's recorded route"
    );

    // Switching to branch B rebinds to that branch's recorded route. Before
    // Phase 17.8 the branch-tip path did not re-apply the recorded model, so the
    // route would have stayed mock:alpha-model.
    harness
        .resume_session_branch_tip("msg-2b")
        .expect("branch tip exists");
    assert_eq!(
        harness.model_spec(),
        "mock:beta-model",
        "branch-tip rebinds to the selected branch's recorded route"
    );

    // Divert the live model away from branch B's chain. `set_model` does not
    // persist a model_change, so the session file still records branch B's route.
    harness.set_model("mock:alpha-model".into());
    assert_eq!(harness.model_spec(), "mock:alpha-model");

    // Fork re-applies the active branch B route from the chain, not the diverted
    // live model. Before Phase 17.8 fork did not re-apply the recorded model.
    harness.fork_current_session().expect("fork succeeds");
    assert_eq!(
        harness.model_spec(),
        "mock:beta-model",
        "fork re-applies the recorded route from the forked chain"
    );

    clear_sessions_dir();
}

// ---------------------------------------------------------------------------
// P17-NXT-005 (phase-exit closure) — product compaction replaces the complete
// provider-visible context through the same atomic transition: after threshold
// compaction, the next provider request drops superseded messages and keeps
// the current turn (no append-only residue).
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn phase17_compaction_replaces_provider_visible_context() {
    let _lock = session_lock();
    let dir = tempfile::tempdir().unwrap();
    set_sessions_dir(dir.path());

    // Threshold 0 so every turn end compacts.
    let mut config = OpiConfig::default();
    config.compaction.threshold_tokens = 0;

    let provider = MockProvider::new(
        "mock",
        vec![
            test_support::text_response("ok one"),
            test_support::text_response("ok two"),
            test_support::text_response("ok three"),
            test_support::text_response("ok four"),
        ],
    );
    let calls = provider.call_log_handle();
    let mut harness = CodingHarness::new(
        Box::new(provider),
        "mock-model".into(),
        config,
        std::env::current_dir().unwrap(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    );
    let compactions = Arc::new(std::sync::Mutex::new(0u32));
    let compactions_c = compactions.clone();
    harness.subscribe(Box::new(move |event| {
        if let opi_agent::event::AgentEvent::CompactionEnd { .. } = event {
            *compactions_c.lock().unwrap() += 1;
        }
    }));

    // Four turns with distinct markers: by the fourth provider call, the first
    // turn's marker must have been compacted away (superseded), while the
    // current turn's marker is present.
    harness
        .prompt("superseded-marker-one please")
        .await
        .unwrap();
    harness
        .prompt("superseded-marker-two please")
        .await
        .unwrap();
    harness
        .prompt("superseded-marker-three please")
        .await
        .unwrap();
    harness.prompt("current-marker-four please").await.unwrap();

    assert!(
        *compactions.lock().unwrap() >= 1,
        "threshold compaction ran during the run"
    );
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 4, "four provider dispatches");
    let request_text = |idx: usize| -> String {
        use opi_ai::message::{AssistantContent, InputContent, Message};
        let mut out = String::new();
        for message in &calls[idx].messages {
            match message {
                Message::User(u) => {
                    for c in &u.content {
                        if let InputContent::Text { text } = c {
                            out.push_str(text);
                        }
                    }
                }
                Message::Assistant(a) => {
                    for c in &a.content {
                        if let AssistantContent::Text { text } = c {
                            out.push_str(text);
                        }
                    }
                }
                _ => {}
            }
        }
        out
    };
    let first_request = request_text(0);
    assert!(
        first_request.contains("superseded-marker-one"),
        "the first request carries the first turn's marker (pre-compaction)"
    );
    // The post-compaction request is the REPLACED context: a summary message
    // leading the buffer, the kept tail, and the current turn — never the
    // appended history. The message count stays bounded instead of growing
    // with turn count, and no standalone superseded message object survives
    // (its content may legitimately be referenced inside the summary text).
    let last = &calls[3];
    assert_eq!(
        last.messages.len(),
        3,
        "the replaced context is [summary, kept tail, current turn], not the appended history"
    );
    let summary_text = request_text(3);
    assert!(
        summary_text.contains("[Context was compacted."),
        "the replaced context leads with the compaction summary"
    );
    assert!(
        summary_text.contains("current-marker-four"),
        "the post-compaction request carries the current turn's marker"
    );
    let standalone_superseded = last
        .messages
        .iter()
        .any(|m| match m {
            opi_ai::message::Message::User(u) => u.content.iter().any(
                |c| matches!(c, opi_ai::message::InputContent::Text { text } if text == "superseded-marker-one please"),
            ),
            opi_ai::message::Message::Assistant(a) => a.content.iter().any(
                |c| matches!(c, opi_ai::message::AssistantContent::Text { text } if text == "superseded-marker-one please"),
            ),
            _ => false,
        });
    assert!(
        !standalone_superseded,
        "the superseded first-turn message object is dropped (complete replacement, not append-only)"
    );
    clear_sessions_dir();
}
