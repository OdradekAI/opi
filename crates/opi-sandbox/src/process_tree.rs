//! L0 subprocess-tree lifecycle primitives for the standalone SDK.
//!
//! Safe API over the OS primitives that terminate a command's subprocess TREE —
//! not just the direct child — on timeout, cancellation, error, or a dropped
//! run. This is an INDEPENDENT RE-IMPLEMENTATION of the Phase 16 task 16.2 L0
//! PATTERN (`opi-coding-agent/src/tool/process_tree.rs`); it does not import
//! that code (the SDK is dependency-neutral) and it carries no command-restriction
//! policy (native confinement is 16.13 / 16.14.1). All `unsafe` FFI lives in
//! THIS module behind safe wrappers; the rest of the crate is
//! `#![forbid(unsafe_code)]`.
//!
//! Unix: the child is made the leader of a new process group at spawn, so the
//! whole tree is signaled later by negating the pid (`libc::kill(-pgid, SIGKILL)`).
//! Windows: the child is assigned to a kill-on-close Job Object whose handle
//! termination (and `Drop`) kills the whole tree. ESRCH (already gone) is a
//! successful no-op; anything else is a redacted [`AttachError`].
//!
//! # Fail-open
//!
//! If tree assignment or termination fails, the caller still kills the direct
//! child via `kill_on_drop(true)` set on the command by the runner, and records
//! the redacted degradation. [`TreeGuard`] and every function here are
//! panic-free; [`TreeGuard::terminate`] is idempotent so the explicit terminal
//! kill and the `Drop` kill never double-signal (Phase 16 task 16.11.1 audit
//! fold: idempotent, panic-free kill on a reaped child).

#[cfg(unix)]
use std::io;
use tokio::process::Command;

/// Stable layer name for the Unix process-group L0 mechanism.
#[cfg(unix)]
const LAYER: &str = "unix-pgroup";
/// Stable layer name for the Windows Job-Object L0 mechanism.
#[cfg(windows)]
const LAYER: &str = "windows-job";
/// Stable layer name on hosts with no L0 mechanism.
#[cfg(not(any(unix, windows)))]
const LAYER: &str = "unsupported";

/// Redacted reason for an L0 attach/termination failure. Carries only a static
/// token — never command text, arguments, environment values, paths, or secrets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeReason {
    /// The child had no resolvable process id (already reaped or unavailable).
    MissingChildProcessId,
    /// Attaching the tree (process group / Job Object) failed.
    AttachFailed,
    /// Terminating the tree failed.
    TerminateFailed,
}

/// Redacted L0 assignment/termination failure: a `{layer, reason}` pair only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachError {
    /// The L0 layer that failed (e.g. `unix-pgroup`, `windows-job`).
    pub layer: &'static str,
    /// The redacted reason.
    pub reason: TreeReason,
}

impl AttachError {
    fn new(layer: &'static str, reason: TreeReason) -> Self {
        Self { layer, reason }
    }

    fn missing_pid() -> Self {
        Self::new(LAYER, TreeReason::MissingChildProcessId)
    }
}

impl std::fmt::Display for AttachError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "L0 attach failed ({}): {:?}", self.layer, self.reason)
    }
}

impl std::error::Error for AttachError {}

/// Outcome of [`TreeGuard::terminate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminationOutcome {
    /// The tree had already been terminated; this call did nothing.
    AlreadyTerminated,
    /// The whole tree was terminated by this call.
    Terminated,
    /// Termination failed at the given layer/reason (fail-open: the direct child
    /// is still killed via `kill_on_drop`).
    Failed(AttachError),
}

/// Configure a [`tokio::process::Command`] for tree containment BEFORE spawn.
///
/// Unix: assigns the child to a brand-new process group (`pgid == child pid`),
/// so the whole tree can be signaled later by negating the pid. Windows: no-op —
/// the Job Object is attached post-spawn in [`TreeGuard::attach`] because it
/// needs the spawned child's pid.
pub fn configure_tree(cmd: &mut Command) {
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }
    #[cfg(not(unix))]
    {
        let _ = cmd;
    }
}

/// Post-spawn L0 tree guard. `Drop` terminates the whole tree best-effort, which
/// is what makes dropping an in-flight run safe (no orphaned descendants). Owned
/// by the runner for the lifetime of the child.
///
/// Construct with [`TreeGuard::attach`] immediately after spawn. Idempotent:
/// [`TreeGuard::terminate`] may be called explicitly (timeout / cancellation) and
/// again on drop without double-signaling.
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
    /// Assignment failed or unsupported; the guard is a no-op so Drop is harmless.
    Disabled,
}

#[cfg(windows)]
#[derive(Debug)]
enum TreeGuardInner {
    /// Child (and descendants that do not break away) are in this kill-on-close
    /// Job Object. Dropping the handle terminates the tree.
    Job(Option<JobGuard>),
    Disabled,
}

#[cfg(not(any(unix, windows)))]
#[derive(Debug)]
enum TreeGuardInner {
    Disabled,
}

impl TreeGuard {
    /// A guard that contains nothing — a fail-open placeholder after an
    /// assignment failure so the runner can still own a guard value.
    pub fn disabled() -> Self {
        Self {
            inner: TreeGuardInner::Disabled,
        }
    }

    /// Attach L0 to an already-spawned child identified by `child_pid`.
    ///
    /// On Unix the pid IS the group id (the child was made a leader by
    /// [`configure_tree`]); on Windows this creates the kill-on-close Job Object
    /// and assigns the child to it. PID zero is always rejected before any OS call.
    pub fn attach(child_pid: u32) -> Result<Self, AttachError> {
        if child_pid == 0 {
            return Err(AttachError::missing_pid());
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

    /// Attach from the optional PID returned by `tokio::process::Child::id`.
    pub fn attach_child(child_pid: Option<u32>) -> Result<Self, AttachError> {
        match child_pid {
            Some(pid) => Self::attach(pid),
            None => Err(AttachError::missing_pid()),
        }
    }

    /// Terminate the whole tree. Idempotent and panic-free; safe to call again
    /// on drop.
    pub fn terminate(&mut self) -> TerminationOutcome {
        #[cfg(unix)]
        {
            match &mut self.inner {
                TreeGuardInner::Group { pgid, terminated } => {
                    if *terminated {
                        return TerminationOutcome::AlreadyTerminated;
                    }
                    // SIGKILL the whole group (negative pid). ESRCH (already
                    // gone) is a successful no-op; anything else is a degrade.
                    let rc = unsafe { libc::kill(-*pgid, libc::SIGKILL) };
                    if rc == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
                        *terminated = true;
                        TerminationOutcome::Terminated
                    } else {
                        TerminationOutcome::Failed(AttachError::new(
                            LAYER,
                            TreeReason::TerminateFailed,
                        ))
                    }
                }
                TreeGuardInner::Disabled => TerminationOutcome::AlreadyTerminated,
            }
        }
        #[cfg(windows)]
        {
            match &mut self.inner {
                TreeGuardInner::Job(job_slot) => terminate_job_slot(job_slot),
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
    /// Kernel handle; 0 == taken/disabled.
    handle: usize,
}

#[cfg(windows)]
fn terminate_job_slot(job_slot: &mut Option<JobGuard>) -> TerminationOutcome {
    let Some(mut job) = job_slot.take() else {
        return TerminationOutcome::AlreadyTerminated;
    };
    match job.terminate() {
        Ok(()) => {
            // Drop closes the handle and fires KILL_ON_JOB_CLOSE.
            TerminationOutcome::Terminated
        }
        Err(error) => {
            // Keep the job armed so TreeGuard::drop retries and JobGuard::drop
            // still enforces kill-on-close.
            *job_slot = Some(job);
            TerminationOutcome::Failed(error)
        }
    }
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
            return Err(AttachError::new(LAYER, TreeReason::AttachFailed));
        }
        // Configure kill-on-close. We deliberately omit the breakaway-OK flag so
        // descendants cannot escape the job.
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
            unsafe { CloseHandle(handle) };
            return Err(AttachError::new(LAYER, TreeReason::AttachFailed));
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

        let proc = unsafe { OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid) };
        if proc.is_null() {
            return Err(AttachError::new(LAYER, TreeReason::AttachFailed));
        }
        let ok = unsafe { AssignProcessToJobObject(self.handle as *mut _, proc) };
        unsafe { CloseHandle(proc) };
        if ok == 0 {
            return Err(AttachError::new(LAYER, TreeReason::AttachFailed));
        }
        Ok(())
    }

    fn terminate(&mut self) -> Result<(), AttachError> {
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;
        if self.handle == 0 {
            return Ok(());
        }
        if unsafe { TerminateJobObject(self.handle as *mut _, 1) } == 0 {
            return Err(AttachError::new(LAYER, TreeReason::TerminateFailed));
        }
        Ok(())
    }
}

#[cfg(windows)]
impl Drop for JobGuard {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        if self.handle != 0 {
            // Closing the last open handle to a KILL_ON_JOB_CLOSE job kills
            // every process still in it.
            unsafe { CloseHandle(self.handle as *mut _) };
            self.handle = 0;
        }
    }
}
