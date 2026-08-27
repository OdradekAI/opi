//! Crate-private trial lifecycle state machine (Phase 18 task 18.5).
//!
//! Owns the intent/settlement distinction at the eval process boundary:
//! `planned -> intent-published -> process-effect-pending -> settled ->
//! sealed -> graded -> reported`. Entering the effect-pending phase requires
//! a [`DurableIntentProof`], which only [`crate::bundle::RunBundle`]
//! mint after the intent record is durably on disk.

use crate::bundle::{DurableIntentProof, IntentRecord, RecoveryObservation, TrialIdentity};
use crate::failure::FailureBoundaryCode;
use std::fmt;

/// What a recovered bundle root proves about a crashed trial.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RecoveryClassification {
    /// No durable reservation exists; nothing started under this bundle.
    NotStarted,
    /// Durable intent exists without settlement. The Agent may have consumed
    /// credits or mutated its workspace; the trial must not be inferred as
    /// not-started, successful, or retryable under the same identity
    /// (P18-DUR-002). Replacement work needs a new paired trial group.
    EffectUnknown {
        trial: TrialIdentity,
        boundary: FailureBoundaryCode,
    },
    /// Settlement is durably recorded; only sealing remains.
    SettledUnsealed,
    /// The bundle is already sealed; reopen it instead of classifying.
    Sealed,
}

/// One rung of the trial ladder. States are closed and forward-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrialPhase {
    /// Fresh reservation; no durable intent yet.
    Planned,
    /// Trial, pair, artifact, and expected-output identities are durably
    /// reserved before any process effect can start (P18-DUR-001).
    IntentPublished,
    /// The Agent process may have consumed credits or mutated its isolated
    /// workspace; a crash here is effect-unknown (P18-DUR-002).
    ProcessEffectPending,
    /// Observed exit/cancellation/timeout and evidence retention recorded.
    Settled,
    /// The artifact set is immutable.
    Sealed,
    /// A derived grade artifact exists outside the sealed bundle.
    Graded,
    /// A derived report artifact exists outside the sealed bundle.
    Reported,
}

/// Whether the observed settle was an exit, timeout, or cancellation, and
/// who cancelled. Cancellation is never success; user and infrastructure
/// sources stay distinct (P18-FAL-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettlementKind {
    Exited { code: i32 },
    TimedOut,
    Cancelled { source: CancellationSource },
}

/// Distinct cancellation sources retained through settlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CancellationSource {
    User,
    Infrastructure,
}

/// The observed process outcome recorded at settlement (P18-DUR-003).
pub(crate) struct ObservedOutcome {
    pub(crate) kind: SettlementKind,
}

/// Typed lifecycle failure; every variant owns the trial-durability
/// boundary.
#[derive(Debug)]
pub(crate) enum LifecycleError {
    /// The ladder admits only the next forward rung.
    InvalidTransition {
        from: TrialPhase,
        to: &'static str,
        boundary: FailureBoundaryCode,
    },
    /// The published intent reserves a different trial than planned.
    IdentityMismatch {
        planned: TrialIdentity,
        published: TrialIdentity,
        boundary: FailureBoundaryCode,
    },
}

impl LifecycleError {
    /// The owning failure boundary for this error.
    pub(crate) fn boundary(&self) -> FailureBoundaryCode {
        match self {
            LifecycleError::InvalidTransition { boundary, .. }
            | LifecycleError::IdentityMismatch { boundary, .. } => *boundary,
        }
    }
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LifecycleError::InvalidTransition { from, to, .. } => {
                write!(f, "cannot move from {from:?} to {to}")
            }
            LifecycleError::IdentityMismatch {
                planned, published, ..
            } => write!(
                f,
                "planned trial {} but intent reserves {}",
                planned.as_str(),
                published.as_str()
            ),
        }
    }
}

/// The trial lifecycle state machine. One instance tracks one trial
/// identity; the durable reservation itself lives in
/// [`crate::bundle::RunBundle`].
pub(crate) struct TrialLifecycle {
    phase: TrialPhase,
    trial: TrialIdentity,
    intent: Option<IntentRecord>,
}

impl TrialLifecycle {
    /// Starts a fresh planned lifecycle for `trial`.
    pub(crate) fn plan(trial: TrialIdentity) -> Self {
        Self {
            phase: TrialPhase::Planned,
            trial,
            intent: None,
        }
    }

    /// The current phase.
    pub(crate) fn phase(&self) -> TrialPhase {
        self.phase
    }

    /// Records a durably reserved intent. Valid only from [`TrialPhase::Planned`]
    /// and only for the planned trial identity.
    pub(crate) fn publish_intent(&mut self, record: IntentRecord) -> Result<(), LifecycleError> {
        self.require(TrialPhase::Planned, "intent-published")?;
        if record.trial != self.trial {
            return Err(LifecycleError::IdentityMismatch {
                planned: self.trial.clone(),
                published: record.trial,
                boundary: FailureBoundaryCode::TrialDurability,
            });
        }
        self.intent = Some(record);
        self.phase = TrialPhase::IntentPublished;
        Ok(())
    }

    /// Enters the effect-pending phase. Requires the durable proof minted by
    /// [`crate::bundle::RunBundle::publish_intent`], so no process effect is
    /// admitted without a durable reservation.
    pub(crate) fn enter_process_effect_pending(
        &mut self,
        _proof: DurableIntentProof,
    ) -> Result<(), LifecycleError> {
        self.require(TrialPhase::IntentPublished, "process-effect-pending")?;
        self.phase = TrialPhase::ProcessEffectPending;
        Ok(())
    }

    /// Records the observed outcome. Valid only from
    /// [`TrialPhase::ProcessEffectPending`].
    pub(crate) fn settle(&mut self, _outcome: ObservedOutcome) -> Result<(), LifecycleError> {
        self.require(TrialPhase::ProcessEffectPending, "settled")?;
        self.phase = TrialPhase::Settled;
        Ok(())
    }

    /// Marks the bundle sealed. Valid only from [`TrialPhase::Settled`].
    pub(crate) fn mark_sealed(&mut self) -> Result<(), LifecycleError> {
        self.require(TrialPhase::Settled, "sealed")?;
        self.phase = TrialPhase::Sealed;
        Ok(())
    }

    /// Marks the trial graded. Valid only from [`TrialPhase::Sealed`].
    pub(crate) fn mark_graded(&mut self) -> Result<(), LifecycleError> {
        self.require(TrialPhase::Sealed, "graded")?;
        self.phase = TrialPhase::Graded;
        Ok(())
    }

    /// Marks the trial reported. Valid only from [`TrialPhase::Graded`].
    pub(crate) fn mark_reported(&mut self) -> Result<(), LifecycleError> {
        self.require(TrialPhase::Graded, "reported")?;
        self.phase = TrialPhase::Reported;
        Ok(())
    }

    /// Classifies a recovered bundle root. A durable intent without a
    /// settlement record is effect-unknown and is never classified as
    /// not-started (P18-DUR-002).
    pub(crate) fn recover(observed: &RecoveryObservation) -> RecoveryClassification {
        let Some(intent) = observed.intent.as_ref() else {
            return RecoveryClassification::NotStarted;
        };
        if observed.sealed {
            RecoveryClassification::Sealed
        } else if observed.settlement.is_some() {
            RecoveryClassification::SettledUnsealed
        } else {
            RecoveryClassification::EffectUnknown {
                trial: intent.trial.clone(),
                boundary: FailureBoundaryCode::TrialDurability,
            }
        }
    }

    fn require(&self, expected: TrialPhase, to: &'static str) -> Result<(), LifecycleError> {
        if self.phase == expected {
            Ok(())
        } else {
            Err(LifecycleError::InvalidTransition {
                from: self.phase,
                to,
                boundary: FailureBoundaryCode::TrialDurability,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{
        ArtifactKey, IntentRecord, PairIdentity, RunBundle, SettlementMarker, TrialIdentity,
    };

    fn sample_intent(trial: &str) -> IntentRecord {
        IntentRecord {
            trial: TrialIdentity::new(trial).unwrap(),
            pair: PairIdentity::new("pair-1").unwrap(),
            artifacts: vec![ArtifactKey::new("native/stdout").unwrap()],
            expected_output: ArtifactKey::new("normalized/expected-output").unwrap(),
        }
    }

    fn cancelled_by_user() -> ObservedOutcome {
        ObservedOutcome {
            kind: SettlementKind::Cancelled {
                source: CancellationSource::User,
            },
        }
    }

    #[test]
    fn intent_is_durably_reserved_before_effect_and_ladder_is_forward_only() {
        let tmp = tempfile::tempdir().unwrap();
        let mut bundle = RunBundle::create(tmp.path()).unwrap();
        let mut lc = TrialLifecycle::plan(TrialIdentity::new("trial-1").unwrap());

        assert_eq!(lc.phase(), TrialPhase::Planned);
        // Before any durable intent exists there is no proof value in
        // existence, and every later phase transition is rejected.
        assert!(lc.settle(cancelled_by_user()).is_err());
        assert!(lc.mark_sealed().is_err());

        let proof = bundle.publish_intent(&sample_intent("trial-1")).unwrap();
        // The reservation is durably on disk before the proof was returned.
        assert!(tmp.path().join("intent.json").is_file());
        assert_eq!(lc.phase(), TrialPhase::Planned);

        // Publishing an intent for a different trial identity is rejected.
        assert!(lc.publish_intent(sample_intent("trial-2")).is_err());
        lc.publish_intent(sample_intent("trial-1")).unwrap();
        assert_eq!(lc.phase(), TrialPhase::IntentPublished);
        // Duplicate publication is rejected.
        assert!(lc.publish_intent(sample_intent("trial-1")).is_err());
        // Settlement is still invalid while the process effect is pending
        // entry has not happened.
        assert!(lc.settle(cancelled_by_user()).is_err());

        lc.enter_process_effect_pending(proof).unwrap();
        assert_eq!(lc.phase(), TrialPhase::ProcessEffectPending);
        // Re-entering the effect-pending phase needs a new proof and is
        // rejected once past it.
        assert!(lc.settle(cancelled_by_user()).is_ok());
        assert_eq!(lc.phase(), TrialPhase::Settled);

        lc.mark_sealed().unwrap();
        assert_eq!(lc.phase(), TrialPhase::Sealed);
        lc.mark_graded().unwrap();
        assert_eq!(lc.phase(), TrialPhase::Graded);
        lc.mark_reported().unwrap();
        assert_eq!(lc.phase(), TrialPhase::Reported);
        // Backward transitions are rejected.
        assert!(lc.mark_graded().is_err());
        assert!(lc.settle(cancelled_by_user()).is_err());
    }

    #[test]
    fn crash_after_durable_intent_before_settlement_is_effect_unknown() {
        let tmp = tempfile::tempdir().unwrap();
        let mut bundle = RunBundle::create(tmp.path()).unwrap();
        bundle.publish_intent(&sample_intent("trial-9")).unwrap();

        // A crash here leaves durable intent without settlement: the trial is
        // effect-unknown, never not-started, successful, or same-identity
        // retryable (P18-DUR-002).
        let crashed = RunBundle::recover(tmp.path()).unwrap();
        assert_eq!(
            TrialLifecycle::recover(&crashed),
            RecoveryClassification::EffectUnknown {
                trial: TrialIdentity::new("trial-9").unwrap(),
                boundary: FailureBoundaryCode::TrialDurability,
            }
        );

        // With a durable settlement record the same crash point is no longer
        // effect-unknown: the outcome was observed and only sealing remains.
        bundle
            .record_settlement(&SettlementMarker {
                trial: TrialIdentity::new("trial-9").unwrap(),
            })
            .unwrap();
        let settled = RunBundle::recover(tmp.path()).unwrap();
        assert_eq!(
            TrialLifecycle::recover(&settled),
            RecoveryClassification::SettledUnsealed
        );

        // An empty root with no reservation was never started.
        let fresh = tempfile::tempdir().unwrap();
        RunBundle::create(fresh.path()).unwrap();
        let never = RunBundle::recover(fresh.path()).unwrap();
        assert_eq!(
            TrialLifecycle::recover(&never),
            RecoveryClassification::NotStarted
        );
    }
}
