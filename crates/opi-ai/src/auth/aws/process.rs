//! `credential_process` execution seam for the local AWS source chain.
//!
//! A shared-config profile's `credential_process` line becomes one frozen
//! [`CredentialProcessSpec`](crate::auth::aws::process::CredentialProcessSpec) (command, cwd, child environment) executed through
//! the injectable [`CredentialProcessRunner`](crate::auth::aws::process::CredentialProcessRunner). The default
//! [`SpawnedCredentialProcess`](crate::auth::aws::process::SpawnedCredentialProcess) runner owns the real spawn: bounded stdout, a
//! fixed attempt window, private process-tree cleanup, and cancellation that
//! reaps every owned process. The existing 10s/64 KiB budgets are declared
//! opi policy (Pi's adapter uses its provider-process seam without explicit
//! caps); no new process, stdio, or total-send caps are introduced here.
//!
//! Per the verified Node chain corpus, every `credential_process` failure
//! (missing/blank command, spawn, exit, output, JSON, version, incomplete
//! keys, unparseable expiry) is a continue-eligible discovery failure: the
//! chain advances instead of failing the whole resolution. The typed
//! [`CredentialProcessError`](crate::auth::aws::process::CredentialProcessError) values stay secret-free.
//!
//! # Unstable
//!
//! Part of the **unstable 0.x extension substrate**. Breaking changes may
//! occur between minor versions without a major version bump.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use secrecy::SecretString;
use time::OffsetDateTime;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::time::{Instant, sleep_until};
use tokio_util::sync::CancellationToken;

use crate::credential::BoxAuthFuture;

/// Child environment for one frozen credential-process invocation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ProcessEnv {
    /// Inherit the ambient process environment (the Pi adapter behavior).
    #[default]
    Inherit,
    /// Run the child with exactly this environment and nothing inherited.
    /// The map is frozen at selection time; the runner performs no ambient
    /// environment reads of its own.
    Exact(BTreeMap<String, String>),
}

/// One frozen `credential_process` invocation selected from a shared-config
/// profile.
///
/// The command line can embed credential material, so [`std::fmt::Debug`]
/// redacts it together with the environment.
#[derive(Clone, PartialEq, Eq)]
pub struct CredentialProcessSpec {
    /// Shell command line exactly as selected from the profile.
    pub command: String,
    /// Frozen working directory for the child, when one was selected.
    pub cwd: Option<PathBuf>,
    /// Frozen child environment.
    pub env: ProcessEnv,
}

impl std::fmt::Debug for CredentialProcessSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialProcessSpec")
            .field("command", &"<redacted>")
            .field("cwd", &self.cwd)
            .field("env", &"<redacted>")
            .finish()
    }
}

impl CredentialProcessSpec {
    /// Build a spec for `command` that inherits the ambient environment and
    /// working directory.
    pub fn inherit(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            cwd: None,
            env: ProcessEnv::Inherit,
        }
    }
}

/// Parsed `credential_process` stdout following the AWS credential-process
/// JSON contract (`Version`, `AccessKeyId`, `SecretAccessKey`, optional
/// `SessionToken` and RFC 3339 `Expiration`).
///
/// Every credential field uses [`SecretString`]; [`std::fmt::Debug`] redacts
/// them. `expiration` records the source-reported expiry without extending it.
#[derive(Clone)]
pub struct CredentialProcessOutput {
    /// Actual `Version` field value; only `1` is accepted.
    pub version: u32,
    /// Access key identifier. Redacted in diagnostics.
    pub access_key_id: SecretString,
    /// Secret access key. Redacted in diagnostics.
    pub secret_access_key: SecretString,
    /// Optional session token. Redacted in diagnostics.
    pub session_token: Option<SecretString>,
    /// Source-reported RFC 3339 expiry, when the process supplied one.
    pub expiration: Option<OffsetDateTime>,
}

impl std::fmt::Debug for CredentialProcessOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialProcessOutput")
            .field("version", &self.version)
            .field("access_key_id", &"<redacted>")
            .field("secret_access_key", &"<redacted>")
            .field(
                "session_token",
                &self.session_token.as_ref().map(|_| "<redacted>"),
            )
            .field("expiration", &self.expiration)
            .finish()
    }
}

/// Typed, secret-free `credential_process` failures.
///
/// Every variant is continue-eligible in the source chain: these are
/// discovery failures of one source family, never a reason to borrow another
/// family's credentials or to hide the failure as success.
#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum CredentialProcessError {
    #[error("credential_process could not be started")]
    Spawn,
    #[error("credential_process exited unsuccessfully")]
    Exit,
    #[error("credential_process timed out")]
    Timeout,
    #[error("credential_process output exceeded the limit")]
    OutputLimit,
    #[error("credential_process output could not be read")]
    Output,
    #[error("credential_process output was not valid JSON")]
    Json,
    #[error("credential_process returned an unsupported version")]
    Version,
    #[error("credential_process returned incomplete credentials")]
    Incomplete,
    #[error("credential_process returned an unparseable expiration")]
    Expiration,
    #[error("credential_process resolution was cancelled")]
    Cancelled,
    #[error("credential_process resolution task failed")]
    Join,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawCredentialProcessOutput {
    version: u32,
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
    expiration: Option<String>,
}

/// Parse and validate raw `credential_process` stdout.
///
/// Validation covers the actual `Version` field (only `1`), complete
/// non-blank key material, and a well-formed RFC 3339 `Expiration` when
/// present. No secret ever enters an error value.
pub fn parse_credential_process_output(
    bytes: &[u8],
) -> Result<CredentialProcessOutput, CredentialProcessError> {
    let raw: RawCredentialProcessOutput =
        serde_json::from_slice(bytes).map_err(|_| CredentialProcessError::Json)?;
    if raw.version != 1 {
        return Err(CredentialProcessError::Version);
    }
    if raw.access_key_id.trim().is_empty() || raw.secret_access_key.trim().is_empty() {
        return Err(CredentialProcessError::Incomplete);
    }
    let expiration = match raw.expiration {
        Some(value) if value.trim().is_empty() => None,
        Some(value) => Some(
            OffsetDateTime::parse(&value, &time::format_description::well_known::Rfc3339)
                .map_err(|_| CredentialProcessError::Expiration)?,
        ),
        None => None,
    };
    Ok(CredentialProcessOutput {
        version: raw.version,
        access_key_id: SecretString::from(raw.access_key_id),
        secret_access_key: SecretString::from(raw.secret_access_key),
        session_token: raw
            .session_token
            .filter(|token| !token.trim().is_empty())
            .map(SecretString::from),
        expiration,
    })
}

/// Injectable `credential_process` execution seam.
///
/// The chain calls the runner at most once per profile per discovery round.
/// Implementations must make the returned future cancellation-safe: dropping
/// it must terminate and reap every owned process.
pub trait CredentialProcessRunner: Send + Sync {
    /// Execute one frozen credential-process invocation.
    fn run<'a>(
        &'a self,
        spec: &'a CredentialProcessSpec,
    ) -> BoxAuthFuture<'a, Result<CredentialProcessOutput, CredentialProcessError>>;
}

/// Local credential processes are given ten seconds and at most 64 KiB of
/// stdout. The budget must absorb cold shell startup on loaded hosts (a
/// Windows PowerShell cold start alone can exceed three seconds), so the
/// process itself is only expected to finish well inside the window. These
/// are declared opi policy, not a Pi parity default. Cancellation kills and
/// reaps the spawned shell before its detached cleanup task exits.
const CREDENTIAL_PROCESS_TIMEOUT: Duration = Duration::from_secs(10);
const CREDENTIAL_PROCESS_MAX_STDOUT: usize = 64 * 1024;
const PROCESS_TREE_TERMINATION_TIMEOUT: Duration = Duration::from_secs(2);

/// Default [`CredentialProcessRunner`] that really spawns the frozen command.
pub struct SpawnedCredentialProcess;

struct CancelProcessOnDrop {
    cancellation: CancellationToken,
    armed: bool,
}

impl Drop for CancelProcessOnDrop {
    fn drop(&mut self) {
        if self.armed {
            self.cancellation.cancel();
        }
    }
}

impl CredentialProcessRunner for SpawnedCredentialProcess {
    fn run<'a>(
        &'a self,
        spec: &'a CredentialProcessSpec,
    ) -> BoxAuthFuture<'a, Result<CredentialProcessOutput, CredentialProcessError>> {
        Box::pin(async move { run_spawned_credential_process(spec).await })
    }
}

async fn run_spawned_credential_process(
    spec: &CredentialProcessSpec,
) -> Result<CredentialProcessOutput, CredentialProcessError> {
    let cancellation = CancellationToken::new();
    let mut cancel_on_drop = CancelProcessOnDrop {
        cancellation: cancellation.clone(),
        armed: true,
    };
    let spec = spec.clone();
    let task =
        tokio::spawn(
            async move { run_spawned_credential_process_inner(&spec, cancellation).await },
        );
    let result = task.await.map_err(|_| CredentialProcessError::Join)?;
    cancel_on_drop.armed = false;
    result
}

async fn run_spawned_credential_process_inner(
    spec: &CredentialProcessSpec,
    cancellation: CancellationToken,
) -> Result<CredentialProcessOutput, CredentialProcessError> {
    let mut command_builder = if cfg!(windows) {
        let mut builder = Command::new("powershell");
        builder.args(["-NoProfile", "-Command", &spec.command]);
        builder
    } else {
        let mut builder = Command::new("sh");
        builder.arg("-c").arg(&spec.command);
        builder
    };
    if let Some(cwd) = &spec.cwd {
        command_builder.current_dir(cwd);
    }
    if let ProcessEnv::Exact(env) = &spec.env {
        command_builder.env_clear().envs(env);
    }
    #[cfg(unix)]
    command_builder.process_group(0);
    let mut child = command_builder
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| CredentialProcessError::Spawn)?;
    let root_pid = child.id();

    let output = read_bounded_process_output(&mut child, root_pid, cancellation).await?;
    let parsed = parse_credential_process_output(&output);
    if parsed.is_err() {
        terminate_tree_and_reap(&mut child, root_pid).await;
    }
    parsed
}

async fn read_bounded_process_output(
    child: &mut Child,
    root_pid: Option<u32>,
    cancellation: CancellationToken,
) -> Result<Vec<u8>, CredentialProcessError> {
    let Some(mut stdout) = child.stdout.take() else {
        terminate_tree_and_reap(child, root_pid).await;
        return Err(CredentialProcessError::Output);
    };
    let deadline = Instant::now() + CREDENTIAL_PROCESS_TIMEOUT;
    let mut output = Vec::new();
    let mut limited_stdout = (&mut stdout).take((CREDENTIAL_PROCESS_MAX_STDOUT + 1) as u64);
    let read_result = tokio::select! {
        _ = cancellation.cancelled() => {
            terminate_tree_and_reap(child, root_pid).await;
            return Err(CredentialProcessError::Cancelled);
        }
        _ = sleep_until(deadline) => {
            terminate_tree_and_reap(child, root_pid).await;
            return Err(CredentialProcessError::Timeout);
        }
        result = limited_stdout.read_to_end(&mut output) => result,
    };
    if read_result.is_err() {
        terminate_tree_and_reap(child, root_pid).await;
        return Err(CredentialProcessError::Output);
    }
    if output.len() > CREDENTIAL_PROCESS_MAX_STDOUT {
        terminate_tree_and_reap(child, root_pid).await;
        return Err(CredentialProcessError::OutputLimit);
    }
    let status = tokio::select! {
        _ = cancellation.cancelled() => {
            terminate_tree_and_reap(child, root_pid).await;
            return Err(CredentialProcessError::Cancelled);
        }
        _ = sleep_until(deadline) => {
            terminate_tree_and_reap(child, root_pid).await;
            return Err(CredentialProcessError::Timeout);
        }
        result = child.wait() => result,
    };
    let status = match status {
        Ok(status) => status,
        Err(_) => {
            terminate_tree_and_reap(child, root_pid).await;
            return Err(CredentialProcessError::Output);
        }
    };
    if !status.success() {
        terminate_tree_and_reap(child, root_pid).await;
        return Err(CredentialProcessError::Exit);
    }
    Ok(output)
}

async fn terminate_tree_and_reap(child: &mut Child, root_pid: Option<u32>) {
    if let Some(root_pid) = root_pid {
        terminate_process_tree(root_pid).await;
    }
    let _ = child.start_kill();
    let _ = child.wait().await;
}

async fn terminate_process_tree(root_pid: u32) {
    #[cfg(unix)]
    let mut command = {
        let mut command = Command::new("kill");
        // The `--` separator is required: procps-ng `kill` parses a bare
        // `-<pgid>` argument as an option and exits successfully without
        // signalling anyone, silently leaving the process group alive.
        command.arg("-KILL").arg("--").arg(format!("-{root_pid}"));
        command
    };
    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("taskkill");
        command.args(["/PID", &root_pid.to_string(), "/T", "/F"]);
        command
    };
    #[cfg(not(any(unix, windows)))]
    return;

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let Ok(mut terminator) = command.spawn() else {
        return;
    };
    if tokio::time::timeout(PROCESS_TREE_TERMINATION_TIMEOUT, terminator.wait())
        .await
        .is_err()
    {
        let _ = terminator.start_kill();
        let _ = terminator.wait().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    static CREDENTIAL_PROCESS_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    #[test]
    fn parsed_output_debug_redacts_credential_values() {
        let output = CredentialProcessOutput {
            version: 1,
            access_key_id: SecretString::from("AKIA_PROCESS_DEBUG"),
            secret_access_key: SecretString::from("process-secret-debug"),
            session_token: Some(SecretString::from("process-session-debug")),
            expiration: None,
        };
        let debug = format!("{output:?}");
        for canary in [
            "AKIA_PROCESS_DEBUG",
            "process-secret-debug",
            "process-session-debug",
        ] {
            assert!(!debug.contains(canary), "leak in debug: {debug}");
        }
    }

    #[test]
    fn output_spec_debug_redacts_command_and_environment() {
        let spec = CredentialProcessSpec {
            command: "cat /secret/fixtures/creds.json".to_owned(),
            cwd: None,
            env: ProcessEnv::Exact(BTreeMap::from([(
                "AWS_ACCESS_KEY_ID".to_owned(),
                "AKIA_SPEC_DEBUG".to_owned(),
            )])),
        };
        let debug = format!("{spec:?}");
        assert!(!debug.contains("creds.json"));
        assert!(!debug.contains("AKIA_SPEC_DEBUG"));
    }

    #[test]
    fn parse_accepts_the_full_contract_with_rfc3339_expiry() {
        let parsed = parse_credential_process_output(
            br#"{"Version":1,"AccessKeyId":"PROC_AKID","SecretAccessKey":"PROC_SAK","SessionToken":"PROC_TOKEN","Expiration":"2100-01-01T00:00:00Z"}"#,
        )
        .expect("full contract parses");
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.access_key_id.expose_secret(), "PROC_AKID");
        assert_eq!(
            parsed.expiration,
            Some(
                OffsetDateTime::parse(
                    "2100-01-01T00:00:00Z",
                    &time::format_description::well_known::Rfc3339
                )
                .expect("fixture expiry")
            )
        );
    }

    #[test]
    fn parse_validates_version_keys_and_expiry() {
        let cases: &[(&[u8], CredentialProcessError)] = &[
            (
                br#"{"Version":2,"AccessKeyId":"AKIA_VERSION_CANARY","SecretAccessKey":"version-secret-canary"}"#,
                CredentialProcessError::Version,
            ),
            (
                br#"{"Version":"1","AccessKeyId":"AKIA_STRING_VERSION","SecretAccessKey":"secret"}"#,
                CredentialProcessError::Json,
            ),
            (
                br#"{"Version":1,"AccessKeyId":"","SecretAccessKey":"secret"}"#,
                CredentialProcessError::Incomplete,
            ),
            (
                br#"{"Version":1,"AccessKeyId":"akid","SecretAccessKey":" \t"}"#,
                CredentialProcessError::Incomplete,
            ),
            (
                br#"{"Version":1,"AccessKeyId":"akid","SecretAccessKey":"secret","Expiration":"not-a-timestamp"}"#,
                CredentialProcessError::Expiration,
            ),
            (b"not-json", CredentialProcessError::Json),
        ];
        for (bytes, expected) in cases {
            assert_eq!(
                parse_credential_process_output(bytes).unwrap_err(),
                *expected
            );
            let rendered = format!("{expected:?}");
            assert!(!rendered.contains("AKIA_"));
            assert!(!rendered.contains("canary"));
        }
    }

    #[tokio::test]
    async fn spawned_runner_executes_a_real_credential_process() {
        let _process_lock = CREDENTIAL_PROCESS_TEST_LOCK.lock().await;
        let output = r#"{"Version":1,"AccessKeyId":"PROC_AKID","SecretAccessKey":"PROC_SAK","SessionToken":"PROC_TOKEN"}"#;
        let command = if cfg!(windows) {
            format!("[Console]::Out.Write('{output}')")
        } else {
            format!("printf '{output}'")
        };
        let spec = CredentialProcessSpec::inherit(command);
        let parsed = SpawnedCredentialProcess
            .run(&spec)
            .await
            .expect("credential process runs");
        assert_eq!(parsed.access_key_id.expose_secret(), "PROC_AKID");
        assert_eq!(
            parsed
                .session_token
                .as_ref()
                .map(|token| token.expose_secret()),
            Some("PROC_TOKEN")
        );
    }

    #[tokio::test]
    async fn spawned_runner_runs_in_the_frozen_cwd_and_exact_environment() {
        let _process_lock = CREDENTIAL_PROCESS_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("marker.txt");
        std::fs::write(
            &marker,
            br#"{"Version":1,"AccessKeyId":"PROC_AKID","SecretAccessKey":"PROC_SAK"}"#,
        )
        .unwrap();
        let command = if cfg!(windows) {
            "Get-Content -Raw -LiteralPath marker.txt".to_owned()
        } else {
            "cat marker.txt".to_owned()
        };
        // The exact environment proves the child saw no ambient variables and
        // the frozen cwd (relative marker path resolves only there).
        let path_var = if cfg!(windows) { "Path" } else { "PATH" };
        let inherited_path = std::env::var(path_var).unwrap_or_default();
        let spec = CredentialProcessSpec {
            command,
            cwd: Some(dir.path().to_path_buf()),
            env: ProcessEnv::Exact(BTreeMap::from([(path_var.to_owned(), inherited_path)])),
        };
        let parsed = SpawnedCredentialProcess
            .run(&spec)
            .await
            .expect("frozen cwd/env run");
        assert_eq!(parsed.access_key_id.expose_secret(), "PROC_AKID");
    }

    #[tokio::test]
    async fn spawned_runner_failures_are_typed_and_bounded() {
        let _process_lock = CREDENTIAL_PROCESS_TEST_LOCK.lock().await;
        let exit = CredentialProcessSpec::inherit("exit 9");
        assert_eq!(
            SpawnedCredentialProcess.run(&exit).await.unwrap_err(),
            CredentialProcessError::Exit
        );

        let timeout_command = if cfg!(windows) {
            "Start-Sleep -Seconds 30"
        } else {
            "sleep 30"
        };
        let timeout = CredentialProcessSpec::inherit(timeout_command);
        assert_eq!(
            SpawnedCredentialProcess.run(&timeout).await.unwrap_err(),
            CredentialProcessError::Timeout
        );

        let output_command = if cfg!(windows) {
            "[Console]::Out.Write('x' * 70000)".to_owned()
        } else {
            "head -c 70000 /dev/zero".to_owned()
        };
        let limit = CredentialProcessSpec::inherit(output_command);
        assert_eq!(
            SpawnedCredentialProcess.run(&limit).await.unwrap_err(),
            CredentialProcessError::OutputLimit
        );
    }

    #[tokio::test]
    async fn spawned_runner_real_process_failures_stay_secret_free() {
        let _process_lock = CREDENTIAL_PROCESS_TEST_LOCK.lock().await;
        let command = if cfg!(windows) {
            r#"[Console]::Out.Write('{"Version":2,"AccessKeyId":"AKIA_VERSION_CANARY","SecretAccessKey":"version-secret-canary"}')"#
        } else {
            r#"printf '{"Version":2,"AccessKeyId":"AKIA_VERSION_CANARY","SecretAccessKey":"version-secret-canary"}'"#
        };
        let error = SpawnedCredentialProcess
            .run(&CredentialProcessSpec::inherit(command))
            .await
            .unwrap_err();
        let rendered = format!("{error:?} {error}");
        assert!(!rendered.contains("AKIA_VERSION_CANARY"));
        assert!(!rendered.contains("version-secret-canary"));
    }

    #[tokio::test]
    async fn cancelling_real_credential_process_resolution_terminates_descendants() {
        let _process_lock = CREDENTIAL_PROCESS_TEST_LOCK.lock().await;
        // Cold shell startup on a loaded CI runner can exceed both this
        // test's marker wait and the runner's own timeout. Only the startup
        // race is retryable: a marker that never appears because the runner
        // gave up first is a slow-host artifact, while a surviving descendant
        // after a launched run is a real defect and fails immediately.
        for attempt in 0..3 {
            if let TestAttempt::Passed = run_cancellation_attempt().await {
                return;
            }
            if attempt == 2 {
                panic!("credential process did not start within three attempts");
            }
        }
    }

    enum TestAttempt {
        Passed,
        RetryStartup,
    }

    async fn run_cancellation_attempt() -> TestAttempt {
        let dir = tempfile::tempdir().unwrap();
        let started = dir.path().join("started");
        let survived = dir.path().join("survived");
        let descendant_script = dir.path().join(if cfg!(windows) {
            "descendant.ps1"
        } else {
            "descendant.sh"
        });
        let command = if cfg!(windows) {
            std::fs::write(
                &descendant_script,
                "param([string]$Started, [string]$Survived)\nSet-Content -LiteralPath $Started -Value started\nStart-Sleep -Milliseconds 1200\nSet-Content -LiteralPath $Survived -Value survived\n",
            )
            .unwrap();
            format!(
                "$child = Start-Process -FilePath 'powershell' -WindowStyle Hidden -PassThru -ArgumentList @('-NoProfile','-File','{}','{}','{}'); Wait-Process -Id $child.Id",
                descendant_script.display(),
                started.display(),
                survived.display()
            )
        } else {
            std::fs::write(
                &descendant_script,
                "#!/bin/sh\nprintf started > \"$1\"\nsleep 1\nprintf survived > \"$2\"\n",
            )
            .unwrap();
            format!(
                "sh '{}' '{}' '{}' & wait",
                descendant_script.display(),
                started.display(),
                survived.display()
            )
        };
        let task_spec = CredentialProcessSpec::inherit(command);
        let task = tokio::spawn(async move { SpawnedCredentialProcess.run(&task_spec).await });

        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while !started.exists() {
                if task.is_finished() {
                    // The runner's own timeout cancelled the process before
                    // the shell managed to start it — retry on a slow host.
                    return;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("credential process started within the wait budget");
        if !started.exists() {
            return TestAttempt::RetryStartup;
        }
        task.abort();
        let _ = task.await;
        tokio::time::sleep(std::time::Duration::from_millis(1800)).await;
        assert!(
            !survived.exists(),
            "credential_process descendant survived runner cancellation"
        );
        TestAttempt::Passed
    }
}
