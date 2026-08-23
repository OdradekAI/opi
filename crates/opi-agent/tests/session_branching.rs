//! Session branching tests (task 4.9).
//!
//! DoD: "The interactive TUI exposes session branch selection/navigation over
//! existing session tree data without corrupting append-only session storage;
//! tests cover branch reconstruction, picker interaction, active branch changes,
//! empty/corrupt session handling, and ratatui snapshots at fixed sizes with
//! explicit approval for snapshot changes."
//!
//! This file covers the opi-agent side: branch reconstruction from JSONL entries.

use opi_agent::session::{
    CompactionEntry, LeafEntry, MessageEntry, SessionEntry, SessionHeader, SessionReader,
    SessionWriter,
};
use opi_agent::session_branch::SessionTree;
use opi_ai::message::{AssistantContent, AssistantMessage, InputContent, Message, UserMessage};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_header(id: &str) -> SessionHeader {
    SessionHeader::new_for_test(
        id.into(),
        "2026-06-01T12:00:00Z".into(),
        "/repo".into(),
        None,
    )
}

fn user_msg(id: &str, parent: Option<&str>, text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        id: id.into(),
        parent_id: parent.map(|s| s.into()),
        timestamp: "2026-06-01T12:00:01Z".into(),
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
        timestamp: "2026-06-01T12:00:02Z".into(),
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

fn compaction(id: &str, parent: Option<&str>, first_kept: &str) -> SessionEntry {
    SessionEntry::Compaction(CompactionEntry {
        id: id.into(),
        parent_id: parent.map(|s| s.into()),
        timestamp: "2026-06-01T13:00:00Z".into(),
        summary: "Compacted.".into(),
        first_kept_entry_id: first_kept.into(),
        tokens_before: 5000,
        tokens_after: 1000,
    })
}

fn leaf(id: &str, parent: Option<&str>, entry_id: &str) -> SessionEntry {
    SessionEntry::Leaf(LeafEntry {
        id: id.into(),
        parent_id: parent.map(|s| s.into()),
        timestamp: "2026-06-01T14:00:00Z".into(),
        entry_id: entry_id.into(),
    })
}

fn write_and_read(
    header: SessionHeader,
    entries: &[SessionEntry],
) -> (SessionHeader, Vec<SessionEntry>) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.jsonl");
    {
        let mut writer = SessionWriter::create(&path, header.clone()).unwrap();
        for entry in entries {
            writer.append(entry).unwrap();
        }
    }
    SessionReader::read_all(&path).unwrap()
}

// ---------------------------------------------------------------------------
// 1. Linear session (no branches)
// ---------------------------------------------------------------------------

#[test]
fn linear_session_has_single_branch() {
    let entries = vec![
        user_msg("e1", None, "Hello"),
        assistant_msg("e2", Some("e1"), "Hi"),
        user_msg("e3", Some("e2"), "How are you?"),
        assistant_msg("e4", Some("e3"), "Fine"),
    ];

    let tree = SessionTree::from_entries(&entries);

    assert_eq!(tree.branches().len(), 1, "linear session has one branch");
    let branch = &tree.branches()[0];
    assert_eq!(branch.tip_id, "e4");
    assert_eq!(branch.depth, 3); // e1-e2, e2-e3, e3-e4
}

#[test]
fn linear_session_active_branch_is_root_branch() {
    let entries = vec![
        user_msg("e1", None, "Hello"),
        assistant_msg("e2", Some("e1"), "Hi"),
    ];

    let tree = SessionTree::from_entries(&entries);

    assert_eq!(tree.active_branch_index(), Some(0));
}

#[test]
fn linear_session_without_leaf_active_tip_is_last() {
    let entries = vec![
        user_msg("e1", None, "Hello"),
        assistant_msg("e2", Some("e1"), "Hi"),
    ];

    let tree = SessionTree::from_entries(&entries);

    assert_eq!(tree.active_tip(), Some("e2"));
}

// ---------------------------------------------------------------------------
// 2. Branched session
// ---------------------------------------------------------------------------

#[test]
fn branched_session_has_multiple_branches() {
    // e1 -> e2a -> e3a
    // e1 -> e2b
    let entries = vec![
        user_msg("e1", None, "Hello"),
        assistant_msg("e2a", Some("e1"), "Branch A"),
        assistant_msg("e2b", Some("e1"), "Branch B"),
        user_msg("e3a", Some("e2a"), "Follow A"),
    ];

    let tree = SessionTree::from_entries(&entries);

    // Branches: trunk (e1), branch-a (e2a-e3a), branch-b (e2b)
    assert!(
        tree.branches().len() >= 2,
        "branched session should have >= 2 branches, got {}",
        tree.branches().len()
    );
}

#[test]
fn branched_session_child_branches_are_sorted_by_tip_id() {
    // Entries arrive in b/a order, but branch presentation must be stable.
    let entries = vec![
        user_msg("e1", None, "Hello"),
        assistant_msg("e2b", Some("e1"), "Branch B"),
        assistant_msg("e2a", Some("e1"), "Branch A"),
    ];

    let tree = SessionTree::from_entries(&entries);
    let tips = tree
        .branches()
        .iter()
        .map(|branch| branch.tip_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(tips, vec!["e1", "e2a", "e2b"]);
}

#[test]
fn branched_session_leaf_determines_active() {
    // e1 -> e2a -> e3a
    // e1 -> e2b
    // leaf points to e2b
    let entries = vec![
        user_msg("e1", None, "Hello"),
        assistant_msg("e2a", Some("e1"), "Branch A"),
        assistant_msg("e2b", Some("e1"), "Branch B"),
        user_msg("e3a", Some("e2a"), "Follow A"),
        leaf("l1", Some("e3a"), "e2b"),
    ];

    let tree = SessionTree::from_entries(&entries);

    assert_eq!(tree.active_tip(), Some("e2b"));
}

#[test]
fn branched_session_last_leaf_wins() {
    let entries = vec![
        user_msg("e1", None, "Hello"),
        assistant_msg("e2a", Some("e1"), "A"),
        assistant_msg("e2b", Some("e1"), "B"),
        leaf("l1", Some("e2a"), "e2a"),
        leaf("l2", Some("e2b"), "e2b"),
    ];

    let tree = SessionTree::from_entries(&entries);

    // Last leaf entry_id = e2b
    assert_eq!(tree.active_tip(), Some("e2b"));
}

#[test]
fn branch_info_contains_tip_and_summary() {
    let entries = vec![
        user_msg("e1", None, "Hello"),
        assistant_msg("e2", Some("e1"), "Reply text here"),
    ];

    let tree = SessionTree::from_entries(&entries);

    let branch = &tree.branches()[0];
    assert_eq!(branch.tip_id, "e2");
    assert!(
        branch
            .summary
            .as_ref()
            .is_some_and(|s| s.contains("Reply text here")),
        "branch summary should contain last message text"
    );
}

// ---------------------------------------------------------------------------
// 3. Compaction in branches
// ---------------------------------------------------------------------------

#[test]
fn compaction_entry_is_part_of_branch() {
    let entries = vec![
        user_msg("e1", None, "Hello"),
        assistant_msg("e2", Some("e1"), "Reply"),
        user_msg("e3", Some("e2"), "More"),
        compaction("c1", Some("e3"), "e3"),
        user_msg("e4", Some("c1"), "After compact"),
    ];

    let tree = SessionTree::from_entries(&entries);

    assert_eq!(tree.active_tip(), Some("e4"));
    let branch = &tree.branches()[0];
    assert_eq!(branch.tip_id, "e4");
}

#[test]
fn branch_with_compaction_counts_depth_correctly() {
    let entries = vec![
        user_msg("e1", None, "Hello"),
        assistant_msg("e2", Some("e1"), "Reply"),
        compaction("c1", Some("e2"), "e2"),
        user_msg("e3", Some("c1"), "After"),
    ];

    let tree = SessionTree::from_entries(&entries);

    let branch = &tree.branches()[0];
    // Depth: e1->e2, e2->c1, c1->e3 = 3 edges
    assert_eq!(branch.depth, 3);
}

// ---------------------------------------------------------------------------
// 4. Empty and single-entry sessions
// ---------------------------------------------------------------------------

#[test]
fn empty_session_has_no_branches() {
    let entries: Vec<SessionEntry> = vec![];

    let tree = SessionTree::from_entries(&entries);

    assert_eq!(tree.branches().len(), 0);
    assert_eq!(tree.active_tip(), None);
    assert_eq!(tree.active_branch_index(), None);
}

#[test]
fn single_entry_session_has_one_branch() {
    let entries = vec![user_msg("e1", None, "Hello")];

    let tree = SessionTree::from_entries(&entries);

    assert_eq!(tree.branches().len(), 1);
    assert_eq!(tree.active_tip(), Some("e1"));
    let branch = &tree.branches()[0];
    assert_eq!(branch.depth, 0, "single entry has depth 0 (no edges)");
}

// ---------------------------------------------------------------------------
// 5. Corrupt session handling
// ---------------------------------------------------------------------------

#[test]
fn corrupt_parent_id_ignored_gracefully() {
    // e2 references a parent that doesn't exist
    let entries = vec![
        user_msg("e1", None, "Hello"),
        user_msg("e2", Some("nonexistent"), "Orphan"),
    ];

    let tree = SessionTree::from_entries(&entries);

    // Should still produce branches without panicking
    assert!(
        !tree.branches().is_empty(),
        "corrupt parent should still produce branches"
    );
}

#[test]
fn cycle_in_parent_id_detected() {
    // e1 -> e2 -> e1 (cycle)
    let entries = vec![
        user_msg("e1", Some("e2"), "Cycle A"),
        user_msg("e2", Some("e1"), "Cycle B"),
    ];

    let tree = SessionTree::from_entries(&entries);

    // Should not loop infinitely; produces branches (possibly incomplete)
    assert!(
        !tree.branches().is_empty(),
        "cycle should be handled without infinite loop"
    );
}

#[test]
fn leaf_pointing_to_nonexistent_entry_handled() {
    let entries = vec![
        user_msg("e1", None, "Hello"),
        leaf("l1", Some("e1"), "nonexistent_tip"),
    ];

    let tree = SessionTree::from_entries(&entries);

    // Leaf points to nonexistent entry, so active_tip falls back to trunk
    assert_eq!(tree.active_tip(), Some("e1"));
}

// ---------------------------------------------------------------------------
// 6. Session tree from JSONL round-trip
// ---------------------------------------------------------------------------

#[test]
fn session_tree_from_jsonl_round_trip() {
    let header = make_header("branch-rt");
    let entries = vec![
        user_msg("e1", None, "Root"),
        assistant_msg("e2a", Some("e1"), "Branch A"),
        assistant_msg("e2b", Some("e1"), "Branch B"),
        leaf("l1", Some("e2a"), "e2a"),
    ];

    let (_, read_entries) = write_and_read(header, &entries);
    let tree = SessionTree::from_entries(&read_entries);

    assert_eq!(tree.active_tip(), Some("e2a"));
    assert!(
        tree.branches().len() >= 2,
        "should have >= 2 branches after round-trip"
    );
}

// ---------------------------------------------------------------------------
// 7. Branch selection
// ---------------------------------------------------------------------------

#[test]
fn select_branch_by_index() {
    let entries = vec![
        user_msg("e1", None, "Root"),
        assistant_msg("e2a", Some("e1"), "Branch A"),
        assistant_msg("e2b", Some("e1"), "Branch B"),
    ];

    let tree = SessionTree::from_entries(&entries);

    // Select the second branch (first child branch after trunk)
    let selected = tree.branch_at(1);
    assert!(selected.is_some(), "index 1 should exist");
    assert_eq!(selected.unwrap().tip_id, "e2a");
}

#[test]
fn select_branch_out_of_bounds_returns_none() {
    let entries = vec![user_msg("e1", None, "Hello")];

    let tree = SessionTree::from_entries(&entries);

    assert!(tree.branch_at(99).is_none());
}

// ---------------------------------------------------------------------------
// 8. Branch timestamps
// ---------------------------------------------------------------------------

#[test]
fn branch_info_has_timestamp() {
    let entries = vec![
        user_msg("e1", None, "Hello"),
        assistant_msg("e2", Some("e1"), "Reply"),
    ];

    let tree = SessionTree::from_entries(&entries);
    let branch = &tree.branches()[0];

    assert!(
        !branch.timestamp.is_empty(),
        "branch should have a timestamp"
    );
}

// ---------------------------------------------------------------------------
// 9. Entry count per branch
// ---------------------------------------------------------------------------

#[test]
fn branch_entry_count() {
    let entries = vec![
        user_msg("e1", None, "Hello"),
        assistant_msg("e2", Some("e1"), "Reply"),
        user_msg("e3", Some("e2"), "Follow-up"),
    ];

    let tree = SessionTree::from_entries(&entries);
    let branch = &tree.branches()[0];

    assert_eq!(branch.entry_count, 3);
}

// ---------------------------------------------------------------------------
// 10. Multiple branches from same fork point
// ---------------------------------------------------------------------------

#[test]
fn three_way_branch() {
    // e1 -> e2a
    // e1 -> e2b
    // e1 -> e2c
    let entries = vec![
        user_msg("e1", None, "Root"),
        assistant_msg("e2a", Some("e1"), "A"),
        assistant_msg("e2b", Some("e1"), "B"),
        assistant_msg("e2c", Some("e1"), "C"),
    ];

    let tree = SessionTree::from_entries(&entries);

    assert!(
        tree.branches().len() >= 3,
        "three-way branch should have >= 3 branches, got {}",
        tree.branches().len()
    );
}

// ---------------------------------------------------------------------------
// 11. Only leaf entries (no content entries)
// ---------------------------------------------------------------------------

#[test]
fn only_leaf_entries_produces_no_branches() {
    let entries = vec![leaf("l1", None, "e1"), leaf("l2", Some("l1"), "e2")];

    let tree = SessionTree::from_entries(&entries);

    // Leaf entries are pointers, not content; branches are built from
    // Message/Compaction entries only.
    assert_eq!(tree.branches().len(), 0);
}

// ---------------------------------------------------------------------------
// 12. Branch fork_point field
// ---------------------------------------------------------------------------

#[test]
fn branch_fork_point_is_set_for_child_branches() {
    let entries = vec![
        user_msg("e1", None, "Root"),
        assistant_msg("e2a", Some("e1"), "A"),
        assistant_msg("e2b", Some("e1"), "B"),
    ];

    let tree = SessionTree::from_entries(&entries);

    // The first branch (trunk) has no fork point (it starts from root).
    // Child branches should have fork_point = "e1".
    let child_branches: Vec<_> = tree
        .branches()
        .iter()
        .filter(|b| b.fork_point.as_deref() == Some("e1"))
        .collect();

    assert!(
        !child_branches.is_empty(),
        "at least one branch should fork from e1"
    );
}
