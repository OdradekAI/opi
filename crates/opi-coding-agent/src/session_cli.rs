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
    /// Immutable v2 binding. A v1 source remains read-only and carries no
    /// binding until trusted product assembly creates its parented v2 child.
    pub runtime_input_binding: Option<opi_agent::evidence::RuntimeInputBinding>,
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
    resume_session_path(path, session_id)
}

fn resume_session_path(
    path: PathBuf,
    missing_session: &str,
) -> Result<ResumedSession, SessionCliError> {
    if !path.exists() {
        return Err(SessionCliError::NotFound(missing_session.into()));
    }

    let (header, entries, recovery) = opi_agent::session::SessionReader::read_with_recovery(&path)
        .map_err(|e| SessionCliError::Corrupt(format!("{}: {e}", path.display())))?;
    let runtime_input_binding = header.runtime_input_binding.clone();

    let skipped_entries = recovery.corrupt_count();
    let diagnostics = recovery.diagnostics();

    Ok(ResumedSession {
        header,
        entries,
        runtime_input_binding,
        path,
        skipped_entries,
        recovery,
        diagnostics,
    })
}

/// Fork a session into a new JSONL file and return the fork as a resumed session.
///
/// The source session file is not modified. The fork copies **every entry on
/// the active branch that resume would reconstruct** - content (Message /
/// Compaction) plus all metadata parented to the active chain (Label,
/// ModelChange, ThinkingLevelChange, SessionInfo, BranchSummary,
/// ExtensionState) - then records the source session ID in `parent_session`
/// and writes a fresh `Leaf` pointer at the active tip. File order is
/// preserved so latest-wins metadata semantics match resume exactly.
///
/// Phase 13.3: this keeps fork consistent with the opi-agent context builder
/// (`reconstruct_context`) so resume and fork observe identical context,
/// including branch summaries and other metadata that the prior Message-only
/// copy dropped.
pub fn fork_session(dir: &Path, session_id: &str) -> Result<ResumedSession, SessionCliError> {
    let source = resume_session(dir, session_id)?;
    let binding = source.runtime_input_binding.clone().ok_or_else(|| {
        SessionCliError::Corrupt(
            "legacy session requires trusted runtime-input binding assembly before forking".into(),
        )
    })?;
    fork_loaded_session(dir, source, binding)
}

/// Fork a validated mutable session into a child carrying `runtime_input_binding`.
///
/// The caller supplies the binding derived from the current trusted material
/// inputs. This is used when those inputs changed between runs, before the next
/// provider or tool side effect.
pub fn fork_session_with_runtime_input_binding(
    dir: &Path,
    session_id: &str,
    runtime_input_binding: opi_agent::evidence::RuntimeInputBinding,
) -> Result<ResumedSession, SessionCliError> {
    let source = resume_session(dir, session_id)?;
    fork_loaded_session(dir, source, runtime_input_binding)
}

/// Fork a validated session file into a child carrying `runtime_input_binding`.
///
/// Resume callers already hold an exact file path, which need not use the
/// standard `<session-id>.jsonl` storage name. Preserve that source identity
/// when a changed runtime binding requires a child before the next run.
pub(crate) fn fork_session_path_with_runtime_input_binding(
    source_path: &Path,
    runtime_input_binding: opi_agent::evidence::RuntimeInputBinding,
) -> Result<ResumedSession, SessionCliError> {
    let source = resume_session_path(
        source_path.to_path_buf(),
        &source_path.display().to_string(),
    )?;
    let dir = source
        .path
        .parent()
        .ok_or_else(|| {
            SessionCliError::Corrupt(format!(
                "session source has no parent directory: {}",
                source.path.display()
            ))
        })?
        .to_path_buf();
    fork_loaded_session(&dir, source, runtime_input_binding)
}

fn fork_loaded_session(
    dir: &Path,
    source: ResumedSession,
    runtime_input_binding: opi_agent::evidence::RuntimeInputBinding,
) -> Result<ResumedSession, SessionCliError> {
    // Active-chain entry ids in root->tip order. Keep the content-only tip for
    // the fresh Leaf, but filter copied metadata against the full chain so
    // degraded sessions whose parent walk passes through metadata preserve the
    // same context that resume reconstructs.
    let active_chain: Vec<String> = active_entry_ids(&source.entries);
    let active_tip = active_content_entry_ids(&source.entries).last().cloned();
    let active_set: std::collections::HashSet<&str> =
        active_chain.iter().map(String::as_str).collect();

    let mut entries: Vec<opi_agent::session::SessionEntry> = Vec::new();
    for entry in &source.entries {
        if entry_on_active_chain(entry, &active_set) {
            entries.push(entry.clone());
        }
    }

    let (header, path) =
        new_fork_session_target(dir, &source.header, runtime_input_binding.clone())?;

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
        runtime_input_binding: Some(runtime_input_binding),
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
/// is in `active_set`, which is the full active chain rather than only content
/// ids. Leaf pointers are excluded - the fork writes a single fresh Leaf at
/// the tip instead of copying stale source Leaves.
fn entry_on_active_chain(
    entry: &opi_agent::session::SessionEntry,
    active_set: &std::collections::HashSet<&str>,
) -> bool {
    use opi_agent::session::SessionEntry;

    let metadata_on_active = |parent_id: Option<&str>| -> bool {
        match parent_id {
            None => active_set.is_empty(),
            Some(id) => active_set.contains(id),
        }
    };

    match entry {
        SessionEntry::Message(m) => active_set.contains(m.id.as_str()),
        SessionEntry::Compaction(c) => active_set.contains(c.id.as_str()),
        SessionEntry::Label(l) => metadata_on_active(l.parent_id.as_deref()),
        SessionEntry::ModelChange(m) => metadata_on_active(m.parent_id.as_deref()),
        SessionEntry::ThinkingLevelChange(t) => metadata_on_active(t.parent_id.as_deref()),
        SessionEntry::SessionInfo(s) => metadata_on_active(s.parent_id.as_deref()),
        SessionEntry::BranchSummary(b) => metadata_on_active(b.parent_id.as_deref()),
        SessionEntry::ExtensionState(x) => metadata_on_active(x.parent_id.as_deref()),
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

/// Branch scope for a session export (Phase 13.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportScope {
    /// Render only the active branch via `opi_agent::session_context::reconstruct_context`.
    ActiveBranch,
    /// Render every Message entry in file order across all branches.
    FullTree,
}

/// Options for `--export-session` (Phase 13.5).
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Session id (resolved against the sessions directory) or a path to a
    /// session JSONL file.
    pub session_ref: String,
    /// Output format.
    pub format: crate::cli::ExportFormat,
    /// Output file path. Only this file is written.
    pub output: PathBuf,
    /// Branch scope.
    pub scope: ExportScope,
    /// Include `Message::ToolResult` output content blocks.
    pub include_tool_output: bool,
    /// Include `AssistantContent::Thinking` blocks.
    pub include_thinking: bool,
    /// Redaction mode. Maps to Phase 7 `RedactionMode` plus an explicit
    /// opt-out.
    pub redact: crate::cli::ExportRedactMode,
}

/// Errors from session export (Phase 13.5).
#[derive(Debug, Error)]
pub enum ExportError {
    #[error("session not found: {0}")]
    NotFound(String),
    #[error("session directory error: {0}")]
    Io(#[from] std::io::Error),
    #[error("session file corrupt: {0}")]
    Corrupt(String),
    #[error("export write failed: {0}")]
    Write(String),
}

/// Resolve a session ref (id or path) to a JSONL file path.
///
/// An absolute path or any string containing a path separator is treated as a
/// direct file path; otherwise the value is treated as a session id and
/// resolved against `dir`. Existing-file check is enforced so callers surface
/// `NotFound` before opening.
fn resolve_session_path(dir: &Path, session_ref: &str) -> Result<PathBuf, ExportError> {
    let direct = PathBuf::from(session_ref);
    if direct.is_absolute() || session_ref.contains('/') || session_ref.contains('\\') {
        if !direct.exists() {
            return Err(ExportError::NotFound(session_ref.into()));
        }
        return Ok(direct);
    }
    validate_session_id(session_ref).map_err(|_| ExportError::NotFound(session_ref.into()))?;
    let path = dir.join(format!("{session_ref}.jsonl"));
    if !path.exists() {
        return Err(ExportError::NotFound(session_ref.into()));
    }
    Ok(path)
}

/// Run a session export (Phase 13.5).
///
/// Reads the source session read-only via `SessionReader::read_with_recovery`,
/// reconstructs the requested scope through the opi-agent context API
/// (active branch) or a full-tree walk, applies the redaction policy, and
/// writes ONLY `options.output`. The source session is never opened for
/// writing; on a write failure the source is byte-for-byte unchanged.
pub fn export_session(options: &ExportOptions) -> Result<(), ExportError> {
    let dir = session_dir();
    let path = resolve_session_path(&dir, &options.session_ref)?;

    let (header, entries, recovery) = opi_agent::session::SessionReader::read_with_recovery(&path)
        .map_err(|e| ExportError::Corrupt(format!("{}: {e}", path.display())))?;

    let messages = match options.scope {
        ExportScope::ActiveBranch => {
            let ctx = opi_agent::session_context::reconstruct_context(&entries, &recovery);
            ctx.messages
        }
        ExportScope::FullTree => collect_full_tree_messages(&entries),
    };

    let rendered = match options.format {
        crate::cli::ExportFormat::Markdown => render_markdown(&header, &messages, options),
        crate::cli::ExportFormat::Json => render_json(&header, &messages, options),
    };

    std::fs::write(&options.output, rendered)
        .map_err(|e| ExportError::Write(format!("{}: {e}", options.output.display())))?;
    Ok(())
}

/// Collect every Message entry as an `AgentMessage::Llm` in file order,
/// including messages on sibling branches. This is the full-tree renderer's
/// deterministic walk: it does not apply compaction or branch summaries (those
/// are active-branch-only semantics owned by `reconstruct_context`).
fn collect_full_tree_messages(
    entries: &[opi_agent::session::SessionEntry],
) -> Vec<opi_agent::message::AgentMessage> {
    use opi_agent::session::SessionEntry;
    entries
        .iter()
        .filter_map(|entry| match entry {
            SessionEntry::Message(m) => {
                Some(opi_agent::message::AgentMessage::Llm(m.message.clone()))
            }
            _ => None,
        })
        .collect()
}

/// Apply the export redaction mode to a single text value.
fn redact_export_text(text: &str, mode: crate::cli::ExportRedactMode) -> String {
    match mode {
        crate::cli::ExportRedactMode::None => text.to_owned(),
        crate::cli::ExportRedactMode::Summary => {
            opi_agent::redact_text(text, opi_agent::RedactionMode::Summary)
        }
        crate::cli::ExportRedactMode::Verbose => redact_export_verbose_text(text),
    }
}

fn redact_export_verbose_text(text: &str) -> String {
    let mut redacted = text.to_owned();
    for pattern in opi_agent::SecretRedactor::default().patterns() {
        let Ok(regex) = regex::Regex::new(pattern) else {
            continue;
        };
        redacted = regex.replace_all(&redacted, "[REDACTED]").into_owned();
    }
    redacted
}

fn redact_export_value(
    value: &serde_json::Value,
    mode: crate::cli::ExportRedactMode,
) -> serde_json::Value {
    match mode {
        crate::cli::ExportRedactMode::None => value.clone(),
        crate::cli::ExportRedactMode::Summary => {
            opi_agent::redact(value, opi_agent::RedactionMode::Summary)
        }
        crate::cli::ExportRedactMode::Verbose => {
            opi_agent::redact(value, opi_agent::RedactionMode::Verbose)
        }
    }
}

/// Render the session as deterministic markdown suitable for review/handoff.
fn render_markdown(
    header: &opi_agent::session::SessionHeader,
    messages: &[opi_agent::message::AgentMessage],
    options: &ExportOptions,
) -> String {
    use opi_agent::message::AgentMessage;
    use opi_ai::message::{AssistantContent, InputContent, Message, OutputContent};

    let mut out = String::new();
    out.push_str(&format!("# Session {}\n\n", header.id));
    out.push_str(&format!("- timestamp: {}\n", header.timestamp));
    out.push_str(&format!(
        "- cwd: {}\n",
        redact_export_text(&header.cwd, options.redact)
    ));
    if let Some(parent) = &header.parent_session {
        out.push_str(&format!("- parent_session: {parent}\n"));
    }
    out.push('\n');

    for msg in messages {
        match msg {
            AgentMessage::Llm(Message::User(u)) => {
                out.push_str("## User\n\n");
                for block in &u.content {
                    if let InputContent::Text { text } = block {
                        out.push_str(&redact_export_text(text, options.redact));
                        out.push('\n');
                    }
                }
                out.push('\n');
            }
            AgentMessage::Llm(Message::Assistant(a)) => {
                out.push_str("## Assistant\n\n");
                for block in &a.content {
                    match block {
                        AssistantContent::Text { text } => {
                            out.push_str(&redact_export_text(text, options.redact));
                            out.push('\n');
                        }
                        AssistantContent::Thinking { thinking } if options.include_thinking => {
                            out.push_str("> thinking: ");
                            out.push_str(&redact_export_text(thinking, options.redact));
                            out.push('\n');
                        }
                        AssistantContent::Thinking { .. } => {}
                        AssistantContent::ToolCall { tool_call } => {
                            out.push_str(&format!(
                                "*tool call: {} ({})* {}\n",
                                tool_call.name,
                                tool_call.id,
                                redact_export_text(&tool_call.arguments, options.redact)
                            ));
                        }
                        // AssistantContent is non_exhaustive.
                        _ => {}
                    }
                }
                out.push('\n');
            }
            AgentMessage::Llm(Message::ToolResult(t)) if options.include_tool_output => {
                out.push_str(&format!("## Tool: {}\n\n", t.tool_name));
                for block in &t.content {
                    if let OutputContent::Text { text } = block {
                        out.push_str(&redact_export_text(text, options.redact));
                        out.push('\n');
                    }
                }
                out.push('\n');
            }
            AgentMessage::Llm(Message::ToolResult(_)) => {}
            AgentMessage::CompactionSummary(c) => {
                out.push_str("## Compaction Summary\n\n");
                out.push_str(&redact_export_text(&c.summary, options.redact));
                out.push_str(&format!(
                    "\n\n_(kept from {}, tokens {} -> {})_\n\n",
                    c.first_kept_entry_id, c.tokens_before, c.tokens_after
                ));
            }
            AgentMessage::BranchSummary(b) => {
                out.push_str("## Branch Summary\n\n");
                out.push_str(&redact_export_text(&b.summary, options.redact));
                out.push('\n');
                if !b.parent_session_id.is_empty() {
                    out.push_str(&format!("\n_parent session: {}_\n", b.parent_session_id));
                }
                out.push('\n');
            }
            AgentMessage::Custom(c) => {
                out.push_str(&format!("## Custom ({})\n\n", c.kind));
                out.push_str(&redact_export_value(&c.data, options.redact).to_string());
                out.push('\n');
            }
            // AgentMessage is non_exhaustive; future variants are omitted from
            // the markdown transcript rather than crashing the export.
            _ => {}
        }
    }

    out
}

/// Render the session as deterministic JSON for local tooling. Object keys are
/// emitted via serde's default BTreeMap-backed mapping so output is stable
/// regardless of insertion order.
fn render_json(
    header: &opi_agent::session::SessionHeader,
    messages: &[opi_agent::message::AgentMessage],
    options: &ExportOptions,
) -> String {
    use opi_agent::message::AgentMessage;
    use opi_ai::message::{AssistantContent, InputContent, Message, OutputContent};

    let mut header_obj = serde_json::Map::new();
    header_obj.insert("id".into(), serde_json::json!(header.id));
    header_obj.insert("timestamp".into(), serde_json::json!(header.timestamp));
    header_obj.insert(
        "cwd".into(),
        serde_json::json!(redact_export_text(&header.cwd, options.redact)),
    );
    if let Some(parent) = &header.parent_session {
        header_obj.insert("parent_session".into(), serde_json::json!(parent));
    }

    let mut msgs_arr = Vec::with_capacity(messages.len());
    for msg in messages {
        let entry = match msg {
            AgentMessage::Llm(Message::User(u)) => {
                let mut content = Vec::new();
                for block in &u.content {
                    if let InputContent::Text { text } = block {
                        content.push(serde_json::json!({
                            "type": "text",
                            "text": redact_export_text(text, options.redact),
                        }));
                    }
                }
                serde_json::json!({
                    "role": "user",
                    "content": content,
                })
            }
            AgentMessage::Llm(Message::Assistant(a)) => {
                let mut content = Vec::new();
                for block in &a.content {
                    match block {
                        AssistantContent::Text { text } => {
                            content.push(serde_json::json!({
                                "type": "text",
                                "text": redact_export_text(text, options.redact),
                            }));
                        }
                        AssistantContent::Thinking { thinking } if options.include_thinking => {
                            content.push(serde_json::json!({
                                "type": "thinking",
                                "thinking": redact_export_text(thinking, options.redact),
                            }));
                        }
                        AssistantContent::Thinking { .. } => {}
                        AssistantContent::ToolCall { tool_call } => {
                            content.push(serde_json::json!({
                                "type": "tool_call",
                                "id": tool_call.id,
                                "name": tool_call.name,
                                "arguments": redact_export_text(&tool_call.arguments, options.redact),
                            }));
                        }
                        // AssistantContent is non_exhaustive.
                        _ => {}
                    }
                }
                serde_json::json!({
                    "role": "assistant",
                    "model": a.model,
                    "provider": a.provider,
                    "content": content,
                })
            }
            AgentMessage::Llm(Message::ToolResult(t)) if options.include_tool_output => {
                let mut content = Vec::new();
                for block in &t.content {
                    if let OutputContent::Text { text } = block {
                        content.push(serde_json::json!({
                            "type": "text",
                            "text": redact_export_text(text, options.redact),
                        }));
                    }
                }
                serde_json::json!({
                    "role": "tool_result",
                    "tool_call_id": t.tool_call_id,
                    "tool_name": t.tool_name,
                    "is_error": t.is_error,
                    "content": content,
                })
            }
            AgentMessage::Llm(Message::ToolResult(_)) => {
                continue;
            }
            AgentMessage::CompactionSummary(c) => serde_json::json!({
                "role": "compaction_summary",
                "summary": redact_export_text(&c.summary, options.redact),
                "first_kept_entry_id": c.first_kept_entry_id,
                "tokens_before": c.tokens_before,
                "tokens_after": c.tokens_after,
            }),
            AgentMessage::BranchSummary(b) => serde_json::json!({
                "role": "branch_summary",
                "summary": redact_export_text(&b.summary, options.redact),
                "parent_session_id": b.parent_session_id,
                "entry_count": b.entry_count,
            }),
            AgentMessage::Custom(c) => serde_json::json!({
                "role": "custom",
                "kind": c.kind,
                "data": redact_export_value(&c.data, options.redact),
                "include_in_llm_context": c.include_in_llm_context,
            }),
            // AgentMessage is non_exhaustive; future variants are skipped.
            _ => continue,
        };
        msgs_arr.push(entry);
    }

    let root = serde_json::json!({
        "session": serde_json::Value::Object(header_obj),
        "messages": serde_json::Value::Array(msgs_arr),
    });
    serde_json::to_string_pretty(&root).unwrap_or_else(|_| "{}".into())
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
        match resume_session(&dir, id) {
            Ok(session) => {
                eprintln!(
                    "Preparing fork of session {id} ({} entries, cwd: {})",
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
    runtime_input_binding: opi_agent::evidence::RuntimeInputBinding,
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
        let header = opi_agent::session::SessionHeader::new_with_runtime_input_binding(
            id,
            now_iso(),
            source.cwd.clone(),
            Some(source.id.clone()),
            runtime_input_binding.clone(),
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
/// (with usage) to seed its compaction buffer - a different concern from
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

/// Return entries on the same active chain selected by
/// `opi_agent::session_context::reconstruct_context`.
///
/// The opi-agent context API owns active-tip fallback and parent-walk
/// semantics. Product code delegates here so fork, coordinator seeding, and
/// resume observe the same degraded-session behavior.
pub(crate) fn select_ordered_entries(
    entries: &[opi_agent::session::SessionEntry],
) -> Vec<&opi_agent::session::SessionEntry> {
    use std::collections::HashMap;

    let by_id: HashMap<&str, &opi_agent::session::SessionEntry> = entries
        .iter()
        .map(|entry| (entry.entry_id(), entry))
        .collect();
    active_entry_ids(entries)
        .into_iter()
        .filter_map(|id| by_id.get(id.as_str()).copied())
        .collect()
}

pub(crate) fn active_entry_ids(entries: &[opi_agent::session::SessionEntry]) -> Vec<String> {
    opi_agent::session_context::active_chain_entry_ids(entries)
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

    let active_ids: HashSet<String> = active_entry_ids(entries).into_iter().collect();
    if active_ids.is_empty() {
        return entries.iter().rev().find_map(|entry| match entry {
            SessionEntry::ExtensionState(state) if state.parent_id.is_none() => Some(state),
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

#[cfg(test)]
mod tests {
    use super::*;

    const KEY_CANARY: &str = "sk-ant-FAKE1234567890abcdefghijklmnop";

    fn test_header() -> opi_agent::session::SessionHeader {
        opi_agent::session::SessionHeader::new_for_test(
            "custom-export".into(),
            "2026-07-06T00:00:00Z".into(),
            "/repo".into(),
            None,
        )
    }

    fn test_export_options(format: crate::cli::ExportFormat) -> ExportOptions {
        ExportOptions {
            session_ref: "custom-export".into(),
            format,
            output: std::path::PathBuf::from("unused"),
            scope: ExportScope::ActiveBranch,
            include_tool_output: true,
            include_thinking: true,
            redact: crate::cli::ExportRedactMode::Summary,
        }
    }

    fn custom_message() -> opi_agent::message::AgentMessage {
        opi_agent::message::AgentMessage::Custom(opi_agent::message::CustomAgentMessage {
            kind: "test-extension".into(),
            data: serde_json::json!({
                "api_key": KEY_CANARY,
                "kept": "ordinary metadata"
            }),
            include_in_llm_context: false,
        })
    }

    #[test]
    fn markdown_export_redacts_custom_message_data() {
        let rendered = render_markdown(
            &test_header(),
            &[custom_message()],
            &test_export_options(crate::cli::ExportFormat::Markdown),
        );

        assert!(
            !rendered.contains(KEY_CANARY),
            "custom message data must be redacted in markdown export: {rendered}"
        );
        assert!(rendered.contains("[REDACTED]"));
        assert!(rendered.contains("ordinary metadata"));
    }

    #[test]
    fn json_export_redacts_custom_message_data() {
        let rendered = render_json(
            &test_header(),
            &[custom_message()],
            &test_export_options(crate::cli::ExportFormat::Json),
        );

        assert!(
            !rendered.contains(KEY_CANARY),
            "custom message data must be redacted in JSON export: {rendered}"
        );
        let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        let data = &value["messages"][0]["data"];
        assert_eq!(data["api_key"], "[REDACTED]");
        assert_eq!(data["kept"], "ordinary metadata");
    }
}
