//! Native macOS restriction contract (Phase 16 task 16.14.1, SC16-11).
//!
//! Drives the REAL `opi-sandbox` binary's `run` and `doctor` subcommands on a
//! macOS kernel, proving the `sandbox-exec`/Seatbelt deny-overlay is actually
//! installed and enforced (not merely claimed). The native restriction is
//! reachable only through the public CLI (`platform::current` is crate-private
//! and selects `MacosRestriction` on macOS), so these are end-to-end behavioral
//! tests against the built binary — the production call site (`opi-sandbox CLI`
//! + `platform::macos`).
//!
//! Asserted contract (design `### macOS`, `### Native platform contract`):
//!   - workspace + invocation-temp writes are ALLOWED;
//!   - writes OUTSIDE the workspace/temp are DENIED;
//!   - host reads OUTSIDE the workspace remain ALLOWED (not a read-confidentiality
//!     boundary);
//!   - `network = deny` blocks INET `bind`/`connect` (`(deny network*)` blocks
//!     bind/connect/inbound/outbound but NOT `socket()` creation itself — so the
//!     sentinel exercises `bind`, NOT bare `socket()` creation, unlike the Linux
//!     seccomp twin which gates `socket()` at `arg[0]`);
//!   - `network = deny` PRESERVES AF_UNIX local IPC;
//!   - `network = allow` permits INET `bind`;
//!   - `doctor` reports `supported = true` with the `seatbelt` mechanism and the
//!     honest legacy/experimental limitations.
//!
//! Missing and unusable helper postures are injected at the production probe
//! seam by the macOS-only inline test in `src/platform/macos.rs`. That test
//! chains each posture through the CLI gate and protocol backend, asserts exit
//! 125 / `failed{Unavailable, Handshake}`, and proves the target sentinel never
//! starts without modifying `/usr/bin/sandbox-exec`.
//!
//! File-gated to macOS: the confinement is macOS-native and the whole file
//! references macOS-only behavior, so on every other target it compiles to no
//! tests. Run on a macOS host where `sandbox-exec` is present and usable.

#![cfg(target_os = "macos")]
#![forbid(unsafe_code)]

#[path = "support/policy_probe.rs"]
mod policy_probe;

use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::OsStringExt;
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
    let temp = std::env::temp_dir();
    let mut checked = Vec::new();
    for candidate in candidates {
        checked.push(candidate.clone());
        if candidate.is_dir()
            && !candidate.starts_with(&temp)
            && !temp.starts_with(&candidate)
            && tempfile::NamedTempFile::new_in(&candidate).is_ok()
        {
            return Ok(candidate);
        }
    }
    Err(format!(
        "no writable outside-grant directory in {checked:?}"
    ))
}

/// A writable directory OUTSIDE the exact invocation-private Seatbelt temp
/// grant. On macOS `/tmp` and `/var/tmp` are outside that private root. Missing
/// coverage is a native-job failure.
fn outside_grant_dir() -> Result<PathBuf, String> {
    first_writable_candidate([PathBuf::from("/tmp"), PathBuf::from("/var/tmp")])
}

#[test]
fn writable_outside_candidate_is_required() {
    let root = tempfile::tempdir().expect("candidate root");
    let error = first_writable_candidate([root.path().join("missing")])
        .expect_err("missing candidates must fail coverage setup");
    assert!(error.contains("no writable outside-grant directory"));
}

/// The doctor reports a supported macOS posture with the seatbelt mechanism and
/// the honest limitations — the mechanism-reporting half of SC16-11.
#[test]
fn doctor_reports_supported_native_with_seatbelt() {
    let json = doctor_json();
    assert!(
        json.contains("\"supported\":true"),
        "supported macOS doctor must report supported=true: {json}"
    );
    assert!(
        json.contains("\"seatbelt\""),
        "doctor must list the seatbelt mechanism: {json}"
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
    assert!(
        json.contains("soft-deprecated"),
        "doctor must report the sandbox-exec soft-deprecation: {json}"
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
    // mktemp creates under TMPDIR (the granted invocation temp root).
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

/// A write OUTSIDE the workspace + temp is DENIED (the seatbelt fs deny-overlay).
#[test]
fn outside_write_denied() {
    let outside = outside_grant_dir().unwrap_or_else(|error| panic!("{error}"));
    let ws = tempfile::tempdir().expect("workspace tempdir");
    let marker = outside.join(format!("opi-outside-{}.txt", std::process::id()));
    let _ = fs::remove_file(&marker);
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
    // /etc/hosts is outside the workspace + temp and must be readable.
    let out = run_sh(ws.path(), "deny", "cat /etc/hosts");
    assert!(
        out.status.success(),
        "host read outside the workspace must be allowed\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&out.stdout).trim().is_empty(),
        "must have read /etc/hosts"
    );
}

/// `network = deny` blocks INET `bind` (`(deny network*)` blocks bind/connect,
/// NOT `socket()` creation — so this sentinel exercises `bind`, not bare
/// `socket()` creation, which would falsely pass under the seatbelt overlay).
#[test]
fn network_deny_blocks_inet_bind() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    let out = run_native_probe(ws.path(), "deny", "inet-bind", &[]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stdout.contains("INET_BIND_OK"),
        "network=deny must block INET bind: {stdout}\nstderr: {stderr}"
    );
    assert!(
        !out.status.success(),
        "the denied bind must surface as a nonzero target exit"
    );
    assert!(
        stderr.contains("INET_BIND_DENIED:"),
        "denied INET bind must emit the native denial marker"
    );
}

/// `network = allow` PERMITS INET `bind`.
#[test]
fn network_allow_permits_inet_bind() {
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
        "network=allow must permit INET bind: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `network = deny` PRESERVES AF_UNIX local IPC (the deny overlay is scoped to
/// `network*`, which does not cover Unix-domain sockets).
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

/// The runner overwrites all portable temporary-directory aliases with the
/// same invocation-private root while preserving unrelated inherited values.
#[test]
fn private_temp_aliases_and_environment_inheritance_are_exact() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    let inherited_temp_dir = tempfile::tempdir().expect("inherited temp base");
    let inherited_temp = inherited_temp_dir.path().display().to_string();
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

/// Seatbelt profile strings cannot safely represent control characters in
/// path literals. Refusal is pre-start and leaves the target marker absent.
#[test]
fn control_character_workspace_is_refused_before_target_start() {
    let parent = tempfile::tempdir().expect("workspace parent");
    let workspace = parent.path().join("workspace\ncontrol");
    fs::create_dir(&workspace).expect("create control-character workspace");
    let marker = parent.path().join("must-not-exist-control");

    let out = run_sh(
        &workspace,
        "deny",
        &format!("printf started > '{}'", marker.display()),
    );

    assert_eq!(out.status.code(), Some(125), "profile refusal is pre-start");
    assert!(
        !marker.exists(),
        "refused profile never releases the target"
    );
}

/// A native path that is not valid UTF-8 is refused rather than changed with a
/// replacement character in the Seatbelt profile.
#[test]
fn non_utf8_workspace_is_refused_before_target_start() {
    let parent = tempfile::tempdir().expect("workspace parent");
    let workspace = parent
        .path()
        .join(OsString::from_vec(vec![b'w', b's', b'-', 0xff]));
    // macOS (HFS+/APFS) rejects non-UTF8 filenames at creation, so a non-UTF8
    // workspace can never exist on disk; the seatbelt profile refuses the
    // verbatim non-UTF8 path before target start regardless.
    let marker = parent.path().join("must-not-exist-native");

    let out = run_sh(
        &workspace,
        "deny",
        &format!("printf started > '{}'", marker.display()),
    );

    assert_eq!(out.status.code(), Some(125), "profile refusal is pre-start");
    assert!(
        !marker.exists(),
        "refused native path never releases target"
    );
}

/// L0 tree-kill through the launcher: when the target forks a surviving
/// grandchild and then exits, opi-sandbox's `supervise` calls `tree.terminate`
/// (a process-GROUP kill) on completion, which must reach the grandchild even
/// though the target ran under the `sandbox-exec` launcher. (`sandbox-exec`
/// `execv`s the target rather than forking a separate process, so the target
/// inherits `configure_tree`'s `process_group(0)` and its descendants share the
/// group the guard kills.) This is the empirical seal the design-audit's
/// tree-kill flag asked for; it deliberately does NOT SIGKILL the CLI parent
/// (that bypasses `Drop`/`kill_on_drop`, so it is not an opi-sandbox path).
#[test]
fn completion_reaps_launcher_wrapped_grandchild() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    let pidfile = ws.path().join("grandchild.pid");
    let pidfile_str = pidfile.to_string_lossy().into_owned();
    // Fork a surviving grandchild in the background, capture its PID, then exit.
    // opi-sandbox completes (target exited) -> supervise -> tree.terminate
    // (group kill) must reap the still-running background sleep.
    let target = format!("sleep 60 & echo $! > {pidfile_str}");
    let out = run_sh(ws.path(), "deny", &target);
    assert!(
        out.status.success(),
        "target must exit 0 (fork grandchild then return)\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let pid: i32 = fs::read_to_string(&pidfile)
        .expect("grandchild pidfile")
        .trim()
        .parse()
        .expect("pid parses");
    // tree.terminate ran inside supervise before run_sh returned, so the
    // grandchild must already be reaped. A short grace covers scheduling.
    let mut reaped = false;
    for _ in 0..20 {
        let alive = Command::new("/bin/sh")
            .arg("-c")
            .arg(format!("kill -0 {pid} 2>/dev/null"))
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !alive {
            reaped = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(
        reaped,
        "the surviving grandchild must be reaped by the group kill on completion"
    );
}
