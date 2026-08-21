//! Bedrock credential resolution.
//!
//! Precedence: explicit config > env vars > shared AWS profile files.
//! No live AWS calls.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use secrecy::SecretString;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::time::{Instant, sleep_until};
use tokio_util::sync::CancellationToken;

use crate::auth::{
    AuthFallback, AuthProvenance, AuthProvenanceSource, AwsCredentialSource, AwsSigV4Credentials,
    ResolvedAuth,
};
use crate::provider::ProviderErrorSummary;

/// Resolved zeroizing AWS credentials for Bedrock.
pub type BedrockCredentials = AwsSigV4Credentials;

/// Source of resolved credentials (for diagnostics, never logged with secrets).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialSource {
    ExplicitConfig,
    Environment,
    ProfileFile,
    ConfigFile,
    CredentialProcess,
}

/// Safe, typed failures from local AWS credential resolution.
///
/// Error values deliberately omit file paths, process output, operating-system
/// diagnostics, and credential material so callers can surface them safely.
#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum CredentialResolutionError {
    #[error("no AWS credentials are available")]
    Exhausted,
    #[error("the attempted {credential_source:?} AWS credential source is incomplete")]
    IncompleteSource { credential_source: CredentialSource },
    #[error("the {credential_source:?} AWS profile could not be read")]
    ProfileIo { credential_source: CredentialSource },
    #[error("the {credential_source:?} AWS profile is malformed")]
    ProfileParse { credential_source: CredentialSource },
    #[error("credential_process could not be started")]
    CredentialProcessSpawn,
    #[error("credential_process command is blank")]
    CredentialProcessCommand,
    #[error("credential_process exited unsuccessfully")]
    CredentialProcessExit,
    #[error("credential_process timed out")]
    CredentialProcessTimeout,
    #[error("credential_process output exceeded the limit")]
    CredentialProcessOutputLimit,
    #[error("credential_process output could not be read")]
    CredentialProcessOutput,
    #[error("credential_process output was not valid JSON")]
    CredentialProcessJson,
    #[error("credential_process returned an unsupported version")]
    CredentialProcessVersion,
    #[error("credential_process returned incomplete credentials")]
    CredentialProcessIncomplete,
    #[error("credential_process resolution was cancelled")]
    CredentialProcessCancelled,
    #[error("credential_process resolution task failed")]
    CredentialProcessJoin,
}

impl CredentialResolutionError {
    /// Convert this closed, non-secret reason into a provider-facing summary.
    /// Unlike arbitrary upstream text, these variants contain no paths,
    /// process output, operating-system diagnostics, or credential material.
    pub fn into_safe_provider_summary(self) -> ProviderErrorSummary {
        ProviderErrorSummary::sanitized(self.to_string())
    }
}

impl From<CredentialSource> for AwsCredentialSource {
    fn from(source: CredentialSource) -> Self {
        match source {
            CredentialSource::ExplicitConfig => Self::ExplicitConfig,
            CredentialSource::Environment => Self::Environment,
            CredentialSource::ProfileFile => Self::ProfileFile,
            CredentialSource::ConfigFile => Self::ConfigFile,
            CredentialSource::CredentialProcess => Self::CredentialProcess,
        }
    }
}

/// Input parameters for credential resolution.
pub struct CredentialResolutionInput<'a> {
    pub config_access_key_id: Option<&'a str>,
    pub config_secret_access_key: Option<&'a str>,
    pub config_session_token: Option<&'a str>,
    pub config_region: Option<&'a str>,
    pub env_access_key_id: Option<&'a str>,
    pub env_secret_access_key: Option<&'a str>,
    pub env_session_token: Option<&'a str>,
    pub env_region: Option<&'a str>,
    pub profile_name: Option<&'a str>,
    pub credentials_file_path: Option<&'a Path>,
    pub config_file_path: Option<&'a Path>,
}

impl<'a> CredentialResolutionInput<'a> {
    /// Build from environment variables.
    ///
    /// The caller must own the strings and pass references:
    /// ```
    /// # use opi_ai::bedrock::credentials::{credentials_from_env, CredentialResolutionInput};
    /// let (akid, sak, token, region) = credentials_from_env();
    /// let input = CredentialResolutionInput::from_env_refs(
    ///     akid.as_deref(), sak.as_deref(), token.as_deref(), region.as_deref(),
    /// );
    /// ```
    pub fn from_env_refs(
        env_access_key_id: Option<&'a str>,
        env_secret_access_key: Option<&'a str>,
        env_session_token: Option<&'a str>,
        env_region: Option<&'a str>,
    ) -> Self {
        Self {
            config_access_key_id: None,
            config_secret_access_key: None,
            config_session_token: None,
            config_region: None,
            env_access_key_id,
            env_secret_access_key,
            env_session_token,
            env_region,
            profile_name: None,
            credentials_file_path: None,
            config_file_path: None,
        }
    }
}

/// Resolve Bedrock credentials with precedence:
/// explicit config > env vars > shared AWS profile files.
///
/// An entirely absent source advances to the next source. A partial or blank
/// attempted pair fails closed with [`CredentialResolutionError::IncompleteSource`].
pub async fn resolve_credentials(
    input: &CredentialResolutionInput<'_>,
) -> Result<(BedrockCredentials, CredentialSource), CredentialResolutionError> {
    // 1. Explicit config takes highest precedence
    let config_attempted = input.config_access_key_id.is_some()
        || input.config_secret_access_key.is_some()
        || input.config_session_token.is_some();
    if config_attempted {
        if let (Some(akid), Some(sak)) = (
            non_blank(input.config_access_key_id),
            non_blank(input.config_secret_access_key),
        ) {
            return Ok((
                BedrockCredentials {
                    access_key_id: SecretString::from(akid),
                    secret_access_key: SecretString::from(sak),
                    session_token: input
                        .config_session_token
                        .filter(|s| !s.trim().is_empty())
                        .map(SecretString::from),
                    region: input
                        .config_region
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or("us-east-1")
                        .to_string(),
                },
                CredentialSource::ExplicitConfig,
            ));
        }
        return Err(CredentialResolutionError::IncompleteSource {
            credential_source: CredentialSource::ExplicitConfig,
        });
    }

    // 2. Environment variables
    let environment_attempted = input.env_access_key_id.is_some()
        || input.env_secret_access_key.is_some()
        || input.env_session_token.is_some();
    if environment_attempted {
        if let (Some(akid), Some(sak)) = (
            non_blank(input.env_access_key_id),
            non_blank(input.env_secret_access_key),
        ) {
            return Ok((
                BedrockCredentials {
                    access_key_id: SecretString::from(akid),
                    secret_access_key: SecretString::from(sak),
                    session_token: input
                        .env_session_token
                        .filter(|s| !s.trim().is_empty())
                        .map(SecretString::from),
                    region: input
                        .env_region
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or("us-east-1")
                        .to_string(),
                },
                CredentialSource::Environment,
            ));
        }
        return Err(CredentialResolutionError::IncompleteSource {
            credential_source: CredentialSource::Environment,
        });
    }

    // 3. Shared AWS credentials/config profiles. We intentionally keep this
    // local-only: no IMDS, SSO browser/device flow, or web-identity network
    // calls are attempted from opi.
    let profile = input
        .profile_name
        .filter(|profile| !profile.trim().is_empty())
        .unwrap_or("default");

    let credentials_props = match input.credentials_file_path {
        Some(path) => {
            read_profile_properties_async(
                path,
                profile,
                ProfileFileKind::Credentials,
                CredentialSource::ProfileFile,
            )
            .await?
        }
        None => None,
    };

    if let Some(props) = credentials_props.as_ref()
        && profile_static_credentials_attempted(props)
        && !profile_static_credentials_complete(props)
    {
        return Err(CredentialResolutionError::IncompleteSource {
            credential_source: CredentialSource::ProfileFile,
        });
    }

    let config_props = match input.config_file_path {
        Some(path) => {
            read_profile_properties_async(
                path,
                profile,
                ProfileFileKind::Config,
                CredentialSource::ConfigFile,
            )
            .await?
        }
        None => None,
    };

    let region = first_non_empty(&[
        input.config_region,
        input.env_region,
        credentials_props
            .as_ref()
            .and_then(|props| props.region.as_deref()),
        config_props
            .as_ref()
            .and_then(|props| props.region.as_deref()),
    ])
    .unwrap_or("us-east-1")
    .to_string();

    if let Some(props) = credentials_props.as_ref()
        && profile_static_credentials_attempted(props)
    {
        return profile_properties_to_credentials(props, region.clone())
            .map(|credentials| (credentials, CredentialSource::ProfileFile))
            .ok_or(CredentialResolutionError::IncompleteSource {
                credential_source: CredentialSource::ProfileFile,
            });
    }
    if let Some(props) = config_props.as_ref()
        && profile_static_credentials_attempted(props)
    {
        return profile_properties_to_credentials(props, region.clone())
            .map(|credentials| (credentials, CredentialSource::ConfigFile))
            .ok_or(CredentialResolutionError::IncompleteSource {
                credential_source: CredentialSource::ConfigFile,
            });
    }
    if let Some(command) = config_props
        .as_ref()
        .and_then(|props| props.credential_process.as_deref())
    {
        if command.trim().is_empty() {
            return Err(CredentialResolutionError::CredentialProcessCommand);
        }
        let credentials = run_credential_process(command, region).await?;
        return Ok((credentials, CredentialSource::CredentialProcess));
    }

    Err(CredentialResolutionError::Exhausted)
}

/// Resolve one complete prepared Bedrock authentication result.
///
/// The selected AWS source is retained as typed, non-secret provenance beside
/// the zeroizing SigV4 credential bundle.
pub async fn resolve_auth(
    input: &CredentialResolutionInput<'_>,
) -> Result<ResolvedAuth, CredentialResolutionError> {
    let (credentials, source) = resolve_credentials(input).await?;
    Ok(ResolvedAuth::aws_sigv4(
        credentials,
        AuthProvenance {
            source: AuthProvenanceSource::AwsSigV4 {
                source: source.into(),
            },
            fallback: AuthFallback::NotAttempted,
        },
    ))
}

/// Read a specific profile from an AWS credentials INI file.
pub fn read_profile(path: &Path, profile_name: &str) -> Option<BedrockCredentials> {
    let props = read_profile_properties(path, profile_name, ProfileFileKind::Credentials)?;
    profile_properties_to_credentials(
        &props,
        props
            .region
            .clone()
            .unwrap_or_else(|| "us-east-1".to_string()),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfileFileKind {
    Credentials,
    Config,
}

#[derive(Clone, Default)]
struct ProfileProperties {
    access_key_id: Option<String>,
    secret_access_key: Option<String>,
    session_token: Option<String>,
    region: Option<String>,
    credential_process: Option<String>,
}

impl std::fmt::Debug for ProfileProperties {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProfileProperties")
            .field(
                "access_key_id",
                &self.access_key_id.as_ref().map(|_| "<redacted>"),
            )
            .field(
                "secret_access_key",
                &self.secret_access_key.as_ref().map(|_| "<redacted>"),
            )
            .field(
                "session_token",
                &self.session_token.as_ref().map(|_| "<redacted>"),
            )
            .field("region", &self.region)
            .field(
                "credential_process",
                &self.credential_process.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

fn first_non_empty<'a>(values: &[Option<&'a str>]) -> Option<&'a str> {
    values
        .iter()
        .filter_map(|value| value.and_then(|s| (!s.trim().is_empty()).then_some(s)))
        .next()
}

fn non_blank(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.trim().is_empty())
}

fn profile_static_credentials_attempted(props: &ProfileProperties) -> bool {
    props.access_key_id.is_some()
        || props.secret_access_key.is_some()
        || props.session_token.is_some()
}

fn profile_static_credentials_complete(props: &ProfileProperties) -> bool {
    non_blank(props.access_key_id.as_deref()).is_some()
        && non_blank(props.secret_access_key.as_deref()).is_some()
}

fn profile_properties_to_credentials(
    props: &ProfileProperties,
    region: String,
) -> Option<BedrockCredentials> {
    match (&props.access_key_id, &props.secret_access_key) {
        (Some(a), Some(s)) if !a.trim().is_empty() && !s.trim().is_empty() => {
            Some(BedrockCredentials {
                access_key_id: a.clone().into(),
                secret_access_key: s.clone().into(),
                session_token: props
                    .session_token
                    .clone()
                    .filter(|token| !token.trim().is_empty())
                    .map(SecretString::from),
                region,
            })
        }
        _ => None,
    }
}

fn read_profile_properties(
    path: &Path,
    profile_name: &str,
    kind: ProfileFileKind,
) -> Option<ProfileProperties> {
    let contents = std::fs::read_to_string(path).ok()?;
    parse_profile_properties(
        &contents,
        profile_name,
        kind,
        match kind {
            ProfileFileKind::Credentials => CredentialSource::ProfileFile,
            ProfileFileKind::Config => CredentialSource::ConfigFile,
        },
    )
    .ok()
    .flatten()
}

async fn read_profile_properties_async(
    path: &Path,
    profile_name: &str,
    kind: ProfileFileKind,
    source: CredentialSource,
) -> Result<Option<ProfileProperties>, CredentialResolutionError> {
    let exists =
        tokio::fs::try_exists(path)
            .await
            .map_err(|_| CredentialResolutionError::ProfileIo {
                credential_source: source,
            })?;
    if !exists {
        return Ok(None);
    }
    let contents = tokio::fs::read_to_string(path).await.map_err(|_| {
        CredentialResolutionError::ProfileIo {
            credential_source: source,
        }
    })?;
    parse_profile_properties(&contents, profile_name, kind, source)
}

fn parse_profile_properties(
    contents: &str,
    profile_name: &str,
    kind: ProfileFileKind,
    source: CredentialSource,
) -> Result<Option<ProfileProperties>, CredentialResolutionError> {
    let target_header = match kind {
        ProfileFileKind::Credentials => format!("[{profile_name}]"),
        ProfileFileKind::Config if profile_name == "default" => "[default]".to_string(),
        ProfileFileKind::Config => format!("[profile {profile_name}]"),
    };

    let mut in_target = false;
    let mut props = ProfileProperties::default();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            if in_target {
                break;
            }
            in_target = line == target_header;
            continue;
        }
        if in_target {
            let Some((key, value)) = line.split_once('=') else {
                return Err(CredentialResolutionError::ProfileParse {
                    credential_source: source,
                });
            };
            let key = key.trim();
            let value = value.trim().to_string();
            match key {
                "aws_access_key_id" => {
                    props.access_key_id = Some(value);
                }
                "aws_secret_access_key" => {
                    props.secret_access_key = Some(value);
                }
                "aws_session_token" => props.session_token = Some(value),
                "region" => props.region = Some(value),
                "credential_process" => props.credential_process = Some(value),
                _ => {}
            }
        }
    }

    Ok((props.access_key_id.is_some()
        || props.secret_access_key.is_some()
        || props.session_token.is_some()
        || props.region.is_some()
        || props.credential_process.is_some())
    .then_some(props))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CredentialProcessOutput {
    version: u32,
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
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
            .finish()
    }
}

/// Local credential processes are given ten seconds and at most 64 KiB of
/// stdout. The budget must absorb cold shell startup on loaded hosts (a
/// Windows PowerShell cold start alone can exceed three seconds), so the
/// process itself is only expected to finish well inside the window.
/// Cancellation kills and reaps the spawned shell before its detached
/// cleanup task exits.
const CREDENTIAL_PROCESS_TIMEOUT: Duration = Duration::from_secs(10);
const CREDENTIAL_PROCESS_MAX_STDOUT: usize = 64 * 1024;
const PROCESS_TREE_TERMINATION_TIMEOUT: Duration = Duration::from_secs(2);

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

async fn run_credential_process(
    command: &str,
    region: String,
) -> Result<BedrockCredentials, CredentialResolutionError> {
    let cancellation = CancellationToken::new();
    let mut cancel_on_drop = CancelProcessOnDrop {
        cancellation: cancellation.clone(),
        armed: true,
    };
    let command = command.to_owned();
    let task =
        tokio::spawn(
            async move { run_credential_process_inner(&command, region, cancellation).await },
        );
    let result = task
        .await
        .map_err(|_| CredentialResolutionError::CredentialProcessJoin)?;
    cancel_on_drop.armed = false;
    result
}

async fn run_credential_process_inner(
    command: &str,
    region: String,
    cancellation: CancellationToken,
) -> Result<BedrockCredentials, CredentialResolutionError> {
    let mut command_builder = if cfg!(windows) {
        let mut builder = Command::new("powershell");
        builder.args(["-NoProfile", "-Command", command]);
        builder
    } else {
        let mut builder = Command::new("sh");
        builder.args(["-c", command]);
        builder
    };
    #[cfg(unix)]
    command_builder.process_group(0);
    let mut child = command_builder
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| CredentialResolutionError::CredentialProcessSpawn)?;
    let root_pid = child.id();

    let result = read_bounded_process_output(&mut child, root_pid, cancellation).await;
    let output = result?;
    let parsed: CredentialProcessOutput = match serde_json::from_slice(&output) {
        Ok(parsed) => parsed,
        Err(_) => {
            terminate_tree_and_reap(&mut child, root_pid).await;
            return Err(CredentialResolutionError::CredentialProcessJson);
        }
    };
    if parsed.version != 1 {
        terminate_tree_and_reap(&mut child, root_pid).await;
        return Err(CredentialResolutionError::CredentialProcessVersion);
    }
    if parsed.access_key_id.trim().is_empty() || parsed.secret_access_key.trim().is_empty() {
        terminate_tree_and_reap(&mut child, root_pid).await;
        return Err(CredentialResolutionError::CredentialProcessIncomplete);
    }
    Ok(BedrockCredentials {
        access_key_id: parsed.access_key_id.into(),
        secret_access_key: parsed.secret_access_key.into(),
        session_token: parsed
            .session_token
            .filter(|token| !token.trim().is_empty())
            .map(SecretString::from),
        region,
    })
}

async fn read_bounded_process_output(
    child: &mut Child,
    root_pid: Option<u32>,
    cancellation: CancellationToken,
) -> Result<Vec<u8>, CredentialResolutionError> {
    let Some(mut stdout) = child.stdout.take() else {
        terminate_tree_and_reap(child, root_pid).await;
        return Err(CredentialResolutionError::CredentialProcessOutput);
    };
    let deadline = Instant::now() + CREDENTIAL_PROCESS_TIMEOUT;
    let mut output = Vec::new();
    let mut limited_stdout = (&mut stdout).take((CREDENTIAL_PROCESS_MAX_STDOUT + 1) as u64);
    let read_result = tokio::select! {
        _ = cancellation.cancelled() => {
            terminate_tree_and_reap(child, root_pid).await;
            return Err(CredentialResolutionError::CredentialProcessCancelled);
        }
        _ = sleep_until(deadline) => {
            terminate_tree_and_reap(child, root_pid).await;
            return Err(CredentialResolutionError::CredentialProcessTimeout);
        }
        result = limited_stdout.read_to_end(&mut output) => result,
    };
    if read_result.is_err() {
        terminate_tree_and_reap(child, root_pid).await;
        return Err(CredentialResolutionError::CredentialProcessOutput);
    }
    if output.len() > CREDENTIAL_PROCESS_MAX_STDOUT {
        terminate_tree_and_reap(child, root_pid).await;
        return Err(CredentialResolutionError::CredentialProcessOutputLimit);
    }
    let status = tokio::select! {
        _ = cancellation.cancelled() => {
            terminate_tree_and_reap(child, root_pid).await;
            return Err(CredentialResolutionError::CredentialProcessCancelled);
        }
        _ = sleep_until(deadline) => {
            terminate_tree_and_reap(child, root_pid).await;
            return Err(CredentialResolutionError::CredentialProcessTimeout);
        }
        result = child.wait() => result,
    };
    let status = match status {
        Ok(status) => status,
        Err(_) => {
            terminate_tree_and_reap(child, root_pid).await;
            return Err(CredentialResolutionError::CredentialProcessOutput);
        }
    };
    if !status.success() {
        terminate_tree_and_reap(child, root_pid).await;
        return Err(CredentialResolutionError::CredentialProcessExit);
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

/// Read AWS credentials from environment variables.
/// Returns (access_key_id, secret_access_key, session_token, region).
pub fn credentials_from_env() -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let akid = std::env::var("AWS_ACCESS_KEY_ID").ok();
    let sak = std::env::var("AWS_SECRET_ACCESS_KEY").ok();
    let token = std::env::var("AWS_SESSION_TOKEN").ok();
    let region = std::env::var("AWS_REGION")
        .ok()
        .or_else(|| std::env::var("AWS_DEFAULT_REGION").ok());
    (akid, sak, token, region)
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;
    use std::io::Write as IoWrite;

    static CREDENTIAL_PROCESS_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    #[allow(clippy::too_many_arguments)]
    fn input<'a>(
        config_akid: Option<&'a str>,
        config_sak: Option<&'a str>,
        config_token: Option<&'a str>,
        config_region: Option<&'a str>,
        env_akid: Option<&'a str>,
        env_sak: Option<&'a str>,
        env_token: Option<&'a str>,
        env_region: Option<&'a str>,
        profile: Option<&'a str>,
        creds_path: Option<&'a Path>,
    ) -> CredentialResolutionInput<'a> {
        CredentialResolutionInput {
            config_access_key_id: config_akid,
            config_secret_access_key: config_sak,
            config_session_token: config_token,
            config_region,
            env_access_key_id: env_akid,
            env_secret_access_key: env_sak,
            env_session_token: env_token,
            env_region,
            profile_name: profile,
            credentials_file_path: creds_path,
            config_file_path: None,
        }
    }

    #[tokio::test]
    async fn explicit_config_takes_precedence() {
        let inp = input(
            Some("CONFIG_AKID"),
            Some("CONFIG_SAK"),
            Some("CONFIG_TOKEN"),
            Some("eu-west-1"),
            Some("ENV_AKID"),
            Some("ENV_SAK"),
            None,
            Some("us-west-2"),
            None,
            None,
        );
        let (result, source) = resolve_credentials(&inp).await.unwrap();
        assert_eq!(result.access_key_id.expose_secret(), "CONFIG_AKID");
        assert_eq!(result.secret_access_key.expose_secret(), "CONFIG_SAK");
        assert_eq!(
            result
                .session_token
                .as_ref()
                .map(|token| token.expose_secret()),
            Some("CONFIG_TOKEN")
        );
        assert_eq!(result.region, "eu-west-1");
        assert_eq!(source, CredentialSource::ExplicitConfig);
    }

    #[tokio::test]
    async fn env_vars_when_no_config() {
        let inp = input(
            None,
            None,
            None,
            None,
            Some("ENV_AKID"),
            Some("ENV_SAK"),
            Some("ENV_TOKEN"),
            Some("ap-southeast-1"),
            None,
            None,
        );
        let (result, source) = resolve_credentials(&inp).await.unwrap();
        assert_eq!(result.access_key_id.expose_secret(), "ENV_AKID");
        assert_eq!(result.secret_access_key.expose_secret(), "ENV_SAK");
        assert_eq!(source, CredentialSource::Environment);
    }

    #[tokio::test]
    async fn profile_file_when_no_config_or_env() {
        let dir = tempfile::tempdir().unwrap();
        let cred_file = dir.path().join("credentials");
        {
            let mut f = std::fs::File::create(&cred_file).unwrap();
            writeln!(f, "[default]").unwrap();
            writeln!(f, "aws_access_key_id = PROFILE_AKID").unwrap();
            writeln!(f, "aws_secret_access_key = PROFILE_SAK").unwrap();
            writeln!(f, "region = us-west-2").unwrap();
        }

        let inp = input(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("default"),
            Some(cred_file.as_path()),
        );
        let (result, source) = resolve_credentials(&inp).await.unwrap();
        assert_eq!(result.access_key_id.expose_secret(), "PROFILE_AKID");
        assert_eq!(result.secret_access_key.expose_secret(), "PROFILE_SAK");
        assert_eq!(result.region, "us-west-2");
        assert_eq!(source, CredentialSource::ProfileFile);
    }

    #[tokio::test]
    async fn default_profile_file_used_when_profile_not_explicit() {
        let dir = tempfile::tempdir().unwrap();
        let cred_file = dir.path().join("credentials");
        {
            let mut f = std::fs::File::create(&cred_file).unwrap();
            writeln!(f, "[default]").unwrap();
            writeln!(f, "aws_access_key_id = DEFAULT_AKID").unwrap();
            writeln!(f, "aws_secret_access_key = DEFAULT_SAK").unwrap();
        }

        let inp = input(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(cred_file.as_path()),
        );
        let (result, source) = resolve_credentials(&inp).await.unwrap();
        assert_eq!(result.access_key_id.expose_secret(), "DEFAULT_AKID");
        assert_eq!(source, CredentialSource::ProfileFile);
    }

    #[tokio::test]
    async fn shared_config_profile_region_is_used_with_credentials_file() {
        let dir = tempfile::tempdir().unwrap();
        let cred_file = dir.path().join("credentials");
        let config_file = dir.path().join("config");
        {
            let mut f = std::fs::File::create(&cred_file).unwrap();
            writeln!(f, "[dev]").unwrap();
            writeln!(f, "aws_access_key_id = DEV_AKID").unwrap();
            writeln!(f, "aws_secret_access_key = DEV_SAK").unwrap();
        }
        {
            let mut f = std::fs::File::create(&config_file).unwrap();
            writeln!(f, "[profile dev]").unwrap();
            writeln!(f, "region = ap-northeast-1").unwrap();
        }

        let mut inp = input(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("dev"),
            Some(cred_file.as_path()),
        );
        inp.config_file_path = Some(config_file.as_path());
        let (result, source) = resolve_credentials(&inp).await.unwrap();
        assert_eq!(result.access_key_id.expose_secret(), "DEV_AKID");
        assert_eq!(result.region, "ap-northeast-1");
        assert_eq!(source, CredentialSource::ProfileFile);
    }

    #[tokio::test]
    async fn partial_credentials_file_does_not_mix_with_complete_config_file_pair() {
        let dir = tempfile::tempdir().unwrap();
        let credentials_file = dir.path().join("credentials");
        let config_file = dir.path().join("config");
        std::fs::write(
            &credentials_file,
            "[dev]\naws_access_key_id=CREDENTIALS_ACCESS\n",
        )
        .unwrap();
        std::fs::write(
            &config_file,
            "[profile dev]\naws_access_key_id=CONFIG_ACCESS\naws_secret_access_key=CONFIG_SECRET\n",
        )
        .unwrap();
        let mut inp = input(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("dev"),
            Some(credentials_file.as_path()),
        );
        inp.config_file_path = Some(config_file.as_path());

        assert_eq!(
            resolve_credentials(&inp).await.unwrap_err(),
            CredentialResolutionError::IncompleteSource {
                credential_source: CredentialSource::ProfileFile,
            }
        );
    }

    #[tokio::test]
    async fn credentials_file_pair_does_not_attach_config_file_session_token() {
        let dir = tempfile::tempdir().unwrap();
        let credentials_file = dir.path().join("credentials");
        let config_file = dir.path().join("config");
        std::fs::write(
            &credentials_file,
            "[dev]\naws_access_key_id=CREDENTIALS_ACCESS\naws_secret_access_key=CREDENTIALS_SECRET\n",
        )
        .unwrap();
        std::fs::write(
            &config_file,
            "[profile dev]\naws_session_token=CONFIG_SESSION\nregion=eu-west-1\n",
        )
        .unwrap();
        let mut inp = input(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("dev"),
            Some(credentials_file.as_path()),
        );
        inp.config_file_path = Some(config_file.as_path());

        let (credentials, source) = resolve_credentials(&inp)
            .await
            .expect("credentials file pair");
        assert_eq!(source, CredentialSource::ProfileFile);
        assert!(credentials.session_token.is_none());
        assert_eq!(credentials.region, "eu-west-1");
    }

    #[tokio::test]
    async fn partial_config_file_pair_does_not_fall_through_to_credential_process() {
        let dir = tempfile::tempdir().unwrap();
        let output_file = dir.path().join("process-output.json");
        let config_file = dir.path().join("config");
        std::fs::write(
            &output_file,
            r#"{"Version":1,"AccessKeyId":"PROC_AKID","SecretAccessKey":"PROC_SAK"}"#,
        )
        .unwrap();
        let command = if cfg!(windows) {
            format!("Get-Content -Raw -LiteralPath '{}'", output_file.display())
        } else {
            format!("cat '{}'", output_file.display())
        };
        std::fs::write(
            &config_file,
            format!(
                "[profile proc]\naws_access_key_id=PARTIAL_ACCESS\ncredential_process={command}\n"
            ),
        )
        .unwrap();
        let mut inp = input(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("proc"),
            None,
        );
        inp.config_file_path = Some(config_file.as_path());

        assert_eq!(
            resolve_credentials(&inp).await.unwrap_err(),
            CredentialResolutionError::IncompleteSource {
                credential_source: CredentialSource::ConfigFile,
            }
        );
    }

    #[tokio::test]
    async fn shared_config_static_credentials_are_supported() {
        let dir = tempfile::tempdir().unwrap();
        let config_file = dir.path().join("config");
        {
            let mut f = std::fs::File::create(&config_file).unwrap();
            writeln!(f, "[profile ci]").unwrap();
            writeln!(f, "aws_access_key_id = CONFIG_AKID").unwrap();
            writeln!(f, "aws_secret_access_key = CONFIG_SAK").unwrap();
            writeln!(f, "region = eu-central-1").unwrap();
        }

        let mut inp = input(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("ci"),
            None,
        );
        inp.config_file_path = Some(config_file.as_path());
        let (result, source) = resolve_credentials(&inp).await.unwrap();
        assert_eq!(result.access_key_id.expose_secret(), "CONFIG_AKID");
        assert_eq!(result.region, "eu-central-1");
        assert_eq!(source, CredentialSource::ConfigFile);
    }

    #[tokio::test]
    async fn credential_process_from_shared_config_is_supported() {
        let _process_lock = CREDENTIAL_PROCESS_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let output_file = dir.path().join("process-output.json");
        let config_file = dir.path().join("config");
        std::fs::write(
            &output_file,
            r#"{"Version":1,"AccessKeyId":"PROC_AKID","SecretAccessKey":"PROC_SAK","SessionToken":"PROC_TOKEN"}"#,
        )
        .unwrap();
        let command = if cfg!(windows) {
            format!("Get-Content -Raw -LiteralPath '{}'", output_file.display())
        } else {
            format!("cat '{}'", output_file.display())
        };
        {
            let mut f = std::fs::File::create(&config_file).unwrap();
            writeln!(f, "[profile proc]").unwrap();
            writeln!(f, "region = us-west-1").unwrap();
            writeln!(f, "credential_process = {command}").unwrap();
        }

        let props = read_profile_properties(&config_file, "proc", ProfileFileKind::Config)
            .expect("profile should parse");
        assert_eq!(props.credential_process.as_deref(), Some(command.as_str()));
        let mut inp = input(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("proc"),
            None,
        );
        inp.config_file_path = Some(config_file.as_path());
        let (result, source) = resolve_credentials(&inp).await.unwrap();
        assert_eq!(result.access_key_id.expose_secret(), "PROC_AKID");
        assert_eq!(
            result
                .session_token
                .as_ref()
                .map(|token| token.expose_secret()),
            Some("PROC_TOKEN")
        );
        assert_eq!(result.region, "us-west-1");
        assert_eq!(source, CredentialSource::CredentialProcess);
    }

    #[tokio::test]
    async fn none_when_no_credentials_available() {
        let inp = input(None, None, None, None, None, None, None, None, None, None);
        assert_eq!(
            resolve_credentials(&inp).await.unwrap_err(),
            CredentialResolutionError::Exhausted
        );
    }

    #[tokio::test]
    async fn blank_config_pair_does_not_fall_through_to_environment() {
        let inp = input(
            Some(""),
            Some(""),
            None,
            None,
            Some("ENV_AKID"),
            Some("ENV_SAK"),
            None,
            None,
            None,
            None,
        );
        assert_eq!(
            resolve_credentials(&inp).await.unwrap_err(),
            CredentialResolutionError::IncompleteSource {
                credential_source: CredentialSource::ExplicitConfig,
            }
        );
    }

    #[tokio::test]
    async fn partial_config_pair_does_not_fall_through_to_environment() {
        let inp = input(
            Some("CONFIG_AKID"),
            None,
            None,
            None,
            Some("ENV_AKID"),
            Some("ENV_SAK"),
            None,
            None,
            None,
            None,
        );
        assert_eq!(
            resolve_credentials(&inp).await.unwrap_err(),
            CredentialResolutionError::IncompleteSource {
                credential_source: CredentialSource::ExplicitConfig,
            }
        );
    }

    #[tokio::test]
    async fn partial_environment_pair_does_not_fall_through_to_profile() {
        let dir = tempfile::tempdir().unwrap();
        let cred_file = dir.path().join("credentials");
        std::fs::write(
            &cred_file,
            "[default]\naws_access_key_id=PROFILE_AKID\naws_secret_access_key=PROFILE_SAK\n",
        )
        .unwrap();
        let inp = input(
            None,
            None,
            None,
            None,
            Some("ENV_AKID"),
            None,
            None,
            None,
            None,
            Some(cred_file.as_path()),
        );
        assert_eq!(
            resolve_credentials(&inp).await.unwrap_err(),
            CredentialResolutionError::IncompleteSource {
                credential_source: CredentialSource::Environment,
            }
        );
    }

    #[tokio::test]
    async fn whitespace_only_static_credential_pairs_are_absent() {
        for inp in [
            input(
                Some(" \t"),
                Some("CONFIG_SAK"),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            input(
                Some("CONFIG_AKID"),
                Some(" \n"),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            input(
                None,
                None,
                None,
                None,
                Some(" \t"),
                Some("ENV_SAK"),
                None,
                None,
                None,
                None,
            ),
            input(
                None,
                None,
                None,
                None,
                Some("ENV_AKID"),
                Some(" \n"),
                None,
                None,
                None,
                None,
            ),
        ] {
            assert!(
                matches!(
                    resolve_credentials(&inp).await,
                    Err(CredentialResolutionError::IncompleteSource { .. })
                ),
                "whitespace-only access/secret values must be absent"
            );
        }
    }

    #[tokio::test]
    async fn whitespace_only_optional_values_are_absent() {
        let inp = input(
            Some("CONFIG_AKID"),
            Some("CONFIG_SAK"),
            Some(" \t"),
            Some(" \n"),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let (credentials, source) = resolve_credentials(&inp)
            .await
            .expect("valid configured pair");
        assert_eq!(source, CredentialSource::ExplicitConfig);
        assert!(credentials.session_token.is_none());
        assert_eq!(credentials.region, "us-east-1");
    }

    #[test]
    fn parsed_credential_helpers_do_not_debug_credential_values() {
        let canaries = [
            "AKIA_PROFILE_DEBUG",
            "profile-secret-debug",
            "profile-session-debug",
        ];
        let profile = ProfileProperties {
            access_key_id: Some(canaries[0].into()),
            secret_access_key: Some(canaries[1].into()),
            session_token: Some(canaries[2].into()),
            region: Some("us-east-1".into()),
            credential_process: None,
        };
        let process = CredentialProcessOutput {
            version: 1,
            access_key_id: canaries[0].into(),
            secret_access_key: canaries[1].into(),
            session_token: Some(canaries[2].into()),
        };

        let debug = format!("{profile:?} {process:?}");
        for canary in canaries {
            assert!(
                !debug.contains(canary),
                "credential leaked through Debug: {debug}"
            );
        }
    }

    #[tokio::test]
    async fn default_region_when_not_specified() {
        let inp = input(
            Some("AKID"),
            Some("SAK"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let (result, _) = resolve_credentials(&inp).await.unwrap();
        assert_eq!(result.region, "us-east-1");
    }

    #[test]
    fn read_profile_with_session_token() {
        let dir = tempfile::tempdir().unwrap();
        let cred_file = dir.path().join("credentials");
        {
            let mut f = std::fs::File::create(&cred_file).unwrap();
            writeln!(f, "[my-profile]").unwrap();
            writeln!(f, "aws_access_key_id = AKID").unwrap();
            writeln!(f, "aws_secret_access_key = SAK").unwrap();
            writeln!(f, "aws_session_token = TOKEN").unwrap();
        }

        let creds = read_profile(&cred_file, "my-profile").unwrap();
        assert_eq!(creds.access_key_id.expose_secret(), "AKID");
        assert_eq!(
            creds
                .session_token
                .as_ref()
                .map(|token| token.expose_secret()),
            Some("TOKEN")
        );
    }

    #[test]
    fn read_profile_missing_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let cred_file = dir.path().join("credentials");
        {
            let mut f = std::fs::File::create(&cred_file).unwrap();
            writeln!(f, "[other-profile]").unwrap();
            writeln!(f, "aws_access_key_id = AKID").unwrap();
        }

        let result = read_profile(&cred_file, "missing-profile");
        assert!(result.is_none());
    }

    #[test]
    fn read_profile_incomplete_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let cred_file = dir.path().join("credentials");
        {
            let mut f = std::fs::File::create(&cred_file).unwrap();
            writeln!(f, "[incomplete]").unwrap();
            writeln!(f, "aws_access_key_id = AKID").unwrap();
            // No secret_access_key
        }

        let result = read_profile(&cred_file, "incomplete");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn typed_resolution_errors_distinguish_exhaustion_incomplete_and_profile_failures() {
        let empty = input(None, None, None, None, None, None, None, None, None, None);
        assert_eq!(
            resolve_credentials(&empty).await.unwrap_err(),
            CredentialResolutionError::Exhausted
        );

        let incomplete = input(
            Some("CONFIG_ACCESS"),
            None,
            None,
            None,
            Some("ENV_ACCESS"),
            Some("ENV_SECRET"),
            None,
            None,
            None,
            None,
        );
        assert_eq!(
            resolve_credentials(&incomplete).await.unwrap_err(),
            CredentialResolutionError::IncompleteSource {
                credential_source: CredentialSource::ExplicitConfig,
            }
        );

        let dir = tempfile::tempdir().unwrap();
        let unreadable = dir.path().join("profile-directory");
        std::fs::create_dir(&unreadable).unwrap();
        let profile_io = input(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("default"),
            Some(unreadable.as_path()),
        );
        assert_eq!(
            resolve_credentials(&profile_io).await.unwrap_err(),
            CredentialResolutionError::ProfileIo {
                credential_source: CredentialSource::ProfileFile,
            }
        );

        let malformed = dir.path().join("malformed-profile");
        std::fs::write(&malformed, "[default]\naws_access_key_id\n").unwrap();
        let profile_parse = input(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("default"),
            Some(malformed.as_path()),
        );
        assert_eq!(
            resolve_credentials(&profile_parse).await.unwrap_err(),
            CredentialResolutionError::ProfileParse {
                credential_source: CredentialSource::ProfileFile,
            }
        );
    }

    #[tokio::test]
    async fn partial_credentials_profile_wins_over_malformed_lower_config() {
        let dir = tempfile::tempdir().unwrap();
        let credentials_file = dir.path().join("credentials");
        let config_file = dir.path().join("config");
        std::fs::write(
            &credentials_file,
            "[dev]\naws_access_key_id=HIGHER_ACCESS\n",
        )
        .unwrap();
        std::fs::write(&config_file, "[profile dev]\naws_secret_access_key\n").unwrap();
        let mut input = input(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("dev"),
            Some(credentials_file.as_path()),
        );
        input.config_file_path = Some(config_file.as_path());

        assert_eq!(
            resolve_credentials(&input).await.unwrap_err(),
            CredentialResolutionError::IncompleteSource {
                credential_source: CredentialSource::ProfileFile,
            }
        );
    }

    #[tokio::test]
    async fn credential_process_timeout_and_output_limit_are_typed_and_bounded() {
        let _process_lock = CREDENTIAL_PROCESS_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let config_file = dir.path().join("config");
        let timeout_command = if cfg!(windows) {
            "Start-Sleep -Seconds 30".to_owned()
        } else {
            "sleep 30".to_owned()
        };
        std::fs::write(
            &config_file,
            format!("[profile proc]\ncredential_process={timeout_command}\n"),
        )
        .unwrap();
        let mut timeout_input = input(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("proc"),
            None,
        );
        timeout_input.config_file_path = Some(config_file.as_path());
        assert_eq!(
            resolve_credentials(&timeout_input).await.unwrap_err(),
            CredentialResolutionError::CredentialProcessTimeout
        );

        let output_command = if cfg!(windows) {
            "[Console]::Out.Write('x' * 70000)".to_owned()
        } else {
            "head -c 70000 /dev/zero".to_owned()
        };
        std::fs::write(
            &config_file,
            format!("[profile proc]\ncredential_process={output_command}\n"),
        )
        .unwrap();
        assert_eq!(
            resolve_credentials(&timeout_input).await.unwrap_err(),
            CredentialResolutionError::CredentialProcessOutputLimit
        );
    }

    #[tokio::test]
    async fn blank_credential_process_is_a_typed_configured_error() {
        let dir = tempfile::tempdir().unwrap();
        let config_file = dir.path().join("config");
        std::fs::write(&config_file, "[profile proc]\ncredential_process=  \n").unwrap();
        let mut input = input(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("proc"),
            None,
        );
        input.config_file_path = Some(config_file.as_path());

        assert_eq!(
            resolve_credentials(&input).await.unwrap_err(),
            CredentialResolutionError::CredentialProcessCommand
        );
    }

    #[tokio::test]
    async fn credential_process_exit_json_and_version_failures_are_typed() {
        let _process_lock = CREDENTIAL_PROCESS_TEST_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let cases = if cfg!(windows) {
            vec![
                (
                    "exit 9".to_owned(),
                    CredentialResolutionError::CredentialProcessExit,
                ),
                (
                    "[Console]::Out.Write('not-json')".to_owned(),
                    CredentialResolutionError::CredentialProcessJson,
                ),
                (
                    r#"[Console]::Out.Write('{"Version":2,"AccessKeyId":"AKIA_VERSION_CANARY","SecretAccessKey":"version-secret-canary"}')"#.to_owned(),
                    CredentialResolutionError::CredentialProcessVersion,
                ),
            ]
        } else {
            vec![
                (
                    "exit 9".to_owned(),
                    CredentialResolutionError::CredentialProcessExit,
                ),
                (
                    "printf 'not-json'".to_owned(),
                    CredentialResolutionError::CredentialProcessJson,
                ),
                (
                    r#"printf '{"Version":2,"AccessKeyId":"AKIA_VERSION_CANARY","SecretAccessKey":"version-secret-canary"}'"#.to_owned(),
                    CredentialResolutionError::CredentialProcessVersion,
                ),
            ]
        };

        for (index, (command, expected)) in cases.into_iter().enumerate() {
            let config_file = dir.path().join(format!("config-{index}"));
            std::fs::write(
                &config_file,
                format!("[profile proc]\ncredential_process={command}\n"),
            )
            .unwrap();
            let input = CredentialResolutionInput {
                config_file_path: Some(config_file.as_path()),
                profile_name: Some("proc"),
                ..input(None, None, None, None, None, None, None, None, None, None)
            };
            let error = resolve_credentials(&input).await.unwrap_err();
            assert_eq!(error, expected);
            let rendered = format!("{error:?} {error}");
            assert!(!rendered.contains("AKIA_VERSION_CANARY"));
            assert!(!rendered.contains("version-secret-canary"));
        }
    }

    #[tokio::test]
    async fn cancelling_real_credential_process_resolution_terminates_descendants() {
        let _process_lock = CREDENTIAL_PROCESS_TEST_LOCK.lock().await;
        // Cold shell startup on a loaded CI runner can exceed both this
        // test's marker wait and the resolver's own credential-process
        // timeout. Only the startup race is retryable: a marker that never
        // appears because the resolver gave up first is a slow-host artifact,
        // while a surviving descendant after a launched run is a real defect
        // and fails immediately.
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
        let config_file = dir.path().join("config");
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
        std::fs::write(
            &config_file,
            format!("[profile proc]\ncredential_process={command}\n"),
        )
        .unwrap();
        let task_config = config_file.clone();
        let task = tokio::spawn(async move {
            let mut process_input = input(
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some("proc"),
                None,
            );
            process_input.config_file_path = Some(task_config.as_path());
            resolve_credentials(&process_input).await
        });

        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while !started.exists() {
                if task.is_finished() {
                    // The resolver's own timeout cancelled the process
                    // before the shell managed to start it — retry the
                    // attempt on a slow host.
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
            "credential_process descendant survived resolver cancellation"
        );
        TestAttempt::Passed
    }
}
