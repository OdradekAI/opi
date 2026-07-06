//! Phase 13.2 — reusable session context reconstruction API.
//!
//! DoD: "opi-agent exposes a reusable context reconstruction API that starts
//! from ordered session entries, treats the last valid Leaf entry as the
//! active leaf, walks the parent_id chain to root, applies compaction and
//! branch_summary entries at documented positions, returns agent-runtime
//! messages plus selected model/thinking/session/label metadata, and emits
//! diagnostics for corrupt middle entries and missing parents. Metadata
//! entries, labels, and extension_state do not advance the content tip and do
//! not enter LLM context... Tests cover multiple leaf entries, labels
//! excluded from LLM context, branch_summary included in LLM context,
//! compaction summary ordering, custom_message implemented-or-deferred
//! behavior, extension_state restore metadata, v1/v2 fixture parity, and
//! deterministic output ordering."

use opi_agent::diagnostic::{Diagnostic, SOURCE_SESSION, Severity, code::*};
use opi_agent::message::AgentMessage;
use opi_agent::session::{
    BranchSummaryEntry, CompactionEntry, CrashRecovery, ExtensionStateEntry, LabelAction,
    LabelEntry, LeafEntry, MessageEntry, ModelChangeEntry, SessionEntry, SessionHeader,
    SessionInfoEntry, SessionReader, SessionWriter, ThinkingLevelChangeEntry,
};
use opi_agent::session_context::{
    ReconstructedContext, active_chain_entry_ids, reconstruct_context,
};
use opi_agent::session_event::ThinkingLevel;
use opi_ai::message::{AssistantContent, AssistantMessage, InputContent, Message, UserMessage};
use serde_json::json;
use std::io::Write;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn user_msg(id: &str, parent: Option<&str>, text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        id: id.into(),
        parent_id: parent.map(str::to_owned),
        timestamp: "2026-07-01T00:00:00Z".into(),
        message: Message::User(UserMessage {
            content: vec![InputContent::Text { text: text.into() }],
            timestamp_ms: 0,
        }),
    })
}

fn assistant_msg(id: &str, parent: Option<&str>, text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        id: id.into(),
        parent_id: parent.map(str::to_owned),
        timestamp: "2026-07-01T00:00:01Z".into(),
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
        parent_id: parent.map(str::to_owned),
        timestamp: "2026-07-01T00:00:02Z".into(),
        summary: "compacted summary".into(),
        first_kept_entry_id: first_kept.into(),
        tokens_before: 5000,
        tokens_after: 1000,
    })
}

fn leaf(id: &str, parent: Option<&str>, entry_id: &str) -> SessionEntry {
    SessionEntry::Leaf(LeafEntry {
        id: id.into(),
        parent_id: parent.map(str::to_owned),
        timestamp: "2026-07-01T00:00:03Z".into(),
        entry_id: entry_id.into(),
    })
}

fn branch_summary(id: &str, parent: Option<&str>, summary: &str) -> SessionEntry {
    SessionEntry::BranchSummary(BranchSummaryEntry {
        id: id.into(),
        parent_id: parent.map(str::to_owned),
        timestamp: "2026-07-01T00:00:04Z".into(),
        summary: summary.into(),
    })
}

fn model_change(id: &str, parent: Option<&str>, model: &str) -> SessionEntry {
    SessionEntry::ModelChange(ModelChangeEntry {
        id: id.into(),
        parent_id: parent.map(str::to_owned),
        timestamp: "2026-07-01T00:00:05Z".into(),
        model: model.into(),
    })
}

fn thinking_change(id: &str, parent: Option<&str>, level: ThinkingLevel) -> SessionEntry {
    SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
        id: id.into(),
        parent_id: parent.map(str::to_owned),
        timestamp: "2026-07-01T00:00:06Z".into(),
        level,
    })
}

fn session_info(id: &str, parent: Option<&str>, name: &str) -> SessionEntry {
    SessionEntry::SessionInfo(SessionInfoEntry {
        id: id.into(),
        parent_id: parent.map(str::to_owned),
        timestamp: "2026-07-01T00:00:07Z".into(),
        name: name.into(),
    })
}

fn label(id: &str, parent: Option<&str>, label: &str, action: LabelAction) -> SessionEntry {
    SessionEntry::Label(LabelEntry {
        id: id.into(),
        parent_id: parent.map(str::to_owned),
        timestamp: "2026-07-01T00:00:08Z".into(),
        label: label.into(),
        action,
    })
}

fn extension_state(id: &str, parent: Option<&str>, state: serde_json::Value) -> SessionEntry {
    SessionEntry::ExtensionState(ExtensionStateEntry {
        id: id.into(),
        parent_id: parent.map(str::to_owned),
        timestamp: "2026-07-01T00:00:09Z".into(),
        state,
    })
}

fn clean() -> CrashRecovery {
    CrashRecovery::default()
}

fn llm_text(msg: &AgentMessage) -> Option<&str> {
    match msg {
        AgentMessage::Llm(Message::User(u)) => u.content.iter().find_map(|c| match c {
            InputContent::Text { text } => Some(text.as_str()),
            _ => None,
        }),
        AgentMessage::Llm(Message::Assistant(a)) => a.content.iter().find_map(|c| match c {
            AssistantContent::Text { text } => Some(text.as_str()),
            _ => None,
        }),
        _ => None,
    }
}

/// Count how many diagnostics in `ctx.diagnostics` carry the given code.
fn count_code(ctx: &ReconstructedContext, code: &str) -> usize {
    ctx.diagnostics.iter().filter(|d| d.code == code).count()
}

// ---------------------------------------------------------------------------
// 1. Linear chain — basic ordering + active tip
// ---------------------------------------------------------------------------

#[test]
fn linear_chain_reconstructs_messages_in_order() {
    let entries = vec![
        user_msg("m1", None, "hello"),
        assistant_msg("m2", Some("m1"), "hi"),
        user_msg("m3", Some("m2"), "how are you"),
    ];

    let ctx = reconstruct_context(&entries, &clean());

    assert_eq!(ctx.messages.len(), 3);
    assert_eq!(llm_text(&ctx.messages[0]), Some("hello"));
    assert_eq!(llm_text(&ctx.messages[1]), Some("hi"));
    assert_eq!(llm_text(&ctx.messages[2]), Some("how are you"));
    assert_eq!(ctx.active_tip_entry_id.as_deref(), Some("m3"));
}

#[test]
fn legacy_rootless_content_without_leaf_replays_file_order() {
    let entries = vec![
        user_msg("m1", None, "old"),
        user_msg("m2", None, "kept"),
        compaction("c1", None, "m2"),
    ];

    assert_eq!(
        active_chain_entry_ids(&entries),
        vec!["m1".to_owned(), "m2".to_owned(), "c1".to_owned()]
    );

    let ctx = reconstruct_context(&entries, &clean());

    assert_eq!(ctx.active_tip_entry_id.as_deref(), Some("c1"));
    assert_eq!(
        ctx.messages.len(),
        2,
        "legacy rootless replay should apply compaction over file order"
    );
    assert!(matches!(
        ctx.messages[0],
        AgentMessage::CompactionSummary(_)
    ));
    assert_eq!(llm_text(&ctx.messages[1]), Some("kept"));
}

#[test]
fn empty_entries_yield_empty_context() {
    let ctx = reconstruct_context(&[], &clean());

    assert!(ctx.messages.is_empty());
    assert_eq!(ctx.active_tip_entry_id, None);
    assert_eq!(ctx.model, None);
    assert_eq!(ctx.thinking_level, None);
    assert_eq!(ctx.session_name, None);
    assert!(ctx.labels.is_empty());
    assert_eq!(ctx.extension_state, None);
    assert!(ctx.diagnostics.is_empty());
}

// ---------------------------------------------------------------------------
// 2. Multiple leaf entries — last leaf wins as active tip
// ---------------------------------------------------------------------------

#[test]
fn multiple_leaf_entries_last_leaf_wins_as_active_tip() {
    // Branches: m1 -> m2a, m1 -> m2b. Two leaves; the later leaf (pointing at
    // m2b) wins, so reconstruction walks the m2b branch, not m2a.
    let entries = vec![
        user_msg("m1", None, "root"),
        assistant_msg("m2a", Some("m1"), "branch A"),
        assistant_msg("m2b", Some("m1"), "branch B"),
        leaf("l1", Some("m2a"), "m2a"),
        leaf("l2", Some("m2b"), "m2b"),
    ];

    let ctx = reconstruct_context(&entries, &clean());

    assert_eq!(ctx.active_tip_entry_id.as_deref(), Some("m2b"));
    // The m2a branch must NOT appear in the reconstructed LLM context.
    let texts: Vec<_> = ctx.messages.iter().filter_map(llm_text).collect();
    assert!(texts.contains(&"root"));
    assert!(texts.contains(&"branch B"));
    assert!(
        !texts.contains(&"branch A"),
        "inactive branch must not enter LLM context"
    );
}

// ---------------------------------------------------------------------------
// 3. Labels excluded from LLM context, present in metadata
// ---------------------------------------------------------------------------

#[test]
fn labels_excluded_from_llm_context_but_in_metadata() {
    let entries = vec![
        user_msg("m1", None, "hello"),
        label("lab1", Some("m1"), "important", LabelAction::Add),
        assistant_msg("m2", Some("m1"), "hi"),
    ];

    let ctx = reconstruct_context(&entries, &clean());

    // Two LLM messages; label injected nothing.
    assert_eq!(ctx.messages.len(), 2);
    assert!(
        !ctx.messages
            .iter()
            .any(|m| matches!(m, AgentMessage::Custom(_))),
        "labels must not synthesize agent messages"
    );
    // Label is in metadata.
    assert_eq!(ctx.labels, vec!["important".to_owned()]);
}

#[test]
fn label_add_remove_yields_active_set() {
    let entries = vec![
        user_msg("m1", None, "hello"),
        label("l1", Some("m1"), "a", LabelAction::Add),
        label("l2", Some("m1"), "b", LabelAction::Add),
        label("l3", Some("m1"), "a", LabelAction::Remove),
    ];

    let ctx = reconstruct_context(&entries, &clean());

    assert_eq!(ctx.labels, vec!["b".to_owned()]);
}

#[test]
fn duplicate_adds_deduplicate() {
    let entries = vec![
        user_msg("m1", None, "hello"),
        label("l1", Some("m1"), "a", LabelAction::Add),
        label("l2", Some("m1"), "a", LabelAction::Add),
    ];

    let ctx = reconstruct_context(&entries, &clean());

    assert_eq!(ctx.labels, vec!["a".to_owned()]);
}

#[test]
fn rootless_metadata_applies_when_chain_is_empty() {
    let entries = vec![
        session_info("info1", None, "pre-first-turn"),
        model_change("mc1", None, "anthropic:claude-sonnet-4"),
        thinking_change("th1", None, ThinkingLevel::Low),
        label("l1", None, "setup", LabelAction::Add),
        extension_state("state1", None, json!({"restored": true})),
    ];

    let ctx = reconstruct_context(&entries, &clean());

    assert!(
        ctx.messages.is_empty(),
        "rootless metadata must not synthesize provider messages"
    );
    assert_eq!(ctx.session_name.as_deref(), Some("pre-first-turn"));
    assert_eq!(ctx.model.as_deref(), Some("anthropic:claude-sonnet-4"));
    assert_eq!(ctx.thinking_level, Some(ThinkingLevel::Low));
    assert_eq!(ctx.labels, vec!["setup".to_owned()]);
    assert_eq!(ctx.extension_state, Some(json!({"restored": true})));
}

// ---------------------------------------------------------------------------
// 4. branch_summary included in LLM context at its chain position
// ---------------------------------------------------------------------------

#[test]
fn branch_summary_included_in_llm_context() {
    let entries = vec![
        user_msg("m1", None, "hello"),
        // Summary is parented to the content tip m1 and does NOT advance it
        // (Phase 13.1 storage invariant). The next message m2 is therefore
        // parented to m1 as well, not to bs1.
        branch_summary("bs1", Some("m1"), "earlier branch context"),
        assistant_msg("m2", Some("m1"), "hi"),
    ];

    let ctx = reconstruct_context(&entries, &clean());

    // Order: m1, BranchSummary(bs1), m2 — the summary injects right after its
    // parent's contribution.
    assert_eq!(ctx.messages.len(), 3);
    assert!(matches!(ctx.messages[0], AgentMessage::Llm(_)));
    match &ctx.messages[1] {
        AgentMessage::BranchSummary(bs) => {
            assert_eq!(bs.summary, "earlier branch context");
        }
        other => panic!("expected BranchSummary, got {:?}", other),
    }
    assert!(matches!(ctx.messages[2], AgentMessage::Llm(_)));
}

// ---------------------------------------------------------------------------
// 5. Compaction summary ordering
// ---------------------------------------------------------------------------

#[test]
fn compaction_summary_replaces_pre_first_kept_messages() {
    // Chain: m1 -> m2 -> m3 -> m4 -> c1(first_kept=m4) -> m5 -> m6
    // Expected reconstruction: [CompactionSummary(c1), m4, m5, m6]
    let entries = vec![
        user_msg("m1", None, "one"),
        assistant_msg("m2", Some("m1"), "two"),
        user_msg("m3", Some("m2"), "three"),
        assistant_msg("m4", Some("m3"), "four"),
        compaction("c1", Some("m4"), "m4"),
        user_msg("m5", Some("c1"), "five"),
        assistant_msg("m6", Some("m5"), "six"),
    ];

    let ctx = reconstruct_context(&entries, &clean());

    assert_eq!(
        ctx.messages.len(),
        4,
        "compaction must drop m1..m3 and inject one summary"
    );
    match &ctx.messages[0] {
        AgentMessage::CompactionSummary(cs) => {
            assert_eq!(cs.summary, "compacted summary");
            assert_eq!(cs.first_kept_entry_id, "m4");
        }
        other => panic!("expected CompactionSummary first, got {:?}", other),
    }
    assert_eq!(llm_text(&ctx.messages[1]), Some("four"));
    assert_eq!(llm_text(&ctx.messages[2]), Some("five"));
    assert_eq!(llm_text(&ctx.messages[3]), Some("six"));
}

#[test]
fn nested_compactions_latest_compaction_wins() {
    // m1 -> c1(first_kept=m1) -> m2 -> c2(first_kept=m2) -> m3
    // After c1: [Summary_c1, m1]. After c2 (compacts Summary_c1 + m1): [Summary_c2, m2].
    // Final: [Summary_c2, m2, m3].
    let entries = vec![
        user_msg("m1", None, "one"),
        compaction("c1", Some("m1"), "m1"),
        user_msg("m2", Some("c1"), "two"),
        compaction("c2", Some("m2"), "m2"),
        user_msg("m3", Some("c2"), "three"),
    ];

    let ctx = reconstruct_context(&entries, &clean());

    assert_eq!(ctx.messages.len(), 3);
    assert!(matches!(
        ctx.messages[0],
        AgentMessage::CompactionSummary(_)
    ));
    match &ctx.messages[0] {
        AgentMessage::CompactionSummary(cs) => assert_eq!(cs.first_kept_entry_id, "m2"),
        _ => unreachable!(),
    }
    assert_eq!(llm_text(&ctx.messages[1]), Some("two"));
    assert_eq!(llm_text(&ctx.messages[2]), Some("three"));
}

// ---------------------------------------------------------------------------
// 6. custom_message deferred — extension_state does NOT inject Custom
// ---------------------------------------------------------------------------

#[test]
fn custom_message_deferred_extension_state_does_not_inject_llm_message() {
    let entries = vec![
        user_msg("m1", None, "hello"),
        // extension_state carries an extension-provided message-like blob, but
        // Phase 13 defers custom_message: it MUST NOT become an
        // AgentMessage::Custom in the reconstructed LLM context.
        extension_state("es1", Some("m1"), json!({ "kind": "note", "data": "hi" })),
    ];

    let ctx = reconstruct_context(&entries, &clean());

    assert_eq!(
        ctx.messages.len(),
        1,
        "extension_state must not synthesize an LLM message"
    );
    assert!(
        !ctx.messages
            .iter()
            .any(|m| matches!(m, AgentMessage::Custom(_))),
        "custom_message is deferred (Phase 13.1 decision)"
    );
    // The blob is exposed as restore metadata only.
    assert_eq!(
        ctx.extension_state,
        Some(json!({ "kind": "note", "data": "hi" }))
    );
}

#[test]
fn extension_state_restore_metadata_latest_wins() {
    let entries = vec![
        user_msg("m1", None, "hello"),
        extension_state("es1", Some("m1"), json!({ "rev": 1 })),
        extension_state("es2", Some("m1"), json!({ "rev": 2 })),
    ];

    let ctx = reconstruct_context(&entries, &clean());

    assert_eq!(ctx.extension_state, Some(json!({ "rev": 2 })));
}

// ---------------------------------------------------------------------------
// 7. model / thinking / session_name metadata — latest wins, file order
// ---------------------------------------------------------------------------

#[test]
fn model_thinking_session_name_metadata_latest_wins() {
    let entries = vec![
        user_msg("m1", None, "hello"),
        model_change("mc1", Some("m1"), "anthropic:claude-a"),
        thinking_change("tc1", Some("m1"), ThinkingLevel::Low),
        session_info("si1", Some("m1"), "first name"),
        model_change("mc2", Some("m1"), "anthropic:claude-b"),
        thinking_change("tc2", Some("m1"), ThinkingLevel::High),
        session_info("si2", Some("m1"), "second name"),
    ];

    let ctx = reconstruct_context(&entries, &clean());

    assert_eq!(ctx.model.as_deref(), Some("anthropic:claude-b"));
    assert_eq!(ctx.thinking_level, Some(ThinkingLevel::High));
    assert_eq!(ctx.session_name.as_deref(), Some("second name"));
    // Metadata did not enter LLM context.
    assert_eq!(ctx.messages.len(), 1);
}

// ---------------------------------------------------------------------------
// 8. v1/v2 fixture parity
// ---------------------------------------------------------------------------

#[test]
fn v1_v2_fixture_parity_messages_identical() {
    // v1 fixture: only Message + Leaf (classic opi entries).
    let v1 = vec![
        user_msg("m1", None, "hello"),
        assistant_msg("m2", Some("m1"), "hi"),
        leaf("l1", Some("m2"), "m2"),
    ];
    // v2 fixture: same content plus additive metadata entries parented to the
    // same content tips. Messages must reconstruct identically; metadata is
    // additionally populated for v2.
    let mut v2 = v1.clone();
    v2.push(model_change("mc1", Some("m2"), "anthropic:claude-b"));
    v2.push(session_info("si1", Some("m2"), "named"));
    v2.push(label("lab1", Some("m2"), "pinned", LabelAction::Add));

    let ctx_v1 = reconstruct_context(&v1, &clean());
    let ctx_v2 = reconstruct_context(&v2, &clean());

    assert_eq!(ctx_v1.messages.len(), ctx_v2.messages.len());
    for (a, b) in ctx_v1.messages.iter().zip(ctx_v2.messages.iter()) {
        // Variant-tag parity first (a non-Llm injection in v2-only would slip
        // past `llm_text` equality alone, which returns `None` for non-Llm),
        // then content parity.
        assert_eq!(
            std::mem::discriminant(a),
            std::mem::discriminant(b),
            "variant tag must match between v1 and v2 reconstructions"
        );
        assert_eq!(llm_text(a), llm_text(b));
    }
    // v1 has no v2 metadata; v2 does.
    assert_eq!(ctx_v1.model, None);
    assert_eq!(ctx_v2.model.as_deref(), Some("anthropic:claude-b"));
    assert!(ctx_v1.labels.is_empty());
    assert_eq!(ctx_v2.labels, vec!["pinned".to_owned()]);
    assert_eq!(ctx_v1.session_name, None);
    assert_eq!(ctx_v2.session_name.as_deref(), Some("named"));
}

// ---------------------------------------------------------------------------
// 9. Deterministic output ordering
// ---------------------------------------------------------------------------

#[test]
fn reconstruction_is_deterministic_across_calls() {
    let entries = vec![
        user_msg("m1", None, "hello"),
        branch_summary("bs1", Some("m1"), "ctx"),
        assistant_msg("m2", Some("bs1"), "hi"),
        model_change("mc1", Some("bs1"), "anthropic:claude-b"),
    ];

    let a = reconstruct_context(&entries, &clean());
    let b = reconstruct_context(&entries, &clean());

    assert_eq!(a.messages.len(), b.messages.len());
    assert_eq!(a.active_tip_entry_id, b.active_tip_entry_id);
    assert_eq!(a.model, b.model);
    assert_eq!(a.thinking_level, b.thinking_level);
    assert_eq!(a.session_name, b.session_name);
    assert_eq!(a.labels, b.labels);
    assert_eq!(a.extension_state, b.extension_state);
}

// ---------------------------------------------------------------------------
// 10. Missing-parent and corrupt-middle diagnostics
// ---------------------------------------------------------------------------

#[test]
fn missing_parent_emits_warning_diagnostic() {
    // m2 references a parent that does not exist anywhere in the file.
    let entries = vec![
        user_msg("m1", None, "hello"),
        assistant_msg("m2", Some("ghost"), "orphan"),
    ];

    let ctx = reconstruct_context(&entries, &clean());

    assert_eq!(
        count_code(&ctx, CODE_SESSION_CONTEXT_MISSING_PARENT),
        1,
        "missing parent must surface exactly one warning"
    );
    // Every missing-parent diagnostic is a Warning and sourced at session.
    assert!(ctx.diagnostics.iter().any(|d| {
        d.code == CODE_SESSION_CONTEXT_MISSING_PARENT
            && d.severity == Severity::Warning
            && d.source == SOURCE_SESSION
    }));
}

#[test]
fn active_chain_entry_ids_falls_back_like_reconstruct_context_for_invalid_leaf() {
    let entries = vec![
        user_msg("m1", None, "root"),
        assistant_msg("m2", Some("m1"), "trunk"),
        leaf("leaf1", Some("m2"), "missing-tip"),
    ];

    let ids = active_chain_entry_ids(&entries);
    let ctx = reconstruct_context(&entries, &clean());

    assert_eq!(ids, vec!["m1".to_owned(), "m2".to_owned()]);
    assert_eq!(ctx.active_tip_entry_id.as_deref(), Some("m2"));
    assert_eq!(ctx.messages.len(), 2);
}

#[test]
fn active_tip_fallback_emits_diagnostic_when_leaf_target_missing() {
    let entries = vec![
        user_msg("m1", None, "root"),
        assistant_msg("m2", Some("m1"), "trunk"),
        leaf("leaf1", Some("m2"), "missing-tip"),
    ];

    let ctx = reconstruct_context(&entries, &clean());

    assert_eq!(ctx.active_tip_entry_id.as_deref(), Some("m2"));
    assert_eq!(count_code(&ctx, CODE_SESSION_LEAF_TIP_MISSING), 1);
    assert!(ctx.diagnostics.iter().any(|d| {
        d.code == CODE_SESSION_LEAF_TIP_MISSING
            && d.severity == Severity::Warning
            && d.source == SOURCE_SESSION
            && d.details.as_ref().and_then(|v| v["leaf_entry_id"].as_str()) == Some("missing-tip")
            && d.details.as_ref().and_then(|v| v["fallback_tip"].as_str()) == Some("m2")
    }));
}

#[test]
fn reconstruct_context_terminates_on_circular_parent_ids() {
    let entries = vec![
        user_msg("a", Some("b"), "cycle a"),
        assistant_msg("b", Some("a"), "cycle b"),
    ];

    let ctx = reconstruct_context(&entries, &clean());

    assert!(
        ctx.messages.len() <= 2,
        "cycle guard must terminate reconstruction without duplicating indefinitely"
    );
}

#[test]
fn corrupt_middle_entries_forwarded_from_crash_recovery() {
    let entries = vec![user_msg("m1", None, "hello")];
    let recovery = CrashRecovery {
        truncated_line: false,
        corrupt_count: 2,
        unknown_count: 0,
    };

    let ctx = reconstruct_context(&entries, &recovery);

    assert_eq!(count_code(&ctx, CODE_SESSION_CORRUPT_ENTRIES), 1);
    assert_eq!(count_code(&ctx, CODE_SESSION_UNKNOWN_ENTRIES), 0);
}

#[test]
fn read_with_recovery_feeds_corrupt_middle_into_reconstruct_context() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("corrupt-middle.jsonl");
    let header = SessionHeader::new(
        "corrupt-middle".into(),
        "2026-07-01T00:00:00Z".into(),
        "/repo".into(),
        None,
    );
    let mut writer = SessionWriter::create(&path, header).unwrap();
    writer.append(&user_msg("m1", None, "hello")).unwrap();
    drop(writer);

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    writeln!(file, "{{not valid json}}").unwrap();

    let (_header, entries, recovery) = SessionReader::read_with_recovery(&path).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(recovery.corrupt_count(), 1);

    let ctx = reconstruct_context(&entries, &recovery);

    assert_eq!(count_code(&ctx, CODE_SESSION_CORRUPT_ENTRIES), 1);
    assert!(ctx.diagnostics.iter().any(|d| {
        d.code == CODE_SESSION_CORRUPT_ENTRIES
            && d.details.as_ref().and_then(|v| v["corrupt_count"].as_u64()) == Some(1)
    }));
}

#[test]
fn unknown_entries_forwarded_from_crash_recovery() {
    let entries = vec![user_msg("m1", None, "hello")];
    let recovery = CrashRecovery {
        truncated_line: false,
        corrupt_count: 0,
        unknown_count: 3,
    };

    let ctx = reconstruct_context(&entries, &recovery);

    assert_eq!(count_code(&ctx, CODE_SESSION_UNKNOWN_ENTRIES), 1);
}

#[test]
fn truncated_line_forwarded_from_crash_recovery() {
    let entries = vec![user_msg("m1", None, "hello")];
    let recovery = CrashRecovery {
        truncated_line: true,
        corrupt_count: 0,
        unknown_count: 0,
    };

    let ctx = reconstruct_context(&entries, &recovery);

    assert_eq!(count_code(&ctx, CODE_SESSION_TRUNCATED_LINE), 1);
}

// ---------------------------------------------------------------------------
// 11. Metadata on an inactive branch does not apply
// ---------------------------------------------------------------------------

#[test]
fn metadata_on_inactive_branch_does_not_apply() {
    // Active branch is m2b (leaf points at it). A model_change parented to the
    // inactive m2a tip must NOT bleed into the active context.
    let entries = vec![
        user_msg("m1", None, "root"),
        assistant_msg("m2a", Some("m1"), "branch A"),
        assistant_msg("m2b", Some("m1"), "branch B"),
        model_change("mc_a", Some("m2a"), "anthropic:claude-a"),
        leaf("l1", Some("m2b"), "m2b"),
    ];

    let ctx = reconstruct_context(&entries, &clean());

    assert_eq!(ctx.active_tip_entry_id.as_deref(), Some("m2b"));
    assert_eq!(
        ctx.model, None,
        "metadata parented to an inactive branch must not apply"
    );
}

// ---------------------------------------------------------------------------
// 12. Diagnostics helper coverage — Diagnostic fields used by tests exist.
// ---------------------------------------------------------------------------

#[test]
fn active_branch_metadata_wins_over_later_inactive_metadata() {
    let entries = vec![
        user_msg("m1", None, "root"),
        assistant_msg("m2a", Some("m1"), "branch A"),
        assistant_msg("m2b", Some("m1"), "branch B"),
        model_change("mc_active", Some("m2b"), "anthropic:active-model"),
        model_change("mc_inactive", Some("m2a"), "anthropic:inactive-model"),
        leaf("l1", Some("m2b"), "m2b"),
    ];

    let ctx = reconstruct_context(&entries, &clean());

    assert_eq!(ctx.active_tip_entry_id.as_deref(), Some("m2b"));
    assert_eq!(ctx.model.as_deref(), Some("anthropic:active-model"));
}

#[test]
fn diagnostic_fields_round_trip() {
    // Guards the public `code`, `severity`, `source` fields used above; if the
    // Diagnostic surface changes shape, this fails at compile time.
    let diag = Diagnostic::new(
        Severity::Warning,
        CODE_SESSION_CONTEXT_MISSING_PARENT,
        SOURCE_SESSION,
        "test",
    );
    assert_eq!(diag.code, CODE_SESSION_CONTEXT_MISSING_PARENT);
    assert_eq!(diag.severity, Severity::Warning);
    assert_eq!(diag.source, SOURCE_SESSION);
}
