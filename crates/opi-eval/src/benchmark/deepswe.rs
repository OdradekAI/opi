//! Crate-private DeepSWE v1.1 benchmark adapter (Phase 18 task 18.10).
//!
//! Owns only what is DeepSWE-v1.1-specific: the pinned declarative profile
//! for `datacurve-ai/deep-swe` (which has no `v1.1` Git tag, so the profile
//! records both the explicit semantic version-anchor commit and the
//! executable revision that adds the `[[verifier.collect]]` patch handoff),
//! the pristine grading semantics that must not be normalized away, and
//! revision-specific native-result settlement through the shared contract
//! in [`super::process`]. The mutable agent worktree is never graded
//! directly: the declared collected patch is graded in a separate pristine
//! verifier environment with no network. DeepSWE's own pinned instruction
//! selects Pier for that lifecycle, so the runner surface is Pier
//! (`v0.3.1`, newer than the 0.3.0 the pinned README requires), not
//! Harbor. Task-package byte tables, the final verifier image, and the
//! native-output schema are unresolved slots owned by task 18.15, so the
//! production profile pins the task-tree identity only and the adapter
//! fails closed until reviewed bytes are registered.

use std::collections::BTreeMap;
use std::path::Path;

/// sha256 hex digest of `bytes`.
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// One pinned task-package file row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PinnedFile {
    pub path: String,
    pub mode: String,
    pub size: u64,
    pub sha256: String,
}

/// How the profile admits the task package. The committed static external
/// lock pins DeepSWE v1.1 by task-tree identity with an empty files table
/// (`tree-identity`); a `byte-table` pin requires a registered complete
/// file closure, which today exists only for synthetic fixtures and
/// arrives for the production task with task 18.15.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PackagePin {
    /// Task-tree identity only: admission fails closed as not materialized.
    TreeIdentity,
    /// Complete registered file table: exact-set, exact-bytes admission.
    ByteTable,
}

/// The pinned native-output surface. No DeepSWE native-output schema is
/// committed; `pier-report` is admitted only for synthetic wiring fixtures
/// (a minimal resolved-flag report) until task 18.15 pins the real schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputKind {
    UnpinnedPending1815,
    PierReport,
}

/// The parsed declarative DeepSWE v1.1 profile: pinned identity (semantic
/// anchor plus executable revision), package pin, the pristine verifier
/// entry with no-network collected-patch semantics, and limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeepSweProfile {
    pub version_anchor: String,
    pub source_commit: String,
    pub tasks_tree: String,
    pub task_id: String,
    pub task_tree: String,
    pub package_pin: PackagePin,
    pub package_manifest_sha256: String,
    pub adapter: String,
    pub runner_version: String,
    pub runner_commit: String,
    pub runner_uv_lock_blob: String,
    pub runner_uv_lock_sha256: String,
    pub launch: Vec<String>,
    pub output_kind: OutputKind,
    pub cpus: u64,
    pub memory_gib: u64,
    pub timeout_secs: u64,
    pub stdout_cap_bytes: u64,
    pub stderr_cap_bytes: u64,
    /// Sorted by path; exactly the pinned package closure (empty unless the
    /// pin is `byte-table`).
    pub package: Vec<PinnedFile>,
}

/// Typed profile failures. Fail-closed on any drift from the pinned surface.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TbProfileError {
    Io(String),
    Parse(String),
    UnsupportedSchema(String),
    /// A pinned invariant drifted (unknown runner/lifecycle/network/output
    /// kind, malformed launch template, bad digests, duplicate/empty package
    /// rows, pin-mode contradictions).
    Drift(String),
}

impl std::fmt::Display for TbProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TbProfileError::Io(message) => write!(f, "profile io failure: {message}"),
            TbProfileError::Parse(message) => write!(f, "profile parse failure: {message}"),
            TbProfileError::UnsupportedSchema(schema) => {
                write!(f, "unsupported benchmark profile schema: {schema}")
            }
            TbProfileError::Drift(message) => {
                write!(
                    f,
                    "benchmark profile drifted from the pinned surface: {message}"
                )
            }
        }
    }
}

impl std::error::Error for TbProfileError {}

/// Raw TOML mirror of the declarative profile (unknown keys are rejected by
/// `deny_unknown_fields`, so any added field fails closed).
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct DeepSweDoc {
    schema: String,
    benchmark: String,
    revision: String,
    identity: IdentityDoc,
    verifier: VerifierDoc,
    resources: ResourcesDoc,
    limits: LimitsDoc,
    #[serde(default)]
    package: Vec<PackageDoc>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentityDoc {
    upstream: String,
    version_anchor: String,
    source_commit: String,
    tasks_tree: String,
    task_id: String,
    task_tree: String,
    package_pin: String,
    package_manifest_sha256: String,
    adapter: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifierDoc {
    runner_kind: String,
    runner_version: String,
    runner_commit: String,
    runner_uv_lock_blob: String,
    runner_uv_lock_sha256: String,
    launch: Vec<String>,
    lifecycle: String,
    network: String,
    artifact_policy: String,
    output_kind: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ResourcesDoc {
    cpus: u64,
    memory_gib: u64,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LimitsDoc {
    timeout_secs: u64,
    stdout_cap_bytes: u64,
    stderr_cap_bytes: u64,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageDoc {
    path: String,
    mode: String,
    size: u64,
    sha256: String,
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

impl DeepSweProfile {
    /// Parse and fully validate a declarative profile document.
    ///
    /// Fails closed on an unsupported schema, a non-DeepSWE-v1.1 identity,
    /// a missing semantic version anchor, a runner surface other than the
    /// pinned Pier closure, a non-pristine or networked verifier lifecycle,
    /// a malformed launch template, bad digests, and any pin-mode
    /// contradiction between `package_pin` and the file table.
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, TbProfileError> {
        let text = String::from_utf8(bytes.to_vec())
            .map_err(|_| TbProfileError::Parse("profile is not valid UTF-8".to_owned()))?;
        let doc: DeepSweDoc =
            toml::from_str(&text).map_err(|e| TbProfileError::Parse(e.to_string()))?;
        if doc.schema != "phase18-benchmark-profile/1" {
            return Err(TbProfileError::UnsupportedSchema(doc.schema));
        }
        if doc.benchmark != "deepswe" || doc.revision != "v1.1" {
            return Err(TbProfileError::Drift(format!(
                "profile is {} {}, not deepswe v1.1",
                doc.benchmark, doc.revision
            )));
        }
        if doc.verifier.runner_kind != "pier" {
            return Err(TbProfileError::Drift(format!(
                "runner_kind {} is not the pinned Pier surface DeepSWE selects",
                doc.verifier.runner_kind
            )));
        }
        // The pinned README requires Pier newer than 0.3.0; the exact
        // admitted closure is v0.3.1 with its committed uv.lock blob and
        // digest so `uv run --locked` cannot resolve unconstrained PyPI.
        if doc.verifier.runner_version != "v0.3.1" {
            return Err(TbProfileError::Drift(format!(
                "runner_version {} is not the pinned Pier v0.3.1 closure",
                doc.verifier.runner_version
            )));
        }
        // The pristine grading semantics are pinned, not normalized away:
        // the declared collected patch is graded in a separate pristine
        // verifier environment with no network; the mutable agent worktree
        // is never graded directly.
        if doc.verifier.lifecycle != "separate-pristine-verifier" {
            return Err(TbProfileError::Drift(format!(
                "lifecycle {} is not the pinned separate pristine verifier",
                doc.verifier.lifecycle
            )));
        }
        if doc.verifier.network != "none" {
            return Err(TbProfileError::Drift(format!(
                "network {} is not the pinned no-network verifier phase",
                doc.verifier.network
            )));
        }
        if doc.verifier.artifact_policy != "collected-patch" {
            return Err(TbProfileError::Drift(format!(
                "artifact_policy {} is not the pinned collected-patch surface",
                doc.verifier.artifact_policy
            )));
        }
        let output_kind = match doc.verifier.output_kind.as_str() {
            "unpinned-pending-18-15" => OutputKind::UnpinnedPending1815,
            "pier-report" => OutputKind::PierReport,
            other => {
                return Err(TbProfileError::Drift(format!(
                    "output_kind {other} is not an admitted kind"
                )));
            }
        };
        // The launch template must use the `<task-dir>` placeholder exactly
        // once; everything else is pinned argv.
        let placeholders: Vec<&String> = doc
            .verifier
            .launch
            .iter()
            .filter(|part| part.starts_with('<') && part.ends_with('>'))
            .collect();
        if placeholders.len() != 1 || placeholders[0] != "<task-dir>" {
            return Err(TbProfileError::Drift(format!(
                "launch template must substitute exactly <task-dir>, got {placeholders:?}"
            )));
        }
        // DeepSWE v1.1 has no tag: the semantic version anchor and the
        // executable revision are both 40-hex commits, and neither may be
        // presented as a tag.
        for value in [
            &doc.identity.version_anchor,
            &doc.identity.source_commit,
            &doc.identity.tasks_tree,
            &doc.identity.task_tree,
            &doc.verifier.runner_commit,
            &doc.verifier.runner_uv_lock_blob,
        ] {
            if value.len() != 40 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(TbProfileError::Drift(format!(
                    "identity {value} is not a 40-hex Git identity"
                )));
            }
        }
        if !is_sha256_hex(&doc.identity.package_manifest_sha256)
            || !is_sha256_hex(&doc.verifier.runner_uv_lock_sha256)
        {
            return Err(TbProfileError::Drift(
                "manifest and runner lock digests must be lowercase 64-hex".to_owned(),
            ));
        }
        if doc.identity.task_id.trim().is_empty() || doc.identity.adapter.trim().is_empty() {
            return Err(TbProfileError::Drift(
                "task_id and adapter must not be empty".to_owned(),
            ));
        }
        let package_pin = match doc.identity.package_pin.as_str() {
            "tree-identity" => PackagePin::TreeIdentity,
            "byte-table" => PackagePin::ByteTable,
            other => {
                return Err(TbProfileError::Drift(format!(
                    "package_pin {other} is not an admitted pin"
                )));
            }
        };

        let mut package: Vec<PinnedFile> = doc
            .package
            .into_iter()
            .map(|row| PinnedFile {
                path: row.path,
                mode: row.mode,
                size: row.size,
                sha256: row.sha256,
            })
            .collect();
        match package_pin {
            PackagePin::TreeIdentity => {
                if !package.is_empty() {
                    return Err(TbProfileError::Drift(
                        "a tree-identity pin must not declare a byte table".to_owned(),
                    ));
                }
            }
            PackagePin::ByteTable => {
                if package.len() < 2 {
                    return Err(TbProfileError::Drift(
                        "a byte-table pin must be a complete multi-file closure".to_owned(),
                    ));
                }
            }
        }
        package.sort_by(|a, b| a.path.cmp(&b.path));
        let mut seen = std::collections::BTreeSet::new();
        for file in &package {
            if file.path.trim().is_empty() || file.path.contains("..") {
                return Err(TbProfileError::Drift(format!(
                    "package path {:?} is empty or escapes the package root",
                    file.path
                )));
            }
            if file.mode != "100644" && file.mode != "100755" {
                return Err(TbProfileError::Drift(format!(
                    "package path {} has non-regular mode {}",
                    file.path, file.mode
                )));
            }
            if !is_sha256_hex(&file.sha256) {
                return Err(TbProfileError::Drift(format!(
                    "package path {} has a malformed sha256",
                    file.path
                )));
            }
            if !seen.insert(file.path.clone()) {
                return Err(TbProfileError::Drift(format!(
                    "package path {} is declared twice",
                    file.path
                )));
            }
        }
        let profile = Self {
            version_anchor: doc.identity.version_anchor,
            source_commit: doc.identity.source_commit,
            tasks_tree: doc.identity.tasks_tree,
            task_id: doc.identity.task_id,
            task_tree: doc.identity.task_tree,
            package_pin,
            package_manifest_sha256: doc.identity.package_manifest_sha256,
            adapter: doc.identity.adapter,
            runner_version: doc.verifier.runner_version,
            runner_commit: doc.verifier.runner_commit,
            runner_uv_lock_blob: doc.verifier.runner_uv_lock_blob,
            runner_uv_lock_sha256: doc.verifier.runner_uv_lock_sha256,
            launch: doc.verifier.launch,
            output_kind,
            cpus: doc.resources.cpus,
            memory_gib: doc.resources.memory_gib,
            timeout_secs: doc.limits.timeout_secs,
            stdout_cap_bytes: doc.limits.stdout_cap_bytes,
            stderr_cap_bytes: doc.limits.stderr_cap_bytes,
            package,
        };
        let computed = profile.canonical_package_digest();
        if computed != profile.package_manifest_sha256 {
            return Err(TbProfileError::Drift(format!(
                "package manifest digest {computed} does not match the declared {}",
                profile.package_manifest_sha256
            )));
        }
        Ok(profile)
    }

    /// Canonical digest of the pinned file table: sha256 over the compact
    /// JSON array of `{mode, path, sha256, size}` rows sorted by path
    /// (object keys serialize in sorted order). A tree-identity pin hashes
    /// the empty table, pinning that no byte table is registered.
    pub(crate) fn canonical_package_digest(&self) -> String {
        let table: Vec<serde_json::Value> = self
            .package
            .iter()
            .map(|file| {
                serde_json::json!({
                    "mode": file.mode,
                    "path": file.path,
                    "sha256": file.sha256,
                    "size": file.size,
                })
            })
            .collect();
        sha256_hex(
            serde_json::to_string(&table)
                .expect("serializable")
                .as_bytes(),
        )
    }

    /// Admit a materialized task package against the pin
    /// (`P18-BMK-002`). A `tree-identity` pin fails closed as not
    /// materialized: no committed byte table exists to validate against, so
    /// no DeepSWE verifier invocation is admitted until task 18.15
    /// registers one. A `byte-table` pin requires exactly the pinned files,
    /// byte-identical digests.
    pub(crate) fn admit_task_package(&self, root: &Path) -> Result<(), TaskPackageError> {
        match self.package_pin {
            PackagePin::TreeIdentity => Err(TaskPackageError::NotMaterialized),
            PackagePin::ByteTable => self.validate_task_package(root),
        }
    }

    /// Validate a materialized task package against the pinned closure:
    /// exactly the pinned files, byte-identical digests.
    fn validate_task_package(&self, root: &Path) -> Result<(), TaskPackageError> {
        let mut observed: BTreeMap<String, (u64, String)> = BTreeMap::new();
        walk_files(root, root, &mut observed).map_err(|e| TaskPackageError::Io(e.to_string()))?;
        let mut expected: BTreeMap<&str, &PinnedFile> =
            self.package.iter().map(|f| (f.path.as_str(), f)).collect();
        for (path, (size, digest)) in &observed {
            match expected.remove(path.as_str()) {
                Some(file) => {
                    if *size != file.size {
                        return Err(TaskPackageError::Mismatch {
                            path: path.clone(),
                            reason: format!("size {size} != pinned {}", file.size),
                        });
                    }
                    if *digest != file.sha256 {
                        return Err(TaskPackageError::Mismatch {
                            path: path.clone(),
                            reason: "sha256 does not match the pinned value".to_owned(),
                        });
                    }
                }
                None => {
                    return Err(TaskPackageError::ExtraFile { path: path.clone() });
                }
            }
        }
        if let Some(path) = expected.keys().next() {
            return Err(TaskPackageError::MissingFile {
                path: (*path).to_owned(),
            });
        }
        Ok(())
    }
}

fn walk_files(
    root: &Path,
    dir: &Path,
    observed: &mut BTreeMap<String, (u64, String)>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)?;
        if metadata.is_dir() {
            walk_files(root, &path, observed)?;
        } else if metadata.is_file() {
            let bytes = std::fs::read(&path)?;
            let relative = path
                .strip_prefix(root)
                .expect("walk stays under root")
                .to_string_lossy()
                .replace('\\', "/");
            observed.insert(relative, (bytes.len() as u64, sha256_hex(&bytes)));
        } else {
            return Err(std::io::Error::other(
                "task package contains a non-regular entry",
            ));
        }
    }
    Ok(())
}

/// Typed task-package admission failures (`P18-BMK-002`).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TaskPackageError {
    Io(String),
    /// The pin is task-tree identity only and no byte table is registered.
    NotMaterialized,
    MissingFile {
        path: String,
    },
    ExtraFile {
        path: String,
    },
    Mismatch {
        path: String,
        reason: String,
    },
}

impl std::fmt::Display for TaskPackageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskPackageError::Io(message) => write!(f, "task package io failure: {message}"),
            TaskPackageError::NotMaterialized => write!(
                f,
                "task package is pinned by tree identity only and no byte table is registered"
            ),
            TaskPackageError::MissingFile { path } => {
                write!(f, "task package is missing pinned file {path}")
            }
            TaskPackageError::ExtraFile { path } => {
                write!(f, "task package contains unpinned file {path}")
            }
            TaskPackageError::Mismatch { path, reason } => {
                write!(f, "task package file {path} drifted: {reason}")
            }
        }
    }
}

impl std::error::Error for TaskPackageError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn crate_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn read(path: &Path) -> Vec<u8> {
        std::fs::read(path).unwrap()
    }

    fn fixture(relative: &str) -> PathBuf {
        crate_root()
            .join("tests/fixtures/benchmarks/deepswe-v1.1")
            .join(relative)
    }

    fn synthetic_bytes() -> Vec<u8> {
        read(&fixture("profile/synthetic.toml"))
    }

    #[test]
    fn production_profile_is_pinned_to_the_static_external_lock() {
        // P18-BMK-001/P18-BMK-008: the profile must restate exactly the
        // identity pins the committed static lock already admits for
        // DeepSWE v1.1 — and only those. The lock has no tag for DeepSWE:
        // the semantic version anchor and the executable revision are both
        // commits, and the lock's files table is empty, so the profile must
        // pin the task-tree identity without inventing bytes.
        let profile = DeepSweProfile::parse(&read(
            &crate_root().join("profiles/benchmarks/deepswe-v1.1.toml"),
        ))
        .unwrap();
        assert_eq!(
            profile.version_anchor,
            "8cae5984d5dd0ee37445beff0e928dc10c331116"
        );
        assert_eq!(
            profile.source_commit,
            "435ee89ec2f2e2289f33b0da4f992f0b7b7266b9"
        );
        assert_eq!(
            profile.tasks_tree,
            "66df25a1b382017d0ae014d94cadb2698baaed48"
        );
        assert_eq!(profile.task_id, "abs-module-cache-flags");
        assert_eq!(
            profile.task_tree,
            "8ee76a3e4a876a9bd24f34edabcdde4e47257db4"
        );
        // Task 18.15 registered the reviewed official byte table: 10
        // pinned files with their canonical digest declared.
        assert_eq!(profile.package_pin, PackagePin::ByteTable);
        assert_eq!(profile.package.len(), 10);
        assert_eq!(
            profile.canonical_package_digest(),
            profile.package_manifest_sha256
        );
        assert_eq!(profile.output_kind, OutputKind::UnpinnedPending1815);
        // The pinned Pier closure: v0.3.1 source with its committed
        // uv.lock blob and digest, taken at the first self-consistent
        // descendant of the v0.3.1 release commit (whose shipped lock
        // still recorded the previous root package version).
        assert_eq!(profile.runner_version, "v0.3.1");
        assert_eq!(
            profile.runner_commit,
            "c1ebc6d145b40fae8425215e3fca528945065124"
        );
        assert_eq!(
            profile.runner_uv_lock_blob,
            "762c6e2c3410021edefe50ce93d9a5d341821b50"
        );
        assert_eq!(
            profile.runner_uv_lock_sha256,
            "0983c4376bb818a984b80badd28886c204c78875e0112387efb03af401ad86c0"
        );
        assert_eq!(profile.launch, vec!["run", "-p", "<task-dir>"]);
        assert_eq!((profile.cpus, profile.memory_gib), (2, 8));

        let lock: serde_json::Value = serde_json::from_slice(&read(
            &crate_root().join("external-locks/static/linux-x86_64.json"),
        ))
        .unwrap();
        let subject = &lock["subjects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == "deepswe-v1.1")
            .unwrap();
        assert_eq!(
            subject["version_anchor"].as_str().unwrap(),
            profile.version_anchor,
            "DeepSWE has no v1.1 tag: the anchor must stay a commit, never a tag"
        );
        assert_eq!(subject["commit"].as_str().unwrap(), profile.source_commit);
        assert_eq!(subject["tasks_tree"].as_str().unwrap(), profile.tasks_tree);
        assert_eq!(subject["task"]["id"].as_str().unwrap(), profile.task_id);
        assert_eq!(subject["task"]["tree"].as_str().unwrap(), profile.task_tree);
        assert_eq!(
            subject["task"]["files"].as_array().map(Vec::len),
            Some(0),
            "the lock pins DeepSWE by tree identity; a byte table would be unregistered bytes"
        );

        // The Pier runner pin lives in the lock's tools table while the
        // profile restates it; both sources must agree exactly, or the
        // producer fetches a different runner than admission pinned.
        let tool = &lock["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["id"] == "pier")
            .unwrap();
        assert_eq!(
            tool["commit"].as_str().unwrap(),
            profile.runner_commit,
            "the static lock and the profile pin different Pier commits"
        );
        assert_eq!(
            tool["uv_lock"]["git_blob"].as_str().unwrap(),
            profile.runner_uv_lock_blob
        );
        assert_eq!(
            tool["uv_lock"]["sha256"].as_str().unwrap(),
            profile.runner_uv_lock_sha256
        );
    }

    #[test]
    fn synthetic_profile_accepts_its_byte_table_package() {
        let profile = DeepSweProfile::parse(&synthetic_bytes()).unwrap();
        assert_eq!(profile.package_pin, PackagePin::ByteTable);
        assert_eq!(profile.output_kind, OutputKind::PierReport);
        assert_eq!(profile.package.len(), 3);
        assert_eq!(
            profile.canonical_package_digest(),
            profile.package_manifest_sha256
        );
        profile
            .admit_task_package(&fixture("task-package"))
            .unwrap();
    }

    #[test]
    fn task_package_admission_fails_closed_on_every_drift() {
        let profile = DeepSweProfile::parse(&synthetic_bytes()).unwrap();
        let dir = tempfile::tempdir().unwrap();

        // Missing pinned file.
        let missing = dir.path().join("missing");
        copy_package(&fixture("task-package"), &missing);
        std::fs::remove_file(missing.join("verifier/collect.sh")).unwrap();
        assert_eq!(
            profile.admit_task_package(&missing),
            Err(TaskPackageError::MissingFile {
                path: "verifier/collect.sh".to_owned()
            })
        );

        // Extra unpinned file.
        let extra = dir.path().join("extra");
        copy_package(&fixture("task-package"), &extra);
        std::fs::write(extra.join("rogue.txt"), b"unregistered").unwrap();
        assert_eq!(
            profile.admit_task_package(&extra),
            Err(TaskPackageError::ExtraFile {
                path: "rogue.txt".to_owned()
            })
        );

        // Byte drift inside a pinned file (same size, different bytes: the
        // digest check must catch it, not just a size check).
        let drifted = dir.path().join("drifted");
        copy_package(&fixture("task-package"), &drifted);
        let original = std::fs::read(drifted.join("README.md")).unwrap();
        let mut tampered = original.clone();
        tampered[0] = b'X';
        std::fs::write(drifted.join("README.md"), tampered).unwrap();
        assert_eq!(
            profile.admit_task_package(&drifted),
            Err(TaskPackageError::Mismatch {
                path: "README.md".to_owned(),
                reason: "sha256 does not match the pinned value".to_owned()
            })
        );

        // The registered byte table (task 18.15) rejects the synthetic
        // fixture bytes with the first drift it meets - missing, extra,
        // or content - never silently.
        let production = DeepSweProfile::parse(&read(
            &crate_root().join("profiles/benchmarks/deepswe-v1.1.toml"),
        ))
        .unwrap();
        assert!(
            production
                .admit_task_package(&fixture("task-package"))
                .is_err()
        );
    }

    fn copy_package(source: &Path, target: &Path) {
        std::fs::create_dir_all(target).unwrap();
        for entry in std::fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name();
                std::fs::create_dir_all(target.join(&name)).unwrap();
                copy_package(&path, &target.join(&name));
            } else {
                let name = entry.file_name();
                std::fs::copy(&path, target.join(&name)).unwrap();
            }
        }
    }

    #[test]
    fn profile_drift_fails_closed() {
        let text = String::from_utf8(synthetic_bytes()).unwrap();

        let reparse = |text: &str| DeepSweProfile::parse(text.as_bytes());

        // Wrong schema family.
        assert!(matches!(
            reparse(&text.replace("phase18-benchmark-profile/1", "phase18-benchmark-profile/2")),
            Err(TbProfileError::UnsupportedSchema(_))
        ));

        // A Terminal-Bench-style revision is not this adapter.
        assert!(matches!(
            reparse(&text.replace("revision = \"v1.1\"", "revision = \"3.0\"")),
            Err(TbProfileError::Drift(_))
        ));

        // Unknown keys are rejected (deny_unknown_fields): a dropped
        // DeepSWE semantic like the no-network constraint is a parse
        // failure, not a silent default.
        assert!(matches!(
            reparse(&text.replace("network = \"none\"", "")),
            Err(TbProfileError::Parse(_))
        ));
        assert!(matches!(
            reparse(&text.replace("network = \"none\"", "network = \"full\"")),
            Err(TbProfileError::Drift(_))
        ));

        // Pristine-verifier lifecycle drift: grading the mutable agent
        // worktree directly is never admitted.
        assert!(matches!(
            reparse(&text.replace(
                "lifecycle = \"separate-pristine-verifier\"",
                "lifecycle = \"in-place\""
            )),
            Err(TbProfileError::Drift(_))
        ));

        // Runner drift: Harbor owns the Terminal-Bench revisions, not
        // DeepSWE.
        assert!(matches!(
            reparse(&text.replace("runner_kind = \"pier\"", "runner_kind = \"harbor\"")),
            Err(TbProfileError::Drift(_))
        ));
        assert!(matches!(
            reparse(&text.replace("runner_version = \"v0.3.1\"", "runner_version = \"v0.3.0\"")),
            Err(TbProfileError::Drift(_))
        ));

        // Bad hex identities.
        assert!(matches!(
            reparse(&text.replace(
                "1111111111111111111111111111111111111111",
                "zz11111111111111111111111111111111111111"
            )),
            Err(TbProfileError::Drift(_))
        ));

        // Pin-mode contradictions: a tree-identity pin with a byte table,
        // and an unadmitted pin mode.
        assert!(matches!(
            reparse(&text.replace(
                "package_pin = \"byte-table\"",
                "package_pin = \"tree-identity\""
            )),
            Err(TbProfileError::Drift(_))
        ));
        assert!(matches!(
            reparse(&text.replace(
                "package_pin = \"byte-table\"",
                "package_pin = \"unverified\""
            )),
            Err(TbProfileError::Drift(_))
        ));
        let digest_flipped = {
            let declared = text
                .lines()
                .find_map(|l| l.strip_prefix("package_manifest_sha256 = "))
                .unwrap()
                .trim_matches('"')
                .to_owned();
            let first_is_1 = declared.starts_with('1');
            let flipped_first = if first_is_1 { '2' } else { '1' };
            let replacement = format!(
                "package_manifest_sha256 = \"{flipped_first}{}\"",
                &declared[1..]
            );
            text.replace(
                &format!("package_manifest_sha256 = \"{declared}\""),
                &replacement,
            )
        };
        assert!(matches!(
            reparse(&digest_flipped),
            Err(TbProfileError::Drift(_))
        ));

        // Launch template drift: two placeholders, or none.
        assert!(matches!(
            reparse(&text.replace(
                "launch = [\"run\", \"-p\", \"<task-dir>\"]",
                "launch = [\"run\", \"-p\", \"<task-dir>\", \"<extra>\"]"
            )),
            Err(TbProfileError::Drift(_))
        ));

        // Output-kind drift: an unadmitted native-output schema.
        assert!(matches!(
            reparse(&text.replace("output_kind = \"pier-report\"", "output_kind = \"jsonl\"")),
            Err(TbProfileError::Drift(_))
        ));

        // A byte-table pin must be a complete closure, not a single row.
        let single_row = {
            let positions: Vec<usize> = text.match_indices("[[package]]").map(|(i, _)| i).collect();
            assert_eq!(positions.len(), 3);
            format!("{}{}", &text[..positions[1]], &text[positions[2]..])
        };
        assert!(matches!(
            reparse(&single_row),
            Err(TbProfileError::Drift(_))
        ));
    }
}

// ---------------------------------------------------------------------------
// Adapter: benchmark-neutral contract implementation for DeepSWE v1.1.

use super::process::{
    BenchmarkAdapter, BenchmarkCompletion, BenchmarkIdentity, BenchmarkRunRequest, ExecutionError,
    failure_kinds,
};
use crate::agent::process::{Fact, NativeArtifact};
use crate::failure::FailureBoundaryCode;
use crate::process::{ExitState, SpawnSpec};
use std::time::Duration;

/// Where a synthetic `pier-report` wiring fixture expects the native
/// report. The production DeepSWE profile pins no output schema, so no
/// filename is claimed for real DeepSWE runs until task 18.15 registers
/// one.
const PIER_REPORT_NAME: &str = "pier-report.json";

/// The DeepSWE v1.1 benchmark adapter. Owns the pinned declarative
/// profile, the tree-identity/byte-table package admission, the exact
/// Pier v0.3.1 launch through `uv run --locked` under pristine
/// no-network collected-patch semantics, and the revision-specific
/// native-result settlement. The native reward is authoritative — it comes
/// only from the pristine verifier's report, and zero is a valid resolved
/// verdict, never a failure.
pub(crate) struct DeepSweAdapter {
    profile: DeepSweProfile,
}

impl DeepSweAdapter {
    /// Build the adapter from a validated declarative profile.
    pub(crate) fn from_profile(profile: DeepSweProfile) -> Self {
        Self { profile }
    }

    /// The profile this adapter pins.
    pub(crate) fn profile(&self) -> &DeepSweProfile {
        &self.profile
    }

    /// Import the minimal Pier-report schema used by synthetic wiring
    /// fixtures: a single object with exactly a `reward` resolved flag
    /// (0 = not resolved, 1 = resolved; zero is valid, never an error).
    /// This is a wiring hypothesis, not a pinned DeepSWE native-output
    /// schema: the real importer shape is registered by task 18.15.
    /// Fails closed on drift.
    fn import_pier_report(bytes: &[u8]) -> Result<u64, PierReportError> {
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| PierReportError::Parse)?;
        let Some(fields) = value.as_object() else {
            return Err(PierReportError::UnsupportedSchema);
        };
        if fields.len() != 1 || !fields.contains_key("reward") {
            return Err(PierReportError::UnsupportedSchema);
        }
        match fields["reward"].as_u64() {
            Some(reward @ (0 | 1)) => Ok(reward),
            _ => Err(PierReportError::UnsupportedSchema),
        }
    }
}

/// Typed Pier-report import failures (wiring-fixture surface).
enum PierReportError {
    Parse,
    UnsupportedSchema,
}

impl BenchmarkAdapter for DeepSweAdapter {
    fn identity(&self) -> BenchmarkIdentity {
        BenchmarkIdentity {
            benchmark: "deepswe".to_owned(),
            revision: "v1.1".to_owned(),
            adapter: self.profile.adapter.clone(),
        }
    }

    fn admission(&self, request: &BenchmarkRunRequest) -> Result<(), ExecutionError> {
        // Revision binding: the integrity record must admit exactly the
        // revision this adapter pins, and the requested task must be the
        // profile's pinned task.
        if request.integrity.revision() != "v1.1" {
            return Err(ExecutionError {
                token: "revision-binding-mismatch",
            });
        }
        if request.task_id != self.profile.task_id {
            return Err(ExecutionError {
                token: "task-binding-mismatch",
            });
        }
        // Task-package admission before any verifier invocation
        // (P18-BMK-002). A tree-identity pin fails closed as not
        // materialized: DeepSWE verifier runs are not admitted until task
        // 18.15 registers a reviewed byte table.
        self.profile
            .admit_task_package(&request.task_dir)
            .map_err(|e| ExecutionError {
                token: match e {
                    TaskPackageError::NotMaterialized => "task-package-not-materialized",
                    _ => "task-package-drift",
                },
            })?;
        Ok(())
    }

    fn spawn_spec(&self, request: &BenchmarkRunRequest) -> SpawnSpec {
        // Pinned evidence: DeepSWE selects Pier v0.3.1 (newer than the
        // 0.3.0 its README requires), driven through `uv run --locked`
        // against the committed uv.lock blob so nothing resolves from
        // unconstrained PyPI; argv[0] is the resolved uv executable, never
        // an ambient PATH lookup. The pristine no-network collected-patch
        // lifecycle is Pier's own, pinned in the profile and not normalized
        // away here. The launch argv is re-pinned against the pinned Pier
        // CLI: `pier run --path/-p <dir>`, no positional task path.
        let mut argv: Vec<std::ffi::OsString> = vec![
            request.verifier_executable.clone().into(),
            "run".into(),
            "--locked".into(),
            "pier".into(),
        ];
        for part in &self.profile.launch {
            argv.push(if part == "<task-dir>" {
                request.task_dir.clone().into()
            } else {
                part.clone().into()
            });
        }
        SpawnSpec {
            argv,
            cwd: Some(request.trace_root.clone()),
            env: request.extra_env.clone(),
            stdout_cap: self.profile.stdout_cap_bytes as usize,
            stderr_cap: self.profile.stderr_cap_bytes as usize,
            timeout: Duration::from_secs(self.profile.timeout_secs),
        }
    }

    fn settle(
        &self,
        outcome: &crate::process::SupervisedOutcome,
        request: &BenchmarkRunRequest,
    ) -> (Fact, BenchmarkCompletion) {
        let reward_unknown = || Fact::Unknown {
            reason: "native-reward-pending-18-15-smoke".to_owned(),
        };
        // The process-level verdict is authoritative first: a non-zero exit,
        // timeout, cancellation, or spawn failure can never be rescued by
        // evidence, and no fallback grader exists (P18-BMK-006).
        match outcome.exit {
            ExitState::Exited { code: 0 } => {}
            ExitState::Exited { code } => {
                return (
                    reward_unknown(),
                    BenchmarkCompletion::Failed(failure_kinds::non_zero_exit(code)),
                );
            }
            ExitState::TimedOut => {
                return (
                    reward_unknown(),
                    BenchmarkCompletion::Failed(failure_kinds::TIMED_OUT),
                );
            }
            ExitState::Cancelled => {
                return (
                    reward_unknown(),
                    BenchmarkCompletion::Failed(failure_kinds::CANCELLED),
                );
            }
            ExitState::FailedToSpawn { reason } => {
                return (
                    reward_unknown(),
                    BenchmarkCompletion::Failed(failure_kinds::spawn(reason)),
                );
            }
        }
        // Exit 0: the native-output schema this task pins is Pier's own
        // `jobs/<timestamp>/result.json` aggregate from
        // `pier run -p`, exactly as verified at the dispatch. The
        // aggregate's structure is validated (one trial, per-metric
        // single reward per trial); the multi-metric DeepSWE reward
        // semantics are not translated into test counters - every metric
        // stays unknown rather than guessed.
        if self.profile.output_kind == OutputKind::UnpinnedPending1815 {
            return match super::process::import_pier_job_result(&request.trace_root) {
                Ok((path, reward, _value)) => (
                    reward,
                    BenchmarkCompletion::Verified {
                        metrics: super::process::NativeMetrics {
                            tests: None,
                            passed: None,
                            failed: None,
                            skipped: None,
                            pending: None,
                            other: None,
                        },
                        artifacts: vec![NativeArtifact {
                            role: "native/pier-result".to_owned(),
                            sha256: sha256_hex(&std::fs::read(&path).unwrap_or_default()),
                            path,
                        }],
                    },
                ),
                Err(_) => (
                    reward_unknown(),
                    BenchmarkCompletion::Failed(super::process::BenchmarkFailure {
                        kind: "native-output-invalid",
                        boundary: FailureBoundaryCode::Adapter,
                    }),
                ),
            };
        }
        // Synthetic wiring path only: import the Pier report the fixture
        // verifier wrote into the run's trace root. The reward is the
        // native resolved flag — authoritative, including zero.
        let report = request.trace_root.join(PIER_REPORT_NAME);
        let bytes = match std::fs::read(&report) {
            Ok(bytes) => bytes,
            Err(_) => {
                return (
                    reward_unknown(),
                    BenchmarkCompletion::Failed(super::process::BenchmarkFailure {
                        kind: "verifier-invalid-output",
                        boundary: FailureBoundaryCode::Grader,
                    }),
                );
            }
        };
        let reward = match Self::import_pier_report(&bytes) {
            Ok(reward) => Fact::Known {
                value: reward,
                origin: "pier-report".to_owned(),
            },
            Err(PierReportError::Parse) => {
                return (
                    reward_unknown(),
                    BenchmarkCompletion::Failed(super::process::BenchmarkFailure {
                        kind: "import-parse-failure",
                        boundary: FailureBoundaryCode::Adapter,
                    }),
                );
            }
            Err(PierReportError::UnsupportedSchema) => {
                return (
                    reward_unknown(),
                    BenchmarkCompletion::Failed(super::process::BenchmarkFailure {
                        kind: "import-unsupported-schema",
                        boundary: FailureBoundaryCode::Adapter,
                    }),
                );
            }
        };
        // The report stays as the content-addressed native artifact,
        // referenced in place inside the capture root.
        let artifact = NativeArtifact {
            role: "native/pier-report".to_owned(),
            sha256: sha256_hex(&bytes),
            path: report,
        };
        // DeepSWE's native surface reports the resolved flag, not a test
        // counter breakdown: every metric stays absent rather than guessed.
        (
            reward,
            BenchmarkCompletion::Verified {
                metrics: super::process::NativeMetrics::default(),
                artifacts: vec![artifact],
            },
        )
    }
}

#[cfg(test)]
mod adapter_tests {
    use super::*;
    #[cfg(unix)]
    use crate::benchmark::process::BenchmarkExecution;
    use crate::integrity::{
        IntegrityRecord, IntegrityReview, OraclePreflight, RevisionStatus, TaskClassification,
    };
    use crate::process::{CleanupEvidence, OutputCapture, SpawnReason, SupervisedOutcome};
    #[cfg(unix)]
    use tokio_util::sync::CancellationToken;

    fn fixture_path(relative: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/benchmarks/deepswe-v1.1")
            .join(relative)
    }

    fn production_profile() -> DeepSweProfile {
        DeepSweProfile::parse(
            &std::fs::read(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/profiles/benchmarks/deepswe-v1.1.toml"
            ))
            .unwrap(),
        )
        .unwrap()
    }

    fn synthetic_profile() -> DeepSweProfile {
        DeepSweProfile::parse(&std::fs::read(fixture_path("profile/synthetic.toml")).unwrap())
            .unwrap()
    }

    fn integrity_for(revision: &str, task: &str) -> IntegrityRecord {
        IntegrityRecord::review(IntegrityReview {
            benchmark: "deepswe".to_owned(),
            revision: revision.to_owned(),
            dataset: "deepswe-v1.1".to_owned(),
            grader: "pier-v0.3.1".to_owned(),
            environment: "separate-pristine-verifier".to_owned(),
            upstream_identity: "abs-module-cache-flags".to_owned(),
            upstream_digest: "0".repeat(64),
            oracle: Some(OraclePreflight::Passed("synthetic wiring".to_owned())),
            status: RevisionStatus::Admitted,
            tasks: BTreeMap::from([(task.to_owned(), TaskClassification::ValidAgentOutcome)]),
            excluded_trials: BTreeMap::new(),
            reviewer: "human-reviewer".to_owned(),
        })
        .unwrap()
    }

    fn request_with(
        adapter: &DeepSweAdapter,
        task_dir: std::path::PathBuf,
        trace_root: std::path::PathBuf,
    ) -> BenchmarkRunRequest {
        BenchmarkRunRequest {
            verifier_executable: "/nonexistent/uv".into(),
            task_dir,
            task_id: adapter.profile().task_id.clone(),
            agent_output: trace_root.join("agent-output"),
            trace_root,
            admitted_lock_digest: "a".repeat(64),
            integrity: integrity_for("v1.1", &adapter.profile().task_id),
            extra_env: BTreeMap::new(),
        }
    }

    fn outcome_exit_zero() -> SupervisedOutcome {
        SupervisedOutcome {
            exit: ExitState::Exited { code: 0 },
            stdout: OutputCapture::default(),
            stderr: OutputCapture::default(),
            cleanup: CleanupEvidence::NotRequired,
        }
    }

    /// Writes a minimal Pier `jobs/<timestamp>/result.json` aggregate with
    /// the multi-metric reward shape a DeepSWE verifier produces (F2P, P2P,
    /// partial): one trial, one eval, every metric awarding exactly one
    /// reward to exactly that trial.
    fn write_pier_job_result(trace_root: &std::path::Path) {
        let jobs = trace_root.join("jobs/2026-08-29__07-25-13");
        std::fs::create_dir_all(&jobs).unwrap();
        let result = serde_json::json!({
            "id": "0b6f6c1e-0000-4000-8000-000000000000",
            "started_at": "2026-08-29T07:23:30Z",
            "finished_at": "2026-08-29T07:25:13Z",
            "n_total_trials": 1,
            "stats": {
                "n_completed_trials": 1,
                "evals": {
                    "adhoc": {
                        "n_trials": 1,
                        "reward_stats": {
                            "reward": { "1": ["abs-module-cache-flags"] },
                            "F2P": { "1.0": ["abs-module-cache-flags"] },
                            "P2P": { "1.0": ["abs-module-cache-flags"] }
                        }
                    }
                }
            }
        });
        std::fs::write(
            jobs.join("result.json"),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
    }

    fn write_pier_report(trace_root: &std::path::Path, name: &str) {
        std::fs::copy(
            fixture_path(&format!("pier-report/{name}")),
            trace_root.join(PIER_REPORT_NAME),
        )
        .unwrap();
    }

    fn copy_fixture_package(source: &std::path::Path, target: &std::path::Path) {
        std::fs::create_dir_all(target).unwrap();
        for entry in std::fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name();
                std::fs::create_dir_all(target.join(&name)).unwrap();
                copy_fixture_package(&path, &target.join(&name));
            } else {
                let name = entry.file_name();
                std::fs::copy(&path, target.join(&name)).unwrap();
            }
        }
    }

    #[test]
    fn adapter_identity_and_admission_bind_the_pinned_revision() {
        let production = DeepSweAdapter::from_profile(production_profile());
        assert_eq!(
            production.identity(),
            BenchmarkIdentity {
                benchmark: "deepswe".to_owned(),
                revision: "v1.1".to_owned(),
                adapter: "opi-eval-deepswe-adapter/1".to_owned(),
            }
        );

        let dir = tempfile::tempdir().unwrap();
        let trace = dir.path().join("trace");
        std::fs::create_dir_all(&trace).unwrap();

        // The production DeepSWE pin is the task-18.15 registered byte
        // table: the synthetic fixture directory drifts against the real
        // official package bytes and admission fails closed as drift.
        let mut request = request_with(&production, fixture_path("task-package"), trace.clone());
        assert_eq!(
            production.admission(&request),
            Err(ExecutionError {
                token: "task-package-drift"
            })
        );

        // Revision binding: a Terminal-Bench integrity record never drives
        // this adapter.
        request.integrity = integrity_for("3.0", &production.profile().task_id);
        assert_eq!(
            production.admission(&request),
            Err(ExecutionError {
                token: "revision-binding-mismatch"
            })
        );

        // Task binding.
        let adapter = DeepSweAdapter::from_profile(synthetic_profile());
        let mut wrong_task = request_with(&adapter, fixture_path("task-package"), trace.clone());
        wrong_task.task_id = "some-other-task".to_owned();
        assert_eq!(
            adapter.admission(&wrong_task),
            Err(ExecutionError {
                token: "task-binding-mismatch"
            })
        );

        // A synthetic byte-table pin admits its byte-identical package and
        // rejects drift.
        let mut good = request_with(&adapter, fixture_path("task-package"), trace.clone());
        adapter.admission(&good).unwrap();
        let drifted = dir.path().join("drifted");
        copy_fixture_package(&fixture_path("task-package"), &drifted);
        std::fs::write(drifted.join("README.md"), b"tampered").unwrap();
        good.task_dir = drifted;
        assert_eq!(
            adapter.admission(&good),
            Err(ExecutionError {
                token: "task-package-drift"
            })
        );
    }

    #[test]
    fn spawn_spec_builds_the_pinned_pier_argv() {
        let adapter = DeepSweAdapter::from_profile(synthetic_profile());
        let dir = tempfile::tempdir().unwrap();
        let trace = dir.path().join("trace");
        std::fs::create_dir_all(&trace).unwrap();
        let request = request_with(&adapter, fixture_path("task-package"), trace.clone());
        let spec = adapter.spawn_spec(&request);
        assert_eq!(
            spec.argv,
            vec![
                "/nonexistent/uv".into(),
                "run".into(),
                "--locked".into(),
                "pier".into(),
                "run".into(),
                "-p".into(),
                fixture_path("task-package"),
            ]
        );
        assert_eq!(spec.cwd.as_deref(), Some(trace.as_path()));
        assert_eq!(spec.stdout_cap, 1048576);
        assert_eq!(spec.timeout, Duration::from_secs(1800));
        assert!(spec.env.is_empty());
        // The pristine no-network collected-patch semantics stay pinned on
        // the profile, not normalized into the neutral spawn. The mutable
        // agent worktree is never the graded subject.
        assert_eq!(
            (adapter.profile().cpus, adapter.profile().memory_gib),
            (2, 8)
        );
    }

    #[test]
    fn settle_matrix_maps_unpinned_and_pier_report_outputs() {
        let adapter = DeepSweAdapter::from_profile(synthetic_profile());
        let dir = tempfile::tempdir().unwrap();
        let trace = dir.path().join("trace");
        std::fs::create_dir_all(&trace).unwrap();
        let request = request_with(&adapter, fixture_path("task-package"), trace.clone());

        let failed = |kind: &'static str, boundary| {
            BenchmarkCompletion::Failed(super::super::process::BenchmarkFailure { kind, boundary })
        };

        // Missing report on exit 0: grader-side invalid output.
        assert_eq!(
            adapter.settle(&outcome_exit_zero(), &request).1,
            failed("verifier-invalid-output", FailureBoundaryCode::Grader)
        );
        // Corrupt JSON.
        write_pier_report(&trace, "corrupt.json");
        assert_eq!(
            adapter.settle(&outcome_exit_zero(), &request).1,
            failed("import-parse-failure", FailureBoundaryCode::Adapter)
        );
        // Schema drift: extra fields or a non-0/1 reward are not admitted.
        write_pier_report(&trace, "drift.json");
        assert_eq!(
            adapter.settle(&outcome_exit_zero(), &request).1,
            failed("import-unsupported-schema", FailureBoundaryCode::Adapter)
        );

        // A zero reward is an authoritative native verdict — a valid
        // Verified completion, never a failure.
        write_pier_report(&trace, "zero.json");
        let (reward, completion) = adapter.settle(&outcome_exit_zero(), &request);
        assert_eq!(
            reward,
            Fact::Known {
                value: 0,
                origin: "pier-report".to_owned()
            }
        );
        assert!(matches!(completion, BenchmarkCompletion::Verified { .. }));
        // DeepSWE's native surface is a resolved flag, not a test counter
        // breakdown: metrics stay absent rather than guessed.
        let BenchmarkCompletion::Verified { metrics, .. } = completion else {
            unreachable!()
        };
        assert_eq!(metrics, super::super::process::NativeMetrics::default());

        // Resolved reward 1 settles the same shape.
        write_pier_report(&trace, "resolved.json");
        let (reward, completion) = adapter.settle(&outcome_exit_zero(), &request);
        assert_eq!(
            reward,
            Fact::Known {
                value: 1,
                origin: "pier-report".to_owned()
            }
        );
        let BenchmarkCompletion::Verified { artifacts, .. } = completion else {
            unreachable!()
        };
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].role, "native/pier-report");
        let bytes = std::fs::read(&artifacts[0].path).unwrap();
        assert_eq!(artifacts[0].sha256, sha256_hex(&bytes));

        // The production DeepSWE profile pins Pier's own job-result
        // aggregate: a structurally valid multi-metric result settles
        // Verified with the result file as the content-addressed
        // artifact, and every metric stays unknown - the multi-metric
        // reward semantics are never guessed into test counters.
        let production = DeepSweAdapter::from_profile(production_profile());
        let native_trace = tempfile::tempdir().unwrap();
        write_pier_job_result(native_trace.path());
        let mut production_request = request_with(
            &production,
            fixture_path("task-package"),
            native_trace.path().to_path_buf(),
        );
        production_request.integrity = integrity_for("v1.1", &production.profile().task_id);
        production_request.task_id = production.profile().task_id.clone();
        let (reward, completion) = production.settle(&outcome_exit_zero(), &production_request);
        assert_eq!(
            reward,
            Fact::Known {
                value: 1,
                origin: "pier-result".to_owned()
            }
        );
        let BenchmarkCompletion::Verified { metrics, artifacts } = completion else {
            unreachable!()
        };
        assert_eq!(metrics, super::super::process::NativeMetrics::default());
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].role, "native/pier-result");
        let bytes = std::fs::read(&artifacts[0].path).unwrap();
        assert_eq!(artifacts[0].sha256, sha256_hex(&bytes));

        // No pier job result anywhere under the trace root: fail closed
        // (boundary Adapter: the importer cannot admit an output shape).
        let empty_trace = tempfile::tempdir().unwrap();
        let mut missing_request = request_with(
            &production,
            fixture_path("task-package"),
            empty_trace.path().to_path_buf(),
        );
        missing_request.integrity = integrity_for("v1.1", &production.profile().task_id);
        missing_request.task_id = production.profile().task_id.clone();
        assert_eq!(
            production.settle(&outcome_exit_zero(), &missing_request).1,
            failed("native-output-invalid", FailureBoundaryCode::Adapter)
        );

        // Process-level verdicts are authoritative and never rescued.
        assert_eq!(
            adapter
                .settle(
                    &SupervisedOutcome {
                        exit: ExitState::Exited { code: 3 },
                        ..outcome_exit_zero()
                    },
                    &request
                )
                .1,
            failed("verifier-non-zero-exit", FailureBoundaryCode::Grader)
        );
        for exit in [
            ExitState::TimedOut,
            ExitState::Cancelled,
            ExitState::FailedToSpawn {
                reason: SpawnReason::NotFound,
            },
        ] {
            let outcome = SupervisedOutcome {
                exit,
                ..outcome_exit_zero()
            };
            assert!(matches!(
                adapter.settle(&outcome, &request).1,
                BenchmarkCompletion::Failed(f) if f.boundary != FailureBoundaryCode::Experiment
            ));
        }
    }

    #[cfg(unix)]
    fn fake_verifier_script(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
        let script = dir.join(name);
        std::fs::write(&script, format!("#!/bin/sh\n{body}\nexit 0\n")).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script
    }

    #[cfg(unix)]
    fn e2e_request(
        dir: &std::path::Path,
        executable: std::path::PathBuf,
        report: &str,
    ) -> BenchmarkRunRequest {
        let adapter = DeepSweAdapter::from_profile(synthetic_profile());
        let trace = dir.join("trace");
        std::fs::create_dir_all(&trace).unwrap();
        let mut request = request_with(&adapter, fixture_path("task-package"), trace);
        request.verifier_executable = executable;
        request.extra_env.insert(
            std::ffi::OsString::from("OPI_EVAL_PIER_REPORT_SOURCE"),
            std::fs::canonicalize(fixture_path(&format!("pier-report/{report}")))
                .unwrap()
                .into(),
        );
        request
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn fake_verifier_settles_verified_records_end_to_end() {
        let adapter = DeepSweAdapter::from_profile(synthetic_profile());
        let dir = tempfile::tempdir().unwrap();
        let script = fake_verifier_script(
            dir.path(),
            "fake-verifier",
            "cp \"$OPI_EVAL_PIER_REPORT_SOURCE\" ./pier-report.json",
        );
        let request = e2e_request(dir.path(), script, "resolved.json");

        let record = BenchmarkExecution::run(&request, &adapter, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(record.exit, ExitState::Exited { code: 0 });
        assert!(matches!(
            record.completion,
            BenchmarkCompletion::Verified { .. }
        ));
        assert_eq!(
            record.reward,
            Fact::Known {
                value: 1,
                origin: "pier-report".to_owned()
            }
        );
        assert_eq!(
            record.provenance.revision, "v1.1",
            "DeepSWE keeps its native revision naming"
        );
        assert_eq!(record.provenance.task_id, "synthetic-fixture-task");

        // Zero reward is equally authoritative end to end.
        let dir2 = tempfile::tempdir().unwrap();
        let script2 = fake_verifier_script(
            dir2.path(),
            "fake-verifier",
            "cp \"$OPI_EVAL_PIER_REPORT_SOURCE\" ./pier-report.json",
        );
        let request2 = e2e_request(dir2.path(), script2, "zero.json");
        let record2 = BenchmarkExecution::run(&request2, &adapter, &CancellationToken::new())
            .await
            .unwrap();
        assert!(matches!(
            record2.completion,
            BenchmarkCompletion::Verified { .. }
        ));
        assert_eq!(
            record2.reward,
            Fact::Known {
                value: 0,
                origin: "pier-report".to_owned()
            }
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn production_pin_never_reaches_a_spawn_through_the_shared_contract() {
        let adapter = DeepSweAdapter::from_profile(production_profile());
        let dir = tempfile::tempdir().unwrap();
        let script = fake_verifier_script(dir.path(), "fake-verifier", "exit 0");
        let trace = dir.path().join("trace");
        std::fs::create_dir_all(&trace).unwrap();
        let mut request = request_with(&adapter, fixture_path("task-package"), trace);
        request.verifier_executable = script;

        // The registered byte table rejects the synthetic fixture bytes
        // before any verifier process exists, even with a runnable
        // executable handed to it.
        assert_eq!(
            BenchmarkExecution::run(&request, &adapter, &CancellationToken::new())
                .await
                .unwrap_err(),
            ExecutionError {
                token: "task-package-drift"
            }
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn fake_verifier_exit_and_spawn_failures_settle_typed() {
        let adapter = DeepSweAdapter::from_profile(synthetic_profile());
        let dir = tempfile::tempdir().unwrap();
        let script = fake_verifier_script(
            dir.path(),
            "fake-verifier-fail",
            "cp \"$OPI_EVAL_PIER_REPORT_SOURCE\" ./pier-report.json\nexit 5",
        );
        let request = e2e_request(dir.path(), script, "resolved.json");

        let record = BenchmarkExecution::run(&request, &adapter, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(record.exit, ExitState::Exited { code: 5 });
        assert_eq!(
            record.completion,
            BenchmarkCompletion::Failed(failure_kinds::non_zero_exit(5))
        );

        // A nonexistent verifier settles as a redacted infrastructure spawn
        // failure.
        let mut missing = request.clone();
        missing.verifier_executable = dir.path().join("no-such-verifier");
        let record = BenchmarkExecution::run(&missing, &adapter, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(
            record.completion,
            BenchmarkCompletion::Failed(failure_kinds::spawn(SpawnReason::NotFound))
        );
        assert!(!format!("{record:?}").contains("no-such-verifier"));
    }
}
