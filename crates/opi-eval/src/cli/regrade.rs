//! `opi-eval regrade` command: actual native regrading (planned).
//!
//! Native regrading re-executes preserved real Agent artifacts with the
//! pinned native grader in a fresh environment. That workflow is not
//! implemented yet, so the command fails closed with an explicit
//! unavailable-operation diagnostic instead of quietly re-running byte
//! verification under the regrade name (a byte check is `opi-eval
//! verify`). Exit code is always 2 until the native regrade workflow
//! lands; nothing is read, verified, or mutated here.

use std::path::PathBuf;

use thiserror::Error;

/// Failures reported by [`regrade`].
#[derive(Debug, Error)]
pub enum RegradeCliError {
    /// Native regrading is not implemented yet. The message names the
    /// missing operation and the interim byte-verification command.
    #[error(
        "unavailable-operation: native regrade is not implemented yet; use `opi-eval verify --root <run-root>` for the original byte/seal checks"
    )]
    Unavailable,
}

/// Arguments of the `regrade` subcommand.
#[derive(Debug, Clone)]
pub struct RegradeArgs {
    /// Run root holding `trials/<id>/bundle` sealed bundles.
    pub root: PathBuf,
}

/// Reject one regrade request.
///
/// This is the production entry seam of the `opi-eval regrade` command
/// until the native regrade workflow exists.
pub fn regrade(_args: &RegradeArgs) -> Result<serde_json::Value, RegradeCliError> {
    Err(RegradeCliError::Unavailable)
}

/// Kept for the dispatch shape: a regrade report never exists today, so
/// any value would already be a contract break and exits non-zero.
pub fn regrade_exit_code(_report: &serde_json::Value) -> i32 {
    1
}
