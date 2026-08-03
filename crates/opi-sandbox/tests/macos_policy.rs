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
//! The "fail before target start when sandbox-exec is missing or rejected" DoD
//! clause is covered by TWO separately-tested links rather than one macOS
//! end-to-end test: (1) the pure `macos_posture_fields(Missing|Unusable)`
//! invariants in `src/platform/macos.rs` prove a missing/rejected probe ->
//! `supported = false` (host-independent); (2) the platform-neutral refusal
//! gate — `cli run` exits 125 and the backend emits `failed{Unavailable,
//! Handshake}` when `!posture.supported` — is exercised by the OFF-NATIVE
//! `cli_contract` / `backend` tests (a separate branch from this file's
//! Available-path sentinels). A stock-macOS end-to-end missing-helper test is
//! impractical (`/usr/bin/sandbox-exec` is always present, and the probe checks
//! that absolute path, not `PATH`), so the two links are proven separately
//! rather than chained on a macOS runner.
//!
//! File-gated to macOS: the confinement is macOS-native and the whole file
//! references macOS-only behavior, so on every other target it compiles to no
//! tests. Run on a macOS host where `sandbox-exec` is present and usable.

#![cfg(target_os = "macos")]
#![forbid(unsafe_code)]

use std::fs;
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

/// `opi-sandbox doctor --json` output.
fn doctor_json() -> String {
    let out = Command::new(binary())
        .arg("doctor")
        .arg("--json")
        .output()
        .expect("spawn opi-sandbox doctor");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A writable directory OUTSIDE the seatbelt temp grant (`std::env::temp_dir()`
/// / TMPDIR). On macOS TMPDIR is under `/var/folders`; `/tmp` (-> `/private/tmp`)
/// and `/var/tmp` are outside it, so a write there is denied. Returns `None` if
/// no such outside dir is usable on this host (the caller skips rather than
/// fails).
fn outside_grant_dir() -> Option<PathBuf> {
    let temp = std::env::temp_dir();
    [PathBuf::from("/tmp"), PathBuf::from("/var/tmp")]
        .into_iter()
        .find(|candidate| {
            candidate.is_dir() && !candidate.starts_with(&temp) && !temp.starts_with(candidate)
        })
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

/// A write to the invocation temporary root (system temp / TMPDIR) succeeds.
#[test]
fn temp_write_allowed() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    // mktemp creates under TMPDIR (the granted invocation temp root).
    let out = run_sh(
        ws.path(),
        "deny",
        "f=$(mktemp) && echo ok > \"$f\" && echo \"$f\"",
    );
    assert!(
        out.status.success(),
        "temp write must be allowed\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert!(!path.is_empty(), "mktemp must print a path: {:?}", out);
    assert_eq!(fs::read_to_string(&path).unwrap().trim(), "ok");
}

/// A write OUTSIDE the workspace + temp is DENIED (the seatbelt fs deny-overlay).
#[test]
fn outside_write_denied() {
    let outside = match outside_grant_dir() {
        Some(d) => d,
        None => {
            eprintln!("skip outside_write_denied: no writable dir outside the grant");
            return;
        }
    };
    let ws = tempfile::tempdir().expect("workspace tempdir");
    let marker = outside.join(format!("opi-outside-{}.txt", std::process::id()));
    let _ = fs::remove_file(&marker);
    let _out = run_sh(ws.path(), "deny", &format!("echo x > {}", marker.display()));
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
    let out = run_sh(
        ws.path(),
        "deny",
        "python3 -c 'import socket,sys; s=socket.socket(socket.AF_INET,socket.SOCK_STREAM); sys.stdout.write(\"BIND_OK\\n\") if s.bind((\"127.0.0.1\",0)) is None else None'",
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    // Under (deny network*) the bind raises (denied); the python either prints
    // nothing on the success path or exits nonzero on the raised bind. Either
    // way BIND_OK must NOT appear.
    assert!(
        !stdout.contains("BIND_OK"),
        "network=deny must block INET bind: {stdout}\nstderr: {stderr}"
    );
    assert!(
        !out.status.success(),
        "the denied bind must surface as a nonzero target exit"
    );
}

/// `network = allow` PERMITS INET `bind`.
#[test]
fn network_allow_permits_inet_bind() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    let out = run_sh(
        ws.path(),
        "allow",
        "python3 -c 'import socket,sys; s=socket.socket(socket.AF_INET,socket.SOCK_STREAM); s.bind((\"127.0.0.1\",0)); sys.stdout.write(\"BIND_OK\\n\")'",
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("BIND_OK"),
        "network=allow must permit INET bind: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `network = deny` PRESERVES AF_UNIX local IPC (the deny overlay is scoped to
/// `network*`, which does not cover Unix-domain sockets).
#[test]
fn network_deny_preserves_af_unix() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    let out = run_sh(
        ws.path(),
        "deny",
        "python3 -c 'import socket,sys; a,b=socket.socketpair(socket.AF_UNIX); sys.stdout.write(\"AFUNIX_OK\\n\")'",
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("AFUNIX_OK"),
        "network=deny must preserve AF_UNIX: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
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

/// L0 tree-kill: a dropped `run` reaps the target even when it is wrapped in
/// the `sandbox-exec` launcher (the launcher must keep the target in its
/// process group so the tree guard's group kill reaches it). The target writes
/// its PID to a file then sleeps; the run is dropped without polling to
/// completion; the target must be reaped shortly after.
#[test]
fn dropped_run_reaps_launcher_wrapped_target() {
    let ws = tempfile::tempdir().expect("workspace tempdir");
    let pidfile = ws.path().join("child.pid");
    let pidfile_str = pidfile.to_string_lossy().into_owned();
    // Target: write the child shell's PID, then sleep well past the test. The
    // child is run under `sandbox-exec` (the launcher); if sandbox-exec kept the
    // target in a different process group, kill(-pgid) would miss it.
    let target = format!("echo $$ > {pidfile_str}; sleep 60");
    let mut cmd = Command::new(binary());
    cmd.arg("run")
        .arg("--workspace")
        .arg(ws.path())
        .arg("--profile")
        .arg("workspace-write")
        .arg("--network")
        .arg("deny")
        .arg("--")
        .arg("/bin/sh")
        .arg("-c")
        .arg(&target)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let mut child = cmd.spawn().expect("spawn opi-sandbox run");
    // Wait for the pidfile to appear (the target started under the launcher).
    let mut started = false;
    for _ in 0..200 {
        if pidfile.is_file() {
            started = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(started, "target never wrote its pidfile");
    let pid_str = fs::read_to_string(&pidfile).unwrap();
    let pid: i32 = pid_str.trim().parse().expect("pid parses");
    // Drop the run (kill_on_drop + tree guard terminate the whole group).
    let _ = child.kill();
    let _ = child.wait();
    // The target must be reaped within a short grace.
    let mut reaped = false;
    for _ in 0..200 {
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
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(reaped, "dropped run must reap the launcher-wrapped target");
}
