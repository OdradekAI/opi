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

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The one admitted manifest schema identity for fresh dispatch.
pub(crate) const MATERIAL_SCHEMA: &str = "opi-eval-native-material/7";
/// The legacy read-only schema: sealed offline verification, reports and
/// verifier-only regrade keep admitting `/6` bytes with their original
/// seals; fresh dispatch never does.
pub(crate) const LEGACY_MATERIAL_SCHEMA: &str = "opi-eval-native-material/6";

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
#[derive(Debug, Clone, Deserialize, Serialize)]
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

/// The byte-pinned resource tree transported with an Agent executable.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RuntimePackage {
    pub(crate) path: PathBuf,
    pub(crate) manifest_sha256: String,
    pub(crate) entrypoint: PathBuf,
}

/// The pinned shared capture helper (P19-AUD-003): the bounded privacy
/// filter executable both integrations stage beside the selected Agent
/// before credentials. `musl` is the production transport (static, any
/// task image); the admitted GNU targets cover hermetic flows whose
/// image matches the build host. Platform incompatibility refuses at
/// the task-side helper preflight, never at dispatch.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CaptureHelperMaterial {
    /// The exact helper executable staged with the Agent.
    pub(crate) executable: PinnedFile,
    /// Compiled target triple.
    pub(crate) target: String,
    /// The helper protocol identity the task-side preflight proves.
    pub(crate) protocol: String,
    /// SHA-256 of the source build's `Cargo.lock`, pinning the exact
    /// dependency graph the helper was compiled from.
    pub(crate) cargo_lock_sha256: String,
}

/// Targets whose helper binaries material admits.
const CAPTURE_HELPER_TARGETS: &[&str] = &[
    "x86_64-unknown-linux-musl",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-musl",
    "aarch64-unknown-linux-gnu",
];

impl CaptureHelperMaterial {
    /// Verifies the helper pin: exact bytes, admitted target, and the
    /// capture helper protocol identity.
    pub(crate) fn verify(&self) -> Result<(), MaterialError> {
        verify_pinned(&self.executable)?;
        if !CAPTURE_HELPER_TARGETS.contains(&self.target.as_str()) {
            return Err(MaterialError::Rejected(format!(
                "capture helper target {:?} is not admitted",
                self.target
            )));
        }
        if self.protocol != crate::native_capture::CAPTURE_HELPER_PROTOCOL {
            return Err(MaterialError::Rejected(format!(
                "capture helper protocol {:?} is not {}",
                self.protocol,
                crate::native_capture::CAPTURE_HELPER_PROTOCOL
            )));
        }
        if self.cargo_lock_sha256.len() != 64
            || !self
                .cargo_lock_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(MaterialError::Rejected(
                "capture helper Cargo.lock digest is malformed".to_owned(),
            ));
        }
        Ok(())
    }
}

/// The per-product agent projection.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentMaterial {
    /// The exact built executable the supervisor spawns verbatim.
    pub(crate) executable: PinnedFile,
    /// Optional runtime resources transported with the executable.
    #[serde(default)]
    pub(crate) runtime_package: Option<RuntimePackage>,
    /// Independent loopback output-control evidence for this exact client.
    #[serde(default)]
    pub(crate) output_control_evidence: Option<PinnedFile>,
    /// The pinned upstream integrations selecting this Agent inside the
    /// graded task environment (native mode; schema v4), keyed by runner
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

/// The source project selected by the native uv invocation.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RunnerProject {
    pub(crate) path: PathBuf,
    /// Tracked Git entries and their working-file bytes, including uv.lock.
    pub(crate) source_manifest_sha256: String,
    /// The installed interpreter the pinned environment selects
    /// (`UV_PYTHON`): exact byte identity, re-verified on load so a
    /// replaced `.venv/bin/python` rejects material use before any
    /// validate/plan/run effect (P19-AUD-003).
    pub(crate) interpreter: PinnedFile,
    /// The installed runner console entrypoint (`.venv/bin/<family>`):
    /// exact byte identity, re-verified on load for the same boundary.
    pub(crate) entrypoint: PinnedFile,
}

impl RunnerProject {
    fn verify(
        &self,
        benchmark: &str,
        env: &BTreeMap<String, String>,
        static_lock: &Path,
    ) -> Result<(), MaterialError> {
        let reject = |reason: &str| MaterialError::Rejected(format!("runner project: {reason}"));
        if !self.path.is_absolute() {
            return Err(reject("root must be absolute"));
        }
        let family = if benchmark == "deepswe-v1.1" {
            "pier"
        } else {
            "harbor"
        };
        // The installed runner bytes must sit at the selected project's
        // pinned locations; a pinned copy of another tree cannot stand in
        // for the environment the dispatch actually selects.
        if self.interpreter.path != self.path.join(".venv/bin/python") {
            return Err(reject(
                "interpreter pin must select the project's .venv/bin/python",
            ));
        }
        if self.entrypoint.path != self.path.join(".venv/bin").join(family) {
            return Err(reject(
                "entrypoint pin must select the project's installed runner console script",
            ));
        }
        verify_pinned(&self.interpreter)
            .map_err(|_| reject("installed interpreter bytes drifted from the pinned identity"))?;
        verify_pinned(&self.entrypoint).map_err(|_| {
            reject("installed runner entrypoint bytes drifted from the pinned identity")
        })?;
        let lock: serde_json::Value = serde_json::from_slice(&std::fs::read(static_lock)?)
            .map_err(|_| reject("cannot parse static lock"))?;
        let pin = lock
            .get("tools")
            .and_then(serde_json::Value::as_array)
            .and_then(|tools| tools.iter().find(|tool| tool["id"] == family))
            .ok_or_else(|| reject("static lock has no selected runner pin"))?;
        let commit = pin["commit"]
            .as_str()
            .ok_or_else(|| reject("static lock has no source commit"))?;
        let blob = pin["uv_lock"]["git_blob"]
            .as_str()
            .ok_or_else(|| reject("static lock has no uv.lock blob"))?;
        let sha = pin["uv_lock"]["sha256"]
            .as_str()
            .ok_or_else(|| reject("static lock has no uv.lock SHA-256"))?;
        if runner_source_digest(&self.path, commit, blob, sha)? != self.source_manifest_sha256 {
            return Err(reject("source manifest digest drift"));
        }
        for (key, expected) in [
            ("UV_PROJECT", self.path.clone()),
            ("UV_PROJECT_ENVIRONMENT", self.path.join(".venv")),
            ("UV_PYTHON", self.path.join(".venv/bin/python")),
        ] {
            if env.get(key).map(String::as_str) != expected.to_str() {
                return Err(reject(
                    "verifier environment does not select the pinned project",
                ));
            }
        }
        if env.contains_key("UV_WORKING_DIRECTORY") {
            return Err(reject(
                "verifier environment must preserve the trial working directory",
            ));
        }
        Ok(())
    }
}

/// One benchmark revision's resolved native material.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskMaterial {
    pub(crate) native_task_name: String,
    pub(crate) profile: PinnedFile,
    /// The materialized official task package directory.
    pub(crate) task_package: PathBuf,
    /// Sorted-manifest digest of the task package the runner recomputes
    /// and compares (fail-closed against producer-side drift).
    pub(crate) task_package_manifest_sha256: String,
}

/// Full pinned source inventory, independent of the selected material map.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CorpusMaterial {
    pub(crate) root: PathBuf,
    pub(crate) source_commit: String,
    pub(crate) tasks_tree: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BenchmarkMaterial {
    pub(crate) corpus: CorpusMaterial,
    pub(crate) tasks: BTreeMap<String, TaskMaterial>,
    /// The pinned verifier entrypoint (the resolved `uv` executable; the
    /// adapter owns the exact `uv run --locked ...` argv).
    pub(crate) verifier_executable: PinnedFile,
    /// The closed verifier environment projection.
    pub(crate) verifier_env: BTreeMap<String, String>,
    /// Required for local preparation; legacy producer-owned wrappers omit it.
    #[serde(default)]
    pub(crate) runner_project: Option<RunnerProject>,
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
    /// Truthful origin of this material: absent for the CI-produced
    /// manifests and `local` for the ones `prepare` freezes from a
    /// seeded local cache. Local origin never manufactures GitHub
    /// provenance (P19-D05).
    #[serde(default)]
    origin: Option<String>,
    /// The admitted static external lock backing every resolved identity.
    pub(crate) static_lock: PinnedFile,
    /// The scripted-provider listener.
    pub(crate) provider: ProviderMaterial,
    /// Exactly the two admitted agent products.
    pub(crate) agents: BTreeMap<String, AgentMaterial>,
    /// The admitted benchmark revisions (keyed by CLI adapter id).
    pub(crate) benchmarks: BTreeMap<String, BenchmarkMaterial>,
    /// The pinned shared capture helper (present on `/7`; fresh dispatch
    /// requires it, legacy `/6` offline reads carry none).
    pub(crate) capture_helper: Option<CaptureHelperMaterial>,
}

impl NativeMaterial {
    /// Loads and fully validates the manifest for both agent products.
    pub(crate) fn load(path: &Path) -> Result<Self, MaterialError> {
        Self::load_for(path, &["opi", "pi"])
    }

    /// Loads and fully validates the manifest for exactly the selected
    /// agent products (a pi-only experiment requires no Opi material;
    /// P19-D05). Every pinned digest is recomputed against the bytes on
    /// disk; every declared path must exist; the agent and benchmark
    /// identity sets must be exact.
    pub(crate) fn load_for(path: &Path, products: &[&str]) -> Result<Self, MaterialError> {
        let text = std::fs::read_to_string(path)?;
        let header: serde_json::Value = serde_json::from_str(&text)
            .map_err(|error| MaterialError::Rejected(format!("manifest parse failed: {error}")))?;
        if header["schema"] != MATERIAL_SCHEMA && header["schema"] != LEGACY_MATERIAL_SCHEMA {
            return Err(MaterialError::Rejected(format!(
                "unknown native-material schema; regenerate material as {MATERIAL_SCHEMA}"
            )));
        }
        let mut material: NativeMaterial = serde_json::from_str(&text)
            .map_err(|error| MaterialError::Rejected(format!("manifest parse failed: {error}")))?;
        // Admission resolves each corpus root once: host layouts whose temp
        // directories traverse links (macOS `/var` -> `/private/var`) must
        // still verify the symlink-free resolved path downstream, while the
        // resolved root keeps every later git and digest check on one path.
        for entry in material.benchmarks.values_mut() {
            if let Ok(resolved) = std::fs::canonicalize(&entry.corpus.root) {
                entry.corpus.root = resolved;
            }
        }
        let mut selected: Vec<&str> = products.to_vec();
        selected.sort_unstable();
        selected.dedup();
        if selected.is_empty()
            || selected
                .iter()
                .any(|product| !["opi", "pi"].contains(product))
        {
            return Err(MaterialError::Rejected(format!(
                "selected products must be a non-empty subset of [opi, pi], found {selected:?}"
            )));
        }
        if material.schema != MATERIAL_SCHEMA && material.schema != LEGACY_MATERIAL_SCHEMA {
            return Err(MaterialError::Rejected(format!(
                "unknown schema {:?} (expected {MATERIAL_SCHEMA:?})",
                material.schema
            )));
        }
        match material.origin.as_deref() {
            None | Some("local") => {}
            Some(other) => {
                return Err(MaterialError::Rejected(format!(
                    "unknown material origin {other:?} (expected none or \"local\")"
                )));
            }
        }
        verify_pinned(&material.static_lock)?;
        verify_pinned(&material.provider.script)?;
        if material.schema == LEGACY_MATERIAL_SCHEMA && material.capture_helper.is_some() {
            return Err(MaterialError::Rejected(
                "legacy material must not carry a capture helper pin".to_owned(),
            ));
        }
        if material.schema == MATERIAL_SCHEMA && material.capture_helper.is_none() {
            return Err(MaterialError::Rejected(
                "fresh material requires the pinned capture helper".to_owned(),
            ));
        }
        if let Some(helper) = &material.capture_helper {
            helper.verify()?;
        }
        let agent_ids = material.agents.keys().cloned().collect::<Vec<_>>();
        if agent_ids != selected {
            return Err(MaterialError::Rejected(format!(
                "agent set must be exactly {selected:?}, found {agent_ids:?}"
            )));
        }
        for (product, agent) in &material.agents {
            verify_pinned(&agent.executable)?;
            if let Some(package) = &agent.runtime_package {
                package.verify(&agent.executable)?;
            }
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
            entry.verify_tasks(benchmark, &material.static_lock.path)?;
            verify_pinned(&entry.verifier_executable)?;
            if let Some(project) = &entry.runner_project {
                project.verify(benchmark, &entry.verifier_env, &material.static_lock.path)?;
            } else if material.origin.as_deref() == Some("local") {
                return Err(MaterialError::Rejected(format!(
                    "benchmark {benchmark} local material requires a runner project"
                )));
            }
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

    /// The schema identity of the loaded manifest. Fresh dispatch sites
    /// branch on it through [`Self::capture_helper`], which already
    /// refuses legacy `/6`; the accessor keeps the boundary observable
    /// for the offline readers that admit both schemas.
    #[expect(
        dead_code,
        reason = "the legacy-boundary accessor is consumed by tests and offline readers"
    )]
    pub(crate) fn schema(&self) -> &str {
        &self.schema
    }

    /// The pinned capture helper for fresh dispatch: legacy `/6` offline
    /// material admits none and fresh dispatch refuses without one.
    pub(crate) fn capture_helper(&self) -> Result<&CaptureHelperMaterial, MaterialError> {
        self.capture_helper.as_ref().ok_or_else(|| {
            MaterialError::Rejected(
                "fresh native dispatch requires material/7 with the pinned capture helper"
                    .to_owned(),
            )
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

impl BenchmarkMaterial {
    pub(crate) fn task(&self, id: &str) -> Result<&TaskMaterial, MaterialError> {
        self.tasks.get(id).ok_or_else(|| {
            MaterialError::Rejected(format!(
                "selected task {id:?} has no byte-bound native material; prepare that task"
            ))
        })
    }

    pub(crate) fn verify_tasks(
        &self,
        adapter: &str,
        lock_path: &Path,
    ) -> Result<(), MaterialError> {
        let lock: serde_json::Value = serde_json::from_slice(&std::fs::read(lock_path)?)
            .map_err(|error| MaterialError::Rejected(error.to_string()))?;
        self.corpus.verify(adapter, &lock)?;
        if self.tasks.is_empty() {
            return Err(MaterialError::Rejected(
                "selected native task map is empty".to_owned(),
            ));
        }
        for (id, task) in &self.tasks {
            if id.is_empty()
                || id
                    .chars()
                    .any(|ch| !(ch.is_ascii_alphanumeric() || "-_.".contains(ch)))
                || id.contains("..")
            {
                return Err(MaterialError::Rejected(
                    "invalid canonical native task id".to_owned(),
                ));
            }
            verify_pinned(&task.profile)?;
            if task_package_manifest_digest(&task.task_package)?
                != task.task_package_manifest_sha256
            {
                return Err(MaterialError::Rejected(format!(
                    "task package manifest drift for {adapter}/{id}"
                )));
            }
            let source = self.corpus.root.join(id);
            let task_tree = git_text(
                &self.corpus.root,
                &["rev-parse", &format!("{}:{id}", self.corpus.tasks_tree)],
            )?;
            let profile: toml::Value = std::fs::read_to_string(&task.profile.path)?
                .parse()
                .map_err(|error| MaterialError::Rejected(format!("profile parse: {error}")))?;
            for (field, expected) in [
                ("task_id", id.as_str()),
                ("task_tree", task_tree.as_str()),
                ("source_commit", self.corpus.source_commit.as_str()),
                ("tasks_tree", self.corpus.tasks_tree.as_str()),
            ] {
                if profile
                    .get("identity")
                    .and_then(|identity| identity.get(field))
                    .and_then(toml::Value::as_str)
                    != Some(expected)
                {
                    return Err(MaterialError::Rejected(format!(
                        "{adapter}/{id} profile {field} differs from pinned corpus"
                    )));
                }
            }
            let profile_bytes = std::fs::read(&task.profile.path)?;
            let admitted = match adapter {
                "terminal-bench-2.1" => {
                    crate::benchmark::terminal_bench_21::Tb21Profile::parse(&profile_bytes[..])
                        .map_err(|error| error.to_string())
                        .and_then(|profile| {
                            profile
                                .validate_task_package(&task.task_package)
                                .map_err(|error| error.to_string())
                        })
                }
                "terminal-bench-3.0" => {
                    crate::benchmark::terminal_bench_30::Tb30Profile::parse(&profile_bytes[..])
                        .map_err(|error| error.to_string())
                        .and_then(|profile| {
                            profile
                                .admit_task_package(&task.task_package)
                                .map(|_| ())
                                .map_err(|error| error.to_string())
                        })
                }
                _ => crate::benchmark::deepswe::DeepSweProfile::parse(&profile_bytes[..])
                    .map_err(|error| error.to_string())
                    .and_then(|profile| {
                        profile
                            .admit_task_package(&task.task_package)
                            .map(|_| ())
                            .map_err(|error| error.to_string())
                    }),
            };
            admitted.map_err(MaterialError::Rejected)?;
            let source_doc: toml::Value = std::fs::read_to_string(source.join("task.toml"))?
                .parse()
                .map_err(|error| MaterialError::Rejected(format!("task metadata: {error}")))?;
            let native_name = source_doc
                .get("task")
                .and_then(|task| task.get("name").or_else(|| task.get("id")))
                .and_then(toml::Value::as_str)
                .unwrap_or(id);
            if task.native_task_name != native_name {
                return Err(MaterialError::Rejected(format!(
                    "{id} native task name differs from pinned corpus"
                )));
            }
            verify_materialized_package(&source, &task.task_package, &lock)?;
        }
        Ok(())
    }
}

/// Git reads use no user configuration, hooks, external diff, or network.
pub(crate) fn git_text(root: &Path, args: &[&str]) -> Result<String, MaterialError> {
    String::from_utf8(git_bytes(root, args)?)
        .map(|text| text.trim_end_matches('\n').to_owned())
        .map_err(|_| MaterialError::Rejected("corpus paths must be UTF-8".to_owned()))
}

fn git_bytes(root: &Path, args: &[&str]) -> Result<Vec<u8>, MaterialError> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_NO_LAZY_FETCH", "1")
        .output()?;
    if !output.status.success() {
        return Err(MaterialError::Rejected(format!(
            "pinned corpus Git inspection failed at {}",
            root.display()
        )));
    }
    Ok(output.stdout)
}

impl CorpusMaterial {
    pub(crate) fn verify(
        &self,
        adapter: &str,
        lock: &serde_json::Value,
    ) -> Result<(), MaterialError> {
        self.verified_manifest_digest(adapter, lock).map(|_| ())
    }

    /// Verifies the complete pinned corpus before hashing regular bytes and
    /// inert root metadata links. Link targets are checked, never transported.
    pub(crate) fn verified_manifest_digest(
        &self,
        adapter: &str,
        lock: &serde_json::Value,
    ) -> Result<String, MaterialError> {
        let pin = lock["subjects"]
            .as_array()
            .and_then(|subjects| subjects.iter().find(|subject| subject["id"] == adapter))
            .ok_or_else(|| {
                MaterialError::Rejected(format!("static lock has no {adapter} corpus"))
            })?;
        if pin["commit"].as_str() != Some(&self.source_commit)
            || pin["tasks_tree"].as_str() != Some(&self.tasks_tree)
        {
            return Err(MaterialError::Rejected(format!(
                "{adapter} corpus identity differs from static lock"
            )));
        }
        for path in self.root.ancestors() {
            if !std::fs::symlink_metadata(path)?.file_type().is_dir() {
                return Err(MaterialError::Rejected(
                    "corpus root traverses a link or non-directory".to_owned(),
                ));
            }
        }
        let project = PathBuf::from(git_text(&self.root, &["rev-parse", "--show-toplevel"])?);
        if project.join("tasks").canonicalize()? != self.root.canonicalize()?
            || git_text(&self.root, &["rev-parse", "HEAD"])? != self.source_commit
            || git_text(&self.root, &["rev-parse", "HEAD:tasks"])? != self.tasks_tree
        {
            return Err(MaterialError::Rejected(format!(
                "{adapter} corpus root/commit/tree drift"
            )));
        }
        git_text(
            &project,
            &["diff", "--no-ext-diff", "--quiet", "HEAD", "--", "tasks"],
        )?;
        for args in [
            vec!["ls-files", "--others", "--exclude-standard", "--", "tasks"],
            vec![
                "ls-files",
                "--others",
                "--ignored",
                "--exclude-standard",
                "--",
                "tasks",
            ],
        ] {
            if !git_text(&project, &args)?.is_empty() {
                return Err(MaterialError::Rejected(
                    "pinned corpus contains untracked task bytes".to_owned(),
                ));
            }
        }
        // Enumerate without following links, including unselected task trees.
        fn entries(
            root: &Path,
            dir: &Path,
            files: &mut BTreeMap<String, PathBuf>,
        ) -> Result<(), MaterialError> {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let name = entry.file_name();
                if name.to_str().is_none_or(|name| {
                    name.chars()
                        .any(|ch| ch.is_control() || matches!(ch, '\\' | ':'))
                }) {
                    return Err(MaterialError::Rejected("ambiguous corpus path".to_owned()));
                }
                let path = entry.path();
                let kind = entry.file_type()?;
                if kind.is_dir() {
                    entries(root, &path, files)?;
                } else if kind.is_file() || kind.is_symlink() {
                    files.insert(
                        path.strip_prefix(root)
                            .expect("corpus entry")
                            .to_str()
                            .expect("checked UTF-8")
                            // Git trees always key paths with '/'; a Windows
                            // walk yields MAIN_SEPARATOR-joined relatives, so
                            // normalize the key (no-op where they already
                            // match).
                            .replace(std::path::MAIN_SEPARATOR, "/"),
                        path,
                    );
                } else {
                    return Err(MaterialError::Rejected("special corpus file".to_owned()));
                }
            }
            Ok(())
        }
        let mut files = BTreeMap::new();
        entries(&self.root, &self.root, &mut files)?;
        let tree = git_text(&project, &["ls-tree", "-r", "-z", &self.tasks_tree])?;
        let mut expected = Vec::new();
        let mut paths = Vec::new();
        let mut rows = Vec::new();
        for row in tree.split('\0').filter(|row| !row.is_empty()) {
            let (identity, relative) = row
                .split_once('\t')
                .ok_or_else(|| MaterialError::Rejected("invalid corpus Git entry".to_owned()))?;
            let fields = identity.split_whitespace().collect::<Vec<_>>();
            if fields.len() != 3
                || fields[1] != "blob"
                || !matches!(fields[0], "100644" | "100755" | "120000")
            {
                return Err(MaterialError::Rejected(
                    "unsupported corpus source entry".to_owned(),
                ));
            }
            let file = files.remove(relative).ok_or_else(|| {
                MaterialError::Rejected("corpus file set differs from pinned tree".to_owned())
            })?;
            if fields[0] == "120000" {
                if relative.contains('/')
                    || !std::fs::symlink_metadata(&file)?.file_type().is_symlink()
                {
                    return Err(MaterialError::Rejected(
                        "only pinned corpus-root metadata links are admitted".to_owned(),
                    ));
                }
                let link = std::fs::read_link(&file)?;
                let text = link.to_str().ok_or_else(|| {
                    MaterialError::Rejected("metadata link target must be UTF-8".to_owned())
                })?;
                if text.is_empty()
                    || text.starts_with('/')
                    || text
                        .chars()
                        .any(|ch| ch.is_control() || matches!(ch, '\\' | ':'))
                {
                    return Err(MaterialError::Rejected(
                        "unsafe corpus metadata target".to_owned(),
                    ));
                }
                if git_bytes(&project, &["cat-file", "blob", fields[2]])? != text.as_bytes() {
                    return Err(MaterialError::Rejected(
                        "corpus metadata link text drift".to_owned(),
                    ));
                }
                let mut parts = vec!["tasks"];
                let mut components = text.split('/').peekable();
                while let Some(part) = components.next() {
                    match part {
                        "" | "." => {
                            return Err(MaterialError::Rejected(
                                "ambiguous corpus metadata target".to_owned(),
                            ));
                        }
                        ".." => {
                            if parts.pop().is_none() {
                                return Err(MaterialError::Rejected(
                                    "corpus metadata target escapes checkout".to_owned(),
                                ));
                            }
                        }
                        _ => parts.push(part),
                    }
                    if components.peek().is_some()
                        && !std::fs::symlink_metadata(project.join(parts.join("/")))?
                            .file_type()
                            .is_dir()
                    {
                        return Err(MaterialError::Rejected(
                            "metadata target traverses a link or non-directory".to_owned(),
                        ));
                    }
                }
                let target_relative = parts.join("/");
                let target_tree =
                    git_text(&project, &["ls-tree", "-z", "HEAD", "--", &target_relative])?;
                let (identity, pinned_path) = target_tree
                    .trim_end_matches('\0')
                    .split_once('\t')
                    .ok_or_else(|| {
                    MaterialError::Rejected(
                        "metadata target is not a pinned regular file".to_owned(),
                    )
                })?;
                let target_fields = identity.split_whitespace().collect::<Vec<_>>();
                if pinned_path != target_relative
                    || target_fields.len() != 3
                    || target_fields[1] != "blob"
                    || !matches!(target_fields[0], "100644" | "100755")
                {
                    return Err(MaterialError::Rejected(
                        "metadata target is not a pinned regular file".to_owned(),
                    ));
                }
                let mut target = project.clone();
                for (index, part) in parts.iter().enumerate() {
                    target.push(part);
                    let kind = std::fs::symlink_metadata(&target)?.file_type();
                    if (index + 1 < parts.len() && !kind.is_dir())
                        || (index + 1 == parts.len() && !kind.is_file())
                    {
                        return Err(MaterialError::Rejected(
                            "metadata target traverses a link or non-regular entry".to_owned(),
                        ));
                    }
                }
                verify_corpus_mode(&target, target_fields[0])?;
                expected.push(target_fields[2].to_owned());
                paths.push(target);
                rows.push((relative.to_owned(), sha256_hex(text.as_bytes())));
            } else {
                verify_corpus_mode(&file, fields[0])?;
                expected.push(fields[2].to_owned());
                rows.push((relative.to_owned(), sha256_hex(&std::fs::read(&file)?)));
                paths.push(file);
            }
        }
        if !files.is_empty() {
            return Err(MaterialError::Rejected(
                "corpus contains untracked entries".to_owned(),
            ));
        }
        let mut observed = Vec::new();
        for batch in paths.chunks(128) {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(&self.root)
                .args(["hash-object", "--no-filters", "--"])
                .args(batch)
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .output()?;
            if !output.status.success() {
                return Err(MaterialError::Rejected(
                    "cannot hash pinned corpus working bytes".to_owned(),
                ));
            }
            observed.extend(
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(str::to_owned),
            );
        }
        if observed != expected {
            return Err(MaterialError::Rejected(
                "corpus working bytes differ from pinned Git blobs".to_owned(),
            ));
        }
        rows.sort();
        Ok(sha256_hex(
            rows.iter()
                .map(|(path, digest)| format!("{path}\n{digest}\n"))
                .collect::<String>()
                .as_bytes(),
        ))
    }
}

fn verify_corpus_mode(path: &Path, mode: &str) -> Result<(), MaterialError> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(MaterialError::Rejected(
            "corpus regular file replaced by link or special file".to_owned(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if (metadata.permissions().mode() & 0o111 != 0) != (mode == "100755") {
            return Err(MaterialError::Rejected(
                "corpus source mode drift".to_owned(),
            ));
        }
    }
    #[cfg(not(unix))]
    let _ = mode;
    Ok(())
}

/// Image resolution may replace a declared image reference with the exact
/// immutable manifest from the admitted lock. All other source bytes remain
/// unchanged; resolved task packages keep their own file table and digest.
pub(crate) fn resolved_image(
    reference: &str,
    lock: &serde_json::Value,
) -> Result<String, MaterialError> {
    let immutable = |image: &str| {
        image.rsplit_once("@sha256:").is_some_and(|(name, digest)| {
            !name.is_empty()
                && digest.len() == 64
                && digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
    };
    if immutable(reference) {
        return Ok(reference.to_owned());
    }
    let image = lock["images"].as_array().and_then(|images| images.iter().find(|image| image["reference"] == reference))
        .ok_or_else(|| MaterialError::Rejected(format!("mutable/unbound task image {reference:?}; resolve its immutable image material first")))?;
    let manifest = image["manifest"].as_str().unwrap_or_default();
    let resolved = format!("{reference}@{manifest}");
    if !immutable(&resolved) {
        return Err(MaterialError::Rejected(format!(
            "image {reference:?} has no immutable manifest"
        )));
    }
    Ok(resolved)
}

pub(crate) fn materialized_task_file(
    relative: &str,
    bytes: &[u8],
    lock: &serde_json::Value,
) -> Result<Vec<u8>, MaterialError> {
    if relative == "task.toml" {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| MaterialError::Rejected("task metadata UTF-8".to_owned()))?;
        let doc: toml::Value = text
            .parse()
            .map_err(|error| MaterialError::Rejected(format!("task metadata: {error}")))?;
        let mut replacements = BTreeMap::new();
        for (section, environment) in [
            ("environment", doc.get("environment")),
            (
                "verifier.environment",
                doc.get("verifier")
                    .and_then(|value| value.get("environment")),
            ),
        ] {
            if let Some(image) = environment.and_then(|value| value.get("docker_image")) {
                let reference = image.as_str().ok_or_else(|| {
                    MaterialError::Rejected("task image must be a string".to_owned())
                })?;
                let resolved = resolved_image(reference, lock)?;
                if reference != resolved {
                    replacements.insert(section, resolved);
                }
            }
        }
        let mut output = String::new();
        let mut section = "";
        for line in text.split_inclusive('\n') {
            let trimmed = line.trim();
            if let Some(name) = trimmed
                .strip_prefix('[')
                .and_then(|name| name.strip_suffix(']'))
            {
                section = name;
            }
            if line
                .split_once('=')
                .is_some_and(|(key, _)| key.trim() == "docker_image")
                && let Some(image) = replacements.remove(section)
            {
                output.push_str(&format!("docker_image = {image:?}\n"));
            } else {
                output.push_str(line);
            }
        }
        if !replacements.is_empty() {
            return Err(MaterialError::Rejected(
                "unsupported image declaration layout".to_owned(),
            ));
        }
        return Ok(output.into_bytes());
    }
    if Path::new(relative)
        .file_name()
        .is_some_and(|name| name == "Dockerfile")
    {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| MaterialError::Rejected("Dockerfile UTF-8".to_owned()))?;
        let mut output = String::new();
        let mut stages = std::collections::BTreeSet::new();
        for line in text.lines() {
            let words = line.split_whitespace().collect::<Vec<_>>();
            if words
                .first()
                .is_some_and(|word| word.eq_ignore_ascii_case("FROM"))
            {
                if words.len() != 2 && !(words.len() == 4 && words[2].eq_ignore_ascii_case("AS")) {
                    return Err(MaterialError::Rejected(
                        "unsupported Dockerfile FROM shape; immutable image identity is unproven"
                            .to_owned(),
                    ));
                }
                let image = if words[1] == "scratch" || stages.contains(words[1]) {
                    words[1].to_owned()
                } else {
                    resolved_image(words[1], lock)?
                };
                output.push_str(&format!("FROM {image}"));
                if words.len() == 4 {
                    stages.insert(words[3]);
                    output.push_str(&format!(" AS {}", words[3]));
                }
            } else {
                output.push_str(line);
            }
            output.push('\n');
        }
        return Ok(output.into_bytes());
    }
    Ok(bytes.to_vec())
}

fn verify_materialized_package(
    source: &Path,
    package: &Path,
    lock: &serde_json::Value,
) -> Result<(), MaterialError> {
    runtime_package_manifest_digest(package)?;
    let mut files = Vec::new();
    visit_files(source, source, &mut files)?;
    let mut observed = Vec::new();
    visit_files(package, package, &mut observed)?;
    files.sort();
    observed.sort();
    if files.iter().map(|(path, _)| path).collect::<Vec<_>>()
        != observed.iter().map(|(path, _)| path).collect::<Vec<_>>()
    {
        return Err(MaterialError::Rejected(
            "materialized task file set differs from pinned source".to_owned(),
        ));
    }
    for (relative, _) in files {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let source_mode = std::fs::metadata(source.join(&relative))?
                .permissions()
                .mode()
                & 0o111;
            let package_mode = std::fs::metadata(package.join(&relative))?
                .permissions()
                .mode()
                & 0o111;
            if source_mode != package_mode {
                return Err(MaterialError::Rejected(format!(
                    "materialized task {relative} executable mode differs from pinned source"
                )));
            }
        }
        let expected =
            materialized_task_file(&relative, &std::fs::read(source.join(&relative))?, lock)?;
        if std::fs::read(package.join(&relative))? != expected {
            return Err(MaterialError::Rejected(format!(
                "materialized task {relative} differs from pinned source and image resolution"
            )));
        }
    }
    Ok(())
}

/// Resolve every task and Dockerfile image through the admitted immutable lock.
pub(crate) fn task_images(
    package: &Path,
    lock: &serde_json::Value,
) -> Result<std::collections::BTreeSet<String>, MaterialError> {
    let mut images = std::collections::BTreeSet::new();
    let metadata = materialized_task_file(
        "task.toml",
        &std::fs::read(package.join("task.toml"))?,
        lock,
    )?;
    let doc: toml::Value = std::str::from_utf8(&metadata)
        .map_err(|_| MaterialError::Rejected("task metadata UTF-8".to_owned()))?
        .parse()
        .map_err(|error| MaterialError::Rejected(format!("task metadata: {error}")))?;
    for env in [
        doc.get("environment"),
        doc.get("verifier")
            .and_then(|value| value.get("environment")),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(image) = env.get("docker_image").and_then(toml::Value::as_str) {
            images.insert(image.to_owned());
        }
    }
    let mut files = Vec::new();
    visit_files(package, package, &mut files)?;
    for (relative, _) in files {
        if Path::new(&relative)
            .file_name()
            .is_none_or(|name| name != "Dockerfile")
        {
            continue;
        }
        let bytes =
            materialized_task_file(&relative, &std::fs::read(package.join(&relative))?, lock)?;
        for line in String::from_utf8_lossy(&bytes).lines() {
            let words = line.split_whitespace().collect::<Vec<_>>();
            if words.first() == Some(&"FROM")
                && words.get(1).is_some_and(|image| image.contains("@sha256:"))
            {
                images.insert(words[1].to_owned());
            }
        }
    }
    Ok(images)
}

/// Inspect immutable image identities already present at the selected engine.
/// This observation never pulls an image or starts a container.
pub(crate) fn image_preflight(
    package: &Path,
    lock: &serde_json::Value,
    environment: &BTreeMap<String, String>,
) -> serde_json::Value {
    let images = match task_images(package, lock) {
        Ok(images) => images,
        Err(error) => return serde_json::json!({"status": "unmet", "reason": error.to_string()}),
    };
    let entries = images.iter().map(|image| {
        let output = super::prepare::observation_command(
            ["docker", "image", "inspect", "--format", "{{json .RepoDigests}}", image]
                .into_iter().map(Into::into).collect(), environment, std::time::Duration::from_secs(5));
        let digests = output.and_then(|output| serde_json::from_slice::<Vec<String>>(&output).ok());
        let present = digests.as_ref().is_some_and(|digests| digests.iter().any(|digest| digest.rsplit_once('@').map(|(_, digest)| digest) == image.rsplit_once('@').map(|(_, digest)| digest)));
        serde_json::json!({"image": image, "status": if present {"known"} else {"unknown"}, "source": "selected-engine:docker-image-inspect",
            "reason": if present { None } else { Some("immutable image missing or unobservable; populate the selected engine cache before freezing") }})
    }).collect::<Vec<_>>();
    let missing = entries
        .iter()
        .filter(|entry| entry["status"] != "known")
        .filter_map(|entry| entry["image"].as_str())
        .collect::<Vec<_>>();
    serde_json::json!({"status": if missing.is_empty() {"ok"} else {"unknown"}, "reason": if missing.is_empty() {String::new()} else {format!("immutable images missing/unobserved: {}; populate the selected engine cache", missing.join(", "))}, "images": entries})
}

/// Materializes one selected task from verified source. Existing cached bytes
/// are reusable only when identical; a stale package is never overwritten.
pub(crate) fn materialize_task(
    corpus: &CorpusMaterial,
    id: &str,
    template: &Path,
    package: &Path,
    profile_path: &Path,
    lock: &serde_json::Value,
) -> Result<TaskMaterial, MaterialError> {
    let source = corpus.root.join(id);
    runtime_package_manifest_digest(&source)?;
    let mut files = Vec::new();
    visit_files(&source, &source, &mut files)?;
    files.sort();
    let mut rows = Vec::new();
    for (relative, _) in files {
        let source_file = source.join(&relative);
        let bytes = materialized_task_file(&relative, &std::fs::read(&source_file)?, lock)?;
        let target = package.join(&relative);
        if target.exists() {
            if std::fs::read(&target)? != bytes {
                return Err(MaterialError::Rejected(format!(
                    "cached task drift at {}; prepare in a fresh cache",
                    target.display()
                )));
            }
        } else {
            std::fs::create_dir_all(target.parent().expect("package file parent"))?;
            std::fs::write(&target, &bytes)?;
            std::fs::set_permissions(&target, std::fs::metadata(&source_file)?.permissions())?;
        }
        #[cfg(unix)]
        let executable = {
            use std::os::unix::fs::PermissionsExt;
            std::fs::metadata(&source_file)?.permissions().mode() & 0o111 != 0
        };
        #[cfg(not(unix))]
        let executable = false;
        rows.push(
            serde_json::json!({"path": relative, "mode": if executable {"100755"} else {"100644"},
            "size": bytes.len(), "sha256": sha256_hex(&bytes)}),
        );
    }
    verify_materialized_package(&source, package, lock)?;
    let mut profile: toml::Value = std::fs::read_to_string(template)?
        .parse()
        .map_err(|error| MaterialError::Rejected(format!("profile template: {error}")))?;
    let identity = profile
        .get_mut("identity")
        .and_then(toml::Value::as_table_mut)
        .ok_or_else(|| MaterialError::Rejected("profile template has no identity".to_owned()))?;
    for (field, value) in [
        ("source_commit", corpus.source_commit.clone()),
        ("tasks_tree", corpus.tasks_tree.clone()),
        ("task_id", id.to_owned()),
        (
            "task_tree",
            git_text(
                &corpus.root,
                &["rev-parse", &format!("{}:{id}", corpus.tasks_tree)],
            )?,
        ),
        (
            "package_manifest_sha256",
            sha256_hex(
                serde_json::to_string(&rows)
                    .expect("file table serializes")
                    .as_bytes(),
            ),
        ),
    ] {
        identity.insert(field.to_owned(), value.into());
    }
    profile["package"] =
        toml::Value::try_from(&rows).map_err(|error| MaterialError::Rejected(error.to_string()))?;
    let bytes = toml::to_string(&profile)
        .map_err(|error| MaterialError::Rejected(error.to_string()))?
        .into_bytes();
    if profile_path.exists() && std::fs::read(profile_path)? != bytes {
        return Err(MaterialError::Rejected(format!(
            "cached profile drift at {}",
            profile_path.display()
        )));
    }
    std::fs::create_dir_all(profile_path.parent().expect("profile parent"))?;
    std::fs::write(profile_path, &bytes)?;
    let metadata: toml::Value = std::fs::read_to_string(source.join("task.toml"))?
        .parse()
        .map_err(|error| MaterialError::Rejected(format!("task metadata: {error}")))?;
    let name = metadata
        .get("task")
        .and_then(|task| task.get("name").or_else(|| task.get("id")))
        .and_then(toml::Value::as_str)
        .unwrap_or(id);
    Ok(TaskMaterial {
        native_task_name: name.to_owned(),
        profile: PinnedFile {
            path: profile_path.canonicalize()?,
            sha256: sha256_hex(&bytes),
        },
        task_package: package.canonicalize()?,
        task_package_manifest_sha256: task_package_manifest_digest(package)?,
    })
}

impl RuntimePackage {
    /// Binds a regular-file resource tree to the exact executable entrypoint.
    pub(crate) fn verify(&self, executable: &PinnedFile) -> Result<(), MaterialError> {
        if !self.path.is_absolute()
            || self.entrypoint.as_os_str().is_empty()
            || self
                .entrypoint
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
            || self.entrypoint.to_str().is_none_or(|path| {
                path.chars()
                    .any(|c| c.is_control() || matches!(c, '\\' | ':'))
                    || path.split('/').any(|part| part.is_empty() || part == ".")
            })
        {
            return Err(MaterialError::Rejected("runtime package requires an absolute root and relative entrypoint without traversal".to_owned()));
        }
        let observed = runtime_package_manifest_digest(&self.path)?;
        if observed != self.manifest_sha256 {
            return Err(MaterialError::Rejected(format!(
                "runtime package digest drift at {}",
                self.path.display()
            )));
        }
        let entrypoint = self.path.join(&self.entrypoint);
        if !entrypoint.is_file() || entrypoint.canonicalize()? != executable.path.canonicalize()? {
            return Err(MaterialError::Rejected(
                "runtime package entrypoint differs from the pinned executable".to_owned(),
            ));
        }
        verify_pinned(executable)
    }
}

/// Runtime packages reject links, special files and ambiguous manifest paths.
pub(crate) fn runtime_package_manifest_digest(root: &Path) -> Result<String, MaterialError> {
    // Same admission rule as corpus roots: verify the resolved path so a
    // host temp layout that traverses links (macOS `/var`) is not misread
    // as package aliasing.
    let root = root.canonicalize()?;
    fn check(path: &Path) -> Result<(), MaterialError> {
        let kind = std::fs::symlink_metadata(path)?.file_type();
        if kind.is_symlink() || !(kind.is_file() || kind.is_dir()) {
            return Err(MaterialError::Rejected(format!(
                "runtime package contains a link or special file: {}",
                path.display()
            )));
        }
        if kind.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let name = entry.file_name();
                if name.to_str().is_none_or(|name| {
                    name.chars()
                        .any(|c| c.is_control() || matches!(c, '\\' | ':'))
                }) {
                    return Err(MaterialError::Rejected(
                        "runtime package contains an ambiguous path".to_owned(),
                    ));
                }
                check(&entry.path())?;
            }
        }
        Ok(())
    }
    for path in root.ancestors() {
        if std::fs::symlink_metadata(path)?.file_type().is_symlink() {
            return Err(MaterialError::Rejected(
                "runtime package root traverses a link".to_owned(),
            ));
        }
    }
    check(&root)?;
    directory_manifest_digest(&root)
}

/// Verifies the selected checkout against the static source and lock pins.
/// Git metadata and the installed virtual environment are not source bytes.
pub(crate) fn runner_source_digest(
    project: &Path,
    commit: &str,
    lock_blob: &str,
    lock_sha256: &str,
) -> Result<String, MaterialError> {
    let reject =
        |reason: &str| MaterialError::Rejected(format!("runner {}: {reason}", project.display()));
    if std::fs::symlink_metadata(project)?.file_type().is_symlink() {
        return Err(reject(
            "project must be a directory inside the cache, not a link",
        ));
    }
    let git = |args: &[&str]| -> Result<Vec<u8>, MaterialError> {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(project)
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .map_err(|_| reject("cannot inspect the seeded Git checkout"))?;
        if !output.status.success() {
            return Err(reject(
                "source inspection failed; seed a clean pinned Git checkout",
            ));
        }
        Ok(output.stdout)
    };
    let git_text = |args: &[&str]| -> Result<String, MaterialError> {
        String::from_utf8(git(args)?)
            .map(|value| value.trim().to_owned())
            .map_err(|_| reject("Git identity is not UTF-8"))
    };
    let root = PathBuf::from(git_text(&["rev-parse", "--show-toplevel"])?);
    if root.canonicalize()? != project.canonicalize()? {
        return Err(reject("project is not its own Git root"));
    }
    if git_text(&["rev-parse", "HEAD"])? != commit {
        return Err(reject("source commit differs from the static lock"));
    }
    if git_text(&["rev-parse", "HEAD:uv.lock"])? != lock_blob
        || sha256_hex(&std::fs::read(project.join("uv.lock"))?) != lock_sha256
    {
        return Err(reject("uv.lock differs from the static lock"));
    }
    git(&["diff", "--no-ext-diff", "--quiet", "HEAD", "--"])?;
    if !git(&["ls-files", "--others", "--exclude-standard", "-z"])?.is_empty() {
        return Err(reject("checkout contains untracked source files"));
    }
    let tree = git(&["ls-tree", "-r", "-z", "--full-tree", "HEAD"])?;
    let tree = std::str::from_utf8(&tree).map_err(|_| reject("source tree is not UTF-8"))?;
    let mut rows = Vec::new();
    for row in tree.split('\0').filter(|row| !row.is_empty()) {
        let (identity, relative) = row
            .split_once('\t')
            .ok_or_else(|| reject("invalid source tree entry"))?;
        if relative.chars().any(char::is_control)
            || Path::new(relative)
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return Err(reject("unsafe source tree path"));
        }
        let file = project.join(relative);
        let kind = std::fs::symlink_metadata(&file)?.file_type();
        let digest = if identity.starts_with("120000 blob ") && kind.is_symlink() {
            sha256_hex(std::fs::read_link(&file)?.as_os_str().as_encoded_bytes())
        } else if (identity.starts_with("100644 blob ") || identity.starts_with("100755 blob "))
            && kind.is_file()
        {
            sha256_hex(&std::fs::read(&file)?)
        } else {
            return Err(reject("source entry type differs from the pinned tree"));
        };
        rows.push(format!("{identity}\t{relative}\n{digest}\n"));
    }
    rows.sort();
    Ok(sha256_hex(rows.concat().as_bytes()))
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
        let project = dir.join("corpus");
        let task = project.join("tasks/unit-fixture-task");
        write(&task.join("instruction.md"), b"instruction");
        write(
            &task.join("task.toml"),
            b"[task]\nid = \"unit-fixture-task\"\n",
        );
        let git = |args: &[&str]| {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(&project)
                .args(args)
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8(output.stdout).unwrap().trim().to_owned()
        };
        git(&["init", "--quiet"]);
        git(&["add", "--", "tasks"]);
        if !project.join(".git/refs/heads/master").exists()
            && !project.join(".git/refs/heads/main").exists()
        {
            git(&[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ]);
        }
        let corpus = CorpusMaterial {
            root: project.join("tasks"),
            source_commit: git(&["rev-parse", "HEAD"]),
            tasks_tree: git(&["rev-parse", "HEAD:tasks"]),
        };
        let lock_value = serde_json::json!({"subjects": [{"id": "terminal-bench-2.1", "commit": corpus.source_commit, "tasks_tree": corpus.tasks_tree}]});
        std::fs::write(dir.join("helper"), b"helper").unwrap();
        let lock = dir.join("lock.json");
        write(
            &lock,
            serde_json::to_string(&lock_value).unwrap().as_bytes(),
        );
        for (name, bytes) in [
            ("provider.py", "provider"),
            ("opi", "opi-bin"),
            ("pi.js", "pi-bin"),
            ("uv", "uv-bin"),
            ("oracle.sh", "oracle"),
            ("integrations/opi_eval_harbor.py", "# pinned integration\n"),
        ] {
            write(&dir.join(name), bytes.as_bytes());
        }
        let template = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/benchmarks/terminal-bench-2.1/profile/synthetic.toml");
        let task_material = materialize_task(
            &corpus,
            "unit-fixture-task",
            &template,
            &dir.join("package"),
            &dir.join("profile.toml"),
            &lock_value,
        )
        .unwrap();
        let pinned = |name: &str| {
            let path = dir.join(name);
            serde_json::json!({"path": path, "sha256": sha256_hex(&std::fs::read(&path).unwrap())})
        };
        let integrations = dir.join("integrations");
        let agent = |product: &str, executable: &str| {
            let integration = |family: &str| {
                serde_json::json!({"module_dir": integrations,
                "module_manifest_sha256": directory_manifest_digest(&integrations).unwrap(),
                "import_path": format!("opi_eval_{family}:{product}Agent"), "agent_name": format!("opi-eval-{product}")})
            };
            serde_json::json!({"executable": pinned(executable), "integrations": {"harbor": integration("harbor"), "pier": integration("pier")}})
        };
        serde_json::json!({"schema": MATERIAL_SCHEMA, "static_lock": pinned("lock.json"),
            "capture_helper": {"executable": pinned("helper"), "target": "x86_64-unknown-linux-gnu",
                "protocol": crate::native_capture::CAPTURE_HELPER_PROTOCOL,
                "cargo_lock_sha256": "0".repeat(64)},
            "provider": {"script": pinned("provider.py"), "endpoint": "http://127.0.0.1:48127/v1", "request_log": dir.join("requests.jsonl")},
            "agents": {"opi": agent("opi", "opi"), "pi": agent("pi", "pi.js")},
            "benchmarks": {"terminal-bench-2.1": {"corpus": corpus, "tasks": {"unit-fixture-task": task_material},
                "verifier_executable": pinned("uv"), "verifier_env": {}, "oracle": pinned("oracle.sh"), "oracle_env": {}}}
        }).to_string()
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
        let text = minimal_manifest(&dir).replace(MATERIAL_SCHEMA, "other/1");
        write(&manifest, text.as_bytes());
        assert!(
            NativeMaterial::load(&manifest)
                .expect_err("schema rejected")
                .to_string()
                .contains("unknown native-material schema")
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
    fn mutable_images_need_an_exact_lock_binding() {
        let digest = format!("sha256:{}", "a".repeat(64));
        let lock = serde_json::json!({"images": [{"reference": "base:1", "manifest": digest}]});
        assert_eq!(
            resolved_image("base:1", &lock).unwrap(),
            format!("base:1@{digest}")
        );
        for reference in ["base:2", "base:1@sha256:bad", "unbound"] {
            assert!(resolved_image(reference, &lock).is_err());
        }
        let raw = b"[environment]\ndocker_image = 'base:1'\n[verifier.environment]\ndocker_image = 'base:2'\n";
        assert!(materialized_task_file("task.toml", raw, &lock).is_err());
        assert!(materialized_task_file("environment/Dockerfile", b"FROM base:2\n", &lock).is_err());
        assert_eq!(
            materialized_task_file(
                "environment/Dockerfile",
                b"FROM base:1 AS build\nFROM build\n",
                &lock
            )
            .unwrap(),
            format!("FROM base:1@{digest} AS build\nFROM build\n").as_bytes()
        );
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
