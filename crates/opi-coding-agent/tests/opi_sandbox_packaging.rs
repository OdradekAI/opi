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
//!   - `--verify` ignores caller-owned staging trees, independently extracts
//!     the recorded-target archive, and rejects archive tampering, missing or
//!     extra members, duplicates, and non-regular members;
//!   - both platform wrappers use one strict SemVer/literal renderer;
//!   - executable headers match one of the four supported release targets;
//!   - usage errors (missing/empty/invalid binary) exit 2.
//!
//! Parity between the `.sh` and `.ps1` is enforced directly by pinning both
//! wrappers to the same helper and exercising its strict version matrix. Every
//! OS also asserts the emitted lock against the same canonical Rust computation
//! (`sha256` lowercase, `manifest_hash` LF-normalized). Header checks never run
//! the binary; native-run behavior remains owned by 16.13/16.14.1.

#![forbid(unsafe_code)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sha2::{Digest, Sha256};

use opi_coding_agent::execution::{PackageSource, validate_executable_contributions};
use opi_coding_agent::package_activation::{host_opi_version, host_target_triple};
use opi_coding_agent::package_discovery::PackageManifest;

const LINUX_X64_TARGET: &str = "x86_64-unknown-linux-gnu";
const LINUX_ARM64_TARGET: &str = "aarch64-unknown-linux-gnu";
const DARWIN_X64_TARGET: &str = "x86_64-apple-darwin";
const DARWIN_ARM64_TARGET: &str = "aarch64-apple-darwin";

fn minimal_elf64(machine: u16) -> Vec<u8> {
    let mut bytes = vec![0; 64];
    bytes[..4].copy_from_slice(b"\x7fELF");
    bytes[4] = 2; // ELFCLASS64
    bytes[5] = 1; // ELFDATA2LSB
    bytes[6] = 1; // EV_CURRENT
    bytes[16..18].copy_from_slice(&2_u16.to_le_bytes()); // ET_EXEC
    bytes[18..20].copy_from_slice(&machine.to_le_bytes());
    bytes[20..24].copy_from_slice(&1_u32.to_le_bytes()); // EV_CURRENT
    bytes[52..54].copy_from_slice(&64_u16.to_le_bytes());
    bytes
}

fn minimal_macho64(cpu_type: u32) -> Vec<u8> {
    let mut bytes = vec![0; 32];
    bytes[..4].copy_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]); // MH_MAGIC_64, little-endian
    bytes[4..8].copy_from_slice(&cpu_type.to_le_bytes());
    bytes[12..16].copy_from_slice(&2_u32.to_le_bytes()); // MH_EXECUTE
    bytes
}

fn executable_fixture(target: &str) -> Vec<u8> {
    match target {
        LINUX_X64_TARGET => minimal_elf64(62),
        LINUX_ARM64_TARGET => minimal_elf64(183),
        DARWIN_X64_TARGET => minimal_macho64(0x0100_0007),
        DARWIN_ARM64_TARGET => minimal_macho64(0x0100_000c),
        _ => panic!("unsupported fixture target {target}"),
    }
}

fn package_target() -> &'static str {
    if cfg!(windows) {
        // Windows has no official package family. Portable wrapper tests use a
        // rustc host shim naming one supported target; real native coverage is
        // cfg-gated below.
        LINUX_X64_TARGET
    } else {
        host_target_triple()
    }
}

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

fn package_helper_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("scripts")
        .join("opi-sandbox-package.py")
}

fn python_command() -> Command {
    Command::new(if cfg!(windows) { "python" } else { "python3" })
}

fn validate_executable_cmd(binary: &Path, target: &str) -> Command {
    let mut command = python_command();
    command
        .arg(package_helper_path())
        .arg("validate-executable")
        .arg("--binary")
        .arg(binary)
        .arg("--target")
        .arg(target);
    command
}

fn run_header_validation(bytes: &[u8], target: &str) -> Output {
    let temp = tempfile::tempdir().expect("header fixture tempdir");
    let binary = temp.path().join("opi-sandbox");
    fs::write(&binary, bytes).unwrap();
    run(validate_executable_cmd(&binary, target))
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn compatible_minor_range(version: &str) -> String {
    let core = version.split(['-', '+']).next().expect("version core");
    let mut parts = core.split('.');
    let major: u64 = parts.next().expect("major").parse().expect("numeric major");
    let minor: u64 = parts.next().expect("minor").parse().expect("numeric minor");
    format!(">={major}.{minor}.0-0,<{major}.{}.0-0", minor + 1)
}

/// Build the pack command for the platform-native script.
fn pack_cmd(script: &Path, fixture: &Path, artifact: &Path) -> Command {
    if cfg!(windows) {
        let shim_dir = artifact
            .parent()
            .expect("artifact has a parent")
            .join("rustc-shim");
        fs::create_dir_all(&shim_dir).unwrap();
        fs::write(
            shim_dir.join("rustc.cmd"),
            "@echo off\r\necho rustc 1.97.0\r\necho host: x86_64-unknown-linux-gnu\r\n",
        )
        .unwrap();
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
        let mut paths = vec![shim_dir];
        paths.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        c.env(
            "PATH",
            std::env::join_paths(paths).expect("compose PATH with rustc shim"),
        );
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

const REWRITE_ARCHIVE_PY: &str = r#"
import hashlib, io, json, pathlib, re, stat, sys, tarfile, zipfile

archive = pathlib.Path(sys.argv[1])
root = pathlib.Path(sys.argv[2])
mode = sys.argv[3]
members = [
    "package.toml",
    "bin/opi-sandbox",
    "schemas/command-execution-jsonl-v1.schema.json",
    "licenses/LICENSE",
]
if mode == "missing":
    members.remove("licenses/LICENSE")

overrides = {}
if mode in ("cpu-swap", "malformed-header"):
    binary = bytearray((root / "bin/opi-sandbox").read_bytes())
    if binary[:4] == b"\x7fELF":
        if mode == "cpu-swap":
            machine = int.from_bytes(binary[18:20], "little")
            binary[18:20] = (183 if machine == 62 else 62).to_bytes(2, "little")
        else:
            binary[16:18] = (0).to_bytes(2, "little")
    elif binary[:4] == b"\xcf\xfa\xed\xfe":
        if mode == "cpu-swap":
            cpu = int.from_bytes(binary[4:8], "little")
            binary[4:8] = (0x0100000c if cpu == 0x01000007 else 0x01000007).to_bytes(4, "little")
        else:
            binary[12:16] = (1).to_bytes(4, "little")
    else:
        raise SystemExit("cpu-swap fixture requires ELF64 or Mach-O64")
    overrides["bin/opi-sandbox"] = bytes(binary)
    executable_sha = hashlib.sha256(binary).hexdigest().encode()
    manifest, count = re.subn(
        br'(?m)^sha256 = "[0-9a-f]{64}"$',
        b'sha256 = "' + executable_sha + b'"',
        (root / "package.toml").read_bytes(),
        count=1,
    )
    if count != 1:
        raise SystemExit("manifest executable SHA fixture not found")
    overrides["package.toml"] = manifest

def archive_name(name):
    if mode == "dot-alias" and name == "package.toml":
        return "./package.toml"
    if mode == "double-slash-alias" and name == "bin/opi-sandbox":
        return "bin//opi-sandbox"
    return name

def member_payload(name):
    if name in overrides:
        return overrides[name]
    payload = (root / name).read_bytes()
    if mode == "same-id-schema" and name == "schemas/command-execution-jsonl-v1.schema.json":
        schema = json.loads(payload)
        schema["same_id_tamper"] = True
        return (json.dumps(schema, separators=(",", ":")) + "\n").encode()
    return payload

if archive.name.endswith(".zip"):
    with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as out:
        for name in members:
            if mode == "wrong-kind" and name == "bin/opi-sandbox":
                info = zipfile.ZipInfo(archive_name(name))
                info.create_system = 3
                info.external_attr = (stat.S_IFLNK | 0o777) << 16
                out.writestr(info, "elsewhere")
            elif mode == "same-id-schema" and name == "schemas/command-execution-jsonl-v1.schema.json":
                info = zipfile.ZipInfo(archive_name(name))
                info.create_system = 3
                info.external_attr = (stat.S_IFREG | 0o644) << 16
                out.writestr(info, member_payload(name))
            elif mode in ("cpu-swap", "malformed-header"):
                info = zipfile.ZipInfo(archive_name(name))
                info.create_system = 3
                permissions = 0o755 if name == "bin/opi-sandbox" else 0o644
                info.external_attr = (stat.S_IFREG | permissions) << 16
                out.writestr(info, member_payload(name))
            elif archive_name(name) != name:
                info = zipfile.ZipInfo(archive_name(name))
                info.create_system = 3
                info.external_attr = (stat.S_IFREG | 0o644) << 16
                out.writestr(info, member_payload(name))
            else:
                out.write(root / name, archive_name(name))
        if mode == "extra":
            out.writestr("unexpected.txt", "unexpected")
        if mode == "duplicate":
            out.write(root / "package.toml", "package.toml")
else:
    with tarfile.open(archive, "w:gz") as out:
        for name in members:
            if mode == "wrong-kind" and name == "bin/opi-sandbox":
                info = tarfile.TarInfo(archive_name(name))
                info.type = tarfile.SYMTYPE
                info.linkname = "elsewhere"
                out.addfile(info)
            elif mode == "nonexec-regular" and name == "bin/opi-sandbox":
                payload = member_payload(name)
                info = tarfile.TarInfo(archive_name(name))
                info.mode = 0o644
                info.size = len(payload)
                out.addfile(info, io.BytesIO(payload))
            elif mode == "same-id-schema" and name == "schemas/command-execution-jsonl-v1.schema.json":
                payload = member_payload(name)
                info = tarfile.TarInfo(archive_name(name))
                info.size = len(payload)
                out.addfile(info, io.BytesIO(payload))
            elif mode in ("cpu-swap", "malformed-header"):
                payload = member_payload(name)
                info = tarfile.TarInfo(archive_name(name))
                info.mode = 0o755 if name == "bin/opi-sandbox" else 0o644
                info.size = len(payload)
                out.addfile(info, io.BytesIO(payload))
            else:
                out.add(root / name, arcname=archive_name(name), recursive=False)
        if mode == "extra":
            info = tarfile.TarInfo("unexpected.txt")
            payload = b"unexpected"
            info.size = len(payload)
            out.addfile(info, io.BytesIO(payload))
        if mode == "duplicate":
            out.add(root / "package.toml", arcname="package.toml", recursive=False)
if mode in ("cpu-swap", "malformed-header"):
    print(hashlib.sha256(overrides["bin/opi-sandbox"]).hexdigest())
    print(hashlib.sha256(overrides["package.toml"].replace(b"\r", b"")).hexdigest())
"#;

fn rewrite_archive_output(p: &Packed, mode: &str) -> String {
    let helper = p.artifact.join("rewrite-archive.py");
    fs::write(&helper, REWRITE_ARCHIVE_PY).unwrap();
    let archive = archive_path(&p.artifact).expect("archive produced");
    let output = python_command()
        .arg(&helper)
        .arg(&archive)
        .arg(&p.extracted)
        .arg(mode)
        .output()
        .expect("run archive rewrite fixture");
    assert!(
        output.status.success(),
        "archive rewrite failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("archive rewrite output is UTF-8")
        .trim()
        .to_string()
}

fn rewrite_archive(p: &Packed, mode: &str) {
    let _ = rewrite_archive_output(p, mode);
}

fn replace_lock_value(path: &Path, key: &str, value: &str) {
    let prefix = format!("{key} = \"");
    let lock = fs::read_to_string(path).unwrap();
    let mut replaced = false;
    let rewritten = lock
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
    fs::write(path, rewritten).unwrap();
}

/// Pack a fresh fixture into a fresh artifact dir; assert success. Returns the
/// resolved paths.
struct Packed {
    _tempdir: tempfile::TempDir,
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
    fs::write(&fixture, executable_fixture(package_target())).unwrap();
    let script = script_path();

    let output = run(pack_cmd(&script, &fixture, &artifact));
    assert!(
        output.status.success(),
        "pack failed:\n--- stderr ---\n{}\n--- stdout ---\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );

    let pkg_dir = artifact.join("package");
    let extracted = artifact.join("extracted");
    Packed {
        _tempdir: tmp,
        artifact,
        script,
        pkg_dir,
        extracted,
    }
}

#[test]
fn packed_fixture_cleans_its_temporary_tree_on_drop() {
    let temp_root = {
        let packed = pack_fresh();
        let root = packed
            .artifact
            .parent()
            .expect("artifact has temporary parent")
            .to_path_buf();
        assert!(
            root.exists(),
            "fixture root must live through the test body"
        );
        drop(packed);
        root
    };

    assert!(
        !temp_root.exists(),
        "dropping Packed must remove its temporary fixture tree"
    );
}

#[test]
fn packer_builds_valid_layout_lock_and_extraction() {
    let p = pack_fresh();
    let target = package_target();
    let fixture_bytes = executable_fixture(target);

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
        target,
        host_opi_version(),
    )
    .expect("rendered manifest round-trips through 16.4 validation");
    let canonical = &validated[0].lock;

    // Canonical LockMaterial correctness.
    assert_eq!(canonical.protocol, "command-execution-jsonl-v1");
    assert_eq!(canonical.adapter_id, "opi-sandbox");
    assert_eq!(canonical.target, target);
    assert_eq!(canonical.executable_rel_path, "bin/opi-sandbox");
    assert_eq!(canonical.package_version, host_opi_version());
    assert_eq!(
        canonical.opi_range,
        compatible_minor_range(host_opi_version())
    );
    // The packager hashed the fixture bytes (lowercase hex).
    assert_eq!(canonical.executable_sha256, sha256_hex(&fixture_bytes));
    assert_eq!(
        fs::read_to_string(p.artifact.join("target"))
            .unwrap()
            .trim(),
        target
    );

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
        fixture_bytes,
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
    let package_schema = p
        .pkg_dir
        .join("schemas")
        .join("command-execution-jsonl-v1.schema.json");
    let extracted_schema = p
        .extracted
        .join("schemas")
        .join("command-execution-jsonl-v1.schema.json");
    let package_license = p.pkg_dir.join("licenses").join("LICENSE");
    let extracted_license = p.extracted.join("licenses").join("LICENSE");
    assert!(extracted_toml.is_file(), "extracted package.toml at root");
    assert!(extracted_bin.is_file(), "extracted bin/opi-sandbox at root");
    assert!(package_schema.is_file(), "package carries the wire schema");
    assert!(
        extracted_schema.is_file(),
        "archive carries the wire schema"
    );
    assert!(
        package_license.is_file(),
        "package carries the project license"
    );
    assert!(
        extracted_license.is_file(),
        "archive carries the project license"
    );
    let schema: serde_json::Value =
        serde_json::from_slice(&fs::read(&package_schema).unwrap()).expect("schema is JSON");
    assert_eq!(
        schema["$id"],
        "https://odradek.ai/schemas/command-execution-jsonl-v1.json"
    );
    assert_eq!(
        fs::read(&package_schema).unwrap(),
        fs::read(&extracted_schema).unwrap(),
    );
    let workspace_license = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("LICENSE");
    assert_eq!(
        fs::read(&package_license).unwrap(),
        fs::read(&workspace_license).unwrap(),
    );
    assert_eq!(
        fs::read(&package_license).unwrap(),
        fs::read(&extracted_license).unwrap(),
    );
    assert_eq!(
        fs::read(&extracted_bin).unwrap(),
        fixture_bytes,
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
fn rendered_manifest_rejects_the_adjacent_minor_version() {
    let p = pack_fresh();
    let pkg_toml_path = p.pkg_dir.join("package.toml");
    let pkg_toml_bytes = fs::read(&pkg_toml_path).unwrap();
    let manifest = PackageManifest::from_toml(
        &String::from_utf8(pkg_toml_bytes.clone()).unwrap(),
        &pkg_toml_path,
    )
    .expect("rendered manifest parses");
    let range = compatible_minor_range(host_opi_version());
    let adjacent = range
        .split('<')
        .nth(1)
        .expect("range has exclusive upper bound");
    let error = validate_executable_contributions(
        &manifest,
        &pkg_toml_bytes,
        &p.pkg_dir,
        PackageSource::Global,
        package_target(),
        &format!("{adjacent}.0"),
    )
    .expect_err("the adjacent minor must remain outside the generated range");
    assert!(error.to_string().contains("unsatisfied"), "{error}");
}

#[test]
fn platform_packagers_share_strict_literal_semver_renderer() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let helper_name = "opi-sandbox-package.py";
    for wrapper in [
        root.join("scripts/package-opi-sandbox.sh"),
        root.join("scripts/package-opi-sandbox.ps1"),
    ] {
        let source = fs::read_to_string(&wrapper).unwrap();
        assert!(
            source.contains(helper_name),
            "{} must delegate SemVer parsing and literal rendering to {helper_name}",
            wrapper.display()
        );
    }
}

#[test]
fn shared_renderer_accepts_strict_semver_and_rejects_malformed_or_metacharacters() {
    let tmp = tempfile::tempdir().unwrap();
    let template = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("packaging/opi-sandbox/package.toml.template");
    let cases = [
        ("1.2.3", true),
        ("1.2.3-rc.1", true),
        ("1.2.3+build.5", true),
        ("1.02.3", false),
        ("1.2.3-rc.01", false),
        ("1.2.3-rc&1", false),
        (r"1.2.3-rc\evil", false),
    ];

    for (index, (version, valid)) in cases.into_iter().enumerate() {
        let manifest = tmp.path().join(format!("Cargo-{index}.toml"));
        let output_path = tmp.path().join(format!("package-{index}.toml"));
        fs::write(
            &manifest,
            format!("[workspace.package]\nversion = \"{version}\"\n"),
        )
        .unwrap();
        let output = python_command()
            .arg(package_helper_path())
            .arg("render")
            .arg("--workspace-manifest")
            .arg(&manifest)
            .arg("--template")
            .arg(&template)
            .arg("--target")
            .arg(LINUX_X64_TARGET)
            .arg("--sha256")
            .arg("a".repeat(64))
            .arg("--output")
            .arg(&output_path)
            .output()
            .expect("run shared renderer");
        assert_eq!(
            output.status.success(),
            valid,
            "unexpected renderer result for {version:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        if valid {
            let rendered = fs::read_to_string(&output_path).unwrap();
            assert!(rendered.contains(&format!("version = \"{version}\"")));
            assert!(rendered.contains("opi_version = \">=1.2.0-0,<1.3.0-0\""));
        } else {
            assert_eq!(
                output.status.code(),
                Some(2),
                "invalid SemVer must use the packager usage exit for {version:?}"
            );
        }
    }
}

#[test]
fn shared_renderer_rejects_non_release_target() {
    let temp = tempfile::tempdir().unwrap();
    let manifest = temp.path().join("Cargo.toml");
    let output_path = temp.path().join("package.toml");
    fs::write(&manifest, "[workspace.package]\nversion = \"1.2.3\"\n").unwrap();
    let output = python_command()
        .arg(package_helper_path())
        .arg("render")
        .arg("--workspace-manifest")
        .arg(&manifest)
        .arg("--template")
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("packaging/opi-sandbox/package.toml.template"),
        )
        .arg("--target")
        .arg("x86_64-pc-windows-msvc")
        .arg("--sha256")
        .arg("a".repeat(64))
        .arg("--output")
        .arg(output_path)
        .output()
        .expect("run shared renderer");

    assert!(!output.status.success(), "{output:#?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unsupported package target"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn executable_header_validator_accepts_all_supported_release_targets() {
    for target in [
        LINUX_X64_TARGET,
        LINUX_ARM64_TARGET,
        DARWIN_X64_TARGET,
        DARWIN_ARM64_TARGET,
    ] {
        let output = run_header_validation(&executable_fixture(target), target);
        assert!(
            output.status.success(),
            "valid {target} header was rejected: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn executable_header_validator_rejects_cpu_and_target_family_swaps() {
    for (bytes, target) in [
        (minimal_elf64(62), LINUX_ARM64_TARGET),
        (minimal_elf64(183), LINUX_X64_TARGET),
        (minimal_macho64(0x0100_0007), DARWIN_ARM64_TARGET),
        (minimal_macho64(0x0100_000c), DARWIN_X64_TARGET),
        (minimal_elf64(62), DARWIN_X64_TARGET),
        (minimal_macho64(0x0100_0007), LINUX_X64_TARGET),
    ] {
        let output = run_header_validation(&bytes, target);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "mismatch for {target} passed");
        assert!(
            stderr.contains("executable architecture/target mismatch"),
            "mismatch for {target} had the wrong diagnostic: {stderr}"
        );
    }
}

#[test]
fn executable_header_validator_rejects_malformed_and_unsupported_formats() {
    let mut truncated_elf = minimal_elf64(62);
    truncated_elf.truncate(20);
    let mut elf32 = minimal_elf64(62);
    elf32[4] = 1;
    let mut big_endian_elf = minimal_elf64(62);
    big_endian_elf[5] = 2;
    let unknown_elf_machine = minimal_elf64(3);
    let mut invalid_elf_ident_version = minimal_elf64(62);
    invalid_elf_ident_version[6] = 0;
    let mut invalid_elf_type = minimal_elf64(62);
    invalid_elf_type[16..18].copy_from_slice(&0_u16.to_le_bytes());
    let mut invalid_elf_version = minimal_elf64(62);
    invalid_elf_version[20..24].copy_from_slice(&0_u32.to_le_bytes());
    let mut invalid_elf_header_size = minimal_elf64(62);
    invalid_elf_header_size[52..54].copy_from_slice(&0_u16.to_le_bytes());
    let mut truncated_macho = minimal_macho64(0x0100_0007);
    truncated_macho.truncate(8);
    let unknown_macho_cpu = minimal_macho64(9);
    let mut invalid_macho_filetype = minimal_macho64(0x0100_0007);
    invalid_macho_filetype[12..16].copy_from_slice(&1_u32.to_le_bytes());
    let cases = [
        (b"not an executable\n".to_vec(), LINUX_X64_TARGET),
        (b"MZ\0\0portable executable".to_vec(), LINUX_X64_TARGET),
        (truncated_elf, LINUX_X64_TARGET),
        (elf32, LINUX_X64_TARGET),
        (big_endian_elf, LINUX_X64_TARGET),
        (unknown_elf_machine, LINUX_X64_TARGET),
        (invalid_elf_ident_version, LINUX_X64_TARGET),
        (invalid_elf_type, LINUX_X64_TARGET),
        (invalid_elf_version, LINUX_X64_TARGET),
        (invalid_elf_header_size, LINUX_X64_TARGET),
        (truncated_macho, DARWIN_X64_TARGET),
        (vec![0xce, 0xfa, 0xed, 0xfe], DARWIN_X64_TARGET),
        (vec![0xca, 0xfe, 0xba, 0xbe], DARWIN_X64_TARGET),
        (vec![0xfe, 0xed, 0xfa, 0xcf], DARWIN_X64_TARGET),
        (unknown_macho_cpu, DARWIN_X64_TARGET),
        (invalid_macho_filetype, DARWIN_X64_TARGET),
    ];

    for (bytes, target) in cases {
        let output = run_header_validation(&bytes, target);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "malformed header for {target} passed"
        );
        assert!(
            stderr.contains("invalid executable format"),
            "malformed header for {target} had the wrong diagnostic: {stderr}"
        );
    }
}

#[test]
fn executable_header_validator_rejects_non_release_target() {
    let output = run_header_validation(&minimal_elf64(62), "x86_64-pc-windows-msvc");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        stderr.contains("executable architecture/target mismatch"),
        "{stderr}"
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

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn packer_accepts_real_native_test_executable() {
    let tmp = tempfile::tempdir().expect("native package tempdir");
    let artifact = tmp.path().join("artifact");
    let binary = std::env::current_exe().expect("current test executable");
    let script = script_path();

    let pack = run(pack_cmd(&script, &binary, &artifact));
    assert!(
        pack.status.success(),
        "native pack failed: {}",
        String::from_utf8_lossy(&pack.stderr)
    );
    let verify = run(verify_cmd(&script, &artifact));
    assert!(
        verify.status.success(),
        "native verify failed: {}",
        String::from_utf8_lossy(&verify.stderr)
    );
}

#[test]
fn verify_ignores_tampered_caller_owned_staging_trees() {
    let p = pack_fresh();
    let pkg_toml = p.pkg_dir.join("package.toml");
    fs::OpenOptions::new()
        .append(true)
        .open(&pkg_toml)
        .unwrap()
        .write_all(b"# tampered\n")
        .unwrap();
    let extracted_bin = p.extracted.join("bin").join("opi-sandbox");
    fs::OpenOptions::new()
        .append(true)
        .open(&extracted_bin)
        .unwrap()
        .write_all(b"tampered\n")
        .unwrap();
    let output = run(verify_cmd(&p.script, &p.artifact));
    assert!(
        output.status.success(),
        "verify must authenticate the archive, not caller-owned staging trees:\n{:#?}",
        output
    );
}

#[test]
fn verify_rejects_tampered_archive_when_staging_trees_are_unchanged() {
    let p = pack_fresh();
    let archive = archive_path(&p.artifact).unwrap();
    fs::write(archive, b"tampered archive bytes").unwrap();
    let output = run(verify_cmd(&p.script, &p.artifact));
    assert!(
        !output.status.success(),
        "verify must reject archive tampering with untouched staging trees:\n{:#?}",
        output
    );
}

#[test]
fn verify_rejects_cpu_swapped_archive_when_hashes_are_consistent() {
    let p = pack_fresh();
    let material = rewrite_archive_output(&p, "cpu-swap");
    let mut lines = material.lines();
    let executable_sha = lines.next().expect("rewritten executable SHA");
    let manifest_hash = lines.next().expect("rewritten manifest hash");
    assert!(
        lines.next().is_none(),
        "unexpected rewrite output: {material}"
    );
    let lock = p.artifact.join("package-lock.toml");
    replace_lock_value(&lock, "executable_sha256", executable_sha);
    replace_lock_value(&lock, "manifest_hash", manifest_hash);

    let output = run(verify_cmd(&p.script, &p.artifact));

    assert!(!output.status.success(), "{output:#?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("executable architecture/target mismatch"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn verify_rejects_structurally_invalid_header_when_hashes_are_consistent() {
    let packed = pack_fresh();
    let material = rewrite_archive_output(&packed, "malformed-header");
    let mut lines = material.lines();
    let executable_sha = lines.next().expect("rewritten executable SHA");
    let manifest_hash = lines.next().expect("rewritten manifest hash");
    assert!(
        lines.next().is_none(),
        "unexpected rewrite output: {material}"
    );
    let lock = packed.artifact.join("package-lock.toml");
    replace_lock_value(&lock, "executable_sha256", executable_sha);
    replace_lock_value(&lock, "manifest_hash", manifest_hash);

    let output = run(verify_cmd(&packed.script, &packed.artifact));

    assert!(!output.status.success(), "{output:#?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("invalid executable format"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn verify_rejects_archive_with_missing_member() {
    let p = pack_fresh();
    rewrite_archive(&p, "missing");
    assert!(!run(verify_cmd(&p.script, &p.artifact)).status.success());
}

#[test]
fn verify_rejects_archive_with_extra_member() {
    let p = pack_fresh();
    rewrite_archive(&p, "extra");
    assert!(!run(verify_cmd(&p.script, &p.artifact)).status.success());
}

#[test]
fn verify_rejects_archive_with_duplicate_member() {
    let p = pack_fresh();
    rewrite_archive(&p, "duplicate");
    assert!(!run(verify_cmd(&p.script, &p.artifact)).status.success());
}

#[test]
fn verify_rejects_archive_with_non_regular_executable() {
    let p = pack_fresh();
    rewrite_archive(&p, "wrong-kind");
    assert!(!run(verify_cmd(&p.script, &p.artifact)).status.success());
}

#[cfg(unix)]
#[test]
fn verify_rejects_byte_identical_non_executable_tar_member_with_untouched_staging_trees() {
    let p = pack_fresh();
    let package_binary = fs::read(p.pkg_dir.join("bin/opi-sandbox")).unwrap();
    let extracted_binary = fs::read(p.extracted.join("bin/opi-sandbox")).unwrap();

    rewrite_archive(&p, "nonexec-regular");

    assert_eq!(
        fs::read(p.pkg_dir.join("bin/opi-sandbox")).unwrap(),
        package_binary,
        "archive rewrite must not alter the package staging bytes"
    );
    assert_eq!(
        fs::read(p.extracted.join("bin/opi-sandbox")).unwrap(),
        extracted_binary,
        "archive rewrite must not alter the extracted staging bytes"
    );
    let output = run(verify_cmd(&p.script, &p.artifact));
    assert!(!output.status.success(), "{output:#?}");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("archive executable mode must be exactly 0755"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn verify_rejects_archive_with_dot_path_alias() {
    let p = pack_fresh();
    rewrite_archive(&p, "dot-alias");
    let output = run(verify_cmd(&p.script, &p.artifact));
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("archive member name is not canonical"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn verify_rejects_archive_with_double_slash_path_alias() {
    let p = pack_fresh();
    rewrite_archive(&p, "double-slash-alias");
    let output = run(verify_cmd(&p.script, &p.artifact));
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("archive member name is not canonical"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn verify_rejects_same_id_schema_tamper_with_untouched_staging_trees() {
    let p = pack_fresh();
    let package_schema = fs::read(
        p.pkg_dir
            .join("schemas/command-execution-jsonl-v1.schema.json"),
    )
    .unwrap();
    let extracted_schema = fs::read(
        p.extracted
            .join("schemas/command-execution-jsonl-v1.schema.json"),
    )
    .unwrap();
    rewrite_archive(&p, "same-id-schema");
    assert_eq!(
        fs::read(
            p.pkg_dir
                .join("schemas/command-execution-jsonl-v1.schema.json")
        )
        .unwrap(),
        package_schema
    );
    assert_eq!(
        fs::read(
            p.extracted
                .join("schemas/command-execution-jsonl-v1.schema.json")
        )
        .unwrap(),
        extracted_schema
    );
    let output = run(verify_cmd(&p.script, &p.artifact));
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("schema does not match the reviewed snapshot"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn verify_rejects_archive_and_lock_with_noncanonical_opi_range() {
    let p = pack_fresh();
    let manifest_path = p.extracted.join("package.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    let canonical = compatible_minor_range(host_opi_version());
    let noncanonical = format!(
        ">={}.{},<{}.{}",
        host_opi_version().split('.').next().unwrap(),
        host_opi_version().split('.').nth(1).unwrap(),
        host_opi_version().split('.').next().unwrap(),
        host_opi_version()
            .split('.')
            .nth(1)
            .unwrap()
            .parse::<u64>()
            .unwrap()
            + 1
    );
    let changed_manifest = manifest.replace(&canonical, &noncanonical);
    assert_ne!(changed_manifest, manifest);
    fs::write(&manifest_path, &changed_manifest).unwrap();
    rewrite_archive(&p, "normal");

    let lock_path = p.artifact.join("package-lock.toml");
    let lock = fs::read_to_string(&lock_path).unwrap();
    let changed_lock = lock.replace(&canonical, &noncanonical).replace(
        &format!(
            "manifest_hash = \"{}\"",
            sha256_hex(&lf_strip(manifest.as_bytes()))
        ),
        &format!(
            "manifest_hash = \"{}\"",
            sha256_hex(&lf_strip(changed_manifest.as_bytes()))
        ),
    );
    fs::write(&lock_path, changed_lock).unwrap();

    assert!(!run(verify_cmd(&p.script, &p.artifact)).status.success());
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
fn pack_rejects_arbitrary_text_binary() {
    let tmp = tempfile::tempdir().unwrap();
    let artifact = tmp.path().join("artifact");
    let fixture = tmp.path().join("text-binary");
    fs::write(&fixture, b"not an executable\n").unwrap();
    let script = script_path();

    let output = run(pack_cmd(&script, &fixture, &artifact));

    assert!(!output.status.success(), "{output:#?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("invalid executable format"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pack_rejects_structurally_invalid_executable_header() {
    let temp = tempfile::tempdir().unwrap();
    let artifact = temp.path().join("artifact");
    let binary = temp.path().join("invalid-header");
    let target = package_target();
    let mut bytes = executable_fixture(target);
    if target.ends_with("-unknown-linux-gnu") {
        bytes[16..18].copy_from_slice(&0_u16.to_le_bytes());
    } else {
        bytes[12..16].copy_from_slice(&1_u32.to_le_bytes());
    }
    fs::write(&binary, bytes).unwrap();

    let output = run(pack_cmd(&script_path(), &binary, &artifact));

    assert!(!output.status.success(), "{output:#?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("invalid executable format"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
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
