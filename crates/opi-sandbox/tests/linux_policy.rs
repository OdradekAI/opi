//! Native Linux restriction contract (Phase 16 task 16.13, SC16-10).
//!
//! Drives the REAL `opi-sandbox` binary's `run` and `doctor` subcommands against
//! the host Linux kernel, proving the Landlock + seccomp confinement contract is
//! actually installed and enforced (not merely claimed). The native restriction
//! is reachable only through the public CLI (`platform::current` is crate-private
//! and selects `LinuxRestriction` on Linux), so these are end-to-end behavioral
//! tests against the built binary — the production call site (`opi-sandbox CLI` +
//! `platform::linux`).
//!
//! Asserted contract (design `### Linux`, `### Native platform contract`):
//!   - workspace + invocation-temp writes are ALLOWED;
//!   - writes OUTSIDE the workspace/temp are DENIED;
//!   - host reads OUTSIDE the workspace remain ALLOWED (not a read-confidentiality
//!     boundary);
//!   - `network = deny` blocks new INET socket creation, denies io_uring setup,
//!     and closes inherited nonessential descriptors, while PRESERVING AF_UNIX;
//!   - `network = allow` permits INET socket creation;
//!   - `doctor` reports `supported = true` with the `landlock` + `seccomp`
//!     mechanisms and the honest limitations.
//!
//! File-gated to Linux: the confinement is Linux-native and the whole file
//! references Linux-only behavior, so on every other target it compiles to no
//! tests. Run on a Linux host (kernel + Landlock ABI >= 4); a kernel below ABI 4
//! makes `network = deny` fail closed, which these tests do not exercise.

#![cfg(target_os = "linux")]
#![deny(unsafe_code)]

#[path = "support/policy_probe.rs"]
mod policy_probe;

use std::fs;
use std::fs::File;
use std::net::TcpListener;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Output};

/// The built `opi-sandbox` binary under test.
fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_opi-sandbox"))
}

/// Run `opi-sandbox run` with a workspace, a network policy, and a shell target
/// command. Returns the process output (the target's mapped exit code is the
/// process exit code).
fn run_sh(workspace: &std::path::Path, network: &str, target: &str) -> Output {
    let mut cmd = Command::new(binary());
    cmd.arg("run")
        .arg("--workspace")
        .arg(workspace)
        .arg("--profile")
        .arg("workspace-write")
        .arg("--network")
        .arg(network)
        .arg("--")
        .arg("/bin/sh")
        .arg("-c")
        .arg(target);
    cmd.output().expect("spawn opi-sandbox run")
}

fn run_native_probe(
    workspace: &std::path::Path,
    network: &str,
    mode: &str,
    environment: &[(&str, String)],
) -> Output {
    let mut cmd = Command::new(binary());
    cmd.arg("run")
        .arg("--workspace")
        .arg(workspace)
        .arg("--profile")
        .arg("workspace-write")
        .arg("--network")
        .arg(network)
        .arg("--")
        .arg(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg(policy_probe::TEST_NAME)
        .arg("--ignored")
        .arg("--nocapture")
        .env("PATH", "/opi-policy-probe-no-path")
        .env("OPI_POLICY_PROBE_MODE", mode);
    for (key, value) in environment {
        cmd.env(key, value);
    }
    cmd.output().expect("spawn native policy probe")
}

fn run_unsandboxed_native_probe(mode: &str, environment: &[(&str, String)]) -> Output {
    let mut cmd = Command::new(std::env::current_exe().expect("current test executable"));
    cmd.arg("--exact")
        .arg(policy_probe::TEST_NAME)
        .arg("--ignored")
        .arg("--nocapture")
        .env("PATH", "/opi-policy-probe-no-path")
        .env("OPI_POLICY_PROBE_MODE", mode);
    for (key, value) in environment {
        cmd.env(key, value);
    }
    cmd.output().expect("spawn unsandboxed native policy probe")
}

#[derive(Debug, Clone, Copy)]
struct SeccompStatus {
    mode: u32,
    filters: u32,
}

fn parse_seccomp_status(output: &Output) -> SeccompStatus {
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let fields = text
        .lines()
        .find_map(|line| line.split_once("SECCOMP_STATUS:").map(|(_, fields)| fields))
        .unwrap_or_else(|| panic!("native probe emitted no seccomp status marker:\n{text}"));
    let (mode, filters) = fields
        .split_once(':')
        .unwrap_or_else(|| panic!("malformed seccomp status marker: {fields}"));
    SeccompStatus {
        mode: mode.parse().expect("seccomp mode must be numeric"),
        filters: filters
            .parse()
            .expect("seccomp filter count must be numeric"),
    }
}

fn assert_seccomp_filter_added(workspace: &std::path::Path, network: &str) {
    let ambient_output = run_unsandboxed_native_probe("seccomp-status", &[]);
    assert!(
        ambient_output.status.success(),
        "unsandboxed seccomp-status control must exit zero\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&ambient_output.stdout),
        String::from_utf8_lossy(&ambient_output.stderr)
    );
    let ambient = parse_seccomp_status(&ambient_output);

    let restricted_output = run_native_probe(workspace, network, "seccomp-status", &[]);
    assert!(
        restricted_output.status.success(),
        "restricted seccomp-status probe must exit zero\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&restricted_output.stdout),
        String::from_utf8_lossy(&restricted_output.stderr)
    );
    let restricted = parse_seccomp_status(&restricted_output);
    assert_eq!(
        restricted.mode, 2,
        "opi-sandbox child must run in seccomp filter mode"
    );
    assert!(
        restricted.filters > ambient.filters,
        "opi-sandbox must add its own seccomp filter above the unsandboxed baseline: ambient={ambient:?}, restricted={restricted:?}"
    );
    eprintln!(
        "SECCOMP_FILTER_EVIDENCE:{network}:ambient_mode={}:ambient_filters={}:restricted_mode={}:restricted_filters={}",
        ambient.mode, ambient.filters, restricted.mode, restricted.filters
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SyscallObservation {
    result: i64,
    errno: i32,
}

fn observe_unsandboxed_syscall(name: &str) -> SyscallObservation {
    let output = run_unsandboxed_native_probe(
        "syscall-observe",
        &[("OPI_POLICY_SYSCALL", name.to_string())],
    );
    assert!(
        output.status.success(),
        "unsandboxed syscall control for {name} must exit zero\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    parse_syscall_observation(name, &output)
}

fn observe_sandboxed_syscall(
    workspace: &std::path::Path,
    network: &str,
    name: &str,
) -> SyscallObservation {
    let output = run_native_probe(
        workspace,
        network,
        "syscall-observe",
        &[("OPI_POLICY_SYSCALL", name.to_string())],
    );
    assert!(
        output.status.success(),
        "sandboxed syscall observation for {name} must exit zero\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    parse_syscall_observation(name, &output)
}

fn parse_syscall_observation(name: &str, output: &Output) -> SyscallObservation {
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let fields = text
        .lines()
        .find_map(|line| {
            line.split_once(&format!("SYSCALL_OBSERVED:{name}:"))
                .map(|(_, fields)| fields)
        })
        .unwrap_or_else(|| {
            panic!("native probe emitted no syscall observation for {name}:\n{text}")
        });
    let (result, errno) = fields
        .split_once(':')
        .unwrap_or_else(|| panic!("malformed syscall observation for {name}: {fields}"));
    SyscallObservation {
        result: result.parse().expect("syscall result must be numeric"),
        errno: errno.parse().expect("syscall errno must be numeric"),
    }
}

#[derive(Debug, Clone, Copy)]
struct E2eSyscallTransition<'a> {
    name: &'a str,
    ambient_errno: i32,
    sandbox_errno: i32,
}

fn mandatory_transition_names<'a>(
    observations: &'a [E2eSyscallTransition<'a>],
) -> Result<Vec<&'a str>, String> {
    const MANDATORY_SENTINELS: &[&str] = &["bpf", "ptrace"];
    let transitions: Vec<&str> = observations
        .iter()
        .filter(|observation| {
            MANDATORY_SENTINELS.contains(&observation.name)
                && observation.ambient_errno != libc::EPERM
                && observation.sandbox_errno == libc::EPERM
        })
        .map(|observation| observation.name)
        .collect();
    if transitions.is_empty() {
        return Err(format!(
            "platform precondition failed: at least one mandatory seccomp attribution sentinel {:?} must transition from a non-EPERM ambient result to sandbox EPERM; observations={observations:?}",
            MANDATORY_SENTINELS
        ));
    }
    Ok(transitions)
}

#[test]
fn zero_mandatory_seccomp_transitions_are_rejected() {
    let all_ambient_eperm = [
        E2eSyscallTransition {
            name: "bpf",
            ambient_errno: libc::EPERM,
            sandbox_errno: libc::EPERM,
        },
        E2eSyscallTransition {
            name: "ptrace",
            ambient_errno: libc::EPERM,
            sandbox_errno: libc::EPERM,
        },
    ];
    assert!(
        mandatory_transition_names(&all_ambient_eperm).is_err(),
        "ambient EPERM for every mandatory sentinel must not false-pass attribution"
    );
}

#[test]
fn mandatory_seccomp_transition_is_reported() {
    let observations = [
        E2eSyscallTransition {
            name: "bpf",
            ambient_errno: libc::EINVAL,
            sandbox_errno: libc::EPERM,
        },
        E2eSyscallTransition {
            name: "ptrace",
            ambient_errno: libc::EPERM,
            sandbox_errno: libc::EPERM,
        },
    ];
    assert_eq!(
        mandatory_transition_names(&observations).expect("bpf is attributable"),
        ["bpf"]
    );
}

/// `opi-sandbox doctor --json` output.
fn doctor_json() -> String {
    let out = Command::new(binary())
        .arg("doctor")
        .arg("--json")
        .output()
        .expect("spawn opi-sandbox doctor");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Select a directory that is demonstrably writable before restriction.
fn first_writable_candidate(
    candidates: impl IntoIterator<Item = PathBuf>,
) -> Result<PathBuf, String> {
    let mut checked = Vec::new();
    for candidate in candidates {
        checked.push(candidate.clone());
        if candidate.is_dir() && tempfile::NamedTempFile::new_in(&candidate).is_ok() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "no writable outside-grant directory in {checked:?}"
    ))
}

/// A writable directory OUTSIDE the Landlock grant (workspace + the exact
/// invocation-private temp root). Missing coverage is a native-job failure.
fn outside_grant_dir() -> Result<PathBuf, String> {
    first_writable_candidate([PathBuf::from("/var/tmp")])
}

#[test]
fn writable_outside_candidate_is_required() {
    let root = tempfile::tempdir().expect("candidate root");
    let error = first_writable_candidate([root.path().join("missing")])
        .expect_err("missing candidates must fail coverage setup");
    assert!(error.contains("no writable outside-grant directory"));
}

/// The doctor reports a supported Linux posture with both native mechanisms and
/// the honest limitations — the mechanism-reporting half of SC16-10.
#[test]
fn doctor_reports_supported_native_with_mechanisms() {
    let json = doctor_json();
    assert!(
        json.contains("\"supported\":true"),
        "supported Linux doctor must report supported=true: {json}"
    );
    assert!(
        json.contains("\"landlock\"") && json.contains("\"seccomp\""),
        "doctor must list landlock + seccomp mechanisms: {json}"
    );
    assert!(
        json.contains("\"workspace-write\""),
        "doctor must list the workspace-write profile: {json}"
    );
    // Honest caveats, not overclaim.
    assert!(
        !json.contains("isolated"),
        "doctor must never claim `isolated`: {json}"
    );
}

/// A write INSIDE the canonical workspace succeeds.
#[test]
fn workspace_write_allowed() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    let marker = ws.path().join("inside.txt");
    let out = run_sh(
        ws.path(),
        "deny",
        &format!("echo ok > {}", marker.display()),
    );
    assert!(
        out.status.success(),
        "workspace write must be allowed\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(marker.is_file(), "workspace marker file must exist");
    assert_eq!(fs::read_to_string(&marker).unwrap().trim(), "ok");
}

/// A write to the exact invocation-private temporary root succeeds.
#[test]
fn temp_write_allowed() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    // TMPDIR is forced to the invocation-private root.
    let out = run_sh(
        ws.path(),
        "deny",
        "f=$(mktemp) && echo ok > \"$f\" && cat \"$f\"",
    );
    assert!(
        out.status.success(),
        "temp write must be allowed\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "ok");
}

/// A sibling in the system temporary directory is not covered by the private
/// temp-root grant.
#[test]
fn system_temp_sibling_write_denied() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    let marker = std::env::temp_dir().join(format!("opi-outside-{}.txt", std::process::id()));
    let _ = fs::remove_file(&marker);
    let out = run_sh(ws.path(), "deny", &format!("echo x > {}", marker.display()));
    assert!(
        !out.status.success(),
        "denied system-temp write must return a nonzero target status"
    );
    assert!(!marker.exists(), "system-temp sibling write must be denied");
}

/// A write OUTSIDE the workspace + temp is DENIED (Landlock fs enforcement).
#[test]
fn outside_write_denied() {
    let outside = outside_grant_dir().unwrap_or_else(|error| panic!("{error}"));
    let ws = tempfile::tempdir().expect("workspace tempdir");
    let marker = outside.join(format!("opi-outside-{}.txt", std::process::id()));
    let _ = fs::remove_file(&marker);
    // `set -e` makes the shell exit nonzero on the denied redirect.
    let out = run_sh(ws.path(), "deny", &format!("echo x > {}", marker.display()));
    assert!(
        !out.status.success(),
        "denied outside write must return a nonzero target status"
    );
    assert!(
        !marker.exists(),
        "write outside the grant must be DENIED (marker must not exist)"
    );
}

/// A host read OUTSIDE the workspace remains ALLOWED (not a read-confidentiality
/// boundary).
#[test]
fn outside_read_allowed() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    // /proc/self/comm is outside the workspace + temp and must be readable.
    let out = run_sh(ws.path(), "deny", "cat /proc/self/comm");
    assert!(
        out.status.success(),
        "host read outside the workspace must be allowed\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&out.stdout).trim().is_empty(),
        "must have read /proc/self/comm"
    );
}

/// `network = deny` blocks new INET socket creation (the seccomp socket gate).
#[test]
fn network_deny_blocks_inet_socket() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    let out = run_native_probe(ws.path(), "deny", "inet-bind", &[]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("INET_BIND_OK"),
        "network=deny must block INET socket creation: {stdout}"
    );
    assert!(
        !out.status.success(),
        "the denied socket must surface as a nonzero target exit"
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("INET_BIND_DENIED:"),
        "denied INET bind must emit the native denial marker"
    );
}

/// `network = allow` PERMITS new INET socket creation.
#[test]
fn network_allow_permits_inet_socket() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    let out = run_native_probe(ws.path(), "allow", "inet-bind", &[]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "network=allow native probe must exit zero\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("INET_BIND_OK"),
        "network=allow must permit INET socket creation: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `network = deny` PRESERVES AF_UNIX socket creation (ordinary local IPC).
#[test]
fn network_deny_preserves_af_unix() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    let out = run_native_probe(ws.path(), "deny", "unix-socket", &[]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "AF_UNIX native probe must exit zero\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("AF_UNIX_OK"),
        "network=deny must preserve AF_UNIX: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The baseline seccomp overlay adds a filter above the ambient host posture,
/// then observes every fixed danger syscall. The compiled-BPF unit test is the
/// per-rule proof; this E2E layer requires a real non-EPERM -> EPERM transition
/// from at least one pinned sentinel and labels ambient EPERM non-attributable.
#[test]
fn baseline_adds_filter_and_denies_every_danger_syscall() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    assert_seccomp_filter_added(ws.path(), "allow");
    let mut ambient_results = Vec::new();
    let mut observations = Vec::new();
    let mut attributable = Vec::new();
    let mut non_attributable = Vec::new();
    for name in danger_syscall_names() {
        let ambient = observe_unsandboxed_syscall(name);
        ambient_results.push(format!("{name}:{}:{}", ambient.result, ambient.errno));
        let out = run_native_probe(
            ws.path(),
            "allow",
            "syscall",
            &[("OPI_POLICY_SYSCALL", name.to_string())],
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success(),
            "seccomp-denied {name} must return a nonzero target status"
        );
        assert!(
            !stdout.contains(&format!("SYSCALL_OK:{name}:")),
            "seccomp-denied {name} must not emit a success marker: {stdout}"
        );
        assert!(
            stderr.contains(&format!("SYSCALL_DENIED:{name}:{}", libc::EPERM)),
            "{name} must be denied with EPERM above ambient {ambient:?}\nstdout: {stdout}\nstderr: {stderr}"
        );
        observations.push(E2eSyscallTransition {
            name,
            ambient_errno: ambient.errno,
            sandbox_errno: libc::EPERM,
        });
        if ambient.errno == libc::EPERM {
            non_attributable.push(name);
        } else {
            attributable.push(name);
        }
    }
    let mandatory = mandatory_transition_names(&observations)
        .unwrap_or_else(|precondition| panic!("{precondition}"));
    eprintln!("SECCOMP_AMBIENT_SYSCALLS:{}", ambient_results.join(","));
    eprintln!(
        "SECCOMP_E2E_ATTRIBUTABLE_TRANSITIONS:{}",
        attributable.join(",")
    );
    eprintln!(
        "SECCOMP_E2E_AMBIENT_EPERM_NON_ATTRIBUTABLE:{}",
        non_attributable.join(",")
    );
    eprintln!("SECCOMP_E2E_MANDATORY_TRANSITIONS:{}", mandatory.join(","));
}

/// `network = deny` denies `io_uring_setup` using the architecture-correct libc
/// syscall number rather than a hard-coded x86_64 value.
#[test]
fn network_deny_denies_io_uring_setup() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    assert_seccomp_filter_added(ws.path(), "deny");
    let ambient = observe_unsandboxed_syscall("io_uring_setup");
    let out = run_native_probe(
        ws.path(),
        "deny",
        "syscall",
        &[("OPI_POLICY_SYSCALL", "io_uring_setup".to_string())],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "denied io_uring_setup must return a nonzero target status"
    );
    assert!(
        !stdout.contains("SYSCALL_OK:io_uring_setup:"),
        "io_uring_setup denial must not emit a success marker: {stdout}"
    );
    assert!(
        stderr.contains(&format!("SYSCALL_DENIED:io_uring_setup:{}", libc::EPERM)),
        "io_uring_setup must be denied with EPERM\nstdout: {stdout}\nstderr: {stderr}"
    );
    if ambient.errno == libc::EPERM {
        eprintln!(
            "IO_URING_E2E_NON_ATTRIBUTABLE:ambient_result={}:ambient_errno={}",
            ambient.result, ambient.errno
        );
    } else {
        eprintln!(
            "IO_URING_E2E_ATTRIBUTABLE_TRANSITION:ambient_result={}:ambient_errno={}:sandbox_errno={}",
            ambient.result,
            ambient.errno,
            libc::EPERM
        );
    }
}

/// `unshare` is intentionally outside the canonical danger blocklist. The
/// harmless zero-flags form remains allowed in both the ambient control and the
/// real baseline-filter child.
#[test]
fn baseline_preserves_unshare_zero() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    let ambient = observe_unsandboxed_syscall("unshare_zero");
    let sandboxed = observe_sandboxed_syscall(ws.path(), "allow", "unshare_zero");
    assert_eq!(
        ambient,
        SyscallObservation {
            result: 0,
            errno: 0
        },
        "ambient unshare(0) control must be harmless and allowed"
    );
    assert_eq!(
        sandboxed,
        SyscallObservation {
            result: 0,
            errno: 0
        },
        "baseline seccomp filter must preserve unshare(0)"
    );
}

/// The runner overwrites all portable temporary-directory aliases with the
/// same invocation-private root while preserving unrelated inherited values.
#[test]
fn private_temp_aliases_and_environment_inheritance_are_exact() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    let inherited_temp = tempfile::tempdir().expect("inherited temp base");
    let inherited_temp = inherited_temp.path().display().to_string();
    let out = run_native_probe(
        ws.path(),
        "deny",
        "environment",
        &[
            ("OPI_POLICY_INHERITED", "inherited-exactly".to_string()),
            ("OPI_POLICY_REQUEST_TEMP", inherited_temp.clone()),
            ("TMPDIR", inherited_temp.clone()),
            ("TMP", inherited_temp.clone()),
            ("TEMP", inherited_temp),
        ],
    );
    assert!(
        out.status.success(),
        "environment probe must exit zero\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("ENVIRONMENT_ALIASES_OK"),
        "environment probe must emit its success marker"
    );
}

/// A known non-CLOEXEC file and INET socket survive the ordinary exec path,
/// then the deny path removes those exact resources while preserving AF_UNIX.
#[test]
fn network_deny_filters_exact_inherited_resources() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    let inherited_file_path = ws.path().join("inherited-file");
    let inherited_file = File::create(&inherited_file_path).expect("create inherited file");
    let inherited_inet = TcpListener::bind(("127.0.0.1", 0)).expect("bind inherited INET");
    let (inherited_unix, _unix_peer) = UnixStream::pair().expect("create inherited AF_UNIX");
    for fd in [
        inherited_file.as_raw_fd(),
        inherited_inet.as_raw_fd(),
        inherited_unix.as_raw_fd(),
    ] {
        clear_cloexec(fd).expect("make descriptor inheritable");
    }

    let environment = [
        ("OPI_POLICY_FILE_FD", inherited_file.as_raw_fd().to_string()),
        ("OPI_POLICY_INET_FD", inherited_inet.as_raw_fd().to_string()),
        ("OPI_POLICY_UNIX_FD", inherited_unix.as_raw_fd().to_string()),
        (
            "OPI_POLICY_FILE_LINK",
            descriptor_link(inherited_file.as_raw_fd()),
        ),
        (
            "OPI_POLICY_INET_LINK",
            descriptor_link(inherited_inet.as_raw_fd()),
        ),
        (
            "OPI_POLICY_UNIX_LINK",
            descriptor_link(inherited_unix.as_raw_fd()),
        ),
    ];

    let allow = run_native_probe(ws.path(), "allow", "descriptors-present", &environment);
    assert!(
        allow.status.success(),
        "control run must observe every exact inherited resource\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&allow.stdout),
        String::from_utf8_lossy(&allow.stderr)
    );
    assert!(String::from_utf8_lossy(&allow.stdout).contains("EXACT_DESCRIPTORS_PRESENT"));

    let deny = run_native_probe(ws.path(), "deny", "descriptors-filtered", &environment);
    assert!(
        deny.status.success(),
        "deny run must remove the exact file/INET resources and retain AF_UNIX\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&deny.stdout),
        String::from_utf8_lossy(&deny.stderr)
    );
    assert!(String::from_utf8_lossy(&deny.stdout).contains("EXACT_DESCRIPTORS_FILTERED"));
}

fn danger_syscall_names() -> Vec<&'static str> {
    let mut names = vec![
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
    names.extend_from_slice(arch_danger_syscall_names());
    names
}

#[cfg(target_arch = "x86_64")]
fn arch_danger_syscall_names() -> &'static [&'static str] {
    &["iopl", "ioperm"]
}

#[cfg(not(target_arch = "x86_64"))]
fn arch_danger_syscall_names() -> &'static [&'static str] {
    &[]
}

fn descriptor_link(fd: RawFd) -> String {
    fs::read_link(format!("/proc/self/fd/{fd}"))
        .unwrap_or_else(|error| panic!("read exact descriptor {fd}: {error}"))
        .to_string_lossy()
        .into_owned()
}

#[allow(unsafe_code)]
fn clear_cloexec(fd: RawFd) -> std::io::Result<()> {
    // SAFETY: fcntl reads and updates only the flags of this live descriptor.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    if flags < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: `fd` remains owned by the caller; clearing FD_CLOEXEC does not
    // transfer or close it.
    if unsafe { libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: read-only verification of the same live descriptor.
    let updated = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    if updated < 0 {
        return Err(std::io::Error::last_os_error());
    }
    if updated & libc::FD_CLOEXEC != 0 {
        return Err(std::io::Error::other("FD_CLOEXEC remained set"));
    }
    Ok(())
}

/// The target's exit code maps verbatim through `run` (the contract preserves
/// the target's outcome).
#[test]
fn run_maps_target_exit_code() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    let out = run_sh(ws.path(), "deny", "exit 7");
    assert_eq!(out.status.code(), Some(7), "target exit 7 maps verbatim");
    let out = run_sh(ws.path(), "deny", "exit 0");
    assert_eq!(out.status.code(), Some(0), "target exit 0 maps verbatim");
}
