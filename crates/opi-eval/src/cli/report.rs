//! `opi-eval report` command (task 18.13): offline normalized reporting.
//!
//! One invocation recomputes the normalized report from the verified
//! sealed bundles of a run root through the report builder's
//! recompute-from-bundle step before rendering, redact-gates publication
//! against declared canaries, and writes the byte-stable
//! conformance-only report (stdout or an explicit output path). It never
//! starts an Agent or provider and never mutates the run root
//! (`P18-RPT-001`). Exit codes: 0 when published, 1 when publication is
//! blocked (typed canary leak) or a contributing bundle failed
//! verification or sealed-input parsing, 2 for a rejected request. The
//! output path is opened with create-new semantics outside the run root,
//! so neither sealed bytes nor a prior report can ever be replaced.
//! Hermetic fixture-grade only; task 18.15 owns the native rerun.

use std::path::{Path, PathBuf};

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
    /// The output path was rejected: it lies inside the run root or
    /// already exists.
    #[error("report output rejected: {0}")]
    OutRejected(String),
    /// The output path could not be written.
    #[error("cannot write report output: {0}")]
    Write(#[from] std::io::Error),
}

/// Arguments of the `report` subcommand.
#[derive(Debug, Clone)]
pub struct ReportArgs {
    /// Run root holding sealed bundles.
    pub root: PathBuf,
    /// Optional output path; when absent the report goes to stdout only.
    pub out: Option<PathBuf>,
    /// Optional file of declared canary secrets (one per line).
    pub canaries: Option<PathBuf>,
}

/// Whether `out` lies inside `root` (both absolutized lexically without
/// requiring the target to exist).
fn absolute(path: &Path) -> std::borrow::Cow<'_, Path> {
    if path.is_absolute() {
        std::borrow::Cow::Borrowed(path)
    } else {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        std::borrow::Cow::Owned(cwd.join(path))
    }
}

/// Whether `out` lies inside `root` (lexically, both absolute).
fn is_inside(root: &Path, out: &Path) -> bool {
    out.starts_with(root)
}

/// Opens the report output with create-new semantics: the target must
/// live outside the run root and must not exist, so sealed bytes and
/// prior reports can never be replaced.
fn open_output(root: &Path, out: &Path, bytes: &[u8]) -> Result<(), ReportCliError> {
    let root_lexical = absolute(root);
    let root_abs = root_lexical
        .canonicalize()
        .unwrap_or(root_lexical.into_owned());
    let out_abs = absolute(out).into_owned();
    if is_inside(&root_abs, &out_abs) {
        return Err(ReportCliError::OutRejected(format!(
            "{} lies inside the run root {}",
            out_abs.display(),
            root_abs.display()
        )));
    }
    if out_abs.exists() {
        return Err(ReportCliError::OutRejected(format!(
            "{} already exists; a prior report is never replaced",
            out_abs.display()
        )));
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&out_abs)?;
    file.write_all(bytes)?;
    Ok(())
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
        let bytes = serde_json::to_vec(&value).map_err(std::io::Error::other)?;
        open_output(&args.root, out, &bytes)?;
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
