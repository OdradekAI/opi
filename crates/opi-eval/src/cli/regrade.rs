//! `opi-eval regrade` command: actual native regrading (P19-AC-009).
//!
//! Native regrading re-grades preserved real Agent artifacts with the
//! pinned native verifier in a fresh environment through a distinct
//! verifier-only observation reader, persisting one derived grading
//! record per trial without rewriting any original score. No Agent
//! executable is spawned, no credential is resolved, and no provider is
//! contacted (byte checking is the distinct `opi-eval verify` command).

use std::path::PathBuf;

use thiserror::Error;

use crate::regrade::{NativeRegradeError, NativeRegrader};

/// Failures reported by [`regrade`].
#[derive(Debug, Error)]
pub enum RegradeCliError {
    /// The request itself was unusable (output exists, material
    /// rejected, run root unreadable). Exit code 2.
    #[error("{0}")]
    Request(String),
}

/// Arguments of the `regrade` subcommand.
#[derive(Debug, Clone)]
pub struct RegradeArgs {
    /// Run root holding `trials/<id>/bundle` sealed bundles and the
    /// preserved Agent artifact job trees.
    pub root: PathBuf,
    /// Native material manifest with the cached immutable task and
    /// verifier materials.
    pub native_material: PathBuf,
    /// Fresh output root for verifier environments and derived records.
    /// Must not exist; a regrade never overwrites prior output.
    pub out: PathBuf,
}

/// Regrade one run root offline.
///
/// This is the production entry seam of the `opi-eval regrade` command.
pub fn regrade(args: &RegradeArgs) -> Result<serde_json::Value, RegradeCliError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| RegradeCliError::Request(format!("runtime unavailable: {error}")))?;
    let report = runtime
        .block_on(async {
            let cancel = tokio_util::sync::CancellationToken::new();
            super::run::install_signal_cancellation(&cancel);
            NativeRegrader::regrade(&args.root, &args.native_material, &args.out, &cancel).await
        })
        .map_err(|error| match error {
            NativeRegradeError::OutputExists(path) => {
                RegradeCliError::Request(format!("output root already exists: {}", path.display()))
            }
            NativeRegradeError::NotNativeRoot => {
                RegradeCliError::Request("run root holds no native trial to regrade".to_owned())
            }
            NativeRegradeError::Material(reason) => {
                RegradeCliError::Request(format!("native material rejected: {reason}"))
            }
            NativeRegradeError::Io(error) => {
                RegradeCliError::Request(format!("cannot read run root: {error}"))
            }
        })?;
    Ok(report.to_json())
}

/// Exit code of one completed regrade report: 0 only when every sealed
/// trial regraded; 1 names the typed rejections.
pub fn regrade_exit_code(report: &serde_json::Value) -> i32 {
    if report.get("outcome").and_then(|value| value.as_str()) == Some("regraded") {
        0
    } else {
        1
    }
}
