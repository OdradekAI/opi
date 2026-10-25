//! Private Windows helper lifetime. The child starts suspended, joins a
//! noninheritable kill-on-close job, and only then runs. The job survives
//! parent exit and is closed before output drainage on cancellation/timeout.

use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

use tokio::process::Child;
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject,
};
use windows_sys::Win32::System::Threading::{
    GetProcessIdOfThread, OpenThread, ResumeThread, THREAD_QUERY_LIMITED_INFORMATION,
    THREAD_SUSPEND_RESUME,
};

/// An exclusively owned Job handle. OwnedHandle closes once and supplies the
/// standard Send/Sync implementation; no raw-handle lifetime escapes here.
pub(super) struct Job(OwnedHandle);

impl Job {
    pub(super) fn new() -> io::Result<Self> {
        // SAFETY: null attributes/name create an anonymous, noninheritable job;
        // a nonnull return transfers exactly one owned handle to us.
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the validated handle is newly created and not owned elsewhere.
        let job = Self(unsafe { OwnedHandle::from_raw_handle(handle) });
        // SAFETY: this Windows POD structure accepts zero initialization; its
        // selected information class receives the complete, correctly sized value.
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: the job and info outlive this synchronous call; Windows reads
        // exactly info_len bytes and retains neither pointer. No breakaway flag
        // is enabled, so descendants remain in this lifecycle owner.
        let ok = unsafe {
            SetInformationJobObject(
                job.0.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(job)
    }

    pub(super) fn attach_and_resume(&self, child: &Child) -> io::Result<()> {
        let handle = child
            .raw_handle()
            .ok_or_else(|| io::Error::other("missing suspended child handle"))?;
        let pid = child
            .id()
            .ok_or_else(|| io::Error::other("missing suspended child PID"))?;
        // SAFETY: the borrowed child handle remains owned by Child throughout
        // this call. CREATE_SUSPENDED guarantees no child code can run or spawn
        // descendants before successful job assignment. No PID lookup is used.
        if unsafe { AssignProcessToJobObject(self.0.as_raw_handle(), handle) } == 0 {
            return Err(io::Error::last_os_error());
        }
        resume_suspended_child(pid)
    }
}

fn resume_suspended_child(pid: u32) -> io::Result<()> {
    // SAFETY: ToolHelp creates an owned snapshot; the invalid sentinel is
    // checked before conversion into OwnedHandle. The snapshot contains threads,
    // and only threads belonging to our still-suspended child are opened.
    let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if raw == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: this valid snapshot is uniquely owned and closed on every branch.
    let snapshot = unsafe { OwnedHandle::from_raw_handle(raw) };
    // SAFETY: THREADENTRY32 is Windows POD; dwSize is set before enumeration.
    let mut entry: THREADENTRY32 = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
    // SAFETY: snapshot is valid and entry points to a correctly sized writable
    // structure. Thread32First/Next retain neither pointer.
    let mut next = unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) } != 0;
    let mut resumed = false;
    while next {
        if entry.th32OwnerProcessID == pid {
            // SAFETY: the child is suspended and its process handle remains
            // owned by the caller, preventing PID reuse. We request only the
            // rights needed to verify and resume its initial thread.
            let raw = unsafe {
                OpenThread(
                    THREAD_SUSPEND_RESUME | THREAD_QUERY_LIMITED_INFORMATION,
                    0,
                    entry.th32ThreadID,
                )
            };
            if raw.is_null() {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: this valid, newly opened handle has one exclusive owner.
            let thread = unsafe { OwnedHandle::from_raw_handle(raw) };
            // SAFETY: the query uses the live owned thread handle. Recheck its
            // owner after opening: a stale snapshot's thread ID must not cause
            // us to resume a foreign thread if it was recycled in the meantime.
            if unsafe { GetProcessIdOfThread(thread.as_raw_handle()) } != pid {
                return Err(io::Error::other("suspended child thread was replaced"));
            }
            // SAFETY: the handle has THREAD_SUSPEND_RESUME and names this child's
            // initial suspended thread. Failure leaves the caller responsible
            // for closing the job and killing/reaping the child before return.
            if unsafe { ResumeThread(thread.as_raw_handle()) } == u32::MAX {
                return Err(io::Error::last_os_error());
            }
            resumed = true;
        }
        // SAFETY: the same live snapshot and initialized entry remain valid.
        next = unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) } != 0;
    }
    if resumed {
        Ok(())
    } else {
        Err(io::Error::other("suspended child thread was not found"))
    }
}
