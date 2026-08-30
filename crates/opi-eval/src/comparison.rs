//! Crate-private pairing and comparability assembly (Phase 18,
//! `P18-EXP-002`, `P18-EXP-004`, `P18-EXP-008`).
//!
//! [`ComparisonSet::assemble`] consumes the frozen
//! [`ResolvedExperiment`](crate::experiment::ResolvedExperiment) and an
//! admitted [`IntegrityRecord`](crate::integrity::IntegrityRecord)
//! **read-only** and assembles exactly one baseline/candidate pair per
//! declared edge, task, and trial group — and only when the frozen
//! experiment controls, the benchmark revision, the task, the trial group,
//! and the control fingerprints all agree.
//!
//! The pairing path never mutates or silently reinterprets the frozen
//! identity. Structural violations fail closed with typed errors; per-pair
//! states that must stay visible rather than fail the assembly — missing
//! trials, control mismatches, unsupported controls, exclusions,
//! infrastructure and grader failures, and invalid task classifications —
//! are typed [`NonComparability`] values carried on the pair
//! (`P18-EXP-006`, `P18-INT-002`), so they remain in the denominator and
//! never silently disappear.

use crate::experiment::ResolvedExperiment;
use crate::integrity::{IntegrityRecord, RevisionStatus, TaskClassification};
use std::collections::BTreeMap;

/// Post-trial fact about one settled trial, supplied by the runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrialFact {
    /// Trial id; must be declared by the frozen experiment.
    pub(crate) id: String,
    /// Owning subject id.
    pub(crate) subject: String,
    /// Benchmark task id.
    pub(crate) task: String,
    /// Trial group used for baseline/candidate pairing.
    pub(crate) group: String,
    /// Manifest digest of the frozen experiment the trial ran under.
    pub(crate) manifest_digest: String,
    /// Digest of the effective shared controls the trial ran under.
    pub(crate) control_fingerprint: String,
    /// Shared controls this subject could not express (`P18-EXP-008`).
    pub(crate) unsupported_controls: Vec<String>,
    /// Settled outcome class of the trial.
    pub(crate) outcome: TrialOutcome,
}

/// Settled outcome class of one trial, kept distinct from Agent
/// success/failure (which is a graded artifact, not a pairing input).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrialOutcome {
    /// The trial produced a graded Agent outcome.
    ValidAgentOutcome,
    /// A scored Agent failure: the Agent's own non-zero exit, crash, or
    /// Agent-owned timeout on a valid task, graded under the native
    /// grader. It stays in the Agent success/failure denominator
    /// (`P18-INT-002`) and is never reclassified as infrastructure.
    AgentFailure,
    /// The trial settled as an infrastructure failure.
    InfrastructureFailure,
    /// The trial settled as a grader failure.
    GraderFailure,
}

/// Why one assembled pair is not comparable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NonComparability {
    /// The baseline-side trial for this pair never settled.
    MissingBaselineTrial,
    /// The candidate-side trial for this pair never settled.
    MissingCandidateTrial,
    /// The pair's control fingerprints disagree (`P18-EXP-004`).
    ControlMismatch { baseline: String, candidate: String },
    /// A required shared control could not be expressed by one subject
    /// (`P18-EXP-008`).
    UnsupportedControl { control: String },
    /// A trial of this pair is excluded by the integrity record with this
    /// stable reason (`P18-INT-005`).
    Excluded { trial: String, reason: String },
    /// A trial of this pair settled as an infrastructure failure.
    InfrastructureFailure { trial: String },
    /// A trial of this pair settled as a grader failure.
    GraderFailure { trial: String },
    /// The task's validity classification is not a valid Agent outcome
    /// (`P18-INT-002`).
    InvalidTaskClassification { classification: TaskClassification },
    /// The admitted record does not classify this task at all.
    TaskNotCovered,
}

/// Comparability of one assembled pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Comparability {
    /// Exactly one baseline and one candidate trial agree on every frozen
    /// control; the pair may carry a paired claim.
    Comparable,
    /// The pair stays visible in coverage but carries no paired claim.
    NonComparable(NonComparability),
}

impl Comparability {
    /// True only for [`Comparability::Comparable`].
    pub(crate) fn is_comparable(&self) -> bool {
        matches!(self, Comparability::Comparable)
    }
}

/// One assembled baseline/candidate pair of a declared edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PairedComparison {
    edge: String,
    task: String,
    group: String,
    baseline_trial: String,
    candidate_trial: String,
    comparability: Comparability,
}

impl PairedComparison {
    /// Declared edge id this pair belongs to.
    pub(crate) fn edge(&self) -> &str {
        &self.edge
    }
    /// Benchmark task id of the pair.
    pub(crate) fn task(&self) -> &str {
        &self.task
    }
    /// Trial group of the pair.
    pub(crate) fn group(&self) -> &str {
        &self.group
    }
    /// Settled baseline trial id, or empty when missing.
    pub(crate) fn baseline_trial(&self) -> &str {
        &self.baseline_trial
    }
    /// Settled candidate trial id, or empty when missing.
    pub(crate) fn candidate_trial(&self) -> &str {
        &self.candidate_trial
    }
    /// Comparability state of the pair.
    pub(crate) fn comparability(&self) -> &Comparability {
        &self.comparability
    }
}

/// Typed assembly failures. These are structural violations of the frozen
/// contract or the admission gate, not per-pair visibility states.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ComparisonError {
    /// The integrity record does not admit the revision (`P18-INT-001`).
    NotAdmitted { status: RevisionStatus },
    /// The experiment's frozen integrity digest does not address this
    /// record.
    IntegrityDigestMismatch {
        experiment: Option<String>,
        record: String,
    },
    /// A supplied fact names a trial the frozen experiment never declared.
    UnknownTrial { trial: String },
    /// A supplied fact's subject is not declared by the frozen experiment.
    UnknownFactSubject { trial: String, subject: String },
    /// The same trial id was supplied twice.
    DuplicateTrialFact { trial: String },
    /// More than one settled fact exists for one edge role, task, and
    /// group; the frozen contract declares uniqueness.
    DuplicateRoleTrial {
        edge: String,
        role: &'static str,
        task: String,
        group: String,
    },
    /// A supplied fact claims a manifest other than the frozen experiment's.
    ManifestDigestMismatch { trial: String },
}

impl std::fmt::Display for ComparisonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComparisonError::NotAdmitted { status } => {
                write!(f, "benchmark revision is not admitted (status {status:?})")
            }
            ComparisonError::IntegrityDigestMismatch { experiment, record } => {
                write!(
                    f,
                    "experiment integrity digest {:?} does not address record {}",
                    experiment.as_deref().unwrap_or("<missing>"),
                    record
                )
            }
            ComparisonError::UnknownTrial { trial } => {
                write!(f, "fact names undeclared trial {trial}")
            }
            ComparisonError::UnknownFactSubject { trial, subject } => {
                write!(
                    f,
                    "fact for trial {trial} names undeclared subject {subject}"
                )
            }
            ComparisonError::DuplicateTrialFact { trial } => {
                write!(f, "trial {trial} was supplied twice")
            }
            ComparisonError::DuplicateRoleTrial {
                edge,
                role,
                task,
                group,
            } => {
                write!(
                    f,
                    "duplicate {role} trial for edge {edge}, task {task}, group {group}"
                )
            }
            ComparisonError::ManifestDigestMismatch { trial } => {
                write!(f, "trial {trial} claims a different experiment manifest")
            }
        }
    }
}

impl std::error::Error for ComparisonError {}

/// The assembled pairing result of one experiment against one admitted
/// integrity record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ComparisonSet {
    pairs: Vec<PairedComparison>,
}

impl ComparisonSet {
    /// Assemble baseline/candidate pairs from the frozen experiment, an
    /// integrity record, and settled trial facts.
    ///
    /// Fails closed when the revision is not admitted, when the experiment's
    /// frozen integrity digest does not address the record, or when the
    /// supplied facts contradict the frozen contract. Per-pair states stay
    /// visible as [`NonComparability`] values.
    pub(crate) fn assemble(
        experiment: &ResolvedExperiment,
        integrity: &IntegrityRecord,
        facts: &[TrialFact],
    ) -> Result<Self, ComparisonError> {
        let Some(experiment_digest) = experiment.benchmark().integrity_digest.as_deref() else {
            return Err(ComparisonError::IntegrityDigestMismatch {
                experiment: None,
                record: integrity.identity_digest().to_owned(),
            });
        };
        if experiment_digest != integrity.identity_digest() {
            return Err(ComparisonError::IntegrityDigestMismatch {
                experiment: Some(experiment_digest.to_owned()),
                record: integrity.identity_digest().to_owned(),
            });
        }
        if !integrity.admitted() {
            return Err(ComparisonError::NotAdmitted {
                status: integrity.status(),
            });
        }
        let mut seen_ids: Vec<&str> = Vec::new();
        for fact in facts {
            let declared = experiment
                .trials()
                .iter()
                .find(|trial| trial.id == fact.id)
                .ok_or_else(|| ComparisonError::UnknownTrial {
                    trial: fact.id.clone(),
                })?;
            if declared.subject != fact.subject {
                return Err(ComparisonError::UnknownFactSubject {
                    trial: fact.id.clone(),
                    subject: fact.subject.clone(),
                });
            }
            if declared.task != fact.task || declared.group != fact.group {
                return Err(ComparisonError::UnknownTrial {
                    trial: fact.id.clone(),
                });
            }
            if fact.manifest_digest != experiment.manifest_digest() {
                return Err(ComparisonError::ManifestDigestMismatch {
                    trial: fact.id.clone(),
                });
            }
            if seen_ids.contains(&fact.id.as_str()) {
                return Err(ComparisonError::DuplicateTrialFact {
                    trial: fact.id.clone(),
                });
            }
            seen_ids.push(fact.id.as_str());
        }
        Self::build_pairs(experiment, integrity, facts)
    }

    fn build_pairs(
        experiment: &ResolvedExperiment,
        integrity: &IntegrityRecord,
        facts: &[TrialFact],
    ) -> Result<Self, ComparisonError> {
        // Index settled facts by (subject, task, group); the frozen contract
        // Index settled facts by (subject, task, group).
        let mut settled: BTreeMap<(String, String, String), &TrialFact> = BTreeMap::new();
        for fact in facts {
            let key = (fact.subject.clone(), fact.task.clone(), fact.group.clone());
            settled.insert(key, fact);
        }
        let mut pairs = Vec::new();
        for edge in experiment.edges() {
            // The pairing universe: every (task, group) the frozen contract
            // declares for this edge's two subjects.
            let mut keys: Vec<(String, String)> = experiment
                .trials()
                .iter()
                .filter(|trial| trial.subject == edge.baseline || trial.subject == edge.candidate)
                .map(|trial| (trial.task.clone(), trial.group.clone()))
                .collect();
            keys.sort();
            keys.dedup();
            for (task, group) in keys {
                // A frozen contract that declares two trials for one pairing
                // slot is ambiguous (`P18-EXP-002`) and fails the assembly
                // regardless of which trials later settled.
                for (role, subject) in
                    [("baseline", &edge.baseline), ("candidate", &edge.candidate)]
                {
                    let declared_for_slot = experiment
                        .trials()
                        .iter()
                        .filter(|trial| {
                            trial.subject == *subject && trial.task == task && trial.group == group
                        })
                        .count();
                    if declared_for_slot > 1 {
                        return Err(ComparisonError::DuplicateRoleTrial {
                            edge: edge.id.clone(),
                            role,
                            task: task.clone(),
                            group: group.clone(),
                        });
                    }
                }
                let mut baseline_trial = String::new();
                let mut candidate_trial = String::new();
                let mut comparability = Comparability::Comparable;
                let classification = match integrity.task_classification(&task) {
                    None => Some(NonComparability::TaskNotCovered),
                    Some(classification) if !classification.is_valid_agent_outcome() => {
                        Some(NonComparability::InvalidTaskClassification {
                            classification: classification.clone(),
                        })
                    }
                    Some(_) => None,
                };
                if let Some(non_comparable) = classification {
                    comparability = Comparability::NonComparable(non_comparable);
                }
                let baseline = settled
                    .get(&(edge.baseline.clone(), task.clone(), group.clone()))
                    .copied();
                let candidate = settled
                    .get(&(edge.candidate.clone(), task.clone(), group.clone()))
                    .copied();
                if comparability.is_comparable() {
                    if let Some(fact) = baseline {
                        baseline_trial = fact.id.clone();
                    } else {
                        comparability =
                            Comparability::NonComparable(NonComparability::MissingBaselineTrial);
                    }
                }
                if comparability.is_comparable() {
                    if let Some(fact) = candidate {
                        candidate_trial = fact.id.clone();
                    } else {
                        comparability =
                            Comparability::NonComparable(NonComparability::MissingCandidateTrial);
                    }
                }
                if comparability.is_comparable() {
                    let (baseline, candidate) = (baseline.unwrap(), candidate.unwrap());
                    if let Some((trial, reason)) = [baseline, candidate].iter().find_map(|fact| {
                        integrity
                            .excluded_trial_reason(&fact.id)
                            .map(|reason| (fact, reason.to_owned()))
                    }) {
                        comparability = Comparability::NonComparable(NonComparability::Excluded {
                            trial: trial.id.clone(),
                            reason,
                        });
                    } else if let Some(state) =
                        [baseline, candidate]
                            .iter()
                            .find_map(|fact| match fact.outcome {
                                // A scored Agent failure is a graded Agent
                                // outcome: the pair keeps its paired claim
                                // and the trial stays in the denominator.
                                TrialOutcome::ValidAgentOutcome | TrialOutcome::AgentFailure => {
                                    None
                                }
                                TrialOutcome::InfrastructureFailure => {
                                    Some(NonComparability::InfrastructureFailure {
                                        trial: fact.id.clone(),
                                    })
                                }
                                TrialOutcome::GraderFailure => {
                                    Some(NonComparability::GraderFailure {
                                        trial: fact.id.clone(),
                                    })
                                }
                            })
                    {
                        comparability = Comparability::NonComparable(state);
                    } else if let Some(control) = [baseline, candidate]
                        .iter()
                        .flat_map(|fact| fact.unsupported_controls.iter())
                        .next()
                    {
                        comparability =
                            Comparability::NonComparable(NonComparability::UnsupportedControl {
                                control: control.clone(),
                            });
                    } else if baseline.control_fingerprint != candidate.control_fingerprint {
                        comparability =
                            Comparability::NonComparable(NonComparability::ControlMismatch {
                                baseline: baseline.control_fingerprint.clone(),
                                candidate: candidate.control_fingerprint.clone(),
                            });
                    }
                }
                pairs.push(PairedComparison {
                    edge: edge.id.clone(),
                    task,
                    group,
                    baseline_trial,
                    candidate_trial,
                    comparability,
                });
            }
        }
        Ok(ComparisonSet { pairs })
    }

    /// All assembled pairs, ordered by edge, task, then group.
    pub(crate) fn pairs(&self) -> &[PairedComparison] {
        &self.pairs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrity::{IntegrityRecord, IntegrityReview, OraclePreflight};
    use std::collections::BTreeMap;

    fn record_review(tasks: BTreeMap<String, TaskClassification>) -> IntegrityReview {
        IntegrityReview {
            benchmark: "terminal-bench".to_owned(),
            revision: "0.9.2".to_owned(),
            dataset: "terminal-bench-0.9.2".to_owned(),
            grader: "tb-native".to_owned(),
            environment: "tb-env-1".to_owned(),
            upstream_identity: "terminal-bench/tasks".to_owned(),
            upstream_digest: "sha256:1a".to_owned(),
            oracle: Some(OraclePreflight::Passed("oracle 41/41".to_owned())),
            status: crate::integrity::RevisionStatus::Admitted,
            tasks,
            excluded_trials: BTreeMap::new(),
            reviewer: "human-reviewer-1".to_owned(),
        }
    }

    fn valid_tasks() -> BTreeMap<String, TaskClassification> {
        BTreeMap::from([
            ("task-1".to_owned(), TaskClassification::ValidAgentOutcome),
            ("task-2".to_owned(), TaskClassification::ValidAgentOutcome),
        ])
    }

    fn experiment_toml(integrity_digest: &str) -> String {
        format!(
            r#"schema = "phase18-experiment/1"
experiment_id = "exp-1"

[benchmark]
name = "terminal-bench"
revision = "0.9.2"
dataset = "terminal-bench-0.9.2"
integrity_digest = "{integrity_digest}"

[[subjects]]
id = "subject-a"
product = "alpha"
version = "1.0"

[[subjects]]
id = "subject-b"
product = "beta"
version = "2.0"

[[edges]]
id = "e1"
baseline = "subject-a"
candidate = "subject-b"

[model_controls]
provider = "p"
model = "m"
endpoint_class = "local"
temperature = 0.0
max_output_tokens = 4096
reasoning = "omitted"

[environment]
platform = "linux"
architecture = "x86_64"
cwd_policy = "isolated"

[[trials]]
id = "t-a1"
subject = "subject-a"
task = "task-1"
group = "g1"

[[trials]]
id = "t-b1"
subject = "subject-b"
task = "task-1"
group = "g1"

[[trials]]
id = "t-a2"
subject = "subject-a"
task = "task-2"
group = "g1"

[[trials]]
id = "t-b2"
subject = "subject-b"
task = "task-2"
group = "g1"
"#
        )
    }

    fn fact(id: &str, subject: &str, task: &str, fingerprint: &str) -> TrialFact {
        TrialFact {
            id: id.to_owned(),
            subject: subject.to_owned(),
            task: task.to_owned(),
            group: "g1".to_owned(),
            manifest_digest: String::new(),
            control_fingerprint: fingerprint.to_owned(),
            unsupported_controls: Vec::new(),
            outcome: TrialOutcome::ValidAgentOutcome,
        }
    }

    fn assemble_with(
        record: &IntegrityRecord,
        facts: Vec<TrialFact>,
    ) -> Result<ComparisonSet, ComparisonError> {
        let experiment =
            ResolvedExperiment::resolve(&experiment_toml(record.identity_digest())).unwrap();
        let mut facts = facts;
        for f in &mut facts {
            if f.manifest_digest.is_empty() {
                f.manifest_digest = experiment.manifest_digest().to_owned();
            }
        }
        ComparisonSet::assemble(&experiment, record, &facts)
    }

    fn assemble_with_foreign(
        record: &crate::integrity::IntegrityRecord,
        facts: Vec<TrialFact>,
    ) -> Result<ComparisonSet, ComparisonError> {
        let experiment = ResolvedExperiment::resolve(&experiment_toml("deadbeef")).unwrap();
        let mut facts = facts;
        for f in &mut facts {
            if f.manifest_digest.is_empty() {
                f.manifest_digest = experiment.manifest_digest().to_owned();
            }
        }
        ComparisonSet::assemble(&experiment, record, &facts)
    }

    #[test]
    fn assemble_fails_closed_without_admitted_addressed_record() {
        let record = IntegrityRecord::review(record_review(valid_tasks())).unwrap();

        // Experiment froze a digest that addresses a different record.
        let err = assemble_with_foreign(&record, vec![]).unwrap_err();
        assert_eq!(
            err,
            ComparisonError::IntegrityDigestMismatch {
                experiment: Some("deadbeef".to_owned()),
                record: record.identity_digest().to_owned(),
            }
        );

        // Experiment never froze an integrity digest.
        let no_digest_toml = experiment_toml("").replace("\nintegrity_digest = \"\"\n", "\n");
        let err = ComparisonSet::assemble(
            &ResolvedExperiment::resolve(&no_digest_toml).unwrap(),
            &record,
            &[],
        )
        .unwrap_err();
        assert_eq!(
            err,
            ComparisonError::IntegrityDigestMismatch {
                experiment: None,
                record: record.identity_digest().to_owned(),
            }
        );

        // Record does not admit the revision it addresses.
        let not_admitted = record
            .with_status(crate::integrity::RevisionStatus::NotAdmitted)
            .unwrap();
        let err = assemble_with(&not_admitted, vec![]).unwrap_err();
        assert!(matches!(err, ComparisonError::NotAdmitted { .. }));
    }

    #[test]
    fn assemble_rejects_facts_that_contradict_the_frozen_contract() {
        let record = IntegrityRecord::review(record_review(valid_tasks())).unwrap();
        let experiment =
            ResolvedExperiment::resolve(&experiment_toml(record.identity_digest())).unwrap();
        let manifest = experiment.manifest_digest().to_owned();

        // Undeclared trial id.
        let mut f = fact("t-x", "subject-a", "task-1", "ctrl-1");
        f.manifest_digest = manifest.clone();
        let err = ComparisonSet::assemble(&experiment, &record, &[f.clone()]).unwrap_err();
        assert_eq!(
            err,
            ComparisonError::UnknownTrial {
                trial: "t-x".to_owned()
            }
        );

        // Fact subject disagrees with the declared trial.
        let mut f = fact("t-a1", "subject-b", "task-1", "ctrl-1");
        f.manifest_digest = manifest.clone();
        let err = ComparisonSet::assemble(&experiment, &record, &[f]).unwrap_err();
        assert_eq!(
            err,
            ComparisonError::UnknownFactSubject {
                trial: "t-a1".to_owned(),
                subject: "subject-b".to_owned(),
            }
        );

        // Same fact supplied twice.
        let mut f = fact("t-a1", "subject-a", "task-1", "ctrl-1");
        f.manifest_digest = manifest.clone();
        let err = ComparisonSet::assemble(&experiment, &record, &[f.clone(), f]).unwrap_err();
        assert_eq!(
            err,
            ComparisonError::DuplicateTrialFact {
                trial: "t-a1".to_owned()
            }
        );

        // Fact claims a foreign experiment manifest.
        let mut f = fact("t-a1", "subject-a", "task-1", "ctrl-1");
        f.manifest_digest = "foreign".to_owned();
        let err = ComparisonSet::assemble(&experiment, &record, &[f]).unwrap_err();
        assert_eq!(
            err,
            ComparisonError::ManifestDigestMismatch {
                trial: "t-a1".to_owned()
            }
        );
    }

    #[test]
    fn assemble_builds_one_comparable_pair_per_edge_task_and_group() {
        let record = IntegrityRecord::review(record_review(valid_tasks())).unwrap();
        let set = assemble_with(
            &record,
            vec![
                fact("t-a1", "subject-a", "task-1", "ctrl-1"),
                fact("t-b1", "subject-b", "task-1", "ctrl-1"),
                fact("t-a2", "subject-a", "task-2", "ctrl-1"),
                fact("t-b2", "subject-b", "task-2", "ctrl-1"),
            ],
        )
        .unwrap();

        let pairs = set.pairs();
        assert_eq!(pairs.len(), 2);
        let (first, second) = (&pairs[0], &pairs[1]);
        assert_eq!(first.edge(), "e1");
        assert_eq!(first.task(), "task-1");
        assert_eq!(first.group(), "g1");
        assert_eq!(first.baseline_trial(), "t-a1");
        assert_eq!(first.candidate_trial(), "t-b1");
        assert!(first.comparability().is_comparable());
        assert_eq!(second.task(), "task-2");
        assert_eq!(second.baseline_trial(), "t-a2");
        assert_eq!(second.candidate_trial(), "t-b2");
        assert!(second.comparability().is_comparable());
    }

    #[test]
    fn pair_states_remain_typed_and_visible() {
        // Missing candidate trial stays a visible pair state.
        let record = IntegrityRecord::review(record_review(valid_tasks())).unwrap();
        let set =
            assemble_with(&record, vec![fact("t-a1", "subject-a", "task-1", "ctrl-1")]).unwrap();
        assert_eq!(set.pairs().len(), 2);
        assert_eq!(
            set.pairs()[0].comparability(),
            &Comparability::NonComparable(NonComparability::MissingCandidateTrial)
        );
        // A missing trial in one pair never fails the assembly: the
        // untouched sibling pair assembles as a visible missing state too.
        assert_eq!(
            set.pairs()[1].comparability(),
            &Comparability::NonComparable(NonComparability::MissingBaselineTrial)
        );

        // Control-fingerprint mismatch invalidates only the pair.
        let record = IntegrityRecord::review(record_review(valid_tasks())).unwrap();
        let set = assemble_with(
            &record,
            vec![
                fact("t-a1", "subject-a", "task-1", "ctrl-1"),
                fact("t-b1", "subject-b", "task-1", "ctrl-2"),
                fact("t-a2", "subject-a", "task-2", "ctrl-1"),
                fact("t-b2", "subject-b", "task-2", "ctrl-1"),
            ],
        )
        .unwrap();
        assert_eq!(
            set.pairs()[0].comparability(),
            &Comparability::NonComparable(NonComparability::ControlMismatch {
                baseline: "ctrl-1".to_owned(),
                candidate: "ctrl-2".to_owned(),
            })
        );

        // Unsupported shared control marks the pair non-comparable.
        let record = IntegrityRecord::review(record_review(valid_tasks())).unwrap();
        let mut unsupported = fact("t-b1", "subject-b", "task-1", "ctrl-1");
        unsupported.unsupported_controls = vec!["reasoning".to_owned()];
        let set = assemble_with(
            &record,
            vec![
                fact("t-a1", "subject-a", "task-1", "ctrl-1"),
                unsupported,
                fact("t-a2", "subject-a", "task-2", "ctrl-1"),
                fact("t-b2", "subject-b", "task-2", "ctrl-1"),
            ],
        )
        .unwrap();
        assert_eq!(
            set.pairs()[0].comparability(),
            &Comparability::NonComparable(NonComparability::UnsupportedControl {
                control: "reasoning".to_owned(),
            })
        );

        // A frozen contract declaring two trials for one pairing slot is
        // ambiguous and fails the assembly even with no settled facts.
        let record = IntegrityRecord::review(record_review(valid_tasks())).unwrap();
        let conflict_toml = experiment_toml(record.identity_digest()).replace(
            "[[trials]]\nid = \"t-a1\"\nsubject = \"subject-a\"\ntask = \"task-1\"\ngroup = \"g1\"\n",
            "[[trials]]\nid = \"t-a1\"\nsubject = \"subject-a\"\ntask = \"task-1\"\ngroup = \"g1\"\n\n[[trials]]\nid = \"t-a1b\"\nsubject = \"subject-a\"\ntask = \"task-1\"\ngroup = \"g1\"\n",
        );
        let experiment = ResolvedExperiment::resolve(&conflict_toml).unwrap();
        let err = ComparisonSet::assemble(&experiment, &record, &[]).unwrap_err();
        assert!(matches!(
            err,
            ComparisonError::DuplicateRoleTrial {
                role: "baseline",
                ..
            }
        ));
    }

    #[test]
    fn exclusions_failures_and_invalid_tasks_stay_typed_and_visible() {
        // Excluded trial with its stable reason.
        let mut review = record_review(valid_tasks());
        review
            .excluded_trials
            .insert("t-b1".to_owned(), "workspace vanished".to_owned());
        let record = IntegrityRecord::review(review).unwrap();
        let set = assemble_with(
            &record,
            vec![
                fact("t-a1", "subject-a", "task-1", "ctrl-1"),
                fact("t-b1", "subject-b", "task-1", "ctrl-1"),
                fact("t-a2", "subject-a", "task-2", "ctrl-1"),
                fact("t-b2", "subject-b", "task-2", "ctrl-1"),
            ],
        )
        .unwrap();
        assert_eq!(
            set.pairs()[0].comparability(),
            &Comparability::NonComparable(NonComparability::Excluded {
                trial: "t-b1".to_owned(),
                reason: "workspace vanished".to_owned(),
            })
        );

        // Infrastructure and grader trial outcomes; a scored Agent failure
        // keeps the pair comparable - it is a graded Agent outcome that
        // stays in the denominator.
        for (trial, outcome, expected) in [
            (
                "t-b1",
                TrialOutcome::InfrastructureFailure,
                NonComparability::InfrastructureFailure {
                    trial: "t-b1".to_owned(),
                },
            ),
            (
                "t-b1",
                TrialOutcome::GraderFailure,
                NonComparability::GraderFailure {
                    trial: "t-b1".to_owned(),
                },
            ),
        ] {
            let record = IntegrityRecord::review(record_review(valid_tasks())).unwrap();
            let mut f = fact(trial, "subject-b", "task-1", "ctrl-1");
            f.outcome = outcome;
            let set = assemble_with(
                &record,
                vec![
                    fact("t-a1", "subject-a", "task-1", "ctrl-1"),
                    f,
                    fact("t-a2", "subject-a", "task-2", "ctrl-1"),
                    fact("t-b2", "subject-b", "task-2", "ctrl-1"),
                ],
            )
            .unwrap();
            assert_eq!(
                set.pairs()[0].comparability(),
                &Comparability::NonComparable(expected)
            );
        }

        // A scored Agent failure on one side keeps the pair comparable:
        // the Agent outcome stays in the graded denominator.
        let record = IntegrityRecord::review(record_review(valid_tasks())).unwrap();
        let mut scored = fact("t-b1", "subject-b", "task-1", "ctrl-1");
        scored.outcome = TrialOutcome::AgentFailure;
        let set = assemble_with(
            &record,
            vec![
                fact("t-a1", "subject-a", "task-1", "ctrl-1"),
                scored,
                fact("t-a2", "subject-a", "task-2", "ctrl-1"),
                fact("t-b2", "subject-b", "task-2", "ctrl-1"),
            ],
        )
        .unwrap();
        assert!(
            set.pairs()[0].comparability().is_comparable(),
            "a scored Agent failure stays a comparable graded outcome"
        );

        // Non-valid task classification and uncovered task.
        let mut tasks = valid_tasks();
        tasks.insert(
            "task-2".to_owned(),
            TaskClassification::GraderFailure {
                reason: "oracle flaked".to_owned(),
            },
        );
        let record = IntegrityRecord::review(record_review(tasks)).unwrap();
        let set = assemble_with(
            &record,
            vec![
                fact("t-a1", "subject-a", "task-1", "ctrl-1"),
                fact("t-b1", "subject-b", "task-1", "ctrl-1"),
                fact("t-a2", "subject-a", "task-2", "ctrl-1"),
                fact("t-b2", "subject-b", "task-2", "ctrl-1"),
            ],
        )
        .unwrap();
        assert_eq!(
            set.pairs()[1].comparability(),
            &Comparability::NonComparable(NonComparability::InvalidTaskClassification {
                classification: TaskClassification::GraderFailure {
                    reason: "oracle flaked".to_owned(),
                },
            })
        );

        let record = IntegrityRecord::review(record_review(BTreeMap::from([(
            "task-1".to_owned(),
            TaskClassification::ValidAgentOutcome,
        )])))
        .unwrap();
        let set = assemble_with(
            &record,
            vec![
                fact("t-a1", "subject-a", "task-1", "ctrl-1"),
                fact("t-b1", "subject-b", "task-1", "ctrl-1"),
                fact("t-a2", "subject-a", "task-2", "ctrl-1"),
                fact("t-b2", "subject-b", "task-2", "ctrl-1"),
            ],
        )
        .unwrap();
        assert_eq!(
            set.pairs()[1].comparability(),
            &Comparability::NonComparable(NonComparability::TaskNotCovered)
        );
    }
}
