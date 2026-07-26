//! Phase 15 task 15.5.2 — pure backend tests for the Linux seccomp + Landlock
//! substrate (`src/sandbox/linux.rs`).
//!
//! **Substrate**: the module is included directly via `#[path]` and is NOT yet
//! wired into the production `sandbox::prepare_production` dispatcher — that
//! runtime attach is task 15.5.3. These tests pin the seccomp deny-overlay
//! policy and the Landlock capability *model* without spawning a confined
//! command. The model is host-independent (Landlock ABI is injected); only the
//! seccomp compile/apply exercises run on a real Linux kernel.

#![cfg(target_os = "linux")]

// 15.5.3: linux.rs now references its parent sandbox module (`super::StrictBackend`,
// `super::Confinement`, ...), so it can no longer be #[path]-included here. Use
// the real public module instead — every item exercised below is `pub`.
use opi_coding_agent::sandbox::linux;

use landlock::{ABI, AccessFs, AccessNet, make_bitflags};

/// The danger blocklist pins the exact L3 syscall set and keeps `clone`/
/// `unshare` allowed.
#[test]
fn danger_blocklist_pins_exact_syscalls_and_allows_clone_unshare() {
    let block = linux::danger_syscalls();
    let names: Vec<&str> = block.iter().map(|(n, _)| *n).collect();
    let expected = [
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
        "iopl",
        "ioperm",
        "acct",
        "settimeofday",
    ];
    for name in expected {
        assert!(names.contains(&name), "danger blocklist missing {name}");
    }
    // clone and unshare MUST remain allowed (not in the blocklist).
    assert!(!names.contains(&"clone"), "clone must remain allowed");
    assert!(!names.contains(&"unshare"), "unshare must remain allowed");
    // Every entry resolves to a real (positive) syscall number on this arch.
    for (_name, sysno) in &block {
        assert!(*sysno > 0, "danger syscall resolved to non-positive number");
    }
}

/// The socket-creation gate denies exactly AF_INET/AF_INET6/AF_NETLINK and
/// preserves AF_UNIX (the Unix-domain IPC path must stay usable).
#[test]
fn socket_creation_gate_denies_inet_inet6_netlink_preserves_unix() {
    let denied: Vec<i64> = linux::DENIED_SOCKET_DOMAINS
        .iter()
        .map(|(_, v)| *v)
        .collect();
    assert!(denied.contains(&(libc::AF_INET as i64)), "AF_INET denied");
    assert!(denied.contains(&(libc::AF_INET6 as i64)), "AF_INET6 denied");
    assert!(
        denied.contains(&(libc::AF_NETLINK as i64)),
        "AF_NETLINK denied"
    );
    assert!(
        !denied.contains(&(libc::AF_UNIX as i64)),
        "AF_UNIX must NOT be denied (IPC must survive)"
    );
    assert_eq!(
        linux::DENIED_SOCKET_DOMAINS.len(),
        3,
        "exactly three socket families denied"
    );
}

/// The built filter encodes the socket-creation gate and the danger blocklist,
/// keeps `clone`/`unshare` rule-less, applies the Allow/Errno(EPERM) action
/// pair, and compiles to a BPF program whose immediates include each denied
/// domain value. This proves the policy encoding (not just that a program
/// compiles); a zero-rule, swapped-action, or missing-family filter would fail.
/// Behavioral forked-child proof (EPERM on a real socket(AF_INET)/bpf/ptrace;
/// AF_UNIX + clone/unshare succeed) is owned by 15.5.3 and is not pulled
/// forward into this substrate task.
#[test]
fn built_filter_encodes_socket_gate_danger_blocklist_and_actions() {
    let arch: seccompiler::TargetArch = std::env::consts::ARCH
        .try_into()
        .expect("host arch maps to a seccompiler TargetArch");

    // --- rule-map structure (SeccompFilter.rules is private, so assert here) ---
    let rules = linux::build_seccomp_rules().expect("rules build");
    // socket gate: exactly one OR-bound rule per denied family.
    assert_eq!(
        rules.get(&libc::SYS_socket).map(|v| v.len()),
        Some(linux::DENIED_SOCKET_DOMAINS.len()),
        "SYS_socket must have one rule per denied family",
    );
    // every danger syscall maps to an empty rule vector (unconditional deny).
    for (_name, sysno) in linux::danger_syscalls() {
        assert_eq!(
            rules.get(&sysno).map(|v| v.len()),
            Some(0),
            "danger syscall {sysno} must map to an empty rule vector",
        );
    }
    // clone/unshare have no rule (they remain allowed).
    assert!(
        !rules.contains_key(&libc::SYS_clone),
        "clone must have no deny rule",
    );
    assert!(
        !rules.contains_key(&libc::SYS_unshare),
        "unshare must have no deny rule",
    );
    // exactly SYS_socket + the danger syscalls, nothing stray.
    assert_eq!(
        rules.len(),
        1 + linux::danger_syscalls().len(),
        "rule map must contain only the socket gate + danger syscalls",
    );

    // --- actions: default-allow (mismatch) / match-deny (Errno EPERM) ---
    assert_eq!(
        linux::FILTER_MISMATCH_ACTION,
        seccompiler::SeccompAction::Allow
    );
    assert_eq!(
        linux::FILTER_MATCH_ACTION,
        seccompiler::SeccompAction::Errno(linux::DENY_ERRNO),
    );

    // --- compiled BPF encodes the gate: each denied domain appears as a
    //     comparison immediate (arg[0] == domain). ---
    let filter = linux::build_seccomp_filter(arch).expect("filter builds");
    let bpf = linux::compile_filter(filter).expect("filter compiles to BPF");
    assert!(!bpf.is_empty(), "compiled BPF program must be non-empty");
    let immediates: Vec<u32> = bpf.iter().map(|f| f.k).collect();
    for (_name, domain) in linux::DENIED_SOCKET_DOMAINS {
        assert!(
            immediates.contains(&(*domain as u32)),
            "compiled BPF must compare socket arg[0] against denied domain value {domain}",
        );
    }
}

/// The Landlock ABI-4 filesystem-rights model is captured (the L1
/// workspace-write layer's capability surface). The expected value is built
/// independently of `from_all(ABI::V4)` (named rights via `make_bitflags!`), so
/// the test detects a real semantic change rather than restating the impl.
#[test]
fn landlock_fs_rights_model_captured() {
    let fs = linux::landlock_fs_rights();
    assert!(
        !fs.is_empty(),
        "ABI-4 filesystem rights set must be non-empty"
    );
    // Core ABI-4 fs rights an L1 workspace-write layer (15.5.3) hinges on:
    // WriteFile/Execute/ReadFile/ReadDir plus Refer (ABI 2) and Truncate (ABI 3).
    let core = make_bitflags!(AccessFs::{
        WriteFile | Execute | ReadFile | ReadDir | Refer | Truncate
    });
    assert!(
        fs.contains(core),
        "ABI-4 fs rights must include the core write/exec/read/rename/truncate subset",
    );
    // IoctlDev is the only filesystem right added after ABI 4 (ABI 5 / Linux
    // 6.10); the ABI-4 surface must NOT include it.
    assert!(
        !fs.contains(AccessFs::IoctlDev),
        "ABI-4 fs rights must exclude IoctlDev (added at ABI 5)",
    );
}

/// Applying an empty filter translates to a stable error and spawns no command.
/// `apply_filter` rejects an empty program before touching `prctl`/`seccomp`,
/// so this exercises the error-translation path without confining anything.
#[test]
fn apply_empty_filter_translates_to_stable_error_without_spawning_command() {
    let empty: seccompiler::BpfProgram = Vec::new();
    let err = linux::apply_raw_filter(&empty).unwrap_err();
    assert_eq!(err, linux::StableErrno::EmptyFilter);
}

/// The Landlock TCP capability follows the observed ABI (>= V4 / Linux 6.7),
/// and the composite capability report distinguishes the seccomp socket-creation
/// layer from the Landlock TCP bind/connect layer.
#[test]
fn landlock_capability_uses_observed_abi_and_distinguishes_layers() {
    // ABI gate: V3 has no network rights; V4+ enables TCP bind/connect.
    assert!(!linux::landlock_tcp_capability(ABI::V3).tcp_bind_connect);
    assert!(linux::landlock_tcp_capability(ABI::V4).tcp_bind_connect);
    assert!(linux::landlock_tcp_capability(ABI::V7).tcp_bind_connect);

    // The TCP rights set is exactly BindTcp | ConnectTcp (by port; ABI-4 model).
    let expected = make_bitflags!(AccessNet::{BindTcp | ConnectTcp});
    assert_eq!(linux::landlock_tcp_rights(), expected);

    // Composite report distinguishes the two layers as separate fields.
    let cap = linux::linux_strict_capability(ABI::V4);
    assert!(
        !cap.seccomp_socket_creation.denied_families.is_empty(),
        "seccomp socket-creation layer must carry the denied families"
    );
    assert_eq!(
        cap.seccomp_socket_creation.deny_errno,
        linux::DENY_ERRNO,
        "socket-creation layer carries the stable deny errno"
    );
    assert!(
        cap.landlock_tcp_bind_connect.tcp_bind_connect,
        "landlock TCP layer must report the ABI-4 capability"
    );
    // Distinctness is structural: the two layers are separate fields, not one.
    let cap_v3 = linux::linux_strict_capability(ABI::V3);
    assert!(
        !cap_v3.landlock_tcp_bind_connect.tcp_bind_connect,
        "V3 kernel has no landlock TCP capability"
    );
    // The seccomp layer is unaffected by the Landlock ABI.
    assert_eq!(
        cap.seccomp_socket_creation, cap_v3.seccomp_socket_creation,
        "seccomp socket-creation layer is independent of the landlock ABI"
    );
}

/// The alternate-network-surface audit classifies every non-`socket(2)` dispatch
/// path (`socketpair`, io_uring) and never claims complete new-socket coverage
/// while a path remains uncovered (Phase 15 task 15.5.3 DoD).
#[test]
fn linux_alternate_network_surface_audit() {
    let audit = linux::alternate_network_surface_audit();
    assert!(!audit.is_empty(), "audit must enumerate alternate surfaces");

    // Every classification is one of the DoD audit buckets this task uses.
    for c in &audit {
        assert!(
            matches!(
                c.classification,
                "mechanically-irrelevant" | "uncovered-residual"
            ),
            "unexpected classification `{}` for {}",
            c.classification,
            c.surface
        );
        assert!(!c.detail.is_empty(), "audit entry needs a detail");
    }

    // socketpair(AF_UNIX) is mechanically irrelevant: AF_UNIX is not one of the
    // three denied creation families (it is preserved, but irrelevant to the
    // denied domains).
    let unix = audit
        .iter()
        .find(|c| c.surface.contains("socketpair(AF_UNIX)"))
        .expect("audit must cover socketpair(AF_UNIX)");
    assert_eq!(
        unix.classification, "mechanically-irrelevant",
        "socketpair(AF_UNIX) is mechanically irrelevant to the denied families"
    );

    // io_uring socket/connect/accept is an EXPLICIT uncovered residual: the
    // artifact must not claim full new-socket coverage while it stands.
    let io_uring = audit
        .iter()
        .find(|c| c.surface.contains("io_uring"))
        .expect("audit must cover io_uring");
    assert_eq!(
        io_uring.classification, "uncovered-residual",
        "io_uring-initiated network ops are an explicit residual, not covered"
    );

    // The audit as a whole carries at least one uncovered residual (no
    // completeness claim).
    assert!(
        audit
            .iter()
            .any(|c| c.classification == "uncovered-residual"),
        "the audit must keep at least one explicit uncovered residual"
    );
}
