//! Linux native restriction leaf (Phase 16 task 16.13).
//!
//! Safe parent-side confinement builder that ports the audited Phase 15 Landlock
//! and seccomp behavior onto the opi-sandbox
//! [`Restriction`](crate::policy::Restriction) seam. This module is under
//! `platform/mod.rs`'s `#![forbid(unsafe_code)]`
//! (which propagates and cannot be overridden), so it contains NO `unsafe`: the
//! audited kernel calls — the observed-ABI probe and the `pre_exec` child-setup
//! (seccomp apply, Landlock `restrict_self`, inherited-descriptor closure) —
//! live in [`crate::process_tree`], the crate's FFI home. This module builds the
//! plan from the safe `landlock`/`seccompiler` APIs and wires that FFI.
//!
//! opi-sandbox is FAIL-CLOSED (no `require` flag): if the requested contract
//! cannot be established before spawn, [`LinuxRestriction::prepare`] returns
//! `Err` and the runner refuses the target (Phase 15's fail-open path is NOT
//! ported). The Landlock TCP layer requires ABI >= 4; a `network = deny`
//! request on an older kernel therefore fails closed rather than degrading.
//!
//! The confinement contract (design `### Linux`):
//! - Landlock filesystem writes restricted to the canonical workspace + the
//!   invocation temporary root (host reads/exec remain unrestricted — this is
//!   NOT a read confidentiality boundary);
//! - a fixed seccomp danger-syscall blocklist (always on);
//! - for `network = deny`: the AF_INET/AF_INET6/AF_NETLINK socket-creation
//!   gate (AF_UNIX preserved), `io_uring` setup denial, closure of inherited
//!   nonessential descriptors, and Landlock TCP bind/connect (ABI >= 4).

#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]

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
    SeccompRule, TargetArch,
};
use tokio::process::Command;

use super::Posture;
use crate::policy::{
    AppliedRestriction, ContractStatus, Mechanism, NetworkPolicy, Restriction, RestrictionCtx,
    RestrictionSetupError,
};

/// Stable errno returned for every seccomp-denied syscall (`EPERM`).
const DENY_ERRNO: u32 = libc::EPERM as u32;

/// Socket address families denied at `socket()` creation when `network = deny`.
/// `AF_UNIX` is intentionally absent — Unix-domain IPC must survive (the seccomp
/// gate matches `arg[0]` against this set; classic seccomp cannot dereference
/// `sockaddr`, so only socket CREATION is gated, not connect/bind/sendto).
const DENIED_SOCKET_DOMAINS: &[(&str, i64)] = &[
    ("AF_INET", libc::AF_INET as i64),
    ("AF_INET6", libc::AF_INET6 as i64),
    ("AF_NETLINK", libc::AF_NETLINK as i64),
];

/// The fixed L3 danger blocklist: privileged syscalls denied UNCONDITIONALLY on
/// every network policy (the seccomp baseline). `clone`/`unshare` are
/// intentionally NOT listed (they remain allowed). `iopl`/`ioperm` are x86
/// IO-port syscalls and cfg-gated to `x86_64`; other supported Linux
/// architectures omit them. `io_uring_setup`/`io_uring_enter` are NOT here —
/// design `### Linux` groups io_uring denial under `network = deny`, so they
/// are added in [`build_seccomp_rules`] only when the network layer is engaged.
fn danger_syscalls() -> Vec<(&'static str, i64)> {
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
    // x86-only IO-port syscalls; absent on aarch64/riscv64. Always extend (with
    // an empty slice on non-x86_64) so `v` stays mutably used on every arch.
    v.extend_from_slice(x86_io_port_syscalls());
    v
}

/// The x86 IO-port syscalls (`iopl`/`ioperm`) appended on `x86_64`; empty on
/// every other supported Linux arch. Trampolined through a cfg-selected helper
/// so [`danger_syscalls`] always mutates its vec and stays warning-clean under
/// `-D warnings` on every target.
#[cfg(target_arch = "x86_64")]
fn x86_io_port_syscalls() -> &'static [(&'static str, i64)] {
    &[("iopl", libc::SYS_iopl), ("ioperm", libc::SYS_ioperm)]
}

#[cfg(not(target_arch = "x86_64"))]
fn x86_io_port_syscalls() -> &'static [(&'static str, i64)] {
    &[]
}

/// Build the syscall -> rules map for the deny-overlay. The danger blocklist is
/// always present; the socket-creation gate and the io_uring setup denial are
/// added only when `network_enabled` (design `### Linux` groups both under
/// `network = deny`). Each danger syscall maps to an empty rule vector (matches
/// regardless of arguments); the socket gate maps `SYS_socket` to one
/// OR-bound equality rule per denied family on `arg[0]` (the scalar domain).
fn build_seccomp_rules(
    network_enabled: bool,
) -> Result<BTreeMap<i64, Vec<SeccompRule>>, seccompiler::Error> {
    let mut rules: BTreeMap<i64, Vec<SeccompRule>> = BTreeMap::new();

    // Danger blocklist (always on).
    for (_name, sysno) in danger_syscalls() {
        rules.insert(sysno, Vec::new());
    }

    if network_enabled {
        // socket(domain) deny: arg[0] == each denied family. AF_UNIX has no
        // rule, so its creation falls through to the Allow mismatch action.
        let mut socket_rules: Vec<SeccompRule> = Vec::with_capacity(DENIED_SOCKET_DOMAINS.len());
        for (_name, domain) in DENIED_SOCKET_DOMAINS.iter() {
            let cond = SeccompCondition::new(
                0,
                SeccompCmpArgLen::Dword,
                SeccompCmpOp::Eq,
                *domain as u64,
            )?;
            socket_rules.push(SeccompRule::new(vec![cond])?);
        }
        rules.insert(libc::SYS_socket, socket_rules);
        // io_uring is a socket/network bypass vector (it submits socket/connect
        // via io_uring_setup/io_uring_enter, sidestepping the socket() gate).
        // Deny setup unconditionally (empty rule) for network = deny.
        rules.insert(libc::SYS_io_uring_setup, Vec::new());
        rules.insert(libc::SYS_io_uring_enter, Vec::new());
    }

    Ok(rules)
}

/// Compile the default-allow / match-deny seccomp BPF program for the target
/// arch. `network_enabled` selects the socket+io_uring gate. Does not touch the
/// kernel.
fn compile_seccomp(
    arch: TargetArch,
    network_enabled: bool,
) -> Result<BpfProgram, seccompiler::Error> {
    let rules = build_seccomp_rules(network_enabled)?;
    let filter = SeccompFilter::new(
        rules,
        SeccompAction::Allow,
        SeccompAction::Errno(DENY_ERRNO),
        arch,
    )?;
    // BpfProgram::try_from(SeccompFilter) returns BackendError; lift it into
    // seccompiler::Error so compile_seccomp has a single error type.
    filter.try_into().map_err(seccompiler::Error::Backend)
}

/// Validate the seccomp target arch. Only the verified release architectures
/// (`x86_64`, `aarch64`) are accepted; any other arch fails closed (the posture
/// reports unsupported).
fn target_arch(arch: &str) -> Result<TargetArch, ()> {
    if !matches!(arch, "x86_64" | "aarch64") {
        return Err(());
    }
    arch.try_into().map_err(|_| ())
}

// ---------------------------------------------------------------------------
// Landlock capability model
// ---------------------------------------------------------------------------

/// Whether the observed ABI grants Landlock filesystem rights (ABI >= 1).
fn abi_supports_fs(abi: ABI) -> bool {
    !matches!(abi, ABI::Unsupported)
}

/// Whether the observed ABI grants Landlock TCP bind/connect rights (ABI >= 4).
fn abi_supports_tcp(abi: ABI) -> bool {
    !AccessNet::from_all(abi).is_empty()
}

/// The filesystem WRITE rights a Landlock ruleset governs for `abi`: every fs
/// right that creates/modifies/removes content or structure. `ReadFile`,
/// `ReadDir`, and `Execute` are deliberately removed so a confined child can
/// still read system files/libraries and exec its shell — only writes are
/// confined to the granted paths. Uses the OBSERVED abi (not a hardcoded V4) so
/// confinement is not weakened on ABI > 4 kernels.
fn landlock_write_rights(abi: ABI) -> BitFlags<AccessFs> {
    let mut w = AccessFs::from_all(abi);
    w.remove(AccessFs::ReadFile);
    w.remove(AccessFs::ReadDir);
    w.remove(AccessFs::Execute);
    w
}

/// Build the Landlock filesystem ruleset granting write rights beneath the
/// canonical workspace and the invocation temporary root.
fn build_landlock_fs_ruleset(
    abi: ABI,
    workspace: &Path,
    temp_root: &Path,
) -> io::Result<RulesetCreated> {
    let write_rights = landlock_write_rights(abi);
    let ruleset = Ruleset::default()
        .handle_access(write_rights)
        .map_err(map_landlock_err("handle_access fs"))?
        .create()
        .map_err(map_landlock_err("create ruleset"))?;
    let ruleset = ruleset
        .add_rule(PathBeneath::new(
            PathFd::new(workspace).map_err(map_pathfd_err("workspace"))?,
            write_rights,
        ))
        .map_err(map_landlock_err("add_rule workspace"))?;
    ruleset
        .add_rule(PathBeneath::new(
            PathFd::new(temp_root).map_err(map_pathfd_err("invocation temp root"))?,
            write_rights,
        ))
        .map_err(map_landlock_err("add_rule temp"))
}

/// Build the Landlock TCP ruleset (bind/connect by port) for `abi` (>= 4).
fn build_landlock_network_ruleset(abi: ABI) -> io::Result<RulesetCreated> {
    Ruleset::default()
        .handle_access(AccessNet::from_all(abi))
        .map_err(map_landlock_err("handle_access net"))?
        .create()
        .map_err(map_landlock_err("create network ruleset"))
}

fn map_landlock_err(
    step: &'static str,
) -> impl FnOnce(landlock::RulesetError) -> io::Error + 'static {
    move |e| io::Error::other(format!("landlock {step}: {e:?}"))
}

fn map_pathfd_err(step: &'static str) -> impl FnOnce(landlock::PathFdError) -> io::Error + 'static {
    move |e| io::Error::other(format!("landlock {step}: {e:?}"))
}

// ---------------------------------------------------------------------------
// LinuxRestriction: the production native restriction for Linux
// ---------------------------------------------------------------------------

/// The Linux native [`Restriction`]: Landlock filesystem-write + TCP confinement
/// plus a fixed seccomp deny-overlay. Probes the observed Landlock ABI and
/// precompiles the seccomp BPF ONCE at construction; [`Restriction::prepare`]
/// rebuilds only the per-spawn Landlock rulesets (which `restrict_self`
/// consumes). Fail-closed: any parent-side build error or an ABI too old for the
/// requested network policy returns `Err` before spawn.
pub(crate) struct LinuxRestriction {
    abi: ABI,
    /// `network = deny`: danger blocklist + socket gate + io_uring denial.
    seccomp_deny: Arc<BpfProgram>,
    /// `network = allow`: danger blocklist only.
    seccomp_allow: Arc<BpfProgram>,
}

impl LinuxRestriction {
    /// Build the restriction for an observed ABI. Returns `None` if the seccomp
    /// target arch is unsupported or the BPF will not compile — the platform
    /// cannot establish the L3 baseline, so the posture reports unsupported.
    fn new(abi: ABI) -> Option<Self> {
        let arch = target_arch(std::env::consts::ARCH).ok()?;
        let seccomp_deny = Arc::new(compile_seccomp(arch, true).ok()?);
        let seccomp_allow = Arc::new(compile_seccomp(arch, false).ok()?);
        Some(Self {
            abi,
            seccomp_deny,
            seccomp_allow,
        })
    }
}

impl Restriction for LinuxRestriction {
    fn prepare(
        &self,
        cmd: &mut Command,
        ctx: &RestrictionCtx<'_>,
    ) -> Result<AppliedRestriction, RestrictionSetupError> {
        if ctx.setup_cancelled() {
            return Err(RestrictionSetupError::Failed("setup-cancelled"));
        }
        let deny = matches!(ctx.network, NetworkPolicy::Deny);
        // Fail-closed: network = deny requires the Landlock TCP layer (ABI >= 4).
        // Phase 15 fail-opens on an older kernel; opi-sandbox has no `require`
        // flag and is fail-closed, so this returns Err -> RestrictionSetup ->
        // the runner refuses the target before spawn.
        if deny && !abi_supports_tcp(self.abi) {
            return Err(RestrictionSetupError::Failed(
                "landlock-tcp-unavailable-for-network-deny",
            ));
        }
        let bpf = if deny {
            Arc::clone(&self.seccomp_deny)
        } else {
            Arc::clone(&self.seccomp_allow)
        };
        // Landlock fs ruleset: workspace + invocation temp root.
        let fs_ruleset = build_landlock_fs_ruleset(self.abi, ctx.workspace, ctx.temp_root)
            .map_err(|_| RestrictionSetupError::Failed("landlock-fs-ruleset"))?;
        if ctx.setup_cancelled() {
            return Err(RestrictionSetupError::Failed("setup-cancelled"));
        }
        // Landlock TCP ruleset: network = deny only (ABI >= 4 checked above).
        let network_ruleset = if deny {
            Some(
                build_landlock_network_ruleset(self.abi)
                    .map_err(|_| RestrictionSetupError::Failed("landlock-network-ruleset"))?,
            )
        } else {
            None
        };
        if ctx.setup_cancelled() {
            return Err(RestrictionSetupError::Failed("setup-cancelled"));
        }
        // Wire the audited pre_exec child-setup (seccomp -> restrict_self ->
        // fd closure). The child is not released on a pre_exec failure: spawn
        // fails -> SpawnFailed (fail-closed).
        crate::process_tree::install_child_confinement(
            cmd,
            bpf,
            Some(fs_ruleset),
            network_ruleset,
            deny,
        );
        if ctx.setup_cancelled() {
            return Err(RestrictionSetupError::Failed("setup-cancelled"));
        }
        Ok(AppliedRestriction {
            mechanism: Mechanism::Landlock,
            contract: ContractStatus::Restricted,
        })
    }
}

// ---------------------------------------------------------------------------
// posture()
// ---------------------------------------------------------------------------

/// The Linux posture: `Supported` (Landlock + seccomp) when the observed ABI
/// grants filesystem writes (ABI >= 1) and the seccomp L3 baseline compiles for
/// the host arch; otherwise `Unsupported` with an honest limitation. Reports
/// both `Landlock` and `Seccomp` mechanisms on a supported host and carries
/// honest per-mechanism caveats for `doctor`.
pub(crate) fn posture() -> Posture {
    let abi = crate::process_tree::observed_landlock_abi();
    let restriction = LinuxRestriction::new(abi);
    let supported = abi_supports_fs(abi) && restriction.is_some();
    Posture {
        supported,
        mechanisms: if supported {
            vec![Mechanism::Landlock, Mechanism::Seccomp]
        } else {
            Vec::new()
        },
        limitations: limitations(abi, supported),
        restriction: restriction.map(|r| Arc::new(r) as Arc<dyn Restriction>),
    }
}

/// Honest per-platform caveats reported by `doctor`. Distinguishes the supported
/// case (specific confinement caveats) from the unsupported cases (Landlock
/// absent vs. seccomp arch unsupported).
fn limitations(abi: ABI, supported: bool) -> Vec<String> {
    if !supported {
        return vec![if matches!(abi, ABI::Unsupported) {
            "Landlock is absent or disabled on this kernel; the requested restriction cannot be established, so the target is refused before start".to_string()
        } else {
            "the host seccomp architecture is unsupported; the requested restriction cannot be established, so the target is refused before start".to_string()
        }];
    }
    let mut v = vec![
        "host reads remain unrestricted; this is not a read or environment confidentiality boundary".to_string(),
        "seccomp gates socket() creation only; connect/bind/sendto are not address-filterable, and (for network=deny) inherited nonessential descriptors are closed at start".to_string(),
        "AF_UNIX socket creation and local IPC are preserved".to_string(),
    ];
    if abi_supports_tcp(abi) {
        v.push("Landlock TCP bind/connect are enforced for network=deny".to_string());
    } else {
        v.push("network=deny is unavailable below Landlock ABI 4 and fails closed".to_string());
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const LANDLOCK_NETWORK_PROBE: &str = "OPI_SANDBOX_LANDLOCK_NETWORK_PROBE";
    const LANDLOCK_UNIX_PATH: &str = "OPI_SANDBOX_LANDLOCK_UNIX_PATH";

    struct LandlockNetworkTestRestriction {
        abi: ABI,
        seccomp_baseline: Arc<BpfProgram>,
        install_network_layer: bool,
    }

    impl Restriction for LandlockNetworkTestRestriction {
        fn prepare(
            &self,
            cmd: &mut Command,
            ctx: &RestrictionCtx<'_>,
        ) -> Result<AppliedRestriction, RestrictionSetupError> {
            let fs_ruleset = build_landlock_fs_ruleset(self.abi, ctx.workspace, ctx.temp_root)
                .map_err(|_| RestrictionSetupError::Failed("landlock-fs-ruleset"))?;
            let network_ruleset = self
                .install_network_layer
                .then(|| build_landlock_network_ruleset(self.abi))
                .transpose()
                .map_err(|_| RestrictionSetupError::Failed("landlock-network-ruleset"))?;
            crate::process_tree::install_child_confinement(
                cmd,
                Arc::clone(&self.seccomp_baseline),
                Some(fs_ruleset),
                network_ruleset,
                false,
            );
            Ok(AppliedRestriction {
                mechanism: Mechanism::Landlock,
                contract: ContractStatus::Restricted,
            })
        }
    }

    async fn run_landlock_network_probe(
        abi: ABI,
        mode: &str,
        install_network_layer: bool,
    ) -> crate::runner::SandboxResult {
        use std::ffi::OsString;
        use std::time::Duration;

        use futures_util::StreamExt as _;
        use opi_protocol::execution::v1::EnvInherit;

        use crate::policy::{Profile, SandboxPolicy};
        use crate::runner::{SandboxEvent, SandboxRequest, SandboxRunner, StdinPolicy};

        let workspace = tempfile::tempdir().expect("Landlock network workspace");
        let socket_path = workspace.path().join("local.sock");
        let baseline = LinuxRestriction::new(abi).expect("host seccomp architecture");
        let restriction = LandlockNetworkTestRestriction {
            abi,
            seccomp_baseline: baseline.seccomp_allow,
            install_network_layer,
        };
        let mut env_additions = BTreeMap::new();
        env_additions.insert(OsString::from(LANDLOCK_NETWORK_PROBE), OsString::from(mode));
        env_additions.insert(
            OsString::from(LANDLOCK_UNIX_PATH),
            socket_path.into_os_string(),
        );
        let request = SandboxRequest {
            program: std::env::current_exe().expect("current test executable"),
            args: [
                OsString::from("--exact"),
                OsString::from("platform::linux::tests::landlock_tcp_rules_are_independent_of_the_seccomp_socket_gate"),
                OsString::from("--nocapture"),
            ]
            .into_iter()
            .collect(),
            workspace: workspace.path().to_path_buf(),
            cwd: workspace.path().to_path_buf(),
            timeout: Duration::from_secs(10),
            env_inherit: EnvInherit::Inherit,
            env_additions,
            stdin: StdinPolicy::Null,
            cancel: None,
        };
        let runner = SandboxRunner::new(
            SandboxPolicy::new(Profile::WorkspaceWrite, NetworkPolicy::Allow),
            Arc::new(restriction),
        );
        let mut run = runner.run(request).expect("Landlock probe starts");
        assert!(matches!(
            run.next().await,
            Some(SandboxEvent::Started { .. })
        ));
        loop {
            match run.next().await {
                Some(SandboxEvent::Completed(result)) => return result,
                Some(_) => {}
                None => panic!("Landlock probe ended without completion"),
            }
        }
    }

    #[derive(Clone, Copy)]
    struct TestSeccompData {
        syscall: i64,
        arch: u32,
        instruction_pointer: u64,
        args: [u64; 6],
    }

    fn evaluate_classic_bpf(program: &BpfProgram, data: TestSeccompData) -> u32 {
        const LD_W_ABS: u16 = 0x20;
        const ALU_AND_K: u16 = 0x54;
        const JMP_JA: u16 = 0x05;
        const JMP_JEQ_K: u16 = 0x15;
        const JMP_JGT_K: u16 = 0x25;
        const JMP_JGE_K: u16 = 0x35;
        const RET_K: u16 = 0x06;

        let mut accumulator = 0_u32;
        let mut pc = 0_usize;
        for _ in 0..=program.len() {
            let instruction = program
                .get(pc)
                .unwrap_or_else(|| panic!("classic-BPF jumped outside program at pc={pc}"));
            match instruction.code {
                LD_W_ABS => {
                    accumulator = seccomp_data_word(data, instruction.k);
                    pc += 1;
                }
                ALU_AND_K => {
                    accumulator &= instruction.k;
                    pc += 1;
                }
                JMP_JA => pc += 1 + instruction.k as usize,
                JMP_JEQ_K => {
                    pc += 1 + if accumulator == instruction.k {
                        instruction.jt as usize
                    } else {
                        instruction.jf as usize
                    };
                }
                JMP_JGT_K => {
                    pc += 1 + if accumulator > instruction.k {
                        instruction.jt as usize
                    } else {
                        instruction.jf as usize
                    };
                }
                JMP_JGE_K => {
                    pc += 1 + if accumulator >= instruction.k {
                        instruction.jt as usize
                    } else {
                        instruction.jf as usize
                    };
                }
                RET_K => return instruction.k,
                code => panic!(
                    "unsupported classic-BPF instruction code {code:#x} at pc={pc}: {instruction:?}"
                ),
            }
        }
        panic!("classic-BPF program did not return within its instruction bound")
    }

    fn seccomp_data_word(data: TestSeccompData, offset: u32) -> u32 {
        match offset {
            0 => data.syscall as u32,
            4 => data.arch,
            8 => data.instruction_pointer as u32,
            12 => (data.instruction_pointer >> 32) as u32,
            16..=63 if (offset - 16).is_multiple_of(4) => {
                let relative = offset - 16;
                let argument = data.args[(relative / 8) as usize];
                if relative.is_multiple_of(8) {
                    argument as u32
                } else {
                    (argument >> 32) as u32
                }
            }
            _ => panic!("invalid seccomp_data word offset {offset}"),
        }
    }

    fn host_audit_arch() -> u32 {
        match std::env::consts::ARCH {
            "x86_64" => 0xc000_003e,
            "aarch64" => 0xc000_00b7,
            arch => panic!("unsupported test architecture: {arch}"),
        }
    }

    fn seccomp_data(syscall: i64) -> TestSeccompData {
        TestSeccompData {
            syscall,
            arch: host_audit_arch(),
            instruction_pointer: 0,
            args: [0; 6],
        }
    }

    #[test]
    fn compiled_bpf_returns_exact_actions_for_canonical_syscalls() {
        let arch = target_arch(std::env::consts::ARCH).expect("host arch supported");
        let baseline = compile_seccomp(arch, false).expect("compile baseline filter");
        let denied = u32::from(SeccompAction::Errno(DENY_ERRNO));
        let allowed = u32::from(SeccompAction::Allow);

        for (name, syscall) in danger_syscalls() {
            assert_eq!(
                evaluate_classic_bpf(&baseline, seccomp_data(syscall)),
                denied,
                "compiled baseline BPF must return EPERM for {name}"
            );
        }
        for (name, syscall) in [
            ("getpid", libc::SYS_getpid),
            ("read", libc::SYS_read),
            ("write", libc::SYS_write),
            ("exit_group", libc::SYS_exit_group),
            ("unshare", libc::SYS_unshare),
            ("io_uring_setup", libc::SYS_io_uring_setup),
        ] {
            assert_eq!(
                evaluate_classic_bpf(&baseline, seccomp_data(syscall)),
                allowed,
                "compiled baseline BPF must allow representative syscall {name}"
            );
        }

        let mut wrong_arch = seccomp_data(libc::SYS_read);
        wrong_arch.arch = 0;
        assert_eq!(
            evaluate_classic_bpf(&baseline, wrong_arch),
            libc::SECCOMP_RET_KILL_PROCESS,
            "compiled BPF architecture guard must kill a mismatched architecture"
        );

        let network_deny = compile_seccomp(arch, true).expect("compile network-deny filter");
        for (name, syscall) in [
            ("io_uring_setup", libc::SYS_io_uring_setup),
            ("io_uring_enter", libc::SYS_io_uring_enter),
        ] {
            assert_eq!(
                evaluate_classic_bpf(&network_deny, seccomp_data(syscall)),
                denied,
                "compiled network-deny BPF must return EPERM for {name}"
            );
        }
        for (name, domain) in DENIED_SOCKET_DOMAINS {
            let mut data = seccomp_data(libc::SYS_socket);
            data.args[0] = *domain as u64;
            assert_eq!(
                evaluate_classic_bpf(&network_deny, data),
                denied,
                "compiled network-deny BPF must return EPERM for socket({name})"
            );
        }
        let mut unix_socket = seccomp_data(libc::SYS_socket);
        unix_socket.args[0] = libc::AF_UNIX as u64;
        assert_eq!(
            evaluate_classic_bpf(&network_deny, unix_socket),
            allowed,
            "compiled network-deny BPF must preserve AF_UNIX"
        );
        for (name, syscall) in [
            ("getpid", libc::SYS_getpid),
            ("read", libc::SYS_read),
            ("write", libc::SYS_write),
            ("unshare", libc::SYS_unshare),
        ] {
            assert_eq!(
                evaluate_classic_bpf(&network_deny, seccomp_data(syscall)),
                allowed,
                "compiled network-deny BPF must allow nonblocked syscall {name}"
            );
        }
    }

    /// The danger blocklist is exactly the audited fixed set; io_uring is NOT
    /// in it (it is network-tied, asserted by `build_seccomp_rules`).
    #[test]
    fn danger_blocklist_is_fixed_and_io_uring_free() {
        let mut names: Vec<&str> = danger_syscalls().iter().map(|(n, _)| *n).collect();
        let mut expected = vec![
            "open_by_handle_at",
            "bpf",
            "perf_event_open",
            "ptrace",
            "kexec_load",
            "kexec_file_load",
            "reboot",
            "init_module",
            "finit_module",
            "delete_module",
            "swapon",
            "swapoff",
            "acct",
            "settimeofday",
        ];
        expected.extend(x86_io_port_syscalls().iter().map(|(name, _)| *name));
        names.sort_unstable();
        expected.sort_unstable();
        assert_eq!(
            names, expected,
            "danger blocklist additions and removals require an explicit contract review"
        );
    }

    /// The denied socket domains are exactly INET/INET6/NETLINK (AF_UNIX kept).
    #[test]
    fn denied_socket_domains_keep_af_unix() {
        let domains: Vec<i64> = DENIED_SOCKET_DOMAINS.iter().map(|(_, d)| *d).collect();
        assert!(domains.contains(&(libc::AF_INET as i64)));
        assert!(domains.contains(&(libc::AF_INET6 as i64)));
        assert!(domains.contains(&(libc::AF_NETLINK as i64)));
        assert!(!domains.contains(&(libc::AF_UNIX as i64)));
    }

    /// Only the verified release architectures are accepted.
    #[test]
    fn target_arch_accepts_only_verified_architectures() {
        assert!(target_arch("x86_64").is_ok());
        assert!(target_arch("aarch64").is_ok());
        assert!(target_arch("riscv64").is_err());
        assert!(target_arch("mips64").is_err());
        assert!(target_arch("unknown").is_err());
    }

    /// The network-enabled seccomp program installs the socket gate and io_uring
    /// denial; the allow program installs neither (danger blocklist only).
    #[test]
    fn seccomp_network_layer_gates_socket_and_io_uring() {
        let arch = target_arch(std::env::consts::ARCH).expect("host arch supported");
        // The compiled BPF is opaque, so assert at the rules layer.
        let deny_rules = build_seccomp_rules(true).expect("deny rules build");
        let allow_rules = build_seccomp_rules(false).expect("allow rules build");
        assert!(
            deny_rules.contains_key(&libc::SYS_socket),
            "deny installs socket gate"
        );
        assert!(
            deny_rules.contains_key(&libc::SYS_io_uring_setup),
            "deny installs io_uring_setup denial"
        );
        assert!(
            !allow_rules.contains_key(&libc::SYS_socket),
            "allow does not install the socket gate"
        );
        assert!(
            !allow_rules.contains_key(&libc::SYS_io_uring_setup),
            "allow does not deny io_uring"
        );
        // Both compile for the host arch.
        assert!(compile_seccomp(arch, true).is_ok());
        assert!(compile_seccomp(arch, false).is_ok());
    }

    #[tokio::test]
    async fn landlock_tcp_rules_are_independent_of_the_seccomp_socket_gate() {
        use std::net::{TcpListener, TcpStream};
        use std::os::unix::net::UnixListener;

        if let Ok(mode) = std::env::var(LANDLOCK_NETWORK_PROBE) {
            match mode.as_str() {
                "tcp-bind-denied" => {
                    let error = TcpListener::bind(("127.0.0.1", 0))
                        .expect_err("Landlock must deny TCP bind");
                    assert_eq!(error.raw_os_error(), Some(libc::EACCES));
                }
                "tcp-connect-denied" => {
                    let error = TcpStream::connect(("127.0.0.1", 9))
                        .expect_err("Landlock must deny TCP connect");
                    assert_eq!(error.raw_os_error(), Some(libc::EACCES));
                }
                "tcp-bind-allowed" => {
                    TcpListener::bind(("127.0.0.1", 0))
                        .expect("baseline seccomp must preserve TCP bind");
                }
                "unix-preserved" => {
                    let path = std::env::var_os(LANDLOCK_UNIX_PATH)
                        .map(PathBuf::from)
                        .expect("Unix socket path");
                    UnixListener::bind(path).expect("Landlock TCP layer must preserve AF_UNIX");
                }
                other => panic!("unknown Landlock network probe mode: {other}"),
            }
            return;
        }

        let abi = crate::process_tree::observed_landlock_abi();
        if !abi_supports_tcp(abi) {
            eprintln!("skipping Landlock TCP isolation test: ABI < 4");
            return;
        }

        for mode in ["tcp-bind-denied", "tcp-connect-denied", "unix-preserved"] {
            let result = run_landlock_network_probe(abi, mode, true).await;
            assert_eq!(
                result.outcome,
                crate::runner::SandboxOutcome::Exited { code: Some(0) },
                "Landlock-only probe {mode} failed: stderr={}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        let control = run_landlock_network_probe(abi, "tcp-bind-allowed", false).await;
        assert_eq!(
            control.outcome,
            crate::runner::SandboxOutcome::Exited { code: Some(0) },
            "baseline control failed: stderr={}",
            String::from_utf8_lossy(&control.stderr)
        );
    }
}
