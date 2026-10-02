//! `opi-eval` binary entry point.

use std::process::ExitCode;

use clap::{Parser, Subcommand};

use opi_eval::cli;

// Binary-local capture modules (P19-AUD-003): the pure filter is shared
// with the library's unit contract through the same source file, and the
// runtime (pipes, process group, signals) exists only in this binary.
#[path = "native_capture.rs"]
mod native_capture;
#[path = "native_capture_runtime.rs"]
mod native_capture_runtime;

#[derive(Debug, Parser)]
#[command(
    name = "opi-eval",
    about = "Unpublished Independent Companion for agent-neutral cross-agent evaluation",
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
        /// Resolved native material manifest (native mode): derives and
        /// appends the native integrity identity for config pinning.
        #[arg(long)]
        native_material: Option<std::path::PathBuf>,
    },
    /// Run a frozen native or hermetic experiment through the evaluation runner.
    Run {
        /// Path to the experiment document.
        #[arg(long, conflicts_with = "plan")]
        config: Option<std::path::PathBuf>,
        /// Path to a frozen run plan (`plan --out`): executes exactly
        /// the plan's trials after re-verifying its digest, material,
        /// and corpus expansion.
        #[arg(long)]
        plan: Option<std::path::PathBuf>,
        /// Fresh run root for trial directories and receipts.
        #[arg(long)]
        root: std::path::PathBuf,
        /// Repository fixtures root (required for hermetic execution only).
        #[arg(long)]
        fixtures: Option<std::path::PathBuf>,
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
        /// Resolved native material manifest (native mode): exact built
        /// agents, materialized task packages, pinned verifier
        /// entrypoints, the selected integration module, and the
        /// scripted-provider listener endpoint.
        #[arg(long)]
        native_material: Option<std::path::PathBuf>,
    },
    /// Verify every sealed trial bundle under a run root against its
    /// original seals without starting an Agent or mutating anything.
    Verify {
        /// Run root holding `trials/<id>/bundle` sealed bundles.
        #[arg(long)]
        root: std::path::PathBuf,
    },
    /// Re-grade preserved Agent artifacts with the pinned native
    /// verifier in fresh offline environments; nothing Agent- or
    /// provider-side ever runs.
    Regrade {
        /// Run root holding `trials/<id>/bundle` sealed bundles and the
        /// preserved Agent artifact job trees.
        #[arg(long)]
        root: std::path::PathBuf,
        /// Resolved native material manifest with the cached immutable
        /// task and verifier materials.
        #[arg(long)]
        native_material: std::path::PathBuf,
        /// Fresh output root for verifier environments and derived
        /// records (must not exist).
        #[arg(long)]
        out: std::path::PathBuf,
    },
    /// Recompute and render the offline normalized report from sealed
    /// assembled outputs.
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
    /// Preview the experiment and freeze its selection into a
    /// digest-bound run plan: enumerates the pinned corpus inventory
    /// from metadata (no image pulled or started), expands the
    /// selection into exact trials, and checks every selected task's
    /// resource requirements against the local observations.
    Plan {
        /// Path to the experiment document.
        #[arg(long)]
        config: std::path::PathBuf,
        /// Resolved native material manifest (native mode).
        #[arg(long)]
        native_material: Option<std::path::PathBuf>,
        /// Repository `crates/opi-eval/tests/fixtures` root (required
        /// without --native-material).
        #[arg(long)]
        fixtures: Option<std::path::PathBuf>,
        /// Output path of the frozen run plan.
        #[arg(long)]
        out: std::path::PathBuf,
    },
    /// Diagnose the local environment and declared materials without
    /// starting a model: engine reachability, architecture, observed
    /// resources, and missing materials with reasons for unknown
    /// observations.
    Doctor {
        /// Path to the experiment document.
        #[arg(long)]
        config: std::path::PathBuf,
    },
    /// Verify the declared local cache against the pinned static lock
    /// and freeze a local-origin native-material manifest consumable by
    /// `validate` and `run`. No model runs and no credential is read.
    Prepare {
        /// Path to the experiment document.
        #[arg(long)]
        config: std::path::PathBuf,
        /// Output directory for the material manifest and report.
        #[arg(long)]
        out: std::path::PathBuf,
        /// Local Git repository providing a selected cold Opi build.
        #[arg(long, requires = "opi_commit")]
        opi_source: Option<std::path::PathBuf>,
        /// Exact local 40-hex Opi commit; never inferred or fetched.
        #[arg(long, requires = "opi_source")]
        opi_commit: Option<String>,
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
        /// `crates/opi-eval/scripts/scripted-provider.py`.
        #[arg(long)]
        provider: std::path::PathBuf,
        /// Resolved native material manifest (native mode): reruns the
        /// admitted case subset through the exact built executables.
        #[arg(long)]
        native_material: Option<std::path::PathBuf>,
    },
}

fn main() -> ExitCode {
    // Hidden binary-local capture dispatch (P19-AUD-003): before any
    // public CLI parsing, `__native-capture` runs the bounded privacy
    // filter. It is an internal Companion mode - no public CLI surface,
    // configuration key or SDK contract - invoked only by the pinned
    // integrations with the staged helper executable.
    let mut arguments = std::env::args_os();
    let program = arguments.next();
    if arguments.next().as_deref().and_then(|value| value.to_str()) == Some("__native-capture") {
        let rest: Vec<std::ffi::OsString> = arguments.collect();
        return native_capture_runtime::run_hidden_mode(&rest, program.as_ref());
    }
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
            plan,
            root,
            fixtures,
            behavior,
            recover,
            replacement_for,
            canaries,
            native_material,
        } => {
            let args = cli::run::RunArgs {
                config,
                plan,
                root,
                fixtures,
                behavior,
                recover,
                replacement_for,
                canaries,
                native_material,
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
        Command::Doctor { config } => {
            let args = cli::prepare::DoctorArgs { config };
            match cli::prepare::doctor(&args) {
                Ok(report) => {
                    println!(
                        "{}",
                        serde_json::to_string(&report).expect("doctor report serializes")
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("error: {error}");
                    ExitCode::from(2)
                }
            }
        }
        Command::Prepare {
            config,
            out,
            opi_source,
            opi_commit,
        } => {
            let args = cli::prepare::PrepareArgs {
                config,
                out,
                opi_source,
                opi_commit,
            };
            match cli::prepare::prepare(&args) {
                Ok(report) => {
                    println!(
                        "{}",
                        serde_json::to_string(&report).expect("prepare report serializes")
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("error: {error}");
                    ExitCode::from(2)
                }
            }
        }
        Command::Plan {
            config,
            native_material,
            fixtures,
            out,
        } => {
            let args = cli::plan::PlanArgs {
                config,
                native_material,
                fixtures,
                out,
            };
            match cli::plan::plan(&args) {
                Ok(preview) => {
                    println!(
                        "{}",
                        serde_json::to_string(&preview).expect("plan preview serializes")
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("error: {error}");
                    ExitCode::from(2)
                }
            }
        }
        Command::Verify { root } => {
            let args = cli::verify::VerifyArgs { root };
            match cli::verify::verify(&args) {
                Ok(report) => {
                    println!(
                        "{}",
                        serde_json::to_string(&report).expect("verification report serializes")
                    );
                    ExitCode::from(cli::verify::verify_exit_code(&report) as u8)
                }
                Err(error) => {
                    eprintln!("error: {error}");
                    ExitCode::from(2)
                }
            }
        }
        Command::Regrade {
            root,
            native_material,
            out,
        } => {
            let args = cli::regrade::RegradeArgs {
                root,
                native_material,
                out,
            };
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
