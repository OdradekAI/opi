use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    let crate_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("crate lives under crates/opi-coding-agent")
        .to_path_buf()
}

fn python_command() -> &'static str {
    if cfg!(windows) { "python" } else { "python3" }
}

fn run_audit_with_args(
    dir: &std::path::Path,
    workspace: &std::path::Path,
    json: bool,
) -> (bool, String, String) {
    let out = Command::new(python_command())
        .arg(
            workspace_root()
                .join("scripts")
                .join("opi-artifact-audit.py"),
        )
        .arg(dir)
        .arg("--workspace-root")
        .arg(workspace)
        .args(json.then_some("--json"))
        .output()
        .expect("run artifact audit");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn run_audit(dir: &std::path::Path) -> (bool, String, String) {
    run_audit_with_args(dir, dir, false)
}

#[test]
fn artifact_audit_fails_on_workspace_root_leak_and_passes_when_removed() {
    let dir = tempfile::tempdir().expect("artifact tempdir");
    let session_dir = dir.path().join("sessions");
    std::fs::create_dir(&session_dir).unwrap();
    // dir.path() on Windows uses backslashes — exercises the normalization path.
    // Escape them so the embedded JSON is valid on Windows: raw `\U`/`\L` are
    // invalid JSON escapes, and the session-header `cwd` must parse so it can be
    // recognized and skipped by the checker.
    let leaked_root = dir.path().display().to_string().replace('\\', "\\\\");

    // NDJSON: a tool-result-style line that embeds the absolute root.
    std::fs::write(
        dir.path().join("run.ndjson"),
        format!(
            "{{\"type\":\"Agent\",\"event\":{{\"type\":\"MessageUpdate\",\"message\":{{\"timestamp_ms\":1,\"content\":[{{\"type\":\"text\",\"text\":\"{leaked_root}/file.txt\"}}]}},\"assistant_event\":{{\"type\":\"text_delta\",\"delta\":\"x\"}}}}}}\n{{\"type\":\"session_summary\",\"turns\":1,\"provider_turns\":1,\"tokens\":{{\"input\":0,\"output\":0,\"cache_read\":0,\"cache_write\":0}}}}\n"
        ),
    )
    .unwrap();
    // Session JSONL: a session header (by-design cwd, must be skipped) + a leaking message.
    std::fs::write(
        session_dir.join("s.jsonl"),
        format!(
            "{{\"type\":\"session\",\"cwd\":\"{leaked_root}\"}}\n{{\"type\":\"message\",\"message\":{{\"content\":\"{leaked_root}/file.txt\"}}}}\n"
        ),
    )
    .unwrap();

    let (ok, stdout, stderr) = run_audit(dir.path());
    assert!(
        !ok,
        "audit must fail on leak: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("workspace_root_leak") || stdout.contains("session_workspace_root_leak"),
        "expected a leak finding, got: {stdout}"
    );

    // Remove the leaks (keep the session header, which is allowed).
    std::fs::write(
        dir.path().join("run.ndjson"),
        "{\"type\":\"Agent\",\"event\":{\"type\":\"TurnStart\"}}\n{\"type\":\"Agent\",\"event\":{\"type\":\"MessageUpdate\",\"message\":{\"timestamp_ms\":1,\"content\":[{\"type\":\"text\",\"text\":\"file.txt\"}]},\"assistant_event\":{\"type\":\"text_delta\",\"delta\":\"x\"}}}\n{\"type\":\"session_summary\",\"turns\":1,\"provider_turns\":1,\"tokens\":{\"input\":0,\"output\":0,\"cache_read\":0,\"cache_write\":0}}\n",
    )
    .unwrap();
    std::fs::write(
        session_dir.join("s.jsonl"),
        format!("{{\"type\":\"session\",\"cwd\":\"{leaked_root}\"}}\n{{\"type\":\"message\",\"message\":{{\"content\":\"file.txt\"}}}}\n"),
    )
    .unwrap();

    let (ok, stdout, stderr) = run_audit(dir.path());
    assert!(
        ok,
        "audit must pass after leak removal: stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn artifact_audit_detects_zero_timestamps_turn_mismatch_and_duplicate_partials() {
    let dir = tempfile::tempdir().expect("artifact tempdir");
    // 60 text_delta lines, all carrying "partial" -> duplicated_text_delta_partials.
    let mut ndjson = String::new();
    ndjson.push_str("{\"type\":\"Agent\",\"event\":{\"type\":\"MessageUpdate\",\"message\":{\"timestamp_ms\":0,\"content\":[{\"type\":\"text\",\"text\":\"x\"}]},\"assistant_event\":{\"type\":\"text_delta\",\"delta\":\"x\",\"partial\":{}}}}\n");
    for _ in 0..59 {
        ndjson.push_str("{\"type\":\"Agent\",\"event\":{\"type\":\"MessageUpdate\",\"message\":{\"timestamp_ms\":0,\"content\":[{\"type\":\"text\",\"text\":\"x\"}]},\"assistant_event\":{\"type\":\"text_delta\",\"delta\":\"x\",\"partial\":{}}}}\n");
    }
    // 2 TurnStart events but provider_turns=5 -> mismatch.
    ndjson.push_str("{\"type\":\"Agent\",\"event\":{\"type\":\"TurnStart\"}}\n");
    ndjson.push_str("{\"type\":\"Agent\",\"event\":{\"type\":\"TurnStart\"}}\n");
    ndjson.push_str("{\"type\":\"session_summary\",\"turns\":1,\"provider_turns\":5,\"tokens\":{\"input\":0,\"output\":0,\"cache_read\":0,\"cache_write\":0}}\n");
    std::fs::write(dir.path().join("run.ndjson"), ndjson).unwrap();

    let (ok, stdout, _stderr) = run_audit(dir.path());
    assert!(!ok, "audit must fail on the synthetic defects");
    assert!(
        stdout.contains("all_zero_timestamps"),
        "missing zero-timestamp finding: {stdout}"
    );
    assert!(
        stdout.contains("duplicated_text_delta_partials"),
        "missing partial-duplication finding: {stdout}"
    );
    assert!(
        stdout.contains("provider_turn_mismatch"),
        "missing provider-turn mismatch finding: {stdout}"
    );
}

#[test]
fn artifact_audit_rejects_every_missing_declared_commit_reference() {
    let dir = tempfile::tempdir().expect("artifact tempdir");
    let missing_summary_commit = "9b607783af14a7e24aed2c259fc1741e14d21a4a";
    let missing_metadata_commit = "ffffffffffffffffffffffffffffffffffffffff";
    std::fs::write(
        dir.path().join("RUN_SUMMARY.md"),
        format!("Head commit at authoring: {missing_summary_commit} (start_commit)\n"),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("metadata.json"),
        format!("{{\"releaseCommit\":\"{missing_metadata_commit}\"}}\n"),
    )
    .unwrap();

    let (ok, stdout, stderr) = run_audit_with_args(dir.path(), &workspace_root(), true);
    assert!(
        !ok,
        "audit must reject missing commit objects: stdout={stdout} stderr={stderr}"
    );
    let report: serde_json::Value =
        serde_json::from_str(&stdout).expect("artifact audit emits JSON on failure");
    let issues = report["issues"]
        .as_array()
        .expect("artifact audit issues are an array");
    let missing = issues
        .iter()
        .filter(|issue| issue["code"] == "missing_commit_reference")
        .collect::<Vec<_>>();
    assert_eq!(
        missing.len(),
        2,
        "every declared missing commit must be reported: {stdout}"
    );
    assert!(
        missing
            .iter()
            .any(|issue| issue["reference"] == missing_summary_commit),
        "summary commit typo must be attributable: {stdout}"
    );
    assert!(
        missing
            .iter()
            .any(|issue| issue["reference"] == missing_metadata_commit),
        "metadata commit must be attributable: {stdout}"
    );
}

#[test]
fn artifact_audit_accepts_real_declared_commit_objects() {
    let dir = tempfile::tempdir().expect("artifact tempdir");
    let real_commit = "9b607783af14a7e24aed2c259fc1741e14d21a4b";
    std::fs::write(
        dir.path().join("RUN_SUMMARY.md"),
        format!("Head commit at authoring: {real_commit} (start_commit)\n"),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("metadata.json"),
        format!("{{\"releaseCommit\":\"{real_commit}\"}}\n"),
    )
    .unwrap();

    let (ok, stdout, stderr) = run_audit_with_args(dir.path(), &workspace_root(), true);
    assert!(
        ok,
        "audit must accept real commit objects: stdout={stdout} stderr={stderr}"
    );
    let report: serde_json::Value =
        serde_json::from_str(&stdout).expect("artifact audit emits JSON on success");
    assert_eq!(
        report["commit_references"].as_array().map(Vec::len),
        Some(2)
    );
}

// ============================================================================
// Phase 16 task 16.15.2: release-archive audit mode (`--release`).
//
// The release audit validates the published native opi-sandbox topology per
// SC16-12b: native target identity, archive layout, extracted-binary
// provenance, direct/backend smoke evidence, and complete non-skipped /
// non-zero-test Linux/macOS/Windows evidence. It rejects absent, wrong-target,
// workspace-only, skipped, or zero-test evidence. These tests drive the audit
// script on synthetic per-platform evidence bundles (good + each defect class).
// ============================================================================

use sha2::{Digest, Sha256};

fn sha256_hex_local(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Run the artifact audit in RELEASE mode on `dir`.
fn run_release_audit(dir: &std::path::Path, json: bool) -> (bool, String, String) {
    let out = Command::new(python_command())
        .arg(
            workspace_root()
                .join("scripts")
                .join("opi-artifact-audit.py"),
        )
        .arg(dir)
        .arg("--release")
        .args(json.then_some("--json"))
        .output()
        .expect("run release audit");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

const LINUX_TARGET: &str = "x86_64-unknown-linux-gnu";
const LINUX_ARM_TARGET: &str = "aarch64-unknown-linux-gnu";
const MACOS_X64_TARGET: &str = "x86_64-apple-darwin";
const MACOS_TARGET: &str = "aarch64-apple-darwin";
const BINARY_BYTES: &[u8] = b"opi-sandbox extracted release binary payload\n";

fn good_smoke_log() -> &'static str {
    "opi-sandbox-direct-smoke: OK archive_sha256=__ARCHIVE_SHA256__\n\
     opi-sandbox-backend-smoke: OK archive_sha256=__ARCHIVE_SHA256__\n\
     opi-sandbox-empty-cwd-smoke: OK archive_sha256=__ARCHIVE_SHA256__\n\
     opi-sandbox-setup-failure-smoke: OK archive_sha256=__ARCHIVE_SHA256__\n\
     opi-sandbox-filesystem-allow-smoke: OK archive_sha256=__ARCHIVE_SHA256__\n\
     opi-sandbox-filesystem-deny-smoke: OK archive_sha256=__ARCHIVE_SHA256__\n\
     opi-sandbox-network-deny-smoke: OK archive_sha256=__ARCHIVE_SHA256__\n\
     opi-sandbox-network-allow-smoke: OK archive_sha256=__ARCHIVE_SHA256__\n\
     opi-sandbox-smoke: OK archive_sha256=__ARCHIVE_SHA256__\n"
}

fn good_windows_log() -> &'static str {
    // Windows unsupported-posture evidence: doctor reports supported=false, and
    // the unsupported-posture cargo tests pass (non-skipped, non-zero-test).
    "doctor: {\"schema_version\":1,\"supported\":false,\"target\":\"windows\",\"mechanisms\":[],\"profiles\":[\"workspace-write\"],\"limitations\":[]}\n\
     run: refused pre-start (exit 125)\n\
     test result: ok. 3 passed; 0 failed; 0 ignored\n"
}

fn compatible_minor_range(version: &str) -> String {
    let core = version.split(['-', '+']).next().expect("version core");
    let mut parts = core.split('.');
    let major: u64 = parts.next().expect("major").parse().expect("numeric major");
    let minor: u64 = parts.next().expect("minor").parse().expect("numeric minor");
    format!(">={major}.{minor}.0-0,<{major}.{}.0-0", minor + 1)
}

fn native_archive_path(dir: &std::path::Path, target: &str) -> std::path::PathBuf {
    dir.join(format!("opi-sandbox-{target}.tar.gz"))
}

fn create_native_archive(
    archive: &std::path::Path,
    manifest: &std::path::Path,
    executable: &std::path::Path,
    extra_member: bool,
) {
    let script = r##"
import io, sys, tarfile
with tarfile.open(sys.argv[1], "w:gz") as out:
    out.add(sys.argv[2], arcname="package.toml", recursive=False)
    executable = out.gettarinfo(sys.argv[3], arcname="bin/opi-sandbox")
    executable.mode = 0o755
    with open(sys.argv[3], "rb") as payload:
        out.addfile(executable, payload)
    snapshot = open(sys.argv[4], encoding="utf-8").read().splitlines()
    markers = [index for index, line in enumerate(snapshot) if line == "---"]
    schema = ("\n".join(snapshot[markers[1] + 1:]) + "\n").encode()
    schema_info = tarfile.TarInfo("schemas/command-execution-jsonl-v1.schema.json")
    schema_info.mode = 0o644
    schema_info.size = len(schema)
    out.addfile(schema_info, io.BytesIO(schema))
    out.add(sys.argv[5], arcname="licenses/LICENSE", recursive=False)
    if sys.argv[6] == "extra":
        info = tarfile.TarInfo("unexpected.txt")
        payload = b"unexpected"
        info.size = len(payload)
        out.addfile(info, io.BytesIO(payload))
"##;
    let output = Command::new(python_command())
        .args(["-c", script])
        .arg(archive)
        .arg(manifest)
        .arg(executable)
        .arg(
            workspace_root()
                .join("crates/opi-protocol/tests/snapshots/execution_v1_schema__schema_v1.snap"),
        )
        .arg(workspace_root().join("LICENSE"))
        .arg(if extra_member { "extra" } else { "exact" })
        .output()
        .expect("create synthetic native archive");
    assert!(
        output.status.success(),
        "archive fixture creation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn rewrite_native_archive(archive: &std::path::Path, mutation: &str) -> String {
    let script = r##"
import hashlib, os, re, sys, tarfile, tempfile
archive, mutation = sys.argv[1], sys.argv[2]
payloads = {}
modes = {}
with tarfile.open(archive, "r:gz") as source:
    for member in source.getmembers():
        name = member.name
        while name.startswith("./"):
            name = name[2:]
        if member.isfile():
            payloads[name] = source.extractfile(member).read()
            modes[name] = member.mode
if mutation == "nonexec":
    modes["bin/opi-sandbox"] = 0o644
elif mutation == "unknown-field":
    payloads["package.toml"] += b"unknown_adapter_field = 1\n"
elif mutation == "oversized-manifest":
    payloads["package.toml"] += b"#" * (1024 * 1024)
elif mutation == "missing-schema":
    del payloads["schemas/command-execution-jsonl-v1.schema.json"]
elif mutation in ("version-trailing-hyphen", "version-metacharacter"):
    version = b"1.2.3-" if mutation == "version-trailing-hyphen" else b"1.2.3-&"
    payloads["package.toml"], count = re.subn(
        br'(?m)^version = "[^"]+"$',
        b'version = "' + version + b'"',
        payloads["package.toml"],
        count=1,
    )
    if count != 1:
        raise SystemExit("manifest version fixture not found")
    payloads["package.toml"], count = re.subn(
        br'(?m)^opi_version = "[^"]+"$',
        b'opi_version = ">=1.2.0-0,<1.3.0-0"',
        payloads["package.toml"],
        count=1,
    )
    if count != 1:
        raise SystemExit("manifest opi_version fixture not found")
elif mutation not in ("dot-alias", "double-slash-alias", "embedded-dot-alias"):
    raise SystemExit("unknown mutation")

def archive_name(name):
    if mutation == "dot-alias" and name == "package.toml":
        return "./package.toml"
    if mutation == "double-slash-alias" and name == "bin/opi-sandbox":
        return "bin//opi-sandbox"
    if mutation == "embedded-dot-alias" and name == "schemas/command-execution-jsonl-v1.schema.json":
        return "schemas/./command-execution-jsonl-v1.schema.json"
    return name
fd, temporary = tempfile.mkstemp(dir=os.path.dirname(archive), suffix=".tar.gz")
os.close(fd)
try:
    with tarfile.open(temporary, "w:gz") as output:
        for name in (
            "package.toml",
            "bin/opi-sandbox",
            "schemas/command-execution-jsonl-v1.schema.json",
            "licenses/LICENSE",
        ):
            if name not in payloads:
                continue
            info = tarfile.TarInfo(archive_name(name))
            info.mode = modes[name]
            info.size = len(payloads[name])
            import io
            output.addfile(info, io.BytesIO(payloads[name]))
    os.replace(temporary, archive)
    print(hashlib.sha256(payloads["package.toml"].replace(b"\r", b"")).hexdigest())
finally:
    if os.path.exists(temporary):
        os.unlink(temporary)
"##;
    let output = Command::new(python_command())
        .args(["-c", script])
        .arg(archive)
        .arg(mutation)
        .output()
        .expect("rewrite synthetic native archive");
    assert!(
        output.status.success(),
        "archive fixture rewrite failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("archive fixture rewrite output is UTF-8")
        .trim()
        .to_string()
}

fn rebind_smoke_to_archive(
    bundle: &std::path::Path,
    previous_archive_sha: &str,
    archive: &std::path::Path,
) {
    let smoke_path = bundle.join("smoke.log");
    let smoke = std::fs::read_to_string(&smoke_path).unwrap();
    let current_archive_sha = sha256_hex_local(&std::fs::read(archive).unwrap());
    let rebound = smoke.replace(previous_archive_sha, &current_archive_sha);
    assert_ne!(
        rebound, smoke,
        "smoke fixture did not contain the prior archive SHA"
    );
    std::fs::write(smoke_path, rebound).unwrap();
}

fn replace_lock_value(lock: &mut String, key: &str, value: &str) {
    let prefix = format!("{key} = \"");
    let mut replaced = false;
    *lock = lock
        .lines()
        .map(|line| {
            if line.starts_with(&prefix) {
                replaced = true;
                format!("{prefix}{value}\"")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert!(replaced, "lock fixture has no {key}");
}

/// Write a native release bundle containing the complete distribution wrapper,
/// LockMaterial, and direct/backend smoke markers bound to the archive digest.
fn write_native_bundle(
    root: &std::path::Path,
    platform: &str,
    target: &str,
    binary_bytes: &[u8],
    smoke_log: &str,
    mismatch_sha: bool,
    omit_archive: bool,
) {
    let dir = root.join(platform);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("target"), target).unwrap();
    let actual_exe_sha = sha256_hex_local(binary_bytes);
    let locked_exe_sha = if mismatch_sha {
        "0".repeat(64)
    } else {
        actual_exe_sha.clone()
    };
    let version = env!("CARGO_PKG_VERSION");
    let opi_range = compatible_minor_range(version);
    let manifest = format!(
        "name = \"opi-sandbox\"\n\
         description = \"Official host-native command restriction backend.\"\n\
         version = \"{version}\"\n\
         opi_version = \"{opi_range}\"\n\n\
         [[contributions.adapters]]\n\
         capability = \"command.execute\"\n\
         id = \"opi-sandbox\"\n\
         transport = \"process-jsonl\"\n\
         command = \"bin/opi-sandbox\"\n\
         args = [\"backend\", \"--stdio\"]\n\
         protocol = \"command-execution-jsonl-v1\"\n\
         target = \"{target}\"\n\
         sha256 = \"{actual_exe_sha}\"\n\
         handshake_timeout_ms = 5000\n\
         adapter_config = {{}}\n"
    );
    let stage = dir.join("archive-stage");
    std::fs::create_dir_all(stage.join("bin")).unwrap();
    std::fs::write(stage.join("package.toml"), &manifest).unwrap();
    std::fs::write(stage.join("bin").join("opi-sandbox"), binary_bytes).unwrap();
    let archive = native_archive_path(&dir, target);
    if !omit_archive {
        create_native_archive(
            &archive,
            &stage.join("package.toml"),
            &stage.join("bin").join("opi-sandbox"),
            false,
        );
    }
    std::fs::remove_dir_all(&stage).unwrap();
    let manifest_hash = sha256_hex_local(manifest.as_bytes());
    std::fs::write(
        dir.join("package-lock.toml"),
        format!(
            "manifest_hash = \"{manifest_hash}\"\n\
             executable_rel_path = \"bin/opi-sandbox\"\n\
             executable_sha256 = \"{locked_exe_sha}\"\n\
             package_version = \"{version}\"\n\
             target = \"{target}\"\n\
             opi_range = \"{opi_range}\"\n\
             protocol = \"command-execution-jsonl-v1\"\n\
             adapter_id = \"opi-sandbox\"\n"
        ),
    )
    .unwrap();
    let archive_sha = if omit_archive {
        "0".repeat(64)
    } else {
        sha256_hex_local(&std::fs::read(&archive).unwrap())
    };
    std::fs::write(
        dir.join("smoke.log"),
        smoke_log.replace("__ARCHIVE_SHA256__", &archive_sha),
    )
    .unwrap();
}

fn write_windows_bundle(root: &std::path::Path, log: &str, with_archive: bool) {
    let dir = root.join("windows");
    std::fs::create_dir_all(&dir).unwrap();
    let mut lines = log.lines();
    let first = lines.next().unwrap_or_default();
    let doctor = first.strip_prefix("doctor: ").unwrap_or(first);
    std::fs::write(dir.join("unsupported.log"), format!("{doctor}\n")).unwrap();
    let posture = lines.collect::<Vec<_>>().join("\n");
    if !posture.is_empty() {
        std::fs::write(dir.join("posture-tests.log"), format!("{posture}\n")).unwrap();
    }
    if with_archive {
        // A Windows opi-sandbox archive must NOT exist (16.14.2 unsupported).
        std::fs::create_dir_all(dir.join("extracted").join("bin")).unwrap();
        std::fs::write(
            dir.join("extracted").join("bin").join("opi-sandbox"),
            b"must not exist",
        )
        .unwrap();
    }
}

/// A complete, correct evidence tree: native linux + macos bundles + a windows
/// unsupported-posture bundle. Audit must PASS.
fn write_complete_good_evidence(root: &std::path::Path) {
    write_native_bundle(
        root,
        &format!("linux/{LINUX_TARGET}"),
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_native_bundle(
        root,
        &format!("linux/{LINUX_ARM_TARGET}"),
        LINUX_ARM_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_native_bundle(
        root,
        &format!("macos/{MACOS_X64_TARGET}"),
        MACOS_X64_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_native_bundle(
        root,
        &format!("macos/{MACOS_TARGET}"),
        MACOS_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_windows_bundle(root, good_windows_log(), false);
}

#[test]
fn release_audit_passes_complete_native_evidence() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_complete_good_evidence(dir.path());
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        ok,
        "complete native evidence must pass: stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn release_audit_rejects_missing_platform() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    // Omit macos entirely.
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_windows_bundle(dir.path(), good_windows_log(), false);
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        !ok,
        "missing platform must fail: stdout={stdout} stderr={stderr}"
    );
    let report: serde_json::Value =
        serde_json::from_str(&stdout).expect("release audit emits JSON on failure");
    assert!(
        report["issues"]
            .as_array()
            .map(|v| v.iter().any(|i| i["code"] == "missing_platform_evidence"))
            .unwrap_or(false),
        "expected missing_platform_evidence: {stdout}"
    );
}

#[test]
fn release_audit_rejects_wrong_target_identity() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    // linux bundle carrying a darwin target triple -> wrong target identity.
    write_native_bundle(
        dir.path(),
        "linux",
        MACOS_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_native_bundle(
        dir.path(),
        "macos",
        MACOS_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_windows_bundle(dir.path(), good_windows_log(), false);
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        !ok,
        "wrong target must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("wrong_target_identity"),
        "expected wrong_target_identity: {stdout}"
    );
}

#[test]
fn release_audit_rejects_windows_opi_sandbox_archive() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_complete_good_evidence(dir.path());
    // Add a Windows opi-sandbox artifact (forbidden: no Windows artifact).
    write_windows_bundle(dir.path(), good_windows_log(), true);
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        !ok,
        "a Windows opi-sandbox archive must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("wrong_target_identity"),
        "Windows archive is a wrong-target defect: {stdout}"
    );
}

#[test]
fn release_audit_rejects_absent_archive() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    // linux bundle with no extracted tree (smoke ran against a workspace
    // target/ binary, not an extracted archive).
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        true,
    );
    write_native_bundle(
        dir.path(),
        "macos",
        MACOS_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_windows_bundle(dir.path(), good_windows_log(), false);
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        !ok,
        "absent archive must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("missing_archive"),
        "expected missing_archive: {stdout}"
    );
}

#[test]
fn release_audit_rejects_provenance_mismatch() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    // linux extracted binary sha != locked executable_sha256.
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        true,
        false,
    );
    write_native_bundle(
        dir.path(),
        "macos",
        MACOS_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_windows_bundle(dir.path(), good_windows_log(), false);
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        !ok,
        "provenance mismatch must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("provenance_mismatch"),
        "expected provenance_mismatch: {stdout}"
    );
}

#[test]
fn release_audit_rejects_tampered_archive() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_complete_good_evidence(dir.path());
    std::fs::write(
        native_archive_path(
            &dir.path().join(format!("linux/{LINUX_TARGET}")),
            LINUX_TARGET,
        ),
        b"not a tar archive",
    )
    .unwrap();
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(!ok, "tampered archive must fail: {stdout} {stderr}");
    assert!(stdout.contains("invalid_archive_layout"), "{stdout}");
}

#[test]
fn release_audit_rejects_caller_prepared_extracted_tree() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_complete_good_evidence(dir.path());
    let linux_bundle = dir.path().join(format!("linux/{LINUX_TARGET}"));
    std::fs::create_dir_all(linux_bundle.join("extracted/bin")).unwrap();
    std::fs::write(linux_bundle.join("extracted/bin/opi-sandbox"), b"caller").unwrap();
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(!ok, "caller extraction must fail: {stdout} {stderr}");
    assert!(
        stdout.contains("caller_prepared_extracted_tree"),
        "{stdout}"
    );
}

#[test]
fn release_audit_rejects_placeholder_manifest() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_complete_good_evidence(dir.path());
    let stage = tempfile::tempdir().unwrap();
    let manifest = stage.path().join("package.toml");
    let executable = stage.path().join("opi-sandbox");
    std::fs::write(
        &manifest,
        "name = \"opi-sandbox\"\nversion = \"__PACKAGE_VERSION__\"\n",
    )
    .unwrap();
    std::fs::write(&executable, BINARY_BYTES).unwrap();
    create_native_archive(
        &native_archive_path(
            &dir.path().join(format!("linux/{LINUX_TARGET}")),
            LINUX_TARGET,
        ),
        &manifest,
        &executable,
        false,
    );
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(!ok, "placeholder manifest must fail: {stdout} {stderr}");
    assert!(stdout.contains("invalid_package_manifest"), "{stdout}");
}

#[test]
fn release_audit_rejects_extra_archive_layout_member() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_complete_good_evidence(dir.path());
    let stage = tempfile::tempdir().unwrap();
    let manifest = stage.path().join("package.toml");
    let executable = stage.path().join("opi-sandbox");
    std::fs::write(&manifest, b"invalid is irrelevant after layout rejection\n").unwrap();
    std::fs::write(&executable, BINARY_BYTES).unwrap();
    create_native_archive(
        &native_archive_path(
            &dir.path().join(format!("linux/{LINUX_TARGET}")),
            LINUX_TARGET,
        ),
        &manifest,
        &executable,
        true,
    );
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(!ok, "extra archive member must fail: {stdout} {stderr}");
    assert!(stdout.contains("invalid_archive_layout"), "{stdout}");
}

#[test]
fn release_audit_rejects_non_executable_archive_binary() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_complete_good_evidence(dir.path());
    let archive = native_archive_path(
        &dir.path().join(format!("linux/{LINUX_TARGET}")),
        LINUX_TARGET,
    );
    rewrite_native_archive(&archive, "nonexec");
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(!ok, "non-executable archive must fail: {stdout} {stderr}");
    assert!(stdout.contains("invalid_archive_layout"), "{stdout}");
}

#[test]
fn release_audit_rejects_archive_without_protocol_schema() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_complete_good_evidence(dir.path());
    let archive = native_archive_path(
        &dir.path().join(format!("linux/{LINUX_TARGET}")),
        LINUX_TARGET,
    );
    rewrite_native_archive(&archive, "missing-schema");
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(!ok, "missing schema must fail: {stdout} {stderr}");
    assert!(stdout.contains("invalid_archive_layout"), "{stdout}");
}

#[test]
fn release_audit_rejects_oversized_archive_member() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_complete_good_evidence(dir.path());
    let archive = native_archive_path(
        &dir.path().join(format!("linux/{LINUX_TARGET}")),
        LINUX_TARGET,
    );
    rewrite_native_archive(&archive, "oversized-manifest");
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(!ok, "oversized archive must fail: {stdout} {stderr}");
    assert!(stdout.contains("invalid_archive_layout"), "{stdout}");
}

#[test]
fn release_audit_rejects_unknown_adapter_manifest_field() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_complete_good_evidence(dir.path());
    let archive = native_archive_path(
        &dir.path().join(format!("linux/{LINUX_TARGET}")),
        LINUX_TARGET,
    );
    rewrite_native_archive(&archive, "unknown-field");
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(!ok, "unknown manifest field must fail: {stdout} {stderr}");
    assert!(stdout.contains("invalid_package_manifest"), "{stdout}");
}

#[test]
fn release_audit_rejects_raw_archive_member_aliases() {
    for mutation in ["dot-alias", "double-slash-alias", "embedded-dot-alias"] {
        let dir = tempfile::tempdir().expect("release evidence tempdir");
        write_complete_good_evidence(dir.path());
        let bundle = dir.path().join(format!("linux/{LINUX_TARGET}"));
        let archive = native_archive_path(&bundle, LINUX_TARGET);
        let previous_archive_sha = sha256_hex_local(&std::fs::read(&archive).unwrap());
        rewrite_native_archive(&archive, mutation);
        rebind_smoke_to_archive(&bundle, &previous_archive_sha, &archive);

        let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
        assert!(
            !ok,
            "raw alias {mutation} must fail: stdout={stdout} stderr={stderr}"
        );
        assert!(
            stdout.contains("invalid_archive_layout"),
            "raw alias {mutation} was not rejected as archive layout: {stdout}"
        );
    }
}

#[test]
fn release_audit_rejects_versions_outside_the_shared_strict_semver_grammar() {
    for (mutation, malformed_version) in [
        ("version-trailing-hyphen", "1.2.3-"),
        ("version-metacharacter", "1.2.3-&"),
    ] {
        let dir = tempfile::tempdir().expect("release evidence tempdir");
        write_complete_good_evidence(dir.path());
        let bundle = dir.path().join(format!("linux/{LINUX_TARGET}"));
        let archive = native_archive_path(&bundle, LINUX_TARGET);
        let previous_archive_sha = sha256_hex_local(&std::fs::read(&archive).unwrap());
        let manifest_hash = rewrite_native_archive(&archive, mutation);
        rebind_smoke_to_archive(&bundle, &previous_archive_sha, &archive);

        let lock_path = bundle.join("package-lock.toml");
        let mut lock = std::fs::read_to_string(&lock_path).unwrap();
        replace_lock_value(&mut lock, "manifest_hash", &manifest_hash);
        replace_lock_value(&mut lock, "package_version", malformed_version);
        replace_lock_value(&mut lock, "opi_range", ">=1.2.0-0,<1.3.0-0");
        std::fs::write(lock_path, lock).unwrap();

        let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
        assert!(
            !ok,
            "malformed version {malformed_version:?} must fail: stdout={stdout} stderr={stderr}"
        );
        assert!(
            stdout.contains("invalid_package_manifest"),
            "malformed version {malformed_version:?} bypassed strict parsing: {stdout}"
        );
    }
}

#[test]
fn release_audit_rejects_wrong_lock_field() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_complete_good_evidence(dir.path());
    let lock_path = dir
        .path()
        .join(format!("linux/{LINUX_TARGET}/package-lock.toml"));
    let lock = std::fs::read_to_string(&lock_path).unwrap();
    std::fs::write(
        &lock_path,
        lock.replace(
            "protocol = \"command-execution-jsonl-v1\"",
            "protocol = \"wrong-protocol\"",
        ),
    )
    .unwrap();
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(!ok, "wrong lock field must fail: {stdout} {stderr}");
    assert!(stdout.contains("provenance_mismatch"), "{stdout}");
}

#[test]
fn release_audit_rejects_mixed_pass_and_failure_evidence() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_complete_good_evidence(dir.path());
    let smoke_path = dir.path().join(format!("linux/{LINUX_TARGET}/smoke.log"));
    let mut smoke = std::fs::read_to_string(&smoke_path).unwrap();
    smoke.push_str("test result: FAILED. 1 passed; 1 failed\n");
    std::fs::write(&smoke_path, smoke).unwrap();
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(!ok, "mixed pass/failure must fail: {stdout} {stderr}");
    assert!(stdout.contains("failed_evidence"), "{stdout}");
}

#[test]
fn release_audit_rejects_evidence_for_a_different_archive() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_complete_good_evidence(dir.path());
    let smoke_path = dir.path().join(format!("linux/{LINUX_TARGET}/smoke.log"));
    let smoke = std::fs::read_to_string(&smoke_path).unwrap();
    let actual = sha256_hex_local(
        &std::fs::read(native_archive_path(
            &dir.path().join(format!("linux/{LINUX_TARGET}")),
            LINUX_TARGET,
        ))
        .unwrap(),
    );
    std::fs::write(&smoke_path, smoke.replace(&actual, &"f".repeat(64))).unwrap();
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(!ok, "wrong digest evidence must fail: {stdout} {stderr}");
    assert!(stdout.contains("archive_digest_mismatch"), "{stdout}");
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn release_audit_accepts_a_real_packager_produced_archive() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_complete_good_evidence(dir.path());

    let pack = tempfile::tempdir().expect("real packager artifact tempdir");
    let fixture = pack.path().join("fixture-binary");
    std::fs::write(&fixture, BINARY_BYTES).unwrap();
    let output = Command::new("bash")
        .arg(workspace_root().join("scripts/package-opi-sandbox.sh"))
        .arg("--binary")
        .arg(&fixture)
        .arg("--artifact-dir")
        .arg(pack.path())
        .output()
        .expect("run real shell packager");
    assert!(
        output.status.success(),
        "real packager failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let target = std::fs::read_to_string(pack.path().join("target")).unwrap();
    let target = target.trim();
    let platform = if target.ends_with("-unknown-linux-gnu") {
        "linux"
    } else if target.ends_with("-apple-darwin") {
        "macos"
    } else {
        panic!("real packager emitted unsupported native target {target}");
    };
    let bundle = dir.path().join(platform).join(target);
    std::fs::remove_dir_all(&bundle).unwrap();
    std::fs::create_dir_all(&bundle).unwrap();
    for file in ["target", "package-lock.toml"] {
        std::fs::copy(pack.path().join(file), bundle.join(file)).unwrap();
    }
    let archive_name = format!("opi-sandbox-{target}.tar.gz");
    let archive = bundle.join(&archive_name);
    std::fs::copy(pack.path().join(&archive_name), &archive).unwrap();
    let archive_sha = sha256_hex_local(&std::fs::read(&archive).unwrap());
    std::fs::write(
        bundle.join("smoke.log"),
        good_smoke_log().replace("__ARCHIVE_SHA256__", &archive_sha),
    )
    .unwrap();

    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        ok,
        "release audit rejected real packager output: stdout={stdout} stderr={stderr}"
    );
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn release_audit_accepts_real_native_smoke_output_without_traceback() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_complete_good_evidence(dir.path());

    let work = tempfile::tempdir().expect("native smoke work tempdir");
    let target_dir = work.path().join("cargo-target");
    let build = Command::new("cargo")
        .current_dir(workspace_root())
        .env("CARGO_TARGET_DIR", &target_dir)
        .args(["build", "-p", "opi-sandbox", "--bin", "opi-sandbox"])
        .output()
        .expect("build real opi-sandbox binary");
    assert!(
        build.status.success(),
        "real opi-sandbox build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let binary = target_dir.join("debug/opi-sandbox");
    let packaged = work.path().join("packaged");
    let pack = Command::new("bash")
        .arg(workspace_root().join("scripts/package-opi-sandbox.sh"))
        .arg("--binary")
        .arg(&binary)
        .arg("--artifact-dir")
        .arg(&packaged)
        .output()
        .expect("run real shell packager");
    assert!(
        pack.status.success(),
        "real packager failed: {}",
        String::from_utf8_lossy(&pack.stderr)
    );

    let target = std::fs::read_to_string(packaged.join("target")).unwrap();
    let target = target.trim();
    let platform = if target.ends_with("-unknown-linux-gnu") {
        "linux"
    } else if target.ends_with("-apple-darwin") {
        "macos"
    } else {
        panic!("real packager emitted unsupported native target {target}");
    };
    let bundle = dir.path().join(platform).join(target);
    std::fs::remove_dir_all(&bundle).unwrap();
    std::fs::create_dir_all(&bundle).unwrap();
    for file in ["target", "package-lock.toml"] {
        std::fs::copy(packaged.join(file), bundle.join(file)).unwrap();
    }
    let archive_name = format!("opi-sandbox-{target}.tar.gz");
    let archive = bundle.join(&archive_name);
    std::fs::copy(packaged.join(&archive_name), &archive).unwrap();

    let smoke_dir = bundle.join("smoke");
    let smoke = Command::new("bash")
        .arg(workspace_root().join("scripts/opi-sandbox-smoke.sh"))
        .arg("--binary")
        .arg(packaged.join("extracted/bin/opi-sandbox"))
        .arg("--artifact-dir")
        .arg(&smoke_dir)
        .arg("--archive")
        .arg(&archive)
        .output()
        .expect("run real extracted-binary smoke");
    assert!(
        smoke.status.success(),
        "real native smoke failed: stdout={} stderr={}",
        String::from_utf8_lossy(&smoke.stdout),
        String::from_utf8_lossy(&smoke.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(smoke_dir.join("network-deny-stdout.txt")).unwrap(),
        "BIND_DENIED\n"
    );
    let denial_stderr = std::fs::read_to_string(smoke_dir.join("network-deny-stderr.txt")).unwrap();
    assert!(!denial_stderr.contains("Traceback"), "{denial_stderr}");

    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        ok,
        "release audit rejected real smoke output: stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn release_audit_requires_every_named_native_archive_sentinel() {
    for marker in [
        "empty-cwd",
        "setup-failure",
        "filesystem-allow",
        "filesystem-deny",
        "network-deny",
        "network-allow",
    ] {
        let dir = tempfile::tempdir().expect("release evidence tempdir");
        write_complete_good_evidence(dir.path());
        let smoke_path = dir.path().join(format!("linux/{LINUX_TARGET}/smoke.log"));
        let smoke = std::fs::read_to_string(&smoke_path).unwrap();
        let filtered = smoke
            .lines()
            .filter(|line| !line.contains(&format!("opi-sandbox-{marker}-smoke:")))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&smoke_path, format!("{filtered}\n")).unwrap();
        let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
        assert!(
            !ok,
            "missing {marker} sentinel must fail: stdout={stdout} stderr={stderr}"
        );
        assert!(stdout.contains("missing_smoke_evidence"), "{stdout}");
    }
}

#[test]
fn release_audit_rejects_skipped_evidence() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    // smoke evidence shows ignored tests (skipped evidence).
    let skipped = "test result: ok. 8 passed; 0 failed; 2 ignored\n";
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        skipped,
        false,
        false,
    );
    write_native_bundle(
        dir.path(),
        "macos",
        MACOS_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_windows_bundle(dir.path(), good_windows_log(), false);
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        !ok,
        "skipped evidence must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("skipped_evidence"),
        "expected skipped_evidence: {stdout}"
    );
}

#[test]
fn release_audit_rejects_zero_test_evidence() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    // smoke evidence shows 0 passed (zero-test evidence).
    let zero = "test result: ok. 0 passed; 0 failed; 0 ignored\n";
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        zero,
        false,
        false,
    );
    write_native_bundle(
        dir.path(),
        "macos",
        MACOS_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_windows_bundle(dir.path(), good_windows_log(), false);
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        !ok,
        "zero-test evidence must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("zero_test_evidence"),
        "expected zero_test_evidence: {stdout}"
    );
}

#[test]
fn release_audit_rejects_windows_unsupported_without_pass_evidence() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_native_bundle(
        dir.path(),
        "macos",
        MACOS_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    // Windows unsupported log with no passing test evidence (zero-test).
    let no_pass = "doctor: {\"schema_version\":1,\"supported\":false,\"target\":\"windows\",\"mechanisms\":[],\"profiles\":[\"workspace-write\"],\"limitations\":[]}\nrun: refused pre-start\n";
    write_windows_bundle(dir.path(), no_pass, false);
    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        !ok,
        "windows evidence without a pass must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("zero_test_evidence"),
        "windows zero-test evidence must be flagged: {stdout}"
    );
}

#[test]
fn release_audit_rejects_supported_windows_doctor_even_with_free_form_claim() {
    let dir = tempfile::tempdir().expect("release evidence tempdir");
    write_complete_good_evidence(dir.path());
    write_windows_bundle(
        dir.path(),
        "{\"schema_version\":1,\"supported\":true,\"target\":\"windows\",\"mechanisms\":[],\"profiles\":[],\"limitations\":[]}\n\
         unsupported posture: supported = false\n\
         test result: ok. 3 passed; 0 failed; 0 ignored\n",
        false,
    );

    let (ok, stdout, stderr) = run_release_audit(dir.path(), true);
    assert!(
        !ok,
        "a free-form claim must not override doctor JSON: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("wrong_target_identity"),
        "supported=true must fail the Windows posture gate: {stdout}"
    );
}

// ============================================================================
// Phase 16 task 16.16.3: phase-exit evidence mode (`--phase-exit`).
//
// The phase-exit audit validates the preserved Phase 16 phase-exit evidence
// (SC16-15b / the 16.16.3 smoke addendum) against the claimed categories and
// rejects absent, skipped, zero-test, wrong-target, and workspace-only
// evidence. Its Linux and macOS inputs are the same authenticated native
// archive bundles required by `--release`; Windows preserves only its explicit
// unsupported-posture evidence. These tests drive the audit on synthetic
// evidence trees (good + each defect class).
// ============================================================================

/// Run the artifact audit in PHASE-EXIT mode on `dir`.
fn run_phase_exit_audit(dir: &std::path::Path) -> (bool, String, String) {
    run_phase_exit_audit_with_json(dir, false)
}

fn run_phase_exit_audit_with_json(dir: &std::path::Path, json: bool) -> (bool, String, String) {
    let out = Command::new(python_command())
        .arg(
            workspace_root()
                .join("scripts")
                .join("opi-artifact-audit.py"),
        )
        .arg(dir)
        .arg("--phase-exit")
        .args(json.then_some("--json"))
        .output()
        .expect("run phase-exit audit");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Write a macOS native-archive bundle for phase-exit evidence.
fn write_macos_phase_exit_bundle(root: &std::path::Path, with_pass: bool, with_archive: bool) {
    let log = if with_pass {
        good_smoke_log()
    } else {
        "cargo check --target aarch64-apple-darwin\n" // no pass marker
    };
    write_native_bundle(
        root,
        "macos",
        MACOS_TARGET,
        BINARY_BYTES,
        log,
        false,
        !with_archive,
    );
}

/// Write the six-target bundle: one preserved `cargo check --target` log per
/// triple. `green` triples carry a `Finished` line; `failed` triples carry an
/// `error[` line; `ambiguous` triples carry neither; triples absent from the
/// map are omitted entirely.
fn write_six_target_bundle(
    root: &std::path::Path,
    triples: &[(&str, &str)], // (triple, "green" | "failed" | "ambiguous")
) {
    let dir = root.join("six-target");
    std::fs::create_dir_all(&dir).unwrap();
    // The phase-exit audit requires a provenance note for the preserved logs.
    std::fs::write(
        dir.join("source"),
        "ci run 123 @deadbeef target_check job\n",
    )
    .unwrap();
    for (index, (triple, kind)) in triples.iter().enumerate() {
        let body = match *kind {
            "green" => format!("cargo check --target {triple}\nFinished dev profile\n"),
            "failed" => format!("cargo check --target {triple}\nerror[E0]: boom\n"),
            _ => format!("cargo check --target {triple}\n"),
        };
        std::fs::write(dir.join(format!("check-{index}.log")), body).unwrap();
    }
}

/// The DoD gate categories the phase-exit audit requires a capture for, keyed
/// by the filename marker (mirrors GATE_CATEGORIES in opi-artifact-audit.py).
const GATE_CATEGORY_MARKERS: &[&str] = &[
    "doc-guards",
    "crate-boundary",
    "packaging",
    "release-topology",
    "workspace-test",
    "doctest",
    "fmt",
    "clippy",
    "rustdoc",
];

/// Write one pass-marked (or marker-free, when `with_pass` is false) capture per
/// DoD gate category under `gates/`.
fn write_gates_bundle(root: &std::path::Path, with_pass: bool) {
    let dir = root.join("gates");
    std::fs::create_dir_all(&dir).unwrap();
    for marker in GATE_CATEGORY_MARKERS {
        let body = if with_pass {
            "test result: ok. 3 passed; 0 failed; 0 ignored\n"
        } else {
            "some gate ran\n" // no pass marker
        };
        std::fs::write(dir.join(format!("gate-{marker}.txt")), body).unwrap();
    }
}

/// A complete phase-exit evidence tree with Linux and macOS archives, Windows
/// unsupported posture, six green target-check logs, and passing gates.
fn write_complete_phase_exit_evidence(root: &std::path::Path) {
    write_native_bundle(
        root,
        &format!("linux/{LINUX_TARGET}"),
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_native_bundle(
        root,
        &format!("linux/{LINUX_ARM_TARGET}"),
        LINUX_ARM_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_native_bundle(
        root,
        &format!("macos/{MACOS_X64_TARGET}"),
        MACOS_X64_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_native_bundle(
        root,
        &format!("macos/{MACOS_TARGET}"),
        MACOS_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_windows_bundle(root, good_windows_log(), false);
    write_six_target_bundle(
        root,
        &[
            ("x86_64-unknown-linux-gnu", "green"),
            ("aarch64-unknown-linux-gnu", "green"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
            ("aarch64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(root, true);
}

#[test]
fn phase_exit_audit_passes_complete_evidence() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_complete_phase_exit_evidence(dir.path());
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        ok,
        "complete phase-exit evidence must pass: stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn phase_exit_audit_rejects_missing_platform() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    // macos omitted entirely.
    write_windows_bundle(dir.path(), good_windows_log(), false);
    write_six_target_bundle(
        dir.path(),
        &[
            ("x86_64-unknown-linux-gnu", "green"),
            ("aarch64-unknown-linux-gnu", "green"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
            ("aarch64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(dir.path(), true);
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "missing platform must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("missing_platform_evidence"),
        "expected missing_platform_evidence: {stdout}"
    );
}

#[test]
fn phase_exit_audit_rejects_macos_archive_without_pass_marker() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    // A macOS archive without a genuine pass marker is zero-test evidence.
    write_macos_phase_exit_bundle(dir.path(), false, true);
    write_windows_bundle(dir.path(), good_windows_log(), false);
    write_six_target_bundle(
        dir.path(),
        &[
            ("x86_64-unknown-linux-gnu", "green"),
            ("aarch64-unknown-linux-gnu", "green"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
            ("aarch64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(dir.path(), true);
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "macOS evidence without a pass marker must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("zero_test_evidence"),
        "macOS evidence without a pass must be flagged zero-test: {stdout}"
    );
}

#[test]
fn phase_exit_audit_rejects_macos_bundle_without_archive() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    // A pass-marked macOS log without its authenticated archive is not accepted.
    write_macos_phase_exit_bundle(dir.path(), true, false);
    write_windows_bundle(dir.path(), good_windows_log(), false);
    write_six_target_bundle(
        dir.path(),
        &[
            ("x86_64-unknown-linux-gnu", "green"),
            ("aarch64-unknown-linux-gnu", "green"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
            ("aarch64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(dir.path(), true);
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "macOS evidence without an archive must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("missing_archive"),
        "expected missing_archive: {stdout}"
    );
}

#[test]
fn phase_exit_audit_rejects_missing_six_target_triple() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_macos_phase_exit_bundle(dir.path(), true, true);
    write_windows_bundle(dir.path(), good_windows_log(), false);
    // Only 5 of the 6 release triples are preserved.
    write_six_target_bundle(
        dir.path(),
        &[
            ("x86_64-unknown-linux-gnu", "green"),
            ("aarch64-unknown-linux-gnu", "green"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(dir.path(), true);
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "a missing six-target triple must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("missing_target_evidence"),
        "expected missing_target_evidence: {stdout}"
    );
}

#[test]
fn phase_exit_audit_rejects_ambiguous_six_target_log() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_macos_phase_exit_bundle(dir.path(), true, true);
    write_windows_bundle(dir.path(), good_windows_log(), false);
    // One log records neither a Finished check nor a compiler error.
    write_six_target_bundle(
        dir.path(),
        &[
            ("x86_64-unknown-linux-gnu", "green"),
            ("aarch64-unknown-linux-gnu", "ambiguous"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
            ("aarch64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(dir.path(), true);
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "an outcome-less target log must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("ambiguous_target_evidence"),
        "expected ambiguous_target_evidence: {stdout}"
    );
}

#[test]
fn phase_exit_audit_rejects_gate_without_pass_marker() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_macos_phase_exit_bundle(dir.path(), true, true);
    write_windows_bundle(dir.path(), good_windows_log(), false);
    write_six_target_bundle(
        dir.path(),
        &[
            ("x86_64-unknown-linux-gnu", "green"),
            ("aarch64-unknown-linux-gnu", "green"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
            ("aarch64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(dir.path(), false);
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "a gate capture without a pass marker must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("zero_test_evidence"),
        "gate capture without a pass must be flagged zero-test: {stdout}"
    );
}

#[test]
fn phase_exit_audit_rejects_failed_target_evidence() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_macos_phase_exit_bundle(dir.path(), true, true);
    write_windows_bundle(dir.path(), good_windows_log(), false);
    // One linux triple records a compiler failure -> the six-target gate is NOT
    // green and must be flagged, even though a preserved log exists.
    write_six_target_bundle(
        dir.path(),
        &[
            ("x86_64-unknown-linux-gnu", "failed"),
            ("aarch64-unknown-linux-gnu", "green"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
            ("aarch64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(dir.path(), true);
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "a compiler-failure target log must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("failed_target_evidence"),
        "expected failed_target_evidence: {stdout}"
    );
}

/// A complete, correct evidence tree whose workspace-test capture carries a
/// genuine pass line plus a `test result: FAILED` line (a run that both passed
/// some binaries and failed one). The audit must reject it as failed evidence.
#[test]
fn phase_exit_audit_rejects_gate_with_failed_test() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_macos_phase_exit_bundle(dir.path(), true, true);
    write_windows_bundle(dir.path(), good_windows_log(), false);
    write_six_target_bundle(
        dir.path(),
        &[
            ("x86_64-unknown-linux-gnu", "green"),
            ("aarch64-unknown-linux-gnu", "green"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
            ("aarch64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(dir.path(), true);
    // Overwrite the workspace-test capture with a run that ended FAILED.
    std::fs::write(
        dir.path().join("gates").join("gate-workspace-test.txt"),
        "test result: ok. 19 passed; 0 failed; 0 ignored\n\
         test result: FAILED. 1 passed; 1 failed\n\
         error: test failed, to rerun pass `-p opi-coding-agent --test x`\n",
    )
    .unwrap();
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "a gate capture recording a failed run must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("failed_gate_evidence"),
        "expected failed_gate_evidence: {stdout}"
    );
}

/// A test-based gate capture with only `0 passed` lines plus a Finished line is
/// zero-test evidence and must be rejected (the Finished fallback is reserved
/// for the non-test gates).
#[test]
fn phase_exit_audit_rejects_zero_test_gate_capture() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_native_bundle(
        dir.path(),
        "linux",
        LINUX_TARGET,
        BINARY_BYTES,
        good_smoke_log(),
        false,
        false,
    );
    write_macos_phase_exit_bundle(dir.path(), true, true);
    write_windows_bundle(dir.path(), good_windows_log(), false);
    write_six_target_bundle(
        dir.path(),
        &[
            ("x86_64-unknown-linux-gnu", "green"),
            ("aarch64-unknown-linux-gnu", "green"),
            ("x86_64-apple-darwin", "green"),
            ("aarch64-apple-darwin", "green"),
            ("x86_64-pc-windows-msvc", "green"),
            ("aarch64-pc-windows-msvc", "green"),
        ],
    );
    write_gates_bundle(dir.path(), true);
    // Overwrite the doctest capture: 0 passed but a Finished line -> zero-test.
    std::fs::write(
        dir.path().join("gates").join("gate-doctest.txt"),
        "test result: ok. 0 passed; 0 failed; 0 ignored\nFinished `test` profile\n",
    )
    .unwrap();
    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "a 0-passed test-based capture must be rejected: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("zero_test_evidence"),
        "expected zero_test_evidence for the 0-passed doctest capture: {stdout}"
    );
}

#[test]
fn phase_exit_audit_rejects_ignored_test_gate_capture() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_complete_phase_exit_evidence(dir.path());
    std::fs::write(
        dir.path().join("gates").join("gate-workspace-test.txt"),
        "test result: ok. 3 passed; 0 failed; 2 ignored\n",
    )
    .unwrap();

    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        !ok,
        "an ignored test-based gate must be rejected: stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("skipped_evidence"),
        "expected skipped_evidence for an ignored test-based gate: {stdout}"
    );
}

#[test]
fn phase_exit_audit_does_not_apply_test_result_rules_to_non_test_gates() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_complete_phase_exit_evidence(dir.path());
    std::fs::write(
        dir.path().join("gates").join("gate-fmt.txt"),
        "test result: ok. 3 passed; 0 failed; 2 ignored\nFinished `dev` profile\n",
    )
    .unwrap();

    let (ok, stdout, stderr) = run_phase_exit_audit(dir.path());
    assert!(
        ok,
        "an incidental test line must not break a non-test gate: stdout={stdout} stderr={stderr}"
    );
}

fn replace_scalar_with_directory(path: &std::path::Path) {
    std::fs::remove_file(path).expect("remove scalar fixture");
    std::fs::create_dir(path).expect("replace scalar fixture with directory");
}

fn assert_structured_file_issue(stdout: &str, code: &str, path: &std::path::Path, case: &str) {
    let report: serde_json::Value = serde_json::from_str(stdout).unwrap_or_else(|error| {
        panic!("{case} did not produce structured JSON: {error}: {stdout}")
    });
    let expected_file = path.display().to_string().replace('\\', "/");
    assert!(
        report["issues"].as_array().is_some_and(|issues| {
            issues.iter().any(|issue| {
                issue["code"] == code
                    && issue["file"]
                        .as_str()
                        .is_some_and(|file| file.replace('\\', "/") == expected_file)
            })
        }),
        "{case} did not produce {code} attributed to {}: {stdout}",
        path.display()
    );
}

#[test]
fn artifact_audit_reports_each_expected_scalar_wrong_kind_without_traceback() {
    for relative in [
        "RUN_SUMMARY.md",
        "REVIEW_REPORT.md",
        "run.stderr.log",
        "run.ndjson",
        "sessions/s.jsonl",
    ] {
        let dir = tempfile::tempdir().expect("artifact evidence tempdir");
        std::fs::create_dir(dir.path().join("sessions")).unwrap();
        for file in [
            "RUN_SUMMARY.md",
            "REVIEW_REPORT.md",
            "run.stderr.log",
            "run.ndjson",
            "sessions/s.jsonl",
        ] {
            std::fs::write(dir.path().join(file), b"").unwrap();
        }
        replace_scalar_with_directory(&dir.path().join(relative));

        let (ok, stdout, stderr) = run_audit_with_args(dir.path(), dir.path(), true);
        assert!(
            !ok,
            "wrong-kind scalar {relative} must fail: stdout={stdout} stderr={stderr}"
        );
        assert!(
            !stderr.contains("Traceback"),
            "wrong-kind scalar {relative} escaped as a traceback: {stderr}"
        );
        let path = dir.path().join(relative);
        assert_structured_file_issue(&stdout, "evidence_filesystem_error", &path, relative);
        let attributed_path = path.display().to_string().replace('\\', "/");

        let (text_ok, text_stdout, text_stderr) = run_audit(dir.path());
        assert!(!text_ok, "wrong-kind scalar {relative} passed in text mode");
        assert!(
            text_stdout.contains("evidence_filesystem_error")
                && text_stdout.replace('\\', "/").contains(&attributed_path),
            "wrong-kind scalar {relative} was not attributable in text mode: {text_stdout}"
        );
        assert!(
            !text_stderr.contains("Traceback"),
            "wrong-kind scalar {relative} escaped in text mode: {text_stderr}"
        );
    }
}

#[test]
fn phase_exit_audit_reports_each_expected_scalar_wrong_kind_without_traceback() {
    for relative in [
        format!("linux/{LINUX_TARGET}/target"),
        format!("linux/{LINUX_TARGET}/package-lock.toml"),
        format!("linux/{LINUX_TARGET}/opi-sandbox-{LINUX_TARGET}.tar.gz"),
        format!("linux/{LINUX_TARGET}/smoke.log"),
        "windows/unsupported.log".to_string(),
        "windows/posture-tests.log".to_string(),
        "six-target/source".to_string(),
        "six-target/check-0.log".to_string(),
        "gates/gate-workspace-test.txt".to_string(),
        "gates/gate-fmt.txt".to_string(),
    ] {
        let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
        write_complete_phase_exit_evidence(dir.path());
        replace_scalar_with_directory(&dir.path().join(&relative));

        let (ok, stdout, stderr) = run_phase_exit_audit_with_json(dir.path(), true);
        assert!(
            !ok,
            "wrong-kind scalar {relative} must fail: stdout={stdout} stderr={stderr}"
        );
        assert!(
            !stderr.contains("Traceback"),
            "wrong-kind scalar {relative} escaped as a traceback: {stderr}"
        );
        let path = dir.path().join(&relative);
        assert_structured_file_issue(&stdout, "evidence_filesystem_error", &path, &relative);
        let attributed_path = path.display().to_string().replace('\\', "/");

        let (text_ok, text_stdout, text_stderr) = run_phase_exit_audit(dir.path());
        assert!(!text_ok, "wrong-kind scalar {relative} passed in text mode");
        assert!(
            text_stdout.contains("evidence_filesystem_error")
                && text_stdout.replace('\\', "/").contains(&attributed_path),
            "wrong-kind scalar {relative} was not attributable in text mode: {text_stdout}"
        );
        assert!(
            !text_stderr.contains("Traceback"),
            "wrong-kind scalar {relative} escaped in text mode: {text_stderr}"
        );
    }
}

#[test]
fn artifact_audit_does_not_misclassify_missing_git_as_evidence_read_failure() {
    let dir = tempfile::tempdir().expect("artifact evidence tempdir");
    std::fs::write(
        dir.path().join("RUN_SUMMARY.md"),
        "Head commit at authoring: ffffffffffffffffffffffffffffffffffffffff\n",
    )
    .unwrap();
    let empty_path = tempfile::tempdir().expect("empty PATH tempdir");
    let python = Command::new(python_command())
        .args(["-c", "import sys; print(sys.executable)"])
        .output()
        .expect("resolve Python executable");
    assert!(python.status.success(), "resolve Python executable");
    let python = String::from_utf8(python.stdout)
        .expect("Python executable path is UTF-8")
        .trim()
        .to_string();

    let out = Command::new(python)
        .arg(
            workspace_root()
                .join("scripts")
                .join("opi-artifact-audit.py"),
        )
        .arg(dir.path())
        .arg("--workspace-root")
        .arg(workspace_root())
        .arg("--json")
        .env("PATH", empty_path.path())
        .output()
        .expect("run audit without git on PATH");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "missing git must fail the audit: {stdout}"
    );
    assert!(
        !stderr.contains("Traceback"),
        "missing git escaped: {stderr}"
    );
    assert!(
        stdout.contains("commit_reference_check_failed"),
        "missing git lost its operational diagnostic: {stdout}"
    );
    assert!(
        !stdout.contains("evidence_filesystem_error"),
        "missing git was misclassified as an evidence read error: {stdout}"
    );
}

#[test]
fn phase_exit_evidence_read_failure_preserves_prior_findings() {
    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_complete_phase_exit_evidence(dir.path());
    std::fs::write(
        dir.path().join(format!("linux/{LINUX_TARGET}/target")),
        MACOS_TARGET,
    )
    .unwrap();
    let unreadable_shape = dir.path().join(format!("linux/{LINUX_ARM_TARGET}/target"));
    replace_scalar_with_directory(&unreadable_shape);

    let (ok, stdout, stderr) = run_phase_exit_audit_with_json(dir.path(), true);
    assert!(
        !ok,
        "defective evidence must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        !stderr.contains("Traceback"),
        "read failure escaped: {stderr}"
    );
    assert!(
        stdout.contains("wrong_target_identity"),
        "finding recorded before the read failure was lost: {stdout}"
    );
    assert_structured_file_issue(
        &stdout,
        "evidence_filesystem_error",
        &unreadable_shape,
        "later wrong-kind target",
    );
}

#[cfg(unix)]
#[test]
fn phase_exit_audit_reports_unreadable_expected_scalar() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("phase-exit evidence tempdir");
    write_complete_phase_exit_evidence(dir.path());
    let target = dir.path().join(format!("linux/{LINUX_TARGET}/target"));
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o000)).unwrap();
    let result = run_phase_exit_audit_with_json(dir.path(), true);
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).unwrap();

    let (ok, stdout, stderr) = result;
    assert!(
        !ok,
        "unreadable target must fail: stdout={stdout} stderr={stderr}"
    );
    assert!(
        !stderr.contains("Traceback"),
        "unreadable target escaped: {stderr}"
    );
    assert_structured_file_issue(
        &stdout,
        "evidence_filesystem_error",
        &target,
        "unreadable target",
    );
}
