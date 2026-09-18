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
    /// Explicit local Git repository for a selected cold Opi build.
    pub opi_source: Option<PathBuf>,
    /// Exact local commit paired with `opi_source`; never inferred or fetched.
    pub opi_commit: Option<String>,
}

/// Verify the declared local cache and freeze a local-origin
/// native-material manifest.
///
/// This is the production entry seam of `opi-eval prepare`; the binary
/// prints the returned preparation report on success and fails with a
/// non-zero exit and a stderr diagnostic otherwise. No model runs and
/// no credential is read.
pub fn prepare(args: &PrepareArgs) -> Result<serde_json::Value, PrepareCliError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| PrepareCliError::Rejected(error.to_string()))?;
    runtime.block_on(async {
        let cancel = tokio_util::sync::CancellationToken::new();
        super::run::install_signal_cancellation(&cancel);
        crate::runner::prepare::prepare(
            &args.config,
            &args.out,
            args.opi_source.as_deref(),
            args.opi_commit.as_deref(),
            &cancel,
        )
        .await
        .map_err(|error| PrepareCliError::Rejected(error.to_string()))
    })
}
