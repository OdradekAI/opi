//! L0 subprocess-tree lifecycle (Phase 15 task 15.4).
//!
//! Safe API over the OS primitives that terminate a bash/adapter subprocess
//! TREE — not just the direct child — on timeout, cancellation, or a dropped
//! exec future. The sibling [`super::operations`] module is
//! `#![forbid(unsafe_code)]`, so every FFI call lives HERE behind a safe
//! wrapper: Unix signals the negative process group via `libc::kill`; Windows
//! assigns the child to a kill-on-close Job Object. Operations and the adapter
//! host call only the safe surface (`configure_tree`, `TreeGuard`).
//!
//! # L0 scope
//!
//! L0 is the ALWAYS-ON baseline and runs for BOTH sandbox modes (`off` and
//! `strict`). It is a lifecycle/correctness mechanism, NOT a security
//! boundary: a dropped exec future, a timeout, or a cancellation must not
//! leave orphaned grandchildren behind. Strict confinement is layered on top
//! by 15.5.x; untrusted code belongs in a container or VM.
//!
//! # Fail-open
//!
//! If tree assignment or termination fails, the caller emits a stable
//! `CODE_SANDBOX_DEGRADED` diagnostic ([`crate::diagnostics`]) and continues at
//! the engaged baseline — the direct child is still killed via the Operations
//! backend. [`TreeGuard`] and every function in this module are panic-free.
//!
//! # Test fault injection
//!
//! Two environment variables force L0 failures so the Operations exec path can
//! be exercised at its diagnostic branches without depending on a real OS
//! fault: `OPI_TEST_L0_ATTACH_FAIL=1` makes [`TreeGuard::attach`] return
//! [`AttachError`]; `OPI_TEST_L0_TERMINATE_FAIL=1` makes
//! [`TreeGuard::terminate`] return [`TerminationOutcome::Failed`]. They are
//! read live (not cached) so a test can scope them under a serializing lock.
//! Never set in production.

use std::io;
use tokio::process::Command;

/// Stable layer name for the Unix process-group L0 mechanism.
#[cfg(unix)]
const LAYER: &str = "unix-pgroup";
/// Stable layer name for the Windows Job-Object L0 mechanism.
#[cfg(windows)]
const LAYER: &str = "windows-job";

#[cfg(not(any(unix, windows)))]
const LAYER: &str = "unsupported";

const ENV_ATTACH_FAIL: &str = "OPI_TEST_L0_ATTACH_FAIL";
const ENV_TERMINATE_FAIL: &str = "OPI_TEST_L0_TERMINATE_FAIL";

fn env_flag(name: &str) -> bool {
    std::env::var(name).as_deref() == Ok("1")
}

/// Configure a [`Command`] for tree containment BEFORE spawn.
///
/// Unix: assigns the child to a brand-new process group (`pgid == child pid`)
/// via tokio's safe `process_group(0)` wrapper over `setpgid`, so the whole
/// tree can be signaled later by negating the pid. Windows: no-op — Job-Object
/// assignment happens post-spawn in [`TreeGuard::attach`] because the job needs
/// the spawned child's pid.
pub fn configure_tree(cmd: &mut Command) {
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }
    #[cfg(not(unix))]
    {
        // No pre-spawn configuration; the Job Object is attached after spawn.
        let _ = cmd;
    }
}

/// Redacted L0 assignment/termination failure. Carries only a `{layer, reason}`
/// pair — no command text, paths, env, or secrets — so it can flow unchanged
/// into the stable `CODE_SANDBOX_DEGRADED` diagnostic.
#[derive(Debug, Clone)]
pub struct AttachError {
    pub layer: &'static str,
    pub reason: String,
}

impl AttachError {
    fn new(layer: &'static str, reason: impl Into<String>) -> Self {
        Self {
            layer,
            reason: reason.into(),
        }
    }
}

impl std::fmt::Display for AttachError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "L0 attach failed ({}): {}", self.layer, self.reason)
    }
}

impl std::error::Error for AttachError {}

/// Outcome of a [`TreeGuard::terminate`] call, used by the Operations exec path
/// to surface the right diagnostic without inspecting the guard's interior.
#[derive(Debug, Clone)]
pub enum TerminationOutcome {
    /// The tree had already been terminated earlier; this call did nothing.
    AlreadyTerminated,
    /// The whole tree was terminated by this call.
    Terminated,
    /// Termination failed at the given layer/reason. Fail-open: the caller
    /// still kills the direct child via the Operations backend and records a
    /// `CODE_SANDBOX_DEGRADED` diagnostic.
    Failed(AttachError),
}

/// Post-spawn L0 tree guard. Drop terminates the whole tree best-effort, which
/// is what makes dropping an in-flight exec future safe (no orphaned
/// grandchildren). Owned by the Operations exec scope and by the adapter host
/// for the lifetime of the child.
///
/// Construct with [`TreeGuard::attach`] immediately after spawn. The guard is
/// idempotent: [`TreeGuard::terminate`] may be called explicitly (timeout /
/// cancellation) and again on drop without double-signaling.
#[derive(Debug)]
pub struct TreeGuard {
    inner: TreeGuardInner,
}

#[cfg(unix)]
#[derive(Debug)]
enum TreeGuardInner {
    /// Child is the leader of process group `pgid`; signaling `-pgid` reaches
    /// the whole tree.
    Group { pgid: i32, terminated: bool },
    /// Assignment failed (or unsupported host); the guard is a no-op so Drop is
    /// harmless. The caller already emitted a degraded diagnostic.
    Disabled,
}

#[cfg(windows)]
#[derive(Debug)]
enum TreeGuardInner {
    /// Child (and all descendants that do not break away) are in this kill-on-
    /// close Job Object. Dropping the handle terminates the tree.
    Job(Option<JobGuard>),
    Disabled,
}

#[cfg(not(any(unix, windows)))]
#[derive(Debug)]
enum TreeGuardInner {
    Disabled,
}

impl TreeGuard {
    /// A guard that contains nothing. Useful as a fail-open placeholder after
    /// an assignment failure so the exec scope can still own a guard value.
    pub fn disabled() -> Self {
        Self {
            inner: TreeGuardInner::Disabled,
        }
    }

    /// Consume the L0 containment so the next [`terminate`] / `Drop` is a
    /// no-op. Called on a clean child exit: the command finished and the tree
    /// must NOT be torn down, matching pre-15.4 behavior for backgrounded
    /// survivors. (On a clean exit the group leader is already gone, so leaving
    /// the guard armed would only ever be a redundant ESRCH no-op anyway; this
    /// keeps the intent explicit.)
    ///
    /// [`terminate`]: TreeGuard::terminate
    pub fn disarm(&mut self) {
        self.inner = TreeGuardInner::Disabled;
    }

    /// Attach L0 to an already-spawned child identified by `child_pid`.
    ///
    /// On the Unix group mechanism the pid IS the group id (the child was made
    /// a leader by [`configure_tree`]), so no kernel call is needed here and
    /// this only fails under test injection. On Windows this creates the
    /// kill-on-close Job Object and assigns the child to it.
    pub fn attach(child_pid: u32) -> Result<Self, AttachError> {
        if env_flag(ENV_ATTACH_FAIL) {
            return Err(AttachError::new(LAYER, "injected attach failure"));
        }
        #[cfg(unix)]
        {
            Ok(Self {
                inner: TreeGuardInner::Group {
                    pgid: child_pid as i32,
                    terminated: false,
                },
            })
        }
        #[cfg(windows)]
        {
            let job = JobGuard::new()?;
            job.assign(child_pid)?;
            Ok(Self {
                inner: TreeGuardInner::Job(Some(job)),
            })
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = child_pid;
            Ok(Self::disabled())
        }
    }

    /// Terminate the whole tree. Idempotent and panic-free. The caller surfaces
    /// the returned [`TerminationOutcome`] as a diagnostic when it is `Failed`.
    pub fn terminate(&mut self) -> TerminationOutcome {
        #[cfg(unix)]
        {
            match &mut self.inner {
                TreeGuardInner::Group { pgid, terminated } => {
                    if *terminated {
                        return TerminationOutcome::AlreadyTerminated;
                    }
                    *terminated = true;
                    if env_flag(ENV_TERMINATE_FAIL) {
                        return TerminationOutcome::Failed(AttachError::new(
                            LAYER,
                            "injected terminate failure",
                        ));
                    }
                    // SIGKILL the whole group (negative pid). ESRCH (already
                    // gone) is a successful no-op; anything else is a degrade.
                    let rc = unsafe { libc::kill(-*pgid, libc::SIGKILL) };
                    if rc == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
                        TerminationOutcome::Terminated
                    } else {
                        TerminationOutcome::Failed(AttachError::new(
                            LAYER,
                            format!("kill process group failed: {}", io::Error::last_os_error()),
                        ))
                    }
                }
                TreeGuardInner::Disabled => TerminationOutcome::AlreadyTerminated,
            }
        }
        #[cfg(windows)]
        {
            match &mut self.inner {
                TreeGuardInner::Job(job_slot) => match job_slot.take() {
                    Some(mut job) => {
                        if env_flag(ENV_TERMINATE_FAIL) {
                            // Keep the handle alive so Drop still closes it
                            // (kill-on-close) even though we report failure.
                            *job_slot = Some(job);
                            return TerminationOutcome::Failed(AttachError::new(
                                LAYER,
                                "injected terminate failure",
                            ));
                        }
                        job.terminate();
                        // Drop closes the handle and fires KILL_ON_JOB_CLOSE.
                        TerminationOutcome::Terminated
                    }
                    None => TerminationOutcome::AlreadyTerminated,
                },
                TreeGuardInner::Disabled => TerminationOutcome::AlreadyTerminated,
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            TerminationOutcome::AlreadyTerminated
        }
    }
}

impl Drop for TreeGuard {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}

// =========================================================================
// Windows Job-Object backend
// =========================================================================

#[cfg(windows)]
#[derive(Debug)]
struct JobGuard {
    handle: usize, // 0 == taken/disabled
}

#[cfg(windows)]
impl JobGuard {
    fn new() -> Result<Self, AttachError> {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::JobObjects::{
            CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        };

        // Anonymous job: no security attributes, no name.
        let handle = unsafe { CreateJobObjectW(core::ptr::null(), core::ptr::null()) };
        if handle.is_null() {
            return Err(AttachError::new(
                LAYER,
                format!("CreateJobObjectW failed: {}", io::Error::last_os_error()),
            ));
        }
        // Configure kill-on-close. We deliberately omit the breakaway-OK flag
        // so descendants cannot escape the job (the LimitFlags below carries
        // only the kill-on-close bit).
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { core::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let info_ptr = &info as *const _ as *const core::ffi::c_void;
        let info_len = core::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32;
        let ok = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                info_ptr,
                info_len,
            )
        };
        if ok == 0 {
            let err = io::Error::last_os_error();
            unsafe { CloseHandle(handle) };
            return Err(AttachError::new(
                LAYER,
                format!("SetInformationJobObject failed: {err}"),
            ));
        }
        Ok(Self {
            handle: handle as usize,
        })
    }

    fn assign(&self, pid: u32) -> Result<(), AttachError> {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
        };

        // We only need enough access to add the process to a job. Opening our
        // own child does not require elevation.
        let proc = unsafe { OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid) };
        if proc.is_null() {
            return Err(AttachError::new(
                LAYER,
                format!("OpenProcess({pid}) failed: {}", io::Error::last_os_error()),
            ));
        }
        let ok = unsafe { AssignProcessToJobObject(self.handle as *mut _, proc) };
        unsafe { CloseHandle(proc) };
        if ok == 0 {
            return Err(AttachError::new(
                LAYER,
                format!(
                    "AssignProcessToJobObject failed: {}",
                    io::Error::last_os_error()
                ),
            ));
        }
        Ok(())
    }

    fn terminate(&mut self) {
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;
        if self.handle != 0 {
            // TerminateJobObject kills every process in the job immediately.
            unsafe { TerminateJobObject(self.handle as *mut _, 1) };
        }
    }
}

#[cfg(windows)]
impl Drop for JobGuard {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        if self.handle != 0 {
            // Closing the last open handle to a KILL_ON_JOB_CLOSE job kills
            // every process still in it. This is the safety net for a dropped
            // exec future / dropped adapter host.
            unsafe { CloseHandle(self.handle as *mut _) };
            self.handle = 0;
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 15 task 15.5.3 — Linux strict confinement FFI
// ---------------------------------------------------------------------------
//
// `sandbox.rs` is `#![forbid(unsafe_code)]` and that forbid propagates to its
// `linux` submodule, so the two audited `unsafe` helpers required by the Linux
// strict backend live HERE. `process_tree` is the documented home for
// spawn-path FFI (its module doc: "every FFI call lives HERE behind a safe
// wrapper"); `sandbox::linux` builds the confinement plan on the parent side
// (safe landlock/seccomp APIs) and calls these helpers to perform the kernel
// calls. `sandbox.rs` and `tool/operations.rs` stay `#![forbid(unsafe_code)]`.

#[cfg(target_os = "linux")]
use std::sync::Arc;

#[cfg(target_os = "linux")]
use landlock::{ABI, RulesetCreated};

#[cfg(target_os = "linux")]
use seccompiler::BpfProgram;

/// Query the kernel's **observed** Landlock ABI (read-only; no confinement).
/// Replicates landlock 0.4.5's private `LandlockStatus::current()` probe — the
/// crate deliberately does not expose it, but the policy resolver must report
/// per-layer availability before spawn. `landlock_create_ruleset(NULL, 0,
/// LANDLOCK_CREATE_RULESET_VERSION)` returns the supported ABI (1..=7) or a
/// negative errno when Landlock is absent/disabled.
#[cfg(target_os = "linux")]
pub fn observed_landlock_abi() -> ABI {
    const LANDLOCK_CREATE_RULESET_VERSION: u32 = 1;
    // SAFETY: read-only capability query. A null attribute pointer, size 0, and
    // the VERSION flag direct the kernel to return the supported ABI integer
    // without creating a ruleset or mutating state. No fd is produced. The
    // landlock crate performs the identical call internally.
    let v = unsafe {
        libc::syscall(
            libc::SYS_landlock_create_ruleset,
            std::ptr::null::<libc::c_void>(),
            0usize,
            LANDLOCK_CREATE_RULESET_VERSION,
        )
    };
    if v < 0 {
        ABI::Unsupported
    } else {
        ABI::from(v as i32)
    }
}

/// Map a `StableErrno` to its raw errno value for `io::Error::from_raw_os_error`.
#[cfg(target_os = "linux")]
fn stable_errno_raw(e: crate::sandbox::linux::StableErrno) -> i32 {
    use crate::sandbox::linux::StableErrno;
    match e {
        StableErrno::Prctl(x) | StableErrno::Seccomp(x) => x,
        StableErrno::EmptyFilter | StableErrno::Backend => libc::EINVAL,
    }
}

/// The one audited child-setup helper: register a `pre_exec` hook on `cmd`
/// (built in the parent) that installs the seccomp deny-overlay and restricts
/// the child via Landlock. Only the std `pre_exec` registration itself is
/// `unsafe`; seccomp and Landlock application are delegated to library APIs.
#[cfg(target_os = "linux")]
pub fn install_child_confinement(
    cmd: &mut tokio::process::Command,
    bpf: Arc<BpfProgram>,
    ruleset: Option<RulesetCreated>,
) {
    use std::os::unix::process::CommandExt;
    let mut ruleset = ruleset;
    // SAFETY: `pre_exec` runs the supplied closure in the child process after
    // fork but before execve, in an async-signal-safe context. The closure calls
    // only async-signal-safe operations: seccomp filter installation (prctl +
    // the seccomp syscall, inside seccompiler::apply_filter) and Landlock
    // `restrict_self` (the landlock_restrict_self syscall, inside the landlock
    // crate). No locking and no heap allocation occur on the success path; the
    // error path may format a message, which is acceptable because the command
    // is already failing. `bpf` (`Arc<Vec<sock_filter>>`) and `ruleset`
    // (fd-bearing `RulesetCreated`) are both `Send + Sync + 'static`, satisfying
    // `pre_exec`'s closure bounds.
    let _ = unsafe {
        cmd.as_std_mut().pre_exec(move || {
            if let Err(e) = crate::sandbox::linux::apply_raw_filter(bpf.as_ref()) {
                return Err(std::io::Error::from_raw_os_error(stable_errno_raw(e)));
            }
            if let Some(rs) = ruleset.take()
                && let Err(e) = rs.restrict_self()
            {
                return Err(std::io::Error::other(format!(
                    "landlock restrict_self: {e:?}"
                )));
            }
            Ok(())
        })
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The disabled guard is a no-op: terminate reports AlreadyTerminated and
    /// Drop is harmless. Verified on every host (no OS calls).
    #[test]
    fn disabled_guard_terminate_is_already_terminated() {
        let mut g = TreeGuard::disabled();
        match g.terminate() {
            TerminationOutcome::AlreadyTerminated => {}
            other => panic!("disabled guard should be no-op, got {other:?}"),
        }
    }

    /// AttachError carries only the redacted layer/reason pair.
    #[test]
    fn attach_error_redacts_to_layer_and_reason() {
        let err = AttachError::new("windows-job", "boom");
        assert_eq!(err.layer, "windows-job");
        assert_eq!(err.reason, "boom");
        assert!(err.to_string().contains("windows-job"));
        assert!(err.to_string().contains("boom"));
    }

    /// Test injection: with the env flag set, attach reports a failure carrying
    /// the active layer. Serialized by the test process; the flag is cleared so
    /// it cannot leak to sibling tests in this binary.
    #[test]
    fn attach_failure_injection_via_env() {
        // Edition 2024: env mutation is unsafe (not thread-safe).
        unsafe { std::env::set_var(ENV_ATTACH_FAIL, "1") };
        let err = TreeGuard::attach(424242).expect_err("env flag must force attach failure");
        unsafe { std::env::remove_var(ENV_ATTACH_FAIL) };
        assert_eq!(err.layer, LAYER);
        assert!(err.reason.contains("injected"));
    }
}
