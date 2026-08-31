//! Crate-private benchmark integrity records (Phase 18, `P18-INT-001..005`).
//!
//! An [`IntegrityRecord`] is the immutable, digest-addressed admission
//! decision for one measured benchmark revision. It binds the benchmark and
//! dataset revision, the native grader and environment identity, the upstream
//! task source identity and digest, the upstream oracle/gold preflight result
//! when provided, the revision's admission status, and the per-task validity
//! classifications with stable reviewed reasons.
//!
//! Authority boundary (`P18-INT-003`): a record is created only through
//! [`IntegrityRecord::review`] with explicit reviewer evidence. There is no
//! mutating method and no field access for the evaluated Agent, its adapter,
//! a report builder, or an LLM diagnostic to admit, retire, or reclassify
//! anything. Reclassification ([`IntegrityRecord::reclassify_task`]) and
//! status changes ([`IntegrityRecord::with_status`]) return a **new** record
//! with a **new** content-addressed identity (`P18-INT-004`); the original is
//! never rewritten.
//!
//! Task validity classes stay distinct from Agent outcomes
//! (`P18-INT-002`): only
//! [`TaskClassification::ValidAgentOutcome`] may enter Agent success/failure
//! denominators. Infrastructure and grader failures are adjudicated
//! classifications here, and every excluded trial carries a stable reason
//! traceable from coverage back to this record (`P18-INT-005`).

use serde::Serialize;
use std::collections::BTreeMap;

/// Admission status of one benchmark revision (`P18-INT-001`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) enum RevisionStatus {
    /// The revision has not been admitted; its outcomes cannot appear as
    /// headline results.
    NotAdmitted,
    /// A reviewed record admits the revision.
    Admitted,
    /// A previously admitted revision has been retired by a newer record.
    Retired,
}

/// Per-task validity classification (`P18-INT-002`). Every class except
/// [`TaskClassification::ValidAgentOutcome`] carries a stable, reviewed
/// reason and is excluded from Agent success/failure denominators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) enum TaskClassification {
    /// A valid Agent outcome; may enter Agent success/failure.
    ValidAgentOutcome,
    /// The task is broken or unsatisfiable.
    BrokenOrUnsatisfiable { reason: String },
    /// The requirement is ambiguous.
    AmbiguousRequirement { reason: String },
    /// The prompt and the test disagree.
    PromptTestMismatch { reason: String },
    /// Adjudicated infrastructure failure.
    InfrastructureFailure { reason: String },
    /// Adjudicated grader failure.
    GraderFailure { reason: String },
}

impl TaskClassification {
    /// True only for the class that may enter Agent success/failure.
    pub(crate) fn is_valid_agent_outcome(&self) -> bool {
        matches!(self, TaskClassification::ValidAgentOutcome)
    }
}

/// Upstream oracle/gold preflight result, when the benchmark provides one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) enum OraclePreflight {
    /// The upstream oracle/gold preflight passed with this detail.
    Passed(String),
    /// The upstream oracle/gold preflight failed with this detail.
    Failed(String),
}

/// Reviewer-supplied parts of one integrity record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntegrityReview {
    /// Benchmark family name.
    pub(crate) benchmark: String,
    /// Immutable benchmark revision identity.
    pub(crate) revision: String,
    /// Dataset reference owned by the revision.
    pub(crate) dataset: String,
    /// Native grader identity.
    pub(crate) grader: String,
    /// Native grader environment identity.
    pub(crate) environment: String,
    /// Upstream task identity (catalog or family identity).
    pub(crate) upstream_identity: String,
    /// Upstream task source digest or reference.
    pub(crate) upstream_digest: String,
    /// Oracle/gold preflight result when provided upstream.
    pub(crate) oracle: Option<OraclePreflight>,
    /// Admission status decided by this review.
    pub(crate) status: RevisionStatus,
    /// Per-task validity classifications.
    pub(crate) tasks: BTreeMap<String, TaskClassification>,
    /// Excluded trials with stable reasons (`P18-INT-005`).
    pub(crate) excluded_trials: BTreeMap<String, String>,
    /// Human reviewer identity owning this admission decision.
    pub(crate) reviewer: String,
}

/// Typed integrity-record construction failures.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum IntegrityError {
    /// A required identity field was empty.
    EmptyField { field: &'static str },
    /// A non-valid classification or exclusion lost its stable reason.
    MissingReason { task: String },
}

impl std::fmt::Display for IntegrityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegrityError::EmptyField { field } => {
                write!(f, "integrity field {field} must not be empty")
            }
            IntegrityError::MissingReason { task } => {
                write!(f, "task {task} requires a stable reviewed reason")
            }
        }
    }
}

impl std::error::Error for IntegrityError {}

/// Canonical digest body: every fact the identity covers. Field order is
/// fixed by serde and both maps are `BTreeMap`s, so equal semantics always
/// digest equally.
#[derive(Serialize)]
struct IntegrityBody<'a> {
    format: &'a str,
    benchmark: &'a str,
    revision: &'a str,
    dataset: &'a str,
    grader: &'a str,
    environment: &'a str,
    upstream_identity: &'a str,
    upstream_digest: &'a str,
    oracle: &'a Option<OraclePreflight>,
    status: &'a RevisionStatus,
    tasks: &'a BTreeMap<String, TaskClassification>,
    excluded_trials: &'a BTreeMap<String, String>,
    reviewer: &'a str,
}

/// One immutable, digest-addressed benchmark integrity record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntegrityRecord {
    parts: IntegrityReview,
    identity: String,
}

impl IntegrityRecord {
    /// Review a benchmark revision into an immutable integrity record.
    ///
    /// Fails closed on any empty identity field and on any non-valid
    /// classification or excluded trial without a stable reason.
    /// Deterministic: the same review always produces the same
    /// content-addressed identity.
    pub(crate) fn review(parts: IntegrityReview) -> Result<Self, IntegrityError> {
        Self::rebuild(parts)
    }

    /// Content-addressed identity of this record. Any change to any bound
    /// fact produces a different identity (`P18-INT-004`).
    pub(crate) fn identity_digest(&self) -> &str {
        &self.identity
    }

    /// The canonical bytes this record's identity addresses: the exact
    /// serialization the digest covers, staged as sealed control evidence
    /// so a sealed bundle retains the admission record itself
    /// (`P18-BND-001`).
    pub(crate) fn canonical_bytes(&self) -> Vec<u8> {
        canonical_body(&self.parts).expect("a reviewed record canonicalizes")
    }

    /// Native grader identity admitted by this record.
    pub(crate) fn grader(&self) -> &str {
        &self.parts.grader
    }

    /// Admission status of the revision.
    pub(crate) fn status(&self) -> RevisionStatus {
        self.parts.status
    }

    /// True only when this record admits the revision (`P18-INT-001`).
    pub(crate) fn admitted(&self) -> bool {
        self.parts.status == RevisionStatus::Admitted
    }

    /// Benchmark revision identity bound by this record.
    pub(crate) fn revision(&self) -> &str {
        &self.parts.revision
    }

    /// Classification of one task, if this record covers it.
    pub(crate) fn task_classification(&self, task: &str) -> Option<&TaskClassification> {
        self.parts.tasks.get(task)
    }

    /// Stable exclusion reason for one trial, if this record excludes it.
    pub(crate) fn excluded_trial_reason(&self, trial: &str) -> Option<&str> {
        self.parts.excluded_trials.get(trial).map(String::as_str)
    }

    /// Oracle/gold preflight result bound by this record.
    pub(crate) fn oracle(&self) -> Option<&OraclePreflight> {
        self.parts.oracle.as_ref()
    }

    /// Reclassify one task, returning a **new** record with a **new**
    /// identity (`P18-INT-004`). The original record is unchanged.
    pub(crate) fn reclassify_task(
        &self,
        task: &str,
        classification: TaskClassification,
    ) -> Result<Self, IntegrityError> {
        let mut parts = self.parts.clone();
        parts.tasks.insert(task.to_owned(), classification);
        Self::rebuild(parts)
    }

    /// Replace the admission status, returning a **new** record with a
    /// **new** identity (`P18-INT-004`). The original record is unchanged.
    pub(crate) fn with_status(&self, status: RevisionStatus) -> Result<Self, IntegrityError> {
        let mut parts = self.parts.clone();
        parts.status = status;
        Self::rebuild(parts)
    }

    fn rebuild(parts: IntegrityReview) -> Result<Self, IntegrityError> {
        for (field, value) in [
            ("benchmark", &parts.benchmark),
            ("revision", &parts.revision),
            ("dataset", &parts.dataset),
            ("grader", &parts.grader),
            ("environment", &parts.environment),
            ("upstream_identity", &parts.upstream_identity),
            ("upstream_digest", &parts.upstream_digest),
            ("reviewer", &parts.reviewer),
        ] {
            if value.trim().is_empty() {
                return Err(IntegrityError::EmptyField { field });
            }
        }
        for (task, classification) in &parts.tasks {
            let reason = match classification {
                TaskClassification::ValidAgentOutcome => None,
                TaskClassification::BrokenOrUnsatisfiable { reason }
                | TaskClassification::AmbiguousRequirement { reason }
                | TaskClassification::PromptTestMismatch { reason }
                | TaskClassification::InfrastructureFailure { reason }
                | TaskClassification::GraderFailure { reason } => Some(reason),
            };
            if reason.is_some_and(|reason| reason.trim().is_empty()) {
                return Err(IntegrityError::MissingReason { task: task.clone() });
            }
        }
        for (trial, reason) in &parts.excluded_trials {
            if reason.trim().is_empty() {
                return Err(IntegrityError::MissingReason {
                    task: trial.clone(),
                });
            }
        }
        let canonical = canonical_body(&parts)?;
        let identity = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(&canonical);
            let digest = hasher.finalize();
            let mut hex = String::with_capacity(digest.len() * 2);
            for byte in digest {
                hex.push_str(&format!("{byte:02x}"));
            }
            hex
        };
        Ok(IntegrityRecord { parts, identity })
    }
}

/// The canonical serialization the record identity addresses.
fn canonical_body(parts: &IntegrityReview) -> Result<Vec<u8>, IntegrityError> {
    let body = IntegrityBody {
        format: "opi-eval-integrity",
        benchmark: &parts.benchmark,
        revision: &parts.revision,
        dataset: &parts.dataset,
        grader: &parts.grader,
        environment: &parts.environment,
        upstream_identity: &parts.upstream_identity,
        upstream_digest: &parts.upstream_digest,
        oracle: &parts.oracle,
        status: &parts.status,
        tasks: &parts.tasks,
        excluded_trials: &parts.excluded_trials,
        reviewer: &parts.reviewer,
    };
    serde_json::to_vec(&body).map_err(|_| IntegrityError::EmptyField {
        field: "canonical_body",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn review_fixture(status: RevisionStatus) -> IntegrityReview {
        IntegrityReview {
            benchmark: "terminal-bench".to_owned(),
            revision: "0.9.2".to_owned(),
            dataset: "terminal-bench-0.9.2".to_owned(),
            grader: "tb-native".to_owned(),
            environment: "tb-env-1".to_owned(),
            upstream_identity: "terminal-bench/tasks".to_owned(),
            upstream_digest: "sha256:1a".to_owned(),
            oracle: Some(OraclePreflight::Passed("oracle 41/41".to_owned())),
            status,
            tasks: BTreeMap::from([
                ("task-1".to_owned(), TaskClassification::ValidAgentOutcome),
                (
                    "task-2".to_owned(),
                    TaskClassification::PromptTestMismatch {
                        reason: "prompt asks ls; test greps find".to_owned(),
                    },
                ),
            ]),
            excluded_trials: BTreeMap::from([(
                "trial-9".to_owned(),
                "workspace vanished under sandbox".to_owned(),
            )]),
            reviewer: "human-reviewer-1".to_owned(),
        }
    }

    #[test]
    fn review_creates_digest_addressed_immutable_record() {
        let record = IntegrityRecord::review(review_fixture(RevisionStatus::Admitted)).unwrap();
        assert_eq!(record.status(), RevisionStatus::Admitted);
        assert!(record.admitted());
        assert_eq!(record.revision(), "0.9.2");
        assert_eq!(record.identity_digest().len(), 64);
        assert_eq!(record.grader(), "tb-native");
        // The canonical bytes address the identity: hashing them reproduces
        // it exactly, so staged control evidence carries the admission
        // record itself.
        {
            use sha2::{Digest, Sha256};
            let digest = Sha256::digest(record.canonical_bytes());
            let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
            assert_eq!(hex, record.identity_digest());
        }
        assert!(matches!(
            record.task_classification("task-2"),
            Some(TaskClassification::PromptTestMismatch { .. })
        ));
        assert_eq!(
            record.excluded_trial_reason("trial-9"),
            Some("workspace vanished under sandbox")
        );

        // Deterministic: an identical review digests to the same identity.
        let twin = IntegrityRecord::review(review_fixture(RevisionStatus::Admitted)).unwrap();
        assert_eq!(record.identity_digest(), twin.identity_digest());

        // Any bound-fact change changes the identity.
        let mut other = review_fixture(RevisionStatus::Admitted);
        other.reviewer = "human-reviewer-2".to_owned();
        let changed = IntegrityRecord::review(other).unwrap();
        assert_ne!(record.identity_digest(), changed.identity_digest());
    }

    #[test]
    fn review_fails_closed_on_missing_identities_and_reasons() {
        let mut parts = review_fixture(RevisionStatus::NotAdmitted);
        parts.dataset = "  ".to_owned();
        assert_eq!(
            IntegrityRecord::review(parts).unwrap_err(),
            IntegrityError::EmptyField { field: "dataset" }
        );

        let mut parts = review_fixture(RevisionStatus::NotAdmitted);
        parts.tasks.insert(
            "task-3".to_owned(),
            TaskClassification::GraderFailure {
                reason: String::new(),
            },
        );
        assert_eq!(
            IntegrityRecord::review(parts).unwrap_err(),
            IntegrityError::MissingReason {
                task: "task-3".to_owned()
            }
        );

        let mut parts = review_fixture(RevisionStatus::NotAdmitted);
        parts
            .excluded_trials
            .insert("trial-10".to_owned(), " ".to_owned());
        assert_eq!(
            IntegrityRecord::review(parts).unwrap_err(),
            IntegrityError::MissingReason {
                task: "trial-10".to_owned()
            }
        );
    }

    #[test]
    fn reclassification_and_status_changes_create_new_identities() {
        let record = IntegrityRecord::review(review_fixture(RevisionStatus::Admitted)).unwrap();

        let reclassified = record
            .reclassify_task(
                "task-1",
                TaskClassification::BrokenOrUnsatisfiable {
                    reason: "fixture missing from revision".to_owned(),
                },
            )
            .unwrap();
        assert_ne!(record.identity_digest(), reclassified.identity_digest());
        // The original record is unchanged.
        assert!(matches!(
            record.task_classification("task-1"),
            Some(TaskClassification::ValidAgentOutcome)
        ));
        assert!(matches!(
            reclassified.task_classification("task-1"),
            Some(TaskClassification::BrokenOrUnsatisfiable { .. })
        ));

        // Re-reviewing identical content keeps the same identity:
        // identity addresses content, not review events. Empty reasons stay
        // rejected.
        let noop = record
            .reclassify_task("task-1", TaskClassification::ValidAgentOutcome)
            .unwrap();
        assert_eq!(record.identity_digest(), noop.identity_digest());
        assert_eq!(
            record
                .reclassify_task(
                    "task-1",
                    TaskClassification::AmbiguousRequirement {
                        reason: "".to_owned()
                    }
                )
                .unwrap_err(),
            IntegrityError::MissingReason {
                task: "task-1".to_owned()
            }
        );

        let retired = record.with_status(RevisionStatus::Retired).unwrap();
        assert!(!retired.admitted());
        assert_ne!(record.identity_digest(), retired.identity_digest());
        assert!(record.admitted());
    }
}
