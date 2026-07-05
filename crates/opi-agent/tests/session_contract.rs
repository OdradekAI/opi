//! Session contract tests (task 2.16).
//!
//! DoD: "JSONL round-trip, tree reconstruction, compaction recovery,
//!       property-based tests for session loader"
//!
//! Tests in this file exercise end-to-end session storage contracts:
//! - Full JSONL round-trip with all entry types
//! - Tree reconstruction from parent_id graph
//! - Compaction recovery (entries before/after compaction point)
//! - Property-based tests for session loader invariants

use std::collections::{HashMap, HashSet};

use opi_agent::message::AgentMessage;
use opi_agent::session::{
    BranchSummaryEntry, CompactionEntry, LabelAction, LabelEntry, LeafEntry, MessageEntry,
    ModelChangeEntry, SessionEntry, SessionHeader, SessionInfoEntry, SessionReader, SessionWriter,
    ThinkingLevelChangeEntry,
};
use opi_agent::session_branch::SessionTree;
use opi_agent::session_event::ThinkingLevel;
use opi_ai::message::{AssistantContent, AssistantMessage, InputContent, Message, UserMessage};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_header(id: &str) -> SessionHeader {
    SessionHeader::new(
        id.into(),
        "2026-05-22T12:00:00Z".into(),
        "/repo".into(),
        None,
    )
}

fn user_msg(id: &str, parent: Option<&str>, text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        id: id.into(),
        parent_id: parent.map(|s| s.into()),
        timestamp: "2026-05-22T12:00:01Z".into(),
        message: Message::User(UserMessage {
            content: vec![InputContent::Text { text: text.into() }],
            timestamp_ms: 0,
        }),
    })
}

fn assistant_msg(id: &str, parent: Option<&str>, text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        id: id.into(),
        parent_id: parent.map(|s| s.into()),
        timestamp: "2026-05-22T12:00:02Z".into(),
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

fn compaction_entry(
    id: &str,
    parent: Option<&str>,
    first_kept: &str,
    tokens_before: u64,
    tokens_after: u64,
) -> SessionEntry {
    SessionEntry::Compaction(CompactionEntry {
        id: id.into(),
        parent_id: parent.map(|s| s.into()),
        timestamp: "2026-05-22T13:00:00Z".into(),
        summary: "Compacted earlier messages.".into(),
        first_kept_entry_id: first_kept.into(),
        tokens_before,
        tokens_after,
    })
}

fn leaf_entry(id: &str, parent: Option<&str>, entry_id: &str) -> SessionEntry {
    SessionEntry::Leaf(LeafEntry {
        id: id.into(),
        parent_id: parent.map(|s| s.into()),
        timestamp: "2026-05-22T14:00:00Z".into(),
        entry_id: entry_id.into(),
    })
}

/// Write entries to a JSONL file and read them back.
fn write_and_read(
    header: SessionHeader,
    entries: &[SessionEntry],
) -> (SessionHeader, Vec<SessionEntry>) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("contract.jsonl");
    {
        let mut writer = SessionWriter::create(&path, header.clone()).unwrap();
        for entry in entries {
            writer.append(entry).unwrap();
        }
    }
    SessionReader::read_all(&path).unwrap()
}

fn entry_id(entry: &SessionEntry) -> String {
    match entry {
        SessionEntry::Message(m) => m.id.clone(),
        SessionEntry::Compaction(c) => c.id.clone(),
        SessionEntry::Leaf(l) => l.id.clone(),
        _ => String::new(),
    }
}

fn entry_parent_id(entry: &SessionEntry) -> Option<String> {
    match entry {
        SessionEntry::Message(m) => m.parent_id.clone(),
        SessionEntry::Compaction(c) => c.parent_id.clone(),
        SessionEntry::Leaf(l) => l.parent_id.clone(),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// 1. JSONL round-trip: all entry types
// ---------------------------------------------------------------------------

#[test]
fn jsonl_round_trip_all_entry_types() {
    let header = make_header("rt-001");
    let entries = vec![
        user_msg("e1", None, "Hello"),
        assistant_msg("e2", Some("e1"), "Hi there"),
        user_msg("e3", Some("e2"), "Read file"),
        compaction_entry("c1", Some("e3"), "e3", 5000, 1000),
        leaf_entry("l1", Some("e3"), "e3"),
    ];

    let (read_header, read_entries) = write_and_read(header.clone(), &entries);

    assert_eq!(read_header.id, "rt-001");
    assert_eq!(read_header.version, 1);
    assert_eq!(entries.len(), read_entries.len(), "entry count mismatch");

    // Verify each entry round-trips via serde (re-serialize and compare JSON)
    for (orig, read) in entries.iter().zip(read_entries.iter()) {
        let orig_json = serde_json::to_string(orig).unwrap();
        let read_json = serde_json::to_string(read).unwrap();
        assert_eq!(orig_json, read_json, "entry JSON mismatch");
    }
}

#[test]
fn jsonl_round_trip_preserves_entry_order() {
    let header = make_header("order-001");
    let entries: Vec<SessionEntry> = (0..20)
        .map(|i| user_msg(&format!("e{i}"), None, &format!("msg {i}")))
        .collect();

    let (_, read_entries) = write_and_read(header, &entries);

    for (i, entry) in read_entries.iter().enumerate() {
        if let SessionEntry::Message(m) = entry {
            assert_eq!(m.id, format!("e{i}"), "entry at index {i} has wrong id");
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 13.1: additive metadata/context entries round-trip through JSONL and
// do not create branch nodes in tree reconstruction.
// ---------------------------------------------------------------------------

#[test]
fn phase13_metadata_entries_round_trip_and_do_not_fork_branches() {
    let header = make_header("rt-meta");
    let entries = vec![
        user_msg("e1", None, "Hello"),
        assistant_msg("e2", Some("e1"), "Hi"),
        // Metadata/context entries parented to e2 (the content tip). None may
        // create branch forks or advance the active content tip.
        SessionEntry::SessionInfo(SessionInfoEntry {
            id: "si1".into(),
            parent_id: Some("e2".into()),
            timestamp: "2026-07-05T12:00:00Z".into(),
            name: "named".into(),
        }),
        SessionEntry::ModelChange(ModelChangeEntry {
            id: "mc1".into(),
            parent_id: Some("e2".into()),
            timestamp: "2026-07-05T12:00:01Z".into(),
            model: "anthropic:claude-sonnet-4-5".into(),
        }),
        SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
            id: "tl1".into(),
            parent_id: Some("e2".into()),
            timestamp: "2026-07-05T12:00:02Z".into(),
            level: ThinkingLevel::Medium,
        }),
        SessionEntry::Label(LabelEntry {
            id: "lb1".into(),
            parent_id: Some("e2".into()),
            timestamp: "2026-07-05T12:00:03Z".into(),
            label: "bug".into(),
            action: LabelAction::Add,
        }),
        SessionEntry::BranchSummary(BranchSummaryEntry {
            id: "bs1".into(),
            parent_id: Some("e2".into()),
            timestamp: "2026-07-05T12:00:04Z".into(),
            summary: "summarized the branch".into(),
        }),
        leaf_entry("l1", Some("e2"), "e2"),
    ];

    let (_h, read_entries) = write_and_read(header, &entries);
    assert_eq!(entries.len(), read_entries.len());

    // Byte-identical JSON round-trip for every entry including the new
    // metadata/context variants.
    for (orig, read) in entries.iter().zip(read_entries.iter()) {
        let orig_json = serde_json::to_string(orig).unwrap();
        let read_json = serde_json::to_string(read).unwrap();
        assert_eq!(orig_json, read_json, "entry JSON mismatch");
    }

    // Branch reconstruction: metadata must not fork the graph. The active tip
    // is still the content entry the leaf points at (e2), and the trunk is a
    // single branch.
    let tree = SessionTree::from_entries(&read_entries);
    assert_eq!(tree.active_tip(), Some("e2"));
    assert_eq!(
        tree.branches().len(),
        1,
        "metadata entries must not create extra branches"
    );
}

// ---------------------------------------------------------------------------
// 2. Tree reconstruction
// ---------------------------------------------------------------------------

/// Reconstruct a parent->children tree from session entries.
/// Returns a map of parent_id -> Vec<child_id> and a set of root IDs (no parent).
fn reconstruct_tree(entries: &[SessionEntry]) -> (HashMap<String, Vec<String>>, HashSet<String>) {
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    let mut all_ids: HashSet<String> = HashSet::new();
    let mut has_parent: HashSet<String> = HashSet::new();

    for entry in entries {
        let id = entry_id(entry);
        let parent_id = entry_parent_id(entry);
        all_ids.insert(id.clone());
        if let Some(pid) = parent_id {
            has_parent.insert(id.clone());
            children.entry(pid).or_default().push(id);
        }
    }

    let roots: HashSet<String> = all_ids.difference(&has_parent).cloned().collect();
    (children, roots)
}

#[test]
fn tree_reconstruction_linear_chain() {
    let entries = vec![
        user_msg("e1", None, "start"),
        assistant_msg("e2", Some("e1"), "reply"),
        user_msg("e3", Some("e2"), "follow-up"),
        assistant_msg("e4", Some("e3"), "final"),
    ];

    let (children, roots) = reconstruct_tree(&entries);

    assert_eq!(roots.len(), 1, "should have one root");
    assert!(roots.contains("e1"), "e1 should be the root");

    assert_eq!(children.get("e1").map(|v| v.len()), Some(1));
    assert_eq!(children.get("e2").map(|v| v.len()), Some(1));
    assert_eq!(children.get("e3").map(|v| v.len()), Some(1));
    assert!(!children.contains_key("e4"), "leaf has no children");
}

#[test]
fn tree_reconstruction_branching() {
    let entries = vec![
        user_msg("e1", None, "root"),
        assistant_msg("e2a", Some("e1"), "branch a"),
        assistant_msg("e2b", Some("e1"), "branch b"),
        user_msg("e3a", Some("e2a"), "follow a"),
    ];

    let (children, roots) = reconstruct_tree(&entries);

    assert_eq!(roots.len(), 1);
    assert!(roots.contains("e1"));

    let e1_children = children.get("e1").unwrap();
    assert_eq!(e1_children.len(), 2, "e1 should have two children");
    assert!(e1_children.contains(&"e2a".to_string()));
    assert!(e1_children.contains(&"e2b".to_string()));
}

#[test]
fn tree_reconstruction_from_jsonl() {
    let header = make_header("tree-001");
    let entries = vec![
        user_msg("e1", None, "root"),
        assistant_msg("e2", Some("e1"), "reply"),
        user_msg("e3", Some("e2"), "follow-up"),
    ];

    let (_, read_entries) = write_and_read(header, &entries);
    let (children, roots) = reconstruct_tree(&read_entries);

    assert_eq!(roots.len(), 1);
    assert!(roots.contains("e1"));
    assert_eq!(children.get("e1").unwrap().len(), 1);
}

#[test]
fn tree_reconstruction_with_leaf_pointers() {
    let entries = vec![
        user_msg("e1", None, "start"),
        assistant_msg("e2", Some("e1"), "reply"),
        leaf_entry("l1", Some("e2"), "e2"),
    ];

    let (children, roots) = reconstruct_tree(&entries);
    assert_eq!(roots.len(), 1);
    assert!(roots.contains("e1"));
    assert_eq!(children.get("e2").unwrap().len(), 1, "leaf is child of e2");
}

// ---------------------------------------------------------------------------
// 3. Compaction recovery
// ---------------------------------------------------------------------------

#[test]
fn compaction_recovery_entry_points_to_kept_message() {
    let header = make_header("compact-001");
    let entries = vec![
        user_msg("e1", None, "msg 1"),
        assistant_msg("e2", Some("e1"), "reply 1"),
        user_msg("e3", Some("e2"), "msg 2"),
        assistant_msg("e4", Some("e3"), "reply 2"),
        compaction_entry("c1", Some("e4"), "e3", 4000, 1000),
        user_msg("e5", Some("c1"), "msg after compaction"),
    ];

    let (_, read_entries) = write_and_read(header, &entries);

    // Find the compaction entry
    let compaction = read_entries
        .iter()
        .find_map(|e| match e {
            SessionEntry::Compaction(c) => Some(c.clone()),
            _ => None,
        })
        .expect("should have a compaction entry");

    // first_kept_entry_id should reference an entry in the file
    let ids: HashSet<String> = read_entries.iter().map(entry_id).collect();

    assert!(
        ids.contains(&compaction.first_kept_entry_id),
        "first_kept_entry_id '{}' should reference an existing entry",
        compaction.first_kept_entry_id
    );

    // Entries after compaction should reference the compaction entry
    let post_compaction: Vec<_> = read_entries
        .iter()
        .filter(|e| match e {
            SessionEntry::Message(m) => m.parent_id.as_deref() == Some("c1"),
            _ => false,
        })
        .collect();
    assert_eq!(post_compaction.len(), 1, "one entry after compaction");
}

#[test]
fn compaction_recovery_multiple_compactions() {
    let header = make_header("compact-002");
    let entries = vec![
        user_msg("e1", None, "msg 1"),
        assistant_msg("e2", Some("e1"), "reply 1"),
        compaction_entry("c1", Some("e2"), "e2", 3000, 1000),
        user_msg("e3", Some("c1"), "msg 2"),
        assistant_msg("e4", Some("e3"), "reply 2"),
        compaction_entry("c2", Some("e4"), "e4", 2500, 800),
        user_msg("e5", Some("c2"), "msg 3"),
    ];

    let (_, read_entries) = write_and_read(header, &entries);

    let compactions: Vec<_> = read_entries
        .iter()
        .filter(|e| matches!(e, SessionEntry::Compaction(_)))
        .collect();
    assert_eq!(compactions.len(), 2, "should have two compaction entries");

    // Verify tokens decrease across compactions
    let tokens: Vec<(u64, u64)> = read_entries
        .iter()
        .filter_map(|e| match e {
            SessionEntry::Compaction(c) => Some((c.tokens_before, c.tokens_after)),
            _ => None,
        })
        .collect();
    assert_eq!(tokens.len(), 2);
    assert!(tokens[0].0 > tokens[0].1, "tokens should decrease");
    assert!(tokens[1].0 > tokens[1].1, "tokens should decrease");
}

#[test]
fn compaction_recovery_with_compaction_summary_message() {
    let header = make_header("compact-003");

    // A CompactionSummary as an AgentMessage — verify serde round-trip
    let summary_msg =
        AgentMessage::CompactionSummary(opi_agent::message::CompactionSummaryMessage {
            summary: "Discussed CLI scaffolding.".into(),
            first_kept_entry_id: "e3".into(),
            tokens_before: 5000,
            tokens_after: 1200,
        });

    let entries = vec![
        user_msg("e1", None, "msg 1"),
        user_msg("e2", Some("e1"), "msg 2"),
        SessionEntry::Message(MessageEntry {
            id: "e3".into(),
            parent_id: Some("e2".into()),
            timestamp: "2026-05-22T13:00:00Z".into(),
            message: Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "kept message".into(),
                }],
                timestamp_ms: 0,
            }),
        }),
    ];

    // Round-trip and verify the summary message survives serde
    let summary_json = serde_json::to_string(&summary_msg).unwrap();
    let back: AgentMessage = serde_json::from_str(&summary_json).unwrap();
    if let AgentMessage::CompactionSummary(cs) = &back {
        assert_eq!(cs.first_kept_entry_id, "e3");
        assert_eq!(cs.tokens_before, 5000);
        assert_eq!(cs.tokens_after, 1200);
    } else {
        panic!("expected CompactionSummary");
    }

    // Also verify the regular entries round-trip
    let (_, read_entries) = write_and_read(header, &entries);
    assert_eq!(read_entries.len(), 3);
}

#[test]
fn compaction_recovery_tree_intact_after_reload() {
    let header = make_header("compact-tree");
    let entries = vec![
        user_msg("e1", None, "a"),
        assistant_msg("e2", Some("e1"), "b"),
        user_msg("e3", Some("e2"), "c"),
        compaction_entry("c1", Some("e3"), "e3", 6000, 2000),
        user_msg("e4", Some("c1"), "d"),
        assistant_msg("e5", Some("e4"), "e"),
    ];

    let (_, read_entries) = write_and_read(header, &entries);
    let (children, roots) = reconstruct_tree(&read_entries);

    assert!(roots.contains("e1"), "e1 is root");
    // c1 is child of e3
    assert!(
        children
            .get("e3")
            .is_some_and(|v| v.contains(&"c1".to_string()))
    );
    // e4 is child of c1
    assert!(
        children
            .get("c1")
            .is_some_and(|v| v.contains(&"e4".to_string()))
    );
}

// ---------------------------------------------------------------------------
// 4. Property-based tests: session loader invariants
// ---------------------------------------------------------------------------

fn arb_user_entry(id: String, parent_id: Option<String>, text: String) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        id,
        parent_id,
        timestamp: "2026-05-22T12:00:01Z".into(),
        message: Message::User(UserMessage {
            content: vec![InputContent::Text { text }],
            timestamp_ms: 0,
        }),
    })
}

use proptest::prelude::*;

proptest! {
    /// Any sequence of entries written to JSONL can be read back unchanged.
    #[test]
    fn prop_entries_round_trip(
        texts in proptest::collection::vec(
            proptest::string::string_regex("[a-zA-Z0-9 ]{0,20}").unwrap(),
            0..20
        )
    ) {
        let header = make_header("prop-rt");
        let entries: Vec<SessionEntry> = texts
            .iter()
            .enumerate()
            .map(|(i, text)| arb_user_entry(format!("e{i}"), None, text.clone()))
            .collect();

        let (read_header, read_entries) = write_and_read(header, &entries);

        prop_assert_eq!(read_header.id, "prop-rt");
        prop_assert_eq!(entries.len(), read_entries.len());

        for (orig, read) in entries.iter().zip(read_entries.iter()) {
            let orig_json = serde_json::to_string(orig).unwrap();
            let read_json = serde_json::to_string(read).unwrap();
            prop_assert_eq!(orig_json, read_json);
        }
    }

    /// Header always round-trips through serde unchanged.
    #[test]
    fn prop_header_round_trip(
        id in proptest::string::string_regex("[a-zA-Z0-9_-]{1,20}").unwrap(),
        cwd in proptest::string::string_regex("[a-zA-Z0-9/]{1,20}").unwrap(),
        parent in proptest::option::of(
            proptest::string::string_regex("[a-zA-Z0-9_-]{1,20}").unwrap()
        )
    ) {
        let header = SessionHeader::new(
            id.clone(),
            "2026-05-22T12:00:00Z".into(),
            cwd.clone(),
            parent.clone(),
        );
        let json = serde_json::to_string(&header).unwrap();
        let back: SessionHeader = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(back.id, id);
        prop_assert_eq!(back.cwd, cwd);
        prop_assert_eq!(back.parent_session, parent);
        prop_assert_eq!(back.version, 1);
        prop_assert_eq!(back.type_, "session");
    }

    /// Tree reconstruction: linear chain has exactly one root,
    /// every non-root has a parent, and each non-leaf has exactly one child.
    #[test]
    fn prop_tree_roots_have_no_parent(chain_len in 1usize..20) {
        let entries: Vec<SessionEntry> = (0..chain_len)
            .map(|i| {
                let parent = if i == 0 {
                    None
                } else {
                    Some(format!("e{}", i - 1))
                };
                arb_user_entry(format!("e{i}"), parent, format!("msg {i}"))
            })
            .collect();

        let (children, roots) = reconstruct_tree(&entries);

        prop_assert_eq!(roots.len(), 1, "linear chain has exactly one root");
        prop_assert!(roots.contains("e0"), "e0 is the root");

        for entry in &entries {
            if let SessionEntry::Message(m) = entry
                && m.id != "e0"
            {
                prop_assert!(m.parent_id.is_some(), "{} should have a parent", m.id);
            }
        }

        for i in 0..chain_len.saturating_sub(1) {
            let id = format!("e{i}");
            let child_count = children.get(&id).map(|v| v.len()).unwrap_or(0);
            prop_assert_eq!(child_count, 1, "entry {} should have 1 child", id);
        }
    }

    /// Session header schema invariant: version=1, type="session".
    #[test]
    fn prop_header_schema_invariant(
        id in proptest::string::string_regex("[a-zA-Z0-9_-]{1,10}").unwrap()
    ) {
        let header = make_header(&id);
        let val: serde_json::Value = serde_json::to_value(&header).unwrap();

        prop_assert_eq!(&val["type"], "session");
        prop_assert_eq!(&val["version"], 1);
        prop_assert!(val["id"].is_string());
        prop_assert!(val["timestamp"].is_string());
        prop_assert!(val["cwd"].is_string());
    }

    /// Compaction entry first_kept_entry_id references an existing entry.
    #[test]
    fn prop_compaction_first_kept_valid(
        n_entries in 2usize..10,
        compact_idx in 0usize..5
    ) {
        let entries: Vec<SessionEntry> = (0..n_entries)
            .map(|i| {
                let parent = if i == 0 {
                    None
                } else {
                    Some(format!("e{}", i - 1))
                };
                arb_user_entry(format!("e{i}"), parent, format!("msg {i}"))
            })
            .collect();

        let compact_at = compact_idx.min(n_entries - 1);
        let first_kept = format!("e{compact_at}");

        let ce = compaction_entry(
            "c1",
            Some(&format!("e{}", n_entries - 1)),
            &first_kept,
            5000,
            1000,
        );

        let mut all = entries.clone();
        all.push(ce);

        let header = make_header("prop-compact");
        let (_, read_entries) = write_and_read(header, &all);

        let ids: HashSet<String> = read_entries.iter().map(entry_id).collect();
        prop_assert!(
            ids.contains(&first_kept),
            "first_kept_entry_id '{}' should exist in session",
            first_kept
        );
    }
}
