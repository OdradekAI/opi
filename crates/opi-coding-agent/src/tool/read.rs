use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use opi_agent::diagnostic::{FsToolError, code};
use opi_agent::tool::ToolDiagnostic;
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult, result};
use opi_ai::message::{OutputContent, ToolDef};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use tokio::io::AsyncReadExt;
use tokio_util::sync::CancellationToken;

use super::PathPolicy;

/// Default number of lines returned when the caller omits `limit`.
///
/// Bounds output for the model without special-casing the explicit-window
/// contract: when the caller supplies `limit`, that value controls the line
/// window and is not capped by `DEFAULT_READ_LINES`. The separate byte cap
/// still bounds the returned body.
const DEFAULT_READ_LINES: usize = 2000;

/// Maximum UTF-8 body bytes returned by one read call.
pub const MAX_READ_OUTPUT_BYTES: usize = 64 * 1024;

const READ_BYTE_CAP_MARKER: &str = "... output truncated by byte cap";
const READ_CHUNK_BYTES: usize = 8 * 1024;
const READ_BODY_BUFFER_BYTES: usize = MAX_READ_OUTPUT_BYTES + READ_BYTE_CAP_MARKER.len() + 8;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadArgs {
    /// Relative path within workspace to read.
    pub path: String,
    /// 1-based line offset (optional, defaults to 1).
    pub offset: Option<usize>,
    /// Maximum number of lines to read (optional, defaults to
    /// [`DEFAULT_READ_LINES`]).
    ///
    /// When omitted, output is capped at `DEFAULT_READ_LINES` lines and the
    /// remainder is reported via `truncated`/`omitted`. When supplied, that many
    /// lines are selected without reapplying the default line cap; the byte cap
    /// still applies. `limit: 0` returns no lines and flags the result
    /// truncated.
    pub limit: Option<usize>,
}

pub struct ReadTool {
    workspace_root: PathBuf,
    path_policy: PathPolicy,
    schema: serde_json::Value,
}

impl ReadTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self::new_with_policy(workspace_root, PathPolicy::WorkspaceOnly)
    }

    pub fn new_with_policy(workspace_root: PathBuf, path_policy: PathPolicy) -> Self {
        let schema = schemars::schema_for!(ReadArgs);
        Self {
            workspace_root,
            path_policy,
            schema: serde_json::to_value(&schema).unwrap_or_default(),
        }
    }
}

/// Display path for a read result: workspace-relative when the file is inside
/// the workspace, otherwise the user-supplied path. Keeping inside-workspace
/// reads relative avoids leaking the resolved absolute workspace root to the
/// model and into public NDJSON `details`.
fn display_path_for_tool_result(
    workspace_root: &std::path::Path,
    file_path: &std::path::Path,
    user_path: &str,
) -> String {
    if let Ok(relative) = file_path.strip_prefix(workspace_root)
        && !relative.as_os_str().is_empty()
    {
        return relative.display().to_string();
    }
    user_path.to_owned()
}

impl Tool for ReadTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "read".into(),
            description: "Read file content with optional line range.".into(),
            input_schema: self.schema.clone(),
        }
    }

    fn execute(
        &self,
        _call_id: &str,
        arguments: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let args: ReadArgs = match serde_json::from_value(arguments) {
            Ok(a) => a,
            Err(e) => {
                return Box::pin(async move {
                    Ok(result::err(vec![OutputContent::Text {
                        text: format!("invalid arguments: {e}"),
                    }]))
                });
            }
        };
        let resolved_path =
            match super::resolve_tool_path(&self.workspace_root, &args.path, self.path_policy) {
                Ok(p) => p,
                Err(e) => {
                    return Box::pin(async move { Ok(super::fs_error_result(e)) });
                }
            };
        let workspace_relation = resolved_path.workspace_relation;
        let file_path = resolved_path.path;
        let workspace_root = self.workspace_root.clone();
        let path_for_display = args.path.clone();
        Box::pin(async move {
            // Directories are rejected before any byte read so the NotAFile cause
            // is reported instead of a binary/encoding error from reading a dir.
            if file_path.is_dir() {
                return Ok(super::fs_error_result(FsToolError::NotAFile {
                    path: file_path.clone(),
                }));
            }

            // offset is 1-based; values below 1 floor to 1 so the reported
            // offset always matches the effective start line.
            let offset_1 = args.offset.unwrap_or(1).max(1);
            let take_n = args.limit.unwrap_or(DEFAULT_READ_LINES);
            let scan = match stream_read_window(&file_path, offset_1, take_n).await {
                Ok(scan) => scan,
                Err(ReadFileError::Io(e)) => match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        return Ok(super::fs_error_result(FsToolError::NotFound {
                            user_path: path_for_display.clone(),
                            resolved_path: Some(file_path.clone()),
                        }));
                    }
                    std::io::ErrorKind::PermissionDenied => {
                        return Ok(super::fs_error_result(FsToolError::PermissionDenied {
                            path: file_path.clone(),
                        }));
                    }
                    _ => {
                        return Ok(result::err(vec![OutputContent::Text {
                            text: format!("failed to read {}: {e}", file_path.display()),
                        }]));
                    }
                },
                Err(ReadFileError::BinaryFile) => {
                    return Ok(super::fs_error_result(FsToolError::BinaryFile {
                        path: file_path.clone(),
                    }));
                }
                Err(ReadFileError::UnsupportedEncoding { byte_offset }) => {
                    // File-content encoding failure is reported with the shared
                    // tool_unsupported_encoding code but a content-appropriate
                    // message: the FsToolError::UnsupportedEncoding variant is
                    // directory/entry-shaped and reused by ls/find, so the
                    // single-file case builds the diagnostic directly. The agent
                    // loop lifts this into Phase 7 traces (task 11.8).
                    let message = format!("'{}' is not valid UTF-8", file_path.display());
                    let mut unsupported = result::err(vec![OutputContent::Text {
                        text: message.clone(),
                    }]);
                    unsupported.diagnostics.push(ToolDiagnostic {
                        code: code::CODE_TOOL_UNSUPPORTED_ENCODING.to_string(),
                        message,
                        context: json!({
                            "path": file_path.display().to_string(),
                            "byte_offset": byte_offset,
                        }),
                    });
                    return Ok(unsupported);
                }
            };

            let total_lines = scan.line_count;
            let line_ending = scan.line_ending;
            let mut body = match String::from_utf8(scan.body) {
                Ok(body) => body,
                Err(e) => {
                    let byte_offset = e.utf8_error().valid_up_to();
                    let message = format!("'{}' is not valid UTF-8", file_path.display());
                    let mut unsupported = result::err(vec![OutputContent::Text {
                        text: message.clone(),
                    }]);
                    unsupported.diagnostics.push(ToolDiagnostic {
                        code: code::CODE_TOOL_UNSUPPORTED_ENCODING.to_string(),
                        message,
                        context: json!({
                            "path": file_path.display().to_string(),
                            "byte_offset": byte_offset,
                        }),
                    });
                    return Ok(unsupported);
                }
            };

            // The clamp keeps the subtraction below safe (offset_idx <= total_lines).
            let offset_idx = offset_1.saturating_sub(1).min(total_lines);
            let available = total_lines - offset_idx;
            let returned = take_n.min(available);
            let omitted = available - returned;

            if omitted > 0 {
                append_read_marker(&mut body, &format!("... {omitted} lines omitted"));
            } else if total_lines > 0 && offset_1 > total_lines {
                // The window started past the end of the file; the read itself
                // succeeded but no lines apply. Surface the mismatch rather than
                // returning an empty body, without marking the result an error.
                body = format!(
                    "offset {offset_1} is past end of file (line_count {total_lines}); no lines returned"
                );
            }

            let mut truncation_reason = None;
            if body.len() > MAX_READ_OUTPUT_BYTES {
                let marker_len = 1 + READ_BYTE_CAP_MARKER.len();
                let mut end = MAX_READ_OUTPUT_BYTES
                    .saturating_sub(marker_len)
                    .min(body.len());
                while !body.is_char_boundary(end) {
                    end -= 1;
                }
                body.truncate(end);
                append_read_marker(&mut body, READ_BYTE_CAP_MARKER);
                truncation_reason = Some("byte_cap");
            }
            let truncated = omitted > 0 || truncation_reason.is_some();

            let mut details = result::path_metadata(
                &workspace_root,
                &path_for_display,
                &file_path,
                workspace_relation,
            );
            details["line_count"] = json!(total_lines);
            details["offset"] = json!(offset_1);
            details["limit"] = json!(take_n);
            details["truncated"] = json!(truncated);
            details["omitted"] = json!(omitted);
            details["line_ending"] = json!(line_ending);
            if let Some(reason) = truncation_reason {
                details["truncation_reason"] = json!(reason);
            }

            // Relativize the structured details for inside-workspace reads so the
            // resolved absolute workspace root does not leak via public NDJSON
            // `ToolExecutionEnd` events (the `details` block is carried verbatim).
            if matches!(workspace_relation, result::WorkspaceRelation::Inside) {
                if details.get("workspace_root").is_some() {
                    details["workspace_root"] = json!(".");
                }
                if let Some(resolved) = details
                    .get("resolved_path")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
                {
                    // Canonicalize the workspace root before stripping: on macOS
                    // the tempdir lives under /var (a symlink to /private/var), so
                    // the canonicalized resolved_path diverges from the raw
                    // workspace_root and strip_prefix would otherwise fail,
                    // leaking the absolute root through public details. On
                    // Linux/Windows without such a symlink this is a no-op.
                    let root_for_strip = std::fs::canonicalize(&workspace_root)
                        .unwrap_or_else(|_| workspace_root.clone());
                    if let Ok(rel) =
                        std::path::Path::new(&resolved).strip_prefix(&root_for_strip)
                    {
                        details["resolved_path"] = json!(rel.display().to_string());
                    }
                }
            }

            let display_path =
                display_path_for_tool_result(&workspace_root, &file_path, &path_for_display);
            let text = format!("{display_path}\n{body}");
            let mut res = result::ok(vec![OutputContent::Text { text }], details);
            res.truncated = truncated;
            Ok(res)
        })
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Parallel
    }
}

struct StreamRead {
    body: Vec<u8>,
    line_count: usize,
    line_ending: &'static str,
}

enum ReadFileError {
    Io(std::io::Error),
    BinaryFile,
    UnsupportedEncoding { byte_offset: usize },
}

async fn stream_read_window(
    path: &Path,
    offset_1: usize,
    take_n: usize,
) -> Result<StreamRead, ReadFileError> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(ReadFileError::Io)?;
    let mut buffer = [0u8; READ_CHUNK_BYTES];
    let mut accumulator = ReadAccumulator::new(offset_1, take_n);
    let mut utf8 = Utf8StreamValidator::default();
    let mut saw_nul = false;

    loop {
        let n = file.read(&mut buffer).await.map_err(ReadFileError::Io)?;
        if n == 0 {
            break;
        }
        let chunk = &buffer[..n];
        if chunk.contains(&0u8) {
            saw_nul = true;
        }
        utf8.push(chunk);
        accumulator.push_chunk(chunk);
    }

    accumulator.finish();
    if saw_nul {
        return Err(ReadFileError::BinaryFile);
    }
    if let Some(byte_offset) = utf8.finish() {
        return Err(ReadFileError::UnsupportedEncoding { byte_offset });
    }

    let line_ending = accumulator.line_ending();
    let mut body = accumulator.selected;
    truncate_to_valid_utf8_prefix(&mut body);
    Ok(StreamRead {
        body,
        line_count: accumulator.line_count,
        line_ending,
    })
}

#[derive(Default)]
struct Utf8StreamValidator {
    pending: Vec<u8>,
    bytes_seen: usize,
    first_error: Option<usize>,
}

impl Utf8StreamValidator {
    fn push(&mut self, chunk: &[u8]) {
        if self.first_error.is_some() {
            self.bytes_seen += chunk.len();
            return;
        }

        let base_offset = self.bytes_seen.saturating_sub(self.pending.len());
        let mut combined = Vec::with_capacity(self.pending.len() + chunk.len());
        combined.extend_from_slice(&self.pending);
        combined.extend_from_slice(chunk);
        match std::str::from_utf8(&combined) {
            Ok(_) => self.pending.clear(),
            Err(e) if e.error_len().is_none() => {
                self.pending = combined[e.valid_up_to()..].to_vec();
            }
            Err(e) => {
                self.first_error = Some(base_offset + e.valid_up_to());
                self.pending.clear();
            }
        }
        self.bytes_seen += chunk.len();
    }

    fn finish(&self) -> Option<usize> {
        self.first_error.or_else(|| {
            (!self.pending.is_empty()).then(|| self.bytes_seen.saturating_sub(self.pending.len()))
        })
    }
}

struct ReadAccumulator {
    offset_1: usize,
    take_n: usize,
    line_count: usize,
    selected: Vec<u8>,
    selected_seen: usize,
    line_has_bytes: bool,
    pending_cr: bool,
    saw_lf: bool,
    saw_crlf: bool,
    saw_cr: bool,
}

impl ReadAccumulator {
    fn new(offset_1: usize, take_n: usize) -> Self {
        Self {
            offset_1,
            take_n,
            line_count: 0,
            selected: Vec::new(),
            selected_seen: 0,
            line_has_bytes: false,
            pending_cr: false,
            saw_lf: false,
            saw_crlf: false,
            saw_cr: false,
        }
    }

    fn push_chunk(&mut self, chunk: &[u8]) {
        for &byte in chunk {
            self.push_byte(byte);
        }
    }

    fn push_byte(&mut self, byte: u8) {
        if self.pending_cr {
            if byte == b'\n' {
                self.push_selected(byte);
                self.finish_line(LineEndingKind::Crlf);
                self.pending_cr = false;
                return;
            }
            self.finish_line(LineEndingKind::Cr);
            self.pending_cr = false;
        }

        self.line_has_bytes = true;
        self.push_selected(byte);
        match byte {
            b'\r' => self.pending_cr = true,
            b'\n' => self.finish_line(LineEndingKind::Lf),
            _ => {}
        }
    }

    fn finish(&mut self) {
        if self.pending_cr {
            self.finish_line(LineEndingKind::Cr);
            self.pending_cr = false;
        } else if self.line_has_bytes {
            self.line_count += 1;
            self.line_has_bytes = false;
        }
    }

    fn push_selected(&mut self, byte: u8) {
        if !self.current_line_selected() {
            return;
        }
        self.selected_seen += 1;
        if self.selected.len() < READ_BODY_BUFFER_BYTES {
            self.selected.push(byte);
        }
    }

    fn current_line_selected(&self) -> bool {
        if self.take_n == 0 {
            return false;
        }
        let line_no = self.line_count + 1;
        line_no >= self.offset_1 && line_no - self.offset_1 < self.take_n
    }

    fn finish_line(&mut self, ending: LineEndingKind) {
        match ending {
            LineEndingKind::Lf => self.saw_lf = true,
            LineEndingKind::Crlf => self.saw_crlf = true,
            LineEndingKind::Cr => self.saw_cr = true,
        }
        self.line_count += 1;
        self.line_has_bytes = false;
    }

    fn line_ending(&self) -> &'static str {
        match (self.saw_lf, self.saw_crlf, self.saw_cr) {
            (false, false, false) => "none",
            (true, false, false) => "lf",
            (false, true, false) => "crlf",
            (false, false, true) => "cr",
            _ => "mixed",
        }
    }
}

enum LineEndingKind {
    Lf,
    Crlf,
    Cr,
}

fn truncate_to_valid_utf8_prefix(bytes: &mut Vec<u8>) {
    while std::str::from_utf8(bytes).is_err() {
        bytes.pop();
    }
}

fn append_read_marker(body: &mut String, marker: &str) {
    if !body.is_empty() && !body.ends_with('\n') && !body.ends_with('\r') {
        body.push('\n');
    }
    body.push_str(marker);
}
