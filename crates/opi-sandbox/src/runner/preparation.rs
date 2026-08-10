//! Request validation, filesystem preparation, and invocation-owned setup cleanup.

use super::gated::{StartProbe, gated_command};
use super::*;

#[cfg(unix)]
pub(super) struct OwnerDeathCleanup {
    child: Option<std::process::Child>,
    keepalive: Option<std::process::ChildStdin>,
}

#[cfg(unix)]
impl OwnerDeathCleanup {
    fn start(temp_root: &Path) -> io::Result<Self> {
        use std::os::unix::process::CommandExt;

        let mut child = std::process::Command::new("/bin/sh");
        child
            .arg("-c")
            .arg("IFS= read -r _; /bin/rm -rf -- \"$1\"")
            .arg("opi-sandbox-owner-cleanup")
            .arg(temp_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0);
        let mut child = child.spawn()?;
        let keepalive = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("owner cleanup keepalive unavailable"))?;
        Ok(Self {
            child: Some(child),
            keepalive: Some(keepalive),
        })
    }

    pub(super) fn finish(mut self) -> bool {
        drop(self.keepalive.take());
        self.child
            .take()
            .is_some_and(|mut child| child.wait().is_ok_and(|status| status.success()))
    }
}

#[cfg(unix)]
impl Drop for OwnerDeathCleanup {
    fn drop(&mut self) {
        drop(self.keepalive.take());
        if let Some(mut child) = self.child.take() {
            let _ = child.wait();
        }
    }
}

#[cfg(unix)]
fn start_owner_death_cleanup(temp_root: &Path) -> io::Result<Option<OwnerDeathCleanup>> {
    OwnerDeathCleanup::start(temp_root).map(Some)
}

#[cfg(not(unix))]
pub(super) struct OwnerDeathCleanup;

#[cfg(not(unix))]
impl OwnerDeathCleanup {
    pub(super) fn finish(self) -> bool {
        true
    }
}

#[cfg(not(unix))]
fn start_owner_death_cleanup(_temp_root: &Path) -> io::Result<Option<OwnerDeathCleanup>> {
    Ok(None)
}

impl PreparedTemp {
    pub(super) fn into_temp(mut self) -> tempfile::TempDir {
        self.temp.take().expect("prepared temp present")
    }

    fn close(mut self) -> bool {
        let temp = self.temp.take().expect("prepared temp present");
        if !self.remove_delay.is_zero() {
            std::thread::sleep(self.remove_delay);
        }
        temp.close().is_ok() && !self.injected_failure
    }
}

impl Drop for PreparedTemp {
    fn drop(&mut self) {
        let Some(temp) = self.temp.take() else {
            return;
        };
        if !self.remove_delay.is_zero() {
            std::thread::sleep(self.remove_delay);
        }
        let _ = temp.close();
    }
}

impl PreparedSandboxRun {
    pub(super) fn setup_expired(&self, deadline_plan: RunDeadlinePlan) -> bool {
        deadline_plan.setup_expired(&self.cancel)
    }

    pub(super) fn cleanup(self) -> bool {
        let Self {
            temp,
            owner_death_cleanup,
            ..
        } = self;
        let removed = temp.close();
        let cleanup_owner_finished = owner_death_cleanup.is_none_or(OwnerDeathCleanup::finish);
        removed && cleanup_owner_finished
    }
}

pub(crate) async fn cleanup_prepared_until(
    prepared: PreparedSandboxRun,
    deadline: Instant,
) -> bool {
    let cleanup = tokio::task::spawn_blocking(move || prepared.cleanup());
    matches!(
        tokio::time::timeout_at(deadline, cleanup).await,
        Ok(Ok(true))
    )
}

impl ValidatedSandboxRequest {
    pub(crate) fn setup_cancel_token(&self) -> CancellationToken {
        self.request.cancel.clone().unwrap_or_default()
    }
}

impl SandboxRunner {
    pub(crate) fn validate_request_shape(
        &self,
        request: SandboxRequest,
    ) -> Result<StructurallyValidatedRequest, SetupFailed> {
        if request.timeout.is_zero()
            || request.program.as_os_str().is_empty()
            || request.workspace.as_os_str().is_empty()
            || request.cwd.as_os_str().is_empty()
        {
            return Err(SetupFailed {
                reason: SetupFailureReason::InvalidRequest,
            });
        }
        if request
            .env_additions
            .keys()
            .any(|key| invalid_environment_key(key))
        {
            return Err(invalid_request());
        }
        #[cfg(windows)]
        if request.env_additions.keys().any(|key| {
            key.to_string_lossy()
                .to_ascii_uppercase()
                .starts_with("OPI_SANDBOX_")
        }) {
            return Err(SetupFailed {
                reason: SetupFailureReason::InvalidRequest,
            });
        }
        Ok(StructurallyValidatedRequest { request })
    }

    pub(crate) fn validate_request_filesystem(
        &self,
        validated: StructurallyValidatedRequest,
    ) -> Result<ValidatedSandboxRequest, SetupFailed> {
        let request = validated.request;
        if !self.faults.validation_delay.is_zero() {
            std::thread::sleep(self.faults.validation_delay);
        }
        let workspace = request
            .workspace
            .canonicalize()
            .map_err(|_| invalid_request())?;
        let cwd = request.cwd.canonicalize().map_err(|_| invalid_request())?;
        if !workspace.is_dir() || !cwd.is_dir() || !cwd.starts_with(&workspace) {
            return Err(SetupFailed {
                reason: SetupFailureReason::InvalidRequest,
            });
        }

        Ok(ValidatedSandboxRequest {
            request,
            workspace,
            cwd,
        })
    }

    /// Validate every request invariant that can be checked without creating
    /// an invocation root, installing restrictions, or spawning a process.
    pub(crate) fn validate_request(
        &self,
        request: SandboxRequest,
    ) -> Result<ValidatedSandboxRequest, SetupFailed> {
        let request = self.validate_request_shape(request)?;
        self.validate_request_filesystem(request)
    }

    /// Perform every blocking preparation step without spawning a process.
    /// This is the only runner operation allowed on a background setup worker.
    pub(crate) fn prepare_validated_until(
        &self,
        validated: ValidatedSandboxRequest,
        setup_deadline: Option<Instant>,
    ) -> Result<PreparedSandboxRun, SetupFailed> {
        let ValidatedSandboxRequest {
            request,
            workspace,
            cwd,
        } = validated;
        let setup_cancel = request.cancel.clone().unwrap_or_default();
        if setup_stopped(setup_deadline, &setup_cancel) {
            return Err(deadline_setup_failure());
        }
        let program = resolve_program(
            &request.program,
            &cwd,
            request.env_inherit,
            &request.env_additions,
        )
        .ok_or(SetupFailed {
            reason: SetupFailureReason::ProgramNotFound,
        })?;
        if setup_stopped(setup_deadline, &setup_cancel) {
            return Err(deadline_setup_failure());
        }
        // Create the invocation-owned temp root. Owned by `run` until it is moved
        // into the supervision future; on any error path below it drops and the
        // dir is removed.
        let temp = tempfile::TempDir::new().map_err(|_| SetupFailed {
            reason: SetupFailureReason::SpawnFailed,
        })?;
        let temp_root = temp.path().canonicalize().map_err(|_| SetupFailed {
            reason: SetupFailureReason::SpawnFailed,
        })?;
        let owner_death_cleanup =
            start_owner_death_cleanup(&temp_root).map_err(|_| SetupFailed {
                reason: SetupFailureReason::SpawnFailed,
            })?;
        let release_gate = temp_root.join("release.armed");
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&release_gate)
            .map_err(|_| SetupFailed {
                reason: SetupFailureReason::SpawnFailed,
            })?;

        let ctx = RestrictionCtx {
            workspace: &workspace,
            temp_root: &temp_root,
            network: self.policy.network,
            setup_deadline,
            setup_cancel: &setup_cancel,
        };

        let launcher = self.restriction.launcher(&ctx).map_err(|_| SetupFailed {
            reason: SetupFailureReason::RestrictionSetup,
        })?;
        if ctx.setup_cancelled() {
            return Err(deadline_setup_failure());
        }

        // Ask the restriction whether the target should be wrapped in a parent
        // program (macOS Seatbelt: `sandbox-exec -p <profile>`). The runner
        // builds the command AROUND the launcher so cwd/stdio/env/process-tree
        // config is then applied IDENTICALLY to the bare-program path.
        // std::process::Command exposes no stdio/kill_on_drop/env_clear getters
        // and no reprogram API, so a launcher cannot be installed later inside
        // `prepare` (a rebuild would drop the runner's piped stdio and env
        // policy) — the launcher spec must be computed before the command is
        // built. `NoRestriction`/Linux return `None` (default) and take the
        // bare path with a `prepare`-driven `pre_exec` hook unchanged.
        let launcher_present = launcher.is_some();
        let start_probe = launcher_present
            .then(|| StartProbe::new(&release_gate))
            .transpose()
            .map_err(|_| SetupFailed {
                reason: SetupFailureReason::RestrictionSetup,
            })?;
        let mut cmd = gated_command(
            &program,
            &request.args,
            &request.env_additions,
            &release_gate,
            start_probe.as_ref().map(|probe| probe.token.as_slice()),
            launcher,
        );
        cmd.current_dir(&cwd)
            .stdin(match request.stdin {
                StdinPolicy::Null => Stdio::null(),
                StdinPolicy::Inherit => Stdio::inherit(),
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        apply_env(
            &mut cmd,
            request.env_inherit,
            &request.env_additions,
            &temp_root,
        );
        #[cfg(windows)]
        apply_windows_bootstrap_env(&mut cmd, &release_gate, &program, &request.args);

        if ctx.setup_cancelled() {
            return Err(deadline_setup_failure());
        }
        let applied = self
            .restriction
            .prepare(&mut cmd, &ctx)
            .map_err(|_| SetupFailed {
                reason: SetupFailureReason::RestrictionSetup,
            })?;
        let contract_is_consistent = match (applied.mechanism, applied.contract) {
            (Mechanism::None, ContractStatus::Unrestricted) => !launcher_present,
            (Mechanism::Landlock | Mechanism::Seccomp, ContractStatus::Restricted) => {
                !launcher_present
            }
            (Mechanism::Seatbelt, ContractStatus::Restricted) => launcher_present,
            _ => false,
        };
        if !contract_is_consistent {
            return Err(SetupFailed {
                reason: SetupFailureReason::RestrictionSetup,
            });
        }

        // Restriction setup is cooperative and may be implemented by external
        // platform code. It must consume this run's original absolute budget.
        if ctx.setup_cancelled() {
            return Err(deadline_setup_failure());
        }

        configure_tree(&mut cmd);

        // A background worker may outlive its caller, so its final operation is
        // this deadline check and return of opaque prepared state. It never owns
        // or invokes the actual spawn operation.
        if ctx.setup_cancelled() {
            return Err(deadline_setup_failure());
        }

        let prepared = PreparedSandboxRun {
            cmd,
            temp: PreparedTemp {
                temp: Some(temp),
                remove_delay: self.faults.prepared_temp_remove_delay,
                injected_failure: self.faults.prepared_temp_remove_fail,
            },
            owner_death_cleanup,
            temp_root,
            release_gate,
            start_probe,
            cancel: request.cancel.unwrap_or_default(),
            mechanism: applied.mechanism,
            contract: applied.contract,
            faults: self.faults,
        };
        if !self.faults.prepared_delivery_delay.is_zero() {
            std::thread::sleep(self.faults.prepared_delivery_delay);
        }
        Ok(prepared)
    }
}

fn setup_stopped(deadline: Option<Instant>, cancel: &CancellationToken) -> bool {
    cancel.is_cancelled() || deadline.is_some_and(|deadline| Instant::now() >= deadline)
}

pub(super) fn invalid_request() -> SetupFailed {
    SetupFailed {
        reason: SetupFailureReason::InvalidRequest,
    }
}

pub(super) fn deadline_setup_failure() -> SetupFailed {
    SetupFailed {
        reason: SetupFailureReason::SpawnFailed,
    }
}

#[cfg(unix)]
fn invalid_environment_key(key: &std::ffi::OsStr) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let bytes = key.as_bytes();
    bytes.is_empty() || bytes.contains(&b'=') || bytes.contains(&0)
}

#[cfg(windows)]
fn invalid_environment_key(key: &std::ffi::OsStr) -> bool {
    use std::os::windows::ffi::OsStrExt;
    let mut units = key.encode_wide();
    let Some(first) = units.next() else {
        return true;
    };
    first == b'=' as u16 || first == 0 || units.any(|unit| unit == b'=' as u16 || unit == 0)
}

#[cfg(not(any(unix, windows)))]
fn invalid_environment_key(key: &std::ffi::OsStr) -> bool {
    let key = key.to_string_lossy();
    key.is_empty() || key.contains('=') || key.contains('\0')
}

fn apply_env(
    cmd: &mut Command,
    inherit: EnvInherit,
    additions: &BTreeMap<OsString, OsString>,
    temp_root: &Path,
) {
    if matches!(inherit, EnvInherit::Clear) {
        cmd.env_clear();
    }
    for (key, value) in additions {
        cmd.env(key, value);
    }
    cmd.env("TMPDIR", temp_root)
        .env("TMP", temp_root)
        .env("TEMP", temp_root);
}

#[cfg(windows)]
fn apply_windows_bootstrap_env(
    cmd: &mut Command,
    release_gate: &Path,
    program: &Path,
    args: &[OsString],
) {
    cmd.env("OPI_SANDBOX_RELEASE_GATE", release_gate)
        .env("OPI_SANDBOX_TARGET_PROGRAM", program)
        .env("OPI_SANDBOX_TARGET_ARG_COUNT", args.len().to_string())
        .env("OPI_SANDBOX_BACKEND_PID", std::process::id().to_string());
    for (index, argument) in args.iter().enumerate() {
        cmd.env(format!("OPI_SANDBOX_TARGET_ARG_{index}"), argument);
    }
}

pub(super) fn resolve_program(
    program: &Path,
    cwd: &Path,
    inherit: EnvInherit,
    additions: &BTreeMap<OsString, OsString>,
) -> Option<PathBuf> {
    let has_path = program.is_absolute() || program.components().count() > 1;
    if has_path {
        let candidate = if program.is_absolute() {
            program.to_path_buf()
        } else {
            cwd.join(program)
        };
        return candidate.is_file().then_some(candidate);
    }

    #[cfg(windows)]
    {
        let direct = cwd.join(program);
        if direct.is_file() {
            return Some(direct);
        }
        let mut executable = program.as_os_str().to_os_string();
        executable.push(".exe");
        let direct_executable = cwd.join(executable);
        if direct_executable.is_file() {
            return Some(direct_executable);
        }
        let path = additions
            .iter()
            .find(|(key, _)| key.to_string_lossy().eq_ignore_ascii_case("PATH"))
            .map(|(_, value)| value.clone())
            .or_else(|| {
                matches!(inherit, EnvInherit::Inherit)
                    .then(|| std::env::var_os("PATH"))
                    .flatten()
            })?;
        std::env::split_paths(&path).find_map(|directory| {
            let base = if directory.as_os_str().is_empty() {
                cwd.to_path_buf()
            } else if directory.is_absolute() {
                directory
            } else {
                cwd.join(directory)
            };
            let candidate = base.join(program);
            if candidate.is_file() {
                return Some(candidate);
            }
            if program.extension().is_none() {
                let mut executable = program.as_os_str().to_os_string();
                executable.push(".exe");
                let candidate = base.join(executable);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
            None
        })
    }

    #[cfg(unix)]
    {
        let path = additions
            .get(std::ffi::OsStr::new("PATH"))
            .cloned()
            .or_else(|| {
                matches!(inherit, EnvInherit::Inherit)
                    .then(|| std::env::var_os("PATH"))
                    .flatten()
            })
            .unwrap_or_else(|| OsString::from("/usr/bin:/bin"));
        std::env::split_paths(&path).find_map(|directory| {
            let base = if directory.as_os_str().is_empty() {
                cwd.to_path_buf()
            } else if directory.is_absolute() {
                directory
            } else {
                cwd.join(directory)
            };
            let candidate = base.join(program);
            candidate.is_file().then_some(candidate)
        })
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = (cwd, inherit, additions);
        Some(program.to_path_buf())
    }
}
