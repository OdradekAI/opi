//! `opi-eval` binary entry point (provisional Phase 18 seam).

use std::process::ExitCode;

use clap::{Parser, Subcommand};

use opi_eval::cli;

#[derive(Debug, Parser)]
#[command(
    name = "opi-eval",
    about = "Unpublished Independent Companion for cross-agent evaluation experiments (provisional Phase 18 seam)",
    disable_version_flag = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Resolve an experiment document into a frozen contract.
    Validate {
        /// Path to the experiment document.
        #[arg(long)]
        config: std::path::PathBuf,
        /// Resolved native material manifest (task 18.14.1): derives and
        /// appends the native integrity identity for config pinning.
        #[arg(long)]
        native_material: Option<std::path::PathBuf>,
    },
    /// Run one fixture-level conformance case against a concrete adapter
    /// through the shared execution seams (task 18.10.1).
    /// Run one assembled hermetic experiment end to end through the
    /// paired evaluation runner (task 18.12).
    Run {
        /// Path to the experiment document.
        #[arg(long)]
        config: std::path::PathBuf,
        /// Fresh run root for trial directories and receipts.
        #[arg(long)]
        root: std::path::PathBuf,
        /// Repository `crates/opi-eval/tests/fixtures` root.
        #[arg(long)]
        fixtures: std::path::PathBuf,
        /// Hermetic staging behavior (helper-process selection).
        #[arg(long, default_value = "happy")]
        behavior: String,
        /// Classify durable trial states instead of running.
        #[arg(long)]
        recover: bool,
        /// Re-run one crashed trial's whole group under fresh identities.
        #[arg(long)]
        replacement_for: Option<String>,
        /// Optional file of declared canary secrets (one per line); any
        /// canary found in staged exportable content blocks sealing.
        #[arg(long)]
        canaries: Option<std::path::PathBuf>,
        /// Resolved native material manifest (task 18.14.1): exact built
        /// agents, materialized task packages, pinned verifier and oracle
        /// entrypoints, and the scripted-provider listener endpoint.
        #[arg(long)]
        native_material: Option<std::path::PathBuf>,
        /// Run only the upstream oracle preflight, then stop (native mode).
        #[arg(long)]
        preflight_only: bool,
    },
    /// Re-verify every sealed trial bundle under a run root without
    /// starting an Agent or mutating anything (task 18.13).
    Regrade {
        /// Run root holding `trials/<id>/bundle` sealed bundles.
        #[arg(long)]
        root: std::path::PathBuf,
    },
    /// Recompute and render the offline normalized report from sealed
    /// assembled outputs (task 18.13).
    Report {
        /// Run root holding sealed bundles, receipts, and the persisted
        /// run report.
        #[arg(long)]
        root: std::path::PathBuf,
        /// Optional output path for the normalized report bytes.
        #[arg(long)]
        out: Option<std::path::PathBuf>,
        /// Optional file of declared canary secrets (one per line); any
        /// canary found in exportable bundle content blocks publication.
        #[arg(long)]
        canaries: Option<std::path::PathBuf>,
    },
    Conformance {
        /// `agent` or `benchmark`.
        #[arg(long)]
        suite: String,
        /// `opi`, `pi`, `terminal-bench-2.1`, `terminal-bench-3.0`, or `deepswe`.
        #[arg(long)]
        adapter: String,
        /// Case id from the pinned conformance matrices.
        #[arg(long)]
        case: String,
        /// Fresh run root for isolated directories and helper processes.
        #[arg(long)]
        root: std::path::PathBuf,
        /// Repository `crates/opi-eval/tests/fixtures` root.
        #[arg(long)]
        fixtures: std::path::PathBuf,
        /// `scripts/phase18-scripted-provider.py`.
        #[arg(long)]
        provider: std::path::PathBuf,
        /// Resolved native material manifest (task 18.14.1): reruns the
        /// admitted case subset through the exact built executables.
        #[arg(long)]
        native_material: Option<std::path::PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate {
            config,
            native_material,
        } => {
            let result: Result<String, String> = match &native_material {
                Some(material_path) => cli::validate_native(&config, material_path)
                    .map(|summary| summary.to_string())
                    .map_err(|error| error.to_string()),
                None => cli::validate(&config)
                    .map(|summary| summary.to_string())
                    .map_err(|error| error.to_string()),
            };
            match result {
                Ok(summary) => {
                    println!("{summary}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("error: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Run {
            config,
            root,
            fixtures,
            behavior,
            recover,
            replacement_for,
            canaries,
            native_material,
            preflight_only,
        } => {
            let args = cli::run::RunArgs {
                config,
                root,
                fixtures,
                behavior,
                recover,
                replacement_for,
                canaries,
                native_material,
                preflight_only,
            };
            match cli::run::run(&args) {
                Ok(report) => {
                    println!(
                        "{}",
                        serde_json::to_string(&report).expect("run report serializes")
                    );
                    ExitCode::from(cli::run::report_exit_code(&report) as u8)
                }
                Err(error) => {
                    eprintln!("error: {error}");
                    ExitCode::from(2)
                }
            }
        }
        Command::Regrade { root } => {
            let args = cli::regrade::RegradeArgs { root };
            match cli::regrade::regrade(&args) {
                Ok(report) => {
                    println!(
                        "{}",
                        serde_json::to_string(&report).expect("regrade report serializes")
                    );
                    ExitCode::from(cli::regrade::regrade_exit_code(&report) as u8)
                }
                Err(error) => {
                    eprintln!("error: {error}");
                    ExitCode::from(2)
                }
            }
        }
        Command::Report {
            root,
            out,
            canaries,
        } => {
            let args = cli::report::ReportArgs {
                root,
                out,
                canaries,
            };
            match cli::report::report(&args) {
                Ok(report) => {
                    println!(
                        "{}",
                        serde_json::to_string(&report).expect("report serializes")
                    );
                    ExitCode::from(cli::report::report_exit_code(&report) as u8)
                }
                Err(error) => {
                    eprintln!("error: {error}");
                    ExitCode::from(2)
                }
            }
        }
        Command::Conformance {
            suite,
            adapter,
            case,
            root,
            fixtures,
            provider,
            native_material,
        } => {
            let args = cli::conformance::ConformanceArgs {
                suite,
                adapter,
                case,
                root,
                fixtures,
                provider,
                native_material,
            };
            match cli::conformance::run(&args) {
                Ok(report) => {
                    println!(
                        "{}",
                        serde_json::to_string(&report).expect("conformance report serializes")
                    );
                    if report.met {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    }
                }
                Err(error) => {
                    eprintln!("error: {error}");
                    ExitCode::from(2)
                }
            }
        }
    }
}
