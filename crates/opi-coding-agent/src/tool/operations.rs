//! Per-tool Operations seam (Phase 15 task 15.1).
//!
//! Object-safe async filesystem and bash-execution backends operating on
//! already-resolved paths. Layered BELOW the parent module's
//! [`super::PathPolicy`]: callers must hand these backends a path already
//! cleared by [`super::resolve_tool_path`] (canonical, verbatim-stripped,
//! policy-cleared). Neither [`FileOperations`] nor [`BashOperations`]
//! performs path expansion or workspace confinement.
//!
//! # Async dispatch convention
//!
//! The trait methods return `Pin<Box<dyn Future<Output = ..> + Send>>`
//! matching the crate's existing object-safe async-trait-method pattern
//! (`opi_agent::tool::Tool::execute`, held behind `Box<dyn Tool>`). The
//! sister `opi_ai::auth::OAuthProvider` seam (held behind
//! `Arc<dyn OAuthProvider>` via `BoxAuthFuture<'a, T>`) uses an explicit
//! lifetime on its boxed future; this module follows the `Tool::execute`
//! no-lifetime form because every impl clones borrowed inputs into owned
//! values before `Box::pin(async move { .. })`, so the returned future is
//! `Send + 'static` and borrows neither `&self` nor the inputs. Both
//! patterns are valid for `Arc<dyn T>` injection.
//!
//! # Type-disambiguation policy
//!
//! The local [`ToolDiagnostic`] is a NEW type DISTINCT from
//! `opi_agent::tool::ToolDiagnostic` (which carries `context:
//! serde_json::Value` as a required field). The 15.2 BashTool/ReadTool/
//! WriteTool/EditTool wrappers refer to this local type as
//! `super::operations::ToolDiagnostic` and to the agent-loop type as
//! `opi_agent::tool::ToolDiagnostic` — never bare `ToolDiagnostic`.

#![forbid(unsafe_code)]

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

// =========================================================================
// Error types (house thiserror style; see policy.rs and credential_store.rs)
// =========================================================================

/// Filesystem-operation errors produced BELOW `PathPolicy`. Distinct from
/// `opi_agent::diagnostic::FsToolError` (which classifies path-POLICY failures
/// like `OutsideWorkspace`); `FsOpError` classifies raw-IO failures on an
/// already-resolved path. The 15.2 tool wrappers map `FsOpError` back into the
/// existing `FsToolError` pipeline via [`super::fs_error_result`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FsOpError {
    #[error("path not found: '{path}'")]
    NotFound { path: PathBuf },

    #[error("permission denied: '{path}'")]
    PermissionDenied { path: PathBuf },

    #[error("'{path}' is not a regular file")]
    NotAFile { path: PathBuf },

    #[error("'{path}' is not a directory")]
    NotADirectory { path: PathBuf },

    /// Catch-all for IO errors that do not classify into a typed variant
    /// above. Stringifies the underlying `io::Error` display (`io::Error` is
    /// not `Clone`, so it cannot be carried by value).
    #[error("filesystem error on '{path}': {message}")]
    Io { path: PathBuf, message: String },
}

/// Bash backend errors. Spawn-failure and wait-failure are the only true
/// BACKEND failures and route through `Err(BashOpError)` from
/// [`BashOperations::exec`]. Exit-nonzero, timeout, and cancellation are
/// properly-executed command results and stay IN-BAND in [`BashResult`].
/// The command text is intentionally NOT carried (preserves the no-leak
/// invariant that excludes raw command text from diagnostics).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BashOpError {
    #[error("failed to spawn command: {message}")]
    SpawnFailed { message: String },

    #[error("failed to wait for process: {message}")]
    WaitFailed { message: String },

    #[error("bash backend error: {message}")]
    Other { message: String },
}

// =========================================================================
// OpMetadata
// =========================================================================

/// Filesystem metadata returned by [`FileOperations::metadata`]. Carries the
/// subset of `std::fs::Metadata` fields the file tools touch today (`len`,
/// `is_dir`, `is_file`) plus `readonly` and `modified` as cheap
/// forward-compat. All fields are `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpMetadata {
    pub len: u64,
    pub is_dir: bool,
    pub is_file: bool,
    pub readonly: bool,
    pub modified: Option<SystemTime>,
}

// =========================================================================
// AccessMode
// =========================================================================

/// Access check requested by [`FileOperations::access`]. The local impl maps
/// these to `tokio::fs::metadata` (Exists), `tokio::fs::File::open` probe
/// (Readable), and `tokio::fs::OpenOptions::new().write(true).open` probe
/// (Writable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    Exists,
    Readable,
    Writable,
}

// =========================================================================
// ToolDiagnostic (LOCAL — distinct from opi_agent::tool::ToolDiagnostic)
// =========================================================================

/// Non-fatal observation recorded during a backend operation. Lives in this
/// module so the Operations contract is self-contained: it does NOT pull in
/// the opi-agent tool-result types and is a NEW type distinct from both
/// `opi_agent::tool::ToolDiagnostic` and `opi_agent::diagnostic::Diagnostic`.
/// The `details` field (Optional) replaces opi-agent's required `context`
/// field to signal the type distinction. Does NOT derive `Eq` because
/// `serde_json::Value` is not `Eq` (it can carry floats).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDiagnostic {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

// =========================================================================
// BashRequest + BashResult
// =========================================================================

/// Bash exec request. `env` is a list of `(KEY, VALUE)` pairs APPLIED VIA
/// `Command::envs` ON TOP OF the inherited environment (augment, not
/// replace); empty in current usage. `signal` is the agent-loop
/// `CancellationToken` so cancellation propagates through the trait.
/// `CancellationToken` is not `Eq`, so `BashRequest` derives only `Debug +
/// Clone`.
#[derive(Debug, Clone)]
pub struct BashRequest {
    pub command: String,
    pub cwd: PathBuf,
    pub timeout: Duration,
    pub signal: CancellationToken,
    pub env: Vec<(String, String)>,
}

/// Bash exec result. Exit-nonzero, timeout, and cancellation are represented
/// here IN-BAND (not as `Err(BashOpError)`): `exit_code` is `None` on
/// timeout/cancellation/signal-death, `Some(code)` on clean exit. The 15.2
/// BashTool wrapper lifts these fields into the agent-loop `ToolResult`
/// details block. Field set matches the Phase-15 spec contract.
#[derive(Debug, Clone)]
pub struct BashResult {
    /// Drained stdout bytes.
    pub stdout: Vec<u8>,
    /// Drained stderr bytes.
    pub stderr: Vec<u8>,
    /// `status.code()`; `None` on signal-death / timeout / cancellation.
    pub exit_code: Option<i32>,
    /// Unix signal number if the process was terminated by a signal;
    /// `None` on non-Unix hosts and on clean exit.
    pub signal: Option<i32>,
    /// Non-fatal operation warnings (kill_error, output-spill failure, etc.).
    pub diagnostics: Vec<ToolDiagnostic>,
}

// =========================================================================
// FileOperations trait
// =========================================================================

/// Object-safe async filesystem backend operating on already-resolved paths.
///
/// Layered BELOW `PathPolicy`: callers must hand it a path already cleared by
/// [`super::resolve_tool_path`]. Performs neither path expansion nor
/// workspace confinement. Object-safe: every method takes `&self` with
/// concrete argument types, returns `Pin<Box<dyn Future + Send>>` with
/// concrete outputs, and has no generic methods, no `Self` by value, and no
/// associated types. `Arc<dyn FileOperations>` is itself `Send + Sync`.
pub trait FileOperations: Send + Sync {
    /// Read the full file bytes (no cap, no UTF-8 validation, no streaming).
    /// Binary detection and windowing stay at the tool layer.
    fn read_file(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, FsOpError>> + Send>>;

    /// Atomically replace `path` with `data` via a same-directory temp +
    /// rename so a failed replacement leaves the previous file intact.
    fn write_file(
        &self,
        path: &Path,
        data: &[u8],
    ) -> Pin<Box<dyn Future<Output = Result<(), FsOpError>> + Send>>;

    /// Create directory `path`. When `recursive` is true, create intermediate
    /// directories as needed (idempotent if `path` already exists). When
    /// false, fail if the parent is missing.
    fn mkdir(
        &self,
        path: &Path,
        recursive: bool,
    ) -> Pin<Box<dyn Future<Output = Result<(), FsOpError>> + Send>>;

    /// Return metadata for `path`.
    fn metadata(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn Future<Output = Result<OpMetadata, FsOpError>> + Send>>;

    /// Probe access to `path` according to `mode` without reading or writing.
    fn access(
        &self,
        path: &Path,
        mode: AccessMode,
    ) -> Pin<Box<dyn Future<Output = Result<(), FsOpError>> + Send>>;
}

// =========================================================================
// BashOperations trait
// =========================================================================

/// Object-safe async bash-execution backend. The Phase 15 T4 sandbox lives
/// INSIDE the local impl's `exec` (it IS a `BashOperations` impl, not a
/// wrapper); remote/custom impls own their own confinement.
pub trait BashOperations: Send + Sync {
    fn exec(
        &self,
        request: BashRequest,
    ) -> Pin<Box<dyn Future<Output = Result<BashResult, BashOpError>> + Send>>;
}

// =========================================================================
// LocalFileOperations (DoD-required local impl)
// =========================================================================

/// Local [`FileOperations`] backend. Plain `tokio::fs::*` wrapper with NO
/// sandbox. Stateless today. Atomic write reuses `super::TempFileGuard` so
/// cancellation/Drop discipline matches the existing `write.rs`/`edit.rs`
/// tools byte-for-byte.
#[derive(Debug, Default)]
pub struct LocalFileOperations;

impl LocalFileOperations {
    pub fn new() -> Self {
        Self
    }
}

impl FileOperations for LocalFileOperations {
    fn read_file(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, FsOpError>> + Send>> {
        let path = path.to_path_buf();
        Box::pin(async move {
            // Probe directory first (mirrors read.rs NotAFile check).
            match tokio::fs::metadata(&path).await {
                Ok(meta) if meta.is_dir() => return Err(FsOpError::NotAFile { path }),
                Ok(_) => {}
                Err(e) => return Err(io_to_fs_error(&path, e)),
            }
            tokio::fs::read(&path)
                .await
                .map_err(|e| io_to_fs_error(&path, e))
        })
    }

    fn write_file(
        &self,
        path: &Path,
        data: &[u8],
    ) -> Pin<Box<dyn Future<Output = Result<(), FsOpError>> + Send>> {
        let path = path.to_path_buf();
        let data = data.to_vec();
        Box::pin(async move {
            atomic_write_bytes(&path, "opi-ops-tmp", &data, AtomicWriteFailPoint::None).await
        })
    }

    fn mkdir(
        &self,
        path: &Path,
        recursive: bool,
    ) -> Pin<Box<dyn Future<Output = Result<(), FsOpError>> + Send>> {
        let path = path.to_path_buf();
        Box::pin(async move {
            let result = if recursive {
                tokio::fs::create_dir_all(&path).await
            } else {
                tokio::fs::create_dir(&path).await
            };
            // NotADirectory classification (first_file_ancestor probe) stays at
            // the 15.2 tool layer; the impl is a thin backend that surfaces
            // raw create_dir failures as FsOpError::Io.
            result.map_err(|e| io_to_fs_error(&path, e))
        })
    }

    fn metadata(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn Future<Output = Result<OpMetadata, FsOpError>> + Send>> {
        let path = path.to_path_buf();
        Box::pin(async move {
            let meta = tokio::fs::metadata(&path)
                .await
                .map_err(|e| io_to_fs_error(&path, e))?;
            Ok(OpMetadata {
                len: meta.len(),
                is_dir: meta.is_dir(),
                is_file: meta.is_file(),
                readonly: meta.permissions().readonly(),
                modified: meta.modified().ok(),
            })
        })
    }

    fn access(
        &self,
        path: &Path,
        mode: AccessMode,
    ) -> Pin<Box<dyn Future<Output = Result<(), FsOpError>> + Send>> {
        let path = path.to_path_buf();
        Box::pin(async move {
            match mode {
                AccessMode::Exists => tokio::fs::metadata(&path)
                    .await
                    .map(|_| ())
                    .map_err(|e| io_to_fs_error(&path, e)),
                AccessMode::Readable => tokio::fs::File::open(&path)
                    .await
                    .map(|_| ())
                    .map_err(|e| io_to_fs_error(&path, e)),
                AccessMode::Writable => tokio::fs::OpenOptions::new()
                    .write(true)
                    .open(&path)
                    .await
                    .map(|_| ())
                    .map_err(|e| io_to_fs_error(&path, e)),
            }
        })
    }
}

// =========================================================================
// Internal helpers
// =========================================================================

/// Map a `std::io::Error` to the appropriate `FsOpError` variant.
fn io_to_fs_error(path: &Path, e: std::io::Error) -> FsOpError {
    match e.kind() {
        std::io::ErrorKind::NotFound => FsOpError::NotFound {
            path: path.to_path_buf(),
        },
        std::io::ErrorKind::PermissionDenied => FsOpError::PermissionDenied {
            path: path.to_path_buf(),
        },
        _ => FsOpError::Io {
            path: path.to_path_buf(),
            message: e.to_string(),
        },
    }
}

/// When to inject a failure into the atomic-write recipe. `None` is the
/// production path; the other variants exist only for the
/// `write_file_with_failure` test seam (see below), so they are never
/// constructed in non-test builds — the per-variant `allow(dead_code)` keeps
/// the lib build warning-free under `-D warnings`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AtomicWriteFailPoint {
    None,
    /// Test-only: fail after staging the temp file, before rename.
    #[cfg_attr(not(test), allow(dead_code))]
    AfterTempStage,
    /// Test-only: simulate a rename failure (temp staged, rename skipped).
    #[cfg_attr(not(test), allow(dead_code))]
    AtRename,
}

/// Atomic-write recipe: stage `data` in a sibling dotfile temp in
/// `target.parent()`, then rename over `target`. On any failure the temp is
/// cleaned up via `super::TempFileGuard` and the prior target (if any) is
/// left intact — `tokio::fs::rename` is atomic on the same filesystem, so an
/// interrupted replacement leaves either the full new content or the prior
/// content (never a partial/truncated mix). Does NOT classify
/// `NotADirectory` (the 15.2 tool wrapper probes via `metadata()`); any
/// `create_dir_all` failure surfaces as [`FsOpError::Io`].
async fn atomic_write_bytes(
    target: &Path,
    tag: &str,
    data: &[u8],
    fail_at: AtomicWriteFailPoint,
) -> Result<(), FsOpError> {
    let parent_dir = target.parent().unwrap_or(target);
    if let Err(e) = tokio::fs::create_dir_all(parent_dir).await {
        return Err(io_to_fs_error(parent_dir, e));
    }
    let file_name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_path = parent_dir.join(format!(".{file_name}.{tag}-{pid}-{nanos}"));
    let mut guard = super::TempFileGuard::new(temp_path);

    if let Err(e) = tokio::fs::write(guard.path(), data).await {
        guard.cleanup().await;
        return Err(FsOpError::Io {
            path: target.to_path_buf(),
            message: e.to_string(),
        });
    }

    if fail_at == AtomicWriteFailPoint::AfterTempStage {
        guard.cleanup().await;
        return Err(FsOpError::Io {
            path: target.to_path_buf(),
            message: "injected temp-stage failure".to_string(),
        });
    }

    if fail_at == AtomicWriteFailPoint::AtRename {
        // Simulate rename failure: temp is staged but rename does not run.
        // The prior target is untouched; cleanup removes the temp.
        guard.cleanup().await;
        return Err(FsOpError::Io {
            path: target.to_path_buf(),
            message: "injected rename failure".to_string(),
        });
    }

    if let Err(e) = tokio::fs::rename(guard.path(), target).await {
        guard.cleanup().await;
        return Err(FsOpError::Io {
            path: target.to_path_buf(),
            message: e.to_string(),
        });
    }
    guard.disarm();
    Ok(())
}

// =========================================================================
// Test-only failure-injection seam
// =========================================================================

#[cfg(test)]
impl LocalFileOperations {
    /// Test-only seam that drives [`atomic_write_bytes`] with an injected
    /// failure point so the rename-stage error branch can be exercised
    /// without contaminating the production trait surface or the production
    /// `write_file` method. Private: only the inline `mod tests` (a descendant
    /// of this module) calls it.
    async fn write_file_with_failure(
        &self,
        path: &Path,
        data: &[u8],
        fail_at: AtomicWriteFailPoint,
    ) -> Result<(), FsOpError> {
        atomic_write_bytes(path, "opi-ops-tmp", data, fail_at).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn local_file_operations_write_then_read_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("f.txt");
        let ops = LocalFileOperations::new();
        ops.write_file(&target, b"hello").await.unwrap();
        let read = ops.read_file(&target).await.unwrap();
        assert_eq!(read, b"hello");
    }

    /// Smoke-addendum (DoD): a failed atomic replacement preserves the prior
    /// file. The rename branch is exercised via the AtRename injection point;
    /// the real `tokio::fs::rename` is atomic so we prove the recipe's
    /// error-handling discipline (cleanup + return Err without touching the
    /// target) rather than forcing a real OS rename failure.
    #[tokio::test]
    async fn atomic_write_failure_at_rename_preserves_prior_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        std::fs::write(&target, "original").unwrap();

        let ops = LocalFileOperations::new();
        let result = ops
            .write_file_with_failure(&target, b"replacement", AtomicWriteFailPoint::AtRename)
            .await;
        assert!(result.is_err(), "injected rename failure must return Err");

        let after = std::fs::read_to_string(&target).unwrap();
        assert_eq!(
            after, "original",
            "prior content must survive failed rename"
        );

        let residue: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains("opi-ops-tmp"))
            .collect();
        assert!(residue.is_empty(), "temp residue left behind: {residue:?}");
    }

    #[tokio::test]
    async fn atomic_write_failure_after_temp_stage_preserves_prior_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        std::fs::write(&target, "original").unwrap();

        let ops = LocalFileOperations::new();
        let result = ops
            .write_file_with_failure(
                &target,
                b"replacement",
                AtomicWriteFailPoint::AfterTempStage,
            )
            .await;
        assert!(
            result.is_err(),
            "injected temp-stage failure must return Err"
        );

        let after = std::fs::read_to_string(&target).unwrap();
        assert_eq!(
            after, "original",
            "prior content must survive temp-stage failure"
        );

        let residue: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains("opi-ops-tmp"))
            .collect();
        assert!(residue.is_empty(), "temp residue left behind: {residue:?}");
    }
}
