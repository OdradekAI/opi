//! Crate-private Terminal-Bench 2.1 benchmark adapter.
//!
//! Owns only what is Terminal-Bench-2.1-specific: the pinned declarative
//! profile, the complete task-package admission matrix, the exact Harbor
//! v0.22.0 verifier launch, and the CTRF-native-result importer with its
//! revision-specific failure mapping. The shared benchmark-neutral execution
//! contract lives in [`super::process`]; grading policy, normalization, and
//! reward fabrication never enter this module. The adapter is pinned to
//! `harbor-framework/terminal-bench-2-1` commit `7131e437…` exactly as
//! admitted by the static external lock (`EVAL-BMK-001`, `EVAL-BMK-008`).

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

/// The parsed declarative Terminal-Bench 2.1 profile: pinned identity, the
/// complete task-package file table, the native verifier entry, and limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tb21Profile {
    pub source_commit: String,
    pub tasks_tree: String,
    pub task_id: String,
    pub task_tree: String,
    pub package_manifest_sha256: String,
    pub adapter: String,
    pub launch: Vec<String>,
    pub timeout_secs: u64,
    pub stdout_cap_bytes: u64,
    pub stderr_cap_bytes: u64,
    /// Sorted by path; exactly the pinned package closure.
    pub package: Vec<PinnedFile>,
}

/// Typed profile failures. Fail-closed on any drift from the pinned surface.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TbProfileError {
    Parse(String),
    UnsupportedSchema(String),
    /// A pinned invariant drifted (unknown runner/output kind, malformed
    /// launch template, bad digests, duplicate/empty package rows).
    Drift(String),
}

impl std::fmt::Display for TbProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
struct Tb21Doc {
    schema: String,
    benchmark: String,
    revision: String,
    identity: IdentityDoc,
    verifier: VerifierDoc,
    #[serde(rename = "resources")]
    _resources: ResourcesDoc,
    limits: LimitsDoc,
    #[serde(default)]
    package: Vec<PackageDoc>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentityDoc {
    #[serde(rename = "upstream")]
    _upstream: String,
    source_commit: String,
    tasks_tree: String,
    task_id: String,
    task_tree: String,
    package_manifest_sha256: String,
    adapter: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifierDoc {
    runner_kind: String,
    #[serde(rename = "runner_version")]
    _runner_version: String,
    #[serde(rename = "runner_commit")]
    _runner_commit: String,
    launch: Vec<String>,
    output_kind: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ResourcesDoc {
    #[serde(rename = "cpus")]
    _cpus: u64,
    #[serde(rename = "memory_gib")]
    _memory_gib: u64,
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

impl Tb21Profile {
    /// Parse and fully validate a declarative profile document.
    ///
    /// Fails closed on an unsupported schema, a non-Terminal-Bench benchmark
    /// or revision, a non-pinned runner/output/launch surface, malformed
    /// digests, and any duplicate, empty, or self-inconsistent package row.
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, TbProfileError> {
        let text = String::from_utf8(bytes.to_vec())
            .map_err(|_| TbProfileError::Parse("profile is not valid UTF-8".to_owned()))?;
        let doc: Tb21Doc =
            toml::from_str(&text).map_err(|e| TbProfileError::Parse(e.to_string()))?;
        if doc.schema != "opi-eval-benchmark-profile/1" {
            return Err(TbProfileError::UnsupportedSchema(doc.schema));
        }
        if doc.benchmark != "terminal-bench" || doc.revision != "2.1" {
            return Err(TbProfileError::Drift(format!(
                "profile is {} {}, not terminal-bench 2.1",
                doc.benchmark, doc.revision
            )));
        }
        if doc.verifier.runner_kind != "harbor" {
            return Err(TbProfileError::Drift(format!(
                "runner_kind {} is not the pinned harbor surface",
                doc.verifier.runner_kind
            )));
        }
        if doc.verifier.output_kind != "ctrf-json" {
            return Err(TbProfileError::Drift(format!(
                "output_kind {} is not the pinned ctrf-json surface",
                doc.verifier.output_kind
            )));
        }
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
        if package.len() < 2 {
            return Err(TbProfileError::Drift(
                "the pinned task package must be a complete multi-file closure".to_owned(),
            ));
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
            source_commit: doc.identity.source_commit,
            tasks_tree: doc.identity.tasks_tree,
            task_id: doc.identity.task_id,
            task_tree: doc.identity.task_tree,
            package_manifest_sha256: doc.identity.package_manifest_sha256,
            adapter: doc.identity.adapter,
            launch: doc.verifier.launch,
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
    /// (object keys serialize in sorted order).
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

    /// Validate a materialized task package against the pinned closure
    /// (`EVAL-BMK-002`): exactly the pinned files, byte-identical digests.
    /// Prompt text alone, missing environment/verifier/oracle material, or
    /// any extra file fails closed.
    pub(crate) fn validate_task_package(&self, root: &Path) -> Result<(), TaskPackageError> {
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

/// Typed task-package admission failures (`EVAL-BMK-002`).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TaskPackageError {
    Io(String),
    MissingFile { path: String },
    ExtraFile { path: String },
    Mismatch { path: String, reason: String },
}

impl std::fmt::Display for TaskPackageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskPackageError::Io(message) => write!(f, "task package io failure: {message}"),
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
            .join("tests/fixtures/benchmarks/terminal-bench-2.1")
            .join(relative)
    }

    #[test]
    fn production_profile_is_pinned_to_the_static_external_lock() {
        // EVAL-BMK-001/EVAL-BMK-008: the profile must restate exactly the
        // task-package pins the committed static lock already admits.
        let profile = Tb21Profile::parse(&read(
            &crate_root().join("profiles/benchmarks/terminal-bench-2.1.toml"),
        ))
        .unwrap();
        assert_eq!(
            profile.source_commit,
            "7131e4375048a0e408a8fb404b5f499d726b695b"
        );
        assert_eq!(
            profile.tasks_tree,
            "2f0f5fdc68f0befd9b4745386eb8698264b00d8a"
        );
        assert_eq!(profile.task_id, "openssl-selfsigned-cert");
        assert_eq!(
            profile.task_tree,
            "4c5b1214db4b807f2bc4c1bff8803402e36b648b"
        );
        assert_eq!(profile.launch, vec!["run", "-p", "<task-dir>"]);

        let lock: serde_json::Value = serde_json::from_slice(&read(
            &crate_root().join("external-locks/static/linux-x86_64.json"),
        ))
        .unwrap();
        let subject = &lock["subjects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == "terminal-bench-2.1")
            .unwrap()["task"];
        assert_eq!(subject["id"].as_str().unwrap(), profile.task_id);
        assert_eq!(subject["tree"].as_str().unwrap(), profile.task_tree);
        let mut locked: Vec<(String, u64, String)> = subject["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                (
                    f["path"].as_str().unwrap().to_owned(),
                    f["size"].as_u64().unwrap(),
                    f["sha256"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        locked.sort();
        let pinned: Vec<(String, u64, String)> = profile
            .package
            .iter()
            .map(|f| (f.path.clone(), f.size, f.sha256.clone()))
            .collect();
        assert_eq!(pinned, locked);
        // And the declared manifest digest covers exactly that table.
        assert_eq!(
            profile.canonical_package_digest(),
            profile.package_manifest_sha256
        );
    }

    #[test]
    fn synthetic_profile_accepts_its_task_package() {
        let profile = Tb21Profile::parse(&read(&fixture("profile/synthetic.toml"))).unwrap();
        profile
            .validate_task_package(&fixture("task-package"))
            .unwrap();
    }

    #[test]
    fn task_package_admission_fails_closed_on_every_drift() {
        let profile = Tb21Profile::parse(&read(&fixture("profile/synthetic.toml"))).unwrap();
        let source = fixture("task-package");

        // Missing pinned file.
        let dir = tempfile::tempdir().unwrap();
        copy_dir(&source, dir.path());
        std::fs::remove_file(dir.path().join("tests/test_outputs.py")).unwrap();
        assert_eq!(
            profile.validate_task_package(dir.path()),
            Err(TaskPackageError::MissingFile {
                path: "tests/test_outputs.py".to_owned()
            })
        );

        // Extra unpinned file.
        let dir = tempfile::tempdir().unwrap();
        copy_dir(&source, dir.path());
        std::fs::write(dir.path().join("stray.txt"), b"leak").unwrap();
        assert_eq!(
            profile.validate_task_package(dir.path()),
            Err(TaskPackageError::ExtraFile {
                path: "stray.txt".to_owned()
            })
        );

        // Byte drift inside a pinned file.
        let dir = tempfile::tempdir().unwrap();
        copy_dir(&source, dir.path());
        std::fs::write(dir.path().join("instruction.md"), b"tampered").unwrap();
        assert!(matches!(
            profile.validate_task_package(dir.path()),
            Err(TaskPackageError::Mismatch { path, .. }) if path == "instruction.md"
        ));
    }

    #[test]
    fn profile_parsing_fails_closed_on_drift() {
        let base = read(&fixture("profile/synthetic.toml"));
        let text = String::from_utf8(base.clone()).unwrap();
        let mutated = |from: &str, to: &str| text.replacen(from, to, 1).into_bytes();

        // Wrong schema.
        assert_eq!(
            Tb21Profile::parse(&mutated(
                "opi-eval-benchmark-profile/1",
                "opi-eval-benchmark-profile/2"
            )),
            Err(TbProfileError::UnsupportedSchema(
                "opi-eval-benchmark-profile/2".to_owned()
            ))
        );
        // Wrong revision.
        assert!(matches!(
            Tb21Profile::parse(&mutated("revision = \"2.1\"", "revision = \"3.0\"")),
            Err(TbProfileError::Drift(_))
        ));
        // Unpinned runner.
        assert!(matches!(
            Tb21Profile::parse(&mutated(
                "runner_kind = \"harbor\"",
                "runner_kind = \"pier\""
            )),
            Err(TbProfileError::Drift(_))
        ));
        // Unknown output kind.
        assert!(matches!(
            Tb21Profile::parse(&mutated(
                "output_kind = \"ctrf-json\"",
                "output_kind = \"junit-xml\""
            )),
            Err(TbProfileError::Drift(_))
        ));
        // Launch template losing its only placeholder.
        assert!(matches!(
            Tb21Profile::parse(&mutated("\"-p\", \"<task-dir>\"", "\"-p\", \".\"")),
            Err(TbProfileError::Drift(_))
        ));
        // Manifest digest no longer covering the table: flip the declared
        // digest (whatever it is) to a different well-formed value.
        let declared: String = text
            .lines()
            .find(|l| l.starts_with("package_manifest_sha256 = "))
            .unwrap()
            .trim_start_matches("package_manifest_sha256 = \"")
            .trim_end_matches('\"')
            .to_owned();
        let flipped: String = declared
            .chars()
            .map(|c| if c == 'a' { 'b' } else { 'a' })
            .collect();
        assert!(matches!(
            Tb21Profile::parse(&mutated(&declared, &flipped)),
            Err(TbProfileError::Drift(_))
        ));
        // Unknown top-level key (deny_unknown_fields at every table).
        let with_extra = format!("{text}\n[extras]\nsurprise = true\n").into_bytes();
        assert!(matches!(
            Tb21Profile::parse(&with_extra),
            Err(TbProfileError::Parse(_))
        ));
    }

    fn copy_dir(source: &Path, target: &Path) {
        for entry in std::fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name();
                std::fs::create_dir_all(target.join(&name)).unwrap();
                copy_dir(&path, &target.join(&name));
            } else {
                let name = entry.file_name();
                std::fs::copy(&path, target.join(&name)).unwrap();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Adapter: benchmark-neutral contract implementation for Terminal-Bench 2.1.

use super::process::{
    BenchmarkAdapter, BenchmarkCompletion, BenchmarkIdentity, BenchmarkRunRequest, ExecutionError,
    failure_kinds,
};
use crate::agent::process::{Fact, NativeArtifact};
use crate::failure::FailureBoundaryCode;
use crate::process::{ExitState, SpawnSpec};

/// Where the pinned native verifier writes its CTRF report. The verifier
/// chain is `tests/test.sh` -> `uvx pytest-json-ctrf==0.3.5`, whose default
/// report filename is `ctrf-report.json` in the verifier working directory;
/// the adapter pins the verifier cwd to the run's trace root.
const CTRF_REPORT_NAME: &str = "ctrf-report.json";

/// The Terminal-Bench 2.1 benchmark adapter. Owns the pinned declarative
/// profile, complete task-package admission, the exact Harbor v0.22.0 launch
/// through `uv run --locked`, and the CTRF importer with its
/// revision-specific failure mapping.
pub(crate) struct TerminalBench21Adapter {
    profile: Tb21Profile,
}

impl TerminalBench21Adapter {
    /// Build the adapter from a validated declarative profile.
    pub(crate) fn from_profile(profile: Tb21Profile) -> Self {
        Self { profile }
    }

    /// The profile this adapter pins.
    #[cfg(test)]
    pub(crate) fn profile(&self) -> &Tb21Profile {
        &self.profile
    }

    /// Import the pinned CTRF schema and project its summary counters with
    /// native names retained (`EVAL-BMK-007`). Fails closed on drift.
    fn import_ctrf(bytes: &[u8]) -> Result<super::process::NativeMetrics, CtrfError> {
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| CtrfError::Parse)?;
        let Some(results) = value.as_object().and_then(|o| o.get("results")) else {
            return Err(CtrfError::UnsupportedSchema);
        };
        if value.as_object().map(|o| o.len()) != Some(1) {
            return Err(CtrfError::UnsupportedSchema);
        }
        let Some(fields) = results.as_object() else {
            return Err(CtrfError::UnsupportedSchema);
        };
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
        // The summary vocabulary is closed: the six counters plus the
        // start/stop timestamps, all integers. An added or missing key is
        // upstream drift and fails closed instead of being renamed.
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

/// Typed CTRF import failures.
enum CtrfError {
    Parse,
    UnsupportedSchema,
}

impl BenchmarkAdapter for TerminalBench21Adapter {
    fn identity(&self) -> BenchmarkIdentity {
        BenchmarkIdentity {
            benchmark: "terminal-bench".to_owned(),
            revision: "2.1".to_owned(),
            adapter: self.profile.adapter.clone(),
        }
    }

    fn admission(&self, request: &BenchmarkRunRequest) -> Result<(), ExecutionError> {
        // Revision binding: the integrity record must admit exactly the
        // revision this adapter pins, and the requested task must be the
        // profile's pinned task.
        if request.integrity.revision() != "2.1" {
            return Err(ExecutionError {
                token: "revision-binding-mismatch",
            });
        }
        if request.task_id != self.profile.task_id {
            return Err(ExecutionError {
                token: "task-binding-mismatch",
            });
        }
        // Complete task-package admission before any verifier invocation
        // (EVAL-BMK-002): prompts alone never reach a spawn.
        self.profile
            .validate_task_package(&request.task_dir)
            .map_err(|_| ExecutionError {
                token: "task-package-drift",
            })?;
        Ok(())
    }

    fn spawn_spec(&self, request: &BenchmarkRunRequest) -> SpawnSpec {
        // Pinned evidence: Harbor v0.22.0 is driven through
        // `uv run --locked harbor run -p <task-dir>`; argv[0] is the
        // resolved uv executable, never an ambient PATH lookup.
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
            // The verifier working directory is the run's trace root so the
            // native CTRF report lands inside the captured evidence.
            cwd: Some(request.trace_root.clone()),
            env: request.extra_env.clone(),
            stdout_cap: self.profile.stdout_cap_bytes as usize,
            stderr_cap: self.profile.stderr_cap_bytes as usize,
            timeout: std::time::Duration::from_secs(self.profile.timeout_secs),
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
        // evidence, and no fallback grader exists (EVAL-BMK-006).
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
        // Exit 0: the native authority is the verifier's own output. Two
        // admitted layouts: the direct CTRF report in the trace root, or
        // - the dispatch reality of the native smoke - harbor's
        // `jobs/<timestamp>/result.json` aggregate from `harbor run -p`.
        // Bounded stdout is retained on the record but is not the
        // authority; neither layout being present is a grader-side
        // invalid output.
        let report = request.trace_root.join(CTRF_REPORT_NAME);
        let bytes = match std::fs::read(&report) {
            Ok(bytes) => bytes,
            Err(_) => {
                return match super::process::import_harbor_result(&request.trace_root) {
                    Ok((metrics, reward, path, _value)) => (
                        reward,
                        BenchmarkCompletion::Verified {
                            metrics,
                            artifacts: vec![NativeArtifact {
                                role: "native/harbor-result".to_owned(),
                                sha256: sha256_hex(&std::fs::read(&path).unwrap_or_default()),
                                path,
                            }],
                        },
                    ),
                    Err(error) => (
                        reward_unknown(),
                        BenchmarkCompletion::Failed(super::process::BenchmarkFailure {
                            kind: match error {
                                super::process::HarborResultError::Missing => {
                                    "verifier-output-missing"
                                }
                                super::process::HarborResultError::Invalid("read") => {
                                    "verifier-output-invalid-read"
                                }
                                super::process::HarborResultError::Invalid("json-parse") => {
                                    "verifier-output-invalid-json-parse"
                                }
                                super::process::HarborResultError::Invalid("no-eval-stats") => {
                                    "verifier-output-invalid-no-eval-stats"
                                }
                                super::process::HarborResultError::Invalid("eval-count") => {
                                    "verifier-output-invalid-eval-count"
                                }
                                super::process::HarborResultError::Invalid("no-reward-stats") => {
                                    "verifier-output-invalid-no-reward-stats"
                                }
                                super::process::HarborResultError::Invalid("reward-count") => {
                                    "verifier-output-invalid-reward-count"
                                }
                                super::process::HarborResultError::Invalid("reward-trials") => {
                                    "verifier-output-invalid-reward-trials"
                                }
                                super::process::HarborResultError::Invalid("trial-count") => {
                                    "verifier-output-invalid-trial-count"
                                }
                                super::process::HarborResultError::Invalid("bad-reward-values") => {
                                    "verifier-output-invalid-bad-reward-values"
                                }
                                super::process::HarborResultError::Invalid(_) => {
                                    "verifier-output-invalid-schema"
                                }
                            },
                            boundary: FailureBoundaryCode::Grader,
                        }),
                    ),
                };
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
    use crate::benchmark::process::NativeMetrics;
    #[cfg(unix)]
    use crate::benchmark::process::{BenchmarkExecution, BenchmarkProvenance};
    use crate::integrity::{
        IntegrityRecord, IntegrityReview, OraclePreflight, RevisionStatus, TaskClassification,
    };
    use crate::process::{CleanupEvidence, OutputCapture, SpawnReason, SupervisedOutcome};

    fn synthetic_profile() -> Tb21Profile {
        Tb21Profile::parse(
            &std::fs::read(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/benchmarks/terminal-bench-2.1/profile/synthetic.toml"
            ))
            .unwrap(),
        )
        .unwrap()
    }

    fn fixture_path(relative: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/benchmarks/terminal-bench-2.1")
            .join(relative)
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

    fn integrity_for(revision: &str, task: &str) -> IntegrityRecord {
        IntegrityRecord::review(IntegrityReview {
            benchmark: "terminal-bench".to_owned(),
            revision: revision.to_owned(),
            dataset: "terminal-bench-2-1".to_owned(),
            grader: "harbor-v0.22.0".to_owned(),
            environment: "shared-verifier".to_owned(),
            upstream_identity: "openssl-selfsigned-cert".to_owned(),
            upstream_digest: "0".repeat(64),
            oracle: Some(OraclePreflight::Passed("six tests passed".to_owned())),
            status: RevisionStatus::Admitted,
            tasks: BTreeMap::from([(task.to_owned(), TaskClassification::ValidAgentOutcome)]),
            excluded_trials: BTreeMap::new(),
            reviewer: "human-reviewer".to_owned(),
        })
        .unwrap()
    }

    fn request_with(
        adapter: &TerminalBench21Adapter,
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
            integrity: integrity_for("2.1", &adapter.profile().task_id),
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

    /// Drop a CTRF fixture into a trace root as the verifier would.
    fn write_ctrf(trace_root: &std::path::Path, name: &str) {
        std::fs::copy(
            fixture_path(&format!("ctrf/{name}")),
            trace_root.join(CTRF_REPORT_NAME),
        )
        .unwrap();
    }

    #[test]
    fn adapter_identity_and_admission_bind_the_pinned_revision() {
        let adapter = TerminalBench21Adapter::from_profile(synthetic_profile());
        assert_eq!(
            adapter.identity(),
            BenchmarkIdentity {
                benchmark: "terminal-bench".to_owned(),
                revision: "2.1".to_owned(),
                adapter: "opi-eval-terminal-bench-21-adapter/1".to_owned(),
            }
        );

        let dir = tempfile::tempdir().unwrap();
        let trace = dir.path().join("trace");
        std::fs::create_dir_all(&trace).unwrap();

        // Correct binding passes with the complete package present.
        let mut request = request_with(&adapter, fixture_path("task-package"), trace.clone());
        adapter.admission(&request).unwrap();

        // Revision mismatch is rejected.
        request.integrity = integrity_for("3.0", &adapter.profile().task_id);
        assert_eq!(
            adapter.admission(&request),
            Err(ExecutionError {
                token: "revision-binding-mismatch"
            })
        );
        request.integrity = integrity_for("2.1", &adapter.profile().task_id);

        // Task mismatch is rejected.
        let mut wrong_task = request.clone();
        wrong_task.task_id = "some-other-task".to_owned();
        assert_eq!(
            adapter.admission(&wrong_task),
            Err(ExecutionError {
                token: "task-binding-mismatch"
            })
        );

        // Incomplete package never reaches a verifier spawn (EVAL-BMK-002).
        let incomplete = dir.path().join("incomplete");
        copy_fixture_package(&fixture_path("task-package"), &incomplete);
        std::fs::remove_file(incomplete.join("tests/test.sh")).unwrap();
        let mut bad_package = request.clone();
        bad_package.task_dir = incomplete;
        assert_eq!(
            adapter.admission(&bad_package),
            Err(ExecutionError {
                token: "task-package-drift"
            })
        );
    }

    #[test]
    fn spawn_spec_builds_the_pinned_harbor_argv() {
        let adapter = TerminalBench21Adapter::from_profile(synthetic_profile());
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
        assert_eq!(spec.timeout, std::time::Duration::from_secs(900));
        assert!(spec.env.is_empty());
    }

    #[test]
    fn settle_imports_ctrf_with_native_names_and_keeps_bytes() {
        let adapter = TerminalBench21Adapter::from_profile(synthetic_profile());
        let dir = tempfile::tempdir().unwrap();
        let trace = dir.path().join("trace");
        std::fs::create_dir_all(&trace).unwrap();
        let request = request_with(&adapter, fixture_path("task-package"), trace.clone());
        write_ctrf(&trace, "ok-six-passed.json");
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
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].role, "native/ctrf-report");
        assert_eq!(artifacts[0].path, trace.join(CTRF_REPORT_NAME));
        let bytes = std::fs::read(&artifacts[0].path).unwrap();
        assert_eq!(artifacts[0].sha256.len(), 64);
        assert_eq!(artifacts[0].sha256, sha256_hex(&bytes));

        // A verifier reporting failing tests is still a valid verification:
        // the native counts stay authoritative (EVAL-BMK-007).
        write_ctrf(&trace, "one-failed.json");
        let (_, completion) = adapter.settle(&outcome_exit_zero(), &request);
        let BenchmarkCompletion::Verified { metrics, .. } = completion else {
            panic!("expected verified, got {completion:?}");
        };
        assert_eq!(
            metrics.failed,
            Some(Fact::Known {
                value: 1,
                origin: "ctrf-summary".to_owned()
            })
        );
    }

    #[test]
    fn settle_failure_matrix_maps_every_typed_path() {
        let adapter = TerminalBench21Adapter::from_profile(synthetic_profile());
        let dir = tempfile::tempdir().unwrap();
        let trace = dir.path().join("trace");
        std::fs::create_dir_all(&trace).unwrap();
        let request = request_with(&adapter, fixture_path("task-package"), trace.clone());

        let failed = |kind: &'static str, boundary| {
            BenchmarkCompletion::Failed(super::super::process::BenchmarkFailure { kind, boundary })
        };

        // Missing report on exit 0: neither admitted layout exists.
        assert_eq!(
            adapter.settle(&outcome_exit_zero(), &request).1,
            failed("verifier-output-missing", FailureBoundaryCode::Grader)
        );
        // Corrupt JSON.
        write_ctrf(&trace, "corrupt.json");
        assert_eq!(
            adapter.settle(&outcome_exit_zero(), &request).1,
            failed("import-parse-failure", FailureBoundaryCode::Adapter)
        );
        // Drifted schema: unknown summary counter / missing summary.
        for name in ["unknown-schema.json", "missing-summary.json"] {
            write_ctrf(&trace, name);
            assert_eq!(
                adapter.settle(&outcome_exit_zero(), &request).1,
                failed("import-unsupported-schema", FailureBoundaryCode::Adapter),
                "fixture {name}"
            );
        }
        // Process-level verdicts are authoritative and never rescued.
        write_ctrf(&trace, "ok-six-passed.json");
        let nonzero = SupervisedOutcome {
            exit: ExitState::Exited { code: 3 },
            ..outcome_exit_zero()
        };
        assert_eq!(
            adapter.settle(&nonzero, &request).1,
            failed("verifier-non-zero-exit", FailureBoundaryCode::Grader)
        );
        for exit in [ExitState::TimedOut, ExitState::Cancelled] {
            let outcome = SupervisedOutcome {
                exit,
                ..outcome_exit_zero()
            };
            let completion = adapter.settle(&outcome, &request).1;
            assert!(matches!(
                completion,
                BenchmarkCompletion::Failed(f) if f.boundary == FailureBoundaryCode::Grader
            ));
        }
        let spawn_fail = SupervisedOutcome {
            exit: ExitState::FailedToSpawn {
                reason: SpawnReason::NotFound,
            },
            ..outcome_exit_zero()
        };
        assert_eq!(
            adapter.settle(&spawn_fail, &request).1,
            failed("spawn-not-found", FailureBoundaryCode::Infrastructure)
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
        let adapter = TerminalBench21Adapter::from_profile(synthetic_profile());
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
        let adapter = TerminalBench21Adapter::from_profile(synthetic_profile());
        let dir = tempfile::tempdir().unwrap();
        let script = fake_verifier_script(
            dir.path(),
            "fake-verifier",
            // The verifier runs with cwd = trace root; the pinned report
            // lands exactly where the importer reads it.
            "cp \"$OPI_EVAL_CTRF_SOURCE\" ./ctrf-report.json",
        );
        let request = e2e_request(dir.path(), script);

        let record = BenchmarkExecution::run(
            &request,
            &adapter,
            &tokio_util::sync::CancellationToken::new(),
        )
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
                revision: "2.1".to_owned(),
                task_id: "fixture-task".to_owned(),
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
    async fn fake_verifier_exit_and_spawn_failures_settle_typed() {
        let adapter = TerminalBench21Adapter::from_profile(synthetic_profile());
        let dir = tempfile::tempdir().unwrap();
        let script = fake_verifier_script(
            dir.path(),
            "fake-verifier-fail",
            "cp \"$OPI_EVAL_CTRF_SOURCE\" ./ctrf-report.json\nexit 5",
        );
        let request = e2e_request(dir.path(), script);

        let record = BenchmarkExecution::run(
            &request,
            &adapter,
            &tokio_util::sync::CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(record.exit, ExitState::Exited { code: 5 });
        assert_eq!(
            record.completion,
            BenchmarkCompletion::Failed(failure_kinds::non_zero_exit(5))
        );
        // A healthy report cannot rescue a failed verifier process.
        assert!(record.stdout.bytes.is_empty() || !record.stdout.truncated);

        // A nonexistent verifier settles as a redacted infrastructure spawn
        // failure.
        let mut missing = request.clone();
        missing.verifier_executable = dir.path().join("no-such-verifier");
        let record = BenchmarkExecution::run(
            &missing,
            &adapter,
            &tokio_util::sync::CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(
            record.completion,
            BenchmarkCompletion::Failed(failure_kinds::spawn(SpawnReason::NotFound))
        );
        assert!(!format!("{record:?}").contains("no-such-verifier"));
    }
}
