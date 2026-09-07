//! `opi-eval run` command: assembled run path.
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
//! call, or official task environment (native smoke owns that execution),
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
    /// Path to the experiment document (mutually exclusive with
    /// `plan`).
    pub config: Option<PathBuf>,
    /// Path to a frozen run plan (`run --plan P`): executes exactly the
    /// plan's trials after re-verifying its digest, material, and
    /// corpus expansion.
    pub plan: Option<PathBuf>,
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
    /// Resolved native material manifest (native mode); when present
    /// the run takes the native driving mode.
    pub native_material: Option<PathBuf>,
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
            config_path: match (&args.config, &args.plan) {
                (Some(config), _) => config.clone(),
                (None, Some(plan)) => plan.clone(),
                (None, None) => {
                    return Err(RunCliError::Rejected(
                        "run requires exactly one of --config or --plan".to_owned(),
                    ));
                }
            },
            root: args.root.clone(),
            fixtures: args.fixtures.clone(),
            behavior: args.behavior.clone(),
            recover: args.recover,
            replacement_for: args.replacement_for.clone(),
            canaries: read_canaries(args.canaries.as_deref())
                .map_err(|error| RunCliError::Rejected(error.to_string()))?,
            material: match (&args.native_material, &args.plan) {
                (Some(path), _) => {
                    let config = args
                        .config
                        .as_ref()
                        .expect("checked above: --native-material pairs with --config");
                    let products = selected_products(config)?;
                    Some(
                        crate::runner::material::NativeMaterial::load_for(path, &products)
                            .map_err(|error| RunCliError::Rejected(error.to_string()))?,
                    )
                }
                (None, Some(plan_path)) => {
                    let plan: serde_json::Value = serde_json::from_str(
                        &std::fs::read_to_string(plan_path)
                            .map_err(|error| RunCliError::Rejected(error.to_string()))?,
                    )
                    .map_err(|error| {
                        RunCliError::Rejected(format!("plan parse failed: {error}"))
                    })?;
                    crate::runner::plan::verify_plan(&plan)
                        .map_err(|error| RunCliError::Rejected(error.to_string()))?;
                    match plan["material"].as_object() {
                        Some(record) => {
                            let recorded_path = std::path::PathBuf::from(
                                record["path"]
                                    .as_str()
                                    .ok_or_else(|| {
                                        RunCliError::Rejected(
                                            "run plan material record has no path".to_owned(),
                                        )
                                    })?
                                    .to_owned(),
                            );
                            let path =
                                crate::runner::plan::resolve_plan_path(plan_path, &recorded_path);
                            let digest = {
                                use sha2::{Digest, Sha256};
                                let bytes = std::fs::read(&path)
                                    .map_err(|error| RunCliError::Rejected(error.to_string()))?;
                                let hash = Sha256::digest(&bytes);
                                hash.iter().map(|b| format!("{b:02x}")).collect::<String>()
                            };
                            if digest != record["manifest_sha256"].as_str().unwrap_or("") {
                                return Err(RunCliError::Rejected(format!(
                                    "run-plan material drift at {}",
                                    path.display()
                                )));
                            }
                            let source = crate::runner::plan::plan_source(&plan)
                                .map_err(|error| RunCliError::Rejected(error.to_string()))?;
                            let experiment =
                                crate::experiment::ResolvedExperiment::resolve(&source)
                                    .map_err(|error| RunCliError::Rejected(error.to_string()))?;
                            let products = selected_products_from(&experiment)
                                .map_err(RunCliError::Rejected)?;
                            Some(
                                crate::runner::material::NativeMaterial::load_for(&path, &products)
                                    .map_err(|error| RunCliError::Rejected(error.to_string()))?,
                            )
                        }
                        None => None,
                    }
                }
                (None, None) => None,
            },
            plan: args.plan.clone(),
        }))
        .map_err(|error| RunCliError::Rejected(error.to_string()))?;
    Ok(report)
}

/// The sorted unique agent products a resolved experiment selects; a
/// pi-only document prepares and runs without any Opi material.
pub(crate) fn selected_products_from(
    experiment: &crate::experiment::ResolvedExperiment,
) -> Result<Vec<&'static str>, String> {
    let mut products: Vec<&'static str> = experiment
        .subjects()
        .iter()
        .filter_map(|subject| match subject.product.as_str() {
            "opi" => Some("opi"),
            "pi" => Some("pi"),
            _ => None,
        })
        .collect();
    products.sort_unstable();
    products.dedup();
    if products.is_empty() {
        return Err("the experiment selects no supported agent product".to_owned());
    }
    Ok(products)
}

/// The sorted unique agent products the experiment document selects; a
/// pi-only document prepares and runs without any Opi material.
fn selected_products(config: &std::path::Path) -> Result<Vec<&'static str>, RunCliError> {
    let source = std::fs::read_to_string(config).map_err(|error| {
        RunCliError::Rejected(format!("cannot read experiment document: {error}"))
    })?;
    let experiment = crate::experiment::ResolvedExperiment::resolve(&source)
        .map_err(|error| RunCliError::Rejected(error.to_string()))?;
    let mut products: Vec<&'static str> = experiment
        .subjects()
        .iter()
        .filter_map(|subject| match subject.product.as_str() {
            "opi" => Some("opi"),
            "pi" => Some("pi"),
            _ => None,
        })
        .collect();
    products.sort_unstable();
    products.dedup();
    if products.is_empty() {
        return Err(RunCliError::Rejected(
            "the experiment selects no supported agent product".to_owned(),
        ));
    }
    Ok(products)
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
        Some("completed") => 0,
        _ => 1,
    }
}
