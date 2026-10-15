//! Hidden `__native-capture` mode runtime (P19-AUD-003, decision D1).
//!
//! Binary-only module: it drives the bounded capture pipes, the process
//! group and the signal handling around the pure filter in
//! [`super::native_capture`]. It is compiled only into the executable
//! and adds no library or SDK surface. Unsafe FFI is confined to the
//! Unix process-group/signal primitives that have no safe alternative;
//! each call is justified inline.

use std::borrow::Cow;
use std::io::{Read, Write};
use std::process::{Command, Stdio};

use sha2::Digest;

use super::native_capture::{
    CAPTURE_HELPER_PROTOCOL, CAPTURE_RECEIPT_SCHEMA, CapturePolicy, CaptureRefusal,
    STREAM_LIMIT_BYTES, StreamKind, StreamSummary, detect_stream_kind, filter_record,
};

// ---------------------------------------------------------------------------
// Hidden capture mode runtime
// ---------------------------------------------------------------------------

/// Exit code used when the capture itself refuses: distinct from any
/// agent exit the caller may observe.
const CAPTURE_REFUSED_EXIT: i32 = 125;

#[cfg(unix)]
static CANCELLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(unix)]
extern "C" fn capture_signal(_signal: i32) {
    CANCELLED.store(true, std::sync::atomic::Ordering::SeqCst);
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Opens one capture destination: exclusive owner-only creation. A
/// preexisting path or link target refuses before any agent byte.
fn open_capture_destination(path: &std::path::Path) -> Result<std::fs::File, CaptureRefusal> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(path)
        .map_err(|error| CaptureRefusal::new(format!("capture destination refused: {error}")))
}

struct HiddenModeArgs {
    policy: CapturePolicy,
    stdout: std::path::PathBuf,
    stderr: std::path::PathBuf,
    receipt: std::path::PathBuf,
    command: Vec<std::ffi::OsString>,
}

fn parse_hidden_mode(args: &[std::ffi::OsString]) -> Result<HiddenModeArgs, String> {
    if args.first().map(|value| value.to_str()) == Some(Some("--selfcheck")) {
        return Err("selfcheck".to_owned());
    }
    let mut policy = None;
    let mut stdout = None;
    let mut stderr = None;
    let mut receipt = None;
    let mut command = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].to_str().ok_or("non-UTF-8 capture argument")?;
        match arg {
            "--policy" => {
                index += 1;
                let value = args
                    .get(index)
                    .and_then(|value| value.to_str())
                    .ok_or("--policy requires a value")?;
                policy = Some(
                    CapturePolicy::from_str(value)
                        .ok_or_else(|| format!("unknown capture policy {value:?}"))?,
                );
            }
            "--stdout" | "--stderr" | "--receipt" => {
                index += 1;
                let value = args
                    .get(index)
                    .cloned()
                    .ok_or_else(|| format!("{arg} requires a value"))?;
                match arg {
                    "--stdout" => stdout = Some(value),
                    "--stderr" => stderr = Some(value),
                    _ => receipt = Some(value),
                }
            }
            "--" => {
                command = args[index + 1..].to_vec();
                break;
            }
            other => return Err(format!("unknown capture argument {other:?}")),
        }
        index += 1;
    }
    let policy = policy.ok_or("--policy is required")?;
    let stdout = stdout.ok_or("--stdout is required")?;
    let stderr = stderr.ok_or("--stderr is required")?;
    let receipt = receipt.ok_or("--receipt is required")?;
    if command.is_empty() {
        return Err("an agent command is required after --".to_owned());
    }
    Ok(HiddenModeArgs {
        policy,
        stdout: stdout.into(),
        stderr: stderr.into(),
        receipt: receipt.into(),
        command,
    })
}

/// The explicitly validated diagnostic vocabulary: the typed provider
/// causes both pinned clients surface on stderr as complete plain
/// records. Unknown stderr is opaque by default; a marker-bearing or
/// malformed stream never persists.
const VALIDATED_STDERR_TOKENS: &[&str] = &[
    "provider authentication failed",
    "rate limited by provider",
    "provider stream failed",
];

/// Crate-attested static stream-failure summaries the pinned `opi-ai`
/// producers own as literal text (`ProviderErrorSummary::attested_static`).
const ATTESTED_STREAM_SUMMARIES: &[&str] = &[
    "stream ended without a terminal event",
    "stream ended without a terminal event (message_stop or error)",
    "invalid UTF-8 in Anthropic SSE frame",
    "Azure OpenAI returned a malformed streaming frame",
    "Vertex returned a malformed streaming frame",
];

/// The pinned structured stream diagnostic the Opi runner appends after
/// the plain summary. Parsing through this closed shape rejects unknown
/// fields and duplicate keys (`serde` reports duplicate fields as errors),
/// and every remaining dynamic surface is constrained by
/// [`proven_safe_cause`].
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StructuredStreamDiagnostic {
    severity: String,
    code: String,
    source: String,
    message: String,
    details: StructuredStreamCause,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StructuredStreamCause {
    cause: String,
}

/// Admit one whole structured stream-diagnostic record: closed field
/// set, the static stream-error identity, and a proven-safe cause.
fn validated_structured_diagnostic(line: &str) -> bool {
    let Ok(record) = serde_json::from_str::<StructuredStreamDiagnostic>(line) else {
        return false;
    };
    record.severity == "error"
        && record.code == "provider_stream_error"
        && record.source == "provider"
        && record.message == "provider stream failed"
        && proven_safe_cause(&record.details.cause)
}

/// Proven-safe cause grammar for the pinned structured Opi stream
/// diagnostic. Every admitted shape is a producer-owned
/// `ProviderErrorSummary` format: the closed redacted placeholder, the
/// HTTP status summaries, the crate-attested static summaries, and the
/// locally formatted usage/SSE-event diagnostics whose payloads are
/// bounded to digits or the lowercase SSE event-type vocabulary. No
/// arbitrary upstream text is admitted: unknown causes keep the whole
/// stream opaque instead of retaining bytes the vocabulary cannot prove
/// safe.
fn proven_safe_cause(cause: &str) -> bool {
    if cause == "[REDACTED]" {
        return true;
    }
    if let Some(rest) = cause.strip_prefix("HTTP ") {
        let Some((status, tail)) = rest.split_once(": ") else {
            return false;
        };
        return tail == "[REDACTED]"
            && (1..=3).contains(&status.len())
            && status.bytes().all(|byte| byte.is_ascii_digit());
    }
    if let Some(event) = cause
        .strip_prefix("malformed Anthropic SSE event '")
        .and_then(|rest| rest.strip_suffix('\''))
    {
        return proven_safe_sse_event_type(event);
    }
    for (left, right) in [
        (
            "cache_creation_input_tokens_1h",
            "cache_creation_input_tokens",
        ),
        ("reasoning_tokens", "completion_tokens"),
    ] {
        let Some(rest) = cause
            .strip_prefix(left)
            .and_then(|rest| rest.strip_prefix(" ("))
        else {
            continue;
        };
        let Some((left_count, remainder)) = rest.split_once(") exceeds ") else {
            continue;
        };
        let Some(counts) = remainder
            .strip_prefix(right)
            .and_then(|rest| rest.strip_prefix(" ("))
            .and_then(|rest| rest.strip_suffix(')'))
        else {
            continue;
        };
        if !left_count.is_empty()
            && left_count.bytes().all(|byte| byte.is_ascii_digit())
            && !counts.is_empty()
            && counts.bytes().all(|byte| byte.is_ascii_digit())
        {
            return true;
        }
    }
    ATTESTED_STREAM_SUMMARIES.contains(&cause)
}

/// SSE event types the pinned Anthropic-compatible wire admits: lowercase
/// snake-case identifiers, bounded by the producer's 256-char excerpt
/// ceiling. Provider-controlled mixed-case or symbolic text stays opaque.
fn proven_safe_sse_event_type(event: &str) -> bool {
    !event.is_empty()
        && event.len() <= 256
        && event
            .bytes()
            .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_'))
}

fn validate_agent_stderr(bytes: &[u8]) -> &'static str {
    if bytes.is_empty() {
        return "empty";
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return "opaque";
    };
    if bytes.len() > 64 * 1024 || text.bytes().any(|byte| byte == 0) {
        return "opaque";
    }
    for line in text.lines() {
        if line.len() > 4096 {
            return "opaque";
        }
        if line.is_empty() {
            continue;
        }
        // Whole-record admission only: a record is retained when it is
        // exactly one plain validated summary or one structured stream
        // diagnostic that passes the closed grammar. Substring matches,
        // prefixed/suffixed text, and unknown structured payloads keep
        // the stream opaque with no retained byte.
        let admitted =
            VALIDATED_STDERR_TOKENS.contains(&line) || validated_structured_diagnostic(line);
        if !admitted {
            return "opaque";
        }
    }
    "validated-diagnostic"
}

/// One filtered stream's persisted state.
struct FilteredStream {
    output: std::fs::File,
    kind: Option<StreamKind>,
    summary: StreamSummary,
    output_digest: sha2::Sha256,
    refusal: Option<CaptureRefusal>,
    source_bytes: u64,
    source_digest: sha2::Sha256,
    source_complete: bool,
}

impl FilteredStream {
    fn new(output: std::fs::File) -> Self {
        Self {
            output,
            kind: None,
            summary: StreamSummary::default(),
            output_digest: sha2::Sha256::new(),
            refusal: None,
            source_bytes: 0,
            source_digest: sha2::Sha256::new(),
            source_complete: false,
        }
    }

    /// Feeds one raw chunk; validates and filters every complete record.
    /// The offending record is never written on refusal.
    fn feed(&mut self, chunk: &[u8], buffered: &mut Vec<u8>) {
        buffered.extend_from_slice(chunk);
        self.source_bytes += chunk.len() as u64;
        self.source_digest.update(chunk);
        if self.source_bytes as usize > STREAM_LIMIT_BYTES {
            self.refusal.get_or_insert_with(|| {
                CaptureRefusal::new("stream exceeded the 64 MiB capture ceiling")
            });
            buffered.clear();
            return;
        }
        while let Some(position) = buffered.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = buffered.drain(..=position).collect();
            let record = &line[..line.len() - 1];
            if self.output_bytes() as usize > STREAM_LIMIT_BYTES {
                self.refusal.get_or_insert_with(|| {
                    CaptureRefusal::new("filtered output exceeded the 64 MiB capture ceiling")
                });
                return;
            }
            if record.is_empty() {
                // Blank separator lines between records are admitted.
                let _ = self.output.write_all(b"\n");
                self.output_digest.update(b"\n");
                self.summary.output_bytes += 1;
                continue;
            }
            let text = match std::str::from_utf8(record) {
                Ok(text) => text,
                Err(_) => {
                    self.refusal
                        .get_or_insert(CaptureRefusal::new("record is not UTF-8"));
                    return;
                }
            };
            let kind = match self.kind {
                Some(kind) => kind,
                None => match detect_stream_kind(text) {
                    Ok(kind) => {
                        self.kind = Some(kind);
                        kind
                    }
                    Err(refusal) => {
                        self.refusal.get_or_insert(refusal);
                        return;
                    }
                },
            };
            match filter_record(text, kind) {
                Ok(filtered) => {
                    let bytes = filtered.as_bytes();
                    let _ = self.output.write_all(bytes);
                    let _ = self.output.write_all(b"\n");
                    self.output_digest.update(bytes);
                    self.output_digest.update(b"\n");
                    self.summary.records += 1;
                    self.summary.output_bytes += bytes.len() as u64 + 1;
                    if matches!(filtered, Cow::Owned(_)) {
                        self.summary.transformed_records += 1;
                    }
                }
                Err(refusal) => {
                    self.refusal.get_or_insert(refusal);
                    return;
                }
            }
        }
        if buffered.len() > STREAM_LIMIT_BYTES {
            self.refusal.get_or_insert(CaptureRefusal::new(
                "single record exceeded the 64 MiB capture ceiling",
            ));
        }
    }

    fn finish(&mut self, buffered: &[u8]) {
        self.source_complete = buffered.is_empty();
        if !buffered.is_empty() {
            // A trailing fragment without its newline is an incomplete
            // record: it is never written.
            self.refusal.get_or_insert(CaptureRefusal::new(
                "stream ended inside a record (no trailing newline)",
            ));
        }
    }

    fn output_bytes(&self) -> u64 {
        self.summary.output_bytes
    }

    fn write_receipt_fields(&self, receipt: &mut serde_json::Map<String, serde_json::Value>) {
        receipt.insert(
            "stream".to_owned(),
            serde_json::json!(match self.kind {
                Some(StreamKind::OpiAgent) => "opi-agent-ndjson",
                Some(StreamKind::PiSession) => "pi-session-ndjson",
                None => "unknown",
            }),
        );
        receipt.insert(
            "records".to_owned(),
            serde_json::json!(self.summary.records),
        );
        receipt.insert(
            "transformed_records".to_owned(),
            serde_json::json!(self.summary.transformed_records),
        );
        receipt.insert(
            "stdout".to_owned(),
            serde_json::json!({
                "bytes": self.summary.output_bytes,
                "sha256": format!("{:x}", self.output_digest.clone().finalize()),
                "truncated": self.summary.truncated,
            }),
        );
        receipt.insert(
            "source".to_owned(),
            serde_json::json!({
                "bytes": self.source_bytes,
                "sha256": format!("{:x}", self.source_digest.clone().finalize()),
                "complete": self.source_complete,
            }),
        );
    }
}

/// The hidden `__native-capture` mode entry point.
pub(crate) fn run_hidden_mode(
    args: &[std::ffi::OsString],
    program: Option<&std::ffi::OsString>,
) -> std::process::ExitCode {
    let _ = program;
    let parsed = match parse_hidden_mode(args) {
        Ok(parsed) => parsed,
        Err(reason) if reason == "selfcheck" => {
            println!(
                "{}",
                serde_json::json!({
                    "schema": CAPTURE_HELPER_PROTOCOL,
                    "protocol": CAPTURE_RECEIPT_SCHEMA,
                    "policies": [CapturePolicy::Full.as_str(), CapturePolicy::TerminalOnly.as_str()],
                })
            );
            return std::process::ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("__native-capture: {error}");
            return std::process::ExitCode::from(125);
        }
    };
    match capture_agent(&parsed) {
        Ok(code) => std::process::ExitCode::from(code.unsigned_abs().min(255) as u8),
        Err(refusal) => {
            eprintln!("__native-capture: {}", refusal.reason);
            std::process::ExitCode::from(CAPTURE_REFUSED_EXIT as u8)
        }
    }
}

fn capture_agent(args: &HiddenModeArgs) -> Result<i32, CaptureRefusal> {
    #[cfg(unix)]
    {
        // The helper leads its own process group so the runner's owned
        // cancellation reaches the helper and the agent together; the
        // agent stays a child of the group and its exit is observed here.
        // SAFETY: setpgid(0, 0) moves this process into a new group led
        // by itself; no other process is addressed.
        if unsafe { libc::setpgid(0, 0) } != 0 {
            return Err(CaptureRefusal::new("cannot lead the capture process group"));
        }
        // SAFETY: signal() registers the handler for this process only.
        unsafe {
            libc::signal(
                libc::SIGTERM,
                capture_signal as *const () as libc::sighandler_t,
            );
            libc::signal(
                libc::SIGINT,
                capture_signal as *const () as libc::sighandler_t,
            );
        }
    }
    let stdout_file = open_capture_destination(&args.stdout)?;
    let mut stderr_file = open_capture_destination(&args.stderr)?;
    // Shell-style environment prefixes on the agent command (for example
    // `PI_CODING_AGENT_DIR=<stage> <agent> ...`) apply to the spawned
    // agent, exactly as the previous direct shell launch applied them.
    let is_assignment = |value: &std::ffi::OsString| {
        value.to_str().is_some_and(|text| {
            let mut characters = text.chars();
            matches!(characters.next(), Some('_' | 'a'..='z' | 'A'..='Z'))
                && text.contains('=')
                && text
                    .chars()
                    .take_while(|character| *character != '=')
                    .all(|character| matches!(character, '_' | 'a'..='z' | 'A'..='Z' | '0'..='9'))
        })
    };
    let split = args
        .command
        .iter()
        .position(|value| !is_assignment(value))
        .unwrap_or(args.command.len());
    let (assignments, argv) = args.command.split_at(split);
    if argv.is_empty() {
        return Err(CaptureRefusal::new("an agent command is required"));
    }
    let mut command = Command::new(&argv[0]);
    command.args(&argv[1..]);
    for assignment in assignments {
        let text = assignment.to_str().expect("checked assignment is UTF-8");
        let (key, value) = text
            .split_once('=')
            .expect("checked assignment contains '='");
        command.env(key, value);
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| CaptureRefusal::new(format!("cannot spawn agent: {error}")))?;
    let mut child_stdout = child.stdout.take().expect("piped");
    let mut child_stderr = child.stderr.take().expect("piped");
    // Drain stderr concurrently: an agent writing a full pipe of stderr
    // while the helper reads stdout would otherwise deadlock. The drain
    // stays bounded by the capture ceiling; overflow is detected after
    // the join and refuses.
    let stderr_drain = std::thread::spawn(move || {
        let mut bytes: Vec<u8> = Vec::new();
        let mut overflow = false;
        let mut chunk = [0u8; 64 * 1024];
        loop {
            match child_stderr.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    if bytes.len() + count > STREAM_LIMIT_BYTES {
                        overflow = true;
                    } else {
                        bytes.extend_from_slice(&chunk[..count]);
                    }
                }
            }
        }
        (bytes, overflow)
    });
    let mut stream = FilteredStream::new(stdout_file);
    let mut buffered: Vec<u8> = Vec::new();
    #[expect(
        unused_variables,
        reason = "the concurrent drain assigns the collected stderr after the loop"
    )]
    let stderr_bytes: Vec<u8>;
    let refusal;
    let cancelled;
    loop {
        #[cfg(unix)]
        {
            if CANCELLED.load(std::sync::atomic::Ordering::SeqCst) {
                let _ = child.kill();
            }
        }
        let mut chunk = [0u8; 64 * 1024];
        let read = child_stdout.read(&mut chunk);
        match read {
            Ok(0) => {
                stream.finish(&buffered);
                cancelled = false;
                refusal = stream.refusal.clone();
                break;
            }
            Ok(count) => {
                stream.feed(&chunk[..count], &mut buffered);
                if stream.refusal.is_some() {
                    let _ = child.kill();
                    // Drain so the child is reaped below; the concurrent
                    // stderr drain keeps collecting until pipe EOF.
                    let mut sink = Vec::new();
                    let _ = child_stdout.read_to_end(&mut sink);
                    stream.finish(b"");
                    cancelled = false;
                    refusal = stream.refusal.clone();
                    break;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => {
                stream.refusal.get_or_insert(CaptureRefusal::new(format!(
                    "agent stdout read failed: {error}"
                )));
                let _ = child.kill();
                stream.finish(b"");
                cancelled = false;
                refusal = stream.refusal.clone();
                break;
            }
        }
    }
    let (stderr_bytes, stderr_overflow) = stderr_drain
        .join()
        .map_err(|_| CaptureRefusal::new("agent stderr drain failed"))?;
    if stderr_overflow {
        stream.refusal.get_or_insert(CaptureRefusal::new(
            "agent stderr exceeded the 64 MiB capture ceiling",
        ));
    }
    let status = child
        .wait()
        .map_err(|error| CaptureRefusal::new(format!("agent wait failed: {error}")))?;
    let agent_exit = status.code();
    // Agent stderr admits exactly two retained forms: empty, and the
    // explicitly validated diagnostic vocabulary the pinned clients'
    // typed provider causes use. Anything else is opaque: no byte is
    // retained, only static count/digest metadata, and the capture
    // excludes a native score.
    let stderr_status = validate_agent_stderr(&stderr_bytes);
    let retained_stderr: &[u8] = if stderr_status == "validated-diagnostic" {
        &stderr_bytes
    } else {
        b""
    };
    let _ = stderr_file.write_all(retained_stderr);
    let _ = stderr_file.flush();
    let _ = stream.output.flush();
    let refusal = refusal.or_else(|| stream.refusal.clone());
    #[cfg(unix)]
    let signal_cancelled = CANCELLED.load(std::sync::atomic::Ordering::SeqCst);
    #[cfg(not(unix))]
    let signal_cancelled = false;
    let cancelled = cancelled || signal_cancelled;
    let status = match (&refusal, cancelled) {
        (Some(_), _) => "refused",
        (None, true) => "incomplete",
        (None, false) => "sanitized",
    };
    let mut receipt = serde_json::Map::new();
    receipt.insert(
        "schema".to_owned(),
        serde_json::json!(CAPTURE_RECEIPT_SCHEMA),
    );
    receipt.insert("policy".to_owned(), serde_json::json!(args.policy.as_str()));
    receipt.insert("status".to_owned(), serde_json::json!(status));
    receipt.insert("agent_exit_code".to_owned(), serde_json::json!(agent_exit));
    receipt.insert("cancelled".to_owned(), serde_json::json!(cancelled));
    if let Some(refusal) = &refusal {
        receipt.insert(
            "refusal_reason".to_owned(),
            serde_json::json!(refusal.reason),
        );
    } else {
        receipt.insert("refusal_reason".to_owned(), serde_json::Value::Null);
    }
    stream.write_receipt_fields(&mut receipt);
    receipt.insert(
        "stderr".to_owned(),
        serde_json::json!({
            "status": stderr_status,
            "bytes": stderr_bytes.len(),
            "sha256": sha256_hex(&stderr_bytes),
        }),
    );
    let receipt_file = open_capture_destination(&args.receipt)?;
    serde_json::to_writer_pretty(&receipt_file, &serde_json::Value::Object(receipt))
        .map_err(|error| CaptureRefusal::new(format!("cannot write capture receipt: {error}")))?;
    match refusal {
        Some(_) => Err(CaptureRefusal::new("capture refused")),
        None => Ok(agent_exit.unwrap_or(-1)),
    }
}
