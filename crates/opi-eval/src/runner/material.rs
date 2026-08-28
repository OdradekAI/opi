//! Crate-private native material manifest (Phase 18 task 18.14.1).
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
//! (`P18-BMK-001`, `P18-AGT-001`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

/// The one admitted manifest schema identity.
pub(crate) const MATERIAL_SCHEMA: &str = "phase18-native-material/1";

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
    /// The pre-resolved listener endpoint both agents project.
    pub(crate) endpoint: String,
    /// Where the provider writes its normalized request log.
    pub(crate) request_log: PathBuf,
}

/// The per-product agent projection.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentMaterial {
    /// The exact built executable the supervisor spawns verbatim.
    pub(crate) executable: PinnedFile,
    /// The verbatim `provider_model` string for the shared request.
    pub(crate) model: String,
    /// The closed environment projection beyond the profile isolation
    /// (dummy credential only; no ambient values).
    pub(crate) provider_env: BTreeMap<String, String>,
    /// The deterministic configuration projection materialized into the
    /// isolated agent directories before dispatch.
    pub(crate) config: AgentConfigMaterial,
}

/// The deterministic configuration each product receives.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentConfigMaterial {
    /// `opi-toml` or `pi-models-json`.
    pub(crate) kind: String,
    /// The OpenAI-compatible base URL (the provider endpoint).
    pub(crate) base_url: String,
    /// The model identity inside the configuration.
    pub(crate) model_id: String,
    /// The declared dummy credential value.
    pub(crate) api_key: String,
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
            if agent.model.split(':').count() != 2 || agent.model.is_empty() {
                return Err(MaterialError::Rejected(format!(
                    "agent {product} model must be provider:model, found {:?}",
                    agent.model
                )));
            }
            if agent.provider_env.values().any(|value| value.is_empty()) {
                return Err(MaterialError::Rejected(format!(
                    "agent {product} provider environment must be closed and non-empty"
                )));
            }
            if agent.config.api_key.is_empty() {
                // The dummy credential is required: an empty credential
                // would silently widen into ambient fallback.
                return Err(MaterialError::Rejected(format!(
                    "agent {product} must declare a dummy credential"
                )));
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

/// Sorted-manifest digest over a task-package directory: every file's
/// relative path and SHA-256, sorted, canonicalized into one digest. This
/// is the runner-side recomputation the manifest pins.
pub(crate) fn task_package_manifest_digest(root: &Path) -> Result<String, MaterialError> {
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
  "schema": "phase18-native-material/1",
  "static_lock": {{"path": {:?}, "sha256": {:?}}},
  "provider": {{
    "script": {{"path": {:?}, "sha256": {:?}}},
    "endpoint": "http://127.0.0.1:48127/v1",
    "request_log": {:?}
  }},
  "agents": {{
    "opi": {{
      "executable": {{"path": {:?}, "sha256": {:?}}},
      "model": "scripted:phase18",
      "provider_env": {{"OPENAI_API_KEY": "<dummy-scripted-credential>"}},
      "config": {{"kind": "opi-toml", "base_url": "http://127.0.0.1:48127/v1",
                  "model_id": "phase18", "api_key": "<dummy>"}}
    }},
    "pi": {{
      "executable": {{"path": {:?}, "sha256": {:?}}},
      "model": "scripted:scripted/phase18",
      "provider_env": {{"PI_API_KEY": "<redacted-dummy>"}},
      "config": {{"kind": "pi-models-json", "base_url": "http://127.0.0.1:48127/v1",
                  "model_id": "scripted/phase18", "api_key": "<redacted-dummy>"}}
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
            pi.to_string_lossy(),
            sha256_hex(b"pi-bin"),
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
        let text = minimal_manifest(&dir).replace("phase18-native-material/1", "other/1");
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
