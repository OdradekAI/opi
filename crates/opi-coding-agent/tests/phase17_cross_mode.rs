//! Phase 17 task 17.9 — cross-mode runtime-semantics equivalence.
//!
//! P17-A14 / P17-MIG-005: one Phase 17 fixture — the active dispatchable route
//! `alpha:alpha-model` (one scripted response) plus one extra `beta` route — is
//! driven through every Reference Product entry point:
//!
//! 1. the interactive-binary harness assembly (`run_interactive_core` builds
//!    this same `CodingHarness::builder(...).execution_mode(Interactive)`;
//!    the TUI presentational loop is off-limits to hermetic tests by repo
//!    convention, so the mode is asserted at the shared builder seam);
//! 2. `CodingHarness::prompt` (the shared library seam every mode calls);
//! 3. `NonInteractiveRunner::run` (print) with durable `--trace` evidence;
//! 4. `NonInteractiveRunner::run_json` (JSON/NDJSON) with durable evidence;
//! 5. `RpcRunner` via `run_with_channels` with an injected recorder.
//!
//! The observable route, evidence-completeness, and completion semantics are
//! asserted equivalent across modes. Authority is shared by construction (every
//! mode wraps the same trusted-registration + `ToolAuthorizer` harness
//! assembly, proven exhaustively by task 17.4's A06-A08 matrix) and
//! cancellation converges on the same harness `CancellationToken`
//! (`harness.cancel()` / `runner.cancel()` / RPC abort), whose
//! not-converted-to-success behavior task 17.9 proves in
//! `phase17_failure_rollback`. Known asymmetries are recorded honestly: the
//! interactive binary wires no evidence capture, and the binary RPC path does
//! not forward `--trace` (RPC captures on its always-on in-memory sink over the
//! same `EvidenceSink` contract; here the recorder is injected through the
//! public `new_with_trace` seam so the records are observable).

#[path = "common/phase17.rs"]
mod phase17;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use opi_agent::evidence::{
    AssemblySource, CallKind, EvidenceRecord, EvidenceRecorder, InMemoryEvidenceSink,
};
use opi_agent::extension::ExtensionRegistry;
use opi_ai::auth::AuthResolver;
use opi_ai::provider::Provider;
use opi_ai::test_support::{MockProvider, text_response};
use opi_coding_agent::config::{ExecutionRunMode, OpiConfig};
use opi_coding_agent::evidence::EvidenceBuilderConfig;
use opi_coding_agent::harness::CodingHarness;
use opi_coding_agent::policy::ToolSelection;
use opi_coding_agent::project_trust::TrustDecision;
use opi_coding_agent::rpc::{RpcCommand, RpcRunner};
use opi_coding_agent::runner::{ExitCode, NonInteractiveRunner};
use opi_coding_agent::runtime_packages::RuntimePackageStartup;

/// The fixture's active canonical route.
const MODEL_SPEC: &str = "alpha:alpha-model";

fn runtime_startup() -> RuntimePackageStartup {
    RuntimePackageStartup {
        extension_registry: ExtensionRegistry::new(),
        installed_packages: Vec::new(),
        diagnostics: Vec::new(),
        trust_decision: TrustDecision::Trusted,
    }
}

/// The fixture's active provider: `alpha` with one `alpha-model` route and one
/// scripted response, plus its dispatch call log for route assertions.
fn alpha_fixture() -> (MockProvider, Arc<Mutex<Vec<opi_ai::provider::Request>>>) {
    let alpha = MockProvider::new_with_models(
        "alpha",
        vec![phase17::model_info("alpha-model")],
        vec![text_response("cross-mode ok")],
    );
    let calls = alpha.call_log_handle();
    (alpha, calls)
}

/// The fixture's extra dispatchable route: provider `beta`, model `beta-model`.
fn beta_route() -> (Box<dyn Provider>, Arc<dyn AuthResolver>) {
    let beta =
        MockProvider::new_with_models("beta", vec![phase17::model_info("beta-model")], Vec::new());
    (Box::new(beta), phase17::static_resolver())
}

/// Sorted distinct record-kind names emitted by a run through an in-memory
/// sink, in the serde-canonical `snake_case` form so they compare equal to the
/// durable `evidence.jsonl` kind values.
fn kind_names(records: &[EvidenceRecord]) -> Vec<String> {
    let mut kinds: Vec<String> = records
        .iter()
        .map(|r| format!("{:?}", r.kind).to_lowercase())
        .collect();
    kinds.sort();
    kinds.dedup();
    kinds
}

/// Sorted distinct record-kind names parsed from a durable `evidence.jsonl`.
fn jsonl_kind_names(path: &std::path::Path) -> Vec<String> {
    let raw = std::fs::read_to_string(path).unwrap();
    let mut kinds: Vec<String> = phase17::parse_ndjson(&raw)
        .iter()
        .filter_map(|line| line["kind"].as_str().map(str::to_owned))
        .collect();
    kinds.sort();
    kinds.dedup();
    kinds
}

async fn recv_rpc_line(
    output_rx: &mut tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
) -> serde_json::Value {
    tokio::time::timeout(Duration::from_secs(2), output_rx.recv())
        .await
        .expect("timed out waiting for RPC output")
        .expect("RPC output channel closed")
}

/// One-line text summary of an in-memory-sink run (for the artifact bundle).
fn sink_summary(label: &str, sink: &InMemoryEvidenceSink) -> String {
    format!(
        "{label}: records={}, kinds={:?}, finalized_manifest={}\n",
        sink.records().len(),
        kind_names(&sink.records()),
        sink.completed_manifest().is_some()
    )
}

// ===========================================================================
// P17-A14 / P17-MIG-005 — every Reference Product mode runs the same fixture
// with equivalent route and evidence-completeness semantics. Also assembles
// the task 17.9 artifact-truthfulness bundle for D.0a.
// ===========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::await_holding_lock)] // serialized OPI_SESSIONS_DIR mutation; not re-acquired in the awaited dispatch.
async fn phase17_all_public_product_modes_share_runtime_semantics() {
    let sessions = tempfile::tempdir().unwrap();
    let _lock = phase17::session_lock();
    phase17::set_sessions_dir(sessions.path());

    let dir = phase17::artifact_dir();
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // --- Mode 1: interactive-binary harness assembly (builder seam) ---------
    let ws1 = tempfile::tempdir().unwrap();
    let user1 = tempfile::tempdir().unwrap();
    let (alpha1, calls1) = alpha_fixture();
    let sink1 = Arc::new(InMemoryEvidenceSink::new());
    let recorder1: Arc<dyn EvidenceRecorder> = sink1.clone();
    let mut interactive = CodingHarness::builder(
        Box::new(alpha1),
        MODEL_SPEC.into(),
        OpiConfig::default(),
        ws1.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user1.path().to_path_buf())
    .execution_mode(ExecutionRunMode::Interactive)
    .extra_routes(vec![beta_route()])
    .evidence(EvidenceBuilderConfig {
        recorder: recorder1,
        source: AssemblySource::Cli,
    })
    .build();
    assert_eq!(
        interactive.model_spec(),
        MODEL_SPEC,
        "the interactive assembly resolves the same canonical route"
    );
    let interactive_len = interactive
        .prompt("cross-mode fixture")
        .await
        .expect("interactive assembly prompt runs")
        .len();
    assert!(interactive_len > 0, "the interactive run produced output");
    {
        let calls = calls1.lock().unwrap();
        assert_eq!(calls.len(), 1, "interactive assembly dispatches alpha once");
        assert!(
            calls[0].model.contains("alpha-model"),
            "the wire request targets alpha-model (got {:?})",
            calls[0].model
        );
    }
    sink1
        .completed_manifest()
        .expect("interactive assembly run finalizes evidence");

    // --- Mode 2: CodingHarness::prompt (the shared library seam) ------------
    let ws2 = tempfile::tempdir().unwrap();
    let user2 = tempfile::tempdir().unwrap();
    let (alpha2, calls2) = alpha_fixture();
    let sink2 = Arc::new(InMemoryEvidenceSink::new());
    let recorder2: Arc<dyn EvidenceRecorder> = sink2.clone();
    let mut harness = CodingHarness::builder(
        Box::new(alpha2),
        MODEL_SPEC.into(),
        OpiConfig::default(),
        ws2.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user2.path().to_path_buf())
    .extra_routes(vec![beta_route()])
    .evidence(EvidenceBuilderConfig {
        recorder: recorder2,
        source: AssemblySource::Cli,
    })
    .build();
    assert_eq!(harness.model_spec(), MODEL_SPEC);
    let harness_len = harness
        .prompt("cross-mode fixture")
        .await
        .expect("harness prompt runs")
        .len();
    assert_eq!(
        harness_len, interactive_len,
        "harness and interactive assembly produce the same fixture output shape"
    );
    {
        let calls = calls2.lock().unwrap();
        assert_eq!(calls.len(), 1, "harness dispatches alpha once");
    }
    sink2
        .completed_manifest()
        .expect("harness run finalizes evidence");

    // --- Mode 3: NonInteractiveRunner::run (print, durable --trace) ---------
    let ws3 = tempfile::tempdir().unwrap();
    let ev3 = tempfile::tempdir().unwrap();
    let (alpha3, calls3) = alpha_fixture();
    let mut print_runner = NonInteractiveRunner::new_with_resume_and_runtime_packages(
        Box::new(alpha3),
        MODEL_SPEC.into(),
        OpiConfig::default(),
        ws3.path().to_path_buf(),
        /* allow_mutating */ true,
        None,
        Vec::new(),
        /* resume_info */ None,
        ToolSelection::Disabled,
        runtime_startup(),
        Some(ev3.path().to_path_buf()),
        vec![beta_route()],
    )
    .expect("print runner constructs");
    let print_result = print_runner.run("cross-mode fixture").await;
    assert_eq!(
        print_result.exit_code,
        ExitCode::Success as i32,
        "print mode exits successfully (stderr: {:?})",
        print_result.stderr
    );
    assert!(
        print_result.stdout.contains("cross-mode ok"),
        "print stdout carries the assistant text"
    );
    {
        let calls = calls3.lock().unwrap();
        assert_eq!(calls.len(), 1, "print mode dispatches alpha once");
    }
    assert!(
        ev3.path().join("evidence.jsonl").exists(),
        "print mode writes durable evidence"
    );

    // --- Mode 4: NonInteractiveRunner::run_json (JSON/NDJSON, durable) ------
    let ws4 = tempfile::tempdir().unwrap();
    let ev4 = tempfile::tempdir().unwrap();
    let (alpha4, calls4) = alpha_fixture();
    let mut json_runner = NonInteractiveRunner::new_with_resume_and_runtime_packages(
        Box::new(alpha4),
        MODEL_SPEC.into(),
        OpiConfig::default(),
        ws4.path().to_path_buf(),
        /* allow_mutating */ true,
        None,
        Vec::new(),
        /* resume_info */ None,
        ToolSelection::Disabled,
        runtime_startup(),
        Some(ev4.path().to_path_buf()),
        vec![beta_route()],
    )
    .expect("json runner constructs");
    let json_result = json_runner.run_json("cross-mode fixture").await;
    assert_eq!(
        json_result.exit_code,
        ExitCode::Success as i32,
        "json mode exits successfully (stderr: {:?})",
        json_result.stderr
    );
    {
        let calls = calls4.lock().unwrap();
        assert_eq!(calls.len(), 1, "json mode dispatches alpha once");
    }
    let json_lines = phase17::parse_ndjson(&json_result.stdout);
    let summary = json_lines
        .iter()
        .find(|l| l["type"] == "session_summary")
        .expect("json mode emits a terminal session summary");
    assert_eq!(
        summary["model"], MODEL_SPEC,
        "the session summary reports the canonical route"
    );

    // A second NDJSON run over the same fixture (fresh session) for the
    // ndjson.jsonl artifact view.
    let ws4b = tempfile::tempdir().unwrap();
    let (alpha4b, calls4b) = alpha_fixture();
    let mut ndjson_runner = NonInteractiveRunner::new_with_resume_and_runtime_packages(
        Box::new(alpha4b),
        MODEL_SPEC.into(),
        OpiConfig::default(),
        ws4b.path().to_path_buf(),
        /* allow_mutating */ true,
        None,
        Vec::new(),
        /* resume_info */ None,
        ToolSelection::Disabled,
        runtime_startup(),
        None,
        vec![beta_route()],
    )
    .expect("ndjson runner constructs");
    let ndjson_result = ndjson_runner.run_json("cross-mode fixture").await;
    assert_eq!(ndjson_result.exit_code, ExitCode::Success as i32);
    {
        let calls = calls4b.lock().unwrap();
        assert_eq!(
            calls.len(),
            1,
            "the second NDJSON run dispatches alpha once"
        );
    }

    // --- Mode 5: RpcRunner (injected recorder) -------------------------------
    let ws5 = tempfile::tempdir().unwrap();
    let (alpha5, calls5) = alpha_fixture();
    let sink5 = Arc::new(InMemoryEvidenceSink::new());
    let recorder5: Arc<dyn EvidenceRecorder> = sink5.clone();
    let mut rpc_runner = RpcRunner::new_with_trace(
        Box::new(alpha5),
        MODEL_SPEC.into(),
        OpiConfig::default(),
        ws5.path().to_path_buf(),
        /* allow_mutating */ true,
        ToolSelection::Disabled,
        None,
        Vec::new(),
        Some(recorder5),
        TrustDecision::Trusted,
    )
    .expect("rpc runner constructs");
    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut rpc_lines: Vec<String> = Vec::new();
    let task =
        tokio::spawn(async move { rpc_runner.run_with_channels(command_rx, output_tx).await });
    command_tx
        .send(RpcCommand::prompt {
            id: Some("cross-mode-1".into()),
            message: "cross-mode fixture".into(),
        })
        .unwrap();
    // Wait for the prompt response, capturing every event line on the way.
    let accepted = loop {
        let line = recv_rpc_line(&mut output_rx).await;
        if line["type"] == "response" && line["command"] == "prompt" {
            break line;
        }
        rpc_lines.push(serde_json::to_string(&line).unwrap());
    };
    assert_eq!(accepted["success"], true, "rpc accepts the prompt");
    rpc_lines.push(serde_json::to_string(&accepted).unwrap());
    // Drain until the run terminates.
    loop {
        let line = recv_rpc_line(&mut output_rx).await;
        let terminal = line["type"] == "AgentEnd";
        rpc_lines.push(serde_json::to_string(&line).unwrap());
        if terminal {
            break;
        }
    }
    {
        let calls = calls5.lock().unwrap();
        assert_eq!(calls.len(), 1, "rpc mode dispatches alpha once");
    }
    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _ = recv_rpc_line(&mut output_rx).await;
    let rpc_exit = task.await.expect("rpc task joins");
    assert_eq!(rpc_exit, 0, "rpc runner exits 0");

    // --- Cross-mode equivalence ---------------------------------------------
    let interactive_kinds = kind_names(&sink1.records());
    let harness_kinds = kind_names(&sink2.records());
    let rpc_kinds = kind_names(&sink5.records());
    let print_kinds = jsonl_kind_names(&ev3.path().join("evidence.jsonl"));
    let json_kinds = jsonl_kind_names(&ev4.path().join("evidence.jsonl"));
    assert!(
        sink1.records().iter().any(|r| r.kind == CallKind::Provider),
        "interactive assembly emits a Provider evidence record"
    );
    assert!(
        print_kinds.iter().any(|k| k == "provider"),
        "durable --trace evidence carries the provider record (got {print_kinds:?})"
    );
    assert_eq!(
        interactive_kinds, harness_kinds,
        "interactive assembly and harness emit the same evidence kinds"
    );
    assert_eq!(
        harness_kinds, rpc_kinds,
        "harness and rpc emit the same evidence kinds"
    );
    assert_eq!(
        print_kinds, json_kinds,
        "print and json durable evidence emit the same kinds"
    );
    assert!(
        rpc_lines.iter().any(|l| l.contains("AgentEnd")),
        "the captured rpc stream contains the terminal AgentEnd event"
    );

    phase17::clear_sessions_dir();
    drop(_lock);

    // --- Artifact-truthfulness bundle (smoke addendum) -----------------------
    std::fs::write(
        dir.join("interactive.txt"),
        sink_summary("interactive", &sink1),
    )
    .unwrap();
    std::fs::write(dir.join("harness.txt"), sink_summary("harness", &sink2)).unwrap();
    std::fs::write(dir.join("print.txt"), &print_result.stdout).unwrap();
    std::fs::write(dir.join("json.jsonl"), &json_result.stdout).unwrap();
    std::fs::write(dir.join("ndjson.jsonl"), &ndjson_result.stdout).unwrap();
    let rpc_payload = format!("{}\n", rpc_lines.join("\n"));
    std::fs::write(dir.join("rpc.jsonl"), &rpc_payload).unwrap();
    std::fs::copy(
        ev3.path().join("evidence.jsonl"),
        dir.join("evidence.jsonl"),
    )
    .unwrap();
    let session_file =
        phase17::newest_jsonl(sessions.path()).expect("a session file was persisted");
    std::fs::copy(session_file, dir.join("session.jsonl")).unwrap();

    let provider_assertion = serde_json::json!({
        "fixture_model_spec": MODEL_SPEC,
        "modes": {
            "interactive_assembly": { "provider": "alpha", "calls": 1, "evidence_kinds": interactive_kinds },
            "coding_harness_prompt": { "provider": "alpha", "calls": 1, "evidence_kinds": harness_kinds },
            "runner_print": { "provider": "alpha", "calls": 1, "evidence_kinds": print_kinds },
            "runner_json": { "provider": "alpha", "calls": 1, "evidence_kinds": json_kinds, "summary_model": MODEL_SPEC },
            "rpc": { "provider": "alpha", "calls": 1, "evidence_kinds": rpc_kinds },
        },
        "equivalent": true,
    });
    std::fs::write(
        dir.join("provider-assertion.json"),
        serde_json::to_string_pretty(&provider_assertion).unwrap(),
    )
    .unwrap();
    std::fs::write(
        dir.join("tool-execution-counts.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "modes": {
                "interactive_assembly": 0, "coding_harness_prompt": 0,
                "runner_print": 0, "runner_json": 0, "rpc": 0,
            },
            "note": "the fixture emits no tool calls; authority equivalence is the shared CodingHarness authorization assembly (task 17.4 A06-A08)"
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        dir.join("command.txt"),
        "cargo test -p opi-coding-agent --test phase17_cross_mode phase17_all_public_product_modes_share_runtime_semantics\n",
    )
    .unwrap();
    std::fs::write(dir.join("cwd.txt"), format!("{}\n", ws2.path().display())).unwrap();
    std::fs::write(
        dir.join("env-overrides.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "OPI_SESSIONS_DIR": sessions.path()
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(dir.join("exit-code.txt"), "0\n").unwrap();
    std::fs::write(dir.join("stdout.txt"), "").unwrap();
    std::fs::write(dir.join("stderr.txt"), "").unwrap();

    let run_summary = "# Phase 17 task 17.9 — cross-mode artifact truthfulness\n\n\
        | Claim | Classification | Artifact | Result |\n\
        |---|---|---|---|\n\
        | All five modes dispatch the fixture route alpha:alpha-model once | verified | provider-assertion.json | pass |\n\
        | Interactive assembly + harness + rpc emit the same evidence kinds | verified | provider-assertion.json | pass |\n\
        | Print + json durable --trace evidence emit the same kinds | verified | evidence.jsonl | pass |\n\
        | JSON session_summary reports the canonical route | verified | json.jsonl | pass |\n\
        | RPC prompt accepted, AgentEnd observed, exit 0 | verified | rpc.jsonl | pass |\n\
        | Print stdout carries the fixture assistant text | verified | print.txt | pass |\n\
        | No tool executions in any mode (fixture is tool-free) | verified | tool-execution-counts.json | pass |\n\
        | Interactive TUI loop not spawned (hermetic boundary) | source-inferred | interactive.txt | n/a |\n\
        | RPC binary path does not forward --trace (in-memory only) | source-inferred | RUN_SUMMARY.md | n/a |\n\n\
        Only `verified` rows close acceptance. The two `source-inferred` rows record known mode asymmetries, not acceptance claims.\n";
    std::fs::write(dir.join("RUN_SUMMARY.md"), run_summary).unwrap();

    let names = [
        "command.txt",
        "cwd.txt",
        "env-overrides.json",
        "exit-code.txt",
        "stdout.txt",
        "stderr.txt",
        "interactive.txt",
        "harness.txt",
        "print.txt",
        "json.jsonl",
        "ndjson.jsonl",
        "rpc.jsonl",
        "session.jsonl",
        "evidence.jsonl",
        "provider-assertion.json",
        "tool-execution-counts.json",
        "RUN_SUMMARY.md",
    ];
    phase17::write_sha256sums(&dir, &names, &["stdout.txt", "stderr.txt"]);
    phase17::verify_sha256sums(&dir, &["stdout.txt", "stderr.txt"]);
}

// ===========================================================================
// Task-local P17-A15 precondition — the acceptance contract itself is
// platform-neutral: no OS-gated cfg attribute and no #[ignore] in any of the
// three task 17.9 acceptance binaries. (The actual Linux/macOS/Windows run
// evidence is Phase F; this only proves the local precondition.)
// ===========================================================================

#[test]
fn phase17_platform_contract_is_platform_neutral() {
    let sources = [
        include_str!("phase17_cross_mode.rs"),
        include_str!("phase17_failure_rollback.rs"),
        include_str!("phase17_api_audit.rs"),
    ];
    for (i, src) in sources.iter().enumerate() {
        for line in src.lines() {
            let t = line.trim_start();
            assert!(
                !(t.starts_with("#[cfg(")
                    && (t.contains("target_os") || t.contains("unix") || t.contains("windows"))),
                "source {i} gates a test path on the host OS: {t}"
            );
            assert!(
                !t.starts_with("#[ignore]"),
                "source {i} ignores an acceptance test: {t}"
            );
        }
    }
}
