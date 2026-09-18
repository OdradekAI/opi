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
    /// Repository fixtures root, required only for hermetic execution.
    pub fixtures: Option<PathBuf>,
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
///
/// One run-level cancellation token is shared with the whole runner:
/// SIGINT or SIGTERM cancels new dispatch, settles the observed outcomes,
/// and lets the run render its interrupted report instead of dying
/// mid-settlement (P19-D07).
pub fn run(args: &RunArgs) -> Result<serde_json::Value, RunCliError> {
    let config_path = match (&args.config, &args.plan) {
        (Some(config), None) => config,
        (None, Some(plan)) => plan,
        _ => {
            return Err(RunCliError::Rejected(
                "run requires exactly one of --config or --plan".to_owned(),
            ));
        }
    };
    let mut material_path = args.native_material.clone();
    let source = match &args.plan {
        Some(plan_path) => {
            let plan: serde_json::Value = serde_json::from_slice(
                &std::fs::read(plan_path)
                    .map_err(|error| RunCliError::Rejected(error.to_string()))?,
            )
            .map_err(|error| RunCliError::Rejected(format!("plan parse failed: {error}")))?;
            let source = crate::runner::plan::plan_source(&plan)
                .map_err(|error| RunCliError::Rejected(error.to_string()))?;
            match plan["material"].as_object() {
                Some(record) => {
                    let recorded = record
                        .get("path")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| {
                            RunCliError::Rejected("run plan material record has no path".to_owned())
                        })?;
                    let path = material_path.unwrap_or_else(|| {
                        crate::runner::plan::resolve_plan_path(
                            plan_path,
                            std::path::Path::new(recorded),
                        )
                    });
                    let bytes = std::fs::read(&path)
                        .map_err(|error| RunCliError::Rejected(error.to_string()))?;
                    if Some(crate::runner::material::sha256_hex(&bytes).as_str())
                        != record
                            .get("manifest_sha256")
                            .and_then(serde_json::Value::as_str)
                    {
                        return Err(RunCliError::Rejected(format!(
                            "run-plan material drift at {}",
                            path.display()
                        )));
                    }
                    material_path = Some(path);
                }
                None if material_path.is_some() => {
                    return Err(RunCliError::Rejected(
                        "run plan does not bind native material".to_owned(),
                    ));
                }
                None => {}
            }
            source
        }
        None => std::fs::read_to_string(config_path).map_err(|error| {
            RunCliError::Rejected(format!("cannot read experiment document: {error}"))
        })?,
    };
    let experiment = crate::experiment::ResolvedExperiment::resolve(&source)
        .map_err(|error| RunCliError::Rejected(error.to_string()))?;
    let material = material_path
        .as_ref()
        .map(|path| {
            let products = selected_products_from(&experiment).map_err(RunCliError::Rejected)?;
            crate::runner::material::NativeMaterial::load_for(path, &products)
                .map_err(|error| RunCliError::Rejected(error.to_string()))
        })
        .transpose()?;
    if material.is_none() && args.fixtures.is_none() {
        return Err(RunCliError::Rejected(
            "--fixtures is required for hermetic execution".to_owned(),
        ));
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| RunCliError::Rejected(error.to_string()))?;
    let report = runtime.block_on(async {
        let cancel = tokio_util::sync::CancellationToken::new();
        install_signal_cancellation(&cancel);
        experiment::run_experiment(
            &RunRequest {
                config_path: config_path.clone(),
                root: args.root.clone(),
                fixtures: args.fixtures.clone(),
                behavior: args.behavior.clone(),
                recover: args.recover,
                replacement_for: args.replacement_for.clone(),
                canaries: read_canaries(args.canaries.as_deref())
                    .map_err(|error| RunCliError::Rejected(error.to_string()))?,
                material,
                plan: args.plan.clone(),
            },
            &cancel,
        )
        .await
        .map_err(|error| RunCliError::Rejected(error.to_string()))
    })?;
    Ok(report)
}

/// Cancels `token` on the first SIGINT or SIGTERM. Signal delivery is
/// cooperative from here on: the process no longer dies mid-settlement;
/// the runner stops new dispatch, cancels only the owned process trees,
/// and still renders its interrupted report.
pub(super) fn install_signal_cancellation(token: &tokio_util::sync::CancellationToken) {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut interrupt = signal(SignalKind::interrupt()).expect("install SIGINT handler");
        let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        let token = token.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = interrupt.recv() => token.cancel(),
                _ = terminate.recv() => token.cancel(),
            }
        });
    }
    #[cfg(not(unix))]
    {
        let token = token.clone();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                token.cancel();
            }
        });
    }
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
