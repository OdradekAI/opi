//! Session JSONL storage with a fail-closed version-2 entry envelope.
//!
//! Append-only, versioned JSONL format for session persistence. The first line
//! is a header; subsequent lines are tree entries forming a conversation tree.
//!
//! # Entry model and compatibility contract
//!
//! Legacy **header version 1** remains readable as immutable historical input.
//! New writers produce header version 2. Its JSONL structure supports the
//! following inner `type` tags:
//! (`session_info`, `model_change`, `thinking_level_change`, `label`,
//! `branch_summary`). A version bump is reserved for a future breaking
//! structural change; v1 files written by older opi builds remain readable
//! and resumable without a migration command.
//!
//! Every v2 entry is wrapped in an envelope declaring it `required` or an
//! `ignorable_observation`. An unknown required entry fails closed; only an
//! explicitly ignorable observation with an unknown inner type is skipped.
//! The v2 header retains the immutable [`crate::evidence::RuntimeInputBinding`]
//! that belongs to the validated committed prefix. Version 1 preserves its
//! historical crash-recovery behavior but cannot be reopened for writing.
//!
//! `branch_summary`, `session_info`, `model_change`, `thinking_level_change`,
//! and `label` are metadata/context attachments: they are parented to the
//! current content tip but do **not** advance it, and they do not enter the
//! branch graph in [`crate::session_branch::SessionTree`]. `branch_summary`
//! context reconstruction and provider conversion are owned by context/product
//! paths, not by this entry model.
//!
//! There is no dedicated `custom_message` durable entry variant.
//! Extension-provided messages persist
//! via the existing `extension_state` entry and the `include_in_llm_context`
//! flag on the in-memory `AgentMessage::Custom` variant. Provider-context
//! injection for custom messages is owned by the context builder and product
//! paths, not by the entry model.
//!
//! opi sessions are **not** pi-session-v3 compatible. opi learns from pi's
//! append-only tree/context shape but does not promise to read or write
//! arbitrary pi session files. See [`SESSION_FORMAT_POLICY`].

use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Current session format version for new writers.
pub const FORMAT_VERSION: u32 = 2;

/// Historical format accepted for read-only recovery and export.
pub const LEGACY_FORMAT_VERSION: u32 = 1;

/// Human-readable session format/compatibility policy. Asserted by tests so the
/// pi-compatibility disclaimer cannot drift silently from the module-level
/// documentation above.
pub const SESSION_FORMAT_POLICY: &str = "opi session format: version 2 (required or ignorable entry envelopes and immutable runtime-input binding); version 1 is read-only legacy input. opi sessions are not pi-session-v3 compatible; opi learns from pi's append-only tree/context shape but does not read or write arbitrary pi session files. Unknown required v2 entries fail closed; only explicitly ignorable unknown observations are skipped.";

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
    /// Immutable binding for the runtime inputs that created this session.
    /// Version 1 headers have no binding and remain read-only legacy input.
    #[serde(default)]
    pub runtime_input_binding: Option<crate::evidence::RuntimeInputBinding>,
}

impl SessionHeader {
    /// Construct a v2 header with the exact immutable runtime-input binding
    /// resolved by trusted product assembly.
    pub fn new(
        id: String,
        timestamp: String,
        cwd: String,
        parent_session: Option<String>,
        runtime_input_binding: crate::evidence::RuntimeInputBinding,
    ) -> Self {
        Self {
            type_: "session".to_owned(),
            version: FORMAT_VERSION,
            id,
            timestamp,
            cwd,
            parent_session,
            runtime_input_binding: Some(runtime_input_binding),
        }
    }

    /// Construct a deterministic v2 header for test fixtures only.
    ///
    /// Production callers must use [`Self::new`] with the exact binding from
    /// trusted product assembly.
    #[doc(hidden)]
    pub fn new_for_test(
        id: String,
        timestamp: String,
        cwd: String,
        parent_session: Option<String>,
    ) -> Self {
        use sha2::Digest;

        let mut hasher = sha2::Sha256::new();
        for value in [
            id.as_str(),
            timestamp.as_str(),
            cwd.as_str(),
            parent_session.as_deref().unwrap_or(""),
        ] {
            hasher.update(value.as_bytes());
            hasher.update([0]);
        }
        let digest = crate::evidence::ContentDigest::from_hex(format!("{:x}", hasher.finalize()))
            .expect("SHA-256 formatter produces a canonical digest");
        let source = crate::evidence::AssemblyIdentity::new("opi.session.fixture")
            .expect("static assembly identity is valid");
        Self::new(
            id,
            timestamp,
            cwd,
            parent_session,
            crate::evidence::RuntimeInputBinding::direct(digest, source),
        )
    }

    /// Backward-compatible spelling for callers that already supply an exact
    /// runtime-input binding.
    pub fn new_with_runtime_input_binding(
        id: String,
        timestamp: String,
        cwd: String,
        parent_session: Option<String>,
        runtime_input_binding: crate::evidence::RuntimeInputBinding,
    ) -> Self {
        Self::new(id, timestamp, cwd, parent_session, runtime_input_binding)
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

/// User-visible session name (`session_info` entry).
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

/// Recorded provider/model change on the active branch (`model_change` entry).
/// `model` uses the canonical `provider:model` spec format; `input_source`
/// records whether that canonical form was supplied directly or produced by
/// normalizing a bare model input.
///
/// `input_source` is `#[serde(default)]` `Option` so legacy on-disk entries
/// that predate the field deserialize as `None` rather than corrupting the
/// session; the read side performs legacy normalization.
/// Normalization today is a non-lossy provider-prefix prepend, so the bare
/// original remains recoverable from the canonical form plus `BareNormalized`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelChangeEntry {
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub model: String,
    /// Whether `model` was supplied canonically or normalized from a bare
    /// input. `None` marks a legacy entry written before this field existed.
    #[serde(default)]
    pub input_source: Option<ModelInputSource>,
}

/// How a `ModelChangeEntry.model` canonical spec was produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelInputSource {
    /// The caller supplied a canonical `provider:model` spec directly.
    Canonical,
    /// The canonical spec was produced by normalizing a bare model input
    /// against the single provable route.
    BareNormalized,
}

/// Recorded thinking/reasoning level change (`thinking_level_change` entry).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingLevelChangeEntry {
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub level: crate::session_event::ThinkingLevel,
}

/// Bookmark or label attached to an entry (`label` entry). UI-visible;
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
/// (`branch_summary` entry). Context reconstruction and provider
/// conversion decide how the stored summary enters later turns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchSummaryEntry {
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub summary: String,
}

/// All tree entry types in the additive format.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEntry {
    Message(MessageEntry),
    Compaction(CompactionEntry),
    Leaf(LeafEntry),
    ExtensionState(ExtensionStateEntry),
    /// Additive metadata/context entries.
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

    /// Return the entry's `parent_id` regardless of variant. Metadata entries
    /// (session_info, model_change, thinking_level_change, label,
    /// branch_summary, extension_state) carry the content tip they were
    /// parented to at write time and are attachments, not chain nodes.
    pub fn parent_id(&self) -> Option<&str> {
        match self {
            SessionEntry::Message(m) => m.parent_id.as_deref(),
            SessionEntry::Compaction(c) => c.parent_id.as_deref(),
            SessionEntry::Leaf(l) => l.parent_id.as_deref(),
            SessionEntry::ExtensionState(s) => s.parent_id.as_deref(),
            SessionEntry::SessionInfo(s) => s.parent_id.as_deref(),
            SessionEntry::ModelChange(m) => m.parent_id.as_deref(),
            SessionEntry::ThinkingLevelChange(t) => t.parent_id.as_deref(),
            SessionEntry::Label(l) => l.parent_id.as_deref(),
            SessionEntry::BranchSummary(b) => b.parent_id.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SessionEntryClassification {
    Required,
    IgnorableObservation,
}

#[derive(Debug, Serialize, Deserialize)]
struct SessionEntryEnvelope {
    #[serde(rename = "type")]
    type_: String,
    classification: SessionEntryClassification,
    entry: serde_json::Value,
}

impl SessionEntryEnvelope {
    fn required(entry: &SessionEntry) -> Result<Self, serde_json::Error> {
        Ok(Self {
            type_: "entry".to_owned(),
            classification: SessionEntryClassification::Required,
            entry: serde_json::to_value(entry)?,
        })
    }
}

/// One validated durable session prefix and its immutable runtime binding.
///
/// This value is available only for v2 sessions. Legacy v1 files remain
/// readable through [`SessionReader::read_with_recovery`] but cannot be used
/// as a mutable resume/fork source because they lack the binding.
#[derive(Debug, Clone)]
pub struct ValidatedSession {
    pub header: SessionHeader,
    pub entries: Vec<SessionEntry>,
    pub recovery: CrashRecovery,
    pub runtime_input_binding: crate::evidence::RuntimeInputBinding,
}

/// Crash recovery status returned by [`SessionReader`].
///
/// Three independent observations keep **unknown future entry types**
/// (forward-compatible
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

/// Append-only JSONL writer with fsync and recoverable line-boundary handling.
pub struct SessionWriter {
    file: std::fs::File,
    format_version: u32,
}

impl SessionWriter {
    /// Create a new session file with the given header.
    pub fn create(path: &Path, header: SessionHeader) -> std::io::Result<Self> {
        if header.version != FORMAT_VERSION || header.runtime_input_binding.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "new session writers require a v2 header with a runtime-input binding",
            ));
        }
        let mut file = std::fs::File::create(path)?;
        let header_json = serde_json::to_string(&header)?;
        writeln!(file, "{header_json}")?;
        file.sync_all()?;
        Ok(Self {
            file,
            format_version: header.version,
        })
    }

    /// Open an existing session file for appending (seeks to end).
    ///
    /// If the file's last line is incomplete (no trailing newline from a
    /// crashed write), the incomplete tail is truncated so subsequent appends
    /// land on a clean line boundary.
    pub fn open(path: &Path) -> std::io::Result<Self> {
        use std::io::{Read, Seek, SeekFrom};

        let header_content = std::fs::read_to_string(path)?;
        let header_line = header_content.lines().next().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "empty session file")
        })?;
        let header: SessionHeader = serde_json::from_str(header_line).map_err(|error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid session header: {error}"),
            )
        })?;
        if header.version != FORMAT_VERSION || header.runtime_input_binding.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "legacy session is read-only; fork or export it without rewriting",
            ));
        }

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

        Ok(Self {
            file,
            format_version: header.version,
        })
    }

    /// Append a session entry as a new JSONL line.
    pub fn append(&mut self, entry: &SessionEntry) -> std::io::Result<()> {
        debug_assert_eq!(self.format_version, FORMAT_VERSION);
        let json = serde_json::to_string(&SessionEntryEnvelope::required(entry)?)?;
        writeln!(self.file, "{json}")?;
        self.file.sync_all()
    }

    /// Record the current durable append boundary for a multi-entry write.
    pub fn checkpoint(&mut self) -> std::io::Result<u64> {
        use std::io::{Seek, SeekFrom};

        self.file.seek(SeekFrom::End(0))
    }

    /// Restore a previously recorded append boundary after a failed batch.
    pub fn rollback_to(&mut self, checkpoint: u64) -> std::io::Result<()> {
        use std::io::{Seek, SeekFrom};

        self.file.set_len(checkpoint)?;
        self.file.seek(SeekFrom::Start(checkpoint))?;
        self.file.sync_all()
    }
}

/// JSONL reader with crash recovery.
pub struct SessionReader;

impl SessionReader {
    /// Read all entries from a session file, discarding recovery metadata.
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
        if header.version != LEGACY_FORMAT_VERSION && header.version != FORMAT_VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "unsupported session version {}, expected {} or {}",
                    header.version, LEGACY_FORMAT_VERSION, FORMAT_VERSION
                ),
            ));
        }
        if header.version == FORMAT_VERSION && header.runtime_input_binding.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "v2 session header is missing its runtime-input binding",
            ));
        }
        if header.version == LEGACY_FORMAT_VERSION && header.runtime_input_binding.is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "legacy v1 session header must not claim a runtime-input binding",
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
            if header.version == LEGACY_FORMAT_VERSION {
                // Legacy v1 has no envelope. Preserve its historical recovery
                // semantics without allowing it to become a mutable source.
                match serde_json::from_str::<serde_json::Value>(line) {
                    Ok(value) => match serde_json::from_value::<SessionEntry>(value.clone()) {
                        Ok(entry) => entries.push(entry),
                        Err(_) => {
                            let unknown = value
                                .get("type")
                                .and_then(serde_json::Value::as_str)
                                .is_some_and(|tag| !KNOWN_ENTRY_TYPES.contains(&tag));
                            if unknown {
                                unknown_count += 1;
                            } else {
                                corrupt_count += 1;
                            }
                        }
                    },
                    Err(_) => corrupt_count += 1,
                }
                continue;
            }

            let envelope: SessionEntryEnvelope = serde_json::from_str(line).map_err(|error| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid required v2 session entry envelope: {error}"),
                )
            })?;
            if envelope.type_ != "entry" {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "unknown required v2 session envelope type '{}'",
                        envelope.type_
                    ),
                ));
            }
            match serde_json::from_value::<SessionEntry>(envelope.entry.clone()) {
                Ok(entry) => {
                    if envelope.classification == SessionEntryClassification::IgnorableObservation {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "known session entry cannot be classified as an ignorable observation",
                        ));
                    }
                    entries.push(entry);
                }
                Err(_) => {
                    let unknown = envelope
                        .entry
                        .get("type")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|tag| !KNOWN_ENTRY_TYPES.contains(&tag));
                    match envelope.classification {
                        SessionEntryClassification::Required => {
                            let detail = if unknown {
                                "unknown required session entry type"
                            } else {
                                "invalid required session entry"
                            };
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                detail,
                            ));
                        }
                        SessionEntryClassification::IgnorableObservation => {
                            if !unknown {
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    "ignorable observation must carry an unknown entry type",
                                ));
                            }
                            unknown_count += 1;
                        }
                    }
                }
            }
        }

        let recovery = CrashRecovery {
            truncated_line: last_line_incomplete,
            corrupt_count,
            unknown_count,
        };

        Ok((header, entries, recovery))
    }

    /// Read the v2 prefix and binding that a mutable product session must use
    /// together for resume, fork, and evidence reconstruction.
    pub fn read_validated_with_recovery(path: &Path) -> std::io::Result<ValidatedSession> {
        let (header, entries, recovery) = Self::read_with_recovery(path)?;
        if header.version != FORMAT_VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "legacy session is read-only because it has no runtime-input binding",
            ));
        }
        let runtime_input_binding = header.runtime_input_binding.clone().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "v2 session header is missing its runtime-input binding",
            )
        })?;
        Ok(ValidatedSession {
            header,
            entries,
            recovery,
            runtime_input_binding,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn user_message() -> opi_ai::message::Message {
        opi_ai::message::Message::User(opi_ai::message::UserMessage {
            content: vec![opi_ai::message::InputContent::Text {
                text: "hello".into(),
            }],
            timestamp_ms: 0,
        })
    }

    fn emitted_tag(entry: SessionEntry) -> String {
        serde_json::to_value(entry)
            .unwrap()
            .get("type")
            .and_then(serde_json::Value::as_str)
            .expect("SessionEntry serializes a type tag")
            .to_owned()
    }

    #[test]
    fn known_entry_types_matches_serde_emitted_variant_tags() {
        let entries = vec![
            SessionEntry::Message(MessageEntry {
                id: "m1".into(),
                parent_id: None,
                timestamp: "2026-07-06T00:00:00Z".into(),
                message: user_message(),
            }),
            SessionEntry::Compaction(CompactionEntry {
                id: "c1".into(),
                parent_id: Some("m1".into()),
                timestamp: "2026-07-06T00:00:00Z".into(),
                summary: "summary".into(),
                first_kept_entry_id: "m1".into(),
                tokens_before: 10,
                tokens_after: 5,
            }),
            SessionEntry::Leaf(LeafEntry {
                id: "l1".into(),
                parent_id: Some("c1".into()),
                timestamp: "2026-07-06T00:00:00Z".into(),
                entry_id: "c1".into(),
            }),
            SessionEntry::ExtensionState(ExtensionStateEntry {
                id: "state1".into(),
                parent_id: Some("m1".into()),
                timestamp: "2026-07-06T00:00:00Z".into(),
                state: serde_json::json!({"ok": true}),
            }),
            SessionEntry::SessionInfo(SessionInfoEntry {
                id: "info1".into(),
                parent_id: Some("m1".into()),
                timestamp: "2026-07-06T00:00:00Z".into(),
                name: "demo".into(),
            }),
            SessionEntry::ModelChange(ModelChangeEntry {
                id: "model1".into(),
                parent_id: Some("m1".into()),
                timestamp: "2026-07-06T00:00:00Z".into(),
                model: "mock:model".into(),
                input_source: None,
            }),
            SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
                id: "thinking1".into(),
                parent_id: Some("m1".into()),
                timestamp: "2026-07-06T00:00:00Z".into(),
                level: crate::session_event::ThinkingLevel::Low,
            }),
            SessionEntry::Label(LabelEntry {
                id: "label1".into(),
                parent_id: Some("m1".into()),
                timestamp: "2026-07-06T00:00:00Z".into(),
                label: "bug".into(),
                action: LabelAction::Add,
            }),
            SessionEntry::BranchSummary(BranchSummaryEntry {
                id: "summary1".into(),
                parent_id: Some("m1".into()),
                timestamp: "2026-07-06T00:00:00Z".into(),
                summary: "parent branch".into(),
            }),
        ];

        let emitted: BTreeSet<String> = entries.into_iter().map(emitted_tag).collect();
        let known: BTreeSet<String> = KNOWN_ENTRY_TYPES
            .iter()
            .map(|tag| (*tag).to_owned())
            .collect();

        assert_eq!(known, emitted);
        assert_eq!(
            KNOWN_ENTRY_TYPES.len(),
            emitted.len(),
            "KNOWN_ENTRY_TYPES must have exactly one tag per serde-emitting variant"
        );
    }
}
