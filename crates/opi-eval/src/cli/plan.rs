//! `opi-eval plan` command: corpus-aware selection freeze (P19-D05).
//!
//! Prints one preview JSON object (subjects, connection, task list with
//! per-task requirement status, selection, trial count, concurrency,
//! bounds, and the unmet/unknown preflight summary) and freezes the
//! exact expansion into the digest-bound run plan at `--out`. Exit 0
/// freezes the plan; exit 2 rejects the request naming the
/// contradiction. No model runs and no image is pulled or started.
use std::path::PathBuf;

use thiserror::Error;

/// Failures reported by [`plan`].
#[derive(Debug, Error)]
pub enum PlanCliError {
    /// The request contradicted the pinned corpus, selection, bounds,
    /// material, or observed resources.
    #[error("{0}")]
    Rejected(String),
}

/// Arguments of the `plan` subcommand.
#[derive(Debug, Clone)]
pub struct PlanArgs {
    /// Path to the experiment document.
    pub config: PathBuf,
    /// Resolved native material manifest (native mode).
    pub native_material: Option<PathBuf>,
    /// Repository `crates/opi-eval/tests/fixtures` root (hermetic mode).
    pub fixtures: Option<PathBuf>,
    /// Output path of the frozen run plan.
    pub out: PathBuf,
}

/// Freeze one experiment's selection into a run plan and return its
/// preview (the production entry seam of `opi-eval plan`; the binary
/// prints the preview on success).
pub fn plan(args: &PlanArgs) -> Result<serde_json::Value, PlanCliError> {
    crate::runner::plan::build_plan(
        &args.config,
        args.native_material.as_deref(),
        args.fixtures.as_deref(),
        &args.out,
    )
    .map_err(|error| PlanCliError::Rejected(error.to_string()))
}
