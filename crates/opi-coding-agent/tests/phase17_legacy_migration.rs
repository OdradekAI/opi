//! Phase 17 task 17.8 — legacy session route migration + opaque trace preservation.
//!
//! Drives the READ-side legacy-normalization contract (P17-MIG-001/002/004,
//! acceptance P17-A13) through the production `CodingHarness` resume/fork/branch
//! seams and the `FileEvidenceSink` file adapter:
//!
//! - a legacy bare `model_change` model normalizes against the dispatchable
//!   collection (proving exactly one route), never guessing the active provider;
//!   missing/ambiguous routes return typed remediation before any dispatch;
//! - load/normalize/resume/fork leave legacy session fixtures byte-identical;
//! - opaque legacy trace files coexist with new evidence, and new evidence never
//!   overwrites, rewrites, upgrades, down-converts, or deletes them.
//!
//! This owns only legacy read/normalization behavior; it does not touch the 17.5
//! canonical write schema (`ModelInputSource`/`append_model_change`).

use std::sync::Mutex;

use opi_agent::session::{
    LeafEntry, MessageEntry, ModelChangeEntry, SessionEntry, SessionHeader, SessionWriter,
};
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::test_support::{MockProvider, text_response};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::harness::CodingHarness;
use opi_coding_agent::project_trust::TrustDecision;

/// Serialize `OPI_SESSIONS_DIR` mutation across this test binary.
static SESSION_TEST_LOCK: Mutex<()> = Mutex::new(());

fn session_lock() -> std::sync::MutexGuard<'static, ()> {
    SESSION_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn set_sessions_dir(dir: &std::path::Path) {
    // SAFETY: test-only env var mutation serialized by SESSION_TEST_LOCK.
    unsafe {
        std::env::set_var("OPI_SESSIONS_DIR", dir);
    }
}

fn clear_sessions_dir() {
    // SAFETY: test-only env var mutation serialized by SESSION_TEST_LOCK.
    unsafe {
        std::env::remove_var("OPI_SESSIONS_DIR");
    }
}

fn static_resolver() -> Arc<dyn opi_ai::auth::AuthResolver> {
    use std::sync::Arc;
    Arc::new(opi_ai::auth::StaticAuthResolver::new(
        opi_ai::auth::AuthScheme::ApiKey,
        secrecy::SecretString::from("opi-test-auth"),
    ))
}

fn model_info(id: &str) -> opi_ai::provider::ModelInfo {
    opi_ai::provider::ModelInfo::new(
        id,
        id,
        opi_ai::WireApi::OpenAiCompletions,
        opi_ai::ModelCapabilities::new(100_000, 4_096),
    )
}

/// Write a legacy session whose active branch records a bare/canonical
/// `model_change` (input_source `None` = pre-17.5 legacy entry). The active
/// chain is `msg-1` (the `Leaf` points at it); the `model_change` is parented to
/// it so `reconstruct_context` surfaces `model` as the recorded route.
fn write_legacy_session(dir: &std::path::Path, session_id: &str, recorded_model: &str) {
    let path = dir.join(format!("{session_id}.jsonl"));
    let header = SessionHeader::new(
        session_id.into(),
        "2026-08-14T12:00:00Z".into(),
        "/repo".into(),
        None,
    );
    let mut writer = SessionWriter::create(&path, header).unwrap();
    let user = SessionEntry::Message(MessageEntry {
        id: "msg-1".into(),
        parent_id: None,
        timestamp: "2026-08-14T12:00:01Z".into(),
        message: Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "seed".into(),
            }],
            timestamp_ms: 0,
        }),
    });
    writer.append(&user).unwrap();
    writer
        .append(&SessionEntry::ModelChange(ModelChangeEntry {
            id: "model-1".into(),
            parent_id: Some("msg-1".into()),
            timestamp: "2026-08-14T12:00:02Z".into(),
            model: recorded_model.into(),
            input_source: None,
        }))
        .unwrap();
    writer
        .append(&SessionEntry::Leaf(LeafEntry {
            id: "leaf-1".into(),
            parent_id: Some("msg-1".into()),
            timestamp: "2026-08-14T12:00:03Z".into(),
            entry_id: "msg-1".into(),
        }))
        .unwrap();
}

use std::sync::Arc;

// ===========================================================================
// P17-MIG-002 — a legacy bare model normalizes only when one dispatchable
// route is provable. Unique case: the bare model belongs to a SECOND
// dispatchable provider, not the active one.
// ===========================================================================

#[test]
fn phase17_legacy_bare_model_normalizes_to_unique_dispatchable_route() {
    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());
    // Legacy fixture: bare `shared-model` (no provider prefix).
    write_legacy_session(sessions.path(), "s-unique", "shared-model");

    let workspace = tempfile::tempdir().unwrap();
    let alpha = MockProvider::new_with_models(
        "alpha",
        vec![model_info("alpha-model")],
        vec![text_response("ok")],
    );
    let beta = MockProvider::new_with_models(
        "beta",
        vec![model_info("shared-model")],
        vec![text_response("ok")],
    );
    let mut harness = CodingHarness::builder(
        Box::new(alpha),
        "alpha:alpha-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .extra_routes(vec![(Box::new(beta), static_resolver())])
    .build();

    harness
        .resume_session_id("s-unique")
        .expect("resume succeeds");

    assert_eq!(
        harness.model_spec(),
        "beta:shared-model",
        "legacy bare model normalizes to the unique dispatchable route, not the active provider"
    );

    clear_sessions_dir();
}

/// Shared builder over an active `alpha` provider plus `extra` dispatchable
/// routes, each carrying one model. The active model is `alpha:alpha-model`.
/// Neither `alpha` nor the extra routes are dispatched by these read-side resume
/// tests (resume is lookup-only), so their response scripts are unused.
fn build_multi_route_harness(
    workspace: &std::path::Path,
    extra: Vec<(String, String)>,
) -> CodingHarness {
    let alpha = MockProvider::new_with_models("alpha", vec![model_info("alpha-model")], Vec::new());
    let extra_routes: Vec<(
        Box<dyn opi_ai::provider::Provider>,
        Arc<dyn opi_ai::auth::AuthResolver>,
    )> = extra
        .into_iter()
        .map(|(pid, mid)| {
            let p = MockProvider::new_with_models(&pid, vec![model_info(&mid)], Vec::new());
            (
                Box::new(p) as Box<dyn opi_ai::provider::Provider>,
                static_resolver(),
            )
        })
        .collect();
    CodingHarness::builder(
        Box::new(alpha),
        "alpha:alpha-model".into(),
        OpiConfig::default(),
        workspace.to_path_buf(),
        TrustDecision::Trusted,
    )
    .extra_routes(extra_routes)
    .record_diagnostics(true)
    .build()
}

#[test]
fn phase17_legacy_bare_model_ambiguous_route_keeps_cli_and_emits_typed_remediation() {
    use opi_agent::diagnostic::code::CODE_SESSION_RESUME_ROUTE_AMBIGUOUS;

    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());
    write_legacy_session(sessions.path(), "s-ambig", "shared-model");

    let workspace = tempfile::tempdir().unwrap();
    let mut harness = build_multi_route_harness(
        workspace.path(),
        vec![
            ("beta".into(), "shared-model".into()),
            ("gamma".into(), "shared-model".into()),
        ],
    );
    harness
        .resume_session_id("s-ambig")
        .expect("resume succeeds");

    // Ambiguity is fail-closed: the CLI/config model is kept (never guessed).
    assert_eq!(
        harness.model_spec(),
        "alpha:alpha-model",
        "ambiguous bare route keeps the CLI/config model"
    );
    assert!(
        harness
            .recorded_diagnostics()
            .iter()
            .any(|d| d.code == CODE_SESSION_RESUME_ROUTE_AMBIGUOUS),
        "ambiguous route emits a typed remediation diagnostic"
    );
    clear_sessions_dir();
}

#[test]
fn phase17_legacy_bare_model_missing_route_keeps_cli_and_emits_typed_remediation() {
    use opi_agent::diagnostic::code::CODE_SESSION_RESUME_ROUTE_MISSING;

    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());
    write_legacy_session(sessions.path(), "s-missing", "ghost-model");

    let workspace = tempfile::tempdir().unwrap();
    let mut harness = build_multi_route_harness(
        workspace.path(),
        vec![("beta".into(), "other-model".into())],
    );
    harness
        .resume_session_id("s-missing")
        .expect("resume succeeds");

    assert_eq!(
        harness.model_spec(),
        "alpha:alpha-model",
        "missing bare route keeps the CLI/config model"
    );
    assert!(
        harness
            .recorded_diagnostics()
            .iter()
            .any(|d| d.code == CODE_SESSION_RESUME_ROUTE_MISSING),
        "missing route emits a typed remediation diagnostic"
    );
    clear_sessions_dir();
}

#[test]
fn phase17_legacy_exact_canonical_route_accepted_on_resume() {
    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());
    // Exact canonical route (provider:model) — accepted as-is.
    write_legacy_session(sessions.path(), "s-exact", "beta:shared-model");

    let workspace = tempfile::tempdir().unwrap();
    let mut harness = build_multi_route_harness(
        workspace.path(),
        vec![("beta".into(), "shared-model".into())],
    );
    harness
        .resume_session_id("s-exact")
        .expect("resume succeeds");

    assert_eq!(
        harness.model_spec(),
        "beta:shared-model",
        "an exact canonical recorded route is accepted unchanged"
    );
    clear_sessions_dir();
}

// ===========================================================================
// P17-MIG-001 — load/normalize/resume/fork leave legacy session fixtures
// byte-identical. Normalization is in-memory only; no read path rewrites the
// source session file.
// ===========================================================================

#[test]
fn phase17_legacy_session_fixture_byte_identical_after_resume_normalize_fork() {
    let _lock = session_lock();
    let sessions = tempfile::tempdir().unwrap();
    set_sessions_dir(sessions.path());
    // Legacy fixture: bare `shared-model` (no provider prefix).
    write_legacy_session(sessions.path(), "s-bytes", "shared-model");
    let fixture_path = sessions.path().join("s-bytes.jsonl");
    let before = std::fs::read(&fixture_path).unwrap();

    let workspace = tempfile::tempdir().unwrap();
    let beta = MockProvider::new_with_models("beta", vec![model_info("shared-model")], Vec::new());
    let mut harness = CodingHarness::builder(
        Box::new(MockProvider::new_with_models(
            "alpha",
            vec![model_info("alpha-model")],
            Vec::new(),
        )),
        "alpha:alpha-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .extra_routes(vec![(Box::new(beta), static_resolver())])
    .build();

    // Resume triggers collection-backed bare normalization (in-memory only).
    harness
        .resume_session_id("s-bytes")
        .expect("resume succeeds");
    assert_eq!(
        harness.model_spec(),
        "beta:shared-model",
        "the route is normalized in memory"
    );
    // Fork writes a NEW session file; the source legacy fixture must be untouched.
    harness.fork_current_session().expect("fork succeeds");

    let after = std::fs::read(&fixture_path).unwrap();
    assert_eq!(
        before, after,
        "legacy session fixture is byte-identical after load/normalize/resume/fork"
    );
    clear_sessions_dir();
}

// ===========================================================================
// P17-MIG-004 — opaque legacy trace files coexist with new evidence. New
// evidence never overwrites, rewrites, upgrades, down-converts, or deletes
// them: the file adapter treats --trace as a DIRECTORY and fails closed when
// the path is an existing legacy trace FILE, leaving it byte-identical.
// ===========================================================================

#[test]
fn phase17_legacy_trace_file_blocks_evidence_setup_and_stays_byte_identical() {
    use opi_agent::evidence::{
        AssemblySource, ContentDigest, EvidenceError, EvidenceSink, RuntimeInputBinding,
    };
    use opi_coding_agent::evidence::FileEvidenceSink;

    let dir = tempfile::tempdir().unwrap();
    // A legacy serialize-only trace FILE occupies the --trace path.
    let trace_path = dir.path().join("legacy-trace.jsonl");
    let legacy_bytes = b"{\"schema_version\":1,\"records\":[]}\n".to_vec();
    std::fs::write(&trace_path, &legacy_bytes).unwrap();

    let binding = RuntimeInputBinding::direct(ContentDigest::from_hex("abcd"), AssemblySource::Cli);
    let sink = FileEvidenceSink::new(trace_path.clone());
    let result = sink.setup(&binding);
    assert!(
        matches!(result, Err(EvidenceError::Setup { .. })),
        "evidence setup must fail closed when the --trace path is an existing legacy file (got {result:?})"
    );

    let after = std::fs::read(&trace_path).unwrap();
    assert_eq!(
        after, legacy_bytes,
        "the legacy trace file is byte-identical after the failed evidence setup"
    );
}

// ===========================================================================
// P17-A13 — legacy sessions load while opaque legacy trace files coexist with
// new evidence. All legacy bytes stay unchanged, route normalization is
// unique/fail-closed, and new evidence never overwrites the legacy trace path.
// Also assembles the task 17.8 artifact-truthfulness bundle for D.0a.
//
// Note: a `prompt` legitimately appends a new turn to the resumed session, so
// session byte-immutability is measured around `resume` (which never rewrites),
// while the trace is proved untouched across the whole run.
// ===========================================================================

/// Resolve the canonical artifact directory under the workspace `target/`.
fn artifact_dir() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_owned());
    std::path::Path::new(&manifest_dir).join("../../target/opi-artifacts/phase17-task-17.8")
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // serialized OPI_SESSIONS_DIR mutation; not re-acquired in the awaited dispatch.
async fn phase17_legacy_sessions_and_opaque_traces_are_byte_preserved() {
    use opi_agent::evidence::{AssemblySource, EvidenceRecorder};
    use opi_coding_agent::config::ExecutionRunMode;
    use opi_coding_agent::evidence::{EvidenceBuilderConfig, FileEvidenceSink};
    use sha2::{Digest, Sha256};

    let dir = artifact_dir();
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    let _lock = session_lock();
    set_sessions_dir(sessions.path());

    // Legacy session fixture: bare `shared-model` (no provider prefix).
    write_legacy_session(sessions.path(), "a13", "shared-model");
    let legacy_session_path = sessions.path().join("a13.jsonl");
    let session_before = std::fs::read(&legacy_session_path).unwrap();
    std::fs::write(dir.join("legacy-session.before.jsonl"), &session_before).unwrap();

    // Opaque legacy serialize-only trace FILE at its existing location.
    let legacy_trace_dir = tempfile::tempdir().unwrap();
    let legacy_trace_path = legacy_trace_dir.path().join("legacy-trace.jsonl");
    let trace_before = b"{\"schema_version\":1,\"event\":\"tool\",\"redacted\":true}\n".to_vec();
    std::fs::write(&legacy_trace_path, &trace_before).unwrap();
    std::fs::write(dir.join("legacy-trace.before.jsonl"), &trace_before).unwrap();

    // Collection: alpha (active, alpha-model) + beta (shared-model) so the bare
    // legacy model normalizes to exactly one dispatchable route (beta).
    let alpha = MockProvider::new_with_models("alpha", vec![model_info("alpha-model")], Vec::new());
    let beta = MockProvider::new_with_models(
        "beta",
        vec![model_info("shared-model")],
        vec![text_response("a13 evidence captured")],
    );
    let sink = Arc::new(FileEvidenceSink::new(dir.clone()));
    let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
    let mut harness = CodingHarness::builder(
        Box::new(alpha),
        "alpha:alpha-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .execution_mode(ExecutionRunMode::Interactive)
    .extra_routes(vec![(Box::new(beta), static_resolver())])
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: AssemblySource::Cli,
    })
    .build();

    // Resume normalizes the bare legacy model to the unique dispatchable route.
    harness.resume_session_id("a13").expect("resume succeeds");
    assert_eq!(
        harness.model_spec(),
        "beta:shared-model",
        "legacy bare model normalizes to the unique dispatchable route"
    );

    // load/normalize/resume MUST NOT rewrite the legacy session fixture.
    let session_after_resume = std::fs::read(&legacy_session_path).unwrap();
    assert_eq!(
        session_before, session_after_resume,
        "legacy session fixture is byte-identical after load/normalize/resume"
    );
    std::fs::write(
        dir.join("legacy-session.after.jsonl"),
        &session_after_resume,
    )
    .unwrap();

    // A prompt emits new evidence (evidence.jsonl + manifest.json) through the
    // real production capture path. It legitimately appends a turn to the
    // session; the opaque legacy trace is never touched.
    let _messages = harness.prompt("summarize").await.expect("prompt runs");
    clear_sessions_dir();
    drop(_lock);

    let trace_after = std::fs::read(&legacy_trace_path).unwrap();
    assert_eq!(
        trace_before, trace_after,
        "legacy trace file is byte-identical; new evidence never overwrote it"
    );
    std::fs::write(dir.join("legacy-trace.after.jsonl"), &trace_after).unwrap();

    // New-schema evidence exists at the evidence dir, distinct from the legacy
    // trace path.
    assert!(
        dir.join("evidence.jsonl").exists(),
        "new evidence.jsonl written"
    );
    assert!(
        dir.join("manifest.json").exists(),
        "new manifest.json written"
    );
    std::fs::copy(dir.join("evidence.jsonl"), dir.join("new-evidence.jsonl")).unwrap();
    assert!(
        !sink.records().is_empty(),
        "the run emitted evidence records"
    );

    // byte-digests.json: identical before/after SHA-256 + length for each pair.
    let sha = |b: &[u8]| hex::encode(Sha256::digest(b));
    let byte_digests = serde_json::json!({
        "legacy_session": {
            "before_sha256": sha(&session_before),
            "after_sha256": sha(&session_after_resume),
            "before_len": session_before.len(),
            "after_len": session_after_resume.len(),
            "identical": session_before == session_after_resume,
        },
        "legacy_trace": {
            "before_sha256": sha(&trace_before),
            "after_sha256": sha(&trace_after),
            "before_len": trace_before.len(),
            "after_len": trace_after.len(),
            "identical": trace_before == trace_after,
        },
    });
    std::fs::write(
        dir.join("byte-digests.json"),
        serde_json::to_string_pretty(&byte_digests).unwrap(),
    )
    .unwrap();

    // Run scaffolding (generated from the in-process CodingHarness run).
    std::fs::write(
        dir.join("command.txt"),
        "CodingHarness::prompt (interactive, FileEvidenceSink) — summarize\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("cwd.txt"),
        format!("{}\n", workspace.path().display()),
    )
    .unwrap();
    std::fs::write(
        dir.join("env-overrides.json"),
        serde_json::to_string_pretty(&serde_json::json!({ "OPI_SESSIONS_DIR": sessions.path() }))
            .unwrap(),
    )
    .unwrap();
    std::fs::write(dir.join("exit-code.txt"), "0\n").unwrap();
    std::fs::write(dir.join("stdout.txt"), "").unwrap();
    std::fs::write(dir.join("stderr.txt"), "").unwrap();

    // RUN_SUMMARY.md: claim/classification/artifact/result table. The run
    // scaffolding is generated from the in-process run, not a captured `opi`
    // process, so it is `source-inferred` and cannot close acceptance.
    let run_summary = "# Phase 17 task 17.8 — legacy migration artifact truthfulness\n\n\
        | Claim | Classification | Artifact | Result |\n\
        |---|---|---|---|\n\
        | Legacy session loads + bare route normalizes uniquely | verified | legacy-session.before.jsonl | pass |\n\
        | Legacy session byte-identical after load/normalize/resume | verified | legacy-session.after.jsonl | pass |\n\
        | Legacy trace file byte-identical (never overwritten) | verified | legacy-trace.after.jsonl | pass |\n\
        | New-schema evidence coexists at a distinct path | verified | new-evidence.jsonl | pass |\n\
        | Before/after byte digests prove immutability | verified | byte-digests.json | pass |\n\
        | Run scaffolding (command/cwd/exit-code/stdout/stderr) | source-inferred | exit-code.txt | n/a |\n\n\
        Only `verified` rows close acceptance; `source-inferred` rows record the harness-generated invocation, not a captured `opi` process.\n";
    std::fs::write(dir.join("RUN_SUMMARY.md"), run_summary).unwrap();

    // SHA256SUMS.txt over every raw artifact; required-presence + non-empty +
    // a clean re-read digest.
    let mut names = vec![
        "command.txt",
        "cwd.txt",
        "env-overrides.json",
        "exit-code.txt",
        "stdout.txt",
        "stderr.txt",
        "legacy-session.before.jsonl",
        "legacy-session.after.jsonl",
        "legacy-trace.before.jsonl",
        "legacy-trace.after.jsonl",
        "new-evidence.jsonl",
        "byte-digests.json",
        "RUN_SUMMARY.md",
    ];
    names.sort();
    let mut sha_lines: Vec<String> = Vec::new();
    for name in &names {
        let path = dir.join(name);
        assert!(path.exists(), "required artifact missing: {name}");
        let bytes = std::fs::read(&path).unwrap();
        if !matches!(*name, "stdout.txt" | "stderr.txt") {
            assert!(!bytes.is_empty(), "artifact must be non-empty: {name}");
        }
        sha_lines.push(format!("{}  {}", hex::encode(Sha256::digest(&bytes)), name));
    }
    use std::io::Write;
    let mut f = std::fs::File::create(dir.join("SHA256SUMS.txt")).unwrap();
    writeln!(f, "{}", sha_lines.join("\n")).unwrap();

    // Verify SHA256SUMS against a fresh re-read of every listed raw file.
    let sums = std::fs::read_to_string(dir.join("SHA256SUMS.txt")).unwrap();
    for line in sums.lines() {
        let Some((digest, name)) = line.split_once("  ") else {
            panic!("malformed SHA256SUMS entry: {line}");
        };
        let fresh = hex::encode(Sha256::digest(std::fs::read(dir.join(name)).unwrap()));
        assert_eq!(digest, fresh, "SHA256SUMS digest mismatch for {name}");
    }
}
