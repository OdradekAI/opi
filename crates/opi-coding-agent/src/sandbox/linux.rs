//! Phase 15 task 15.5.2 — Linux strict substrate (seccomp deny-overlay + Landlock
//! capability model).
//!
//! Two independent capability layers for the future Linux strict backend:
//!
//! 1. **seccomp deny-overlay** — a default-allow / match-deny BPF filter built
//!    in the parent ([`build_seccomp_filter`] -> [`compile_filter`]) and applied
//!    as a raw program in the confined child ([`apply_raw_filter`]). It denies
//!    new `socket(AF_INET | AF_INET6 | AF_NETLINK, ...)` creation (the network
//!    reduction, preserving `AF_UNIX` IPC) and the exact L3 danger blocklist
//!    ([`danger_syscalls`]) unconditionally.
//!
//!    Classic seccomp cannot dereference `sockaddr` pointers, so only socket
//!    *creation* is domain-filtered; `connect`/`bind`/`sendto`/`recvfrom`/
//!    `accept` are not domain-filterable. Those residuals (inherited fds,
//!    non-TCP traffic) are documented in the phase research notes and owned by
//!    task 15.5.3's runtime attach.
//!
//! 2. **Landlock capability model** — the ABI-4 TCP bind/connect-by-port rights
//!    ([`landlock_tcp_rights`], [`landlock_tcp_capability`]), keyed to the
//!    *observed* Landlock ABI rather than a kernel release string.
//!
//! **Substrate only.** This module is not yet wired into
//! [`super::prepare_production`]; task 15.5.3 attaches it to the production
//! `LocalBashOperations::exec` -> `sandbox::prepare` path and owns the runtime
//! ABI query + inherited-fd residuals. Task 15.5.6 owns the cross-arch release
//! matrix (`iopl`/`ioperm` are x86_64-only and cfg-gated here).
//!
//! `#![forbid(unsafe_code)]`: seccompiler and landlock expose safe APIs; the raw
//! `seccomp(2)` and `landlock_create_ruleset(2)` syscalls live inside those
//! crates, not in this module.

#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use landlock::{ABI, Access, AccessFs, AccessNet, BitFlags};
use seccompiler::{
    BpfProgram, SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter,
    SeccompRule, TargetArch, apply_filter,
};

/// Stable errno returned for every denied syscall.
///
/// `EPERM` ("operation not permitted") is the conventional sandbox denial.
/// seccompiler applies a single match-action per filter, so all denied syscalls
/// (socket-creation gate and danger blocklist) share this errno.
pub const DENY_ERRNO: u32 = libc::EPERM as u32;

/// Socket address families denied at `socket()` creation (the network
/// reduction). `AF_UNIX` is intentionally absent — Unix-domain IPC must survive.
pub const DENIED_SOCKET_DOMAINS: &[(&str, i64)] = &[
    ("AF_INET", libc::AF_INET as i64),
    ("AF_INET6", libc::AF_INET6 as i64),
    ("AF_NETLINK", libc::AF_NETLINK as i64),
];

/// The exact L3 danger blocklist: privileged syscalls denied unconditionally.
///
/// `clone` and `unshare` are intentionally **not** listed (they remain allowed).
/// `iopl` and `ioperm` are x86 IO-port syscalls and cfg-gated to `x86_64`; other
/// supported Linux architectures omit them (task 15.5.6 owns the cross-arch
/// matrix).
pub fn danger_syscalls() -> Vec<(&'static str, i64)> {
    let mut v: Vec<(&'static str, i64)> = vec![
        ("open_by_handle_at", libc::SYS_open_by_handle_at),
        ("bpf", libc::SYS_bpf),
        ("perf_event_open", libc::SYS_perf_event_open),
        ("ptrace", libc::SYS_ptrace),
        ("kexec_load", libc::SYS_kexec_load),
        ("kexec_file_load", libc::SYS_kexec_file_load),
        ("reboot", libc::SYS_reboot),
        ("init_module", libc::SYS_init_module),
        ("finit_module", libc::SYS_finit_module),
        ("delete_module", libc::SYS_delete_module),
        ("swapon", libc::SYS_swapon),
        ("swapoff", libc::SYS_swapoff),
        ("acct", libc::SYS_acct),
        ("settimeofday", libc::SYS_settimeofday),
    ];
    // x86-only IO-port syscalls; absent on aarch64/riscv64.
    #[cfg(target_arch = "x86_64")]
    v.extend_from_slice(&[("iopl", libc::SYS_iopl), ("ioperm", libc::SYS_ioperm)]);
    v
}

/// Mismatch action (no rule matches a syscall): allow. The filter is
/// default-allow — only the explicitly-denied syscalls below are blocked.
pub const FILTER_MISMATCH_ACTION: SeccompAction = SeccompAction::Allow;

/// Match action (a rule matches): deny with [`DENY_ERRNO`].
pub const FILTER_MATCH_ACTION: SeccompAction = SeccompAction::Errno(DENY_ERRNO);

/// Build the syscall -> rules map for the deny-overlay (**parent** side, before
/// wrapping in [`SeccompFilter`]). Exposed so tests can assert the policy
/// encoding directly (the compiled [`BpfProgram`] alone cannot reveal which
/// syscalls/rules seccompiler built, because [`SeccompFilter`] keeps its rules
/// private).
///
/// - `socket(domain, ...)`: one OR-bound rule per denied family comparing
///   `arg[0]` (the scalar `domain`) for equality. `AF_UNIX` has no rule, so its
///   creation falls through to [`FILTER_MISMATCH_ACTION`] (Allow).
/// - each danger syscall: an **empty rule vector** (matches regardless of
///   arguments) -> denied unconditionally.
pub fn build_seccomp_rules() -> Result<BTreeMap<i64, Vec<SeccompRule>>, seccompiler::Error> {
    let mut rules: BTreeMap<i64, Vec<SeccompRule>> = BTreeMap::new();

    // socket(domain) deny: arg[0] == each denied family.
    let mut socket_rules: Vec<SeccompRule> = Vec::with_capacity(DENIED_SOCKET_DOMAINS.len());
    for (_name, domain) in DENIED_SOCKET_DOMAINS.iter() {
        let cond =
            SeccompCondition::new(0, SeccompCmpArgLen::Dword, SeccompCmpOp::Eq, *domain as u64)?;
        socket_rules.push(SeccompRule::new(vec![cond])?);
    }
    rules.insert(libc::SYS_socket, socket_rules);

    // Danger syscalls: empty rule vector -> match regardless of arguments.
    for (_name, sysno) in danger_syscalls() {
        rules.insert(sysno, Vec::new());
    }

    Ok(rules)
}

/// Build the default-allow / match-deny seccomp filter (**parent** side).
///
/// Wraps [`build_seccomp_rules`] with [`FILTER_MISMATCH_ACTION`] /
/// [`FILTER_MATCH_ACTION`] and the target arch. Compilation does not install the
/// filter; [`compile_filter`] produces the loadable [`BpfProgram`] and
/// [`apply_raw_filter`] installs it in the child.
pub fn build_seccomp_filter(target_arch: TargetArch) -> Result<SeccompFilter, seccompiler::Error> {
    let rules = build_seccomp_rules()?;
    Ok(SeccompFilter::new(
        rules,
        FILTER_MISMATCH_ACTION,
        FILTER_MATCH_ACTION,
        target_arch,
    )?)
}

/// Compile a [`SeccompFilter`] into a loadable [`BpfProgram`] (**parent** side,
/// before fork). Does not touch the kernel.
pub fn compile_filter(filter: SeccompFilter) -> Result<BpfProgram, seccompiler::Error> {
    let bpf: BpfProgram = filter.try_into()?;
    Ok(bpf)
}

/// Stable, errno-bearing error from installing a seccomp filter. One stable
/// variant per [`seccompiler::Error`] family so callers and tests match on the
/// error class rather than on `io::Error` internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StableErrno {
    /// The supplied BPF program was empty (nothing to install).
    EmptyFilter,
    /// `prctl(PR_SET_NO_NEW_PRIVS)` failed with this errno.
    Prctl(i32),
    /// `seccomp(2)` (or thread sync) failed with this errno.
    Seccomp(i32),
    /// Backend validation/compilation rejected the filter.
    Backend,
}

impl From<seccompiler::Error> for StableErrno {
    fn from(err: seccompiler::Error) -> Self {
        match err {
            seccompiler::Error::EmptyFilter => StableErrno::EmptyFilter,
            seccompiler::Error::Prctl(e) => StableErrno::Prctl(e.raw_os_error().unwrap_or(0)),
            seccompiler::Error::Seccomp(e) => StableErrno::Seccomp(e.raw_os_error().unwrap_or(0)),
            seccompiler::Error::ThreadSync(_) => StableErrno::Seccomp(libc::EINVAL),
            seccompiler::Error::Backend(_) => StableErrno::Backend,
        }
    }
}

/// Apply a raw BPF filter to the calling thread (**child** side).
///
/// Wraps [`seccompiler::apply_filter`] and translates its error to a stable
/// [`StableErrno`]. Task 15.5.3 calls this inside the confined child's setup
/// (after fork, before exec). An empty program is rejected before any
/// `prctl`/`seccomp` syscall, so invoking it with `&[]` is the safe error-path
/// probe used by tests (no command is spawned, nothing is confined).
pub fn apply_raw_filter(bpf: &BpfProgram) -> Result<(), StableErrno> {
    apply_filter(bpf).map_err(StableErrno::from)
}

// ---------------------------------------------------------------------------
// Landlock capability model
// ---------------------------------------------------------------------------

/// Landlock network capability for an observed ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LandlockNetCapability {
    /// Whether TCP bind/connect-by-port is available (Landlock ABI >= 4).
    pub tcp_bind_connect: bool,
    /// The observed ABI the capability was computed from.
    pub abi: ABI,
}

/// Compute the Landlock TCP capability from an *observed* ABI (not a kernel
/// release string). ABI 4 (Linux 6.7) is the first to provide
/// `AccessNet::{BindTcp, ConnectTcp}`; ABI <= 3 has no network rights.
///
/// Pure and host-independent: the runtime caller (15.5.3) passes the ABI it
/// queried from the kernel; tests inject ABI values to verify the gate.
pub fn landlock_tcp_capability(abi: ABI) -> LandlockNetCapability {
    LandlockNetCapability {
        tcp_bind_connect: !AccessNet::from_all(abi).is_empty(),
        abi,
    }
}

/// The TCP access rights a Landlock ABI-4+ ruleset can enforce: bind and connect
/// by 16-bit TCP port. Equivalent to `AccessNet::from_all(ABI::V4)`.
pub fn landlock_tcp_rights() -> BitFlags<AccessNet> {
    AccessNet::from_all(ABI::V4)
}

/// The Landlock ABI-4 filesystem-rights model: the access rights a Landlock
/// ABI-4+ ruleset can enforce over filesystem objects (the L1 workspace-write
/// layer's capability surface). Equivalent to `AccessFs::from_all(ABI::V4)`.
///
/// Pure and host-independent; the runtime L1 attach (configured workspace/temp
/// writes) is task 15.5.3.
pub fn landlock_fs_rights() -> BitFlags<AccessFs> {
    AccessFs::from_all(ABI::V4)
}

// ---------------------------------------------------------------------------
// Composite capability report
// ---------------------------------------------------------------------------

/// Seccomp socket-creation layer of the strict capability report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketCreationCapability {
    /// Address families denied at `socket()` creation.
    pub denied_families: &'static [(&'static str, i64)],
    /// Stable errno returned on denial.
    pub deny_errno: u32,
}

/// Composite Linux strict capability report, separating the two layers so a
/// caller or diagnostic can distinguish a seccomp socket-creation denial from a
/// Landlock TCP bind/connect gap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxStrictCapability {
    /// Seccomp new-socket creation gate (INET/INET6/NETLINK denied; AF_UNIX kept).
    pub seccomp_socket_creation: SocketCreationCapability,
    /// Landlock TCP bind/connect-by-port capability (ABI-gated).
    pub landlock_tcp_bind_connect: LandlockNetCapability,
}

/// Build the composite strict capability report for an observed Landlock ABI.
/// The seccomp layer is ABI-independent; the Landlock layer follows `abi`.
pub fn linux_strict_capability(abi: ABI) -> LinuxStrictCapability {
    LinuxStrictCapability {
        seccomp_socket_creation: SocketCreationCapability {
            denied_families: DENIED_SOCKET_DOMAINS,
            deny_errno: DENY_ERRNO,
        },
        landlock_tcp_bind_connect: landlock_tcp_capability(abi),
    }
}
