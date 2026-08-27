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
    },
    /// Run one fixture-level conformance case against a concrete adapter
    /// through the shared execution seams (task 18.10.1).
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
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate { config } => match cli::validate(&config) {
            Ok(summary) => {
                println!("{summary}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
        Command::Conformance {
            suite,
            adapter,
            case,
            root,
            fixtures,
            provider,
        } => {
            let args = cli::conformance::ConformanceArgs {
                suite,
                adapter,
                case,
                root,
                fixtures,
                provider,
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
