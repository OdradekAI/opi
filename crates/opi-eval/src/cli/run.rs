//! `opi-eval run` command (tasks 18.12, 18.14.1): the assembled run path.
//!
//! One invocation resolves the experiment document, drives the paired
//! trials end to end through the crate-private runner (durable intent,
//! agent dispatch, settlement, verifier dispatch, pre-seal projection,
//! sealing, receipts, and comparison coverage), and prints a single-line
//! JSON run report. Exit codes mirror the conformance contract: 0 when the
//! run completed with every pair comparable, 1 for a non-success or
//! explicitly incomplete outcome, 2 for a rejected request. The resolved
//! executables are runtime-generated deterministic helpers over the pinned
//! fixtures tree: this facade never claims a real Opi/pi program, provider
//! call, or official task environment (task 18.15 owns the native rerun),
//! and never touches paid providers or user-global resources.

use std::path::PathBuf;

use thiserror::Error;

use crate::runner::experiment::{self, RunRequest};

/// Failures reported by [`run`].
#[derive(Debug, Error)]
pub enum RunCliError {
    /// The assembled runner rejected the request before any trial ran.
    #[error("{0}")]
    Rejected(String),
}

/// Arguments of the `run` subcommand.
#[derive(Debug, Clone)]
pub struct RunArgs {
    /// Path to the experiment document.
    pub config: PathBuf,
    /// Fresh run root for trial directories, helpers, and receipts.
    pub root: PathBuf,
    /// Repository `crates/opi-eval/tests/fixtures` root.
    pub fixtures: PathBuf,
    /// Hermetic staging behavior (helper-process selection).
    pub behavior: String,
    /// Recovery mode: classify durable trial states instead of running.
    pub recover: bool,
    /// Re-run one crashed trial's whole group under fresh identities.
    pub replacement_for: Option<String>,
    /// Optional file of declared canary secrets (one per line); any
    /// canary found in staged exportable content blocks sealing.
    pub canaries: Option<PathBuf>,
    /// Resolved native material manifest (task 18.14.1); when present
    /// the run takes the native driving mode.
    pub native_material: Option<PathBuf>,
    /// Run only the upstream oracle preflight, then stop.
    pub preflight_only: bool,
}

/// Run one assembled experiment and return its JSON report.
///
/// This is the production entry seam of the `opi-eval run` command; the
/// binary prints the returned report on success and fails with a non-zero
/// exit and a stderr diagnostic otherwise.
pub fn run(args: &RunArgs) -> Result<serde_json::Value, RunCliError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| RunCliError::Rejected(error.to_string()))?;
    let report = runtime
        .block_on(experiment::run_experiment(&RunRequest {
            config_path: args.config.clone(),
            root: args.root.clone(),
            fixtures: args.fixtures.clone(),
            behavior: args.behavior.clone(),
            recover: args.recover,
            replacement_for: args.replacement_for.clone(),
            canaries: read_canaries(args.canaries.as_deref())
                .map_err(|error| RunCliError::Rejected(error.to_string()))?,
            material: match &args.native_material {
                Some(path) => Some(
                    crate::runner::material::NativeMaterial::load(path)
                        .map_err(|error| RunCliError::Rejected(error.to_string()))?,
                ),
                None => None,
            },
            preflight_only: args.preflight_only,
        }))
        .map_err(|error| RunCliError::Rejected(error.to_string()))?;
    Ok(report)
}

/// Reads the declared canary file (one secret per line, blank lines
/// ignored). An unreadable declaration is rejected, never silently empty.
fn read_canaries(path: Option<&std::path::Path>) -> Result<Vec<String>, std::io::Error> {
    let Some(path) = path else {
        return Ok(Vec::new());
    };
    let text = std::fs::read_to_string(path)?;
    let canaries: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    if canaries.is_empty() {
        return Err(std::io::Error::other(format!(
            "canary declaration {} holds no secrets",
            path.display()
        )));
    }
    Ok(canaries)
}

/// Whether a run report warrants a non-success process exit.
pub fn report_exit_code(report: &serde_json::Value) -> i32 {
    match report["outcome"].as_str() {
        // A passing oracle preflight without trials is a successful
        // native preflight-only invocation (task 18.14.1).
        Some("completed") | Some("preflight-only") => 0,
        _ => 1,
    }
}
