//! OS process-tree layer for [`super::ProcessSupervisor`].
//!
//! This is the single unsafe-FFI home of the `opi-eval` crate (see the
//! crate-root lint comment): Unix terminates the child's whole process group
//! with `killpg`, Windows assigns the child to a kill-on-close Job Object.
//! Every call is wrapped in a safe, panic-free API consumed only by
//! `process.rs`; no OS primitive leaves this module family.
//!
//! Unix: the child is made the leader of a new process group at spawn
//! (`process_group(0)`), so the whole tree is signaled later with
//! `killpg(pgid, SIGKILL)`. `ESRCH` (already gone) is a successful no-op.
//! Windows: the child starts suspended, is assigned to a kill-on-close Job
//! Object, and is resumed only after successful assignment; termination is
//! `TerminateJobObject` and emptiness is observed through the job's active
//! process count.

// This module overrides the crate-root `deny(unsafe_code)` because it IS the
// crate's single documented FFI home (see the crate-root comment): every
// `unsafe` block below wraps one audited OS call behind a safe wrapper.
#![allow(unsafe_code)]

use std::time::Duration;

/// Stable layer name for the Unix process-group mechanism.
#[cfg(unix)]
pub(super) const LAYER: &str = "unix-pgroup";
/// Stable layer name for the Windows Job-Object mechanism.
#[cfg(windows)]
pub(super) const LAYER: &str = "windows-job";

#[cfg(unix)]
pub(super) fn configure(cmd: &mut tokio::process::Command) {
    // New process group led by the child (pgid == child pid): the whole tree
    // can be signaled later with killpg. Safe std-backed tokio API.
    cmd.process_group(0);
}

#[cfg(windows)]
pub(super) fn configure(cmd: &mut tokio::process::Command) {
    // Start suspended so the Job Object is assigned before any child code
    // (including descendants) runs; `attach` resumes the threads.
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::CREATE_SUSPENDED;
    cmd.as_std_mut().creation_flags(CREATE_SUSPENDED);
}

#[cfg(not(any(unix, windows)))]
pub(super) fn configure(_cmd: &mut tokio::process::Command) {}

/// Post-spawn guard over the child's whole descendant tree.
///
/// `terminate` is idempotent and panic-free; `verify_terminated` observes
/// emptiness for a bounded window and never blocks the caller past it.
pub(super) struct TreeGuard {
    inner: TreeGuardInner,
}

#[cfg(unix)]
enum TreeGuardInner {
    Group {
        pgid: i32,
    },
    /// Attachment is impossible on this host; termination is a no-op.
    Disabled,
}

#[cfg(windows)]
enum TreeGuardInner {
    Job(Option<JobGuard>),
    Disabled,
}

#[cfg(not(any(unix, windows)))]
enum TreeGuardInner {
    Disabled,
}

impl TreeGuard {
    /// Bind the tree of already-spawned `child_pid` to this guard. On Unix the
    /// pid IS the group leader id (set by [`configure`]); on Windows this
    /// creates and assigns the Job Object and resumes the child.
    pub(super) fn attach(child_pid: Option<u32>) -> Self {
        match child_pid {
            #[cfg(unix)]
            Some(pid) if pid != 0 => Self {
                inner: TreeGuardInner::Group { pgid: pid as i32 },
            },
            #[cfg(windows)]
            Some(pid) if pid != 0 => Self {
                inner: JobGuard::new()
                    .and_then(|job| job.assign(pid).is_ok().then_some(job))
                    .map(|job| TreeGuardInner::Job(Some(job)))
                    .unwrap_or(TreeGuardInner::Disabled),
            },
            _ => Self {
                inner: TreeGuardInner::Disabled,
            },
        }
    }

    /// Terminate the whole tree. Returns whether the termination request
    /// succeeded (including "already gone").
    pub(super) fn terminate(&mut self) -> bool {
        #[cfg(unix)]
        {
            match &self.inner {
                TreeGuardInner::Group { pgid } => {
                    // SAFETY: killpg(2) with a positive pgid that is not our own
                    // group (the child leads a fresh one). ESRCH = already gone.
                    let rc = unsafe { libc::killpg(*pgid, libc::SIGKILL) };
                    rc == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
                }
                TreeGuardInner::Disabled => false,
            }
        }
        #[cfg(windows)]
        {
            match &mut self.inner {
                TreeGuardInner::Job(slot) => {
                    let Some(mut job) = slot.take() else {
                        return true; // already terminated
                    };
                    match job.terminate() {
                        Ok(()) => true,
                        Err(_) => {
                            *slot = Some(job); // keep kill-on-close armed
                            false
                        }
                    }
                }
                TreeGuardInner::Disabled => false,
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            false
        }
    }

    /// Observe whether the tree is empty within `window`, polling at a fixed
    /// cadence. Returns `false` on timeout: cleanup stays reported unverified
    /// rather than silently claimed.
    pub(super) async fn verify_terminated(&self, window: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + window;
        loop {
            if self.is_empty() {
                return true;
            }
            if tokio::time::Instant::now() >= deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    /// One immediate emptiness probe over the tree. Used after a natural
    /// child exit: a non-empty group or a still-open inherited pipe means
    /// descendants remain and cleanup must run instead of being reported
    /// as not required.
    pub(super) fn is_empty_now(&self) -> bool {
        self.is_empty()
    }

    #[cfg(unix)]
    fn is_empty(&self) -> bool {
        match &self.inner {
            TreeGuardInner::Group { pgid } => {
                let rc = unsafe { libc::killpg(*pgid, 0) };
                rc == -1 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
            }
            TreeGuardInner::Disabled => true,
        }
    }

    #[cfg(windows)]
    fn is_empty(&self) -> bool {
        match &self.inner {
            TreeGuardInner::Job(Some(job)) => job.active_processes() == 0,
            _ => true,
        }
    }

    #[cfg(not(any(unix, windows)))]
    fn is_empty(&self) -> bool {
        true
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
impl JobGuard {
    fn new() -> Option<Self> {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::JobObjects::{
            CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        };

        // Anonymous job: no security attributes, no name.
        // SAFETY: CreateJobObjectW with null inputs returns a fresh handle.
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return None;
        }
        // Configure kill-on-close; no breakaway-OK so descendants cannot
        // escape the job. SAFETY: `info` is a zeroed value of the exact type
        // the call expects, passed with its true size.
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let ok = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if ok == 0 {
            unsafe { CloseHandle(handle) };
            return None;
        }
        Some(Self {
            handle: handle as usize,
        })
    }

    fn assign(&self, pid: u32) -> Result<(), ()> {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
        };

        // SAFETY: OpenProcess on the just-spawned child pid with the exact
        // rights AssignProcessToJobObject needs; the handle is closed below.
        let proc = unsafe { OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid) };
        if proc.is_null() {
            return Err(());
        }
        // SAFETY: job handle is owned by this guard; the process handle was
        // just opened and is valid.
        let ok = unsafe { AssignProcessToJobObject(self.handle as *mut _, proc) };
        unsafe { CloseHandle(proc) };
        if ok == 0 {
            return Err(());
        }
        // Resume every thread of the still-suspended bootstrap (a fresh
        // process has one; enumeration stays correct if that changes).
        self.resume_child(pid)
    }

    fn resume_child(&self, pid: u32) -> Result<(), ()> {
        use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
        };
        use windows_sys::Win32::System::Threading::{
            OpenThread, ResumeThread, THREAD_SUSPEND_RESUME,
        };

        // SAFETY: snapshot over live threads; invalid handle checked after.
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(());
        }
        let mut entry: THREADENTRY32 = unsafe { std::mem::zeroed() };
        entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
        let mut found = false;
        // SAFETY: standard ToolHelp first/next walk over `entry`.
        let mut ok = unsafe { Thread32First(snapshot, &mut entry) } != 0;
        while ok {
            if entry.th32OwnerProcessID == pid {
                // SAFETY: thread handle with resume rights for a pid we own.
                let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
                if thread.is_null() || unsafe { ResumeThread(thread) } == u32::MAX {
                    if !thread.is_null() {
                        unsafe { CloseHandle(thread) };
                    }
                    unsafe { CloseHandle(snapshot) };
                    return Err(());
                }
                unsafe { CloseHandle(thread) };
                found = true;
            }
            // SAFETY: continue the walk.
            ok = unsafe { Thread32Next(snapshot, &mut entry) } != 0;
        }
        unsafe { CloseHandle(snapshot) };
        found.then_some(()).ok_or(())
    }

    fn terminate(&mut self) -> Result<(), ()> {
        use windows_sys::Win32::System::JobObjects::TerminateJobObject;
        if self.handle == 0 {
            return Ok(());
        }
        // SAFETY: owned job handle.
        if unsafe { TerminateJobObject(self.handle as *mut _, 1) } == 0 {
            return Err(());
        }
        Ok(())
    }

    fn active_processes(&self) -> u32 {
        use windows_sys::Win32::System::JobObjects::{
            JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JobObjectBasicAccountingInformation,
            QueryInformationJobObject,
        };
        if self.handle == 0 {
            return 0;
        }
        let mut info: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = unsafe { std::mem::zeroed() };
        // SAFETY: query on the owned job handle into a same-typed outparam.
        let ok = unsafe {
            QueryInformationJobObject(
                self.handle as *mut _,
                JobObjectBasicAccountingInformation,
                &mut info as *mut _ as *mut core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            // Unqueryable job: never report empty on doubt.
            return u32::MAX;
        }
        info.ActiveProcesses
    }
}

#[cfg(windows)]
impl Drop for JobGuard {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        if self.handle != 0 {
            // Closing the last handle to a KILL_ON_JOB_CLOSE job kills every
            // process still in it. SAFETY: owned, non-null handle.
            unsafe { CloseHandle(self.handle as *mut _) };
            self.handle = 0;
        }
    }
}
