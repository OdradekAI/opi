//! Static validation of executable `command.execute` adapter contributions.
//!
//! Parses and hard-gates the `[[contributions.adapters]]` section of a global
//! `package.toml`. This is a **pure, static seam**: it reads the manifest and
//! the referenced executable file's bytes (for an exact SHA-256) and produces a
//! [`ValidatedExecutableContribution`] plus exact [`LockMaterial`]. It never
//! spawns a process, never touches the network, and never executes package
//! code. Trust persistence, enablement, and pre-spawn revalidation are owned by
//! the package store (Phase 16.5); this module only computes the validated
//! lock material.
//!
//! # Gates (each maps to a distinct [`ContributionValidationError`] variant)
//!
//! - **project-local**: a project-local package may not carry executable adapter
//!   contributions (execution-adapter packages are global-only).
//! - **compatibility versions**: a contribution-bearing manifest must declare
//!   `version` and `opi_version`; the `opi_version` range must be satisfied by
//!   the running Opi (a HARD gate here, unlike the advisory path for
//!   non-execution resources).
//! - **closed field set**: only the named contribution fields are accepted;
//!   unknown keys fail closed.
//! - **identity**: `capability == "command.execute"`, `transport ==
//!   "process-jsonl"`, `protocol == command-execution-jsonl-v1` (the
//!   [`opi_protocol::execution::v1`] wire identity), unique adapter id within
//!   the manifest, valid id charset, and not a reserved id (`local`).
//! - **command path**: a relative path with separators, contained by the
//!   canonical package root. Absolute, drive-relative, bare-PATH (no
//!   separator), traversal (`..`/`.`), and symlink-escape paths are rejected.
//! - **executable**: a regular file, executable on Unix (any mode bit `0o111`);
//!   on Windows executability is deferred to spawn. Regularity is confirmed via
//!   metadata before any byte read, so a FIFO or device under the root is
//!   rejected without blocking.
//! - **SHA-256**: the declared digest must be exactly 64 lowercase hex chars
//!   (uppercase or wrong length is malformed, not a mismatch) and must equal
//!   the SHA-256 computed over the resolved regular executable file's bytes.
//!   The lock stores the computed digest.
//! - **target**: full-string byte equality with the host target triple; empty
//!   targets on either side are rejected.
//! - **handshake timeout**: `handshake_timeout_ms` in `1..=60_000`.
//! - **adapter configuration**: serialized JSON byte size within the protocol's
//!   `max_configuration_size` bound.
//!
//! Cross-package adapter-id collision is a store-level gate owned by 16.5.

use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::Read as _;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use crate::package_discovery::{OpiVersionDiagnostic, PackageManifest};

/// Inclusive upper bound on a contribution's `handshake_timeout_ms`.
const MAX_HANDSHAKE_TIMEOUT_MS: u64 = 60_000;

/// Adapter ids reserved for built-in backends; a package may not claim them.
const RESERVED_ADAPTER_IDS: &[&str] = &["local"];

/// Every field accepted on a `[[contributions.adapters]]` table. Any other key
/// is rejected as [`ContributionValidationError::UnknownContributionField`].
const KNOWN_ADAPTER_FIELDS: &[&str] = &[
    "capability",
    "id",
    "transport",
    "command",
    "args",
    "protocol",
    "target",
    "sha256",
    "handshake_timeout_ms",
    "adapter_config",
];

/// Where a package was discovered from. Project-local packages may not declare
/// executable adapter contributions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageSource {
    /// A globally-installed package (the only source that may carry executable
    /// adapter contributions).
    Global,
    /// A project-local package.
    ProjectLocal,
}

/// Exact material recorded in the package lock for one validated contribution.
///
/// The stored hash detects drift but does not authenticate the publisher.
///
/// `Serialize`/`Deserialize` are derived so the lock material can be persisted
/// in `package-lock.toml` (Phase 16.5), per the design's lock contract.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LockMaterial {
    /// SHA-256 over the LF-normalized `package.toml` bytes that were parsed.
    pub manifest_hash: String,
    /// The contribution `command`, as a relative path contained by the root.
    pub executable_rel_path: String,
    /// SHA-256 computed over the resolved regular executable file's bytes.
    pub executable_sha256: String,
    /// The package `version`.
    pub package_version: String,
    /// The contribution `target` triple.
    pub target: String,
    /// The package `opi_version` compatibility range.
    pub opi_range: String,
    /// The wire protocol identity (`command-execution-jsonl-v1`).
    pub protocol: String,
    /// The contribution adapter `id`.
    pub adapter_id: String,
}

/// A `command.execute` contribution that passed every static gate.
#[derive(Debug, Clone)]
pub struct ValidatedExecutableContribution {
    pub capability: String,
    pub id: String,
    pub transport: String,
    /// Resolved canonical absolute path of the executable (contained by root).
    pub command: PathBuf,
    /// Immutable launch material. Unix owns a private copied descriptor (sealed
    /// on Linux); Windows owns the validated no-write/no-delete-sharing handle.
    pub executable: Arc<File>,
    pub args: Vec<String>,
    pub protocol: String,
    pub target: String,
    pub handshake_timeout_ms: u64,
    pub adapter_config: serde_json::Value,
    /// Exact lock material for this contribution.
    pub lock: LockMaterial,
}

impl ValidatedExecutableContribution {
    /// Path passed to the process launcher for the already-open executable.
    pub fn bound_launch_path(&self) -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            use std::os::fd::AsRawFd as _;
            return PathBuf::from(format!("/proc/self/fd/{}", self.executable.as_raw_fd()));
        }
        #[cfg(target_os = "macos")]
        {
            use std::os::fd::AsRawFd as _;
            return PathBuf::from(format!("/dev/fd/{}", self.executable.as_raw_fd()));
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            self.command.clone()
        }
    }
}

/// A static contribution-validation failure. One distinct variant per gate so
/// remediation and the runtime failure-code mapping are unambiguous.
#[derive(Debug, thiserror::Error)]
pub enum ContributionValidationError {
    #[error("project-local packages may not declare executable adapter contributions")]
    ProjectLocalExecutableContribution,
    #[error("contribution-bearing manifest is missing required package/opi compatibility version")]
    MissingCompatibilityVersion,
    #[error("opi version range {range:?} is unparseable or unsatisfied by host {host:?}")]
    IncompatibleOpiRange { range: String, host: String },
    #[error("duplicate adapter id {id:?} within one manifest")]
    DuplicateContributionId { id: String },
    #[error("malformed contribution: {reason}")]
    Malformed { reason: String },
    #[error("unknown contribution field {field:?}")]
    UnknownContributionField { field: String },
    #[error("adapter capability must be \"command.execute\", got {0:?}")]
    CapabilityNotCommandExecute(String),
    #[error("transport must be \"process-jsonl\", got {0:?}")]
    TransportNotProcessJsonl(String),
    #[error("protocol must be {wanted:?}, got {got:?}")]
    ProtocolIncompatible { wanted: &'static str, got: String },
    #[error("adapter id invalid: {reason}")]
    AdapterIdInvalid { reason: String },
    #[error("adapter id {0:?} is reserved")]
    ReservedAdapterId(String),
    #[error("command must be a relative path contained by the package root (absolute rejected)")]
    AbsoluteCommand,
    #[error(
        "command must be a relative path contained by the package root (drive-relative rejected)"
    )]
    DriveRelativeCommand,
    #[error("command must be a relative path with separators (bare PATH lookup rejected)")]
    BarePathCommand,
    #[error("command path contains a parent/current-dir component (traversal rejected)")]
    TraversalCommand,
    #[error("command path escapes the canonical package root (symlink escape)")]
    SymlinkEscape,
    #[error("command path could not be canonicalized: {0}")]
    CommandNotCanonicalizable(String),
    #[error("executable is not a regular file")]
    NonRegularExecutable,
    #[error("executable file is not executable")]
    NonExecutableFile,
    #[error("declared sha256 is malformed (need 64 lowercase hex chars): {0}")]
    MalformedSha256(String),
    #[error("declared sha256 {declared} does not match computed {computed}")]
    Sha256Mismatch { declared: String, computed: String },
    #[error("manifest target {got:?} does not exactly match host target {wanted:?}")]
    IncompatibleTarget { wanted: String, got: String },
    #[error("target is empty")]
    EmptyTarget,
    #[error("handshake_timeout_ms {value} is out of range [1, {max}]")]
    OutOfRangeHandshakeTimeout { value: u64, max: u64 },
    #[error("adapter configuration serialized size {actual} exceeds limit {limit}")]
    AdapterConfigTooLarge { actual: usize, limit: usize },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Raw (typed but unvalidated) `[[contributions.adapters]]` entry, deserialized
/// from a raw TOML table after the closed-field check.
#[derive(Debug, Clone, serde::Deserialize)]
struct RawAdapterContribution {
    capability: String,
    id: String,
    transport: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
    protocol: String,
    target: String,
    sha256: String,
    handshake_timeout_ms: u64,
    adapter_config: serde_json::Value,
}

/// Validate every executable `command.execute` contribution on `manifest`.
///
/// `raw_manifest_bytes` must be the exact bytes parsed into `manifest` (the
/// manifest hash is computed over their LF-normalized form); the validator
/// never re-reads the manifest file. `package_root` is canonicalized on both
/// sides for the containment proof. `host_target` and `host_opi_version` are
/// pure inputs (the caller decides how the host learns them), keeping this
/// function unit-testable and free of process/env introspection.
///
/// Returns one [`ValidatedExecutableContribution`] per contribution (with its
/// [`LockMaterial`]), or the first gate failure.
pub fn validate_executable_contributions(
    manifest: &PackageManifest,
    raw_manifest_bytes: &[u8],
    package_root: &Path,
    package_source: PackageSource,
    host_target: &str,
    host_opi_version: &str,
) -> Result<Vec<ValidatedExecutableContribution>, ContributionValidationError> {
    if manifest.adapter_contributions.is_empty() {
        return Ok(Vec::new());
    }
    if package_source == PackageSource::ProjectLocal {
        return Err(ContributionValidationError::ProjectLocalExecutableContribution);
    }

    let package_version = manifest
        .version
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .ok_or(ContributionValidationError::MissingCompatibilityVersion)?;
    let opi_range = manifest
        .opi_version
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .ok_or(ContributionValidationError::MissingCompatibilityVersion)?;
    // Hard compatibility gate for executable contributions (distinct from the
    // advisory OpiVersionDiagnostic path retained for non-execution resources).
    if OpiVersionDiagnostic::check(opi_range, host_opi_version).is_some() {
        return Err(ContributionValidationError::IncompatibleOpiRange {
            range: opi_range.to_string(),
            host: host_opi_version.to_string(),
        });
    }

    if host_target.trim().is_empty() {
        return Err(ContributionValidationError::EmptyTarget);
    }

    let manifest_hash = sha256_hex(&lf_normalize(raw_manifest_bytes));
    let canonical_root = package_root.canonicalize().map_err(|e| {
        ContributionValidationError::CommandNotCanonicalizable(format!("package root: {e}"))
    })?;

    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut validated: Vec<ValidatedExecutableContribution> = Vec::new();

    for table_value in &manifest.adapter_contributions {
        let raw = deserialize_raw_contribution(table_value)?;
        let contribution = validate_one(
            &raw,
            &canonical_root,
            &manifest_hash,
            package_version,
            opi_range,
            host_target,
            &mut seen_ids,
        )?;
        validated.push(contribution);
    }

    Ok(validated)
}

/// Closed-field check + typed deserialization of one raw TOML contribution.
fn deserialize_raw_contribution(
    table_value: &toml::Value,
) -> Result<RawAdapterContribution, ContributionValidationError> {
    if !matches!(table_value, toml::Value::Table(_)) {
        return Err(ContributionValidationError::Malformed {
            reason: "contribution is not a table".into(),
        });
    }
    if let toml::Value::Table(table) = table_value {
        for key in table.keys() {
            if !KNOWN_ADAPTER_FIELDS.contains(&key.as_str()) {
                return Err(ContributionValidationError::UnknownContributionField {
                    field: key.clone(),
                });
            }
        }
    }
    // Toml -> serde_json -> typed struct. The closed-field check above is the
    // gate; this hop yields concrete types (adapter_config becomes a JSON value
    // matching the wire representation sized by the codec).
    let json =
        serde_json::to_value(table_value).map_err(|e| ContributionValidationError::Malformed {
            reason: format!("contribution conversion: {e}"),
        })?;
    serde_json::from_value::<RawAdapterContribution>(json).map_err(|e| {
        ContributionValidationError::Malformed {
            reason: e.to_string(),
        }
    })
}

#[allow(clippy::too_many_arguments)]
fn validate_one(
    raw: &RawAdapterContribution,
    canonical_root: &Path,
    manifest_hash: &str,
    package_version: &str,
    opi_range: &str,
    host_target: &str,
    seen_ids: &mut HashSet<String>,
) -> Result<ValidatedExecutableContribution, ContributionValidationError> {
    if raw.capability != "command.execute" {
        return Err(ContributionValidationError::CapabilityNotCommandExecute(
            raw.capability.clone(),
        ));
    }
    if raw.transport != "process-jsonl" {
        return Err(ContributionValidationError::TransportNotProcessJsonl(
            raw.transport.clone(),
        ));
    }
    if raw.protocol != opi_protocol::execution::v1::WIRE_IDENTITY {
        return Err(ContributionValidationError::ProtocolIncompatible {
            wanted: opi_protocol::execution::v1::WIRE_IDENTITY,
            got: raw.protocol.clone(),
        });
    }
    validate_adapter_id(&raw.id)?;
    if !seen_ids.insert(raw.id.clone()) {
        return Err(ContributionValidationError::DuplicateContributionId { id: raw.id.clone() });
    }
    if RESERVED_ADAPTER_IDS.contains(&raw.id.as_str()) {
        return Err(ContributionValidationError::ReservedAdapterId(
            raw.id.clone(),
        ));
    }

    if raw.target.trim().is_empty() {
        return Err(ContributionValidationError::EmptyTarget);
    }
    if raw.target != host_target {
        return Err(ContributionValidationError::IncompatibleTarget {
            wanted: host_target.to_string(),
            got: raw.target.clone(),
        });
    }

    if raw.handshake_timeout_ms == 0 || raw.handshake_timeout_ms > MAX_HANDSHAKE_TIMEOUT_MS {
        return Err(ContributionValidationError::OutOfRangeHandshakeTimeout {
            value: raw.handshake_timeout_ms,
            max: MAX_HANDSHAKE_TIMEOUT_MS,
        });
    }

    let config_bytes = serde_json::to_vec(&raw.adapter_config).map_err(|e| {
        ContributionValidationError::Malformed {
            reason: format!("adapter_config serialization: {e}"),
        }
    })?;
    let config_limit = opi_protocol::execution::v1::Bounds::DEFAULT.max_configuration_size;
    if config_bytes.len() > config_limit {
        return Err(ContributionValidationError::AdapterConfigTooLarge {
            actual: config_bytes.len(),
            limit: config_limit,
        });
    }

    let canonical_cmd = validate_command_path(&raw.command, canonical_root)?;

    // Inspect regularity before opening so a FIFO/device cannot block this
    // path, then repeat the check on the opened handle to close a replacement
    // race. Unix opens nonblocking/no-follow; Windows denies write/delete
    // sharing for the lifetime of the validated handle.
    let metadata = std::fs::metadata(&canonical_cmd)?;
    if !metadata.file_type().is_file() {
        return Err(ContributionValidationError::NonRegularExecutable);
    }
    if !is_executable(&metadata) {
        return Err(ContributionValidationError::NonExecutableFile);
    }
    let source_executable = open_executable(&canonical_cmd)?;
    let opened_metadata = source_executable.metadata()?;
    if !opened_metadata.file_type().is_file() {
        return Err(ContributionValidationError::NonRegularExecutable);
    }
    if !is_executable(&opened_metadata) {
        return Err(ContributionValidationError::NonExecutableFile);
    }
    // Bind launch to private copied material before hashing. A concurrent
    // in-place write to the package inode can only make the copied digest fail;
    // it cannot alter the descriptor that is later executed.
    let executable = bind_launch_material(&source_executable)?;
    let mut file_bytes = Vec::new();
    let mut reader = &executable;
    reader.read_to_end(&mut file_bytes)?;
    let computed = sha256_hex(&file_bytes);

    if !is_lower_hex64(&raw.sha256) {
        return Err(ContributionValidationError::MalformedSha256(
            raw.sha256.clone(),
        ));
    }
    if raw.sha256 != computed {
        return Err(ContributionValidationError::Sha256Mismatch {
            declared: raw.sha256.clone(),
            computed: computed.clone(),
        });
    }

    Ok(ValidatedExecutableContribution {
        capability: raw.capability.clone(),
        id: raw.id.clone(),
        transport: raw.transport.clone(),
        command: canonical_cmd,
        executable: Arc::new(executable),
        args: raw.args.clone(),
        protocol: raw.protocol.clone(),
        target: raw.target.clone(),
        handshake_timeout_ms: raw.handshake_timeout_ms,
        adapter_config: raw.adapter_config.clone(),
        lock: LockMaterial {
            manifest_hash: manifest_hash.to_string(),
            executable_rel_path: raw.command.clone(),
            executable_sha256: computed,
            package_version: package_version.to_string(),
            target: raw.target.clone(),
            opi_range: opi_range.to_string(),
            protocol: raw.protocol.clone(),
            adapter_id: raw.id.clone(),
        },
    })
}

fn open_executable(path: &Path) -> Result<File, std::io::Error> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;
        options.share_mode(FILE_SHARE_READ);
    }
    options.open(path)
}

#[cfg(unix)]
fn bind_launch_material(source: &File) -> Result<File, std::io::Error> {
    use std::io::{Seek as _, Write as _};
    use std::os::fd::{AsRawFd as _, FromRawFd as _};
    use std::os::unix::fs::PermissionsExt as _;

    #[cfg(target_os = "linux")]
    let mut snapshot = {
        let name = c"opi-adapter";
        // SAFETY: `name` is a live NUL-terminated static string; flags are the
        // documented memfd_create bitset. The returned fd is checked before it
        // is transferred exactly once into `File` ownership.
        let fd = unsafe {
            libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING)
        };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: `fd` is a newly-created owned descriptor on the success path.
        unsafe { File::from_raw_fd(fd) }
    };
    #[cfg(not(target_os = "linux"))]
    let mut snapshot = tempfile::tempfile()?;

    let mut source_reader = source;
    source_reader.seek(std::io::SeekFrom::Start(0))?;
    std::io::copy(&mut source_reader, &mut snapshot)?;
    snapshot.flush()?;
    snapshot.set_permissions(std::fs::Permissions::from_mode(0o500))?;

    #[cfg(target_os = "linux")]
    {
        let seals =
            libc::F_SEAL_SEAL | libc::F_SEAL_SHRINK | libc::F_SEAL_GROW | libc::F_SEAL_WRITE;
        // SAFETY: the descriptor is owned and has no writable mappings; adding
        // seals mutates only this anonymous file's kernel metadata.
        if unsafe { libc::fcntl(snapshot.as_raw_fd(), libc::F_ADD_SEALS, seals) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
    }

    let descriptor_path = if cfg!(target_os = "linux") {
        format!("/proc/self/fd/{}", snapshot.as_raw_fd())
    } else {
        format!("/dev/fd/{}", snapshot.as_raw_fd())
    };
    let launch = File::open(descriptor_path)?;
    let flags = unsafe { libc::fcntl(launch.as_raw_fd(), libc::F_GETFD) };
    if flags < 0
        || unsafe { libc::fcntl(launch.as_raw_fd(), libc::F_SETFD, flags & !libc::FD_CLOEXEC) } < 0
    {
        return Err(std::io::Error::last_os_error());
    }
    Ok(launch)
}

#[cfg(not(unix))]
fn bind_launch_material(source: &File) -> Result<File, std::io::Error> {
    source.try_clone()
}

/// Lexical reject of disallowed command shapes, then canonicalize-both-sides
/// containment. `normalize_path` is intentionally NOT reused: it is purely
/// lexical and resolves no symlinks, so it cannot catch symlink escape.
fn validate_command_path(
    command: &str,
    canonical_root: &Path,
) -> Result<PathBuf, ContributionValidationError> {
    let has_separator = command.contains('/') || command.contains('\\');
    for component in Path::new(command).components() {
        match component {
            Component::Prefix(_) => return Err(ContributionValidationError::DriveRelativeCommand),
            Component::RootDir => return Err(ContributionValidationError::AbsoluteCommand),
            Component::CurDir | Component::ParentDir => {
                return Err(ContributionValidationError::TraversalCommand);
            }
            Component::Normal(_) => {}
        }
    }
    if !has_separator {
        return Err(ContributionValidationError::BarePathCommand);
    }

    let canonical_cmd = canonical_root
        .join(command)
        .canonicalize()
        .map_err(|e| ContributionValidationError::CommandNotCanonicalizable(e.to_string()))?;
    if !canonical_cmd.starts_with(canonical_root) {
        return Err(ContributionValidationError::SymlinkEscape);
    }
    Ok(canonical_cmd)
}

fn validate_adapter_id(id: &str) -> Result<(), ContributionValidationError> {
    if id.is_empty() || id.len() > 64 {
        return Err(ContributionValidationError::AdapterIdInvalid {
            reason: "id must be 1..=64 characters".into(),
        });
    }
    for ch in id.chars() {
        if !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-') {
            return Err(ContributionValidationError::AdapterIdInvalid {
                reason: format!("invalid character {ch:?} (lowercase a-z, 0-9, hyphen only)"),
            });
        }
    }
    Ok(())
}

/// Regular-file executability. On Unix, any mode bit `0o111`; on Windows,
/// executability is enforced by the OS at spawn (Phase 16.7), so a regular file
/// is accepted here.
fn is_executable(metadata: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        true
    }
}

fn is_lower_hex64(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}

/// CRLF -> LF so the manifest hash is reproducible across checkouts (git may
/// materialize LF as CRLF on Windows).
fn lf_normalize(bytes: &[u8]) -> Vec<u8> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_hex64_predicate() {
        assert!(is_lower_hex64(
            "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
        ));
        assert!(!is_lower_hex64("ABC...")); // uppercase rejected (not shown fully)
        assert!(!is_lower_hex64(&"a".repeat(63)));
        assert!(!is_lower_hex64(&"a".repeat(65)));
        assert!(!is_lower_hex64(&format!("{:x<64}", 'z'))); // 'z' is not hex
    }

    #[test]
    fn adapter_id_charset() {
        assert!(validate_adapter_id("opi-sandbox").is_ok());
        assert!(matches!(
            validate_adapter_id("").unwrap_err(),
            ContributionValidationError::AdapterIdInvalid { .. }
        ));
        assert!(matches!(
            validate_adapter_id("Bad_Id").unwrap_err(),
            ContributionValidationError::AdapterIdInvalid { .. }
        ));
    }
}
