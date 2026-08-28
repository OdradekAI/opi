//! Crate-private Terminal-Bench 3.0 benchmark adapter (Phase 18 task 18.9).
//!
//! Owns only what is Terminal-Bench-3.0-specific: the pinned declarative
//! profile (tag `v3.0.0` of canonical `harbor-framework/terminal-bench`),
//! the separate-verifier-container semantics that must not be normalized
//! away (`P18-BMK-003`), task-package admission for the two pin modes the
//! committed evidence supports, and the revision-specific native-result
//! settlement through the shared contract in [`super::process`]. 3.0 is a
//! separate revision, not a data revision of 2.1: byte-level task-package
//! tables, task/verifier images, and the native-output schema are
//! unresolved slots owned by task 18.15, so the production profile pins the
//! task-tree identity only and the adapter fails closed until reviewed
//! bytes are registered (`P18-BMK-001`, `P18-BMK-002`, `P18-BMK-008`).

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
/// lock pins Terminal-Bench 3.0 by task-tree identity with an empty files
/// table (`tree-identity`); a `byte-table` pin requires a registered
/// complete file closure, which today exists only for synthetic fixtures
/// and arrives for the production task with task 18.15.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PackagePin {
    /// Task-tree identity only: admission fails closed as not materialized.
    TreeIdentity,
    /// Complete registered file table: exact-set, exact-bytes admission.
    ByteTable,
}

/// The pinned native-output surface. No 3.0 native-output schema is
/// committed; `ctrf-json` is admitted only for synthetic wiring fixtures
/// until task 18.15 pins the real schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputKind {
    UnpinnedPending1815,
    CtrfJson,
}

/// The parsed declarative Terminal-Bench 3.0 profile: pinned identity,
/// package pin, the native verifier entry with separate-verifier semantics,
/// and limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tb30Profile {
    pub tag: String,
    pub source_commit: String,
    pub tasks_tree: String,
    pub task_id: String,
    pub task_tree: String,
    pub package_pin: PackagePin,
    pub package_manifest_sha256: String,
    pub adapter: String,
    pub verifier_workdir: String,
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
    /// A pinned invariant drifted (unknown runner/lifecycle/output kind,
    /// malformed launch template, bad digests, duplicate/empty package rows,
    /// pin-mode contradictions).
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
struct Tb30Doc {
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
    tag: String,
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
    launch: Vec<String>,
    lifecycle: String,
    verifier_workdir: String,
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

impl Tb30Profile {
    /// Parse and fully validate a declarative profile document.
    ///
    /// Fails closed on an unsupported schema, a non-Terminal-Bench-3.0
    /// identity, a missing tag, a non-pinned runner/lifecycle/artifact
    /// policy/output kind, a malformed launch template, bad digests, and any
    /// pin-mode contradiction between `package_pin` and the file table.
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, TbProfileError> {
        let text = String::from_utf8(bytes.to_vec())
            .map_err(|_| TbProfileError::Parse("profile is not valid UTF-8".to_owned()))?;
        let doc: Tb30Doc =
            toml::from_str(&text).map_err(|e| TbProfileError::Parse(e.to_string()))?;
        if doc.schema != "phase18-benchmark-profile/1" {
            return Err(TbProfileError::UnsupportedSchema(doc.schema));
        }
        if doc.benchmark != "terminal-bench" || doc.revision != "3.0" {
            return Err(TbProfileError::Drift(format!(
                "profile is {} {}, not terminal-bench 3.0",
                doc.benchmark, doc.revision
            )));
        }
        if doc.verifier.runner_kind != "harbor" {
            return Err(TbProfileError::Drift(format!(
                "runner_kind {} is not the pinned harbor surface",
                doc.verifier.runner_kind
            )));
        }
        // The 3.0 verifier semantics are pinned, not normalized away: a
        // separate verifier container declaring its own workdir, receiving
        // only declared artifacts at their original paths.
        if doc.verifier.lifecycle != "separate-container" {
            return Err(TbProfileError::Drift(format!(
                "lifecycle {} is not the pinned separate-container verifier",
                doc.verifier.lifecycle
            )));
        }
        if !doc.verifier.verifier_workdir.starts_with('/') {
            return Err(TbProfileError::Drift(format!(
                "verifier_workdir {:?} is not an absolute container path",
                doc.verifier.verifier_workdir
            )));
        }
        if doc.verifier.artifact_policy != "declared-original-paths" {
            return Err(TbProfileError::Drift(format!(
                "artifact_policy {} is not the pinned declared-original-paths surface",
                doc.verifier.artifact_policy
            )));
        }
        let output_kind = match doc.verifier.output_kind.as_str() {
            "unpinned-pending-18-15" => OutputKind::UnpinnedPending1815,
            "ctrf-json" => OutputKind::CtrfJson,
            other => {
                return Err(TbProfileError::Drift(format!(
                    "output_kind {other} is not an admitted kind"
                )));
            }
        };
        // The launch template must use the `<task-dir>` placeholder exactly
        // once and no other templated slot; everything else is pinned argv.
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
        // Unlike 2.1, 3.0 has an immutable tag; both tag and commit are the
        // version lock.
        if !doc.identity.tag.starts_with('v') || doc.identity.tag.len() < 2 {
            return Err(TbProfileError::Drift(format!(
                "tag {:?} is not a v-prefixed version tag",
                doc.identity.tag
            )));
        }
        for value in [
            &doc.identity.source_commit,
            &doc.identity.tasks_tree,
            &doc.identity.task_tree,
        ] {
            if value.len() != 40 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(TbProfileError::Drift(format!(
                    "identity {value} is not a 40-hex Git identity"
                )));
            }
        }
        if !is_sha256_hex(&doc.identity.package_manifest_sha256) {
            return Err(TbProfileError::Drift(
                "package_manifest_sha256 is not a lowercase 64-hex digest".to_owned(),
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
            tag: doc.identity.tag,
            source_commit: doc.identity.source_commit,
            tasks_tree: doc.identity.tasks_tree,
            task_id: doc.identity.task_id,
            task_tree: doc.identity.task_tree,
            package_pin,
            package_manifest_sha256: doc.identity.package_manifest_sha256,
            adapter: doc.identity.adapter,
            verifier_workdir: doc.verifier.verifier_workdir,
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
    /// no 3.0 verifier invocation is admitted until task 18.15 registers
    /// one. A `byte-table` pin requires exactly the pinned files,
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
            .join("tests/fixtures/benchmarks/terminal-bench-3.0")
            .join(relative)
    }

    fn synthetic_bytes() -> Vec<u8> {
        read(&fixture("profile/synthetic.toml"))
    }

    #[test]
    fn production_profile_is_pinned_to_the_static_external_lock() {
        // P18-BMK-001/P18-BMK-008: the profile must restate exactly the
        // identity pins the committed static lock already admits for 3.0 —
        // and only those: the lock's files table is empty, so the profile
        // must pin the task-tree identity without inventing bytes.
        let profile = Tb30Profile::parse(&read(
            &crate_root().join("profiles/benchmarks/terminal-bench-3.0.toml"),
        ))
        .unwrap();
        assert_eq!(profile.tag, "v3.0.0");
        assert_eq!(
            profile.source_commit,
            "2b0442c3c583b710ca8da14c8e601b99f2f1f244"
        );
        assert_eq!(
            profile.tasks_tree,
            "a10dbfde7cd4d1c3ceaf22ed39b52a98dc775d54"
        );
        assert_eq!(profile.task_id, "batched-eval-parity");
        assert_eq!(
            profile.task_tree,
            "729916599199dd1de6be0e0c543da9788d6129b5"
        );
        assert_eq!(profile.package_pin, PackagePin::TreeIdentity);
        assert!(profile.package.is_empty());
        assert_eq!(
            profile.canonical_package_digest(),
            profile.package_manifest_sha256
        );
        assert_eq!(profile.output_kind, OutputKind::UnpinnedPending1815);
        assert_eq!(profile.verifier_workdir, "/app/evalbench");
        assert_eq!(profile.launch, vec!["run", "-p", "<task-dir>"]);
        assert_eq!((profile.cpus, profile.memory_gib), (1, 4));

        let lock: serde_json::Value = serde_json::from_slice(&read(
            &crate_root().join("external-locks/static/linux-x86_64.json"),
        ))
        .unwrap();
        let subject = &lock["subjects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == "terminal-bench-3.0")
            .unwrap();
        assert_eq!(subject["tag"].as_str().unwrap(), profile.tag);
        assert_eq!(subject["commit"].as_str().unwrap(), profile.source_commit);
        assert_eq!(subject["tasks_tree"].as_str().unwrap(), profile.tasks_tree);
        assert_eq!(subject["task"]["id"].as_str().unwrap(), profile.task_id);
        assert_eq!(subject["task"]["tree"].as_str().unwrap(), profile.task_tree);
        assert_eq!(
            subject["task"]["files"].as_array().map(Vec::len),
            Some(0),
            "the lock pins 3.0 by tree identity; a byte table would be unregistered bytes"
        );
    }

    #[test]
    fn synthetic_profile_accepts_its_byte_table_package() {
        let profile = Tb30Profile::parse(&synthetic_bytes()).unwrap();
        assert_eq!(profile.package_pin, PackagePin::ByteTable);
        assert_eq!(profile.output_kind, OutputKind::CtrfJson);
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
        let profile = Tb30Profile::parse(&synthetic_bytes()).unwrap();
        let dir = tempfile::tempdir().unwrap();

        // Missing pinned file.
        let missing = dir.path().join("missing");
        copy_package(&fixture("task-package"), &missing);
        std::fs::remove_file(missing.join("tests/verify.sh")).unwrap();
        assert_eq!(
            profile.admit_task_package(&missing),
            Err(TaskPackageError::MissingFile {
                path: "tests/verify.sh".to_owned()
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

        // A tree-identity pin is never materializable: no committed byte
        // table exists to validate against, so nothing admits.
        let production = Tb30Profile::parse(&read(
            &crate_root().join("profiles/benchmarks/terminal-bench-3.0.toml"),
        ))
        .unwrap();
        assert_eq!(
            production.admit_task_package(&fixture("task-package")),
            Err(TaskPackageError::NotMaterialized)
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

        let reparse = |text: &str| Tb30Profile::parse(text.as_bytes());

        // Wrong schema family.
        assert!(matches!(
            reparse(&text.replace("phase18-benchmark-profile/1", "phase18-benchmark-profile/2")),
            Err(TbProfileError::UnsupportedSchema(_))
        ));

        // A 2.1-style data revision is not this adapter.
        assert!(matches!(
            reparse(&text.replace("revision = \"3.0\"", "revision = \"2.1\"")),
            Err(TbProfileError::Drift(_))
        ));

        // Unknown keys are rejected (deny_unknown_fields): a dropped 3.0
        // semantic like the separate-container lifecycle is a parse failure,
        // not a silent default.
        assert!(matches!(
            reparse(&text.replace("lifecycle = \"separate-container\"", "")),
            Err(TbProfileError::Parse(_))
        ));
        assert!(matches!(
            reparse(&text.replace(
                "lifecycle = \"separate-container\"",
                "lifecycle = \"shared\""
            )),
            Err(TbProfileError::Drift(_))
        ));

        // Tag drift: 3.0's immutable tag is part of the lock.
        assert!(matches!(
            reparse(&text.replace("tag = \"v0.0.0-synthetic\"", "tag = \"main\"")),
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
        // and a byte-table pin whose manifest digest does not cover it.
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
            let flipped = text.clone();
            let declared = flipped
                .lines()
                .find_map(|l| l.strip_prefix("package_manifest_sha256 = "))
                .unwrap()
                .trim_matches('"')
                .to_owned();
            let mut chars = declared.chars();
            let first = chars.next().unwrap();
            let flipped_first = if first == '5' { '4' } else { '5' };
            let replacement = format!(
                "package_manifest_sha256 = \"{flipped_first}{}\"",
                chars.as_str()
            );
            flipped.replace(
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
            reparse(&text.replace("output_kind = \"ctrf-json\"", "output_kind = \"jsonl\"")),
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
// Adapter: benchmark-neutral contract implementation for Terminal-Bench 3.0.

use super::process::{
    BenchmarkAdapter, BenchmarkCompletion, BenchmarkIdentity, BenchmarkRunRequest, ExecutionError,
    failure_kinds,
};
use crate::agent::process::{Fact, NativeArtifact};
use crate::failure::FailureBoundaryCode;
use crate::process::{ExitState, SpawnSpec};
use std::time::Duration;

/// Where a synthetic `ctrf-json` wiring fixture expects the native report.
/// The production 3.0 profile pins no output schema, so no filename is
/// claimed for real 3.0 runs until task 18.15 registers one.
const CTRF_REPORT_NAME: &str = "ctrf-report.json";

/// The Terminal-Bench 3.0 benchmark adapter. Owns the pinned declarative
/// profile, the tree-identity/byte-table package admission, the exact
/// Harbor v0.22.0 launch through `uv run --locked` under separate-verifier
/// semantics, and the revision-specific native-result settlement.
pub(crate) struct TerminalBench30Adapter {
    profile: Tb30Profile,
}

impl TerminalBench30Adapter {
    /// Build the adapter from a validated declarative profile.
    pub(crate) fn from_profile(profile: Tb30Profile) -> Self {
        Self { profile }
    }

    /// The profile this adapter pins.
    pub(crate) fn profile(&self) -> &Tb30Profile {
        &self.profile
    }

    /// Import the CTRF schema used by synthetic wiring fixtures. This is a
    /// wiring hypothesis, not a pinned 3.0 native-output schema: the real
    /// importer shape is registered by task 18.15. Fails closed on drift.
    fn import_ctrf(bytes: &[u8]) -> Result<super::process::NativeMetrics, CtrfError> {
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| CtrfError::Parse)?;
        let Some(fields) = value
            .as_object()
            .and_then(|o| o.get("results"))
            .and_then(|r| r.as_object())
        else {
            return Err(CtrfError::UnsupportedSchema);
        };
        if value.as_object().map(|o| o.len()) != Some(1) {
            return Err(CtrfError::UnsupportedSchema);
        }
        for key in fields.keys() {
            if !matches!(
                key.as_str(),
                "tool" | "summary" | "tests" | "environment" | "extra"
            ) {
                return Err(CtrfError::UnsupportedSchema);
            }
        }
        let Some(summary) = fields.get("summary").and_then(|s| s.as_object()) else {
            return Err(CtrfError::UnsupportedSchema);
        };
        // Closed summary vocabulary: the six counters plus start/stop
        // timestamps, all integers. Added or missing keys fail closed.
        const SUMMARY_KEYS: [&str; 8] = [
            "tests", "passed", "failed", "skipped", "pending", "other", "start", "stop",
        ];
        if summary.len() != SUMMARY_KEYS.len()
            || SUMMARY_KEYS
                .iter()
                .any(|key| !summary.get(*key).is_some_and(|v| v.is_u64()))
        {
            return Err(CtrfError::UnsupportedSchema);
        }
        let counter = |key: &str| -> Option<Fact> {
            summary
                .get(key)
                .and_then(|v| v.as_u64())
                .map(|value| Fact::Known {
                    value,
                    origin: "ctrf-summary".to_owned(),
                })
        };
        Ok(super::process::NativeMetrics {
            tests: counter("tests"),
            passed: counter("passed"),
            failed: counter("failed"),
            skipped: counter("skipped"),
            pending: counter("pending"),
            other: counter("other"),
        })
    }
}

/// Typed CTRF import failures (wiring-fixture surface).
enum CtrfError {
    Parse,
    UnsupportedSchema,
}

impl BenchmarkAdapter for TerminalBench30Adapter {
    fn identity(&self) -> BenchmarkIdentity {
        BenchmarkIdentity {
            benchmark: "terminal-bench".to_owned(),
            revision: "3.0".to_owned(),
            adapter: self.profile.adapter.clone(),
        }
    }

    fn admission(&self, request: &BenchmarkRunRequest) -> Result<(), ExecutionError> {
        // Revision binding: the integrity record must admit exactly the
        // revision this adapter pins, and the requested task must be the
        // profile's pinned task.
        if request.integrity.revision() != "3.0" {
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
        // materialized: 3.0 verifier runs are not admitted until task
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
        // Pinned evidence: Harbor v0.22.0 is driven through
        // `uv run --locked harbor run -p <task-dir>`; argv[0] is the
        // resolved uv executable, never an ambient PATH lookup. The
        // separate verifier container lifecycle is Harbor's own, pinned in
        // the profile and not normalized away here.
        let mut argv: Vec<std::ffi::OsString> = vec![
            request.verifier_executable.clone().into(),
            "run".into(),
            "--locked".into(),
            "harbor".into(),
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
        // Exit 0 under an unpinned output schema settles fail-closed
        // unless task 18.15's pinned harbor layout is present: the 3.0
        // native-output schema this task pins is harbor's
        // `jobs/<timestamp>/result.json` aggregate from `harbor run -p`,
        // exactly as verified at the dispatch. Nothing is guessed or
        // normalized beyond that aggregate.
        if self.profile.output_kind == OutputKind::UnpinnedPending1815 {
            return match super::process::import_harbor_result(&request.trace_root) {
                Ok((metrics, path, _value)) => (
                    reward_unknown(),
                    BenchmarkCompletion::Verified {
                        metrics,
                        artifacts: vec![NativeArtifact {
                            role: "native/harbor-result".to_owned(),
                            sha256: sha256_hex(&std::fs::read(&path).unwrap_or_default()),
                            path,
                        }],
                    },
                ),
                Err(_) => (
                    reward_unknown(),
                    BenchmarkCompletion::Failed(super::process::BenchmarkFailure {
                        kind: "native-output-schema-unpinned",
                        boundary: FailureBoundaryCode::Adapter,
                    }),
                ),
            };
        }
        // Synthetic wiring path only: import the CTRF report the fixture
        // verifier wrote into the run's trace root.
        let report = request.trace_root.join(CTRF_REPORT_NAME);
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
        let metrics = match Self::import_ctrf(&bytes) {
            Ok(metrics) => metrics,
            Err(CtrfError::Parse) => {
                return (
                    reward_unknown(),
                    BenchmarkCompletion::Failed(super::process::BenchmarkFailure {
                        kind: "import-parse-failure",
                        boundary: FailureBoundaryCode::Adapter,
                    }),
                );
            }
            Err(CtrfError::UnsupportedSchema) => {
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
            role: "native/ctrf-report".to_owned(),
            sha256: sha256_hex(&bytes),
            path: report,
        };
        (
            reward_unknown(),
            BenchmarkCompletion::Verified {
                metrics,
                artifacts: vec![artifact],
            },
        )
    }
}

#[cfg(test)]
mod adapter_tests {
    use super::*;
    use crate::benchmark::process::{BenchmarkExecution, BenchmarkProvenance, NativeMetrics};
    use crate::integrity::{
        IntegrityRecord, IntegrityReview, OraclePreflight, RevisionStatus, TaskClassification,
    };
    use crate::process::{CleanupEvidence, OutputCapture, SpawnReason, SupervisedOutcome};
    use tokio_util::sync::CancellationToken;

    fn fixture_path(relative: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/benchmarks/terminal-bench-3.0")
            .join(relative)
    }

    fn production_profile() -> Tb30Profile {
        Tb30Profile::parse(
            &std::fs::read(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/profiles/benchmarks/terminal-bench-3.0.toml"
            ))
            .unwrap(),
        )
        .unwrap()
    }

    fn synthetic_profile() -> Tb30Profile {
        Tb30Profile::parse(&std::fs::read(fixture_path("profile/synthetic.toml")).unwrap()).unwrap()
    }

    fn integrity_for(revision: &str, task: &str) -> IntegrityRecord {
        IntegrityRecord::review(IntegrityReview {
            benchmark: "terminal-bench".to_owned(),
            revision: revision.to_owned(),
            dataset: "terminal-bench-v3.0.0".to_owned(),
            grader: "harbor-v0.22.0".to_owned(),
            environment: "separate-verifier-container".to_owned(),
            upstream_identity: "batched-eval-parity".to_owned(),
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
        adapter: &TerminalBench30Adapter,
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
            integrity: integrity_for("3.0", &adapter.profile().task_id),
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

    fn write_ctrf(trace_root: &std::path::Path, name: &str) {
        std::fs::copy(
            fixture_path(&format!("ctrf/{name}")),
            trace_root.join(CTRF_REPORT_NAME),
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
        let production = TerminalBench30Adapter::from_profile(production_profile());
        assert_eq!(
            production.identity(),
            BenchmarkIdentity {
                benchmark: "terminal-bench".to_owned(),
                revision: "3.0".to_owned(),
                adapter: "opi-eval-terminal-bench-30-adapter/1".to_owned(),
            }
        );

        let dir = tempfile::tempdir().unwrap();
        let trace = dir.path().join("trace");
        std::fs::create_dir_all(&trace).unwrap();

        // The production 3.0 pin is tree-identity only: admission fails
        // closed as not materialized even for a byte-perfect directory,
        // because no committed byte table exists to validate against
        // (P18-BMK-002; the real table arrives with task 18.15).
        let mut request = request_with(&production, fixture_path("task-package"), trace.clone());
        assert_eq!(
            production.admission(&request),
            Err(ExecutionError {
                token: "task-package-not-materialized"
            })
        );

        // Revision binding: a 2.1 integrity record never drives this
        // adapter (3.0 is not a data revision of 2.1).
        request.integrity = integrity_for("2.1", &production.profile().task_id);
        assert_eq!(
            production.admission(&request),
            Err(ExecutionError {
                token: "revision-binding-mismatch"
            })
        );

        // Task binding.
        let adapter = TerminalBench30Adapter::from_profile(synthetic_profile());
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
    fn spawn_spec_builds_the_pinned_harbor_argv() {
        let adapter = TerminalBench30Adapter::from_profile(synthetic_profile());
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
                "harbor".into(),
                "run".into(),
                "-p".into(),
                fixture_path("task-package"),
            ]
        );
        assert_eq!(spec.cwd.as_deref(), Some(trace.as_path()));
        assert_eq!(spec.stdout_cap, 1048576);
        assert_eq!(spec.timeout, Duration::from_secs(900));
        assert!(spec.env.is_empty());
        // The separate-verifier semantics stay pinned on the profile, not
        // normalized into the neutral spawn.
        assert_eq!(adapter.profile().verifier_workdir, "/app/evalbench");
        assert_eq!(
            (adapter.profile().cpus, adapter.profile().memory_gib),
            (1, 4)
        );
    }

    #[test]
    fn settle_matrix_maps_unpinned_and_ctrf_outputs() {
        let adapter = TerminalBench30Adapter::from_profile(synthetic_profile());
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
        write_ctrf(&trace, "corrupt.json");
        assert_eq!(
            adapter.settle(&outcome_exit_zero(), &request).1,
            failed("import-parse-failure", FailureBoundaryCode::Adapter)
        );
        // Valid CTRF with failing tests is still a valid verification: the
        // native counts stay authoritative (P18-BMK-007).
        write_ctrf(&trace, "one-failed.json");
        let (reward, completion) = adapter.settle(&outcome_exit_zero(), &request);
        assert_eq!(
            reward,
            Fact::Unknown {
                reason: "native-reward-pending-18-15-smoke".to_owned()
            }
        );
        let BenchmarkCompletion::Verified { metrics, artifacts } = completion else {
            panic!("expected verified, got {completion:?}");
        };
        assert_eq!(
            metrics.failed,
            Some(Fact::Known {
                value: 1,
                origin: "ctrf-summary".to_owned()
            })
        );
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].role, "native/ctrf-report");
        let bytes = std::fs::read(&artifacts[0].path).unwrap();
        assert_eq!(artifacts[0].sha256, sha256_hex(&bytes));

        // The production 3.0 profile has no pinned output schema: even a
        // byte-perfect CTRF-shaped file settles fail-closed instead of
        // being guessed at (boundary Adapter: the importer lacks an
        // admitted schema).
        let production = TerminalBench30Adapter::from_profile(production_profile());
        let mut production_request =
            request_with(&production, fixture_path("task-package"), trace.clone());
        production_request.integrity = integrity_for("3.0", &production.profile().task_id);
        production_request.task_id = production.profile().task_id.clone();
        assert_eq!(
            production
                .settle(&outcome_exit_zero(), &production_request)
                .1,
            failed(
                "native-output-schema-unpinned",
                FailureBoundaryCode::Adapter
            )
        );

        // Process-level verdicts are authoritative and never rescued.
        for exit in [
            ExitState::Exited { code: 3 },
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
            let completion = adapter.settle(&outcome, &request).1;
            assert!(matches!(
                completion,
                BenchmarkCompletion::Failed(f) if f.boundary != FailureBoundaryCode::Experiment
            ));
        }
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
    fn e2e_request(dir: &std::path::Path, executable: std::path::PathBuf) -> BenchmarkRunRequest {
        let adapter = TerminalBench30Adapter::from_profile(synthetic_profile());
        let trace = dir.join("trace");
        std::fs::create_dir_all(&trace).unwrap();
        let mut request = request_with(&adapter, fixture_path("task-package"), trace);
        request.verifier_executable = executable;
        request.extra_env.insert(
            std::ffi::OsString::from("OPI_EVAL_CTRF_SOURCE"),
            std::fs::canonicalize(fixture_path("ctrf/ok-six-passed.json"))
                .unwrap()
                .into(),
        );
        request
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn fake_verifier_settles_a_verified_record_end_to_end() {
        let adapter = TerminalBench30Adapter::from_profile(synthetic_profile());
        let dir = tempfile::tempdir().unwrap();
        let script = fake_verifier_script(
            dir.path(),
            "fake-verifier",
            "cp \"$OPI_EVAL_CTRF_SOURCE\" ./ctrf-report.json",
        );
        let request = e2e_request(dir.path(), script);

        let record = BenchmarkExecution::run(&request, &adapter, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(record.exit, ExitState::Exited { code: 0 });
        let BenchmarkCompletion::Verified { metrics, artifacts } = &record.completion else {
            panic!("expected verified, got {:?}", record.completion);
        };
        assert_eq!(
            metrics.tests,
            Some(Fact::Known {
                value: 6,
                origin: "ctrf-summary".to_owned()
            })
        );
        assert_eq!(artifacts[0].role, "native/ctrf-report");
        assert_eq!(
            record.provenance,
            BenchmarkProvenance {
                admitted_lock_digest: "a".repeat(64),
                integrity_digest: request.integrity.identity_digest().to_owned(),
                revision: "3.0".to_owned(),
                task_id: "synthetic-fixture-task".to_owned(),
            }
        );
        assert_eq!(
            record.reward,
            Fact::Unknown {
                reason: "native-reward-pending-18-15-smoke".to_owned()
            }
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn production_pin_never_reaches_a_spawn_through_the_shared_contract() {
        let adapter = TerminalBench30Adapter::from_profile(production_profile());
        let dir = tempfile::tempdir().unwrap();
        let script = fake_verifier_script(dir.path(), "fake-verifier", "exit 0");
        let trace = dir.path().join("trace");
        std::fs::create_dir_all(&trace).unwrap();
        let mut request = request_with(&adapter, fixture_path("task-package"), trace);
        request.verifier_executable = script;

        // The tree-identity production pin rejects before any verifier
        // process exists, even with a runnable executable handed to it.
        assert_eq!(
            BenchmarkExecution::run(&request, &adapter, &CancellationToken::new())
                .await
                .unwrap_err(),
            ExecutionError {
                token: "task-package-not-materialized"
            }
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn fake_verifier_exit_and_spawn_failures_settle_typed() {
        let adapter = TerminalBench30Adapter::from_profile(synthetic_profile());
        let dir = tempfile::tempdir().unwrap();
        let script = fake_verifier_script(
            dir.path(),
            "fake-verifier-fail",
            "cp \"$OPI_EVAL_CTRF_SOURCE\" ./ctrf-report.json\nexit 5",
        );
        let request = e2e_request(dir.path(), script);

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

    #[test]
    fn native_metrics_shape_is_shared_with_the_21_contract() {
        // The synthetic wiring path projects the same native counter shape
        // the shared contract defines, with native names retained.
        let adapter = TerminalBench30Adapter::from_profile(synthetic_profile());
        let dir = tempfile::tempdir().unwrap();
        let trace = dir.path().join("trace");
        std::fs::create_dir_all(&trace).unwrap();
        let request = request_with(&adapter, fixture_path("task-package"), trace.clone());
        write_ctrf(&trace, "ok-six-passed.json");
        let (_, completion) = adapter.settle(&outcome_exit_zero(), &request);
        let BenchmarkCompletion::Verified { metrics, .. } = completion else {
            panic!("expected verified, got {completion:?}");
        };
        let origin = "ctrf-summary".to_owned();
        assert_eq!(
            metrics,
            NativeMetrics {
                tests: Some(Fact::Known {
                    value: 6,
                    origin: origin.clone()
                }),
                passed: Some(Fact::Known {
                    value: 6,
                    origin: origin.clone()
                }),
                failed: Some(Fact::Known {
                    value: 0,
                    origin: origin.clone()
                }),
                skipped: Some(Fact::Known {
                    value: 0,
                    origin: origin.clone()
                }),
                pending: Some(Fact::Known {
                    value: 0,
                    origin: origin.clone()
                }),
                other: Some(Fact::Known { value: 0, origin }),
            }
        );
    }
}
