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
use crate::process::{CleanupEvidence, ExitState, ProcessSupervisor, SpawnSpec};
use crate::runner::material::{
    directory_manifest_digest, runner_source_digest, runtime_package_manifest_digest, sha256_hex,
};

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

    // Device capacity requires an observation in the selected engine namespace.
    let resources = observe_resources(
        &context.cache,
        config_path.parent().unwrap_or(Path::new(".")),
        None,
    );
    observations.push(if !engine_reachable {
        unknown("gpu", "engine unreachable; GPU state cannot be observed")
    } else {
        match resources.engine_gpus.value {
            Some(count) => {
                let mut observation =
                    Observation::new("gpu", if count > 0 { "available" } else { "absent" });
                observation.count = Some(count);
                observation.detail = Some(resources.gpu_types.join(", "));
                observation
            }
            None => unknown(
                "gpu",
                "selected engine GPU devices are unobserved; runtime registration is not capacity",
            ),
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
        "resources": resources,
    }))
}

/// The pinned static lock location inside the cache.
pub(crate) fn static_lock_path(cache: &Path) -> PathBuf {
    cache.join("pins/static/linux-x86_64.json")
}

/// One selected product's built executable location inside the cache.
pub(crate) fn agent_executable_path(cache: &Path, product: &str) -> PathBuf {
    let path = cache.join("agents").join(product);
    if path.is_dir() {
        path.join(product)
    } else {
        path
    }
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

    #[cfg(unix)]
    #[tokio::test]
    async fn acquisition_settles_deadline_capture_and_failures() {
        for (case, script, successful) in [
            ("timeout", "echo $$ > child.pid; exec sleep 30", false),
            (
                "noisy",
                "touch \"$TMPDIR/owned\"; head -c 4096 /dev/zero; head -c 4096 /dev/zero >&2",
                true,
            ),
            ("nonzero", "echo acquisition-failed >&2; exit 7", false),
            ("spawn", "", false),
        ] {
            let room = tempfile::tempdir().unwrap();
            let cache = room.path().join("verified-cache-entry");
            std::fs::write(&cache, b"verified unchanged bytes").unwrap();
            let spec = SpawnSpec {
                argv: if case == "spawn" {
                    vec![room.path().join("absent").into_os_string()]
                } else {
                    vec!["/bin/sh".into(), "-c".into(), script.into()]
                },
                cwd: Some(room.path().to_owned()),
                env: BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]),
                stdout_cap: 128,
                stderr_cap: 128,
                timeout: std::time::Duration::from_millis(if case == "timeout" {
                    100
                } else {
                    5000
                }),
            };
            let result = settle_acquisition(
                &spec,
                room.path(),
                &tokio_util::sync::CancellationToken::new(),
            )
            .await;
            assert_eq!(result.is_ok(), successful, "{case}: {result:?}");
            let log = std::fs::read_to_string(room.path().join("acquisition.log")).unwrap();
            assert!(log.len() < 2048, "{case}: bounded diagnostic");
            match case {
                "timeout" => {
                    assert!(log.contains("TimedOut"), "{log}");
                    assert!(log.contains("verified: true"), "{log}");
                    let pid = std::fs::read_to_string(room.path().join("child.pid")).unwrap();
                    assert!(!Path::new(&format!("/proc/{}", pid.trim())).exists());
                }
                "noisy" => {
                    assert!(log.contains("stdout_truncated: true"), "{log}");
                    assert!(log.contains("stderr_truncated: true"), "{log}");
                }
                "spawn" => assert!(log.contains("FailedToSpawn"), "{log}"),
                _ => assert!(log.contains("code: 7"), "{log}"),
            }
            assert!(!room.path().join("material.json").exists());
            assert!(!room.path().join("acquisition-staging").exists());
            assert_eq!(std::fs::read(cache).unwrap(), b"verified unchanged bytes");
        }
    }

    fn execution() -> crate::experiment::ResolvedExecution {
        ResolvedExperiment::resolve(include_str!(
            "../../tests/fixtures/experiment/local-paired.toml"
        ))
        .unwrap()
        .execution()
        .clone()
    }

    fn requirements(
        text: &str,
        family: crate::benchmark::task_lifecycle::TaskFamily,
        execution: &crate::experiment::ResolvedExecution,
    ) -> Result<EffectiveTaskRequirements, PrepareError> {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("task.toml"), text).unwrap();
        effective_task_requirements(root.path(), family, execution)
    }

    fn resources() -> ResourceObservations {
        let observed = |value, source| ResourceObservation::observed(Some(value), source, "");
        ResourceObservations {
            engine_id: Some("selected-engine".to_owned()),
            engine_cpu: observed(4, "docker-info:NCPU"),
            engine_memory: observed(8 << 30, "docker-info:MemTotal"),
            engine_storage: observed(30 << 30, "engine-root"),
            cache_storage: observed(30 << 30, "cache"),
            output_storage: observed(30 << 30, "output"),
            engine_gpus: observed(1, "engine-nvidia-smi"),
            gpu_types: vec!["NVIDIA A100".to_owned()],
        }
    }

    #[cfg(unix)]
    #[test]
    fn observations_use_selected_engine_and_runtime_registration_stays_unknown() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let docker = root.path().join("docker");
        std::fs::write(&docker, "#!/bin/sh\n[ \"$DOCKER_HOST\" = tcp://selected.invalid:2375 ] || exit 9\ncase \"$1\" in\ninfo) printf '%s\\n' '{\"ID\":\"selected\",\"NCPU\":1,\"MemTotal\":4096,\"DockerRootDir\":\"/remote\",\"Runtimes\":{\"nvidia\":{}}}';;\ncontext) echo tcp://selected.invalid:2375;;\n*) exit 8;;\nesac\n").unwrap();
        std::fs::set_permissions(docker, std::fs::Permissions::from_mode(0o755)).unwrap();
        let env = BTreeMap::from([
            ("PATH".to_owned(), root.path().display().to_string()),
            (
                "DOCKER_HOST".to_owned(),
                "tcp://selected.invalid:2375".to_owned(),
            ),
        ]);
        let observed = observe_resources(root.path(), root.path(), Some(&env));
        assert_eq!(observed.engine_id.as_deref(), Some("selected"));
        assert_eq!(observed.engine_cpu.value, Some(1));
        assert_eq!(observed.engine_memory.value, Some(4096));
        assert_eq!(observed.engine_storage.status, "unknown");
        assert_eq!(observed.engine_gpus.status, "unknown");
        let req = requirements(
            "[environment]\ncpus=2",
            crate::benchmark::task_lifecycle::TaskFamily::Harbor,
            &execution(),
        )
        .unwrap();
        assert_eq!(check_requirements(&req, &observed)["status"], "unmet");
    }

    #[test]
    fn pinned_requirement_defaults_and_role_inheritance_are_explicit() {
        use crate::benchmark::task_lifecycle::TaskFamily::{Harbor, Pier};
        for family in [Harbor, Pier] {
            let defaults = requirements("", family, &execution()).unwrap();
            assert_eq!(defaults.agent.cpus, None);
            assert_eq!(defaults.agent.memory_bytes, None);
            assert_eq!(
                defaults.agent.storage_bytes,
                (family == Pier).then_some(10240 * 1024 * 1024)
            );
            assert_eq!(defaults.agent.gpus, (family == Pier).then_some(0));
            assert_eq!(defaults.agent.timeout_seconds, None);
            assert_eq!(defaults.verifier.timeout_seconds, Some(600.0));
            assert_eq!(defaults.agent.build_timeout_seconds, 600.0);
            assert!(
                defaults
                    .agent
                    .origins
                    .values()
                    .all(|origin| origin.contains("default"))
            );
            let separate = requirements("[environment]\ncpus=2\nmemory_mb=100\nstorage_mb=7\ngpus=0\n[verifier.environment]\ncpus=3\nmemory_mb=200\nstorage_mb=8\ngpus=1\ngpu_types=['A100']\n", family, &execution()).unwrap();
            assert_eq!(separate.agent.cpus, Some(2));
            assert_eq!(separate.verifier.cpus, Some(3));
            assert_eq!(separate.agent.memory_bytes, Some(100 * 1024 * 1024));
            assert_eq!(separate.verifier.storage_bytes, Some(8 * 1024 * 1024));
            assert_eq!(
                separate.verifier.origins["cpus"],
                "task.toml:verifier.environment.cpus"
            );
            assert_eq!(check_requirements(&separate, &resources())["status"], "ok");
            let shared = requirements("[environment]\ncpus=2\n[verifier]\nenvironment_mode='shared'\n[verifier.environment]\ncpus=9\n", family, &execution()).unwrap();
            assert_eq!(shared.verifier.cpus, Some(2));
            assert_eq!(
                shared.verifier.origins["cpus"],
                "task.toml:environment.cpus"
            );
        }
    }

    #[test]
    fn independent_resource_deficits_and_unknowns_reject_each_role() {
        use crate::benchmark::task_lifecycle::TaskFamily::Harbor;
        for role in ["environment", "verifier.environment"] {
            for (declaration, resource) in [
                ("cpus=5", "cpus"),
                ("memory_mb=9000", "memory_bytes"),
                ("storage_mb=40000", "engine_storage_bytes"),
                ("gpus=2", "gpus"),
                ("gpus=1\ngpu_types=['H100']", "gpu_types"),
            ] {
                let required =
                    requirements(&format!("[{role}]\n{declaration}"), Harbor, &execution())
                        .unwrap();
                let report = check_requirements(&required, &resources());
                assert_eq!(report["status"], "unmet", "{report}");
                assert!(
                    report["checks"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|check| check["resource"] == resource && check["status"] == "unmet")
                );
            }
        }
        let required = requirements(
            "[environment]\nstorage_mb=1\ngpus=1\ngpu_types=['A100']",
            Harbor,
            &execution(),
        )
        .unwrap();
        let mut observed = resources();
        observed.engine_storage =
            ResourceObservation::observed(None, "remote-engine", "engine namespace unavailable");
        observed.engine_gpus = ResourceObservation::observed(
            None,
            "docker-info:nvidia-runtime",
            "device capacity unobserved",
        );
        let report = check_requirements(&required, &observed);
        assert_eq!(report["status"], "unknown");
        assert!(
            report["reason"]
                .as_str()
                .unwrap()
                .contains("namespace unavailable")
        );
        for location in ["cache", "output"] {
            let mut observed = resources();
            let disk = if location == "cache" {
                &mut observed.cache_storage
            } else {
                &mut observed.output_storage
            };
            *disk = ResourceObservation::observed(Some(0), location, "");
            assert_eq!(check_requirements(&required, &observed)["status"], "unmet");
        }
    }

    #[test]
    fn invalid_units_and_unimplemented_requirements_never_disappear() {
        use crate::benchmark::task_lifecycle::TaskFamily::{Harbor, Pier};
        for family in [Harbor, Pier] {
            for field in [
                "cpus=-1",
                "cpus=0",
                "memory_mb=-1",
                "memory_mb=9223372036854775807",
                "memory_mb='one'",
                "storage_mb=-1",
                "storage_mb=9223372036854775807",
                "gpus=-1",
                "gpu_types=1",
                "gpu_vram_mb=1000",
                "tpu='v4'",
                "memory='1G'",
            ] {
                assert!(
                    requirements(&format!("[environment]\n{field}"), family, &execution()).is_err(),
                    "{field}"
                );
            }
            for phase in ["agent", "verifier"] {
                for value in ["-1", "0", "nan", "inf", "'1'"] {
                    assert!(
                        requirements(
                            &format!("[{phase}]\ntimeout_sec={value}"),
                            family,
                            &execution()
                        )
                        .is_err()
                    );
                }
            }
        }
    }

    #[test]
    fn effective_network_lifecycle_collect_and_deadlines_keep_origins() {
        use crate::benchmark::task_lifecycle::TaskFamily::{Harbor, Pier};
        let mut bounds = execution();
        bounds.agent_timeout_seconds = Some(9);
        bounds.time_limit_seconds = Some(7);
        bounds.verifier_timeout_seconds = Some(11);
        for family in [Harbor, Pier] {
            for (declared, wanted) in [("", 7.0), ("timeout_sec=3", 3.0), ("timeout_sec=20", 7.0)] {
                let req = requirements(
                    &format!("[agent]\n{declared}\n[verifier]\ntimeout_sec=5"),
                    family,
                    &bounds,
                )
                .unwrap();
                assert_eq!(req.agent.timeout_seconds, Some(wanted));
                assert_eq!(req.verifier.timeout_seconds, Some(5.0));
                assert!(req.agent.origins["timeout_seconds"].contains("execution cap"));
            }
        }
        let harbor = requirements("artifacts=[{source='/app/out',destination='out',service='main'}]\n[environment]\nnetwork_mode='no-network'\n[agent]\nnetwork_mode='public'\n[verifier.environment]\nnetwork_mode='allowlist'\nallowed_hosts=[' EXAMPLE.COM. ', '10.0.0.0/8', '*.example.org']", Harbor, &bounds).unwrap();
        assert_eq!(harbor.agent.network_mode, "public");
        assert_eq!(harbor.verifier.network_mode, "allowlist");
        assert_eq!(
            harbor.verifier.allowed_hosts,
            ["example.com", "10.0.0.0/8", "*.example.org"]
        );
        assert_eq!(
            harbor.lifecycle.declared_artifacts[0]
                .destination
                .as_deref(),
            Some("out")
        );
        assert_eq!(harbor.origins["artifacts"], "task.toml:artifacts");
        let pier = requirements("[agent]\nnetwork_mode='no-network'\n[verifier]\nenvironment_mode='separate'\n[[verifier.collect]]\ncommand='git diff'", Pier, &bounds).unwrap();
        assert_eq!(pier.agent.network_mode, "no-network");
        assert_eq!(pier.verifier.network_mode, "no-network");
        assert_eq!(pier.collect[0]["timeout_sec"], 60.0);
        assert_eq!(pier.collect[0]["service"], "main");
        assert!(
            pier.collect[0]["origins"]["service"]
                .as_str()
                .unwrap()
                .contains("default")
        );
        for (family, text) in [
            (
                Harbor,
                "[agent]\nnetwork_mode='allowlist'\nallowed_hosts=['https://example.com']",
            ),
            (
                Harbor,
                "[agent]\nnetwork_mode='allowlist'\nallowed_hosts=['10.1.0.0/8']",
            ),
            (Pier, "[agent]\nnetwork_mode='allowlist'"),
            (
                Pier,
                "[agent]\nnetwork_mode='public'\n[verifier]\nnetwork_mode='no-network'",
            ),
            (Pier, "[[verifier.collect]]\ncommand='x'\nservice='sidecar'"),
        ] {
            assert!(requirements(text, family, &bounds).is_err(), "{text}");
        }
    }

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
    #[serde(default)]
    tools: Vec<ToolProjection>,
}

#[derive(Debug, Deserialize)]
struct ToolProjection {
    id: String,
    version: Option<String>,
    commit: Option<String>,
    uv_lock: Option<RunnerLockProjection>,
}

#[derive(Debug, Deserialize)]
struct RunnerLockProjection {
    git_blob: String,
    sha256: String,
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
pub(crate) async fn prepare(
    config_path: &Path,
    out: &Path,
    opi_source: Option<&Path>,
    opi_commit: Option<&str>,
    cancel: &tokio_util::sync::CancellationToken,
) -> Result<serde_json::Value, PrepareError> {
    if opi_source.is_some() != opi_commit.is_some()
        || opi_commit.is_some_and(|commit| {
            commit.len() != 40
                || !commit
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        })
    {
        return Err(PrepareError::Rejected("--opi-source and --opi-commit require a paired local directory and full lowercase 40-hex commit".to_owned()));
    }
    let opi_source = opi_source.map(std::fs::canonicalize).transpose()?;
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
    let corpus_root = context.cache.join("corpora").join(&adapter).join("tasks");
    let verifier = context.cache.join("tools/uv");
    let runner = if adapter == "deepswe-v1.1" {
        "pier"
    } else {
        "harbor"
    };
    let runner_pin = lock
        .tools
        .iter()
        .find(|tool| tool.id == runner)
        .ok_or_else(|| PrepareError::Rejected(format!("static lock has no runner {runner} pin")))?;
    let uv_version = lock
        .tools
        .iter()
        .find(|tool| tool.id == "uv")
        .and_then(|tool| tool.version.as_deref())
        .filter(|version| *version == "0.9.7")
        .ok_or_else(|| {
            PrepareError::Rejected("static lock must pin supported uv 0.9.7".to_owned())
        })?;
    let runner_project = context.cache.join("runners").join(runner);
    let runner_commit = runner_pin.commit.as_deref().ok_or_else(|| {
        PrepareError::Rejected(format!("runner {runner}: static lock has no source commit"))
    })?;
    let runner_lock = runner_pin.uv_lock.as_ref().ok_or_else(|| {
        PrepareError::Rejected(format!(
            "runner {runner}: static lock has no uv.lock identity"
        ))
    })?;
    // Witness dispatch explicitly selects the upstream oracle on the same
    // uv entrypoint; no additional local launch wrapper is needed.
    let oracle = verifier.clone();
    let provider = context.cache.join("pins/scripts/scripted-provider.py");
    let mut required: Vec<(String, PathBuf)> = std::iter::once((
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
        (format!("profiles:{adapter}"), profile.clone()),
        ("tools:uv".to_owned(), verifier.clone()),
        (format!("runners:{runner}"), runner_project.clone()),
        (
            format!("runners:{runner}:project"),
            runner_project.join("pyproject.toml"),
        ),
        (
            format!("runners:{runner}:lock"),
            runner_project.join("uv.lock"),
        ),
        (
            format!("runners:{runner}:python"),
            runner_project.join(".venv/bin/python"),
        ),
        (
            format!("runners:{runner}:entrypoint"),
            runner_project.join(".venv/bin").join(runner),
        ),
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
    .chain([(format!("corpora:{adapter}"), corpus_root.clone())])
    .collect();
    for product in &products {
        let package = context.cache.join("agents").join(product);
        if package.is_dir() {
            required.push((format!("runtime:{product}"), package));
        }
    }
    let missing: Vec<(&String, &PathBuf)> = required
        .iter()
        .map(|(key, path)| (key, path))
        .filter(|(key, path)| {
            let present = if key.starts_with("integrations:")
                || key.starts_with("corpora:")
                || key.starts_with("runtime:")
                || key.as_str() == format!("runners:{runner}")
            {
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
        if missing.iter().any(|(key, _)| key.as_str() == "agents:opi") && opi_source.is_none() {
            return Err(PrepareError::Rejected(
                "a selected cold Opi build requires --opi-source and --opi-commit".to_owned(),
            ));
        }
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
        if !lock
            .authority
            .producers
            .iter()
            .any(|producer| Path::new(&producer.path).file_name() == materializer.file_name())
        {
            return Err(PrepareError::Rejected(
                "the acquisition materializer has no static producer pin".to_owned(),
            ));
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
                serde_json::json!({"id": relative, "key": key, "products": products,
                    "benchmark": adapter, "opi_source": opi_source, "opi_commit": opi_commit})
            })
            .collect();
        std::fs::write(
            &plan_path,
            serde_json::to_string_pretty(&plan).expect("acquisition plan serializes"),
        )?;
        let spec = SpawnSpec {
            argv: vec![
                materializer.into_os_string(),
                "prepare".into(),
                "--cache".into(),
                context.cache.as_os_str().to_owned(),
                "--plan".into(),
                plan_path.into_os_string(),
            ],
            cwd: None,
            env: acquisition_environment(),
            stdout_cap: 1_048_576,
            stderr_cap: 1_048_576,
            timeout: std::time::Duration::from_secs(3600),
        };
        settle_acquisition(&spec, out, cancel).await?;
        for (key, original_path) in &missing {
            let acquired_executable = key
                .strip_prefix("agents:")
                .map(|product| agent_executable_path(&context.cache, product));
            let path = acquired_executable.as_ref().unwrap_or(original_path);
            let present = if key.starts_with("integrations:")
                || key.starts_with("corpora:")
                || key.starts_with("runtime:")
                || key.as_str() == format!("runners:{runner}")
            {
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

    for (key, path) in &mut required {
        if let Some(product) = key.strip_prefix("agents:") {
            *path = agent_executable_path(&context.cache, product);
        }
    }
    for product in &products {
        let package = context.cache.join("agents").join(product);
        let key = format!("runtime:{product}");
        if package.is_dir() && !required.iter().any(|(present, _)| *present == key) {
            required.push((key, package));
        }
    }

    // 6. Verify every identity and freeze it into the reusable
    //    inventory; any drift against a prior freeze rejects.
    let version = Command::new(&verifier)
        .arg("--version")
        .env_clear()
        .output()?;
    if !version.status.success()
        || String::from_utf8_lossy(&version.stdout).trim() != format!("uv {uv_version}")
    {
        return Err(PrepareError::Rejected(
            "verifier executable is not the pinned uv 0.9.7".to_owned(),
        ));
    }
    let mut inventory = read_inventory(&context.cache)?;
    for product in &products {
        if inventory
            .entries
            .contains_key(&format!("runtime:{product}"))
            && !context.cache.join("agents").join(product).is_dir()
        {
            return Err(PrepareError::Rejected(format!(
                "runtime package removed for {product}"
            )));
        }
    }
    let mut verified = Vec::new();
    for (key, path) in &required {
        let observed = if key == &format!("runners:{runner}") {
            runner_source_digest(
                path,
                runner_commit,
                &runner_lock.git_blob,
                &runner_lock.sha256,
            )
            .map_err(|error| PrepareError::Rejected(error.to_string()))?
        } else if key.starts_with("runtime:") {
            runtime_package_manifest_digest(path)
                .map_err(|error| PrepareError::Rejected(error.to_string()))?
        } else if path.is_dir() {
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

    // 7. Emit the local-origin native-material manifest.
    std::fs::create_dir_all(out)?;
    let material_path = out.join("material.json");
    let runner_home = context.cache.join("runner-homes").join(runner);
    let uv_cache = context.cache.join("uv-cache");
    std::fs::create_dir_all(&runner_home)?;
    std::fs::create_dir_all(&uv_cache)?;
    let mut verifier_env: BTreeMap<String, String> = serde_json::from_value(serde_json::json!({
        "PATH": "/usr/local/bin:/usr/bin:/bin",
        "HOME": runner_home.canonicalize()?,
        "UV_PROJECT": runner_project.canonicalize()?,
        "UV_PROJECT_ENVIRONMENT": runner_project.canonicalize()?.join(".venv"),
        "UV_PYTHON": runner_project.canonicalize()?.join(".venv/bin/python"),
        "UV_CACHE_DIR": uv_cache.canonicalize()?,
        "UV_OFFLINE": "1",
        "UV_NO_DEV": "1",
        "UV_PYTHON_DOWNLOADS": "never",
        "UV_NO_ENV_FILE": "1",
        "PYTHON_DOTENV_DISABLED": "1",
        "PYTHONDONTWRITEBYTECODE": "1",
        "HARBOR_TELEMETRY": "0",
    }))
    .expect("runner environment strings");
    // Resolve the operator's selected endpoint before isolating HOME. Only
    // Docker routing is projected; provider credentials remain unobserved.
    for key in ["DOCKER_HOST", "DOCKER_TLS_VERIFY", "DOCKER_CERT_PATH"] {
        if let Ok(value) = std::env::var(key) {
            verifier_env.insert(key.to_owned(), value);
        }
    }
    if std::env::var_os("DOCKER_CONTEXT").is_some() || !verifier_env.contains_key("DOCKER_HOST") {
        let endpoint = Command::new("docker")
            .args([
                "context",
                "inspect",
                "--format",
                "{{(index .Endpoints \"docker\").Host}}",
            ])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned());
        if let Some(endpoint) = endpoint {
            verifier_env.insert("DOCKER_HOST".to_owned(), endpoint);
        }
    }
    let lock_value: serde_json::Value = serde_json::from_slice(&lock_bytes)
        .map_err(|error| PrepareError::Rejected(error.to_string()))?;
    let pin = lock_value["subjects"]
        .as_array()
        .and_then(|subjects| subjects.iter().find(|subject| subject["id"] == adapter))
        .ok_or_else(|| {
            PrepareError::Rejected(format!("static lock has no {adapter} corpus pin"))
        })?;
    let corpus = super::material::CorpusMaterial {
        root: corpus_root.canonicalize()?,
        source_commit: pin["commit"].as_str().unwrap_or_default().to_owned(),
        tasks_tree: pin["tasks_tree"].as_str().unwrap_or_default().to_owned(),
    };
    corpus
        .verify(&adapter, &lock_value)
        .map_err(|error| PrepareError::Rejected(error.to_string()))?;
    let inventory_tasks = super::plan::enumerate_corpus(&corpus.root)
        .map_err(|error| PrepareError::Rejected(error.to_string()))?;
    let requested = match context.experiment.selection() {
        Some(selection) if selection.scope == crate::experiment::SelectionScope::Full => None,
        Some(selection) => Some(selection.task_ids.clone()),
        None => Some(
            context
                .experiment
                .trials()
                .iter()
                .map(|trial| trial.task.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect(),
        ),
    };
    let mut selected = Vec::new();
    if let Some(requested) = requested {
        for id in requested {
            let task = inventory_tasks
                .iter()
                .find(|task| {
                    task.dir.file_name().and_then(|name| name.to_str()) == Some(&id)
                        || task.requirements.task_id == id
                })
                .ok_or_else(|| {
                    PrepareError::Rejected(format!(
                        "selected task {id:?} is outside the pinned corpus"
                    ))
                })?;
            let canonical = task
                .dir
                .file_name()
                .and_then(|name| name.to_str())
                .expect("corpus directory")
                .to_owned();
            if selected.contains(&canonical) {
                return Err(PrepareError::Rejected(
                    "selection repeats a canonical task".to_owned(),
                ));
            }
            selected.push(canonical);
        }
    } else {
        selected = inventory_tasks
            .iter()
            .map(|task| task.dir.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
    }
    let family = if runner == "pier" {
        crate::benchmark::task_lifecycle::TaskFamily::Pier
    } else {
        crate::benchmark::task_lifecycle::TaskFamily::Harbor
    };
    let observations = observe_resources(&context.cache, out, Some(&verifier_env));
    let mut tasks = BTreeMap::new();
    let mut entries = Vec::new();
    for task in &inventory_tasks {
        let id = task
            .dir
            .file_name()
            .and_then(|name| name.to_str())
            .expect("corpus directory");
        let chosen = selected.iter().any(|selected| selected == id);
        let mut entry =
            match effective_task_requirements(&task.dir, family, context.experiment.execution()) {
                Ok(requirements) => check_requirements(&requirements, &observations),
                Err(error) => serde_json::json!({"status": "unmet", "reason": error.to_string()}),
            };
        let images = super::material::image_preflight(&task.dir, &lock_value, &verifier_env);
        if entry["status"] == "ok" && images["status"] != "ok" {
            entry["status"] = images["status"].clone();
            entry["reason"] = images["reason"].clone();
        }
        entry["image_material"] = images;
        entry["id"] = serde_json::json!(id);
        entry["selected"] = serde_json::json!(chosen);
        if chosen {
            if entry["status"] != "ok" {
                return Err(PrepareError::Rejected(format!(
                    "task {id} resource preflight: {}",
                    entry["reason"]
                )));
            }
            let material = super::material::materialize_task(
                &corpus,
                id,
                &profile,
                &context.cache.join("packages").join(&adapter).join(id),
                &context
                    .cache
                    .join("profiles")
                    .join(&adapter)
                    .join(format!("{id}.toml")),
                &lock_value,
            )
            .map_err(|error| PrepareError::Rejected(error.to_string()))?;
            tasks.insert(id.to_owned(), material);
        }
        entries.push(entry);
    }
    let preflight = serde_json::json!({"tasks": entries, "observations": observations});
    let digest_of = |path: &Path| sha256_hex(&std::fs::read(path).unwrap_or_default());
    let agents: BTreeMap<String, serde_json::Value> = products
        .iter()
        .map(|product| {
            let executable = agent_executable_path(&context.cache, product);
            let package = context.cache.join("agents").join(product);
            let runtime_package = inventory.entries.get(&format!("runtime:{product}")).map(|entry| {
                serde_json::json!({"path": package, "manifest_sha256": entry.sha256, "entrypoint": product})
            });
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
                    "runtime_package": runtime_package,
                    "integrations": integrations,
                }),
            )
        })
        .collect();
    let material = serde_json::json!({
        "schema": super::material::MATERIAL_SCHEMA,
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
                "corpus": corpus,
                "tasks": tasks,
                "verifier_executable": {"path": verifier, "sha256": digest_of(&verifier)},
                "verifier_env": verifier_env,
                "runner_project": {
                    "path": runner_project.canonicalize()?,
                    "source_manifest_sha256": inventory.entries[&format!("runners:{runner}")].sha256,
                },
                "oracle": {"path": oracle, "sha256": digest_of(&oracle)},
                "oracle_env": verifier_env,
            },
        },
    });
    let frozen: super::material::NativeMaterial = serde_json::from_value(material.clone())
        .map_err(|error| PrepareError::Rejected(error.to_string()))?;
    frozen
        .benchmark(&adapter)
        .and_then(|entry| entry.verify_tasks(&adapter, &lock_path))
        .map_err(|error| PrepareError::Rejected(error.to_string()))?;
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

/// Build-tool inputs projected over the supervisor's empty environment.
fn acquisition_environment() -> BTreeMap<std::ffi::OsString, std::ffi::OsString> {
    [
        "PATH",
        "HOME",
        "CARGO_HOME",
        "RUSTUP_HOME",
        "RUSTUP_TOOLCHAIN",
        "CARGO_TARGET_DIR",
        "CARGO_INCREMENTAL",
        "CC",
        "CXX",
        "AR",
        "CFLAGS",
        "CXXFLAGS",
        "PKG_CONFIG_PATH",
        "TMPDIR",
        "TMP",
        "TEMP",
        "SYSTEMROOT",
        "PATHEXT",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
    ]
    .into_iter()
    .filter_map(|key| std::env::var_os(key).map(|value| (key.into(), value)))
    .collect()
}

/// Persist the bounded settled evidence before admitting any acquired material.
async fn settle_acquisition(
    spec: &SpawnSpec,
    out: &Path,
    cancel: &tokio_util::sync::CancellationToken,
) -> Result<(), PrepareError> {
    let staging = out.join("acquisition-staging");
    std::fs::create_dir(&staging)?;
    let mut spec = spec.clone();
    spec.env
        .insert("TMPDIR".into(), staging.canonicalize()?.into_os_string());
    let invocation = ProcessSupervisor::run(&spec, cancel).await;
    let verified_cleanup = matches!(
        invocation.cleanup,
        CleanupEvidence::NotRequired | CleanupEvidence::TreeTerminated { verified: true, .. }
    );
    let staging_removed = verified_cleanup && std::fs::remove_dir_all(&staging).is_ok();
    let log = format!(
        "argv: {:?}\nexit: {:?}\ncleanup: {:?}\nstaging_removed: {}\nstdout_truncated: {}\nstderr_truncated: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        spec.argv,
        invocation.exit,
        invocation.cleanup,
        staging_removed,
        invocation.stdout.truncated,
        invocation.stderr.truncated,
        String::from_utf8_lossy(&invocation.stdout.bytes),
        String::from_utf8_lossy(&invocation.stderr.bytes),
    );
    std::fs::write(out.join("acquisition.log"), log)?;
    if invocation.exit != (ExitState::Exited { code: 0 }) || !staging_removed {
        return Err(PrepareError::Rejected(format!(
            "supervised acquisition failed (see {})",
            out.join("acquisition.log").display(),
        )));
    }
    Ok(())
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

/// Effective values of one native execution role. Origins distinguish task
/// declarations, inherited environment settings, and pinned runner defaults.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct RoleRequirements {
    pub(crate) cpus: Option<u64>,
    pub(crate) memory_bytes: Option<u64>,
    pub(crate) storage_bytes: Option<u64>,
    pub(crate) gpus: Option<u64>,
    pub(crate) gpu_types: Option<Vec<String>>,
    pub(crate) network_mode: String,
    pub(crate) allowed_hosts: Vec<String>,
    pub(crate) timeout_seconds: Option<f64>,
    pub(crate) build_timeout_seconds: f64,
    pub(crate) origins: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct EffectiveTaskRequirements {
    pub(crate) agent: RoleRequirements,
    pub(crate) verifier: RoleRequirements,
    pub(crate) lifecycle: crate::benchmark::task_lifecycle::TaskLifecycle,
    pub(crate) collect: Vec<serde_json::Value>,
    pub(crate) origins: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ResourceObservation {
    status: &'static str,
    value: Option<u64>,
    source: String,
    reason: Option<String>,
}

impl ResourceObservation {
    fn observed(value: Option<u64>, source: impl Into<String>, reason: &str) -> Self {
        Self {
            status: if value.is_some() { "known" } else { "unknown" },
            value,
            source: source.into(),
            reason: value.is_none().then(|| reason.to_owned()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ResourceObservations {
    engine_id: Option<String>,
    engine_cpu: ResourceObservation,
    engine_memory: ResourceObservation,
    engine_storage: ResourceObservation,
    cache_storage: ResourceObservation,
    output_storage: ResourceObservation,
    engine_gpus: ResourceObservation,
    gpu_types: Vec<String>,
}

/// A local engine can expose its filesystem without starting an image. Bind
/// the selected Unix listener to dockerd's own open descriptor before reading
/// its root namespace; a Desktop proxy or remote endpoint remains unknown.
fn local_engine_pid(endpoint: &str) -> Option<u32> {
    let socket = Path::new(endpoint.strip_prefix("unix://")?);
    let socket = socket.canonicalize().ok()?;
    let sockets = std::fs::read_to_string("/proc/net/unix").ok()?;
    let inode = sockets.lines().find_map(|line| {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        let path = Path::new(fields.get(7)?);
        (path.canonicalize().ok()? == socket).then(|| fields[6].to_owned())
    })?;
    let target = format!("socket:[{inode}]");
    for entry in std::fs::read_dir("/proc").ok()?.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        if std::fs::read_to_string(entry.path().join("comm"))
            .ok()
            .as_deref()
            != Some("dockerd\n")
        {
            continue;
        }
        let Ok(descriptors) = std::fs::read_dir(entry.path().join("fd")) else {
            continue;
        };
        if descriptors.flatten().any(|entry| {
            std::fs::read_link(entry.path())
                .ok()
                .is_some_and(|path| path.as_os_str() == target.as_str())
        }) {
            return Some(pid);
        }
    }
    None
}

pub(crate) fn observe_resources(
    cache: &Path,
    output: &Path,
    environment: Option<&BTreeMap<String, String>>,
) -> ResourceObservations {
    let docker = |args: &[&str]| -> Option<String> {
        let mut command = Command::new("docker");
        command.args(args);
        if let Some(environment) = environment {
            command.env_clear().envs(environment);
        }
        let output = command.output().ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    };
    let info: serde_json::Value = docker(&["info", "--format", "{{json .}}"])
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    let variable = |key: &str| {
        environment
            .and_then(|env| env.get(key).cloned())
            .or_else(|| {
                environment
                    .is_none()
                    .then(|| std::env::var(key).ok())
                    .flatten()
            })
    };
    let endpoint = if variable("DOCKER_CONTEXT").is_some_and(|value| !value.is_empty()) {
        docker(&[
            "context",
            "inspect",
            "--format",
            "{{(index .Endpoints \"docker\").Host}}",
        ])
    } else {
        variable("DOCKER_HOST").or_else(|| {
            docker(&[
                "context",
                "inspect",
                "--format",
                "{{(index .Endpoints \"docker\").Host}}",
            ])
        })
    };
    let pid = endpoint.as_deref().and_then(local_engine_pid);
    let engine_root = pid.zip(info["DockerRootDir"].as_str()).map(|(pid, root)| {
        PathBuf::from(format!("/proc/{pid}/root")).join(root.trim_start_matches('/'))
    });
    let engine_storage = ResourceObservation::observed(
        engine_root.as_deref().and_then(available_disk_bytes),
        engine_root.as_ref().map_or_else(
            || "selected-docker-engine".to_owned(),
            |path| path.display().to_string(),
        ),
        "selected engine storage namespace is not locally observable",
    );
    let disk = |path: &Path| {
        let root = nearest_existing_ancestor(path);
        ResourceObservation::observed(
            available_disk_bytes(&root),
            format!("df:{}", root.display()),
            "storage observation unavailable",
        )
    };
    let same_mount = pid.is_some_and(|pid| {
        let current = std::fs::read_link("/proc/self/ns/mnt").ok();
        current.is_some() && current == std::fs::read_link(format!("/proc/{pid}/ns/mnt")).ok()
    });
    let gpu_types = if same_mount && info["Runtimes"].get("nvidia").is_some() {
        Command::new("nvidia-smi")
            .args(["--query-gpu=name", "--format=csv,noheader"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
    } else {
        None
    };
    ResourceObservations {
        engine_id: info["ID"].as_str().map(str::to_owned),
        engine_cpu: ResourceObservation::observed(
            info["NCPU"].as_u64().filter(|count| *count > 0),
            "docker-info:NCPU",
            "selected engine CPU count unavailable",
        ),
        engine_memory: ResourceObservation::observed(
            info["MemTotal"].as_u64().filter(|bytes| *bytes > 0),
            "docker-info:MemTotal",
            "selected engine memory unavailable",
        ),
        engine_storage,
        cache_storage: disk(cache),
        output_storage: disk(output),
        engine_gpus: ResourceObservation::observed(
            gpu_types.as_ref().map(|types| types.len() as u64),
            "selected-engine-namespace:nvidia-smi",
            "GPU devices/types unobserved; runtime registration is not capacity",
        ),
        gpu_types: gpu_types.unwrap_or_default(),
    }
}

/// Every required comparison is retained. One missing required observation
/// rejects the task rather than converting unknown capacity into zero or pass.
pub(crate) fn check_requirements(
    requirements: &EffectiveTaskRequirements,
    observations: &ResourceObservations,
) -> serde_json::Value {
    let mut checks = Vec::new();
    for (role, required) in [
        ("agent", &requirements.agent),
        ("verifier", &requirements.verifier),
    ] {
        for (name, value, observation) in [
            ("cpus", required.cpus, &observations.engine_cpu),
            (
                "memory_bytes",
                required.memory_bytes,
                &observations.engine_memory,
            ),
            (
                "engine_storage_bytes",
                required.storage_bytes,
                &observations.engine_storage,
            ),
            (
                "cache_storage_bytes",
                required.storage_bytes,
                &observations.cache_storage,
            ),
            (
                "output_storage_bytes",
                required.storage_bytes,
                &observations.output_storage,
            ),
            ("gpus", required.gpus, &observations.engine_gpus),
        ] {
            let status = match (value, observation.value) {
                (None | Some(0), _) => "not-required",
                (Some(required), Some(observed)) if observed >= required => "ok",
                (Some(_), Some(_)) => "unmet",
                (Some(_), None) => "unknown",
            };
            checks.push(
                serde_json::json!({"role": role, "resource": name, "required": value,
                "observation": observation, "status": status}),
            );
        }
        if let Some(types) = &required.gpu_types {
            let matching = observations
                .gpu_types
                .iter()
                .filter(|observed| {
                    types.iter().any(|allowed| {
                        observed.as_str() == allowed
                            || observed.strip_prefix("NVIDIA ") == Some(allowed.as_str())
                            || observed.strip_prefix("Tesla ") == Some(allowed.as_str())
                    })
                })
                .count() as u64;
            let status = if observations.engine_gpus.value.is_none() {
                "unknown"
            } else if matching < required.gpus.unwrap_or(1).max(1) {
                "unmet"
            } else {
                "ok"
            };
            checks.push(serde_json::json!({"role": role, "resource": "gpu_types", "required": types,
                "observation": {"value": observations.gpu_types, "source": observations.engine_gpus.source,
                    "reason": observations.engine_gpus.reason}, "status": status}));
        }
    }
    let status = if checks.iter().any(|check| check["status"] == "unmet") {
        "unmet"
    } else if checks.iter().any(|check| check["status"] == "unknown") {
        "unknown"
    } else {
        "ok"
    };
    let reasons = checks
        .iter()
        .filter(|check| matches!(check["status"].as_str(), Some("unmet" | "unknown")))
        .map(|check| {
            format!(
                "{} {} {} ({}: {})",
                check["role"].as_str().unwrap_or_default(),
                check["resource"].as_str().unwrap_or_default(),
                check["status"].as_str().unwrap_or_default(),
                check["observation"]["source"].as_str().unwrap_or_default(),
                check["observation"]["reason"]
                    .as_str()
                    .unwrap_or("observed capacity below requirement")
            )
        })
        .collect::<Vec<_>>();
    serde_json::json!({"status": status, "reason": reasons.join("; "), "checks": checks,
        "requirements": requirements, "observations": observations})
}

fn allowed_host(value: &str) -> Result<String, PrepareError> {
    let host = value
        .trim()
        .to_ascii_lowercase()
        .trim_end_matches('.')
        .to_owned();
    let reject = || {
        PrepareError::Rejected(
            "invalid allowed_hosts entry (hostname, IP or strict CIDR required)".to_owned(),
        )
    };
    if let Some((address, prefix)) = host.split_once('/') {
        let address = address.parse::<std::net::IpAddr>().map_err(|_| reject())?;
        let prefix = prefix.parse::<u32>().map_err(|_| reject())?;
        let (value, bits) = match address {
            std::net::IpAddr::V4(address) => (u32::from(address) as u128, 32),
            std::net::IpAddr::V6(address) => (u128::from(address), 128),
        };
        if prefix > bits || (prefix < bits && value & (u128::MAX >> (128 - (bits - prefix))) != 0) {
            return Err(reject());
        }
        return Ok(format!("{address}/{prefix}"));
    }
    if host.contains(':') {
        return host
            .parse::<std::net::Ipv6Addr>()
            .map(|address| address.to_string())
            .map_err(|_| reject());
    }
    let name = host.strip_prefix("*.").unwrap_or(&host);
    if name != host && name.parse::<std::net::IpAddr>().is_ok() {
        return Err(reject());
    }
    if !name.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label.as_bytes()[0].is_ascii_alphanumeric()
            && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
            && label
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-')
    }) {
        return Err(reject());
    }
    Ok(host)
}

/// Resolves the single-step task under the exact Harbor/Pier defaults. This
/// value is shared by preparation, selection and pre-dispatch admission.
pub(crate) fn effective_task_requirements(
    task_package: &Path,
    family: crate::benchmark::task_lifecycle::TaskFamily,
    execution: &crate::experiment::ResolvedExecution,
) -> Result<EffectiveTaskRequirements, PrepareError> {
    use crate::benchmark::task_lifecycle::{TaskFamily, VerifierEnvironmentMode};
    let bytes = std::fs::read(task_package.join("task.toml"))?;
    let lifecycle = crate::benchmark::task_lifecycle::parse_task_lifecycle_bytes(&bytes, family)
        .map_err(|error| PrepareError::Rejected(error.to_string()))?;
    let document: toml::Value = std::str::from_utf8(&bytes)
        .map_err(|_| PrepareError::Rejected("task.toml is not UTF-8".to_owned()))?
        .parse()
        .map_err(|error| PrepareError::Rejected(format!("task.toml parse failed: {error}")))?;
    let empty = toml::Value::Table(Default::default());
    let environment = document.get("environment").unwrap_or(&empty);
    let verifier = document.get("verifier").unwrap_or(&empty);
    let verifier_environment = if lifecycle.environment_mode == VerifierEnvironmentMode::Separate {
        verifier.get("environment").unwrap_or(environment)
    } else {
        environment
    };
    let default_origin = match family {
        TaskFamily::Harbor => "harbor-v0.22.0-default",
        TaskFamily::Pier => "pier-v0.3.1-default",
    };
    let reject = |field: &str| PrepareError::Rejected(format!("invalid task requirement: {field}"));
    let seconds = |value: Option<&toml::Value>, fallback: Option<f64>, field: &str| {
        let parsed = match value {
            Some(toml::Value::Integer(number)) => Some(*number as f64),
            Some(toml::Value::Float(number)) => Some(*number),
            Some(_) => return Err(reject(field)),
            None => fallback,
        };
        if parsed.is_some_and(|number| !number.is_finite() || number <= 0.0) {
            return Err(reject(field));
        }
        Ok(parsed)
    };
    let mut roles: Vec<RoleRequirements> = Vec::new();
    for (role, env, path, cap) in [
        (
            "agent",
            environment,
            "environment",
            super::plan::narrowed_agent_timeout(execution),
        ),
        (
            "verifier",
            verifier_environment,
            if lifecycle.environment_mode == VerifierEnvironmentMode::Separate
                && verifier.get("environment").is_some()
            {
                "verifier.environment"
            } else {
                "environment"
            },
            execution.verifier_timeout_seconds,
        ),
    ] {
        let phase = document.get(role).unwrap_or(&empty);
        let mut origins = BTreeMap::new();
        let mut number =
            |key: &str, default: Option<u64>, scale: u64| -> Result<Option<u64>, PrepareError> {
                let value = match env.get(key) {
                    Some(value) => Some(
                        value
                            .as_integer()
                            .and_then(|n| u64::try_from(n).ok())
                            .ok_or_else(|| reject(&format!("{path}.{key}")))?,
                    ),
                    None => default,
                };
                origins.insert(
                    key.to_owned(),
                    if env.get(key).is_some() {
                        format!("task.toml:{path}.{key}")
                    } else {
                        default_origin.to_owned()
                    },
                );
                value
                    .map(|value| {
                        value
                            .checked_mul(scale)
                            .ok_or_else(|| reject(&format!("{path}.{key} overflows bytes")))
                    })
                    .transpose()
            };
        let cpus = number("cpus", None, 1)?;
        let memory_bytes = number("memory_mb", None, 1024 * 1024)?;
        let storage_bytes = number(
            "storage_mb",
            (family == TaskFamily::Pier).then_some(10240),
            1024 * 1024,
        )?;
        let gpus = number("gpus", (family == TaskFamily::Pier).then_some(0), 1)?;
        if cpus == Some(0) || memory_bytes == Some(0) {
            return Err(reject(&format!("{path} CPU and memory must be positive")));
        }
        // The deprecated string sizes and accelerator shapes need exact
        // interpretation; this route does not silently erase them.
        for key in ["memory", "storage", "tpu", "gpu_vram_mb"] {
            if env.get(key).is_some() {
                return Err(PrepareError::Rejected(format!(
                    "unsupported task requirement: {path}.{key}"
                )));
            }
        }
        let gpu_types = env
            .get("gpu_types")
            .map(|value| {
                value
                    .as_array()
                    .ok_or_else(|| reject("gpu_types"))?
                    .iter()
                    .map(|value| {
                        value
                            .as_str()
                            .filter(|value| !value.trim().is_empty())
                            .map(str::to_owned)
                            .ok_or_else(|| reject("gpu_types"))
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;
        origins.insert(
            "gpu_types".to_owned(),
            if gpu_types.is_some() {
                format!("task.toml:{path}.gpu_types")
            } else {
                default_origin.to_owned()
            },
        );
        let network_source = if phase.get("network_mode").is_some() {
            (phase, role)
        } else if family == TaskFamily::Pier && environment.get("network_mode").is_some() {
            (environment, "environment")
        } else {
            (env, path)
        };
        let (mut network_mode, mut network_origin) =
            if let Some(mode) = network_source.0.get("network_mode") {
                (
                    mode.as_str()
                        .ok_or_else(|| reject("network_mode"))?
                        .to_owned(),
                    format!("task.toml:{}.network_mode", network_source.1),
                )
            } else if let Some(allow) = env.get("allow_internet") {
                (
                    if allow.as_bool().ok_or_else(|| reject("allow_internet"))? {
                        "public"
                    } else {
                        "no-network"
                    }
                    .to_owned(),
                    format!("task.toml:{path}.allow_internet"),
                )
            } else {
                ("public".to_owned(), default_origin.to_owned())
            };
        if family == TaskFamily::Pier
            && role == "verifier"
            && phase.get("network_mode").is_none()
            && environment.get("network_mode").is_none()
            && (lifecycle.environment_mode == VerifierEnvironmentMode::Shared
                || verifier.get("environment").is_none())
        {
            network_mode.clone_from(&roles[0].network_mode);
            network_origin = format!("inherited-agent: {}", roles[0].origins["network_mode"]);
        }
        if !matches!(network_mode.as_str(), "public" | "no-network" | "allowlist")
            || (network_mode == "allowlist" && family == TaskFamily::Pier)
        {
            return Err(reject("unsupported network_mode"));
        }
        let allowed_hosts = match network_source.0.get("allowed_hosts") {
            Some(hosts) => {
                if network_mode != "allowlist" {
                    return Err(reject("allowed_hosts requires allowlist"));
                }
                hosts
                    .as_array()
                    .ok_or_else(|| reject("allowed_hosts"))?
                    .iter()
                    .map(|host| allowed_host(host.as_str().ok_or_else(|| reject("allowed_hosts"))?))
                    .collect::<Result<Vec<_>, _>>()?
            }
            None => vec![],
        };
        origins.insert("network_mode".to_owned(), network_origin);
        origins.insert(
            "allowed_hosts".to_owned(),
            if network_source.0.get("allowed_hosts").is_some() {
                format!("task.toml:{}.allowed_hosts", network_source.1)
            } else {
                default_origin.to_owned()
            },
        );
        let timeout = seconds(
            phase.get("timeout_sec"),
            (role == "verifier").then_some(600.0),
            "timeout_sec",
        )?;
        let timeout_seconds = match (timeout, cap) {
            (Some(task), Some(cap)) => Some(task.min(cap as f64)),
            (task, cap) => task.or(cap.map(|cap| cap as f64)),
        };
        origins.insert(
            "timeout_seconds".to_owned(),
            format!(
                "{}; execution cap={cap:?}",
                if phase.get("timeout_sec").is_some() {
                    format!("task.toml:{role}.timeout_sec")
                } else {
                    default_origin.to_owned()
                }
            ),
        );
        let build_timeout_seconds = seconds(
            env.get("build_timeout_sec"),
            Some(600.0),
            "build_timeout_sec",
        )?
        .expect("default");
        origins.insert(
            "build_timeout_seconds".to_owned(),
            if env.get("build_timeout_sec").is_some() {
                format!("task.toml:{path}.build_timeout_sec")
            } else {
                default_origin.to_owned()
            },
        );
        roles.push(RoleRequirements {
            cpus,
            memory_bytes,
            storage_bytes,
            gpus,
            gpu_types,
            network_mode,
            allowed_hosts,
            timeout_seconds,
            build_timeout_seconds,
            origins,
        });
    }
    let verifier = roles.pop().expect("two roles");
    let agent = roles.pop().expect("two roles");
    if family == TaskFamily::Pier
        && lifecycle.environment_mode == VerifierEnvironmentMode::Shared
        && agent.network_mode != verifier.network_mode
    {
        return Err(reject("shared Pier verifier cannot change network policy"));
    }
    let collect = document
        .get("verifier")
        .and_then(|verifier| verifier.get("collect"))
        .and_then(toml::Value::as_array)
        .map(|hooks| {
            hooks
                .iter()
                .enumerate()
                .map(|(index, hook)| {
                    let declared = hook;
                    let timeout =
                        seconds(hook.get("timeout_sec"), Some(60.0), "collect.timeout_sec")?;
                    let mut hook = serde_json::to_value(hook)
                        .map_err(|error| PrepareError::Rejected(error.to_string()))?;
                    hook["timeout_sec"] = serde_json::json!(timeout);
                    if hook.get("service").is_none() {
                        hook["service"] = serde_json::json!("main");
                    }
                    if hook["service"] != "main" {
                        return Err(reject(
                            "unsupported collect service; Pier skips non-main services",
                        ));
                    }
                    if hook["command"]
                        .as_str()
                        .is_none_or(|command| command.is_empty())
                    {
                        return Err(reject("collect.command"));
                    }
                    let origins = ["command", "service", "timeout_sec", "user"]
                        .into_iter()
                        .map(|key| {
                            (
                                key,
                                if declared.get(key).is_some() {
                                    format!("task.toml:verifier.collect[{index}].{key}")
                                } else {
                                    default_origin.to_owned()
                                },
                            )
                        })
                        .collect::<BTreeMap<_, _>>();
                    if hook.get("user").is_none() {
                        hook["user"] = serde_json::Value::Null;
                    }
                    hook["origins"] = serde_json::json!(origins);
                    Ok::<_, PrepareError>(hook)
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    Ok(EffectiveTaskRequirements {
        agent,
        verifier,
        lifecycle,
        collect,
        origins: [
            ("schema_version", document.get("schema_version").is_some()),
            ("artifacts", document.get("artifacts").is_some()),
            (
                "verifier.environment_mode",
                document
                    .get("verifier")
                    .and_then(|value| value.get("environment_mode"))
                    .is_some(),
            ),
            (
                "verifier.collect",
                document
                    .get("verifier")
                    .and_then(|value| value.get("collect"))
                    .is_some(),
            ),
        ]
        .into_iter()
        .map(|(field, declared)| {
            (
                field.to_owned(),
                if declared {
                    format!("task.toml:{field}")
                } else {
                    default_origin.to_owned()
                },
            )
        })
        .collect(),
    })
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
        .unwrap_or_else(|| {
            task_package
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unnamed-task")
        })
        .to_owned();
    let environment = document.get("environment");
    let get = |key: &str| -> Option<i64> {
        environment
            .and_then(|table| table.get(key))
            .and_then(|value| value.as_integer())
    };
    let memory_bytes = get("memory_mb")
        .and_then(|mb| u64::try_from(mb).ok())
        .and_then(|mb| mb.checked_mul(1024 * 1024));
    Ok(TaskRequirements {
        task_id,
        cpus: get("cpus"),
        memory_bytes,
        gpus: get("gpus"),
    })
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
