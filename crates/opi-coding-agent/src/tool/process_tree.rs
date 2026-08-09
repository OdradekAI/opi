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
//! L0 is the ALWAYS-ON baseline. It is a lifecycle/correctness mechanism, NOT
//! a security boundary: a dropped exec future, a timeout, or a cancellation
//! must not leave orphaned grandchildren behind. Native restriction (strict
//! confinement) was removed from core by 16.16.1; untrusted code belongs in a
//! container or VM.
//!
//! # Failure policy
//!
//! Tree assignment is a hard precondition: callers kill and reap the child and
//! return a failed execution instead of running it uncontained. If later tree
//! termination fails, the caller emits a stable `CODE_PROCESS_TREE_DEGRADED`
//! diagnostic ([`crate::diagnostics`]) and does not claim confirmed cleanup at
//! the engaged baseline — the direct child is still killed via the Operations
//! backend. [`TreeGuard`] and every function in this module are panic-free.
//!
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

/// Configure a [`Command`] for tree containment BEFORE spawn.
///
/// Unix: assigns the child to a brand-new process group (`pgid == child pid`)
/// via tokio's safe `process_group(0)` wrapper over `setpgid`, so the whole
/// tree can be signaled later by negating the pid. Windows creates the child
/// suspended so [`TreeGuard::attach`] can assign its Job Object before any
/// child code runs; the caller resumes it only after successful assignment.
pub fn configure_tree(cmd: &mut Command) {
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        use windows_sys::Win32::System::Threading::CREATE_SUSPENDED;
        cmd.as_std_mut().creation_flags(CREATE_SUSPENDED);
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = cmd;
    }
}

#[cfg(windows)]
pub fn resume_child(child_pid: u32) -> Result<(), AttachError> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
    };
    use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(AttachError::new(
            LAYER,
            crate::diagnostics::SandboxReason::ProcessTreeAttachFailed,
        ));
    }
    let mut entry: THREADENTRY32 = unsafe { core::mem::zeroed() };
    entry.dwSize = core::mem::size_of::<THREADENTRY32>() as u32;
    let mut found = false;
    let mut ok = unsafe { Thread32First(snapshot, &mut entry) } != 0;
    while ok {
        if entry.th32OwnerProcessID == child_pid {
            let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
            if thread.is_null() || unsafe { ResumeThread(thread) } == u32::MAX {
                if !thread.is_null() {
                    unsafe { CloseHandle(thread) };
                }
                unsafe { CloseHandle(snapshot) };
                return Err(AttachError::new(
                    LAYER,
                    crate::diagnostics::SandboxReason::ProcessTreeAttachFailed,
                ));
            }
            unsafe { CloseHandle(thread) };
            found = true;
        }
        ok = unsafe { Thread32Next(snapshot, &mut entry) } != 0;
    }
    unsafe { CloseHandle(snapshot) };
    found.then_some(()).ok_or_else(|| {
        AttachError::new(
            LAYER,
            crate::diagnostics::SandboxReason::ProcessTreeAttachFailed,
        )
    })
}

/// Redacted L0 assignment/termination failure. Carries only a `{layer, reason}`
/// pair — no command text, paths, env, or secrets — so it can flow unchanged
/// into the stable `CODE_PROCESS_TREE_DEGRADED` diagnostic.
#[derive(Debug, Clone, thiserror::Error)]
#[error("L0 attach failed ({layer}): {reason}")]
pub struct AttachError {
    pub layer: &'static str,
    pub reason: crate::diagnostics::SandboxReason,
}

impl AttachError {
    fn new(layer: &'static str, reason: crate::diagnostics::SandboxReason) -> Self {
        Self { layer, reason }
    }

    pub(crate) fn missing_pid() -> Self {
        Self::new(
            LAYER,
            crate::diagnostics::SandboxReason::MissingChildProcessId,
        )
    }

    #[cfg(all(windows, test))]
    pub(crate) fn attach_failed() -> Self {
        Self::new(
            LAYER,
            crate::diagnostics::SandboxReason::ProcessTreeAttachFailed,
        )
    }
}

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
    /// `CODE_PROCESS_TREE_DEGRADED` diagnostic.
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
    #[cfg(test)]
    fail_terminate_once: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct TestTreeFaults {
    attach: bool,
    terminate: bool,
    #[cfg(windows)]
    resume: bool,
}

#[cfg(test)]
impl TestTreeFaults {
    pub(crate) fn attach() -> Self {
        Self {
            attach: true,
            terminate: false,
            #[cfg(windows)]
            resume: false,
        }
    }

    pub(crate) fn terminate() -> Self {
        Self {
            attach: false,
            terminate: true,
            #[cfg(windows)]
            resume: false,
        }
    }

    #[cfg(windows)]
    pub(crate) fn resume() -> Self {
        Self {
            attach: false,
            terminate: false,
            resume: true,
        }
    }

    #[cfg(windows)]
    pub(crate) fn resume_fails(self) -> bool {
        self.resume
    }
}

/// Fail-closed cleanup for the Unix spawn-to-attach window. The child must
/// still be the leader of the process group configured before spawn; otherwise
/// no group is signaled, avoiding an unrelated target after a setup failure.
#[cfg(unix)]
pub(crate) fn terminate_verified_configured_group(child_pid: u32) -> TerminationOutcome {
    let Ok(pgid) = i32::try_from(child_pid) else {
        return TerminationOutcome::Failed(AttachError::new(
            LAYER,
            crate::diagnostics::SandboxReason::ProcessTreeTerminationFailed,
        ));
    };
    let actual_pgid = unsafe { libc::getpgid(pgid) };
    if actual_pgid != pgid {
        return TerminationOutcome::Failed(AttachError::new(
            LAYER,
            crate::diagnostics::SandboxReason::ProcessTreeTerminationFailed,
        ));
    }

    let rc = unsafe { libc::kill(-pgid, libc::SIGKILL) };
    if rc == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
        TerminationOutcome::Terminated
    } else {
        TerminationOutcome::Failed(AttachError::new(
            LAYER,
            crate::diagnostics::SandboxReason::ProcessTreeTerminationFailed,
        ))
    }
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
    /// A guard that contains nothing. Used only where no live child tree is
    /// being transferred; assignment failures with a live child fail closed.
    pub fn disabled() -> Self {
        Self {
            inner: TreeGuardInner::Disabled,
            #[cfg(test)]
            fail_terminate_once: false,
        }
    }

    /// Consume the L0 containment so the next [`terminate`] / `Drop` is a
    /// no-op. Reserved for callers that deliberately transfer tree ownership;
    /// normal command and adapter lifecycles keep the guard armed even after a
    /// clean direct-child exit so backgrounded descendants are terminated.
    ///
    /// [`terminate`]: TreeGuard::terminate
    pub fn disarm(&mut self) {
        self.inner = TreeGuardInner::Disabled;
    }

    /// Attach L0 to an already-spawned child identified by `child_pid`.
    ///
    /// On the Unix group mechanism the pid IS the group id (the child was made
    /// a leader by [`configure_tree`]), so no kernel call is needed here. On
    /// Windows this creates the kill-on-close Job Object and assigns the child
    /// to it. PID zero is always rejected before any OS call.
    pub fn attach(child_pid: u32) -> Result<Self, AttachError> {
        Self::attach_inner(child_pid, false, false)
    }

    /// Attach from the optional PID returned by `tokio::process::Child::id`.
    /// A reaped or otherwise unavailable child PID is a named, redacted attach
    /// failure and never falls back to the sentinel PID zero.
    pub fn attach_child(child_pid: Option<u32>) -> Result<Self, AttachError> {
        match child_pid {
            Some(pid) => Self::attach(pid),
            None => Err(AttachError::missing_pid()),
        }
    }

    #[cfg(test)]
    pub(crate) fn attach_with_faults(
        child_pid: u32,
        faults: TestTreeFaults,
    ) -> Result<Self, AttachError> {
        Self::attach_inner(child_pid, faults.attach, faults.terminate)
    }

    fn attach_inner(
        child_pid: u32,
        inject_attach_failure: bool,
        _inject_terminate_failure: bool,
    ) -> Result<Self, AttachError> {
        if child_pid == 0 {
            return Err(AttachError::missing_pid());
        }
        if inject_attach_failure {
            return Err(AttachError::new(
                LAYER,
                crate::diagnostics::SandboxReason::ProcessTreeAttachFailed,
            ));
        }
        #[cfg(unix)]
        {
            Ok(Self {
                inner: TreeGuardInner::Group {
                    pgid: child_pid as i32,
                    terminated: false,
                },
                #[cfg(test)]
                fail_terminate_once: _inject_terminate_failure,
            })
        }
        #[cfg(windows)]
        {
            let job = JobGuard::new()?;
            job.assign(child_pid)?;
            Ok(Self {
                inner: TreeGuardInner::Job(Some(job)),
                #[cfg(test)]
                fail_terminate_once: _inject_terminate_failure,
            })
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (child_pid, _inject_terminate_failure);
            Ok(Self::disabled())
        }
    }

    /// Terminate the whole tree. Idempotent and panic-free. The caller surfaces
    /// the returned [`TerminationOutcome`] as a diagnostic when it is `Failed`.
    pub fn terminate(&mut self) -> TerminationOutcome {
        #[cfg(test)]
        if std::mem::take(&mut self.fail_terminate_once) {
            return TerminationOutcome::Failed(AttachError::new(
                LAYER,
                crate::diagnostics::SandboxReason::ProcessTreeTerminationFailed,
            ));
        }
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
                            crate::diagnostics::SandboxReason::ProcessTreeTerminationFailed,
                        ))
                    }
                }
                TreeGuardInner::Disabled => TerminationOutcome::AlreadyTerminated,
            }
        }
        #[cfg(windows)]
        {
            match &mut self.inner {
                TreeGuardInner::Job(job_slot) => {
                    terminate_job_slot_with(job_slot, JobGuard::terminate)
                }
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
fn terminate_job_slot_with<F>(job_slot: &mut Option<JobGuard>, terminate: F) -> TerminationOutcome
where
    F: FnOnce(&mut JobGuard) -> Result<(), AttachError>,
{
    let Some(mut job) = job_slot.take() else {
        return TerminationOutcome::AlreadyTerminated;
    };
    match terminate(&mut job) {
        Ok(()) => {
            // Drop closes the handle and fires KILL_ON_JOB_CLOSE.
            TerminationOutcome::Terminated
        }
        Err(error) => {
            // Keep the job armed so TreeGuard::drop retries termination and
            // JobGuard::drop still enforces kill-on-close.
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
            return Err(AttachError::new(
                LAYER,
                crate::diagnostics::SandboxReason::ProcessTreeAttachFailed,
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
            unsafe { CloseHandle(handle) };
            return Err(AttachError::new(
                LAYER,
                crate::diagnostics::SandboxReason::ProcessTreeAttachFailed,
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
                crate::diagnostics::SandboxReason::ProcessTreeAttachFailed,
            ));
        }
        let ok = unsafe { AssignProcessToJobObject(self.handle as *mut _, proc) };
        unsafe { CloseHandle(proc) };
        if ok == 0 {
            return Err(AttachError::new(
                LAYER,
                crate::diagnostics::SandboxReason::ProcessTreeAttachFailed,
            ));
        }
        Ok(())
    }

    fn terminate(&mut self) -> Result<(), AttachError> {
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;
        self.terminate_with(
            |handle, exit_code| unsafe { TerminateJobObject(handle as *mut _, exit_code) },
            io::Error::last_os_error,
        )
    }

    fn terminate_with<T, E>(
        &mut self,
        terminate_job_object: T,
        last_os_error: E,
    ) -> Result<(), AttachError>
    where
        T: FnOnce(usize, u32) -> i32,
        E: FnOnce() -> io::Error,
    {
        if self.handle == 0 {
            return Ok(());
        }
        if terminate_job_object(self.handle, 1) == 0 {
            let error = last_os_error();
            let _ = error;
            return Err(AttachError::new(
                LAYER,
                crate::diagnostics::SandboxReason::ProcessTreeTerminationFailed,
            ));
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
            // every process still in it. This is the safety net for a dropped
            // exec future / dropped adapter host.
            unsafe { CloseHandle(self.handle as *mut _) };
            self.handle = 0;
        }
    }
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
        let err = AttachError::new(
            "windows-job",
            crate::diagnostics::SandboxReason::ProcessTreeAttachFailed,
        );
        assert_eq!(err.layer, "windows-job");
        assert_eq!(
            err.reason,
            crate::diagnostics::SandboxReason::ProcessTreeAttachFailed
        );
        assert!(err.to_string().contains("windows-job"));
        assert!(err.to_string().contains("containment attach failed"));
    }

    #[test]
    fn injected_attach_fault_is_explicit_and_redacted() {
        let err = TreeGuard::attach_with_faults(424242, TestTreeFaults::attach())
            .expect_err("injected strategy must force attach failure");
        assert_eq!(err.layer, LAYER);
        assert_eq!(
            err.reason,
            crate::diagnostics::SandboxReason::ProcessTreeAttachFailed
        );
    }

    #[test]
    fn zero_or_missing_pid_is_a_named_attach_failure() {
        for result in [TreeGuard::attach(0), TreeGuard::attach_child(None)] {
            let err = result.expect_err("PID 0/missing PID must not attach");
            assert_eq!(err.layer, LAYER);
            assert_eq!(
                err.reason,
                crate::diagnostics::SandboxReason::MissingChildProcessId
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn attach_failure_cleanup_refuses_an_unverified_process_group() {
        let mut child = std::process::Command::new("sh")
            .args(["-c", "sleep 60"])
            .spawn()
            .expect("spawn child without process-group configuration");
        let pid = child.id();

        let outcome = terminate_verified_configured_group(pid);

        match outcome {
            TerminationOutcome::Failed(error) => assert_eq!(
                error.reason,
                crate::diagnostics::SandboxReason::ProcessTreeTerminationFailed
            ),
            other => panic!("unverified group must be rejected, got {other:?}"),
        }
        assert!(
            child.try_wait().unwrap().is_none(),
            "verification failure must not signal the unrelated process group"
        );
        let _ = child.kill();
        let _ = child.wait();
    }

    #[cfg(windows)]
    #[test]
    fn failed_job_termination_is_reported_and_keeps_kill_on_close_armed() {
        let mut job_slot = Some(JobGuard { handle: 1 });
        let outcome = terminate_job_slot_with(&mut job_slot, |job| {
            job.terminate_with(|_handle, _exit_code| 0, || io::Error::from_raw_os_error(5))
        });

        match outcome {
            TerminationOutcome::Failed(error) => {
                assert_eq!(error.layer, "windows-job");
                assert_eq!(
                    error.reason,
                    crate::diagnostics::SandboxReason::ProcessTreeTerminationFailed
                );
            }
            other => panic!("expected failed termination, got {other:?}"),
        }
        assert_eq!(
            job_slot.as_ref().map(|job| job.handle),
            Some(1),
            "failed termination must retain the job for kill-on-close Drop"
        );

        // The injected handle is not a real kernel handle; disarm it before
        // test cleanup so JobGuard::drop does not call CloseHandle(1).
        job_slot.as_mut().unwrap().handle = 0;
    }
}
