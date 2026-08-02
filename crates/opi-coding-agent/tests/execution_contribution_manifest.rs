//! Contract tests for task 16.4: parsing and hard-gating executable
//! `command.execute` adapter contributions.
//!
//! One test per static gate (each rejection maps to an exact
//! [`ContributionValidationError`] variant), plus the green-path lock-material
//! exactness anchor and manifest-hash CRLF stability.

use std::path::{Path, PathBuf};

use opi_coding_agent::execution::{
    ContributionValidationError, PackageSource, validate_executable_contributions,
};
use opi_coding_agent::package_discovery::PackageManifest;

const HOST_TARGET: &str = "x86_64-unknown-linux-gnu";
const HOST_OPI_VERSION: &str = "0.7.2";
const EXE_CONTENT: &[u8] = b"#!/bin/sh\necho hi\n";

// --- test helpers ------------------------------------------------------------

fn t_sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}

fn t_lf_normalize(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'\r' && bytes[i + 1] == b'\n' {
            out.push(b'\n');
            i += 2;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

fn make_executable(path: &Path) {
    let _ = path;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// Build a package dir with a real executable at `bin/opi-sandbox`; return the
/// dir (kept alive by the caller), its root, and the executable's SHA-256.
fn make_package() -> (tempfile::TempDir, PathBuf, String) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("bin")).unwrap();
    let exe = dir.path().join("bin").join("opi-sandbox");
    std::fs::write(&exe, EXE_CONTENT).unwrap();
    make_executable(&exe);
    let sha = t_sha256(EXE_CONTENT);
    let root = dir.path().to_path_buf();
    (dir, root, sha)
}

#[allow(clippy::too_many_arguments)]
fn manifest_toml(
    capability: &str,
    id: &str,
    transport: &str,
    command: &str,
    protocol: &str,
    target: &str,
    sha256: &str,
    handshake_timeout_ms: u64,
    adapter_config: &str,
    opi_version: Option<&str>,
    version: Option<&str>,
) -> String {
    let opi_version_line = opi_version
        .map(|v| format!("opi_version = \"{v}\"\n"))
        .unwrap_or_default();
    let version_line = version
        .map(|v| format!("version = \"{v}\"\n"))
        .unwrap_or_default();
    format!(
        "{version_line}{opi_version_line}\
name = \"opi-sandbox\"\n\
description = \"official host-native command restriction backend\"\n\
\n\
[[contributions.adapters]]\n\
capability = \"{capability}\"\n\
id = \"{id}\"\n\
transport = \"{transport}\"\n\
command = \"{command}\"\n\
args = [\"backend\", \"--stdio\"]\n\
protocol = \"{protocol}\"\n\
target = \"{target}\"\n\
sha256 = \"{sha256}\"\n\
handshake_timeout_ms = {handshake_timeout_ms}\n\
adapter_config = {adapter_config}\n"
    )
}

fn default_manifest_toml(sha: &str) -> String {
    manifest_toml(
        "command.execute",
        "opi-sandbox",
        "process-jsonl",
        "bin/opi-sandbox",
        "command-execution-jsonl-v1",
        HOST_TARGET,
        sha,
        5000,
        "{}",
        Some(">=0.7,<0.8"),
        Some("0.8.0"),
    )
}

fn parse(toml: &str) -> PackageManifest {
    PackageManifest::from_toml(toml, Path::new("package.toml")).unwrap()
}

fn validate(
    manifest: &PackageManifest,
    toml: &str,
    root: &Path,
    source: PackageSource,
) -> Result<
    Vec<opi_coding_agent::execution::ValidatedExecutableContribution>,
    ContributionValidationError,
> {
    validate_executable_contributions(
        manifest,
        toml.as_bytes(),
        root,
        source,
        HOST_TARGET,
        HOST_OPI_VERSION,
    )
}

// --- green path: lock material exactness -------------------------------------

#[test]
fn valid_contribution_yields_exact_lock_material() {
    let (_dir, root, sha) = make_package();
    let toml = default_manifest_toml(&sha);
    let manifest = parse(&toml);
    let validated = validate(&manifest, &toml, &root, PackageSource::Global).unwrap();
    assert_eq!(validated.len(), 1);
    let v = &validated[0];
    assert_eq!(v.capability, "command.execute");
    assert_eq!(v.id, "opi-sandbox");
    assert_eq!(v.transport, "process-jsonl");
    assert_eq!(v.protocol, "command-execution-jsonl-v1");
    assert_eq!(v.target, HOST_TARGET);
    assert_eq!(v.handshake_timeout_ms, 5000);
    assert_eq!(v.args, vec!["backend", "--stdio"]);
    // Command resolved to a canonical absolute path contained by the root.
    assert!(v.command.is_absolute());
    assert!(v.command.starts_with(root.canonicalize().unwrap()));

    let lock = &v.lock;
    assert_eq!(
        lock.manifest_hash,
        t_sha256(&t_lf_normalize(toml.as_bytes()))
    );
    assert_eq!(lock.executable_rel_path, "bin/opi-sandbox");
    assert_eq!(lock.executable_sha256, sha);
    assert_eq!(lock.package_version, "0.8.0");
    assert_eq!(lock.target, HOST_TARGET);
    assert_eq!(lock.opi_range, ">=0.7,<0.8");
    assert_eq!(lock.protocol, "command-execution-jsonl-v1");
    assert_eq!(lock.adapter_id, "opi-sandbox");
}

#[test]
fn empty_contributions_validate_to_empty() {
    let (_dir, root, _sha) = make_package();
    let toml = "name = \"opi-sandbox\"\ndescription = \"x\"\n";
    let manifest = parse(toml);
    let validated = validate(&manifest, toml, &root, PackageSource::Global).unwrap();
    assert!(validated.is_empty());
}

#[test]
fn manifest_hash_is_crlf_stable() {
    let (_dir, root, sha) = make_package();
    let lf = default_manifest_toml(&sha);
    let crlf = lf.replace('\n', "\r\n");
    let manifest_lf = parse(&lf);
    let manifest_crlf = parse(&crlf);
    let lf_hash = validate(&manifest_lf, &lf, &root, PackageSource::Global).unwrap()[0]
        .lock
        .manifest_hash
        .clone();
    let crlf_hash = validate(&manifest_crlf, &crlf, &root, PackageSource::Global).unwrap()[0]
        .lock
        .manifest_hash
        .clone();
    assert_eq!(lf_hash, crlf_hash);
}

// --- closed field set --------------------------------------------------------

#[test]
fn unknown_contribution_field_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = default_manifest_toml(&sha) + "bogus = true\n";
    let manifest = parse(&toml);
    let err = validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err();
    assert!(matches!(
        err,
        ContributionValidationError::UnknownContributionField { field } if field == "bogus"
    ));
}

// --- identity gates ----------------------------------------------------------

#[test]
fn wrong_capability_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = manifest_one_capability(&sha, "tool.run");
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::CapabilityNotCommandExecute(_)
    ));
}

#[test]
fn wrong_transport_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = manifest_one(&sha, |m| m.replacen("process-jsonl", "stdio-raw", 1));
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::TransportNotProcessJsonl(_)
    ));
}

#[test]
fn wrong_protocol_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = manifest_one(&sha, |m| {
        m.replacen("command-execution-jsonl-v1", "opi-extension-jsonl-v1", 1)
    });
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::ProtocolIncompatible { .. }
    ));
}

#[test]
fn invalid_adapter_id_charset_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = manifest_one(&sha, |m| {
        m.replacen("id = \"opi-sandbox\"", "id = \"Bad_Id\"", 1)
    });
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::AdapterIdInvalid { .. }
    ));
}

#[test]
fn reserved_adapter_id_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = manifest_one(&sha, |m| {
        m.replacen("id = \"opi-sandbox\"", "id = \"local\"", 1)
    });
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::ReservedAdapterId(_)
    ));
}

#[test]
fn duplicate_adapter_id_within_manifest_rejected() {
    let (_dir, root, sha) = make_package();
    // Two identical contributions share the same id.
    let full = default_manifest_toml(&sha);
    let block = full.split_once("[[contributions.adapters]]").unwrap().1;
    let toml = format!(
        "name = \"opi-sandbox\"\ndescription = \"x\"\nversion = \"0.8.0\"\nopi_version = \">=0.7,<0.8\"\n\n[[contributions.adapters]]{block}[[contributions.adapters]]{block}"
    );
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::DuplicateContributionId { id } if id == "opi-sandbox"
    ));
}

// --- target gate -------------------------------------------------------------

#[test]
fn target_mismatch_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = manifest_one(&sha, |m| {
        m.replacen(HOST_TARGET, "aarch64-unknown-linux-gnu", 1)
    });
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::IncompatibleTarget { .. }
    ));
}

#[test]
fn empty_manifest_target_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = manifest_toml(
        "command.execute",
        "opi-sandbox",
        "process-jsonl",
        "bin/opi-sandbox",
        "command-execution-jsonl-v1",
        "",
        &sha,
        5000,
        "{}",
        Some(">=0.7,<0.8"),
        Some("0.8.0"),
    );
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::EmptyTarget
    ));
}

#[test]
fn empty_host_target_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = default_manifest_toml(&sha);
    let manifest = parse(&toml);
    let err = validate_executable_contributions(
        &manifest,
        toml.as_bytes(),
        &root,
        PackageSource::Global,
        "",
        HOST_OPI_VERSION,
    )
    .unwrap_err();
    assert!(matches!(err, ContributionValidationError::EmptyTarget));
}

// --- handshake timeout gate --------------------------------------------------

#[test]
fn handshake_timeout_zero_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = manifest_one(&sha, |m| {
        m.replacen("handshake_timeout_ms = 5000", "handshake_timeout_ms = 0", 1)
    });
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::OutOfRangeHandshakeTimeout { value: 0, .. }
    ));
}

#[test]
fn handshake_timeout_over_cap_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = manifest_one(&sha, |m| {
        m.replacen(
            "handshake_timeout_ms = 5000",
            "handshake_timeout_ms = 99000000",
            1,
        )
    });
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::OutOfRangeHandshakeTimeout {
            value: 99_000_000,
            ..
        }
    ));
}

// --- adapter configuration gate ----------------------------------------------

#[test]
fn oversized_adapter_config_rejected() {
    let (_dir, root, sha) = make_package();
    // > 256 KiB serialized JSON.
    let big = "A".repeat(280_000);
    let cfg = format!("{{ big = \"{big}\" }}");
    let toml = manifest_toml(
        "command.execute",
        "opi-sandbox",
        "process-jsonl",
        "bin/opi-sandbox",
        "command-execution-jsonl-v1",
        HOST_TARGET,
        &sha,
        5000,
        &cfg,
        Some(">=0.7,<0.8"),
        Some("0.8.0"),
    );
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::AdapterConfigTooLarge { .. }
    ));
}

// --- command path gates ------------------------------------------------------

#[test]
fn absolute_command_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = manifest_one(&sha, |m| m.replacen("bin/opi-sandbox", "/usr/bin/evil", 1));
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::AbsoluteCommand
    ));
}

#[test]
fn bare_path_command_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = manifest_one(&sha, |m| m.replacen("bin/opi-sandbox", "evil", 1));
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::BarePathCommand
    ));
}

#[test]
fn traversal_command_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = manifest_one(&sha, |m| {
        m.replacen("bin/opi-sandbox", "bin/../bin/evil", 1)
    });
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::TraversalCommand
    ));
}

#[test]
fn nonexistent_command_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = manifest_one(&sha, |m| m.replacen("bin/opi-sandbox", "bin/missing", 1));
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::CommandNotCanonicalizable(_)
    ));
}

#[cfg(unix)]
#[test]
fn symlink_escape_command_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    std::fs::create_dir_all(root.join("bin")).unwrap();
    // A file OUTSIDE the package root.
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("secret");
    std::fs::write(&outside_file, EXE_CONTENT).unwrap();
    make_executable(&outside_file);
    std::os::unix::fs::symlink(&outside_file, root.join("bin").join("escape")).unwrap();
    let toml = manifest_one("deadbeef", |m| {
        m.replacen("bin/opi-sandbox", "bin/escape", 1)
    });
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::SymlinkEscape
    ));
}

#[cfg(unix)]
#[test]
fn non_regular_executable_rejected() {
    use std::os::unix::net::UnixListener;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    std::fs::create_dir_all(root.join("bin")).unwrap();
    // A named pipe at the command path (regularity gate fires before read).
    let fifo = root.join("bin").join("pipe");
    // Use a Unix domain socket as a portable non-regular special file.
    UnixListener::bind(&fifo).unwrap();
    let toml = manifest_one("deadbeef", |m| m.replacen("bin/opi-sandbox", "bin/pipe", 1));
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::NonRegularExecutable
    ));
}

#[cfg(unix)]
#[test]
fn non_executable_file_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    std::fs::create_dir_all(root.join("bin")).unwrap();
    let exe = root.join("bin").join("noexec");
    std::fs::write(&exe, EXE_CONTENT).unwrap();
    // Deliberately no execute bit (mode 0o644).
    let toml = manifest_one(&t_sha256(EXE_CONTENT), |m| {
        m.replacen("bin/opi-sandbox", "bin/noexec", 1)
    });
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::NonExecutableFile
    ));
}

// --- SHA-256 gate ------------------------------------------------------------

#[test]
fn malformed_sha256_uppercase_rejected() {
    let (_dir, root, sha) = make_package();
    let upper = sha.to_uppercase();
    let toml = manifest_one(&sha, |m| m.replacen(&sha, &upper, 1));
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::MalformedSha256(_)
    ));
}

#[test]
fn malformed_sha256_wrong_length_rejected() {
    let (_dir, root, sha) = make_package();
    let short = "abcdef".to_string();
    let toml = manifest_one(&sha, |m| m.replacen(&sha, &short, 1));
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::MalformedSha256(_)
    ));
}

#[test]
fn sha256_mismatch_rejected() {
    let (_dir, root, sha) = make_package();
    // Well-formed 64-char lowercase hex that is not the file's digest.
    let wrong = format!("{:0<64}", 'a');
    let toml = manifest_one(&sha, |m| m.replacen(&sha, &wrong, 1));
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::Sha256Mismatch { .. }
    ));
}

// --- compatibility version + opi-range gates ---------------------------------

#[test]
fn missing_package_version_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = manifest_toml(
        "command.execute",
        "opi-sandbox",
        "process-jsonl",
        "bin/opi-sandbox",
        "command-execution-jsonl-v1",
        HOST_TARGET,
        &sha,
        5000,
        "{}",
        Some(">=0.7,<0.8"),
        None,
    );
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::MissingCompatibilityVersion
    ));
}

#[test]
fn missing_opi_version_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = manifest_toml(
        "command.execute",
        "opi-sandbox",
        "process-jsonl",
        "bin/opi-sandbox",
        "command-execution-jsonl-v1",
        HOST_TARGET,
        &sha,
        5000,
        "{}",
        None,
        Some("0.8.0"),
    );
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::MissingCompatibilityVersion
    ));
}

#[test]
fn incompatible_opi_range_rejected() {
    let (_dir, root, sha) = make_package();
    // Host is 0.7.2; require >=0.8.
    let toml = manifest_one(&sha, |m| m.replacen(">=0.7,<0.8", ">=0.8,<0.9", 1));
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::IncompatibleOpiRange { .. }
    ));
}

#[test]
fn unparseable_opi_range_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = manifest_one(&sha, |m| m.replacen(">=0.7,<0.8", "not-a-version", 1));
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::Global).unwrap_err(),
        ContributionValidationError::IncompatibleOpiRange { .. }
    ));
}

// --- project-local gate ------------------------------------------------------

#[test]
fn project_local_executable_contribution_rejected() {
    let (_dir, root, sha) = make_package();
    let toml = default_manifest_toml(&sha);
    let manifest = parse(&toml);
    assert!(matches!(
        validate(&manifest, &toml, &root, PackageSource::ProjectLocal).unwrap_err(),
        ContributionValidationError::ProjectLocalExecutableContribution
    ));
}

// --- helpers building one-defect manifests -----------------------------------

fn manifest_one(sha: &str, tweak: impl FnOnce(&str) -> String) -> String {
    tweak(&default_manifest_toml(sha))
}

fn manifest_one_capability(sha: &str, capability: &str) -> String {
    default_manifest_toml(sha).replace("command.execute", capability)
}
