//! `opi-eval report` command (task 18.13): offline normalized reporting.
//!
//! One invocation recomputes the normalized report from the sealed
//! assembled outputs of a run root through the report builder's
//! recompute-from-bundle step before rendering, redact-gates publication
//! against declared canaries, and
//! writes the byte-stable conformance-only report (stdout or an explicit
//! output path). It never starts an Agent or provider and never mutates
//! the run root (`P18-RPT-001`). Exit codes: 0 when published, 1 when
//! publication is blocked (typed canary leak) or a bundle failed
//! verification, 2 for a rejected request. Hermetic fixture-grade only;
//! task 18.15 owns the native rerun.

use std::path::PathBuf;

use thiserror::Error;

use crate::report::{RedactionGuard, ReportBuilder};

/// Failures reported by [`report`].
#[derive(Debug, Error)]
pub enum ReportCliError {
    /// The run root or a declared canary file could not be read. The
    /// variant carries the rendered diagnostic so the CLI error stays
    /// self-contained at the public seam.
    #[error("{0}")]
    Read(String),
    /// The output path could not be written.
    #[error("cannot write report output: {0}")]
    Write(#[from] std::io::Error),
}

/// Arguments of the `report` subcommand.
#[derive(Debug, Clone)]
pub struct ReportArgs {
    /// Run root holding sealed bundles, receipts, and the persisted run
    /// report.
    pub root: PathBuf,
    /// Optional output path; when absent the report goes to stdout only.
    pub out: Option<PathBuf>,
    /// Optional file of declared canary secrets (one per line).
    pub canaries: Option<PathBuf>,
}

/// Recompute, redact-gate, and render one offline report.
///
/// This is the production entry seam of the `opi-eval report` command. The
/// returned value is printed by the binary; when `--out` is given the same
/// canonical bytes are written to that path so saved evidence equals the
/// stdout claim.
pub fn report(args: &ReportArgs) -> Result<serde_json::Value, ReportCliError> {
    let guard = match &args.canaries {
        Some(path) => RedactionGuard::from_declared_file(path)
            .map_err(|error| ReportCliError::Read(error.to_string()))?,
        None => RedactionGuard::none(),
    };
    let value = ReportBuilder::new(&args.root)
        .build(&guard)
        .map_err(|error| ReportCliError::Read(error.to_string()))?;
    if let Some(out) = &args.out {
        std::fs::write(
            out,
            serde_json::to_vec(&value).map_err(std::io::Error::other)?,
        )?;
    }
    Ok(value)
}

/// Whether a report warrants a non-success process exit.
pub fn report_exit_code(report: &serde_json::Value) -> i32 {
    if report["outcome"] == "published" {
        0
    } else {
        1
    }
}
