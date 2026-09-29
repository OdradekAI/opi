//! Crate-private assembled evaluation runner.
//!
//! [`run_experiment`] is the end-to-end owner of one resolved experiment:
//! durable intent reservation, Agent dispatch, settlement, native verifier
//! dispatch, pre-seal trajectory projection, bundle sealing, receipt
//! emission, and comparison/coverage assembly - every later authority
//! transition mechanically gated by [`crate::authority::AuthorityLedger`]
//! (`EVAL-FAL-002`). Two driving modes share this flow. The hermetic
//! fixture-grade path resolves runtime-generated deterministic helpers and
//! pinned fixtures-tree native bytes. The native driving mode consumes the
//! resolved-material manifest: exact built agent
//! executables, the materialized official task package, the pinned uv
//! verifier entrypoint, one upstream oracle preflight per task, and the
//! scripted-provider listener endpoint - never synthesized helpers and
//! never fixture native bytes. Executables are never resolved from ambient
//! PATH.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::json;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

use crate::agent::opi::OpiProcessAdapter;
use crate::agent::pi::PiProcessAdapter;
use crate::agent::process::{
    AgentAdapter, AgentCompletion, AgentExecution, AgentFailureClass, AgentRunRequest, Fact,
    IsolationDirs,
};
use crate::authority::{
    AgentOutcomeObservation, AuthorityLedger, AuthorityTransition, boundary_token,
};
use crate::benchmark::process::{
    BenchmarkAdapter, BenchmarkCompletion, BenchmarkExecution, BenchmarkRunRequest,
};
use crate::benchmark::terminal_bench_21::{Tb21Profile, TerminalBench21Adapter};
use crate::bundle::{
    ArtifactKey, ArtifactRole, ArtifactSpec, IntentRecord, PairIdentity, RunBundle, Sensitivity,
    SettlementMarker, SourceIdentity, TrialIdentity,
};
use crate::comparison::{ComparisonSet, TrialFact, TrialOutcome};
use crate::experiment::ResolvedExperiment;
use crate::failure::FailureBoundaryCode;
use crate::integrity::{
    IntegrityRecord, IntegrityReview, OraclePreflight, RevisionStatus, TaskClassification,
};
use crate::process::ExitState;
use crate::runner::lifecycle::{
    CancellationSource, ObservedOutcome, SettlementKind, TrialLifecycle,
};
use crate::runner::material::NativeMaterial;
use crate::runner::native_cleanup::{OwnedCleanup, cleanup_docker_spec, cleanup_owned_resources};
use crate::trajectory::{ProjectionPipeline, TrialInputs};

/// Run report schema identity.
const RUN_REPORT_SCHEMA: &str = "opi-eval-run-report/1";
/// Per-trial receipt schema identity.
const TRIAL_RECEIPT_SCHEMA: &str = "opi-eval-trial-receipt/1";
/// Bundle-internal key reserved for the normalized expected output.
const EXPECTED_OUTPUT_KEY: &str = "normalized/expected-output";
/// Bundle-internal key of the sealed resolved-experiment control evidence.
const CONTROL_EXPERIMENT_KEY: &str = "control/experiment.json";
/// The sealed per-trial settlement record: the runner's final outcome
/// classification with its failure kind and boundary, staged before
/// sealing so offline recomputation consumes the sealed interpretation
/// instead of re-deriving one from process side effects (P19-D06).
const CONTROL_SETTLEMENT_KEY: &str = "control/settlement.json";
/// Bundle-internal key of the sealed integrity-record control evidence.
const CONTROL_INTEGRITY_KEY: &str = "control/integrity.json";
/// Bundle-internal key of the sealed trajectory projection evidence.
const TRAJECTORY_KEY: &str = "evidence/trajectory.json";
/// Bundle-internal keys of the native agent execution evidence every
/// sealed trial covers on every path.
const AGENT_STDOUT_KEY: &str = "native/agent-stdout.log";
const AGENT_STDERR_KEY: &str = "native/agent-stderr.log";
const AGENT_ANSWER_KEY: &str = "native/agent-answer.txt";
/// Bundle-internal key of the sealed authority-ledger evidence.
const AUTHORITY_LEDGER_KEY: &str = "native/authority-ledger.json";

/// The complete pre-effect artifact reservation of one trial: every
/// artifact identity the sealed bundle must cover on every path (EVAL-DUR-001,
/// EVAL-BND-001). Adapter-produced native evidence - agent completion
/// artifacts and native grader evidence - is not reserved here because it
/// materializes only with its producing record; the runner declares it as
/// produced native evidence at sealing.
fn reserved_artifact_keys() -> Vec<ArtifactKey> {
    let mut keys: Vec<ArtifactKey> = [
        EXPECTED_OUTPUT_KEY,
        AGENT_ANSWER_KEY,
        AGENT_STDERR_KEY,
        AGENT_STDOUT_KEY,
        AUTHORITY_LEDGER_KEY,
        CONTROL_EXPERIMENT_KEY,
        CONTROL_INTEGRITY_KEY,
        CONTROL_SETTLEMENT_KEY,
        TRAJECTORY_KEY,
    ]
    .iter()
    .map(|key| ArtifactKey::new(key).expect("reserved keys are canonical"))
    .collect();
    keys.sort();
    keys
}

/// One assembled run request. `behavior` is a hermetic staging knob that
/// selects helper-process behavior (never a production claim).
#[derive(Clone)]
pub(crate) struct RunRequest {
    pub(crate) config_path: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) fixtures: Option<PathBuf>,
    pub(crate) behavior: String,
    /// Recovery mode: classify the durable state of every existing trial
    /// root instead of running anything.
    pub(crate) recover: bool,
    /// Re-run one crashed trial's whole group under fresh trial identities
    /// and a new paired trial group (EVAL-DUR-002, EVAL-EXP-005).
    pub(crate) replacement_for: Option<String>,
    /// Declared canary secrets (one per line from the request). Any
    /// canary found in staged exportable content blocks sealing
    /// (`EVAL-A18`, `EVAL-SEC-005`).
    pub(crate) canaries: Vec<String>,
    /// Resolved native material (native mode): when present the runner
    /// takes the native driving mode over the hermetic fixture path.
    pub(crate) material: Option<crate::runner::material::NativeMaterial>,
    /// Frozen run plan (`run --plan`): the plan file's verified JSON is
    /// re-derived here; the run executes exactly its trials.
    pub(crate) plan: Option<std::path::PathBuf>,
}

/// The pinned hermetic canary the `canary-leak` fixture behavior emits
/// into agent output. It is a fixture fact, not a real secret; sealing
/// blocks only when it is declared to the run.
const HERMETIC_CANARY: &str = "OPZ-EVAL-CANARY-7f3a9c";

/// Typed run-path failures that abort before any report is rendered.
#[derive(Debug, Error)]
pub(crate) enum RunError {
    /// The experiment document could not be read.
    #[error("cannot read experiment document: {0}")]
    Io(#[from] std::io::Error),
    /// The document did not resolve into a frozen contract.
    #[error("experiment document rejected: {0}")]
    Resolve(#[from] crate::experiment::ResolveError),
    /// The pinned integrity digest does not address the fixture review the
    /// runner derives from this document.
    #[error("integrity digest mismatch: experiment pins {expected}, derived review is {derived}")]
    IntegrityMismatch { expected: String, derived: String },
    /// The experiment declares a benchmark this runner has no concrete
    /// adapter for.
    #[error("unsupported benchmark {benchmark} revision {revision}")]
    UnsupportedBenchmark { benchmark: String, revision: String },
    /// A subject product with no concrete adapter.
    #[error("unsupported subject product {product}")]
    UnsupportedSubject { product: String },
    /// Hermetic staging failed (helper generation, package copy).
    #[error("hermetic staging failed: {0}")]
    Staging(String),
    /// The resolved native material was rejected (drift, missing
    /// identity, or a failed upstream oracle preflight).
    #[error("native material rejected: {0}")]
    Native(String),
    /// A durable bundle operation failed outside the per-trial failure
    /// paths.
    #[error("bundle operation failed: {0}")]
    Bundle(String),
}

/// Settlement status of one assembled trial.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrialStatus {
    Sealed,
    Failed,
}

/// Everything one settled trial contributes to the report.
struct TrialResult {
    receipt: serde_json::Value,
    fact: Option<TrialFact>,
    sealed: bool,
}

/// The deterministic fixture-grade integrity review for one experiment.
///
/// Derived only from the frozen document and the staging behavior so its
/// content-addressed identity is stable; experiment documents pin that
/// digest in `benchmark.integrity_digest`.
fn fixture_integrity_review(experiment: &ResolvedExperiment, behavior: &str) -> IntegrityReview {
    let first_task = experiment
        .trials()
        .first()
        .map(|trial| trial.task.clone())
        .unwrap_or_default();
    let tasks: BTreeMap<String, TaskClassification> = experiment
        .trials()
        .iter()
        .map(|trial| trial.task.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|task| {
            let classification = if behavior == "invalid-task" && task == first_task {
                TaskClassification::PromptTestMismatch {
                    reason: "fixture: prompt asks ls; test greps find".to_owned(),
                }
            } else {
                TaskClassification::ValidAgentOutcome
            };
            (task, classification)
        })
        .collect();
    let mut excluded_trials = BTreeMap::new();
    if behavior == "integrity-exclusion"
        && let Some(first) = experiment.trials().first()
    {
        excluded_trials.insert(
            first.id.clone(),
            "fixture exclusion: workspace vanished under sandbox".to_owned(),
        );
    }
    IntegrityReview {
        benchmark: experiment.benchmark().name.clone(),
        revision: experiment.benchmark().revision.clone(),
        dataset: experiment.benchmark().dataset.clone(),
        grader: "harbor-v0.22.0-fixture".to_owned(),
        environment: format!(
            "{}-{}",
            experiment.environment().platform,
            experiment.environment().architecture
        ),
        upstream_identity: "fixture://terminal-bench-2.1".to_owned(),
        upstream_digest: "1111111111111111111111111111111111111111".to_owned(),
        oracle: Some(OraclePreflight::Passed(
            "fixture oracle preflight".to_owned(),
        )),
        status: RevisionStatus::Admitted,
        tasks,
        excluded_trials,
        reviewer: "opi-eval-fixture".to_owned(),
    }
}

/// Canonical control fingerprint of the frozen shared controls.
fn control_fingerprint(experiment: &ResolvedExperiment) -> String {
    let canonical = serde_json::to_string(&experiment.model_controls())
        .unwrap_or_else(|_| "uncanonicalizable".to_owned());
    crate::agent::opi::sha256_hex(canonical.as_bytes())
}

/// The deterministic helper standing in for one agent product. Behavior is
/// selected via `OPI_EVAL_RUN_BEHAVIOR`; every branch is bounded and emits
/// only pinned fixture bytes into the isolated workspace/trace roots.
#[cfg(unix)]
fn agent_helper_script(product: &str) -> String {
    let trace_copy = if product == "opi" {
        // Opi receives the fresh trace root as the `--trace` argv slot ($4).
        r#"  cp -r "$OPI_EVAL_TRACE_SOURCE/run-0001" "$4/""#
    } else {
        // pi emits its native stream on stdout.
        r#"  cat "$PI_EVAL_STREAM_SOURCE""#
    };
    let argv_guard = if product == "opi" {
        r#"[ "$1" = "--json" ] || { echo "argv drift: expected --json first" >&2; exit 9; }"#
    } else {
        r#"[ "$1" = "--mode" ] && [ "$2" = "json" ] || { echo "argv drift: expected --mode json" >&2; exit 9; }"#
    };
    // Excess-output arm: opi's importer reads trace files, so capped junk
    // on stderr proves the cap without failing settlement; pi's settlement
    // reads stdout, so the junk itself is the capped native stream.
    let excess_output = if product == "opi" {
        format!("{trace_copy}\n  head -c 2097152 /dev/zero | tr '\\0' 'a' >&2")
    } else {
        "  head -c 2097152 /dev/zero | tr '\\0' 'a'".to_owned()
    };
    format!(
        "#!/bin/sh\n\
# assembled-run fixture helper standing in for the {product} agent (never the real binary)\n\
{argv_guard}\n\
echo answer > answer.txt\n\
case \"$OPI_EVAL_RUN_BEHAVIOR\" in\n\
  happy|prompt-only-package|verifier-failure|seal-failure|agent-unknown-schema|agent-malformed-stream|agent-missing-terminal)\n\
{trace_copy} ;;\n\
  canary-leak)\n\
  printf '%s\n' \"$OPI_EVAL_CANARY_SECRET\" > answer.txt\n\
{trace_copy} ;;\n\
  agent-excess-output)\n\
{excess_output} ;;\n\
  agent-crash)\n\
  echo \"simulated agent crash\" >&2; exit 5 ;;\n\
  agent-timeout|agent-cancelled)\n\
  sleep 30 ;;\n\
  *) echo \"unknown behavior\" >&2; exit 9 ;;\nesac\n\
exit 0\n"
    )
}

#[cfg(windows)]
fn agent_helper_script(product: &str) -> String {
    let trace_copy = if product == "opi" {
        r#"if not exist "%~4\run-0001" mkdir "%~4\run-0001"
if exist "%OPI_EVAL_TRACE_SOURCE%\run-0001\manifest.json" copy /Y "%OPI_EVAL_TRACE_SOURCE%\run-0001\manifest.json" "%~4\run-0001\manifest.json" >nul
if exist "%OPI_EVAL_TRACE_SOURCE%\run-0001\evidence.jsonl" copy /Y "%OPI_EVAL_TRACE_SOURCE%\run-0001\evidence.jsonl" "%~4\run-0001\evidence.jsonl" >nul"#
    } else {
        r#"type "%PI_EVAL_STREAM_SOURCE%""#
    };
    let argv_guard = if product == "opi" {
        r#"if /I not "%~1"=="--json" echo argv drift: expected --json first 1>&2 & exit /b 9"#
    } else {
        r#"if /I not "%~1"=="--mode" echo argv drift: expected --mode json 1>&2 & exit /b 9
if /I not "%~2"=="json" echo argv drift: expected --mode json 1>&2 & exit /b 9"#
    };
    let excess_chunk = "a".repeat(1024);
    format!(
        "@echo off\r\n\
rem assembled-run fixture helper standing in for the {product} agent (never the real binary)\r\n\
{argv_guard}\r\n\
>answer.txt echo answer\r\n\
if /I \"%OPI_EVAL_RUN_BEHAVIOR%\"==\"happy\" goto success\r\n\
if /I \"%OPI_EVAL_RUN_BEHAVIOR%\"==\"prompt-only-package\" goto success\r\n\
if /I \"%OPI_EVAL_RUN_BEHAVIOR%\"==\"verifier-failure\" goto success\r\n\
if /I \"%OPI_EVAL_RUN_BEHAVIOR%\"==\"seal-failure\" goto success\r\n\
if /I \"%OPI_EVAL_RUN_BEHAVIOR%\"==\"agent-unknown-schema\" goto success\r\n\
if /I \"%OPI_EVAL_RUN_BEHAVIOR%\"==\"agent-malformed-stream\" goto success\r\n\
if /I \"%OPI_EVAL_RUN_BEHAVIOR%\"==\"agent-missing-terminal\" goto success\r\n\
if /I \"%OPI_EVAL_RUN_BEHAVIOR%\"==\"canary-leak\" goto canary\r\n\
if /I \"%OPI_EVAL_RUN_BEHAVIOR%\"==\"agent-excess-output\" goto excess\r\n\
if /I \"%OPI_EVAL_RUN_BEHAVIOR%\"==\"agent-crash\" goto crash\r\n\
if /I \"%OPI_EVAL_RUN_BEHAVIOR%\"==\"agent-timeout\" goto wait_forever\r\n\
if /I \"%OPI_EVAL_RUN_BEHAVIOR%\"==\"agent-cancelled\" goto wait_forever\r\n\
echo unknown behavior 1>&2\r\n\
exit /b 9\r\n\
:success\r\n\
{trace_copy}\r\n\
exit /b 0\r\n\
:canary\r\n\
>answer.txt <nul set /p \"=%OPI_EVAL_CANARY_SECRET%\"\r\n\
{trace_copy}\r\n\
exit /b 0\r\n\
:excess\r\n\
{trace_copy}\r\n\
for /L %%i in (1,1,2048) do <nul set /p \"={excess_chunk}\" 1>&2\r\n\
exit /b 0\r\n\
:crash\r\n\
echo simulated agent crash 1>&2\r\n\
exit /b 3\r\n\
:wait_forever\r\n\
goto wait_forever\r\n"
    )
}

/// The deterministic helper standing in for the native verifier.
#[cfg(unix)]
fn verifier_helper_script(report_name: &str, _native_source: &Path) -> Result<String, RunError> {
    Ok(format!(
        "#!/bin/sh\n\
# assembled-run fixture helper standing in for the native verifier (never the real grader)\n\
case \"$OPI_EVAL_RUN_BEHAVIOR\" in\n\
  happy|canary-leak|agent-crash|agent-timeout)\n\
    cp \"$OPI_EVAL_NATIVE_SOURCE\" ./{report_name} ;;\n\
  verifier-failure)\n\
    cp \"$OPI_EVAL_NATIVE_SOURCE\" ./{report_name}\n\
    exit 5 ;;\n\
  *) echo \"unknown behavior\" >&2; exit 9 ;;\nesac\n\
exit 0\n"
    ))
}

#[cfg(windows)]
fn verifier_helper_script(report_name: &str, native_source: &Path) -> Result<String, RunError> {
    let native_source = std::fs::canonicalize(native_source)
        .map_err(|error| RunError::Staging(error.to_string()))?;
    Ok(format!(
        "@echo off\r\n\
rem assembled-run fixture helper standing in for the native verifier (never the real grader)\r\n\
if /I \"%OPI_EVAL_RUN_BEHAVIOR%\"==\"happy\" goto success\r\n\
if /I \"%OPI_EVAL_RUN_BEHAVIOR%\"==\"canary-leak\" goto success\r\n\
if /I \"%OPI_EVAL_RUN_BEHAVIOR%\"==\"agent-crash\" goto success\r\n\
if /I \"%OPI_EVAL_RUN_BEHAVIOR%\"==\"agent-timeout\" goto success\r\n\
if /I \"%OPI_EVAL_RUN_BEHAVIOR%\"==\"verifier-failure\" goto failure\r\n\
echo unknown behavior 1>&2\r\n\
exit /b 9\r\n\
:success\r\n\
copy /Y \"{}\" \".\\{report_name}\" >nul\r\n\
if errorlevel 1 exit /b 9\r\n\
exit /b 0\r\n\
:failure\r\n\
copy /Y \"{}\" \".\\{report_name}\" >nul\r\n\
if errorlevel 1 exit /b 9\r\n\
exit /b 5\r\n",
        native_source.display(),
        native_source.display(),
    ))
}

fn helper_path(root: &Path, stem: &str) -> PathBuf {
    #[cfg(unix)]
    let extension = "sh";
    #[cfg(windows)]
    let extension = "cmd";
    root.join(format!("{stem}.{extension}"))
}

/// Scans the staged bundle files for declared canaries. Reads exactly the
/// bytes sealing would cover; returns the first leaking logical key.
fn scan_staged_for_canaries(
    bundle_root: &Path,
    staged_keys: &[ArtifactKey],
    canaries: &[String],
) -> Option<ArtifactKey> {
    for canary in canaries {
        for key in staged_keys {
            let Ok(bytes) = std::fs::read(bundle_root.join("artifacts").join(key.as_str())) else {
                continue;
            };
            if !bytes.is_empty()
                && !canary.is_empty()
                && bytes
                    .windows(canary.len())
                    .any(|window| window == canary.as_bytes())
            {
                return Some(key.clone());
            }
        }
    }
    None
}

fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)
            .unwrap_or_else(|error| panic!("helper metadata: {error}"))
            .permissions();
        permissions.set_mode(0o755);
        let _ = std::fs::set_permissions(path, permissions);
    }
    #[cfg(not(unix))]
    let _ = path;
}

fn copy_dir_recursive(source: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            copy_dir_recursive(&path, &target.join(entry.file_name()))?;
        } else {
            std::fs::copy(&path, target.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn write_helper(path: &Path, script: &str) -> Result<(), RunError> {
    std::fs::write(path, script).map_err(|error| RunError::Staging(error.to_string()))?;
    make_executable(path);
    Ok(())
}

/// Run one assembled experiment and return its run report.
///
/// `run_cancel` is the one run-level cancellation token shared with every
/// dispatch (P19-D07): once cancelled, no new trial dispatches start, the
/// in-flight owned process trees cancel, and the run still settles and
/// renders its interrupted report.
pub(crate) async fn run_experiment(
    request: &RunRequest,
    run_cancel: &CancellationToken,
) -> Result<serde_json::Value, RunError> {
    let plan_json = match &request.plan {
        Some(path) => {
            let text = std::fs::read_to_string(path)?;
            let plan: serde_json::Value = serde_json::from_str(&text)
                .map_err(|error| RunError::Native(format!("run plan parse failed: {error}")))?;
            crate::runner::plan::verify_plan(&plan)
                .map_err(|error| RunError::Native(error.to_string()))?;
            Some(plan)
        }
        None => None,
    };
    let source = match &plan_json {
        Some(plan) => crate::runner::plan::plan_source(plan)
            .map_err(|error| RunError::Native(error.to_string()))?,
        None => std::fs::read_to_string(&request.config_path)?,
    };
    let experiment = ResolvedExperiment::resolve(&source)?;
    if let Some(plan) = &plan_json {
        let recorded = plan["experiment"]["manifest_digest"]
            .as_str()
            .ok_or_else(|| RunError::Native("run plan carries no manifest digest".to_owned()))?;
        if experiment.manifest_digest() != recorded {
            return Err(RunError::Native(format!(
                "run-plan identity drift: plan records {recorded}, the frozen source resolves {}",
                experiment.manifest_digest()
            )));
        }
    }
    if request.recover {
        return recovery_report(&experiment, &request.root, plan_json.as_ref());
    }
    if request.material.is_some() && request.behavior != "happy" {
        return Err(RunError::Native(
            "the native driving mode admits only the default staging behavior".to_owned(),
        ));
    }
    // Execution-mode gate (P19-D02): native material is consistent only
    // with the declared native modes, and a declared native mode requires
    // material. Hermetic fixture runs never mix with native material.
    match (experiment.execution().mode, request.material.is_some()) {
        (crate::experiment::ExecutionMode::Hermetic, true) => {
            return Err(RunError::Native(
                "native material requires execution mode native-fixture or live".to_owned(),
            ));
        }
        (
            crate::experiment::ExecutionMode::NativeFixture
            | crate::experiment::ExecutionMode::Live,
            false,
        ) => {
            return Err(RunError::Native(
                "execution mode native-fixture/live requires --native-material".to_owned(),
            ));
        }
        _ => {}
    }
    let native = match &request.material {
        Some(material) => Some(prepare_native(material, &experiment)?),
        None => None,
    };
    // Corpus-aware selection expansion (P19-D05): `[selection]`
    // documents expand against the enumerated task inventory here, on
    // the same code path `plan` freezes; a plan run must re-expand to
    // exactly the frozen trials.
    let experiment = match experiment.selection() {
        Some(_) => {
            let root = crate::runner::plan::task_package_root(
                &experiment,
                request.fixtures.as_deref(),
                request.material.as_ref(),
            )
            .map_err(|error| RunError::Native(error.to_string()))?;
            let corpus = crate::runner::plan::enumerate_corpus(&root)
                .map_err(|error| RunError::Native(error.to_string()))?;
            let bound = match &request.material {
                Some(material) => Some(
                    crate::runner::plan::profile_bound_task(&experiment, material)
                        .map_err(|error| RunError::Native(error.to_string()))?,
                ),
                None => None,
            };
            let expansion = crate::runner::plan::expand_selection(
                &experiment,
                &corpus,
                bound.as_ref(),
                request.material.as_ref(),
                &request.root,
            )
            .map_err(|error| RunError::Native(error.to_string()))?;
            if let Some(plan) = &plan_json {
                let frozen: Vec<crate::experiment::DeclaredTrial> = plan["trials"]
                    .as_array()
                    .map(std::borrow::ToOwned::to_owned)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|entry| {
                        Some(crate::experiment::DeclaredTrial {
                            id: entry["id"].as_str()?.to_owned(),
                            subject: entry["subject"].as_str()?.to_owned(),
                            task: entry["task"].as_str()?.to_owned(),
                            group: entry["group"].as_str()?.to_owned(),
                        })
                    })
                    .collect();
                if frozen != expansion.trials {
                    return Err(RunError::Native(
                        "run-plan trial drift: the frozen trials no longer match the                          corpus expansion (materials or corpus changed; freeze a new plan)"
                            .to_owned(),
                    ));
                }
            }
            experiment.clone().replace_trials(expansion.trials)
        }
        None => experiment,
    };
    let experiment = match &request.replacement_for {
        Some(crashed) => replace_group(&source, &experiment, crashed, &request.root)?,
        None => experiment,
    };
    if let (Some(tasks), Some(material)) = (&native, &request.material) {
        let adapter = adapter_key_for(&experiment)?;
        let benchmark = material
            .benchmark(&adapter)
            .map_err(|error| RunError::Native(error.to_string()))?;
        let cache = benchmark
            .verifier_env
            .get("UV_CACHE_DIR")
            .map(Path::new)
            .unwrap_or(&benchmark.corpus.root);
        let observations =
            super::prepare::observe_resources(cache, &request.root, Some(&benchmark.verifier_env));
        let family = if adapter == "deepswe-v1.1" {
            crate::benchmark::task_lifecycle::TaskFamily::Pier
        } else {
            crate::benchmark::task_lifecycle::TaskFamily::Harbor
        };
        for id in experiment
            .trials()
            .iter()
            .map(|trial| &trial.task)
            .collect::<BTreeSet<_>>()
        {
            let inputs = tasks.get(id).ok_or_else(|| {
                RunError::Native(format!("selected task {id:?} is not materialized"))
            })?;
            let requirements = super::prepare::effective_task_requirements(
                &inputs.task_package,
                family,
                experiment.execution(),
            )
            .map_err(|error| RunError::Native(error.to_string()))?;
            let preflight = super::prepare::check_requirements(&requirements, &observations);
            if preflight["status"] != "ok" {
                return Err(RunError::Native(format!(
                    "task {id} resource preflight: {}",
                    preflight["reason"]
                )));
            }
            let lock: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&material.static_lock.path)?)
                    .map_err(|error| RunError::Native(error.to_string()))?;
            let images = super::material::image_preflight(
                &inputs.task_package,
                &lock,
                &benchmark.verifier_env,
            );
            if images["status"] != "ok" {
                return Err(RunError::Native(format!("task {id}: {}", images["reason"])));
            }
        }
    }
    let integrity = match &native {
        Some(inputs) => IntegrityRecord::review(native_integrity_review(
            &experiment,
            request.material.as_ref().expect("checked above"),
            inputs,
        ))
        .map_err(|error| RunError::Native(format!("native integrity rejected: {error}")))?,
        None => IntegrityRecord::review(fixture_integrity_review(&experiment, &request.behavior))
            .map_err(|error| {
            RunError::Staging(format!("fixture integrity rejected: {error}"))
        })?,
    };
    match experiment.benchmark().integrity_digest.as_deref() {
        Some(pinned) if pinned == integrity.identity_digest() => {}
        other => {
            return Err(RunError::IntegrityMismatch {
                expected: other.unwrap_or("<absent>").to_owned(),
                derived: integrity.identity_digest().to_owned(),
            });
        }
    }
    let benchmark_revision = match &native {
        Some(_) => None,
        None => Some(BenchmarkRevision::from_experiment(
            &experiment,
            request.fixtures.as_deref().ok_or_else(|| {
                RunError::Staging("--fixtures is required for hermetic execution".to_owned())
            })?,
        )?),
    };

    // Run-level planned coverage is bound before any effect
    // (P19-D06): the durable run-control artifact records the exact
    // planned trial set under the resolved experiment identity, so
    // offline recomputation preserves unstarted trials even when no
    // trial seals - a crash at the first durable point still leaves
    // the planned denominator on disk.
    let corpus_tasks = match (experiment.selection(), request.material.as_ref()) {
        (Some(_), _) | (_, Some(_)) => {
            let root = crate::runner::plan::task_package_root(
                &experiment,
                request.fixtures.as_deref(),
                request.material.as_ref(),
            )
            .map_err(|error| RunError::Native(error.to_string()))?;
            crate::runner::plan::enumerate_corpus(&root)
                .map_err(|error| RunError::Native(error.to_string()))?
                .len()
        }
        (None, None) => experiment
            .trials()
            .iter()
            .map(|trial| trial.task.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
    };
    let run_control = json!({
        "schema": "opi-eval-run-control/1",
        "experiment": experiment.experiment_id(),
        "manifest_digest": experiment.manifest_digest(),
        "corpus_tasks": corpus_tasks,
        "selection": experiment.selection().map(|selection| json!({
            "scope": selection.scope.as_str(),
            "task_ids": selection.task_ids,
            "repetitions": selection.repetitions,
        })),
        "planned_trials": experiment
            .trials()
            .iter()
            .map(|trial| json!({
                "id": trial.id,
                "subject": trial.subject,
                "task": trial.task,
                "group": trial.group,
            }))
            .collect::<Vec<_>>(),
        "plan_digest": plan_json.as_ref().and_then(|plan| {
            plan["plan_digest"].as_str().map(str::to_owned)
        }),
    });
    std::fs::create_dir_all(&request.root).map_err(|error| RunError::Staging(error.to_string()))?;
    for entry in std::fs::read_dir(&request.root)? {
        if matches!(
            entry?.file_name().to_str(),
            Some("trials" | "run-report.json")
        ) {
            return Err(RunError::Staging(
                "run root already contains execution evidence; use recovery or a fresh root"
                    .to_owned(),
            ));
        }
    }
    // Claim the run before any trial effect. A concurrent or repeated caller
    // cannot replace control bytes; even an interrupted write keeps its claim.
    let mut control_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(request.root.join("run-control.json"))
        .map_err(|error| RunError::Staging(format!("cannot reserve fresh run control: {error}")))?;
    use std::io::Write;
    control_file.write_all(
        &serde_json::to_vec_pretty(&run_control)
            .map_err(|error| RunError::Staging(error.to_string()))?,
    )?;
    control_file.sync_all()?;

    let mut trials_out: Vec<serde_json::Value> = Vec::new();
    let mut facts: Vec<TrialFact> = Vec::new();
    let mut all_sealed = true;
    // Bounded execution width (P19-D05): the declared concurrency is an
    // upper bound. Hermetic trials are fully isolated under their own
    // trial roots, so widths above one execute in parallel waves of
    // independent per-trial runtimes; native trials stay serial (the
    // shared scripted-provider listener is one resource) - the bound
    // never exceeds the declaration either way.
    let width = if native.is_some() {
        1
    } else {
        experiment.execution().concurrency.max(1) as usize
    };
    eprintln!(
        "execution: {} trials, width {width}",
        experiment.trials().len()
    );
    let mut declared_trials: Vec<&crate::experiment::DeclaredTrial> =
        experiment.trials().iter().collect();
    for chunk in declared_trials.chunks_mut(width.max(1)) {
        if run_cancel.is_cancelled() {
            break;
        }
        let mut waves = Vec::new();
        for declared in chunk {
            let subject = experiment
                .subjects()
                .iter()
                .find(|subject| subject.id == declared.subject)
                .expect("resolved experiments reference known subjects");
            eprintln!("trial {}: stage=start", declared.id);
            waves.push((declared.clone(), subject.product.clone()));
        }
        let mut settled = Vec::with_capacity(waves.len());
        if waves.len() == 1 || native.is_some() {
            for (declared, product) in waves {
                let result = run_trial(
                    &experiment,
                    &integrity,
                    benchmark_revision.as_ref(),
                    &declared,
                    &product,
                    native.as_ref(),
                    request,
                    run_cancel,
                )
                .await?;
                settled.push((declared, result));
            }
        } else {
            // Parallel wave: each hermetic trial runs on its own thread
            // and single-threaded runtime over fully cloned inputs (the
            // trials are isolated under their own roots, so nothing is
            // shared).
            let mut joins = Vec::with_capacity(waves.len());
            for (declared, product) in waves {
                let experiment = experiment.clone();
                let integrity = integrity.clone();
                let revision = benchmark_revision.clone();
                let request = (*request).clone();
                let run_cancel = run_cancel.clone();
                joins.push((
                    declared.clone(),
                    std::thread::spawn(move || {
                        let runtime = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .map_err(|error| RunError::Staging(error.to_string()))?;
                        runtime.block_on(run_trial(
                            &experiment,
                            &integrity,
                            revision.as_ref(),
                            &declared,
                            &product,
                            None,
                            &request,
                            &run_cancel,
                        ))
                    }),
                ));
            }
            for (declared, join) in joins {
                let result = join
                    .join()
                    .map_err(|_| RunError::Staging("parallel trial panicked".to_owned()))??;
                settled.push((declared, result));
            }
        }
        for (declared, result) in settled {
            eprintln!(
                "trial {}: settled={}",
                declared.id,
                if result.sealed { "sealed" } else { "failed" }
            );
            if let Some(fact) = result.fact {
                facts.push(fact);
            }
            all_sealed &= result.sealed;
            trials_out.push(result.receipt);
        }
        // Stop new affected dispatch (P19-D07): once the run-level token
        // is cancelled, remaining planned trials never start; the run
        // keeps every already-observed settlement and reports the rest as
        // explicit missing coverage instead of dispatching them.
        if run_cancel.is_cancelled() {
            break;
        }
    }

    // Coverage assembly: per-pair states stay visible; a structural
    // assembly failure is a typed non-success outcome, never a hidden pair.
    let (pairs, outcome, comparison_error) =
        match ComparisonSet::assemble(&experiment, &integrity, &facts) {
            Ok(set) => {
                let pairs: Vec<serde_json::Value> = set
                    .pairs()
                    .iter()
                    .map(|pair| {
                        json!({
                            "edge": pair.edge(),
                            "task": pair.task(),
                            "group": pair.group(),
                            "baseline_trial": pair.baseline_trial(),
                            "candidate_trial": pair.candidate_trial(),
                            "comparability": comparability_token(pair.comparability()),
                        })
                    })
                    .collect();
                let all_comparable = set
                    .pairs()
                    .iter()
                    .all(|pair| pair.comparability().is_comparable());
                let outcome = if all_comparable && all_sealed {
                    "completed"
                } else {
                    "incomplete"
                };
                (pairs, outcome, None)
            }
            Err(error) => (Vec::new(), "incomplete", Some(error.to_string())),
        };

    let mut report = json!({
        "schema": RUN_REPORT_SCHEMA,
        "experiment": experiment.experiment_id(),
        "manifest_digest": experiment.manifest_digest(),
        "integrity_digest": integrity.identity_digest(),
        "outcome": outcome,
        "trials": trials_out,
        "pairs": pairs,
    });
    if let Some(selection) = experiment.selection() {
        report["selection"] = json!({
            "scope": selection.scope.as_str(),
            "task_ids": experiment
                .trials()
                .iter()
                .map(|trial| trial.task.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>(),
            "repetitions": selection.repetitions,
        });
        report["execution_width"] = json!(width);
    }
    if let Some(plan) = &plan_json {
        report["plan_digest"] = plan["plan_digest"].clone();
    }
    if let Some(reason) = comparison_error {
        report["comparison_error"] = json!(reason);
    }
    // The offline report path recomputes from sealed
    // assembled outputs only: persist the run report at the run root so
    // regrade/report never re-run anything to rebuild the denominator.
    persist_run_report(&request.root, &report)?;
    Ok(report)
}

/// Persists the run report under the run root as one canonical JSON file.
/// Writing it is part of the run itself; offline commands only read it.
fn persist_run_report(root: &Path, report: &serde_json::Value) -> Result<(), RunError> {
    let bytes = serde_json::to_vec(report).map_err(|error| RunError::Staging(error.to_string()))?;
    std::fs::create_dir_all(root).map_err(|error| RunError::Staging(error.to_string()))?;
    std::fs::write(root.join("run-report.json"), &bytes)
        .map_err(|error| RunError::Staging(error.to_string()))
}

/// The concrete benchmark revision an experiment declares, with the staging
/// inputs its adapter needs.
/// The package subdirectory one declared task stages from: a
/// single-task package root stages itself; a corpus root stages the
/// task's own subdirectory (P19-D06 selection).
fn task_source_dir(package: &Path, task: &str) -> Result<PathBuf, RunError> {
    if package.join("task.toml").is_file() {
        return Ok(package.to_path_buf());
    }
    let dir = package.join(task);
    if dir.join("task.toml").is_file() {
        Ok(dir)
    } else {
        Err(RunError::Staging(format!(
            "task {task:?} has no package under {}",
            package.display()
        )))
    }
}

#[derive(Clone)]
enum BenchmarkRevision {
    TerminalBench21 {
        profile_bytes: Vec<u8>,
        task_package: PathBuf,
        /// Hermetic-only saved fixture bytes the helper replays; `None`
        /// in native mode, where the real verifier writes the report.
        fixture_report: Option<PathBuf>,
        /// Hermetic-only benchmark profile directory: a corpus package
        /// resolves each task's own pinned profile from
        /// `<root>/corpus/<task>.toml` (P19-D06 selection).
        profile_root: Option<PathBuf>,
    },
    TerminalBench30 {
        profile_bytes: Vec<u8>,
        task_package: PathBuf,
        fixture_report: Option<PathBuf>,
        profile_root: Option<PathBuf>,
    },
    DeepSwe {
        profile_bytes: Vec<u8>,
        task_package: PathBuf,
        fixture_report: Option<PathBuf>,
        profile_root: Option<PathBuf>,
    },
}

impl BenchmarkRevision {
    /// The effective profile bytes for one declared task: a corpus
    /// task's own pinned profile when the fixture ships one, else the
    /// benchmark-level profile.
    fn profile_bytes_for(&self, task: &str) -> Vec<u8> {
        let (bytes, root) = match self {
            Self::TerminalBench21 {
                profile_bytes,
                profile_root,
                ..
            }
            | Self::TerminalBench30 {
                profile_bytes,
                profile_root,
                ..
            }
            | Self::DeepSwe {
                profile_bytes,
                profile_root,
                ..
            } => (profile_bytes, profile_root),
        };
        match root
            .as_ref()
            .map(|root| root.join("corpus").join(format!("{task}.toml")))
        {
            Some(path) if path.is_file() => std::fs::read(&path).unwrap_or_else(|_| bytes.clone()),
            _ => bytes.clone(),
        }
    }
}

/// The native report file name a revision's verifier writes into its trace
/// root.
impl BenchmarkRevision {
    fn report_name(&self) -> &'static str {
        match self {
            BenchmarkRevision::TerminalBench21 { .. }
            | BenchmarkRevision::TerminalBench30 { .. } => "ctrf-report.json",
            BenchmarkRevision::DeepSwe { .. } => "pier-report.json",
        }
    }
}

impl BenchmarkRevision {
    fn from_experiment(experiment: &ResolvedExperiment, fixtures: &Path) -> Result<Self, RunError> {
        let benchmark = experiment.benchmark();
        let root = fixtures.join("benchmarks");
        if benchmark.name == "terminal-bench" && benchmark.revision == "2.1" {
            // The synthetic fixture profile pins the synthetic task package
            // bytes; the fixture CTRF report is the verifier's native
            // output source. All paths resolve under the caller-supplied
            // fixtures root.
            let tb21 = root.join("terminal-bench-2.1");
            return Ok(BenchmarkRevision::TerminalBench21 {
                profile_bytes: std::fs::read(tb21.join("profile/synthetic.toml"))
                    .map_err(|error| RunError::Staging(error.to_string()))?,
                task_package: tb21.join("task-package"),
                fixture_report: Some(tb21.join("ctrf/ok-six-passed.json")),
                profile_root: Some(tb21.join("profile")),
            });
        }
        if benchmark.name == "terminal-bench" && benchmark.revision == "3.0" {
            let tb30 = root.join("terminal-bench-3.0");
            return Ok(BenchmarkRevision::TerminalBench30 {
                profile_bytes: std::fs::read(tb30.join("profile/synthetic.toml"))
                    .map_err(|error| RunError::Staging(error.to_string()))?,
                task_package: tb30.join("task-package"),
                fixture_report: Some(tb30.join("ctrf/ok-six-passed.json")),
                profile_root: Some(tb30.join("profile")),
            });
        }
        if benchmark.name == "deepswe" && benchmark.revision == "v1.1" {
            let deepswe = root.join("deepswe-v1.1");
            return Ok(BenchmarkRevision::DeepSwe {
                profile_bytes: std::fs::read(deepswe.join("profile/synthetic.toml"))
                    .map_err(|error| RunError::Staging(error.to_string()))?,
                task_package: deepswe.join("task-package"),
                fixture_report: Some(deepswe.join("pier-report/resolved.json")),
                profile_root: Some(deepswe.join("profile")),
            });
        }
        Err(RunError::UnsupportedBenchmark {
            benchmark: benchmark.name.clone(),
            revision: benchmark.revision.clone(),
        })
    }

    /// The native construction: the materialized official task package,
    /// the production profile bytes, and no fixture report - the real
    /// verifier writes the native report into its trace root.
    fn from_native(inputs: &NativeInputs) -> Self {
        match inputs.adapter_key.as_str() {
            "terminal-bench-2.1" => BenchmarkRevision::TerminalBench21 {
                profile_bytes: inputs.profile_bytes.clone(),
                task_package: inputs.task_package.clone(),
                fixture_report: None,
                profile_root: None,
            },
            "terminal-bench-3.0" => BenchmarkRevision::TerminalBench30 {
                profile_bytes: inputs.profile_bytes.clone(),
                task_package: inputs.task_package.clone(),
                fixture_report: None,
                profile_root: None,
            },
            _ => BenchmarkRevision::DeepSwe {
                profile_bytes: inputs.profile_bytes.clone(),
                task_package: inputs.task_package.clone(),
                fixture_report: None,
                profile_root: None,
            },
        }
    }
}

/// The CLI adapter id for an experiment's declared benchmark revision.
pub(crate) fn adapter_key_for(experiment: &ResolvedExperiment) -> Result<String, RunError> {
    let benchmark = experiment.benchmark();
    match (benchmark.name.as_str(), benchmark.revision.as_str()) {
        ("terminal-bench", "2.1") => Ok("terminal-bench-2.1".to_owned()),
        ("terminal-bench", "3.0") => Ok("terminal-bench-3.0".to_owned()),
        ("deepswe", "v1.1") => Ok("deepswe-v1.1".to_owned()),
        _ => Err(RunError::UnsupportedBenchmark {
            benchmark: benchmark.name.clone(),
            revision: benchmark.revision.clone(),
        }),
    }
}

/// Everything the native driving mode resolves from the manifest before
/// any process runs: the concrete benchmark revision inputs and the
/// fail-closed package verification. The official instruction reaches the
/// selected Agent inside the task environment through the pinned runner's
/// own lifecycle - the runner never rewrites it and never appends
/// answer.txt instructions (P19-D01).
struct NativeInputs {
    adapter_key: String,
    profile_bytes: Vec<u8>,
    task_package: PathBuf,
    verifier_executable: PathBuf,
    verifier_env: std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString>,
}

/// Resolves and verifies the native material for one experiment.
fn prepare_native(
    material: &NativeMaterial,
    experiment: &ResolvedExperiment,
) -> Result<BTreeMap<String, NativeInputs>, RunError> {
    experiment.admit_native_controls()?;
    super::output_control::admit(material, experiment)
        .map_err(|error| RunError::Native(error.to_string()))?;
    let adapter_key = adapter_key_for(experiment)?;
    let benchmark = material
        .benchmark(&adapter_key)
        .map_err(|error| RunError::Native(error.to_string()))?;
    for declared in experiment.trials() {
        benchmark
            .task(&declared.task)
            .map_err(|error| RunError::Native(error.to_string()))?;
    }
    benchmark
        .tasks
        .iter()
        .map(|(id, task)| {
            Ok((
                id.clone(),
                NativeInputs {
                    adapter_key: adapter_key.clone(),
                    profile_bytes: std::fs::read(&task.profile.path)?,
                    task_package: task.task_package.clone(),
                    verifier_executable: benchmark.verifier_executable.path.clone(),
                    verifier_env: benchmark
                        .verifier_env
                        .iter()
                        .map(|(key, value)| (key.clone().into(), value.clone().into()))
                        .collect(),
                },
            ))
        })
        .collect()
}

/// The native integrity review: every identity derives from the verified
/// material, and the oracle entry records the preflight this run performs
/// before any agent trial.
fn native_integrity_review(
    experiment: &ResolvedExperiment,
    material: &NativeMaterial,
    inputs: &BTreeMap<String, NativeInputs>,
) -> IntegrityReview {
    let benchmark = experiment.benchmark();
    let adapter_key = &inputs
        .values()
        .next()
        .expect("validated non-empty task map")
        .adapter_key;
    let (grader, environment) = match adapter_key.as_str() {
        "terminal-bench-2.1" => ("harbor-v0.22.0", "separate-container"),
        "terminal-bench-3.0" => ("harbor-v0.22.0", "separate-verifier-container"),
        _ => ("pier-v0.3.1", "separate-pristine-verifier"),
    };
    IntegrityReview {
        benchmark: benchmark.name.clone(),
        revision: benchmark.revision.clone(),
        dataset: benchmark.dataset.clone(),
        grader: grader.to_owned(),
        environment: environment.to_owned(),
        upstream_identity: format!("{}@{}", adapter_key, material.static_lock.sha256),
        upstream_digest: {
            let benchmark = material.benchmark(adapter_key).expect("prepared benchmark");
            let tasks = benchmark
                .tasks
                .iter()
                .map(|(id, task)| {
                    json!({
                        "id": id, "native_task_name": task.native_task_name,
                        "profile_sha256": task.profile.sha256,
                        "task_package_manifest_sha256": task.task_package_manifest_sha256,
                    })
                })
                .collect::<Vec<_>>();
            super::material::sha256_hex(
                json!({"source_commit": benchmark.corpus.source_commit,
                "tasks_tree": benchmark.corpus.tasks_tree, "tasks": tasks})
                .to_string()
                .as_bytes(),
            )
        },
        // The oracle is an explicitly selected conformance diagnostic,
        // never a prerequisite silently run by `run` (P19-D01): the
        // admission record carries no oracle claim.
        oracle: None,
        status: crate::integrity::RevisionStatus::Admitted,
        tasks: inputs
            .keys()
            .map(|id| {
                (
                    id.clone(),
                    crate::integrity::TaskClassification::ValidAgentOutcome,
                )
            })
            .collect(),
        excluded_trials: std::collections::BTreeMap::new(),
        reviewer: "opi-eval-native-material".to_owned(),
    }
}

/// Resolves the selected connection's credential reference inside the
/// supervised Agent launch boundary (P19-D02). An absent or unreadable
/// reference is a typed rejection before any model effect; the value
/// never enters exportable artifacts.
pub(crate) fn resolve_connection_credential(
    connection: &crate::experiment::Connection,
) -> Result<crate::experiment::ResolvedCredential, RunError> {
    match &connection.credential {
        crate::experiment::CredentialReference::Env { variable } => match std::env::var(variable) {
            Ok(value) if !value.trim().is_empty() => {
                Ok(crate::experiment::ResolvedCredential::FromEnv {
                    variable: variable.clone(),
                    value,
                })
            }
            _ => Err(RunError::Native(format!(
                "selected connection credential environment variable {variable:?} is not set"
            ))),
        },
        crate::experiment::CredentialReference::File { path } => {
            let value = std::fs::read_to_string(path)
                .map_err(|error| {
                    RunError::Native(format!(
                        "selected connection credential file {path:?} is unreadable: {error}"
                    ))
                })?
                .trim()
                .to_owned();
            if value.is_empty() {
                return Err(RunError::Native(format!(
                    "selected connection credential file {path:?} is empty"
                )));
            }
            Ok(crate::experiment::ResolvedCredential::FromFile {
                path: path.clone(),
                value,
            })
        }
    }
}

/// Derives the native integrity identity for one experiment document plus
/// material manifest without running anything (the producer's config
/// materialization tool, the native material contract).
pub(crate) fn native_integrity_identity(
    config_path: &Path,
    material_path: &Path,
) -> Result<String, String> {
    let source = std::fs::read_to_string(config_path)
        .map_err(|error| format!("cannot read experiment document: {error}"))?;
    let experiment = ResolvedExperiment::resolve(&source)
        .map_err(|error| format!("experiment document rejected: {error}"))?;
    let products = experiment
        .subjects()
        .iter()
        .map(|subject| subject.product.as_str())
        .collect::<Vec<_>>();
    let material =
        NativeMaterial::load_for(material_path, &products).map_err(|error| error.to_string())?;
    let inputs = prepare_native(&material, &experiment).map_err(|error| error.to_string())?;
    let review = IntegrityRecord::review(native_integrity_review(&experiment, &material, &inputs))
        .map_err(|error| format!("native integrity rejected: {error}"))?;
    Ok(review.identity_digest().to_owned())
}

/// The concrete adapter for one resolved revision (shared by trials and
/// the oracle preflight).
fn benchmark_adapter_for(
    revision: &BenchmarkRevision,
) -> Result<Box<dyn BenchmarkAdapter>, RunError> {
    let profile_bytes = match revision {
        BenchmarkRevision::TerminalBench21 { profile_bytes, .. }
        | BenchmarkRevision::TerminalBench30 { profile_bytes, .. }
        | BenchmarkRevision::DeepSwe { profile_bytes, .. } => profile_bytes.clone(),
    };
    match revision {
        BenchmarkRevision::TerminalBench21 { .. } => {
            let profile = Tb21Profile::parse(profile_bytes.as_slice())
                .map_err(|error| RunError::Staging(error.to_string()))?;
            Ok(Box::new(TerminalBench21Adapter::from_profile(profile)))
        }
        BenchmarkRevision::TerminalBench30 { .. } => {
            let profile =
                crate::benchmark::terminal_bench_30::Tb30Profile::parse(profile_bytes.as_slice())
                    .map_err(|error| RunError::Staging(error.to_string()))?;
            Ok(Box::new(
                crate::benchmark::terminal_bench_30::TerminalBench30Adapter::from_profile(profile),
            ))
        }
        BenchmarkRevision::DeepSwe { .. } => {
            let profile =
                crate::benchmark::deepswe::DeepSweProfile::parse(profile_bytes.as_slice())
                    .map_err(|error| RunError::Staging(error.to_string()))?;
            Ok(Box::new(
                crate::benchmark::deepswe::DeepSweAdapter::from_profile(profile),
            ))
        }
    }
}

/// Run one declared trial end to end and persist its receipts.
#[allow(clippy::too_many_arguments)]
async fn run_trial(
    experiment: &ResolvedExperiment,
    integrity: &IntegrityRecord,
    revision: Option<&BenchmarkRevision>,
    declared: &crate::experiment::DeclaredTrial,
    product: &str,
    native: Option<&BTreeMap<String, NativeInputs>>,
    request: &RunRequest,
    run_cancel: &CancellationToken,
) -> Result<TrialResult, RunError> {
    match native {
        Some(tasks) => {
            let inputs = tasks
                .get(&declared.task)
                .ok_or_else(|| RunError::Native("selected task material missing".to_owned()))?;
            let revision = BenchmarkRevision::from_native(inputs);
            run_native_trial(
                experiment, integrity, &revision, declared, product, inputs, request, run_cancel,
            )
            .await
        }
        None => {
            run_fixture_trial(
                experiment,
                integrity,
                revision.expect("hermetic revision"),
                declared,
                product,
                request,
                run_cancel,
            )
            .await
        }
    }
}

/// One final trial classification for a native trial (P19-D01): combines
/// the typed native Agent terminal observation, the admitted task and
/// artifact identity, the native grading, and the lifecycle into one
/// sealed interpretation with evidence references. Neither the job exit
/// code alone nor a [`FailureBoundaryCode`] alone determines score
/// eligibility.
#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeTrialClass {
    /// The bound native grade is preserved: Agent success, Agent task
    /// failure (zero score), or a benchmark-valid Agent timeout.
    GradedNative {
        reward: u64,
        agent_failed: bool,
        timed_out: bool,
    },
    /// A terminal failure that preserves attempted coverage without an
    /// Agent score: provider/access, adapter, identity, infrastructure,
    /// or invalid grading.
    InvalidExecution {
        reason: String,
        boundary: FailureBoundaryCode,
        timed_out: bool,
    },
    /// Cancelled, unstarted, or effect-unknown: no outcome is invented
    /// (EVAL-DUR-002).
    NoOutcome { reason: &'static str },
}

/// The one classifier from a settled native job record and its bound
/// identity to the sealed trial interpretation. The exception taxonomy is
/// the pinned runner's own: `NonZeroAgentExitCodeError` and
/// `AgentTimeoutError` are graded Agent outcomes (the verifier still ran
/// on the Agent's environment); the API-error family is a terminal
/// provider/access failure that must never score; anything else is an
/// infrastructure or grader-side terminal failure.
fn classify_native_trial(
    record: Option<&crate::benchmark::process::BenchmarkRecord>,
    binding: Option<&crate::benchmark::process::NativeTrialBinding>,
    agent: Option<&crate::agent::process::AgentRecord>,
) -> NativeTrialClass {
    let Some(record) = record else {
        return NativeTrialClass::NoOutcome {
            reason: "job-never-dispatched",
        };
    };
    // Job-process-level verdicts come from the settled record.
    let process_failure = match &record.completion {
        BenchmarkCompletion::VerifiedNative { .. } => None,
        BenchmarkCompletion::Verified { .. } => None,
        BenchmarkCompletion::Failed(failure) => Some(failure),
    };
    if record.exit == ExitState::Cancelled {
        return NativeTrialClass::NoOutcome {
            reason: "job-cancelled",
        };
    }
    if let Some(failure) = process_failure {
        // A bound completion is impossible here: identity failures settle
        // as `Failed` at the Integrity boundary with the binding token.
        return NativeTrialClass::InvalidExecution {
            reason: failure.kind.to_owned(),
            boundary: failure.boundary,
            timed_out: record.exit == ExitState::TimedOut,
        };
    }
    let Some(binding) = binding else {
        // A `Verified` (hermetic-shaped) completion on a native dispatch
        // means the job never bound: no Agent score is possible.
        return NativeTrialClass::InvalidExecution {
            reason: "native-binding-absent".to_owned(),
            boundary: FailureBoundaryCode::Integrity,
            timed_out: false,
        };
    };
    if agent.is_none() {
        return NativeTrialClass::InvalidExecution {
            reason: "native-task-observation-absent".to_owned(),
            boundary: FailureBoundaryCode::Evidence,
            timed_out: false,
        };
    }
    if let Some(AgentCompletion::Failed(failure)) = agent.map(|agent| &agent.completion)
        && failure.class() == AgentFailureClass::AuthorityBoundary
        && failure.kind != "agent-outcome-not-success"
    {
        return NativeTrialClass::InvalidExecution {
            reason: failure.kind.to_owned(),
            boundary: failure.boundary,
            timed_out: false,
        };
    }
    match binding.exception_type.as_deref() {
        None => NativeTrialClass::GradedNative {
            reward: fact_value(&binding.reward),
            agent_failed: fact_value(&binding.reward) == 0
                || agent
                    .is_some_and(|agent| matches!(agent.completion, AgentCompletion::Failed(_))),
            timed_out: binding.terminal.as_ref().is_some_and(|t| t.timed_out),
        },
        Some("NonZeroAgentExitCodeError") => NativeTrialClass::GradedNative {
            reward: fact_value(&binding.reward),
            agent_failed: true,
            timed_out: false,
        },
        Some("AgentTimeoutError") => NativeTrialClass::GradedNative {
            reward: fact_value(&binding.reward),
            agent_failed: true,
            timed_out: true,
        },
        Some(
            "ApiError"
            | "ApiRateLimitError"
            | "AgentAuthenticationError"
            | "ModelNotFoundError"
            | "NetworkConnectionError",
        ) => NativeTrialClass::InvalidExecution {
            reason: format!(
                "agent-provider-failure:{}",
                binding.exception_type.as_deref().unwrap_or("")
            ),
            boundary: FailureBoundaryCode::AgentProcess,
            timed_out: false,
        },
        Some("VerifierTimeoutError") => NativeTrialClass::InvalidExecution {
            reason: "verifier-timeout".to_owned(),
            boundary: FailureBoundaryCode::Grader,
            timed_out: true,
        },
        Some(other) => NativeTrialClass::InvalidExecution {
            reason: format!("job-exception:{other}"),
            boundary: FailureBoundaryCode::Infrastructure,
            timed_out: false,
        },
    }
}

/// The known value of a fact, or zero when unknown (classification input
/// only; the fact itself stays sealed evidence).
fn fact_value(fact: &Fact) -> u64 {
    match fact {
        Fact::Known { value, .. } => *value,
        Fact::Unknown { .. } => 0,
    }
}

/// Snapshot of the job directories that already exist under a trace root
/// before dispatch (the stale-job defense of the binding).
fn snapshot_job_names(trace_root: &Path) -> Vec<std::ffi::OsString> {
    std::fs::read_dir(trace_root.join("jobs"))
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.file_name())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

/// The native reserved artifact keys: no host-Agent artifacts exist, the
/// normalized expected output is the bound native trial result.
fn reserved_artifact_keys_native() -> Vec<ArtifactKey> {
    [
        EXPECTED_OUTPUT_KEY,
        AUTHORITY_LEDGER_KEY,
        CONTROL_EXPERIMENT_KEY,
        CONTROL_INTEGRITY_KEY,
        CONTROL_SETTLEMENT_KEY,
        TRAJECTORY_KEY,
    ]
    .iter()
    .map(|key| ArtifactKey::new(key).expect("reserved keys are canonical"))
    .collect()
}

/// Stages the per-trial integration handoff (P19-D02): the typed identity
/// the pinned Harbor integration reads - exact built executable, the
/// projected selected connection (exact configuration bytes plus agent
/// environment), and the model selector - all derived from the frozen
/// experiment contract and its resolved credential. The handoff lives
/// outside the sealed bundle and outside the task environment; the
/// integration applies and then removes its credential projection before
/// the verifier phase.
fn stage_native_handoff(
    trial_root: &Path,
    declared: &crate::experiment::DeclaredTrial,
    product: &str,
    agent: &crate::runner::material::AgentMaterial,
    integration: &crate::runner::material::IntegrationMaterial,
    projection: &crate::experiment::ConnectionProjection,
    model: &str,
) -> Result<NativeHandoff, RunError> {
    let integration_dir = trial_root.join("integration");
    std::fs::create_dir_all(&integration_dir)
        .map_err(|error| RunError::Native(error.to_string()))?;
    // Own cleanup before the first write, including partial staging failures.
    let staged = NativeHandoff(integration_dir.join("handoff.json"));
    std::fs::write(
        integration_dir.join(projection.config_filename),
        &projection.config_bytes,
    )
    .map_err(|error| RunError::Native(error.to_string()))?;
    let handoff_path = integration_dir.join("handoff.json");
    let handoff = json!({
        "schema": "opi-eval-native-handoff/3",
        "trial": declared.id,
        "product": product,
        "integration_agent_name": integration.agent_name,
        "integration_import_path": integration.import_path,
        "agent_executable": agent.executable.path,
        "agent_executable_sha256": agent.executable.sha256,
        "runtime_package": agent.runtime_package,
        "model": model,
        "inference_endpoint": projection.connection.base_url,
        "config_path": projection.config_filename,
        "agent_env": projection.agent_env,
    });
    std::fs::write(
        &handoff_path,
        serde_json::to_vec(&handoff).map_err(|error| RunError::Native(error.to_string()))?,
    )
    .map_err(|error| RunError::Native(error.to_string()))?;
    let mut identity = handoff;
    identity
        .as_object_mut()
        .expect("handoff object")
        .remove("agent_env");
    identity["schema"] = json!("opi-eval-native-handoff-identity/1");
    std::fs::write(
        integration_dir.join("handoff-identity.json"),
        serde_json::to_vec(&identity).map_err(|error| RunError::Native(error.to_string()))?,
    )
    .map_err(|error| RunError::Native(error.to_string()))?;
    Ok(staged)
}

/// A setup refusal contains lifecycle evidence without an Agent execution
/// or grader node. The preflight exit belongs only to its typed proof.
fn native_refusal_trajectory(
    product: &str,
    lifecycle: &TrialLifecycle,
) -> crate::trajectory::TrialTrajectory {
    use crate::runner::lifecycle::TrialPhase;
    use crate::trajectory::{Edge, EdgeRelation, Node, NodeId, NodeKind, NodeSource, Provenance};
    debug_assert_eq!(lifecycle.phase(), TrialPhase::Settled);
    let provenance = Provenance {
        artifact_role: "trial/lifecycle".to_owned(),
        artifact_sha256: None,
        rule: "runner-lifecycle/1".to_owned(),
    };
    let nodes = [
        TrialPhase::Planned,
        TrialPhase::IntentPublished,
        TrialPhase::ProcessEffectPending,
        TrialPhase::Settled,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, phase)| Node {
        id: NodeId(index as u32),
        source: NodeSource::Lifecycle,
        kind: NodeKind::Lifecycle { phase },
        provenance: provenance.clone(),
        source_order: Some(index as u64),
        facts: BTreeMap::new(),
    })
    .collect();
    let edges = (1..4)
        .map(|index| Edge {
            from: NodeId(index - 1),
            to: NodeId(index),
            relation: EdgeRelation::SourceOrder,
        })
        .collect();
    crate::trajectory::TrialTrajectory {
        schema: crate::trajectory::TRAJECTORY_SCHEMA,
        product: product.to_owned(),
        nodes,
        edges,
        spans: Vec::new(),
    }
}

/// The observed Agent record for one bound native trial: every
/// field derives from the integration's typed terminal observation and
/// the bound identity, never from invented host-side execution. The
/// native event streams stay sealed raw evidence (P19-D01: no new
/// canonical trace format); observation coverage expands later.
fn native_agent_record(
    product: &str,
    provider_model: &str,
    binding: &crate::benchmark::process::NativeTrialBinding,
) -> (crate::agent::process::AgentRecord, serde_json::Value) {
    let Some(terminal) = binding.terminal.as_ref() else {
        // A diagnostic witness carries no integration observation, and the
        // run path never dispatches one: reaching this arm means the
        // projection was handed a witness binding, which has no honest
        // Agent terminal state to project. Fail closed instead of
        // inventing one.
        unreachable!("witness bindings never project an Agent record");
    };
    // The integration observes process exit; the pinned runner alone knows
    // whether its cancellation came from the Agent-stage deadline.
    let timed_out =
        terminal.timed_out || binding.exception_type.as_deref() == Some("AgentTimeoutError");
    let task_exit = terminal
        .exit_code
        .expect("an observed task exit is required");
    let exit = if timed_out {
        ExitState::TimedOut
    } else {
        ExitState::Exited {
            code: task_exit.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
        }
    };
    let capture = binding
        .results_path
        .parent()
        .expect("bound trial directory")
        .join("agent");
    let stdout = read_native_agent_capture(&capture, "stdout.log");
    let stderr = read_native_agent_capture(&capture, "stderr.log");
    let capture_failed = stdout.is_err() || stderr.is_err();
    let stdout = stdout.unwrap_or_default();
    let stderr = stderr.unwrap_or_default();
    let capture_overflow = stdout.truncated || stderr.truncated;
    let mut client_message = serde_json::Value::Null;
    let completion = if capture_overflow {
        AgentCompletion::Failed(crate::agent::process::AgentFailure {
            kind: "native-client-capture-overflow",
            boundary: FailureBoundaryCode::Evidence,
        })
    } else if timed_out {
        AgentCompletion::Failed(crate::agent::process::failure_kinds::TIMED_OUT)
    } else if capture_failed {
        AgentCompletion::Failed(crate::agent::process::AgentFailure {
            kind: "native-client-capture-unreadable",
            boundary: FailureBoundaryCode::Evidence,
        })
    } else if product == "pi" {
        let (completion, message) =
            crate::agent::pi::native_terminal_completion(&stdout, provider_model);
        client_message = message;
        match completion {
            AgentCompletion::Completed { .. } if task_exit != 0 => {
                AgentCompletion::Failed(crate::agent::process::failure_kinds::non_zero_exit(1))
            }
            completion => completion,
        }
    } else if let ExitState::Exited { code } = exit {
        match crate::agent::opi::opi_provider_failure_kind(code, &stderr.bytes) {
            Some(kind) => AgentCompletion::Failed(crate::agent::process::AgentFailure {
                kind,
                boundary: FailureBoundaryCode::AgentProcess,
            }),
            None if code != 0 => {
                AgentCompletion::Failed(crate::agent::process::failure_kinds::non_zero_exit(code))
            }
            None => AgentCompletion::Completed { artifacts: vec![] },
        }
    } else {
        unreachable!("native terminal is an exit or timeout");
    };
    // Both policies preserve the actual terminal message/diagnostic and the
    // interpretation that excluded a score. Full captures additionally retain
    // bounded raw client streams; terminal-only omits intermediate events.
    let evidence = json!({
        "schema": "opi-eval-native-agent-terminal/2",
        "product": product,
        "trial": terminal.trial,
        "agent_executable_sha256": terminal.agent_executable_sha256,
        "runtime_preflight": terminal.runtime_preflight,
        "task_dispatched": terminal.task_dispatched,
        "exit_code": terminal.exit_code,
        "timed_out": timed_out,
        "delivered": terminal.delivered,
        "client_message": client_message,
        "diagnostic": if product == "opi" && matches!(task_exit, 3 | 4) && !stderr.truncated {
            // The diagnostic is a preview; the complete safe stderr stream
            // has its own capture/digest and must not inflate this record.
            Some(String::from_utf8_lossy(&stderr.bytes[..stderr.bytes.len().min(1024 * 1024)]))
        } else { None },
        "stdout_sha256": crate::runner::material::sha256_hex(&stdout.bytes),
        "stderr_sha256": crate::runner::material::sha256_hex(&stderr.bytes),
        "stdout_truncated": stdout.truncated,
        "stderr_truncated": stderr.truncated,
        "failure_kind": match &completion {
            AgentCompletion::Failed(failure) => Some(failure.kind),
            AgentCompletion::Completed { .. } => None,
        },
    });
    let record = crate::agent::process::AgentRecord {
        identity: crate::agent::process::AgentIdentity {
            product: product.to_owned(),
            package: format!("native-integration:{}", binding.agent_name),
            adapter: format!("harbor-integration/{}", binding.agent_version),
            executable: PathBuf::from("<native-task-environment>"),
        },
        workspace: PathBuf::new(),
        exit,
        stdout,
        stderr,
        cleanup: crate::process::CleanupEvidence::NotRequired,
        wall_time: crate::agent::process::Fact::Unknown {
            reason: "native-agent-duration-unobserved".to_owned(),
        },
        usage: crate::agent::process::UsageProjection::default(),
        completion,
    };
    (record, evidence)
}

/// Reads a bounded native client stream after owned writers have stopped.
/// Missing streams remain empty evidence; links and special files fail closed.
fn read_native_agent_capture(
    directory: &Path,
    name: &str,
) -> std::io::Result<crate::process::OutputCapture> {
    use std::io::Read;
    const CAP: usize = crate::bundle::MAX_ARTIFACT_BYTES;
    let metadata = std::fs::symlink_metadata(directory)?;
    if !metadata.is_dir() || metadata.is_symlink() {
        return Err(std::io::Error::other(
            "unsafe native client capture directory",
        ));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = match options.open(directory.join(name)) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Default::default()),
        Err(error) => return Err(error),
    };
    let metadata = file.metadata()?;
    #[cfg(unix)]
    let single_link = {
        use std::os::unix::fs::MetadataExt;
        metadata.nlink() == 1
    };
    #[cfg(not(unix))]
    let single_link = false;
    if !metadata.is_file() || !single_link {
        return Err(std::io::Error::other("unsafe native client capture file"));
    }
    let mut bytes = Vec::new();
    file.take(CAP as u64 + 1).read_to_end(&mut bytes)?;
    let truncated = bytes.len() > CAP;
    bytes.truncate(CAP);
    Ok(crate::process::OutputCapture { bytes, truncated })
}

/// Run one declared native trial: one explicitly selected upstream job
/// runs the selected Agent inside the graded task environment, grades
/// that same trial after the integration's credential cleanup, and the
/// imported result binds to the frozen identity before anything seals
/// (P19-D01).
#[allow(clippy::too_many_arguments)]
async fn run_native_trial(
    experiment: &ResolvedExperiment,
    integrity: &IntegrityRecord,
    revision: &BenchmarkRevision,
    declared: &crate::experiment::DeclaredTrial,
    product: &str,
    inputs: &NativeInputs,
    request: &RunRequest,
    run_cancel: &CancellationToken,
) -> Result<TrialResult, RunError> {
    let trial_root = request.root.join("trials").join(&declared.id);
    if trial_root.join("bundle/intent.json").is_file() {
        return Err(RunError::Bundle(format!(
            "trial {} already has a durable intent reservation",
            declared.id
        )));
    }
    let verifier_trace = trial_root.join("verifier-trace");
    let task_dir = trial_root.join("task-package");
    for dir in [&verifier_trace, &task_dir] {
        std::fs::create_dir_all(dir).map_err(|error| RunError::Staging(error.to_string()))?;
    }
    copy_dir_recursive(
        &task_source_dir(&inputs.task_package, &declared.task)?,
        &task_dir,
    )
    .map_err(|error| RunError::Native(error.to_string()))?;
    let material = request.material.as_ref().expect("native implies material");
    let agent = material
        .agent(product)
        .map_err(|error| RunError::Native(error.to_string()))?;
    let output_control = super::output_control::snapshot(agent, product, experiment)
        .map_err(|error| RunError::Native(error.to_string()))?;
    // The selected connection projection is the single owner of request
    // identity (P19-D02): the credential resolves only here, inside the
    // supervised Agent launch boundary, before any model effect.
    let connection = experiment.selected_connection();
    let credential = resolve_connection_credential(connection)?;
    // The resolved secret value joins a trial-local in-memory scan set
    // before the projection consumes the credential (P19-AUD-001):
    // without --canaries, an exact echo of the selected credential in
    // any staged native export blocks sealing through the same
    // fail-closed guard. The value is never serialized into a guard
    // list or artifact; it exists only here and in the private launch
    // handoff.
    let mut seal_scan_secrets = request.canaries.clone();
    match &credential {
        crate::experiment::ResolvedCredential::FromEnv { value, .. }
        | crate::experiment::ResolvedCredential::FromFile { value, .. } => {
            seal_scan_secrets.push(value.clone());
        }
    }
    let client = match product {
        "opi" => crate::experiment::NativeClient::Opi,
        _ => crate::experiment::NativeClient::Pi,
    };
    let projection = experiment
        .project_connection(client, credential)
        .map_err(|error| RunError::Native(error.to_string()))?;
    let model = format!("{}:{}", connection.provider, connection.model);
    // The pinned integration the runner imports is family-specific: the
    // Terminal-Bench dispatches select the Harbor module and the DeepSWE
    // dispatch the Pier module (P19-D04).
    let task_family = match revision {
        BenchmarkRevision::DeepSwe { .. } => crate::benchmark::task_lifecycle::TaskFamily::Pier,
        _ => crate::benchmark::task_lifecycle::TaskFamily::Harbor,
    };
    let integration_family = match task_family {
        crate::benchmark::task_lifecycle::TaskFamily::Pier => "pier",
        crate::benchmark::task_lifecycle::TaskFamily::Harbor => "harbor",
    };
    let integration = agent
        .integration_for(integration_family)
        .map_err(|error| RunError::Native(error.to_string()))?;
    let handoff = stage_native_handoff(
        &trial_root,
        declared,
        product,
        agent,
        integration,
        &projection,
        &model,
    )?;

    // Durable intent before any process effect (EVAL-DUR-001): the native
    // reservation covers the control, trajectory, authority, and bound
    // native result identities; host-Agent artifacts do not exist.
    let trial_id =
        TrialIdentity::new(&declared.id).map_err(|error| RunError::Staging(error.to_string()))?;
    let mut bundle = RunBundle::create(&trial_root.join("bundle"))
        .map_err(|error| RunError::Bundle(error.to_string()))?;
    let intent = IntentRecord {
        trial: trial_id.clone(),
        pair: PairIdentity::new(
            owning_pair_seed(experiment, declared)
                .map_err(|error| RunError::Staging(error.to_string()))?
                .as_str(),
        )
        .map_err(|error| RunError::Staging(error.to_string()))?,
        artifacts: reserved_artifact_keys_native(),
        expected_output: ArtifactKey::new(EXPECTED_OUTPUT_KEY)
            .map_err(|error| RunError::Staging(error.to_string()))?,
    };
    let mut lifecycle = TrialLifecycle::plan(trial_id.clone());
    lifecycle
        .publish_intent(intent.clone())
        .map_err(|error| RunError::Bundle(error.to_string()))?;
    let proof = bundle
        .publish_intent(&intent)
        .map_err(|error| RunError::Bundle(error.to_string()))?;
    lifecycle
        .enter_process_effect_pending(proof)
        .map_err(|error| RunError::Bundle(error.to_string()))?;

    let mut ledger = AuthorityLedger::new();
    let runner_source =
        SourceIdentity::new("runner").map_err(|error| RunError::Staging(error.to_string()))?;
    let mut staged_keys: Vec<ArtifactKey> = Vec::new();
    let mut produced_keys: Vec<ArtifactKey> = Vec::new();
    staged_keys.push(stage(
        &mut bundle,
        &runner_source,
        ArtifactRole::Derived,
        CONTROL_EXPERIMENT_KEY,
        &serde_json::to_vec(experiment).map_err(|error| RunError::Staging(error.to_string()))?,
    )?);
    staged_keys.push(stage(
        &mut bundle,
        &runner_source,
        ArtifactRole::Derived,
        CONTROL_INTEGRITY_KEY,
        &integrity.canonical_bytes(),
    )?);

    if let Some(bytes) = &output_control {
        produced_keys.push(stage(
            &mut bundle,
            &runner_source,
            ArtifactRole::Derived,
            "control/output-control.json",
            bytes,
        )?);
        staged_keys.push(produced_keys.last().cloned().expect("just pushed"));
    }

    // One upstream job: the selected integration runs inside the task
    // environment and the same trial's verifier grades it. The
    // pre-dispatch job snapshot backs the binding's stale-job defense.
    // The task's declared lifecycle binds into the dispatch (P19-D03,
    // P19-D04): unsupported shapes reject before any process effect, and
    // the parse runs under the revision's runner-family policy.
    let task_lifecycle =
        crate::benchmark::task_lifecycle::parse_task_lifecycle(&task_dir, task_family)
            .map_err(|error| RunError::Native(error.to_string()))?;
    let stale_job_names = snapshot_job_names(&verifier_trace);
    let selection = crate::benchmark::process::NativeSelection {
        kind: crate::benchmark::process::SelectionKind::Integration {
            product: product.to_owned(),
            executable_sha256: agent.executable.sha256.clone(),
            trial: declared.id.clone(),
            runtime_package_manifest_sha256: agent
                .runtime_package
                .as_ref()
                .map(|package| package.manifest_sha256.clone()),
            runtime_package_entrypoint: agent
                .runtime_package
                .as_ref()
                .map(|package| package.entrypoint.to_string_lossy().replace('\\', "/")),
        },
        lifecycle: crate::benchmark::process::TaskLifecycleSelection {
            environment_mode: task_lifecycle.environment_mode,
            declared: task_lifecycle.declared_artifacts,
            base_revision: task_lifecycle.base_revision,
            docker_image: task_lifecycle.docker_image,
        },
        agent_name: integration.agent_name.clone(),
        agent_import_path: integration.import_path.clone(),
        task_name: declared_task_name(revision, &task_dir),
        model: model.clone(),
        reward_metric: "reward".to_owned(),
        module_dir: integration.module_dir.clone(),
        handoff_path: handoff.0.clone(),
        stale_job_names: stale_job_names.clone(),
    };
    let mut job_record = None;
    let mut job_rejection: Option<String> = None;
    // Both pinned runners pass these private task roots as Compose's
    // --project-directory, for the Agent and separate verifier respectively.
    // Multi-step tasks are rejected by the lifecycle parser above.
    let task_root = task_dir.canonicalize()?;
    let owned_directories = [task_root.join("environment"), task_root.join("tests")];
    let owned_resources_path = trial_root.join("integration/owned-resources.json");
    std::fs::write(
        &owned_resources_path,
        serde_json::to_vec(&json!({
            "schema": "opi-eval-owned-resources/2",
            "trial": declared.id,
            "compose_working_directories": owned_directories,
            "cleanup": serde_json::Value::Null,
        }))
        .map_err(|error| RunError::Native(error.to_string()))?,
    )
    .map_err(|error| RunError::Native(error.to_string()))?;
    if ledger.attempt(AuthorityTransition::AgentDispatch) {
        let adapter = benchmark_adapter_for(revision)?;
        let benchmark_request = BenchmarkRunRequest {
            verifier_executable: inputs.verifier_executable.clone(),
            task_dir: task_dir.clone(),
            task_id: declared.task.clone(),
            native_selection: Some(selection.clone()),
            agent_timeout_seconds: super::plan::narrowed_agent_timeout(experiment.execution()),
            verifier_timeout_seconds: experiment.execution().verifier_timeout_seconds,
            trace_root: verifier_trace.clone(),
            admitted_lock_digest: material.static_lock.sha256.clone(),
            integrity: integrity.clone(),
            extra_env: inputs.verifier_env.clone(),
        };
        match BenchmarkExecution::run(&benchmark_request, adapter.as_ref(), run_cancel).await {
            Ok(record) => job_record = Some(record),
            // Pre-spawn admission refusal: no job process ever existed.
            Err(rejection) => {
                ledger.fail(FailureBoundaryCode::Integrity);
                job_rejection = Some(rejection.token.to_owned());
            }
        }
    }

    // Upstream job exit zero does not prove that trial setup or teardown
    // succeeded. Remove credentials first, then verify cleanup on every exit.
    let handoff_removed = remove_credential_material(&handoff.0);
    let docker = cleanup_docker_spec(&inputs.verifier_env, &verifier_trace);
    let owned_cleanup = cleanup_owned_resources(
        &declared.id,
        &owned_directories,
        &owned_resources_path,
        job_record.as_ref().map(|record| &record.exit),
        &docker,
        job_record.as_ref().and_then(|record| {
            matches!(record.exit, ExitState::TimedOut | ExitState::Cancelled)
                .then_some(verifier_trace.as_path())
        }),
    )
    .await;
    // Interrupted native logs are still private, unsealed capture. Only bound
    // them once all owned writers are gone; never manufacture an Agent terminal.
    let partial_capture_failed = matches!(
        job_record.as_ref().map(|record| &record.exit),
        Some(ExitState::TimedOut | ExitState::Cancelled)
    ) && (!owned_cleanup.confirmed()
        || !job_record.as_ref().is_some_and(|record| {
            matches!(
                record.cleanup,
                crate::process::CleanupEvidence::NotRequired
                    | crate::process::CleanupEvidence::TreeTerminated { verified: true, .. }
            )
        })
        || bound_partial_capture(
            &verifier_trace,
            &trial_root.join("integration/partial-capture.json"),
            experiment.storage().capture_policy == crate::experiment::CapturePolicy::Full,
            &stale_job_names,
        )
        .is_err());
    let cleanup_failure = if !handoff_removed {
        Some("credential-material-removal-failed")
    } else if !owned_cleanup.confirmed() {
        Some("owned-resource-cleanup-unconfirmed")
    } else if partial_capture_failed {
        Some("partial-capture-unconfirmed")
    } else if job_record.as_ref().is_some_and(|record| matches!(
        &record.completion,
        BenchmarkCompletion::Failed(failure) if failure.kind == "native-command-termination-unconfirmed"
    )) {
        Some("native-command-termination-unconfirmed")
    } else {
        None
    };
    if cleanup_failure.is_some() {
        ledger.fail(FailureBoundaryCode::Infrastructure);
        run_cancel.cancel();
    }

    // Settlement and grade admission: the observed job outcome settles
    // durably; the grade transition is the binding admission (import plus
    // identity checks) over the job's own result tree.
    let settlement_kind = match job_record.as_ref().map(|record| &record.exit) {
        Some(ExitState::Exited { code }) => SettlementKind::Exited { code: *code },
        Some(ExitState::TimedOut) => SettlementKind::TimedOut,
        Some(ExitState::Cancelled) => SettlementKind::Cancelled {
            source: CancellationSource::Infrastructure,
        },
        _ => SettlementKind::Exited { code: -1 },
    };
    let settle_executed = ledger.attempt(AuthorityTransition::Settle);
    debug_assert!(settle_executed, "settle follows only a dispatch attempt");
    lifecycle
        .settle(ObservedOutcome {
            kind: settlement_kind,
        })
        .map_err(|error| RunError::Bundle(error.to_string()))?;
    bundle
        .record_settlement(&SettlementMarker {
            trial: trial_id.clone(),
        })
        .map_err(|error| RunError::Bundle(error.to_string()))?;

    let binding = match job_record.as_ref().map(|record| &record.completion) {
        Some(BenchmarkCompletion::VerifiedNative { binding, .. }) => Some(binding),
        _ => None,
    };
    // Identity/proof admission is independent of reward admission: setup
    // refusal can carry a real observation without any task or grade.
    let observation = job_record.as_ref().and_then(|_| {
        crate::benchmark::process::import_bound_native_observation(&verifier_trace, &selection).ok()
    });
    let observed_terminal = observation
        .as_ref()
        .and_then(|observation| observation.terminal.as_ref());
    let setup_refused = observed_terminal
        .is_some_and(|terminal| !terminal.task_dispatched && terminal.exit_code.is_none())
        && job_record.as_ref().is_some_and(|record| {
            matches!(record.exit, ExitState::Exited { .. })
                && !matches!(&record.completion, BenchmarkCompletion::Failed(failure)
                        if failure.kind == "native-stage-configuration-drift")
        });
    let preflight_refused = setup_refused
        && observed_terminal.is_some_and(|terminal| terminal.runtime_preflight.status != "passed");
    if setup_refused {
        ledger.fail(FailureBoundaryCode::Infrastructure);
        ledger.observe_agent(AgentOutcomeObservation::BoundaryFailure);
    }
    if job_record.is_some() && !ledger.attempt(AuthorityTransition::GradeDispatch) {
        // A boundary failure already recorded mechanically stops the
        // grade admission.
    }
    if job_record
        .as_ref()
        .is_some_and(|record| matches!(record.completion, BenchmarkCompletion::Failed(_)))
        && binding.is_none()
        && !setup_refused
    {
        let boundary = job_record
            .as_ref()
            .and_then(|record| match &record.completion {
                BenchmarkCompletion::Failed(failure) => Some(failure.boundary),
                _ => None,
            })
            .unwrap_or(FailureBoundaryCode::Grader);
        ledger.attempt_failed(AuthorityTransition::GradeDispatch, boundary);
    }

    let mut native_result = None;
    let mut native_result_digest = None;
    let native_agent = binding
        .filter(|binding| {
            binding.terminal.as_ref().is_some_and(|terminal| {
                terminal.runtime_preflight.status == "passed"
                    && terminal.task_dispatched
                    && terminal.exit_code.is_some()
            })
        })
        .map(|binding| native_agent_record(product, &model, binding));
    if let Some(observation) = &observation {
        if let Some(terminal) = &observation.terminal {
            let capture = observation
                .results_path
                .parent()
                .expect("bound trial directory")
                .join("agent");
            for (name, key) in [
                (
                    "runtime-preflight-stdout.log",
                    "native/runtime-preflight-stdout.log",
                ),
                (
                    "runtime-preflight-stderr.log",
                    "native/runtime-preflight-stderr.log",
                ),
            ] {
                let capture = read_native_agent_capture(&capture, name).map_err(|error| {
                    RunError::Native(format!("bound preflight capture unreadable: {error}"))
                })?;
                produced_keys.push(stage(
                    &mut bundle,
                    &runner_source,
                    ArtifactRole::Native,
                    key,
                    &capture.bytes,
                )?);
                staged_keys.push(produced_keys.last().cloned().expect("just pushed"));
            }
            if native_agent.is_none() {
                let evidence = json!({
                    "schema": "opi-eval-native-agent-terminal/2",
                    "product": terminal.product,
                    "trial": terminal.trial,
                    "agent_executable_sha256": terminal.agent_executable_sha256,
                    "runtime_preflight": terminal.runtime_preflight,
                    "task_dispatched": terminal.task_dispatched,
                    "exit_code": terminal.exit_code,
                    "timed_out": terminal.timed_out,
                    "delivered": terminal.delivered.iter().map(|file| json!({"source": file.source, "sha256": file.sha256})).collect::<Vec<_>>(),
                });
                produced_keys.push(stage(
                    &mut bundle,
                    &runner_source,
                    ArtifactRole::Native,
                    "native/agent-terminal.json",
                    &serde_json::to_vec(&evidence)
                        .map_err(|error| RunError::Staging(error.to_string()))?,
                )?);
                staged_keys.push(produced_keys.last().cloned().expect("just pushed"));
            }
        }
        if setup_refused {
            let bytes = std::fs::read(&observation.results_path)
                .map_err(|error| RunError::Native(format!("bound refusal unreadable: {error}")))?;
            native_result =
                Some(serde_json::from_slice(&bytes).map_err(|error| {
                    RunError::Native(format!("bound refusal unparsable: {error}"))
                })?);
            native_result_digest = Some(super::material::sha256_hex(&bytes));
            for key in ["native/trial-result.json", EXPECTED_OUTPUT_KEY] {
                let role = if key == EXPECTED_OUTPUT_KEY {
                    ArtifactRole::Normalized
                } else {
                    ArtifactRole::Native
                };
                produced_keys.push(stage(&mut bundle, &runner_source, role, key, &bytes)?);
                staged_keys.push(produced_keys.last().cloned().expect("just pushed"));
            }
            let config = std::fs::read(&observation.config_path).map_err(|error| {
                RunError::Native(format!("bound refusal config unreadable: {error}"))
            })?;
            produced_keys.push(stage(
                &mut bundle,
                &runner_source,
                ArtifactRole::Native,
                "native/trial-config.json",
                &config,
            )?);
            staged_keys.push(produced_keys.last().cloned().expect("just pushed"));
        }
    }
    if let Some((agent, terminal_evidence)) = &native_agent {
        let bytes = serde_json::to_vec(terminal_evidence)
            .map_err(|error| RunError::Staging(error.to_string()))?;
        produced_keys.push(stage(
            &mut bundle,
            &runner_source,
            ArtifactRole::Native,
            "native/agent-terminal.json",
            &bytes,
        )?);
        staged_keys.push(produced_keys.last().cloned().expect("just pushed"));
        if experiment.storage().capture_policy == crate::experiment::CapturePolicy::Full {
            for (key, capture) in [
                (AGENT_STDOUT_KEY, &agent.stdout),
                (AGENT_STDERR_KEY, &agent.stderr),
            ] {
                // Overflow stays private diagnostic evidence; a retained
                // prefix cannot represent the complete native stream.
                if capture.truncated {
                    continue;
                }
                produced_keys.push(stage(
                    &mut bundle,
                    &runner_source,
                    ArtifactRole::Native,
                    key,
                    &capture.bytes,
                )?);
                staged_keys.push(produced_keys.last().cloned().expect("just pushed"));
            }
        }
    }

    // Native job evidence into the staging bundle, under the grader's
    // source identity: bounded stream captures on every path, and the
    // bound identity references when admission succeeded.
    let grader_source = SourceIdentity::new(&format!("grader-{}", integrity.grader()))
        .map_err(|error| RunError::Staging(error.to_string()))?;
    if let Some(record) = &job_record {
        // The capture policy decides the non-terminal evidence streams:
        // `terminal-only` keeps the binding-required artifacts and the
        // terminal observation without the full job logs.
        if experiment.storage().capture_policy == crate::experiment::CapturePolicy::Full {
            produced_keys.push(stage(
                &mut bundle,
                &grader_source,
                ArtifactRole::Native,
                "native/job-stdout.log",
                &record.stdout.bytes,
            )?);
            produced_keys.push(stage(
                &mut bundle,
                &grader_source,
                ArtifactRole::Native,
                "native/job-stderr.log",
                &record.stderr.bytes,
            )?);
            staged_keys.extend(produced_keys[produced_keys.len() - 2..].to_vec());
        }
        if let Some(binding) = binding {
            for (role, path) in [
                ("native/trial-result.json", binding.results_path.clone()),
                ("native/trial-config.json", binding.config_path.clone()),
            ] {
                let bytes = std::fs::read(&path).map_err(|error| {
                    RunError::Native(format!("bound evidence unreadable: {error}"))
                })?;
                if role == "native/trial-result.json" {
                    native_result =
                        Some(serde_json::from_slice::<serde_json::Value>(&bytes).map_err(
                            |error| RunError::Native(format!("bound result unparsable: {error}")),
                        )?);
                    native_result_digest = Some(super::material::sha256_hex(&bytes));
                }
                produced_keys.push(stage(
                    &mut bundle,
                    &grader_source,
                    ArtifactRole::Native,
                    role,
                    &bytes,
                )?);
                staged_keys.push(produced_keys.last().cloned().expect("just pushed"));
            }
            if let Some(manifest) = &binding.manifest_path {
                let bytes = std::fs::read(manifest).map_err(|error| {
                    RunError::Native(format!("bound evidence unreadable: {error}"))
                })?;
                produced_keys.push(stage(
                    &mut bundle,
                    &grader_source,
                    ArtifactRole::Native,
                    "native/artifacts-manifest.json",
                    &bytes,
                )?);
                staged_keys.push(produced_keys.last().cloned().expect("just pushed"));
            }
            // The verifier-input identity (P19-D03, P19-D04): the
            // admitted task lifecycle and every declared artifact that
            // entered the verifier, with the digests of exactly the
            // collected bytes. The Pier family additionally carries the
            // committed-patch handoff identity: the base revision the
            // collect hook diffed against and the image the pristine
            // verifier rebuilt from.
            let verifier_input = serde_json::json!({
                "environment_mode": match binding.environment_mode {
                    crate::benchmark::task_lifecycle::VerifierEnvironmentMode::Shared => "shared",
                    crate::benchmark::task_lifecycle::VerifierEnvironmentMode::Separate => "separate",
                },
                "base_revision": binding.base_revision,
                "docker_image": binding.docker_image,
                "collected": binding.collected.iter().map(|artifact| serde_json::json!({
                    "source": artifact.source,
                    "destination": artifact.destination,
                    "sha256": artifact.sha256,
                })).collect::<Vec<_>>(),
            });
            produced_keys.push(stage(
                &mut bundle,
                &grader_source,
                ArtifactRole::Native,
                "native/verifier-input.json",
                &serde_json::to_vec(&verifier_input)
                    .map_err(|error| RunError::Staging(error.to_string()))?,
            )?);
            staged_keys.push(produced_keys.last().cloned().expect("just pushed"));
            // The normalized expected output is the bound native result:
            // the sealed interpretation source of this trial's grade.
            staged_keys.push(stage(
                &mut bundle,
                &runner_source,
                ArtifactRole::Normalized,
                EXPECTED_OUTPUT_KEY,
                &std::fs::read(&binding.results_path).map_err(|error| {
                    RunError::Native(format!("bound result unreadable: {error}"))
                })?,
            )?);
        }
    }

    // The one classification (P19-D01).
    let mut class = classify_native_trial(
        job_record.as_ref(),
        binding.map(std::convert::AsRef::as_ref),
        native_agent.as_ref().map(|(agent, _)| agent),
    );
    if preflight_refused {
        class = NativeTrialClass::InvalidExecution {
            reason: format!(
                "runtime-preflight-{}",
                observed_terminal
                    .expect("refusal is observed")
                    .runtime_preflight
                    .status
            ),
            boundary: FailureBoundaryCode::Infrastructure,
            timed_out: false,
        };
    } else if setup_refused {
        class = NativeTrialClass::InvalidExecution {
            reason: "native-task-setup-refused".to_owned(),
            boundary: FailureBoundaryCode::Infrastructure,
            timed_out: false,
        };
    }
    if let Some(reason) = cleanup_failure {
        class = NativeTrialClass::InvalidExecution {
            reason: reason.to_owned(),
            boundary: FailureBoundaryCode::Infrastructure,
            timed_out: matches!(
                class,
                NativeTrialClass::GradedNative {
                    timed_out: true,
                    ..
                } | NativeTrialClass::InvalidExecution {
                    timed_out: true,
                    ..
                }
            ),
        };
    }
    if matches!(
        class,
        NativeTrialClass::InvalidExecution {
            boundary: FailureBoundaryCode::Integrity,
            ..
        }
    ) {
        run_cancel.cancel();
    }

    // Pre-seal projection over the settled facts: the observed Agent
    // record is the typed native terminal observation.
    let projection = match (&job_record, &native_agent) {
        (Some(record), Some((agent, _))) => ProjectionPipeline::project(&TrialInputs {
            agent,
            benchmark: Some(record),
            lifecycle: &lifecycle,
            workspace: None,
        })
        .ok(),
        _ if setup_refused => Some(native_refusal_trajectory(product, &lifecycle)),
        _ => None,
    };
    if let Some(trajectory) = &projection {
        staged_keys.push(stage(
            &mut bundle,
            &runner_source,
            ArtifactRole::Derived,
            TRAJECTORY_KEY,
            &trajectory.canonical_bytes(),
        )?);
    }
    {
        let ledger_json =
            serde_json::to_vec(&ledger).map_err(|error| RunError::Staging(error.to_string()))?;
        staged_keys.push(stage(
            &mut bundle,
            &runner_source,
            ArtifactRole::Native,
            AUTHORITY_LEDGER_KEY,
            &ledger_json,
        )?);
    }

    // Sealing happens only after evidence admission succeeded (a bound
    // identity exists); a rejected admission retains truthful private
    // diagnostics and rejects scoring/export instead.
    // The sealed settlement record keeps the one classification durable
    // for offline recomputation (P19-D06).
    let (native_graded, native_agent_failure, native_timed_out, native_reason, native_boundary) =
        match &class {
            NativeTrialClass::GradedNative {
                agent_failed,
                timed_out,
                ..
            } => (true, *agent_failed, *timed_out, None, None),
            NativeTrialClass::InvalidExecution {
                reason,
                boundary,
                timed_out,
            } => (
                false,
                false,
                *timed_out,
                Some(reason.clone()),
                Some(boundary_token(*boundary)),
            ),
            NativeTrialClass::NoOutcome { reason } => {
                (false, false, false, Some((*reason).to_owned()), None)
            }
        };
    let settlement = json!({
        "schema": "opi-eval-trial-settlement/1",
        "trial": declared.id,
        "graded": native_graded,
        "agent_failure": native_agent_failure,
        "timed_out": native_timed_out,
        "failure_kind": native_reason,
        "boundary": native_boundary,
        "runtime_preflight": observed_terminal.map(|terminal| &terminal.runtime_preflight),
        "task_dispatched": observed_terminal.map(|terminal| terminal.task_dispatched),
    });
    staged_keys.push(stage(
        &mut bundle,
        &runner_source,
        ArtifactRole::Derived,
        CONTROL_SETTLEMENT_KEY,
        &serde_json::to_vec(&settlement).map_err(|error| RunError::Staging(error.to_string()))?,
    )?);

    let canary_leak =
        scan_staged_for_canaries(&trial_root.join("bundle"), &staged_keys, &seal_scan_secrets);
    if canary_leak.is_some() {
        run_cancel.cancel();
    }
    let seal_outcome = if cleanup_failure.is_some() {
        ledger.attempt_failed(
            AuthorityTransition::Seal,
            FailureBoundaryCode::Infrastructure,
        );
        None
    } else if binding.is_none() && !setup_refused {
        // Evidence admission did not succeed: the seal transition is
        // recorded as failed so the report (and every export derived from
        // it) is refused mechanically, never silently skipped.
        ledger.attempt_failed(AuthorityTransition::Seal, FailureBoundaryCode::Integrity);
        None
    } else if canary_leak.is_some() {
        ledger.attempt_failed(AuthorityTransition::Seal, FailureBoundaryCode::Evidence);
        None
    } else if ledger.attempt(AuthorityTransition::Seal) {
        match bundle.seal(&produced_keys) {
            Ok(receipt) => {
                lifecycle
                    .mark_sealed()
                    .map_err(|error| RunError::Bundle(error.to_string()))?;
                Some(receipt)
            }
            Err(error) => {
                ledger.attempt_failed(AuthorityTransition::Seal, error.boundary());
                None
            }
        }
    } else {
        None
    };

    // Trial facts for pairing from the one classification.
    let mut unsupported_controls = match experiment.model_controls().reasoning {
        crate::experiment::ControlValue::Value(_) => vec!["reasoning".to_owned()],
        _ => Vec::new(),
    };
    unsupported_controls.extend(experiment.unsupported_native_sampling(client));
    if matches!(
        experiment.model_controls().max_output_tokens,
        crate::experiment::ControlValue::Value(_)
    ) && output_control.is_none()
    {
        unsupported_controls.push("max_output_tokens".to_owned());
    }
    let fact = if seal_outcome.is_some() {
        let outcome = match &class {
            NativeTrialClass::GradedNative { agent_failed, .. } => {
                if *agent_failed {
                    TrialOutcome::AgentFailure
                } else {
                    TrialOutcome::ValidAgentOutcome
                }
            }
            NativeTrialClass::InvalidExecution { boundary, .. } => match boundary {
                FailureBoundaryCode::Grader => TrialOutcome::GraderFailure,
                _ => TrialOutcome::InfrastructureFailure,
            },
            NativeTrialClass::NoOutcome { .. } => {
                return Err(RunError::Bundle(format!(
                    "trial {} sealed without a classification",
                    declared.id
                )));
            }
        };
        Some(TrialFact {
            id: declared.id.clone(),
            subject: declared.subject.clone(),
            task: declared.task.clone(),
            group: declared.group.clone(),
            manifest_digest: experiment.manifest_digest().to_owned(),
            control_fingerprint: control_fingerprint(experiment),
            unsupported_controls,
            outcome,
        })
    } else {
        None
    };

    let report_executed = ledger.attempt(AuthorityTransition::Report);
    let mut receipt = json!({
        "schema": TRIAL_RECEIPT_SCHEMA,
        "id": declared.id,
        "subject": declared.subject,
        "task": declared.task,
        "group": declared.group,
        "status": if seal_outcome.is_some() { "sealed" } else { "failed" },
        "mode": "native",
        "runtime_preflight": observed_terminal.map(|terminal| &terminal.runtime_preflight),
        "task_dispatched": observed_terminal.map(|terminal| terminal.task_dispatched),
        "native_grader": crate::benchmark::process::native_grader_observation(
            native_result.as_ref(), native_result_digest.as_deref(),
            seal_outcome.is_some() && matches!(class, NativeTrialClass::GradedNative { .. }),
        ),
        "agent": match binding {
            Some(binding) => json!({
                "product": product,
                "name": binding.agent_name,
                "version": binding.agent_version,
                "model": binding.model_name,
                "terminal": binding.terminal.as_ref().map(|terminal| json!({
                    "exit_code": terminal.exit_code,
                    "timed_out": terminal.timed_out,
                    "executable_sha256": terminal.agent_executable_sha256,
                    "delivered": terminal.delivered.iter().map(|file| json!({
                        "source": file.source,
                        "sha256": file.sha256,
                    })).collect::<Vec<_>>(),
                })),
                "collected": binding.collected.iter().map(|artifact| json!({
                    "source": artifact.source,
                    "destination": artifact.destination,
                    "sha256": artifact.sha256,
                })).collect::<Vec<_>>(),
                "exception_type": binding.exception_type,
            }),
            None => json!({
                "product": product,
                "observed": false,
            }),
        },
        "verifier": job_rejection.as_ref().map(|token| {
            json!({"rejected": token, "boundary": "integrity"})
        }).or_else(|| job_record.as_ref().map(|record| {
            json!({
                "exit_state": exit_state_token(&record.exit),
                "reward": fact_token(&record.reward),
                "cleanup": format!(
                    "{}{}",
                    cleanup_token(&record.cleanup),
                    owned_cleanup_receipt(
                        &owned_cleanup,
                        if handoff_removed {
                            "removed"
                        } else {
                            "removal-failed"
                        },
                    ),
                ),
                "completion": match &record.completion {
                    BenchmarkCompletion::Verified { .. } => "verified",
                    BenchmarkCompletion::VerifiedNative { .. } => "verified-native",
                    BenchmarkCompletion::Failed(_) => "failed",
                },
                "failure_kind": benchmark_failure_kind(record),
                "boundary": benchmark_failure_boundary(record),
                "binding": binding.map(|binding| json!({
                    "job": binding.job_name.to_string_lossy(),
                    "trial": binding.trial_name,
                    "agent": binding.agent_name,
                    "task": binding.task_name,
                })),
            })
        })),
        "classification": match &class {
            NativeTrialClass::GradedNative { reward, agent_failed, timed_out } => json!({
                "class": "graded-native",
                "reward": reward,
                "agent_failed": agent_failed,
                "timed_out": timed_out,
            }),
            NativeTrialClass::InvalidExecution { reason, boundary, timed_out } => json!({
                "class": "invalid-execution",
                "reason": reason,
                "boundary": boundary_token(*boundary),
                "timed_out": timed_out,
            }),
            NativeTrialClass::NoOutcome { reason } => json!({
                "class": "no-outcome",
                "reason": reason,
            }),
        },
        "authority": authority_json(&ledger),
    });
    if let Some(sealed) = &seal_outcome {
        receipt["bundle_identity"] = json!(sealed.bundle_identity());
    }
    if let Some(trajectory) = &projection {
        let mut trajectory_receipt =
            crate::trajectory::TrajectoryReceipt::for_trajectory(trajectory);
        if let Some(sealed) = &seal_outcome {
            trajectory_receipt.record_seal(crate::trajectory::SealOutcome::Sealed {
                bundle_digest: sealed.bundle_identity().to_owned(),
            });
        }
        receipt["pre_seal_digest"] = json!(trajectory_receipt.pre_seal_digest);
        receipt["seal_result"] = match &trajectory_receipt.seal_result {
            Some(crate::trajectory::SealOutcome::Sealed { bundle_digest }) => {
                json!({"sealed": {"bundle_digest": bundle_digest}})
            }
            Some(crate::trajectory::SealOutcome::SealFailed { reason }) => {
                json!({"seal_failed": {"reason": reason}})
            }
            None => serde_json::Value::Null,
        };
    }
    if report_executed {
        let receipt_path = trial_root.join("receipt.json");
        std::fs::write(
            &receipt_path,
            serde_json::to_vec(&receipt).map_err(|error| RunError::Staging(error.to_string()))?,
        )
        .map_err(|error| RunError::Staging(error.to_string()))?;
        if seal_outcome.is_some() {
            lifecycle
                .mark_graded()
                .and_then(|()| lifecycle.mark_reported())
                .map_err(|error| RunError::Bundle(error.to_string()))?;
        }
    }

    Ok(TrialResult {
        receipt,
        fact,
        sealed: seal_outcome.is_some(),
    })
}

/// The task name an upstream trial of this revision's staged package
/// reports (the binding's expected task identity).
fn declared_task_name(revision: &BenchmarkRevision, task_dir: &Path) -> String {
    match revision {
        BenchmarkRevision::TerminalBench21 { profile_bytes, .. } => {
            Tb21Profile::parse(profile_bytes.as_slice())
                .ok()
                .and_then(|profile| profile.declared_task_name(task_dir))
                .unwrap_or_default()
        }
        BenchmarkRevision::TerminalBench30 { profile_bytes, .. } => {
            crate::benchmark::terminal_bench_30::Tb30Profile::parse(profile_bytes.as_slice())
                .ok()
                .and_then(|profile| profile.declared_task_name(task_dir))
                .unwrap_or_default()
        }
        // Pier records the task's own name (`[task] name`) as the bound
        // trial's task_name (P19-D04).
        BenchmarkRevision::DeepSwe { .. } => {
            crate::benchmark::task_lifecycle::parse_task_lifecycle(
                task_dir,
                crate::benchmark::task_lifecycle::TaskFamily::Pier,
            )
            .ok()
            .and_then(|lifecycle| lifecycle.task_name)
            .unwrap_or_default()
        }
    }
}
/// Run one declared hermetic fixture trial end to end and persist its
/// receipts: the deterministic host helper stands in for the Agent and
/// the fixture verifier replays pinned native bytes (conformance-only,
/// never an Agent score claim).
async fn run_fixture_trial(
    experiment: &ResolvedExperiment,
    integrity: &IntegrityRecord,
    revision: &BenchmarkRevision,
    declared: &crate::experiment::DeclaredTrial,
    product: &str,
    request: &RunRequest,
    run_cancel: &CancellationToken,
) -> Result<TrialResult, RunError> {
    let trial_root = request.root.join("trials").join(&declared.id);

    // A trial identity is never reused: an existing durable reservation
    // refuses the run before any staging (EVAL-EXP-005). Retries and
    // replacements take fresh identities.
    if trial_root.join("bundle/intent.json").is_file() {
        return Err(RunError::Bundle(format!(
            "trial {} already has a durable intent reservation",
            declared.id
        )));
    }

    let workspace = trial_root.join("workspace");
    let agent_trace = trial_root.join("agent-trace");
    let verifier_trace = trial_root.join("verifier-trace");
    let isolation = IsolationDirs {
        home: trial_root.join("iso/home"),
        app_data: trial_root.join("iso/appdata"),
        sessions: trial_root.join("iso/sessions"),
    };
    for dir in [
        &workspace,
        &agent_trace,
        &verifier_trace,
        &isolation.home,
        &isolation.app_data,
        &isolation.sessions,
    ] {
        std::fs::create_dir_all(dir).map_err(|error| RunError::Staging(error.to_string()))?;
    }
    let config_path = trial_root.join("bench.toml");
    std::fs::write(
        &config_path,
        "# assembled-run fixture config (never a user config)\n",
    )
    .map_err(|error| RunError::Staging(error.to_string()))?;

    // Durable intent before any process effect (EVAL-DUR-001). The
    // reservation covers every artifact identity the sealed bundle must
    // retain, including the normalized expected output, and the pair
    // identity is the unique comparison edge that owns this trial.
    let trial_id =
        TrialIdentity::new(&declared.id).map_err(|error| RunError::Staging(error.to_string()))?;
    let mut bundle = RunBundle::create(&trial_root.join("bundle"))
        .map_err(|error| RunError::Bundle(error.to_string()))?;
    let intent = IntentRecord {
        trial: trial_id.clone(),
        pair: PairIdentity::new(
            owning_pair_seed(experiment, declared)
                .map_err(|error| RunError::Staging(error.to_string()))?
                .as_str(),
        )
        .map_err(|error| RunError::Staging(error.to_string()))?,
        artifacts: reserved_artifact_keys(),
        expected_output: ArtifactKey::new(EXPECTED_OUTPUT_KEY)
            .map_err(|error| RunError::Staging(error.to_string()))?,
    };
    let mut lifecycle = TrialLifecycle::plan(trial_id.clone());
    lifecycle
        .publish_intent(intent.clone())
        .map_err(|error| RunError::Bundle(error.to_string()))?;
    let proof = bundle
        .publish_intent(&intent)
        .map_err(|error| RunError::Bundle(error.to_string()))?;
    lifecycle
        .enter_process_effect_pending(proof)
        .map_err(|error| RunError::Bundle(error.to_string()))?;

    if request.behavior == "crash-after-intent" {
        // Hermetic crash simulation at the exact durable point: the
        // process dies with the intent reservation on disk and nothing
        // else, which is precisely the state a real crash leaves.
        std::process::exit(70);
    }

    let mut ledger = AuthorityLedger::new();

    // Sealed control evidence: the resolved experiment contract and the
    // integrity record enter the bundle itself, so the sealed identity
    // commits to the exact admission inputs the trial ran under
    // (EVAL-BND-001).
    let runner_source =
        SourceIdentity::new("runner").map_err(|error| RunError::Staging(error.to_string()))?;
    let mut staged_keys: Vec<ArtifactKey> = Vec::new();
    let mut produced_keys: Vec<ArtifactKey> = Vec::new();
    staged_keys.push(stage(
        &mut bundle,
        &runner_source,
        ArtifactRole::Derived,
        CONTROL_EXPERIMENT_KEY,
        &serde_json::to_vec(experiment).map_err(|error| RunError::Staging(error.to_string()))?,
    )?);
    staged_keys.push(stage(
        &mut bundle,
        &runner_source,
        ArtifactRole::Derived,
        CONTROL_INTEGRITY_KEY,
        &integrity.canonical_bytes(),
    )?);

    // Agent dispatch through the shared execution driver. Hermetic mode
    // synthesizes the deterministic helper and replays fixture bytes;
    // native mode spawns the exact built executable from the material
    // with the deterministic configuration projection and the official
    // instruction as the prompt.
    let (agent_executable, request_env, provider_model, allow_mutating, prompt) = {
        let helper = helper_path(&trial_root, "helper-agent");
        write_helper(&helper, &agent_helper_script(product))?;
        (
            helper,
            agent_env(
                product,
                request.fixtures.as_deref().ok_or_else(|| {
                    RunError::Staging("--fixtures is required for hermetic execution".to_owned())
                })?,
                &request.behavior,
            )?,
            format!(
                "{}:{}",
                experiment.selected_connection().provider,
                experiment.selected_connection().model
            ),
            false,
            format!("assembled run: solve task {}", declared.task),
        )
    };
    let agent_request = AgentRunRequest {
        executable: agent_executable,
        prompt,
        workspace: workspace.clone(),
        trace_root: agent_trace.clone(),
        config_path,
        provider_model,
        allow_mutating,
        isolation,
        extra_env: request_env,
    };

    let agent_record = if ledger.attempt(AuthorityTransition::AgentDispatch) {
        // The trial's own cancellation stays a child of the run-level
        // token: a behavior watcher cancels only this trial, while a run
        // signal cancels every dispatch (P19-D07).
        let cancel = run_cancel.child_token();
        let adapter = if request.behavior == "agent-timeout" {
            timeout_patched_adapter(product)?
        } else {
            agent_adapter(product, &request.behavior)?
        };
        if request.behavior == "agent-cancelled" {
            // The cancellation must race a live process that has provably
            // mutated the workspace: poll the run (which spawns the child),
            // wait for the mutation marker, and only then cancel - never a
            // pre-spawn kill.
            let mut run = Box::pin(AgentExecution::run(
                &agent_request,
                adapter.as_ref(),
                &cancel,
            ));
            let marker = workspace.join("answer.txt");
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(6);
            loop {
                tokio::select! {
                    biased;
                    record = &mut run => {
                        break record.map_err(|rejection| {
                            RunError::Staging(rejection.to_string())
                        })?;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {
                        if marker.is_file() || std::time::Instant::now() > deadline {
                            cancel.cancel();
                        }
                    }
                }
            }
        } else {
            AgentExecution::run(&agent_request, adapter.as_ref(), &cancel)
                .await
                .map_err(|rejection| RunError::Staging(rejection.to_string()))?
        }
    } else {
        // Structurally impossible: nothing precedes the agent dispatch.
        return Err(RunError::Staging(
            "agent dispatch refused before any failure".to_owned(),
        ));
    };

    // Settlement of the observed outcome, durably (EVAL-DUR-003). The
    // failure classification decides authority: a scored Agent outcome
    // (the Agent's own non-zero exit or budget timeout) never fails the
    // ledger, so the native grader stays authorized over the settled
    // workspace; every boundary failure (spawn, cancellation, adapter or
    // evidence rejection, infrastructure) keeps its mechanical stop.
    let settlement_kind = agent_record.settlement_kind(CancellationSource::Infrastructure);
    let agent_failure_class = match &agent_record.completion {
        AgentCompletion::Failed(failure) => Some(failure.class()),
        AgentCompletion::Completed { .. } => None,
    };
    if agent_failure_class == Some(AgentFailureClass::AuthorityBoundary)
        && let AgentCompletion::Failed(failure) = &agent_record.completion
    {
        ledger.fail(failure.boundary);
    }
    ledger.observe_agent(match &agent_record.completion {
        AgentCompletion::Completed { .. } => AgentOutcomeObservation::Completed,
        AgentCompletion::Failed(failure) => match failure.class() {
            AgentFailureClass::ScoredAgentOutcome => AgentOutcomeObservation::ScoredFailure,
            AgentFailureClass::AuthorityBoundary => AgentOutcomeObservation::BoundaryFailure,
        },
    });
    let settle_executed = ledger.attempt(AuthorityTransition::Settle);
    debug_assert!(settle_executed, "settle follows only a dispatch attempt");
    lifecycle
        .settle(ObservedOutcome {
            kind: settlement_kind,
        })
        .map_err(|error| RunError::Bundle(error.to_string()))?;

    // Native agent evidence into the staging bundle.
    let source = SourceIdentity::new(&format!("agent-{product}"))
        .map_err(|error| RunError::Staging(error.to_string()))?;
    staged_keys.push(stage(
        &mut bundle,
        &source,
        ArtifactRole::Native,
        AGENT_STDOUT_KEY,
        &agent_record.stdout.bytes,
    )?);
    staged_keys.push(stage(
        &mut bundle,
        &source,
        ArtifactRole::Native,
        AGENT_STDERR_KEY,
        &agent_record.stderr.bytes,
    )?);
    let answer_path = workspace.join("answer.txt");
    // The expected Agent output is reserved, promised evidence: a vanished
    // or unreadable answer is a typed staging failure, never a silently
    // skipped artifact.
    let answer = std::fs::read(&answer_path).map_err(|error| {
        RunError::Staging(format!(
            "trial {}: expected agent output is unreadable: {error}",
            declared.id
        ))
    })?;
    staged_keys.push(stage(
        &mut bundle,
        &source,
        ArtifactRole::Native,
        AGENT_ANSWER_KEY,
        &answer,
    )?);
    // The normalized settled output: the same retained bytes under the
    // reserved normalized key, so the sealed expected output exists.
    staged_keys.push(stage(
        &mut bundle,
        &runner_source,
        ArtifactRole::Normalized,
        EXPECTED_OUTPUT_KEY,
        &answer,
    )?);
    if let AgentCompletion::Completed { artifacts } = &agent_record.completion {
        for artifact in artifacts {
            let bytes = std::fs::read(&artifact.path).map_err(|error| {
                RunError::Staging(format!(
                    "trial {}: agent artifact {} is unreadable: {error}",
                    declared.id, artifact.role
                ))
            })?;
            produced_keys.push(stage(
                &mut bundle,
                &source,
                ArtifactRole::Native,
                &format!("native/{}", artifact.role),
                &bytes,
            )?);
            staged_keys.push(produced_keys.last().cloned().expect("just pushed"));
        }
    }
    bundle
        .record_settlement(&SettlementMarker {
            trial: trial_id.clone(),
        })
        .map_err(|error| RunError::Bundle(error.to_string()))?;

    // Native verifier dispatch through the shared execution driver. All
    // verifier-produced bytes - the bounded stream captures and the
    // imported native report - carry the pinned grader's source identity,
    // never the Agent's, so sealed provenance names the byte producer.
    let grader_source = SourceIdentity::new(&format!("grader-{}", integrity.grader()))
        .map_err(|error| RunError::Staging(error.to_string()))?;
    let mut verifier_record = None;
    let mut verifier_rejection: Option<String> = None;
    if ledger.attempt(AuthorityTransition::GradeDispatch) {
        let fixture_report = match revision {
            BenchmarkRevision::TerminalBench21 { fixture_report, .. }
            | BenchmarkRevision::TerminalBench30 { fixture_report, .. }
            | BenchmarkRevision::DeepSwe { fixture_report, .. } => fixture_report.clone(),
        };
        let native_source = fixture_report
            .as_deref()
            .expect("hermetic benchmark revisions carry fixture reports");
        let verifier_helper = helper_path(&trial_root, "helper-verifier");
        write_helper(
            &verifier_helper,
            &verifier_helper_script(revision.report_name(), native_source)?,
        )?;
        let task_package = match revision {
            BenchmarkRevision::TerminalBench21 { task_package, .. }
            | BenchmarkRevision::TerminalBench30 { task_package, .. }
            | BenchmarkRevision::DeepSwe { task_package, .. } => task_package.clone(),
        };
        // Corpus-aware selection: a corpus package root stages the
        // declared task's own subdirectory (P19-D06).
        let task_package = task_source_dir(&task_package, &declared.task)?;
        let task_dir = trial_root.join("task-package");
        if request.behavior == "prompt-only-package" {
            // The incomplete package: the prompt alone, without the
            // image/resource/verifier contract (EVAL-BMK-002).
            std::fs::create_dir_all(&task_dir)
                .map_err(|error| RunError::Staging(error.to_string()))?;
            std::fs::copy(
                task_package.join("instruction.md"),
                task_dir.join("instruction.md"),
            )
            .map_err(|error| RunError::Staging(error.to_string()))?;
        } else {
            copy_dir_recursive(&task_package, &task_dir)
                .map_err(|error| RunError::Staging(error.to_string()))?;
        }
        let task_profile_bytes = revision.profile_bytes_for(&declared.task);
        let benchmark_request_adapter: Box<dyn BenchmarkAdapter> = match revision {
            BenchmarkRevision::TerminalBench21 { .. } => {
                let profile = Tb21Profile::parse(task_profile_bytes.as_slice())
                    .map_err(|error| RunError::Staging(error.to_string()))?;
                Box::new(TerminalBench21Adapter::from_profile(profile))
            }
            BenchmarkRevision::TerminalBench30 { .. } => {
                let profile = crate::benchmark::terminal_bench_30::Tb30Profile::parse(
                    task_profile_bytes.as_slice(),
                )
                .map_err(|error| RunError::Staging(error.to_string()))?;
                Box::new(
                    crate::benchmark::terminal_bench_30::TerminalBench30Adapter::from_profile(
                        profile,
                    ),
                )
            }
            BenchmarkRevision::DeepSwe { .. } => {
                let profile =
                    crate::benchmark::deepswe::DeepSweProfile::parse(task_profile_bytes.as_slice())
                        .map_err(|error| RunError::Staging(error.to_string()))?;
                Box::new(crate::benchmark::deepswe::DeepSweAdapter::from_profile(
                    profile,
                ))
            }
        };
        let mut env: BTreeMap<std::ffi::OsString, std::ffi::OsString> = BTreeMap::new();
        if let Some(report) = &fixture_report {
            env.insert(
                "OPI_EVAL_NATIVE_SOURCE".into(),
                report.clone().into_os_string(),
            );
            env.insert(
                "OPI_EVAL_RUN_BEHAVIOR".into(),
                request.behavior.clone().into(),
            );
        }
        let benchmark_request = BenchmarkRunRequest {
            verifier_executable: verifier_helper,
            task_dir,
            task_id: declared.task.clone(),
            native_selection: None,
            agent_timeout_seconds: None,
            verifier_timeout_seconds: None,
            trace_root: verifier_trace.clone(),
            // Hermetic staging pins a per-trial fixture digest; the native
            // mode binds the admitted static external lock instead.
            admitted_lock_digest: crate::agent::opi::sha256_hex(declared.id.as_bytes()),
            integrity: integrity.clone(),
            extra_env: env,
        };
        let record = BenchmarkExecution::run(
            &benchmark_request,
            benchmark_request_adapter.as_ref(),
            &run_cancel.child_token(),
        )
        .await;
        match record {
            Ok(record) => {
                if let BenchmarkCompletion::Failed(failure) = &record.completion {
                    ledger.fail(failure.boundary);
                }
                // Native verifier evidence into the staging bundle, under
                // the grader's source identity.
                produced_keys.push(stage(
                    &mut bundle,
                    &grader_source,
                    ArtifactRole::Native,
                    "native/verifier-stdout.log",
                    &record.stdout.bytes,
                )?);
                produced_keys.push(stage(
                    &mut bundle,
                    &grader_source,
                    ArtifactRole::Native,
                    "native/verifier-stderr.log",
                    &record.stderr.bytes,
                )?);
                staged_keys.extend(produced_keys[produced_keys.len() - 2..].to_vec());
                if let BenchmarkCompletion::Verified { artifacts, .. } = &record.completion {
                    for artifact in artifacts {
                        let bytes = std::fs::read(&artifact.path).map_err(|error| {
                            RunError::Staging(format!(
                                "trial {}: verifier artifact {} is unreadable: {error}",
                                declared.id, artifact.role
                            ))
                        })?;
                        produced_keys.push(stage(
                            &mut bundle,
                            &grader_source,
                            ArtifactRole::Native,
                            &format!("native/{}", artifact.role),
                            &bytes,
                        )?);
                        staged_keys.push(produced_keys.last().cloned().expect("just pushed"));
                    }
                }
                verifier_record = Some(record);
            }
            Err(rejection) => {
                // Pre-spawn admission refusal: no verifier process ever
                // existed. The owning token is persisted, never retried
                // against another revision or grader.
                ledger.fail(FailureBoundaryCode::Integrity);
                verifier_rejection = Some(rejection.token.to_owned());
            }
        }
    }

    // Pre-seal projection over the settled facts. The
    // projection itself fails closed if the ladder ever passed settlement,
    // so no phase guard is needed here.
    let projection = match &verifier_record {
        Some(benchmark) => ProjectionPipeline::project(&TrialInputs {
            agent: &agent_record,
            benchmark: Some(benchmark),
            lifecycle: &lifecycle,
            workspace: Some(&workspace),
        })
        .ok(),
        None => ProjectionPipeline::project(&TrialInputs {
            agent: &agent_record,
            benchmark: None,
            lifecycle: &lifecycle,
            workspace: Some(&workspace),
        })
        .ok(),
    };

    // Pre-seal redaction gate (`EVAL-A18`, `EVAL-SEC-005`): a declared
    // canary anywhere in the staged exportable content blocks sealing, so
    // the leak never enters a published manifest. The refusal is the
    // evidence boundary and the bundle stays unsealed on disk.
    //
    // The projection is sealed trial evidence: the trajectory projection
    // itself enters the bundle under the reserved evidence key. A failed
    // projection leaves the key unstaged, so sealing refuses the broken
    // reservation instead of sealing without the trajectory.
    if let Some(trajectory) = &projection {
        staged_keys.push(stage(
            &mut bundle,
            &runner_source,
            ArtifactRole::Derived,
            TRAJECTORY_KEY,
            &trajectory.canonical_bytes(),
        )?);
    }

    // Trial facts for pairing and reporting: the classification is
    // computed once here from the settled records and sealed as the
    // durable settlement record. A scored Agent failure whose native
    // grade verified stays an Agent outcome in the graded denominator;
    // infrastructure and grader failures stay attempted-but-invalid.
    let agent_failed = matches!(agent_record.completion, AgentCompletion::Failed(_));
    let scored_agent_failure = agent_failure_class == Some(AgentFailureClass::ScoredAgentOutcome);
    let verifier_failed = verifier_rejection.is_some()
        || verifier_record
            .as_ref()
            .is_some_and(|record| matches!(record.completion, BenchmarkCompletion::Failed(_)));
    let timed_out = exit_state_token(&agent_record.exit) == "timed-out";
    let trial_outcome = if agent_failed && !scored_agent_failure {
        TrialOutcome::InfrastructureFailure
    } else if verifier_failed {
        TrialOutcome::GraderFailure
    } else if agent_failed {
        // A scored Agent failure whose native grade verified stays an
        // Agent outcome in the graded denominator.
        TrialOutcome::AgentFailure
    } else {
        TrialOutcome::ValidAgentOutcome
    };
    let settlement = json!({
        "schema": "opi-eval-trial-settlement/1",
        "trial": declared.id,
        "graded": matches!(
            trial_outcome,
            TrialOutcome::ValidAgentOutcome | TrialOutcome::AgentFailure
        ),
        "agent_failure": trial_outcome == TrialOutcome::AgentFailure,
        "timed_out": timed_out,
        "failure_kind": agent_failure_kind(&agent_record.completion)
            .or_else(|| verifier_record.as_ref().and_then(benchmark_failure_kind)),
        "boundary": agent_failure_boundary(&agent_record.completion)
            .or_else(|| verifier_record.as_ref().and_then(benchmark_failure_boundary)),
    });
    staged_keys.push(stage(
        &mut bundle,
        &runner_source,
        ArtifactRole::Derived,
        CONTROL_SETTLEMENT_KEY,
        &serde_json::to_vec(&settlement).map_err(|error| RunError::Staging(error.to_string()))?,
    )?);

    // The authority ledger itself is durable evidence: its execution
    // counts enter the sealed bundle (EVAL-FAL-002 call-count proof). The
    // sealed copy honestly covers every transition up to sealing; the
    // seal and report states live only in the outer receipt.
    {
        let ledger_json =
            serde_json::to_vec(&ledger).map_err(|error| RunError::Staging(error.to_string()))?;
        staged_keys.push(stage(
            &mut bundle,
            &runner_source,
            ArtifactRole::Native,
            AUTHORITY_LEDGER_KEY,
            &ledger_json,
        )?);
    }

    let canary_leak =
        scan_staged_for_canaries(&trial_root.join("bundle"), &staged_keys, &request.canaries);
    let seal_outcome = if canary_leak.is_some() {
        ledger.attempt_failed(AuthorityTransition::Seal, FailureBoundaryCode::Evidence);
        None
    } else if ledger.attempt(AuthorityTransition::Seal) {
        if request.behavior == "seal-failure" {
            // Hermetic seal-failure simulation: a staged artifact is
            // tampered between staging and sealing, so canonical sealing
            // fails its digest validation exactly as real drift would.
            let tamper = trial_root.join("bundle/artifacts/native/agent-stdout.log");
            std::fs::write(&tamper, b"tampered\n")
                .map_err(|error| RunError::Staging(error.to_string()))?;
        }
        match bundle.seal(&produced_keys) {
            Ok(receipt) => {
                lifecycle
                    .mark_sealed()
                    .map_err(|error| RunError::Bundle(error.to_string()))?;
                Some(receipt)
            }
            Err(error) => {
                ledger.attempt_failed(AuthorityTransition::Seal, error.boundary());
                None
            }
        }
    } else {
        None
    };

    // Trial facts for pairing: a sealed graded trial contributes its
    // outcome class; failures contribute their failure class so coverage
    // shows the exact reason instead of hiding the trial.
    let fact = if seal_outcome.is_some() {
        // Honest per-profile control realization: neither pinned agent
        // profile expresses a reasoning launch control, so a shared
        // reasoning value is unsupported by every subject and the pair
        // must stay non-comparable (EVAL-EXP-008).
        let unsupported_controls = match experiment.model_controls().reasoning {
            crate::experiment::ControlValue::Value(_) => vec!["reasoning".to_owned()],
            _ => Vec::new(),
        };
        Some(TrialFact {
            id: declared.id.clone(),
            subject: declared.subject.clone(),
            task: declared.task.clone(),
            group: declared.group.clone(),
            manifest_digest: experiment.manifest_digest().to_owned(),
            control_fingerprint: control_fingerprint(experiment),
            unsupported_controls,
            outcome: trial_outcome,
        })
    } else {
        None
    };

    let report_executed = ledger.attempt(AuthorityTransition::Report);
    let mut receipt = json!({
        "schema": TRIAL_RECEIPT_SCHEMA,
        "id": declared.id,
        "subject": declared.subject,
        "task": declared.task,
        "group": declared.group,
        "status": if seal_outcome.is_some() { "sealed" } else { "failed" },
        "agent": {
            "product": product,
            "exit_state": exit_state_token(&agent_record.exit),
            "completion": if agent_failed { "failed" } else { "completed" },
            "failure_kind": agent_failure_kind(&agent_record.completion),
            "boundary": agent_failure_boundary(&agent_record.completion),
            "stdout_truncated": agent_record.stdout.truncated,
            "stderr_truncated": agent_record.stderr.truncated,
            "cleanup": cleanup_token(&agent_record.cleanup),
            "stdout_bytes": agent_record.stdout.bytes.len(),
            "stderr_bytes": agent_record.stderr.bytes.len(),
        },
        "verifier": verifier_rejection.as_ref().map(|token| {
            json!({
                "rejected": token,
                "boundary": "integrity",
            })
        }).or_else(|| verifier_record.as_ref().map(|record| {
            json!({
                "exit_state": exit_state_token(&record.exit),
                "reward": fact_token(&record.reward),
                "cleanup": cleanup_token(&record.cleanup),
                "completion": match &record.completion {
                    BenchmarkCompletion::Verified { .. } => "verified",
                    BenchmarkCompletion::VerifiedNative { .. } => "verified-native",
                    BenchmarkCompletion::Failed(_) => "failed",
                },
                "failure_kind": benchmark_failure_kind(record),
                "boundary": benchmark_failure_boundary(record),
            })
        })),
        "authority": authority_json(&ledger),
    });
    let status = if seal_outcome.is_some() {
        TrialStatus::Sealed
    } else {
        TrialStatus::Failed
    };
    if let Some(sealed) = &seal_outcome {
        receipt["bundle_identity"] = json!(sealed.bundle_identity());
    }
    if let Some(trajectory) = &projection {
        let mut trajectory_receipt =
            crate::trajectory::TrajectoryReceipt::for_trajectory(trajectory);
        if let Some(sealed) = &seal_outcome {
            trajectory_receipt.record_seal(crate::trajectory::SealOutcome::Sealed {
                bundle_digest: sealed.bundle_identity().to_owned(),
            });
        }
        receipt["pre_seal_digest"] = json!(trajectory_receipt.pre_seal_digest);
        receipt["seal_result"] = match &trajectory_receipt.seal_result {
            Some(crate::trajectory::SealOutcome::Sealed { bundle_digest }) => {
                json!({"sealed": {"bundle_digest": bundle_digest}})
            }
            Some(crate::trajectory::SealOutcome::SealFailed { reason }) => {
                json!({"seal_failed": {"reason": reason}})
            }
            None => serde_json::Value::Null,
        };
    }

    if report_executed {
        let receipt_path = trial_root.join("receipt.json");
        std::fs::write(
            &receipt_path,
            serde_json::to_vec(&receipt).map_err(|error| RunError::Staging(error.to_string()))?,
        )
        .map_err(|error| RunError::Staging(error.to_string()))?;
        lifecycle
            .mark_graded()
            .and_then(|()| lifecycle.mark_reported())
            .map_err(|error| RunError::Bundle(error.to_string()))?;
    }

    Ok(TrialResult {
        receipt,
        fact,
        sealed: status == TrialStatus::Sealed,
    })
}

/// Stage one artifact into `bundle` and return its key.
fn stage(
    bundle: &mut RunBundle,
    source: &SourceIdentity,
    role: ArtifactRole,
    key: &str,
    bytes: &[u8],
) -> Result<ArtifactKey, RunError> {
    let key = ArtifactKey::new(key).map_err(|error| RunError::Staging(error.to_string()))?;
    bundle
        .insert(
            ArtifactSpec {
                role,
                source: source.clone(),
                path: key.clone(),
                bytes: bytes.to_vec(),
                classification: Sensitivity::Exportable,
            },
            Vec::new(),
        )
        .map_err(|error| RunError::Bundle(error.to_string()))?;
    Ok(key)
}

/// Resolve the concrete agent adapter for one subject product.
fn agent_adapter(product: &str, behavior: &str) -> Result<Box<dyn AgentAdapter>, RunError> {
    let _ = behavior;
    match product {
        "opi" => Ok(Box::new(OpiProcessAdapter::new())),
        "pi" => Ok(Box::new(PiProcessAdapter::new())),
        other => Err(RunError::UnsupportedSubject {
            product: other.to_owned(),
        }),
    }
}

/// The bounded-timeout variant of a pinned agent profile for the
/// timeout-race behavior: the same pinned identity and launch surface with
/// only the supervisor timeout lowered. Fails closed when the pinned
/// timeout line is absent.
fn timeout_patched_adapter(product: &str) -> Result<Box<dyn AgentAdapter>, RunError> {
    let text = if product == "opi" {
        include_str!("../../profiles/agents/opi.toml")
    } else {
        include_str!("../../profiles/agents/pi.toml")
    };
    let patched = text
        .replace("timeout_secs = 900", "timeout_secs = 2")
        .replace("timeout_secs = 1800", "timeout_secs = 2");
    if patched == text {
        return Err(RunError::Staging(
            "pinned timeout line not found for the timeout variant".to_owned(),
        ));
    }
    match product {
        "opi" => {
            let profile = crate::agent::opi::OpiProfile::parse(&patched)
                .map_err(|error| RunError::Staging(error.to_string()))?;
            Ok(Box::new(OpiProcessAdapter::from_profile(profile)))
        }
        "pi" => {
            let profile = crate::agent::pi::PiProfile::parse(&patched)
                .map_err(|error| RunError::Staging(error.to_string()))?;
            Ok(Box::new(PiProcessAdapter::from_profile(profile)))
        }
        other => Err(RunError::UnsupportedSubject {
            product: other.to_owned(),
        }),
    }
}

/// Native-source environment for the agent helper.
fn agent_env(
    product: &str,
    fixtures: &Path,
    behavior: &str,
) -> Result<BTreeMap<std::ffi::OsString, std::ffi::OsString>, RunError> {
    let mut env: BTreeMap<std::ffi::OsString, std::ffi::OsString> = BTreeMap::new();
    let agents = fixtures.join("agents");
    // The saved-bytes fixture each stream-contract behavior replays.
    let trace_source = match behavior {
        "agent-unknown-schema" => "trace-unknown-schema",
        "agent-malformed-stream" => "trace-corrupt",
        "agent-missing-terminal" => "trace-missing-evidence",
        _ => "trace-complete",
    };
    let stream_source = match behavior {
        "agent-unknown-schema" => "unknown-event.jsonl",
        "agent-malformed-stream" => "corrupt.jsonl",
        "agent-missing-terminal" => "no-agent-end.jsonl",
        _ => "stream-ok.jsonl",
    };
    if behavior == "canary-leak" {
        env.insert("OPI_EVAL_CANARY_SECRET".into(), HERMETIC_CANARY.into());
    }
    if product == "opi" {
        env.insert(
            "OPI_EVAL_TRACE_SOURCE".into(),
            agents.join("opi").join(trace_source).into_os_string(),
        );
    } else {
        env.insert(
            "PI_EVAL_STREAM_SOURCE".into(),
            agents.join("pi").join(stream_source).into_os_string(),
        );
    }
    env.insert("OPI_EVAL_RUN_BEHAVIOR".into(), behavior.to_owned().into());
    Ok(env)
}

fn exit_state_token(exit: &ExitState) -> String {
    match exit {
        ExitState::Exited { code } => format!("exited:{code}"),
        ExitState::TimedOut => "timed-out".to_owned(),
        ExitState::Cancelled => "cancelled".to_owned(),
        ExitState::FailedToSpawn { .. } => "failed-to-spawn".to_owned(),
    }
}

/// Copies bounded private capture after owned writers have been removed.
/// Raw upstream bytes stay unchanged. Links and special files are skipped. A
/// traversal that exceeds its fixed work budget fails visibly, retaining the
/// interrupted outcome rather than claiming bounded evidence or a grade.
fn bound_partial_capture(
    trace: &Path,
    receipt: &Path,
    full: bool,
    stale: &[std::ffi::OsString],
) -> std::io::Result<()> {
    const FILE_BYTES: u64 = 1024 * 1024;
    const TOTAL_BYTES: u64 = 16 * FILE_BYTES;
    const FILES: usize = 256;
    const ENTRIES: usize = 4096;
    use std::io::Read;
    let destination = receipt.with_extension("");
    let mut directories = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        directories.mode(0o700);
    }
    directories.create(&destination)?;
    directories.recursive(true);
    let mut visited = 0;
    let mut files = 0;
    let mut bytes = 0;
    let mut truncated = 0;
    let mut discarded = 0;
    let invalid = || std::io::Error::other("partial capture traversal limit or unsafe root");
    let mut children = |root: &Path| -> std::io::Result<Vec<PathBuf>> {
        match std::fs::symlink_metadata(root) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Ok(metadata) if metadata.is_dir() && !metadata.is_symlink() => {}
            _ => return Err(invalid()),
        }
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(root)? {
            visited += 1;
            if visited > ENTRIES {
                return Err(invalid());
            }
            paths.push(entry?.path());
        }
        paths.sort();
        Ok(paths)
    };
    let mut pending = Vec::new();
    for job in children(&trace.join("jobs"))? {
        if job
            .file_name()
            .is_some_and(|name| stale.iter().any(|old| old == name))
        {
            continue;
        }
        if !std::fs::symlink_metadata(&job)?.is_dir() {
            continue;
        }
        for trial in children(&job)? {
            if !std::fs::symlink_metadata(&trial)?.is_dir() {
                continue;
            }
            let agent = trial.join("agent");
            for path in children(&agent)? {
                if matches!(
                    path.file_name().and_then(|n| n.to_str()),
                    Some("stdout.log" | "stderr.log" | "trace")
                ) {
                    pending.push((path, 0));
                }
            }
        }
    }
    // Streams precede trace files so an excessive trace cannot crowd them out.
    pending.sort_by(|a, b| b.0.cmp(&a.0));
    while let Some((path, depth)) = pending.pop() {
        let metadata = std::fs::symlink_metadata(&path)?;
        if metadata.is_dir() && !metadata.is_symlink() {
            if depth >= 16 {
                return Err(invalid());
            }
            let mut entries = children(&path)?;
            entries.reverse();
            pending.extend(entries.into_iter().map(|path| (path, depth + 1)));
            continue;
        }
        #[cfg(unix)]
        let single_link = {
            use std::os::unix::fs::MetadataExt;
            metadata.nlink() == 1
        };
        #[cfg(not(unix))]
        let single_link = false; // Native capture uses the admitted Linux runtime.
        if !full
            || !metadata.is_file()
            || metadata.is_symlink()
            || !single_link
            || files >= FILES
            || bytes >= TOTAL_BYTES
        {
            discarded += 1;
            continue;
        }
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let file = options.open(&path)?;
        let opened = file.metadata()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if !opened.is_file()
                || opened.nlink() != 1
                || opened.ino() != metadata.ino()
                || opened.dev() != metadata.dev()
            {
                return Err(invalid());
            }
        }
        let retained = opened.len().min(FILE_BYTES).min(TOTAL_BYTES - bytes);
        if retained < metadata.len() {
            truncated += 1;
        }
        let target = destination.join(path.strip_prefix(trace).map_err(|_| invalid())?);
        directories.create(target.parent().ok_or_else(invalid)?)?;
        let mut output = std::fs::OpenOptions::new();
        output.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            output.mode(0o600);
        }
        let copied = std::io::copy(&mut file.take(retained), &mut output.open(&target)?)?;
        files += 1;
        bytes += copied;
    }
    std::fs::write(
        receipt,
        serde_json::to_vec(&json!({
            "schema": "opi-eval-partial-capture/1",
            "sensitivity": "private-unredacted",
            "files": files, "bytes": bytes,
            "truncated_files": truncated, "discarded_entries": discarded,
            "file_byte_limit": FILE_BYTES, "total_byte_limit": TOTAL_BYTES,
            "file_limit": FILES,
        }))
        .expect("capture counts serialize"),
    )
}

/// The guard also removes partially written handoffs on pre-dispatch errors.
struct NativeHandoff(PathBuf);

impl Drop for NativeHandoff {
    fn drop(&mut self) {
        remove_credential_material(&self.0);
    }
}

/// Remove actual connection projections on every settlement, without reading
/// secret bytes or trusting filenames supplied by the integration process.
fn remove_credential_material(handoff_path: &Path) -> bool {
    let Some(directory) = handoff_path.parent() else {
        return true;
    };
    let mut absent = true;
    for path in [
        handoff_path.to_path_buf(),
        directory.join("config.toml"),
        directory.join("models.json"),
    ] {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => absent = false,
        }
    }
    absent
}

fn owned_cleanup_receipt(cleanup: &OwnedCleanup, credential_material: &str) -> String {
    format!(
        "+owned-containers:{}+credential-material:{credential_material}",
        cleanup.token()
    )
}

fn cleanup_token(cleanup: &crate::process::CleanupEvidence) -> String {
    match cleanup {
        crate::process::CleanupEvidence::NotRequired => "not-required".to_owned(),
        crate::process::CleanupEvidence::TreeTerminated { layer, verified } => {
            format!(
                "tree-terminated:{layer}:{}",
                if *verified { "verified" } else { "unverified" }
            )
        }
        crate::process::CleanupEvidence::TreeTerminationFailed { layer } => {
            format!("tree-termination-failed:{layer}")
        }
    }
}

fn fact_token(fact: &Fact) -> String {
    match fact {
        Fact::Known { value, origin } => format!("known:{value}({origin})"),
        Fact::Unknown { reason } => format!("unknown:{reason}"),
    }
}

fn agent_failure_kind(completion: &AgentCompletion) -> Option<String> {
    match completion {
        AgentCompletion::Failed(failure) => Some(failure.kind.to_owned()),
        AgentCompletion::Completed { .. } => None,
    }
}

fn agent_failure_boundary(completion: &AgentCompletion) -> Option<String> {
    match completion {
        AgentCompletion::Failed(failure) => Some(boundary_token(failure.boundary).to_owned()),
        AgentCompletion::Completed { .. } => None,
    }
}

fn benchmark_failure_kind(record: &crate::benchmark::process::BenchmarkRecord) -> Option<String> {
    match &record.completion {
        BenchmarkCompletion::Failed(failure) => Some(failure.kind.to_owned()),
        BenchmarkCompletion::Verified { .. } | BenchmarkCompletion::VerifiedNative { .. } => None,
    }
}

fn benchmark_failure_boundary(
    record: &crate::benchmark::process::BenchmarkRecord,
) -> Option<String> {
    match &record.completion {
        BenchmarkCompletion::Failed(failure) => Some(boundary_token(failure.boundary).to_owned()),
        BenchmarkCompletion::Verified { .. } | BenchmarkCompletion::VerifiedNative { .. } => None,
    }
}

fn authority_json(ledger: &AuthorityLedger) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for record in ledger.records() {
        map.insert(record.transition.token().to_owned(), json!(record.state));
    }
    serde_json::Value::Object(map)
}

fn comparability_token(comparability: &crate::comparison::Comparability) -> String {
    match comparability {
        crate::comparison::Comparability::Comparable => "comparable".to_owned(),
        crate::comparison::Comparability::NonComparable(reason) => non_comparability_token(reason),
    }
}

fn non_comparability_token(reason: &crate::comparison::NonComparability) -> String {
    use crate::comparison::NonComparability as N;
    match reason {
        N::MissingBaselineTrial => "missing-baseline-trial".to_owned(),
        N::MissingCandidateTrial => "missing-candidate-trial".to_owned(),
        N::ControlMismatch { .. } => "control-mismatch".to_owned(),
        N::UnsupportedControl { control } => format!("unsupported-control:{control}"),
        N::Excluded { trial, reason } => format!("excluded:{trial}:{reason}"),
        N::InfrastructureFailure { trial } => format!("infrastructure-failure:{trial}"),
        N::GraderFailure { trial } => format!("grader-failure:{trial}"),
        N::InvalidTaskClassification { .. } => "invalid-task-classification".to_owned(),
        N::TaskNotCovered => "task-not-covered".to_owned(),
        N::ProtocolGroupMismatch => "protocol-group-mismatch".to_owned(),
    }
}

/// Resolve the unique comparison edge owning one declared trial: the edge
/// over the trial's subject whose counterpart subject declares a trial in
/// the same task and group (`EVAL-EXP-002`). Zero or multiple owners reject
/// the trial before any process effect or intent publication, so no
/// durable pair identity can default to an unrelated edge.
/// The durable pair-identity seed for one trial: the owning edge id, or
/// for a single-subject experiment (no artificial edge, P19-D05) the
/// solo subject identity itself.
fn owning_pair_seed(
    experiment: &ResolvedExperiment,
    declared: &crate::experiment::DeclaredTrial,
) -> Result<String, RunError> {
    if experiment.edges().is_empty() && experiment.subjects().len() == 1 {
        return Ok(format!("solo:{}", declared.subject));
    }
    Ok(owning_edge(experiment, declared)?.id.clone())
}

fn owning_edge<'a>(
    experiment: &'a ResolvedExperiment,
    declared: &crate::experiment::DeclaredTrial,
) -> Result<&'a crate::experiment::ComparisonEdge, RunError> {
    let mut owners = experiment.edges().iter().filter(|edge| {
        let counterpart = if edge.baseline == declared.subject {
            &edge.candidate
        } else if edge.candidate == declared.subject {
            &edge.baseline
        } else {
            return false;
        };
        experiment.trials().iter().any(|trial| {
            trial.subject == *counterpart
                && trial.task == declared.task
                && trial.group == declared.group
        })
    });
    let Some(first) = owners.next() else {
        return Err(RunError::Staging(format!(
            "trial {} (subject {}, task {}, group {}) has no owning comparison edge",
            declared.id, declared.subject, declared.task, declared.group
        )));
    };
    if owners.next().is_some() {
        return Err(RunError::Staging(format!(
            "trial {} matches multiple comparison edges for one task and group",
            declared.id
        )));
    }
    Ok(first)
}

/// Derive a fresh attempt from the verified, expanded original contract.
/// Selection metadata and every non-identity control remain frozen.
fn replace_group(
    source: &str,
    original: &ResolvedExperiment,
    crashed_trial: &str,
    root: &Path,
) -> Result<ResolvedExperiment, RunError> {
    let Some(crashed) = original.trials().iter().find(|t| t.id == crashed_trial) else {
        return Err(RunError::Staging(format!(
            "replacement target {crashed_trial:?} is not a resolved trial"
        )));
    };
    let group = &crashed.group;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| RunError::Staging(error.to_string()))?;
    let token = crate::agent::opi::sha256_hex(
        format!(
            "{}:{}:{}:{}",
            now.as_nanos(),
            std::process::id(),
            root.display(),
            original.manifest_digest()
        )
        .as_bytes(),
    );
    let mut trials = original.trials().to_vec();
    for trial in &mut trials {
        // Every trial the replacement actually re-executes takes a fresh
        // identity (CTRL-006), including retained unrelated groups; only
        // the targeted comparison group carries the attempt suffix, so
        // unrelated groups keep their frozen group identities while the
        // whole declared scope stays dispatched.
        trial.id = format!("{}.r{token}", trial.id);
        if trial.group == *group {
            trial.group = format!("{}.r{token}", trial.group);
        }
    }
    trials.sort_by(|left, right| left.id.cmp(&right.id));
    let mut document: toml::Value =
        toml::from_str(source).map_err(|error| RunError::Staging(error.to_string()))?;
    document["experiment_id"] =
        toml::Value::String(format!("{}.r{token}", original.experiment_id()));
    if original.selection().is_none() {
        document["trials"] = toml::Value::Array(
            trials
                .iter()
                .map(|trial| {
                    toml::Value::try_from(trial).expect("declared trials serialize as TOML tables")
                })
                .collect(),
        );
    }
    let derived = ResolvedExperiment::resolve(
        &toml::to_string(&document).map_err(|error| RunError::Staging(error.to_string()))?,
    )?;
    Ok(derived.replace_trials(trials))
}

/// Classify every existing trial root under `root` from its durable files
/// only. Effect-unknown trials keep their original identity and boundary
/// and are never reclassified as not-started (EVAL-DUR-002).
fn recovery_report(
    experiment: &ResolvedExperiment,
    root: &Path,
    plan: Option<&serde_json::Value>,
) -> Result<serde_json::Value, RunError> {
    let control = match std::fs::read(root.join("run-control.json")) {
        Ok(bytes) => {
            let value: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|error| RunError::Bundle(error.to_string()))?;
            if value["schema"] != "opi-eval-run-control/1"
                || value["experiment"] != experiment.experiment_id()
                || value["manifest_digest"] != experiment.manifest_digest()
            {
                return Err(RunError::Bundle(
                    "recovery run-control identity drift".to_owned(),
                ));
            }
            Some(value)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let inventory = control
        .as_ref()
        .map(|value| &value["planned_trials"])
        .or_else(|| plan.map(|value| &value["trials"]));
    let mut planned = Vec::new();
    if let Some(inventory) = inventory {
        let entries = inventory
            .as_array()
            .ok_or_else(|| RunError::Bundle("recovery planned inventory missing".to_owned()))?;
        for entry in entries {
            let field = |name: &str| {
                entry[name]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .ok_or_else(|| RunError::Bundle(format!("recovery trial lacks {name}")))
            };
            let trial = crate::experiment::DeclaredTrial {
                id: field("id")?,
                subject: field("subject")?,
                task: field("task")?,
                group: field("group")?,
            };
            TrialIdentity::new(&trial.id).map_err(|error| RunError::Bundle(error.to_string()))?;
            if !experiment
                .subjects()
                .iter()
                .any(|subject| subject.id == trial.subject)
                || planned
                    .iter()
                    .any(|prior: &crate::experiment::DeclaredTrial| prior.id == trial.id)
            {
                return Err(RunError::Bundle(
                    "recovery planned trial identity drift".to_owned(),
                ));
            }
            planned.push(trial);
        }
        planned.sort_by(|a, b| a.id.cmp(&b.id));
        if experiment.selection().is_none() && planned != experiment.trials() {
            return Err(RunError::Bundle(
                "recovery inventory differs from resolved trials".to_owned(),
            ));
        }
        if let Some(plan) = plan {
            let frozen: Vec<_> = planned
                .iter()
                .map(
                    |t| json!({"id": t.id, "subject": t.subject, "task": t.task, "group": t.group}),
                )
                .collect();
            let recorded: Vec<_> = plan["trials"].as_array().ok_or_else(|| RunError::Bundle("recovery plan trials missing".to_owned()))?
                .iter().map(|t| json!({"id": t["id"], "subject": t["subject"], "task": t["task"], "group": t["group"]})).collect();
            if frozen != recorded {
                return Err(RunError::Bundle(
                    "recovery inventory differs from frozen plan".to_owned(),
                ));
            }
        }
    } else {
        planned = experiment.trials().to_vec();
        if planned.is_empty() {
            return Err(RunError::Bundle(
                "recovery of selection requires its frozen planned inventory".to_owned(),
            ));
        }
    }
    let experiment = experiment.clone().replace_trials(planned);
    let mut recovery = Vec::new();
    let mut effect_unknown = 0;
    let trials_root = root.join("trials");
    if trials_root.exists() {
        for entry in std::fs::read_dir(&trials_root)? {
            let entry = entry?;
            if entry.path().is_dir()
                && !experiment
                    .trials()
                    .iter()
                    .any(|trial| entry.file_name() == std::ffi::OsStr::new(&trial.id))
            {
                return Err(RunError::Bundle(
                    "recovery found an unplanned trial root".to_owned(),
                ));
            }
        }
    }
    for trial in experiment.trials() {
        let id = &trial.id;
        let observed = RunBundle::recover(&trials_root.join(id).join("bundle"));
        if let Ok(observed) = &observed {
            if observed
                .intent
                .as_ref()
                .is_some_and(|intent| intent.trial.as_str() != id)
                || observed
                    .settlement
                    .as_ref()
                    .is_some_and(|marker| marker.trial.as_str() != id)
            {
                return Err(RunError::Bundle(
                    "recovery durable trial identity drift".to_owned(),
                ));
            }
            if observed.sealed {
                RunBundle::verify(&trials_root.join(id).join("bundle"))
                    .map_err(|error| RunError::Bundle(error.to_string()))?;
            }
        }
        let mut row = match observed {
            Ok(observed) => match TrialLifecycle::recover(&observed) {
                crate::runner::lifecycle::RecoveryClassification::NotStarted => {
                    json!({"id": id, "status": "not-started"})
                }
                crate::runner::lifecycle::RecoveryClassification::EffectUnknown {
                    trial,
                    boundary,
                } => {
                    effect_unknown += 1;
                    json!({
                        "id": id,
                        "trial": trial.as_str(),
                        "status": "effect-unknown",
                        "boundary": boundary_token(boundary),
                    })
                }
                crate::runner::lifecycle::RecoveryClassification::SettledUnsealed => {
                    json!({"id": id, "status": "settled-unsealed"})
                }
                crate::runner::lifecycle::RecoveryClassification::Sealed => {
                    json!({"id": id, "status": "sealed"})
                }
            },
            Err(error) => json!({"id": id, "status": "unreadable", "error": error.to_string()}),
        };
        let settlement_path = trials_root
            .join(id)
            .join("bundle/artifacts")
            .join(CONTROL_SETTLEMENT_KEY);
        if settlement_path.is_file() {
            let settlement: serde_json::Value =
                serde_json::from_slice(&std::fs::read(settlement_path)?)
                    .map_err(|error| RunError::Bundle(error.to_string()))?;
            if settlement["trial"] != *id {
                return Err(RunError::Bundle(
                    "recovery settlement identity drift".to_owned(),
                ));
            }
            row["settlement"] = settlement;
        }
        recovery.push(row);
    }
    // Pairing visibility for the recovery view: for every edge, the sides
    // whose subjects have no durably settled trial root are named exactly.
    // No fact is reconstructed from disk - recovery reports durable state
    // only.
    let settled_ids: std::collections::BTreeSet<&str> = recovery
        .iter()
        .filter(|row| row["status"] == "sealed" || row["status"] == "settled-unsealed")
        .map(|row| row["id"].as_str().unwrap_or_default())
        .collect();
    let pairs: Vec<serde_json::Value> = experiment
        .edges()
        .iter()
        .map(|edge| {
            let mut missing = Vec::new();
            for (role, subject) in [("baseline", &edge.baseline), ("candidate", &edge.candidate)] {
                let settled = experiment.trials().iter().any(|trial| {
                    trial.subject == *subject && settled_ids.contains(trial.id.as_str())
                });
                if !settled {
                    missing.push(role);
                }
            }
            let comparability = if missing.contains(&"baseline") {
                "missing-baseline-trial"
            } else if missing.contains(&"candidate") {
                "missing-candidate-trial"
            } else {
                "settled-unsealed"
            };
            json!({
                "edge": edge.id,
                "missing_sides": missing,
                "comparability": comparability,
            })
        })
        .collect();
    let _ = &mut effect_unknown;
    Ok(json!({
        "schema": RUN_REPORT_SCHEMA,
        "experiment": experiment.experiment_id(),
        "manifest_digest": experiment.manifest_digest(),
        "outcome": "incomplete",
        "recovery": recovery,
        "trials": [],
        "pairs": pairs,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn partial_capture_limits_and_policy_preserve_raw_evidence() {
        for (count, size, full, expected_files, expected_bytes) in [
            (20, 2 * 1024 * 1024, true, 16, 16 * 1024 * 1024),
            (270, 0, true, 256, 0),
            (2, 32, false, 0, 0),
        ] {
            let root = tempfile::tempdir().unwrap();
            let trace = root.path().join("upstream");
            let source = trace.join("jobs/new/trial/agent/trace/run");
            std::fs::create_dir_all(&source).unwrap();
            for n in 0..count {
                let file = std::fs::File::create(source.join(format!("{n:03}.jsonl"))).unwrap();
                file.set_len(size).unwrap();
            }
            let stale = trace.join("jobs/old/trial/agent");
            std::fs::create_dir_all(&stale).unwrap();
            std::fs::write(stale.join("stdout.log"), "excluded prior evidence").unwrap();
            let receipt = root.path().join("partial.json");
            bound_partial_capture(&trace, &receipt, full, &["old".into()]).unwrap();
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(receipt).unwrap()).unwrap();
            assert_eq!(value["files"], expected_files);
            assert_eq!(value["bytes"], expected_bytes);
            assert!(!root.path().join("partial/jobs/old").exists());
            for n in 0..count {
                assert_eq!(
                    std::fs::metadata(source.join(format!("{n:03}.jsonl")))
                        .unwrap()
                        .len(),
                    size
                );
            }
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(root.path().join("partial"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn partial_capture_rejects_linked_roots_and_bounds_traversal() {
        for kind in ["linked-agent", "excess-entries", "excess-depth"] {
            let root = tempfile::tempdir().unwrap();
            let trace = root.path().join("upstream");
            let source = trace.join("jobs/new/trial/agent");
            std::fs::create_dir_all(source.parent().unwrap()).unwrap();
            if kind == "linked-agent" {
                let outside = root.path().join("outside");
                std::fs::create_dir(&outside).unwrap();
                std::fs::write(outside.join("stdout.log"), "private-neighbor").unwrap();
                std::os::unix::fs::symlink(outside, &source).unwrap();
            } else {
                std::fs::create_dir_all(source.join("trace")).unwrap();
                if kind == "excess-entries" {
                    for n in 0..4096 {
                        std::fs::write(source.join("trace").join(n.to_string()), "").unwrap();
                    }
                } else {
                    let deep = (0..17).fold(source.join("trace"), |p, _| p.join("deeper"));
                    std::fs::create_dir_all(deep).unwrap();
                }
            }
            let receipt = root.path().join("partial.json");
            assert!(
                bound_partial_capture(&trace, &receipt, true, &[]).is_err(),
                "{kind}"
            );
            assert!(
                !receipt.exists(),
                "no false completed capture receipt: {kind}"
            );
        }
    }

    #[test]
    fn fixture_experiment_pins_the_derived_integrity_identity() {
        // (document, staging behavior) pairs and the review they derive.
        let cases: &[(&str, &str)] = &[
            ("local-paired.toml", "happy"),
            ("duplicate-pair.toml", "happy"),
            ("unsupported-control.toml", "happy"),
            ("terminal-bench-3.0.toml", "happy"),
            ("deepswe.toml", "happy"),
            ("multi-edge.toml", "happy"),
            ("integrity-exclusion.toml", "integrity-exclusion"),
            ("invalid-task.toml", "invalid-task"),
        ];
        for (name, behavior) in cases {
            let text =
                std::fs::read_to_string(format!("tests/fixtures/experiment/{name}")).unwrap();
            let experiment = ResolvedExperiment::resolve(&text).unwrap();
            let review = fixture_integrity_review(&experiment, behavior);
            let record = IntegrityRecord::review(review).unwrap();
            assert_eq!(
                experiment.benchmark().integrity_digest.as_deref(),
                Some(record.identity_digest()),
                "{name} must pin the digest of its derived fixture review"
            );
        }
    }

    #[test]
    fn owning_edge_resolution_rejects_ambiguous_and_unpaired_trials() {
        let text = std::fs::read_to_string("tests/fixtures/experiment/multi-edge.toml").unwrap();
        let experiment = ResolvedExperiment::resolve(&text).unwrap();
        // Every declared trial resolves exactly one owning edge: group A
        // binds to the pi-vs-opi edge, group B to the opi-vs-pi2 edge.
        for (trial, expected_edge) in [
            ("trial-pi-a", "edge-pi-vs-opi"),
            ("trial-opi-a", "edge-pi-vs-opi"),
            ("trial-opi-b", "edge-opi-vs-pi2"),
            ("trial-pi2-b", "edge-opi-vs-pi2"),
        ] {
            let declared = experiment.trials().iter().find(|t| t.id == trial).unwrap();
            assert_eq!(
                owning_edge(&experiment, declared).unwrap().id,
                expected_edge
            );
        }

        // A trial whose group has no counterpart trial on any of its
        // subject's edges is unpaired and must reject before dispatch.
        let unpaired = text.replace(
            "group = \"group-b\"\n\n[[trials]]\nid = \"trial-pi2-b\"",
            "group = \"group-c\"\n\n[[trials]]\nid = \"trial-pi2-b\"",
        );
        let experiment = ResolvedExperiment::resolve(&unpaired).unwrap();
        for trial in ["trial-opi-b", "trial-pi2-b"] {
            let declared = experiment.trials().iter().find(|t| t.id == trial).unwrap();
            let error = owning_edge(&experiment, declared).unwrap_err();
            assert!(
                error.to_string().contains("no owning comparison edge"),
                "{trial}: {error}"
            );
        }

        // Two edges whose counterparts both declare a trial in the same
        // task and group make every trial of that group ambiguous:
        // rejected before dispatch.
        let ambiguous = text.clone().trim_end().to_owned()
            + "\n\n[[trials]]\nid = \"trial-pi-b\"\nsubject = \"baseline-pi\"\ntask = \"fixture-task\"\ngroup = \"group-b\"\n";
        let experiment = ResolvedExperiment::resolve(&ambiguous).unwrap();
        let declared = experiment
            .trials()
            .iter()
            .find(|t| t.id == "trial-opi-b")
            .unwrap();
        let error = owning_edge(&experiment, declared).unwrap_err();
        assert!(
            error.to_string().contains("multiple comparison edges"),
            "{error}"
        );
    }
}

#[cfg(all(test, unix))]
#[test]
fn native_client_capture_is_bounded_and_rejects_links() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let stream = root.path().join("stdout.log");
    assert!(
        read_native_agent_capture(root.path(), "stdout.log")
            .unwrap()
            .bytes
            .is_empty()
    );
    std::fs::write(&stream, vec![b'x'; 1024 * 1024 + 1]).unwrap();
    let capture = read_native_agent_capture(root.path(), "stdout.log").unwrap();
    assert_eq!(capture.bytes.len(), 1024 * 1024 + 1);
    assert!(!capture.truncated);
    std::fs::write(&stream, vec![b'x'; crate::bundle::MAX_ARTIFACT_BYTES + 1]).unwrap();
    let capture = read_native_agent_capture(root.path(), "stdout.log").unwrap();
    assert_eq!(capture.bytes.len(), crate::bundle::MAX_ARTIFACT_BYTES);
    assert!(capture.truncated);
    symlink(&stream, root.path().join("stderr.log")).unwrap();
    assert!(read_native_agent_capture(root.path(), "stderr.log").is_err());
    std::fs::remove_file(root.path().join("stderr.log")).unwrap();
    std::fs::hard_link(&stream, root.path().join("stderr.log")).unwrap();
    assert!(read_native_agent_capture(root.path(), "stdout.log").is_err());
}
