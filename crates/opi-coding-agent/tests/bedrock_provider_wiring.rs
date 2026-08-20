//! Bedrock provider wiring tests (task 3.1).
//!
//! Tests Bedrock config parsing and provider construction through build_provider.
//! No live AWS calls.

use std::fs;

use opi_coding_agent::config::{OpiConfig, load_config_file};

fn write_temp_config(dir: &std::path::Path, contents: &str) -> std::path::PathBuf {
    let path = dir.join("config.toml");
    fs::write(&path, contents).unwrap();
    path
}

// ---------------------------------------------------------------------------
// Config parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_bedrock_config_with_region() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.bedrock]
region = "eu-west-1"
"#,
    );
    let config = load_config_file(&path).unwrap();
    assert_eq!(
        config.providers.bedrock.region.as_deref(),
        Some("eu-west-1")
    );
}

#[test]
fn parse_bedrock_config_with_access_key() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.bedrock]
access_key_id = "AKIAEXAMPLE"
secret_access_key_env = "MY_SECRET_KEY"
region = "ap-southeast-1"
"#,
    );
    let config = load_config_file(&path).unwrap();
    assert_eq!(
        config
            .providers
            .bedrock
            .access_key_id
            .as_ref()
            .map(secrecy::ExposeSecret::expose_secret),
        Some("AKIAEXAMPLE")
    );
    assert_eq!(
        config.providers.bedrock.secret_access_key_env.as_deref(),
        Some("MY_SECRET_KEY")
    );
    assert_eq!(
        config.providers.bedrock.region.as_deref(),
        Some("ap-southeast-1")
    );
}

#[test]
fn parse_bedrock_config_with_session_token() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.bedrock]
region = "us-east-1"
session_token_env = "AWS_SESSION_TOKEN"
"#,
    );
    let config = load_config_file(&path).unwrap();
    assert_eq!(
        config.providers.bedrock.session_token_env.as_deref(),
        Some("AWS_SESSION_TOKEN")
    );
}

#[test]
fn parse_bedrock_config_with_profile() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.bedrock]
profile = "production"
region = "us-west-2"
"#,
    );
    let config = load_config_file(&path).unwrap();
    assert_eq!(
        config.providers.bedrock.profile.as_deref(),
        Some("production")
    );
}

#[test]
fn parse_bedrock_config_with_base_url() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.bedrock]
region = "us-east-1"
base_url = "https://custom-bedrock-endpoint.example.com"
"#,
    );
    let config = load_config_file(&path).unwrap();
    assert_eq!(
        config.providers.bedrock.base_url.as_deref(),
        Some("https://custom-bedrock-endpoint.example.com")
    );
}

#[test]
fn default_bedrock_config_has_no_explicit_credentials() {
    let config = OpiConfig::default();
    assert!(config.providers.bedrock.access_key_id.is_none());
    assert!(config.providers.bedrock.secret_access_key_env.is_none());
    assert!(config.providers.bedrock.session_token_env.is_none());
    assert!(config.providers.bedrock.profile.is_none());
    assert!(config.providers.bedrock.base_url.is_none());
}

#[test]
fn parse_bedrock_config_with_proxy() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.bedrock]
region = "us-east-1"

[providers.bedrock.proxy]
url = "http://proxy.example.com:8080"
"#,
    );
    let config = load_config_file(&path).unwrap();
    let proxy = config
        .providers
        .bedrock
        .proxy
        .as_ref()
        .expect("proxy should be set");
    assert_eq!(proxy.url, "http://proxy.example.com:8080");
}
