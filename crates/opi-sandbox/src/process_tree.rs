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
//! # Fail-closed attachment
//!
//! Assignment failure is returned to the runner, which kills the unreleased
//! bootstrap and refuses the run. Termination failure is retained as a redacted
//! [`TerminationOutcome::Failed`] so cleanup cannot be reported as confirmed.
//! [`TreeGuard::terminate`] is idempotent and panic-free.

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
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("L0 attach failed ({layer}): {reason:?}")]
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

/// Outcome of [`TreeGuard::terminate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminationOutcome {
    /// The tree had already been terminated; this call did nothing.
    AlreadyTerminated,
    /// The whole tree was terminated by this call.
    Terminated,
    /// Termination failed at the given layer/reason; the caller must report
    /// cleanup as unconfirmed while the drop guards retry best-effort cleanup.
    Failed(AttachError),
}

/// Configure a [`tokio::process::Command`] for tree containment BEFORE spawn.
///
/// Unix: assigns the child to a brand-new process group (`pgid == child pid`),
/// so the whole tree can be signaled later by negating the pid. Windows creates
/// the child suspended so [`TreeGuard::attach`] can assign its Job Object before
/// any child code runs; the caller resumes it only after successful assignment.
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

/// Resume every primary-process thread after the suspended child has been
/// assigned to its Job Object. A newly-created suspended process has exactly
/// one thread, but enumeration keeps the wrapper correct if Windows changes
/// startup internals.
#[cfg(windows)]
pub fn resume_child(child_pid: u32) -> Result<(), AttachError> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
    };
    use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(AttachError::new(LAYER, TreeReason::AttachFailed));
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
                return Err(AttachError::new(LAYER, TreeReason::AttachFailed));
            }
            unsafe { CloseHandle(thread) };
            found = true;
        }
        ok = unsafe { Thread32Next(snapshot, &mut entry) } != 0;
    }
    unsafe { CloseHandle(snapshot) };
    found
        .then_some(())
        .ok_or_else(|| AttachError::new(LAYER, TreeReason::AttachFailed))
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
    /// A guard that contains nothing on targets with no native tree primitive.
    /// Supported Unix and Windows runners never use this after attachment
    /// failure.
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

// =========================================================================
// Phase 16 task 16.13 — Linux native confinement FFI
// =========================================================================
//
// `platform/mod.rs` (and therefore `platform/linux.rs`) is
// `#![forbid(unsafe_code)]`, which propagates to submodules and cannot be
// overridden — the Phase 15 trap (opi-coding-agent `sandbox/linux.rs:36-38`).
// `process_tree` is the crate's documented FFI home ("every FFI call lives HERE
// behind a safe wrapper"), so the two audited `unsafe` helpers the Linux native
// leaf needs — the observed-ABI probe and the `pre_exec` child-setup — live
// here. `platform/linux.rs` builds the confinement plan from the safe
// `landlock`/`seccompiler` APIs and calls these helpers to perform the kernel
// calls. Compiled only on Linux.

#[cfg(target_os = "linux")]
use std::sync::Arc;

#[cfg(target_os = "linux")]
use landlock::{ABI, RulesetCreated};

#[cfg(target_os = "linux")]
use seccompiler::BpfProgram;

/// Query the kernel's **observed** Landlock ABI (read-only; no confinement).
/// `landlock_create_ruleset(NULL, 0, LANDLOCK_CREATE_RULESET_VERSION)` returns
/// the supported ABI (1..) or a negative errno when Landlock is absent/disabled.
/// Replicates the landlock crate's private probe, which it does not expose, so
/// the posture resolver can report per-layer availability before spawn.
#[cfg(target_os = "linux")]
pub(crate) fn observed_landlock_abi() -> ABI {
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

/// Map a `seccompiler::Error` to a stable raw errno without allocating (the
/// `pre_exec` context is async-signal-safe).
#[cfg(target_os = "linux")]
fn seccomp_errno(err: &seccompiler::Error) -> i32 {
    match err {
        seccompiler::Error::EmptyFilter | seccompiler::Error::Backend(_) => libc::EINVAL,
        seccompiler::Error::Prctl(e) | seccompiler::Error::Seccomp(e) => {
            e.raw_os_error().unwrap_or(libc::EINVAL)
        }
        seccompiler::Error::ThreadSync(_) => libc::EBUSY,
    }
}

/// The one audited child-setup helper: register a `pre_exec` hook on `cmd` that
/// installs confinement BEFORE execve, in the order the audit (wf_e03e0e6e-c84)
/// fixed as load-bearing for fd lifetimes:
///   1. apply the seccomp deny-overlay (`prctl` PR_SET_NO_NEW_PRIVS + `seccomp`);
///   2. Landlock `restrict_self` for the fs then network rulesets — this
///      CONSUMES the ruleset fds (they close when the moved `RulesetCreated`
///      values drop), so it MUST precede the fd closure (closing them first
///      would make restrict_self fail with EBADF on every run);
///   3. close inherited nonessential descriptors (preserve stdio 0/1/2 and
///      AF_UNIX sockets), closing any inherited INET/INET6/NETLINK socket the
///      seccomp socket-creation gate could not have prevented.
///
/// Only the std `pre_exec` registration is `unsafe`. A failure at any step
/// returns an `Err(io::Error)` so `execve` is never reached: the spawn fails and
/// the target is never released (fail-closed; the runner maps the spawn failure
/// to `SetupFailureReason::SpawnFailed`). The closure performs only
/// async-signal-safe operations on every path (syscalls + errno-backed
/// `io::Error`s; no allocation, locking, or stdio after fork).
#[cfg(target_os = "linux")]
pub(crate) fn install_child_confinement(
    cmd: &mut tokio::process::Command,
    bpf: Arc<BpfProgram>,
    fs_ruleset: Option<RulesetCreated>,
    network_ruleset: Option<RulesetCreated>,
    close_inherited: bool,
) {
    use std::os::unix::process::CommandExt;
    let mut fs_ruleset = fs_ruleset;
    let mut network_ruleset = network_ruleset;
    // SAFETY: `pre_exec` runs the closure in the child after fork, before
    // execve, in an async-signal-safe context. The closure captures `bpf`
    // (`Arc<BpfProgram>`), `fs_ruleset`, and `network_ruleset`
    // (`Option<RulesetCreated>`), all `Send + Sync + 'static`. On the success
    // path it calls only syscalls (seccomp apply, landlock restrict_self,
    // getdtablesize/getsockopt/close). Error paths return only errno-backed
    // `io::Error` values — no allocator, formatting, or locks touched after
    // fork.
    let _ = unsafe {
        cmd.as_std_mut().pre_exec(move || {
            // (1) seccomp deny-overlay.
            if let Err(error) = seccompiler::apply_filter(bpf.as_ref()) {
                return Err(std::io::Error::from_raw_os_error(seccomp_errno(&error)));
            }
            // (2) Landlock restrict_self (fs then network). The moved
            // `RulesetCreated` values drop at the end of their `if let` scope,
            // closing the ruleset fds; this MUST happen before (3) so the fd
            // closure never closes a fd restrict_self still needs.
            for ruleset in [&mut fs_ruleset, &mut network_ruleset] {
                if let Some(rs) = ruleset.take()
                    && let Err(error) = rs.restrict_self()
                {
                    return Err(std::io::Error::from_raw_os_error(*landlock::Errno::from(
                        error,
                    )));
                }
            }
            // (3) close inherited nonessential descriptors (last). Only for
            // network = deny (design `### Linux` groups descriptor closure
            // under the network-deny clause).
            if close_inherited {
                close_nonessential_inherited_fds();
            }
            Ok(())
        })
    };
}

/// Close every inherited descriptor `>= 3` that is not an AF_UNIX socket,
/// preserving stdio (0/1/2) and AF_UNIX sockets (ordinary local IPC). Runs AFTER
/// Landlock restrict_self, so the consumed ruleset fds are already closed.
/// Best-effort: a `close` error (EBADF = already closed) is ignored. This closes
/// the documented inherited-fd residual — an inherited INET/INET6/NETLINK socket
/// the seccomp socket-creation gate cannot have blocked (it only gates
/// `socket()` CREATION, not inherited fds). Cooperating descriptor transfer
/// remains outside the non-malicious-command threat model (design `### Linux`).
#[cfg(target_os = "linux")]
fn close_nonessential_inherited_fds() {
    let max = fd_table_size();
    let mut fd: i32 = 3;
    while fd < max {
        if !is_af_unix_socket(fd) {
            // SAFETY: close(2) on a valid or already-closed fd; EBADF is
            // ignored. No allocation.
            let _ = unsafe { libc::close(fd) };
        }
        fd += 1;
    }
}

/// The upper bound for fd iteration: the process soft `RLIMIT_NOFILE`
/// (`getdtablesize`), which bounds the loop to the actually-usable fds.
#[cfg(target_os = "linux")]
fn fd_table_size() -> i32 {
    // SAFETY: getdtablesize is a read-only glibc query returning the soft
    // RLIMIT_NOFILE; no side effects.
    let max = unsafe { libc::getdtablesize() };
    if max <= 3 { 3 } else { max }
}

/// Whether `fd` is an AF_UNIX socket, queried via `getsockopt(SO_DOMAIN)`. A
/// non-socket fd returns `false` (getsockopt fails with ENOTSOCK). `getsockopt`
/// is a Linux syscall; like the seccomp/landlock syscalls already used in this
/// `pre_exec` context, it performs no userspace locking or allocation, so it is
/// safe in the async-signal-safe child-setup path.
#[cfg(target_os = "linux")]
fn is_af_unix_socket(fd: i32) -> bool {
    let mut domain: libc::c_int = 0;
    let mut len = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
    // SAFETY: read-only getsockopt query. `domain` is a valid i32 outparam of
    // the size `len` advertises; the fd is not consumed.
    let rc = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_DOMAIN,
            &mut domain as *mut _ as *mut libc::c_void,
            &mut len,
        )
    };
    rc == 0 && domain == libc::AF_UNIX
}
