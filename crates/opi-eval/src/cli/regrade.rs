//! `opi-eval regrade` command: offline bundle re-verification.
//!
//! One invocation re-verifies every sealed trial bundle under a run root
//! without starting an Agent, calling a provider, or mutating anything: no
//! repair, no rehash, no rewrite. The command prints a single-line JSON
//! regrade report. Exit codes: 0 when every sealed bundle verified, 1 when
//! any bundle failed verification (typed failure, bytes untouched), 2 for a
//! rejected request. This is the hermetic fixture-grade offline path; native
//! execution is verified separately by the native-smoke workflow.

use std::path::PathBuf;

use thiserror::Error;

use crate::regrade::OfflineRegrader;

/// Failures reported by [`regrade`].
#[derive(Debug, Error)]
pub enum RegradeCliError {
    /// The run root could not be read.
    #[error("cannot read run root: {0}")]
    Io(#[from] std::io::Error),
}

/// Arguments of the `regrade` subcommand.
#[derive(Debug, Clone)]
pub struct RegradeArgs {
    /// Run root holding `trials/<id>/bundle` sealed bundles.
    pub root: PathBuf,
}

/// Regrade one run root and return its JSON report.
///
/// This is the production entry seam of the `opi-eval regrade` command; the
/// binary prints the returned report and derives its exit code from the
/// outcome.
pub fn regrade(args: &RegradeArgs) -> Result<serde_json::Value, RegradeCliError> {
    let report = OfflineRegrader::regrade(&args.root)?;
    Ok(report.to_json())
}

/// Whether a regrade report warrants a non-success process exit.
pub fn regrade_exit_code(report: &serde_json::Value) -> i32 {
    if report["outcome"] == "verified" {
        0
    } else {
        1
    }
}
