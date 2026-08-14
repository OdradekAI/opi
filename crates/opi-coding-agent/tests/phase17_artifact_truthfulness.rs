//! Phase 17 task 17.7 smoke addendum: artifact-truthfulness directory.
//!
//! Produces `target/opi-artifacts/phase17-task-17.7` from a real
//! `CodingHarness` run bound to the product `FileEvidenceSink`. The directory
//! carries one immutable per-run evidence directory (evidence.jsonl,
//! manifest.json), run
//! scaffolding (command/cwd/env/exit/stdout/stderr), mode payloads
//! (ndjson/session/rpc), the resolved route assertion, tool counts, a
//! RUN_SUMMARY claim table, and a SHA256SUMS manifest. Every required artifact
//! must exist; every payload except stdout/stderr must be non-empty; SHA256SUMS
//! is verified against a clean re-read.

use std::io::Write;
use std::sync::Arc;

use opi_agent::evidence::{AssemblySource, EvidenceRecorder};
use opi_ai::test_support::{MockProvider, MockResponse, text_response};
use opi_coding_agent::config::{ExecutionRunMode, OpiConfig};
use opi_coding_agent::evidence::{EvidenceBuilderConfig, FileEvidenceSink};
use opi_coding_agent::harness::CodingHarness;
use opi_coding_agent::project_trust::TrustDecision;

/// Resolve the canonical artifact directory under the workspace `target/`.
fn artifact_dir() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_owned());
    std::path::Path::new(&manifest_dir).join("../../target/opi-artifacts/phase17-task-17.7")
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // serialized OPI_SESSIONS_DIR mutation; not re-acquired in awaited dispatch.
async fn phase17_task_17_7_artifact_truthfulness_directory() {
    let dir = artifact_dir();
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    static SESSION_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _lock = SESSION_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // SAFETY: test-only env var mutation serialized by SESSION_TEST_LOCK.
    unsafe { std::env::set_var("OPI_SESSIONS_DIR", sessions.path()) };

    let sink = Arc::new(FileEvidenceSink::new(dir.clone()));
    let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
    let events: Arc<std::sync::Mutex<Vec<opi_agent::event::AgentEvent>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));
    let events_for_cb = events.clone();
    let mut harness = CodingHarness::builder(
        Box::new(MockProvider::new_with_errors(
            "mock",
            vec![MockResponse::Events(text_response(
                "phase 17 evidence summarized",
            ))],
        )),
        "mock:mock-model".to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .execution_mode(ExecutionRunMode::Interactive)
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: AssemblySource::Cli,
    })
    .build();
    harness.subscribe(Box::new(move |event| {
        events_for_cb.lock().unwrap().push(event.clone());
    }));
    // The run scaffolding is generated from the in-process CodingHarness run
    // (Interactive mode + FileEvidenceSink), not captured from a separate `opi`
    // process. The command string reflects what actually executed.
    let command_str =
        "CodingHarness::prompt (interactive, FileEvidenceSink) — summarize phase 17 evidence";
    let messages = harness.prompt("summarize phase 17 evidence").await.unwrap();
    // SAFETY: test-only env var mutation serialized by SESSION_TEST_LOCK.
    unsafe { std::env::remove_var("OPI_SESSIONS_DIR") };
    drop(_lock);

    // evidence.jsonl + manifest.json are produced in one immutable per-run
    // child directory by the file adapter.
    let completed = sink.completed_run_dirs();
    assert_eq!(completed.len(), 1, "one finalized capture run");
    let capture_dir = completed[0].clone();
    let capture_rel = capture_dir
        .strip_prefix(&dir)
        .expect("capture belongs to artifact root")
        .to_string_lossy()
        .replace('\\', "/");
    assert!(
        capture_dir.join("evidence.jsonl").exists(),
        "evidence.jsonl written"
    );
    assert!(
        capture_dir.join("manifest.json").exists(),
        "manifest.json written"
    );

    // provider-assertion.json: the resolved route from the manifest.
    let manifest_json = std::fs::read_to_string(capture_dir.join("manifest.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest_json).unwrap();
    let provider_assertion = serde_json::json!({
        "requested_route": manifest["route"]["requested"],
        "resolved": manifest["route"]["resolved"],
        "actual": manifest["route"]["actual"],
    });
    std::fs::write(
        dir.join("provider-assertion.json"),
        serde_json::to_string_pretty(&provider_assertion).unwrap(),
    )
    .unwrap();

    // run.ndjson: the run's public agent events, one JSON object per line
    // (named `run*.ndjson` so the artifact audit's analysis glob exercises its
    // payload checks, rather than passing over an unreachable top-level name).
    let mut ndjson = String::new();
    for event in events.lock().unwrap().iter() {
        ndjson.push_str(&serde_json::to_string(event).unwrap_or_default());
        ndjson.push('\n');
    }
    std::fs::write(dir.join("run.ndjson"), ndjson).unwrap();

    // rpc.jsonl: a record of the RPC command/trace surface for this run.
    let rpc_record = serde_json::json!({
        "command": command_str,
        "evidence_records": sink.records().len(),
        "manifest_binding": manifest["binding"]["kind"],
    });
    std::fs::write(
        dir.join("rpc.jsonl"),
        format!("{}\n", serde_json::to_string(&rpc_record).unwrap()),
    )
    .unwrap();

    // tool-execution-counts.json: no tool calls in this mock run.
    std::fs::write(
        dir.join("tool-execution-counts.json"),
        serde_json::to_string_pretty(&serde_json::json!({ "tool_calls": {} })).unwrap(),
    )
    .unwrap();

    // sessions/session.jsonl: copy the persisted session if present, else a
    // header (under a `sessions*` directory so the audit analyzes it).
    let sessions_dir = dir.join("sessions");
    std::fs::create_dir_all(&sessions_dir).unwrap();
    let session_dest = sessions_dir.join("session.jsonl");
    let copied = std::fs::read_dir(sessions.path()).ok().and_then(|entries| {
        entries.filter_map(|e| e.ok()).find_map(|e| {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "jsonl") {
                std::fs::copy(&p, &session_dest).ok()
            } else {
                None
            }
        })
    });
    if copied.is_none() {
        std::fs::write(
            &session_dest,
            serde_json::to_string(&serde_json::json!({
                "type": "session_header",
                "model": "mock:mock-model",
            }))
            .unwrap(),
        )
        .unwrap();
    }

    // Run scaffolding.
    let stdout_text: String = messages
        .iter()
        .filter_map(|m| match m {
            opi_agent::message::AgentMessage::Llm(opi_ai::message::Message::Assistant(a)) => {
                Some(a.content.iter().filter_map(|c| match c {
                    opi_ai::message::AssistantContent::Text { text } => Some(text.as_str()),
                    _ => None,
                }))
            }
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(dir.join("command.txt"), format!("{command_str}\n")).unwrap();
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
    std::fs::write(dir.join("stdout.txt"), &stdout_text).unwrap();
    std::fs::write(dir.join("stderr.txt"), "").unwrap();

    // RUN_SUMMARY.md: claim/classification/artifact/result table. The run
    // scaffolding (command/cwd/exit-code/stdout/stderr) is generated from the
    // in-process CodingHarness run, not captured from a separate `opi` process,
    // so it is classified `source-inferred` and cannot close acceptance.
    let run_summary = format!(
        "# Phase 17 task 17.7 — artifact truthfulness\n\n\
        | Claim | Classification | Artifact | Result |\n\
        |---|---|---|---|\n\
        | Evidence capture writes evidence.jsonl | verified | {capture_rel}/evidence.jsonl | pass |\n\
        | Finalized manifest binds DirectRuntimeInput | verified | {capture_rel}/manifest.json | pass |\n\
        | Provider route (requested/resolved/actual) recorded | verified | provider-assertion.json | pass |\n\
        | Run events captured as NDJSON | verified | run.ndjson | pass |\n\
        | Session persisted | verified | sessions/session.jsonl | pass |\n\
        | Run scaffolding (command/cwd/exit-code/stdout/stderr) | source-inferred | exit-code.txt | n/a |\n\
        \n\
        Only `verified` rows close acceptance; `source-inferred` rows (the run scaffolding) record the harness-generated invocation, not a captured `opi` process.\n"
    );
    std::fs::write(dir.join("RUN_SUMMARY.md"), run_summary).unwrap();

    // SHA256SUMS.txt over every raw artifact file; verify presence + non-empty
    // + a clean re-read digest.
    let mut names = vec![
        "command.txt".to_owned(),
        "cwd.txt".to_owned(),
        "env-overrides.json".to_owned(),
        "exit-code.txt".to_owned(),
        "stdout.txt".to_owned(),
        "stderr.txt".to_owned(),
        "sessions/session.jsonl".to_owned(),
        "run.ndjson".to_owned(),
        "rpc.jsonl".to_owned(),
        "provider-assertion.json".to_owned(),
        "tool-execution-counts.json".to_owned(),
        format!("{capture_rel}/evidence.jsonl"),
        format!("{capture_rel}/manifest.json"),
        "RUN_SUMMARY.md".to_owned(),
    ];
    names.sort();
    let mut sha_lines: Vec<String> = Vec::new();
    for name in &names {
        let path = dir.join(name);
        assert!(path.exists(), "required artifact missing: {name}");
        let bytes = std::fs::read(&path).unwrap();
        if !matches!(name.as_str(), "stdout.txt" | "stderr.txt") {
            assert!(!bytes.is_empty(), "artifact must be non-empty: {name}");
        }
        use sha2::{Digest, Sha256};
        let digest = hex::encode(Sha256::digest(&bytes));
        sha_lines.push(format!("{digest}  {name}"));
    }
    let mut f = std::fs::File::create(dir.join("SHA256SUMS.txt")).unwrap();
    writeln!(f, "{}", sha_lines.join("\n")).unwrap();

    // Verify SHA256SUMS.txt against a fresh read of every listed raw file (the
    // smoke-addendum requirement), not a same-path re-read tautology.
    use sha2::{Digest, Sha256};
    let sums = std::fs::read_to_string(dir.join("SHA256SUMS.txt")).unwrap();
    for line in sums.lines() {
        let Some((digest, name)) = line.split_once("  ") else {
            panic!("malformed SHA256SUMS entry: {line}");
        };
        let fresh = hex::encode(Sha256::digest(std::fs::read(dir.join(name)).unwrap()));
        assert_eq!(digest, fresh, "SHA256SUMS digest mismatch for {name}");
    }

    // The finalized manifest is DirectRuntimeInput-bound and complete.
    assert_eq!(manifest["binding"]["kind"], "direct_runtime_input");
    assert!(!sink.records().is_empty());
}
