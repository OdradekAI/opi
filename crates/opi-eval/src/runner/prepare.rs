//! Local material preparation (P19-D05).
//!
//! [`doctor`] resolves the experiment document and reports environment
//! and material observations without starting a model: engine
//! reachability, host architecture, WSL/Docker-visible resources,
//! missing materials, and explicit `unknown` observations with reasons.
//! [`prepare`] (added with the local material freeze) consumes the same
//! observation core. Every observation is a truthful report, never a
//! silent substitute: an unreachable engine yields `unknown` GPU state
//! with a reason, and a missing cache entry yields `missing` with its
//! expected path.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::experiment::ResolvedExperiment;
use crate::runner::material::{directory_manifest_digest, sha256_hex};

/// Failures of the local preparation commands.
#[derive(Debug, Error)]
pub(crate) enum PrepareError {
    /// The experiment document could not be read.
    #[error("cannot read experiment document: {0}")]
    Io(#[from] std::io::Error),
    /// The document could not be resolved into a frozen contract.
    #[error("cannot resolve experiment document: {0}")]
    Resolve(#[from] crate::experiment::ResolveError),
    /// The document or the requested preparation is rejected.
    #[error("preparation rejected: {0}")]
    Rejected(String),
}

/// One doctor observation: a status plus optional facts and reason.
#[derive(Debug, Serialize)]
pub(crate) struct Observation {
    /// Stable observation id (`linux-engine`, `material:agent:pi`, ...).
    pub(crate) id: String,
    /// `ok`, `unreachable`, `missing`, `unknown`, `unsupported`, ...
    pub(crate) status: &'static str,
    /// Supporting fact (architecture string, runtime name, ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) detail: Option<String>,
    /// Why the observation could not be made.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reason: Option<String>,
    /// Observed logical CPU count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) count: Option<u64>,
    /// Observed total memory in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) total_bytes: Option<u64>,
    /// Observed available disk bytes for the cache location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) available_bytes: Option<u64>,
}

impl Observation {
    fn new(id: &str, status: &'static str) -> Self {
        Self {
            id: id.to_owned(),
            status,
            detail: None,
            reason: None,
            count: None,
            total_bytes: None,
            available_bytes: None,
        }
    }
}

/// The doctor report schema identity.
pub(crate) const DOCTOR_SCHEMA: &str = "opi-eval-doctor/1";

/// The resolved local context every preparation command shares: the
/// frozen experiment contract, the cache directory, and the selected
/// agent products and benchmark revision.
pub(crate) struct PreparedContext {
    /// The frozen experiment document.
    pub(crate) experiment: ResolvedExperiment,
    /// Absolute local material cache directory (`[storage].cache`,
    /// resolved against the document's directory).
    pub(crate) cache: PathBuf,
}

impl PreparedContext {
    /// Resolves the document and its cache location.
    pub(crate) fn load(config_path: &Path) -> Result<Self, PrepareError> {
        let source = std::fs::read_to_string(config_path)?;
        let experiment = ResolvedExperiment::resolve(&source)?;
        let cache = resolve_cache(config_path, &experiment);
        Ok(Self { experiment, cache })
    }
}

/// `[storage].cache` is a directory name resolved against the
/// experiment document's directory.
fn resolve_cache(config_path: &Path, experiment: &ResolvedExperiment) -> PathBuf {
    let declared = Path::new(&experiment.storage().cache);
    if declared.is_absolute() {
        declared.to_owned()
    } else {
        config_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(declared)
    }
}

/// Runs `doctor --config`: reports every observation, starts no model,
/// and mutates nothing.
pub(crate) fn doctor(config_path: &Path) -> Result<serde_json::Value, PrepareError> {
    let context = PreparedContext::load(config_path)?;
    let mut observations = Vec::new();

    // 1. Linux engine reachability. The Docker client honors
    //    DOCKER_HOST; a missing client, a failed probe, or an unusable
    //    endpoint is `unreachable` with the reason, never a guess.
    let engine = probe_engine();
    let mut engine_observation = Observation::new(
        "linux-engine",
        if engine.is_ok() {
            "reachable"
        } else {
            "unreachable"
        },
    );
    match &engine {
        Ok(identity) => engine_observation.detail = Some(identity.clone()),
        Err(reason) => engine_observation.reason = Some(reason.clone()),
    }
    let engine_reachable = engine.is_ok();
    observations.push(engine_observation);

    // 2. Host architecture: preparation is admitted for x86_64 only.
    let mut architecture = Observation::new(
        "architecture",
        if cfg!(target_arch = "x86_64") {
            "ok"
        } else {
            "unsupported"
        },
    );
    architecture.detail = Some(std::env::consts::ARCH.to_owned());
    observations.push(architecture);

    // 3. Logical CPUs from /proc.
    observations.push(match logical_cpu_count() {
        Some(count) => {
            let mut observation = Observation::new("cpu", "ok");
            observation.count = Some(count);
            observation
        }
        None => unknown("cpu", "/proc/cpuinfo is unavailable on this host"),
    });

    // 4. Total memory from /proc.
    observations.push(match total_memory_bytes() {
        Some(total_bytes) => {
            let mut observation = Observation::new("memory", "ok");
            observation.total_bytes = Some(total_bytes);
            observation
        }
        None => unknown("memory", "/proc/meminfo is unavailable on this host"),
    });

    // 5. GPU: observable only through the engine's runtime registry.
    observations.push(if !engine_reachable {
        unknown("gpu", "engine unreachable; GPU state cannot be observed")
    } else {
        match gpu_runtime() {
            Some(runtime) => {
                let mut observation = Observation::new("gpu", "available");
                observation.detail = Some(runtime);
                observation
            }
            None => {
                let mut observation = Observation::new("gpu", "absent");
                observation.reason = Some("engine reports no GPU runtime".to_owned());
                observation
            }
        }
    });

    // 6. Disk space at the cache location (nearest existing ancestor).
    let probe_root = nearest_existing_ancestor(&context.cache);
    observations.push(match available_disk_bytes(&probe_root) {
        Some(available_bytes) => {
            let mut observation = Observation::new("disk", "ok");
            observation.detail = Some(probe_root.to_string_lossy().into_owned());
            observation.available_bytes = Some(available_bytes);
            observation
        }
        None => unknown(
            "disk",
            "df -B1 -P is unavailable or failed for the cache location",
        ),
    });

    // 7. Declared materials at their pinned cache locations.
    let mut static_lock = Observation::new(
        "material:static-lock",
        if static_lock_path(&context.cache).is_file() {
            "present"
        } else {
            "missing"
        },
    );
    static_lock.detail = Some(
        static_lock_path(&context.cache)
            .to_string_lossy()
            .into_owned(),
    );
    observations.push(static_lock);
    let mut products: Vec<&str> = context
        .experiment
        .subjects()
        .iter()
        .map(|subject| subject.product.as_str())
        .collect();
    products.sort_unstable();
    products.dedup();
    for product in products {
        let executable = agent_executable_path(&context.cache, product);
        let mut observation = Observation::new(
            &format!("material:agent:{product}"),
            if executable.is_file() {
                "present"
            } else {
                "missing"
            },
        );
        observation.detail = Some(executable.to_string_lossy().into_owned());
        observations.push(observation);
    }

    Ok(serde_json::json!({
        "schema": DOCTOR_SCHEMA,
        "experiment": context.experiment.experiment_id(),
        "observations": observations,
    }))
}

/// The pinned static lock location inside the cache.
pub(crate) fn static_lock_path(cache: &Path) -> PathBuf {
    cache.join("pins/static/linux-x86_64.json")
}

/// One selected product's built executable location inside the cache.
pub(crate) fn agent_executable_path(cache: &Path, product: &str) -> PathBuf {
    cache.join("agents").join(product)
}

fn unknown(id: &str, reason: &str) -> Observation {
    let mut observation = Observation::new(id, "unknown");
    observation.reason = Some(reason.to_owned());
    observation
}

/// Probes the local Linux engine; returns its `os/arch` identity.
fn probe_engine() -> Result<String, String> {
    let output = Command::new("docker")
        .args(["version", "--format", "{{.Server.Os}}/{{.Server.Arch}}"])
        .output()
        .map_err(|error| format!("docker client not found: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = stderr
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("");
        let reason = reason.chars().take(200).collect::<String>();
        return Err(format!("engine probe failed: {reason}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// The engine's GPU runtime name, when one is registered.
pub(crate) fn gpu_runtime() -> Option<String> {
    let output = Command::new("docker")
        .args([
            "info",
            "--format",
            "{{range $name, $_ := .Runtimes}}{{$name}} {{end}}",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let runtimes = String::from_utf8_lossy(&output.stdout);
    runtimes
        .split_whitespace()
        .find(|runtime| runtime.contains("nvidia"))
        .map(|runtime| runtime.to_owned())
}

pub(crate) fn logical_cpu_count() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    let count = text
        .lines()
        .filter(|line| line.starts_with("processor"))
        .count();
    if count == 0 { None } else { Some(count as u64) }
}

pub(crate) fn total_memory_bytes() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = text.lines().find(|line| line.starts_with("MemTotal:"))?;
    let kib = line.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    Some(kib * 1024)
}

fn available_disk_bytes(path: &Path) -> Option<u64> {
    let output = Command::new("df")
        .args(["-B1", "-P", &path.to_string_lossy()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().nth(1)?;
    line.split_whitespace().nth(3)?.parse::<u64>().ok()
}

/// The deepest existing ancestor of `path` (the path itself when it
/// exists; the current directory as the final fallback).
fn nearest_existing_ancestor(path: &Path) -> PathBuf {
    let mut current = path.to_owned();
    while !current.exists() {
        match current.parent() {
            Some(parent) => current = parent.to_owned(),
            None => return PathBuf::from("."),
        }
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_existing_ancestor_walks_up() {
        let dir = tempfile::tempdir().unwrap();
        let deep = dir.path().join("a/b/c");
        assert_eq!(nearest_existing_ancestor(&deep), dir.path());
        assert_eq!(nearest_existing_ancestor(dir.path()), dir.path());
    }
}

/// The frozen local-inventory schema identity.
pub(crate) const INVENTORY_SCHEMA: &str = "opi-eval-local-inventory/1";

/// The prepare report schema identity.
pub(crate) const PREPARE_SCHEMA: &str = "opi-eval-prepare/1";

/// The one admitted static-lock schema identity.
const STATIC_LOCK_SCHEMA: &str = "opi-eval-external-lock/static/1";

/// The fields `prepare` consumes from the seeded static lock. The lock's
/// full schema is owned by the pinned producers; this projection is
/// deliberately partial and non-strict.
#[derive(Debug, Deserialize)]
struct StaticLockProjection {
    #[serde(default)]
    schema: Option<String>,
    #[serde(default)]
    authority: AuthorityProjection,
    #[serde(default)]
    images: Vec<ImageProjection>,
}

#[derive(Debug, Default, Deserialize)]
struct AuthorityProjection {
    #[serde(default)]
    producers: Vec<ProducerRecord>,
}

#[derive(Debug, Deserialize)]
struct ProducerRecord {
    path: String,
    #[serde(default)]
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct ImageProjection {
    id: String,
    #[serde(default)]
    manifest: Option<String>,
}

/// One frozen inventory entry: the observed digest of a verified
/// material identity.
#[derive(Debug, Serialize, Deserialize)]
struct InventoryEntry {
    sha256: String,
}

/// Runs `prepare --config C --out M`: verifies the seeded cache against
/// the pinned static lock, freezes the selected benchmark's and agents'
/// identities plus every requested image identity, and emits the
/// local-origin native-material manifest plus this report. Never starts
/// a model, never reads credentials, and never writes outside the
/// declared cache and output tree.
pub(crate) fn prepare(config_path: &Path, out: &Path) -> Result<serde_json::Value, PrepareError> {
    let context = PreparedContext::load(config_path)?;
    let adapter = crate::runner::experiment::adapter_key_for(&context.experiment)
        .map_err(|error| PrepareError::Rejected(error.to_string()))?;

    // 1. The pinned static lock is the local authority.
    let lock_path = static_lock_path(&context.cache);
    if !lock_path.is_file() {
        return Err(PrepareError::Rejected(format!(
            "static lock missing at {}; seed the cache pin set from a repository checkout first",
            lock_path.display()
        )));
    }
    let lock_bytes = std::fs::read(&lock_path)?;
    let lock: StaticLockProjection = serde_json::from_slice(&lock_bytes)
        .map_err(|error| PrepareError::Rejected(format!("static lock parse failed: {error}")))?;
    if lock.schema.as_deref() != Some(STATIC_LOCK_SCHEMA) {
        return Err(PrepareError::Rejected(format!(
            "static lock schema {:?} is not {STATIC_LOCK_SCHEMA:?}",
            lock.schema
        )));
    }

    // 2. Every seeded producer script must still match the digest the
    //    lock records for it.
    for producer in &lock.authority.producers {
        let name = Path::new(&producer.path)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                PrepareError::Rejected(format!(
                    "producer path {:?} has no file name",
                    producer.path
                ))
            })?;
        let seeded = context.cache.join("pins/scripts").join(name);
        if seeded.is_file() {
            // Repo-tracked text producers hash over LF-normalized bytes
            // (the convention the static lock records), so a seed from a
            // CRLF checkout still verifies.
            let bytes = std::fs::read(&seeded)?;
            let observed = sha256_hex(&crlf_to_lf(&bytes));
            if observed != producer.sha256 {
                return Err(PrepareError::Rejected(format!(
                    "producer script drift at {}: lock pins {}, observed {}",
                    seeded.display(),
                    producer.sha256,
                    observed
                )));
            }
        }
    }

    // 3. Selection: the config's benchmark revision and subject products.
    let mut products: Vec<String> = context
        .experiment
        .subjects()
        .iter()
        .map(|subject| subject.product.clone())
        .collect();
    products.sort();
    products.dedup();

    // 4. Image identities resolve before the freeze: every admitted
    //    image must carry an immutable manifest digest.
    let mut images = Vec::new();
    for image in &lock.images {
        let Some(manifest) = &image.manifest else {
            return Err(PrepareError::Rejected(format!(
                "image {} has no resolved manifest digest; resolve image identities before freeze",
                image.id
            )));
        };
        images.push(serde_json::json!({"id": image.id, "manifest": manifest}));
    }

    // 5. Required material identities at their pinned cache locations.
    let profile = context
        .cache
        .join("pins/profiles/benchmarks")
        .join(format!("{adapter}.toml"));
    let task_package = context.cache.join("packages").join(&adapter);
    let verifier = context.cache.join("tools/uv");
    let oracle = context
        .cache
        .join("wrappers")
        .join(format!("{adapter}-oracle.sh"));
    let provider = context.cache.join("pins/scripts/scripted-provider.py");
    let required: Vec<(String, PathBuf)> = std::iter::once((
        "pins:static-lock".to_owned(),
        static_lock_path(&context.cache),
    ))
    .chain(products.iter().map(|product| {
        (
            format!("agents:{product}"),
            agent_executable_path(&context.cache, product),
        )
    }))
    .chain([
        ("profiles".to_owned(), profile.clone()),
        ("tools:uv".to_owned(), verifier.clone()),
        ("wrappers:oracle".to_owned(), oracle.clone()),
        (
            "pins:scripts:scripted-provider.py".to_owned(),
            provider.clone(),
        ),
        (
            "integrations:harbor".to_owned(),
            context.cache.join("pins/integrations/harbor"),
        ),
        (
            "integrations:pier".to_owned(),
            context.cache.join("pins/integrations/pier"),
        ),
    ])
    .chain([(format!("packages:{adapter}"), task_package.clone())])
    .collect();
    let missing: Vec<(&String, &PathBuf)> = required
        .iter()
        .map(|(key, path)| (key, path))
        .filter(|(key, path)| {
            let present = if key.starts_with("integrations:") || key.starts_with("packages:") {
                path.is_dir()
            } else {
                path.is_file()
            };
            !present
        })
        .collect();

    // Supervised acquisition: with the pinned materializer seeded, a
    // cache miss invokes it exactly once under supervision - the plan
    // names every missing identity, the invocation log is captured
    // under the output root, and every acquired byte is re-verified by
    // the normal freeze below. Without a seeded materializer the miss
    // is an actionable rejection, never a silent fetch.
    let mut acquired: Vec<String> = Vec::new();
    if !missing.is_empty() {
        let materializer = context
            .cache
            .join("pins/scripts/materialize-external-locks.sh");
        if !materializer.is_file() {
            let (key, path) = &missing[0];
            return Err(PrepareError::Rejected(format!(
                "material {key} missing at {}; populate the cache (the pinned materializer \
                 acquires it under supervision) and retry",
                path.display()
            )));
        }
        std::fs::create_dir_all(out)?;
        let plan_path = out.join("acquisition-plan.json");
        let plan: Vec<serde_json::Value> = missing
            .iter()
            .map(|(key, path)| {
                let relative = path
                    .strip_prefix(&context.cache)
                    .expect("required paths live inside the cache")
                    .to_string_lossy()
                    .replace('\\', "/");
                let _ = key;
                serde_json::json!({"id": relative})
            })
            .collect();
        std::fs::write(
            &plan_path,
            serde_json::to_string_pretty(&plan).expect("acquisition plan serializes"),
        )?;
        let invocation = Command::new(&materializer)
            .arg("prepare")
            .arg("--cache")
            .arg(&context.cache)
            .arg("--plan")
            .arg(&plan_path)
            .output()
            .map_err(|error| {
                PrepareError::Rejected(format!(
                    "cannot invoke the pinned materializer at {}: {error}",
                    materializer.display()
                ))
            })?;
        let log = format!(
            "$ {} prepare --cache {} --plan {}\n--- stdout ---\n{}--- stderr ---\n{}",
            materializer.display(),
            context.cache.display(),
            plan_path.display(),
            String::from_utf8_lossy(&invocation.stdout),
            String::from_utf8_lossy(&invocation.stderr),
        );
        std::fs::write(out.join("acquisition.log"), &log)?;
        if !invocation.status.success() {
            return Err(PrepareError::Rejected(format!(
                "supervised acquisition failed (see {})",
                out.join("acquisition.log").display()
            )));
        }
        for (key, path) in &missing {
            let present = if key.starts_with("integrations:") || key.starts_with("packages:") {
                path.is_dir()
            } else {
                path.is_file()
            };
            if !present {
                return Err(PrepareError::Rejected(format!(
                    "supervised acquisition left material {key} missing at {}",
                    path.display()
                )));
            }
            acquired.push((*key).clone());
        }
        acquired.sort();
    }

    // 6. Verify every identity and freeze it into the reusable
    //    inventory; any drift against a prior freeze rejects.
    let mut inventory = read_inventory(&context.cache)?;
    let mut verified = Vec::new();
    for (key, path) in &required {
        let observed = if path.is_dir() {
            directory_manifest_digest(path)
                .map_err(|error| PrepareError::Rejected(error.to_string()))?
        } else {
            sha256_hex(&std::fs::read(path)?)
        };
        if let Some(entry) = inventory.entries.get(key) {
            if entry.sha256 != observed {
                return Err(PrepareError::Rejected(format!(
                    "material drift at {key} ({}): frozen {}, observed {}",
                    path.display(),
                    entry.sha256,
                    observed
                )));
            }
        } else {
            inventory
                .entries
                .insert(key.clone(), InventoryEntry { sha256: observed });
        }
        verified.push(key.clone());
    }
    verified.sort();
    write_inventory(&context.cache, &inventory)?;

    // 8. Resource preflight: the frozen task's declared requirements
    //    are compared against the local observations. An unmet or
    //    unobservable requirement rejects the whole request - the
    //    requested set never silently shrinks.
    let preflight = preflight_tasks(&task_package)?;

    // 7. Emit the local-origin native-material manifest.
    std::fs::create_dir_all(out)?;
    let material_path = out.join("material.json");
    let digest_of = |path: &Path| sha256_hex(&std::fs::read(path).unwrap_or_default());
    let agents: BTreeMap<String, serde_json::Value> = products
        .iter()
        .map(|product| {
            let executable = agent_executable_path(&context.cache, product);
            let class = match product.as_str() {
                "opi" => "Opi",
                "pi" => "Pi",
                other => other,
            };
            let integrations = ["harbor", "pier"]
                .into_iter()
                .map(|family| {
                    let module_dir = context.cache.join("pins/integrations").join(family);
                    (
                        family.to_owned(),
                        serde_json::json!({
                            "module_dir": module_dir,
                            "module_manifest_sha256": directory_manifest_digest(&module_dir)
                                .unwrap_or_default(),
                            "import_path": format!("opi_eval_{family}:{class}Agent"),
                            "agent_name": format!("opi-eval-{product}"),
                        }),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            (
                product.clone(),
                serde_json::json!({
                    "executable": {"path": executable, "sha256": digest_of(&executable)},
                    "integrations": integrations,
                }),
            )
        })
        .collect();
    let material = serde_json::json!({
        "schema": "opi-eval-native-material/3",
        "origin": "local",
        "static_lock": {"path": lock_path, "sha256": sha256_hex(&lock_bytes)},
        "provider": {
            "script": {"path": provider, "sha256": digest_of(&provider)},
            "endpoint": context.experiment.selected_connection().base_url,
            "request_log": out.join("requests.jsonl"),
        },
        "agents": agents,
        "benchmarks": {
            adapter.clone(): {
                "profile": profile,
                "task_package": task_package,
                "task_package_manifest_sha256": directory_manifest_digest(&task_package)
                    .unwrap_or_default(),
                "verifier_executable": {"path": verifier, "sha256": digest_of(&verifier)},
                "verifier_env": {},
                "oracle": {"path": oracle, "sha256": digest_of(&oracle)},
                "oracle_env": {},
            },
        },
    });
    std::fs::write(
        &material_path,
        serde_json::to_string_pretty(&material).expect("material serializes"),
    )?;

    Ok(serde_json::json!({
        "schema": PREPARE_SCHEMA,
        "experiment": context.experiment.experiment_id(),
        "origin": "local",
        "cache": context.cache,
        "material": material_path,
        "selection": {"benchmark": adapter, "products": products},
        "images": images,
        "verified": verified,
        "preflight": preflight,
        "acquired": acquired,
        "inventory": context.cache.join("inventory.json"),
    }))
}

/// The reusable verified-identity inventory inside the cache.
#[derive(Debug, Serialize, Deserialize)]
struct Inventory {
    schema: String,
    entries: BTreeMap<String, InventoryEntry>,
}

fn read_inventory(cache: &Path) -> Result<Inventory, PrepareError> {
    let path = cache.join("inventory.json");
    if !path.is_file() {
        return Ok(Inventory {
            schema: INVENTORY_SCHEMA.to_owned(),
            entries: BTreeMap::new(),
        });
    }
    let inventory: Inventory = serde_json::from_slice(&std::fs::read(&path)?)
        .map_err(|error| PrepareError::Rejected(format!("inventory parse failed: {error}")))?;
    if inventory.schema != INVENTORY_SCHEMA {
        return Err(PrepareError::Rejected(format!(
            "inventory schema {:?} is not {INVENTORY_SCHEMA:?}",
            inventory.schema
        )));
    }
    Ok(inventory)
}

fn write_inventory(cache: &Path, inventory: &Inventory) -> Result<(), PrepareError> {
    std::fs::create_dir_all(cache)?;
    std::fs::write(
        cache.join("inventory.json"),
        serde_json::to_string_pretty(inventory).expect("inventory serializes"),
    )?;
    Ok(())
}

/// The frozen task package's declared environment requirements.
#[derive(Debug, Clone)]
pub(crate) struct TaskRequirements {
    pub(crate) task_id: String,
    pub(crate) cpus: Option<i64>,
    pub(crate) memory_bytes: Option<u64>,
    pub(crate) gpus: Option<i64>,
}

pub(crate) fn parse_task_requirements(
    task_package: &Path,
) -> Result<TaskRequirements, PrepareError> {
    let text = std::fs::read_to_string(task_package.join("task.toml"))?;
    let document: toml::Value = text
        .parse()
        .map_err(|error| PrepareError::Rejected(format!("task.toml parse failed: {error}")))?;
    let task_id = document
        .get("task")
        .and_then(|task| {
            task.get("name")
                .or_else(|| task.get("id"))
                .and_then(|value| value.as_str())
        })
        .unwrap_or("unnamed-task")
        .to_owned();
    let environment = document.get("environment");
    let get = |key: &str| -> Option<i64> {
        environment
            .and_then(|table| table.get(key))
            .and_then(|value| value.as_integer())
    };
    let memory_bytes = get("memory_mb").map(|mb| mb as u64 * 1024 * 1024);
    Ok(TaskRequirements {
        task_id,
        cpus: get("cpus"),
        memory_bytes,
        gpus: get("gpus"),
    })
}

/// Compares every frozen task's declared requirements against the
/// local observations; each comparison is explicit and any unmet or
/// unobservable requirement rejects.
fn preflight_tasks(task_package: &Path) -> Result<serde_json::Value, PrepareError> {
    let requirements = parse_task_requirements(task_package)?;
    let gpu = gpu_runtime();
    let reject = |reason: String| -> PrepareError { PrepareError::Rejected(reason) };
    if let Some(required) = requirements.cpus {
        match logical_cpu_count() {
            Some(observed) if observed as i64 >= required => {}
            Some(observed) => {
                return Err(reject(format!(
                    "task {} requires cpus={} but the host observes {} logical CPUs",
                    requirements.task_id, required, observed
                )));
            }
            None => {
                return Err(reject(format!(
                    "task {} requires cpus={} but the CPU observation is unavailable",
                    requirements.task_id, required
                )));
            }
        }
    }
    if let Some(required) = requirements.memory_bytes {
        match total_memory_bytes() {
            Some(observed) if observed >= required => {}
            Some(observed) => {
                return Err(reject(format!(
                    "task {} requires {} memory bytes but the host observes {}",
                    requirements.task_id, required, observed
                )));
            }
            None => {
                return Err(reject(format!(
                    "task {} requires memory but the memory observation is unavailable",
                    requirements.task_id
                )));
            }
        }
    }
    if requirements.gpus.is_some_and(|required| required > 0) && gpu.is_none() {
        return Err(reject(format!(
            "task {} requires gpus={} but the GPU observation is unavailable \
             (engine unreachable or no GPU runtime registered)",
            requirements.task_id,
            requirements.gpus.unwrap_or_default()
        )));
    }
    Ok(serde_json::json!({
        "tasks": [{
            "id": requirements.task_id,
            "status": "ok",
            "requirements": {
                "cpus": requirements.cpus,
                "memory_bytes": requirements.memory_bytes,
                "gpus": requirements.gpus,
            },
            "gpu_observation": if gpu.is_some() { "available" } else { "unknown" },
        }],
    }))
}

/// Replaces two-byte CRLF line endings with LF, the convention the
/// static lock's repo-tracked text records hash over.
fn crlf_to_lf(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
            out.push(b'\n');
            index += 2;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    out
}
