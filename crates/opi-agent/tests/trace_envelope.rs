//! Phase 7 task 7.3 — local trace envelope and trace sink.
//!
//! These tests pin the unstable 0.x trace envelope substrate that lives in
//! `opi-agent` before any CLI/RPC surface wires it up (that wiring is task
//! 7.5). They cover: the schema-version stamp, monotonic per-run sequence
//! numbers, the run/turn/provider/tool/diagnostic-linked record structure,
//! summary and verbose redaction of details, and the trace-sink failure
//! contract — fail closed before a run (a prepare error aborts) and fail open
//! during a run (a write error emits a `trace_sink_failed` diagnostic then
//! disables the sink so the run continues).

mod common;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use opi_agent::diagnostic::code::*;
use opi_agent::diagnostic::{RedactionMode, SOURCE_AGENT, SOURCE_PROVIDER, SOURCE_TOOL, Severity};
use opi_agent::diagnostic_sink::RecordingSink;
use opi_agent::session_event::CompactionReason;
use opi_agent::{
    AgentEvent, FileTraceSink, RecordingTraceSink, TRACE_SCHEMA_VERSION, TraceCollector,
    TraceError, TraceKind, TraceRecord, TraceSink,
};

// A 20+ char Anthropic-style key body that SecretRedactor scrubs by value
// pattern (sk-ant-[a-zA-Z0-9]{20,}) in both Summary and Verbose modes.
const SCRUBBED_KEY: &str = "sk-ant-AAAAAAAAAAAAAAAAAAAAsecret";

// ---------------------------------------------------------------------------
// Schema version + envelope fields
// ---------------------------------------------------------------------------

#[test]
fn schema_version_is_unstable_v1() {
    // Unstable 0.x envelope; pin the literal so a bump is a deliberate change.
    assert_eq!(TRACE_SCHEMA_VERSION, 1u32);
}

#[test]
fn every_record_carries_schema_version_run_id_source() {
    let sink = Arc::new(RecordingTraceSink::new());
    let collector = TraceCollector::new("run-7", RedactionMode::default(), sink.clone(), None);
    collector
        .prepare()
        .expect("recording sink prepare is infallible");
    collector.record(SOURCE_AGENT, TraceKind::RunStarted).emit();

    let rec = &sink.snapshot()[0];
    assert_eq!(rec.schema_version, TRACE_SCHEMA_VERSION);
    assert_eq!(rec.run_id, "run-7");
    assert_eq!(rec.source, SOURCE_AGENT);
    assert!(rec.timestamp > 0, "timestamp must be populated");
}

#[test]
fn trace_kind_serializes_to_stable_snake_case() {
    let cases: &[(TraceKind, &str)] = &[
        (TraceKind::RunStarted, "run_started"),
        (TraceKind::RunEnded, "run_ended"),
        (TraceKind::TurnStarted, "turn_started"),
        (TraceKind::TurnEnded, "turn_ended"),
        (TraceKind::ProviderRequest, "provider_request"),
        (
            TraceKind::ProviderStreamCompletion,
            "provider_stream_completion",
        ),
        (TraceKind::ProviderRetry, "provider_retry"),
        (TraceKind::ProviderFailure, "provider_failure"),
        (TraceKind::ToolCallStarted, "tool_call_started"),
        (TraceKind::ToolCallCompleted, "tool_call_completed"),
        (TraceKind::ToolCallFailed, "tool_call_failed"),
        (TraceKind::ToolCallCancelled, "tool_call_cancelled"),
        (TraceKind::DiagnosticLinked, "diagnostic_linked"),
    ];
    let mut seen = std::collections::HashSet::new();
    for (kind, expected) in cases {
        let json = serde_json::to_string(kind).expect("kind serializes");
        assert_eq!(
            json,
            format!("\"{expected}\""),
            "wrong snake_case for {kind:?}"
        );
        assert!(
            seen.insert(json),
            "two TraceKind variants collapsed to the same wire string"
        );
    }
}

#[test]
fn sequence_is_strictly_monotonic() {
    let sink = Arc::new(RecordingTraceSink::new());
    let collector = TraceCollector::new("run-seq", RedactionMode::default(), sink.clone(), None);
    collector.prepare().unwrap();
    for kind in [
        TraceKind::RunStarted,
        TraceKind::TurnStarted,
        TraceKind::ProviderRequest,
    ] {
        collector.record(SOURCE_AGENT, kind).emit();
    }
    let seqs: Vec<u64> = sink.snapshot().iter().map(|r| r.sequence).collect();
    assert_eq!(seqs, vec![0, 1, 2], "sequences must be strictly monotonic");
}

// ---------------------------------------------------------------------------
// run / turn / provider / tool / diagnostic-linked record structure
// ---------------------------------------------------------------------------

#[test]
fn run_records_have_no_turn_id_turn_records_do() {
    let sink = Arc::new(RecordingTraceSink::new());
    let collector = TraceCollector::new("run-rt", RedactionMode::default(), sink.clone(), None);
    collector.prepare().unwrap();

    collector.record(SOURCE_AGENT, TraceKind::RunStarted).emit();
    collector
        .record(SOURCE_AGENT, TraceKind::TurnStarted)
        .turn("turn-1")
        .emit();

    let snap = sink.snapshot();
    // Run record carries no turn id and omits the field on the wire.
    assert!(snap[0].turn_id.is_none());
    let run_json = serde_json::to_string(&snap[0]).unwrap();
    assert!(
        !run_json.contains("turn_id"),
        "run record must omit turn_id, got: {run_json}"
    );
    // Turn record carries the turn id on the wire.
    assert_eq!(snap[1].turn_id.as_deref(), Some("turn-1"));
    let turn_json = serde_json::to_string(&snap[1]).unwrap();
    assert!(
        turn_json.contains("\"turn_id\":\"turn-1\""),
        "turn record must serialize turn_id, got: {turn_json}"
    );
}

#[test]
fn provider_and_tool_records_carry_their_sources() {
    let sink = Arc::new(RecordingTraceSink::new());
    let collector = TraceCollector::new("run-pt", RedactionMode::default(), sink.clone(), None);
    collector.prepare().unwrap();
    collector
        .record(SOURCE_PROVIDER, TraceKind::ProviderRequest)
        .emit();
    collector
        .record(SOURCE_TOOL, TraceKind::ToolCallStarted)
        .emit();

    let snap = sink.snapshot();
    assert_eq!(snap[0].source, SOURCE_PROVIDER);
    assert_eq!(snap[0].kind, TraceKind::ProviderRequest);
    assert_eq!(snap[1].source, SOURCE_TOOL);
    assert_eq!(snap[1].kind, TraceKind::ToolCallStarted);
}

#[test]
fn diagnostic_linked_record_carries_severity_and_code() {
    let sink = Arc::new(RecordingTraceSink::new());
    let collector = TraceCollector::new("run-dl", RedactionMode::default(), sink.clone(), None);
    collector.prepare().unwrap();
    collector
        .record(SOURCE_PROVIDER, TraceKind::DiagnosticLinked)
        .severity(Severity::Error)
        .diagnostic_code(CODE_PROVIDER_TIMEOUT)
        .emit();

    let rec = &sink.snapshot()[0];
    assert_eq!(rec.kind, TraceKind::DiagnosticLinked);
    assert_eq!(rec.severity, Some(Severity::Error));
    assert_eq!(rec.diagnostic_code, Some(CODE_PROVIDER_TIMEOUT));
}

/// Phase 7 task 7.6 — DoD SC6 (trace diagnostic-linked boundary): if a
/// diagnostic-linked record ever carries structured details (today the agent
/// loop attaches only severity + code, but the envelope must stay safe if a
/// future caller adds details), the centralized redaction at the trace emit
/// boundary scrubs every sensitive class. Pins `trace.rs` emit-inner redaction
/// for the diagnostic-linked path in both modes.
#[test]
fn phase7_trace_redacts_sensitive_values_in_diagnostic_linked() {
    fn secret_details() -> serde_json::Value {
        serde_json::json!({
            "api_key": "sk-ant-1234567890abcdefghijklmnopqrstuv",
            "github_pat": "ghp_01234567890123456789012345678901234567",
            "package_source": "https://alice:s3cr3t@gitlab.example.com/o/r.git",
            "authorization": "Bearer opaqueValue123",
            "prompt": "hidden system prompt",
            "tool_output": "stdout with sk-ant-1234567890abcdefghijklmnopqrstuv",
            "benign": "kept"
        })
    }

    // Summary mode: secrets AND content-sensitive fields scrubbed.
    let sink = Arc::new(RecordingTraceSink::new());
    let collector =
        TraceCollector::new("run-dl-redact", RedactionMode::Summary, sink.clone(), None);
    collector.prepare().unwrap();
    collector
        .record(SOURCE_PROVIDER, TraceKind::DiagnosticLinked)
        .severity(Severity::Warning)
        .diagnostic_code(CODE_PROVIDER_RATE_LIMITED)
        .details(secret_details())
        .emit();

    let snapshot = sink.snapshot();
    let details = snapshot[0]
        .details
        .as_ref()
        .expect("diagnostic-linked details present");
    assert_eq!(details["api_key"], "[REDACTED]");
    assert_eq!(details["github_pat"], "[REDACTED]");
    assert_eq!(details["package_source"], "[REDACTED]");
    assert_eq!(details["authorization"], "[REDACTED]");
    assert_eq!(details["prompt"], "[REDACTED]");
    assert_eq!(details["tool_output"], "[REDACTED]");
    assert_eq!(details["benign"], "kept");

    // Verbose mode: structural content fields and secrets are scrubbed;
    // non-sensitive metadata is retained.
    let sink_v = Arc::new(RecordingTraceSink::new());
    let collector_v =
        TraceCollector::new("run-dl-vrb", RedactionMode::Verbose, sink_v.clone(), None);
    collector_v.prepare().unwrap();
    collector_v
        .record(SOURCE_PROVIDER, TraceKind::DiagnosticLinked)
        .severity(Severity::Warning)
        .diagnostic_code(CODE_PROVIDER_RATE_LIMITED)
        .details(secret_details())
        .emit();

    let snapshot_v = sink_v.snapshot();
    let details_v = snapshot_v[0]
        .details
        .as_ref()
        .expect("diagnostic-linked details present");
    assert_eq!(details_v["api_key"], "[REDACTED]");
    assert_eq!(details_v["github_pat"], "[REDACTED]");
    assert_eq!(details_v["package_source"], "[REDACTED]");
    assert_eq!(details_v["authorization"], "[REDACTED]");
    assert_eq!(details_v["prompt"], "[REDACTED]");
}

// ---------------------------------------------------------------------------
// Redaction modes
// ---------------------------------------------------------------------------

#[test]
fn summary_mode_redacts_secret_and_prompt() {
    let sink = Arc::new(RecordingTraceSink::new());
    let collector = TraceCollector::new("run-sum", RedactionMode::Summary, sink.clone(), None);
    collector.prepare().unwrap();
    collector
        .record(SOURCE_AGENT, TraceKind::TurnStarted)
        .details(serde_json::json!({
            "prompt": "the user's secret plan",
            "api_key": SCRUBBED_KEY,
            "model": "gpt-4o",
            // A benign key whose VALUE is an absolute path: Summary's
            // ABSOLUTE_PATH_RE branch must redact it, which is Summary-specific.
            "workspace": "/home/alice/project",
        }))
        .emit();

    let snapshot = sink.snapshot();
    let details = snapshot[0].details.as_ref().expect("details present");
    assert_eq!(
        details["prompt"], "[REDACTED]",
        "summary redacts prompt content"
    );
    assert_eq!(details["api_key"], "[REDACTED]", "summary redacts api_key");
    assert_eq!(
        details["workspace"], "[REDACTED]",
        "summary redacts absolute-path values"
    );
    assert_eq!(details["model"], "gpt-4o", "summary keeps benign fields");
}

#[test]
fn verbose_mode_redacts_structural_prompt_and_secret() {
    let sink = Arc::new(RecordingTraceSink::new());
    let collector = TraceCollector::new("run-vrb", RedactionMode::Verbose, sink.clone(), None);
    collector.prepare().unwrap();
    collector
        .record(SOURCE_AGENT, TraceKind::TurnStarted)
        .details(serde_json::json!({
            "prompt": "the user's secret plan",
            "api_key": SCRUBBED_KEY,
            "model": "gpt-4o",
            "workspace": "/home/alice/project",
        }))
        .emit();

    let snapshot = sink.snapshot();
    let details = snapshot[0].details.as_ref().expect("details present");
    assert_eq!(
        details["prompt"], "[REDACTED]",
        "verbose redacts structural prompt content"
    );
    assert_eq!(
        details["api_key"], "[REDACTED]",
        "verbose still redacts secrets"
    );
    assert_eq!(
        details["workspace"], "/home/alice/project",
        "verbose keeps absolute-path values (Summary-only redaction)"
    );
    assert_eq!(details["model"], "gpt-4o");
}

#[test]
fn phase7_verbose_trace_redacts_structural_tool_context() {
    let value = serde_json::json!({
        "command": "echo OPI_VERBOSE_COMMAND_SECRET",
        "cwd": r"C:\Users\Luiz\secret-worktree",
        "exit_code": 1,
        "safe_counter": 7,
    });
    let redacted =
        opi_agent::diagnostic::redact(&value, opi_agent::diagnostic::RedactionMode::Verbose);
    let text = serde_json::to_string(&redacted).unwrap();
    assert!(!text.contains("OPI_VERBOSE_COMMAND_SECRET"), "{text}");
    assert!(!text.contains("secret-worktree"), "{text}");
    assert_eq!(redacted["safe_counter"], 7);
}

#[test]
fn redaction_recurses_into_nested_structures() {
    // Summary and Verbose redaction both recurse into nested objects/arrays;
    // a bug that only scrubbed top-level keys would leak nested secrets.
    let nested = serde_json::json!({
        "outer": {
            "api_key": SCRUBBED_KEY,
            "prompt": "nested plan",
            "workspace": "/home/alice/project",
        },
        "items": [
            { "token": "eyJabcdefghij.aaa.aaa" },
        ],
    });

    let summary_sink = Arc::new(RecordingTraceSink::new());
    let summary = TraceCollector::new(
        "run-nest-sum",
        RedactionMode::Summary,
        summary_sink.clone(),
        None,
    );
    summary.prepare().unwrap();
    summary
        .record(SOURCE_AGENT, TraceKind::TurnStarted)
        .details(nested.clone())
        .emit();
    let summary_snap = summary_sink.snapshot();
    let sum = summary_snap[0].details.as_ref().unwrap();
    assert_eq!(
        sum["outer"]["api_key"], "[REDACTED]",
        "nested api_key (summary)"
    );
    assert_eq!(
        sum["outer"]["prompt"], "[REDACTED]",
        "nested prompt (summary)"
    );
    assert_eq!(
        sum["outer"]["workspace"], "[REDACTED]",
        "nested absolute path (summary)"
    );
    assert_eq!(
        sum["items"][0]["token"], "[REDACTED]",
        "array token (summary)"
    );

    let verbose_sink = Arc::new(RecordingTraceSink::new());
    let verbose = TraceCollector::new(
        "run-nest-vrb",
        RedactionMode::Verbose,
        verbose_sink.clone(),
        None,
    );
    verbose.prepare().unwrap();
    verbose
        .record(SOURCE_AGENT, TraceKind::TurnStarted)
        .details(nested)
        .emit();
    let verbose_snap = verbose_sink.snapshot();
    let vrb = verbose_snap[0].details.as_ref().unwrap();
    assert_eq!(
        vrb["outer"]["api_key"], "[REDACTED]",
        "nested api_key (verbose)"
    );
    assert_eq!(
        vrb["outer"]["prompt"], "[REDACTED]",
        "verbose redacts nested prompt content"
    );
    assert_eq!(
        vrb["outer"]["workspace"], "/home/alice/project",
        "verbose keeps nested absolute path"
    );
    assert_eq!(
        vrb["items"][0]["token"], "[REDACTED]",
        "array token (verbose)"
    );
}

#[test]
fn redaction_mode_default_is_summary() {
    // Safe by default: RedactionMode::default() must be Summary.
    assert_eq!(RedactionMode::default(), RedactionMode::Summary);
}

// ---------------------------------------------------------------------------
// Trace sink failure contract
// ---------------------------------------------------------------------------

/// A sink whose `prepare` always fails, to exercise fail-closed behavior.
struct FailingPrepareSink;

impl TraceSink for FailingPrepareSink {
    fn prepare(&self) -> Result<(), TraceError> {
        Err(TraceError::Prepare(std::io::Error::other(
            "permission denied",
        )))
    }
    fn write(&self, _record: &TraceRecord) -> Result<(), TraceError> {
        Ok(())
    }
    fn finish(&self) -> Result<(), TraceError> {
        Ok(())
    }
}

/// A sink whose `write` always fails and counts attempts, to exercise fail-open.
struct FailingWriteSink {
    writes: AtomicU64,
}

impl FailingWriteSink {
    fn new() -> Self {
        Self {
            writes: AtomicU64::new(0),
        }
    }
    fn write_attempts(&self) -> u64 {
        self.writes.load(Ordering::SeqCst)
    }
}

impl TraceSink for FailingWriteSink {
    fn prepare(&self) -> Result<(), TraceError> {
        Ok(())
    }
    fn write(&self, _record: &TraceRecord) -> Result<(), TraceError> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        Err(TraceError::Write(std::io::Error::other("disk full")))
    }
    fn finish(&self) -> Result<(), TraceError> {
        Ok(())
    }
}

/// A sink whose `finish` always fails and counts writes, to confirm finish
/// errors are swallowed (best-effort) and the collector still disables.
struct FailingFinishSink {
    writes: AtomicU64,
}

impl FailingFinishSink {
    fn new() -> Self {
        Self {
            writes: AtomicU64::new(0),
        }
    }
    fn write_attempts(&self) -> u64 {
        self.writes.load(Ordering::SeqCst)
    }
}

impl TraceSink for FailingFinishSink {
    fn prepare(&self) -> Result<(), TraceError> {
        Ok(())
    }
    fn write(&self, _record: &TraceRecord) -> Result<(), TraceError> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn finish(&self) -> Result<(), TraceError> {
        Err(TraceError::Finish(std::io::Error::other("flush failed")))
    }
}

#[test]
fn prepare_failure_aborts_before_run_fail_closed() {
    let collector = TraceCollector::new(
        "run-fc",
        RedactionMode::default(),
        Arc::new(FailingPrepareSink),
        None,
    );
    // Fail closed: a prepare error surfaces to the caller so the run aborts.
    let result = collector.prepare();
    assert!(
        result.is_err(),
        "prepare failure must be surfaced (fail closed), got {result:?}"
    );
}

#[test]
fn file_sink_prepare_fails_on_missing_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    // A path whose parent does not exist cannot be created by OpenOptions.
    let missing = dir.path().join("does_not_exist").join("trace.jsonl");
    let sink = FileTraceSink::new(&missing);
    assert!(
        sink.prepare().is_err(),
        "prepare must fail when the parent dir is missing"
    );
}

#[test]
fn write_failure_emits_diagnostic_and_disables_sink_fail_open() {
    let trace_sink = Arc::new(FailingWriteSink::new());
    let diag_sink = Arc::new(RecordingSink::new());
    let collector = TraceCollector::new(
        "run-fo",
        RedactionMode::default(),
        trace_sink.clone(),
        Some(diag_sink.clone()),
    );
    collector.prepare().unwrap();

    // First emit: write fails -> emit a diagnostic and disable the sink.
    collector.record(SOURCE_AGENT, TraceKind::RunStarted).emit();

    assert_eq!(
        trace_sink.write_attempts(),
        1,
        "the failing write was attempted exactly once"
    );
    let diags = diag_sink.snapshot();
    assert_eq!(diags.len(), 1, "exactly one diagnostic is emitted");
    let d = &diags[0];
    assert_eq!(d.code, CODE_TRACE_SINK_FAILED);
    assert_eq!(d.source, SOURCE_AGENT);
    assert_eq!(d.severity, Severity::Warning);
}

#[test]
fn fail_open_subsequent_emits_are_noops() {
    let trace_sink = Arc::new(FailingWriteSink::new());
    let diag_sink = Arc::new(RecordingSink::new());
    let collector = TraceCollector::new(
        "run-fo2",
        RedactionMode::default(),
        trace_sink.clone(),
        Some(diag_sink.clone()),
    );
    collector.prepare().unwrap();

    // First emit disables the sink; the second must be a no-op.
    collector.record(SOURCE_AGENT, TraceKind::RunStarted).emit();
    collector
        .record(SOURCE_PROVIDER, TraceKind::ProviderRequest)
        .emit();
    collector
        .record(SOURCE_TOOL, TraceKind::ToolCallStarted)
        .emit();

    assert_eq!(
        trace_sink.write_attempts(),
        1,
        "subsequent emits must not call write after the sink is disabled"
    );
    assert_eq!(
        diag_sink.snapshot().len(),
        1,
        "no second diagnostic after the sink is disabled"
    );
}

#[test]
fn file_sink_writes_versioned_jsonl_in_order() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path: PathBuf = dir.path().join("trace.jsonl");
    let sink = Arc::new(FileTraceSink::new(&path));
    let collector = TraceCollector::new("run-file", RedactionMode::default(), sink.clone(), None);
    collector.prepare().expect("prepare creates the file");
    collector.record(SOURCE_AGENT, TraceKind::RunStarted).emit();
    collector
        .record(SOURCE_PROVIDER, TraceKind::ProviderRequest)
        .turn("turn-1")
        .emit();
    collector
        .record(SOURCE_TOOL, TraceKind::ToolCallStarted)
        .turn("turn-1")
        .emit();
    collector.finish();

    let contents = fs::read_to_string(&path).expect("trace file written");
    let records: Vec<serde_json::Value> = contents
        .trim_end()
        .split('\n')
        .map(|line| serde_json::from_str(line).expect("each line is a JSON record"))
        .collect();
    assert_eq!(records.len(), 3, "three records written in order");
    // Every record carries the unstable schema version.
    for r in &records {
        assert_eq!(r["schema_version"], TRACE_SCHEMA_VERSION);
        assert_eq!(r["run_id"], "run-file");
    }
    assert_eq!(records[0]["sequence"], 0);
    assert_eq!(records[1]["sequence"], 1);
    assert_eq!(records[2]["sequence"], 2);
    assert_eq!(records[0]["kind"], "run_started");
    assert_eq!(records[1]["kind"], "provider_request");
    assert_eq!(records[2]["kind"], "tool_call_started");
}

#[test]
fn file_not_created_until_prepared_opt_in() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path: PathBuf = dir.path().join("not_yet.jsonl");
    let sink = FileTraceSink::new(&path);
    // Constructing the sink must not touch the filesystem.
    assert!(!path.exists(), "file must not exist until prepare runs");
    sink.prepare().expect("prepare creates the file");
    assert!(path.exists(), "file exists after prepare");
}

#[test]
fn fail_open_works_without_diagnostic_sink() {
    // No DiagnosticSink attached: a write failure must still disable the sink
    // and must not panic.
    let trace_sink = Arc::new(FailingWriteSink::new());
    let collector = TraceCollector::new(
        "run-fo-none",
        RedactionMode::default(),
        trace_sink.clone(),
        None,
    );
    collector.prepare().unwrap();
    collector.record(SOURCE_AGENT, TraceKind::RunStarted).emit();
    assert_eq!(trace_sink.write_attempts(), 1, "write attempted once");
    // Subsequent emit is a no-op (disabled), proving disable happened even with
    // no diagnostic sink to observe it.
    collector
        .record(SOURCE_PROVIDER, TraceKind::ProviderRequest)
        .emit();
    assert_eq!(
        trace_sink.write_attempts(),
        1,
        "disabled: no further writes without a diagnostic sink"
    );
}

#[test]
fn emit_after_finish_is_a_noop() {
    let trace_sink = Arc::new(RecordingTraceSink::new());
    let collector = TraceCollector::new(
        "run-fin",
        RedactionMode::default(),
        trace_sink.clone(),
        None,
    );
    collector.prepare().unwrap();
    collector.record(SOURCE_AGENT, TraceKind::RunStarted).emit();
    let before = trace_sink.len();
    collector.finish();
    // Emitting after finish() must not reach the sink.
    collector.record(SOURCE_AGENT, TraceKind::RunEnded).emit();
    assert_eq!(trace_sink.len(), before, "emit after finish is a no-op");
}

#[test]
fn finish_error_is_swallowed_and_disables() {
    // finish() is best-effort: a finish error must not propagate and the sink
    // is disabled so no further records are written.
    let trace_sink = Arc::new(FailingFinishSink::new());
    let collector = TraceCollector::new(
        "run-finerr",
        RedactionMode::default(),
        trace_sink.clone(),
        None,
    );
    collector.prepare().unwrap();
    collector.record(SOURCE_AGENT, TraceKind::RunStarted).emit();
    assert_eq!(trace_sink.write_attempts(), 1, "first emit written");
    // finish() returns () and swallows the Finish error.
    collector.finish();
    // After finish, emit is a no-op even though FailingFinishSink::write would
    // succeed — the unchanged counter proves disable.
    collector.record(SOURCE_AGENT, TraceKind::RunEnded).emit();
    assert_eq!(
        trace_sink.write_attempts(),
        1,
        "finish disables the collector even when finish() errored"
    );
}

#[test]
fn file_sink_fail_open_when_written_before_prepare() {
    // Misuse guard: writing through a FileTraceSink before prepare returns a
    // Write error, which the collector treats as fail-open (disable + diagnostic).
    let dir = tempfile::tempdir().expect("tempdir");
    let path: PathBuf = dir.path().join("unprepared.jsonl");
    let trace_sink = Arc::new(FileTraceSink::new(&path));
    let diag_sink = Arc::new(RecordingSink::new());
    let collector = TraceCollector::new(
        "run-noprep",
        RedactionMode::default(),
        trace_sink.clone(),
        Some(diag_sink.clone()),
    );

    // Deliberately no prepare(): the emit hits the not-prepared write guard.
    collector.record(SOURCE_AGENT, TraceKind::RunStarted).emit();

    let diags = diag_sink.snapshot();
    assert_eq!(
        diags.len(),
        1,
        "write-before-prepare emits trace_sink_failed"
    );
    assert_eq!(diags[0].code, CODE_TRACE_SINK_FAILED);
    assert!(!path.exists(), "no file created without prepare");
}

// ===========================================================================
// Phase 7 task 7.5 — agent_loop trace wiring
//
// Drives the real `agent_loop` with a MockProvider and a `TraceCollector`
// threaded through `AgentLoopContext.trace`, then asserts the run/turn/
// provider/tool/diagnostic-linked records the loop emits. Pins fail-open (a
// trace write error never aborts the run) and the untraced `None` path (no
// behavior change). These exercise the production call site
// `opi_agent::agent_loop trace sink integration`.
// ===========================================================================

mod wiring {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    use opi_agent::agent_loop;
    use opi_agent::diagnostic::code::*;
    use opi_agent::diagnostic::{RedactionMode, SOURCE_TOOL, Severity};
    use opi_agent::diagnostic_sink::RecordingSink;
    use opi_agent::event::{AgentEvent, AgentEventSink};
    use opi_agent::hooks::AgentHooks;
    use opi_agent::loop_types::{
        AgentLoopConfig, AgentLoopContext, InferenceConfig, ModelSelection, NextTurnState,
    };
    use opi_agent::message::AgentMessage;
    use opi_agent::tool::{ExecutionMode, Tool, ToolResult};
    use opi_agent::{
        DiagnosticSink, RecordingTraceSink, TraceCollector, TraceError, TraceKind, TraceRecord,
        TraceSink,
    };
    use opi_ai::message::{InputContent, Message, OutputContent, ToolDef, UserMessage};
    use opi_ai::provider::ProviderError;
    use opi_ai::retry::RetryConfig;
    use opi_ai::test_support::{self, MockProvider, MockResponse, single_route_collection};

    /// Hooks that forward LLM messages unchanged and otherwise do nothing.
    struct NoopHooks;
    impl AgentHooks for NoopHooks {
        fn convert_to_llm(
            &self,
            messages: &[AgentMessage],
        ) -> Result<Vec<Message>, opi_agent::loop_types::AgentError> {
            Ok(messages
                .iter()
                .filter_map(|m| {
                    if let AgentMessage::Llm(m) = m {
                        Some(m.clone())
                    } else {
                        None
                    }
                })
                .collect())
        }
    }

    fn user_msg(text: &str) -> AgentMessage {
        AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text { text: text.into() }],
            timestamp_ms: 0,
        }))
    }

    fn collector(
        trace_sink: Arc<dyn TraceSink>,
        diag_sink: Arc<RecordingSink>,
    ) -> Arc<TraceCollector> {
        let c = TraceCollector::new(
            "run-wiring",
            RedactionMode::default(),
            trace_sink,
            Some(diag_sink as Arc<dyn DiagnosticSink>),
        );
        c.prepare().expect("recording sink prepare is infallible");
        Arc::new(c)
    }

    fn ctx(
        provider: MockProvider,
        sink: Arc<RecordingSink>,
        trace: Option<Arc<TraceCollector>>,
        tools: Vec<Box<dyn Tool>>,
    ) -> AgentLoopContext {
        AgentLoopContext {
            collection: Arc::new(single_route_collection(Box::new(provider))),
            registry: crate::common::test_registry(tools),
            authorizer: Some(crate::common::permissive_authorizer()),
            evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
            state: NextTurnState::new(
                vec![user_msg("hello")],
                ModelSelection::parse_spec("mock:mock-model").unwrap(),
                InferenceConfig::default(),
            ),
            system: None,
            steering_queue: None,
            follow_up_queue: None,
            diagnostic_sink: Some(sink as Arc<dyn DiagnosticSink>),
            trace,
            session_id: None,
        }
    }

    fn config(retry: Option<RetryConfig>) -> AgentLoopConfig {
        AgentLoopConfig {
            max_turns: 10,
            retry,
        }
    }

    fn fast_retry() -> RetryConfig {
        RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 1,
            max_delay_ms: 10,
        }
    }

    fn null_event_sink() -> AgentEventSink {
        Box::new(|_: AgentEvent| {})
    }

    fn kinds_of(sink: &RecordingTraceSink) -> Vec<TraceKind> {
        sink.snapshot().iter().map(|r| r.kind).collect()
    }

    /// A minimal tool that always succeeds, used to exercise the
    /// ToolCallStarted/Completed records.
    struct EchoTool;
    impl Tool for EchoTool {
        fn definition(&self) -> ToolDef {
            ToolDef {
                name: "echo".into(),
                description: "echo the call".into(),
                input_schema: serde_json::json!({"type": "object"}),
            }
        }
        fn execute(
            &self,
            _call_id: &str,
            _arguments: serde_json::Value,
            _signal: tokio_util::sync::CancellationToken,
            _on_update: Option<opi_agent::tool::UpdateCallback>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<ToolResult, opi_agent::tool::ToolError>>
                    + Send,
            >,
        > {
            Box::pin(async move {
                Ok(ToolResult {
                    content: vec![OutputContent::Text { text: "ok".into() }],
                    details: None,
                    is_error: false,
                    terminate: false,
                    truncated: false,
                    diagnostics: vec![],
                })
            })
        }
        fn execution_mode(&self) -> ExecutionMode {
            ExecutionMode::Sequential
        }
    }

    /// A tool whose process completed but reported a semantic tool error.
    struct ErrorResultTool;
    impl Tool for ErrorResultTool {
        fn definition(&self) -> ToolDef {
            ToolDef {
                name: "soft_fail".into(),
                description: "returns an error result".into(),
                input_schema: serde_json::json!({"type": "object"}),
            }
        }
        fn execute(
            &self,
            _call_id: &str,
            _arguments: serde_json::Value,
            _signal: tokio_util::sync::CancellationToken,
            _on_update: Option<opi_agent::tool::UpdateCallback>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<ToolResult, opi_agent::tool::ToolError>>
                    + Send,
            >,
        > {
            Box::pin(async move {
                Ok(ToolResult {
                    content: vec![OutputContent::Text {
                        text: "tool reported failure".into(),
                    }],
                    details: None,
                    is_error: true,
                    terminate: false,
                    truncated: false,
                    diagnostics: vec![],
                })
            })
        }
        fn execution_mode(&self) -> ExecutionMode {
            ExecutionMode::Sequential
        }
    }

    /// A tool whose result carries a populated `ToolDiagnostic` (Phase 11.8 S1):
    /// the agent loop must lift the per-cause code + structured context into a
    /// Phase 7 Diagnostic + DiagnosticLinked trace, NOT collapse to the generic
    /// execution-failed fallback.
    struct StructuredErrorTool;
    impl Tool for StructuredErrorTool {
        fn definition(&self) -> ToolDef {
            ToolDef {
                name: "soft_fail".into(),
                description: "returns a structured error result".into(),
                input_schema: serde_json::json!({"type": "object"}),
            }
        }
        fn execute(
            &self,
            _call_id: &str,
            _arguments: serde_json::Value,
            _signal: tokio_util::sync::CancellationToken,
            _on_update: Option<opi_agent::tool::UpdateCallback>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<ToolResult, opi_agent::tool::ToolError>>
                    + Send,
            >,
        > {
            Box::pin(async move {
                Ok(ToolResult {
                    content: vec![OutputContent::Text {
                        text: "path not found".into(),
                    }],
                    details: None,
                    is_error: true,
                    terminate: false,
                    truncated: false,
                    diagnostics: vec![opi_agent::tool::ToolDiagnostic {
                        code: CODE_TOOL_PATH_NOT_FOUND.to_string(),
                        message: "path 'missing.txt' does not exist".into(),
                        context: serde_json::json!({
                            "user_path": "missing.txt",
                            "resolved_path": "/workspace/missing.txt",
                        }),
                    }],
                })
            })
        }
        fn execution_mode(&self) -> ExecutionMode {
            ExecutionMode::Sequential
        }
    }

    #[tokio::test]
    async fn phase7_run_emits_boundary_and_provider_records() {
        let provider = MockProvider::new("mock", vec![test_support::text_response("hi")]);
        let diag = Arc::new(RecordingSink::new());
        let trace_sink = Arc::new(RecordingTraceSink::new());
        let trace = collector(trace_sink.clone(), diag.clone());

        let result = agent_loop(
            ctx(provider, diag.clone(), Some(trace), vec![]),
            config(None),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(result.is_ok(), "{:?}", result.err());

        let kinds = kinds_of(&trace_sink);
        assert!(kinds.contains(&TraceKind::RunStarted), "missing RunStarted");
        assert!(
            kinds.contains(&TraceKind::TurnStarted),
            "missing TurnStarted"
        );
        assert!(
            kinds.contains(&TraceKind::ProviderRequest),
            "missing ProviderRequest"
        );
        assert!(
            kinds.contains(&TraceKind::ProviderStreamCompletion),
            "missing ProviderStreamCompletion"
        );
        assert!(kinds.contains(&TraceKind::TurnEnded), "missing TurnEnded");
        assert!(kinds.contains(&TraceKind::RunEnded), "missing RunEnded");

        // RunStarted must precede RunEnded (run boundary ordering).
        let seqs: Vec<TraceKind> = trace_sink.snapshot().iter().map(|r| r.kind).collect();
        let start = seqs
            .iter()
            .position(|k| *k == TraceKind::RunStarted)
            .unwrap();
        let end = seqs.iter().position(|k| *k == TraceKind::RunEnded).unwrap();
        assert!(start < end, "RunStarted must precede RunEnded");
    }

    #[tokio::test]
    async fn phase7_provider_failure_emits_provider_failure_and_diagnostic_linked() {
        // Non-retryable provider error: the loop classifies it (RequestFailed ->
        // error) and must emit a ProviderFailure record plus a DiagnosticLinked
        // mirror carrying the provider failure code.
        let provider = MockProvider::new_with_errors(
            "mock",
            vec![MockResponse::Error(ProviderError::RequestFailed(
                "boom".into(),
            ))],
        );
        let diag = Arc::new(RecordingSink::new());
        let trace_sink = Arc::new(RecordingTraceSink::new());
        let trace = collector(trace_sink.clone(), diag.clone());

        let result = agent_loop(
            ctx(provider, diag.clone(), Some(trace), vec![]),
            config(None),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(result.is_err(), "provider failure must propagate");

        let kinds = kinds_of(&trace_sink);
        assert!(
            kinds.contains(&TraceKind::ProviderFailure),
            "missing ProviderFailure"
        );
        // The classified diagnostic is mirrored as DiagnosticLinked.
        let snap = trace_sink.snapshot();
        let linked = snap
            .iter()
            .filter(|r| r.kind == TraceKind::DiagnosticLinked)
            .count();
        assert!(linked > 0, "expected at least one DiagnosticLinked");
        assert!(
            snap.iter().any(|r| {
                r.kind == TraceKind::DiagnosticLinked && r.severity == Some(Severity::Error)
            }),
            "provider failure diagnostic should be Error severity"
        );
    }

    #[tokio::test]
    async fn provider_failure_trace_may_leave_turn_open() {
        let provider = MockProvider::new_with_errors(
            "mock",
            vec![MockResponse::Error(ProviderError::RequestFailed(
                "boom".into(),
            ))],
        );
        let diag = Arc::new(RecordingSink::new());
        let trace_sink = Arc::new(RecordingTraceSink::new());
        let trace = collector(trace_sink.clone(), diag.clone());

        let result = agent_loop(
            ctx(provider, diag, Some(trace), vec![]),
            config(None),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;

        assert!(result.is_err(), "provider failure must propagate");
        let kinds = kinds_of(&trace_sink);
        assert!(kinds.contains(&TraceKind::TurnStarted));
        assert!(kinds.contains(&TraceKind::ProviderFailure));
        assert!(kinds.contains(&TraceKind::RunEnded));
        assert!(
            !kinds.contains(&TraceKind::TurnEnded),
            "provider failure exits mid-turn; trace consumers must tolerate an open turn"
        );
    }

    #[tokio::test]
    async fn phase7_retry_emits_provider_retry_and_diagnostic_linked() {
        // One retryable error then success: a ProviderRetry record and a
        // DiagnosticLinked mirror of the retry-attempt diagnostic, followed by a
        // successful stream completion. No ProviderFailure on the success path.
        let provider = MockProvider::new_with_errors(
            "mock",
            vec![
                MockResponse::Error(ProviderError::RateLimited {
                    retry_after_ms: Some(1),
                }),
                MockResponse::Events(test_support::text_response("ok")),
            ],
        );
        let diag = Arc::new(RecordingSink::new());
        let trace_sink = Arc::new(RecordingTraceSink::new());
        let trace = collector(trace_sink.clone(), diag.clone());

        let result = agent_loop(
            ctx(provider, diag.clone(), Some(trace), vec![]),
            config(Some(fast_retry())),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(result.is_ok(), "{:?}", result.err());

        let kinds = kinds_of(&trace_sink);
        assert!(
            kinds.contains(&TraceKind::ProviderRetry),
            "missing ProviderRetry"
        );
        assert!(
            kinds.contains(&TraceKind::ProviderStreamCompletion),
            "missing ProviderStreamCompletion after retry success"
        );
        assert!(
            !kinds.contains(&TraceKind::ProviderFailure),
            "successful retry must not emit ProviderFailure"
        );
        // The retry-attempt diagnostic is mirrored as DiagnosticLinked with the
        // provider retry-attempt code.
        let retry_linked = trace_sink.snapshot().iter().any(|r| {
            r.kind == TraceKind::DiagnosticLinked
                && r.diagnostic_code == Some(CODE_PROVIDER_RETRY_ATTEMPT)
        });
        assert!(retry_linked, "retry attempt diagnostic must be mirrored");
    }

    #[tokio::test]
    async fn phase7_partial_after_retry_emits_suppressed_not_exhausted_diagnostic_linked() {
        let mut partial_events = test_support::text_response("partial after retry");
        partial_events.pop();

        let provider = MockProvider::new_with_errors(
            "mock",
            vec![
                MockResponse::Error(ProviderError::RateLimited {
                    retry_after_ms: Some(1),
                }),
                MockResponse::EventsThenError(
                    partial_events,
                    ProviderError::RateLimited {
                        retry_after_ms: Some(1),
                    },
                ),
            ],
        );
        let diag = Arc::new(RecordingSink::new());
        let trace_sink = Arc::new(RecordingTraceSink::new());
        let trace = collector(trace_sink.clone(), diag.clone());

        let result = agent_loop(
            ctx(provider, diag.clone(), Some(trace), vec![]),
            config(Some(fast_retry())),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(
            result.is_err(),
            "partial stream error after retry should fail"
        );

        let kinds = kinds_of(&trace_sink);
        assert!(
            kinds.contains(&TraceKind::ProviderRetry),
            "first retry attempt should still emit ProviderRetry"
        );
        assert!(
            kinds.contains(&TraceKind::ProviderFailure),
            "partial stream error after retry should still end in ProviderFailure"
        );

        let suppressed = trace_sink.snapshot().iter().any(|r| {
            r.kind == TraceKind::DiagnosticLinked
                && r.diagnostic_code == Some(CODE_PROVIDER_RETRY_SUPPRESSED_AFTER_PARTIAL_OUTPUT)
        });
        assert!(
            suppressed,
            "partial-after-retry suppression diagnostic must be mirrored"
        );
        let exhausted = trace_sink.snapshot().iter().any(|r| {
            r.kind == TraceKind::DiagnosticLinked
                && r.diagnostic_code == Some(CODE_PROVIDER_RETRY_EXHAUSTED)
        });
        assert!(
            !exhausted,
            "partial-after-retry suppression must not mirror retry_exhausted"
        );
    }

    #[tokio::test]
    async fn phase7_tool_call_started_and_completed() {
        // Turn 0 emits a tool call to "echo" (registered); turn 1 emits text.
        let provider = MockProvider::new(
            "mock",
            vec![
                test_support::tool_call_response("tc-1", "echo", r#"{"arg":"x"}"#),
                test_support::text_response("done"),
            ],
        );
        let diag = Arc::new(RecordingSink::new());
        let trace_sink = Arc::new(RecordingTraceSink::new());
        let trace = collector(trace_sink.clone(), diag.clone());

        let result = agent_loop(
            ctx(
                provider,
                diag.clone(),
                Some(trace),
                vec![Box::new(EchoTool)],
            ),
            config(None),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(result.is_ok(), "{:?}", result.err());

        let kinds = kinds_of(&trace_sink);
        assert!(
            kinds.contains(&TraceKind::ToolCallStarted),
            "missing ToolCallStarted"
        );
        assert!(
            kinds.contains(&TraceKind::ToolCallCompleted),
            "missing ToolCallCompleted"
        );
    }

    #[tokio::test]
    async fn phase7_tool_error_result_emits_failed_trace_and_diagnostic() {
        // Phase 11.8 S1: a tool that returns is_error=true WITH a populated
        // ToolDiagnostic must have its per-cause code + structured context
        // lifted into a Phase 7 Diagnostic (mirrored as a DiagnosticLinked
        // trace), NOT collapsed to the generic execution-failed diagnostic.
        let provider = MockProvider::new(
            "mock",
            vec![
                test_support::tool_call_response("tc-1", "soft_fail", r#"{}"#),
                test_support::text_response("done"),
            ],
        );
        let diag = Arc::new(RecordingSink::new());
        let trace_sink = Arc::new(RecordingTraceSink::new());
        let trace = collector(trace_sink.clone(), diag.clone());

        let result = agent_loop(
            ctx(
                provider,
                diag.clone(),
                Some(trace),
                vec![Box::new(StructuredErrorTool)],
            ),
            config(None),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(result.is_ok(), "{:?}", result.err());

        let kinds = kinds_of(&trace_sink);
        assert!(kinds.contains(&TraceKind::ToolCallStarted));
        assert!(
            kinds.contains(&TraceKind::ToolCallFailed),
            "is_error=true tool result must be traced as failed: {kinds:?}"
        );
        assert!(
            !kinds.contains(&TraceKind::ToolCallCompleted),
            "is_error=true tool result must not be traced as completed: {kinds:?}"
        );

        // Sink: exactly ONE diagnostic for soft_fail, carrying the per-cause
        // code + structured context (not the generic collapse).
        let snap = diag.snapshot();
        let soft_fail: Vec<_> = snap
            .iter()
            .filter(|d| {
                d.details.as_ref().and_then(|v| v["tool_name"].as_str()) == Some("soft_fail")
            })
            .collect();
        assert_eq!(
            soft_fail.len(),
            1,
            "exactly one lifted diagnostic for soft_fail: {snap:?}"
        );
        assert_eq!(
            soft_fail[0].code, CODE_TOOL_PATH_NOT_FOUND,
            "per-cause code must surface, not the generic execution-failed"
        );
        assert_eq!(soft_fail[0].severity, Severity::Error);
        assert_eq!(soft_fail[0].source, SOURCE_TOOL);
        let details = soft_fail[0].details.as_ref().expect("context present");
        assert_eq!(details["tool_name"], "soft_fail");
        assert_eq!(details["user_path"], "missing.txt");
        // D1 de-dup invariant: no generic execution-failed for this tool.
        assert!(
            !snap.iter().any(|d| d.code == CODE_TOOL_EXECUTION_FAILED
                && d.details.as_ref().and_then(|v| v["tool_name"].as_str()) == Some("soft_fail")),
            "generic execution-failed must NOT be emitted when per-cause diagnostics are present"
        );

        // DiagnosticLinked trace carries the per-cause code.
        assert!(
            trace_sink.snapshot().iter().any(|r| {
                r.kind == TraceKind::DiagnosticLinked
                    && r.diagnostic_code == Some(CODE_TOOL_PATH_NOT_FOUND)
            }),
            "DiagnosticLinked trace must carry the per-cause code: {:?}",
            trace_sink.snapshot()
        );
    }

    #[tokio::test]
    async fn phase7_tool_error_generic_fallback_when_no_diagnostics() {
        // Phase 11.8 S1: a bare is_error result with NO ToolDiagnostic falls
        // back to the generic execution-failed diagnostic (preserves the
        // pre-11.8 contract; guards D1's de-duplication invariant).
        let provider = MockProvider::new(
            "mock",
            vec![
                test_support::tool_call_response("tc-1", "soft_fail", r#"{}"#),
                test_support::text_response("done"),
            ],
        );
        let diag = Arc::new(RecordingSink::new());
        let trace_sink = Arc::new(RecordingTraceSink::new());
        let trace = collector(trace_sink.clone(), diag.clone());

        let result = agent_loop(
            ctx(
                provider,
                diag.clone(),
                Some(trace),
                vec![Box::new(ErrorResultTool)],
            ),
            config(None),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(result.is_ok(), "{:?}", result.err());

        let snap = diag.snapshot();
        let soft_fail: Vec<_> = snap
            .iter()
            .filter(|d| {
                d.details.as_ref().and_then(|v| v["tool_name"].as_str()) == Some("soft_fail")
            })
            .collect();
        assert_eq!(
            soft_fail.len(),
            1,
            "exactly one fallback diagnostic: {snap:?}"
        );
        assert_eq!(soft_fail[0].code, CODE_TOOL_EXECUTION_FAILED);
        assert!(
            !snap.iter().any(|d| d.code == CODE_TOOL_PATH_NOT_FOUND),
            "no per-cause code synthesized from empty diagnostics"
        );
    }

    #[tokio::test]
    async fn phase7_tool_call_failed_for_unknown_tool() {
        // The model calls a tool that is not registered: the loop records an
        // unknown-tool diagnostic (mirrored as DiagnosticLinked) and a
        // ToolCallFailed record.
        let provider = MockProvider::new(
            "mock",
            vec![
                test_support::tool_call_response("tc-1", "missing", r#"{}"#),
                test_support::text_response("done"),
            ],
        );
        let diag = Arc::new(RecordingSink::new());
        let trace_sink = Arc::new(RecordingTraceSink::new());
        let trace = collector(trace_sink.clone(), diag.clone());

        let result = agent_loop(
            ctx(provider, diag.clone(), Some(trace), vec![]),
            config(None),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(result.is_ok(), "{:?}", result.err());

        let kinds = kinds_of(&trace_sink);
        assert!(
            kinds.contains(&TraceKind::ToolCallStarted),
            "ToolCallStarted"
        );
        assert!(kinds.contains(&TraceKind::ToolCallFailed), "ToolCallFailed");
        assert!(
            trace_sink.snapshot().iter().any(|r| {
                r.kind == TraceKind::DiagnosticLinked
                    && r.diagnostic_code == Some(CODE_TOOL_UNKNOWN)
            }),
            "unknown-tool diagnostic must be mirrored as DiagnosticLinked"
        );
    }

    #[tokio::test]
    async fn phase7_trace_none_runs_untraced_with_no_behavior_change() {
        // No collector: the loop runs normally and emits no trace records.
        let provider = MockProvider::new("mock", vec![test_support::text_response("hi")]);
        let diag = Arc::new(RecordingSink::new());
        let trace_sink = Arc::new(RecordingTraceSink::new());
        // trace_sink is intentionally NOT attached; nothing should reach it.
        let _ = trace_sink.clone();

        let result = agent_loop(
            ctx(provider, diag.clone(), None, vec![]),
            config(None),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(result.is_ok(), "{:?}", result.err());
        assert!(trace_sink.is_empty(), "untraced run must emit no records");
    }

    #[tokio::test]
    async fn phase7_trace_write_failure_is_fail_open() {
        // A trace sink whose write always fails: the run must still complete
        // (fail-open); tracing must never abort the agent loop.
        struct FailingWriteSink {
            writes: AtomicU64,
        }
        impl TraceSink for FailingWriteSink {
            fn prepare(&self) -> Result<(), TraceError> {
                Ok(())
            }
            fn write(&self, _record: &TraceRecord) -> Result<(), TraceError> {
                self.writes.fetch_add(1, Ordering::SeqCst);
                Err(TraceError::Write(std::io::Error::other("disk full")))
            }
            fn finish(&self) -> Result<(), TraceError> {
                Ok(())
            }
        }

        let provider = MockProvider::new("mock", vec![test_support::text_response("hi")]);
        let diag = Arc::new(RecordingSink::new());
        let failing = Arc::new(FailingWriteSink {
            writes: AtomicU64::new(0),
        });
        let c = TraceCollector::new(
            "run-fo-wiring",
            RedactionMode::default(),
            failing.clone() as Arc<dyn TraceSink>,
            Some(diag.clone() as Arc<dyn DiagnosticSink>),
        );
        c.prepare().unwrap();
        let trace = Arc::new(c);

        let result = agent_loop(
            ctx(provider, diag.clone(), Some(trace), vec![]),
            config(None),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(
            result.is_ok(),
            "a trace write failure must not abort the run, got {:?}",
            result.err()
        );
        // The first failing write disables the sink; the fail-open diagnostic
        // is observable on the diagnostic sink.
        let codes: Vec<&'static str> = diag.snapshot().iter().map(|d| d.code).collect();
        assert!(
            codes.contains(&CODE_TRACE_SINK_FAILED),
            "expected trace_sink_failed diagnostic, got {codes:?}"
        );
    }
}

// ===========================================================================
// Phase 8 task 8.6 — runtime-contract failure trace + DiagnosticLinked mirror.
//
// Drives the real `agent_loop` through each runtime-contract failure path
// (tool validation, tool execution error, hook deny, cancellation) and pins
// BOTH the structural TraceKind record the loop emits AND the DiagnosticLinked
// mirror (same diagnostic_code + severity) that `observe()` writes in
// lockstep. The diagnostic sink is also asserted so the in-process record is
// covered alongside the trace record.
//
// Capability-invalid is intentionally not exercised here: it is structurally
// identical to the already-tested provider-failure path and is pinned by the
// classification tests in `diagnostics_runtime.rs`.
// ===========================================================================

mod phase8_runtime_contract_failures {
    use std::sync::Arc;

    use futures_util::StreamExt;
    use opi_agent::agent_loop;
    use opi_agent::diagnostic::code::*;
    use opi_agent::diagnostic::{RedactionMode, Severity};
    use opi_agent::diagnostic_sink::RecordingSink;
    use opi_agent::event::{AgentEvent, AgentEventSink};
    use opi_agent::hooks::{AgentHooks, BeforeToolCallContext, BeforeToolCallResult};
    use opi_agent::loop_types::{
        AgentError, AgentLoopConfig, AgentLoopContext, InferenceConfig, ModelSelection,
        NextTurnState,
    };
    use opi_agent::message::AgentMessage;
    use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult};
    use opi_agent::{DiagnosticSink, RecordingTraceSink, TraceCollector, TraceKind};
    use opi_ai::message::{
        AssistantContent, AssistantMessage, InputContent, Message, OutputContent, ToolDef,
        UserMessage,
    };
    use opi_ai::provider::{EventStream, Provider, ProviderError, Request};
    use opi_ai::retry::RetryConfig;
    use opi_ai::stream::{AssistantStreamEvent, StopReason, Usage};
    use opi_ai::test_support::{self, MockProvider, single_route_collection};

    /// Hooks that forward LLM messages unchanged and otherwise do nothing.
    struct NoopHooks;
    impl AgentHooks for NoopHooks {
        fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
            Ok(messages
                .iter()
                .filter_map(|m| {
                    if let AgentMessage::Llm(m) = m {
                        Some(m.clone())
                    } else {
                        None
                    }
                })
                .collect())
        }
    }

    /// Hooks that deny a tool call to the named tool with the given reason.
    struct DenyHooks {
        denied_tool: String,
        reason: String,
    }
    impl AgentHooks for DenyHooks {
        fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
            Ok(messages
                .iter()
                .filter_map(|m| {
                    if let AgentMessage::Llm(m) = m {
                        Some(m.clone())
                    } else {
                        None
                    }
                })
                .collect())
        }
        fn before_tool_call(
            &self,
            ctx: BeforeToolCallContext,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = BeforeToolCallResult> + Send>>
        {
            let denied = self.denied_tool.clone();
            let reason = self.reason.clone();
            Box::pin(async move {
                if ctx.tool_name == denied {
                    BeforeToolCallResult::Deny { reason }
                } else {
                    BeforeToolCallResult::Continue
                }
            })
        }
    }

    fn user_msg(text: &str) -> AgentMessage {
        AgentMessage::Llm(Message::User(UserMessage {
            content: vec![InputContent::Text { text: text.into() }],
            timestamp_ms: 0,
        }))
    }

    fn collector(
        trace_sink: Arc<RecordingTraceSink>,
        diag_sink: Arc<RecordingSink>,
    ) -> Arc<TraceCollector> {
        let c = TraceCollector::new(
            "run-p8",
            RedactionMode::default(),
            trace_sink,
            Some(diag_sink as Arc<dyn DiagnosticSink>),
        );
        c.prepare().expect("recording sink prepare is infallible");
        Arc::new(c)
    }

    fn ctx(
        provider: impl Provider + 'static,
        sink: Arc<RecordingSink>,
        trace: Option<Arc<TraceCollector>>,
        tools: Vec<Box<dyn Tool>>,
    ) -> AgentLoopContext {
        let model_spec = format!("{}:mock-model", provider.id());
        AgentLoopContext {
            collection: Arc::new(single_route_collection(Box::new(provider))),
            registry: crate::common::test_registry(tools),
            authorizer: Some(crate::common::permissive_authorizer()),
            evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
            state: NextTurnState::new(
                vec![user_msg("hello")],
                ModelSelection::parse_spec(&model_spec).unwrap(),
                InferenceConfig::default(),
            ),
            system: None,
            steering_queue: None,
            follow_up_queue: None,
            diagnostic_sink: Some(sink as Arc<dyn DiagnosticSink>),
            trace,
            session_id: None,
        }
    }

    fn config() -> AgentLoopConfig {
        AgentLoopConfig {
            max_turns: 10,
            retry: Some(RetryConfig {
                max_attempts: 3,
                initial_delay_ms: 1,
                max_delay_ms: 10,
            }),
        }
    }

    fn null_event_sink() -> AgentEventSink {
        Box::new(|_: AgentEvent| {})
    }

    fn kinds_of(sink: &RecordingTraceSink) -> Vec<TraceKind> {
        sink.snapshot().iter().map(|r| r.kind).collect()
    }

    fn base_msg() -> AssistantMessage {
        AssistantMessage {
            content: vec![],
            api: opi_ai::ApiKind::Anthropic,
            provider: "hanging".into(),
            model: "mock".into(),
            response_model: None,
            response_id: None,
            usage: Usage::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp_ms: 0,
        }
    }

    struct HangingStreamProvider {
        entered_stream: Arc<tokio::sync::Notify>,
    }

    impl Provider for HangingStreamProvider {
        fn id(&self) -> &str {
            "hanging"
        }

        fn models(&self) -> &[opi_ai::provider::ModelInfo] {
            static MODELS: std::sync::OnceLock<Vec<opi_ai::provider::ModelInfo>> =
                std::sync::OnceLock::new();
            MODELS
                .get_or_init(|| {
                    vec![opi_ai::provider::ModelInfo::new(
                        "mock-model",
                        "Mock Model",
                        opi_ai::WireApi::OpenAiCompletions,
                        opi_ai::ModelCapabilities::new(100_000, 4_096),
                    )]
                })
                .as_slice()
        }

        fn stream(&self, _request: Request) -> EventStream {
            let mut partial = base_msg();
            partial.content.push(AssistantContent::Text {
                text: "partial".into(),
            });
            let events: Vec<Result<AssistantStreamEvent, ProviderError>> = vec![
                Ok(AssistantStreamEvent::Start {
                    partial: base_msg(),
                }),
                Ok(AssistantStreamEvent::TextDelta {
                    content_index: 0,
                    delta: "partial".into(),
                    partial,
                }),
            ];
            self.entered_stream.notify_one();
            Box::pin(
                futures_util::stream::iter(events).chain(futures_util::stream::pending::<
                    Result<AssistantStreamEvent, ProviderError>,
                >()),
            )
        }
    }

    /// A tool whose schema requires property "x"; an empty-args call fails
    /// validation before execute() runs.
    struct StrictSchemaTool;
    impl Tool for StrictSchemaTool {
        fn definition(&self) -> ToolDef {
            ToolDef {
                name: "strict".into(),
                description: "requires x".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": { "x": { "type": "string" } },
                    "required": ["x"],
                }),
            }
        }
        fn execute(
            &self,
            _call_id: &str,
            _arguments: serde_json::Value,
            _signal: tokio_util::sync::CancellationToken,
            _on_update: Option<opi_agent::tool::UpdateCallback>,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<ToolResult, ToolError>> + Send>,
        > {
            // Unreachable: validation rejects the empty-args call first.
            Box::pin(async move {
                Ok(ToolResult {
                    content: vec![OutputContent::Text { text: "ok".into() }],
                    details: None,
                    is_error: false,
                    terminate: false,
                    truncated: false,
                    diagnostics: vec![],
                })
            })
        }
        fn execution_mode(&self) -> ExecutionMode {
            ExecutionMode::Sequential
        }
    }

    struct PermissiveTool;
    impl Tool for PermissiveTool {
        fn definition(&self) -> ToolDef {
            ToolDef {
                name: "permissive".into(),
                description: "accepts any object".into(),
                input_schema: serde_json::json!({ "type": "object" }),
            }
        }
        fn execute(
            &self,
            _call_id: &str,
            _arguments: serde_json::Value,
            _signal: tokio_util::sync::CancellationToken,
            _on_update: Option<opi_agent::tool::UpdateCallback>,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<ToolResult, ToolError>> + Send>,
        > {
            Box::pin(async {
                Ok(ToolResult {
                    content: vec![OutputContent::Text {
                        text: "unexpected".into(),
                    }],
                    details: None,
                    is_error: false,
                    terminate: false,
                    truncated: false,
                    diagnostics: vec![],
                })
            })
        }

        fn execution_mode(&self) -> ExecutionMode {
            ExecutionMode::Sequential
        }
    }

    struct ParallelPermissiveTool;
    impl Tool for ParallelPermissiveTool {
        fn definition(&self) -> ToolDef {
            ToolDef {
                name: "permissive_parallel".into(),
                description: "accepts any object in parallel mode".into(),
                input_schema: serde_json::json!({ "type": "object" }),
            }
        }
        fn execute(
            &self,
            _call_id: &str,
            _arguments: serde_json::Value,
            _signal: tokio_util::sync::CancellationToken,
            _on_update: Option<opi_agent::tool::UpdateCallback>,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<ToolResult, ToolError>> + Send>,
        > {
            Box::pin(async {
                Ok(ToolResult {
                    content: vec![OutputContent::Text {
                        text: "unexpected".into(),
                    }],
                    details: None,
                    is_error: false,
                    terminate: false,
                    truncated: false,
                    diagnostics: vec![],
                })
            })
        }
    }

    /// A tool whose execute() always returns a Err(ToolError).
    struct FailingTool;
    impl Tool for FailingTool {
        fn definition(&self) -> ToolDef {
            ToolDef {
                name: "boom".into(),
                description: "always errors".into(),
                input_schema: serde_json::json!({"type": "object"}),
            }
        }
        fn execute(
            &self,
            _call_id: &str,
            _arguments: serde_json::Value,
            _signal: tokio_util::sync::CancellationToken,
            _on_update: Option<opi_agent::tool::UpdateCallback>,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<ToolResult, ToolError>> + Send>,
        > {
            Box::pin(async move { Err(ToolError::ExecutionFailed("boom".into())) })
        }
        fn execution_mode(&self) -> ExecutionMode {
            ExecutionMode::Sequential
        }
    }

    #[tokio::test]
    async fn phase8_runtime_contract_failure_trace_tool_validation() {
        // Tool "strict" requires property x; the model calls it with empty
        // args, so schema validation rejects the call before execute() runs.
        let provider = MockProvider::new(
            "mock",
            vec![
                test_support::tool_call_response("tc-1", "strict", r#"{}"#),
                test_support::text_response("done"),
            ],
        );
        let diag = Arc::new(RecordingSink::new());
        let trace_sink = Arc::new(RecordingTraceSink::new());
        let trace = collector(trace_sink.clone(), diag.clone());

        let result = agent_loop(
            ctx(
                provider,
                diag.clone(),
                Some(trace),
                vec![Box::new(StrictSchemaTool)],
            ),
            config(),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(result.is_ok(), "{:?}", result.err());

        // Structural trace: Started + Failed, never Completed.
        let kinds = kinds_of(&trace_sink);
        assert!(
            kinds.contains(&TraceKind::ToolCallStarted),
            "missing ToolCallStarted: {kinds:?}"
        );
        assert!(
            kinds.contains(&TraceKind::ToolCallFailed),
            "validation failure must be traced as ToolCallFailed: {kinds:?}"
        );
        assert!(
            !kinds.contains(&TraceKind::ToolCallCompleted),
            "validation failure must not be traced as completed: {kinds:?}"
        );

        // DiagnosticLinked mirror: validation-failed code, Error severity.
        let snap = trace_sink.snapshot();
        let linked = snap.iter().find(|r| {
            r.kind == TraceKind::DiagnosticLinked
                && r.diagnostic_code == Some(CODE_TOOL_VALIDATION_FAILED)
        });
        let linked = linked.expect(
            "validation failure must mirror a DiagnosticLinked with CODE_TOOL_VALIDATION_FAILED",
        );
        assert_eq!(
            linked.severity,
            Some(Severity::Error),
            "validation failure diagnostic must be Error severity"
        );

        // The in-process diagnostic sink carries the same code.
        assert!(
            diag.snapshot()
                .iter()
                .any(|d| d.code == CODE_TOOL_VALIDATION_FAILED),
            "diagnostic sink must carry CODE_TOOL_VALIDATION_FAILED"
        );
    }

    #[tokio::test]
    async fn phase8_runtime_contract_failure_trace_malformed_tool_arguments() {
        let provider = MockProvider::new(
            "mock",
            vec![
                test_support::tool_call_response("tc-1", "permissive", "{not-json"),
                test_support::text_response("done"),
            ],
        );
        let diag = Arc::new(RecordingSink::new());
        let trace_sink = Arc::new(RecordingTraceSink::new());
        let trace = collector(trace_sink.clone(), diag.clone());

        let result = agent_loop(
            ctx(
                provider,
                diag.clone(),
                Some(trace),
                vec![Box::new(PermissiveTool)],
            ),
            config(),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(result.is_ok(), "{:?}", result.err());

        let kinds = kinds_of(&trace_sink);
        assert!(kinds.contains(&TraceKind::ToolCallStarted));
        assert!(kinds.contains(&TraceKind::ToolCallFailed));
        assert!(!kinds.contains(&TraceKind::ToolCallCompleted));
        assert!(trace_sink.snapshot().iter().any(|r| {
            r.kind == TraceKind::DiagnosticLinked
                && r.diagnostic_code == Some(CODE_TOOL_VALIDATION_FAILED)
        }));
        assert!(
            diag.snapshot()
                .iter()
                .any(|d| d.code == CODE_TOOL_VALIDATION_FAILED)
        );
    }

    #[tokio::test]
    async fn phase8_runtime_contract_failure_trace_malformed_tool_arguments_parallel() {
        let provider = MockProvider::new(
            "mock",
            vec![
                test_support::tool_call_response("tc-1", "permissive_parallel", "{not-json"),
                test_support::text_response("done"),
            ],
        );
        let diag = Arc::new(RecordingSink::new());
        let trace_sink = Arc::new(RecordingTraceSink::new());
        let trace = collector(trace_sink.clone(), diag.clone());

        let result = agent_loop(
            ctx(
                provider,
                diag.clone(),
                Some(trace),
                vec![Box::new(ParallelPermissiveTool)],
            ),
            config(),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(result.is_ok(), "{:?}", result.err());

        let kinds = kinds_of(&trace_sink);
        assert!(kinds.contains(&TraceKind::ToolCallStarted));
        assert!(kinds.contains(&TraceKind::ToolCallFailed));
        assert!(!kinds.contains(&TraceKind::ToolCallCompleted));
        assert!(trace_sink.snapshot().iter().any(|r| {
            r.kind == TraceKind::DiagnosticLinked
                && r.diagnostic_code == Some(CODE_TOOL_VALIDATION_FAILED)
        }));
        assert!(
            diag.snapshot()
                .iter()
                .any(|d| d.code == CODE_TOOL_VALIDATION_FAILED)
        );
    }

    #[tokio::test]
    async fn phase8_runtime_contract_failure_trace_execute_error() {
        // Tool "boom" execute() returns Err(ToolError): the loop must trace
        // ToolCallFailed (NOT Cancelled, since the token is not set) and mirror
        // a CODE_TOOL_EXECUTION_FAILED diagnostic.
        let provider = MockProvider::new(
            "mock",
            vec![
                test_support::tool_call_response("tc-1", "boom", r#"{}"#),
                test_support::text_response("done"),
            ],
        );
        let diag = Arc::new(RecordingSink::new());
        let trace_sink = Arc::new(RecordingTraceSink::new());
        let trace = collector(trace_sink.clone(), diag.clone());

        let result = agent_loop(
            ctx(
                provider,
                diag.clone(),
                Some(trace),
                vec![Box::new(FailingTool)],
            ),
            config(),
            &NoopHooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(result.is_ok(), "{:?}", result.err());

        // Structural trace: Failed, never Cancelled (token is not set).
        let kinds = kinds_of(&trace_sink);
        assert!(
            kinds.contains(&TraceKind::ToolCallStarted),
            "missing ToolCallStarted: {kinds:?}"
        );
        assert!(
            kinds.contains(&TraceKind::ToolCallFailed),
            "execute error must be traced as ToolCallFailed: {kinds:?}"
        );
        assert!(
            !kinds.contains(&TraceKind::ToolCallCancelled),
            "execute error with an unset token must not be traced as cancelled: {kinds:?}"
        );

        // DiagnosticLinked mirror: execution-failed code, Error severity.
        let snap = trace_sink.snapshot();
        let linked = snap.iter().find(|r| {
            r.kind == TraceKind::DiagnosticLinked
                && r.diagnostic_code == Some(CODE_TOOL_EXECUTION_FAILED)
        });
        let linked = linked
            .expect("execute error must mirror a DiagnosticLinked with CODE_TOOL_EXECUTION_FAILED");
        assert_eq!(
            linked.severity,
            Some(Severity::Error),
            "execute error diagnostic must be Error severity"
        );

        // The in-process diagnostic sink carries the same code.
        assert!(
            diag.snapshot()
                .iter()
                .any(|d| d.code == CODE_TOOL_EXECUTION_FAILED),
            "diagnostic sink must carry CODE_TOOL_EXECUTION_FAILED"
        );
    }

    #[tokio::test]
    async fn phase8_runtime_contract_failure_trace_hook_deny() {
        // DenyHooks rejects the "denied" tool call. The production loop routes
        // the Deny path through the same execute-failure trace/diagnostic
        // pipeline as a failing tool, so the diagnostic code is
        // CODE_TOOL_EXECUTION_FAILED (message "tool call denied by hook"), NOT
        // CODE_HOOK_FAILED. This test pins that mapping exactly.
        let provider = MockProvider::new(
            "mock",
            vec![
                test_support::tool_call_response("tc-1", "denied", r#"{}"#),
                test_support::text_response("done"),
            ],
        );
        let diag = Arc::new(RecordingSink::new());
        let trace_sink = Arc::new(RecordingTraceSink::new());
        let trace = collector(trace_sink.clone(), diag.clone());

        // A registered tool named "denied" is required so the loop reaches the
        // before_tool_call hook (an unknown tool would short-circuit on
        // CODE_TOOL_UNKNOWN before the hook fires).
        struct PassiveTool;
        impl Tool for PassiveTool {
            fn definition(&self) -> ToolDef {
                ToolDef {
                    name: "denied".into(),
                    description: "passive".into(),
                    input_schema: serde_json::json!({"type": "object"}),
                }
            }
            fn execute(
                &self,
                _call_id: &str,
                _arguments: serde_json::Value,
                _signal: tokio_util::sync::CancellationToken,
                _on_update: Option<opi_agent::tool::UpdateCallback>,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<ToolResult, ToolError>> + Send>,
            > {
                // Unreachable: the deny hook fires before execute().
                Box::pin(async move {
                    Ok(ToolResult {
                        content: vec![OutputContent::Text { text: "ok".into() }],
                        details: None,
                        is_error: false,
                        terminate: false,
                        truncated: false,
                        diagnostics: vec![],
                    })
                })
            }
            fn execution_mode(&self) -> ExecutionMode {
                ExecutionMode::Sequential
            }
        }

        let hooks = DenyHooks {
            denied_tool: "denied".into(),
            reason: "blocked".into(),
        };

        let result = agent_loop(
            ctx(
                provider,
                diag.clone(),
                Some(trace),
                vec![Box::new(PassiveTool)],
            ),
            config(),
            &hooks,
            null_event_sink(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(result.is_ok(), "{:?}", result.err());

        // Structural trace: the Deny path emits ToolCallFailed.
        let kinds = kinds_of(&trace_sink);
        assert!(
            kinds.contains(&TraceKind::ToolCallStarted),
            "missing ToolCallStarted: {kinds:?}"
        );
        assert!(
            kinds.contains(&TraceKind::ToolCallFailed),
            "hook-deny must be traced as ToolCallFailed: {kinds:?}"
        );
        assert!(
            !kinds.contains(&TraceKind::ToolCallCompleted),
            "hook-deny must not be traced as completed: {kinds:?}"
        );

        // DiagnosticLinked mirror: the production loop emits
        // CODE_TOOL_EXECUTION_FAILED (NOT CODE_HOOK_FAILED) for the deny path,
        // with Error severity.
        let snap = trace_sink.snapshot();
        let linked = snap.iter().find(|r| {
            r.kind == TraceKind::DiagnosticLinked
                && r.diagnostic_code == Some(CODE_TOOL_EXECUTION_FAILED)
        });
        let linked = linked
            .expect("hook-deny must mirror a DiagnosticLinked with CODE_TOOL_EXECUTION_FAILED");
        assert_eq!(
            linked.severity,
            Some(Severity::Error),
            "hook-deny diagnostic must be Error severity"
        );

        // The in-process diagnostic sink carries the same code.
        assert!(
            diag.snapshot()
                .iter()
                .any(|d| d.code == CODE_TOOL_EXECUTION_FAILED),
            "diagnostic sink must carry CODE_TOOL_EXECUTION_FAILED for the deny path"
        );
    }

    #[tokio::test]
    async fn phase8_runtime_contract_failure_trace_cancellation() {
        // Pre-cancel the token: the loop's before-turn cancel guard fires
        // first, emits a RunStarted boundary, observes the cancellation
        // diagnostic (mirrored as DiagnosticLinked with Info severity), then
        // emits RunEnded and returns Err(AgentError::Cancelled).
        let provider = MockProvider::new("mock", vec![test_support::text_response("hi")]);
        let diag = Arc::new(RecordingSink::new());
        let trace_sink = Arc::new(RecordingTraceSink::new());
        let trace = collector(trace_sink.clone(), diag.clone());

        let cancel = tokio_util::sync::CancellationToken::new();
        cancel.cancel();

        let result = agent_loop(
            ctx(provider, diag.clone(), Some(trace), vec![]),
            config(),
            &NoopHooks,
            null_event_sink(),
            cancel,
        )
        .await;
        assert!(
            matches!(result, Err(AgentError::Cancelled)),
            "pre-cancelled loop must return Err(AgentError::Cancelled), got {result:?}"
        );

        // Run boundaries are still emitted around the cancellation.
        let kinds = kinds_of(&trace_sink);
        assert!(
            kinds.contains(&TraceKind::RunStarted),
            "missing RunStarted: {kinds:?}"
        );
        assert!(
            kinds.contains(&TraceKind::RunEnded),
            "missing RunEnded: {kinds:?}"
        );

        // DiagnosticLinked mirror: cancellation code, Info severity
        // (cancellation is harness/user-initiated, not a failure).
        let snap = trace_sink.snapshot();
        let linked = snap.iter().find(|r| {
            r.kind == TraceKind::DiagnosticLinked && r.diagnostic_code == Some(CODE_AGENT_CANCELLED)
        });
        let linked =
            linked.expect("cancellation must mirror a DiagnosticLinked with CODE_AGENT_CANCELLED");
        assert_eq!(
            linked.severity,
            Some(Severity::Info),
            "cancellation diagnostic must be Info severity"
        );

        // The in-process diagnostic sink carries the cancellation code.
        assert!(
            diag.snapshot()
                .iter()
                .any(|d| d.code == CODE_AGENT_CANCELLED),
            "diagnostic sink must carry CODE_AGENT_CANCELLED"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn phase8_provider_stream_cancel_trace_may_leave_turn_open() {
        let diag = Arc::new(RecordingSink::new());
        let trace_sink = Arc::new(RecordingTraceSink::new());
        let trace = collector(trace_sink.clone(), diag.clone());
        let cancel = tokio_util::sync::CancellationToken::new();
        let cancel_for_task = cancel.clone();
        let entered_stream = Arc::new(tokio::sync::Notify::new());
        let provider = HangingStreamProvider {
            entered_stream: entered_stream.clone(),
        };

        let handle = tokio::spawn(async move {
            agent_loop(
                ctx(provider, diag, Some(trace), vec![]),
                config(),
                &NoopHooks,
                null_event_sink(),
                cancel_for_task,
            )
            .await
        });

        entered_stream.notified().await;
        cancel.cancel();

        let result = handle.await.expect("agent_loop task panicked");
        assert!(matches!(result, Err(AgentError::Cancelled)));

        let kinds = kinds_of(&trace_sink);
        assert!(kinds.contains(&TraceKind::TurnStarted));
        assert!(kinds.contains(&TraceKind::RunEnded));
        assert!(
            !kinds.contains(&TraceKind::TurnEnded),
            "provider-stream cancellation exits mid-turn; trace consumers must tolerate an open turn"
        );
        assert!(trace_sink.snapshot().iter().any(|r| {
            r.kind == TraceKind::DiagnosticLinked && r.diagnostic_code == Some(CODE_AGENT_CANCELLED)
        }));
    }
}

// ---------------------------------------------------------------------------
// Phase 12 task 12.2 — provider error metadata redacted at the public boundary.
//
// `AutoRetryStart.error_message` and `AutoRetryEnd.final_error` carry a
// provider-error string into JSON/RPC/trace public surfaces. They must be
// scrubbed by `AgentEvent::redacted_for_public` (the gate every public emit
// passes through), so a secret embedded in the provider error cannot leak.
// ---------------------------------------------------------------------------

#[test]
fn auto_retry_provider_error_messages_are_redacted_for_public() {
    let start: AgentEvent = serde_json::from_value(serde_json::json!({
        "type": "AutoRetryStart",
        "attempt": 1,
        "max_attempts": 3,
        "delay_ms": 100,
        "error_message": format!("rate limited; rejected key {SCRUBBED_KEY}"),
    }))
    .expect("AutoRetryStart deserializes");
    match start.redacted_for_public() {
        AgentEvent::AutoRetryStart { error_message, .. } => {
            assert!(
                !error_message.contains(SCRUBBED_KEY),
                "AutoRetryStart error_message must be redacted for public emit: {error_message}"
            );
        }
        other => panic!("redacted_for_public changed variant: {other:?}"),
    }

    let end: AgentEvent = serde_json::from_value(serde_json::json!({
        "type": "AutoRetryEnd",
        "success": false,
        "attempt": 3,
        "final_error": format!("exhausted retries with key {SCRUBBED_KEY}"),
    }))
    .expect("AutoRetryEnd deserializes");
    match end.redacted_for_public() {
        AgentEvent::AutoRetryEnd {
            final_error: Some(msg),
            ..
        } => {
            assert!(
                !msg.contains(SCRUBBED_KEY),
                "AutoRetryEnd final_error must be redacted for public emit: {msg}"
            );
        }
        AgentEvent::AutoRetryEnd {
            final_error: None, ..
        } => panic!("final_error dropped during redaction"),
        other => panic!("redacted_for_public changed variant: {other:?}"),
    }
}

#[test]
fn compaction_and_session_persist_errors_are_redacted_for_public() {
    let secret = "sk-ant-1234567890abcdefghijklmnopqrstuv";
    let query_url = "https://api.example.test/v1?api_key=query-secret&token=query-token";

    let compaction = AgentEvent::CompactionEnd {
        reason: CompactionReason::Manual,
        result: None,
        aborted: true,
        error_message: Some(format!("failed with {secret} at {query_url}")),
    }
    .redacted_for_public();

    match compaction {
        AgentEvent::CompactionEnd {
            error_message: Some(message),
            ..
        } => {
            assert!(!message.contains(secret));
            assert!(!message.contains("query-secret"));
            assert!(!message.contains("query-token"));
            assert!(message.contains("[REDACTED]"));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let persist = AgentEvent::SessionPersistError {
        message: format!("persist failed with {secret} at {query_url}"),
    }
    .redacted_for_public();

    match persist {
        AgentEvent::SessionPersistError { message } => {
            assert!(!message.contains(secret));
            assert!(!message.contains("query-secret"));
            assert!(!message.contains("query-token"));
            assert!(message.contains("[REDACTED]"));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let query_only = AgentEvent::SessionPersistError {
        message: "persist failed at https://api.example.test/v1?api_key=only-query-secret"
            .to_owned(),
    }
    .redacted_for_public();

    match query_only {
        AgentEvent::SessionPersistError { message } => {
            assert!(
                !message.contains("only-query-secret"),
                "query-string api_key values must be redacted even when no key prefix is present: {message}"
            );
            assert!(message.contains("[REDACTED]"));
        }
        other => panic!("unexpected event: {other:?}"),
    }
}
