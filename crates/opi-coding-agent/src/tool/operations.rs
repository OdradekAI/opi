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

use std::fs::{File, OpenOptions};
use std::future::Future;
use std::io::{self, Read as IoRead, Write as IoWrite};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions as CapOpenOptions};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
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
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum BashOpError {
    #[error("failed to spawn command: {message}")]
    SpawnFailed { message: String },

    #[error("failed to wait for process: {message}")]
    WaitFailed { message: String },

    #[error("bash backend error: {message}")]
    Other { message: String },

    /// A backend failure accompanied by non-fatal diagnostics observed before
    /// the terminal error. The source preserves the existing typed cause while
    /// the wrapper lets callers surface lifecycle/sandbox degradation instead
    /// of dropping it on spawn/wait/custom-backend error paths.
    #[error("{source}")]
    BackendFailure {
        source: Box<BashOpError>,
        diagnostics: Vec<ToolDiagnostic>,
    },
}

impl BashOpError {
    pub fn with_diagnostics(self, diagnostics: Vec<ToolDiagnostic>) -> Self {
        if diagnostics.is_empty() {
            self
        } else {
            match self {
                Self::BackendFailure {
                    source,
                    diagnostics: mut existing,
                } => {
                    existing.extend(diagnostics);
                    Self::BackendFailure {
                        source,
                        diagnostics: existing,
                    }
                }
                source => Self::BackendFailure {
                    source: Box::new(source),
                    diagnostics,
                },
            }
        }
    }

    pub fn root_cause(&self) -> &Self {
        match self {
            Self::BackendFailure { source, .. } => source.root_cause(),
            other => other,
        }
    }

    pub fn diagnostics(&self) -> &[ToolDiagnostic] {
        match self {
            Self::BackendFailure { diagnostics, .. } => diagnostics,
            _ => &[],
        }
    }
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
///
/// `backend` carries the model-supplied backend id under the `model` execution
/// strategy (Phase 16.9). It is `None` for the local backend and for
/// `fixed`/`rules` strategies, where the router ignores it; only
/// `RoutedBashOperations::exec` forwards it to `resolve_selection`.
#[derive(Debug, Clone)]
pub struct BashRequest {
    pub command: String,
    pub cwd: PathBuf,
    pub timeout: Duration,
    pub signal: CancellationToken,
    pub env: Vec<(String, String)>,
    pub backend: Option<String>,
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

/// Local [`FileOperations`] backend.
///
/// Paths inside `workspace_root` are resolved beneath a held directory
/// capability, so an ancestor symlink/junction swap after `PathPolicy` cannot
/// redirect an operation outside the workspace. Explicitly allowed external
/// reads retain ambient-path behavior. Atomic workspace writes stage and rename
/// through a held parent-directory capability.
#[derive(Debug)]
pub struct LocalFileOperations {
    workspace_root: PathBuf,
    workspace: Result<Arc<Dir>, FsOpError>,
}

impl LocalFileOperations {
    pub fn new(workspace_root: PathBuf) -> Self {
        match std::fs::canonicalize(&workspace_root) {
            Ok(canonical) => {
                let canonical = super::strip_verbatim_prefix(&canonical);
                let workspace = Dir::open_ambient_dir(&canonical, ambient_authority())
                    .map(Arc::new)
                    .map_err(|error| io_to_fs_error(&canonical, error));
                Self {
                    workspace_root: canonical,
                    workspace,
                }
            }
            Err(error) => Self {
                workspace_root: workspace_root.clone(),
                workspace: Err(io_to_fs_error(&workspace_root, error)),
            },
        }
    }

    fn workspace_target(&self, path: &Path) -> Option<Result<(Arc<Dir>, PathBuf), FsOpError>> {
        let relative = path.strip_prefix(&self.workspace_root).ok()?;
        let relative = if relative.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            relative.to_path_buf()
        };
        Some(
            self.workspace
                .clone()
                .map(|workspace| (workspace, relative)),
        )
    }
}

impl FileOperations for LocalFileOperations {
    fn read_file(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, FsOpError>> + Send>> {
        let path = path.to_path_buf();
        if let Some(target) = self.workspace_target(&path) {
            return Box::pin(async move {
                let (workspace, relative) = target?;
                run_blocking_fs(path.clone(), move || {
                    let metadata = workspace
                        .metadata(&relative)
                        .map_err(|error| io_to_fs_error(&path, error))?;
                    if metadata.is_dir() {
                        return Err(FsOpError::NotAFile { path });
                    }
                    let mut file = workspace
                        .open(&relative)
                        .map_err(|error| io_to_fs_error(&path, error))?;
                    let mut bytes = Vec::new();
                    file.read_to_end(&mut bytes)
                        .map_err(|error| io_to_fs_error(&path, error))?;
                    Ok(bytes)
                })
                .await
            });
        }
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
        if let Some(target) = self.workspace_target(&path) {
            return Box::pin(async move {
                let (workspace, relative) = target?;
                run_blocking_fs(path.clone(), move || {
                    cap_atomic_write_bytes(&workspace, &relative, &path, &data)
                })
                .await
            });
        }
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
        if let Some(target) = self.workspace_target(&path) {
            return Box::pin(async move {
                let (workspace, relative) = target?;
                run_blocking_fs(path.clone(), move || {
                    let result = if recursive {
                        workspace.create_dir_all(&relative)
                    } else {
                        workspace.create_dir(&relative)
                    };
                    result.map_err(|error| io_to_fs_error(&path, error))
                })
                .await
            });
        }
        Box::pin(async move {
            let result = if recursive {
                tokio::fs::create_dir_all(&path).await
            } else {
                tokio::fs::create_dir(&path).await
            };
            // The shared mapper retains NotADirectory across platform-specific
            // create-dir failures; the tool layer walks ancestors through this
            // same injected metadata backend to identify the file component.
            result.map_err(|e| io_to_fs_error(&path, e))
        })
    }

    fn metadata(
        &self,
        path: &Path,
    ) -> Pin<Box<dyn Future<Output = Result<OpMetadata, FsOpError>> + Send>> {
        let path = path.to_path_buf();
        if let Some(target) = self.workspace_target(&path) {
            return Box::pin(async move {
                let (workspace, relative) = target?;
                run_blocking_fs(path.clone(), move || {
                    let meta = workspace
                        .metadata(&relative)
                        .map_err(|error| io_to_fs_error(&path, error))?;
                    Ok(OpMetadata {
                        len: meta.len(),
                        is_dir: meta.is_dir(),
                        is_file: meta.is_file(),
                        readonly: meta.permissions().readonly(),
                        modified: meta.modified().ok().map(|time| time.into_std()),
                    })
                })
                .await
            });
        }
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
        if let Some(target) = self.workspace_target(&path) {
            return Box::pin(async move {
                let (workspace, relative) = target?;
                run_blocking_fs(path.clone(), move || {
                    let result = match mode {
                        AccessMode::Exists => workspace.metadata(&relative).map(|_| ()),
                        AccessMode::Readable => workspace.open(&relative).map(|_| ()),
                        AccessMode::Writable => {
                            let mut options = CapOpenOptions::new();
                            options.write(true);
                            workspace.open_with(&relative, &options).map(|_| ())
                        }
                    };
                    result.map_err(|error| io_to_fs_error(&path, error))
                })
                .await
            });
        }
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

async fn run_blocking_fs<T, F>(path: PathBuf, operation: F) -> Result<T, FsOpError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, FsOpError> + Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|error| FsOpError::Io {
            path,
            message: format!("filesystem worker failed: {error}"),
        })?
}

fn cap_atomic_write_bytes(
    workspace: &Dir,
    relative: &Path,
    display_path: &Path,
    data: &[u8],
) -> Result<(), FsOpError> {
    let parent = relative
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    workspace
        .create_dir_all(parent)
        .map_err(|error| io_to_fs_error(display_path, error))?;
    let parent_dir = workspace
        .open_dir(parent)
        .map_err(|error| io_to_fs_error(display_path, error))?;
    let file_name = relative.file_name().ok_or_else(|| FsOpError::NotAFile {
        path: display_path.to_path_buf(),
    })?;

    let mut allocated = None;
    for _ in 0..MAX_ATOMIC_TEMP_ATTEMPTS {
        let candidate = strong_atomic_temp_name(file_name, "opi-ops-tmp", display_path)?;
        let mut options = CapOpenOptions::new();
        options.write(true).create_new(true);
        match parent_dir.open_with(&candidate, &options) {
            Ok(file) => {
                allocated = Some((candidate, file));
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(io_to_fs_error(display_path, error)),
        }
    }
    let (temp_name, mut temp_file) = allocated.ok_or_else(|| FsOpError::Io {
        path: display_path.to_path_buf(),
        message: format!(
            "failed to allocate atomic staging file after {MAX_ATOMIC_TEMP_ATTEMPTS} collisions"
        ),
    })?;
    let mut guard = CapTempFileGuard::new(parent_dir, temp_name);

    temp_file
        .write_all(data)
        .map_err(|error| io_to_fs_error(display_path, error))?;
    temp_file
        .flush()
        .map_err(|error| io_to_fs_error(display_path, error))?;
    drop(temp_file);
    guard
        .rename_to(file_name)
        .map_err(|error| io_to_fs_error(display_path, error))?;
    Ok(())
}

struct CapTempFileGuard {
    parent: Dir,
    name: PathBuf,
    armed: bool,
}

impl CapTempFileGuard {
    fn new(parent: Dir, name: PathBuf) -> Self {
        Self {
            parent,
            name,
            armed: true,
        }
    }

    fn rename_to(&mut self, target: &std::ffi::OsStr) -> io::Result<()> {
        self.parent
            .rename(&self.name, &self.parent, Path::new(target))?;
        self.armed = false;
        Ok(())
    }
}

impl Drop for CapTempFileGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.parent.remove_file(&self.name);
        }
    }
}

/// Map a `std::io::Error` to the appropriate `FsOpError` variant.
fn io_to_fs_error(path: &Path, e: std::io::Error) -> FsOpError {
    match e.kind() {
        std::io::ErrorKind::NotFound => FsOpError::NotFound {
            path: path.to_path_buf(),
        },
        std::io::ErrorKind::PermissionDenied => FsOpError::PermissionDenied {
            path: path.to_path_buf(),
        },
        std::io::ErrorKind::NotADirectory => FsOpError::NotADirectory {
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

const MAX_ATOMIC_TEMP_ATTEMPTS: usize = 16;

fn strong_atomic_temp_name(
    file_name: &std::ffi::OsStr,
    tag: &str,
    display_path: &Path,
) -> Result<PathBuf, FsOpError> {
    let mut random = [0u8; 16];
    rand::rngs::OsRng
        .try_fill_bytes(&mut random)
        .map_err(|error| FsOpError::Io {
            path: display_path.to_path_buf(),
            message: format!("failed to generate atomic staging name: {error}"),
        })?;
    let suffix = u128::from_le_bytes(random);
    Ok(PathBuf::from(format!(
        ".{}.{}-{suffix:032x}",
        file_name.to_string_lossy(),
        tag
    )))
}

/// Atomic-write recipe: exclusively create a strongly-named sibling temp in
/// `target.parent()`, stage `data`, then rename over `target`. `create_new`
/// prevents following or overwriting a colliding regular file or symlink.
/// On any failure the temp is cleaned up and the prior target (if any) is
/// left intact — `tokio::fs::rename` is atomic on the same filesystem, so an
/// interrupted replacement leaves either the full new content or the prior
/// content (never a partial/truncated mix).
async fn atomic_write_bytes(
    target: &Path,
    tag: &str,
    data: &[u8],
    fail_at: AtomicWriteFailPoint,
) -> Result<(), FsOpError> {
    atomic_write_bytes_with_temp_path(target, data, fail_at, || {
        strong_atomic_temp_path(target, tag)
    })
    .await
}

fn strong_atomic_temp_path(target: &Path, tag: &str) -> Result<PathBuf, FsOpError> {
    let parent_dir = target.parent().unwrap_or(target);
    let file_name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let mut random = [0u8; 16];
    rand::rngs::OsRng
        .try_fill_bytes(&mut random)
        .map_err(|error| FsOpError::Io {
            path: target.to_path_buf(),
            message: format!("failed to generate atomic staging name: {error}"),
        })?;
    let suffix = u128::from_le_bytes(random);
    Ok(parent_dir.join(format!(".{file_name}.{tag}-{suffix:032x}")))
}

async fn atomic_write_bytes_with_temp_path<F>(
    target: &Path,
    data: &[u8],
    fail_at: AtomicWriteFailPoint,
    mut next_temp_path: F,
) -> Result<(), FsOpError>
where
    F: FnMut() -> Result<PathBuf, FsOpError> + Send,
{
    let parent_dir = target.parent().unwrap_or(target);
    if let Err(e) = tokio::fs::create_dir_all(parent_dir).await {
        return Err(io_to_fs_error(parent_dir, e));
    }

    let mut allocated = None;
    for _ in 0..MAX_ATOMIC_TEMP_ATTEMPTS {
        let candidate = next_temp_path()?;
        match tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
            .await
        {
            Ok(file) => {
                allocated = Some((candidate, file));
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(io_to_fs_error(target, error)),
        }
    }
    let (temp_path, opened_temp_file) = allocated.ok_or_else(|| FsOpError::Io {
        path: target.to_path_buf(),
        message: format!(
            "failed to allocate atomic staging file after {MAX_ATOMIC_TEMP_ATTEMPTS} collisions"
        ),
    })?;
    let mut guard = super::TempFileGuard::new(temp_path);
    // Declare the open handle after the guard so cancellation drops the handle
    // first, allowing the guard's synchronous Windows cleanup to succeed.
    let mut temp_file = opened_temp_file;

    if let Err(e) = temp_file.write_all(data).await {
        drop(temp_file);
        guard.cleanup().await;
        return Err(io_to_fs_error(target, e));
    }
    if let Err(e) = temp_file.flush().await {
        drop(temp_file);
        guard.cleanup().await;
        return Err(io_to_fs_error(target, e));
    }
    drop(temp_file);

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
        return Err(io_to_fs_error(target, e));
    }
    guard.disarm();
    Ok(())
}

// =========================================================================
// LocalBashOperations (DoD-required local impl) + moved bash machinery
// =========================================================================

/// Maximum number of bytes of merged stdout+stderr the local bash backend
/// retains inline (per stream) and reports through [`BashResult::stdout`] /
/// [`BashResult::stderr`]. Output beyond the cap is truncated: the
/// operation-context diagnostic carries `truncated = true` and the COMPLETE
/// merged output is spilled to a temp file whose path is reported as
/// `full_output`. Mirrors the Phase 11.1 bash cap; moved here from `bash.rs`
// in 15.2 so the bounded capture lives inside `LocalBashOperations::exec`.
pub const MAX_BASH_OUTPUT_BYTES: usize = 64 * 1024; // 64 KiB

/// Local diagnostic code carried in [`BashResult::diagnostics`] to surface the
/// operation-context flags the 15.2 `BashTool` wrapper lifts into the agent
/// `ToolResult`. Distinct from `opi_agent` diagnostic codes: the Operations
/// contract is self-contained (it does not pull in opi-agent tool-result
/// types), and the wrapper remaps this to `CODE_TOOL_EXECUTION_FAILED`. Exactly
/// one such diagnostic is emitted on every in-band [`BashResult`] (Done,
/// TimedOut, Cancelled); spawn/wait failures route through `Err(BashOpError)`.
pub const LOCAL_BASH_OPERATION_DIAGNOSTIC: &str = "opi.operations.bash.operation_context";

/// Local [`BashOperations`] backend. Owns the bash spawn path; the bounded
/// `StreamCapture`, timeout/cancel/`wait` race, and exit/signal extraction all
/// live here, and `BashTool::execute` is a thin caller that maps the
/// [`BashResult`] into the agent `ToolResult`. The L0 process-tree supervision
/// (timeout, cancel, drop, tree kill, bounded drain) attaches to every spawn.
#[derive(Debug, Default)]
pub struct LocalBashOperations {
    #[cfg(test)]
    test_tree_faults: super::process_tree::TestTreeFaults,
}

impl LocalBashOperations {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    fn with_test_tree_faults(test_tree_faults: super::process_tree::TestTreeFaults) -> Self {
        Self { test_tree_faults }
    }
}

/// Compose the exact command used by the production bash spawn path: the
/// shell/flag/command tail with cwd, environment, kill-on-drop, and the L0
/// process-tree configuration applied once.
fn build_bash_command(
    shell: &str,
    flag: &str,
    command: &str,
    cwd: &Path,
    env: &[(String, String)],
) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(shell);
    cmd.arg(flag).arg(command);
    cmd.current_dir(cwd).kill_on_drop(true);
    super::process_tree::configure_tree(&mut cmd);
    cmd.envs(env.iter().map(|(key, value)| (key, value)));
    cmd
}

impl BashOperations for LocalBashOperations {
    fn exec(
        &self,
        request: BashRequest,
    ) -> Pin<Box<dyn Future<Output = Result<BashResult, BashOpError>> + Send>> {
        #[cfg(test)]
        let test_tree_faults = self.test_tree_faults;
        Box::pin(async move {
            let BashRequest {
                command,
                cwd,
                timeout,
                signal,
                env,
                // `backend` is model-strategy routing only; the local backend
                // is always `local`, so drop it here.
                ..
            } = request;
            let shell = if cfg!(windows) { "cmd" } else { "sh" };
            let flag = if cfg!(windows) { "/C" } else { "-c" };
            // L0 degraded-diagnostics accumulator (Phase 15.4): attach/terminate
            // failures append one `CODE_PROCESS_TREE_DEGRADED` ToolDiagnostic each;
            // the result arms fold them in alongside the operation-context diag.
            let mut degraded_diagnostics: Vec<ToolDiagnostic> = Vec::new();

            let mut cmd = build_bash_command(shell, flag, &command, &cwd, &env);
            let mut child = match cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
                Ok(c) => c,
                Err(e) => {
                    return Err(BashOpError::SpawnFailed {
                        message: e.to_string(),
                    }
                    .with_diagnostics(degraded_diagnostics));
                }
            };

            // Policy-neutral L0 supervision (Phase 16 task 16.2): attach the
            // process tree, race wait/timeout/cancel, terminate the whole tree
            // on every branch, and boundedly drain stdout/stderr. Sandbox policy
            // stays in this method (above); the supervision seam is policy
            // neutral and carries only redacted `{layer, reason}` degradations.
            #[cfg(test)]
            let outcome = super::supervision::supervise_with_faults(
                &mut child,
                timeout,
                signal,
                MAX_BASH_OUTPUT_BYTES,
                test_tree_faults,
                false,
            )
            .await;
            #[cfg(not(test))]
            let outcome =
                super::supervision::supervise(&mut child, timeout, signal, MAX_BASH_OUTPUT_BYTES)
                    .await;

            let super::supervision::SupervisionOutcome {
                kind,
                out: mut out_cap,
                err: mut err_cap,
                degradations,
            } = outcome;

            // Map the redacted supervision degradations into the local
            // CODE_PROCESS_TREE_DEGRADED diagnostics, appended as they
            // accumulate during L0 supervision.
            degraded_diagnostics.extend(degradations.iter().map(l0_degraded_diagnostic));

            match kind {
                super::supervision::SupervisionKind::Cancelled { kill_error } => {
                    cleanup_spill(&mut out_cap);
                    cleanup_spill(&mut err_cap);
                    let diag = bash_operation_context_diagnostic(
                        None,
                        true,
                        false,
                        false,
                        None,
                        kill_error.as_ref().map(|e| e.to_string()).as_deref(),
                    );
                    Ok(BashResult {
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                        exit_code: None,
                        signal: None,
                        diagnostics: vec![diag]
                            .into_iter()
                            .chain(degraded_diagnostics.iter().cloned())
                            .collect(),
                    })
                }
                super::supervision::SupervisionKind::TimedOut { kill_error } => {
                    cleanup_spill(&mut out_cap);
                    cleanup_spill(&mut err_cap);
                    let diag = bash_operation_context_diagnostic(
                        None,
                        false,
                        true,
                        false,
                        None,
                        kill_error.as_ref().map(|e| e.to_string()).as_deref(),
                    );
                    Ok(BashResult {
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                        exit_code: None,
                        signal: None,
                        diagnostics: vec![diag]
                            .into_iter()
                            .chain(degraded_diagnostics.iter().cloned())
                            .collect(),
                    })
                }
                super::supervision::SupervisionKind::WaitFailed => Err(BashOpError::WaitFailed {
                    message: "failed to wait for process".to_string(),
                }
                .with_diagnostics(degraded_diagnostics)),
                super::supervision::SupervisionKind::Done(status) => {
                    let exit_code = status.code();
                    #[cfg(unix)]
                    let signal_num = {
                        use std::os::unix::process::ExitStatusExt;
                        status.signal()
                    };
                    #[cfg(not(unix))]
                    let signal_num: Option<i32> = None;

                    let total = out_cap.total.saturating_add(err_cap.total);
                    let truncated = total > MAX_BASH_OUTPUT_BYTES as u64;
                    // On truncation, spill the COMPLETE merged output (stdout
                    // then stderr) to one temp file and report its path; the
                    // per-stream spill files are then removed.
                    let full_output = if truncated {
                        write_merged_full_output(&out_cap, &err_cap)
                    } else {
                        None
                    };
                    cleanup_spill(&mut out_cap);
                    cleanup_spill(&mut err_cap);

                    let stdout = std::mem::take(&mut out_cap.preview);
                    let stderr = std::mem::take(&mut err_cap.preview);
                    let diag = bash_operation_context_diagnostic(
                        exit_code,
                        false,
                        false,
                        truncated,
                        full_output.as_deref(),
                        None,
                    );
                    Ok(BashResult {
                        stdout,
                        stderr,
                        exit_code,
                        signal: signal_num,
                        diagnostics: vec![diag]
                            .into_iter()
                            .chain(degraded_diagnostics.iter().cloned())
                            .collect(),
                    })
                }
            }
        })
    }
}

// The wait/timeout/cancel race (`Control`), the bounded-drain grace
// (`TERMINATED_PIPE_DRAIN_GRACE`), and the drain glue (`spawn_stream_capture`,
// `OwnedCaptureTask`) moved to the policy-neutral `super::supervision` seam in
// Phase 16 task 16.2. The redacted-attach/terminate-to-diagnostic mapper stays
// here alongside the other bash diagnostics.

/// Build the in-band operation-context [`ToolDiagnostic`] (local type) that
/// carries the flags the `BashTool` wrapper needs to reconstruct the agent
/// `ToolResult`: `exit_code`, `cancelled`, `timed_out`, `truncated`,
/// `full_output`, and `kill_error`. `command_included` is always `false`
/// (commands may contain secrets). The wrapper remaps this diagnostic's code to
/// `CODE_TOOL_EXECUTION_FAILED` and pushes it only on an error result, matching
/// the pre-15.2 bash behavior.
///
/// It also carries the local execution-backend report (`guarantee="supervised"`,
/// `placement="host"`) mandated by the Phase 16 Execution Backend contract (spec
/// table line 146). The wrapper lifts these redaction-safe contract fields into
/// `ToolResult::details`, matching the routed twin in `execution/runtime.rs`.
#[allow(clippy::too_many_arguments)]
fn bash_operation_context_diagnostic(
    exit_code: Option<i32>,
    cancelled: bool,
    timed_out: bool,
    truncated: bool,
    full_output: Option<&str>,
    kill_error: Option<&str>,
) -> ToolDiagnostic {
    let message = if cancelled {
        "command cancelled"
    } else if timed_out {
        "command timed out"
    } else {
        "command executed"
    };
    let mut details = serde_json::json!({
        "exit_code": exit_code,
        "cancelled": cancelled,
        "timed_out": timed_out,
        "truncated": truncated,
        "command_included": false,
        // Execution-backend guarantee for the LOCAL identity (Phase 16 task
        // 16.14.2). A compile-time CONSTANT, never sourced from the prepared
        // sandbox state: spec table line 146 assigns `local -> supervised`
        // (placement `host`), and the execution-backend guarantee axis is
        // distinct from the Phase 15 host-sandbox restriction axis (reported via
        // `CODE_PROCESS_TREE_DEGRADED`, not here). The literals mirror the opi-sandbox
        // wire vocabulary origin (`crates/opi-sandbox/src/helper.rs:154-161`) so
        // the two cannot drift; `restricted` belongs to the `opi-sandbox`
        // adapter identity (line 147), never the local path. Local reports only
        // placement+guarantee (no `policy`/`limitations`): a constant
        // `policy="unrestricted"` would be dishonest on Linux-Engaged, where the
        // Phase 15 host sandbox restricts.
        "guarantee": "supervised",
        "placement": "host",
    });
    if let Some(full) = full_output {
        details["full_output"] = serde_json::json!(full);
    }
    if let Some(kill) = kill_error {
        details["kill_error"] = serde_json::json!(kill);
    }
    ToolDiagnostic {
        code: LOCAL_BASH_OPERATION_DIAGNOSTIC.to_string(),
        message: message.to_string(),
        details: Some(details),
    }
}

/// Build the L0 process-tree degraded [`ToolDiagnostic`] (local type) from an
/// [`super::process_tree::AttachError`]. Reuses the stable
/// `CODE_PROCESS_TREE_DEGRADED` literal from [`crate::diagnostics`] so embedders
/// can match it by string, and restricts `details` to the redacted
/// `{layer, reason}` pair — no command text, paths, env, or secrets.
fn l0_degraded_diagnostic(err: &super::process_tree::AttachError) -> ToolDiagnostic {
    ToolDiagnostic {
        code: crate::diagnostics::CODE_PROCESS_TREE_DEGRADED.to_string(),
        message: "subprocess tree lifecycle degraded".to_string(),
        details: Some(serde_json::json!({
            "layer": err.layer,
            "reason": err.reason.as_str(),
        })),
    }
}

/// Bounded capture of one output stream (stdout or stderr). Holds the first
/// `cap` bytes in memory as `preview` and, once the stream exceeds `cap`,
/// spills the COMPLETE stream to a temp file. Memory is bounded to ~`cap` bytes
/// regardless of total output; the spill file is byte-for-byte complete so it
/// can serve as the `full_output` reference.
///
/// The append logic enforces a single-cursor invariant: every input byte routes
/// to exactly one sink (see the moved-from-`bash.rs` stream-capture tests at the
/// bottom of this module).
pub(crate) struct StreamCapture {
    preview: Vec<u8>,
    spill: Option<File>,
    spill_path: Option<PathBuf>,
    spill_failed: bool,
    total: u64,
    cap: usize,
}

impl StreamCapture {
    pub(crate) fn new(cap: usize) -> Self {
        Self {
            preview: Vec::new(),
            spill: None,
            spill_path: None,
            spill_failed: false,
            total: 0,
            cap,
        }
    }

    /// Append one read chunk. Single-cursor invariant.
    pub(crate) fn append(&mut self, chunk: &[u8]) {
        self.total = self.total.saturating_add(chunk.len() as u64);
        if self.spill_failed {
            return;
        }

        if self.preview.len() < self.cap {
            let room = self.cap - self.preview.len();
            let take = chunk.len().min(room);
            self.preview.extend_from_slice(&chunk[..take]);
            let rest = &chunk[take..];
            if !rest.is_empty() && self.write_to_spill(rest).is_err() {
                self.mark_spill_failed();
            }
        } else if self.write_to_spill(chunk).is_err() {
            self.mark_spill_failed();
        }
    }

    #[cfg(test)]
    pub(crate) fn preview(&self) -> &[u8] {
        &self.preview
    }

    #[cfg(test)]
    pub(crate) fn spill_path(&self) -> Option<PathBuf> {
        self.spill_path.clone()
    }

    fn write_to_spill(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.ensure_spill()?;
        self.spill.as_mut().expect("spill ensured").write_all(bytes)
    }

    fn mark_spill_failed(&mut self) {
        self.spill_failed = true;
        cleanup_spill(self);
    }

    /// Lazily create the spill file the first time output overflows. Seeded with
    /// the frozen `cap`-byte preview so the file is the COMPLETE stream.
    fn ensure_spill(&mut self) -> io::Result<()> {
        if self.spill.is_none() {
            let path = bash_output_temp_path();
            let mut file = create_private_temp_file(&path)?;
            file.write_all(&self.preview)?;
            self.spill = Some(file);
            self.spill_path = Some(path);
        }
        Ok(())
    }

    /// The complete stream bytes: spill file contents if it overflowed, else the
    /// in-memory preview (which holds the whole stream because `total <= cap`).
    fn complete_bytes(&self) -> io::Result<Vec<u8>> {
        if self.spill_failed {
            return Err(io::Error::other("bash output spill failed"));
        }
        match &self.spill_path {
            Some(path) => std::fs::read(path),
            None => Ok(self.preview.clone()),
        }
    }
}

impl Drop for StreamCapture {
    fn drop(&mut self) {
        cleanup_spill(self);
    }
}

/// Drop the spill file handle (if any) and best-effort remove the temp file.
fn cleanup_spill(cap: &mut StreamCapture) {
    cap.spill.take();
    if let Some(path) = cap.spill_path.take() {
        let _ = std::fs::remove_file(path);
    }
}

/// Write the COMPLETE merged output (stdout-then-stderr) to one temp file and
/// return its path. Returns `None` only if the file cannot be created/written.
fn write_merged_full_output(out: &StreamCapture, err: &StreamCapture) -> Option<String> {
    let out_bytes = out.complete_bytes().ok()?;
    let err_bytes = err.complete_bytes().ok()?;
    let path = bash_output_temp_path();
    let mut file = create_private_temp_file(&path).ok()?;
    file.write_all(&out_bytes).ok()?;
    file.write_all(&err_bytes).ok()?;
    let _ = file.sync_all();
    drop(file);
    Some(path.to_string_lossy().into_owned())
}

/// Create a private spill file at a caller-chosen temp path.
fn create_private_temp_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

static BASH_OUTPUT_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A unique OS-temp path for a bash full-output spill file. Lives outside the
/// workspace so it never appears in `git status` and is reaped by the OS.
fn bash_output_temp_path() -> PathBuf {
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let counter = BASH_OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("opi-bash-output-{pid}-{nanos}-{counter}.log"))
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
    use super::super::process_tree::TestTreeFaults;
    use super::*;

    fn pipe_holding_descendant_command(dir: &Path, pidfile: &Path) -> String {
        #[cfg(windows)]
        {
            let script = dir.join("spawn-pipe-holder.ps1");
            let body = format!(
                "$p = Start-Process powershell -ArgumentList '-NoProfile','-Command','Start-Sleep -Seconds 60' -NoNewWindow -PassThru\n$p.Id | Set-Content -NoNewline '{}'\nexit 0\n",
                pidfile.to_string_lossy()
            );
            std::fs::write(&script, body).unwrap();
            format!(
                "powershell -NoProfile -ExecutionPolicy Bypass -File {}",
                script.to_string_lossy()
            )
        }
        #[cfg(unix)]
        {
            let _ = dir;
            format!(
                "sh -c 'echo $$ > \"{}\"; sleep 60' & while [ ! -s \"{}\" ]; do sleep 0.01; done; exit 0",
                pidfile.to_string_lossy(),
                pidfile.to_string_lossy()
            )
        }
    }

    async fn read_test_pid(path: &Path) -> u32 {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        loop {
            if let Ok(value) = std::fs::read_to_string(path)
                && let Ok(pid) = value.trim().parse()
            {
                return pid;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "descendant did not record its PID"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    fn test_process_alive(pid: u32) -> bool {
        #[cfg(windows)]
        {
            std::process::Command::new("tasklist")
                .args(["/FI", &format!("PID eq {pid}"), "/NH"])
                .output()
                .map(|output| {
                    let text = String::from_utf8_lossy(&output.stdout);
                    text.contains(&pid.to_string()) && !text.contains("No tasks")
                })
                .unwrap_or(false)
        }
        #[cfg(unix)]
        {
            std::process::Command::new("kill")
                .args(["-0", &pid.to_string()])
                .status()
                .map(|status| status.success())
                .unwrap_or(false)
        }
    }

    fn cleanup_test_process(pid: u32) {
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .output();
        }
        #[cfg(unix)]
        {
            let _ = std::process::Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .output();
        }
    }

    async fn run_pipe_holder_with_fault(faults: TestTreeFaults) -> Result<BashResult, BashOpError> {
        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("descendant.pid");
        let request = BashRequest {
            command: pipe_holding_descendant_command(dir.path(), &pidfile),
            cwd: dir.path().to_path_buf(),
            timeout: Duration::from_secs(10),
            signal: CancellationToken::new(),
            env: vec![],
            backend: None,
        };
        let result = tokio::time::timeout(
            Duration::from_secs(3),
            LocalBashOperations::with_test_tree_faults(faults).exec(request),
        )
        .await
        .expect("pipe drains must be bounded after tree termination");
        if let Ok(text) = std::fs::read_to_string(&pidfile)
            && let Ok(pid) = text.trim().parse::<u32>()
        {
            cleanup_test_process(pid);
        }
        result
    }

    #[tokio::test]
    async fn injected_attach_failure_cannot_hang_on_descendant_held_pipes() {
        let error = run_pipe_holder_with_fault(TestTreeFaults::attach())
            .await
            .expect_err("L0 attachment failure must fail closed");
        assert!(error.diagnostics().iter().any(|diagnostic| {
            diagnostic.code == crate::diagnostics::CODE_PROCESS_TREE_DEGRADED
                && diagnostic
                    .details
                    .as_ref()
                    .and_then(|details| details.get("reason"))
                    .and_then(serde_json::Value::as_str)
                    == Some(crate::diagnostics::SandboxReason::ProcessTreeAttachFailed.as_str())
        }));
    }

    #[tokio::test]
    async fn injected_terminate_failure_cannot_hang_on_descendant_held_pipes() {
        let result = run_pipe_holder_with_fault(TestTreeFaults::terminate())
            .await
            .expect("termination degradation remains an observed result");
        assert_eq!(result.exit_code, Some(0));
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == crate::diagnostics::CODE_PROCESS_TREE_DEGRADED
                && diagnostic
                    .details
                    .as_ref()
                    .and_then(|details| details.get("reason"))
                    .and_then(serde_json::Value::as_str)
                    == Some(
                        crate::diagnostics::SandboxReason::ProcessTreeTerminationFailed.as_str(),
                    )
        }));
    }

    #[tokio::test]
    async fn clean_shell_exit_kills_remaining_tree_and_preserves_zero_status() {
        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("descendant.pid");
        let result = LocalBashOperations::new()
            .exec(BashRequest {
                command: pipe_holding_descendant_command(dir.path(), &pidfile),
                cwd: dir.path().to_path_buf(),
                timeout: Duration::from_secs(10),
                signal: CancellationToken::new(),
                env: vec![],
                backend: None,
            })
            .await
            .unwrap();
        let pid = read_test_pid(&pidfile).await;

        assert_eq!(result.exit_code, Some(0));
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while test_process_alive(pid) && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        if test_process_alive(pid) {
            cleanup_test_process(pid);
            panic!("clean shell exit left descendant PID {pid} alive");
        }
    }

    #[tokio::test]
    async fn local_file_operations_write_then_read_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("f.txt");
        let ops = LocalFileOperations::new(dir.path().to_path_buf());
        ops.write_file(&target, b"hello").await.unwrap();
        let read = ops.read_file(&target).await.unwrap();
        assert_eq!(read, b"hello");
    }

    #[test]
    fn io_mapper_classifies_portable_error_kinds() {
        let path = PathBuf::from("injected");
        let cases = [
            (
                io::ErrorKind::NotFound,
                FsOpError::NotFound { path: path.clone() },
            ),
            (
                io::ErrorKind::PermissionDenied,
                FsOpError::PermissionDenied { path: path.clone() },
            ),
            (
                io::ErrorKind::NotADirectory,
                FsOpError::NotADirectory { path: path.clone() },
            ),
        ];

        for (kind, expected) in cases {
            assert_eq!(io_to_fs_error(&path, io::Error::from(kind)), expected);
        }
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

        let ops = LocalFileOperations::new(dir.path().to_path_buf());
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

        let ops = LocalFileOperations::new(dir.path().to_path_buf());
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

    #[test]
    fn capability_temp_guard_drop_removes_cancelled_stage() {
        let dir = tempfile::tempdir().unwrap();
        let name = PathBuf::from(".target.txt.opi-ops-tmp-cancelled");
        let parent = Dir::open_ambient_dir(dir.path(), ambient_authority()).unwrap();
        parent.create(&name).unwrap();

        drop(CapTempFileGuard::new(parent, name.clone()));

        assert!(
            !dir.path().join(name).exists(),
            "dropping an in-flight atomic stage must leave no production-tagged residue"
        );
    }

    #[tokio::test]
    async fn atomic_write_retries_without_overwriting_regular_file_collision() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        let collision = dir.path().join(".target.txt.collision");
        let available = dir.path().join(".target.txt.available");
        std::fs::write(&collision, "sentinel").unwrap();
        let mut candidates = [collision.clone(), available].into_iter();

        atomic_write_bytes_with_temp_path(
            &target,
            b"replacement",
            AtomicWriteFailPoint::None,
            || {
                Ok(candidates
                    .next()
                    .expect("atomic write requested too many candidates"))
            },
        )
        .await
        .unwrap();

        assert_eq!(std::fs::read_to_string(&target).unwrap(), "replacement");
        assert_eq!(
            std::fs::read_to_string(&collision).unwrap(),
            "sentinel",
            "create_new staging must not overwrite a colliding regular file"
        );
    }

    async fn assert_atomic_write_does_not_follow_symlink_collision(
        dir: &Path,
        sentinel: &Path,
        collision: &Path,
    ) {
        let target = dir.join("target.txt");
        let available = dir.join(".target.txt.available");
        let mut candidates = [collision.to_path_buf(), available].into_iter();

        atomic_write_bytes_with_temp_path(
            &target,
            b"replacement",
            AtomicWriteFailPoint::None,
            || {
                Ok(candidates
                    .next()
                    .expect("atomic write requested too many candidates"))
            },
        )
        .await
        .unwrap();

        assert_eq!(std::fs::read_to_string(&target).unwrap(), "replacement");
        assert_eq!(
            std::fs::read_to_string(sentinel).unwrap(),
            "sentinel",
            "create_new staging must not follow a colliding symlink"
        );
        assert!(
            std::fs::symlink_metadata(collision)
                .unwrap()
                .file_type()
                .is_symlink(),
            "colliding symlink must remain in place"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn atomic_write_retries_without_following_symlink_collision() {
        let dir = tempfile::tempdir().unwrap();
        let sentinel = dir.path().join("sentinel.txt");
        let collision = dir.path().join(".target.txt.collision-link");
        std::fs::write(&sentinel, "sentinel").unwrap();
        std::os::unix::fs::symlink(&sentinel, &collision).unwrap();

        assert_atomic_write_does_not_follow_symlink_collision(dir.path(), &sentinel, &collision)
            .await;
    }

    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "requires Windows symbolic-link privilege; Unix CI exercises this behavior"]
    async fn atomic_write_retries_without_following_symlink_collision_windows_privileged() {
        let dir = tempfile::tempdir().unwrap();
        let sentinel = dir.path().join("sentinel.txt");
        let collision = dir.path().join(".target.txt.collision-link");
        std::fs::write(&sentinel, "sentinel").unwrap();
        std::os::windows::fs::symlink_file(&sentinel, &collision)
            .expect("run ignored test with Windows symbolic-link privilege");

        assert_atomic_write_does_not_follow_symlink_collision(dir.path(), &sentinel, &collision)
            .await;
    }

    #[tokio::test]
    async fn atomic_write_bounds_collision_retries() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        let collision = dir.path().join(".target.txt.collision");
        std::fs::write(&collision, "sentinel").unwrap();
        let mut attempts = 0;

        let error = atomic_write_bytes_with_temp_path(
            &target,
            b"replacement",
            AtomicWriteFailPoint::None,
            || {
                attempts += 1;
                Ok(collision.clone())
            },
        )
        .await
        .unwrap_err();

        assert_eq!(attempts, MAX_ATOMIC_TEMP_ATTEMPTS);
        assert!(matches!(error, FsOpError::Io { .. }));
        assert!(!target.exists());
        assert_eq!(std::fs::read_to_string(collision).unwrap(), "sentinel");
    }

    // ---- StreamCapture single-cursor invariant (moved here from bash.rs in
    //      15.2 when the bounded capture relocated into LocalBashOperations). --

    #[test]
    fn stream_capture_holds_small_stream_in_preview() {
        let mut c = StreamCapture::new(8);
        c.append(b"abc");
        c.append(b"de");
        assert_eq!(c.total, 5);
        assert_eq!(c.preview, b"abcde");
        assert!(c.spill.is_none());
        assert_eq!(c.complete_bytes().unwrap(), b"abcde");
    }

    #[test]
    fn stream_capture_spills_complete_stream_on_overflow() {
        let mut c = StreamCapture::new(4);
        // Single huge chunk (6 bytes, cap 4): preview freezes at 4, spill holds all 6.
        c.append(b"abcdef");
        assert_eq!(c.total, 6);
        assert_eq!(c.preview, b"abcd");
        assert!(c.spill.is_some());
        assert_eq!(c.complete_bytes().unwrap(), b"abcdef");
    }

    #[test]
    fn stream_capture_mid_chunk_overflow_is_byte_complete() {
        let mut c = StreamCapture::new(4);
        c.append(b"ab"); // preview=2, no spill
        c.append(b"cdefgh"); // fills preview to 4 (cd), spills complete (abcdefgh)
        assert_eq!(c.total, 8);
        assert_eq!(c.preview, b"abcd");
        assert_eq!(c.complete_bytes().unwrap(), b"abcdefgh");
    }

    #[test]
    fn stream_capture_exact_boundary_does_not_spill() {
        let mut c = StreamCapture::new(4);
        c.append(b"abcd"); // exactly cap, not overflow
        assert_eq!(c.total, 4);
        assert_eq!(c.preview, b"abcd");
        assert!(c.spill.is_none());
    }

    #[test]
    fn stream_capture_cap_plus_one_overflows() {
        let mut c = StreamCapture::new(4);
        c.append(b"abcde"); // cap+1 -> overflow
        assert_eq!(c.total, 5);
        assert_eq!(c.complete_bytes().unwrap(), b"abcde");
    }

    /// Regression: preview frozen at EXACTLY cap by an earlier fitting chunk (no
    /// crossing remainder), then a LATER chunk overflows. The spill must be
    /// seeded with the frozen preview so complete_bytes() is the full stream.
    #[test]
    fn stream_capture_exact_fit_then_overflow_is_byte_complete() {
        let mut c = StreamCapture::new(4);
        c.append(b"abcd"); // freezes preview at exactly cap, no spill
        c.append(b"e"); // ELSE branch -> first overflow
        assert_eq!(c.total, 5);
        assert_eq!(c.complete_bytes().unwrap(), b"abcde");
    }

    /// Regression (many small chunks): preview reaches cap across several chunks
    /// with no crossing remainder, then a later chunk overflows.
    #[test]
    fn stream_capture_many_small_exact_fit_then_overflow_is_byte_complete() {
        let mut c = StreamCapture::new(4);
        c.append(b"ab");
        c.append(b"cd"); // freezes preview at exactly cap, no spill
        c.append(b"efg"); // ELSE branch -> first overflow
        assert_eq!(c.total, 7);
        assert_eq!(c.complete_bytes().unwrap(), b"abcdefg");
    }

    #[test]
    fn stream_capture_drop_removes_spill_file() {
        let mut capture = StreamCapture::new(4);
        capture.append(b"overflow");
        let spill = capture.spill_path.clone().expect("spill path");
        assert!(spill.is_file());

        drop(capture);

        assert!(!spill.exists(), "dropping a capture must remove its spill");
    }

    #[tokio::test]
    async fn owned_capture_task_drop_aborts_task_and_removes_spill() {
        use super::super::supervision::OwnedCaptureTask;
        use tokio::io::AsyncWriteExt as _;

        let (mut writer, reader) = tokio::io::duplex(64);
        let task = OwnedCaptureTask::new(Some(reader), 4);
        writer.write_all(b"overflow").await.unwrap();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        let spill = loop {
            if let Some(path) = task.spill_path() {
                break path;
            }
            assert!(tokio::time::Instant::now() < deadline, "spill not created");
            tokio::task::yield_now().await;
        };
        assert!(spill.is_file());

        drop(task);
        drop(writer);

        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        while spill.exists() && tokio::time::Instant::now() < deadline {
            tokio::task::yield_now().await;
        }
        assert!(
            !spill.exists(),
            "aborting an owned capture task must remove its spill"
        );
    }
}
