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
    }
}
