//! Session JSONL storage — format version 1, additive entry model (S9.3,
//! Phase 13.1).
//!
//! Append-only, versioned JSONL format for session persistence. The first line
//! is a header; subsequent lines are tree entries forming a conversation tree.
//!
//! # Phase 13.1 entry model and compatibility contract
//!
//! Phase 13 keeps **header version 1** with **additive** entry variants. The
//! JSONL structure is unchanged; Phase 13 only adds new `type` tags
//! (`session_info`, `model_change`, `thinking_level_change`, `label`,
//! `branch_summary`). A version bump is reserved for a future breaking
//! structural change; v1 files written by older opi builds remain readable
//! and resumable without a migration command.
//!
//! `SessionEntry` is `#[non_exhaustive]` and [`SessionReader`] distinguishes
//! three line outcomes during load:
//! - known `type` tag → deserialized into the matching variant;
//! - **unknown** `type` tag (a valid JSON object whose `type` this build does
//!   not recognize) → skipped and counted in [`CrashRecovery::unknown_count`],
//!   never fatal — this is the forward-compatibility path for entries a newer
//!   opi might write;
//! - **corrupt** line (invalid JSON, or a JSON object without a `type` field)
//!   → skipped and counted in [`CrashRecovery::corrupt_count`].
//!
//! `branch_summary`, `session_info`, `model_change`, `thinking_level_change`,
//! and `label` are metadata/context attachments: they are parented to the
//! current content tip but do **not** advance it, and they do not enter the
//! branch graph in [`crate::session_branch::SessionTree`]. Whether and how
//! `branch_summary` enters provider LLM context is owned by the Phase 13
//! context builder (task 13.2), not by this entry model.
//!
//! `custom_message` is **deferred**: Phase 13 does not add a dedicated
//! `custom_message` durable entry variant. Extension-provided messages persist
//! via the existing `extension_state` entry and the `include_in_llm_context`
//! flag on the in-memory `AgentMessage::Custom` variant. Provider-context
//! injection for custom messages is owned by the context builder (13.2) and
//! product paths (13.4), not by the entry model.
//!
//! opi sessions are **not** pi-session-v3 compatible. opi learns from pi's
//! append-only tree/context shape but does not promise to read or write
//! arbitrary pi session files. See [`SESSION_FORMAT_POLICY`].

use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Current session format version.
///
/// Phase 13 keeps version 1 with additive entries. See the module docs for the
/// full compatibility policy.
pub const FORMAT_VERSION: u32 = 1;

/// Human-readable session format/compatibility policy. Asserted by tests so the
/// pi-compatibility disclaimer cannot drift silently. Phase 13 keeps this in
/// sync with the module-level documentation above.
pub const SESSION_FORMAT_POLICY: &str = "opi session format: version 1 (additive entries). \
 opi sessions are not pi-session-v3 compatible; opi learns from pi's append-only \
 tree/context shape but does not read or write arbitrary pi session files. Unknown \
 future entry types are skipped on load and reported via CrashRecovery::unknown_count.";

/// Session header — the first line of a JSONL file (S9.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHeader {
    #[serde(rename = "type")]
    pub type_: String,
    pub version: u32,
    pub id: String,
    pub timestamp: String,
    pub cwd: String,
    pub parent_session: Option<String>,
}

impl SessionHeader {
    pub fn new(id: String, timestamp: String, cwd: String, parent_session: Option<String>) -> Self {
        Self {
            type_: "session".to_owned(),
            version: FORMAT_VERSION,
            id,
            timestamp,
            cwd,
            parent_session,
        }
    }
}

/// A message tree entry (S9.3 `message` type).
///
/// The `message` field uses the provider-facing `Message` type (S7.1), not
/// `AgentMessage`. Each S9.3 entry type maps to its own payload structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEntry {
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub message: opi_ai::message::Message,
}

/// A compaction tree entry (S9.3 `compaction` type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionEntry {
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub summary: String,
    pub first_kept_entry_id: String,
    pub tokens_before: u64,
    pub tokens_after: u64,
}

/// A leaf pointer entry (S9.3 `leaf` type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeafEntry {
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub entry_id: String,
}

/// Serialized extension state attached to the current content branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionStateEntry {
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub state: serde_json::Value,
}

/// User-visible session name (Phase 13 `session_info` entry).
///
/// Append-only: each entry records a rename; readers take the latest value on
/// the active branch. Parented to the content tip but does not advance it and
/// does not enter the branch graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfoEntry {
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub name: String,
}

/// Recorded provider/model change on the active branch (Phase 13
/// `model_change` entry). `model` uses the `provider:model` spec format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelChangeEntry {
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub model: String,
}

/// Recorded thinking/reasoning level change (Phase 13
/// `thinking_level_change` entry).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingLevelChangeEntry {
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub level: crate::session_event::ThinkingLevel,
}

/// Bookmark or label attached to an entry (Phase 13 `label` entry). UI-visible;
/// does not enter LLM context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelEntry {
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub label: String,
    pub action: LabelAction,
}

/// Whether a [`LabelEntry`] adds or removes a label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabelAction {
    Add,
    Remove,
}

/// Branch summary preserving context when leaving or forking a branch
/// (Phase 13 `branch_summary` entry). Whether it enters provider LLM context
/// is decided by the Phase 13 context builder (task 13.2); the durable entry
/// only stores the summary text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchSummaryEntry {
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub summary: String,
}

/// All tree entry types (S9.3 + Phase 13.1 additive entries).
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEntry {
    Message(MessageEntry),
    Compaction(CompactionEntry),
    Leaf(LeafEntry),
    ExtensionState(ExtensionStateEntry),
    /// Phase 13.1 additive metadata/context entries.
    SessionInfo(SessionInfoEntry),
    ModelChange(ModelChangeEntry),
    ThinkingLevelChange(ThinkingLevelChangeEntry),
    Label(LabelEntry),
    BranchSummary(BranchSummaryEntry),
}

impl SessionEntry {
    /// Return the entry's unique ID regardless of variant.
    pub fn entry_id(&self) -> &str {
        match self {
            SessionEntry::Message(m) => &m.id,
            SessionEntry::Compaction(c) => &c.id,
            SessionEntry::Leaf(l) => &l.id,
            SessionEntry::ExtensionState(s) => &s.id,
            SessionEntry::SessionInfo(s) => &s.id,
            SessionEntry::ModelChange(m) => &m.id,
            SessionEntry::ThinkingLevelChange(t) => &t.id,
            SessionEntry::Label(l) => &l.id,
            SessionEntry::BranchSummary(b) => &b.id,
        }
    }
}

/// Crash recovery status returned by [`SessionReader`].
///
/// Phase 13.1 splits the historical single enum into three independent
/// observations so that **unknown future entry types** (forward-compatible
/// additive entries a newer opi might write) are no longer bucketed as
/// corruption. A clean load has all three fields zero/false.
///
/// - `truncated_line`: the file's final line had no trailing newline (a
///   crashed partial write); the preceding entries were recovered.
/// - `corrupt_count`: lines that were invalid JSON, or JSON without a `type`
///   field, and could not be recovered.
/// - `unknown_count`: lines that were valid JSON objects carrying a `type` tag
///   this build does not recognize. They are skipped (not preserved on
///   rewrite) and never fatal — the forward-compatibility path.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CrashRecovery {
    pub truncated_line: bool,
    pub corrupt_count: usize,
    pub unknown_count: usize,
}

impl CrashRecovery {
    /// Number of corrupt/unparseable entries found during load.
    pub fn corrupt_count(&self) -> usize {
        self.corrupt_count
    }

    /// Number of unknown future entry types skipped during load.
    pub fn unknown_count(&self) -> usize {
        self.unknown_count
    }

    /// True when the load was clean (no truncation, no corrupt, no unknown).
    pub fn is_clean(&self) -> bool {
        !self.truncated_line && self.corrupt_count == 0 && self.unknown_count == 0
    }

    /// Project this recovery outcome into shared diagnostics.
    ///
    /// A clean load produces nothing. Each non-empty field produces one warning
    /// diagnostic carrying only the relevant count (never entry content) so the
    /// observation is safe to surface. The harness records these after a session
    /// load; producing them here keeps the classification testable without
    /// driving the harness.
    pub fn diagnostics(&self) -> Vec<crate::diagnostic::Diagnostic> {
        use crate::diagnostic::{Diagnostic, SOURCE_SESSION, Severity, code::*};
        let mut out = Vec::new();
        if self.truncated_line {
            out.push(Diagnostic::new(
                Severity::Warning,
                CODE_SESSION_TRUNCATED_LINE,
                SOURCE_SESSION,
                "session had a truncated trailing line; recovered preceding entries",
            ));
        }
        if self.corrupt_count > 0 {
            out.push(
                Diagnostic::new(
                    Severity::Warning,
                    CODE_SESSION_CORRUPT_ENTRIES,
                    SOURCE_SESSION,
                    "session recovered with corrupt entries skipped",
                )
                .details(serde_json::json!({ "corrupt_count": self.corrupt_count })),
            );
        }
        if self.unknown_count > 0 {
            out.push(
                Diagnostic::new(
                    Severity::Warning,
                    CODE_SESSION_UNKNOWN_ENTRIES,
                    SOURCE_SESSION,
                    "session loaded with unknown future entry types skipped",
                )
                .details(serde_json::json!({ "unknown_count": self.unknown_count })),
            );
        }
        out
    }
}

/// Append-only JSONL writer with crash-safe flush.
pub struct SessionWriter {
    file: std::fs::File,
}

impl SessionWriter {
    /// Create a new session file with the given header.
    pub fn create(path: &Path, header: SessionHeader) -> std::io::Result<Self> {
        let mut file = std::fs::File::create(path)?;
        let header_json = serde_json::to_string(&header)?;
        writeln!(file, "{header_json}")?;
        file.sync_all()?;
        Ok(Self { file })
    }

    /// Open an existing session file for appending (seeks to end).
    ///
    /// If the file's last line is incomplete (no trailing newline from a
    /// crashed write), the incomplete tail is truncated so subsequent appends
    /// land on a clean line boundary.
    pub fn open(path: &Path) -> std::io::Result<Self> {
        use std::io::{Read, Seek, SeekFrom};

        // Open read+write (not append) so set_len works on Windows.
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?;

        // Check whether the file ends with a newline. If not, truncate the
        // incomplete trailing line so the first appended entry lands cleanly.
        let len = file.seek(SeekFrom::End(0))?;
        if len > 0 {
            let mut last = [0u8; 1];
            file.seek(SeekFrom::End(-1))?;
            file.read_exact(&mut last)?;
            if last[0] != b'\n' {
                // Scan backwards for the last newline to find the truncation point.
                let mut pos = len;
                let mut buf = [0u8; 1];
                let mut found_newline = false;
                loop {
                    if pos == 0 {
                        // No newline found — truncate the whole file to empty.
                        break;
                    }
                    pos -= 1;
                    file.seek(SeekFrom::Start(pos))?;
                    file.read_exact(&mut buf)?;
                    if buf[0] == b'\n' {
                        found_newline = true;
                        break;
                    }
                }
                // When a newline was found, keep it (truncate to pos+1) so the
                // next append starts on a fresh line. Without this the prior
                // complete entry and the new entry would be concatenated on one
                // line, corrupting the JSONL.
                file.set_len(if found_newline { pos + 1 } else { pos })?;
            }
            file.seek(SeekFrom::End(0))?;
        }

        Ok(Self { file })
    }

    /// Append a session entry as a new JSONL line.
    pub fn append(&mut self, entry: &SessionEntry) -> std::io::Result<()> {
        let json = serde_json::to_string(entry)?;
        writeln!(self.file, "{json}")?;
        self.file.sync_all()
    }
}

/// JSONL reader with crash recovery.
pub struct SessionReader;

impl SessionReader {
    /// Read all entries from a session file (strict mode — errors on corrupt data).
    pub fn read_all(path: &Path) -> std::io::Result<(SessionHeader, Vec<SessionEntry>)> {
        let (header, entries, _recovery) = Self::read_with_recovery(path)?;
        Ok((header, entries))
    }

    /// Read all entries with crash recovery metadata.
    pub fn read_with_recovery(
        path: &Path,
    ) -> std::io::Result<(SessionHeader, Vec<SessionEntry>, CrashRecovery)> {
        let content = std::fs::read_to_string(path)?;

        if content.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "empty session file",
            ));
        }

        let last_line_incomplete = !content.ends_with('\n') && !content.ends_with('\r');

        // Single-pass: collect lines, then parse.
        let all_lines: Vec<&str> = content.lines().collect();
        if all_lines.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "empty session file",
            ));
        }

        // First line is the header.
        let header: SessionHeader = serde_json::from_str(all_lines[0]).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid session header: {e}"),
            )
        })?;

        // Validate header type and version.
        if header.type_ != "session" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("expected header type 'session', got '{}'", header.type_),
            ));
        }
        if header.version != FORMAT_VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "unsupported session version {}, expected {}",
                    header.version, FORMAT_VERSION
                ),
            ));
        }

        let data_lines = &all_lines[1..];
        let total = data_lines.len();
        let mut entries = Vec::new();
        let mut corrupt_count = 0;
        let mut unknown_count = 0;

        for (i, line) in data_lines.iter().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            // Skip the last line if the file ended without a newline (truncated write).
            if last_line_incomplete && i == total - 1 {
                continue;
            }
            // Parse to a `Value` first so an unrecognized `type` tag can be
            // distinguished from genuine corruption. This is the Phase 13.1
            // forward-compatibility path: an additive entry written by a newer
            // opi build is skipped and reported via `unknown_count`, never
            // fatal. A JSON object without a `type` field, or a known type tag
            // with a malformed payload, stays corrupt; only a present but
            // unrecognized `type` tag lands in the unknown bucket.
            match serde_json::from_str::<serde_json::Value>(line) {
                Ok(value) => match serde_json::from_value::<SessionEntry>(value.clone()) {
                    Ok(entry) => entries.push(entry),
                    Err(_) => {
                        let unknown = value
                            .get("type")
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|t| !KNOWN_ENTRY_TYPES.contains(&t));
                        if unknown {
                            unknown_count += 1;
                        } else {
                            corrupt_count += 1;
                        }
                    }
                },
                Err(_) => corrupt_count += 1,
            }
        }

        let recovery = CrashRecovery {
            truncated_line: last_line_incomplete,
            corrupt_count,
            unknown_count,
        };

        Ok((header, entries, recovery))
    }
}

/// Type tags this build recognizes as [`SessionEntry`] variants.
///
/// Used by [`SessionReader::read_with_recovery`] to split "unknown future
/// entry type" from "corrupt entry". Kept in sync with the
/// `#[serde(tag = "type", rename_all = "snake_case")]` names on
/// [`SessionEntry`]; the round-trip tests in `session_storage.rs` pin each tag.
const KNOWN_ENTRY_TYPES: &[&str] = &[
    "message",
    "compaction",
    "leaf",
    "extension_state",
    "session_info",
    "model_change",
    "thinking_level_change",
    "label",
    "branch_summary",
];
