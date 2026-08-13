//! Task 10.5 contract tests for the generic session repo/facade seam.
//!
//! These tests prove the opi-agent session boundary defined in Workstream 10.3:
//! a generic repo/facade can append and read durable entries in order, preserve
//! v1 readability (including against unknown future entries), reconstruct the
//! active branch leaf deterministically, flush pending writes in documented
//! order at save points, never silently drop accepted writes on abort, and keep
//! product directory policy out of opi-agent. They depend ONLY on opi-agent and
//! opi-ai -- no opi-coding-agent product policy.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use opi_agent::harness::{HarnessError, JsonlSessionRepo, SessionFacade, SessionRepo};
use opi_agent::session::{
    CrashRecovery, LabelAction, LeafEntry, MessageEntry, SessionEntry, SessionHeader,
};
use opi_agent::session_event::ThinkingLevel;
use opi_ai::message::{InputContent, Message, UserMessage};
use serde_json::json;
use tempfile::tempdir;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn user_msg(text: &str) -> Message {
    Message::User(UserMessage {
        content: vec![InputContent::Text { text: text.into() }],
        timestamp_ms: 0,
    })
}

fn header(id: &str) -> SessionHeader {
    SessionHeader::new(id.into(), "0".into(), "/repo".into(), None)
}

/// In-memory `SessionRepo` that records appended entries. Used for fast
/// queue/ordering checks that do not need a real JSONL file.
#[derive(Default)]
struct RecordingSessionRepo {
    entries: Vec<SessionEntry>,
}

impl SessionRepo for RecordingSessionRepo {
    fn append(&mut self, entry: &SessionEntry) -> std::io::Result<()> {
        self.entries.push(entry.clone());
        Ok(())
    }
    fn load(&self) -> std::io::Result<(SessionHeader, Vec<SessionEntry>, CrashRecovery)> {
        Ok((
            header("rec"),
            self.entries.clone(),
            CrashRecovery::default(),
        ))
    }
    fn message_count(&self) -> std::io::Result<usize> {
        Ok(self.entries.len())
    }
}

/// `SessionRepo` whose `append` always fails. Used to prove accepted pending
/// writes survive a flush failure and that abort reports them.
struct FailingSessionRepo;

impl SessionRepo for FailingSessionRepo {
    fn append(&mut self, _entry: &SessionEntry) -> std::io::Result<()> {
        Err(std::io::Error::other("simulated flush failure"))
    }
    fn load(&self) -> std::io::Result<(SessionHeader, Vec<SessionEntry>, CrashRecovery)> {
        Ok((header("fail"), Vec::new(), CrashRecovery::default()))
    }
    fn message_count(&self) -> std::io::Result<usize> {
        Ok(0)
    }
}

struct FailOnAppend {
    entries: Vec<SessionEntry>,
    fail_on_call: usize,
    calls: usize,
}

impl FailOnAppend {
    fn new(fail_on_call: usize) -> Self {
        Self {
            entries: Vec::new(),
            fail_on_call,
            calls: 0,
        }
    }
}

impl SessionRepo for FailOnAppend {
    fn append(&mut self, entry: &SessionEntry) -> std::io::Result<()> {
        self.calls += 1;
        if self.calls == self.fail_on_call {
            return Err(std::io::Error::other("simulated partial failure"));
        }
        self.entries.push(entry.clone());
        Ok(())
    }

    fn load(&self) -> std::io::Result<(SessionHeader, Vec<SessionEntry>, CrashRecovery)> {
        Ok((
            header("partial"),
            self.entries.clone(),
            CrashRecovery::default(),
        ))
    }

    fn message_count(&self) -> std::io::Result<usize> {
        Ok(self.entries.len())
    }
}

// ---------------------------------------------------------------------------
// Scenario 1: append + read in order, v1 readability, deterministic leaf,
// product policy stays in opi-coding-agent.
// ---------------------------------------------------------------------------

#[test]
fn session_facade_hydrates_tip_and_uses_unique_ids_after_open() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("resume-append.jsonl");
    {
        let repo = JsonlSessionRepo::create(&path, header("s1")).unwrap();
        let mut facade = SessionFacade::new(Box::new(repo)).unwrap();
        facade.enqueue_message(user_msg("first")).unwrap();
        facade.flush().unwrap();
    }

    let repo = JsonlSessionRepo::open(&path).unwrap();
    let mut facade = SessionFacade::new(Box::new(repo)).unwrap();
    facade.enqueue_message(user_msg("second")).unwrap();
    facade.flush().unwrap();

    let (_header, entries, _recovery) = facade.load().unwrap();
    let messages: Vec<_> = entries
        .iter()
        .filter_map(|entry| match entry {
            SessionEntry::Message(message) => Some(message),
            _ => None,
        })
        .collect();
    assert_eq!(messages.len(), 2);
    assert_ne!(messages[0].id, messages[1].id);
    assert_eq!(
        messages[1].parent_id.as_deref(),
        Some(messages[0].id.as_str())
    );
}

#[test]
fn extension_state_does_not_become_content_parent_when_enqueued_first() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("extension-first.jsonl");
    let repo = JsonlSessionRepo::create(&path, header("s1")).unwrap();
    let mut facade = SessionFacade::new(Box::new(repo)).unwrap();

    facade.enqueue_extension_state(json!({"n": 1})).unwrap();
    facade.enqueue_message(user_msg("agent-text")).unwrap();
    facade.flush().unwrap();

    let (_header, entries, _recovery) = facade.load().unwrap();
    let message = entries
        .iter()
        .find_map(|entry| match entry {
            SessionEntry::Message(message) => Some(message),
            _ => None,
        })
        .unwrap();
    assert_eq!(message.parent_id, None);
    assert_eq!(
        facade.active_tip().unwrap().as_deref(),
        Some(message.id.as_str())
    );
}

#[test]
fn extension_state_attaches_to_content_tip_without_advancing_it() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("extension-sidecar.jsonl");
    let repo = JsonlSessionRepo::create(&path, header("s1")).unwrap();
    let mut facade = SessionFacade::new(Box::new(repo)).unwrap();

    facade.enqueue_message(user_msg("first")).unwrap();
    facade.enqueue_extension_state(json!({"n": 1})).unwrap();
    facade.enqueue_message(user_msg("second")).unwrap();
    facade.flush().unwrap();

    let (_header, entries, _recovery) = facade.load().unwrap();
    let messages: Vec<_> = entries
        .iter()
        .filter_map(|entry| match entry {
            SessionEntry::Message(message) => Some(message),
            _ => None,
        })
        .collect();
    let state = entries
        .iter()
        .find_map(|entry| match entry {
            SessionEntry::ExtensionState(state) => Some(state),
            _ => None,
        })
        .unwrap();
    assert_eq!(state.parent_id.as_deref(), Some(messages[0].id.as_str()));
    assert_eq!(
        messages[1].parent_id.as_deref(),
        Some(messages[0].id.as_str())
    );
}

#[test]
fn partial_flush_never_persists_message_parented_to_extension_state() {
    let mut facade = SessionFacade::new(Box::new(FailOnAppend::new(2))).unwrap();
    facade.enqueue_extension_state(json!({"n": 1})).unwrap();
    facade
        .enqueue_message(user_msg("message survives first"))
        .unwrap();

    let err = facade.flush().unwrap_err();
    assert!(matches!(err, HarnessError::Write(_)));

    let (_header, entries, _recovery) = facade.load().unwrap();
    assert_eq!(entries.len(), 1);
    match &entries[0] {
        SessionEntry::Message(message) => assert_eq!(message.parent_id, None),
        other => panic!("expected the content message to be written first, got {other:?}"),
    }
}

#[test]
fn session_facade_supports_in_memory_repo_backend() {
    // The SessionRepo trait is usable without a JSONL file: an in-memory
    // backend receives ordered appends and reports them on load.
    let mut facade = SessionFacade::new(Box::new(RecordingSessionRepo::default())).unwrap();
    facade.enqueue_message(user_msg("in-mem")).unwrap();
    facade.enqueue_extension_state(json!({"k": 1})).unwrap();

    let sp = facade.flush().unwrap();
    assert_eq!(sp.pending_after, 0);

    let (_header, entries, _recovery) = facade.load().unwrap();
    assert_eq!(entries.len(), 2);
    assert!(matches!(entries[0], SessionEntry::Message(_)));
    assert!(matches!(entries[1], SessionEntry::ExtensionState(_)));
    assert_eq!(facade.message_count().unwrap(), 2);
}

#[test]
fn session_facade_appends_and_reads_in_order() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ordered.jsonl");
    let repo = JsonlSessionRepo::create(&path, header("s1")).unwrap();
    let mut facade = SessionFacade::new(Box::new(repo)).unwrap();

    facade.enqueue_message(user_msg("hello")).unwrap();
    facade.enqueue_extension_state(json!({"k": "v"})).unwrap();
    assert_eq!(facade.pending_count(), 2);

    let sp = facade.flush().unwrap();
    assert_eq!(sp.pending_before, 2);
    assert_eq!(sp.pending_after, 0);
    assert_eq!(sp.seq, 1);
    assert_eq!(facade.pending_count(), 0);

    let (h, entries, _recovery) = facade.load().unwrap();
    assert_eq!(h.id, "s1");
    assert_eq!(h.version, 1);
    assert_eq!(entries.len(), 2);
    // Agent-emitted message persists before the extension-state write.
    assert!(matches!(entries[0], SessionEntry::Message(_)));
    assert!(matches!(entries[1], SessionEntry::ExtensionState(_)));
    assert_eq!(facade.message_count().unwrap(), 2);
}

/// The key forward-compatibility proof: a v1 session file containing an
/// unknown future entry type (any additive tag this build does not recognize)
/// is read without fatal error. The known entry survives and the unknown entry
/// is reported via `CrashRecovery::unknown_count`, distinct from corruption.
/// This operationalizes Decision D1 (additive-v1) and Success Criteria 1/2/3.
#[test]
fn session_facade_preserves_v1_readability_with_unknown_future_entry() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("future.jsonl");
    {
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "{}", serde_json::to_string(&header("s1")).unwrap()).unwrap();
        let real = SessionEntry::Message(MessageEntry {
            id: "m1".into(),
            parent_id: None,
            timestamp: "0".into(),
            message: user_msg("real"),
        });
        writeln!(f, "{}", serde_json::to_string(&real).unwrap()).unwrap();
        // An entry type this build does not know -- stands in for any additive
        // entry written into a v1 file by a newer opi. Must be a tag no Phase
        // 13 variant uses so it stays genuinely unknown.
        writeln!(
            f,
            r#"{{"type":"some_future_v2_entry","id":"bs1","parent_id":"m1","timestamp":"0","summary":"fork"}}"#
        )
        .unwrap();
    }

    let repo = JsonlSessionRepo::open(&path).unwrap();
    let facade = SessionFacade::new(Box::new(repo)).unwrap();
    let (h, entries, recovery) = facade.load().unwrap();

    // v1 stays readable; the known message survives.
    assert_eq!(h.version, 1);
    assert_eq!(entries.len(), 1);
    assert!(matches!(entries[0], SessionEntry::Message(_)));
    // The unknown entry is reported as unknown (not corrupt), never fatal.
    assert!(
        recovery.unknown_count >= 1,
        "unknown future entry must be reported via unknown_count, got {recovery:?}"
    );
    assert_eq!(
        recovery.corrupt_count, 0,
        "unknown future entry must not be counted as corrupt"
    );
}

#[test]
fn session_facade_reconstructs_active_branch_leaf_deterministically() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("branch.jsonl");
    {
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "{}", serde_json::to_string(&header("s1")).unwrap()).unwrap();
        let m1 = SessionEntry::Message(MessageEntry {
            id: "m1".into(),
            parent_id: None,
            timestamp: "0".into(),
            message: user_msg("first"),
        });
        let m2 = SessionEntry::Message(MessageEntry {
            id: "m2".into(),
            parent_id: Some("m1".into()),
            timestamp: "0".into(),
            message: user_msg("second"),
        });
        let leaf = SessionEntry::Leaf(LeafEntry {
            id: "l1".into(),
            parent_id: Some("m2".into()),
            timestamp: "0".into(),
            entry_id: "m2".into(),
        });
        for e in [m1, m2, leaf] {
            writeln!(f, "{}", serde_json::to_string(&e).unwrap()).unwrap();
        }
    }

    let facade = SessionFacade::new(Box::new(JsonlSessionRepo::open(&path).unwrap())).unwrap();
    let tip = facade.active_tip().unwrap();
    assert_eq!(tip.as_deref(), Some("m2"));
    // Deterministic: a second read over the same repo yields the same tip.
    let tip_again = facade.active_tip().unwrap();
    assert_eq!(tip, tip_again);
}

// ---------------------------------------------------------------------------
// Phase 13.1: additive metadata/context entries attach to the content tip
// without advancing it, survive JSONL round-trip, and never enter branch graph
// reconstruction as content nodes.
// ---------------------------------------------------------------------------

#[test]
fn facade_model_change_records_input_source_provenance() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("input-source.jsonl");
    let repo = JsonlSessionRepo::create(&path, header("s1")).unwrap();
    let mut facade = SessionFacade::new(Box::new(repo)).unwrap();

    // A canonical `provider:model` spec carries the separator; a bare model id
    // does not. The facade records the truthful source of each (Phase 17.5).
    facade
        .enqueue_model_change("anthropic:claude-sonnet-4-5".into())
        .unwrap();
    facade
        .enqueue_model_change("claude-haiku-4".into())
        .unwrap();
    facade.flush().unwrap();

    let (_h, entries, recovery) = facade.load().unwrap();
    assert!(
        recovery.is_clean(),
        "no recovery expected, got {recovery:?}"
    );
    let model_changes: Vec<_> = entries
        .iter()
        .filter_map(|entry| match entry {
            SessionEntry::ModelChange(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(model_changes.len(), 2);
    assert_eq!(
        model_changes[0].input_source,
        Some(opi_agent::session::ModelInputSource::Canonical)
    );
    assert_eq!(
        model_changes[1].input_source,
        Some(opi_agent::session::ModelInputSource::BareNormalized)
    );
}

#[test]
fn facade_metadata_entries_attach_to_tip_without_advancing_it() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("metadata.jsonl");
    let repo = JsonlSessionRepo::create(&path, header("s1")).unwrap();
    let mut facade = SessionFacade::new(Box::new(repo)).unwrap();

    // Establish a content tip with one message.
    facade.enqueue_message(user_msg("first")).unwrap();
    facade.flush().unwrap();
    let tip_before = facade
        .active_tip()
        .unwrap()
        .expect("content tip after first message");

    // Append every Phase 13 metadata/context entry. None may advance the tip.
    facade.enqueue_session_info("renamed".into()).unwrap();
    facade
        .enqueue_model_change("anthropic:claude-sonnet-4-5".into())
        .unwrap();
    facade
        .enqueue_thinking_level_change(ThinkingLevel::Medium)
        .unwrap();
    facade
        .enqueue_label("bug".into(), LabelAction::Add)
        .unwrap();
    facade
        .enqueue_branch_summary("summarized the branch".into())
        .unwrap();
    facade.flush().unwrap();

    // Tip unchanged: metadata did not advance the content tip.
    let tip_after = facade.active_tip().unwrap();
    assert_eq!(
        tip_after.as_deref(),
        Some(tip_before.as_str()),
        "metadata append must not advance the content tip"
    );

    // Every metadata entry is parented to the content tip and survives reload.
    let (_h, entries, recovery) = facade.load().unwrap();
    assert!(
        recovery.is_clean(),
        "no recovery expected, got {recovery:?}"
    );

    let mut saw_session_info = false;
    let mut saw_model_change = false;
    let mut saw_thinking_level_change = false;
    let mut saw_label = false;
    let mut saw_branch_summary = false;
    for entry in &entries {
        match entry {
            SessionEntry::SessionInfo(s) => {
                assert_eq!(s.name, "renamed");
                assert_eq!(s.parent_id.as_deref(), Some(tip_before.as_str()));
                saw_session_info = true;
            }
            SessionEntry::ModelChange(m) => {
                assert_eq!(m.model, "anthropic:claude-sonnet-4-5");
                assert_eq!(m.parent_id.as_deref(), Some(tip_before.as_str()));
                assert_eq!(
                    m.input_source,
                    Some(opi_agent::session::ModelInputSource::Canonical)
                );
                saw_model_change = true;
            }
            SessionEntry::ThinkingLevelChange(t) => {
                assert_eq!(t.level, ThinkingLevel::Medium);
                assert_eq!(t.parent_id.as_deref(), Some(tip_before.as_str()));
                saw_thinking_level_change = true;
            }
            SessionEntry::Label(l) => {
                assert_eq!(l.label, "bug");
                assert_eq!(l.action, LabelAction::Add);
                assert_eq!(l.parent_id.as_deref(), Some(tip_before.as_str()));
                saw_label = true;
            }
            SessionEntry::BranchSummary(b) => {
                assert_eq!(b.summary, "summarized the branch");
                assert_eq!(b.parent_id.as_deref(), Some(tip_before.as_str()));
                saw_branch_summary = true;
            }
            // Content entries and any future additive variant are not metadata.
            _ => {}
        }
    }
    assert!(saw_session_info, "session_info entry must round-trip");
    assert!(saw_model_change, "model_change entry must round-trip");
    assert!(
        saw_thinking_level_change,
        "thinking_level_change entry must round-trip"
    );
    assert!(saw_label, "label entry must round-trip");
    assert!(saw_branch_summary, "branch_summary entry must round-trip");

    // Metadata does not create branch nodes: the active branch still has a
    // single message and the tip is unchanged.
    let messages: Vec<_> = entries
        .iter()
        .filter_map(|e| match e {
            SessionEntry::Message(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(messages.len(), 1, "metadata must not add content messages");
}

#[test]
fn facade_metadata_appends_after_message_in_flush_order() {
    // A message enqueued AFTER metadata must still persist before the metadata
    // writes (PendingWriteKind::Metadata ranks like ExtensionState).
    let mut facade = SessionFacade::new(Box::new(RecordingSessionRepo::default())).unwrap();
    facade.enqueue_message(user_msg("first")).unwrap();
    facade.enqueue_session_info("named".into()).unwrap();
    facade.enqueue_message(user_msg("second")).unwrap();

    facade.flush().unwrap();
    let (_h, entries, _r) = facade.load().unwrap();
    // Both messages precede the session_info write.
    let si_idx = entries
        .iter()
        .position(|e| matches!(e, SessionEntry::SessionInfo(_)))
        .expect("session_info present");
    let last_msg_idx = entries
        .iter()
        .rposition(|e| matches!(e, SessionEntry::Message(_)))
        .expect("at least one message");
    assert!(
        last_msg_idx < si_idx,
        "messages must flush before metadata writes, got msg@{last_msg_idx} vs si@{si_idx}"
    );
}

#[test]
fn facade_metadata_append_does_not_advance_active_tip() {
    // Phase 13.2 DoD: "SessionFacade exposes a named metadata append API and
    // tests compare active_tip before and after metadata append." Every flavor
    // of metadata append parents to the content tip without advancing it.
    let mut facade = SessionFacade::new(Box::new(RecordingSessionRepo::default())).unwrap();
    facade.enqueue_message(user_msg("first")).unwrap();
    facade.flush().unwrap();
    let tip_before = facade.active_tip().unwrap();

    // Append every metadata flavor the facade exposes.
    facade.enqueue_session_info("named".into()).unwrap();
    facade
        .enqueue_model_change("anthropic:claude-b".into())
        .unwrap();
    facade
        .enqueue_thinking_level_change(ThinkingLevel::High)
        .unwrap();
    facade
        .enqueue_label("pinned".into(), LabelAction::Add)
        .unwrap();
    facade
        .enqueue_branch_summary("earlier context".into())
        .unwrap();
    facade.flush().unwrap();

    let tip_after = facade.active_tip().unwrap();
    assert_eq!(
        tip_before, tip_after,
        "metadata appends must not advance the content tip"
    );

    // And the metadata is reachable on the reconstructed active chain via the
    // reusable context API (the tip it was parented to is still the tip).
    let (_h, entries, _r) = facade.load().unwrap();
    let ctx = opi_agent::session_context::reconstruct_context(&entries, &CrashRecovery::default());
    assert_eq!(ctx.active_tip_entry_id.as_deref(), tip_after.as_deref());
    assert_eq!(ctx.session_name.as_deref(), Some("named"));
    assert_eq!(ctx.model.as_deref(), Some("anthropic:claude-b"));
    assert_eq!(ctx.thinking_level, Some(ThinkingLevel::High));
    assert_eq!(ctx.labels, vec!["pinned".to_owned()]);
}

#[test]
fn pending_writes_flush_in_order_at_save_points() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("queue.jsonl");
    let repo = JsonlSessionRepo::create(&path, header("s1")).unwrap();
    let mut facade = SessionFacade::new(Box::new(repo)).unwrap();

    // Enqueue extension state FIRST, then an agent message. Flush order must
    // still place the agent-emitted message before the extension-state write.
    facade.enqueue_extension_state(json!({"n": 1})).unwrap();
    facade.enqueue_message(user_msg("agent-text")).unwrap();

    let sp1 = facade.flush().unwrap();
    assert_eq!(sp1.seq, 1);
    assert_eq!(sp1.pending_after, 0);

    let (_h, entries, _recovery) = facade.load().unwrap();
    assert_eq!(entries.len(), 2);
    assert!(matches!(entries[0], SessionEntry::Message(_)));
    assert!(matches!(entries[1], SessionEntry::ExtensionState(_)));

    // A second save point advances the monotonic sequence.
    facade.enqueue_message(user_msg("second")).unwrap();
    let sp2 = facade.flush().unwrap();
    assert_eq!(sp2.seq, 2);
}

#[test]
fn pending_writes_not_dropped_on_abort() {
    let mut facade = SessionFacade::new(Box::new(FailingSessionRepo)).unwrap();

    facade.enqueue_message(user_msg("a")).unwrap();
    facade.enqueue_extension_state(json!({"x": 1})).unwrap();
    assert_eq!(facade.pending_count(), 2);

    let err = facade.abort().unwrap_err();
    assert!(
        matches!(err, HarnessError::AbortLeftPending(2)),
        "abort must report unflushed pending writes, got {err:?}"
    );
    // Accepted writes remain queued -- never silently discarded.
    assert_eq!(facade.pending_count(), 2);
}

// ---------------------------------------------------------------------------
// Boundary guard: product session directory/CLI policy stays out of opi-agent.
// ---------------------------------------------------------------------------

/// Strip Rust line and (nested) block comments while preserving string/char
/// literal contents, so doc prose naming product types does not trip the guard.
fn strip_rust_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'/' && i + 1 < bytes.len() {
            if bytes[i + 1] == b'/' {
                // line comment -> skip to newline
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            } else if bytes[i + 1] == b'*' {
                // block comment (nestable)
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
        // Skip string/char literal contents so comment markers inside them
        // are not treated as comments and so code-only tokens are scanned.
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

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_rs_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn session_facade_keeps_product_directory_policy_out_of_opi_agent() {
    let opi_agent_src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("opi-agent")
        .join("src");
    let mut files = Vec::new();
    collect_rs_files(&opi_agent_src, &mut files);
    assert!(!files.is_empty(), "expected opi-agent src files to scan");

    // Tokens that are unambiguously coding-agent product policy (CLI flags,
    // directory paths, list/resume/fork/delete commands, the product session
    // coordinator). Generic opi-agent must not own any of them.
    let product_policy_tokens = [
        "OPI_SESSIONS_DIR",
        "session_cli",
        "fork_session",
        "list_sessions",
        "delete_session",
        "resume_session",
        "SessionCoordinator",
        "session_dir",
        "handle_session_cli",
        "format_resume_recovery_warnings",
    ];
    // Sanity tokens that MUST be present in opi-agent so the guard is
    // non-vacuous (the generic seam this task introduces).
    let generic_seam_tokens = ["SessionFacade", "SessionRepo"];

    for file in &files {
        let src = fs::read_to_string(file).unwrap();
        let stripped = strip_rust_comments(&src);
        for token in product_policy_tokens {
            assert!(
                !stripped.contains(token),
                "product-policy token `{token}` leaked into opi-agent non-comment code at {}",
                file.display()
            );
        }
    }

    let harness_src =
        fs::read_to_string(opi_agent_src.join("harness.rs")).expect("opi-agent harness.rs exists");
    let stripped = strip_rust_comments(&harness_src);
    for token in generic_seam_tokens {
        assert!(
            stripped.contains(token),
            "generic seam token `{token}` missing from opi-agent harness.rs (non-vacuous sanity)"
        );
    }
}
