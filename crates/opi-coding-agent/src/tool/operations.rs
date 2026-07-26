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
use std::io::{self, Write as IoWrite};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use tokio_util::sync::CancellationToken;

use super::process_tree::{TerminationOutcome, TreeGuard};
use crate::sandbox::{PreparedSandbox, StrictOutcome, TemporaryGap};

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

    /// `--sandbox strict` with `require = true` (or `[sandbox] require = true`)
    /// could not engage a requested layer, so the command is refused BEFORE any
    /// spawn side effect (Phase 15.5.1 fail-closed path). The message is the
    /// redacted layer summary from [`crate::sandbox`] — no command/env/paths.
    #[error("sandbox required but unavailable: {message}")]
    SandboxUnavailable { message: String },

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

/// Local [`BashOperations`] backend. Owns the bash spawn path the Phase 15 T4
/// sandbox attaches to (T4 lives INSIDE this impl, not as a wrapper). The
/// bounded `StreamCapture`, timeout/cancel/`wait` race, and exit/signal
/// extraction all live here; `BashTool::execute` is a thin caller that maps the
/// [`BashResult`] into the agent `ToolResult`. Carries the resolved sandbox
/// policy (Phase 15.5.1); the default [`PreparedSandbox::Off`] runs the L0-only
/// baseline used by every pre-15.5.1 caller.
#[derive(Debug, Default)]
pub struct LocalBashOperations {
    prepared: PreparedSandbox,
}

impl LocalBashOperations {
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct with a resolved sandbox policy (Phase 15.5.1). Used by
    /// [`crate::harness::CodingHarness::build_tools`], which resolves the policy
    /// once at startup via [`crate::sandbox::prepare_production`] so per-command
    /// exec enforces the decision and the permanent startup diagnostics surface
    /// once. `new()` keeps the [`PreparedSandbox::Off`] default for direct/test
    /// callers.
    pub fn with_prepared(prepared: PreparedSandbox) -> Self {
        Self { prepared }
    }
}

impl BashOperations for LocalBashOperations {
    fn exec(
        &self,
        request: BashRequest,
    ) -> Pin<Box<dyn Future<Output = Result<BashResult, BashOpError>> + Send>> {
        let prepared = self.prepared.clone();
        Box::pin(async move {
            let BashRequest {
                command,
                cwd,
                timeout,
                signal,
                env,
            } = request;
            let shell = if cfg!(windows) { "cmd" } else { "sh" };
            let flag = if cfg!(windows) { "/C" } else { "-c" };
            // L0 degraded-diagnostics accumulator (Phase 15.4): attach/terminate
            // failures append one `CODE_SANDBOX_DEGRADED` ToolDiagnostic each;
            // the result arms fold them in alongside the operation-context diag.
            let mut degraded_diagnostics: Vec<ToolDiagnostic> = Vec::new();

            // Phase 15.5.1: enforce the resolved sandbox policy BEFORE any spawn
            // side effect. FailClosed returns a named error here (require=true +
            // an unavailable layer); FailOpen records one per-command degraded
            // diagnostic per TEMPORARY gap (permanent gaps were already emitted
            // once at startup and are deliberately not repeated per command);
            // Off/Engaged proceed to the L0 spawn below.
            //
            // Phase 15.5.3: an Engaged Linux decision carries a parent-built
            // confinement plan; it is applied to the spawn `Command` between the
            // L0 tree setup and `spawn()`.
            let confinement: Option<&crate::sandbox::Confinement> = match &prepared {
                PreparedSandbox::Off => None,
                PreparedSandbox::Strict(decision) => match &decision.outcome {
                    StrictOutcome::Engaged => decision.confinement.as_ref(),
                    StrictOutcome::FailOpen {
                        per_command_temporary,
                    } => {
                        for gap in per_command_temporary {
                            degraded_diagnostics.push(temporary_gap_diagnostic(gap));
                        }
                        None
                    }
                    StrictOutcome::FailClosed { reason } => {
                        return Err(BashOpError::SandboxUnavailable {
                            message: reason.clone(),
                        });
                    }
                },
            };

            let mut cmd = tokio::process::Command::new(shell);
            cmd.arg(flag)
                .arg(&command)
                .current_dir(&cwd)
                .kill_on_drop(true);
            // Phase 15.4 L0: put the child in a tree-containment scope so the
            // whole subprocess tree (not just the direct child) is torn down on
            // timeout, cancellation, or a dropped exec future. Pre-spawn the
            // child enters a new process group (Unix); the Windows Job Object is
            // attached just after spawn. All FFI lives in `super::process_tree`;
            // this module stays #![forbid(unsafe_code)].
            super::process_tree::configure_tree(&mut cmd);
            // Phase 15.5.3: apply the strict confinement plan (Linux seccomp +
            // Landlock `pre_exec` hook) when the decision engaged. Safe call: the
            // confinement closure registers the audited `pre_exec` helper; the
            // `unsafe` lives inside `sandbox/linux.rs`, not here.
            if let Some(confinement) = &confinement {
                confinement.apply(&mut cmd);
            }
            // env augments the inherited environment on top of what the child
            // already receives; empty in current usage.
            for (key, value) in &env {
                cmd.env(key, value);
            }
            // Phase 15.5.4: macOS launcher confinement. `sandbox-exec` IS the
            // helper, so it must be the spawn program with its prefix args
            // (`-p <profile>`) before the original shell invocation. A `pre_exec`
            // hook cannot change the program, so when the confinement carries a
            // launcher we rebuild the Command from scratch and re-apply the same
            // L0 process_group(0) + kill_on_drop + cwd + env configured above.
            // `apply` is a no-op for a launcher plan, so the Linux pre_exec path
            // (which returns `None` from `launcher_prefix`) is untouched.
            if let Some(confinement) = &confinement
                && let Some((launcher, prefix_args)) = confinement.launcher_prefix()
            {
                let mut restarted = tokio::process::Command::new(launcher);
                for a in prefix_args {
                    restarted.arg(a);
                }
                restarted
                    .arg(shell)
                    .arg(flag)
                    .arg(&command)
                    .current_dir(&cwd)
                    .kill_on_drop(true);
                super::process_tree::configure_tree(&mut restarted);
                for (key, value) in &env {
                    restarted.env(key, value);
                }
                cmd = restarted;
            }
            let mut child = match cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
                Ok(c) => c,
                Err(e) => {
                    return Err(BashOpError::SpawnFailed {
                        message: e.to_string(),
                    });
                }
            };

            // Attach L0 to the spawned child. Fail-open: on failure we keep a
            // disabled guard (the direct child is still killed via kill_on_drop
            // + child.kill on timeout/cancel) and record one degraded diagnostic.
            let mut l0_tree = match TreeGuard::attach(child.id().unwrap_or(0)) {
                Ok(guard) => guard,
                Err(e) => {
                    degraded_diagnostics.push(l0_degraded_diagnostic(&e));
                    TreeGuard::disabled()
                }
            };

            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            let timeout_future = tokio::time::sleep(timeout);
            let cancel_future = signal.cancelled();
            tokio::pin!(timeout_future);
            tokio::pin!(cancel_future);

            let mut out_cap = StreamCapture::new(MAX_BASH_OUTPUT_BYTES);
            let mut err_cap = StreamCapture::new(MAX_BASH_OUTPUT_BYTES);

            // Drain stdout/stderr concurrently with the wait/timeout/cancel race
            // to avoid the stdout-then-stderr pipe deadlock. On timeout/cancel
            // the child is killed, pipes hit EOF, drains finish, and captures
            // are discarded (with spill files cleaned up).
            let drain_out = async {
                if let Some(mut s) = stdout {
                    let mut buf = [0u8; 8192];
                    loop {
                        match s.read(&mut buf).await {
                            Ok(0) | Err(_) => break,
                            Ok(n) => out_cap.append(&buf[..n]),
                        }
                    }
                }
            };
            let drain_err = async {
                if let Some(mut s) = stderr {
                    let mut buf = [0u8; 8192];
                    loop {
                        match s.read(&mut buf).await {
                            Ok(0) | Err(_) => break,
                            Ok(n) => err_cap.append(&buf[..n]),
                        }
                    }
                }
            };
            let control = async {
                tokio::select! {
                    biased;
                    _ = &mut cancel_future => {
                        let kill_error = child.kill().await.err().map(|e| e.to_string());
                        // L0: terminate the whole tree; surface a degrade if it
                        // fails (the direct-child kill above already ran).
                        let term_diag = match l0_tree.terminate() {
                            TerminationOutcome::Failed(e) => Some(l0_degraded_diagnostic(&e)),
                            _ => None,
                        };
                        (Control::Cancelled { kill_error }, term_diag)
                    }
                    _ = &mut timeout_future => {
                        let kill_error = child.kill().await.err().map(|e| e.to_string());
                        let term_diag = match l0_tree.terminate() {
                            TerminationOutcome::Failed(e) => Some(l0_degraded_diagnostic(&e)),
                            _ => None,
                        };
                        (Control::TimedOut { kill_error }, term_diag)
                    }
                    status = child.wait() => match status {
                        Ok(s) => {
                            // Clean exit: disarm L0 so the tree is NOT torn down
                            // (matches pre-15.4 behavior for backgrounded survivors).
                            l0_tree.disarm();
                            (Control::Done(s), None)
                        }
                        Err(_) => {
                            let term_diag = match l0_tree.terminate() {
                                TerminationOutcome::Failed(e) => Some(l0_degraded_diagnostic(&e)),
                                _ => None,
                            };
                            (Control::WaitFailed, term_diag)
                        }
                    },
                }
            };

            let (_, _, (ctrl, term_diag)) = tokio::join!(drain_out, drain_err, control);
            if let Some(d) = term_diag {
                degraded_diagnostics.push(d);
            }

            match ctrl {
                Control::Cancelled { kill_error } => {
                    cleanup_spill(&mut out_cap);
                    cleanup_spill(&mut err_cap);
                    let diag = bash_operation_context_diagnostic(
                        None,
                        true,
                        false,
                        false,
                        None,
                        kill_error.as_deref(),
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
                Control::TimedOut { kill_error } => {
                    cleanup_spill(&mut out_cap);
                    cleanup_spill(&mut err_cap);
                    let diag = bash_operation_context_diagnostic(
                        None,
                        false,
                        true,
                        false,
                        None,
                        kill_error.as_deref(),
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
                Control::WaitFailed => Err(BashOpError::WaitFailed {
                    message: "failed to wait for process".to_string(),
                }),
                Control::Done(status) => {
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

                    let stdout = out_cap.preview;
                    let stderr = err_cap.preview;
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

/// Which control branch won the wait/timeout/cancel race.
enum Control {
    Done(std::process::ExitStatus),
    TimedOut { kill_error: Option<String> },
    Cancelled { kill_error: Option<String> },
    WaitFailed,
}

/// Build the in-band operation-context [`ToolDiagnostic`] (local type) that
/// carries the flags the `BashTool` wrapper needs to reconstruct the agent
/// `ToolResult`: `exit_code`, `cancelled`, `timed_out`, `truncated`,
/// `full_output`, and `kill_error`. `command_included` is always `false`
/// (commands may contain secrets). The wrapper remaps this diagnostic's code to
/// `CODE_TOOL_EXECUTION_FAILED` and pushes it only on an error result, matching
/// the pre-15.2 bash behavior.
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

/// Build the L0 degraded [`ToolDiagnostic`] (local type) from an
/// [`super::process_tree::AttachError`]. Reuses the stable
/// `CODE_SANDBOX_DEGRADED` literal from [`crate::diagnostics`] so embedders can
/// match it by string, and restricts `details` to the redacted `{layer, reason}`
/// pair — no command text, paths, env, or secrets.
fn l0_degraded_diagnostic(err: &super::process_tree::AttachError) -> ToolDiagnostic {
    ToolDiagnostic {
        code: crate::diagnostics::CODE_SANDBOX_DEGRADED.to_string(),
        message: "subprocess tree lifecycle degraded".to_string(),
        details: Some(serde_json::json!({
            "layer": err.layer,
            "reason": err.reason,
        })),
    }
}

/// Build a per-command strict-sandbox degraded [`ToolDiagnostic`] (local type)
/// for a Phase 15.5.1 fail-open temporary gap. Reuses the stable
/// `CODE_SANDBOX_DEGRADED` literal so embedders match it by string, and
/// restricts `details` to the redacted `{ layer, reason }` pair. Permanent gaps
/// do NOT use this path: they surface once at startup via the builder's
/// `startup_diagnostics` channel as `CODE_SANDBOX_UNAVAILABLE`.
fn temporary_gap_diagnostic(gap: &TemporaryGap) -> ToolDiagnostic {
    ToolDiagnostic {
        code: crate::diagnostics::CODE_SANDBOX_DEGRADED.to_string(),
        message: "sandbox layer degraded".to_string(),
        details: Some(serde_json::json!({
            "layer": gap.layer.as_str(),
            "reason": gap.reason,
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
struct StreamCapture {
    preview: Vec<u8>,
    spill: Option<File>,
    spill_path: Option<PathBuf>,
    spill_failed: bool,
    total: u64,
    cap: usize,
}

impl StreamCapture {
    fn new(cap: usize) -> Self {
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
    fn append(&mut self, chunk: &[u8]) {
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
}
