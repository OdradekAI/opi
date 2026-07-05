//! Session management CLI operations (task 2.7).
//!
//! Provides list/resume/delete operations for session JSONL files stored in
//! the platform-specific sessions directory (S9.2).

use std::path::{Path, PathBuf};

use opi_agent::{Diagnostic, RedactionMode};
use thiserror::Error;

/// Errors from session CLI operations.
#[derive(Debug, Error)]
pub enum SessionCliError {
    #[error("session not found: {0}")]
    NotFound(String),
    #[error("session directory error: {0}")]
    Io(#[from] std::io::Error),
    #[error("session file corrupt: {0}")]
    Corrupt(String),
}

/// Summary of a session for display purposes.
///
/// Phase 13.3 extends this beyond the header-derived fields to carry the
/// reconstructed active-branch metadata (name, labels, active_branch, model,
/// thinking) so `--list-sessions --json` can emit stable, deterministic rows
/// for tooling and Phase 14 handoff.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub timestamp: String,
    pub cwd: String,
    pub parent_session: Option<String>,
    /// Latest `session_info` name on the active branch, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Active label set (Add/Remove applied in file order). UI-visible.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    /// Active branch tip entry id (the last valid Leaf target), if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_branch: Option<String>,
    /// Latest `model_change` model spec on the active branch, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Latest `thinking_level_change` level on the active branch, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<opi_agent::session_event::ThinkingLevel>,
}

/// Result of a resume operation.
pub struct ResumedSession {
    pub header: opi_agent::session::SessionHeader,
    pub entries: Vec<opi_agent::session::SessionEntry>,
    /// Filesystem path of the resumed session JSONL file. Used by the harness
    /// to open the file in append mode instead of creating a new session.
    pub path: PathBuf,
    /// Number of corrupt/unparseable entries skipped during load.
    pub skipped_entries: usize,
    /// Structured crash recovery observed while reading the session. The
    /// harness passes this to `opi_agent::session_context::reconstruct_context`
    /// so load-time diagnostics and missing-parent warnings are unified.
    pub recovery: opi_agent::session::CrashRecovery,
    /// Structured recovery diagnostics produced while reading the session
    /// (a snapshot of `recovery.diagnostics()` at load time).
    pub diagnostics: Vec<Diagnostic>,
}

/// Format redacted warning lines for degraded session recovery outcomes.
pub fn format_resume_recovery_warnings(session: &ResumedSession) -> Vec<String> {
    session
        .diagnostics
        .iter()
        .map(|diagnostic| {
            let payload = diagnostic.redacted_payload(RedactionMode::Summary);
            let mut line = format!(
                "opi: warning: {}::{}: {}",
                payload.source, payload.code, payload.message
            );
            if let Some(action) = payload.action {
                line.push_str(&format!(" (action: {action})"));
            }
            line
        })
        .collect()
}

/// Return the platform-specific session storage directory (S9.2).
///
/// Checks `OPI_SESSIONS_DIR` env var first (for testing), then falls back to
/// the platform default.
///
/// Unix: `~/.local/share/opi/sessions/`
/// Windows: `%LOCALAPPDATA%\opi\sessions\`
pub fn session_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("OPI_SESSIONS_DIR") {
        return PathBuf::from(dir);
    }
    if cfg!(windows) {
        std::env::var("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("opi").join("sessions"))
            .unwrap_or_else(|_| PathBuf::from(".opi").join("sessions"))
    } else {
        dirs_home()
            .map(|h| h.join(".local").join("share").join("opi").join("sessions"))
            .unwrap_or_else(|| PathBuf::from(".opi").join("sessions"))
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

/// Validate that a session ID is safe to use as a filename component.
/// Rejects empty strings, path separators, and `..` traversal.
fn validate_session_id(id: &str) -> Result<(), SessionCliError> {
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(SessionCliError::NotFound(id.into()));
    }
    Ok(())
}

/// List all sessions in the given directory.
///
/// Reads each `.jsonl` file fully and reconstructs active-branch metadata
/// through the opi-agent context API (Phase 13.3), so each row carries the
/// latest `session_info` name, active label set, active branch tip, latest
/// `model_change`, and latest `thinking_level_change` observed on the active
/// branch. Files whose header cannot be parsed are skipped; files with corrupt
/// middle entries still surface (their entries are read with recovery).
pub fn list_sessions(dir: &Path) -> Result<Vec<SessionInfo>, SessionCliError> {
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut sessions = Vec::new();
    let entries = std::fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }

        let (header, entries, recovery) =
            match opi_agent::session::SessionReader::read_with_recovery(&path) {
                Ok(t) => t,
                Err(_) => continue,
            };

        let ctx = opi_agent::session_context::reconstruct_context(&entries, &recovery);

        sessions.push(SessionInfo {
            id: header.id,
            timestamp: header.timestamp,
            cwd: header.cwd,
            parent_session: header.parent_session,
            name: ctx.session_name,
            labels: ctx.labels,
            active_branch: ctx.active_tip_entry_id,
            model: ctx.model,
            thinking: ctx.thinking_level,
        });
    }

    sessions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(sessions)
}

/// Resume a session by reading its header and entries.
pub fn resume_session(dir: &Path, session_id: &str) -> Result<ResumedSession, SessionCliError> {
    validate_session_id(session_id)?;
    let path = dir.join(format!("{session_id}.jsonl"));
    if !path.exists() {
        return Err(SessionCliError::NotFound(session_id.into()));
    }

    let (header, entries, recovery) = opi_agent::session::SessionReader::read_with_recovery(&path)
        .map_err(|e| SessionCliError::Corrupt(format!("{}: {e}", path.display())))?;

    let skipped_entries = recovery.corrupt_count();
    let diagnostics = recovery.diagnostics();

    Ok(ResumedSession {
        header,
        entries,
        path,
        skipped_entries,
        recovery,
        diagnostics,
    })
}

/// Fork a session into a new JSONL file and return the fork as a resumed session.
///
/// The source session file is not modified. The fork copies **every entry on
/// the active branch that resume would reconstruct** — content (Message /
/// Compaction) plus all metadata parented to the active chain (Label,
/// ModelChange, ThinkingLevelChange, SessionInfo, BranchSummary,
/// ExtensionState) — then records the source session ID in `parent_session`
/// and writes a fresh `Leaf` pointer at the active tip. File order is
/// preserved so latest-wins metadata semantics match resume exactly.
///
/// Phase 13.3: this keeps fork consistent with the opi-agent context builder
/// (`reconstruct_context`) so resume and fork observe identical context,
/// including branch summaries and other metadata that the prior Message-only
/// copy dropped.
pub fn fork_session(dir: &Path, session_id: &str) -> Result<ResumedSession, SessionCliError> {
    let source = resume_session(dir, session_id)?;

    // Active-chain content entry ids in root->tip order. Metadata parented to
    // any of these is on the active branch and must be copied so the fork
    // reconstructs the same context resume would observe.
    let active_chain: Vec<String> = active_content_entry_ids(&source.entries);
    let active_tip = active_chain.last().cloned();
    let active_set: std::collections::HashSet<&str> =
        active_chain.iter().map(String::as_str).collect();

    let mut entries: Vec<opi_agent::session::SessionEntry> = Vec::new();
    for entry in &source.entries {
        if entry_on_active_chain(entry, &active_set) {
            entries.push(entry.clone());
        }
    }

    let (header, path) = new_fork_session_target(dir, &source.header)?;

    std::fs::create_dir_all(dir)?;
    let mut writer = opi_agent::session::SessionWriter::create(&path, header.clone())?;
    for entry in &entries {
        writer.append(entry)?;
    }
    // Write a fresh Leaf at the active tip so the forked session resumes from
    // the same branch point as the source.
    if let Some(tip) = active_tip {
        let leaf = opi_agent::session::SessionEntry::Leaf(opi_agent::session::LeafEntry {
            id: format!("leaf-fork-{}", generate_session_id()),
            parent_id: Some(tip.clone()),
            timestamp: now_iso(),
            entry_id: tip,
        });
        writer.append(&leaf)?;
        entries.push(leaf);
    }
    drop(writer);

    Ok(ResumedSession {
        header,
        entries,
        path,
        skipped_entries: 0,
        recovery: opi_agent::session::CrashRecovery::default(),
        diagnostics: Vec::new(),
    })
}

/// Whether `entry` lies on the active branch.
///
/// Content entries (Message / Compaction) qualify when their own id is in
/// `active_set`. Metadata entries (Label, ModelChange, ThinkingLevelChange,
/// SessionInfo, BranchSummary, ExtensionState) qualify when their `parent_id`
/// is in `active_set`. Leaf pointers are excluded — the fork writes a single
/// fresh Leaf at the tip instead of copying stale source Leaves.
fn entry_on_active_chain(
    entry: &opi_agent::session::SessionEntry,
    active_set: &std::collections::HashSet<&str>,
) -> bool {
    use opi_agent::session::SessionEntry;

    match entry {
        SessionEntry::Message(m) => active_set.contains(m.id.as_str()),
        SessionEntry::Compaction(c) => active_set.contains(c.id.as_str()),
        SessionEntry::Label(l) => l
            .parent_id
            .as_deref()
            .is_some_and(|id| active_set.contains(id)),
        SessionEntry::ModelChange(m) => m
            .parent_id
            .as_deref()
            .is_some_and(|id| active_set.contains(id)),
        SessionEntry::ThinkingLevelChange(t) => t
            .parent_id
            .as_deref()
            .is_some_and(|id| active_set.contains(id)),
        SessionEntry::SessionInfo(s) => s
            .parent_id
            .as_deref()
            .is_some_and(|id| active_set.contains(id)),
        SessionEntry::BranchSummary(b) => b
            .parent_id
            .as_deref()
            .is_some_and(|id| active_set.contains(id)),
        SessionEntry::ExtensionState(x) => x
            .parent_id
            .as_deref()
            .is_some_and(|id| active_set.contains(id)),
        SessionEntry::Leaf(_) => false,
        // Future entry variants are off-chain for fork purposes; the context
        // builder decides whether they enter LLM context at runtime.
        _ => false,
    }
}

/// Delete a session file by ID.
pub fn delete_session(dir: &Path, session_id: &str) -> Result<(), SessionCliError> {
    validate_session_id(session_id)?;
    let path = dir.join(format!("{session_id}.jsonl"));
    if !path.exists() {
        return Err(SessionCliError::NotFound(session_id.into()));
    }
    std::fs::remove_file(&path)?;
    Ok(())
}

/// Format a list of session info for stdout display (text mode).
pub fn format_sessions(sessions: &[SessionInfo]) -> String {
    if sessions.is_empty() {
        return String::new();
    }

    let mut lines = Vec::new();
    for s in sessions {
        let mut line = format!("{}  {}  {}", s.id, s.timestamp, s.cwd);
        if let Some(parent) = &s.parent_session {
            line.push_str(&format!("  (parent: {parent})"));
        }
        lines.push(line);
    }
    lines.join("\n")
}

/// Format a list of session info as a deterministic JSON array (Phase 13.3).
///
/// Used by `--list-sessions --json`. Each row carries id, cwd, timestamp,
/// parent_session, and (when present) name, labels, active_branch, model, and
/// thinking metadata reconstructed from the active branch. Empty optional
/// fields are omitted from the wire shape for stable, forward-compatible
/// output.
pub fn format_sessions_json(sessions: &[SessionInfo]) -> String {
    serde_json::to_string_pretty(sessions).unwrap_or_else(|_| "[]".into())
}

/// Handle session CLI dispatch.
///
/// Returns `(handled, Some(ResumedSession))` for `--resume`,
/// `(true, None)` for list/delete (caller should exit),
/// `(false, None)` if no session command was given (normal execution continues).
///
/// `json` selects the `--list-sessions --json` output shape: when true and
/// `list` is set, rows are printed as a JSON array (one row per line via
/// `println!` of the pretty-printed array). Resume/fork/delete ignore `json`
/// and continue to print human-readable status to stderr so NDJSON stdout
/// stays clean.
pub fn handle_session_cli(
    list: bool,
    json: bool,
    resume: Option<&str>,
    fork: Option<&str>,
    delete: Option<&str>,
) -> Result<(bool, Option<ResumedSession>), i32> {
    let dir = session_dir();

    if list {
        match list_sessions(&dir) {
            Ok(sessions) => {
                if json {
                    let output = format_sessions_json(&sessions);
                    println!("{output}");
                } else {
                    let output = format_sessions(&sessions);
                    if !output.is_empty() {
                        println!("{output}");
                    }
                }
                Ok((true, None))
            }
            Err(e) => {
                eprintln!("opi: {e}");
                Err(1)
            }
        }
    } else if let Some(id) = resume {
        match resume_session(&dir, id) {
            Ok(session) => {
                // Print to stderr so it doesn't corrupt NDJSON stdout in --json mode.
                eprintln!(
                    "Resuming session {} ({} entries, cwd: {})",
                    session.header.id,
                    session.entries.len(),
                    session.header.cwd,
                );
                for warning in format_resume_recovery_warnings(&session) {
                    eprintln!("{warning}");
                }
                Ok((true, Some(session)))
            }
            Err(e) => {
                eprintln!("opi: {e}");
                Err(1)
            }
        }
    } else if let Some(id) = fork {
        match fork_session(&dir, id) {
            Ok(session) => {
                eprintln!(
                    "Forked session {id} -> {} ({} entries, cwd: {})",
                    session.header.id,
                    session.entries.len(),
                    session.header.cwd,
                );
                Ok((true, Some(session)))
            }
            Err(e) => {
                eprintln!("opi: {e}");
                Err(1)
            }
        }
    } else if let Some(id) = delete {
        match delete_session(&dir, id) {
            Ok(()) => {
                println!("Deleted session {id}");
                Ok((true, None))
            }
            Err(e) => {
                eprintln!("opi: {e}");
                Err(1)
            }
        }
    } else {
        Ok((false, None))
    }
}

fn new_fork_session_target(
    dir: &Path,
    source: &opi_agent::session::SessionHeader,
) -> Result<(opi_agent::session::SessionHeader, PathBuf), SessionCliError> {
    for suffix in 0..1000 {
        let base = generate_session_id();
        let id = if suffix == 0 {
            base
        } else {
            format!("{base}-{suffix}")
        };
        let path = dir.join(format!("{id}.jsonl"));
        if path.exists() {
            continue;
        }
        let header = opi_agent::session::SessionHeader::new(
            id,
            now_iso(),
            source.cwd.clone(),
            Some(source.id.clone()),
        );
        return Ok((header, path));
    }

    Err(SessionCliError::Io(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "failed to allocate unique fork session id",
    )))
}

fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{ts:x}")
}

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    let tod = secs % 86400;
    let h = tod / 3600;
    let m = (tod % 3600) / 60;
    let s = tod % 60;
    let (y, mo, d) = days_to_ymd(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let diy = if is_leap(year) { 366 } else { 365 };
        if days < diy {
            break;
        }
        days -= diy;
        year += 1;
    }

    let md = [
        31,
        if is_leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0u64;
    for &d in &md {
        if days < d {
            break;
        }
        days -= d;
        month += 1;
    }
    (year, month + 1, days + 1)
}

fn is_leap(y: u64) -> bool {
    (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400)
}

/// Reconstruct agent messages from session entries for resume.
///
/// Thin ergonomic facade over
/// [`opi_agent::session_context::reconstruct_context`]: returns just the
/// agent message buffer for a clean load (no recovery diagnostics). Callers
/// that need recovery diagnostics, missing-parent warnings, model/thinking
/// metadata, or labels should call the opi-agent API directly and read those
/// fields off [`opi_agent::session_context::ReconstructedContext`].
///
/// Phase 13.3 retired the product-only walker that used to live here; this
/// facade delegates to the opi-agent context builder so resume/fork/list all
/// share one deterministic reconstruction path. The branch-order selector
/// (`select_ordered_entries`) is retained separately because
/// `SessionCoordinator::open_existing` needs the raw active-chain entries
/// (with usage) to seed its compaction buffer — a different concern from
/// building agent messages.
pub fn reconstruct_context(
    entries: &[opi_agent::session::SessionEntry],
) -> Vec<opi_agent::message::AgentMessage> {
    opi_agent::session_context::reconstruct_context(
        entries,
        &opi_agent::session::CrashRecovery::default(),
    )
    .messages
}

/// Return session entries ordered by the active branch.
///
/// When the session contains `Leaf` pointer entries, the last Leaf's
/// `entry_id` is used as the branch tip and the parent chain is walked
/// backward to collect only the active-branch entries (root to tip).
/// Without Leaves, all Message/Compaction entries are returned in file order
/// (legacy linear sessions). This is the shared ordering logic used by both
/// `reconstruct_context` (Agent message buffer) and
/// `SessionCoordinator::open_existing` (compaction buffer).
pub(crate) fn select_ordered_entries(
    entries: &[opi_agent::session::SessionEntry],
) -> Vec<&opi_agent::session::SessionEntry> {
    use opi_agent::session::SessionEntry;

    let last_leaf_tip: Option<&str> = entries.iter().rev().find_map(|e| match e {
        SessionEntry::Leaf(l) => Some(l.entry_id.as_str()),
        _ => None,
    });

    match last_leaf_tip {
        Some(tip) => walk_active_branch(entries, tip),
        None => entries
            .iter()
            .filter(|e| content_entry_id(e).is_some())
            .collect(),
    }
}

pub(crate) fn active_content_entry_ids(
    entries: &[opi_agent::session::SessionEntry],
) -> Vec<String> {
    select_ordered_entries(entries)
        .into_iter()
        .filter_map(content_entry_id)
        .map(str::to_owned)
        .collect()
}

pub(crate) fn latest_extension_state_entry_for_active_branch(
    entries: &[opi_agent::session::SessionEntry],
) -> Option<&opi_agent::session::ExtensionStateEntry> {
    use std::collections::HashSet;

    use opi_agent::session::SessionEntry;

    let active_ids: HashSet<String> = active_content_entry_ids(entries).into_iter().collect();
    if active_ids.is_empty() {
        return entries.iter().rev().find_map(|entry| match entry {
            SessionEntry::ExtensionState(state) => Some(state),
            _ => None,
        });
    }

    entries.iter().rev().find_map(|entry| match entry {
        SessionEntry::ExtensionState(state)
            if state
                .parent_id
                .as_deref()
                .is_some_and(|id| active_ids.contains(id)) =>
        {
            Some(state)
        }
        _ => None,
    })
}

fn content_entry_id(entry: &opi_agent::session::SessionEntry) -> Option<&str> {
    use opi_agent::session::SessionEntry;

    match entry {
        SessionEntry::Message(m) => Some(m.id.as_str()),
        SessionEntry::Compaction(c) => Some(c.id.as_str()),
        _ => None,
    }
}

/// Walk the active branch backward from `tip_entry_id`, returning entries
/// from root to tip (ancestors first). `Leaf` entries themselves are
/// excluded from the result — they are pointers, not content.
///
/// If the tip id is not found, returns an empty vector; callers fall back
/// to legacy behavior or treat the resume as empty depending on context.
fn walk_active_branch<'a>(
    entries: &'a [opi_agent::session::SessionEntry],
    tip_entry_id: &str,
) -> Vec<&'a opi_agent::session::SessionEntry> {
    use std::collections::HashMap;

    use opi_agent::session::SessionEntry;

    let mut by_id: HashMap<&str, &SessionEntry> = HashMap::new();
    for entry in entries {
        let id = match entry {
            SessionEntry::Message(m) => Some(m.id.as_str()),
            SessionEntry::Compaction(c) => Some(c.id.as_str()),
            // Leaf pointers are excluded from the chain; the tip references
            // a Message/Compaction directly.
            SessionEntry::Leaf(_) => None,
            _ => None,
        };
        if let Some(id) = id {
            by_id.insert(id, entry);
        }
    }

    let mut chain: Vec<&SessionEntry> = Vec::new();
    let mut cursor: Option<&str> = Some(tip_entry_id);
    let mut visited: std::collections::HashSet<&str> = std::collections::HashSet::new();
    while let Some(id) = cursor {
        if !visited.insert(id) {
            // Cycle in parent_id graph: stop walking to avoid an infinite
            // loop on a corrupt file.
            break;
        }
        let Some(entry) = by_id.get(id).copied() else {
            break;
        };
        chain.push(entry);
        cursor = match entry {
            SessionEntry::Message(m) => m.parent_id.as_deref(),
            SessionEntry::Compaction(c) => c.parent_id.as_deref(),
            _ => None,
        };
    }
    chain.reverse();
    chain
}
