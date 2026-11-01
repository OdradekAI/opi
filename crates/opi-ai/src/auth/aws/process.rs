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

#[cfg(windows)]
mod windows;

/// Own the configured child's tree across parent exit and caller cancellation.
/// Unix retains its process group; Windows attaches before child code runs.
pub(crate) struct ProcessTree {
    #[cfg(windows)]
    job: Option<windows::Job>,
}

impl ProcessTree {
    pub(crate) fn prepare(command: &mut Command) -> std::io::Result<Self> {
        #[cfg(windows)]
        {
            let job = windows::Job::new()?;
            command.creation_flags(windows_sys::Win32::System::Threading::CREATE_SUSPENDED);
            Ok(Self { job: Some(job) })
        }
        #[cfg(not(windows))]
        {
            #[cfg(unix)]
            command.process_group(0);
            #[cfg(not(unix))]
            let _ = command;
            Ok(Self {})
        }
    }

    pub(crate) fn attach_and_resume(&self, child: &Child) -> std::io::Result<()> {
        #[cfg(windows)]
        {
            self.job
                .as_ref()
                .ok_or_else(|| std::io::Error::other("process job is closed"))?
                .attach_and_resume(child)
        }
        #[cfg(not(windows))]
        {
            let _ = child;
            Ok(())
        }
    }

    #[cfg(windows)]
    fn terminate(&mut self) {
        // Closing this last, noninherited handle kills every member even if
        // the original parent process has already exited.
        drop(self.job.take());
    }
}

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

/// Declared opi policy for one spawned `credential_process`: how long the
/// frozen command may run and how much stdout it may produce. The locked
/// Node corpus has no explicit timeout/output cap here, so these budgets
/// are an opi caller policy (AIR-023), not an SDK parity default: callers
/// inject their own values; the default below is the opi-declared budget
/// (it must absorb cold shell startup on loaded hosts — a Windows
/// PowerShell cold start alone can exceed three seconds).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CredentialProcessPolicy {
    /// Maximum wall-clock runtime of the spawned process.
    pub timeout: Duration,
    /// Maximum accepted stdout size in bytes.
    pub max_stdout: usize,
}

impl Default for CredentialProcessPolicy {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(10),
            max_stdout: 64 * 1024,
        }
    }
}

#[cfg(unix)]
const PROCESS_TREE_TERMINATION_TIMEOUT: Duration = Duration::from_secs(2);

/// Default [`CredentialProcessRunner`] that really spawns the frozen
/// command under the injected [`CredentialProcessPolicy`].
#[derive(Default)]
pub struct SpawnedCredentialProcess {
    policy: CredentialProcessPolicy,
}

impl SpawnedCredentialProcess {
    /// Build the runner with an explicit caller policy.
    pub fn with_policy(policy: CredentialProcessPolicy) -> Self {
        Self { policy }
    }

    /// The injected policy (non-secret; observable for diagnostics).
    pub fn policy(&self) -> CredentialProcessPolicy {
        self.policy
    }
}

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
        Box::pin(async move { run_spawned_credential_process(spec, self.policy).await })
    }
}

async fn run_spawned_credential_process(
    spec: &CredentialProcessSpec,
    policy: CredentialProcessPolicy,
) -> Result<CredentialProcessOutput, CredentialProcessError> {
    let spec = spec.clone();
    own_process(move |cancellation| async move {
        run_spawned_credential_process_inner(&spec, cancellation, policy).await
    })
    .await
    .map_err(|_| CredentialProcessError::Join)?
}

/// The host runtime owns cleanup after a helper caller stops waiting.
/// Dropping this wait signals cancellation but leaves the worker alive to
/// terminate and reap its child tree.
pub(crate) async fn own_process<T, F, Fut>(work: F) -> Result<T, tokio::task::JoinError>
where
    T: Send + 'static,
    F: FnOnce(CancellationToken) -> Fut,
    Fut: std::future::Future<Output = T> + Send + 'static,
{
    let cancellation = CancellationToken::new();
    let mut cancel_on_drop = CancelProcessOnDrop {
        cancellation: cancellation.clone(),
        armed: true,
    };
    let result = tokio::spawn(work(cancellation)).await;
    cancel_on_drop.armed = false;
    result
}

async fn run_spawned_credential_process_inner(
    spec: &CredentialProcessSpec,
    cancellation: CancellationToken,
    policy: CredentialProcessPolicy,
) -> Result<CredentialProcessOutput, CredentialProcessError> {
    if cancellation.is_cancelled() {
        return Err(CredentialProcessError::Cancelled);
    }
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
    let mut tree =
        ProcessTree::prepare(&mut command_builder).map_err(|_| CredentialProcessError::Spawn)?;
    let mut child = command_builder
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| CredentialProcessError::Spawn)?;
    let root_pid = child.id();
    if tree.attach_and_resume(&child).is_err() {
        terminate_tree_and_reap(&mut child, root_pid, &mut tree).await;
        return Err(CredentialProcessError::Spawn);
    }

    let output =
        read_bounded_process_output(&mut child, root_pid, &mut tree, cancellation, policy).await?;
    let parsed = parse_credential_process_output(&output);
    if parsed.is_err() {
        terminate_tree_and_reap(&mut child, root_pid, &mut tree).await;
    }
    parsed
}

async fn read_bounded_process_output(
    child: &mut Child,
    root_pid: Option<u32>,
    tree: &mut ProcessTree,
    cancellation: CancellationToken,
    policy: CredentialProcessPolicy,
) -> Result<Vec<u8>, CredentialProcessError> {
    let Some(mut stdout) = child.stdout.take() else {
        terminate_tree_and_reap(child, root_pid, tree).await;
        return Err(CredentialProcessError::Output);
    };
    let deadline = Instant::now() + policy.timeout;
    let mut output = Vec::new();
    let mut limited_stdout = (&mut stdout).take((policy.max_stdout + 1) as u64);
    let read_result = tokio::select! {
        _ = cancellation.cancelled() => {
            terminate_tree_and_reap(child, root_pid, tree).await;
            return Err(CredentialProcessError::Cancelled);
        }
        _ = sleep_until(deadline) => {
            terminate_tree_and_reap(child, root_pid, tree).await;
            return Err(CredentialProcessError::Timeout);
        }
        result = limited_stdout.read_to_end(&mut output) => result,
    };
    if read_result.is_err() {
        terminate_tree_and_reap(child, root_pid, tree).await;
        return Err(CredentialProcessError::Output);
    }
    if output.len() > policy.max_stdout {
        terminate_tree_and_reap(child, root_pid, tree).await;
        return Err(CredentialProcessError::OutputLimit);
    }
    let status = tokio::select! {
        _ = cancellation.cancelled() => {
            terminate_tree_and_reap(child, root_pid, tree).await;
            return Err(CredentialProcessError::Cancelled);
        }
        _ = sleep_until(deadline) => {
            terminate_tree_and_reap(child, root_pid, tree).await;
            return Err(CredentialProcessError::Timeout);
        }
        result = child.wait() => result,
    };
    let status = match status {
        Ok(status) => status,
        Err(_) => {
            terminate_tree_and_reap(child, root_pid, tree).await;
            return Err(CredentialProcessError::Output);
        }
    };
    if !status.success() {
        terminate_tree_and_reap(child, root_pid, tree).await;
        return Err(CredentialProcessError::Exit);
    }
    Ok(output)
}

pub(crate) async fn terminate_tree_and_reap(
    child: &mut Child,
    root_pid: Option<u32>,
    tree: &mut ProcessTree,
) {
    #[cfg(windows)]
    {
        let _ = root_pid;
        tree.terminate();
    }
    #[cfg(not(windows))]
    let _ = tree;
    #[cfg(unix)]
    if let Some(root_pid) = root_pid {
        terminate_process_tree(root_pid).await;
    }
    #[cfg(not(any(windows, unix)))]
    let _ = root_pid;
    let _ = child.start_kill();
    let _ = child.wait().await;
}

#[cfg(unix)]
async fn terminate_process_tree(root_pid: u32) {
    let mut command = {
        let mut command = Command::new("kill");
        // The `--` separator is required: procps-ng `kill` parses a bare
        // `-<pgid>` argument as an option and exits successfully without
        // signalling anyone, silently leaving the process group alive.
        command.arg("-KILL").arg("--").arg(format!("-{root_pid}"));
        command
    };
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
        let parsed = SpawnedCredentialProcess::default()
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
        let env = BTreeMap::from([(path_var.to_owned(), inherited_path)]);
        #[cfg(windows)]
        let env = {
            let mut env = env;
            env.insert(
                "SystemRoot".to_owned(),
                std::env::var("SystemRoot").expect("Windows system root"),
            );
            for name in ["USERPROFILE", "HOME", "TMP", "TEMP"] {
                env.insert(name.to_owned(), dir.path().to_string_lossy().into_owned());
            }
            env
        };
        let spec = CredentialProcessSpec {
            command,
            cwd: Some(dir.path().to_path_buf()),
            env: ProcessEnv::Exact(env),
        };
        // This test exercises the frozen cwd/env contract, not the default
        // 10s budget (the dedicated timeout test owns that). A cold CI
        // windows runner can spend more than 10s in Defender-scanned
        // PowerShell startup under the near-empty Exact environment, so
        // inject the caller-policy seam with an inflated startup budget.
        let runner = SpawnedCredentialProcess::with_policy(CredentialProcessPolicy {
            timeout: Duration::from_secs(60),
            max_stdout: 64 * 1024,
        });
        let parsed = runner.run(&spec).await.expect("frozen cwd/env run");
        assert_eq!(parsed.access_key_id.expose_secret(), "PROC_AKID");
    }

    #[tokio::test]
    async fn spawned_runner_failures_are_typed_and_bounded() {
        let _process_lock = CREDENTIAL_PROCESS_TEST_LOCK.lock().await;
        let exit = CredentialProcessSpec::inherit("exit 9");
        assert_eq!(
            SpawnedCredentialProcess::default()
                .run(&exit)
                .await
                .unwrap_err(),
            CredentialProcessError::Exit
        );

        let timeout_command = if cfg!(windows) {
            "Start-Sleep -Seconds 30"
        } else {
            "sleep 30"
        };
        let timeout = CredentialProcessSpec::inherit(timeout_command);
        assert_eq!(
            SpawnedCredentialProcess::default()
                .run(&timeout)
                .await
                .unwrap_err(),
            CredentialProcessError::Timeout
        );

        let output_command = if cfg!(windows) {
            "[Console]::Out.Write('x' * 70000)".to_owned()
        } else {
            "head -c 70000 /dev/zero".to_owned()
        };
        let limit = CredentialProcessSpec::inherit(output_command);
        assert_eq!(
            SpawnedCredentialProcess::default()
                .run(&limit)
                .await
                .unwrap_err(),
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
        let error = SpawnedCredentialProcess::default()
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
        let task =
            tokio::spawn(async move { SpawnedCredentialProcess::default().run(&task_spec).await });

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
