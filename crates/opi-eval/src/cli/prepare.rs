//! `opi-eval doctor` and `opi-eval prepare` commands: local material
//! preparation (P19-D05).
//!
//! The facades resolve the experiment document, delegate to the
//! crate-private runner preparation core, and print one JSON report on
//! success. Failures exit non-zero with a stderr diagnostic and never
//! start a model or touch provider credentials.

use std::path::PathBuf;

use thiserror::Error;

/// Failures reported by the preparation commands.
#[derive(Debug, Error)]
pub enum PrepareCliError {
    /// The request was rejected before any observation or freeze.
    #[error("{0}")]
    Rejected(String),
}

/// Arguments of the `doctor` subcommand.
#[derive(Debug, Clone)]
pub struct DoctorArgs {
    /// Path to the experiment document.
    pub config: PathBuf,
}

/// Diagnose the local environment and declared materials.
///
/// This is the production entry seam of `opi-eval doctor`; the binary
/// prints the returned observation report on success and fails with a
/// non-zero exit and a stderr diagnostic otherwise.
pub fn doctor(args: &DoctorArgs) -> Result<serde_json::Value, PrepareCliError> {
    crate::runner::prepare::doctor(&args.config)
        .map_err(|error| PrepareCliError::Rejected(error.to_string()))
}

/// Arguments of the `prepare` subcommand.
#[derive(Debug, Clone)]
pub struct PrepareArgs {
    /// Path to the experiment document.
    pub config: PathBuf,
    /// Output directory receiving the local-origin material manifest
    /// and the preparation report.
    pub out: PathBuf,
}

/// Verify the declared local cache and freeze a local-origin
/// native-material manifest.
///
/// This is the production entry seam of `opi-eval prepare`; the binary
/// prints the returned preparation report on success and fails with a
/// non-zero exit and a stderr diagnostic otherwise. No model runs and
/// no credential is read.
pub fn prepare(args: &PrepareArgs) -> Result<serde_json::Value, PrepareCliError> {
    crate::runner::prepare::prepare(&args.config, &args.out)
        .map_err(|error| PrepareCliError::Rejected(error.to_string()))
}
