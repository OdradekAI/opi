//! Shared diagnostic model and redaction core (Phase 7).
//!
//! This module defines the workspace-wide diagnostic vocabulary: a structured
//! [`Diagnostic`] record with a stable severity, snake_case code, subsystem
//! source, human message, optional structured details, and an optional next
//! action. It also provides redaction helpers that scrub known secrets and
//! sensitive content from diagnostic details before they leave the runtime.
//!
//! Redaction reuses [`crate::streaming_proxy::SecretRedactor`] so that the
//! secret patterns used by the streaming proxy (API keys, bearer tokens, and
//! sensitive field names) stay consistent across the codebase.
//!
//! Human/terminal formatting (color, alignment, tables) is intentionally kept
//! out of this layer; callers near the CLI render diagnostics. The [`Diagnostic`]
//! [`core::fmt::Display`] implementation is a stable single-line form suitable
//! for logs and tests, not a presentation format.

use std::fmt;
use std::path::PathBuf;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::streaming_proxy::SecretRedactor;

/// Diagnostic severity, ordered `Error` > `Warning` > `Info`.
///
/// The declaration order gives the derived [`Ord`] the desired ranking so that
/// diagnostics can be sorted by severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational observation; never represents a failure.
    Info,
    /// Recoverable or degraded behavior worth surfacing.
    Warning,
    /// A failure that should be acted on.
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        })
    }
}

/// How aggressively diagnostic details are redacted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionMode {
    /// Default: scrub known secrets and sensitive content (prompts, tool
    /// output, environment blocks, commands, working directories, and absolute
    /// paths) so details are safe for a support report.
    Summary,
    /// Include additional local metadata but still scrub known secrets.
    Verbose,
}

/// The default redaction mode is [`RedactionMode::Summary`] (safe by default).
impl Default for RedactionMode {
    fn default() -> Self {
        RedactionMode::Summary
    }
}

/// A structured diagnostic record shared across runtime, provider, tool,
/// package, adapter, session, config, and RPC surfaces.
///
/// `code` and `source` are `&'static str` because they are stable identifiers
/// by design; a diagnostic is constructed from known literals, not dynamic
/// strings. `message` is human-readable and may be dynamic inside the runtime;
/// use [`Diagnostic::redacted_payload`] before crossing public boundaries.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Diagnostic {
    /// Severity of the observation.
    pub severity: Severity,
    /// Stable snake_case identifier suitable for tests and matching.
    pub code: &'static str,
    /// Owning subsystem, e.g. one of the `SOURCE_*` constants.
    pub source: &'static str,
    /// Short human-readable explanation.
    pub message: String,
    /// Optional structured metadata; emitted redacted via [`Self::redacted_details`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Optional suggested next step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

/// Public diagnostic representation after applying the selected redaction mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticPayload {
    /// Severity of the observation.
    pub severity: Severity,
    /// Stable snake_case identifier suitable for tests and matching.
    pub code: String,
    /// Owning subsystem, e.g. one of the `SOURCE_*` constants.
    pub source: String,
    /// Redacted short human-readable explanation.
    pub message: String,
    /// Optional redacted structured metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Optional redacted suggested next step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

impl Diagnostic {
    /// Create a diagnostic with the required stable fields.
    pub fn new(
        severity: Severity,
        code: &'static str,
        source: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code,
            source,
            message: message.into(),
            details: None,
            action: None,
        }
    }

    /// Attach structured details.
    pub fn details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Attach a suggested next action.
    pub fn action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// Return the details redacted for the given mode, or `None` if unset.
    ///
    /// This helper preserves the legacy internal behavior: it only touches
    /// `details`. Use [`Self::redacted_payload`] before exposing diagnostics
    /// outside the runtime.
    pub fn redacted_details(&self, mode: RedactionMode) -> Option<serde_json::Value> {
        self.details.as_ref().map(|value| redact(value, mode))
    }

    /// Return a public diagnostic payload with all dynamic fields redacted.
    pub fn redacted_payload(&self, mode: RedactionMode) -> DiagnosticPayload {
        DiagnosticPayload {
            severity: self.severity,
            code: self.code.to_owned(),
            source: self.source.to_owned(),
            message: redact_text(&self.message, mode),
            details: self.redacted_details(mode),
            action: self.action.as_ref().map(|action| redact_text(action, mode)),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.action {
            Some(action) => write!(
                f,
                "[{}] {}::{}: {} (action: {})",
                self.severity, self.source, self.code, self.message, action
            ),
            None => write!(
                f,
                "[{}] {}::{}: {}",
                self.severity, self.source, self.code, self.message
            ),
        }
    }
}

impl fmt::Display for DiagnosticPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.action {
            Some(action) => write!(
                f,
                "[{}] {}::{}: {} (action: {})",
                self.severity, self.source, self.code, self.message, action
            ),
            None => write!(
                f,
                "[{}] {}::{}: {}",
                self.severity, self.source, self.code, self.message
            ),
        }
    }
}

/// Stable subsystem source identifiers, forming the shared diagnostic
/// vocabulary. Application-specific sources live where they are produced but
/// share these constant spellings.
pub const SOURCE_PROVIDER: &str = "provider";
pub const SOURCE_TOOL: &str = "tool";
pub const SOURCE_PACKAGE: &str = "package";
pub const SOURCE_ADAPTER: &str = "adapter";
pub const SOURCE_SESSION: &str = "session";
pub const SOURCE_CONFIG: &str = "config";
pub const SOURCE_RPC: &str = "rpc";
pub const SOURCE_TUI: &str = "tui";
/// Agent-runtime-internal observations: hook failures, cancellation, and the
/// max-turns guard. Distinct from `SOURCE_RPC` (the JSONL protocol) and
/// `SOURCE_TOOL` (built-in/extension tools) so the producing subsystem is
/// reported accurately regardless of whether the loop is driven by RPC,
/// interactive, or non-interactive mode.
pub const SOURCE_AGENT: &str = "agent";

/// Stable snake_case diagnostic codes. Declared as named constants so a typo
/// in a code literal is a compile error (when referenced via the constant) and
/// a typo in the constant value is caught by the code-stability tests.
pub mod code {
    pub const CODE_PROVIDER_AUTH_FAILED: &str = "provider_auth_failed";
    pub const CODE_PROVIDER_RATE_LIMITED: &str = "provider_rate_limited";
    pub const CODE_PROVIDER_TIMEOUT: &str = "provider_timeout";
    pub const CODE_PROVIDER_REQUEST_FAILED: &str = "provider_request_failed";
    pub const CODE_PROVIDER_STREAM_ERROR: &str = "provider_stream_error";
    pub const CODE_PROVIDER_NETWORK: &str = "provider_network_error";
    pub const CODE_PROVIDER_CONFIG: &str = "provider_config_error";
    pub const CODE_PROVIDER_SIDE: &str = "provider_side_error";
    pub const CODE_PROVIDER_CANCELLED: &str = "provider_cancelled";
    /// Generic provider failure surfaced through the agent loop when the
    /// structured [`opi_ai::provider::ProviderError`] category is no longer
    /// recoverable (e.g. retries exhausted).
    pub const CODE_PROVIDER_ERROR: &str = "provider_error";
    pub const CODE_TOOL_FAILED: &str = "tool_failed";
    pub const CODE_HOOK_FAILED: &str = "hook_failed";
    pub const CODE_AGENT_CANCELLED: &str = "agent_cancelled";
    pub const CODE_AGENT_MAX_TURNS_EXCEEDED: &str = "agent_max_turns_exceeded";
    // Runtime emission codes (agent loop).
    pub const CODE_PROVIDER_RETRY_ATTEMPT: &str = "provider_retry_attempt";
    pub const CODE_PROVIDER_RETRY_SUCCEEDED: &str = "provider_retry_succeeded";
    pub const CODE_PROVIDER_RETRY_EXHAUSTED: &str = "provider_retry_exhausted";
    pub const CODE_PROVIDER_RETRY_SUPPRESSED_AFTER_PARTIAL_OUTPUT: &str =
        "provider_retry_suppressed_after_partial_output";
    pub const CODE_PROVIDER_CAPABILITY_INVALID: &str = "provider_capability_invalid";
    pub const CODE_TOOL_UNKNOWN: &str = "tool_unknown";
    pub const CODE_TOOL_VALIDATION_FAILED: &str = "tool_validation_failed";
    pub const CODE_TOOL_EXECUTION_FAILED: &str = "tool_execution_failed";
    // Filesystem/tool-error taxonomy codes (Phase 11.2). Each maps to a distinct
    // `FsToolError` variant, replacing the single `CODE_TOOL_EXECUTION_FAILED`
    // collapse for tool-reported path/filesystem causes.
    pub const CODE_TOOL_PATH_NOT_FOUND: &str = "tool_path_not_found";
    pub const CODE_TOOL_NOT_A_FILE: &str = "tool_not_a_file";
    pub const CODE_TOOL_NOT_A_DIRECTORY: &str = "tool_not_a_directory";
    pub const CODE_TOOL_PERMISSION_DENIED: &str = "tool_permission_denied";
    pub const CODE_TOOL_BINARY_FILE: &str = "tool_binary_file";
    pub const CODE_TOOL_UNSUPPORTED_ENCODING: &str = "tool_unsupported_encoding";
    pub const CODE_TOOL_OUTSIDE_WORKSPACE: &str = "tool_outside_workspace";
    pub const CODE_TOOL_UNRESOLVED_WORKSPACE_ROOT: &str = "tool_unresolved_workspace_root";
    // Session/compaction classification codes.
    pub const CODE_SESSION_COMPACTED: &str = "session_compacted";
    pub const CODE_COMPACTION_NOTHING_TO_COMPACT: &str = "compaction_nothing_to_compact";
    pub const CODE_SESSION_CORRUPT_ENTRIES: &str = "session_corrupt_entries";
    pub const CODE_SESSION_TRUNCATED_LINE: &str = "session_truncated_line";
    pub const CODE_SESSION_UNKNOWN_ENTRIES: &str = "session_unknown_entries";
    /// Context reconstruction encountered an entry whose `parent_id` does not
    /// resolve to any known entry. The orphan entry is included in the
    /// reconstructed chain up to the break; the diagnostic explains where the
    /// chain could not be extended further. Phase 13.2.
    pub const CODE_SESSION_CONTEXT_MISSING_PARENT: &str = "session_context_missing_parent";
    // opi-coding-agent bridges (package/config). Package diagnostics carry a
    // dynamic granular code in `details.package_code`; the shared code is stable.
    pub const CODE_PACKAGE_DIAGNOSTIC: &str = "package_diagnostic";
    pub const CODE_PACKAGE_RESOLUTION_FAILED: &str = "package_resolution_failed";
    pub const CODE_CONFIG_PARSE_FAILED: &str = "config_parse_failed";
    pub const CODE_CONFIG_READ_FAILED: &str = "config_read_failed";
    pub const CODE_ADAPTER_PROTOCOL_UNSUPPORTED: &str = "adapter_protocol_unsupported";
    pub const CODE_ADAPTER_KIND_UNSUPPORTED: &str = "adapter_kind_unsupported";
    pub const CODE_ADAPTER_COMMAND_INVALID: &str = "adapter_command_invalid";
    pub const CODE_ADAPTER_STARTUP_FAILED: &str = "adapter_startup_failed";
    pub const CODE_ADAPTER_REGISTRATION_FAILED: &str = "adapter_registration_failed";
    pub const CODE_ADAPTER_HOST_DIAGNOSTIC: &str = "adapter_host_diagnostic";
    /// A local trace sink failed mid-run and was disabled (fail-open).
    pub const CODE_TRACE_SINK_FAILED: &str = "trace_sink_failed";
    /// A requested trace could not be prepared before the run (fail-closed).
    pub const CODE_TRACE_SETUP_FAILED: &str = "trace_setup_failed";
}

/// Shared filesystem/tool-error taxonomy (Phase 11.2).
///
/// Each variant maps to a distinct [`code::CODE_TOOL_*`](code) identifier so
/// tool-reported path/filesystem causes are no longer collapsed into
/// [`code::CODE_TOOL_EXECUTION_FAILED`]. Tools construct the relevant variant
/// and call [`FsToolError::to_diagnostic`] to attach a
/// [`crate::tool::ToolDiagnostic`] to the failing [`crate::tool::ToolResult`];
/// the agent loop lifts those into Phase 7 diagnostics/traces in a later task.
#[derive(Debug, Clone, PartialEq)]
pub enum FsToolError {
    /// A path that should exist was not found.
    NotFound {
        user_path: String,
        resolved_path: Option<PathBuf>,
    },
    /// A path expected to be a regular file was a directory or other type.
    NotAFile { path: PathBuf },
    /// A path expected to be a directory was a file or other type.
    NotADirectory { path: PathBuf },
    /// The process lacks permission to access the path.
    PermissionDenied { path: PathBuf },
    /// The path points at binary content. Substrate variant; content detection
    /// is owned by the read-hardening task, not the taxonomy itself.
    BinaryFile { path: PathBuf },
    /// One or more filesystem entry names could not be converted to valid UTF-8.
    /// Carries the count of omitted entries so the diagnostic never has to embed
    /// a lossy (U+FFFD) path string.
    UnsupportedEncoding { omitted_count: usize },
    /// A path resolves outside the workspace under a workspace-only policy.
    /// `symlink_traversed` records whether the escape crossed a symlink/junction
    /// so callers report traversal rather than silently collapsing to the
    /// workspace-boundary denial.
    OutsideWorkspace {
        user_path: String,
        symlink_traversed: bool,
    },
    /// The workspace root itself could not be canonicalized/resolved.
    UnresolvedWorkspaceRoot { source: String },
}

impl FsToolError {
    /// Stable diagnostic code for this cause.
    pub fn code(&self) -> &'static str {
        match self {
            FsToolError::NotFound { .. } => code::CODE_TOOL_PATH_NOT_FOUND,
            FsToolError::NotAFile { .. } => code::CODE_TOOL_NOT_A_FILE,
            FsToolError::NotADirectory { .. } => code::CODE_TOOL_NOT_A_DIRECTORY,
            FsToolError::PermissionDenied { .. } => code::CODE_TOOL_PERMISSION_DENIED,
            FsToolError::BinaryFile { .. } => code::CODE_TOOL_BINARY_FILE,
            FsToolError::UnsupportedEncoding { .. } => code::CODE_TOOL_UNSUPPORTED_ENCODING,
            FsToolError::OutsideWorkspace { .. } => code::CODE_TOOL_OUTSIDE_WORKSPACE,
            FsToolError::UnresolvedWorkspaceRoot { .. } => {
                code::CODE_TOOL_UNRESOLVED_WORKSPACE_ROOT
            }
        }
    }

    /// User-facing, byte-stable cause message. This is also the
    /// [`fmt::Display`] rendering, so `.to_string()` preserves the message tools
    /// emit today (no variant-name prefix leaks into agent-visible text).
    pub fn message(&self) -> String {
        match self {
            FsToolError::NotFound { user_path, .. } => {
                format!("path '{user_path}' does not exist")
            }
            FsToolError::NotAFile { path } => format!("'{}' is not a file", path.display()),
            FsToolError::NotADirectory { path } => {
                format!("'{}' is not a directory", path.display())
            }
            FsToolError::PermissionDenied { path } => {
                format!("permission denied: '{}'", path.display())
            }
            FsToolError::BinaryFile { path } => {
                format!("'{}' appears to be a binary file", path.display())
            }
            FsToolError::UnsupportedEncoding { omitted_count } => {
                if *omitted_count == 1 {
                    "1 entry with a non-UTF-8 name omitted".to_string()
                } else {
                    format!("{omitted_count} entries with non-UTF-8 names omitted")
                }
            }
            FsToolError::OutsideWorkspace { user_path, .. } => {
                format!("path '{user_path}' resolves outside the workspace")
            }
            FsToolError::UnresolvedWorkspaceRoot { source } => {
                format!("cannot resolve workspace root: {source}")
            }
        }
    }

    /// Structured per-cause context payload (becomes `Diagnostic::details` at the
    /// agent-loop lift).
    pub fn context(&self) -> serde_json::Value {
        match self {
            FsToolError::NotFound {
                user_path,
                resolved_path,
            } => {
                let mut obj = serde_json::Map::new();
                obj.insert("user_path".into(), serde_json::json!(user_path));
                if let Some(resolved) = resolved_path {
                    obj.insert(
                        "resolved_path".into(),
                        serde_json::json!(resolved.display().to_string()),
                    );
                }
                serde_json::Value::Object(obj)
            }
            FsToolError::NotAFile { path }
            | FsToolError::NotADirectory { path }
            | FsToolError::PermissionDenied { path }
            | FsToolError::BinaryFile { path } => {
                serde_json::json!({ "path": path.display().to_string() })
            }
            FsToolError::UnsupportedEncoding { omitted_count } => {
                serde_json::json!({ "omitted_count": omitted_count })
            }
            FsToolError::OutsideWorkspace {
                user_path,
                symlink_traversed,
            } => {
                serde_json::json!({ "user_path": user_path, "symlink_traversed": symlink_traversed })
            }
            FsToolError::UnresolvedWorkspaceRoot { source } => {
                serde_json::json!({ "source": source })
            }
        }
    }

    /// Build the tool-owned [`crate::tool::ToolDiagnostic`] carrying this cause.
    pub fn to_diagnostic(&self) -> crate::tool::ToolDiagnostic {
        crate::tool::ToolDiagnostic {
            code: self.code().to_string(),
            message: self.message(),
            context: self.context(),
        }
    }
}

impl fmt::Display for FsToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

const REDACTED: &str = "[REDACTED]";

/// Field names whose full values are redacted in every mode because they
/// carry prompts, tool output, environment blocks, commands, or working
/// directories.
const CONTENT_SENSITIVE_KEYS: &[&str] = &[
    "prompt",
    "prompts",
    "tool_output",
    "tool_result",
    "env",
    "environment",
    "command",
    "args",
    "cwd",
    "body",
    "request_body",
    "response_body",
    "provider_error",
    "headers",
    "stdout",
    "stderr",
    "tool_error",
    "hook_error",
    "trace_error",
    "package_error",
    "package_message",
    "adapter_error",
    // Filesystem/tool taxonomy context (Phase 11.2): per-cause path fields are
    // scrubbed in Summary mode so the 11.8 diagnostic lift is safe regardless of
    // the absolute-path regex heuristic.
    "path",
    "resolved_path",
    "user_path",
];

/// Heuristic absolute-path detector used in summary mode. Matches Windows drive
/// paths (`C:\`, `D:/`), UNC paths (`\\server`), and common POSIX absolute
/// roots (`/Users/`, `/home/`, `/var/`, ...). The Windows alternative requires a
/// non-alphanumeric boundary before the drive letter so that URLs such as
/// `https://` are not mistaken for drive paths.
static ABSOLUTE_PATH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:^|[^A-Za-z0-9])[A-Za-z]:[\\/]|\\\\|/(?:Users|home|root|tmp|var|etc|opt|mnt|private|proc|sys|dev|srv|lib|run|app|data|usr|bin|sbin)/",
    )
    .expect("absolute path redaction regex must compile")
});

/// Redact a structured JSON value for the given mode.
///
/// Both modes scrub known secrets via [`SecretRedactor`] (API keys, bearer
/// tokens, and sensitive field names) and structural content-sensitive fields
/// such as commands, working directories, prompts, and tool output.
/// [`RedactionMode::Summary`] additionally redacts any string value that looks
/// like an absolute path.
pub fn redact(value: &serde_json::Value, mode: RedactionMode) -> serde_json::Value {
    let scrubbed = SecretRedactor::default().redact(value);
    let structurally_scrubbed = redact_structural_sensitive_fields(&scrubbed);
    match mode {
        RedactionMode::Summary => redact_summary_paths(&structurally_scrubbed),
        RedactionMode::Verbose => structurally_scrubbed,
    }
}

/// Redact a value for public event/session boundaries.
///
/// This is stricter than provider conversion and matches summary diagnostics:
/// tool arguments, paths, commands, env metadata, stdout/stderr-like fields, and
/// recognizable secret values are scrubbed before JSON/RPC/session exposure.
pub fn redact_public_value(value: &serde_json::Value) -> serde_json::Value {
    redact(value, RedactionMode::Summary)
}

/// Redact a single string using the same policy as structured details.
pub fn redact_text(text: &str, mode: RedactionMode) -> String {
    match redact(&serde_json::Value::String(text.to_owned()), mode) {
        serde_json::Value::String(redacted) => redacted,
        _ => REDACTED.to_owned(),
    }
}

fn redact_structural_sensitive_fields(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (key, value) in map {
                let redacted = if is_content_sensitive_key(key) {
                    serde_json::Value::String(REDACTED.to_owned())
                } else {
                    redact_structural_sensitive_fields(value)
                };
                out.insert(key.clone(), redacted);
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => serde_json::Value::Array(
            items
                .iter()
                .map(redact_structural_sensitive_fields)
                .collect(),
        ),
        other => other.clone(),
    }
}

fn redact_summary_paths(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), redact_summary_paths(value)))
                .collect(),
        ),
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(redact_summary_paths).collect())
        }
        serde_json::Value::String(s) => {
            if ABSOLUTE_PATH_RE.is_match(s) {
                serde_json::Value::String(REDACTED.to_owned())
            } else {
                serde_json::Value::String(s.clone())
            }
        }
        other => other.clone(),
    }
}

fn is_content_sensitive_key(key: &str) -> bool {
    CONTENT_SENSITIVE_KEYS
        .iter()
        .any(|sensitive| sensitive.eq_ignore_ascii_case(key))
}

/// Resolve a tool-owned diagnostic code string (carried on a
/// [`crate::tool::ToolDiagnostic`]) to its stable `&'static str` identifier,
/// defaulting to [`code::CODE_TOOL_EXECUTION_FAILED`] for unrecognized codes.
///
/// Phase 11.8 lifts each tool-owned `ToolDiagnostic` into a Phase 7
/// [`Diagnostic`] at the agent-loop boundary. `Diagnostic::code` is `&'static
/// str` by design (a stable identifier constructed from known literals); this
/// bridge lets the lift preserve the per-cause code without relaxing that
/// invariant to an owned string.
///
/// [`code::CODE_TOOL_UNKNOWN`] and [`code::CODE_TOOL_VALIDATION_FAILED`] are
/// intentionally excluded: those codes are emitted directly by the agent loop
/// (unknown-tool and schema-validation paths), never via a `ToolDiagnostic`.
pub fn resolve_tool_code(raw: &str) -> &'static str {
    match raw {
        code::CODE_TOOL_PATH_NOT_FOUND => code::CODE_TOOL_PATH_NOT_FOUND,
        code::CODE_TOOL_NOT_A_FILE => code::CODE_TOOL_NOT_A_FILE,
        code::CODE_TOOL_NOT_A_DIRECTORY => code::CODE_TOOL_NOT_A_DIRECTORY,
        code::CODE_TOOL_PERMISSION_DENIED => code::CODE_TOOL_PERMISSION_DENIED,
        code::CODE_TOOL_BINARY_FILE => code::CODE_TOOL_BINARY_FILE,
        code::CODE_TOOL_UNSUPPORTED_ENCODING => code::CODE_TOOL_UNSUPPORTED_ENCODING,
        code::CODE_TOOL_OUTSIDE_WORKSPACE => code::CODE_TOOL_OUTSIDE_WORKSPACE,
        code::CODE_TOOL_UNRESOLVED_WORKSPACE_ROOT => code::CODE_TOOL_UNRESOLVED_WORKSPACE_ROOT,
        code::CODE_TOOL_EXECUTION_FAILED => code::CODE_TOOL_EXECUTION_FAILED,
        // Unrecognized (incl. forward-compat or provider/extension-defined) codes
        // collapse to the generic tool-execution-failed identifier.
        _ => code::CODE_TOOL_EXECUTION_FAILED,
    }
}

// ---------------------------------------------------------------------------
// Classification bridges: map provider and agent-loop errors into Diagnostics.
// ---------------------------------------------------------------------------

/// Remediation hint attached to authentication failures.
const ACTION_CHECK_CREDENTIALS: &str = "check the API key or provider credentials";
/// Remediation hint attached to rate-limited responses.
const ACTION_SLOW_DOWN: &str = "reduce request frequency or wait for the retry-after delay";

/// Classify a [`opi_ai::provider::ProviderError`] into a [`Diagnostic`].
///
/// The provider taxonomy itself lives in `opi-ai` (`ProviderError::category`);
/// this bridge only fixes the diagnostic `severity`, `code`, and `source` for
/// each category. Runtime behavior is unchanged — the error is still returned
/// as-is; this only describes how it would be surfaced diagnostically.
impl From<&opi_ai::provider::ProviderError> for Diagnostic {
    fn from(error: &opi_ai::provider::ProviderError) -> Self {
        use opi_ai::provider::ProviderError;
        match error {
            ProviderError::RateLimited { retry_after_ms } => Diagnostic::new(
                Severity::Warning,
                code::CODE_PROVIDER_RATE_LIMITED,
                SOURCE_PROVIDER,
                "rate limited by provider",
            )
            .details_option(retry_after_ms.map(|ms| serde_json::json!({ "retry_after_ms": ms })))
            .action(ACTION_SLOW_DOWN),
            ProviderError::Timeout => Diagnostic::new(
                Severity::Warning,
                code::CODE_PROVIDER_TIMEOUT,
                SOURCE_PROVIDER,
                "provider request timed out",
            ),
            ProviderError::RequestFailed(message) => Diagnostic::new(
                Severity::Error,
                code::CODE_PROVIDER_REQUEST_FAILED,
                SOURCE_PROVIDER,
                "provider request failed",
            )
            .details(serde_json::json!({ "provider_error": message })),
            ProviderError::StreamError(message) => Diagnostic::new(
                Severity::Error,
                code::CODE_PROVIDER_STREAM_ERROR,
                SOURCE_PROVIDER,
                "provider stream failed",
            )
            .details(serde_json::json!({ "provider_error": message })),
            ProviderError::AuthFailed(message) => Diagnostic::new(
                Severity::Error,
                code::CODE_PROVIDER_AUTH_FAILED,
                SOURCE_PROVIDER,
                "provider authentication failed",
            )
            .details(serde_json::json!({ "provider_error": message }))
            .action(ACTION_CHECK_CREDENTIALS),
            ProviderError::Network(message) => Diagnostic::new(
                Severity::Warning,
                code::CODE_PROVIDER_NETWORK,
                SOURCE_PROVIDER,
                "provider network or transport error",
            )
            .details(serde_json::json!({ "provider_error": message })),
            ProviderError::Config(message) => Diagnostic::new(
                Severity::Error,
                code::CODE_PROVIDER_CONFIG,
                SOURCE_PROVIDER,
                "invalid provider configuration",
            )
            .details(serde_json::json!({ "provider_error": message })),
            ProviderError::ProviderSide(message) => Diagnostic::new(
                Severity::Error,
                code::CODE_PROVIDER_SIDE,
                SOURCE_PROVIDER,
                "provider returned an error response",
            )
            .details(serde_json::json!({ "provider_error": message })),
            ProviderError::UnsupportedCapability(message) => Diagnostic::new(
                Severity::Error,
                code::CODE_PROVIDER_CAPABILITY_INVALID,
                SOURCE_PROVIDER,
                "provider does not support this capability",
            )
            .details(serde_json::json!({ "provider_error": message })),
            ProviderError::Cancelled => Diagnostic::new(
                Severity::Info,
                code::CODE_PROVIDER_CANCELLED,
                SOURCE_PROVIDER,
                "provider request cancelled",
            ),
        }
    }
}

/// Classify an [`crate::loop_types::AgentError`] into a [`Diagnostic`].
///
/// `AgentError::Provider`/`AuthFailed` map onto the provider vocabulary; tool
/// and hook failures carry their own sources; cancellation is informational
/// (harness/user-initiated, not a failure); the max-turns guard is a warning.
impl From<&crate::loop_types::AgentError> for Diagnostic {
    fn from(error: &crate::loop_types::AgentError) -> Self {
        use crate::loop_types::AgentError;
        match error {
            AgentError::Provider(message) => Diagnostic::new(
                Severity::Error,
                code::CODE_PROVIDER_ERROR,
                SOURCE_PROVIDER,
                "provider error",
            )
            .details(serde_json::json!({ "provider_error": message })),
            AgentError::AuthFailed(message) => Diagnostic::new(
                Severity::Error,
                code::CODE_PROVIDER_AUTH_FAILED,
                SOURCE_PROVIDER,
                "provider authentication failed",
            )
            .details(serde_json::json!({ "provider_error": message }))
            .action(ACTION_CHECK_CREDENTIALS),
            AgentError::Tool(message) => Diagnostic::new(
                Severity::Error,
                code::CODE_TOOL_FAILED,
                SOURCE_TOOL,
                "tool failed",
            )
            .details(serde_json::json!({ "tool_error": message })),
            AgentError::Hook(message) => Diagnostic::new(
                Severity::Error,
                code::CODE_HOOK_FAILED,
                SOURCE_AGENT,
                "hook failed",
            )
            .details(serde_json::json!({ "hook_error": message })),
            AgentError::Cancelled => Diagnostic::new(
                Severity::Info,
                code::CODE_AGENT_CANCELLED,
                SOURCE_AGENT,
                "agent run cancelled",
            ),
            AgentError::MaxTurnsExceeded(max_turns) => Diagnostic::new(
                Severity::Warning,
                code::CODE_AGENT_MAX_TURNS_EXCEEDED,
                SOURCE_AGENT,
                format!("max turns exceeded ({max_turns})"),
            )
            .details(serde_json::json!({ "max_turns": max_turns }))
            .action("increase max_turns or narrow the task"),
            AgentError::TraceSetup(message) => Diagnostic::new(
                Severity::Error,
                code::CODE_TRACE_SETUP_FAILED,
                SOURCE_AGENT,
                "trace setup failed",
            )
            .details(serde_json::json!({ "trace_error": message }))
            .action("check the trace path is writable and its parent directory exists"),
        }
    }
}

impl Diagnostic {
    /// Like [`Diagnostic::details`] but takes an `Option`, leaving details
    /// unset when `None`. Used by classification bridges that only attach
    /// details for some variants.
    fn details_option(self, details: Option<serde_json::Value>) -> Self {
        match details {
            Some(value) => self.details(value),
            None => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacted_constant_is_stable() {
        assert_eq!(REDACTED, "[REDACTED]");
    }

    #[test]
    fn content_sensitive_keys_match_case_insensitively() {
        assert!(is_content_sensitive_key("prompt"));
        assert!(is_content_sensitive_key("TOOL_OUTPUT"));
        assert!(is_content_sensitive_key("Env"));
        assert!(!is_content_sensitive_key("endpoint"));
    }

    #[test]
    fn fs_tool_error_codes_are_stable_literals() {
        assert_eq!(code::CODE_TOOL_PATH_NOT_FOUND, "tool_path_not_found");
        assert_eq!(code::CODE_TOOL_NOT_A_FILE, "tool_not_a_file");
        assert_eq!(code::CODE_TOOL_NOT_A_DIRECTORY, "tool_not_a_directory");
        assert_eq!(code::CODE_TOOL_PERMISSION_DENIED, "tool_permission_denied");
        assert_eq!(code::CODE_TOOL_BINARY_FILE, "tool_binary_file");
        assert_eq!(
            code::CODE_TOOL_UNSUPPORTED_ENCODING,
            "tool_unsupported_encoding"
        );
        assert_eq!(code::CODE_TOOL_OUTSIDE_WORKSPACE, "tool_outside_workspace");
        assert_eq!(
            code::CODE_TOOL_UNRESOLVED_WORKSPACE_ROOT,
            "tool_unresolved_workspace_root"
        );
    }

    #[test]
    fn fs_tool_error_taxonomy_maps_each_variant_to_its_code() {
        use std::path::PathBuf;
        let cases: Vec<(FsToolError, &str)> = vec![
            (
                FsToolError::NotFound {
                    user_path: "a/b.txt".into(),
                    resolved_path: None,
                },
                code::CODE_TOOL_PATH_NOT_FOUND,
            ),
            (
                FsToolError::NotAFile {
                    path: PathBuf::from("a"),
                },
                code::CODE_TOOL_NOT_A_FILE,
            ),
            (
                FsToolError::NotADirectory {
                    path: PathBuf::from("a"),
                },
                code::CODE_TOOL_NOT_A_DIRECTORY,
            ),
            (
                FsToolError::PermissionDenied {
                    path: PathBuf::from("a"),
                },
                code::CODE_TOOL_PERMISSION_DENIED,
            ),
            (
                FsToolError::BinaryFile {
                    path: PathBuf::from("a"),
                },
                code::CODE_TOOL_BINARY_FILE,
            ),
            (
                FsToolError::UnsupportedEncoding { omitted_count: 2 },
                code::CODE_TOOL_UNSUPPORTED_ENCODING,
            ),
            (
                FsToolError::OutsideWorkspace {
                    user_path: "../escape".into(),
                    symlink_traversed: false,
                },
                code::CODE_TOOL_OUTSIDE_WORKSPACE,
            ),
            (
                FsToolError::UnresolvedWorkspaceRoot {
                    source: "io error".into(),
                },
                code::CODE_TOOL_UNRESOLVED_WORKSPACE_ROOT,
            ),
        ];
        for (err, expected_code) in cases {
            assert_eq!(err.code(), expected_code, "variant code mismatch");
            assert!(
                !err.message().is_empty(),
                "variant message must be non-empty"
            );
            let diag = err.to_diagnostic();
            assert_eq!(diag.code, expected_code);
            assert_eq!(diag.message, err.message());
        }
    }
}
