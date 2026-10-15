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
        let config_path = std::path::absolute(config_path)?;
        let source = std::fs::read_to_string(&config_path)?;
        let experiment = ResolvedExperiment::resolve(&source)?;
        let cache = resolve_cache(&config_path, &experiment);
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
/// and mutates nothing. Every observer shares one finite operation
/// deadline (P19-AUD-003): a stalled or overflowing observer becomes a
/// truthful bounded report instead of an unbounded wait.
pub(crate) fn doctor(config_path: &Path) -> Result<serde_json::Value, PrepareError> {
    let context = PreparedContext::load(config_path)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(12);
    let mut observations = Vec::new();

    // 1. Linux engine reachability. The Docker client honors
    //    DOCKER_HOST; a missing client, a failed probe, or an unusable
    //    endpoint is `unreachable` with the reason, never a guess.
    let engine = probe_engine(deadline);
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
        deadline,
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
    observations.push(match available_disk_bytes(&probe_root, deadline) {
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

    // 8. Every selected preparation input (P19-AUD-010): the same
    // selected benchmark/runner requirements `prepare` verifies —
    // profile and corpus, uv tool pin, runner project/lock/interpreter/
    // entrypoint, applicable integrations, the scripted provider and
    // the capture helper — with expected locations and present/missing/
    // unknown reasons. Only selected products are required, and no
    // download, model, credential or Agent effect happens here.
    observations.extend(selected_material_observations(
        &context.cache,
        &context.experiment,
    ));

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

/// Observations for every selected preparation input `prepare` verifies
/// (P19-AUD-010). Each row names the expected cache location and its
/// present/missing/unknown state without any acquisition, model,
/// credential or Agent effect; unselected products are not required.
fn selected_material_observations(
    cache: &Path,
    experiment: &ResolvedExperiment,
) -> Vec<Observation> {
    let mut observations = Vec::new();
    let mut observe = |key: &str, path: &Path, reason: Option<String>| {
        let mut observation = match &reason {
            Some(reason) => {
                let mut observation = unknown(&format!("material:{key}"), reason);
                observation.detail = Some(path.to_string_lossy().into_owned());
                observation
            }
            None => Observation::new(
                &format!("material:{key}"),
                if path.is_file() || path.is_dir() {
                    "present"
                } else {
                    "missing"
                },
            ),
        };
        if reason.is_none() {
            observation.detail = Some(path.to_string_lossy().into_owned());
        }
        observations.push(observation);
    };
    let adapter = match crate::runner::experiment::adapter_key_for(experiment) {
        Ok(adapter) => adapter,
        Err(error) => {
            let mut observation = unknown(
                "material:selected-inputs",
                &format!("selected benchmark inputs cannot be derived: {error}"),
            );
            observation.detail = Some(cache.to_string_lossy().into_owned());
            observations.push(observation);
            return observations;
        }
    };
    let runner = if adapter == "deepswe-v1.1" {
        "pier"
    } else {
        "harbor"
    };
    let lock: Option<StaticLockProjection> = std::fs::read(static_lock_path(cache))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let lock_available = lock.is_some();
    observe(
        &format!("profiles:{adapter}"),
        &cache
            .join("pins/profiles/benchmarks")
            .join(format!("{adapter}.toml")),
        None,
    );
    observe(
        &format!("corpora:{adapter}"),
        &cache.join("corpora").join(&adapter).join("tasks"),
        None,
    );
    let uv_pinned = lock.as_ref().is_some_and(|lock| {
        lock.tools
            .iter()
            .any(|tool| tool.id == "uv" && tool.version.as_deref() == Some("0.9.7"))
    });
    observe(
        "tools:uv",
        &cache.join("tools/uv"),
        if lock_available && !uv_pinned {
            Some("static lock does not pin the supported uv 0.9.7".to_owned())
        } else if !lock_available {
            Some("static lock unavailable; the uv tool pin cannot be verified".to_owned())
        } else {
            None
        },
    );
    let runner_project = cache.join("runners").join(runner);
    let runner_pinned = lock.as_ref().is_some_and(|lock| {
        lock.tools
            .iter()
            .any(|tool| tool.id == runner && tool.commit.is_some() && tool.uv_lock.is_some())
    });
    let runner_reason = if !lock_available {
        Some("static lock unavailable; runner pins cannot be verified".to_owned())
    } else if !runner_pinned {
        Some(format!("static lock has no {runner} source/lock pin"))
    } else {
        None
    };
    for (key, path) in [
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
    ] {
        observe(&key, &path, runner_reason.clone());
    }
    observe(
        "pins:scripts:scripted-provider.py",
        &cache.join("pins/scripts/scripted-provider.py"),
        None,
    );
    observe(
        "integrations:harbor",
        &cache.join("pins/integrations/harbor"),
        None,
    );
    observe(
        "integrations:pier",
        &cache.join("pins/integrations/pier"),
        None,
    );
    observe(
        "tools:native-capture-helper",
        &cache.join("tools/native-capture-helper"),
        None,
    );
    observations
}

fn unknown(id: &str, reason: &str) -> Observation {
    let mut observation = Observation::new(id, "unknown");
    observation.reason = Some(reason.to_owned());
    observation
}

/// The ambient observation environment: only routing, identity, and
/// filesystem keys observers need, never provider credentials.
fn observation_environment(source: Option<&BTreeMap<String, String>>) -> BTreeMap<String, String> {
    [
        "HOME",
        "PATH",
        "DOCKER_HOST",
        "DOCKER_CONTEXT",
        "DOCKER_CONFIG",
        "DOCKER_TLS_VERIFY",
        "DOCKER_CERT_PATH",
    ]
    .into_iter()
    .filter_map(|key| {
        let value = match source {
            Some(env) => env.get(key).cloned(),
            None => std::env::var(key).ok(),
        };
        value.map(|value| (key.to_owned(), value))
    })
    .collect()
}

/// Probes the local Linux engine; returns its `os/arch` identity. The
/// probe is supervised with a finite bound, so a stalled or overflowing
/// engine client becomes a truthful bounded refusal reason (P19-AUD-003).
fn probe_engine(deadline: std::time::Instant) -> Result<String, String> {
    let bound = observer_bound(deadline);
    if bound.is_zero() {
        return Err("engine probe: observation budget elapsed".to_owned());
    }
    let argv = vec![
        "docker".into(),
        "version".into(),
        "--format".into(),
        "{{.Server.Os}}/{{.Server.Arch}}".into(),
    ];
    let Some(outcome) = supervised_observation(argv, &observation_environment(None), bound) else {
        return Err("engine probe could not be supervised".to_owned());
    };
    if observation_clean(&outcome) {
        return Ok(String::from_utf8_lossy(&outcome.stdout.bytes)
            .trim()
            .to_owned());
    }
    Err(observation_refusal("engine probe", &outcome))
}

/// The engine's GPU runtime name, when one is registered.
pub(crate) fn gpu_runtime(deadline: std::time::Instant) -> Option<String> {
    let bound = observer_bound(deadline);
    if bound.is_zero() {
        return None;
    }
    let argv = vec![
        "docker".into(),
        "info".into(),
        "--format".into(),
        "{{range $name, $_ := .Runtimes}}{{$name}} {{end}}".into(),
    ];
    let bytes = observation_command(argv, &observation_environment(None), bound)?;
    let runtimes = String::from_utf8_lossy(&bytes);
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

fn available_disk_bytes(path: &Path, deadline: std::time::Instant) -> Option<u64> {
    let bound = observer_bound(deadline);
    if bound.is_zero() {
        return None;
    }
    let bytes = observation_command(
        vec![
            "df".into(),
            "-B1".into(),
            "-P".into(),
            path.to_string_lossy().as_ref().into(),
        ],
        &observation_environment(None),
        bound,
    )?;
    let text = String::from_utf8_lossy(&bytes);
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
    #[cfg(target_os = "linux")]
    use std::process::Command;

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
        let observed = observe_resources(
            root.path(),
            root.path(),
            Some(&env),
            std::time::Instant::now() + std::time::Duration::from_secs(12),
        );
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
    fn desktop_sample_rejects_unbound_or_malformed_capacity() {
        let good = "engine\t42\t123\tmnt:[9]\t10240\t/var/lib/docker\n";
        assert_eq!(
            desktop_storage_sample(good.as_bytes(), "engine", "/var/lib/docker"),
            Some(10485760)
        );
        for bad in [
            good.replace("engine", "other"),
            good.replace("/var/lib/docker", "/other"),
            good.replace("42", "0"),
            good.replace("123", "-1"),
            good.replace("mnt:[9]", "mnt:[]"),
            good.replace("10240", "-1"),
            good.replace("10240", "+1"),
            good.replace("10240", "NaN"),
            good.replace("10240", "18446744073709551615"),
            good.trim_end().to_owned(),
            format!("{good}{good}"),
            good.replace('\t', " "),
        ] {
            assert_eq!(
                desktop_storage_sample(bad.as_bytes(), "engine", "/var/lib/docker"),
                None,
                "{bad:?}"
            );
        }
        for root in [
            "relative",
            "/var/../lib",
            "/var/./lib",
            "/var//lib",
            "/var/lib/",
            "/var\n/lib",
        ] {
            assert!(!normalized_absolute(root), "{root:?}");
        }
        assert!(normalized_absolute("/var/lib/docker"));
        assert!(desktop_is_running(b"Ubuntu\ndocker-desktop\n"));
        let utf16: Vec<_> = "\u{feff}Ubuntu\r\ndocker-desktop\r\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert!(desktop_is_running(&utf16));
        for names in [b"Ubuntu\n".as_slice(), b"docker-desktop-stopped\n", b"d\0x"] {
            assert!(!desktop_is_running(names));
        }
    }

    #[cfg(target_os = "linux")]
    fn desktop_fixture() -> (
        tempfile::TempDir,
        BTreeMap<String, String>,
        serde_json::Value,
    ) {
        use std::os::unix::fs::PermissionsExt;
        let room = tempfile::tempdir().unwrap();
        let info = serde_json::json!({"ID":"engine", "DockerRootDir":"/var/lib/docker", "OperatingSystem":"Docker Desktop", "NCPU":2, "MemTotal":4096});
        std::fs::write(room.path().join("info.json"), info.to_string()).unwrap();
        std::fs::write(
            room.path().join("endpoint"),
            "unix:///nonexistent/desktop.sock\n",
        )
        .unwrap();
        for (name, script) in [
            (
                "docker",
                "#!/bin/sh\ncd -- \"${0%/*}\"\ncase $1 in info) /bin/cat info.json;; context) /bin/cat endpoint;; *) exit 9;; esac\n",
            ),
            (
                "wsl.exe",
                "#!/bin/sh\ncd -- \"${0%/*}\"\n[ -z \"$SECRET_CANARY\" ] && [ -z \"$DOCKER_HOST\" ] || exit 9\nif [ \"$1\" = --list ]; then /bin/cat running; else /bin/sh action; fi\n",
            ),
        ] {
            let path = room.path().join(name);
            std::fs::write(&path, script).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        std::fs::write(room.path().join("running"), "docker-desktop\n").unwrap();
        std::fs::write(
            room.path().join("action"),
            "printf 'engine\\t42\\t123\\tmnt:[9]\\t10240\\t/var/lib/docker\\n'\n",
        )
        .unwrap();
        let env = BTreeMap::from([
            ("PATH".into(), room.path().display().to_string()),
            ("HOME".into(), room.path().display().to_string()),
            (
                "DOCKER_HOST".into(),
                "unix:///nonexistent/desktop.sock".into(),
            ),
            (
                "SECRET_CANARY".into(),
                "synthetic-observation-canary".into(),
            ),
        ]);
        (room, env, info)
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn desktop_observation_binds_selection_and_rejects_failures() {
        let (room, mut env, info) = desktop_fixture();
        assert_eq!(
            observe_resources(
                room.path(),
                room.path(),
                Some(&env),
                std::time::Instant::now() + std::time::Duration::from_secs(12),
            )
            .engine_storage
            .value,
            Some(10485760)
        );
        // Context takes precedence over a conflicting host, including the recheck.
        env.insert("DOCKER_CONTEXT".into(), "chosen".into());
        env.insert("DOCKER_HOST".into(), "tcp://ignored:2375".into());
        assert_eq!(
            observe_resources(
                room.path(),
                room.path(),
                Some(&env),
                std::time::Instant::now() + std::time::Duration::from_secs(12),
            )
            .engine_storage
            .value,
            Some(10485760)
        );
        env.remove("DOCKER_CONTEXT");
        for endpoint in [
            "tcp://remote:2375",
            "ssh://remote",
            "npipe:////pipe/docker",
            "unix://relative",
            "unix:///var/../run/docker.sock",
        ] {
            assert_eq!(
                desktop_storage(
                    &info,
                    Some(endpoint),
                    &env,
                    std::time::Instant::now() + std::time::Duration::from_secs(12)
                ),
                None
            );
        }
        let endpoint = "unix:///nonexistent/desktop.sock";
        for (key, value) in [
            ("ID", ""),
            ("ID", "bad\tidentity"),
            ("DockerRootDir", "/var/../lib"),
            ("OperatingSystem", "Linux"),
        ] {
            let mut invalid = info.clone();
            invalid[key] = value.into();
            assert_eq!(
                desktop_storage(
                    &invalid,
                    Some(endpoint),
                    &env,
                    std::time::Instant::now() + std::time::Duration::from_secs(12)
                ),
                None
            );
        }
        env.insert("DOCKER_HOST".into(), endpoint.into());
        for script in [
            "exit 17",
            "exit 19",
            "printf truncated",
            "printf 'wrong\\t42\\t123\\tmnt:[9]\\t1\\t/var/lib/docker\\n'",
            "/usr/bin/head -c 20000 /dev/zero",
            "/usr/bin/head -c 20000 /dev/zero >&2",
            "printf 'engine\\t42\\t123\\tmnt:[9]\\t1\\t/var/lib/docker\\n'; echo '{}' > info.json",
        ] {
            std::fs::write(room.path().join("info.json"), info.to_string()).unwrap();
            std::fs::write(room.path().join("action"), script).unwrap();
            assert_eq!(
                desktop_storage(
                    &info,
                    Some(endpoint),
                    &env,
                    std::time::Instant::now() + std::time::Duration::from_secs(12)
                ),
                None,
                "{script}"
            );
        }
        std::fs::write(room.path().join("action"), "printf 'engine\\t42\\t123\\tmnt:[9]\\t1\\t/var/lib/docker\\n'; echo tcp://changed > endpoint").unwrap();
        env.insert("DOCKER_CONTEXT".into(), "chosen".into());
        std::fs::write(room.path().join("info.json"), info.to_string()).unwrap();
        assert_eq!(
            desktop_storage(
                &info,
                Some(endpoint),
                &env,
                std::time::Instant::now() + std::time::Duration::from_secs(12)
            ),
            None
        );
        std::fs::write(room.path().join("running"), "Ubuntu\n").unwrap();
        std::fs::write(room.path().join("action"), "touch should-not-enter").unwrap();
        assert_eq!(
            desktop_storage(
                &info,
                Some(endpoint),
                &env,
                std::time::Instant::now() + std::time::Duration::from_secs(12)
            ),
            None
        );
        assert!(!room.path().join("should-not-enter").exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn desktop_containerized_storage_observer_binds_current_identity() {
        let (room, env, mut info) = desktop_fixture();
        info["OperatingSystem"] = "Docker Desktop (containerized)".into();
        std::fs::write(room.path().join("info.json"), info.to_string()).unwrap();
        let observed = observe_resources(
            room.path(),
            room.path(),
            Some(&env),
            std::time::Instant::now() + std::time::Duration::from_secs(12),
        );
        assert_eq!(observed.engine_id.as_deref(), Some("engine"));
        assert_eq!(observed.engine_storage.value, Some(10485760));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn desktop_storage_rejects_near_match_and_unknown_names() {
        // P19-AUD-008: only the two exact Desktop identities bind the
        // namespace observer; near matches and unknown names stay
        // unknown storage without entering the WSL observation.
        let (room, env, mut info) = desktop_fixture();
        for name in [
            "docker desktop",
            "Docker Desktop (containerized)",
            "Docker Desktop on Ubuntu",
            "Docker Desktop Enterprise",
            "Ubuntu",
        ] {
            info["OperatingSystem"] = name.into();
            std::fs::write(room.path().join("info.json"), info.to_string()).unwrap();
            let expected = if name == "Docker Desktop (containerized)" {
                Some(10485760)
            } else {
                None
            };
            std::fs::write(
                room.path().join("action"),
                if expected.is_some() {
                    "printf 'engine\\t42\\t123\\tmnt:[9]\\t10240\\t/var/lib/docker\\n'\n"
                } else {
                    "touch should-not-enter\n"
                },
            )
            .unwrap();
            assert_eq!(
                desktop_storage(
                    &info,
                    Some("unix:///nonexistent/desktop.sock"),
                    &env,
                    std::time::Instant::now() + std::time::Duration::from_secs(12),
                ),
                expected,
                "{name}"
            );
            if expected.is_none() {
                assert!(!room.path().join("should-not-enter").exists(), "{name}");
            }
        }
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn desktop_observation_supports_tokio_and_bounds_commands() {
        let (room, env, _) = desktop_fixture();
        assert_eq!(
            observe_resources(
                room.path(),
                room.path(),
                Some(&env),
                std::time::Instant::now() + std::time::Duration::from_secs(12),
            )
            .engine_storage
            .value,
            Some(10485760)
        );
        let deadline = std::time::Duration::from_millis(50);
        let started = std::time::Instant::now();
        assert_eq!(
            observation_command(vec!["/bin/sleep".into(), "30".into()], &env, deadline),
            None
        );
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        for argv in [
            vec!["/absent-observer".into()],
            vec![room.path().join("action").into_os_string()],
        ] {
            assert_eq!(observation_command(argv, &env, deadline), None);
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn awaited_observers_settle_under_cancellation_and_bounds() {
        // P19-AUD-003: awaited observer probes honor the preparation
        // cancellation token and finite bounds instead of blocking the
        // current-thread executor, and refusal reasons stay truthful and
        // bounded for stalled and overflowing observers.
        let cancel = tokio_util::sync::CancellationToken::new();
        cancel.cancel();
        let started = std::time::Instant::now();
        assert_eq!(
            supervised_docker_endpoint(&cancel, std::time::Instant::now() + OBSERVER_PROBE_BOUND)
                .await,
            None
        );
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
        let environment = std::collections::BTreeMap::new();
        let stalled = supervised_observation(
            vec!["/bin/sleep".into(), "30".into()],
            &environment,
            std::time::Duration::from_millis(150),
        )
        .expect("supervision runs");
        assert!(matches!(stalled.exit, ExitState::TimedOut));
        assert!(observation_refusal("probe", &stalled).contains("timed out"));
        let overflowing = supervised_observation(
            vec![
                "/bin/sh".into(),
                "-c".into(),
                "head -c 131072 /dev/zero".into(),
            ],
            &environment,
            OBSERVER_PROBE_BOUND,
        )
        .expect("supervision runs");
        assert!(overflowing.stdout.truncated);
        assert!(!observation_clean(&overflowing));
        assert!(
            observation_refusal("probe", &overflowing).contains("capture ceiling"),
            "{}",
            observation_refusal("probe", &overflowing)
        );
        let refused = supervised_observation(
            vec![
                "/bin/sh".into(),
                "-c".into(),
                "echo refused >&2; exit 7".into(),
            ],
            &environment,
            OBSERVER_PROBE_BOUND,
        )
        .expect("supervision runs");
        assert!(!observation_clean(&refused));
        assert_eq!(
            observation_refusal("engine probe", &refused),
            "engine probe: refused"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn desktop_remote_script_rejects_namespace_and_process_drift() {
        use std::os::unix::fs::PermissionsExt;
        use std::os::unix::fs::symlink;
        for case in [
            "ok",
            "no-match",
            "ambiguous",
            "missing",
            "identity",
            "process",
            "mount",
            "negative",
            "nonnumeric",
        ] {
            let room = tempfile::tempdir().unwrap();
            let proc = room.path().join("proc");
            for pid in [42, 43] {
                let dir = proc.join(pid.to_string());
                std::fs::create_dir_all(dir.join("root/var/lib/docker")).unwrap();
                std::fs::create_dir_all(dir.join("ns")).unwrap();
                std::fs::write(dir.join("comm"), "dockerd\n").unwrap();
                std::fs::write(
                    dir.join("stat"),
                    format!("{} 123\n", vec!["0"; 21].join(" ")),
                )
                .unwrap();
                symlink("mnt:[9]", dir.join("ns/mnt")).unwrap();
                std::fs::write(
                    dir.join("root/var/lib/docker/engine-id"),
                    if case == "no-match" || (pid == 43 && case != "ambiguous") {
                        "other\n"
                    } else {
                        "engine\n"
                    },
                )
                .unwrap();
            }
            let dir = proc.join("42");
            let effect = match case {
                "missing" => format!(
                    "rm '{}'",
                    dir.join("root/var/lib/docker/engine-id").display()
                ),
                "identity" => format!(
                    "echo replacement > '{}'",
                    dir.join("root/var/lib/docker/engine-id").display()
                ),
                "process" => format!("echo replaced > '{}'", dir.join("stat").display()),
                "mount" => format!(
                    "rm '{0}'; ln -s 'mnt:[10]' '{0}'",
                    dir.join("ns/mnt").display()
                ),
                _ => ":".into(),
            };
            let available = match case {
                "negative" => "-1",
                "nonnumeric" => "NaN",
                _ => "10240",
            };
            let df = room.path().join("df");
            std::fs::write(&df, format!("#!/bin/sh\n{effect}\nprintf 'Filesystem 1024-blocks Used Available Capacity Mounted\\nfixture 20000 1 {available} 1%% /\\n'\n")).unwrap();
            std::fs::set_permissions(&df, std::fs::Permissions::from_mode(0o755)).unwrap();
            // The production script is unchanged apart from its test-only proc tree.
            let script = DESKTOP_STORAGE_SCRIPT.replace("/proc/", &format!("{}/", proc.display()));
            let out = Command::new("/bin/sh")
                .args(["-c", &script, "test", "engine", "/var/lib/docker"])
                .env_clear()
                .env("PATH", format!("{}:/usr/bin:/bin", room.path().display()))
                .output()
                .unwrap();
            assert_eq!(out.status.success(), case == "ok", "{case}: {out:?}");
            if case == "ok" {
                assert_eq!(
                    desktop_storage_sample(&out.stdout, "engine", "/var/lib/docker"),
                    Some(10485760)
                );
            }
        }
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
/// a model or reads provider credentials. Writes are limited to the declared
/// cache/output tree and admitted immutable images in the selected engine.
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
    let acquisition_deadline = std::time::Instant::now() + std::time::Duration::from_secs(3600);
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
    // source cache miss invokes it under supervision - the plan
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
            timeout: acquisition_deadline.saturating_duration_since(std::time::Instant::now()),
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
    let lock_value: serde_json::Value = serde_json::from_slice(&lock_bytes)
        .map_err(|error| PrepareError::Rejected(error.to_string()))?;
    let pin = lock_value["subjects"]
        .as_array()
        .and_then(|subjects| subjects.iter().find(|subject| subject["id"] == adapter))
        .ok_or_else(|| {
            PrepareError::Rejected(format!("static lock has no {adapter} corpus pin"))
        })?;
    let corpus = super::material::CorpusMaterial {
        root: corpus_root,
        source_commit: pin["commit"].as_str().unwrap_or_default().to_owned(),
        tasks_tree: pin["tasks_tree"].as_str().unwrap_or_default().to_owned(),
    };
    let corpus_digest = corpus
        .verified_manifest_digest(&adapter, &lock_value)
        .map_err(|error| PrepareError::Rejected(error.to_string()))?;
    // Supervised, cancellation-aware verifier identity probe
    // (P19-AUD-003): the observation awaits `ProcessSupervisor` with the
    // preparation token and a finite bound instead of blocking the
    // current-thread executor on a raw child.
    let uv_probe = SpawnSpec {
        argv: vec![verifier.clone().into(), "--version".into()],
        cwd: None,
        env: BTreeMap::new(),
        stdout_cap: 16 * 1024,
        stderr_cap: 16 * 1024,
        timeout: acquisition_deadline
            .saturating_duration_since(std::time::Instant::now())
            .min(OBSERVER_PROBE_BOUND),
    };
    let version = ProcessSupervisor::run(&uv_probe, cancel).await;
    if !observation_clean(&version)
        || String::from_utf8_lossy(&version.stdout.bytes).trim() != format!("uv {uv_version}")
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
        } else if key == &format!("corpora:{adapter}") {
            corpus_digest.clone()
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
        let endpoint = supervised_docker_endpoint(cancel, acquisition_deadline).await;
        if let Some(endpoint) = endpoint {
            verifier_env.insert("DOCKER_HOST".to_owned(), endpoint);
        }
    }
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
    let image_environment: BTreeMap<String, String> = [
        "HOME",
        "PATH",
        "DOCKER_HOST",
        "DOCKER_CONTEXT",
        "DOCKER_CONFIG",
        "DOCKER_TLS_VERIFY",
        "DOCKER_CERT_PATH",
    ]
    .into_iter()
    .filter_map(|key| std::env::var(key).ok().map(|value| (key.to_owned(), value)))
    .collect();
    let observations = observe_resources(
        &context.cache,
        out,
        Some(&image_environment),
        std::time::Instant::now() + std::time::Duration::from_secs(12),
    );
    let mut image_plan = BTreeMap::new();
    for task in &inventory_tasks {
        let id = task.dir.file_name().unwrap().to_string_lossy();
        if !selected.iter().any(|selected| selected == id.as_ref()) {
            continue;
        }
        let requirements =
            effective_task_requirements(&task.dir, family, context.experiment.execution())
                .map_err(|error| PrepareError::Rejected(error.to_string()))?;
        let checked = check_requirements(&requirements, &observations);
        if checked["status"] != "ok" {
            return Err(PrepareError::Rejected(format!(
                "task {id} resource preflight: {}",
                checked["reason"]
            )));
        }
        let preflight =
            super::material::image_preflight(&task.dir, &lock_value, &image_environment);
        if preflight["status"] == "unmet" {
            return Err(PrepareError::Rejected(format!(
                "task {id} image preflight: {}",
                preflight["reason"]
            )));
        }
        for image in preflight["images"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|image| image["status"] != "known")
        {
            let reference = image["image"].as_str().expect("preflight image reference");
            let pins: Vec<_> = lock_value["images"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|pin| {
                    pin["reference"].as_str().is_some_and(|name| {
                        super::material::resolved_image(name, &lock_value)
                            .ok()
                            .as_deref()
                            == Some(reference)
                    })
                })
                .collect();
            if pins.len() != 1 {
                return Err(PrepareError::Rejected(format!(
                    "image {reference} requires one exact static acquisition pin"
                )));
            }
            let pin_id = pins[0]["id"]
                .as_str()
                .filter(|id| {
                    !id.is_empty()
                        && id
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
                })
                .ok_or_else(|| {
                    PrepareError::Rejected("image acquisition pin has an invalid id".to_owned())
                })?;
            image_plan.insert(pin_id.to_owned(), serde_json::json!({"id":format!("images/{pin_id}"),"key":format!("images:{pin_id}"),"products":products,"benchmark":adapter,"task_ids":selected}));
        }
    }
    if !image_plan.is_empty() {
        let materializer = context
            .cache
            .join("pins/scripts/materialize-external-locks.sh");
        if !materializer.is_file()
            || !lock
                .authority
                .producers
                .iter()
                .any(|pin| Path::new(&pin.path).file_name() == materializer.file_name())
        {
            return Err(PrepareError::Rejected(
                "missing images require the pinned acquisition materializer".to_owned(),
            ));
        }
        let identity = image_engine_identity(&image_environment)?;
        if observations.engine_id.as_deref() != Some(identity.0.as_str()) {
            return Err(PrepareError::Rejected(
                "selected engine changed before image acquisition".to_owned(),
            ));
        }
        let image_out = out.join("image-acquisition");
        std::fs::create_dir(&image_out)?;
        let plan_path = image_out.join("acquisition-plan.json");
        std::fs::write(
            &plan_path,
            serde_json::to_vec_pretty(&image_plan.values().collect::<Vec<_>>())
                .expect("image plan serializes"),
        )?;
        let timeout = acquisition_deadline.saturating_duration_since(std::time::Instant::now());
        if timeout.is_zero() {
            return Err(PrepareError::Rejected(
                "material acquisition deadline elapsed".to_owned(),
            ));
        }
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
            env: image_environment
                .iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
            stdout_cap: 1_048_576,
            stderr_cap: 1_048_576,
            timeout,
        };
        settle_acquisition(&spec, &image_out, cancel).await?;
        if image_engine_identity(&image_environment)? != identity {
            return Err(PrepareError::Rejected(
                "selected engine changed during image acquisition".to_owned(),
            ));
        }
        acquired.extend(image_plan.keys().map(|id| format!("images:{id}")));
        acquired.sort();
    }
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
        let images = super::material::image_preflight(&task.dir, &lock_value, &image_environment);
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
    let capture_helper = acquire_capture_helper(
        &context.cache,
        opi_source.as_deref(),
        out,
        cancel,
        acquisition_deadline,
        &mut acquired,
    )
    .await?;
    let material = serde_json::json!({
        "schema": super::material::MATERIAL_SCHEMA,
        "origin": "local",
        "capture_helper": capture_helper,
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
                    "interpreter": {
                        "path": runner_project.canonicalize()?.join(".venv/bin/python"),
                        "sha256": inventory.entries[&format!("runners:{runner}:python")].sha256,
                    },
                    "entrypoint": {
                        "path": runner_project.canonicalize()?.join(".venv/bin").join(runner),
                        "sha256": inventory.entries[&format!("runners:{runner}:entrypoint")].sha256,
                    },
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

/// Bind image acquisition to the same selected endpoint, engine and storage root.
fn image_engine_identity(
    environment: &BTreeMap<String, String>,
) -> Result<(String, String, String), PrepareError> {
    let docker = |args: &[&str]| {
        observation_command(
            std::iter::once("docker")
                .chain(args.iter().copied())
                .map(Into::into)
                .collect(),
            environment,
            std::time::Duration::from_secs(5),
        )
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map(|text| text.trim().to_owned())
    };
    let snapshot = || -> Option<(String, String, String)> {
        let endpoint = selected_endpoint(environment, &docker)?;
        let info: serde_json::Value =
            serde_json::from_str(&docker(&["info", "--format", ENGINE_INFO_FORMAT])?).ok()?;
        let id = info["ID"].as_str().filter(|id| !id.is_empty())?;
        let root = info["DockerRootDir"]
            .as_str()
            .filter(|root| normalized_absolute(root))?;
        Some((id.to_owned(), root.to_owned(), endpoint))
    };
    snapshot().ok_or_else(|| {
        PrepareError::Rejected(
            "selected engine identity is unobservable for image acquisition".to_owned(),
        )
    })
}

/// Build-tool inputs projected over the supervisor's empty environment.
/// The warm-cache sidecar pin of the shared capture helper.
#[derive(Debug, Serialize, Deserialize)]
struct CaptureHelperPin {
    schema: String,
    target: String,
    protocol: String,
    cargo_lock_sha256: String,
    sha256: String,
}

/// Acquires the shared capture helper (P19-AUD-003): warm cache bytes
/// with a matching sidecar pin first; a cold build requires the exact
/// local Opi source and compiles the Companion for the host-arch musl
/// target through the Cargo compiler-artifact locator. A musl helper
/// must be statically linked (no dynamic loader); unavailable targets,
/// failed builds and dynamically linked results all refuse.
async fn acquire_capture_helper(
    cache: &Path,
    opi_source: Option<&Path>,
    out: &Path,
    cancel: &tokio_util::sync::CancellationToken,
    deadline: std::time::Instant,
    acquired: &mut Vec<String>,
) -> Result<serde_json::Value, PrepareError> {
    let tools = cache.join("tools");
    let executable = tools.join("native-capture-helper");
    let sidecar = tools.join("native-capture-helper.json");
    let protocol = crate::native_capture::CAPTURE_HELPER_PROTOCOL.to_owned();
    let sha256_of = |path: &Path| sha256_hex(&std::fs::read(path).unwrap_or_default());
    if executable.is_file() && sidecar.is_file() {
        let pin: CaptureHelperPin =
            serde_json::from_slice(&std::fs::read(&sidecar)?).map_err(|error| {
                PrepareError::Rejected(format!("capture helper pin parse failed: {error}"))
            })?;
        let warm = pin.schema == "opi-eval-capture-helper-pin/1"
            && pin.protocol == protocol
            && sha256_of(&executable) == pin.sha256
            && !pin.target.is_empty()
            && pin.cargo_lock_sha256.len() == 64;
        if warm {
            return Ok(serde_json::json!({
                "executable": {"path": executable, "sha256": pin.sha256},
                "target": pin.target,
                "protocol": pin.protocol,
                "cargo_lock_sha256": pin.cargo_lock_sha256,
            }));
        }
        return Err(PrepareError::Rejected(
            "cache/tools/native-capture-helper(.json) drift: rebuild the helper pin".to_owned(),
        ));
    }
    let source = opi_source.ok_or_else(|| {
        PrepareError::Rejected(
            "fresh native material requires the capture helper: seed \
cache/tools/native-capture-helper from a completed build or pass --opi-source \
so prepare can compile it for the host musl target"
                .to_owned(),
        )
    })?;
    let arch = std::env::consts::ARCH;
    if !matches!(arch, "x86_64" | "aarch64") {
        return Err(PrepareError::Rejected(format!(
            "no capture helper target for host architecture {arch:?}"
        )));
    }
    let target = format!("{arch}-unknown-linux-musl");
    let cargo_lock = source.join("Cargo.lock");
    if !cargo_lock.is_file() {
        return Err(PrepareError::Rejected(format!(
            "--opi-source has no Cargo.lock at {}",
            cargo_lock.display()
        )));
    }
    let cargo_lock_sha256 = sha256_hex(&std::fs::read(&cargo_lock)?);
    let helper_target = out.join("helper-target");
    std::fs::create_dir_all(&helper_target)?;
    let mut env = acquisition_environment();
    env.insert(
        "CARGO_TARGET_DIR".into(),
        helper_target.canonicalize()?.into_os_string(),
    );
    let spec = SpawnSpec {
        argv: [
            "cargo".into(),
            "build".into(),
            "--locked".into(),
            "--release".into(),
            "--manifest-path".into(),
            source.join("Cargo.toml").into_os_string(),
            "--package".into(),
            "opi-eval".into(),
            "--bin".into(),
            "opi-eval".into(),
            "--target".into(),
            target.clone().into(),
            "--message-format=json".into(),
        ]
        .to_vec(),
        cwd: None,
        env,
        stdout_cap: 8 * 1024 * 1024,
        stderr_cap: 1024 * 1024,
        timeout: deadline.saturating_duration_since(std::time::Instant::now()),
    };
    let staging = out.join("helper-build-staging");
    std::fs::create_dir(&staging)?;
    let mut spec = spec.clone();
    spec.env
        .insert("TMPDIR".into(), staging.canonicalize()?.into_os_string());
    let invocation = ProcessSupervisor::run(&spec, cancel).await;
    std::fs::remove_dir_all(&staging).ok();
    std::fs::write(
        out.join("helper-build.log"),
        format!(
            "argv: {:?}\nexit: {:?}\nstdout_truncated: {}\nstderr_truncated: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            spec.argv,
            invocation.exit,
            invocation.stdout.truncated,
            invocation.stderr.truncated,
            String::from_utf8_lossy(&invocation.stdout.bytes),
            String::from_utf8_lossy(&invocation.stderr.bytes),
        ),
    )?;
    if invocation.exit != (ExitState::Exited { code: 0 }) {
        return Err(PrepareError::Rejected(format!(
            "capture helper build failed for target {target} (see {})",
            out.join("helper-build.log").display()
        )));
    }
    // The Cargo compiler-artifact locator: the last bin artifact for the
    // requested target names the exact executable to pin.
    let mut helper_path = None;
    for line in String::from_utf8_lossy(&invocation.stdout.bytes).lines() {
        let Ok(message) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if message.get("reason").and_then(|value| value.as_str()) != Some("compiler-artifact") {
            continue;
        }
        let Some(target_fields) = message.get("target").and_then(|value| value.as_object()) else {
            continue;
        };
        let kind = target_fields
            .get("kind")
            .and_then(|value| value.as_array())
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| row.as_str().map(str::to_owned))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if target_fields.get("name").and_then(|value| value.as_str()) != Some("opi-eval")
            || !kind.iter().any(|kind| kind == "bin")
        {
            continue;
        }
        if let Some(path) = message
            .get("executable")
            .and_then(|value| value.as_str())
            .filter(|path| path.contains(&target))
        {
            helper_path = Some(std::path::PathBuf::from(path));
        }
    }
    let helper_path = helper_path.ok_or_else(|| {
        PrepareError::Rejected(format!(
            "no opi-eval compiler-artifact executable was reported for target {target}"
        ))
    })?;
    if !helper_path.is_file() {
        return Err(PrepareError::Rejected(format!(
            "reported capture helper executable is missing at {}",
            helper_path.display()
        )));
    }
    let bytes = std::fs::read(&helper_path)?;
    if elf_has_dynamic_loader(&bytes) {
        return Err(PrepareError::Rejected(
            "musl capture helper must not depend on a dynamic loader".to_owned(),
        ));
    }
    std::fs::create_dir_all(&tools)?;
    std::fs::write(&executable, &bytes)?;
    let digest = sha256_hex(&bytes);
    let pin = CaptureHelperPin {
        schema: "opi-eval-capture-helper-pin/1".to_owned(),
        target: target.clone(),
        protocol: protocol.clone(),
        cargo_lock_sha256: cargo_lock_sha256.clone(),
        sha256: digest.clone(),
    };
    std::fs::write(
        &sidecar,
        serde_json::to_vec_pretty(&pin).expect("helper pin serializes"),
    )?;
    acquired.push("tools:native-capture-helper".to_owned());
    Ok(serde_json::json!({
        "executable": {"path": executable, "sha256": digest},
        "target": target,
        "protocol": protocol,
        "cargo_lock_sha256": cargo_lock_sha256,
    }))
}

/// Minimal ELF program-header scan: true when the binary declares an
/// interpreter (dynamic loader). Little-endian ELF64 only, matching the
/// admitted musl targets.
fn elf_has_dynamic_loader(bytes: &[u8]) -> bool {
    if bytes.len() < 64 || &bytes[0..4] != b"\x7fELF" || bytes[4] != 2 || bytes[5] != 1 {
        return true;
    }
    let phoff = u64::from_le_bytes(bytes[0x20..0x28].try_into().expect("slice")) as usize;
    let phentsize = u16::from_le_bytes(bytes[0x36..0x38].try_into().expect("slice")) as usize;
    let phnum = u16::from_le_bytes(bytes[0x38..0x3a].try_into().expect("slice")) as usize;
    for index in 0..phnum {
        let start = phoff + index * phentsize;
        let end = start + 4;
        if end > bytes.len() {
            return true;
        }
        if u32::from_le_bytes(bytes[start..end].try_into().expect("slice")) == 3 {
            return true;
        }
    }
    false
}

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

// Every command observes only the selected engine/host, with bounded captures.
// A dedicated runtime also permits callers already inside Tokio to stay synchronous.
const ENGINE_INFO_FORMAT: &str = r#"{"ID":{{json .ID}},"DockerRootDir":{{json .DockerRootDir}},"OperatingSystem":{{json .OperatingSystem}},"NCPU":{{json .NCPU}},"MemTotal":{{json .MemTotal}},"Runtimes":{{json .Runtimes}}}"#;

/// Finite per-probe ceiling for every production observer. Healthy
/// observers settle in milliseconds; the bound turns a stalled or
/// overflowing observer into a truthful refusal instead of an
/// unbounded wait, and callers' remaining operation deadlines can only
/// lower it.
pub(crate) const OBSERVER_PROBE_BOUND: std::time::Duration = std::time::Duration::from_secs(5);

/// Bound one observer by the caller's remaining operation deadline.
fn observer_bound(deadline: std::time::Instant) -> std::time::Duration {
    deadline
        .saturating_duration_since(std::time::Instant::now())
        .min(OBSERVER_PROBE_BOUND)
}

/// Whether a supervised observation settled cleanly: exit zero, both
/// captures within their ceilings, and verified owned-tree cleanup.
fn observation_clean(result: &crate::process::SupervisedOutcome) -> bool {
    result.exit == (ExitState::Exited { code: 0 })
        && !result.stdout.truncated
        && !result.stderr.truncated
        && matches!(
            result.cleanup,
            CleanupEvidence::NotRequired | CleanupEvidence::TreeTerminated { verified: true, .. }
        )
}

/// A truthful bounded refusal reason for one supervised observation.
fn observation_refusal(context: &str, result: &crate::process::SupervisedOutcome) -> String {
    let reason = match result.exit {
        ExitState::FailedToSpawn {
            reason: crate::process::SpawnReason::NotFound,
        } => "observer executable not found".to_owned(),
        ExitState::FailedToSpawn { reason } => {
            format!("observer could not start ({reason:?})")
        }
        ExitState::TimedOut => "observer timed out under bounded supervision".to_owned(),
        ExitState::Cancelled => "observer cancelled".to_owned(),
        ExitState::Exited { code } if code != 0 => {
            let stderr = String::from_utf8_lossy(&result.stderr.bytes);
            match stderr.lines().map(str::trim).find(|line| !line.is_empty()) {
                Some(line) => line.chars().take(200).collect::<String>(),
                None => format!("observer exited with status {code}"),
            }
        }
        ExitState::Exited { .. } => {
            if result.stdout.truncated || result.stderr.truncated {
                "observer output exceeded the capture ceiling".to_owned()
            } else {
                "observer tree cleanup could not be verified".to_owned()
            }
        }
    };
    format!("{context}: {reason}")
}

/// Runs one observation under `ProcessSupervisor` on a private thread
/// runtime. `None` only reports supervision itself failing (thread or
/// runtime); every probe outcome — including timeout, refusal, and
/// truncation — is a returned outcome callers classify.
fn supervised_observation(
    argv: Vec<std::ffi::OsString>,
    environment: &BTreeMap<String, String>,
    timeout: std::time::Duration,
) -> Option<crate::process::SupervisedOutcome> {
    if timeout.is_zero() {
        return None;
    }
    let spec = SpawnSpec {
        argv,
        cwd: None,
        env: environment
            .iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect(),
        stdout_cap: 16 * 1024,
        stderr_cap: 16 * 1024,
        timeout,
    };
    std::thread::Builder::new()
        .name("engine-observation".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .ok()?;
            let result = runtime.block_on(ProcessSupervisor::run(
                &spec,
                &tokio_util::sync::CancellationToken::new(),
            ));
            Some(result)
        })
        .ok()?
        .join()
        .ok()?
}

pub(super) fn observation_command(
    argv: Vec<std::ffi::OsString>,
    environment: &BTreeMap<String, String>,
    timeout: std::time::Duration,
) -> Option<Vec<u8>> {
    supervised_observation(argv, environment, timeout)
        .filter(observation_clean)
        .map(|result| result.stdout.bytes)
}

/// The selected Docker endpoint through one supervised, awaited probe
/// (P19-AUD-003): asynchronous callers keep their current-thread
/// executor free and honor cancellation instead of blocking on a raw
/// child. A refused, stalled, or overflowing probe yields `None`.
async fn supervised_docker_endpoint(
    cancel: &tokio_util::sync::CancellationToken,
    deadline: std::time::Instant,
) -> Option<String> {
    let bound = observer_bound(deadline);
    if bound.is_zero() {
        return None;
    }
    let spec = SpawnSpec {
        argv: vec![
            "docker".into(),
            "context".into(),
            "inspect".into(),
            "--format".into(),
            "{{(index .Endpoints \"docker\").Host}}".into(),
        ],
        cwd: None,
        env: observation_environment(None)
            .iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect(),
        stdout_cap: 16 * 1024,
        stderr_cap: 16 * 1024,
        timeout: bound,
    };
    let outcome = ProcessSupervisor::run(&spec, cancel).await;
    observation_clean(&outcome).then(|| {
        String::from_utf8_lossy(&outcome.stdout.bytes)
            .trim()
            .to_owned()
    })
}

fn normalized_absolute(path: &str) -> bool {
    path.starts_with('/')
        && !path.bytes().any(|byte| byte.is_ascii_control())
        && (path == "/"
            || path[1..]
                .split('/')
                .all(|part| !matches!(part, "" | "." | "..")))
}

// The remote timeout bounds work even if the local WSL bridge is terminated.
// ID/root are arguments, never interpolated shell source. No container is started.
const DESKTOP_STORAGE_SCRIPT: &str = r#"
set -eu
engine=$1
root=$2
case "$root" in /*) ;; *) exit 16;; esac
case "$root" in /) ;; */) exit 16;; esac
case "$root/" in *"/../"*|*"/./"*|*"//"*) exit 16;; esac
match=""
count=0
for comm in /proc/[0-9]*/comm; do
    [ "$(cat "$comm" 2>/dev/null || true)" = dockerd ] || continue
    pid=${comm#/proc/}; pid=${pid%/comm}
    candidate="/proc/$pid/root$root"
    actual=$(cat "$candidate/engine-id" 2>/dev/null || true)
    [ -n "$actual" ] && [ "$actual" = "$engine" ] || continue
    match=$pid
    count=$((count + 1))
done
[ "$count" -eq 1 ] || exit 17
pid=$match
target="/proc/$pid/root$root"
start=$(awk '{print $22}' "/proc/$pid/stat")
mount=$(readlink "/proc/$pid/ns/mnt")
before=$(cat "$target/engine-id")
space=$(df -Pk "$target" | awk 'NR==2 {print $4}')
case "$space" in ""|*[!0-9]*) exit 18;; esac
[ "$before" = "$engine" ] || exit 19
[ "$(cat "/proc/$pid/comm")" = dockerd ] || exit 19
[ "$(cat "$target/engine-id")" = "$before" ] || exit 19
[ "$(awk '{print $22}' "/proc/$pid/stat")" = "$start" ] || exit 19
[ "$(readlink "/proc/$pid/ns/mnt")" = "$mount" ] || exit 19
printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$engine" "$pid" "$start" "$mount" "$space" "$root"
"#;

fn desktop_storage_sample(bytes: &[u8], engine: &str, root: &str) -> Option<u64> {
    let line = std::str::from_utf8(bytes).ok()?.strip_suffix('\n')?;
    let fields: Vec<_> = line.split('\t').collect();
    let numeric = |value: &str| {
        (!value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()))
            .then(|| value.parse::<u64>().ok())
            .flatten()
    };
    if fields.len() != 6
        || fields[0] != engine
        || fields[5] != root
        || numeric(fields[1])? == 0
        || numeric(fields[2])? == 0
        || numeric(fields[3].strip_prefix("mnt:[")?.strip_suffix(']')?)? == 0
    {
        return None;
    }
    numeric(fields[4])?.checked_mul(1024)
}

fn desktop_is_running(bytes: &[u8]) -> bool {
    let text = if bytes.contains(&0) {
        if !bytes.len().is_multiple_of(2) {
            return false;
        }
        let units: Vec<_> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes(*b))
            .collect();
        String::from_utf16(&units).ok()
    } else {
        String::from_utf8(bytes.to_vec()).ok()
    };
    text.is_some_and(|text| {
        text.trim_start_matches('\u{feff}')
            .lines()
            .any(|name| name.trim() == "docker-desktop")
    })
}

/// The exact Desktop operating-system identities whose storage binds
/// through the bounded WSL namespace observer. Near matches and unknown
/// names stay unknown (P19-AUD-008).
const DESKTOP_OS_IDENTITIES: &[&str] = &["Docker Desktop", "Docker Desktop (containerized)"];

fn desktop_storage(
    info: &serde_json::Value,
    endpoint: Option<&str>,
    environment: &BTreeMap<String, String>,
    deadline: std::time::Instant,
) -> Option<u64> {
    if !cfg!(target_os = "linux")
        || !DESKTOP_OS_IDENTITIES.contains(&info["OperatingSystem"].as_str().unwrap_or(""))
    {
        return None;
    }
    let endpoint = endpoint?;
    let engine = info["ID"].as_str()?;
    let root = info["DockerRootDir"].as_str()?;
    if engine.is_empty()
        || engine.len() > 256
        || !engine
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_:".contains(&b))
        || !normalized_absolute(root)
        || !normalized_absolute(endpoint.strip_prefix("unix://")?)
    {
        return None;
    }
    let executable = |path: &Path| {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            path.metadata()
                .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        }
        #[cfg(not(unix))]
        {
            path.is_file()
        }
    };
    let wsl = std::env::split_paths(environment.get("PATH")?)
        .map(|dir| dir.join("wsl.exe"))
        .chain([PathBuf::from("/mnt/c/Windows/system32/wsl.exe")])
        .find(|path| executable(path))?;
    let host_environment = environment
        .iter()
        .filter(|(k, _)| matches!(k.as_str(), "HOME" | "PATH"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let remaining = || deadline.saturating_duration_since(std::time::Instant::now());
    let running = observation_command(
        vec![
            wsl.clone().into_os_string(),
            "--list".into(),
            "--running".into(),
            "--quiet".into(),
        ],
        &host_environment,
        remaining(),
    )?;
    if !desktop_is_running(&running) {
        return None;
    }
    let sample = observation_command(
        [wsl.into_os_string()]
            .into_iter()
            .chain(
                [
                    "--distribution",
                    "docker-desktop",
                    "--user",
                    "root",
                    "--exec",
                    "timeout",
                    "-k",
                    "1",
                    "5",
                    "sh",
                    "-c",
                    DESKTOP_STORAGE_SCRIPT,
                    "observe-storage",
                    engine,
                    root,
                ]
                .into_iter()
                .map(Into::into),
            )
            .collect(),
        &host_environment,
        remaining(),
    )?;
    let available = desktop_storage_sample(&sample, engine, root)?;
    let docker = |args: &[&str]| {
        let bytes = observation_command(
            std::iter::once("docker")
                .chain(args.iter().copied())
                .map(Into::into)
                .collect(),
            environment,
            remaining(),
        )?;
        String::from_utf8(bytes)
            .ok()
            .map(|text| text.trim().to_owned())
    };
    let after: serde_json::Value =
        serde_json::from_str(&docker(&["info", "--format", ENGINE_INFO_FORMAT])?).ok()?;
    let after_endpoint = selected_endpoint(environment, &docker)?;
    (["ID", "DockerRootDir", "OperatingSystem"]
        .iter()
        .all(|key| after[*key] == info[*key])
        && after_endpoint == endpoint
        && !remaining().is_zero())
    .then_some(available)
}

fn selected_endpoint(
    environment: &BTreeMap<String, String>,
    docker: &impl Fn(&[&str]) -> Option<String>,
) -> Option<String> {
    if environment
        .get("DOCKER_CONTEXT")
        .is_none_or(String::is_empty)
        && let Some(host) = environment
            .get("DOCKER_HOST")
            .filter(|host| !host.is_empty())
    {
        return Some(host.clone());
    }
    docker(&[
        "context",
        "inspect",
        "--format",
        "{{(index .Endpoints \"docker\").Host}}",
    ])
}

pub(crate) fn observe_resources(
    cache: &Path,
    output: &Path,
    environment: Option<&BTreeMap<String, String>>,
    deadline: std::time::Instant,
) -> ResourceObservations {
    let environment = observation_environment(environment);
    let docker = |args: &[&str]| -> Option<String> {
        let bytes = observation_command(
            std::iter::once("docker")
                .chain(args.iter().copied())
                .map(Into::into)
                .collect(),
            &environment,
            observer_bound(deadline),
        )?;
        String::from_utf8(bytes)
            .ok()
            .map(|text| text.trim().to_owned())
    };
    let info: serde_json::Value = docker(&["info", "--format", ENGINE_INFO_FORMAT])
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    let endpoint = selected_endpoint(&environment, &docker);
    let pid = endpoint.as_deref().and_then(local_engine_pid);
    let engine_root = pid.zip(info["DockerRootDir"].as_str()).map(|(pid, root)| {
        PathBuf::from(format!("/proc/{pid}/root")).join(root.trim_start_matches('/'))
    });
    let local_storage = engine_root
        .as_deref()
        .and_then(|root| available_disk_bytes(root, deadline));
    let desktop = local_storage
        .is_none()
        .then(|| desktop_storage(&info, endpoint.as_deref(), &environment, deadline))
        .flatten();
    let engine_storage = ResourceObservation::observed(
        local_storage.or(desktop),
        engine_root.as_ref().map_or_else(
            || {
                if desktop.is_some() {
                    "selected-docker-engine:desktop-namespace"
                } else {
                    "selected-docker-engine"
                }
                .to_owned()
            },
            |path| path.display().to_string(),
        ),
        "selected engine storage namespace is not locally observable",
    );
    let disk = |path: &Path| {
        let root = nearest_existing_ancestor(path);
        ResourceObservation::observed(
            available_disk_bytes(&root, deadline),
            format!("df:{}", root.display()),
            "storage observation unavailable",
        )
    };
    let same_mount = pid.is_some_and(|pid| {
        let current = std::fs::read_link("/proc/self/ns/mnt").ok();
        current.is_some() && current == std::fs::read_link(format!("/proc/{pid}/ns/mnt")).ok()
    });
    let gpu_types = if same_mount && info["Runtimes"].get("nvidia").is_some() {
        observation_command(
            vec![
                "nvidia-smi".into(),
                "--query-gpu=name".into(),
                "--format=csv,noheader".into(),
            ],
            &environment,
            observer_bound(deadline),
        )
        .map(|bytes| {
            String::from_utf8_lossy(&bytes)
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
