//! Offline regrade over sealed assembled outputs.
//!
//! [`OfflineRegrader::regrade`] walks the sealed trial bundles under one
//! run root and re-verifies each one through [`crate::bundle::RunBundle::verify`]:
//! it recomputes the manifest identity, re-reads every covered artifact,
//! and fails on any mutation. It never repairs, rehashes, or rewrites
//! anything (`EVAL-A15`), never starts an Agent or provider, and never
//! mutates a sealed bundle (`EVAL-RPT-001`): the walk only reads the run
//! root and reports what the durable bytes prove. Machine-local absolute
//! paths never enter the report; trials are identified by their durable
//! ids only.

use std::path::Path;

use serde::Serialize;

use crate::bundle::{BundleError, RunBundle};

/// Regrade report schema identity.
const REGRADE_SCHEMA: &str = "opi-eval-regrade-report/1";

/// One verified sealed bundle: trial id plus content-addressed identity.
#[derive(Debug, Serialize)]
struct VerifiedBundle {
    trial: String,
    bundle_identity: String,
}

/// One verification failure with the typed reason the durable bytes proved.
#[derive(Debug, Serialize)]
struct VerificationFailure {
    trial: String,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact: Option<String>,
}

/// The full regrade report: every sealed bundle verified or the typed
/// failures that blocked verification. Field order is fixed by serde and
/// collections are ordered, so identical sealed inputs serialize to
/// identical bytes (`EVAL-RPT-002`).
#[derive(Debug, Serialize)]
pub(crate) struct RegradeReport {
    schema: &'static str,
    outcome: &'static str,
    bundles: Vec<VerifiedBundle>,
    failures: Vec<VerificationFailure>,
}

impl RegradeReport {
    /// The serialized canonical JSON report (byte-stable).
    pub(crate) fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("regrade report serializes")
    }
}

/// The offline regrader. Constructing it starts nothing; only
/// [`OfflineRegrader::regrade`] reads the run root.
pub(crate) struct OfflineRegrader;

impl OfflineRegrader {
    /// Verifies every sealed trial bundle under `run_root/trials/*` without
    /// mutating anything. A bundle directory missing `manifest.json` is
    /// reported as unsealed (a crashed or seal-blocked trial), not skipped
    /// silently: the denominator keeps every durable trial visible
    /// (`EVAL-EXP-006`).
    pub(crate) fn regrade(run_root: &Path) -> RegradeReport {
        let mut bundles = Vec::new();
        let mut failures = Vec::new();
        let mut trials = read_trial_ids(run_root);
        for trial in trials.drain(..) {
            let bundle_root = run_root.join("trials").join(&trial).join("bundle");
            match RunBundle::verify(&bundle_root) {
                Ok(receipt) => bundles.push(VerifiedBundle {
                    trial,
                    bundle_identity: receipt.bundle_identity().to_owned(),
                }),
                Err(error) => failures.push(classify(&trial, error)),
            }
        }
        let outcome = if failures.is_empty() {
            "verified"
        } else {
            "mutation-detected"
        };
        RegradeReport {
            schema: REGRADE_SCHEMA,
            outcome,
            bundles,
            failures,
        }
    }
}

/// Sorted durable trial ids under one run root. Ordering is by sorted id so
/// the report never depends on directory iteration order.
fn read_trial_ids(run_root: &Path) -> Vec<String> {
    let mut ids: Vec<String> = std::fs::read_dir(run_root.join("trials"))
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_dir())
                .filter_map(|entry| entry.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default();
    ids.sort();
    ids
}

/// Maps one typed bundle failure to its report token. Symlinks and manifest
/// corruption are verification failures of the same fail-closed kind; the
/// artifact key is carried only for digest mismatches and symlink escapes.
fn classify(trial: &str, error: BundleError) -> VerificationFailure {
    match error {
        BundleError::DigestMismatch { key, .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "digest-mismatch",
            artifact: Some(key.as_str().to_owned()),
        },
        BundleError::SymlinkEscape { key, .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "symlink-escape",
            artifact: Some(key.as_str().to_owned()),
        },
        BundleError::ManifestInvalid { .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "manifest-invalid",
            artifact: None,
        },
        BundleError::SidecarDrift { which, .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "sidecar-drift",
            artifact: Some(which.to_owned()),
        },
        BundleError::UnmanifestedFile { path, .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "unmanifested-file",
            artifact: Some(path),
        },
        BundleError::MissingArtifact { key, .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "missing-artifact",
            artifact: Some(key.as_str().to_owned()),
        },
        BundleError::ReservationBroken { key, .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "reservation-broken",
            artifact: Some(key.as_str().to_owned()),
        },
        BundleError::UnreservedArtifact { key, .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "unreserved-artifact",
            artifact: Some(key.as_str().to_owned()),
        },
        BundleError::Io { .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "io",
            artifact: None,
        },
        // verify() reads only the sealed manifest and covered artifacts, so
        // the staging-lifecycle variants are unreachable here; they keep one
        // token so the mapping stays exhaustive and fail-closed.
        BundleError::IntentAlreadyReserved { .. }
        | BundleError::IntentNotPublished { .. }
        | BundleError::SettlementAlreadyRecorded { .. }
        | BundleError::SealWithoutIntent { .. }
        | BundleError::SealWithoutSettlement { .. }
        | BundleError::SealedMutation { .. }
        | BundleError::ArtifactTooLarge { .. }
        | BundleError::DuplicateArtifact { .. }
        | BundleError::UnknownCausalEdge { .. } => VerificationFailure {
            trial: trial.to_owned(),
            kind: "lifecycle-invalid",
            artifact: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regrade_report_is_byte_stable_for_identical_inputs() {
        let report = RegradeReport {
            schema: REGRADE_SCHEMA,
            outcome: "verified",
            bundles: vec![VerifiedBundle {
                trial: "trial-opi-1".to_owned(),
                bundle_identity: "abc".to_owned(),
            }],
            failures: Vec::new(),
        };
        let first = serde_json::to_vec(&report).unwrap();
        let second = serde_json::to_vec(&report).unwrap();
        assert_eq!(first, second);
        let value = report.to_json();
        assert_eq!(value["schema"], "opi-eval-regrade-report/1");
        assert_eq!(value["outcome"], "verified");
    }
}
