//! `opi-eval validate` command (provisional Phase 18 seam).
//! `opi-eval conformance` command (task 18.10.1): see [`conformance`].
//! `opi-eval regrade` and `opi-eval report` commands (task 18.13): see
//! [`regrade`] and [`report`].

pub mod conformance;
pub mod regrade;
pub mod report;
pub mod run;

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

/// Failures reported by [`validate_native`].
#[derive(Debug, Error)]
pub enum NativeValidateError {
    /// The experiment document or manifest could not be read.
    #[error(transparent)]
    Validate(#[from] ValidateError),
    /// The resolved native material was rejected.
    #[error("native material rejected: {0}")]
    Material(String),
}

/// Human-readable summary of one native experiment contract: everything
/// [`ValidationSummary`] carries plus the derived native integrity
/// identity the experiment document must pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeValidationSummary {
    /// The hermetic-resolution fields of the same document.
    pub base: ValidationSummary,
    /// The derived native integrity digest (`phase18-native-material/1`).
    pub integrity_digest: String,
}

impl fmt::Display for NativeValidationSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} native_integrity={}",
            self.base, self.integrity_digest
        )
    }
}

/// Validates one experiment document against one resolved native
/// material manifest and derives the native integrity identity (task
/// 18.14.1). The producer uses this to pin `benchmark.integrity_digest`
/// before materializing dispatch configs; no process runs.
pub fn validate_native(
    config_path: &Path,
    material_path: &Path,
) -> Result<NativeValidationSummary, NativeValidateError> {
    let base = validate(config_path)?;
    let digest = crate::runner::experiment::native_integrity_identity(config_path, material_path)
        .map_err(NativeValidateError::Material)?;
    Ok(NativeValidationSummary {
        base,
        integrity_digest: digest,
    })
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
