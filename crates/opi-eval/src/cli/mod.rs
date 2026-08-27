//! `opi-eval validate` command (provisional Phase 18 seam).
//! `opi-eval conformance` command (task 18.10.1): see [`conformance`].

pub mod conformance;

use std::fmt;
use std::path::Path;

use thiserror::Error;

use crate::experiment::{ResolveError, ResolvedExperiment};

/// Failures reported by [`validate`].
#[derive(Debug, Error)]
pub enum ValidateError {
    /// The experiment document could not be read.
    #[error("cannot read experiment document: {0}")]
    Io(#[from] std::io::Error),
    /// The document did not resolve into a frozen contract.
    #[error("experiment document rejected: {0}")]
    Resolve(#[from] ResolveError),
}

/// Human-readable summary of one validated experiment contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationSummary {
    /// Experiment id.
    pub experiment_id: String,
    /// Schema identity.
    pub schema: String,
    /// Canonical manifest digest.
    pub manifest_digest: String,
    /// Number of identified subjects.
    pub subject_count: usize,
    /// Number of directed comparison edges.
    pub edge_count: usize,
    /// Number of declared trials.
    pub trial_count: usize,
}

impl fmt::Display for ValidationSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "experiment {} resolved: schema={} digest={} subjects={} edges={} trials={}",
            self.experiment_id,
            self.schema,
            self.manifest_digest,
            self.subject_count,
            self.edge_count,
            self.trial_count
        )
    }
}

/// Validate an experiment document at `config_path` and return its summary.
///
/// This is the production entry seam of the `opi-eval validate` command; the
/// binary prints the returned summary on success and fails with a non-zero
/// exit and a stderr diagnostic otherwise.
pub fn validate(config_path: &Path) -> Result<ValidationSummary, ValidateError> {
    let source = std::fs::read_to_string(config_path)?;
    let resolved = ResolvedExperiment::resolve(&source)?;
    Ok(ValidationSummary {
        experiment_id: resolved.experiment_id().to_owned(),
        schema: resolved.schema().to_owned(),
        manifest_digest: resolved.manifest_digest().to_owned(),
        subject_count: resolved.subjects().len(),
        edge_count: resolved.edges().len(),
        trial_count: resolved.trials().len(),
    })
}
