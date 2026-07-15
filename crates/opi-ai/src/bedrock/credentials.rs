//! Bedrock credential resolution (task 3.1).
//!
//! Precedence: explicit config > env vars > shared AWS profile files.
//! No live AWS calls.

use std::path::Path;
use std::process::Command;

/// Resolved AWS credentials for Bedrock.
///
/// Custom Debug redacts secret_access_key and session_token.
#[derive(Clone)]
pub struct BedrockCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
    pub region: String,
}

impl std::fmt::Debug for BedrockCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BedrockCredentials")
            .field("access_key_id", &self.access_key_id)
            .field("secret_access_key", &"***")
            .field("session_token", &self.session_token.as_ref().map(|_| "***"))
            .field("region", &self.region)
            .finish()
    }
}

/// Source of resolved credentials (for diagnostics, never logged with secrets).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialSource {
    ExplicitConfig,
    Environment,
    ProfileFile,
    ConfigFile,
    CredentialProcess,
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
/// Returns `None` when no credentials are found.
pub fn resolve_credentials(
    input: &CredentialResolutionInput<'_>,
) -> Option<(BedrockCredentials, CredentialSource)> {
    // 1. Explicit config takes highest precedence
    if let (Some(akid), Some(sak)) = (
        non_blank(input.config_access_key_id),
        non_blank(input.config_secret_access_key),
    ) {
        return Some((
            BedrockCredentials {
                access_key_id: akid.to_string(),
                secret_access_key: sak.to_string(),
                session_token: input
                    .config_session_token
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.to_string()),
                region: input
                    .config_region
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or("us-east-1")
                    .to_string(),
            },
            CredentialSource::ExplicitConfig,
        ));
    }

    // 2. Environment variables
    if let (Some(akid), Some(sak)) = (
        non_blank(input.env_access_key_id),
        non_blank(input.env_secret_access_key),
    ) {
        return Some((
            BedrockCredentials {
                access_key_id: akid.to_string(),
                secret_access_key: sak.to_string(),
                session_token: input
                    .env_session_token
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.to_string()),
                region: input
                    .env_region
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or("us-east-1")
                    .to_string(),
            },
            CredentialSource::Environment,
        ));
    }

    // 3. Shared AWS credentials/config profiles. We intentionally keep this
    // local-only: no IMDS, SSO browser/device flow, or web-identity network
    // calls are attempted from opi.
    let profile = input
        .profile_name
        .filter(|profile| !profile.trim().is_empty())
        .unwrap_or("default");

    let credentials_props = input
        .credentials_file_path
        .filter(|path| path.exists())
        .and_then(|path| read_profile_properties(path, profile, ProfileFileKind::Credentials));
    let config_props = input
        .config_file_path
        .filter(|path| path.exists())
        .and_then(|path| read_profile_properties(path, profile, ProfileFileKind::Config));

    if credentials_props.is_some() || config_props.is_some() {
        let merged = merge_profile_properties(credentials_props, config_props);
        let region = first_non_empty(&[
            input.config_region,
            input.env_region,
            merged.region.as_deref(),
        ])
        .unwrap_or("us-east-1")
        .to_string();

        if let Some(creds) = profile_properties_to_credentials(&merged, region.clone()) {
            let source = if merged.static_source == Some(ProfileFileKind::Config) {
                CredentialSource::ConfigFile
            } else {
                CredentialSource::ProfileFile
            };
            return Some((creds, source));
        }

        if let Some(command) = merged.credential_process.as_deref()
            && let Some(creds) = run_credential_process(command, region)
        {
            return Some((creds, CredentialSource::CredentialProcess));
        }
    }

    None
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

#[derive(Debug, Clone, Default)]
struct ProfileProperties {
    access_key_id: Option<String>,
    secret_access_key: Option<String>,
    session_token: Option<String>,
    region: Option<String>,
    credential_process: Option<String>,
    static_source: Option<ProfileFileKind>,
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

fn merge_profile_properties(
    credentials: Option<ProfileProperties>,
    config: Option<ProfileProperties>,
) -> ProfileProperties {
    let mut merged = ProfileProperties::default();
    for props in [credentials, config].into_iter().flatten() {
        if merged.access_key_id.is_none() {
            merged.access_key_id = props.access_key_id;
        }
        if merged.secret_access_key.is_none() {
            merged.secret_access_key = props.secret_access_key;
        }
        if merged.session_token.is_none() {
            merged.session_token = props.session_token;
        }
        if merged.region.is_none() {
            merged.region = props.region;
        }
        if merged.credential_process.is_none() {
            merged.credential_process = props.credential_process;
        }
        if merged.static_source.is_none() {
            merged.static_source = props.static_source;
        }
    }
    merged
}

fn profile_properties_to_credentials(
    props: &ProfileProperties,
    region: String,
) -> Option<BedrockCredentials> {
    match (&props.access_key_id, &props.secret_access_key) {
        (Some(a), Some(s)) if !a.trim().is_empty() && !s.trim().is_empty() => {
            Some(BedrockCredentials {
                access_key_id: a.clone(),
                secret_access_key: s.clone(),
                session_token: props
                    .session_token
                    .clone()
                    .filter(|token| !token.trim().is_empty()),
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
        if in_target && let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim().to_string();
            match key {
                "aws_access_key_id" => {
                    props.access_key_id = Some(value);
                    props.static_source = Some(kind);
                }
                "aws_secret_access_key" => {
                    props.secret_access_key = Some(value);
                    props.static_source = Some(kind);
                }
                "aws_session_token" => props.session_token = Some(value),
                "region" => props.region = Some(value),
                "credential_process" => props.credential_process = Some(value),
                _ => {}
            }
        }
    }

    (props.access_key_id.is_some()
        || props.secret_access_key.is_some()
        || props.session_token.is_some()
        || props.region.is_some()
        || props.credential_process.is_some())
    .then_some(props)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CredentialProcessOutput {
    version: u32,
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
}

fn run_credential_process(command: &str, region: String) -> Option<BedrockCredentials> {
    let output = if cfg!(windows) {
        Command::new("powershell")
            .args(["-NoProfile", "-Command", command])
            .output()
            .ok()?
    } else {
        Command::new("sh").args(["-c", command]).output().ok()?
    };
    if !output.status.success() {
        return None;
    }
    let parsed: CredentialProcessOutput = serde_json::from_slice(&output.stdout).ok()?;
    if parsed.version != 1
        || parsed.access_key_id.trim().is_empty()
        || parsed.secret_access_key.trim().is_empty()
    {
        return None;
    }
    Some(BedrockCredentials {
        access_key_id: parsed.access_key_id,
        secret_access_key: parsed.secret_access_key,
        session_token: parsed
            .session_token
            .filter(|token| !token.trim().is_empty()),
        region,
    })
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
    use std::io::Write as IoWrite;

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

    #[test]
    fn explicit_config_takes_precedence() {
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
        let (result, source) = resolve_credentials(&inp).unwrap();
        assert_eq!(result.access_key_id, "CONFIG_AKID");
        assert_eq!(result.secret_access_key, "CONFIG_SAK");
        assert_eq!(result.session_token.as_deref(), Some("CONFIG_TOKEN"));
        assert_eq!(result.region, "eu-west-1");
        assert_eq!(source, CredentialSource::ExplicitConfig);
    }

    #[test]
    fn env_vars_when_no_config() {
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
        let (result, source) = resolve_credentials(&inp).unwrap();
        assert_eq!(result.access_key_id, "ENV_AKID");
        assert_eq!(result.secret_access_key, "ENV_SAK");
        assert_eq!(source, CredentialSource::Environment);
    }

    #[test]
    fn profile_file_when_no_config_or_env() {
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
        let (result, source) = resolve_credentials(&inp).unwrap();
        assert_eq!(result.access_key_id, "PROFILE_AKID");
        assert_eq!(result.secret_access_key, "PROFILE_SAK");
        assert_eq!(result.region, "us-west-2");
        assert_eq!(source, CredentialSource::ProfileFile);
    }

    #[test]
    fn default_profile_file_used_when_profile_not_explicit() {
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
        let (result, source) = resolve_credentials(&inp).unwrap();
        assert_eq!(result.access_key_id, "DEFAULT_AKID");
        assert_eq!(source, CredentialSource::ProfileFile);
    }

    #[test]
    fn shared_config_profile_region_is_used_with_credentials_file() {
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
        let (result, source) = resolve_credentials(&inp).unwrap();
        assert_eq!(result.access_key_id, "DEV_AKID");
        assert_eq!(result.region, "ap-northeast-1");
        assert_eq!(source, CredentialSource::ProfileFile);
    }

    #[test]
    fn shared_config_static_credentials_are_supported() {
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
        let (result, source) = resolve_credentials(&inp).unwrap();
        assert_eq!(result.access_key_id, "CONFIG_AKID");
        assert_eq!(result.region, "eu-central-1");
        assert_eq!(source, CredentialSource::ConfigFile);
    }

    #[test]
    fn credential_process_from_shared_config_is_supported() {
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
        let (result, source) = resolve_credentials(&inp).unwrap();
        assert_eq!(result.access_key_id, "PROC_AKID");
        assert_eq!(result.session_token.as_deref(), Some("PROC_TOKEN"));
        assert_eq!(result.region, "us-west-1");
        assert_eq!(source, CredentialSource::CredentialProcess);
    }

    #[test]
    fn none_when_no_credentials_available() {
        let inp = input(None, None, None, None, None, None, None, None, None, None);
        assert!(resolve_credentials(&inp).is_none());
    }

    #[test]
    fn empty_config_values_fall_through() {
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
        let (result, source) = resolve_credentials(&inp).unwrap();
        assert_eq!(result.access_key_id, "ENV_AKID");
        assert_eq!(source, CredentialSource::Environment);
    }

    #[test]
    fn whitespace_only_static_credential_pairs_are_absent() {
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
                resolve_credentials(&inp).is_none(),
                "whitespace-only access/secret values must be absent"
            );
        }
    }

    #[test]
    fn whitespace_only_optional_values_are_absent() {
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
        let (credentials, source) = resolve_credentials(&inp).expect("valid configured pair");
        assert_eq!(source, CredentialSource::ExplicitConfig);
        assert_eq!(credentials.session_token, None);
        assert_eq!(credentials.region, "us-east-1");
    }

    #[test]
    fn default_region_when_not_specified() {
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
        let (result, _) = resolve_credentials(&inp).unwrap();
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
        assert_eq!(creds.access_key_id, "AKID");
        assert_eq!(creds.session_token.as_deref(), Some("TOKEN"));
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
}
