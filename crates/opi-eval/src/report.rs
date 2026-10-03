//! Normalized offline report over sealed assembled outputs.
//!
//! [`ReportBuilder::recompute_from_bundle`] rebuilds the normalized view
//! from verified sealed bundles: graded trial views, the pair
//! coverage, the integrity provenance, the native rewards, and the
//! diagnostics are reconstructed from the sealed control evidence (the
//! resolved experiment contract and the integrity record), the sealed
//! trajectory projection, the sealed authority ledger, and the sealed
//! manifest identities. Unsealed settlements retain attempted failure
//! coverage as explicitly unverified diagnostics, never as scores. Outer
//! receipts cannot change sealed trial facts (`EVAL-RPT-001`, `EVAL-OUT-004`).
//! Both paths are effect-free: no Agent,
//! no provider, no spawn, no mutation of sealed bytes.
//!
//! Report contract enforced here: headline outcomes come only from the
//! admitted grader-sourced native report artifact with per-headline
//! provenance (`EVAL-RPT-003`); pair coverage keeps every declared pair
//! visible with its exact state, so exclusions, failures, and unknowns
//! never leave the denominator silently (`EVAL-RPT-004`, `EVAL-EXP-006`);
//! quality, cost, safety, efficiency, and authority are never collapsed
//! into one composite score or best-trial verdict (`EVAL-RPT-005`); the
//! report labels its evidence `conformance-evidence` and claims no
//! official leaderboard verification (`EVAL-RPT-006`). Asymmetric native
//! facts stay measured values (cited by sealed artifact digest) or typed
//! unknowns - never fabricated parity (`EVAL-A16`). Declared canary
//! secrets in exportable bundle content block publication (`EVAL-A18`,
//! `EVAL-SEC-005`). Identical sealed inputs and tool identities serialize
//! to byte-identical output (`EVAL-RPT-002`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::benchmark::process::native_grader_observation;
use crate::bundle::RunBundle;

/// Normalized report schema identity.
const REPORT_SCHEMA: &str = "opi-eval-normalized-report/2";
/// Pinned reporter identity: part of every byte-stability contract
/// (`EVAL-RPT-002` - same sealed inputs, grader identity, and reporter
/// version).
pub(crate) const REPORTER_VERSION: &str = "opi-eval-reporter/2";
/// The single classification wording of opi-eval paired results
/// (`EVAL-RPT-006`): conformance evidence only, never leaderboard
/// verification or superiority.
const CLASSIFICATION: &str = "conformance-evidence";
/// Outcome token when publication succeeded.
const OUTCOME_PUBLISHED: &str = "published";
/// Outcome token when a declared canary blocked publication.
const OUTCOME_BLOCKED: &str = "publication-blocked";
/// Outcome token when a contributing bundle failed verification or a
/// sealed input failed to parse: nothing is published.
const OUTCOME_UNVERIFIED: &str = "verification-failed";

/// The asymmetric native fact families the common report tracks. A family
/// is `measured` for a product only when the product's own sealed native
/// artifact carries it; otherwise it stays a typed unknown
/// (`EVAL-A16`).
const NATIVE_FACT_FAMILIES: [&str; 4] = ["usage", "cost", "retry", "compaction"];

/// Which sealed native artifact carries one fact family for one product.
/// Products absent from this table expose none of the families natively in
/// the pinned hermetic profiles.
const NATIVE_FACT_SOURCES: &[(&str, &str)] = &[("opi", "native/evidence/records")];

/// Sealed control-evidence and execution keys shared with the runner.
const CONTROL_EXPERIMENT_KEY: &str = "control/experiment.json";
const CONTROL_INTEGRITY_KEY: &str = "control/integrity.json";
/// The sealed per-trial settlement record: the runner's final outcome
/// classification (P19-D06). The report consumes it instead of
/// re-deriving a second failure class from stderr tokens or exits.
const CONTROL_SETTLEMENT_KEY: &str = "control/settlement.json";
const TRAJECTORY_KEY: &str = "evidence/trajectory.json";
const AUTHORITY_LEDGER_KEY: &str = "native/authority-ledger.json";
/// The bound result carrying native-mode rewards, distinct from collection
/// manifests, configs, and process streams.
const NATIVE_RESULT_KEY: &str = "native/trial-result.json";

/// The offline report builder over one run root.
pub(crate) struct ReportBuilder {
    run_root: PathBuf,
}

/// One reconstructed sealed trial.
#[derive(Debug)]
struct SealedTrial {
    /// Durable trial id (the bundle directory and the manifest intent).
    id: String,
    /// Owning subject id from the sealed experiment contract.
    subject: String,
    /// Benchmark task id from the sealed experiment contract.
    task: String,
    /// Trial group from the sealed experiment contract.
    group: String,
    /// The manifest's artifact entries (logical key to entry).
    entries: BTreeMap<String, serde_json::Value>,
    /// The sealed trajectory projection.
    trajectory: serde_json::Value,
    /// The sealed authority ledger.
    ledger: serde_json::Value,
    /// The sealed settlement record when the runner staged one; legacy
    /// bundles seal without it and classification falls back to the
    /// sealed headline and ledger observation.
    settlement: Option<serde_json::Value>,
    /// Exact raw native result admitted as the sealed headline source.
    native_result: Option<serde_json::Value>,
    /// Native output limits require a sealed snapshot bound to this trial's controls.
    output_control_proven: bool,
}

/// One recomputed normalized view of a run root.
#[derive(Debug)]
pub(crate) struct NormalizedReport {
    /// The sealed resolved-experiment contract every trial ran under.
    experiment: serde_json::Value,
    /// The sealed integrity record every trial ran under.
    integrity: serde_json::Value,
    /// Content-addressed identity of the sealed integrity record,
    /// recomputed from its canonical bytes.
    integrity_digest: String,
    trials: Vec<SealedTrial>,
    /// Unverified lifecycle diagnostics, excluded from grades and seals.
    unsealed: Vec<TrialView>,
    /// Ids of trials that started (a durable receipt exists), sealed or
    /// not: the attempted set of the planned coverage.
    attempted_ids: Vec<String>,
    /// The run-level planned-coverage artifact bound before any effect
    /// (`P19-D06`): planned trial ids, selection, and corpus size under
    /// the resolved experiment identity. Absent on legacy roots.
    run_control: Option<serde_json::Value>,
    /// Typed failures that block publication (bundle verification or
    /// sealed-input parse failures). Empty when every contributing bundle
    /// verified and parsed.
    failures: Vec<serde_json::Value>,
}

/// One recomputed trial view (rendered per trial in the report).
#[derive(Debug, Serialize)]
struct TrialView {
    id: String,
    subject: String,
    task: String,
    group: String,
    status: String,
    /// The sealed settlement classification: graded (with its Agent
    /// failure and timeout flags) or invalid with its sealed reason.
    outcome: serde_json::Value,
    /// Outcome headline: only from the grader-sourced admitted native
    /// report artifact (`EVAL-RPT-003`). Absent when the grader never
    /// admitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    headline: Option<HeadlineView>,
    /// Native test counters and opaque fields, separate from the task reward.
    native_grader: serde_json::Value,
    /// Asymmetric native facts: measured or typed unknown (`EVAL-A16`).
    native_facts: BTreeMap<String, serde_json::Value>,
    /// Separately labelled diagnostics: sealed agent and verifier
    /// observations and the sealed authority transition states, never
    /// mixed into the headline.
    diagnostics: serde_json::Value,
}

/// The native-grader headline with its provenance citation.
#[derive(Debug, Serialize)]
struct HeadlineView {
    /// The native reward fact token (measured value or typed unknown).
    reward: serde_json::Value,
    /// Where the headline came from: the grader-sourced sealed native
    /// artifact and its digest.
    native_source: BTreeMap<&'static str, String>,
}

/// Failures reported by the report path.
#[derive(Debug)]
pub(crate) enum ReportError {
    /// The run root could not be read.
    Io(std::io::Error),
}

impl std::error::Error for ReportError {}

impl std::fmt::Display for ReportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportError::Io(error) => write!(f, "cannot read run root: {error}"),
        }
    }
}

/// The declared canary-secret guard (`EVAL-A18`, `EVAL-SEC-005`).
pub(crate) struct RedactionGuard {
    tokens: Vec<String>,
}

impl RedactionGuard {
    /// No declared canaries: nothing to scan. Publication still verifies
    /// every bundle; callers that declare secrets scan with
    /// [`RedactionGuard::from_declared_file`].
    pub(crate) fn none() -> Self {
        Self { tokens: Vec::new() }
    }

    /// Loads declared canary secrets from a file: one secret per line,
    /// trimmed, blank lines ignored. An unreadable or empty declaration is
    /// an error, never a silently empty guard.
    pub(crate) fn from_declared_file(path: &Path) -> Result<Self, ReportError> {
        let text = std::fs::read_to_string(path).map_err(ReportError::Io)?;
        let tokens: Vec<String> = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect();
        if tokens.is_empty() {
            return Err(ReportError::Io(std::io::Error::other(format!(
                "canary declaration {} holds no secrets",
                path.display()
            ))));
        }
        Ok(Self { tokens })
    }

    /// Whether any declared canary is contained in `content`.
    fn find_leak(&self, content: &[u8]) -> bool {
        self.tokens.iter().any(|token| {
            !content.is_empty()
                && !token.is_empty()
                && content
                    .windows(token.len())
                    .any(|window| window == token.as_bytes())
        })
    }
}

impl ReportBuilder {
    /// Creates the builder over one run root. Constructing it starts
    /// nothing.
    pub(crate) fn new(run_root: &Path) -> Self {
        Self {
            run_root: run_root.to_path_buf(),
        }
    }

    /// Recomputes grades from verified sealed bundles and retains unsealed
    /// failure coverage separately:
    /// every sealed trial bundle is re-verified from its durable bytes,
    /// and every reconstructed fact is parsed from the sealed control,
    /// trajectory, ledger, and manifest evidence. Nothing is inferred from
    /// memory; unsealed control records never establish grades. Nothing re-runs.
    pub(crate) fn recompute_from_bundle(&self) -> Result<NormalizedReport, ReportError> {
        // The run-level planned-coverage binding (P19-D06): written
        // before any effect, it keeps the planned denominator
        // reconstructible even when no trial sealed.
        let mut failures = Vec::new();
        let run_control = match self.read_run_control() {
            Ok(control) => control,
            Err(kind) => {
                failures.push(serde_json::json!({"trial": null, "kind": kind}));
                None
            }
        };
        let trials_root = match self.checked_input_path(Path::new("trials")) {
            Ok(path) => Some(path),
            Err(kind) => {
                failures.push(serde_json::json!({"trial": null, "kind": kind}));
                None
            }
        };
        if trials_root.as_ref().is_none_or(|path| !path.is_dir())
            && run_control.is_none()
            && failures.is_empty()
        {
            return Err(ReportError::Io(std::io::Error::other(format!(
                "run root {} holds no trials directory",
                self.run_root.display()
            ))));
        }
        let mut ids: Vec<String> =
            if let Some(trials_root) = trials_root.filter(|path| path.is_dir()) {
                std::fs::read_dir(&trials_root)
                    .map_err(ReportError::Io)?
                    .flatten()
                    .filter(|entry| {
                        entry
                            .file_type()
                            .is_ok_and(|kind| kind.is_dir() || kind.is_symlink())
                    })
                    .filter_map(|entry| entry.file_name().into_string().ok())
                    .collect()
            } else {
                Vec::new()
            };
        ids.sort();

        // A manifest claim remains an attempt even when verification fails.
        // Unsealed durable settlements are added after their binding checks.
        let mut attempted_ids: Vec<String> = Vec::new();

        let mut trials = Vec::new();
        for id in &ids {
            let manifest = match self
                .checked_input_path(&Path::new("trials").join(id).join("bundle/manifest.json"))
            {
                Ok(path) => path,
                Err(kind) => {
                    failures.push(serde_json::json!({"trial": id, "kind": kind}));
                    continue;
                }
            };
            let bundle_root = manifest.parent().expect("manifest has bundle parent");
            if !manifest.is_file() {
                // Reconstruct unsealed lifecycle evidence separately, after
                // resolving the common experiment and planned denominator.
                continue;
            }
            attempted_ids.push(id.clone());
            if let Err(error) = RunBundle::verify(bundle_root) {
                failures.push(verification_failure(id, &error));
                continue;
            }
            match self.sealed_trial(id, bundle_root) {
                Ok(trial) => trials.push(trial),
                Err(kind) => failures.push(serde_json::json!({
                    "trial": id,
                    "kind": kind,
                })),
            }
        }
        if trials.is_empty() && run_control.is_none() {
            failures.push(serde_json::json!({
                "trial": null,
                "kind": "no-sealed-trials",
            }));
        }

        // Every sealed trial of one run root must share the exact control
        // evidence: a run root mixing experiments or admissions is a
        // verification failure, never silently normalized.
        let mut experiment_bytes: Option<Vec<u8>> = None;
        let mut integrity_bytes: Vec<u8> = Vec::new();
        let mut experiment = serde_json::Value::Null;
        let mut integrity = serde_json::Value::Null;
        for trial in &trials {
            let trial_experiment = self.sealed_bytes(&trial.id, CONTROL_EXPERIMENT_KEY);
            let trial_integrity = self.sealed_bytes(&trial.id, CONTROL_INTEGRITY_KEY);
            let (trial_experiment, trial_integrity) = match (trial_experiment, trial_integrity) {
                (Ok(experiment), Ok(integrity)) => (experiment, integrity),
                _ => {
                    failures.push(serde_json::json!({
                        "trial": trial.id,
                        "kind": "control-evidence-missing",
                    }));
                    continue;
                }
            };
            match &experiment_bytes {
                None => {
                    experiment = serde_json::from_slice(&trial_experiment).map_err(|_| {
                        ReportError::Io(std::io::Error::other(
                            "sealed experiment contract unparsable",
                        ))
                    })?;
                    integrity = serde_json::from_slice(&trial_integrity).map_err(|_| {
                        ReportError::Io(std::io::Error::other("sealed integrity record unparsable"))
                    })?;
                    experiment_bytes = Some(trial_experiment);
                    integrity_bytes = trial_integrity;
                }
                Some(first_experiment)
                    if trial_experiment != *first_experiment
                        || trial_integrity != integrity_bytes =>
                {
                    failures.push(serde_json::json!({
                        "trial": trial.id,
                        "kind": "control-drift",
                    }));
                }
                _ => {}
            }
        }

        // Every sealed trial must be declared by the sealed experiment
        // contract it claims to have run under.
        for trial in &trials {
            let declared = experiment["trials"].as_array().is_some_and(|entries| {
                entries
                    .iter()
                    .any(|entry| entry["id"].as_str() == Some(trial.id.as_str()))
            });
            if !declared {
                failures.push(serde_json::json!({
                    "trial": trial.id,
                    "kind": "undeclared-trial",
                }));
            }
        }
        // The complete frozen snapshot is authoritative when no trial
        // sealed (P19-AUD-004): an interrupted /2 run still reports its
        // declared topology, native benchmark identity, exclusions and
        // controls. `/1` roots keep the legacy incomplete meaning: no
        // contract was frozen, so no topology is invented.
        if experiment_bytes.is_none()
            && let Some(control) = &run_control
            && control["schema"] == "opi-eval-run-control/2"
            && let (Ok(contract), Ok(record)) = (
                serde_json::from_value::<serde_json::Value>(control["experiment_contract"].clone()),
                serde_json::from_value::<serde_json::Value>(control["integrity_record"].clone()),
            )
        {
            experiment = contract;
            integrity = record;
            integrity_bytes = serde_json::to_vec(&integrity).unwrap_or_default();
        }
        let integrity_digest = if integrity_bytes.is_empty() {
            String::new()
        } else {
            sha256_hex(&integrity_bytes)
        };
        // The run-level binding must match the sealed control evidence:
        // a drifted run-control identity never silently normalizes a
        // run root (`P19-D06`), and a `/2` frozen contract or integrity
        // record that drifted from the sealed bytes refuses publication
        // (P19-AUD-004).
        if let Some(control) = &run_control
            && !trials.is_empty()
            && (!control_matches_experiment(control, &experiment)
                || (control["schema"] == "opi-eval-run-control/2"
                    && (control["experiment_contract"] != experiment
                        || control["integrity_record"] != integrity)))
        {
            failures.push(serde_json::json!({
                "trial": null,
                "kind": "run-control-drift",
            }));
        }
        let mut unsealed = Vec::new();
        let mut diagnostic_experiment = experiment.clone();
        for id in &ids {
            if attempted_ids.contains(id) || failures.iter().any(|failure| failure["trial"] == *id)
            {
                continue;
            }
            match self.unsealed_trial(id, &diagnostic_experiment, run_control.as_ref()) {
                Ok(Some((trial, contract))) => {
                    diagnostic_experiment = contract;
                    attempted_ids.push(id.clone());
                    unsealed.push(trial);
                }
                Ok(None) => {}
                Err(kind) => failures.push(serde_json::json!({"trial": id, "kind": kind})),
            }
        }
        attempted_ids.sort();
        Ok(NormalizedReport {
            experiment,
            integrity,
            integrity_digest,
            trials,
            unsealed,
            attempted_ids,
            run_control,
            failures,
        })
    }

    /// A durable settlement proves an attempt, but without a seal its
    /// classification is diagnostic only and cannot establish a reward.
    fn unsealed_trial(
        &self,
        id: &str,
        experiment: &serde_json::Value,
        run_control: Option<&serde_json::Value>,
    ) -> Result<Option<(TrialView, serde_json::Value)>, String> {
        let relative = Path::new("trials").join(id).join("bundle");
        let bundle = self.checked_input_path(&relative)?;
        self.checked_input_path(&relative.join("intent.json"))?;
        self.checked_input_path(&relative.join("settlement.json"))?;
        let recovered = RunBundle::recover(&bundle).map_err(|_| "unsealed-lifecycle-unparsable")?;
        let Some(marker) = recovered.settlement else {
            return Ok(None);
        };
        if marker.trial.as_str() != id
            || recovered
                .intent
                .as_ref()
                .is_none_or(|intent| intent.trial.as_str() != id)
        {
            return Err("unsealed-trial-drift".to_owned());
        }
        let declared = experiment["trials"]
            .as_array()
            .or_else(|| run_control.and_then(|control| control["planned_trials"].as_array()))
            .and_then(|trials| trials.iter().find(|trial| trial["id"] == id))
            .ok_or("undeclared-unsealed-trial")?;
        let staged_experiment = read_optional_json(
            &self.checked_input_path(&relative.join("artifacts").join(CONTROL_EXPERIMENT_KEY))?,
        )?
        .ok_or("unsealed-experiment-missing")?;
        if (!experiment.is_null() && staged_experiment != *experiment)
            || run_control
                .is_some_and(|control| !control_matches_experiment(control, &staged_experiment))
        {
            return Err("unsealed-control-drift".to_owned());
        }
        let pair = serde_json::to_value(
            &recovered
                .intent
                .as_ref()
                .expect("intent checked above")
                .pair,
        )
        .expect("pair serializes");
        let owns_pair = staged_experiment["edges"].as_array().is_some_and(|edges| {
            edges.iter().any(|edge| {
                edge["id"] == pair
                    && (edge["baseline"] == declared["subject"]
                        || edge["candidate"] == declared["subject"])
            }) || (edges.is_empty()
                && pair == format!("solo:{}", declared["subject"].as_str().unwrap_or_default()))
        });
        if !owns_pair {
            return Err("unsealed-pair-drift".to_owned());
        }
        let settlement = read_optional_json(
            &self.checked_input_path(&relative.join("artifacts").join(CONTROL_SETTLEMENT_KEY))?,
        )?
        .ok_or("unsealed-settlement-missing")?;
        if settlement["schema"] != "opi-eval-trial-settlement/1"
            || settlement["trial"] != id
            || settlement["graded"].as_bool().is_none()
            || settlement["agent_failure"].as_bool().is_none()
            || settlement["timed_out"].as_bool().is_none()
            || (settlement["graded"] == false
                && (settlement["agent_failure"] != false
                    || settlement["failure_kind"]
                        .as_str()
                        .is_none_or(str::is_empty)))
            || (settlement["graded"] == true && !settlement["failure_kind"].is_null())
        {
            return Err("unsealed-settlement-invalid".to_owned());
        }
        // An optional outer receipt can contradict an unsealed settlement,
        // but can never supply a grade or override the durable control facts.
        if let Some(receipt) = read_optional_json(
            &self.checked_input_path(&Path::new("trials").join(id).join("receipt.json"))?,
        )? {
            let class = &receipt["classification"];
            let agrees = if settlement["graded"] == true {
                class["class"] == "graded-native"
                    && class["agent_failed"] == settlement["agent_failure"]
                    && class["timed_out"] == settlement["timed_out"]
            } else {
                matches!(
                    class["class"].as_str(),
                    Some("invalid-execution" | "no-outcome")
                ) && class["reason"] == settlement["failure_kind"]
                    && class["boundary"] == settlement["boundary"]
            };
            if receipt["schema"] != "opi-eval-trial-receipt/1"
                || receipt["id"] != id
                || receipt["status"] != "failed"
                || ["subject", "task", "group"]
                    .iter()
                    .any(|key| receipt[*key] != declared[*key])
                || !receipt["bundle_identity"].is_null()
                || !receipt["seal_result"]["sealed"].is_null()
                || !agrees
            {
                return Err("unsealed-receipt-drift".to_owned());
            }
        }
        let reason = if settlement["graded"] == true {
            serde_json::json!("unsealed-grade")
        } else {
            settlement["failure_kind"].clone()
        };
        Ok(Some((
            TrialView {
                id: id.to_owned(),
                subject: declared["subject"].as_str().unwrap_or_default().to_owned(),
                task: declared["task"].as_str().unwrap_or_default().to_owned(),
                group: declared["group"].as_str().unwrap_or_default().to_owned(),
                status: "unsealed".to_owned(),
                outcome: serde_json::json!({
                    "class": "invalid", "reason": reason, "boundary": settlement["boundary"],
                    "source": "unsealed-settlement",
                }),
                headline: None,
                native_grader: native_grader_observation(None, None, false),
                native_facts: BTreeMap::new(),
                diagnostics: serde_json::json!({
                    "label": "diagnostic", "evidence": "unverified",
                    "artifact": CONTROL_SETTLEMENT_KEY,
                }),
            },
            staged_experiment,
        )))
    }

    /// Only absence permits legacy binding; malformed coverage never
    /// silently replaces a declared denominator with the surviving bundles.
    /// `/1` roots keep their legacy read-only meaning (identity and
    /// planned slots only); `/2` additionally validates the complete
    /// frozen experiment/integrity snapshot persisted before the first
    /// intent (P19-AUD-004).
    fn read_run_control(&self) -> Result<Option<serde_json::Value>, String> {
        let Some(control) =
            read_optional_json(&self.checked_input_path(Path::new("run-control.json"))?)?
        else {
            return Ok(None);
        };
        if !matches!(
            control["schema"].as_str(),
            Some("opi-eval-run-control/1" | "opi-eval-run-control/2")
        ) || control["experiment"].as_str().is_none_or(str::is_empty)
            || control["manifest_digest"].as_str().is_none_or(|digest| {
                digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit())
            })
            || trial_slots(&control["planned_trials"]).is_none()
        {
            return Err("run-control-invalid".to_owned());
        }
        if control["schema"] == "opi-eval-run-control/2" {
            let contract = &control["experiment_contract"];
            let record = &control["integrity_record"];
            if !contract.is_object()
                || !record.is_object()
                || contract["experiment_id"] != control["experiment"]
                || contract["manifest_digest"] != control["manifest_digest"]
                || trial_slots(&contract["trials"]).is_none()
                || trial_slots(&contract["trials"])
                    .is_some_and(|planned| Some(planned) != trial_slots(&control["planned_trials"]))
            {
                return Err("run-control-invalid".to_owned());
            }
        }
        Ok(Some(control))
    }

    /// Unsealed metadata has no manifest traversal guard. Check every
    /// descendant component before following it, rejecting links and Windows
    /// junctions/reparse points, including dangling leaf links. The supplied
    /// root itself defines the authority boundary.
    fn checked_input_path(&self, relative: &Path) -> Result<PathBuf, String> {
        let mut path = self.run_root.clone();
        for component in relative.components() {
            let std::path::Component::Normal(component) = component else {
                return Err("input-path-escape".to_owned());
            };
            path.push(component);
            match std::fs::symlink_metadata(&path) {
                Ok(metadata) => {
                    let linked = metadata.file_type().is_symlink();
                    #[cfg(windows)]
                    let linked = {
                        use std::os::windows::fs::MetadataExt;
                        linked || metadata.file_attributes() & 0x400 != 0
                    };
                    if linked {
                        return Err("input-path-link".to_owned());
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err("input-path-unreadable".to_owned()),
            }
        }
        Ok(path)
    }

    /// Builds the published report: recompute first, then redact-gate,
    /// then render. Publication is blocked (typed outcome, no normalized
    /// content) when a contributing bundle failed verification or a sealed
    /// input failed to parse, or when a declared canary appears in
    /// exportable sealed content.
    pub(crate) fn build(&self, guard: &RedactionGuard) -> Result<serde_json::Value, ReportError> {
        let normalized = self.recompute_from_bundle()?;
        if !normalized.failures.is_empty() {
            return Ok(unverified_report(&normalized));
        }
        if let Some(leak) = self.scan_canaries(&normalized, guard) {
            return Ok(blocked_report(&normalized, leak));
        }
        Ok(self.render(&normalized))
    }

    /// Scans exportable sealed bundle content for declared canaries. The
    /// leak report carries only the trial, logical artifact key, and canary
    /// name - never the secret itself or a machine-local path.
    fn scan_canaries(
        &self,
        normalized: &NormalizedReport,
        guard: &RedactionGuard,
    ) -> Option<serde_json::Value> {
        for trial in &normalized.unsealed {
            if guard.find_leak(&serde_json::to_vec(trial).expect("trial serializes")) {
                return Some(serde_json::json!({
                    "artifact": CONTROL_SETTLEMENT_KEY,
                    "reason": "declared-canary-present",
                }));
            }
        }
        for trial in &normalized.trials {
            for (key, entry) in &trial.entries {
                if entry.get("classification") != Some(&serde_json::json!("Exportable")) {
                    continue;
                }
                let artifact = bundle_artifact_path(&self.run_root, &trial.id, key);
                if let Ok(content) = std::fs::read(&artifact)
                    && guard.find_leak(&content)
                {
                    // The blocked report names where the leak sits but
                    // never echoes the canary itself.
                    return Some(serde_json::json!({
                        "trial": trial.id,
                        "artifact": key,
                        "reason": "declared-canary-present",
                    }));
                }
            }
        }
        None
    }

    /// Renders the byte-stable normalized report. Per-run evidence
    /// identities (content-addressed bundle identities that legitimately
    /// vary between identical independent runs) are never echoed: the
    /// normalized view cites only stable sealed facts.
    fn render(&self, normalized: &NormalizedReport) -> serde_json::Value {
        let mut trials: Vec<serde_json::Value> = normalized
            .trials
            .iter()
            .map(|trial| {
                serde_json::to_value(self.trial_view(trial, normalized)).expect("trial serializes")
            })
            .collect();
        trials.extend(
            normalized
                .unsealed
                .iter()
                .map(|trial| serde_json::to_value(trial).expect("trial serializes")),
        );
        trials.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
        let coverage = coverage(normalized);
        // The planned denominator: the pre-effect run-control binding
        // when present, else the sealed experiment contract (legacy
        // roots). Unexecuted keeps every planned trial that never left
        // a durable receipt visible (`P19-D06`).
        let planned_ids: Vec<String> = match &normalized.run_control {
            Some(control) => control["planned_trials"]
                .as_array()
                .map(|ids| {
                    ids.iter()
                        .filter_map(|entry| entry["id"].as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
            None => normalized.experiment["trials"]
                .as_array()
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|entry| entry["id"].as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
        };
        let planned_count = planned_ids.len();
        let unexecuted = planned_ids
            .iter()
            .filter(|id| !normalized.attempted_ids.contains(id))
            .count();
        // Outcome classes come from the sealed settlement records:
        // valid graded trials form the metric denominator (zero scores
        // and benchmark-defined Agent timeouts included); attempted
        // trials without a grade stay invalid, never Agent outcomes.
        let valid = normalized
            .trials
            .iter()
            .filter(|trial| outcome_buckets(trial).0)
            .count();
        let agent_failure = normalized
            .trials
            .iter()
            .filter(|trial| outcome_buckets(trial).1)
            .count();
        let invalid = normalized.trials.len().saturating_sub(valid) + normalized.unsealed.len();
        let all_sealed = planned_count > 0 && planned_count == normalized.trials.len();
        let all_comparable = coverage
            .iter()
            .all(|pair| pair["comparability"] == "comparable");
        let experiment_id = normalized.experiment["experiment_id"]
            .as_str()
            .map(serde_json::Value::from)
            .or_else(|| {
                normalized
                    .run_control
                    .as_ref()
                    .and_then(|control| control["experiment"].as_str())
                    .map(serde_json::Value::from)
            })
            .unwrap_or(serde_json::Value::Null);
        let manifest_digest = normalized.experiment["manifest_digest"]
            .as_str()
            .map(serde_json::Value::from)
            .or_else(|| {
                normalized
                    .run_control
                    .as_ref()
                    .and_then(|control| control["manifest_digest"].as_str())
                    .map(serde_json::Value::from)
            })
            .unwrap_or(serde_json::Value::Null);
        let mut verification: BTreeMap<&str, serde_json::Value> = normalized
            .trials
            .iter()
            .map(|trial| (trial.id.as_str(), serde_json::json!({"state": "verified"})))
            .collect();
        verification.extend(
            normalized
                .unsealed
                .iter()
                .map(|trial| (trial.id.as_str(), serde_json::json!({"state": "unsealed"}))),
        );
        // Each benchmark reports its own metric definition and
        // denominator; results from different benchmarks are never
        // averaged into one cross-benchmark headline (P19-AC-008).
        let benchmark = format!(
            "{}/{}",
            normalized.experiment["benchmark"]["name"]
                .as_str()
                .unwrap_or("unknown"),
            normalized.experiment["benchmark"]["revision"]
                .as_str()
                .unwrap_or("unknown")
        );
        let selection_scope = normalized
            .run_control
            .as_ref()
            .and_then(|control| control["selection"]["scope"].as_str())
            .or_else(|| normalized.experiment["selection"]["scope"].as_str());
        let (metric, definition) = match benchmark.as_str() {
            "terminal-bench/2.1" => (
                "native-task-success",
                "native binary task success; zero scores and benchmark-defined Agent timeouts remain valid graded outcomes",
            ),
            "terminal-bench/3.0" => (
                "native-task-success-after-collection",
                "native binary task success after declared artifact collection; zero scores and benchmark-defined Agent timeouts remain valid graded outcomes",
            ),
            "deepswe/v1.1" => (
                "native-resolved-task",
                "native binary resolved task outcome after committed-patch verification; zero scores and benchmark-defined Agent timeouts remain valid graded outcomes",
            ),
            _ => (
                "native-task-outcome",
                "native task outcome for the recorded benchmark revision; zero scores remain valid graded outcomes",
            ),
        };
        let measured_counts = trials
            .iter()
            .filter(|trial| trial["native_grader"]["test_case_fraction"]["state"] == "measured")
            .count();
        serde_json::json!({
            "schema": REPORT_SCHEMA,
            "reporter_version": REPORTER_VERSION,
            "classification": CLASSIFICATION,
            "outcome": OUTCOME_PUBLISHED,
            "experiment": experiment_id,
            "manifest_digest": manifest_digest,
            "integrity_digest": normalized.integrity_digest,
            // The run's own outcome stays machine-visible: incomplete
            // coverage publishes with its exact reason, never silently.
            "run_outcome": if all_sealed && all_comparable { "completed" } else { "incomplete" },
            "planned_binding": {
                "source": if normalized.run_control.is_some() { "run-control" } else { "sealed-contract" },
                "trials": planned_count,
            },
            "selection_scope": selection_scope,
            "coverage_counts": {
                "corpus": normalized.run_control.as_ref()
                    .and_then(|control| control["corpus_tasks"].as_u64()),
                "selected": normalized.run_control.as_ref()
                    .and_then(|control| {
                        if control["selection"]["scope"] == "full" {
                            control["corpus_tasks"].as_u64()
                        } else {
                            control["selection"]["task_ids"].as_array()
                                .map(|ids| ids.len() as u64)
                        }
                    }),
                "planned": planned_count,
                "attempted": normalized.attempted_ids.len(),
                "valid": valid,
                "agent_failure": agent_failure,
                "invalid": invalid,
                "unexecuted": unexecuted,
            },
            "metrics": [
                {
                    "benchmark": benchmark,
                    "metric": metric,
                    "definition": definition,
                    "domain": [0, 1],
                    "denominator": valid,
                },
                {
                    "benchmark": benchmark,
                    "metric": "native-test-case-fraction",
                    "definition": "per-trial native passed / total; independent of binary task outcome, without cross-trial pooling",
                    "observation_coverage": {"eligible": valid, "measured": measured_counts, "unknown": valid - measured_counts},
                },
            ],
            "trials": trials,
            "coverage": coverage,
            "control_pairs": control_pairs(normalized),
            "bundle_verification": verification,
        })
    }

    /// Reads one sealed trial's control, trajectory, and ledger evidence.
    fn sealed_trial(&self, id: &str, bundle_root: &Path) -> Result<SealedTrial, String> {
        let trajectory: serde_json::Value = serde_json::from_slice(
            &self
                .sealed_bytes(id, TRAJECTORY_KEY)
                .map_err(|_| "trajectory-missing".to_owned())?,
        )
        .map_err(|_| "trajectory-unparsable".to_owned())?;
        let ledger: serde_json::Value = serde_json::from_slice(
            &self
                .sealed_bytes(id, AUTHORITY_LEDGER_KEY)
                .map_err(|_| "ledger-missing".to_owned())?,
        )
        .map_err(|_| "ledger-unparsable".to_owned())?;
        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(bundle_root.join("manifest.json"))
                .map_err(|_| "manifest-unreadable".to_owned())?,
        )
        .map_err(|_| "manifest-unparsable".to_owned())?;
        let entries: BTreeMap<String, serde_json::Value> = manifest["entries"]
            .as_object()
            .ok_or("manifest-without-entries")?
            .clone()
            .into_iter()
            .collect();
        let experiment: serde_json::Value = serde_json::from_slice(
            &self
                .sealed_bytes(id, CONTROL_EXPERIMENT_KEY)
                .map_err(|_| "experiment-missing".to_owned())?,
        )
        .map_err(|_| "experiment-unparsable".to_owned())?;
        let declared = experiment["trials"]
            .as_array()
            .and_then(|trials| {
                trials
                    .iter()
                    .find(|entry| entry["id"].as_str() == Some(id))
                    .cloned()
            })
            .ok_or("undeclared-trial".to_owned())?;
        let mut trial = SealedTrial {
            id: id.to_owned(),
            subject: declared["subject"].as_str().unwrap_or_default().to_owned(),
            task: declared["task"].as_str().unwrap_or_default().to_owned(),
            group: declared["group"].as_str().unwrap_or_default().to_owned(),
            entries,
            trajectory,
            ledger,
            settlement: self
                .sealed_bytes(id, CONTROL_SETTLEMENT_KEY)
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok()),
            native_result: None,
            output_control_proven: false,
        };
        if trial.entries.contains_key("control/output-control.json") {
            let bytes = self
                .sealed_bytes(id, "control/output-control.json")
                .map_err(|_| "output-control-missing")?;
            trial.output_control_proven =
                sealed_output_control(&bytes, &experiment, &trial.subject)
                    .ok_or("output-control-invalid")?;
        }
        if let Some(grader) = trajectory_node_matching(&trial.trajectory, "grader/")
            && grader["facts"]["reward"]["Known"]["origin"] == "native-trial-result"
        {
            // A digest proves retained bytes, not that a collection or
            // configuration artifact actually carries the projected reward.
            let result: serde_json::Value = serde_json::from_slice(
                &self
                    .sealed_bytes(id, NATIVE_RESULT_KEY)
                    .map_err(|_| "native-reward-source-missing")?,
            )
            .map_err(|_| "native-reward-source-unparsable")?;
            let reward = result["verifier_result"]["rewards"]["reward"].as_f64();
            if headline(&trial).is_none()
                || reward.is_none()
                || reward != grader["facts"]["reward"]["Known"]["value"].as_f64()
            {
                return Err("native-reward-source-mismatch".to_owned());
            }
            trial.native_result = Some(result);
        }
        Ok(trial)
    }

    /// Reads the sealed artifact bytes of one key of one trial.
    fn sealed_bytes(&self, trial: &str, key: &str) -> std::io::Result<Vec<u8>> {
        std::fs::read(bundle_artifact_path(&self.run_root, trial, key))
    }

    /// Recomputes one trial view from the sealed evidence.
    fn trial_view(&self, trial: &SealedTrial, normalized: &NormalizedReport) -> TrialView {
        let product = subject_product(normalized, &trial.subject);
        TrialView {
            id: trial.id.clone(),
            subject: trial.subject.clone(),
            task: trial.task.clone(),
            group: trial.group.clone(),
            status: "sealed".to_owned(),
            outcome: outcome_view(trial),
            headline: headline(trial),
            native_grader: native_grader_observation(
                trial.native_result.as_ref(),
                trial
                    .native_result
                    .as_ref()
                    .and_then(|_| trial.entries.get(NATIVE_RESULT_KEY))
                    .and_then(|entry| entry["digest"].as_str()),
                outcome_buckets(trial).0,
            ),
            native_facts: native_facts(&product, &trial.entries),
            diagnostics: diagnostics(trial, &product),
        }
    }
}

/// The sealed settlement classification of one trial: graded (a valid
/// graded Agent outcome, zero scores and benchmark-defined timeouts
/// included) or invalid with its sealed reason. The sealed record is
/// authoritative; a legacy bundle without one falls back to the sealed
/// headline plus the ledger's agent observation - never to stderr
/// tokens or process exits (P19-D06, P19-AC-008).
fn outcome_view(trial: &SealedTrial) -> serde_json::Value {
    if let Some(settlement) = &trial.settlement {
        if settlement["graded"] == serde_json::Value::Bool(true) {
            return serde_json::json!({
                "class": "graded",
                "agent_failed": settlement["agent_failure"].as_bool().unwrap_or(false),
                "timed_out": settlement["timed_out"].as_bool().unwrap_or(false),
                "source": "settlement",
            });
        }
        return serde_json::json!({
            "class": "invalid",
            "reason": settlement["failure_kind"].clone(),
            "boundary": settlement["boundary"].clone(),
            "source": "settlement",
        });
    }
    // Legacy fallback: a graded headline is a valid graded outcome whose
    // Agent-failure flag comes from the sealed ledger observation.
    if headline(trial).is_some() {
        return serde_json::json!({
            "class": "graded",
            "agent_failed": trial.ledger["agent_outcome"] == "scored_failure",
            "timed_out": false,
            "source": "legacy-derived",
        });
    }
    serde_json::json!({
        "class": "invalid",
        "reason": trial.ledger["failed_boundary"].clone(),
        "boundary": trial.ledger["failed_boundary"].clone(),
        "source": "legacy-derived",
    })
}

/// Optional unverified metadata is absent only on NotFound; bad bytes or
/// unreadable records cannot silently erase coverage or contradictions.
fn read_optional_json(path: &Path) -> Result<Option<serde_json::Value>, String> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_| "control-unparsable".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err("control-unreadable".to_owned()),
    }
}

/// Exact trial identities, independent of declaration order; duplicates or
/// incomplete slots cannot establish a denominator.
fn trial_slots(value: &serde_json::Value) -> Option<BTreeMap<String, Vec<String>>> {
    let mut slots = BTreeMap::new();
    for trial in value.as_array()? {
        let id = trial["id"].as_str().filter(|id| !id.is_empty())?;
        let fields = ["subject", "task", "group"]
            .iter()
            .map(|key| {
                trial[*key]
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
            })
            .collect::<Option<Vec<_>>>()?;
        if slots.insert(id.to_owned(), fields).is_some() {
            return None;
        }
    }
    Some(slots)
}

fn control_matches_experiment(control: &serde_json::Value, experiment: &serde_json::Value) -> bool {
    control["manifest_digest"] == experiment["manifest_digest"]
        && control["experiment"] == experiment["experiment_id"]
        && ["scope", "task_ids", "repetitions"]
            .iter()
            .all(|key| control["selection"][*key] == experiment["selection"][*key])
        && trial_slots(&control["planned_trials"])
            .is_some_and(|planned| Some(planned) == trial_slots(&experiment["trials"]))
}

/// Whether one sealed trial is a valid graded outcome (the metric
/// denominator member) and whether it counts as an Agent failure.
fn outcome_buckets(trial: &SealedTrial) -> (bool, bool) {
    let outcome = outcome_view(trial);
    (
        outcome["class"] == "graded",
        outcome["class"] == "graded" && outcome["agent_failed"] == serde_json::Value::Bool(true),
    )
}

/// The product of one subject id from the sealed experiment contract.
fn subject_product(normalized: &NormalizedReport, subject: &str) -> String {
    normalized.experiment["subjects"]
        .as_array()
        .and_then(|subjects| {
            subjects
                .iter()
                .find(|entry| entry["id"].as_str() == Some(subject))
        })
        .and_then(|entry| entry["product"].as_str())
        .unwrap_or_default()
        .to_owned()
}

/// Absolute on-disk location of one sealed artifact.
fn bundle_artifact_path(run_root: &Path, trial: &str, key: &str) -> PathBuf {
    run_root
        .join("trials")
        .join(trial)
        .join("bundle/artifacts")
        .join(key)
}

/// Maps one bundle verification failure to its typed report token.
fn verification_failure(trial: &str, error: &crate::bundle::BundleError) -> serde_json::Value {
    use crate::bundle::BundleError;
    let (kind, artifact): (&str, Option<String>) = match error {
        BundleError::DigestMismatch { key, .. } => {
            ("digest-mismatch", Some(key.as_str().to_owned()))
        }
        BundleError::SymlinkEscape { key, .. } => ("symlink-escape", Some(key.as_str().to_owned())),
        BundleError::ManifestInvalid { .. } => ("manifest-invalid", None),
        BundleError::SidecarDrift { which, .. } => ("sidecar-drift", Some((*which).to_owned())),
        BundleError::UnmanifestedFile { path, .. } => ("unmanifested-file", Some(path.clone())),
        BundleError::MissingArtifact { key, .. } => {
            ("missing-artifact", Some(key.as_str().to_owned()))
        }
        BundleError::Io { .. } => ("io", None),
        _ => ("lifecycle-invalid", None),
    };
    serde_json::json!({
        "trial": trial,
        "kind": kind,
        "artifact": artifact,
    })
}

/// The first trajectory node whose kind tag equals `tag`.
fn trajectory_node<'a>(
    trajectory: &'a serde_json::Value,
    tag: &str,
) -> Option<&'a serde_json::Value> {
    trajectory["nodes"]
        .as_array()?
        .iter()
        .find(|node| node["kind"].as_str() == Some(tag))
}

/// The first trajectory node whose kind tag starts with `prefix`.
fn trajectory_node_matching<'a>(
    trajectory: &'a serde_json::Value,
    prefix: &str,
) -> Option<&'a serde_json::Value> {
    trajectory["nodes"].as_array()?.iter().find(|node| {
        node["kind"]
            .as_str()
            .is_some_and(|kind| kind.starts_with(prefix))
    })
}

/// The fact-token form of one trajectory fact (`known:v(origin)` or
/// `unknown:reason`), matching the receipt vocabulary.
fn fact_token(fact: &serde_json::Value) -> Option<String> {
    if let Some(known) = fact.get("Known") {
        return Some(format!(
            "known:{}({})",
            known["value"],
            known["origin"].as_str().unwrap_or_default()
        ));
    }
    fact.get("Unknown")
        .map(|unknown| format!("unknown:{}", unknown["reason"].as_str().unwrap_or_default()))
}

/// The native-grader headline of one trial: the reward fact from the
/// sealed trajectory's grader node plus the provenance citation of the
/// grader-sourced native report artifact. Absent when the grader never
/// admitted; a grader node without its grader-sourced artifact yields no
/// headline - the report never guesses a producer.
fn headline(trial: &SealedTrial) -> Option<HeadlineView> {
    let grader = trajectory_node_matching(&trial.trajectory, "grader/")?;
    let reward = fact_token(&grader["facts"]["reward"])?;
    let native_reward = grader["facts"]["reward"]["Known"]["origin"] == "native-trial-result";
    let artifact = trial
        .entries
        .iter()
        .find(|(key, entry)| {
            (if native_reward {
                key.as_str() == NATIVE_RESULT_KEY
            } else {
                matches!(
                    key.as_str(),
                    "native/native/ctrf-report" | "native/native/pier-report"
                )
            }) && entry["role"].as_str() == Some("Native")
                && entry["source"]
                    .as_str()
                    .is_some_and(|source| source.starts_with("grader-"))
        })
        .map(|(key, entry)| {
            (
                key.clone(),
                entry["digest"].as_str().unwrap_or_default().to_owned(),
            )
        })?;
    let mut native_source = BTreeMap::new();
    native_source.insert("artifact", artifact.0);
    native_source.insert("digest", artifact.1);
    Some(HeadlineView {
        reward: serde_json::json!(reward),
        native_source,
    })
}

/// The asymmetric native facts of one product over one sealed bundle: a
/// family is measured only when the product's own native artifact carries
/// it; otherwise it stays the typed unknown `<product>-<family>-not-native`
/// (`EVAL-A16`). Measured facts cite the sealed artifact digest; unknown
/// facts carry no value and no digest.
fn native_facts(
    product: &str,
    entries: &BTreeMap<String, serde_json::Value>,
) -> BTreeMap<String, serde_json::Value> {
    let source = NATIVE_FACT_SOURCES
        .iter()
        .find(|(owner, _)| *owner == product)
        .map(|(_, key)| *key);
    NATIVE_FACT_FAMILIES
        .iter()
        .map(|family| {
            let value = match source {
                Some(key) if *family == "usage" && entries.contains_key(key) => {
                    serde_json::json!({
                        "state": "measured",
                        "artifact": key,
                        "digest": entries[key]["digest"],
                    })
                }
                _ => serde_json::json!({
                    "state": format!("unknown:{product}-{family}-not-native"),
                }),
            };
            ((*family).to_owned(), value)
        })
        .collect()
}

/// Sealed-derived diagnostics: agent and verifier observations from the
/// sealed trajectory and the authority transition states from the sealed
/// ledger, never mixed into the headline. Only stable sealed facts are
/// echoed (timing stays retained in the sealed trajectory itself).
fn diagnostics(trial: &SealedTrial, product: &str) -> serde_json::Value {
    let agent = trajectory_node(&trial.trajectory, "agent-execution");
    let mut agent_view = serde_json::Map::new();
    agent_view.insert("product".to_owned(), serde_json::json!(product));
    agent_view.insert("outcome".to_owned(), trial.ledger["agent_outcome"].clone());
    if let Some(facts) = agent.map(|node| &node["facts"]) {
        if let Some(code) = facts["exit_code"]["Known"]["value"].as_u64() {
            agent_view.insert("exit_code".to_owned(), serde_json::json!(code));
        }
        for name in ["input_tokens", "output_tokens"] {
            if let Some(token) = fact_token(&facts[name]) {
                agent_view.insert(name.to_owned(), serde_json::json!(token));
            }
        }
    }
    let mut view = serde_json::Map::new();
    view.insert("label".to_owned(), serde_json::json!("diagnostic"));
    view.insert("agent".to_owned(), serde_json::Value::Object(agent_view));
    if let Some(verifier) = trajectory_node(&trial.trajectory, "verifier-execution") {
        let mut verifier_view = serde_json::Map::new();
        if let Some(code) = verifier["facts"]["exit_code"]["Known"]["value"].as_u64() {
            verifier_view.insert("exit_code".to_owned(), serde_json::json!(code));
        }
        let completion = trajectory_node_matching(&trial.trajectory, "grader/")
            .map(|node| &node["facts"]["completion"])
            .and_then(|fact| fact.get("Known"))
            .and_then(|known| known["value"].as_u64());
        // The sealed settlement records successful native admission even
        // when an older trajectory projected VerifiedNative as zero.
        let native_graded = trial.settlement.as_ref().is_some_and(|settlement| {
            settlement["graded"] == true
                && trajectory_node_matching(&trial.trajectory, "grader/").is_some_and(|node| {
                    node["facts"]["reward"]["Known"]["origin"] == "native-trial-result"
                })
        });
        if let Some(completion) = completion {
            verifier_view.insert(
                "completion".to_owned(),
                serde_json::json!(if completion == 1 || native_graded {
                    "verified"
                } else {
                    "failed"
                }),
            );
        }
        view.insert(
            "verifier".to_owned(),
            serde_json::Value::Object(verifier_view),
        );
    }
    // The authority map keyed by transition token, mirroring the receipt
    // vocabulary.
    if let Some(records) = trial.ledger["records"].as_array() {
        let mut map = serde_json::Map::new();
        for record in records {
            map.insert(
                record["transition"].as_str().unwrap_or_default().to_owned(),
                record["state"].clone(),
            );
        }
        view.insert("authority".to_owned(), serde_json::Value::Object(map));
    }
    serde_json::Value::Object(view)
}

/// Control pairs (P19-AC-008): every (subject, task) repetition set of
/// the frozen contract with more than one group surfaces as one control
/// pair with its exact state - consistent (all repetition outcomes
/// equal), divergent, or incomplete with its missing groups. Single
/// subject reports carry these instead of any artificial comparison
/// edge; nothing is inferred for groups that never ran.
fn control_pairs(normalized: &NormalizedReport) -> Vec<serde_json::Value> {
    // The declared repetition slots come from the sealed contract, or -
    // when no trial sealed - from the pre-effect run-control binding.
    let declared_trials: Vec<&serde_json::Value> = normalized.experiment["trials"]
        .as_array()
        .map(|entries| entries.iter().collect::<Vec<_>>())
        .or_else(|| {
            normalized
                .run_control
                .as_ref()
                .and_then(|control| control["planned_trials"].as_array())
                .map(|entries| entries.iter().collect::<Vec<_>>())
        })
        .unwrap_or_default();
    // (subject, task) -> ordered distinct (group, declared trial id).
    let mut slots: BTreeMap<(String, String), Vec<(String, String)>> = BTreeMap::new();
    for trial in declared_trials {
        let (Some(subject), Some(task), Some(group), Some(id)) = (
            trial["subject"].as_str(),
            trial["task"].as_str(),
            trial["group"].as_str(),
            trial["id"].as_str(),
        ) else {
            continue;
        };
        slots
            .entry((subject.to_owned(), task.to_owned()))
            .or_default()
            .push((group.to_owned(), id.to_owned()));
    }
    let mut pairs = Vec::new();
    for ((subject, task), mut groups) in slots {
        groups.sort();
        groups.dedup_by(|a, b| a.0 == b.0);
        if groups.len() < 2 {
            continue;
        }
        let mut missing = Vec::new();
        let mut rewards: Vec<serde_json::Value> = Vec::new();
        for (group, id) in &groups {
            match normalized.trials.iter().find(|trial| &trial.id == id) {
                None => missing.push(serde_json::json!(group)),
                Some(sealed) => rewards.push(
                    headline(sealed)
                        .map(|headline| headline.reward)
                        .unwrap_or(serde_json::Value::Null),
                ),
            }
        }
        let state = if !missing.is_empty() {
            "incomplete"
        } else if rewards.iter().all(|reward| reward == &rewards[0]) {
            "consistent"
        } else {
            "divergent"
        };
        pairs.push(serde_json::json!({
            "subject": subject,
            "task": task,
            "groups": groups.iter().map(|(group, _)| group).collect::<Vec<_>>(),
            "state": state,
            "missing_groups": missing,
        }));
    }
    pairs
}

/// Pair coverage derived from sealed content only (`EVAL-RPT-004`,
/// `EVAL-EXP-006`): every edge's pairing universe over the sealed
/// experiment's declared (task, group) slots, with the exact visible state
/// of each side derived from the sealed bundles and the sealed integrity
/// record. A scored Agent failure stays a comparable graded Agent outcome.
fn coverage(normalized: &NormalizedReport) -> Vec<serde_json::Value> {
    let mut pairs = Vec::new();
    let Some(edges) = normalized.experiment["edges"].as_array() else {
        return pairs;
    };
    let Some(declared_trials) = normalized.experiment["trials"].as_array() else {
        return pairs;
    };
    let subject_product = |id: &str| {
        normalized.experiment["subjects"]
            .as_array()
            .and_then(|subjects| {
                subjects
                    .iter()
                    .find(|entry| entry["id"].as_str() == Some(id))
            })
            .and_then(|entry| entry["product"].as_str().map(str::to_owned))
    };
    for edge in edges {
        // Protocol-safe pairing (P19-AC-008): an edge pairing two protocol
        // groups of one product never carries a paired claim.
        let protocol_mismatch = match (
            edge["baseline"].as_str().and_then(&subject_product),
            edge["candidate"].as_str().and_then(&subject_product),
        ) {
            (Some(baseline), Some(candidate)) => baseline == candidate,
            _ => false,
        };
        let mut keys: Vec<(String, String)> = declared_trials
            .iter()
            .filter(|trial| {
                trial["subject"] == edge["baseline"] || trial["subject"] == edge["candidate"]
            })
            .filter_map(|trial| {
                Some((
                    trial["task"].as_str()?.to_owned(),
                    trial["group"].as_str()?.to_owned(),
                ))
            })
            .collect();
        keys.sort();
        keys.dedup();
        for (task, group) in keys {
            let mut baseline_trial = String::new();
            let mut candidate_trial = String::new();
            let mut comparability = if protocol_mismatch {
                "protocol-group-mismatch".to_owned()
            } else {
                "comparable".to_owned()
            };
            // The task's validity classification gates the whole pair:
            // only `ValidAgentOutcome` may carry a paired claim.
            let classification = normalized.integrity["tasks"]
                .as_object()
                .and_then(|tasks| tasks.get(&task));
            match classification {
                None => comparability = "task-not-covered".to_owned(),
                Some(classification) if classification.as_str() != Some("ValidAgentOutcome") => {
                    comparability = "invalid-task-classification".to_owned()
                }
                _ => {}
            }
            let side_trial = |subject: &str, sealed: &mut String| {
                for trial in &normalized.trials {
                    let declared = declared_trials.iter().any(|entry| {
                        entry["id"].as_str() == Some(trial.id.as_str())
                            && entry["subject"].as_str() == Some(subject)
                            && entry["task"].as_str() == Some(task.as_str())
                            && entry["group"].as_str() == Some(group.as_str())
                    });
                    if declared {
                        *sealed = trial.id.clone();
                        return true;
                    }
                }
                if let Some(trial) = normalized.unsealed.iter().find(|trial| {
                    trial.subject == subject && trial.task == task && trial.group == group
                }) {
                    *sealed = trial.id.clone();
                    return true;
                }
                false
            };
            let has_baseline = side_trial(
                edge["baseline"].as_str().unwrap_or_default(),
                &mut baseline_trial,
            );
            let has_candidate = side_trial(
                edge["candidate"].as_str().unwrap_or_default(),
                &mut candidate_trial,
            );
            if comparability == "comparable" && !has_baseline {
                comparability = "missing-baseline-trial".to_owned();
            }
            if comparability == "comparable" && !has_candidate {
                comparability = "missing-candidate-trial".to_owned();
            }
            // Exclusions carry their stable sealed reason.
            if comparability == "comparable"
                && let Some(excluded) = normalized.integrity["excluded_trials"].as_object()
            {
                for trial in [&baseline_trial, &candidate_trial] {
                    if let Some(reason) = excluded.get(trial) {
                        comparability =
                            format!("excluded:{trial}:{}", reason.as_str().unwrap_or_default());
                        break;
                    }
                }
            }
            // Sealed outcome classes are owned by the sealed settlement
            // record: a graded trial (zero scores and scored Agent
            // failures included) stays comparable; an invalid settlement
            // (provider/access, adapter, identity, infrastructure, or
            // invalid grading) downgrades the pair naming the sealed
            // reason. Legacy bundles without a settlement fall back to
            // the ledger observation and grader-completion facts
            // (P19-D06).
            if comparability == "comparable" {
                for trial in [&baseline_trial, &candidate_trial] {
                    if let Some(unsealed) = normalized.unsealed.iter().find(|t| &t.id == trial) {
                        comparability = format!(
                            "invalid:{trial}:{}",
                            unsealed.outcome["reason"].as_str().unwrap_or("unsealed")
                        );
                        break;
                    }
                    if let Some(sealed) = normalized.trials.iter().find(|t| &t.id == trial) {
                        let outcome = outcome_view(sealed);
                        if outcome["source"] == "settlement" {
                            if outcome["class"] == "invalid" {
                                let reason =
                                    outcome["reason"].as_str().unwrap_or("invalid-execution");
                                comparability = format!("invalid:{trial}:{reason}");
                                break;
                            }
                            continue;
                        }
                        // Legacy derivation.
                        if sealed.ledger["agent_outcome"] == "boundary_failure" {
                            comparability = format!("infrastructure-failure:{trial}");
                            break;
                        }
                        let grader_failed =
                            sealed.ledger["records"].as_array().is_some_and(|records| {
                                records.iter().any(|record| {
                                    record["transition"] == "grade_dispatch"
                                        && record["state"] != "executed"
                                })
                            }) || trajectory_node_matching(&sealed.trajectory, "grader/")
                                .map(|node| &node["facts"]["completion"])
                                .and_then(|fact| fact.get("Known"))
                                .and_then(|known| known["value"].as_u64())
                                .is_some_and(|value| value == 0);
                        if grader_failed {
                            comparability = format!("grader-failure:{trial}");
                            break;
                        }
                    }
                }
            }
            // A shared control no pinned subject profile can express: a
            // reasoning VALUE (not the explicit `omitted`/`unknown`
            // markers) is unsupported by every pinned profile.
            if comparability == "comparable"
                && normalized.experiment["model_controls"]["reasoning"]
                    .as_str()
                    .is_some_and(|reasoning| reasoning != "omitted" && reasoning != "unknown")
            {
                comparability = "unsupported-control:reasoning".to_owned();
            }
            if comparability == "comparable"
                && matches!(
                    normalized.experiment["execution"]["mode"].as_str(),
                    Some("live" | "native-fixture")
                )
            {
                let selected =
                    normalized.experiment["connections"]
                        .as_array()
                        .and_then(|connections| {
                            connections
                                .iter()
                                .find(|c| c["id"] == normalized.experiment["connection_id"])
                        });
                let api = selected.and_then(|c| c["api"].as_str());
                let mut unsupported = std::collections::BTreeSet::new();
                for id in [&baseline_trial, &candidate_trial] {
                    if let Some(trial) = normalized.trials.iter().find(|trial| &trial.id == id) {
                        if subject_product(&trial.subject).as_deref() != Some("pi")
                            || !matches!(api, Some("openai-completions" | "openai-responses"))
                        {
                            for name in ["temperature", "top_p"] {
                                if normalized.experiment["model_controls"][name].is_number() {
                                    unsupported.insert(name);
                                }
                            }
                        }
                        if normalized.experiment["model_controls"]["max_output_tokens"].is_number()
                            && !trial.output_control_proven
                        {
                            unsupported.insert("max_output_tokens");
                        }
                    }
                }
                if let Some(control) = unsupported.first() {
                    comparability = format!("unsupported-control:{control}");
                }
            }
            pairs.push(serde_json::json!({
                "edge": edge["id"],
                "task": task,
                "group": group,
                "baseline_trial": baseline_trial,
                "candidate_trial": candidate_trial,
                "comparability": comparability,
            }));
        }
    }
    pairs
}

/// Interpret only verified sealed snapshot bytes; embedded host paths are never followed.
fn sealed_output_control(
    bytes: &[u8],
    experiment: &serde_json::Value,
    subject: &str,
) -> Option<bool> {
    use crate::runner::material::sha256_hex;
    let snapshot: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let evidence = snapshot["evidence"].as_str()?;
    let proof: serde_json::Value = serde_json::from_str(evidence).ok()?;
    let config = snapshot["config"].as_str()?;
    let requests = snapshot["requests"].as_str()?;
    let product = experiment["subjects"]
        .as_array()?
        .iter()
        .find(|entry| entry["id"] == subject)?["product"]
        .as_str()?;
    let connection = experiment["connections"]
        .as_array()?
        .iter()
        .find(|entry| entry["id"] == experiment["connection_id"])?;
    let limit = experiment["model_controls"]["max_output_tokens"].as_u64()?;
    if limit == 0
        || proof["schema"] != "opi-eval-output-control/1"
        || proof["product"] != product
        || proof["exit_code"] != 0
        || proof["model_controls"] != experiment["model_controls"]
        || proof["connection"]
            != serde_json::json!({"provider": connection["provider"], "api": connection["api"], "model": connection["model"]})
        || snapshot["evidence_sha256"] != sha256_hex(evidence.as_bytes())
        || proof["config"]["sha256"] != sha256_hex(config.as_bytes())
        || proof["requests"]["sha256"] != sha256_hex(requests.as_bytes())
    {
        return None;
    }
    let rows = requests
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if rows.len() != 2 {
        return None;
    }
    let (paths, fields): (&[&str], &[&str]) = match connection["api"].as_str()? {
        "anthropic-messages" => (&["/v1/messages", "/v1/messages?beta=true"], &["max_tokens"]),
        "openai-completions" => (
            &["/chat/completions", "/v1/chat/completions"],
            &["max_tokens", "max_completion_tokens"],
        ),
        "openai-responses" => (&["/responses", "/v1/responses"], &["max_output_tokens"]),
        _ => return None,
    };
    for (index, row) in rows.iter().enumerate() {
        let limits = row["output_limits"].as_object()?;
        if row["schema"] != "opi-eval-scripted-provider-log/1"
            || row["seq"].as_u64() != Some(index as u64 + 1)
            || row["status"] != 200
            || row["method"] != "POST"
            || row["model"] != connection["model"]
            || !paths.contains(&row["path"].as_str()?)
            || limits.len() != 1
            || !fields
                .iter()
                .any(|field| limits.get(*field).and_then(serde_json::Value::as_u64) == Some(limit))
        {
            return None;
        }
    }
    Some(true)
}

/// The blocked publication report: typed leak reason, no normalized
/// content, no secret echo.
fn blocked_report(normalized: &NormalizedReport, leak: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "schema": REPORT_SCHEMA,
        "reporter_version": REPORTER_VERSION,
        "classification": CLASSIFICATION,
        "outcome": OUTCOME_BLOCKED,
        "experiment": normalized.experiment["experiment_id"],
        "leak": leak,
    })
}

/// The non-published verification-failure report: every typed failure,
/// no normalized content.
fn unverified_report(normalized: &NormalizedReport) -> serde_json::Value {
    serde_json::json!({
        "schema": REPORT_SCHEMA,
        "reporter_version": REPORTER_VERSION,
        "classification": CLASSIFICATION,
        "outcome": OUTCOME_UNVERIFIED,
        "experiment": normalized.experiment["experiment_id"],
        "failures": normalized.failures,
    })
}

/// Lowercase hex SHA-256 of `bytes`.
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_finds_declared_canaries_and_never_echoes_them_in_structures() {
        let guard = RedactionGuard {
            tokens: vec!["OPZ-CANARY".to_owned()],
        };
        assert!(guard.find_leak(b"prefix OPZ-CANARY suffix"));
        assert!(!guard.find_leak(b"clean content"));
    }

    #[test]
    fn native_facts_are_measured_or_typed_unknown_never_fabricated() {
        let mut entries = BTreeMap::new();
        entries.insert(
            "native/evidence/records".to_owned(),
            serde_json::json!({"digest": "a".repeat(64), "classification": "Exportable"}),
        );
        let opi = native_facts("opi", &entries);
        assert_eq!(opi["usage"]["state"], "measured");
        assert_eq!(opi["cost"]["state"], "unknown:opi-cost-not-native");
        let pi = native_facts("pi", &entries);
        assert_eq!(pi["usage"]["state"], "unknown:pi-usage-not-native");
        assert!(pi["usage"].get("digest").is_none());
        // A product that never staged its own evidence never measures it,
        // even when another product's artifact is present in scope.
        let empty = native_facts("opi", &BTreeMap::new());
        assert_eq!(empty["usage"]["state"], "unknown:opi-usage-not-native");
    }

    #[test]
    fn fact_tokens_match_the_receipt_vocabulary() {
        assert_eq!(
            fact_token(&serde_json::json!({"Known": {"value": 1, "origin": "pier-report"}}))
                .as_deref(),
            Some("known:1(pier-report)")
        );
        assert_eq!(
            fact_token(&serde_json::json!({"Unknown": {"reason": "pending"}})).as_deref(),
            Some("unknown:pending")
        );
        assert_eq!(fact_token(&serde_json::json!({})), None);
    }

    #[test]
    fn headline_selection_requires_the_grader_source_and_native_role() {
        let entries: BTreeMap<String, serde_json::Value> = BTreeMap::from([
            (
                "native/native/ctrf-report".to_owned(),
                serde_json::json!({
                    "role": "Native",
                    "source": "grader-harbor-v0.22.0-fixture",
                    "digest": "a".repeat(64),
                }),
            ),
            (
                "native/verifier-stdout.log".to_owned(),
                serde_json::json!({
                    "role": "Native",
                    "source": "grader-harbor-v0.22.0-fixture",
                    "digest": "b".repeat(64),
                }),
            ),
        ]);
        let trial = SealedTrial {
            id: "trial-1".to_owned(),
            subject: "s".to_owned(),
            task: "t".to_owned(),
            group: "g".to_owned(),
            settlement: None,
            native_result: None,
            output_control_proven: false,
            entries,
            trajectory: serde_json::json!({
                "nodes": [{
                    "kind": "grader/tb/2.1",
                    "facts": {
                        "reward": {"Known": {"value": 1, "origin": "x"}},
                        "completion": {"Known": {"value": 1, "origin": "native-completion"}},
                    },
                }],
            }),
            ledger: serde_json::json!({}),
        };
        // The bounded stream capture is never selected; the grader-sourced
        // report is.
        let selected = headline(&trial).unwrap();
        assert_eq!(
            selected.native_source.get("artifact").map(String::as_str),
            Some("native/native/ctrf-report")
        );

        // A source-role mismatch - the native report attributed to the
        // agent - yields no headline: the report path refuses to guess.
        let mut mismatched = trial;
        mismatched.entries.insert(
            "native/native/ctrf-report".to_owned(),
            serde_json::json!({
                "role": "Native",
                "source": "agent-opi",
                "digest": "a".repeat(64),
            }),
        );
        assert!(headline(&mismatched).is_none());
    }
}
