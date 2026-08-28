//! Normalized offline report over sealed assembled outputs (task 18.13).
//!
//! [`ReportBuilder::recompute_from_bundle`] rebuilds the normalized view
//! purely from what the run root durably holds: the persisted run report,
//! the per-trial receipts, and the sealed bundle manifests, re-verified
//! through [`crate::bundle::RunBundle::verify`]. [`ReportBuilder::build`]
//! invokes the recompute step before rendering so no report can be
//! rendered from stale or unverified state. Both paths are effect-free:
//! no Agent, no provider, no spawn, no mutation of sealed bytes
//! (`P18-RPT-001`).
//!
//! Report contract enforced here: headline outcomes come only from
//! admitted benchmark-native grader artifacts with per-headline provenance
//! (`P18-RPT-003`); pair coverage keeps every declared pair visible with
//! its exact state, so exclusions, failures, and unknowns never leave the
//! denominator silently (`P18-RPT-004`, `P18-EXP-006`); quality, cost,
//! safety, efficiency, and authority are never collapsed into one
//! composite score or best-trial verdict (`P18-RPT-005`); the report
//! labels its evidence `conformance-evidence` and claims no official
//! leaderboard verification (`P18-RPT-006`). Asymmetric native facts stay
//! measured values (cited by sealed artifact digest) or typed unknowns -
//! never fabricated parity (`P18-A16`). Declared canary secrets in
//! exportable bundle content block publication (`P18-A18`,
//! `P18-SEC-005`). Identical sealed inputs and tool identities serialize
//! to byte-identical output (`P18-RPT-002`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::bundle::RunBundle;

/// Normalized report schema identity.
const REPORT_SCHEMA: &str = "phase18-normalized-report/1";
/// Pinned reporter identity: part of every byte-stability contract
/// (`P18-RPT-002` - same bundle, grader identity, and reporter version).
pub(crate) const REPORTER_VERSION: &str = "phase18-reporter/1";
/// The single classification wording of Phase 18 paired results
/// (`P18-RPT-006`): conformance evidence only, never leaderboard
/// verification or superiority.
const CLASSIFICATION: &str = "conformance-evidence";
/// Outcome token when publication succeeded.
const OUTCOME_PUBLISHED: &str = "published";
/// Outcome token when a declared canary blocked publication.
const OUTCOME_BLOCKED: &str = "publication-blocked";

/// The asymmetric native fact families the common report tracks. A family
/// is `measured` for a product only when the product's own sealed native
/// artifact carries it; otherwise it stays a typed unknown
/// (`P18-A16`).
const NATIVE_FACT_FAMILIES: [&str; 4] = ["usage", "cost", "retry", "compaction"];

/// Which sealed native artifact carries one fact family for one product.
/// Products absent from this table expose none of the families natively in
/// the pinned hermetic profiles.
const NATIVE_FACT_SOURCES: &[(&str, &str)] = &[("opi", "native/evidence/records")];

/// The offline report builder over one run root.
pub(crate) struct ReportBuilder {
    run_root: PathBuf,
}

/// One recomputed normalized view of a run root.
#[derive(Debug)]
pub(crate) struct NormalizedReport {
    run_report: serde_json::Value,
    trials: Vec<TrialView>,
    verification: BTreeMap<String, BundleVerification>,
}

/// The re-verification outcome of one sealed bundle.
#[derive(Debug)]
enum BundleVerification {
    /// Verified with its content-addressed identity.
    Verified(String),
    /// Failed verification with the typed token.
    Failed(&'static str),
}

/// One recomputed trial view (rendered per trial in the report).
#[derive(Debug, Serialize)]
struct TrialView {
    id: String,
    subject: String,
    task: String,
    group: String,
    status: String,
    /// Outcome headline: only from the admitted native grader artifact
    /// (`P18-RPT-003`). Absent when the grader never admitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    headline: Option<HeadlineView>,
    /// Asymmetric native facts: measured or typed unknown (`P18-A16`).
    native_facts: BTreeMap<String, serde_json::Value>,
    /// Separately labelled diagnostics: agent execution observations and
    /// authority counts from the durable receipt, never mixed into the
    /// headline.
    diagnostics: serde_json::Value,
    /// The verified content-addressed bundle identity of this trial.
    #[serde(skip_serializing_if = "Option::is_none")]
    bundle_identity: Option<String>,
}

/// The native-grader headline with its provenance citation.
#[derive(Debug, Serialize)]
struct HeadlineView {
    /// The native reward fact token (measured value or typed unknown).
    reward: serde_json::Value,
    /// Where the headline came from: the sealed native artifact and the
    /// bundle identity covering it.
    native_source: BTreeMap<&'static str, String>,
}

/// Failures reported by the report path.
#[derive(Debug)]
pub(crate) enum ReportError {
    /// The run root does not hold a persisted run report.
    MissingRunReport,
    /// A durable file could not be read.
    Io(std::io::Error),
}

impl std::error::Error for ReportError {}

impl std::fmt::Display for ReportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportError::MissingRunReport => {
                write!(f, "run root holds no persisted run report")
            }
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

    /// Recomputes the normalized view from sealed assembled outputs only:
    /// the persisted run report (denominator and declared identities), the
    /// per-trial receipts (agent diagnostics and authority counts), and
    /// every sealed bundle re-verified from its durable bytes. Nothing is
    /// inferred from memory, nothing re-runs.
    pub(crate) fn recompute_from_bundle(&self) -> Result<NormalizedReport, ReportError> {
        let run_report_path = self.run_root.join("run-report.json");
        let run_text = std::fs::read_to_string(&run_report_path).map_err(|error| {
            ReportError::Io(std::io::Error::other(format!(
                "persisted run report missing ({error})"
            )))
        })?;
        let run_report: serde_json::Value = serde_json::from_str(&run_text).map_err(|error| {
            ReportError::Io(std::io::Error::other(format!(
                "persisted run report unparsable: {error}"
            )))
        })?;

        let mut verification = BTreeMap::new();
        let mut trials = Vec::new();
        for declared in run_report["trials"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
        {
            let id = declared["id"].as_str().unwrap_or_default().to_owned();
            let bundle_root = self.run_root.join("trials").join(&id).join("bundle");
            let verified = RunBundle::verify(&bundle_root)
                .ok()
                .map(|receipt| receipt.bundle_identity().to_owned());
            let token = match &verified {
                Some(_) => "verified",
                None => "mutation-detected",
            };
            verification.insert(
                id.clone(),
                match &verified {
                    Some(identity) => BundleVerification::Verified(identity.clone()),
                    None => BundleVerification::Failed(token),
                },
            );
            let receipt = self.read_receipt(&id);
            let manifest = self.read_manifest_entries(&id);
            trials.push(self.trial_view(&id, declared, &receipt, &manifest, verified));
        }
        Ok(NormalizedReport {
            run_report,
            trials,
            verification,
        })
    }

    /// Builds the published report: recompute first, then redact-gate, then
    /// render. Publication is blocked (typed outcome, no normalized
    /// content) when a declared canary appears in exportable sealed
    /// content.
    pub(crate) fn build(&self, guard: &RedactionGuard) -> Result<serde_json::Value, ReportError> {
        let normalized = self.recompute_from_bundle()?;
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
        for trial in normalized.run_report["trials"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
        {
            let id = trial["id"].as_str().unwrap_or_default();
            let bundle_root = self.run_root.join("trials").join(id).join("bundle");
            let entries = self.read_manifest_entries(id);
            for (key, entry) in &entries {
                if entry.get("classification") != Some(&serde_json::json!("Exportable")) {
                    continue;
                }
                let artifact = bundle_root.join("artifacts").join(key);
                if let Ok(content) = std::fs::read(&artifact)
                    && guard.find_leak(&content)
                {
                    // The blocked report names where the leak sits but
                    // never echoes the canary itself.
                    return Some(serde_json::json!({
                        "trial": id,
                        "artifact": key,
                        "reason": "declared-canary-present",
                    }));
                }
            }
        }
        None
    }

    /// Renders the byte-stable normalized report.
    fn render(&self, normalized: &NormalizedReport) -> serde_json::Value {
        let verification: BTreeMap<&str, serde_json::Value> = normalized
            .verification
            .iter()
            .map(|(trial, verdict)| {
                let value = match verdict {
                    BundleVerification::Verified(identity) => serde_json::json!({
                        "state": "verified",
                        "bundle_identity": identity,
                    }),
                    BundleVerification::Failed(kind) => {
                        serde_json::json!({"state": kind})
                    }
                };
                (trial.as_str(), value)
            })
            .collect();
        serde_json::json!({
            "schema": REPORT_SCHEMA,
            "reporter_version": REPORTER_VERSION,
            "classification": CLASSIFICATION,
            "outcome": OUTCOME_PUBLISHED,
            "experiment": normalized.run_report["experiment"],
            "manifest_digest": normalized.run_report["manifest_digest"],
            "integrity_digest": normalized.run_report["integrity_digest"],
            // The run's own outcome stays machine-visible: incomplete
            // coverage publishes with its exact reason, never silently.
            "run_outcome": normalized.run_report["outcome"],
            "trials": normalized.trials,
            "coverage": coverage(&normalized.run_report),
            "bundle_verification": verification,
        })
    }

    /// Reads one trial receipt from the run root, if present.
    fn read_receipt(&self, trial: &str) -> serde_json::Value {
        std::fs::read_to_string(
            self.run_root
                .join("trials")
                .join(trial)
                .join("receipt.json"),
        )
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(serde_json::Value::Null)
    }

    /// Reads the sealed manifest's artifact entries (logical key to entry)
    /// of one trial, if present. The manifest is a durable wire format; it
    /// is parsed generically here so the report path owns no bundle
    /// internals beyond that format.
    fn read_manifest_entries(&self, trial: &str) -> BTreeMap<String, serde_json::Value> {
        let path = self
            .run_root
            .join("trials")
            .join(trial)
            .join("bundle/manifest.json");
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .and_then(|manifest| {
                manifest["entries"]
                    .as_object()
                    .cloned()
                    .map(|entries| entries.into_iter().collect())
            })
            .unwrap_or_default()
    }

    /// Recomputes one trial view from the declared run-report entry, the
    /// durable receipt, and the sealed manifest entries.
    fn trial_view(
        &self,
        id: &str,
        declared: &serde_json::Value,
        receipt: &serde_json::Value,
        manifest: &BTreeMap<String, serde_json::Value>,
        bundle_identity: Option<String>,
    ) -> TrialView {
        let subject = declared["subject"].as_str().unwrap_or_default().to_owned();
        let product = receipt["agent"]["product"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        TrialView {
            id: id.to_owned(),
            subject,
            task: declared["task"].as_str().unwrap_or_default().to_owned(),
            group: declared["group"].as_str().unwrap_or_default().to_owned(),
            status: declared["status"].as_str().unwrap_or_default().to_owned(),
            headline: headline(receipt, manifest),
            native_facts: native_facts(&product, manifest),
            diagnostics: serde_json::json!({
                "label": "diagnostic",
                "agent": receipt["agent"],
                "authority": receipt["authority"],
            }),
            bundle_identity,
        }
    }
}

/// The native-grader headline of one trial: the reward fact from the
/// durable receipt plus the provenance citation of the admitted native
/// artifact. Absent when the grader never admitted a native report.
fn entry_of<'a>(
    manifest: &'a BTreeMap<String, serde_json::Value>,
    key: &str,
) -> &'a serde_json::Value {
    manifest.get(key).unwrap_or(&serde_json::Value::Null)
}

/// The serialized role token of one manifest entry.
fn entry_role(entry: &serde_json::Value) -> &str {
    entry["role"].as_str().unwrap_or_default()
}

fn headline(
    receipt: &serde_json::Value,
    manifest: &BTreeMap<String, serde_json::Value>,
) -> Option<HeadlineView> {
    let verifier = receipt.get("verifier")?;
    let reward = verifier.get("reward")?;
    if reward.is_null() {
        return None;
    }
    // The admitted native grader report is identified by its well-known
    // logical role suffix (Terminal-Bench CTRF, DeepSWE pier).
    let artifact = manifest
        .iter()
        .find(|(key, _)| {
            (key.ends_with("ctrf-report") || key.ends_with("pier-report"))
                && entry_role(entry_of(manifest, key)) == "Native"
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
        reward: reward.clone(),
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
    manifest: &BTreeMap<String, serde_json::Value>,
) -> BTreeMap<String, serde_json::Value> {
    let source = NATIVE_FACT_SOURCES
        .iter()
        .find(|(owner, _)| *owner == product)
        .map(|(_, key)| *key);
    NATIVE_FACT_FAMILIES
        .iter()
        .map(|family| {
            let value = match source {
                Some(key) if *family == "usage" && manifest.contains_key(key) => {
                    serde_json::json!({
                        "state": "measured",
                        "artifact": key,
                        "digest": manifest[key]["digest"],
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

/// Coverage denominator straight from the persisted run report: every
/// declared pair with its exact comparability state (`P18-RPT-004`,
/// `P18-EXP-006`). Pairs are copied as durable evidence, not recomputed or
/// filtered.
fn coverage(run_report: &serde_json::Value) -> serde_json::Value {
    run_report["pairs"].clone()
}

/// The blocked publication report: typed leak reason, no normalized
/// content, no secret echo.
fn blocked_report(normalized: &NormalizedReport, leak: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "schema": REPORT_SCHEMA,
        "reporter_version": REPORTER_VERSION,
        "classification": CLASSIFICATION,
        "outcome": OUTCOME_BLOCKED,
        "experiment": normalized.run_report["experiment"],
        "leak": leak,
    })
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
        let mut manifest = BTreeMap::new();
        manifest.insert(
            "native/evidence/records".to_owned(),
            serde_json::json!({"digest": "a".repeat(64), "classification": "Exportable"}),
        );
        let opi = native_facts("opi", &manifest);
        assert_eq!(opi["usage"]["state"], "measured");
        assert_eq!(opi["cost"]["state"], "unknown:opi-cost-not-native");
        let pi = native_facts("pi", &manifest);
        assert_eq!(pi["usage"]["state"], "unknown:pi-usage-not-native");
        assert!(pi["usage"].get("digest").is_none());
        // A product that never staged its own evidence never measures it,
        // even when another product's artifact is present in scope.
        let empty = native_facts("opi", &BTreeMap::new());
        assert_eq!(empty["usage"]["state"], "unknown:opi-usage-not-native");
    }
}
