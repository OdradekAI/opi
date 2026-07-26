//! Phase 15 task 15.5.2 (+ 15.5.3 runtime) — Linux strict backend: seccomp
//! deny-overlay + Landlock, wired into production by [`crate::sandbox::linux::LinuxStrictBackend`].
//!
//! Two independent capability layers:
//!
//! 1. **seccomp deny-overlay** — a default-allow / match-deny BPF filter built
//!    in the parent ([`crate::sandbox::linux::build_seccomp_filter`] -> [`crate::sandbox::linux::compile_filter`])
//!    and applied as a raw program in the confined child via
//!    [`crate::sandbox::linux::apply_raw_filter`]. It denies new
//!    `socket(AF_INET | AF_INET6 | AF_NETLINK, ...)` creation (the network
//!    reduction, preserving `AF_UNIX` IPC) and the exact L3 danger blocklist
//!    ([`crate::sandbox::linux::danger_syscalls`]) unconditionally.
//!
//!    Classic seccomp cannot dereference `sockaddr` pointers, so only socket
//!    *creation* is domain-filtered; `connect`/`bind`/`sendto`/`recvfrom`/
//!    `accept` are not domain-filterable. Those residuals (inherited fds,
//!    non-TCP traffic) are documented in the phase research notes and covered by
//!    the Landlock TCP layer + the explicit residual list.
//!
//! 2. **Landlock capability model** — the ABI-4 TCP bind/connect-by-port rights
//!    ([`crate::sandbox::linux::landlock_tcp_rights`], [`crate::sandbox::linux::landlock_tcp_capability`]), keyed
//!    to the *observed* Landlock ABI rather than a kernel release string.
//!
//! [`crate::sandbox::linux::LinuxStrictBackend`] is selected by
//! [`crate::sandbox::prepare_production`] on Linux; its confinement plan is
//! applied to the spawn `Command` by [`crate::sandbox::Confinement::apply`] in
//! `LocalBashOperations::exec`. Task 15.5.6 owns the cross-arch release matrix
//! (`iopl`/`ioperm` are x86_64-only and cfg-gated here).
//!
//! # `unsafe` in this module
//!
//! This module contains NO `unsafe`: the seccomp/Landlock *build* paths use the
//! libraries' safe APIs. The two audited `unsafe` helpers the runtime needs —
//! the observed-ABI probe and the `pre_exec` child-setup — live in
//! `crate::tool::process_tree`, because this module is under `sandbox.rs`'s
//! `#![forbid(unsafe_code)]` (which propagates to submodules and cannot be
//! overridden). `sandbox.rs` and `tool/operations.rs` stay
//! `#![forbid(unsafe_code)]`.

#![cfg(target_os = "linux")]

use std::collections::BTreeMap;
use std::io;
use std::path::Path;
use std::sync::Arc;

use landlock::{
    ABI, Access, AccessFs, AccessNet, BitFlags, PathBeneath, PathFd, Ruleset, RulesetAttr,
    RulesetCreated, RulesetCreatedAttr,
};
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

// ---------------------------------------------------------------------------
// Phase 15 task 15.5.3 — production runtime: observed-ABI query, L1 fs rights,
// parent-built confinement plan, and the one audited pre_exec child-setup helper
// ---------------------------------------------------------------------------

/// The L1 filesystem-rights surface the Linux backend governs: every fs right
/// that creates/modifies/removes file content or structure. `ReadFile`,
/// `ReadDir`, and `Execute` are **deliberately not handled**, so a confined child
/// can still exec its shell and read system files/libraries — only writes are
/// confined to the configured workspace/temp paths.
fn landlock_write_rights(abi: ABI) -> BitFlags<AccessFs> {
    // Handle every fs right EXCEPT read/exec. enumflags2's `BitFlags` does not
    // implement `Sub`, so remove the read/exec flags in place.
    let mut w = AccessFs::from_all(abi);
    w.remove(AccessFs::ReadFile);
    w.remove(AccessFs::ReadDir);
    w.remove(AccessFs::Execute);
    w
}

/// Build (parent side) a Landlock ruleset that permits writes only to
/// `workspace` and the process temp dir. The returned [`RulesetCreated`] owns its
/// ruleset file descriptor; it is inherited by the forked child and enforced via
/// `restrict_self()` in the child's `pre_exec`. Errors map to [`io::Error`] so
/// the confinement installer can surface build failures as a failed spawn rather
/// than running a strict command unconfined.
fn build_landlock_ruleset(abi: ABI, workspace: &Path) -> io::Result<RulesetCreated> {
    let write_rights = landlock_write_rights(abi);
    let net_rights = AccessNet::from_all(abi);
    let ruleset = Ruleset::default()
        .handle_access(write_rights)
        .map_err(map_landlock_err("handle_access fs"))?
        .handle_access(net_rights)
        .map_err(map_landlock_err("handle_access net"))?
        .create()
        .map_err(map_landlock_err("create ruleset"))?;
    let mut ruleset = ruleset
        .add_rule(PathBeneath::new(
            PathFd::new(workspace).map_err(map_pathfd_err("workspace"))?,
            write_rights,
        ))
        .map_err(map_landlock_err("add_rule workspace"))?;
    if let Ok(temp_fd) = PathFd::new(std::env::temp_dir()) {
        ruleset = ruleset
            .add_rule(PathBeneath::new(temp_fd, write_rights))
            .map_err(map_landlock_err("add_rule temp"))?;
    }
    Ok(ruleset)
}

fn map_landlock_err(
    step: &'static str,
) -> impl FnOnce(landlock::RulesetError) -> io::Error + 'static {
    move |e| io::Error::other(format!("landlock {step}: {e:?}"))
}

fn map_pathfd_err(step: &'static str) -> impl FnOnce(landlock::PathFdError) -> io::Error + 'static {
    move |e| io::Error::other(format!("landlock {step}: {e:?}"))
}

// `install_child_confinement` and `stable_errno_raw` live in
// `crate::tool::process_tree`: this module is under `sandbox.rs`'s
// `#![forbid(unsafe_code)]`, which propagates to submodules and cannot be
// overridden, so the audited `pre_exec` FFI helper belongs in the crate's
// spawn-FFI home alongside the L0 process-tree helpers.

/// Build the production Linux confinement plan (parent side) for an observed ABI
/// and workspace. The plan captures the compiled seccomp `BpfProgram` (reused via
/// `Arc`) and rebuilds the Landlock ruleset fresh per spawn (each child is a
/// separate fork that consumes its `RulesetCreated` via `restrict_self`). Returns
/// `None` only if the seccomp filter itself cannot compile (it is
/// ABI-independent, so this is a backend error, not a capability gap).
pub fn build_linux_confinement(abi: ABI, workspace: Arc<Path>) -> Option<super::Confinement> {
    let arch: seccompiler::TargetArch = std::env::consts::ARCH.try_into().ok()?;
    let filter = build_seccomp_filter(arch).ok()?;
    let bpf = compile_filter(filter).ok()?;
    Some(super::Confinement::new(
        move |cmd: &mut tokio::process::Command| {
            let bpf = Arc::new(bpf.clone());
            let ruleset = if abi_supports_fs(abi) {
                build_landlock_ruleset(abi, &workspace).ok()
            } else {
                None
            };
            crate::tool::process_tree::install_child_confinement(cmd, bpf, ruleset);
        },
    ))
}

fn abi_supports_fs(abi: ABI) -> bool {
    !matches!(abi, ABI::Unsupported)
}

/// Whether the observed Landlock ABI enables TCP bind/connect (ABI >= 4).
fn abi_supports_tcp(abi: ABI) -> bool {
    matches!(abi, ABI::V4 | ABI::V5 | ABI::V6 | ABI::V7)
}

// ---------------------------------------------------------------------------
// Alternate network-surface audit (DoD: classify every dispatch path; never
// claim complete new-socket coverage while a path remains uncovered)
// ---------------------------------------------------------------------------

/// One entry in the alternate-network-surface audit classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlternateSurfaceClass {
    /// The alternate dispatch surface audited.
    pub surface: &'static str,
    /// One of the DoD's three audit buckets. This audit uses
    /// `mechanically-irrelevant` (the surface does not create one of the three
    /// denied families) and `uncovered-residual` (the surface bypasses the
    /// audited `socket(2)` gate unblocked). The DoD's third bucket, `blocked with
    /// a stable error`, does not apply to any alternate surface here.
    pub classification: &'static str,
    /// Why it carries that classification.
    pub detail: &'static str,
}

/// Enumerate the alternate new-socket / connect dispatch surfaces (`socketpair`,
/// `io_uring`) and classify each against the seccomp socket-creation gate. Every
/// path that is not provably blocked by the gate is recorded as an explicit
/// `uncovered-residual`; the artifact therefore never claims complete
/// new-socket coverage while a path remains uncovered.
pub fn alternate_network_surface_audit() -> Vec<AlternateSurfaceClass> {
    vec![
        AlternateSurfaceClass {
            surface: "socketpair(AF_UNIX)",
            classification: "mechanically-irrelevant",
            detail: "AF_UNIX is not one of the three denied creation families \
                     (AF_INET/AF_INET6/AF_NETLINK), so socketpair(AF_UNIX) is \
                     mechanically irrelevant to the denied domains. AF_UNIX IPC \
                     itself is preserved, proven by the engaged \
                     linux_af_unix_survives_socket_creation_gate test.",
        },
        AlternateSurfaceClass {
            surface: "socketpair(AF_INET | AF_INET6)",
            classification: "mechanically-irrelevant",
            detail: "socketpair(2) requires a connection-oriented family and in \
                     practice only AF_UNIX is usable; an AF_INET/AF_INET6 \
                     socketpair has no defined semantics on Linux, so it is \
                     mechanically irrelevant to the three denied creation domains.",
        },
        AlternateSurfaceClass {
            surface: "io_uring socket/connect/accept",
            classification: "uncovered-residual",
            detail: "io_uring submits socket(2)/connect(2)/accept(2) via the \
                     io_uring_setup(2)/io_uring_enter(2) syscalls and an in-kernel \
                     submission queue, bypassing the audited socket(2) creation \
                     path. Neither io_uring syscall is in the seccomp blocklist, so \
                     io_uring-initiated network operations are an explicit, \
                     documented residual — the artifact must not claim coverage.",
        },
    ]
}

// ---------------------------------------------------------------------------
// LinuxStrictBackend: the production StrictBackend for Linux
// ---------------------------------------------------------------------------

/// Production Linux strict backend. Queries the observed Landlock ABI at
/// construction and reports per-layer availability to the shared
/// [`super::prepare`] resolver. The seccomp L2 socket-creation gate and L3
/// danger blocklist are ABI-independent (always engage when the filter
/// compiles); the Landlock L1 fs layer needs ABI >= 1 and the Landlock TCP
/// bind/connect layer needs ABI >= 4.
#[derive(Debug)]
pub struct LinuxStrictBackend {
    abi: ABI,
    workspace: Arc<Path>,
}

impl LinuxStrictBackend {
    /// Production constructor: probe the kernel's observed Landlock ABI.
    pub fn new(workspace: Arc<Path>) -> Self {
        Self {
            abi: crate::tool::process_tree::observed_landlock_abi(),
            workspace,
        }
    }

    /// Inject the observed ABI instead of probing the kernel (for the capability
    /// matrix tests that cover release/ABI mismatches).
    pub fn with_observed_abi(workspace: Arc<Path>, abi: ABI) -> Self {
        Self { abi, workspace }
    }

    /// The observed Landlock ABI this backend resolved.
    pub fn observed_abi(&self) -> ABI {
        self.abi
    }
}

impl super::StrictBackend for LinuxStrictBackend {
    fn availability(&self, layer: super::SandboxLayer) -> super::LayerAvailability {
        match layer {
            // L3 danger blocklist via seccomp: ABI-independent.
            super::SandboxLayer::Syscalls => super::LayerAvailability::Engaged,
            // L1 fs writes via Landlock: ABI >= 1.
            super::SandboxLayer::Fs => {
                if abi_supports_fs(self.abi) {
                    super::LayerAvailability::Engaged
                } else {
                    super::LayerAvailability::TemporarilyUnavailable {
                        reason: "landlock filesystem rights unavailable (kernel reports ABI 0)"
                            .to_string(),
                    }
                }
            }
            // L2 network: seccomp new-socket gate (always) + Landlock TCP (ABI >= 4).
            // The layer engages only when the TCP bind/connect half is available;
            // an ABI < 4 kernel still gets the seccomp new-socket denial but lacks
            // Landlock TCP confinement, so the network layer is temporarily
            // unavailable as a whole.
            super::SandboxLayer::Network => {
                if abi_supports_tcp(self.abi) {
                    super::LayerAvailability::Engaged
                } else {
                    super::LayerAvailability::TemporarilyUnavailable {
                        reason: format!(
                            "landlock TCP bind/connect needs ABI >= 4 (observed {:?})",
                            self.abi
                        ),
                    }
                }
            }
        }
    }

    fn build_confinement(&self, _workspace: &Path) -> Option<super::Confinement> {
        build_linux_confinement(self.abi, self.workspace.clone())
    }
}
