//! Crate-private native material manifest (native mode).
//!
//! [`NativeMaterial`] is the resolved-material contract between the
//! committed producer stages and the native driving entry: the producer
//! materializes and pins every real identity (fetched external clones,
//! exact built agent executables, the pinned uv verifier entrypoint, the
//! oracle wrapper, the scripted-provider listener endpoint, and the
//! admitted static external lock), and the runner consumes exactly those
//! paths and digests, never synthesizing helper executables and never
//! reading the hermetic fixtures tree for native bytes. Loading is
//! fail-closed: schema drift, digest drift, missing executables, and
//! unknown benchmark or agent identities are typed rejections
//! (`EVAL-BMK-001`, `EVAL-AGT-001`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

/// The one admitted manifest schema identity.
pub(crate) const MATERIAL_SCHEMA: &str = "opi-eval-native-material/3";

/// Typed material rejections (exit-path failures, never settled outcomes).
#[derive(Debug, Error)]
pub(crate) enum MaterialError {
    /// The manifest file could not be read.
    #[error("cannot read native material manifest: {0}")]
    Io(#[from] std::io::Error),
    /// The manifest is not the admitted schema.
    #[error("native material rejected: {0}")]
    Rejected(String),
}

/// One pinned file identity: the path plus its required SHA-256.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PinnedFile {
    pub(crate) path: PathBuf,
    pub(crate) sha256: String,
}

/// The scripted-provider listener identity for the run.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProviderMaterial {
    pub(crate) script: PinnedFile,
    /// The pre-resolved listener endpoint the conformance connection
    /// projects.
    pub(crate) endpoint: String,
    /// Where the provider writes its normalized request log.
    #[expect(
        dead_code,
        reason = "required native-material manifest field validated on load"
    )]
    pub(crate) request_log: PathBuf,
}

/// The pinned upstream integration that runs this Agent inside the
/// graded task environment (P19-D01): the module the runner imports, the
/// name it reports, and the byte-pinned module directory.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IntegrationMaterial {
    /// Directory holding the pinned integration module; its sorted file
    /// manifest must digest to `module_manifest_sha256`.
    pub(crate) module_dir: PathBuf,
    /// Sorted-manifest digest of the module directory (fail-closed
    /// against producer-side drift).
    pub(crate) module_manifest_sha256: String,
    /// The runner agent import path (`module:ClassName`) the dispatch
    /// selects on the upstream argv.
    pub(crate) import_path: String,
    /// The agent name the integration reports and every bound trial
    /// must carry (`agent_info.name`).
    pub(crate) agent_name: String,
}

/// The runner families a material pins integrations for: the Terminal-
/// Bench route imports its integration through Harbor and the DeepSWE
/// route through Pier (P19-D04).
pub(crate) const INTEGRATION_FAMILIES: [&str; 2] = ["harbor", "pier"];

/// The per-product agent projection.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentMaterial {
    /// The exact built executable the supervisor spawns verbatim.
    pub(crate) executable: PinnedFile,
    /// The pinned upstream integrations selecting this Agent inside the
    /// graded task environment (native mode; schema v3), keyed by runner
    /// family: exactly one Harbor import and one Pier import, because
    /// the two runner surfaces are not module-compatible. Request
    /// identity comes from the experiment's selected connection
    /// projection, never from the exportable material (P19-D02).
    pub(crate) integrations: BTreeMap<String, IntegrationMaterial>,
}

impl AgentMaterial {
    /// The pinned integration for one runner family.
    pub(crate) fn integration_for(
        &self,
        family: &str,
    ) -> Result<&IntegrationMaterial, MaterialError> {
        self.integrations.get(family).ok_or_else(|| {
            MaterialError::Rejected(format!("no {family} integration pinned for this agent"))
        })
    }
}

/// One benchmark revision's resolved native material.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BenchmarkMaterial {
    /// The production profile TOML (declarative identity, never the
    /// synthetic fixture profile).
    pub(crate) profile: PathBuf,
    /// The materialized official task package directory.
    pub(crate) task_package: PathBuf,
    /// Sorted-manifest digest of the task package the runner recomputes
    /// and compares (fail-closed against producer-side drift).
    pub(crate) task_package_manifest_sha256: String,
    /// The pinned verifier entrypoint (the resolved `uv` executable; the
    /// adapter owns the exact `uv run --locked ...` argv).
    pub(crate) verifier_executable: PinnedFile,
    /// The closed verifier environment projection.
    pub(crate) verifier_env: BTreeMap<String, String>,
    /// The upstream oracle wrapper: consumes the adapter's exact spawn
    /// spec, applies the official reference solution, and grades it with
    /// the unchanged native verifier.
    pub(crate) oracle: PinnedFile,
    /// The closed oracle environment projection.
    pub(crate) oracle_env: BTreeMap<String, String>,
}

/// The whole resolved-material manifest.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeMaterial {
    schema: String,
    /// The admitted static external lock backing every resolved identity.
    pub(crate) static_lock: PinnedFile,
    /// The scripted-provider listener.
    pub(crate) provider: ProviderMaterial,
    /// Exactly the two admitted agent products.
    pub(crate) agents: BTreeMap<String, AgentMaterial>,
    /// The admitted benchmark revisions (keyed by CLI adapter id).
    pub(crate) benchmarks: BTreeMap<String, BenchmarkMaterial>,
}

impl NativeMaterial {
    /// Loads and fully validates the manifest. Every pinned digest is
    /// recomputed against the bytes on disk; every declared path must
    /// exist; the agent and benchmark identity sets must be exact.
    pub(crate) fn load(path: &Path) -> Result<Self, MaterialError> {
        let text = std::fs::read_to_string(path)?;
        let material: NativeMaterial = serde_json::from_str(&text)
            .map_err(|error| MaterialError::Rejected(format!("manifest parse failed: {error}")))?;
        if material.schema != MATERIAL_SCHEMA {
            return Err(MaterialError::Rejected(format!(
                "unknown schema {:?} (expected {MATERIAL_SCHEMA:?})",
                material.schema
            )));
        }
        verify_pinned(&material.static_lock)?;
        verify_pinned(&material.provider.script)?;
        let agent_ids = material.agents.keys().cloned().collect::<Vec<_>>();
        if agent_ids != ["opi", "pi"] {
            return Err(MaterialError::Rejected(format!(
                "agent set must be exactly [opi, pi], found {agent_ids:?}"
            )));
        }
        for (product, agent) in &material.agents {
            verify_pinned(&agent.executable)?;
            let families = agent.integrations.keys().cloned().collect::<Vec<_>>();
            if families != INTEGRATION_FAMILIES {
                return Err(MaterialError::Rejected(format!(
                    "agent {product} integration families must be exactly {INTEGRATION_FAMILIES:?}, found {families:?}"
                )));
            }
            for (family, integration) in &agent.integrations {
                // The pinned integration module directory must digest to
                // its declared manifest and carry the import path it
                // selects.
                let observed = directory_manifest_digest(&integration.module_dir)?;
                if observed != integration.module_manifest_sha256 {
                    return Err(MaterialError::Rejected(format!(
                        "agent {product} {family} integration module drift: pinned {}, observed {}",
                        integration.module_manifest_sha256, observed
                    )));
                }
                if integration.import_path.split(':').count() != 2
                    || integration.import_path.is_empty()
                    || integration.agent_name.trim().is_empty()
                {
                    return Err(MaterialError::Rejected(format!(
                        "agent {product} {family} integration must pin module:Class and the reported agent name"
                    )));
                }
            }
        }
        let admitted = ["terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1"];
        for (benchmark, entry) in &material.benchmarks {
            if !admitted.contains(&benchmark.as_str()) {
                return Err(MaterialError::Rejected(format!(
                    "benchmark {benchmark:?} is not admitted"
                )));
            }
            if !entry.profile.is_file() {
                return Err(MaterialError::Rejected(format!(
                    "benchmark {benchmark} profile is missing: {}",
                    entry.profile.display()
                )));
            }
            if !entry.task_package.join("instruction.md").is_file() {
                return Err(MaterialError::Rejected(format!(
                    "benchmark {benchmark} task package has no instruction.md"
                )));
            }
            verify_pinned(&entry.verifier_executable)?;
            verify_pinned(&entry.oracle)?;
        }
        if material.benchmarks.is_empty() {
            return Err(MaterialError::Rejected(
                "at least one benchmark revision is required".to_owned(),
            ));
        }
        Ok(material)
    }

    /// The agent material for one product.
    pub(crate) fn agent(&self, product: &str) -> Result<&AgentMaterial, MaterialError> {
        self.agents.get(product).ok_or_else(|| {
            MaterialError::Rejected(format!("agent product {product:?} is not in the material"))
        })
    }

    /// The benchmark material for one CLI adapter id.
    pub(crate) fn benchmark(&self, adapter: &str) -> Result<&BenchmarkMaterial, MaterialError> {
        self.benchmarks.get(adapter).ok_or_else(|| {
            MaterialError::Rejected(format!(
                "benchmark revision {adapter:?} is not in the material"
            ))
        })
    }
}

/// Verifies one pinned file: existence plus digest match.
pub(crate) fn verify_pinned(pinned: &PinnedFile) -> Result<(), MaterialError> {
    let bytes = std::fs::read(&pinned.path)?;
    let digest = sha256_hex(&bytes);
    if digest != pinned.sha256 {
        return Err(MaterialError::Rejected(format!(
            "digest drift at {}: pinned {}, observed {}",
            pinned.path.display(),
            pinned.sha256,
            digest
        )));
    }
    Ok(())
}

/// Sorted-manifest digest over an arbitrary directory: every file's
/// relative path and SHA-256, sorted, canonicalized into one digest.
/// Shared by task packages and integration module directories.
pub(crate) fn directory_manifest_digest(root: &Path) -> Result<String, MaterialError> {
    let mut rows: Vec<(String, String)> = Vec::new();
    visit_files(root, root, &mut rows)?;
    rows.sort();
    let mut canonical = String::new();
    for (path, digest) in rows {
        canonical.push_str(&path);
        canonical.push('\n');
        canonical.push_str(&digest);
        canonical.push('\n');
    }
    Ok(sha256_hex(canonical.as_bytes()))
}

/// Sorted-manifest digest over a task-package directory: every file's
/// relative path and SHA-256, sorted, canonicalized into one digest. This
/// is the runner-side recomputation the manifest pins.
pub(crate) fn task_package_manifest_digest(root: &Path) -> Result<String, MaterialError> {
    directory_manifest_digest(root)
}

fn visit_files(
    root: &Path,
    dir: &Path,
    rows: &mut Vec<(String, String)>,
) -> Result<(), MaterialError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            visit_files(root, &path, rows)?;
        } else {
            let bytes = std::fs::read(&path)?;
            rows.push((
                path.strip_prefix(root)
                    .expect("visitor stays under root")
                    .to_string_lossy()
                    .replace('\\', "/"),
                sha256_hex(&bytes),
            ));
        }
    }
    Ok(())
}

/// Lowercase hex SHA-256 of `bytes`.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().expect("parent")).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    fn minimal_manifest(dir: &Path) -> String {
        let lock = dir.join("lock.json");
        write(&lock, b"{}");
        let provider = dir.join("provider.py");
        write(&provider, b"provider");
        let integrations = dir.join("integrations");
        write(
            &integrations.join("opi_eval_harbor.py"),
            b"# pinned integration module\n",
        );
        let opi = dir.join("opi");
        write(&opi, b"opi-bin");
        let pi = dir.join("pi.js");
        write(&pi, b"pi-bin");
        let uv = dir.join("uv");
        write(&uv, b"uv-bin");
        let oracle = dir.join("oracle.sh");
        write(&oracle, b"oracle");
        let profile = dir.join("tb21.toml");
        write(&profile, b"profile");
        let task = dir.join("task");
        write(&task.join("instruction.md"), b"instruction");
        format!(
            r#"{{
  "schema": "opi-eval-native-material/3",
  "static_lock": {{"path": {:?}, "sha256": {:?}}},
  "provider": {{
    "script": {{"path": {:?}, "sha256": {:?}}},
    "endpoint": "http://127.0.0.1:48127/v1",
    "request_log": {:?}
  }},
  "agents": {{
    "opi": {{
      "executable": {{"path": {:?}, "sha256": {:?}}},
      "integrations": {{
        "harbor": {{"module_dir": {:?}, "module_manifest_sha256": {:?},
                    "import_path": "opi_eval_harbor:OpiAgent",
                    "agent_name": "opi-eval-opi"}},
        "pier": {{"module_dir": {:?}, "module_manifest_sha256": {:?},
                   "import_path": "opi_eval_pier:OpiAgent",
                   "agent_name": "opi-eval-opi"}}
      }}
    }},
    "pi": {{
      "executable": {{"path": {:?}, "sha256": {:?}}},
      "integrations": {{
        "harbor": {{"module_dir": {:?}, "module_manifest_sha256": {:?},
                    "import_path": "opi_eval_harbor:PiAgent",
                    "agent_name": "opi-eval-pi"}},
        "pier": {{"module_dir": {:?}, "module_manifest_sha256": {:?},
                   "import_path": "opi_eval_pier:PiAgent",
                   "agent_name": "opi-eval-pi"}}
      }}
    }}
  }},
  "benchmarks": {{
    "terminal-bench-2.1": {{
      "profile": {:?},
      "task_package": {:?},
      "task_package_manifest_sha256": "{}",
      "verifier_executable": {{"path": {:?}, "sha256": {:?}}},
      "verifier_env": {{}},
      "oracle": {{"path": {:?}, "sha256": {:?}}},
      "oracle_env": {{}}
    }}
  }}
}}"#,
            lock.to_string_lossy(),
            sha256_hex(b"{}"),
            provider.to_string_lossy(),
            sha256_hex(b"provider"),
            dir.join("requests.jsonl").to_string_lossy(),
            opi.to_string_lossy(),
            sha256_hex(b"opi-bin"),
            integrations.to_string_lossy(),
            directory_manifest_digest(&integrations).unwrap(),
            integrations.to_string_lossy(),
            directory_manifest_digest(&integrations).unwrap(),
            pi.to_string_lossy(),
            sha256_hex(b"pi-bin"),
            integrations.to_string_lossy(),
            directory_manifest_digest(&integrations).unwrap(),
            integrations.to_string_lossy(),
            directory_manifest_digest(&integrations).unwrap(),
            profile.to_string_lossy(),
            task.to_string_lossy(),
            task_package_manifest_digest(&task).unwrap(),
            uv.to_string_lossy(),
            sha256_hex(b"uv-bin"),
            oracle.to_string_lossy(),
            sha256_hex(b"oracle"),
        )
    }

    #[test]
    fn loads_and_validates_a_minimal_manifest() {
        let dir = std::env::temp_dir().join(format!("opi-material-ok-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let manifest = dir.join("material.json");
        write(&manifest, minimal_manifest(&dir).as_bytes());
        let material = NativeMaterial::load(&manifest).expect("valid manifest");
        assert_eq!(material.agents.len(), 2);
        assert_eq!(material.provider.endpoint, "http://127.0.0.1:48127/v1");
        assert!(material.benchmark("terminal-bench-2.1").is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn digest_drift_is_a_typed_rejection() {
        let dir = std::env::temp_dir().join(format!("opi-material-drift-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let manifest = dir.join("material.json");
        write(&manifest, minimal_manifest(&dir).as_bytes());
        // Tamper with the pinned agent executable after materialization.
        write(&dir.join("opi"), b"tampered");
        let error = NativeMaterial::load(&manifest).expect_err("drift rejected");
        assert!(error.to_string().contains("digest drift"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_schema_and_agent_sets_are_rejected() {
        let dir = std::env::temp_dir().join(format!("opi-material-schema-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let manifest = dir.join("material.json");
        let text = minimal_manifest(&dir).replace("opi-eval-native-material/3", "other/1");
        write(&manifest, text.as_bytes());
        assert!(
            NativeMaterial::load(&manifest)
                .expect_err("schema rejected")
                .to_string()
                .contains("unknown schema")
        );
        let text = minimal_manifest(&dir).replace("\"opi\":", "\"opix\":");
        write(&manifest, text.as_bytes());
        assert!(
            NativeMaterial::load(&manifest)
                .expect_err("agent set rejected")
                .to_string()
                .contains("agent set")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn task_package_manifest_digest_is_path_normalized() {
        let dir = std::env::temp_dir().join(format!("opi-material-pkg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let pkg = dir.join("pkg");
        write(&pkg.join("a.txt"), b"one");
        write(&pkg.join("sub/b.txt"), b"two");
        let digest = task_package_manifest_digest(&pkg).unwrap();
        // Adding a file must change the digest (no silent drift).
        write(&pkg.join("c.txt"), b"three");
        assert_ne!(task_package_manifest_digest(&pkg).unwrap(), digest);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
