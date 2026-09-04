//! Crate-private failure-boundary codes.
//!
//! Failures are owned at the narrowest boundary that observed them. This enum
//! is the stable owning-boundary code every typed eval failure carries; it
//! stays crate-private until the opi-eval integration matrix fixes the seam.

/// Stable owning-boundary code for every typed eval failure.
///
/// One variant per boundary row of the opi-eval failure table. A variant
/// names who owns classification and downstream authority stopping, not how
/// severe the failure is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum FailureBoundaryCode {
    /// Invalid experiment schema, unresolved identity, control mismatch,
    /// budget rejection, unsupported capability.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "required closed opi-eval failure-table boundary")
    )]
    Experiment,
    /// Not started, effect unknown, settlement failure, sealing failure,
    /// post-seal mutation.
    TrialDurability,
    /// Configuration/auth/provider failure, Agent crash, invalid native
    /// output, timeout, cancellation, process cleanup unknown.
    AgentProcess,
    /// Unsupported required schema, parse failure, missing terminal
    /// predicate, bounded-output violation.
    Adapter,
    /// Missing/incomplete native evidence, redaction failure, artifact
    /// validation failure, unknown measurement.
    Evidence,
    /// Revision not admitted, invalid/ambiguous/broken task, prompt/test
    /// mismatch, exclusion conflict.
    Integrity,
    /// Grader resolution failure, non-zero exit, invalid output, timeout,
    /// cancellation, provenance mismatch.
    Grader,
    /// Container/image/tool acquisition, host resource failure, shared
    /// provider outage, orchestration failure outside the Agent.
    Infrastructure,
    /// Missing/duplicate pair, control mismatch, incomplete coverage,
    /// offline recomputation mismatch.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "required closed opi-eval failure-table boundary")
    )]
    PairReport,
}

#[cfg(test)]
mod tests {
    use super::FailureBoundaryCode;

    #[test]
    fn failure_boundary_codes_are_pairwise_distinct() {
        let codes = [
            FailureBoundaryCode::Experiment,
            FailureBoundaryCode::TrialDurability,
            FailureBoundaryCode::AgentProcess,
            FailureBoundaryCode::Adapter,
            FailureBoundaryCode::Evidence,
            FailureBoundaryCode::Integrity,
            FailureBoundaryCode::Grader,
            FailureBoundaryCode::Infrastructure,
            FailureBoundaryCode::PairReport,
        ];
        for (i, a) in codes.iter().enumerate() {
            for b in &codes[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}
