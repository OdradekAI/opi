//! Phase 17 task 17.9 — cross-mode runtime-semantics equivalence.
//!
//! P17-A14 / P17-MIG-005: one Phase 17 fixture — the active dispatchable route
//! `alpha:alpha-model` (one tool turn plus completion) plus one extra `beta`
//! route — is
//! driven through every Reference Product entry point:
//!
//! 1. the interactive-binary harness assembly through the production TUI loop
//!    and its race-safe headless driver;
//! 2. `CodingHarness::prompt` (the shared library seam every mode calls);
//! 3. `NonInteractiveRunner::run` (print) with durable `--trace` evidence;
//! 4. `NonInteractiveRunner::run_json` (JSON/NDJSON) with durable evidence;
//! 5. `RpcRunner` via `run_with_channels` with durable `--trace` evidence.
//!
//! The observable route, authority, cancellation, evidence completeness, and
//! completion semantics are exercised directly in every mode. The success
//! fixture asks for a real command tool whose adapter permission is not granted
//! and proves zero executions. A second pending-provider fixture is
//! cancelled only after canonical-route dispatch and proves the shared typed
//! cancellation lifecycle.

#[path = "common/phase17.rs"]
mod phase17;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opi_agent::event::AgentEvent;
use opi_agent::evidence::{
    CallKind, EvidenceCompleteness, EvidencePayload, EvidenceRecord, EvidenceRecorder,
    InMemoryEvidenceSink, ProviderInvocationFacts, ProviderNotApplicableReason, TerminalOutcome,
    ToolAuthorizationFacts,
};
use opi_agent::extension::{Extension, ExtensionError, ExtensionRegistry};
use opi_agent::loop_types::AgentError;
use opi_ai::auth::AuthResolver;
use opi_ai::provider::{EventStream, Provider, Request};
use opi_ai::test_support::{MockProvider, text_response, tool_call_response};
use opi_coding_agent::config::{
    ExecutionRunMode, ExecutionStrategy, OpiConfig, PermissionDecision,
};
use opi_coding_agent::credential_store::{FakeKeyringBackend, KeychainCredentialStore};
use opi_coding_agent::evidence::{EvidenceBuilderConfig, FileEvidenceSink};
use opi_coding_agent::execution::LOCAL_ADAPTER_ID;
use opi_coding_agent::harness::{CodingHarness, ResumeInfo};
use opi_coding_agent::interactive::{
    install_interactive_tui_test_driver, install_interactive_tui_test_driver_with_abort_readiness,
    run_interactive_tui,
};
use opi_coding_agent::policy::ToolSelection;
use opi_coding_agent::project_trust::TrustDecision;
use opi_coding_agent::rpc::{RpcCommand, RpcRunner};
use opi_coding_agent::runner::{ExitCode, NonInteractiveRunner};
use opi_coding_agent::runtime_packages::RuntimePackageStartup;
use opi_tui::Keybindings;

/// The fixture's active canonical route.
const MODEL_SPEC: &str = "alpha:alpha-model";
const AUTHORITY_TOOL: &str = "bash";
const AUTHORITY_COMMAND: &str = "echo unauthorized > authority-executed.txt";
static TUI_DRIVER_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn runtime_startup() -> RuntimePackageStartup {
    RuntimePackageStartup {
        extension_registry: ExtensionRegistry::new(),
        installed_packages: Vec::new(),
        diagnostics: Vec::new(),
        trust_decision: TrustDecision::Trusted,
    }
}

fn authority_config() -> OpiConfig {
    let mut config = OpiConfig::default();
    config.execution.strategy = ExecutionStrategy::Model;
    config
        .execution
        .permissions
        .insert(LOCAL_ADAPTER_ID.to_owned(), PermissionDecision::Ask);
    config
}

fn authority_selection() -> ToolSelection {
    ToolSelection::Allowlist(vec![AUTHORITY_TOOL.to_owned()])
}

fn assert_authority_probe_not_executed(workspace: &std::path::Path, label: &str) {
    assert!(
        !workspace.join("authority-executed.txt").exists(),
        "{label} must deny the command before tool execution"
    );
}

/// The fixture's active provider: `alpha` with one `alpha-model` route and one
/// scripted tool-denial turn and completion response, plus its dispatch call
/// log for route assertions.
fn alpha_fixture() -> (MockProvider, Arc<Mutex<Vec<opi_ai::provider::Request>>>) {
    let alpha = MockProvider::new_with_models(
        "alpha",
        vec![phase17::model_info("alpha-model")],
        vec![
            tool_call_response(
                "authority-call",
                AUTHORITY_TOOL,
                &serde_json::json!({
                    "command": AUTHORITY_COMMAND,
                    "backend": LOCAL_ADAPTER_ID
                })
                .to_string(),
            ),
            text_response("cross-mode authority denied"),
        ],
    );
    let calls = alpha.call_log_handle();
    (alpha, calls)
}

struct PendingProvider {
    calls: Arc<Mutex<Vec<Request>>>,
    dispatched: Arc<tokio::sync::Semaphore>,
}

impl Provider for PendingProvider {
    fn id(&self) -> &str {
        "alpha"
    }

    fn models(&self) -> &[opi_ai::provider::ModelInfo] {
        static MODELS: std::sync::OnceLock<Vec<opi_ai::provider::ModelInfo>> =
            std::sync::OnceLock::new();
        MODELS
            .get_or_init(|| vec![phase17::model_info("alpha-model")])
            .as_slice()
    }

    fn stream_prepared(&self, request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        self.calls.lock().unwrap().push(request);
        self.dispatched.add_permits(1);
        Box::pin(futures_util::stream::pending())
    }
}

fn cancellation_fixture() -> (
    PendingProvider,
    Arc<Mutex<Vec<Request>>>,
    Arc<tokio::sync::Semaphore>,
) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let dispatched = Arc::new(tokio::sync::Semaphore::new(0));
    (
        PendingProvider {
            calls: calls.clone(),
            dispatched: dispatched.clone(),
        },
        calls,
        dispatched,
    )
}

async fn wait_for_dispatch(dispatched: &tokio::sync::Semaphore) {
    let permit = tokio::time::timeout(Duration::from_secs(2), dispatched.acquire())
        .await
        .expect("provider dispatch timed out")
        .expect("dispatch semaphore remains open");
    permit.forget();
}

fn empty_resume_info(
    workspace: &std::path::Path,
    sessions: &std::path::Path,
    session_id: &str,
) -> ResumeInfo {
    let path = sessions.join(format!("{session_id}.jsonl"));
    opi_agent::session::SessionWriter::create(
        &path,
        opi_agent::session::SessionHeader::new(
            session_id.to_owned(),
            "2026-08-20T00:00:00Z".to_owned(),
            workspace.display().to_string(),
            None,
        ),
    )
    .unwrap();
    ResumeInfo {
        path,
        session_id: session_id.to_owned(),
        entries: Vec::new(),
        original_cwd: workspace.to_path_buf(),
        diagnostics: Vec::new(),
        recorded_model: None,
        recorded_thinking: None,
    }
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

fn finalized_run_dir(root: &std::path::Path) -> std::path::PathBuf {
    let mut candidates = std::fs::read_dir(root)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join("manifest.json").is_file())
        .collect::<Vec<_>>();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected one finalized run under {root:?}"
    );
    candidates.pop().unwrap()
}

fn assert_wire_requests_use_canonical_route(
    label: &str,
    calls: &Mutex<Vec<Request>>,
    count: usize,
) {
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), count, "{label} dispatch count");
    let models = calls
        .iter()
        .map(|request| request.model.as_str())
        .collect::<Vec<_>>();
    assert!(
        calls
            .iter()
            .all(|request| request.model.ends_with("alpha-model")),
        "{label} dispatches only the resolved alpha:alpha-model route: {models:?}"
    );
}

fn assert_typed_route(records: &[EvidenceRecord], label: &str) {
    let provider_records = records.iter().filter_map(|record| match &record.payload {
        EvidencePayload::Provider(facts) => Some(facts),
        _ => None,
    });
    let mut count = 0;
    for facts in provider_records {
        count += 1;
        assert_eq!(facts.route.requested().provider_id(), "alpha", "{label}");
        assert_eq!(facts.route.requested().model_id(), "alpha-model", "{label}");
        assert_eq!(facts.route.resolved().provider_id(), "alpha", "{label}");
        assert_eq!(facts.route.resolved().model_id(), "alpha-model", "{label}");
    }
    assert!(count > 0, "{label} emits typed provider route evidence");
}

fn assert_typed_denial(records: &[EvidenceRecord], label: &str) {
    assert!(
        records.iter().any(|record| matches!(
            &record.payload,
            EvidencePayload::Tool(facts)
                if matches!(facts.authorization_facts(), ToolAuthorizationFacts::Denied { .. })
        )),
        "{label} emits a typed authority denial: {records:?}"
    );
}

fn assert_in_memory_manifest(sink: &InMemoryEvidenceSink, expected: TerminalOutcome, label: &str) {
    let manifest = sink
        .completed_manifest()
        .unwrap_or_else(|| panic!("{label} finalizes evidence"));
    assert_eq!(manifest.facts().outcome, expected, "{label}");
    assert_eq!(
        manifest.facts().completeness,
        EvidenceCompleteness::Complete,
        "{label}"
    );
    let route = manifest
        .facts()
        .provider
        .route()
        .unwrap_or_else(|| panic!("{label} manifest retains provider route"));
    assert_eq!(route.requested().provider_id(), "alpha", "{label}");
    assert_eq!(route.requested().model_id(), "alpha-model", "{label}");
    assert_eq!(route.resolved().provider_id(), "alpha", "{label}");
    assert_eq!(route.resolved().model_id(), "alpha-model", "{label}");
}

fn durable_evidence(run_dir: &std::path::Path) -> Vec<serde_json::Value> {
    phase17::parse_ndjson(&std::fs::read_to_string(run_dir.join("evidence.jsonl")).unwrap())
}

fn assert_durable_manifest(run_dir: &std::path::Path, expected_outcome: &str, label: &str) {
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(run_dir.join("manifest.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["outcome"], expected_outcome, "{label}");
    assert_eq!(manifest["completeness"], "complete", "{label}");
    assert_eq!(
        manifest["provider"]["route"]["requested"]["provider_id"], "alpha",
        "{label}"
    );
    assert_eq!(
        manifest["provider"]["route"]["requested"]["model_id"], "alpha-model",
        "{label}"
    );
    assert_eq!(
        manifest["provider"]["route"]["resolved"]["provider_id"], "alpha",
        "{label}"
    );
    assert_eq!(
        manifest["provider"]["route"]["resolved"]["model_id"], "alpha-model",
        "{label}"
    );
}

fn assert_durable_denial(run_dir: &std::path::Path, label: &str) {
    assert!(
        durable_evidence(run_dir).iter().any(|record| {
            record["kind"] == "tool"
                && record["payload"]["Tool"]["authorization"]
                    .get("denied")
                    .is_some()
        }),
        "{label} emits a durable typed authority denial"
    );
}

async fn recv_rpc_line(
    output_rx: &mut tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
) -> serde_json::Value {
    tokio::time::timeout(Duration::from_secs(2), output_rx.recv())
        .await
        .expect("timed out waiting for RPC output")
        .expect("RPC output channel closed")
}

async fn join_task_with_timeout<T: Send + 'static>(
    task: tokio::task::JoinHandle<T>,
    label: &str,
) -> T {
    join_task_with_timeouts(
        task,
        Duration::from_secs(2),
        Duration::from_millis(100),
        label,
    )
    .await
    .unwrap_or_else(|error| panic!("{error}"))
}

async fn join_task_with_timeouts<T: Send + 'static>(
    mut task: tokio::task::JoinHandle<T>,
    timeout: Duration,
    cleanup_timeout: Duration,
    label: &str,
) -> Result<T, String> {
    match tokio::time::timeout(timeout, &mut task).await {
        Ok(joined) => joined.map_err(|error| format!("{label} task failed: {error}")),
        Err(_) => {
            task.abort();
            let cleanup = match tokio::time::timeout(cleanup_timeout, &mut task).await {
                Ok(Ok(_)) => "task completed while abort was being delivered".to_owned(),
                Ok(Err(error)) => format!("task abort join result: {error}"),
                Err(_) => {
                    format!("abort cleanup did not finish within {cleanup_timeout:?}")
                }
            };
            Err(format!(
                "{label} did not terminate within {timeout:?}; task aborted ({cleanup})"
            ))
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase17_join_guard_aborts_cooperative_pending_task() {
    struct DropSignal(Option<tokio::sync::oneshot::Sender<()>>);

    impl Drop for DropSignal {
        fn drop(&mut self) {
            if let Some(signal) = self.0.take() {
                let _ = signal.send(());
            }
        }
    }

    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (dropped_tx, dropped_rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let _drop_signal = DropSignal(Some(dropped_tx));
        let _ = started_tx.send(());
        std::future::pending::<i32>().await
    });
    tokio::time::timeout(Duration::from_millis(100), started_rx)
        .await
        .expect("pending canary starts")
        .expect("pending canary reports readiness");

    let error = tokio::time::timeout(
        Duration::from_millis(100),
        join_task_with_timeouts(
            task,
            Duration::from_millis(10),
            Duration::from_millis(10),
            "cross-mode join-guard canary",
        ),
    )
    .await
    .expect("post-abort cleanup has its own bound")
    .expect_err("a pending task must return a bounded diagnostic");
    assert!(error.contains("cross-mode join-guard canary"), "{error}");
    tokio::time::timeout(Duration::from_millis(100), dropped_rx)
        .await
        .expect("aborted pending task is dropped within the bound")
        .expect("aborted pending task reports that it is no longer live");
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
async fn phase17_all_public_product_modes_share_runtime_semantics() {
    let _tui_driver_lock = TUI_DRIVER_LOCK.lock().await;
    let sessions = tempfile::tempdir().unwrap();

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
        authority_config(),
        ws1.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user1.path().to_path_buf())
    .resume(empty_resume_info(
        ws1.path(),
        sessions.path(),
        "interactive-authority",
    ))
    .execution_mode(ExecutionRunMode::Interactive)
    .tool_selection(authority_selection())
    .extra_routes(vec![beta_route()])
    .evidence(EvidenceBuilderConfig {
        recorder: recorder1,
        source: opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
    })
    .build();
    assert_eq!(
        interactive.model_spec(),
        MODEL_SPEC,
        "the interactive assembly resolves the same canonical route"
    );
    interactive.credential_store = Some(Arc::new(KeychainCredentialStore::new(
        Box::new(FakeKeyringBackend::new()),
        user1.path().to_path_buf(),
    )));
    let interactive_driver =
        install_interactive_tui_test_driver(["cross-mode fixture", "<escape>", "exit"])
            .expect("interactive headless driver installs");
    tokio::time::timeout(
        Duration::from_secs(2),
        run_interactive_tui(
            interactive,
            MODEL_SPEC.to_owned(),
            "default",
            Keybindings::default(),
        ),
    )
    .await
    .expect("production TUI permission denial must terminate")
    .expect("interactive assembly runs through the production TUI loop");
    let interactive_capture = interactive_driver.capture();
    assert_eq!(interactive_capture.user_messages, 1);
    assert_eq!(interactive_capture.provider_calls, 2);
    assert_wire_requests_use_canonical_route("interactive assembly", &calls1, 2);
    assert_authority_probe_not_executed(ws1.path(), "interactive assembly");
    assert_typed_route(&sink1.records(), "interactive assembly");
    assert_typed_denial(&sink1.records(), "interactive assembly");
    assert_in_memory_manifest(&sink1, TerminalOutcome::Success, "interactive assembly");

    // --- Mode 2: CodingHarness::prompt (the shared library seam) ------------
    let ws2 = tempfile::tempdir().unwrap();
    let user2 = tempfile::tempdir().unwrap();
    let (alpha2, calls2) = alpha_fixture();
    let sink2 = Arc::new(InMemoryEvidenceSink::new());
    let recorder2: Arc<dyn EvidenceRecorder> = sink2.clone();
    let mut harness = CodingHarness::builder(
        Box::new(alpha2),
        MODEL_SPEC.into(),
        authority_config(),
        ws2.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user2.path().to_path_buf())
    .resume(empty_resume_info(
        ws2.path(),
        sessions.path(),
        "harness-authority",
    ))
    .execution_mode(ExecutionRunMode::NonInteractive)
    .tool_selection(authority_selection())
    .extra_routes(vec![beta_route()])
    .evidence(EvidenceBuilderConfig {
        recorder: recorder2,
        source: opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
    })
    .build();
    assert_eq!(harness.model_spec(), MODEL_SPEC);
    let harness_len = harness
        .prompt("cross-mode fixture")
        .await
        .expect("harness prompt runs")
        .len();
    assert!(harness_len > 0, "the harness run produced output");
    assert_wire_requests_use_canonical_route("CodingHarness", &calls2, 2);
    assert_authority_probe_not_executed(ws2.path(), "CodingHarness");
    assert_typed_route(&sink2.records(), "CodingHarness");
    assert_typed_denial(&sink2.records(), "CodingHarness");
    assert_in_memory_manifest(&sink2, TerminalOutcome::Success, "CodingHarness");

    // --- Mode 3: NonInteractiveRunner::run (print, durable --trace) ---------
    let ws3 = tempfile::tempdir().unwrap();
    let ev3 = tempfile::tempdir().unwrap();
    let (alpha3, calls3) = alpha_fixture();
    let mut print_runner = NonInteractiveRunner::new_with_resume_and_runtime_packages(
        Box::new(alpha3),
        MODEL_SPEC.into(),
        authority_config(),
        ws3.path().to_path_buf(),
        /* allow_mutating */ true,
        None,
        Vec::new(),
        /* resume_info */
        Some(empty_resume_info(
            ws3.path(),
            sessions.path(),
            "print-authority",
        )),
        authority_selection(),
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
        print_result.stdout.contains("cross-mode authority denied"),
        "print stdout carries the assistant text"
    );
    assert_wire_requests_use_canonical_route("print", &calls3, 2);
    assert_authority_probe_not_executed(ws3.path(), "print");
    let ev3_run = finalized_run_dir(ev3.path());
    assert!(ev3_run.join("evidence.jsonl").exists());
    assert_durable_manifest(&ev3_run, "success", "print");
    assert_durable_denial(&ev3_run, "print");

    // --- Mode 4: NonInteractiveRunner::run_json (JSON/NDJSON, durable) ------
    let ws4 = tempfile::tempdir().unwrap();
    let ev4 = tempfile::tempdir().unwrap();
    let (alpha4, calls4) = alpha_fixture();
    let mut json_runner = NonInteractiveRunner::new_with_resume_and_runtime_packages(
        Box::new(alpha4),
        MODEL_SPEC.into(),
        authority_config(),
        ws4.path().to_path_buf(),
        /* allow_mutating */ true,
        None,
        Vec::new(),
        /* resume_info */
        Some(empty_resume_info(
            ws4.path(),
            sessions.path(),
            "json-authority",
        )),
        authority_selection(),
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
    assert_wire_requests_use_canonical_route("JSON/NDJSON", &calls4, 2);
    assert_authority_probe_not_executed(ws4.path(), "JSON/NDJSON");
    let json_lines = phase17::parse_ndjson(&json_result.stdout);
    let summary = json_lines
        .iter()
        .find(|l| l["type"] == "session_summary")
        .expect("json mode emits a terminal session summary");
    assert_eq!(
        summary["model"], MODEL_SPEC,
        "the session summary reports the canonical route"
    );
    let ev4_run = finalized_run_dir(ev4.path());
    assert_durable_manifest(&ev4_run, "success", "JSON/NDJSON");
    assert_durable_denial(&ev4_run, "JSON/NDJSON");

    // A second NDJSON run over the same fixture (fresh session) for the
    // ndjson.jsonl artifact view.
    let ws4b = tempfile::tempdir().unwrap();
    let (alpha4b, calls4b) = alpha_fixture();
    let mut ndjson_runner = NonInteractiveRunner::new_with_resume_and_runtime_packages(
        Box::new(alpha4b),
        MODEL_SPEC.into(),
        authority_config(),
        ws4b.path().to_path_buf(),
        /* allow_mutating */ true,
        None,
        Vec::new(),
        /* resume_info */
        Some(empty_resume_info(
            ws4b.path(),
            sessions.path(),
            "ndjson-authority",
        )),
        authority_selection(),
        runtime_startup(),
        None,
        vec![beta_route()],
    )
    .expect("ndjson runner constructs");
    let ndjson_result = ndjson_runner.run_json("cross-mode fixture").await;
    assert_eq!(ndjson_result.exit_code, ExitCode::Success as i32);
    assert_wire_requests_use_canonical_route("second NDJSON", &calls4b, 2);
    assert_authority_probe_not_executed(ws4b.path(), "second NDJSON");

    // --- Mode 5: RpcRunner (durable --trace root) ----------------------------
    let ws5 = tempfile::tempdir().unwrap();
    let ev5 = tempfile::tempdir().unwrap();
    let (alpha5, calls5) = alpha_fixture();
    let mut rpc_runner = RpcRunner::new_with_runtime_packages_and_auth(
        Box::new(alpha5),
        MODEL_SPEC.into(),
        authority_config(),
        ws5.path().to_path_buf(),
        /* allow_mutating */ true,
        authority_selection(),
        None,
        Vec::new(),
        runtime_startup(),
        Some(empty_resume_info(
            ws5.path(),
            sessions.path(),
            "rpc-authority",
        )),
        Some(ev5.path().to_path_buf()),
        phase17::static_resolver(),
        vec![beta_route()],
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
    assert_wire_requests_use_canonical_route("RPC", &calls5, 2);
    assert_authority_probe_not_executed(ws5.path(), "RPC");
    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _ = recv_rpc_line(&mut output_rx).await;
    let rpc_exit = join_task_with_timeout(task, "RPC success run_loop").await;
    assert_eq!(rpc_exit, 0, "rpc runner exits 0");
    let ev5_run = finalized_run_dir(ev5.path());
    assert_durable_manifest(&ev5_run, "success", "RPC");
    assert_durable_denial(&ev5_run, "RPC");

    // --- Cross-mode equivalence ---------------------------------------------
    let interactive_kinds = kind_names(&sink1.records());
    let harness_kinds = kind_names(&sink2.records());
    let rpc_kinds = jsonl_kind_names(&ev5_run.join("evidence.jsonl"));
    let print_kinds = jsonl_kind_names(&ev3_run.join("evidence.jsonl"));
    let json_kinds = jsonl_kind_names(&ev4_run.join("evidence.jsonl"));
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
    std::fs::copy(ev3_run.join("evidence.jsonl"), dir.join("evidence.jsonl")).unwrap();
    let session_file = sessions.path().join("harness-authority.jsonl");
    assert!(
        session_file.is_file(),
        "the explicit harness session persists"
    );
    std::fs::copy(session_file, dir.join("session.jsonl")).unwrap();

    // Every numeric claim below is measured from the mode's own dispatch log
    // and workspace marker, not transcribed, so the `verified` rows in
    // RUN_SUMMARY.md stay truthful about what this execution observed.
    let provider_assertion = serde_json::json!({
        "fixture_model_spec": MODEL_SPEC,
        "modes": {
            "interactive_assembly": { "provider": "alpha", "calls": calls1.lock().unwrap().len(), "evidence_kinds": interactive_kinds },
            "coding_harness_prompt": { "provider": "alpha", "calls": calls2.lock().unwrap().len(), "evidence_kinds": harness_kinds },
            "runner_print": { "provider": "alpha", "calls": calls3.lock().unwrap().len(), "evidence_kinds": print_kinds },
            "runner_json": { "provider": "alpha", "calls": calls4.lock().unwrap().len(), "evidence_kinds": json_kinds, "summary_model": MODEL_SPEC },
            "rpc": { "provider": "alpha", "calls": calls5.lock().unwrap().len(), "evidence_kinds": rpc_kinds },
        },
        "equivalent": true,
    });
    std::fs::write(
        dir.join("provider-assertion.json"),
        serde_json::to_string_pretty(&provider_assertion).unwrap(),
    )
    .unwrap();
    let authority_probe_count = |workspace: &std::path::Path| {
        usize::from(workspace.join("authority-executed.txt").exists())
    };
    std::fs::write(
        dir.join("tool-execution-counts.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "modes": {
                "interactive_assembly": authority_probe_count(ws1.path()),
                "coding_harness_prompt": authority_probe_count(ws2.path()),
                "runner_print": authority_probe_count(ws3.path()),
                "runner_json": authority_probe_count(ws4.path()),
                "rpc": authority_probe_count(ws5.path()),
            },
            "note": "measured from each mode's authority-executed.txt marker; the real product authorizer denies the command before execution"
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
        serde_json::to_string_pretty(&serde_json::json!({})).unwrap(),
    )
    .unwrap();
    std::fs::write(dir.join("exit-code.txt"), format!("{rpc_exit}\n")).unwrap();
    std::fs::write(dir.join("stdout.txt"), "").unwrap();
    std::fs::write(dir.join("stderr.txt"), "").unwrap();

    let run_summary = "# Phase 17 task 17.9 — cross-mode artifact truthfulness\n\n\
        | Claim | Classification | Artifact | Result |\n\
        |---|---|---|---|\n\
        | All five modes dispatch both fixture turns through alpha:alpha-model | verified | provider-assertion.json | pass |\n\
        | Interactive assembly + harness + rpc emit the same evidence kinds | verified | provider-assertion.json | pass |\n\
        | Print + json durable --trace evidence emit the same kinds | verified | evidence.jsonl | pass |\n\
        | JSON session_summary reports the canonical route | verified | json.jsonl | pass |\n\
        | RPC prompt accepted, AgentEnd observed, exit 0 | verified | rpc.jsonl | pass |\n\
        | Print stdout carries the fixture assistant text | verified | print.txt | pass |\n\
        | A real tool request is denied with zero executions in every mode | verified | tool-execution-counts.json | pass |\n\
        | Interactive permission denial traverses the production TUI loop | verified | interactive.txt | pass |\n\
        | RPC durable --trace uses the same evidence lifecycle | verified | provider-assertion.json | pass |\n\n\
        Only `verified` rows close acceptance.\n";
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

async fn exercise_harness_cancellation(
    label: &str,
    explicit_interactive_mode: bool,
) -> Vec<String> {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    let (provider, calls, dispatched) = cancellation_fixture();
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
    let mut builder = CodingHarness::builder(
        Box::new(provider),
        MODEL_SPEC.to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .resume(empty_resume_info(
        workspace.path(),
        sessions.path(),
        &format!("{label}-cancel"),
    ))
    .tool_selection(ToolSelection::Disabled)
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
    });
    if explicit_interactive_mode {
        builder = builder.execution_mode(ExecutionRunMode::Interactive);
    }
    let mut harness = builder.build();
    let control = harness.control_handle();
    let task = tokio::spawn(async move {
        let result = harness.prompt("cancel after canonical dispatch").await;
        (harness, result)
    });

    wait_for_dispatch(&dispatched).await;
    control.abort();
    let (_harness, result) =
        join_task_with_timeout(task, &format!("{label} harness cancellation")).await;

    assert!(
        matches!(result, Err(AgentError::Cancelled)),
        "{label} retains the exact AgentError::Cancelled class: {result:?}"
    );
    assert_wire_requests_use_canonical_route(label, &calls, 1);
    assert_typed_route(&sink.records(), label);
    assert_in_memory_manifest(&sink, TerminalOutcome::Cancelled, label);
    kind_names(&sink.records())
}

async fn exercise_runner_cancellation(json: bool) -> Vec<String> {
    let label = if json { "JSON/NDJSON" } else { "print" };
    let workspace = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    let evidence = tempfile::tempdir().unwrap();
    let (provider, calls, dispatched) = cancellation_fixture();
    let mut runner = NonInteractiveRunner::new_with_resume_and_runtime_packages(
        Box::new(provider),
        MODEL_SPEC.to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        None,
        Vec::new(),
        Some(empty_resume_info(
            workspace.path(),
            sessions.path(),
            &format!("{}-cancel", label.to_lowercase().replace('/', "-")),
        )),
        ToolSelection::Disabled,
        runtime_startup(),
        Some(evidence.path().to_path_buf()),
        vec![beta_route()],
    )
    .unwrap_or_else(|error| panic!("{label} runner constructs: {error}"));
    let cancellation = runner.cancel_token();
    let task = tokio::spawn(async move {
        if json {
            runner.run_json("cancel after canonical dispatch").await
        } else {
            runner.run("cancel after canonical dispatch").await
        }
    });

    wait_for_dispatch(&dispatched).await;
    cancellation.cancel();
    let result = join_task_with_timeout(task, &format!("{label} runner cancellation")).await;

    assert_eq!(result.exit_code, ExitCode::Interrupted as i32, "{label}");
    assert_eq!(result.stderr, "cancelled", "{label}");
    assert_wire_requests_use_canonical_route(label, &calls, 1);
    if json {
        let lines = phase17::parse_ndjson(&result.stdout);
        assert_eq!(
            lines
                .iter()
                .filter(|line| line["type"] == "Agent" && line["event"]["type"] == "AgentEnd")
                .count(),
            1,
            "JSON/NDJSON emits exactly one real terminal AgentEnd"
        );
        let summary = lines
            .iter()
            .find(|line| line["type"] == "session_summary")
            .expect("JSON/NDJSON emits its session summary after cancellation");
        assert_eq!(summary["model"], MODEL_SPEC);
    } else {
        assert!(
            result.stdout.is_empty(),
            "cancelled print emits no completion"
        );
    }
    let run_dir = finalized_run_dir(evidence.path());
    assert_durable_manifest(&run_dir, "cancelled", label);
    jsonl_kind_names(&run_dir.join("evidence.jsonl"))
}

async fn exercise_rpc_cancellation() -> Vec<String> {
    let workspace = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    let evidence = tempfile::tempdir().unwrap();
    let (provider, calls, dispatched) = cancellation_fixture();
    let mut rpc = RpcRunner::new_with_runtime_packages_and_auth(
        Box::new(provider),
        MODEL_SPEC.to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        ToolSelection::Disabled,
        None,
        Vec::new(),
        runtime_startup(),
        Some(empty_resume_info(
            workspace.path(),
            sessions.path(),
            "rpc-cancel",
        )),
        Some(evidence.path().to_path_buf()),
        phase17::static_resolver(),
        vec![beta_route()],
    )
    .expect("RPC runner constructs");
    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move { rpc.run_with_channels(command_rx, output_tx).await });
    command_tx
        .send(RpcCommand::prompt {
            id: Some("rpc-cancel-prompt".to_owned()),
            message: "cancel after canonical dispatch".to_owned(),
        })
        .unwrap();

    wait_for_dispatch(&dispatched).await;
    command_tx
        .send(RpcCommand::abort {
            id: Some("rpc-cancel-abort".to_owned()),
        })
        .unwrap();
    let mut output = Vec::new();
    loop {
        let line = recv_rpc_line(&mut output_rx).await;
        let completed = line["type"] == "run_summary";
        output.push(line);
        if completed {
            break;
        }
    }
    assert_eq!(
        output
            .iter()
            .filter(|line| line["type"] == "AgentEnd")
            .count(),
        1,
        "RPC emits exactly one real terminal AgentEnd"
    );
    assert!(output.iter().any(|line| {
        line["type"] == "response" && line["id"] == "rpc-cancel-abort" && line["success"] == true
    }));
    assert!(
        output.iter().all(|line| !matches!(
            line["type"].as_str(),
            Some("MessageStart" | "MessageUpdate" | "MessageEnd")
        )),
        "RPC cancellation is not converted to an assistant completion"
    );
    assert_wire_requests_use_canonical_route("RPC", &calls, 1);
    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let exit = join_task_with_timeout(task, "RPC cancellation shutdown").await;
    assert_eq!(exit, ExitCode::Success as i32);
    let run_dir = finalized_run_dir(evidence.path());
    assert_durable_manifest(&run_dir, "cancelled", "RPC");
    jsonl_kind_names(&run_dir.join("evidence.jsonl"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase17_real_cancellation_is_typed_and_complete_in_every_public_mode() {
    let observations = [
        exercise_harness_cancellation("interactive assembly", true).await,
        exercise_harness_cancellation("CodingHarness", false).await,
        exercise_runner_cancellation(false).await,
        exercise_runner_cancellation(true).await,
        exercise_rpc_cancellation().await,
    ];
    for (index, kinds) in observations.iter().enumerate().skip(1) {
        assert_eq!(
            kinds, &observations[0],
            "mode {index} emits the same complete cancellation evidence kinds"
        );
    }
}

struct AwaitedRestoreExtension {
    entered: Arc<tokio::sync::Semaphore>,
    release: Arc<tokio::sync::Semaphore>,
}

struct PendingAuthResolver {
    entered: Arc<tokio::sync::Semaphore>,
}

impl AuthResolver for PendingAuthResolver {
    fn resolve<'a>(
        &'a self,
    ) -> opi_ai::credential::BoxAuthFuture<
        'a,
        Result<opi_ai::auth::ResolvedAuth, opi_ai::provider::ProviderError>,
    > {
        Box::pin(async move {
            self.entered.add_permits(1);
            std::future::pending().await
        })
    }
}

impl Extension for AwaitedRestoreExtension {
    fn name(&self) -> &str {
        "awaited-restore"
    }

    fn restore_state_async(
        &self,
        _state: serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ExtensionError>> + Send + '_>>
    {
        Box::pin(async move {
            self.entered.add_permits(1);
            let permit = self
                .release
                .acquire()
                .await
                .expect("restore release semaphore remains open");
            permit.forget();
            Ok(())
        })
    }
}

fn resume_with_awaited_extension_state(
    workspace: &std::path::Path,
    sessions: &std::path::Path,
    session_id: &str,
) -> ResumeInfo {
    let mut resume = empty_resume_info(workspace, sessions, session_id);
    let state =
        opi_agent::session::SessionEntry::ExtensionState(opi_agent::session::ExtensionStateEntry {
            id: format!("{session_id}-extension-state"),
            parent_id: None,
            timestamp: "2026-08-20T00:00:01Z".to_owned(),
            state: serde_json::json!({"awaited-restore": {"restored": true}}),
        });
    opi_agent::session::SessionWriter::open(&resume.path)
        .expect("resume fixture opens")
        .append(&state)
        .expect("extension state appends");
    resume.entries.push(state);
    resume
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase17_pre_dispatch_abort_survives_awaited_harness_preflight() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    let (provider, calls, _dispatched) = cancellation_fixture();
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
    let entered = Arc::new(tokio::sync::Semaphore::new(0));
    let release = Arc::new(tokio::sync::Semaphore::new(0));
    let mut extensions = ExtensionRegistry::new();
    extensions
        .register(Box::new(AwaitedRestoreExtension {
            entered: entered.clone(),
            release: release.clone(),
        }))
        .expect("awaited restore extension registers");
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        MODEL_SPEC.to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .resume(resume_with_awaited_extension_state(
        workspace.path(),
        sessions.path(),
        "awaited-preflight-cancel",
    ))
    .extension_registry(extensions)
    .tool_selection(ToolSelection::Disabled)
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
    })
    .build();
    let control = harness.control_handle();
    let task = tokio::spawn(async move {
        let result = harness.prompt("cancel during extension restoration").await;
        (harness, result)
    });

    wait_for_dispatch(&entered).await;
    control.abort();
    release.add_permits(1);
    let (_harness, result) =
        join_task_with_timeout(task, "pre-dispatch awaited-preflight cancellation").await;

    assert!(
        matches!(result, Err(AgentError::Cancelled)),
        "awaited preflight preserves the exact AgentError::Cancelled class: {result:?}"
    );
    assert_eq!(calls.lock().unwrap().len(), 0, "provider is not dispatched");
    let manifest = sink
        .completed_manifest()
        .expect("pre-dispatch cancellation finalizes evidence");
    assert_eq!(manifest.facts().outcome, TerminalOutcome::Cancelled);
    assert_eq!(
        manifest.facts().completeness,
        EvidenceCompleteness::Complete
    );
    assert!(matches!(
        &manifest.facts().provider,
        ProviderInvocationFacts::NotApplicable {
            reason: ProviderNotApplicableReason::CancelledBeforeProvider
        }
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase17_pending_auth_cancel_emits_one_terminal_cancellation_record() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    let (provider, calls, _dispatched) = cancellation_fixture();
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
    let auth_entered = Arc::new(tokio::sync::Semaphore::new(0));
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        MODEL_SPEC.to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .resume(empty_resume_info(
        workspace.path(),
        sessions.path(),
        "pending-auth-cancel",
    ))
    .auth_resolver(Arc::new(PendingAuthResolver {
        entered: auth_entered.clone(),
    }))
    .tool_selection(ToolSelection::Disabled)
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
    })
    .build();
    let control = harness.control_handle();
    let task = tokio::spawn(async move {
        let result = harness.prompt("cancel during pending authentication").await;
        (harness, result)
    });

    wait_for_dispatch(&auth_entered).await;
    control.abort();
    let (_harness, result) = join_task_with_timeout(task, "pending-auth cancellation").await;

    assert!(matches!(result, Err(AgentError::Cancelled)));
    assert_eq!(calls.lock().unwrap().len(), 0, "provider is not dispatched");
    let records = sink.records();
    let cancellation_records = records
        .iter()
        .filter(|record| {
            matches!(
                &record.payload,
                EvidencePayload::Diagnostic(diagnostic)
                    if diagnostic.code == opi_agent::diagnostic::code::CODE_AGENT_CANCELLED
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        cancellation_records.len(),
        1,
        "pending auth emits one typed cancellation record: {records:?}"
    );
    let manifest = sink
        .completed_manifest()
        .expect("pending-auth cancellation finalizes evidence");
    assert_eq!(manifest.facts().outcome, TerminalOutcome::Cancelled);
    assert_eq!(
        manifest.facts().completeness,
        EvidenceCompleteness::Complete
    );
    assert_eq!(
        manifest.facts().correlation.call,
        Some(cancellation_records[0].call),
        "terminal correlation uses the sole cancellation record"
    );
    assert!(matches!(
        &manifest.facts().provider,
        ProviderInvocationFacts::NotApplicable {
            reason: ProviderNotApplicableReason::CancelledBeforeProvider
        }
    ));
}

async fn exercise_immediate_runner_abort(json: bool) {
    let label = if json { "JSON/NDJSON" } else { "print" };
    let workspace = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    let evidence = tempfile::tempdir().unwrap();
    let (provider, calls, _dispatched) = cancellation_fixture();
    let mut runner = NonInteractiveRunner::new_with_resume_and_runtime_packages(
        Box::new(provider),
        MODEL_SPEC.to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        None,
        Vec::new(),
        Some(empty_resume_info(
            workspace.path(),
            sessions.path(),
            &format!(
                "{}-immediate-cancel",
                label.to_lowercase().replace('/', "-")
            ),
        )),
        ToolSelection::Disabled,
        runtime_startup(),
        Some(evidence.path().to_path_buf()),
        vec![beta_route()],
    )
    .unwrap_or_else(|error| panic!("{label} runner constructs: {error}"));
    let cancellation = runner.cancel_token();
    cancellation.cancel();
    let task = tokio::spawn(async move {
        if json {
            runner.run_json("cancel before runner dispatch").await
        } else {
            runner.run("cancel before runner dispatch").await
        }
    });

    let result = join_task_with_timeout(task, &format!("{label} immediate cancellation")).await;
    assert_eq!(result.exit_code, ExitCode::Interrupted as i32, "{label}");
    assert_eq!(result.stderr, "cancelled", "{label}");
    assert_eq!(calls.lock().unwrap().len(), 0, "{label} pre-dispatch");
    let run_dir = finalized_run_dir(evidence.path());
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(run_dir.join("manifest.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["outcome"], "cancelled", "{label}");
    assert_eq!(manifest["completeness"], "complete", "{label}");
    assert_eq!(manifest["provider"]["kind"], "not_applicable", "{label}");
    assert_eq!(
        manifest["provider"]["reason"], "cancelled_before_provider",
        "{label}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase17_print_and_json_honor_immediate_pre_dispatch_abort() {
    exercise_immediate_runner_abort(false).await;
    exercise_immediate_runner_abort(true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn phase17_production_tui_targets_two_consecutive_armed_runs() {
    let _tui_driver_lock = TUI_DRIVER_LOCK.lock().await;
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    let evidence = tempfile::tempdir().unwrap();
    let (provider, calls, dispatched) = cancellation_fixture();
    let recorder: Arc<dyn EvidenceRecorder> =
        Arc::new(FileEvidenceSink::new(evidence.path().to_path_buf()));
    let terminal_count = Arc::new(AtomicUsize::new(0));
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        MODEL_SPEC.to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .resume(empty_resume_info(
        workspace.path(),
        sessions.path(),
        "tui-consecutive-cancel",
    ))
    .execution_mode(ExecutionRunMode::Interactive)
    .tool_selection(ToolSelection::Disabled)
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
    })
    .build();
    harness.credential_store = Some(Arc::new(KeychainCredentialStore::new(
        Box::new(FakeKeyringBackend::new()),
        user.path().to_path_buf(),
    )));
    harness.subscribe(Box::new({
        let terminal_count = terminal_count.clone();
        move |event| {
            if matches!(event, AgentEvent::AgentEnd { .. }) {
                terminal_count.fetch_add(1, Ordering::SeqCst);
            }
        }
    }));
    let driver = install_interactive_tui_test_driver_with_abort_readiness(
        [
            "first cancellation",
            "<abort>",
            "second cancellation",
            "<abort>",
            "exit",
        ],
        dispatched,
    )
    .expect("interactive cancellation driver installs");

    tokio::time::timeout(
        Duration::from_secs(2),
        run_interactive_tui(
            harness,
            MODEL_SPEC.to_owned(),
            "default",
            Keybindings::default(),
        ),
    )
    .await
    .expect("two consecutive production TUI cancellations must terminate")
    .expect("production TUI cancellation fixture succeeds");

    let capture = driver.capture();
    assert_eq!(capture.user_messages, 2);
    assert_eq!(capture.provider_calls, 2);
    assert_eq!(terminal_count.load(Ordering::SeqCst), 2);
    assert_wire_requests_use_canonical_route("production TUI cancellation", &calls, 2);
    let mut run_dirs = std::fs::read_dir(evidence.path())
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join("manifest.json").is_file())
        .collect::<Vec<_>>();
    run_dirs.sort();
    assert_eq!(run_dirs.len(), 2, "each TUI cancellation finalizes one run");
    for run_dir in run_dirs {
        assert_durable_manifest(&run_dir, "cancelled", "production TUI cancellation");
    }
}
