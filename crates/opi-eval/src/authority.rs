//! Crate-private authority-transition ledger (Phase 18 task 18.12,
//! `P18-FAL-002`).
//!
//! The assembled runner performs five authority transitions per trial:
//! dispatching the Agent process, recording settlement, dispatching the
//! native verifier, sealing the bundle, and emitting the trial receipt.
//! A boundary failure must stop the later transitions mechanically - not by
//! convention - and must never be converted into success, a zero, a native
//! grader pass, or a silent exclusion. This ledger owns that mechanism:
//! every transition is either `executed` or carries the exact refusal
//! reason, and the serialized record enters the persisted trial receipt so
//! tests and audits can count downstream executions after a failure.

use crate::failure::FailureBoundaryCode;
use serde::Serialize;

/// One authority transition of the assembled run path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuthorityTransition {
    /// Spawn the Agent process.
    AgentDispatch,
    /// Durably record the observed Agent outcome.
    Settle,
    /// Spawn the native verifier.
    GradeDispatch,
    /// Seal the trial bundle.
    Seal,
    /// Emit the trial receipt.
    Report,
}

impl AuthorityTransition {
    /// Stable receipt token for this transition.
    pub(crate) fn token(&self) -> &'static str {
        match self {
            AuthorityTransition::AgentDispatch => "agent_dispatch",
            AuthorityTransition::Settle => "settle",
            AuthorityTransition::GradeDispatch => "grade_dispatch",
            AuthorityTransition::Seal => "seal",
            AuthorityTransition::Report => "report",
        }
    }
}

/// Receipt token for a failure boundary. Kept here, next to the transition
/// tokens that consume it, so the receipt vocabulary lives in one place.
pub(crate) fn boundary_token(boundary: FailureBoundaryCode) -> &'static str {
    match boundary {
        FailureBoundaryCode::Experiment => "experiment",
        FailureBoundaryCode::TrialDurability => "trial-durability",
        FailureBoundaryCode::AgentProcess => "agent-process",
        FailureBoundaryCode::Adapter => "adapter",
        FailureBoundaryCode::Evidence => "evidence",
        FailureBoundaryCode::Integrity => "integrity",
        FailureBoundaryCode::Grader => "grader",
        FailureBoundaryCode::Infrastructure => "infrastructure",
        FailureBoundaryCode::PairReport => "pair-report",
    }
}

/// Serialized execution record of one transition attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct TransitionRecord {
    pub(crate) transition: AuthorityTransition,
    /// `executed` or `refused:<reason>`.
    pub(crate) state: String,
}

/// The mechanical authority-stop ledger for one trial.
///
/// Gating rules pinned by the task DoD: a boundary failure observed by the
/// Agent process or evidence validation refuses `grade_dispatch`; a
/// settlement-recording failure refuses `seal`; a sealing or grader failure
/// refuses `report`. Every refusal is recorded and every later transition
/// consults the same ledger, so downstream execution counts are provable
/// from the receipt alone.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub(crate) struct AuthorityLedger {
    records: Vec<TransitionRecord>,
    /// First boundary failure observed, if any.
    failed_boundary: Option<&'static str>,
}

impl AuthorityLedger {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Record a boundary failure. The first failure wins; a failure is
    /// never cleared and never converted into success.
    pub(crate) fn fail(&mut self, boundary: FailureBoundaryCode) {
        if self.failed_boundary.is_none() {
            self.failed_boundary = Some(boundary_token(boundary));
        }
    }

    /// Ask whether `transition` may execute under the recorded failures;
    /// the attempt is recorded either way.
    pub(crate) fn attempt(&mut self, transition: AuthorityTransition) -> bool {
        let refused = match transition {
            AuthorityTransition::AgentDispatch | AuthorityTransition::Settle => None,
            AuthorityTransition::GradeDispatch => self
                .failed_boundary
                .filter(|token| {
                    matches!(
                        *token,
                        "agent-process" | "adapter" | "evidence" | "infrastructure"
                    )
                })
                .map(|token| format!("refused:stopped-at-{token}")),
            AuthorityTransition::Seal => self
                .failed_boundary
                .filter(|token| *token == "trial-durability")
                .map(|token| format!("refused:stopped-at-{token}")),
            AuthorityTransition::Report => {
                let seal_failed = self.records.iter().any(|record| {
                    record.transition == AuthorityTransition::Seal
                        && record.state.starts_with("failed")
                });
                if seal_failed {
                    Some("refused:seal-failed".to_owned())
                } else {
                    self.failed_boundary
                        .filter(|token| matches!(*token, "trial-durability" | "grader"))
                        .map(|token| format!("refused:stopped-at-{token}"))
                }
            }
        };

        match refused {
            Some(reason) => {
                self.records.push(TransitionRecord {
                    transition,
                    state: reason,
                });
                false
            }
            None => {
                self.records.push(TransitionRecord {
                    transition,
                    state: "executed".to_owned(),
                });
                true
            }
        }
    }

    /// Record that `transition` was attempted and itself failed at
    /// `boundary`. A failed transition executes zero downstream work and,
    /// for the seal transition, refuses the report mechanically.
    pub(crate) fn attempt_failed(
        &mut self,
        transition: AuthorityTransition,
        boundary: FailureBoundaryCode,
    ) {
        self.fail(boundary);
        self.records.push(TransitionRecord {
            transition,
            state: format!("failed:at-{}", boundary_token(boundary)),
        });
    }

    /// All recorded transition states, in execution order.
    pub(crate) fn records(&self) -> &[TransitionRecord] {
        &self.records
    }

    /// First recorded failure boundary token, if any.
    pub(crate) fn failed_boundary(&self) -> Option<&'static str> {
        self.failed_boundary
    }

    /// How often `transition` executed.
    pub(crate) fn executed_count(&self, transition: AuthorityTransition) -> usize {
        self.records
            .iter()
            .filter(|record| record.transition == transition && record.state == "executed")
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_boundary_failure_mechanically_stops_later_transitions() {
        let mut ledger = AuthorityLedger::new();
        assert!(ledger.attempt(AuthorityTransition::AgentDispatch));
        // The Agent process failed: grade dispatch is refused, recorded,
        // and repeatable.
        ledger.fail(FailureBoundaryCode::AgentProcess);
        assert!(!ledger.attempt(AuthorityTransition::GradeDispatch));
        assert!(!ledger.attempt(AuthorityTransition::GradeDispatch));
        // Refusals stay visible with the owning boundary token.
        assert_eq!(ledger.executed_count(AuthorityTransition::GradeDispatch), 0);
        assert_eq!(ledger.failed_boundary(), Some("agent-process"));

        // A fresh ledger: a grader failure refuses only the report.
        let mut grader = AuthorityLedger::new();
        grader.fail(FailureBoundaryCode::Grader);
        assert!(grader.attempt(AuthorityTransition::Settle));
        assert!(grader.attempt(AuthorityTransition::Seal));
        assert!(!grader.attempt(AuthorityTransition::Report));

        // A durability failure refuses seal and report.
        let mut durability = AuthorityLedger::new();
        durability.fail(FailureBoundaryCode::TrialDurability);
        assert!(!durability.attempt(AuthorityTransition::Seal));
        assert!(!durability.attempt(AuthorityTransition::Report));

        // A failed seal transition refuses the report mechanically.
        let mut seal_failed = AuthorityLedger::new();
        seal_failed.attempt_failed(AuthorityTransition::Seal, FailureBoundaryCode::Evidence);
        assert!(!seal_failed.attempt(AuthorityTransition::Report));
        assert_eq!(
            seal_failed
                .records()
                .iter()
                .find(|record| record.transition == AuthorityTransition::Report)
                .map(|record| record.state.as_str()),
            Some("refused:seal-failed")
        );

        // The first failure wins and is never cleared.
        let mut first = AuthorityLedger::new();
        first.fail(FailureBoundaryCode::Evidence);
        first.fail(FailureBoundaryCode::Grader);
        assert_eq!(first.failed_boundary(), Some("evidence"));
    }
}
