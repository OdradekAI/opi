//! ADC external-account executable (pluggable) supplier (task 20.21.2).
//!
//! Pinned from `google-auth-library@10.6.2` (`PluggableAuthClient` +
//! `PluggableAuthHandler` + `ExecutableResponse`): the admission
//! environment gate (`GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES=1`) is
//! checked **before** the output-file cache and any spawn; the command
//! string is split into a direct argv (quote-aware, never a shell); a
//! configured `output_file` is consulted first (malformed caches are
//! terminal, stale or unsuccessful caches are misses); stdout and stderr
//! are merged; a response must carry `version`/`success`, a supported
//! token-type URN with its token field, `expiration_time` when an
//! output file is configured, and must not be expired. The spawn reuses
//! the crate's private process owner (Unix process group or Windows
//! kill-on-close Job, cancellation and tree reaping); this owner never rewrites the
//! `output_file`.

use std::path::Path;

use std::process::Stdio;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use crate::auth::{AuthContext, AuthProcessSnapshot};
use crate::provider::ProviderErrorSummary;

use super::native::{AdcClock, AdcResolutionError};

/// Admission gate variable; the value must be exactly `1`.
const ALLOW_EXECUTABLES_ENV: &str = "GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES";

/// Default executable timeout when none is configured (30 s).
pub const DEFAULT_EXECUTABLE_TIMEOUT_MILLIS: u64 = 30_000;

/// Minimum allowed executable timeout (5 s).
pub const MINIMUM_EXECUTABLE_TIMEOUT_MILLIS: u64 = 5_000;

/// Maximum allowed executable timeout (120 s).
pub const MAXIMUM_EXECUTABLE_TIMEOUT_MILLIS: u64 = 120_000;

/// Maximum supported executable response version.
const MAXIMUM_EXECUTABLE_VERSION: i64 = 1;

/// Admitted token-type URNs with their token field.
const SAML_TOKEN_TYPE: &str = "urn:ietf:params:oauth:token-type:saml2";
const OIDC_TOKEN_TYPES: [&str; 2] = [
    "urn:ietf:params:oauth:token-type:id_token",
    "urn:ietf:params:oauth:token-type:jwt",
];

/// Environment variables handed to the executable.
const ENV_AUDIENCE: &str = "GOOGLE_EXTERNAL_ACCOUNT_AUDIENCE";
const ENV_TOKEN_TYPE: &str = "GOOGLE_EXTERNAL_ACCOUNT_TOKEN_TYPE";
const ENV_INTERACTIVE: &str = "GOOGLE_EXTERNAL_ACCOUNT_INTERACTIVE";
const ENV_OUTPUT_FILE: &str = "GOOGLE_EXTERNAL_ACCOUNT_OUTPUT_FILE";
const ENV_IMPERSONATED_EMAIL: &str = "GOOGLE_EXTERNAL_ACCOUNT_IMPERSONATED_EMAIL";

/// Parse the executable command into a direct argv: split on spaces outside
/// double quotes; a fully-quoted argument loses its outer quotes, mixed
/// unquoted/quoted segments concatenate with inner quotes preserved.
pub fn parse_command(command: &str) -> Option<Vec<String>> {
    let mut components: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut started = false;
    for ch in command.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                started = true;
                current.push('"');
            }
            ch if ch.is_whitespace() && !in_quotes => {
                if started {
                    components.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            ch => {
                started = true;
                current.push(ch);
            }
        }
    }
    if started {
        components.push(current);
    }
    if components.is_empty() {
        return None;
    }
    Some(
        components
            .into_iter()
            .map(|component| {
                if component.len() >= 2 && component.starts_with('"') && component.ends_with('"') {
                    component[1..component.len() - 1].to_owned()
                } else {
                    component
                }
            })
            .collect(),
    )
}

/// One validated executable response (cache or spawned).
#[derive(Debug, Clone)]
struct ExecutableResponse {
    version: i64,
    success: bool,
    expiration_time: Option<i64>,
    subject_token: Option<String>,
    error_code: Option<String>,
    error_message: Option<String>,
}

fn invalid(reason: &'static str) -> AdcResolutionError {
    AdcResolutionError::ExecutableResponseInvalid { reason }
}

impl ExecutableResponse {
    fn parse(raw: &str) -> Result<Self, AdcResolutionError> {
        let json: serde_json::Value =
            serde_json::from_str(raw).map_err(|_| invalid("the response is not valid JSON"))?;
        let version = json
            .get("version")
            .and_then(|value| value.as_i64())
            .filter(|version| *version > 0)
            .ok_or(invalid("the response must contain a version field"))?;
        let success = json
            .get("success")
            .and_then(|value| value.as_bool())
            .ok_or(invalid("the response must contain a success field"))?;
        let expiration_time = json.get("expiration_time").and_then(|value| value.as_i64());
        let (subject_token, error_code, error_message) = if success {
            let token_type = json
                .get("token_type")
                .and_then(|value| value.as_str())
                .ok_or(invalid(
                    "the response must contain a token_type field when successful",
                ))?;
            let subject_token = if token_type == SAML_TOKEN_TYPE {
                json.get("saml_response")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                    .ok_or(invalid(
                        "the response must contain a saml_response field for its token_type",
                    ))?
            } else if OIDC_TOKEN_TYPES.contains(&token_type) {
                json.get("id_token")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                    .ok_or(invalid(
                        "the response must contain an id_token field for its token_type",
                    ))?
            } else {
                return Err(invalid("the response token_type is not admitted"));
            };
            (Some(subject_token.to_owned()), None, None)
        } else {
            let code = json.get("code").and_then(|value| value.as_str());
            let message = json.get("message").and_then(|value| value.as_str());
            if code.is_none() || message.is_none() {
                return Err(invalid(
                    "unsuccessful responses must contain both code and message",
                ));
            }
            (None, code.map(str::to_owned), message.map(str::to_owned))
        };
        Ok(Self {
            version,
            success,
            expiration_time,
            subject_token,
            error_code,
            error_message,
        })
    }

    fn is_expired(&self, clock: &dyn AdcClock) -> bool {
        match self.expiration_time {
            Some(expiration) => expiration < clock.now().unix_timestamp(),
            None => false,
        }
    }

    /// Cache-validity: successful and not expired.
    fn is_valid_cache(&self, clock: &dyn AdcClock) -> bool {
        self.success && !self.is_expired(clock)
    }
}

/// Read the output-file cache. `Ok(None)` is a miss (absent, empty, stale,
/// or unsuccessful); a malformed cache is a typed terminal error.
async fn cached_response(
    output_file: &str,
    clock: &dyn AdcClock,
) -> Result<Option<ExecutableResponse>, AdcResolutionError> {
    let path = Path::new(output_file);
    let Ok(meta) = tokio::fs::metadata(path).await else {
        return Ok(None);
    };
    if !meta.is_file() {
        return Ok(None);
    }
    let raw = tokio::fs::read_to_string(path)
        .await
        .map_err(|_| invalid("the output file could not be read"))?;
    if raw.is_empty() {
        return Ok(None);
    }
    let response = ExecutableResponse::parse(&raw)?;
    if response.is_valid_cache(clock) {
        Ok(Some(response))
    } else {
        Ok(None)
    }
}

/// Collect one merged OS pipe under the source timeout. A detached owner
/// performs cancellation cleanup while the host runtime remains alive.
async fn run_executable(
    argv: &[String],
    snapshot: AuthProcessSnapshot,
    timeout_millis: u64,
) -> Result<String, AdcResolutionError> {
    let argv = argv.to_vec();
    crate::auth::aws::process::own_process(move |cancellation| async move {
        run_executable_owned(&argv, snapshot, timeout_millis, cancellation).await
    })
    .await
    .map_err(|_| AdcResolutionError::ExecutableSpawnFailed)?
}

async fn run_executable_owned(
    argv: &[String],
    snapshot: AuthProcessSnapshot,
    timeout_millis: u64,
    cancellation: CancellationToken,
) -> Result<String, AdcResolutionError> {
    if cancellation.is_cancelled() {
        return Err(AdcResolutionError::ExecutableCancelled);
    }
    let (mut reader, stdout) =
        std::io::pipe().map_err(|_| AdcResolutionError::ExecutableSpawnFailed)?;
    let stderr = stdout
        .try_clone()
        .map_err(|_| AdcResolutionError::ExecutableSpawnFailed)?;
    let mut command = Command::new(&argv[0]);
    command
        .args(&argv[1..])
        .env_clear()
        .envs(snapshot.env)
        .current_dir(snapshot.cwd);
    let mut tree = crate::auth::aws::process::ProcessTree::prepare(&mut command)
        .map_err(|_| AdcResolutionError::ExecutableSpawnFailed)?;
    command
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|_| AdcResolutionError::ExecutableSpawnFailed)?;
    let root_pid = child.id();
    if tree.attach_and_resume(&child).is_err() {
        crate::auth::aws::process::terminate_tree_and_reap(&mut child, root_pid, &mut tree).await;
        return Err(AdcResolutionError::ExecutableSpawnFailed);
    }
    // Command retains its Stdio configuration after spawn; close its copies
    // so only the child tree owns the write ends and EOF is observable.
    drop(command);
    let mut read_job = tokio::task::spawn_blocking(move || {
        use std::io::Read;
        let mut output = Vec::new();
        reader.read_to_end(&mut output).map(|_| output)
    });
    let mut read_consumed = false;
    let collect = async {
        let read = (&mut read_job).await;
        read_consumed = true;
        let output = read
            .map_err(|_| AdcResolutionError::ExecutableOutput)?
            .map_err(|_| AdcResolutionError::ExecutableOutput)?;
        let status = child
            .wait()
            .await
            .map_err(|_| AdcResolutionError::ExecutableSpawnFailed)?;
        Ok::<_, AdcResolutionError>((output, status))
    };
    let result = tokio::select! {
        biased;
        _ = cancellation.cancelled() => Err(AdcResolutionError::ExecutableCancelled),
        result = tokio::time::timeout(std::time::Duration::from_millis(timeout_millis), collect) => {
            result.unwrap_or(Err(AdcResolutionError::ExecutableTimeout))
        }
    };
    match result {
        Ok((output, status)) => {
            if !status.success() {
                let code = status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".to_owned());
                return Err(AdcResolutionError::ExecutableFailed {
                    code: ProviderErrorSummary::sanitized(code),
                    message: ProviderErrorSummary::from_untrusted(String::from_utf8_lossy(&output)),
                });
            }
            Ok(String::from_utf8_lossy(&output).into_owned())
        }
        Err(error) => {
            crate::auth::aws::process::terminate_tree_and_reap(&mut child, root_pid, &mut tree)
                .await;
            if !read_consumed {
                let _ = read_job.await;
            }
            Err(error)
        }
    }
}

/// Retrieve the subject token from the executable supplier.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn retrieve_executable_subject_token(
    command: &str,
    timeout_millis: u64,
    output_file: Option<&str>,
    audience: &str,
    subject_token_type: &str,
    impersonated_email: Option<&str>,
    ambient: &dyn AuthContext,
    clock: &dyn AdcClock,
) -> Result<String, AdcResolutionError> {
    // Admission gate first: before the cache and any spawn.
    let allowed = matches!(
        ambient.env(ALLOW_EXECUTABLES_ENV),
        Ok(Some(value)) if value == "1"
    );
    if !allowed {
        return Err(AdcResolutionError::ExecutablesNotAllowed);
    }
    let response = match output_file {
        Some(output_file) => match cached_response(output_file, clock).await? {
            Some(response) => response,
            None => {
                spawn_and_validate(
                    command,
                    timeout_millis,
                    output_file,
                    audience,
                    subject_token_type,
                    impersonated_email,
                    ambient,
                )
                .await?
            }
        },
        None => {
            spawn_and_validate(
                command,
                timeout_millis,
                "",
                audience,
                subject_token_type,
                impersonated_email,
                ambient,
            )
            .await?
        }
    };

    if response.version > MAXIMUM_EXECUTABLE_VERSION {
        return Err(invalid("the executable response version is not supported"));
    }
    if !response.success {
        return Err(AdcResolutionError::ExecutableFailed {
            code: ProviderErrorSummary::from_untrusted(response.error_code.unwrap_or_default()),
            message: ProviderErrorSummary::from_untrusted(
                response.error_message.unwrap_or_default(),
            ),
        });
    }
    if output_file.is_some() && response.expiration_time.is_none() {
        return Err(invalid(
            "the response must contain expiration_time when an output_file is configured",
        ));
    }
    if response.is_expired(clock) {
        return Err(AdcResolutionError::ExecutableResponseExpired);
    }
    response
        .subject_token
        .ok_or(invalid("the response carried no subject token"))
}

#[allow(clippy::too_many_arguments)]
async fn spawn_and_validate(
    command: &str,
    timeout_millis: u64,
    output_file: &str,
    audience: &str,
    subject_token_type: &str,
    impersonated_email: Option<&str>,
    ambient: &dyn AuthContext,
) -> Result<ExecutableResponse, AdcResolutionError> {
    let argv = parse_command(command).ok_or(AdcResolutionError::CredentialSourceInvalid {
        reason: "the executable command could not be parsed",
    })?;
    let mut snapshot = ambient
        .process_snapshot()
        .map_err(|_| AdcResolutionError::ExecutableProcessContext)?;
    let mut env = vec![
        (ENV_AUDIENCE.to_owned(), audience.to_owned()),
        (ENV_TOKEN_TYPE.to_owned(), subject_token_type.to_owned()),
        (ENV_INTERACTIVE.to_owned(), "0".to_owned()),
    ];
    if !output_file.is_empty() {
        env.push((ENV_OUTPUT_FILE.to_owned(), output_file.to_owned()));
    }
    if let Some(email) = impersonated_email {
        env.push((ENV_IMPERSONATED_EMAIL.to_owned(), email.to_owned()));
    }
    for (name, value) in env {
        // Windows environment names are case-insensitive. An inherited
        // spelling must not override a selected protocol variable merely
        // because the owned map has a different key ordering.
        #[cfg(windows)]
        snapshot
            .env
            .retain(|existing, _| !existing.eq_ignore_ascii_case(&name));
        snapshot.env.insert(name, value);
    }
    let output = run_executable(&argv, snapshot, timeout_millis).await?;
    ExecutableResponse::parse(&output)
}

/// Extract the impersonated service-account email from a
/// `service_account_impersonation_url`
/// (`.../serviceAccounts/{email}:generateAccessToken`).
pub(crate) fn impersonated_email_from_url(url: &str) -> Option<String> {
    let marker = "serviceAccounts/";
    let start = url.find(marker)? + marker.len();
    let tail = &url[start..];
    let end = tail.find(":generateAccessToken")?;
    let email = &tail[..end];
    if email.is_empty() {
        None
    } else {
        Some(email.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_command_matches_the_source_semantics() {
        assert_eq!(
            parse_command("helper --flag \"quoted value\" tail").unwrap(),
            vec!["helper", "--flag", "quoted value", "tail"]
        );
        assert_eq!(parse_command("a\"b c\"d").unwrap(), vec!["a\"b c\"d"]);
        assert_eq!(parse_command("\"x y\" z").unwrap(), vec!["x y", "z"]);
        assert!(parse_command("   ").is_none());
    }

    #[test]
    fn impersonated_email_extracts_from_the_generate_url() {
        assert_eq!(
            impersonated_email_from_url(
                "https://iamcredentials.googleapis.com/v1/projects/-/serviceAccounts/sa@p.iam.gserviceaccount.com:generateAccessToken"
            )
            .as_deref(),
            Some("sa@p.iam.gserviceaccount.com")
        );
        assert_eq!(impersonated_email_from_url("https://example.com/"), None);
    }
}
