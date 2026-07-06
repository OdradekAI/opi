//! Session v1 JSONL storage fixture tests (task 2.6).
//!
//! Verifies: AgentMessage serde with new variants, AgentSessionEvent serde,
//! session header parsing, tree entry round-trip, crash recovery,
//! JSONL append-only writer/reader, and versioned envelope.

use std::io::Write;

use opi_agent::AgentEvent;
use opi_agent::message::AgentMessage;
use opi_agent::session::{
    BranchSummaryEntry, CompactionEntry, ExtensionStateEntry, FORMAT_VERSION, LabelAction,
    LabelEntry, LeafEntry, MessageEntry, ModelChangeEntry, SessionEntry, SessionHeader,
    SessionInfoEntry, SessionReader, SessionWriter, ThinkingLevelChangeEntry,
};
use opi_agent::session_event::{
    AgentSessionEvent, CompactionReason, CompactionResult, ThinkingLevel,
};
use opi_ai::message::{InputContent, Message, UserMessage};

// ---------------------------------------------------------------------------
// AgentMessage serde — new variants
// ---------------------------------------------------------------------------

#[test]
fn agent_message_llm_round_trip() {
    let msg = AgentMessage::Llm(Message::User(UserMessage {
        content: vec![InputContent::Text {
            text: "Hello".into(),
        }],
        timestamp_ms: 1000,
    }));
    let json = serde_json::to_string(&msg).unwrap();
    let back: AgentMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, AgentMessage::Llm(Message::User(_))));
}

#[test]
fn agent_message_compaction_summary_round_trip() {
    let msg = AgentMessage::CompactionSummary(opi_agent::message::CompactionSummaryMessage {
        summary: "Session discussed CLI scaffolding.".into(),
        first_kept_entry_id: "entry-42".into(),
        tokens_before: 45000,
        tokens_after: 8000,
    });
    let json = serde_json::to_string(&msg).unwrap();
    let back: AgentMessage = serde_json::from_str(&json).unwrap();
    if let AgentMessage::CompactionSummary(c) = &back {
        assert_eq!(c.summary, "Session discussed CLI scaffolding.");
        assert_eq!(c.first_kept_entry_id, "entry-42");
        assert_eq!(c.tokens_before, 45000);
        assert_eq!(c.tokens_after, 8000);
    } else {
        panic!("expected CompactionSummary variant, got {back:?}");
    }
}

#[test]
fn agent_message_branch_summary_round_trip() {
    let msg = AgentMessage::BranchSummary(opi_agent::message::BranchSummaryMessage {
        parent_session_id: "parent-123".into(),
        summary: "Branch explored error handling.".into(),
        entry_count: 15,
    });
    let json = serde_json::to_string(&msg).unwrap();
    let back: AgentMessage = serde_json::from_str(&json).unwrap();
    if let AgentMessage::BranchSummary(b) = &back {
        assert_eq!(b.parent_session_id, "parent-123");
        assert_eq!(b.summary, "Branch explored error handling.");
        assert_eq!(b.entry_count, 15);
    } else {
        panic!("expected BranchSummary variant, got {back:?}");
    }
}

#[test]
fn agent_message_custom_round_trip() {
    let msg = AgentMessage::Custom(opi_agent::message::CustomAgentMessage {
        kind: "my_extension".into(),
        data: serde_json::json!({"key": "value"}),
        include_in_llm_context: true,
    });
    let json = serde_json::to_string(&msg).unwrap();
    let back: AgentMessage = serde_json::from_str(&json).unwrap();
    if let AgentMessage::Custom(c) = &back {
        assert_eq!(c.kind, "my_extension");
        assert_eq!(c.data["key"], "value");
        assert!(c.include_in_llm_context);
    } else {
        panic!("expected Custom variant, got {back:?}");
    }
}

// ---------------------------------------------------------------------------
// AgentSessionEvent serde
// ---------------------------------------------------------------------------

#[test]
fn session_event_agent_round_trip() {
    let event = AgentSessionEvent::Agent {
        event: AgentEvent::AgentStart,
    };
    let json = serde_json::to_string(&event).unwrap();
    let back: AgentSessionEvent = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        back,
        AgentSessionEvent::Agent {
            event: AgentEvent::AgentStart
        }
    ));
}

#[test]
fn session_event_queue_update_round_trip() {
    let event = AgentSessionEvent::QueueUpdate {
        steering: vec!["steer1".into()],
        follow_up: vec!["follow1".into()],
    };
    let json = serde_json::to_string(&event).unwrap();
    let back: AgentSessionEvent = serde_json::from_str(&json).unwrap();
    if let AgentSessionEvent::QueueUpdate {
        steering,
        follow_up,
    } = &back
    {
        assert_eq!(steering.len(), 1);
        assert_eq!(follow_up.len(), 1);
    } else {
        panic!("expected QueueUpdate variant");
    }
}

#[test]
fn session_event_compaction_start_round_trip() {
    let event = AgentSessionEvent::CompactionStart {
        reason: CompactionReason::Threshold,
    };
    let json = serde_json::to_string(&event).unwrap();
    let back: AgentSessionEvent = serde_json::from_str(&json).unwrap();
    if let AgentSessionEvent::CompactionStart { reason } = &back {
        assert_eq!(*reason, CompactionReason::Threshold);
    } else {
        panic!("expected CompactionStart variant");
    }
}

#[test]
fn session_event_compaction_end_round_trip() {
    let event = AgentSessionEvent::CompactionEnd {
        reason: CompactionReason::Overflow,
        result: Some(CompactionResult {
            summary: "Compacted.".into(),
            first_kept_entry_id: "e10".into(),
            tokens_before: 50000,
            tokens_after: 10000,
        }),
        aborted: false,
        will_retry: false,
        error_message: None,
    };
    let json = serde_json::to_string(&event).unwrap();
    let back: AgentSessionEvent = serde_json::from_str(&json).unwrap();
    if let AgentSessionEvent::CompactionEnd { reason, result, .. } = &back {
        assert_eq!(*reason, CompactionReason::Overflow);
        assert!(result.is_some());
        assert_eq!(result.as_ref().unwrap().tokens_before, 50000);
    } else {
        panic!("expected CompactionEnd variant");
    }
}

#[test]
fn session_event_auto_retry_round_trip() {
    let start = AgentSessionEvent::AutoRetryStart {
        attempt: 2,
        max_attempts: 3,
        delay_ms: 1000,
        error_message: "rate limited".into(),
    };
    let json = serde_json::to_string(&start).unwrap();
    let back: AgentSessionEvent = serde_json::from_str(&json).unwrap();
    if let AgentSessionEvent::AutoRetryStart {
        attempt,
        max_attempts,
        ..
    } = &back
    {
        assert_eq!(*attempt, 2);
        assert_eq!(*max_attempts, 3);
    } else {
        panic!("expected AutoRetryStart variant");
    }

    let end = AgentSessionEvent::AutoRetryEnd {
        success: true,
        attempt: 3,
        final_error: None,
    };
    let json2 = serde_json::to_string(&end).unwrap();
    let back2: AgentSessionEvent = serde_json::from_str(&json2).unwrap();
    assert!(matches!(
        back2,
        AgentSessionEvent::AutoRetryEnd { success: true, .. }
    ));
}

#[test]
fn session_event_session_info_changed_round_trip() {
    let event = AgentSessionEvent::SessionInfoChanged {
        session_id: "sess-1".into(),
        name: Some("my session".into()),
    };
    let json = serde_json::to_string(&event).unwrap();
    let back: AgentSessionEvent = serde_json::from_str(&json).unwrap();
    if let AgentSessionEvent::SessionInfoChanged { session_id, name } = &back {
        assert_eq!(session_id, "sess-1");
        assert_eq!(name.as_deref(), Some("my session"));
    } else {
        panic!("expected SessionInfoChanged variant");
    }
}

#[test]
fn session_event_thinking_level_changed_round_trip() {
    for level in [
        ThinkingLevel::None,
        ThinkingLevel::Low,
        ThinkingLevel::Medium,
        ThinkingLevel::High,
    ] {
        let event = AgentSessionEvent::ThinkingLevelChanged { level };
        let json = serde_json::to_string(&event).unwrap();
        let back: AgentSessionEvent = serde_json::from_str(&json).unwrap();
        if let AgentSessionEvent::ThinkingLevelChanged { level: l } = &back {
            assert_eq!(*l, level);
        } else {
            panic!("expected ThinkingLevelChanged variant");
        }
    }
}

// ---------------------------------------------------------------------------
// CompactionReason and ThinkingLevel exhaustive serde
// ---------------------------------------------------------------------------

#[test]
fn compaction_reason_all_variants_round_trip() {
    for reason in [
        CompactionReason::Manual,
        CompactionReason::Threshold,
        CompactionReason::Overflow,
    ] {
        let json = serde_json::to_string(&reason).unwrap();
        let back: CompactionReason = serde_json::from_str(&json).unwrap();
        assert_eq!(back, reason);
    }
}

// ---------------------------------------------------------------------------
// SessionHeader
// ---------------------------------------------------------------------------

fn make_header(id: &str) -> SessionHeader {
    SessionHeader::new(
        id.into(),
        "2026-05-22T12:00:00Z".into(),
        "/repo".into(),
        None,
    )
}

fn make_header_with_parent(id: &str, parent: &str) -> SessionHeader {
    SessionHeader::new(
        id.into(),
        "2026-05-22T12:00:00Z".into(),
        "/repo".into(),
        Some(parent.into()),
    )
}

#[test]
fn session_header_round_trip() {
    let header = make_header("018f-abc");
    let json = serde_json::to_string(&header).unwrap();
    let back: SessionHeader = serde_json::from_str(&json).unwrap();
    assert_eq!(back.version, 1);
    assert_eq!(back.id, "018f-abc");
    assert_eq!(back.cwd, "/repo");
    assert!(back.parent_session.is_none());
}

#[test]
fn session_header_serializes_with_type_field() {
    let header = make_header_with_parent("018f-abc", "parent-sess");
    let val: serde_json::Value = serde_json::to_value(&header).unwrap();
    assert_eq!(val["type"], "session");
    assert_eq!(val["version"], 1);
    assert_eq!(val["parent_session"], "parent-sess");
}

// ---------------------------------------------------------------------------
// SessionEntry tree entries
// ---------------------------------------------------------------------------

#[test]
fn message_entry_round_trip() {
    let entry = SessionEntry::Message(MessageEntry {
        id: "a1b2".into(),
        parent_id: None,
        timestamp: "2026-05-22T12:00:01Z".into(),
        message: Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "Read src/main.rs".into(),
            }],
            timestamp_ms: 1000,
        }),
    });
    let json = serde_json::to_string(&entry).unwrap();
    let val: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(val["type"], "message");
    let back: SessionEntry = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, SessionEntry::Message(_)));
}

#[test]
fn compaction_entry_round_trip() {
    let entry = SessionEntry::Compaction(CompactionEntry {
        id: "c3d4".into(),
        parent_id: Some("b2c3".into()),
        timestamp: "2026-05-22T13:00:00Z".into(),
        summary: "The session inspected CLI scaffolding.".into(),
        first_kept_entry_id: "b2c3".into(),
        tokens_before: 45000,
        tokens_after: 8000,
    });
    let json = serde_json::to_string(&entry).unwrap();
    let val: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(val["type"], "compaction");
    let back: SessionEntry = serde_json::from_str(&json).unwrap();
    if let SessionEntry::Compaction(c) = &back {
        assert_eq!(c.summary, "The session inspected CLI scaffolding.");
        assert_eq!(c.tokens_before, 45000);
    } else {
        panic!("expected Compaction entry");
    }
}

#[test]
fn leaf_entry_round_trip() {
    let entry = SessionEntry::Leaf(LeafEntry {
        id: "leaf-1".into(),
        parent_id: Some("entry-5".into()),
        timestamp: "2026-05-22T14:00:00Z".into(),
        entry_id: "entry-5".into(),
    });
    let json = serde_json::to_string(&entry).unwrap();
    let val: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(val["type"], "leaf");
    let back: SessionEntry = serde_json::from_str(&json).unwrap();
    if let SessionEntry::Leaf(l) = &back {
        assert_eq!(l.entry_id, "entry-5");
    } else {
        panic!("expected Leaf entry");
    }
}

#[test]
fn extension_state_entry_round_trip() {
    let entry = SessionEntry::ExtensionState(ExtensionStateEntry {
        id: "state-1".into(),
        parent_id: Some("msg-1".into()),
        timestamp: "2026-05-22T14:00:01Z".into(),
        state: serde_json::json!({"todo": {"items": []}}),
    });
    let json = serde_json::to_string(&entry).unwrap();
    let val: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(val["type"], "extension_state");

    let back: SessionEntry = serde_json::from_str(&json).unwrap();
    match &back {
        SessionEntry::ExtensionState(state) => {
            assert_eq!(state.parent_id.as_deref(), Some("msg-1"));
            assert_eq!(state.state["todo"]["items"], serde_json::json!([]));
        }
        other => panic!("expected extension state entry, got {other:?}"),
    }
}

#[test]
fn session_jsonl_round_trips_extension_state_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.jsonl");
    let header = make_header("sess-state");

    let mut writer = SessionWriter::create(&path, header).unwrap();
    writer
        .append(&SessionEntry::ExtensionState(ExtensionStateEntry {
            id: "state-1".to_string(),
            parent_id: Some("msg-1".to_string()),
            timestamp: "2026-05-22T14:00:01Z".to_string(),
            state: serde_json::json!({"todo": {"items": []}}),
        }))
        .unwrap();
    drop(writer);

    let (_header, entries) = SessionReader::read_all(&path).unwrap();
    assert_eq!(entries.len(), 1);
    match &entries[0] {
        SessionEntry::ExtensionState(entry) => {
            assert_eq!(entry.parent_id.as_deref(), Some("msg-1"));
            assert_eq!(entry.state["todo"]["items"], serde_json::json!([]));
        }
        other => panic!("expected extension state entry, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// JSONL Writer + Reader round-trip
// ---------------------------------------------------------------------------

#[test]
fn jsonl_write_and_read_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");

    let header = make_header("sess-001");

    {
        let mut writer = SessionWriter::create(&path, header.clone()).unwrap();
        writer
            .append(&SessionEntry::Message(MessageEntry {
                id: "e1".into(),
                parent_id: None,
                timestamp: "2026-05-22T12:00:01Z".into(),
                message: Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "Hello".into(),
                    }],
                    timestamp_ms: 1000,
                }),
            }))
            .unwrap();
        writer
            .append(&SessionEntry::Message(MessageEntry {
                id: "e2".into(),
                parent_id: Some("e1".into()),
                timestamp: "2026-05-22T12:00:02Z".into(),
                message: Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "World".into(),
                    }],
                    timestamp_ms: 2000,
                }),
            }))
            .unwrap();
    }

    let (read_header, entries) = SessionReader::read_all(&path).unwrap();
    assert_eq!(read_header.version, 1);
    assert_eq!(read_header.id, "sess-001");
    assert_eq!(entries.len(), 2);

    if let SessionEntry::Message(m) = &entries[0] {
        assert_eq!(m.id, "e1");
        assert!(m.parent_id.is_none());
    } else {
        panic!("first entry should be a Message");
    }
}

// ---------------------------------------------------------------------------
// Crash recovery — incomplete final line
// ---------------------------------------------------------------------------

/// Helper: create a minimal message entry for use in crash recovery tests.
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

#[test]
fn crash_recovery_skips_incomplete_final_line() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("crash.jsonl");

    // Write valid session using the writer, then append an incomplete line.
    {
        let mut writer = SessionWriter::create(&path, make_header("crash-1")).unwrap();
        writer.append(&test_message_entry("e1", "Hi")).unwrap();
    }
    // Append incomplete line simulating a crash.
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        write!(f, "{{\"type\":\"message\",\"id\":\"e2").unwrap(); // no newline, incomplete
    }

    let (_read_header, entries, recovery) = SessionReader::read_with_recovery(&path).unwrap();
    assert_eq!(entries.len(), 1, "should have one valid entry");
    assert!(
        recovery.truncated_line,
        "should report truncated final line, got {recovery:?}"
    );
    assert_eq!(recovery.corrupt_count, 0);
    assert_eq!(recovery.unknown_count, 0);
}

// ---------------------------------------------------------------------------
// Crash recovery — corrupt middle entry
// ---------------------------------------------------------------------------

#[test]
fn crash_recovery_reports_corrupt_middle_entry() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("corrupt.jsonl");

    // Write a valid session, then inject corrupt lines by rewriting.
    let header = make_header("corrupt-1");
    let entry1 = test_message_entry("e1", "Hi");
    let entry3 = test_message_entry("e3", "Bye");
    let entry1_json = serde_json::to_string(&entry1).unwrap();
    let entry3_json = serde_json::to_string(&entry3).unwrap();
    let header_json = serde_json::to_string(&header).unwrap();

    {
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "{header_json}").unwrap();
        writeln!(f, "{entry1_json}").unwrap();
        writeln!(f, "NOT VALID JSON").unwrap();
        writeln!(f, "{entry3_json}").unwrap();
    }

    let (_read_header, entries, recovery) = SessionReader::read_with_recovery(&path).unwrap();
    assert_eq!(entries.len(), 2, "should have 2 valid entries");
    assert_eq!(
        recovery.corrupt_count, 1,
        "should report 1 corrupt entry, got {recovery:?}"
    );
    assert!(!recovery.truncated_line);
    assert_eq!(recovery.unknown_count, 0);
}

// ---------------------------------------------------------------------------
// Writer appends to existing session
// ---------------------------------------------------------------------------

#[test]
fn writer_appends_to_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("append.jsonl");

    let header = make_header("append-1");

    // First write
    {
        let mut writer = SessionWriter::create(&path, header.clone()).unwrap();
        writer
            .append(&SessionEntry::Message(MessageEntry {
                id: "e1".into(),
                parent_id: None,
                timestamp: "2026-05-22T12:00:01Z".into(),
                message: Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "First".into(),
                    }],
                    timestamp_ms: 0,
                }),
            }))
            .unwrap();
    }

    // Append more
    {
        let mut writer = SessionWriter::open(&path).unwrap();
        writer
            .append(&SessionEntry::Message(MessageEntry {
                id: "e2".into(),
                parent_id: Some("e1".into()),
                timestamp: "2026-05-22T12:00:02Z".into(),
                message: Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "Second".into(),
                    }],
                    timestamp_ms: 0,
                }),
            }))
            .unwrap();
    }

    let (_, entries) = SessionReader::read_all(&path).unwrap();
    assert_eq!(entries.len(), 2);
}

// ---------------------------------------------------------------------------
// Crash recovery — open after incomplete tail, then append + read_all
// ---------------------------------------------------------------------------

#[test]
fn writer_truncates_incomplete_tail_preserving_trailing_newline() {
    // Simulate a crash: valid entries followed by an incomplete JSON fragment.
    // SessionWriter::open must truncate the incomplete line *without* removing
    // the trailing newline of the last valid entry. Then appending a new entry
    // must produce valid JSONL (each entry on its own line).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tail-truncation.jsonl");

    // Write a valid session with two entries.
    {
        let mut writer = SessionWriter::create(&path, make_header("tail-test")).unwrap();
        writer.append(&test_message_entry("e1", "first")).unwrap();
        writer.append(&test_message_entry("e2", "second")).unwrap();
    }

    // Simulate crash: append an incomplete line (no trailing newline).
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        // Intentionally incomplete JSON, no newline.
        write!(f, "{{\"type\":\"message\",\"id\":\"e3\"").unwrap();
    }

    // Open with SessionWriter — should truncate the incomplete tail.
    {
        let mut writer = SessionWriter::open(&path).unwrap();
        writer.append(&test_message_entry("e3", "third")).unwrap();
    }

    let (_, entries) = SessionReader::read_all(&path).unwrap();
    assert_eq!(
        entries.len(),
        3,
        "should have 3 entries after truncating incomplete tail + appending"
    );
    // Verify the last entry is the one we appended after recovery.
    if let SessionEntry::Message(m) = &entries[2] {
        assert_eq!(m.id, "e3");
    } else {
        panic!("expected Message entry at index 2");
    }
}

#[test]
fn writer_truncates_all_when_no_newline_in_file() {
    // Edge case: a file whose only content is an incomplete line (no newline
    // at all). SessionWriter::open should truncate to empty, leaving only the
    // header line from the initial create.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("no-newline.jsonl");

    // Write header + one valid entry.
    {
        let mut writer = SessionWriter::create(&path, make_header("no-nl")).unwrap();
        writer.append(&test_message_entry("e1", "only")).unwrap();
    }

    // Overwrite the entire file content with an incomplete JSON (no newlines).
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&path).unwrap();
        write!(f, "GARBAGE_NO_NEWLINES").unwrap();
    }

    // This is a degenerate case — the file no longer has a valid header.
    // SessionWriter::open still opens it for append; the truncation logic
    // should handle "no newline found" by truncating to 0.
    let mut writer = SessionWriter::open(&path).unwrap();
    // After truncation to 0, appending writes a valid JSONL line.
    writer
        .append(&test_message_entry("e-new", "post-recovery"))
        .unwrap();

    // The file is no longer a valid session (header was destroyed), but the
    // new entry should be on its own line.
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(
        content.ends_with('\n'),
        "file should end with a newline after append"
    );
}

// ---------------------------------------------------------------------------
// Phase 13.1: additive session entries — JSON shape + round trip
// ---------------------------------------------------------------------------

#[test]
fn session_info_entry_round_trip_and_shape() {
    let entry = SessionEntry::SessionInfo(SessionInfoEntry {
        id: "si-1".into(),
        parent_id: Some("msg-1".into()),
        timestamp: "2026-07-05T12:00:00Z".into(),
        name: "my session".into(),
    });
    let json = serde_json::to_string(&entry).unwrap();
    let val: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(val["type"], "session_info");
    assert_eq!(val["name"], "my session");
    let back: SessionEntry = serde_json::from_str(&json).unwrap();
    if let SessionEntry::SessionInfo(s) = &back {
        assert_eq!(s.name, "my session");
        assert_eq!(s.parent_id.as_deref(), Some("msg-1"));
    } else {
        panic!("expected SessionInfo entry, got {back:?}");
    }
}

#[test]
fn model_change_entry_round_trip_and_shape() {
    let entry = SessionEntry::ModelChange(ModelChangeEntry {
        id: "mc-1".into(),
        parent_id: Some("msg-1".into()),
        timestamp: "2026-07-05T12:00:01Z".into(),
        model: "anthropic:claude-sonnet-4-5".into(),
    });
    let json = serde_json::to_string(&entry).unwrap();
    let val: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(val["type"], "model_change");
    assert_eq!(val["model"], "anthropic:claude-sonnet-4-5");
    let back: SessionEntry = serde_json::from_str(&json).unwrap();
    if let SessionEntry::ModelChange(m) = &back {
        assert_eq!(m.model, "anthropic:claude-sonnet-4-5");
    } else {
        panic!("expected ModelChange entry, got {back:?}");
    }
}

#[test]
fn thinking_level_change_entry_round_trip_and_shape() {
    for level in [
        ThinkingLevel::None,
        ThinkingLevel::Low,
        ThinkingLevel::Medium,
        ThinkingLevel::High,
    ] {
        let entry = SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
            id: "tl-1".into(),
            parent_id: Some("msg-1".into()),
            timestamp: "2026-07-05T12:00:02Z".into(),
            level,
        });
        let json = serde_json::to_string(&entry).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(val["type"], "thinking_level_change");
        let back: SessionEntry = serde_json::from_str(&json).unwrap();
        if let SessionEntry::ThinkingLevelChange(t) = &back {
            assert_eq!(t.level, level);
        } else {
            panic!("expected ThinkingLevelChange entry, got {back:?}");
        }
    }
}

#[test]
fn label_entry_round_trip_and_shape_for_add_and_remove() {
    for action in [LabelAction::Add, LabelAction::Remove] {
        let entry = SessionEntry::Label(LabelEntry {
            id: "lbl-1".into(),
            parent_id: Some("msg-1".into()),
            timestamp: "2026-07-05T12:00:03Z".into(),
            label: "bug".into(),
            action,
        });
        let json = serde_json::to_string(&entry).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(val["type"], "label");
        assert_eq!(val["label"], "bug");
        let back: SessionEntry = serde_json::from_str(&json).unwrap();
        if let SessionEntry::Label(l) = &back {
            assert_eq!(l.label, "bug");
            assert_eq!(l.action, action);
        } else {
            panic!("expected Label entry, got {back:?}");
        }
    }
}

#[test]
fn branch_summary_entry_round_trip_and_shape() {
    let entry = SessionEntry::BranchSummary(BranchSummaryEntry {
        id: "bs-1".into(),
        parent_id: Some("msg-1".into()),
        timestamp: "2026-07-05T12:00:04Z".into(),
        summary: "Explored session entry model.".into(),
    });
    let json = serde_json::to_string(&entry).unwrap();
    let val: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(val["type"], "branch_summary");
    assert_eq!(val["summary"], "Explored session entry model.");
    let back: SessionEntry = serde_json::from_str(&json).unwrap();
    if let SessionEntry::BranchSummary(b) = &back {
        assert_eq!(b.summary, "Explored session entry model.");
    } else {
        panic!("expected BranchSummary entry, got {back:?}");
    }
}

#[test]
fn known_entry_types_match_session_entry_serde_tags() {
    let fixtures = vec![
        test_message_entry("msg-1", "hello"),
        SessionEntry::Compaction(CompactionEntry {
            id: "cmp-1".into(),
            parent_id: Some("msg-1".into()),
            timestamp: "2026-07-05T12:00:01Z".into(),
            summary: "summary".into(),
            first_kept_entry_id: "msg-1".into(),
            tokens_before: 10,
            tokens_after: 4,
        }),
        SessionEntry::Leaf(LeafEntry {
            id: "leaf-1".into(),
            parent_id: Some("msg-1".into()),
            timestamp: "2026-07-05T12:00:02Z".into(),
            entry_id: "msg-1".into(),
        }),
        SessionEntry::ExtensionState(opi_agent::session::ExtensionStateEntry {
            id: "state-1".into(),
            parent_id: Some("msg-1".into()),
            timestamp: "2026-07-05T12:00:03Z".into(),
            state: serde_json::json!({"ok": true}),
        }),
        SessionEntry::SessionInfo(SessionInfoEntry {
            id: "info-1".into(),
            parent_id: Some("msg-1".into()),
            timestamp: "2026-07-05T12:00:04Z".into(),
            name: "demo".into(),
        }),
        SessionEntry::ModelChange(ModelChangeEntry {
            id: "model-1".into(),
            parent_id: Some("msg-1".into()),
            timestamp: "2026-07-05T12:00:05Z".into(),
            model: "anthropic:claude-sonnet-4".into(),
        }),
        SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
            id: "thinking-1".into(),
            parent_id: Some("msg-1".into()),
            timestamp: "2026-07-05T12:00:06Z".into(),
            level: ThinkingLevel::Low,
        }),
        SessionEntry::Label(LabelEntry {
            id: "label-1".into(),
            parent_id: Some("msg-1".into()),
            timestamp: "2026-07-05T12:00:07Z".into(),
            label: "bug".into(),
            action: LabelAction::Add,
        }),
        SessionEntry::BranchSummary(BranchSummaryEntry {
            id: "branch-1".into(),
            parent_id: Some("msg-1".into()),
            timestamp: "2026-07-05T12:00:08Z".into(),
            summary: "branch summary".into(),
        }),
    ];

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("known-tags.jsonl");
    {
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(
            file,
            "{}",
            serde_json::to_string(&make_header("known-tags")).unwrap()
        )
        .unwrap();
        for fixture in &fixtures {
            let value = serde_json::to_value(fixture).unwrap();
            let tag = value["type"].as_str().expect("serde tag").to_owned();
            // The type tag is known, but the payload is intentionally malformed.
            // If KNOWN_ENTRY_TYPES omits the tag, this lands in unknown_count
            // instead of corrupt_count.
            writeln!(file, "{}", serde_json::json!({ "type": tag })).unwrap();
        }
    }

    let (_header, entries, recovery) = SessionReader::read_with_recovery(&path).unwrap();
    assert!(entries.is_empty(), "malformed entries must be skipped");
    assert_eq!(recovery.corrupt_count, fixtures.len());
    assert_eq!(recovery.unknown_count, 0);
}

// ---------------------------------------------------------------------------
// Phase 13.1: forward-compat unknown-entry behavior (distinct from corruption)
// ---------------------------------------------------------------------------

#[test]
fn crash_recovery_reports_unknown_future_type_separately_from_corrupt() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("future.jsonl");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            "{}",
            serde_json::to_string(&make_header("future-1")).unwrap()
        )
        .unwrap();
        writeln!(
            f,
            "{}",
            serde_json::to_string(&test_message_entry("e1", "real")).unwrap()
        )
        .unwrap();
        // Valid JSON object with a type tag this build does not recognize.
        // Stands in for any additive entry a future opi might introduce.
        writeln!(
            f,
            r#"{{"type":"some_future_entry_type","id":"x1","parent_id":"e1","timestamp":"2026-07-05T12:00:00Z"}}"#
        )
        .unwrap();
    }

    let (_header, entries, recovery) = SessionReader::read_with_recovery(&path).unwrap();
    assert_eq!(entries.len(), 1, "known entry survives; unknown skipped");
    assert_eq!(
        recovery.unknown_count, 1,
        "unknown future entry counted separately, got {recovery:?}"
    );
    assert_eq!(
        recovery.corrupt_count, 0,
        "unknown must not be counted as corrupt"
    );
    assert!(!recovery.truncated_line);
}

#[test]
fn crash_recovery_unknown_and_corrupt_are_independent_buckets() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mixed.jsonl");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            "{}",
            serde_json::to_string(&make_header("mixed-1")).unwrap()
        )
        .unwrap();
        writeln!(
            f,
            "{}",
            serde_json::to_string(&test_message_entry("e1", "real")).unwrap()
        )
        .unwrap();
        // Unknown future type -> unknown bucket.
        writeln!(
            f,
            r#"{{"type":"another_future_type","id":"x1","parent_id":"e1","timestamp":"t"}}"#
        )
        .unwrap();
        // Genuine corruption -> corrupt bucket.
        writeln!(f, "NOT VALID JSON").unwrap();
    }

    let (_header, entries, recovery) = SessionReader::read_with_recovery(&path).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(recovery.unknown_count, 1);
    assert_eq!(recovery.corrupt_count, 1);
    assert!(!recovery.truncated_line);
}

#[test]
fn crash_recovery_json_object_without_type_field_is_corrupt() {
    // A JSON object with no `type` field is structurally invalid — corrupt,
    // not a forward-compat unknown entry. Pins the doc contract in session.rs.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("no-type.jsonl");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            "{}",
            serde_json::to_string(&make_header("notype-1")).unwrap()
        )
        .unwrap();
        writeln!(
            f,
            "{}",
            serde_json::to_string(&test_message_entry("e1", "real")).unwrap()
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"id":"x1","parent_id":"e1","timestamp":"t","note":"no type here"}}"#
        )
        .unwrap();
    }

    let (_header, entries, recovery) = SessionReader::read_with_recovery(&path).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        recovery.corrupt_count, 1,
        "missing-type object must be corrupt, got {recovery:?}"
    );
    assert_eq!(recovery.unknown_count, 0);
}

// ---------------------------------------------------------------------------
// Phase 13.1: version policy + pi session v3 compatibility disclaimer
// ---------------------------------------------------------------------------

#[test]
fn session_format_version_is_v1_and_policy_disclaims_pi_v3_compatibility() {
    // Phase 13 keeps header version 1 with additive entries; it does not
    // introduce version 2 and does not claim pi session v3 compatibility.
    assert_eq!(FORMAT_VERSION, 1);
    let policy = opi_agent::session::SESSION_FORMAT_POLICY;
    assert!(
        policy.contains("version 1"),
        "policy must state version 1, got: {policy}"
    );
    let lower = policy.to_lowercase();
    assert!(
        lower.contains("pi") && lower.contains("not") && lower.contains("compatib"),
        "policy must disclaim pi compatibility, got: {policy}"
    );
}
