//! Exact-root cleanup for native trials and diagnostic witnesses.
//!
//! Only daemon-observed container identities bound to private task roots
//! authorize removal. Cleanup has bounded supervision independent of the
//! settled job, and uncertainty remains explicit in its persisted evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::process::{ExitState, ProcessSupervisor, SpawnSpec};

/// Only daemon-observed identities with an exact private Compose root can
/// authorize removal. Names, trial-name prefixes and task output are not proof.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct OwnedContainer {
    id: String,
    working_dir: PathBuf,
    project: String,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
enum OwnedCleanupError {
    DockerCommandFailed,
    DockerOutputInvalid,
    IdentityMismatch,
    RemovalUnconfirmed,
    EvidenceWriteFailed,
    CaptureOwnershipUnconfirmed,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
enum ContainerRemoval {
    Removed,
    Unconfirmed,
}

#[derive(Debug, serde::Serialize)]
struct ContainerCleanup {
    #[serde(flatten)]
    identity: OwnedContainer,
    status: ContainerRemoval,
}

/// An unconfirmed cleanup must never admit a successful seal.
#[derive(Debug, Default)]
pub(crate) struct OwnedCleanup {
    containers: Vec<ContainerCleanup>,
    capture_helpers: Vec<CaptureHelperCleanup>,
    error: Option<OwnedCleanupError>,
}

#[derive(Debug, serde::Serialize)]
struct CaptureHelperCleanup {
    id: String,
    removed: bool,
}

impl OwnedCleanup {
    pub(crate) fn confirmed(&self) -> bool {
        self.error.is_none()
    }

    pub(crate) fn token(&self) -> String {
        if let Some(error) = self.error {
            return format!(
                "unconfirmed:{}",
                serde_json::to_value(error)
                    .expect("cleanup error serializes")
                    .as_str()
                    .expect("error token")
            );
        }
        if self.containers.is_empty() {
            return "none-found".to_owned();
        }
        format!(
            "containers:{}",
            self.containers
                .iter()
                .map(|entry| format!("{}:removed", entry.identity.id))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

/// Cleanup has its own bounded supervision after the job's cancellation.
/// Only Docker connection settings cross this boundary; output/error text
/// cannot disclose container environment or credentials into receipts.
pub(crate) fn cleanup_docker_spec(
    job_env: &BTreeMap<std::ffi::OsString, std::ffi::OsString>,
    job_cwd: &Path,
) -> SpawnSpec {
    let env = [
        "PATH",
        "HOME",
        "USERPROFILE",
        "SystemRoot",
        "DOCKER_HOST",
        "DOCKER_CONTEXT",
        "DOCKER_CONFIG",
        "DOCKER_TLS_VERIFY",
        "DOCKER_CERT_PATH",
    ]
    .into_iter()
    .filter_map(|key| {
        job_env
            .get(std::ffi::OsStr::new(key))
            .cloned()
            .map(|value| (key.into(), value))
    })
    .collect();
    SpawnSpec {
        argv: vec!["docker".into()],
        cwd: Some(job_cwd.to_path_buf()),
        env,
        stdout_cap: 64 * 1024,
        stderr_cap: 1024,
        timeout: std::time::Duration::from_secs(15),
    }
}

async fn cleanup_docker(spec: &SpawnSpec, args: &[&str]) -> Result<Vec<u8>, OwnedCleanupError> {
    let mut spec = spec.clone();
    spec.argv.extend(args.iter().map(Into::into));
    let outcome = ProcessSupervisor::run(&spec, &CancellationToken::new()).await;
    if outcome.exit != (ExitState::Exited { code: 0 }) || outcome.stdout.truncated {
        return Err(OwnedCleanupError::DockerCommandFailed);
    }
    Ok(outcome.stdout.bytes)
}

fn full_container_id(id: &str) -> bool {
    id.len() == 64
        && id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

async fn owned_container_ids(
    docker: &SpawnSpec,
    directory: &Path,
) -> Result<BTreeSet<String>, OwnedCleanupError> {
    let filter = format!(
        "label=com.docker.compose.project.working_dir={}",
        directory.display()
    );
    let bytes = cleanup_docker(
        docker,
        &["ps", "--all", "--quiet", "--no-trunc", "--filter", &filter],
    )
    .await?;
    let text = std::str::from_utf8(&bytes).map_err(|_| OwnedCleanupError::DockerOutputInvalid)?;
    let ids: BTreeSet<_> = text.lines().map(str::to_owned).collect();
    if ids.len() > 256 || ids.iter().any(|id| !full_container_id(id)) {
        return Err(OwnedCleanupError::DockerOutputInvalid);
    }
    Ok(ids)
}

struct OwnedCaptureMount {
    image: String,
    source: PathBuf,
}

async fn owned_capture_mount(
    docker: &SpawnSpec,
    id: &str,
    trace: &Path,
) -> Result<Option<OwnedCaptureMount>, OwnedCleanupError> {
    #[derive(serde::Deserialize)]
    struct CaptureInspect {
        image: String,
        sources: Vec<PathBuf>,
    }
    let bytes = cleanup_docker(docker, &["inspect", "--type", "container", "--format",
        r#"{"image":{{json .Image}},"sources":[{{range .Mounts}}{{if and (eq .Type "bind") (eq .Destination "/logs/agent") .RW}}{{json .Source}}{{end}}{{end}}]}"#,
        id]).await?;
    let inspected: CaptureInspect =
        serde_json::from_slice(&bytes).map_err(|_| OwnedCleanupError::DockerOutputInvalid)?;
    if inspected.sources.is_empty() {
        return Ok(None);
    }
    if inspected.sources.len() != 1
        || !inspected
            .image
            .strip_prefix("sha256:")
            .is_some_and(full_container_id)
    {
        return Err(OwnedCleanupError::IdentityMismatch);
    }
    let source = &inspected.sources[0];
    let relative = source
        .strip_prefix(trace)
        .map_err(|_| OwnedCleanupError::IdentityMismatch)?;
    let components: Vec<_> = relative.components().collect();
    if components.len() != 4
        || components[0].as_os_str() != "jobs"
        || components[3].as_os_str() != "agent"
        || components
            .iter()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
        || source.to_string_lossy().contains(',')
    {
        return Err(OwnedCleanupError::IdentityMismatch);
    }
    let mut checked = trace.to_path_buf();
    for part in components {
        checked.push(part);
        let metadata = std::fs::symlink_metadata(&checked)
            .map_err(|_| OwnedCleanupError::CaptureOwnershipUnconfirmed)?;
        if metadata.is_symlink() || !metadata.is_dir() {
            return Err(OwnedCleanupError::IdentityMismatch);
        }
    }
    Ok(Some(OwnedCaptureMount {
        image: inspected.image,
        source: source.clone(),
    }))
}

/// The original container is gone before this helper starts. It has no
/// credentials, network, host configuration or writable root filesystem, only
/// the exact daemon-observed Agent log mount. No chmod, symlink traversal or
/// hardlink ownership change is permitted. Its removal gates capture success.
#[cfg(unix)]
async fn restore_capture_owner(
    docker: &SpawnSpec,
    image: &str,
    source: &Path,
    trace: &Path,
    helpers: &mut Vec<CaptureHelperCleanup>,
) -> Result<(), OwnedCleanupError> {
    use std::os::unix::fs::MetadataExt;
    let host =
        std::fs::metadata(trace).map_err(|_| OwnedCleanupError::CaptureOwnershipUnconfirmed)?;
    let owner = format!("{}:{}", host.uid(), host.gid());
    let bind = format!("type=bind,source={},target=/capture", source.display());
    let id = cleanup_docker(docker, &[
        "create", "--pull", "never", "--network", "none", "--read-only", "--cap-drop", "ALL",
        "--cap-add", "CHOWN", "--cap-add", "DAC_OVERRIDE", "--user", "0:0",
        "--security-opt", "no-new-privileges", "--entrypoint", "/bin/sh",
        "--mount", &bind, image, "-c",
        "find /capture -xdev -type d -exec chown -h \"$1\" {} + && find /capture -xdev -type f -links 1 -exec chown -h \"$1\" {} +",
        "opi-eval-capture", &owner,
    ]).await?;
    let id = std::str::from_utf8(&id)
        .map_err(|_| OwnedCleanupError::DockerOutputInvalid)?
        .trim();
    if !full_container_id(id) {
        return Err(OwnedCleanupError::DockerOutputInvalid);
    }
    helpers.push(CaptureHelperCleanup {
        id: id.to_owned(),
        removed: false,
    });
    let restored = cleanup_docker(docker, &["start", "--attach", id]).await;
    let removed = cleanup_docker(docker, &["rm", "--force", "--volumes", id]).await;
    let remaining = cleanup_docker(
        docker,
        &[
            "ps",
            "--all",
            "--quiet",
            "--no-trunc",
            "--filter",
            &format!("id={id}"),
        ],
    )
    .await;
    let gone = removed.is_ok() && remaining.is_ok_and(|bytes| bytes.is_empty());
    helpers
        .last_mut()
        .expect("helper recorded before start")
        .removed = gone;
    if restored.is_err() || !gone {
        return Err(OwnedCleanupError::CaptureOwnershipUnconfirmed);
    }
    Ok(())
}

#[cfg(not(unix))]
async fn restore_capture_owner(
    _: &SpawnSpec,
    _: &str,
    _: &Path,
    _: &Path,
    _: &mut Vec<CaptureHelperCleanup>,
) -> Result<(), OwnedCleanupError> {
    Err(OwnedCleanupError::CaptureOwnershipUnconfirmed)
}

/// Verify every candidate before removing any, using complete immutable IDs.
/// The exact-root query includes stopped containers and every Compose service.
async fn remove_owned_containers(
    docker: &SpawnSpec,
    directories: &[PathBuf],
    capture: Option<&Path>,
) -> OwnedCleanup {
    let mut cleanup = OwnedCleanup::default();
    let result = async {
        for directory in directories {
            for id in owned_container_ids(docker, directory).await? {
                let bytes = cleanup_docker(docker, &["inspect", "--type", "container", "--format",
                    r#"{"id":{{json .Id}},"working_dir":{{json (index .Config.Labels "com.docker.compose.project.working_dir")}},"project":{{json (index .Config.Labels "com.docker.compose.project")}}}"#,
                    &id]).await?;
                let identity: OwnedContainer = serde_json::from_slice(&bytes)
                    .map_err(|_| OwnedCleanupError::DockerOutputInvalid)?;
                if identity.id != id || identity.working_dir != *directory
                    || !identity.project.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
                    || !identity.project.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_') {
                    return Err(OwnedCleanupError::IdentityMismatch);
                }
                cleanup.containers.push(ContainerCleanup { identity, status: ContainerRemoval::Unconfirmed });
            }
        }
        // Capture mounts and immutable images come from the daemon, never Agent
        // output. Retain them before removal so no Agent can race the helper.
        let mut mounts = Vec::new();
        let mut capture_error = None;
        if let Some(trace) = capture {
            for entry in &cleanup.containers {
                match owned_capture_mount(docker, &entry.identity.id, trace).await {
                    Ok(Some(mount)) => mounts.push(mount),
                    Ok(None) => {}
                    Err(error) => capture_error = Some(error),
                }
            }
        }
        let mut removed = true;
        for entry in &mut cleanup.containers {
            if cleanup_docker(docker, &["rm", "--force", &entry.identity.id]).await.is_ok() {
                entry.status = ContainerRemoval::Removed;
            } else {
                removed = false;
            }
        }
        // A successful CLI exit is not proof of removal either.
        for directory in directories {
            let remaining = owned_container_ids(docker, directory).await?;
            for entry in &mut cleanup.containers {
                if remaining.contains(&entry.identity.id) {
                    entry.status = ContainerRemoval::Unconfirmed;
                }
            }
            removed &= remaining.is_empty();
        }
        if !removed { return Err(OwnedCleanupError::RemovalUnconfirmed); }
        if let Some(error) = capture_error { return Err(error); }
        if let Some(trace) = capture {
            for mount in mounts {
                restore_capture_owner(docker, &mount.image, &mount.source, trace, &mut cleanup.capture_helpers).await?;
            }
        }
        Ok(())
    }.await;
    cleanup.error = result.err();
    cleanup
}

pub(crate) async fn cleanup_owned_resources(
    trial: &str,
    directories: &[PathBuf],
    evidence_path: &Path,
    exit: Option<&ExitState>,
    docker: &SpawnSpec,
    capture: Option<&Path>,
) -> OwnedCleanup {
    let mut cleanup = match exit {
        None | Some(ExitState::FailedToSpawn { .. }) => OwnedCleanup::default(),
        _ => remove_owned_containers(docker, directories, capture).await,
    };
    let evidence = json!({
        "schema": "opi-eval-owned-resources/2",
        "trial": trial,
        "compose_working_directories": directories,
        "cleanup": cleanup.token(),
        "containers": cleanup.containers,
        "capture_helpers": cleanup.capture_helpers,
        "error": cleanup.error,
    });
    if std::fs::write(
        evidence_path,
        serde_json::to_vec(&evidence).expect("cleanup serializes"),
    )
    .is_err()
    {
        cleanup.error = Some(OwnedCleanupError::EvidenceWriteFailed);
    }
    cleanup
}
