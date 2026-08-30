//! Normalized offline report over sealed assembled outputs (task 18.13).
//!
//! [`ReportBuilder::recompute_from_bundle`] rebuilds the normalized view
//! purely from verified sealed bundles: every trial view, the pair
//! coverage, the integrity provenance, the native rewards, and the
//! diagnostics are reconstructed from the sealed control evidence (the
//! resolved experiment contract and the integrity record), the sealed
//! provisional trajectory, the sealed authority ledger, and the sealed
//! manifest identities. No mutable side file of the run root is read, so
//! mutating outer run artifacts can never change a published report
//! (`P18-RPT-001`, `P18-OUT-004`). Both paths are effect-free: no Agent,
//! no provider, no spawn, no mutation of sealed bytes.
//!
//! Report contract enforced here: headline outcomes come only from the
//! admitted grader-sourced native report artifact with per-headline
//! provenance (`P18-RPT-003`); pair coverage keeps every declared pair
//! visible with its exact state, so exclusions, failures, and unknowns
//! never leave the denominator silently (`P18-RPT-004`, `P18-EXP-006`);
//! quality, cost, safety, efficiency, and authority are never collapsed
//! into one composite score or best-trial verdict (`P18-RPT-005`); the
//! report labels its evidence `conformance-evidence` and claims no
//! official leaderboard verification (`P18-RPT-006`). Asymmetric native
//! facts stay measured values (cited by sealed artifact digest) or typed
//! unknowns - never fabricated parity (`P18-A16`). Declared canary
//! secrets in exportable bundle content block publication (`P18-A18`,
//! `P18-SEC-005`). Identical sealed inputs and tool identities serialize
//! to byte-identical output (`P18-RPT-002`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::bundle::RunBundle;

/// Normalized report schema identity.
const REPORT_SCHEMA: &str = "phase18-normalized-report/1";
/// Pinned reporter identity: part of every byte-stability contract
/// (`P18-RPT-002` - same sealed inputs, grader identity, and reporter
/// version).
pub(crate) const REPORTER_VERSION: &str = "phase18-reporter/1";
/// The single classification wording of Phase 18 paired results
/// (`P18-RPT-006`): conformance evidence only, never leaderboard
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
/// (`P18-A16`).
const NATIVE_FACT_FAMILIES: [&str; 4] = ["usage", "cost", "retry", "compaction"];

/// Which sealed native artifact carries one fact family for one product.
/// Products absent from this table expose none of the families natively in
/// the pinned hermetic profiles.
const NATIVE_FACT_SOURCES: &[(&str, &str)] = &[("opi", "native/evidence/records")];

/// Sealed control-evidence and execution keys shared with the runner.
const CONTROL_EXPERIMENT_KEY: &str = "control/experiment.json";
const CONTROL_INTEGRITY_KEY: &str = "control/integrity.json";
const TRAJECTORY_KEY: &str = "evidence/trajectory.json";
const AUTHORITY_LEDGER_KEY: &str = "native/authority-ledger.json";
/// The bounded verifier stream captures: grader-sourced Native entries
/// that are not the native grader report itself.
const VERIFIER_STREAM_KEYS: [&str; 2] =
    ["native/verifier-stdout.log", "native/verifier-stderr.log"];

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
    /// The sealed provisional trajectory.
    trajectory: serde_json::Value,
    /// The sealed authority ledger.
    ledger: serde_json::Value,
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
    /// Outcome headline: only from the grader-sourced admitted native
    /// report artifact (`P18-RPT-003`). Absent when the grader never
    /// admitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    headline: Option<HeadlineView>,
    /// Asymmetric native facts: measured or typed unknown (`P18-A16`).
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

/// The declared canary-secret guard (`P18-A18`, `P18-SEC-005`).
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

    /// Recomputes the normalized view from verified sealed bundles only:
    /// every sealed trial bundle is re-verified from its durable bytes,
    /// and every reconstructed fact is parsed from the sealed control,
    /// trajectory, ledger, and manifest evidence. Nothing is inferred from
    /// memory or from mutable run-root side files, nothing re-runs.
    pub(crate) fn recompute_from_bundle(&self) -> Result<NormalizedReport, ReportError> {
        let trials_root = self.run_root.join("trials");
        if !trials_root.is_dir() {
            return Err(ReportError::Io(std::io::Error::other(format!(
                "run root {} holds no trials directory",
                self.run_root.display()
            ))));
        }
        let mut ids: Vec<String> = std::fs::read_dir(&trials_root)
            .map_err(ReportError::Io)?
            .flatten()
            .filter(|entry| entry.path().is_dir())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect();
        ids.sort();

        let mut failures = Vec::new();
        let mut trials = Vec::new();
        for id in &ids {
            let bundle_root = trials_root.join(id).join("bundle");
            if let Err(error) = RunBundle::verify(&bundle_root) {
                failures.push(verification_failure(id, &error));
                continue;
            }
            match self.sealed_trial(id, &bundle_root) {
                Ok(trial) => trials.push(trial),
                Err(kind) => failures.push(serde_json::json!({
                    "trial": id,
                    "kind": kind,
                })),
            }
        }
        if trials.is_empty() {
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
        let integrity_digest = if integrity_bytes.is_empty() {
            String::new()
        } else {
            sha256_hex(&integrity_bytes)
        };
        Ok(NormalizedReport {
            experiment,
            integrity,
            integrity_digest,
            trials,
            failures,
        })
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
        let trials: Vec<TrialView> = normalized
            .trials
            .iter()
            .map(|trial| self.trial_view(trial, normalized))
            .collect();
        let coverage = coverage(normalized);
        let all_sealed = declared_trial_count(normalized) == normalized.trials.len();
        let all_comparable = coverage
            .iter()
            .all(|pair| pair["comparability"] == "comparable");
        let verification: BTreeMap<&str, serde_json::Value> = normalized
            .trials
            .iter()
            .map(|trial| (trial.id.as_str(), serde_json::json!({"state": "verified"})))
            .collect();
        serde_json::json!({
            "schema": REPORT_SCHEMA,
            "reporter_version": REPORTER_VERSION,
            "classification": CLASSIFICATION,
            "outcome": OUTCOME_PUBLISHED,
            "experiment": normalized.experiment["experiment_id"],
            "manifest_digest": normalized.experiment["manifest_digest"],
            "integrity_digest": normalized.integrity_digest,
            // The run's own outcome stays machine-visible: incomplete
            // coverage publishes with its exact reason, never silently.
            "run_outcome": if all_sealed && all_comparable { "completed" } else { "incomplete" },
            "trials": trials,
            "coverage": coverage,
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
        Ok(SealedTrial {
            id: id.to_owned(),
            subject: declared["subject"].as_str().unwrap_or_default().to_owned(),
            task: declared["task"].as_str().unwrap_or_default().to_owned(),
            group: declared["group"].as_str().unwrap_or_default().to_owned(),
            entries,
            trajectory,
            ledger,
        })
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
            headline: headline(trial),
            native_facts: native_facts(&product, &trial.entries),
            diagnostics: diagnostics(trial, &product),
        }
    }
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
    let artifact = trial
        .entries
        .iter()
        .find(|(key, entry)| {
            !VERIFIER_STREAM_KEYS.contains(&key.as_str())
                && entry["role"].as_str() == Some("Native")
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
/// (`P18-A16`). Measured facts cite the sealed artifact digest; unknown
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
        if let Some(completion) = completion {
            verifier_view.insert(
                "completion".to_owned(),
                serde_json::json!(if completion == 1 {
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

/// How many trials the sealed experiment contract declares.
fn declared_trial_count(normalized: &NormalizedReport) -> usize {
    normalized.experiment["trials"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0)
}

/// Pair coverage derived from sealed content only (`P18-RPT-004`,
/// `P18-EXP-006`): every edge's pairing universe over the sealed
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
    for edge in edges {
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
            let mut comparability = "comparable".to_owned();
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
            // Sealed outcome classes: a boundary failure is
            // infrastructure; a dispatched grade that failed is a grader
            // failure. A scored Agent failure stays a comparable graded
            // Agent outcome.
            if comparability == "comparable" {
                for trial in [&baseline_trial, &candidate_trial] {
                    if let Some(sealed) = normalized.trials.iter().find(|t| &t.id == trial) {
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
