//! `opi-eval conformance` command (tasks 18.10.1, 18.14.1): the minimum
//! provisional process facade over the crate-private `AgentExecution` and
//! `BenchmarkExecution` seams, with an optional native rerun mode that
//! drives the admitted case subset through the exact built executables,
//! the materialized official task packages, and the pinned verifier
//! entrypoints of a resolved native material manifest.
//!
//! One invocation stages and runs exactly one conformance case against one
//! concrete adapter through the real shared execution driver, then prints a
//! single-line JSON report and exits 0 only when the settled outcome met
//! the pinned expectation. Case staging is fixture-level hermetic
//! conformance: a bounded deterministic helper process stands in for the
//! real agent or verifier program, native bytes come from pinned saved
//! fixtures, and the local scripted-provider fixture carries the
//! provider-plumbing case. This facade never claims an exact built
//! Opi/pi program, a real provider call, or an official task environment
//! (task 18.15 reruns the same suites through exact executables); it never
//! calls a paid provider or loads user resources (`P18-AGT-002`,
//! `P18-AGT-006`).

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

use crate::agent::opi::OpiProcessAdapter;

use crate::agent::pi::PiProcessAdapter;
use crate::agent::process::{
    AgentAdapter, AgentCompletion, AgentExecution, AgentRunRequest, Fact, IsolationDirs,
    UsageProjection,
};
use crate::benchmark::deepswe::{DeepSweAdapter, DeepSweProfile};

/// The conformance adapter id for DeepSWE differs from the material
/// manifest key the runner resolves (`deepswe` vs `deepswe-v1.1`).
fn material_key(adapter: &str) -> &str {
    match adapter {
        "deepswe" => "deepswe-v1.1",
        other => other,
    }
}
use crate::benchmark::process::{
    BenchmarkAdapter, BenchmarkCompletion, BenchmarkExecution, BenchmarkRunRequest,
};
use crate::benchmark::terminal_bench_21::{Tb21Profile, TerminalBench21Adapter};
use crate::benchmark::terminal_bench_30::{Tb30Profile, TerminalBench30Adapter};
use crate::integrity::{
    IntegrityRecord, IntegrityReview, OraclePreflight, RevisionStatus, TaskClassification,
};
use crate::process::{ExitState, SpawnReason};

/// Bounded sleep for helper processes in timeout/cancellation cases. The
/// helper outlives every bounded limit; only the supervisor settles it.
const HELPER_SLEEP_SECS: u64 = 30;
/// Delay before the driver cancels a running case (cancellation cases).
const CANCEL_AFTER_MS: u64 = 300;

/// Failures of the conformance facade itself (never settled run outcomes:
/// those live in the report). Exit code 2 from the binary.
#[derive(Debug, Error)]
pub enum ConformanceError {
    /// The requested suite/adapter/case triple is not part of the pinned
    /// conformance surface.
    #[error("unsupported conformance selection: {0}")]
    Unsupported(String),
    /// The run root could not be staged.
    #[error("cannot stage conformance run: {0}")]
    Io(#[from] std::io::Error),
    /// The async runtime could not be built.
    #[error("cannot build execution runtime: {0}")]
    Runtime(String),
}

/// One conformance invocation. Paths are absolute and caller-supplied so
/// the facade never depends on the working directory.
#[derive(Debug, Clone)]
pub struct ConformanceArgs {
    /// `agent` or `benchmark`.
    pub suite: String,
    /// `opi`, `pi`, `terminal-bench-2.1`, `terminal-bench-3.0`, or `deepswe`.
    pub adapter: String,
    /// Case id from the pinned matrices.
    pub case: String,
    /// Fresh run root the driver stages isolated directories and helper
    /// processes under.
    pub root: PathBuf,
    /// Repository `crates/opi-eval/tests/fixtures` root.
    pub fixtures: PathBuf,
    /// `scripts/phase18-scripted-provider.py`.
    pub provider: PathBuf,
    /// Resolved native material manifest (task 18.14.1): when present,
    /// the admitted native case subset runs through the exact built
    /// executables, the material task package, and the pinned verifier
    /// entrypoint instead of fixture helpers.
    pub native_material: Option<PathBuf>,
}

/// The conformance cases admitted in native mode: only cases whose
/// expectation is a real settled Completed/Verified verdict. Failure
/// injection, fixture-replay, and helper-marker cases are hermetic-only.
const NATIVE_AGENT_CASES: &[&str] = &["completed", "identity"];
const NATIVE_BENCHMARK_CASES: &[&str] = &["completed", "identity", "immutable-capture"];

/// The settled one-line report printed by the binary. `met` is the
/// conformance verdict: the pinned expectation for this (adapter, case).
#[derive(Debug, Serialize)]
pub struct ConformanceReport {
    pub schema: &'static str,
    pub suite: String,
    pub adapter: String,
    pub case: String,
    /// `exited:<code>`, `timed-out`, `cancelled`, `spawn:<token>`, or
    /// `rejected:<token>` (pre-spawn admission refusal).
    pub exit_state: String,
    /// `completed`, `verified`, `failed`, or `rejected`.
    pub outcome: String,
    pub failure_kind: Option<String>,
    pub boundary: Option<String>,
    pub identity: serde_json::Value,
    pub usage: Option<serde_json::Value>,
    pub reward: Option<serde_json::Value>,
    pub metrics: Option<serde_json::Value>,
    pub artifact_roles: Vec<String>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub notes: Vec<String>,
    pub met: bool,
}

/// The pinned expectation for one case.
enum Expect {
    Completed,
    Verified,
    Failed(&'static str, &'static str),
    Rejected(&'static str),
}

fn fact_json(fact: &Fact) -> serde_json::Value {
    match fact {
        Fact::Known { value, origin } => serde_json::json!({
            "state": "known", "value": value, "origin": origin,
        }),
        Fact::Unknown { reason } => serde_json::json!({
            "state": "unknown", "reason": reason,
        }),
    }
}

fn usage_json(usage: &UsageProjection) -> serde_json::Value {
    serde_json::json!({
        "input_tokens": usage.input_tokens.as_ref().map(fact_json),
        "output_tokens": usage.output_tokens.as_ref().map(fact_json),
    })
}

/// sha256 hex of `bytes` (report artifact re-verification).
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).unwrap();
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}

/// Run one conformance case end to end and settle its report.
pub fn run(args: &ConformanceArgs) -> Result<ConformanceReport, ConformanceError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| ConformanceError::Runtime(e.to_string()))?;
    runtime.block_on(async {
        match args.suite.as_str() {
            "agent" => run_agent_case(args).await,
            "benchmark" => run_benchmark_case(args).await,
            other => Err(ConformanceError::Unsupported(format!(
                "suite {other:?} (expected agent or benchmark)"
            ))),
        }
    })
}

// ---------------------------------------------------------------------------
// Agent suite
// ---------------------------------------------------------------------------

/// Which native product the helper stands in for, and the saved-bytes
/// fixture names its importer consumes.
struct AgentProduct {
    product: &'static str,
    /// Opi: trace fixtures under `tests/fixtures/agents/opi`.
    trace_source_field: &'static str,
    complete: &'static str,
    invalid: &'static str,
    corrupt: &'static str,
}

const OPI_PRODUCT: AgentProduct = AgentProduct {
    product: "opi",
    trace_source_field: "OPI_EVAL_TRACE_SOURCE",
    complete: "trace-complete",
    invalid: "trace-unknown-schema",
    corrupt: "trace-corrupt",
};

const PI_PRODUCT: AgentProduct = AgentProduct {
    product: "pi",
    trace_source_field: "PI_EVAL_STREAM_SOURCE",
    complete: "stream-ok.jsonl",
    invalid: "unknown-event.jsonl",
    corrupt: "corrupt.jsonl",
};

fn agent_expect(product: &AgentProduct, case: &str) -> Option<Expect> {
    match case {
        "completed" | "identity" | "isolation" | "provider-fixture" => Some(Expect::Completed),
        "nonzero-exit" => Some(Expect::Failed("agent-non-zero-exit", "AgentProcess")),
        "invalid-output" => Some(Expect::Failed("import-unsupported-schema", "Adapter")),
        "parse-failure" => Some(Expect::Failed("import-parse-failure", "Adapter")),
        // Opi's importer does not read the capped stream; pi's terminal
        // event can never be proved from a truncated stdout stream.
        "bounded-output" => {
            if product.product == "opi" {
                Some(Expect::Completed)
            } else {
                Some(Expect::Failed("import-evidence-incomplete", "Evidence"))
            }
        }
        "timeout" => Some(Expect::Failed("agent-timeout", "AgentProcess")),
        "cancellation" => Some(Expect::Failed("agent-cancelled", "AgentProcess")),
        "spawn-failure" => Some(Expect::Failed("spawn-not-found", "AgentProcess")),
        _ => None,
    }
}

/// The deterministic helper script standing in for the real agent binary.
/// Behavior is selected via `OPI_EVAL_CONFORMANCE_BEHAVIOR`; every branch
/// is bounded and emits only pinned fixture bytes.
fn agent_helper_script(product: &AgentProduct) -> String {
    let source = format!("${}", product.trace_source_field);
    // Opi receives the fresh trace root as the `--trace` argv slot ($4);
    // pi emits its native stream on stdout. Both helpers run with the
    // isolated workspace as cwd.
    let native_output = if product.product == "opi" {
        format!("cp -r \"{source}/run-0001\" \"$4/\"")
    } else {
        format!("cat \"{source}\"")
    };
    // Launch-surface guard: the helper stands in at the exact pinned argv
    // slot, so a drifted launch policy is a typed failure, not silence.
    let argv_guard = if product.product == "opi" {
        r#"[ "$1" = "--json" ] || { echo "argv drift: expected --json first" >&2; exit 9; }"#
    } else {
        r#"[ "$1" = "--mode" ] && [ "$2" = "json" ] || { echo "argv drift: expected --mode json" >&2; exit 9; }"#
    };
    // Bounded-output case: opi's settlement reads trace files, so a capped
    // stderr proves the cap without touching settlement; pi's settlement
    // reads stdout, so the junk must hit the native stream itself.
    let bounded_output = if product.product == "opi" {
        "head -c 2097152 /dev/zero | tr '\\0' 'a' >&2"
    } else {
        "head -c 2097152 /dev/zero | tr '\\0' 'a'"
    };
    let isolation_markers = if product.product == "opi" {
        "echo m > \"$HOME/conformance-home-marker\"\n  echo m > \"$APPDATA/conformance-appdata-marker\"\n  echo m > \"$OPI_SESSIONS_DIR/conformance-sessions-marker\"\n  pwd > conformance-cwd-marker"
    } else {
        "echo m > \"$HOME/conformance-home-marker\"\n  echo m > \"$USERPROFILE/conformance-appdata-marker\"\n  echo m > \"$PI_CODING_AGENT_SESSION_DIR/conformance-sessions-marker\"\n  echo m > \"$PI_CODING_AGENT_DIR/conformance-agentdir-marker\"\n  pwd > conformance-cwd-marker"
    };
    format!(
        "#!/bin/sh\n\
# conformance fixture helper standing in for the real {} binary (never the real product)\n\
{argv_guard}\n\
case \"$OPI_EVAL_CONFORMANCE_BEHAVIOR\" in\n\
  completed|identity)\n\
    {native_output} ;;\n\
  invalid-output)\n\
    {native_output} ;;\n\
  parse-failure)\n\
    {native_output} ;;\n\
  bounded-output)\n\
    {native_output}\n\
    {bounded_output} ;;\n\
  isolation)\n\
    {native_output}\n\
    {isolation_markers} ;;\n\
  provider-fixture)\n\
    printf '%s\\n' '{{\"prompt\":\"conformance\"}}' | \"$OPI_EVAL_SCRIPTED_PROVIDER\" > conformance-provider-response\n\
    {native_output} ;;\n\
  nonzero-exit)\n\
    {native_output}\n\
    exit 3 ;;\n\
  timeout|cancellation)\n\
    sleep {HELPER_SLEEP_SECS} ;;\n\
  *) echo \"unknown behavior\" >&2; exit 9 ;;\n\
esac\n\
exit 0\n",
        product.product
    )
}

/// Stage the native-bytes source env entries for the case.
fn agent_case_env(
    args: &ConformanceArgs,
    product: &AgentProduct,
    case: &str,
) -> BTreeMap<OsString, OsString> {
    let fixture_name = match case {
        "invalid-output" => product.invalid,
        "parse-failure" => product.corrupt,
        _ => product.complete,
    };
    let source = args
        .fixtures
        .join("agents")
        .join(product.product)
        .join(fixture_name);
    let mut env = BTreeMap::new();
    env.insert(
        OsString::from(product.trace_source_field),
        source.into_os_string(),
    );
    env.insert(
        OsString::from("OPI_EVAL_CONFORMANCE_BEHAVIOR"),
        OsString::from(case),
    );
    if case == "provider-fixture" {
        env.insert(
            OsString::from("OPI_EVAL_SCRIPTED_PROVIDER"),
            args.provider.clone().into_os_string(),
        );
    }
    env
}

/// The bounded-timeout variant of a checked-in agent profile: identical
/// identity and launch pins, only the supervisor limit is lowered so the
/// timeout case settles in bounded wall time.
fn timeout_patched_profile_text(embedded: &str) -> String {
    patched_timeout_secs(embedded).expect("profile pins a timeout_secs limit")
}

/// Lower a profile's pinned `timeout_secs` to a bounded value so the
/// timeout case settles in bounded wall time. Fails closed when no pinned
/// limit is found to lower.
fn patched_timeout_secs(text: &str) -> Option<String> {
    for pinned in ["timeout_secs = 900", "timeout_secs = 1800"] {
        if text.contains(pinned) {
            return Some(text.replace(pinned, "timeout_secs = 2"));
        }
    }
    None
}

async fn run_agent_case(args: &ConformanceArgs) -> Result<ConformanceReport, ConformanceError> {
    let product = match args.adapter.as_str() {
        "opi" => &OPI_PRODUCT,
        "pi" => &PI_PRODUCT,
        other => {
            return Err(ConformanceError::Unsupported(format!(
                "agent adapter {other:?}"
            )));
        }
    };
    let expect = agent_expect(product, &args.case).ok_or_else(|| {
        ConformanceError::Unsupported(format!(
            "agent case {:?} for {}",
            args.case, product.product
        ))
    })?;

    // Stage the isolated run under the caller-supplied root.
    let run = args.root.join("agent");
    let workspace = run.join("ws");
    let trace_root = run.join("trace");
    let isolation = IsolationDirs {
        home: run.join("iso/home"),
        app_data: run.join("iso/appdata"),
        sessions: run.join("iso/sessions"),
    };
    for dir in [
        &workspace,
        &trace_root,
        &isolation.home,
        &isolation.app_data,
        &isolation.sessions,
    ] {
        std::fs::create_dir_all(dir)?;
    }
    let config_path = run.join("bench.toml");
    let (executable, provider_model, extra_env) = match &args.native_material {
        Some(material_path) => {
            if !NATIVE_AGENT_CASES.contains(&args.case.as_str()) {
                return Err(ConformanceError::Unsupported(format!(
                    "agent case {:?} is hermetic-only; native mode admits {NATIVE_AGENT_CASES:?}",
                    args.case
                )));
            }
            let material = crate::runner::material::NativeMaterial::load(material_path)
                .map_err(|error| ConformanceError::Unsupported(error.to_string()))?;
            let agent = material
                .agent(product.product)
                .map_err(|error| ConformanceError::Unsupported(error.to_string()))?;
            std::fs::write(
                &config_path,
                crate::runner::experiment::native_opi_config(&agent.config, &agent.model),
            )?;
            let mut env = crate::runner::experiment::native_agent_env(
                product.product,
                &agent.config,
                &isolation,
            )
            .map_err(ConformanceError::Unsupported)?;
            for (key, value) in &agent.provider_env {
                env.insert(key.clone().into(), value.clone().into());
            }
            (agent.executable.path.clone(), agent.model.clone(), env)
        }
        None => {
            std::fs::write(
                &config_path,
                "# conformance fixture config (never a user config)\n",
            )?;
            // The helper (or a deliberately missing path for the
            // spawn-failure case) is the resolved executable: never an
            // ambient PATH lookup.
            let executable = if args.case == "spawn-failure" {
                run.join("no-such-agent")
            } else {
                let helper = run.join("helper-agent.sh");
                std::fs::write(&helper, agent_helper_script(product))?;
                make_executable(&helper);
                helper
            };
            (
                executable,
                "local:scripted".to_owned(),
                agent_case_env(args, product, &args.case),
            )
        }
    };

    let request = AgentRunRequest {
        executable,
        prompt: "conformance: solve the pinned fixture task".to_owned(),
        workspace: workspace.clone(),
        trace_root: trace_root.clone(),
        config_path,
        provider_model,
        allow_mutating: args.native_material.is_some(),
        isolation: isolation.clone(),
        extra_env,
    };

    // The adapter for the timeout case carries the bounded-timeout variant
    // of the same pinned profile; every other case uses the checked-in one.
    let record = match product.product {
        "opi" => {
            let adapter = if args.case == "timeout" {
                let text =
                    timeout_patched_profile_text(include_str!("../../profiles/agents/opi.toml"));
                OpiProcessAdapter::from_profile(
                    crate::agent::opi::OpiProfile::parse(&text).map_err(|e| {
                        ConformanceError::Unsupported(format!(
                            "timeout-profile variant rejected: {e}"
                        ))
                    })?,
                )
            } else {
                OpiProcessAdapter::new()
            };
            dispatch_agent(&request, &adapter, &args.case).await?
        }
        _ => {
            let adapter = if args.case == "timeout" {
                let text =
                    timeout_patched_profile_text(include_str!("../../profiles/agents/pi.toml"));
                PiProcessAdapter::from_profile(crate::agent::pi::PiProfile::parse(&text).map_err(
                    |e| {
                        ConformanceError::Unsupported(format!(
                            "timeout-profile variant rejected: {e}"
                        ))
                    },
                )?)
            } else {
                PiProcessAdapter::new()
            };
            dispatch_agent(&request, &adapter, &args.case).await?
        }
    };

    let mut notes = Vec::new();
    let mut extra_ok = true;

    // Native conformance requires the promised final-workspace output,
    // independently of whether the Agent process and evidence importer
    // otherwise settled successfully.
    if args.native_material.is_some() {
        let answer = std::fs::read(workspace.join("answer.txt"));
        if answer.is_ok_and(|bytes| !bytes.is_empty()) {
            notes.push("final-workspace-answer-verified".to_owned());
        } else {
            extra_ok = false;
            notes.push("final-workspace-answer-missing-or-empty".to_owned());
        }
    }

    // Isolation case: the helper proved the projected environment and cwd
    // by dropping markers only the isolated projection could receive.
    if args.case == "isolation" {
        let cwd_marker = std::fs::read_to_string(workspace.join("conformance-cwd-marker"))
            .map(|p| p.trim().to_owned())
            .unwrap_or_default();
        let markers_ok = [
            isolation.home.join("conformance-home-marker"),
            isolation.app_data.join("conformance-appdata-marker"),
            isolation.sessions.join("conformance-sessions-marker"),
        ]
        .iter()
        .all(|p| p.is_file())
            && cwd_marker == workspace.to_string_lossy();
        if markers_ok {
            notes.push("isolation-verified".to_owned());
        } else {
            extra_ok = false;
            notes.push("isolation-markers-missing".to_owned());
        }
    }

    // Provider-fixture case: the helper round-tripped one request through
    // the local scripted provider; the response bytes must be exactly the
    // fixture's deterministic line.
    if args.case == "provider-fixture" {
        let response = std::fs::read_to_string(workspace.join("conformance-provider-response"))
            .unwrap_or_default();
        if response.trim_end()
            == "{\"content\":\"scripted-provider: acknowledged\",\"schema\":\"phase18-scripted-provider/1\"}"
        {
            notes.push("provider-fixture-verified".to_owned());
        } else {
            extra_ok = false;
            notes.push("provider-fixture-response-drift".to_owned());
        }
    }

    let mut artifact_roles = Vec::new();
    if let AgentCompletion::Completed { artifacts } = &record.completion {
        for artifact in artifacts {
            // Immutable capture: every retained artifact is
            // content-addressed against the bytes still on disk in place.
            let bytes = std::fs::read(&artifact.path).unwrap_or_default();
            if sha256_hex(&bytes) != artifact.sha256 || bytes.is_empty() {
                extra_ok = false;
            }
            artifact_roles.push(artifact.role.clone());
        }
    }

    let (outcome, failure_kind, boundary) = match &record.completion {
        AgentCompletion::Completed { .. } => ("completed", None, None),
        AgentCompletion::Failed(failure) => (
            "failed",
            Some(failure.kind.to_owned()),
            Some(format!("{:?}", failure.boundary)),
        ),
    };
    let met = extra_ok
        && match (&expect, &record.completion) {
            (Expect::Completed, AgentCompletion::Completed { .. }) => true,
            (Expect::Failed(kind, boundary), AgentCompletion::Failed(failure)) => {
                failure.kind == *kind && format!("{:?}", failure.boundary) == *boundary
            }
            _ => false,
        };
    if !met && outcome == "completed" {
        notes.push("expectation-not-met".to_owned());
    }

    Ok(ConformanceReport {
        schema: "phase18-conformance-report/1",
        suite: "agent".to_owned(),
        adapter: args.adapter.clone(),
        case: args.case.clone(),
        exit_state: exit_state_json(&record.exit),
        outcome: outcome.to_owned(),
        failure_kind,
        boundary,
        identity: serde_json::json!({
            "product": record.identity.product,
            "package": record.identity.package,
            "adapter": record.identity.adapter,
            "provider_model": request.provider_model,
        }),
        usage: Some(usage_json(&record.usage)),
        reward: None,
        metrics: None,
        artifact_roles,
        stdout_truncated: record.stdout.truncated,
        stderr_truncated: record.stderr.truncated,
        notes,
        met,
    })
}

/// Drive one agent request through the real shared execution driver,
/// cancelling after a bounded delay for the cancellation case.
async fn dispatch_agent(
    request: &AgentRunRequest,
    adapter: &dyn AgentAdapter,
    case: &str,
) -> Result<crate::agent::process::AgentRecord, ConformanceError> {
    if case == "cancellation" {
        let cancel = CancellationToken::new();
        let run = Box::pin(AgentExecution::run(request, adapter, &cancel));
        tokio::time::sleep(Duration::from_millis(CANCEL_AFTER_MS)).await;
        cancel.cancel();
        return run
            .await
            .map_err(|e| ConformanceError::Unsupported(e.to_string()));
    }
    AgentExecution::run(request, adapter, &CancellationToken::new())
        .await
        .map_err(|e| ConformanceError::Unsupported(e.to_string()))
}

fn exit_state_json(exit: &ExitState) -> String {
    match exit {
        ExitState::Exited { code } => format!("exited:{code}"),
        ExitState::TimedOut => "timed-out".to_owned(),
        ExitState::Cancelled => "cancelled".to_owned(),
        ExitState::FailedToSpawn { reason } => format!("spawn:{}", spawn_token(reason)),
    }
}

/// Static redacted token for a spawn failure reason (mirrors the shared
/// failure-kind mapping; diagnostics never echo request paths).
fn spawn_token(reason: &SpawnReason) -> &'static str {
    match reason {
        SpawnReason::NotFound => "spawn-not-found",
        SpawnReason::PermissionDenied => "spawn-permission-denied",
        SpawnReason::BadCwd => "spawn-bad-cwd",
        SpawnReason::SpawnFailed => "spawn-failed",
    }
}

// ---------------------------------------------------------------------------
// Benchmark suite
// ---------------------------------------------------------------------------

/// One pinned benchmark revision the conformance facade drives.
struct BenchmarkRevision {
    /// CLI adapter id.
    adapter: &'static str,
    benchmark: &'static str,
    revision: &'static str,
    /// Directory under `tests/fixtures/benchmarks/` holding the synthetic
    /// profile, task package, and native-output fixtures.
    fixture_dir: &'static str,
    /// Report filename the fake verifier writes into the trace root.
    report_name: &'static str,
    dataset: &'static str,
    grader: &'static str,
    environment: &'static str,
}

const TB21: BenchmarkRevision = BenchmarkRevision {
    adapter: "terminal-bench-2.1",
    benchmark: "terminal-bench",
    revision: "2.1",
    fixture_dir: "terminal-bench-2.1",
    report_name: "ctrf-report.json",
    dataset: "terminal-bench-v2.1.0",
    grader: "harbor-v0.3.41",
    environment: "separate-container",
};

const TB30: BenchmarkRevision = BenchmarkRevision {
    adapter: "terminal-bench-3.0",
    benchmark: "terminal-bench",
    revision: "3.0",
    fixture_dir: "terminal-bench-3.0",
    report_name: "ctrf-report.json",
    dataset: "terminal-bench-v3.0.0",
    grader: "harbor-v0.22.0",
    environment: "separate-verifier-container",
};

const DEEPSWE: BenchmarkRevision = BenchmarkRevision {
    adapter: "deepswe",
    benchmark: "deepswe",
    revision: "v1.1",
    fixture_dir: "deepswe-v1.1",
    report_name: "pier-report.json",
    dataset: "deepswe-v1.1",
    grader: "pier-v0.3.1",
    environment: "separate-pristine-verifier",
};

/// The saved native bytes a case feeds the importer.
fn native_fixture(revision: &BenchmarkRevision, case: &str) -> String {
    let ctrf = |name: &str| format!("ctrf/{name}");
    match (revision.adapter, case) {
        ("terminal-bench-2.1", "one-failed") => ctrf("one-failed.json"),
        ("terminal-bench-2.1", "parse-failure") => ctrf("corrupt.json"),
        ("terminal-bench-2.1", "schema-drift") => ctrf("unknown-schema.json"),
        ("terminal-bench-2.1", _) => ctrf("ok-six-passed.json"),
        // Terminal-Bench 3.0 committed no unknown-schema CTRF fixture; the
        // drift bytes live in the shared conformance fixture root.
        ("terminal-bench-3.0", "one-failed") => ctrf("one-failed.json"),
        ("terminal-bench-3.0", "parse-failure") => ctrf("corrupt.json"),
        ("terminal-bench-3.0", "schema-drift") => {
            "../../conformance/benchmarks/ctrf-unknown-schema.json".to_owned()
        }
        ("terminal-bench-3.0", _) => ctrf("ok-six-passed.json"),
        ("deepswe", "zero-reward") => "pier-report/zero.json".to_owned(),
        ("deepswe", "parse-failure") => "pier-report/corrupt.json".to_owned(),
        ("deepswe", "schema-drift") => "pier-report/drift.json".to_owned(),
        ("deepswe", _) => "pier-report/resolved.json".to_owned(),
        _ => unreachable!("adapter pinned above"),
    }
}

fn benchmark_expect(revision: &BenchmarkRevision, case: &str) -> Option<Expect> {
    match case {
        "completed" | "identity" | "immutable-capture" | "one-failed" | "zero-reward" => {
            Some(Expect::Verified)
        }
        "parse-failure" => Some(Expect::Failed("import-parse-failure", "Adapter")),
        "schema-drift" => Some(Expect::Failed("import-unsupported-schema", "Adapter")),
        "nonzero-exit" => Some(Expect::Failed("verifier-non-zero-exit", "Grader")),
        "timeout" => Some(Expect::Failed("verifier-timeout", "Grader")),
        "cancellation" => Some(Expect::Failed("verifier-cancelled", "Grader")),
        "spawn-failure" => Some(Expect::Failed("spawn-not-found", "Infrastructure")),
        "package-drift" => Some(Expect::Rejected("task-package-drift")),
        // Production pins fail closed against fixture bytes: Terminal-Bench
        // 2.1's production byte table pins the real official package (the
        // synthetic fixture bytes drift), while 3.0 and DeepSWE pin
        // task-tree identity only. Real bytes arrive with task 18.15.
        "production-pin-drift" if revision.adapter == TB21.adapter => {
            Some(Expect::Rejected("task-package-drift"))
        }
        // The case name predates task 18.15's byte-table registration:
        // every production pin now rejects the synthetic fixture bytes
        // as drift instead of failing as not materialized.
        "package-not-materialized" => Some(Expect::Rejected("task-package-drift")),
        _ => None,
    }
}

/// The deterministic helper script standing in for the native verifier
/// program: it writes pinned saved bytes into the trace root (its cwd) or
/// sleeps past every bounded limit. Never the real grader.
fn benchmark_helper_script(revision: &BenchmarkRevision) -> String {
    format!(
        "#!/bin/sh\n\
# conformance fixture helper standing in for the native verifier (never the real grader)\n\
case \"$OPI_EVAL_CONFORMANCE_BEHAVIOR\" in\n\
  completed|identity|immutable-capture|production-pin-drift|one-failed|zero-reward|parse-failure|schema-drift)\n\
    cp \"$OPI_EVAL_NATIVE_SOURCE\" \"./{}\" ;;\n\
  nonzero-exit)\n\
    cp \"$OPI_EVAL_NATIVE_SOURCE\" \"./{}\"\n\
    exit 5 ;;\n\
  timeout|cancellation)\n\
    sleep {} ;;\n\
  *) echo \"unknown behavior\" >&2; exit 9 ;;\n\
esac\n\
exit 0\n",
        revision.report_name, revision.report_name, HELPER_SLEEP_SECS
    )
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

async fn run_benchmark_case(args: &ConformanceArgs) -> Result<ConformanceReport, ConformanceError> {
    let revision = match args.adapter.as_str() {
        "terminal-bench-2.1" => &TB21,
        "terminal-bench-3.0" => &TB30,
        "deepswe" => &DEEPSWE,
        other => {
            return Err(ConformanceError::Unsupported(format!(
                "benchmark adapter {other:?}"
            )));
        }
    };
    let expect = benchmark_expect(revision, &args.case).ok_or_else(|| {
        ConformanceError::Unsupported(format!(
            "benchmark case {:?} for {}",
            args.case, revision.adapter
        ))
    })?;

    // Stage the isolated run: a staged copy of the pinned fixture task
    // package (tamperable for the drift case), a fresh trace root the fake
    // verifier writes into, and the collected-patch placeholder. Native
    // mode stages the materialized official package instead.
    let run = args.root.join("benchmark");
    let trace_root = run.join("trace");
    let agent_output = run.join("agent-output");
    let task_dir = run.join("task-package");
    std::fs::create_dir_all(&trace_root)?;
    std::fs::create_dir_all(&agent_output)?;
    let native_material = match &args.native_material {
        Some(material_path) => {
            if !NATIVE_BENCHMARK_CASES.contains(&args.case.as_str()) {
                return Err(ConformanceError::Unsupported(format!(
                    "benchmark case {:?} is hermetic-only; native mode admits                      {NATIVE_BENCHMARK_CASES:?}",
                    args.case
                )));
            }
            Some(
                crate::runner::material::NativeMaterial::load(material_path)
                    .map_err(|error| ConformanceError::Unsupported(error.to_string()))?,
            )
        }
        None => None,
    };
    let fixture_package = match &native_material {
        Some(material) => material
            .benchmark(material_key(revision.adapter))
            .map_err(|error| ConformanceError::Unsupported(error.to_string()))?
            .task_package
            .clone(),
        None => args
            .fixtures
            .join("benchmarks")
            .join(revision.fixture_dir)
            .join("task-package"),
    };
    copy_dir_recursive(&fixture_package, &task_dir)?;
    if args.case == "package-drift" {
        std::fs::write(task_dir.join("rogue-fixture.txt"), b"unregistered")?;
    }

    // Profile source: the synthetic fixture profile (byte-table pin), the
    // embedded production profile (production-admission for 2.1,
    // package-not-materialized for 3.0/DeepSWE), or a bounded-timeout
    // variant of the synthetic profile for the timeout case.
    let synthetic_path = args
        .fixtures
        .join("benchmarks")
        .join(revision.fixture_dir)
        .join("profile/synthetic.toml");
    let profile_text: String = match &native_material {
        Some(material) => std::fs::read_to_string(
            material
                .benchmark(material_key(revision.adapter))
                .map_err(|error| ConformanceError::Unsupported(error.to_string()))?
                .profile
                .clone(),
        )?,
        None => match args.case.as_str() {
            "production-pin-drift" | "package-not-materialized" => match revision.adapter {
                "terminal-bench-2.1" => {
                    include_str!("../../profiles/benchmarks/terminal-bench-2.1.toml").to_owned()
                }
                "terminal-bench-3.0" => {
                    include_str!("../../profiles/benchmarks/terminal-bench-3.0.toml").to_owned()
                }
                _ => include_str!("../../profiles/benchmarks/deepswe-v1.1.toml").to_owned(),
            },
            "timeout" => patched_timeout_secs(&std::fs::read_to_string(&synthetic_path)?)
                .ok_or_else(|| {
                    ConformanceError::Unsupported(format!(
                        "no pinned timeout_secs limit found to lower for {}",
                        revision.adapter
                    ))
                })?,
            _ => std::fs::read_to_string(&synthetic_path)?,
        },
    };

    let mut extra_env: BTreeMap<OsString, OsString> = BTreeMap::new();
    match &native_material {
        Some(material) => {
            let benchmark = material
                .benchmark(material_key(revision.adapter))
                .map_err(|error| ConformanceError::Unsupported(error.to_string()))?;
            for (key, value) in &benchmark.verifier_env {
                extra_env.insert(key.clone().into(), value.clone().into());
            }
        }
        None => {
            let native_source = args
                .fixtures
                .join("benchmarks")
                .join(revision.fixture_dir)
                .join(native_fixture(revision, &args.case));
            extra_env.insert(
                OsString::from("OPI_EVAL_CONFORMANCE_BEHAVIOR"),
                OsString::from(args.case.clone()),
            );
            extra_env.insert(
                OsString::from("OPI_EVAL_NATIVE_SOURCE"),
                native_source.into_os_string(),
            );
        }
    }

    let verifier_executable = match &native_material {
        Some(material) => material
            .benchmark(material_key(revision.adapter))
            .map_err(|error| ConformanceError::Unsupported(error.to_string()))?
            .verifier_executable
            .path
            .clone(),
        None if args.case == "spawn-failure" => run.join("no-such-verifier"),
        None => {
            let helper = run.join("helper-verifier.sh");
            std::fs::write(&helper, benchmark_helper_script(revision))?;
            make_executable(&helper);
            helper
        }
    };

    let report_for_record = |record: crate::benchmark::process::BenchmarkRecord,
                             identity_json: serde_json::Value,
                             mut notes: Vec<String>,
                             extra_ok: bool|
     -> ConformanceReport {
        let mut artifact_roles = Vec::new();
        let mut capture_ok = extra_ok;
        if let BenchmarkCompletion::Verified { metrics, artifacts } = &record.completion {
            for artifact in artifacts {
                // Immutable capture: every retained artifact stays
                // content-addressed against the bytes on disk, in place.
                let bytes = std::fs::read(&artifact.path).unwrap_or_default();
                if bytes.is_empty() || sha256_hex(&bytes) != artifact.sha256 {
                    capture_ok = false;
                }
                artifact_roles.push(artifact.role.clone());
            }
            let _ = metrics;
        }
        let metrics_json = match &record.completion {
            BenchmarkCompletion::Verified { metrics, .. } => Some(serde_json::json!({
                "tests": metrics.tests.as_ref().map(fact_json),
                "passed": metrics.passed.as_ref().map(fact_json),
                "failed": metrics.failed.as_ref().map(fact_json),
                "skipped": metrics.skipped.as_ref().map(fact_json),
                "pending": metrics.pending.as_ref().map(fact_json),
                "other": metrics.other.as_ref().map(fact_json),
            })),
            _ => None,
        };
        if args.case == "immutable-capture" {
            let provenance_ok = !record.provenance.admitted_lock_digest.is_empty()
                && !record.provenance.integrity_digest.is_empty()
                && record.provenance.task_id == identity_json["task_id"].as_str().unwrap_or("");
            if capture_ok && provenance_ok {
                notes.push("immutable-capture-verified".to_owned());
            } else {
                capture_ok = false;
                notes.push("immutable-capture-drift".to_owned());
            }
            // Honesty: this digest is the staged fixture value, not an
            // admitted resolved external lock (that arrives with
            // 18.12/18.15, which own resolved-lock assembly).
            notes.push(
                "admitted-digest-is-fixture-value: resolved external locks arrive with 18.12/18.15"
                    .to_owned(),
            );
        }
        let (outcome, failure_kind, boundary) = match &record.completion {
            BenchmarkCompletion::Verified { .. } => ("verified", None, None),
            BenchmarkCompletion::Failed(failure) => (
                "failed",
                Some(failure.kind.to_owned()),
                Some(format!("{:?}", failure.boundary)),
            ),
        };
        let met = capture_ok
            && match (&expect, &record.completion) {
                (Expect::Verified, BenchmarkCompletion::Verified { .. }) => true,
                (Expect::Failed(kind, boundary), BenchmarkCompletion::Failed(failure)) => {
                    failure.kind == *kind && format!("{:?}", failure.boundary) == *boundary
                }
                _ => false,
            };
        // A failed native verifier keeps its captured output tails in the
        // report notes: the process verdict is authoritative, but the
        // upstream tool's own diagnostics are the only way to see why it
        // refused. Tails only, never full logs.
        if !matches!(&record.completion, BenchmarkCompletion::Verified { .. }) {
            let tail = |bytes: &[u8]| -> String {
                let text = String::from_utf8_lossy(bytes);
                let kept: Vec<char> = text.chars().collect();
                let start = kept.len().saturating_sub(1200);
                kept[start..].iter().collect()
            };
            notes.push(format!(
                "verifier-stdout-tail: {}",
                tail(&record.stdout.bytes)
            ));
            notes.push(format!(
                "verifier-stderr-tail: {}",
                tail(&record.stderr.bytes)
            ));
        }
        ConformanceReport {
            schema: "phase18-conformance-report/1",
            suite: "benchmark".to_owned(),
            adapter: args.adapter.clone(),
            case: args.case.clone(),
            exit_state: exit_state_json(&record.exit),
            outcome: outcome.to_owned(),
            failure_kind,
            boundary,
            identity: identity_json,
            usage: None,
            reward: Some(fact_json(&record.reward)),
            metrics: metrics_json,
            artifact_roles,
            stdout_truncated: record.stdout.truncated,
            stderr_truncated: record.stderr.truncated,
            notes,
            met,
        }
    };

    match revision.adapter {
        "terminal-bench-2.1" => {
            let profile = Tb21Profile::parse(profile_text.as_bytes()).map_err(|e| {
                ConformanceError::Unsupported(format!("tb21 profile rejected: {e}"))
            })?;
            let adapter = TerminalBench21Adapter::from_profile(profile.clone());
            finish_benchmark_case(
                args,
                revision,
                &adapter,
                profile.task_id.clone(),
                task_dir,
                trace_root,
                agent_output,
                verifier_executable,
                extra_env,
                &expect,
                report_for_record,
            )
            .await
        }
        "terminal-bench-3.0" => {
            let profile = Tb30Profile::parse(profile_text.as_bytes()).map_err(|e| {
                ConformanceError::Unsupported(format!("tb30 profile rejected: {e}"))
            })?;
            let adapter = TerminalBench30Adapter::from_profile(profile.clone());
            finish_benchmark_case(
                args,
                revision,
                &adapter,
                profile.task_id.clone(),
                task_dir,
                trace_root,
                agent_output,
                verifier_executable,
                extra_env,
                &expect,
                report_for_record,
            )
            .await
        }
        _ => {
            let profile = DeepSweProfile::parse(profile_text.as_bytes()).map_err(|e| {
                ConformanceError::Unsupported(format!("deepswe profile rejected: {e}"))
            })?;
            let adapter = DeepSweAdapter::from_profile(profile.clone());
            finish_benchmark_case(
                args,
                revision,
                &adapter,
                profile.task_id.clone(),
                task_dir,
                trace_root,
                agent_output,
                verifier_executable,
                extra_env,
                &expect,
                report_for_record,
            )
            .await
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn finish_benchmark_case<A: BenchmarkAdapter>(
    args: &ConformanceArgs,
    revision: &BenchmarkRevision,
    adapter: &A,
    task_id: String,
    task_dir: PathBuf,
    trace_root: PathBuf,
    agent_output: PathBuf,
    verifier_executable: PathBuf,
    extra_env: BTreeMap<OsString, OsString>,
    expect: &Expect,
    report_for_record: impl Fn(
        crate::benchmark::process::BenchmarkRecord,
        serde_json::Value,
        Vec<String>,
        bool,
    ) -> ConformanceReport,
) -> Result<ConformanceReport, ConformanceError> {
    // The staged fixture digest stands in for the admitted resolved lock
    // digest: honest fixture plumbing, not a resolved-lock claim. Native
    // mode binds the material's admitted static lock instead.
    let admitted_lock_digest = match &args.native_material {
        Some(material_path) => crate::runner::material::NativeMaterial::load(material_path)
            .map_err(|error| ConformanceError::Unsupported(error.to_string()))?
            .static_lock
            .sha256
            .clone(),
        None => sha256_hex(args.case.as_bytes()),
    };
    let integrity = IntegrityRecord::review(IntegrityReview {
        benchmark: revision.benchmark.to_owned(),
        revision: revision.revision.to_owned(),
        dataset: revision.dataset.to_owned(),
        grader: revision.grader.to_owned(),
        environment: revision.environment.to_owned(),
        upstream_identity: "conformance-fixture".to_owned(),
        upstream_digest: "0".repeat(64),
        oracle: Some(OraclePreflight::Passed("fixture conformance".to_owned())),
        status: RevisionStatus::Admitted,
        tasks: BTreeMap::from([(task_id.clone(), TaskClassification::ValidAgentOutcome)]),
        excluded_trials: BTreeMap::new(),
        reviewer: "conformance-fixture".to_owned(),
    })
    .map_err(|e| ConformanceError::Unsupported(format!("fixture integrity rejected: {e}")))?;
    let request = BenchmarkRunRequest {
        verifier_executable,
        task_dir,
        task_id,
        agent_output,
        trace_root,
        admitted_lock_digest,
        integrity,
        extra_env,
    };

    let identity = adapter.identity();
    let identity_json = serde_json::json!({
        "benchmark": identity.benchmark,
        "revision": identity.revision,
        "adapter": identity.adapter,
        "task_id": request.task_id,
        "admitted_lock_digest": request.admitted_lock_digest,
    });

    let settled = if args.case == "cancellation" {
        let cancel = CancellationToken::new();
        let run = Box::pin(BenchmarkExecution::run(&request, adapter, &cancel));
        tokio::time::sleep(Duration::from_millis(CANCEL_AFTER_MS)).await;
        cancel.cancel();
        run.await
    } else {
        BenchmarkExecution::run(&request, adapter, &CancellationToken::new()).await
    };
    match settled {
        Err(rejection) => {
            // Pre-spawn admission refusal: no process ever existed.
            let token = rejection.token;
            let met = matches!(expect, Expect::Rejected(expected) if *expected == token);
            Ok(ConformanceReport {
                schema: "phase18-conformance-report/1",
                suite: "benchmark".to_owned(),
                adapter: args.adapter.clone(),
                case: args.case.clone(),
                exit_state: format!("rejected:{token}"),
                outcome: "rejected".to_owned(),
                failure_kind: None,
                boundary: None,
                identity: identity_json,
                usage: None,
                reward: None,
                metrics: None,
                artifact_roles: Vec::new(),
                stdout_truncated: false,
                stderr_truncated: false,
                notes: Vec::new(),
                met,
            })
        }
        Ok(record) => Ok(report_for_record(record, identity_json, Vec::new(), true)),
    }
}
