//! Crate-private shared benchmark execution contract (Phase 18 task 18.8).
//!
//! [`BenchmarkExecution`] consumes [`crate::process::ProcessSupervisor`], an
//! admitted external lock digest, a read-only [`crate::integrity::IntegrityRecord`],
//! and one [`BenchmarkAdapter`] to produce a single settled
//! [`BenchmarkRecord`]: exit state, bounded captures, cleanup evidence,
//! native metrics with their benchmark-defined names, the native reward as a
//! typed fact, content-addressed native artifacts, and a typed completion
//! verdict mapped into [`crate::failure::FailureBoundaryCode`]. The contract
//! is benchmark-neutral: task-package shape, verifier argv, native schemas,
//! and failure mapping are owned by each adapter, never by this module
//! (`P18-BMK-005`, `P18-BMK-007`).

use crate::agent::process::{Fact, NativeArtifact};
use crate::failure::FailureBoundaryCode;
use crate::integrity::IntegrityRecord;
use crate::process::{
    CleanupEvidence, ExitState, OutputCapture, ProcessSupervisor, SpawnReason, SpawnSpec,
};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

/// Exact benchmark-verifier identity retained on every settled record
/// (`P18-BMK-001`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BenchmarkIdentity {
    /// Benchmark family, e.g. `terminal-bench`.
    pub benchmark: String,
    /// Admitted revision identity, e.g. `2.1`.
    pub revision: String,
    /// Adapter identity plus its contract identity.
    pub adapter: String,
}

/// One frozen verification request: the exact resolved verifier executable,
/// the materialized task package, the sealed Agent output, the admitted
/// external lock digest, and the immutable integrity admission. Limits come
/// from the adapter's declarative profile.
#[derive(Debug, Clone)]
pub(crate) struct BenchmarkRunRequest {
    /// Resolved verifier entry executable (argv[0]; e.g. the pinned `uv`
    /// that drives `harbor --locked`). Never resolved from ambient PATH.
    pub verifier_executable: PathBuf,
    /// Materialized complete task package root (validated by the adapter).
    pub task_dir: PathBuf,
    /// Task id inside the admitted revision.
    pub task_id: String,
    /// Sealed final Agent output the verifier grades.
    #[expect(
        dead_code,
        reason = "benchmark request retains the sealed Agent-output identity required by the contract"
    )]
    pub agent_output: PathBuf,
    /// Fresh capture root for native verifier outputs.
    pub trace_root: PathBuf,
    /// Digest of the admitted resolved external lock.
    /// The external-lock verifier scripts bind this digest to the admitted
    /// lock under `crates/opi-eval/external-locks/`.
    pub admitted_lock_digest: String,
    /// Immutable integrity record admitting the revision and classifying the
    /// task. Consumed read-only; no adapter or Agent path can mutate it
    /// (`P18-INT-003`).
    pub integrity: IntegrityRecord,
    /// Additional exact environment entries.
    pub extra_env: BTreeMap<OsString, OsString>,
}

/// Native metrics with their benchmark-defined names retained verbatim
/// (`P18-BMK-007`): one fact per CTRF summary counter, never a composite
/// score, never renamed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct NativeMetrics {
    pub tests: Option<Fact>,
    pub passed: Option<Fact>,
    pub failed: Option<Fact>,
    pub skipped: Option<Fact>,
    pub pending: Option<Fact>,
    pub other: Option<Fact>,
}

/// Typed rejections of the harbor `jobs/<timestamp>/result.json` layout
/// (`harbor run -p`, task 18.15 pin): the newest job directory is the
/// authority and the single trial's verifier rewards are the aggregate.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum HarborResultError {
    /// No `jobs/*/result.json` exists under the trace root.
    Missing,
    /// The file exists but is not the pinned schema; the token names the
    /// drift precisely (read, parse, trial list, reward chain, values).
    Invalid(&'static str),
}

/// Locates the newest `jobs/<timestamp>/result.json` under `trace_root`.
fn newest_job_result(trace_root: &std::path::Path) -> Option<PathBuf> {
    let jobs = trace_root.join("jobs");
    let entries = std::fs::read_dir(&jobs).ok()?;
    let mut newest: Option<(std::path::PathBuf, std::ffi::OsString)> = None;
    for entry in entries.flatten() {
        let name = entry.file_name();
        if entry.path().join("result.json").is_file()
            && newest.as_ref().is_none_or(|(_, best)| name > *best)
        {
            newest = Some((entry.path().join("result.json"), name));
        }
    }
    newest.map(|(path, _)| path)
}

/// Locates the newest `jobs/<timestamp>/result.json` under `trace_root`
/// and derives the aggregate metrics from the single trial's verifier
/// rewards. Per-test counters stay typed unknowns: the harbor result
/// exposes only the reward aggregate, and nothing is inferred beyond it.
pub(crate) fn import_harbor_result(
    trace_root: &std::path::Path,
) -> Result<(NativeMetrics, Fact, PathBuf, serde_json::Value), HarborResultError> {
    let path = newest_job_result(trace_root).ok_or(HarborResultError::Missing)?;
    let bytes = std::fs::read(&path).map_err(|_| HarborResultError::Invalid("read"))?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| HarborResultError::Invalid("json-parse"))?;
    // Harbor's completion path writes the job-level result with the
    // trial list excluded (per-trial results live in the trial
    // directories); the aggregate authority here is
    // stats.evals.<eval>.reward_stats, a map from reward value to the
    // trial names that earned it. One job, one eval, one trial: any
    // broader shape is drift and fails closed.
    if value.get("n_total_trials").and_then(|n| n.as_u64()) != Some(1) {
        return Err(HarborResultError::Invalid("trial-count"));
    }
    let evals = value
        .get("stats")
        .and_then(|s| s.get("evals"))
        .and_then(|e| e.as_object())
        .ok_or(HarborResultError::Invalid("no-eval-stats"))?;
    if evals.len() != 1 {
        return Err(HarborResultError::Invalid("eval-count"));
    }
    let reward_stats = evals
        .values()
        .next()
        .and_then(|stats| stats.get("reward_stats"))
        .and_then(|r| r.as_object())
        .ok_or(HarborResultError::Invalid("no-reward-stats"))?;
    if reward_stats.len() != 1 {
        return Err(HarborResultError::Invalid("reward-count"));
    }
    // reward_stats nests twice: metric name -> { reward value -> trial
    // names }. One metric, one value, one trial; anything else is drift.
    let (_metric, by_value) = reward_stats.iter().next().expect("len checked");
    let by_value = by_value
        .as_object()
        .ok_or(HarborResultError::Invalid("reward-values"))?;
    if by_value.len() != 1 {
        return Err(HarborResultError::Invalid("reward-count"));
    }
    let (reward_key, trial_names) = by_value.iter().next().expect("len checked");
    let trial_names = trial_names
        .as_array()
        .ok_or(HarborResultError::Invalid("reward-trials"))?;
    if trial_names.len() != 1 {
        return Err(HarborResultError::Invalid("reward-trials"));
    }
    let reward: f64 = reward_key
        .parse()
        .map_err(|_| HarborResultError::Invalid("bad-reward-values"))?;
    if !reward.is_finite() {
        return Err(HarborResultError::Invalid("bad-reward-values"));
    }
    // The aggregate convention: the single `reward` metric is the
    // verifier-reported integration reward, a zero-or-one value. That
    // is the only counter the aggregate can honestly carry; the rest
    // stay unknown.
    if reward.fract() != 0.0 || !(0.0..=1.0).contains(&reward) {
        return Err(HarborResultError::Invalid("bad-reward-values"));
    }
    let reward_fact = Fact::Known {
        value: reward as u64,
        origin: "harbor-result".to_owned(),
    };
    let metrics = NativeMetrics {
        tests: None,
        passed: Some(Fact::Known {
            value: reward as u64,
            origin: "harbor-reward".to_owned(),
        }),
        failed: None,
        skipped: None,
        pending: None,
        other: None,
    };
    Ok((metrics, reward_fact, path, value))
}

/// Locates the newest `jobs/<timestamp>/result.json` under `trace_root`
/// and validates it as Pier's job-result aggregate (`pier run -p`, task
/// 18.15 pin). Unlike the Terminal-Bench aggregate, a DeepSWE job scores
/// multiple verifier metrics per trial (F2P, P2P, partial, ...), so the
/// structural contract is validated - one trial, at least one eval,
/// every metric awarding exactly one reward to exactly that one trial -
/// while no metric is translated into a test counter; callers keep the
/// reward semantics unknown instead of guessing an aggregate.
pub(crate) fn import_pier_job_result(
    trace_root: &std::path::Path,
) -> Result<(PathBuf, Fact, serde_json::Value), HarborResultError> {
    let path = newest_job_result(trace_root).ok_or(HarborResultError::Missing)?;
    let bytes = std::fs::read(&path).map_err(|_| HarborResultError::Invalid("read"))?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| HarborResultError::Invalid("json-parse"))?;
    if value.get("n_total_trials").and_then(|n| n.as_u64()) != Some(1) {
        return Err(HarborResultError::Invalid("trial-count"));
    }
    let evals = value
        .get("stats")
        .and_then(|s| s.get("evals"))
        .and_then(|e| e.as_object())
        .ok_or(HarborResultError::Invalid("no-eval-stats"))?;
    if evals.is_empty() {
        return Err(HarborResultError::Invalid("eval-count"));
    }
    // The first eval's aggregate carries the authoritative `reward`
    // metric (the verifier-reported zero-or-one integration reward);
    // the remaining keys are score breakdowns the adapter never
    // translates into test counters.
    let mut reward_fact: Option<Fact> = None;
    for stats in evals.values() {
        let reward_stats = stats
            .get("reward_stats")
            .and_then(|r| r.as_object())
            .ok_or(HarborResultError::Invalid("no-reward-stats"))?;
        // reward_stats nests twice: metric name -> { reward value ->
        // trial names }. Every metric awards exactly one finite reward
        // to exactly the one trial; anything else is drift.
        for (metric, by_value) in reward_stats {
            let by_value = by_value
                .as_object()
                .ok_or(HarborResultError::Invalid("reward-values"))?;
            if by_value.len() != 1 {
                return Err(HarborResultError::Invalid("reward-count"));
            }
            let (reward_key, trial_names) = by_value.iter().next().expect("len checked");
            let trial_names = trial_names
                .as_array()
                .ok_or(HarborResultError::Invalid("reward-trials"))?;
            if trial_names.len() != 1 {
                return Err(HarborResultError::Invalid("reward-trials"));
            }
            let reward: f64 = reward_key
                .parse()
                .map_err(|_| HarborResultError::Invalid("bad-reward-values"))?;
            if !reward.is_finite() {
                return Err(HarborResultError::Invalid("bad-reward-values"));
            }
            if metric == "reward" {
                // Only the authoritative aggregate reward lives in the
                // zero-or-one domain. Native score breakdowns retain their
                // benchmark-defined finite numeric values without being
                // translated into shared counters.
                if reward.fract() != 0.0 || !(0.0..=1.0).contains(&reward) {
                    return Err(HarborResultError::Invalid("bad-reward-values"));
                }
                reward_fact = Some(Fact::Known {
                    value: reward as u64,
                    origin: "pier-result".to_owned(),
                });
            }
        }
    }
    let reward_fact = reward_fact.ok_or(HarborResultError::Invalid("no-reward-key"))?;
    Ok((path, reward_fact, value))
}

/// Typed failure carried on a settled record. A failed verification run is
/// settled evidence, not an unsettable error (`P18-BMK-006`: no fallback to
/// another revision, grader, cached score, heuristic, or LLM judgment).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BenchmarkFailure {
    /// Static redacted token describing the failure kind.
    pub kind: &'static str,
    /// Owning failure boundary.
    pub boundary: FailureBoundaryCode,
}

/// Authoritative completion verdict of one native verification run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BenchmarkCompletion {
    /// The verifier ran to completion and its native output was imported.
    Verified {
        /// Native summary counters with benchmark-defined names.
        metrics: NativeMetrics,
        /// Raw native outputs as content-addressed references.
        artifacts: Vec<NativeArtifact>,
    },
    /// The run settled as a typed failure.
    Failed(BenchmarkFailure),
}

/// Provenance binding one settled record to its admitted inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BenchmarkProvenance {
    /// Digest of the admitted resolved external lock.
    pub admitted_lock_digest: String,
    /// Identity digest of the consumed integrity record.
    pub integrity_digest: String,
    /// Revision the integrity record admitted.
    pub revision: String,
    /// Task id verified.
    pub task_id: String,
}

/// The settled benchmark record: one per native verification run, on every
/// path.
#[derive(Debug, Clone)]
pub(crate) struct BenchmarkRecord {
    pub identity: BenchmarkIdentity,
    pub exit: ExitState,
    pub stdout: OutputCapture,
    pub stderr: OutputCapture,
    #[expect(
        dead_code,
        reason = "settled benchmark record retains process-cleanup evidence"
    )]
    pub cleanup: CleanupEvidence,
    /// Wall-clock duration of the supervised verification run.
    pub wall_time: Duration,
    /// Native reward as a typed fact. Until the real native smoke captures
    /// the harbor reward path it stays `Unknown` with a typed reason — it is
    /// never computed from the summary counters (`P18-BMK-007`).
    pub reward: Fact,
    pub completion: BenchmarkCompletion,
    pub provenance: BenchmarkProvenance,
}

impl BenchmarkRecord {
    /// The failure boundary of a failed completion, or `None` when verified.
    #[cfg_attr(
        not(all(test, unix)),
        expect(
            dead_code,
            reason = "failure-boundary projection is exercised by Unix process tests"
        )
    )]
    pub(crate) fn failure_boundary(&self) -> Option<FailureBoundaryCode> {
        match &self.completion {
            BenchmarkCompletion::Verified { .. } => None,
            BenchmarkCompletion::Failed(failure) => Some(failure.boundary),
        }
    }
}

/// The benchmark-neutral benchmark-specific seam: one implementation per
/// pinned benchmark revision. Crate-private because the only consumers are
/// this crate's execution driver and the shared conformance suite.
pub(crate) trait BenchmarkAdapter {
    /// Exact identity declared by the declarative profile.
    fn identity(&self) -> BenchmarkIdentity;

    /// Revision-specific admission check run before any spawn: the request's
    /// integrity admission, task id, and lock must bind to this adapter's
    /// pinned revision.
    fn admission(&self, request: &BenchmarkRunRequest) -> Result<(), ExecutionError>;

    /// Structured spawn request for one verification run. Owns argv, cwd,
    /// environment projection, and limits from the declarative profile.
    fn spawn_spec(&self, request: &BenchmarkRunRequest) -> SpawnSpec;

    /// Settle one supervised outcome into the native half of the record:
    /// imported metrics, reward fact, artifacts, and the authoritative
    /// completion verdict. Infallible: every observed verifier failure
    /// settles inside the record.
    fn settle(
        &self,
        outcome: &crate::process::SupervisedOutcome,
        request: &BenchmarkRunRequest,
    ) -> (Fact, BenchmarkCompletion);
}

/// Pre-spawn contract violation: the request itself cannot produce a legal
/// verification. Static tokens only; request bytes are not echoed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExecutionError {
    pub token: &'static str,
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "benchmark execution rejected: {}", self.token)
    }
}

impl std::error::Error for ExecutionError {}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// The shared execution driver. One verification run, one settled record,
/// every path.
pub(crate) struct BenchmarkExecution;

impl BenchmarkExecution {
    /// Run `request` under `adapter` to settlement.
    ///
    /// Never panics and never returns an `Err` for an observed run: spawn
    /// failures, non-zero exits, timeouts, cancellations, and invalid native
    /// output all settle inside the [`BenchmarkRecord`]. `Err` means the
    /// request was structurally unusable before any process existed: an
    /// unadmitted revision, an invalid task classification, or a malformed
    /// admitted lock digest.
    pub(crate) async fn run(
        request: &BenchmarkRunRequest,
        adapter: &dyn BenchmarkAdapter,
        cancel: &CancellationToken,
    ) -> Result<BenchmarkRecord, ExecutionError> {
        if request.task_id.trim().is_empty() {
            return Err(ExecutionError {
                token: "empty-task-id",
            });
        }
        if !is_sha256_hex(&request.admitted_lock_digest) {
            return Err(ExecutionError {
                token: "lock-digest-malformed",
            });
        }
        if !request.integrity.admitted() {
            return Err(ExecutionError {
                token: "revision-not-admitted",
            });
        }
        if !request
            .integrity
            .task_classification(&request.task_id)
            .is_some_and(|c| c.is_valid_agent_outcome())
        {
            return Err(ExecutionError {
                token: "task-not-valid",
            });
        }
        adapter.admission(request)?;
        let identity = adapter.identity();
        let spec = adapter.spawn_spec(request);
        let started = Instant::now();
        let outcome = ProcessSupervisor::run(&spec, cancel).await;
        let wall_time = started.elapsed();
        let (reward, completion) = adapter.settle(&outcome, request);
        Ok(BenchmarkRecord {
            identity,
            exit: outcome.exit,
            stdout: outcome.stdout,
            stderr: outcome.stderr,
            cleanup: outcome.cleanup,
            wall_time,
            reward,
            completion,
            provenance: BenchmarkProvenance {
                admitted_lock_digest: request.admitted_lock_digest.clone(),
                integrity_digest: request.integrity.identity_digest().to_owned(),
                revision: request.integrity.revision().to_owned(),
                task_id: request.task_id.clone(),
            },
        })
    }
}

/// Failure kinds for settled records, kept as static tokens.
pub(crate) mod failure_kinds {
    use crate::failure::FailureBoundaryCode;

    /// Map a verifier spawn failure to its settled token + boundary. Tool or
    /// runner acquisition failures are infrastructure, not grader, failures.
    pub(crate) fn spawn(reason: super::SpawnReason) -> super::BenchmarkFailure {
        let token = match reason {
            super::SpawnReason::NotFound => "spawn-not-found",
            super::SpawnReason::PermissionDenied => "spawn-permission-denied",
            super::SpawnReason::BadCwd => "spawn-bad-cwd",
            super::SpawnReason::SpawnFailed => "spawn-failed",
        };
        super::BenchmarkFailure {
            kind: token,
            boundary: FailureBoundaryCode::Infrastructure,
        }
    }

    /// Non-zero verifier exit (`P18-BMK-006`: authoritative, no fallback).
    pub(crate) fn non_zero_exit(_code: i32) -> super::BenchmarkFailure {
        super::BenchmarkFailure {
            kind: "verifier-non-zero-exit",
            boundary: FailureBoundaryCode::Grader,
        }
    }

    /// Verifier timeout settled by the supervisor.
    pub(crate) const TIMED_OUT: super::BenchmarkFailure = super::BenchmarkFailure {
        kind: "verifier-timeout",
        boundary: FailureBoundaryCode::Grader,
    };

    /// Verifier cancellation settled by the supervisor.
    pub(crate) const CANCELLED: super::BenchmarkFailure = super::BenchmarkFailure {
        kind: "verifier-cancelled",
        boundary: FailureBoundaryCode::Grader,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrity::{
        IntegrityRecord, IntegrityReview, OraclePreflight, RevisionStatus, TaskClassification,
    };
    use std::collections::BTreeMap;

    /// A helper adapter proving the shared driver without any benchmark
    /// semantics: argv is a real process, settle maps exit 0 to Verified and
    /// everything else to the shared failure kinds.
    struct HelperAdapter;

    impl BenchmarkAdapter for HelperAdapter {
        fn identity(&self) -> BenchmarkIdentity {
            BenchmarkIdentity {
                benchmark: "helper".to_owned(),
                revision: "helper-1".to_owned(),
                adapter: "helper-adapter/0".to_owned(),
            }
        }

        fn admission(&self, _request: &BenchmarkRunRequest) -> Result<(), ExecutionError> {
            Ok(())
        }

        fn spawn_spec(&self, _request: &BenchmarkRunRequest) -> SpawnSpec {
            SpawnSpec {
                argv: vec!["/usr/bin/true".into()],
                cwd: None,
                env: BTreeMap::new(),
                stdout_cap: 1024,
                stderr_cap: 1024,
                timeout: Duration::from_secs(10),
            }
        }

        fn settle(
            &self,
            outcome: &crate::process::SupervisedOutcome,
            _request: &BenchmarkRunRequest,
        ) -> (Fact, BenchmarkCompletion) {
            let completion = match outcome.exit {
                ExitState::Exited { code: 0 } => BenchmarkCompletion::Verified {
                    metrics: NativeMetrics::default(),
                    artifacts: vec![],
                },
                ExitState::Exited { code } => {
                    BenchmarkCompletion::Failed(failure_kinds::non_zero_exit(code))
                }
                ExitState::TimedOut => BenchmarkCompletion::Failed(failure_kinds::TIMED_OUT),
                ExitState::Cancelled => BenchmarkCompletion::Failed(failure_kinds::CANCELLED),
                ExitState::FailedToSpawn { reason } => {
                    BenchmarkCompletion::Failed(failure_kinds::spawn(reason))
                }
            };
            (
                Fact::Unknown {
                    reason: "native-reward-pending-18-15-smoke".to_owned(),
                },
                completion,
            )
        }
    }

    fn integrity(status: RevisionStatus, task: TaskClassification) -> IntegrityRecord {
        IntegrityRecord::review(IntegrityReview {
            benchmark: "helper-bench".to_owned(),
            revision: "helper-1".to_owned(),
            dataset: "helper-dataset".to_owned(),
            grader: "helper-grader".to_owned(),
            environment: "helper-environment".to_owned(),
            upstream_identity: "helper-upstream".to_owned(),
            upstream_digest: "0".repeat(64),
            oracle: Some(OraclePreflight::Passed("six tests passed".to_owned())),
            status,
            tasks: BTreeMap::from([("helper-task".to_owned(), task)]),
            excluded_trials: BTreeMap::new(),
            reviewer: "human-reviewer".to_owned(),
        })
        .unwrap()
    }

    fn request(integrity: IntegrityRecord) -> BenchmarkRunRequest {
        BenchmarkRunRequest {
            verifier_executable: "/nonexistent/uv".into(),
            task_dir: "/nonexistent/task".into(),
            task_id: "helper-task".to_owned(),
            agent_output: "/nonexistent/agent-output".into(),
            trace_root: std::env::temp_dir(),
            admitted_lock_digest: "a".repeat(64),
            integrity,
            extra_env: BTreeMap::new(),
        }
    }

    #[tokio::test]
    async fn structurally_unusable_requests_reject_before_spawn() {
        // Unadmitted revision (P18-INT-001).
        let bad = request(integrity(
            RevisionStatus::NotAdmitted,
            TaskClassification::ValidAgentOutcome,
        ));
        assert_eq!(
            BenchmarkExecution::run(&bad, &HelperAdapter, &CancellationToken::new())
                .await
                .unwrap_err(),
            ExecutionError {
                token: "revision-not-admitted"
            }
        );

        // Task not classified as a valid agent outcome (P18-INT-002).
        for task in [
            TaskClassification::BrokenOrUnsatisfiable {
                reason: "unsatisfiable".to_owned(),
            },
            TaskClassification::InfrastructureFailure {
                reason: "flaky host".to_owned(),
            },
        ] {
            let bad = request(integrity(RevisionStatus::Admitted, task));
            assert_eq!(
                BenchmarkExecution::run(&bad, &HelperAdapter, &CancellationToken::new())
                    .await
                    .unwrap_err(),
                ExecutionError {
                    token: "task-not-valid"
                }
            );
        }

        // Missing task classification entirely.
        let mut unclassified = request(integrity(
            RevisionStatus::Admitted,
            TaskClassification::ValidAgentOutcome,
        ));
        unclassified.task_id = "not-in-record".to_owned();
        assert_eq!(
            BenchmarkExecution::run(&unclassified, &HelperAdapter, &CancellationToken::new())
                .await
                .unwrap_err(),
            ExecutionError {
                token: "task-not-valid"
            }
        );

        // Empty task id and malformed lock digest.
        let mut empty = request(integrity(
            RevisionStatus::Admitted,
            TaskClassification::ValidAgentOutcome,
        ));
        empty.task_id = "  ".to_owned();
        assert_eq!(
            BenchmarkExecution::run(&empty, &HelperAdapter, &CancellationToken::new())
                .await
                .unwrap_err(),
            ExecutionError {
                token: "empty-task-id"
            }
        );
        let mut bad_digest = request(integrity(
            RevisionStatus::Admitted,
            TaskClassification::ValidAgentOutcome,
        ));
        bad_digest.admitted_lock_digest = "not-a-digest".to_owned();
        assert_eq!(
            BenchmarkExecution::run(&bad_digest, &HelperAdapter, &CancellationToken::new())
                .await
                .unwrap_err(),
            ExecutionError {
                token: "lock-digest-malformed"
            }
        );

        // Adapter-owned admission runs after the shared gates.
        struct RejectingAdapter(HelperAdapter);
        impl BenchmarkAdapter for RejectingAdapter {
            fn identity(&self) -> BenchmarkIdentity {
                self.0.identity()
            }
            fn admission(&self, _request: &BenchmarkRunRequest) -> Result<(), ExecutionError> {
                Err(ExecutionError {
                    token: "revision-binding-mismatch",
                })
            }
            fn spawn_spec(&self, r: &BenchmarkRunRequest) -> SpawnSpec {
                self.0.spawn_spec(r)
            }
            fn settle(
                &self,
                o: &crate::process::SupervisedOutcome,
                r: &BenchmarkRunRequest,
            ) -> (Fact, BenchmarkCompletion) {
                self.0.settle(o, r)
            }
        }
        let good = request(integrity(
            RevisionStatus::Admitted,
            TaskClassification::ValidAgentOutcome,
        ));
        assert_eq!(
            BenchmarkExecution::run(
                &good,
                &RejectingAdapter(HelperAdapter),
                &CancellationToken::new()
            )
            .await
            .unwrap_err(),
            ExecutionError {
                token: "revision-binding-mismatch"
            }
        );
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn helper_run_settles_a_verified_record_with_provenance() {
        let record = integrity(
            RevisionStatus::Admitted,
            TaskClassification::ValidAgentOutcome,
        );
        let digest = record.identity_digest().to_owned();
        let request = request(record);
        let settled = BenchmarkExecution::run(&request, &HelperAdapter, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(settled.exit, ExitState::Exited { code: 0 });
        assert_eq!(
            settled.completion,
            BenchmarkCompletion::Verified {
                metrics: NativeMetrics::default(),
                artifacts: vec![]
            }
        );
        assert_eq!(settled.failure_boundary(), None);
        assert_eq!(
            settled.provenance,
            BenchmarkProvenance {
                admitted_lock_digest: "a".repeat(64),
                integrity_digest: digest,
                revision: "helper-1".to_owned(),
                task_id: "helper-task".to_owned(),
            }
        );
        // The reward fact stays a typed unknown until the real native smoke.
        assert_eq!(
            settled.reward,
            Fact::Unknown {
                reason: "native-reward-pending-18-15-smoke".to_owned()
            }
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn helper_verifier_failures_settle_with_typed_boundaries() {
        struct FailingHelper(HelperAdapter);
        impl BenchmarkAdapter for FailingHelper {
            fn identity(&self) -> BenchmarkIdentity {
                self.0.identity()
            }
            fn admission(&self, r: &BenchmarkRunRequest) -> Result<(), ExecutionError> {
                self.0.admission(r)
            }
            fn spawn_spec(&self, r: &BenchmarkRunRequest) -> SpawnSpec {
                let mut spec = self.0.spawn_spec(r);
                spec.argv = vec!["/bin/sh".into(), "-c".into(), "exit 4".into()];
                spec
            }
            fn settle(
                &self,
                o: &crate::process::SupervisedOutcome,
                r: &BenchmarkRunRequest,
            ) -> (Fact, BenchmarkCompletion) {
                self.0.settle(o, r)
            }
        }
        let request = request(integrity(
            RevisionStatus::Admitted,
            TaskClassification::ValidAgentOutcome,
        ));
        let settled = BenchmarkExecution::run(
            &request,
            &FailingHelper(HelperAdapter),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(settled.exit, ExitState::Exited { code: 4 });
        assert_eq!(
            settled.completion,
            BenchmarkCompletion::Failed(failure_kinds::non_zero_exit(4))
        );
        assert_eq!(
            settled.failure_boundary(),
            Some(FailureBoundaryCode::Grader)
        );

        struct SleepingHelper(HelperAdapter);
        impl BenchmarkAdapter for SleepingHelper {
            fn identity(&self) -> BenchmarkIdentity {
                self.0.identity()
            }
            fn admission(&self, r: &BenchmarkRunRequest) -> Result<(), ExecutionError> {
                self.0.admission(r)
            }
            fn spawn_spec(&self, r: &BenchmarkRunRequest) -> SpawnSpec {
                let mut spec = self.0.spawn_spec(r);
                spec.argv = vec!["/bin/sleep".into(), "15".into()];
                spec.timeout = Duration::from_millis(200);
                spec
            }
            fn settle(
                &self,
                o: &crate::process::SupervisedOutcome,
                r: &BenchmarkRunRequest,
            ) -> (Fact, BenchmarkCompletion) {
                self.0.settle(o, r)
            }
        }
        let settled = BenchmarkExecution::run(
            &request,
            &SleepingHelper(HelperAdapter),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(settled.exit, ExitState::TimedOut);
        assert_eq!(
            settled.completion,
            BenchmarkCompletion::Failed(failure_kinds::TIMED_OUT)
        );
        assert_eq!(
            settled.failure_boundary(),
            Some(FailureBoundaryCode::Grader)
        );

        struct MissingHelper(HelperAdapter);
        impl BenchmarkAdapter for MissingHelper {
            fn identity(&self) -> BenchmarkIdentity {
                self.0.identity()
            }
            fn admission(&self, r: &BenchmarkRunRequest) -> Result<(), ExecutionError> {
                self.0.admission(r)
            }
            fn spawn_spec(&self, r: &BenchmarkRunRequest) -> SpawnSpec {
                let mut spec = self.0.spawn_spec(r);
                spec.argv = vec!["/nonexistent/verifier".into()];
                spec
            }
            fn settle(
                &self,
                o: &crate::process::SupervisedOutcome,
                r: &BenchmarkRunRequest,
            ) -> (Fact, BenchmarkCompletion) {
                self.0.settle(o, r)
            }
        }
        let settled = BenchmarkExecution::run(
            &request,
            &MissingHelper(HelperAdapter),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(
            settled.completion,
            BenchmarkCompletion::Failed(failure_kinds::spawn(SpawnReason::NotFound))
        );
        assert_eq!(
            settled.failure_boundary(),
            Some(FailureBoundaryCode::Infrastructure)
        );
    }
}

#[test]
fn harbor_result_import_reads_the_completion_aggregate() {
    let dir = tempfile::tempdir().unwrap();
    let jobs = dir.path().join("jobs").join("2026-08-28__12-33-17");
    std::fs::create_dir_all(&jobs).unwrap();
    std::fs::write(
        jobs.join("result.json"),
        br#"{"id": "0b0a", "started_at": "2026-08-28T12:33:17Z",
                "finished_at": "2026-08-28T12:33:50Z", "n_total_trials": 1,
                "stats": {"evals": {"adhoc/terminal-bench-2-1/oracle": {
                    "reward_stats": {"reward": {"1.0": ["openssl-selfsigned-cert"]}}}}}}"#,
    )
    .unwrap();
    let (metrics, reward, path, _value) = import_harbor_result(dir.path()).unwrap();
    assert!(path.ends_with("result.json"));
    assert_eq!(
        metrics.passed,
        Some(Fact::Known {
            value: 1,
            origin: "harbor-reward".to_owned()
        })
    );
    assert_eq!(
        reward,
        Fact::Known {
            value: 1,
            origin: "harbor-result".to_owned()
        }
    );
    assert_eq!(metrics.tests, None);
}

#[test]
fn harbor_result_import_fails_closed_on_trial_count_drift() {
    let dir = tempfile::tempdir().unwrap();
    let jobs = dir.path().join("jobs").join("2026-08-28__12-33-17");
    std::fs::create_dir_all(&jobs).unwrap();
    std::fs::write(
        jobs.join("result.json"),
        br#"{"n_total_trials": 2, "stats": {"evals": {}}}"#,
    )
    .unwrap();
    assert!(matches!(
        import_harbor_result(dir.path()),
        Err(HarborResultError::Invalid("trial-count"))
    ));
}

/// Writes one Pier `jobs/<timestamp>/result.json` aggregate whose `reward`
/// metric awards `reward_key` to the single trial.
#[cfg(test)]
fn pier_job(dir: &std::path::Path, reward_key: &str) {
    let jobs = dir.join("jobs").join("2026-08-29__07-25-13");
    std::fs::create_dir_all(&jobs).unwrap();
    std::fs::write(
        jobs.join("result.json"),
        format!(
            r#"{{"n_total_trials": 1, "stats": {{"evals": {{"x": {{"reward_stats": {{"reward": {{"{reward_key}": ["t"]}}}}}}}}}}}}"#
        ),
    )
    .unwrap();
}

#[test]
fn pier_job_import_enforces_the_native_reward_domain() {
    // Out-of-domain rewards are drift, never clamped or cast: negative,
    // above-one, and fractional values fail the import before any `u64`
    // conversion could silently wrap them.
    for bad in ["-1.0", "2.0", "1.5", "NaN"] {
        let dir = tempfile::tempdir().unwrap();
        pier_job(dir.path(), bad);
        assert_eq!(
            import_pier_job_result(dir.path()),
            Err(HarborResultError::Invalid("bad-reward-values")),
            "{bad}"
        );
    }
    // Zero and one are valid measured native rewards: zero may grade a
    // trial, it just cannot admit an oracle preflight.
    for good in ["0.0", "1.0"] {
        let dir = tempfile::tempdir().unwrap();
        pier_job(dir.path(), good);
        let (_path, reward, _value) = import_pier_job_result(dir.path()).unwrap();
        assert_eq!(
            reward,
            Fact::Known {
                value: good.trim_end_matches(".0").parse::<u64>().unwrap(),
                origin: "pier-result".to_owned()
            },
            "{good}"
        );
    }
    // An aggregate with no `reward` metric at all stays a typed failure.
    let dir = tempfile::tempdir().unwrap();
    let jobs = dir.path().join("jobs").join("2026-08-29__07-25-13");
    std::fs::create_dir_all(&jobs).unwrap();
    std::fs::write(
        jobs.join("result.json"),
        br#"{"n_total_trials": 1, "stats": {"evals": {"x": {"reward_stats": {"f2p": {"1.0": ["t"]}}}}}}"#,
    )
    .unwrap();
    assert_eq!(
        import_pier_job_result(dir.path()),
        Err(HarborResultError::Invalid("no-reward-key"))
    );
}

#[test]
fn pier_job_import_accepts_native_multi_metric_breakdowns() {
    let dir = tempfile::tempdir().unwrap();
    let jobs = dir.path().join("jobs").join("2026-08-29__07-25-13");
    std::fs::create_dir_all(&jobs).unwrap();
    std::fs::write(
        jobs.join("result.json"),
        br#"{"n_total_trials": 1, "stats": {"evals": {"x": {"reward_stats": {
            "F2P": {"20.0": ["t"]}, "P2P": {"3.0": ["t"]},
            "partial": {"1.0": ["t"]}, "reward": {"1.0": ["t"]}
        }}}}}"#,
    )
    .unwrap();

    let (_path, reward, _value) = import_pier_job_result(dir.path()).unwrap();
    assert_eq!(
        reward,
        Fact::Known {
            value: 1,
            origin: "pier-result".to_owned()
        }
    );
}
