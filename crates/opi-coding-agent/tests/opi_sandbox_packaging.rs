//! Host-neutral opi-sandbox packaging contract (Phase 16 task 16.15.1).
//!
//! Drives the platform-native packager (`cfg!(windows)` ->
//! `package-opi-sandbox.ps1`, otherwise `package-opi-sandbox.sh`) against a
//! fixture binary, then proves:
//!   - the rendered manifest round-trips through 16.4 `validate_executable_
//!     contributions` (which internally runs `deserialize_raw_contribution`,
//!     so the required `adapter_config` table is exercised);
//!   - the emitted build-time lock matches the canonical `LockMaterial` (same
//!     manifest_hash / executable_sha256 / target / protocol / adapter_id);
//!   - the extracted staging tree carries identical bytes and hashes, with
//!     package contents at the archive root (no wrapping directory);
//!   - `--verify` recomputes manifest_hash + both trees' executable hashes and
//!     rejects a tampered manifest, a tampered extracted binary, or a missing
//!     layout member;
//!   - usage errors (missing/empty binary) exit 2.
//!
//! Parity between the `.sh` and `.ps1` is enforced indirectly: every OS asserts
//! the script's emitted lock against the same canonical Rust computation
//! (`sha256` lowercase, `manifest_hash` LF-normalized), so an encoding or casing
//! drift in either script surfaces as a failure on its own OS. The packager only
//! hashes/copies bytes and never runs a binary (executability + native run are
//! install-time 16.4 + native-run 16.13/16.14.1), so a small fixture file
//! faithfully exercises every packager code path; real-binary execution is owned
//! by 16.13/16.14.1 (`substrate_only` classification).

#![forbid(unsafe_code)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sha2::{Digest, Sha256};

use opi_coding_agent::execution::{PackageSource, validate_executable_contributions};
use opi_coding_agent::package_activation::host_target_triple;
use opi_coding_agent::package_discovery::PackageManifest;

/// The fixture payload. The packager hashes/copies these bytes verbatim; it
/// never executes them.
const FIXTURE_BYTES: &[u8] = b"opi-sandbox packaging fixture payload\n";

fn script_path() -> PathBuf {
    // CARGO_MANIFEST_DIR is the opi-coding-agent crate dir
    // (<workspace>/crates/opi-coding-agent); the packaging scripts live at the
    // workspace root. Canonicalize, then strip the \\?\ verbatim prefix, which
    // breaks PowerShell $PSScriptRoot under -File.
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = if cfg!(windows) {
        crate_dir
            .join("..")
            .join("..")
            .join("scripts")
            .join("package-opi-sandbox.ps1")
    } else {
        crate_dir
            .join("..")
            .join("..")
            .join("scripts")
            .join("package-opi-sandbox.sh")
    };
    let canonical = script
        .canonicalize()
        .unwrap_or_else(|e| panic!("packager script not found: {e}"));
    let s = canonical.to_string_lossy().into_owned();
    let stripped = s.strip_prefix(r"\\?\").unwrap_or(&s);
    PathBuf::from(stripped)
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Build the pack command for the platform-native script.
fn pack_cmd(script: &Path, fixture: &Path, artifact: &Path) -> Command {
    if cfg!(windows) {
        let mut c = Command::new("powershell");
        c.args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ]);
        c.arg(script);
        c.arg("-BinaryPath").arg(fixture);
        c.arg("-ArtifactDir").arg(artifact);
        c
    } else {
        let mut c = Command::new("bash");
        c.arg(script)
            .arg("--binary")
            .arg(fixture)
            .arg("--artifact-dir")
            .arg(artifact);
        c
    }
}

/// Build the verify command for the platform-native script.
fn verify_cmd(script: &Path, artifact: &Path) -> Command {
    if cfg!(windows) {
        let mut c = Command::new("powershell");
        c.args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ]);
        c.arg(script);
        c.arg("-ArtifactDir").arg(artifact);
        c.arg("-Verify");
        c
    } else {
        let mut c = Command::new("bash");
        c.arg(script)
            .arg("--artifact-dir")
            .arg(artifact)
            .arg("--verify");
        c
    }
}

/// Run a command, panicking with captured stderr on failure.
fn run(mut cmd: Command) -> Output {
    cmd.output()
        .unwrap_or_else(|e| panic!("failed to spawn packager: {e}"))
}

fn assert_pack_failure(output: &Output, expected_code: i32) {
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code,
        expected_code,
        "expected packager exit {expected_code}, got {code}\n--- stderr ---\n{}\n--- stdout ---\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );
}

/// The distribution archive produced under the artifact dir.
fn archive_path(artifact: &Path) -> Option<PathBuf> {
    fs::read_dir(artifact)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            name.starts_with("opi-sandbox-")
                && (name.ends_with(".tar.gz") || name.ends_with(".zip"))
        })
}

/// Pack a fresh fixture into a fresh artifact dir; assert success. Returns the
/// resolved paths.
struct Packed {
    artifact: PathBuf,
    script: PathBuf,
    pkg_dir: PathBuf,
    extracted: PathBuf,
}

fn pack_fresh() -> Packed {
    let tmp = tempfile::tempdir().expect("tempdir");
    let artifact = tmp.path().join("artifact");
    let fixture_dir = tmp.path().join("in");
    fs::create_dir_all(&fixture_dir).unwrap();
    let fixture = fixture_dir.join("fixture-binary");
    fs::write(&fixture, FIXTURE_BYTES).unwrap();
    let script = script_path();

    let output = run(pack_cmd(&script, &fixture, &artifact));
    assert!(
        output.status.success(),
        "pack failed:\n--- stderr ---\n{}\n--- stdout ---\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );

    // Keep the tempdir alive for the test body.
    std::mem::forget(tmp);

    let pkg_dir = artifact.join("package");
    let extracted = artifact.join("extracted");
    Packed {
        artifact,
        script,
        pkg_dir,
        extracted,
    }
}

#[test]
fn packer_builds_valid_layout_lock_and_extraction() {
    let p = pack_fresh();
    let host_target = host_target_triple();

    // Rendered manifest round-trips through PackageManifest + 16.4 validation.
    let pkg_toml_path = p.pkg_dir.join("package.toml");
    let pkg_toml_bytes = fs::read(&pkg_toml_path).unwrap();
    let manifest = PackageManifest::from_toml(
        &String::from_utf8(pkg_toml_bytes.clone()).unwrap(),
        &pkg_toml_path,
    )
    .expect("rendered manifest parses");
    assert_eq!(
        manifest.adapter_contributions.len(),
        1,
        "one adapter contribution"
    );
    let validated = validate_executable_contributions(
        &manifest,
        &pkg_toml_bytes,
        &p.pkg_dir,
        PackageSource::Global,
        host_target,
        "0.8.5",
    )
    .expect("rendered manifest round-trips through 16.4 validation");
    let canonical = &validated[0].lock;

    // Canonical LockMaterial correctness.
    assert_eq!(canonical.protocol, "command-execution-jsonl-v1");
    assert_eq!(canonical.adapter_id, "opi-sandbox");
    assert_eq!(canonical.target, host_target);
    assert_eq!(canonical.executable_rel_path, "bin/opi-sandbox");
    assert_eq!(canonical.package_version, "0.8.0");
    assert_eq!(canonical.opi_range, ">=0.8,<0.9");
    // The packager hashed the fixture bytes (lowercase hex).
    assert_eq!(canonical.executable_sha256, sha256_hex(FIXTURE_BYTES));

    // Emitted build-time lock matches the canonical lock (fixed format).
    let lock_str = fs::read_to_string(p.artifact.join("package-lock.toml")).unwrap();
    for (key, value) in [
        ("manifest_hash", canonical.manifest_hash.as_str()),
        ("executable_sha256", canonical.executable_sha256.as_str()),
        ("target", canonical.target.as_str()),
        (
            "executable_rel_path",
            canonical.executable_rel_path.as_str(),
        ),
        ("package_version", canonical.package_version.as_str()),
        ("opi_range", canonical.opi_range.as_str()),
        ("protocol", canonical.protocol.as_str()),
        ("adapter_id", canonical.adapter_id.as_str()),
    ] {
        assert!(
            lock_str.contains(&format!("{key} = \"{value}\"")),
            "package-lock.toml missing `{key} = \"{value}\"`\n{lock_str}"
        );
    }

    // Package layout: bin/opi-sandbox present, identical to the fixture.
    let pkg_bin = p.pkg_dir.join("bin").join("opi-sandbox");
    assert_eq!(
        fs::read(&pkg_bin).unwrap(),
        FIXTURE_BYTES,
        "package copy == fixture"
    );

    // Archive produced.
    let archive = archive_path(&p.artifact).expect("archive produced");
    assert!(
        archive.is_file(),
        "archive is a file: {}",
        archive.display()
    );

    // Extracted staging tree: package contents at the archive root (no wrapping
    // directory), identical bytes + hash.
    let extracted_toml = p.extracted.join("package.toml");
    let extracted_bin = p.extracted.join("bin").join("opi-sandbox");
    assert!(extracted_toml.is_file(), "extracted package.toml at root");
    assert!(extracted_bin.is_file(), "extracted bin/opi-sandbox at root");
    assert_eq!(
        fs::read(&extracted_bin).unwrap(),
        FIXTURE_BYTES,
        "extracted bytes == fixture"
    );
    assert_eq!(
        sha256_hex(&fs::read(&extracted_bin).unwrap()),
        canonical.executable_sha256,
    );
    // The extracted manifest hashes identically (no CRLF/encoding drift).
    assert_eq!(
        sha256_hex(&lf_strip(&fs::read(&extracted_toml).unwrap())),
        canonical.manifest_hash,
        "extracted manifest hashes to the locked manifest_hash",
    );
}

#[test]
fn verify_passes_immediately_after_pack() {
    let p = pack_fresh();
    let output = run(verify_cmd(&p.script, &p.artifact));
    assert!(
        output.status.success(),
        "verify should pass on a fresh pack:\n--- stderr ---\n{}\n--- stdout ---\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );
}

#[test]
fn verify_rejects_tampered_manifest() {
    let p = pack_fresh();
    // Append a TOML comment to the packaged manifest; manifest_hash diverges.
    let pkg_toml = p.pkg_dir.join("package.toml");
    fs::OpenOptions::new()
        .append(true)
        .open(&pkg_toml)
        .unwrap()
        .write_all(b"# tampered\n")
        .unwrap();
    let output = run(verify_cmd(&p.script, &p.artifact));
    assert!(
        !output.status.success(),
        "verify must reject a tampered manifest:\n{:#?}",
        output
    );
}

#[test]
fn verify_rejects_tampered_extracted_executable() {
    let p = pack_fresh();
    let extracted_bin = p.extracted.join("bin").join("opi-sandbox");
    fs::OpenOptions::new()
        .append(true)
        .open(&extracted_bin)
        .unwrap()
        .write_all(b"tampered\n")
        .unwrap();
    let output = run(verify_cmd(&p.script, &p.artifact));
    assert!(
        !output.status.success(),
        "verify must reject a tampered extracted executable:\n{:#?}",
        output
    );
}

#[test]
fn verify_rejects_missing_layout_member() {
    let p = pack_fresh();
    fs::remove_file(p.extracted.join("bin").join("opi-sandbox")).unwrap();
    let output = run(verify_cmd(&p.script, &p.artifact));
    assert!(
        !output.status.success(),
        "verify must reject a missing layout member:\n{:#?}",
        output
    );
}

#[test]
fn pack_rejects_missing_binary() {
    let tmp = tempfile::tempdir().unwrap();
    let artifact = tmp.path().join("artifact");
    let script = script_path();
    let output = run(pack_cmd(
        &script,
        &tmp.path().join("does-not-exist"),
        &artifact,
    ));
    assert_pack_failure(&output, 2);
}

#[test]
fn pack_rejects_empty_binary() {
    let tmp = tempfile::tempdir().unwrap();
    let artifact = tmp.path().join("artifact");
    let fixture = tmp.path().join("empty-binary");
    fs::write(&fixture, b"").unwrap();
    let script = script_path();
    let output = run(pack_cmd(&script, &fixture, &artifact));
    assert_pack_failure(&output, 2);
}

/// Drop every 0x0D byte (mirror `execution::contribution::lf_normalize`'s
/// CR-stripping) so an independent manifest_hash recomputation matches the
/// packager + 16.4 regardless of host line endings.
fn lf_strip(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().filter(|&&b| b != 0x0D).copied().collect()
}
