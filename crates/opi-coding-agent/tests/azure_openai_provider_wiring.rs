//! Azure OpenAI provider wiring tests (task 3.2).
//!
//! Tests config parsing, provider construction, and secret redaction
//! for the Azure provider. No live Azure calls.

use opi_ai::Provider;
use std::fs;

fn write_temp_config(dir: &std::path::Path, contents: &str) -> std::path::PathBuf {
    let path = dir.join("config.toml");
    fs::write(&path, contents).unwrap();
    path
}

fn load_config_file(path: &std::path::Path) -> opi_coding_agent::config::OpiConfig {
    opi_coding_agent::config::load_config_file(path).unwrap()
}

// ---------------------------------------------------------------------------
// Config parsing tests
// ---------------------------------------------------------------------------

#[test]
fn parse_azure_config_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(dir.path(), "");
    let config = load_config_file(&path);
    // Default api_key_env should be empty (no default like other providers)
    assert!(config.providers.azure.api_key_env.is_empty());
    assert!(config.providers.azure.endpoint.is_none());
    assert!(config.providers.azure.api_version.is_none());
    assert!(config.providers.azure.deployments.is_empty());
}

#[test]
fn parse_azure_config_with_endpoint() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.azure]
api_key_env = "MY_AZURE_KEY"
endpoint = "https://myresource.openai.azure.com"
api_version = "2024-08-01-preview"
"#,
    );
    let config = load_config_file(&path);
    assert_eq!(config.providers.azure.api_key_env, "MY_AZURE_KEY");
    assert_eq!(
        config.providers.azure.endpoint.as_deref(),
        Some("https://myresource.openai.azure.com")
    );
    assert_eq!(
        config.providers.azure.api_version.as_deref(),
        Some("2024-08-01-preview")
    );
}

#[test]
fn parse_azure_config_with_deployments() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.azure]
endpoint = "https://myresource.openai.azure.com"
deployments = ["gpt4o-prod", "gpt4o-mini-prod"]
"#,
    );
    let config = load_config_file(&path);
    assert_eq!(config.providers.azure.deployments.len(), 2);
    assert_eq!(config.providers.azure.deployments[0], "gpt4o-prod");
    assert_eq!(config.providers.azure.deployments[1], "gpt4o-mini-prod");
}

#[test]
fn azure_config_independent_from_other_providers() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.anthropic]
api_key_env = "ANTHROPIC_KEY"

[providers.azure]
api_key_env = "AZURE_KEY"
endpoint = "https://myresource.openai.azure.com"
"#,
    );
    let config = load_config_file(&path);
    assert_eq!(config.providers.anthropic.api_key_env, "ANTHROPIC_KEY");
    assert_eq!(config.providers.azure.api_key_env, "AZURE_KEY");
}

// ---------------------------------------------------------------------------
// Provider construction tests (no live calls)
// ---------------------------------------------------------------------------

#[test]
fn azure_provider_builds_with_key() {
    let provider = opi_ai::azure_openai::AzureOpenAIProvider::new(
        Some("https://myresource.openai.azure.com".into()),
        "my-deploy".into(),
        Some("2024-06-01".into()),
    )
    .unwrap();
    assert_eq!(provider.id(), "azure");
}

#[test]
fn azure_provider_from_config_with_deployments() {
    let provider = opi_ai::azure_openai::AzureOpenAIProvider::from_config(
        Some("https://myresource.openai.azure.com".into()),
        vec!["deploy1".into(), "deploy2".into()],
        None,
    )
    .unwrap();
    assert_eq!(provider.id(), "azure");
    assert_eq!(provider.models().len(), 2);
    assert_eq!(provider.models()[0].id, "deploy1");
    assert_eq!(provider.models()[1].id, "deploy2");
}

// ---------------------------------------------------------------------------
// Secret redaction tests
// ---------------------------------------------------------------------------

#[test]
fn azure_api_key_not_in_debug() {
    use opi_ai::auth::{AuthProvenance, AuthScheme, ResolvedAuth};
    use secrecy::SecretString;

    // Phase 17.5 moved auth off the provider object onto the collection-owned
    // route, so {provider:?} no longer carries any secret field. The redaction
    // surface is now ResolvedAuth, whose Debug redacts the secret.
    let provider = opi_ai::azure_openai::AzureOpenAIProvider::new(
        Some("https://myresource.openai.azure.com".into()),
        "my-deploy".into(),
        None,
    )
    .unwrap();
    let provider_debug = format!("{provider:?}");
    assert!(
        !provider_debug.contains("super-secret-key-12345"),
        "API key leaked in provider Debug: {provider_debug}"
    );

    let auth = ResolvedAuth {
        scheme: AuthScheme::ApiKey,
        secret: SecretString::from("super-secret-key-12345"),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    };
    let auth_debug = format!("{auth:?}");
    assert!(
        !auth_debug.contains("super-secret-key-12345"),
        "API key leaked in ResolvedAuth Debug: {auth_debug}"
    );
    assert!(
        auth_debug.contains("<redacted>"),
        "ResolvedAuth Debug must redact the secret: {auth_debug}"
    );
}

#[test]
fn azure_config_endpoint_visible_in_debug() {
    let provider = opi_ai::azure_openai::AzureOpenAIProvider::new(
        Some("https://myresource.openai.azure.com".into()),
        "my-deploy".into(),
        None,
    )
    .unwrap();
    let debug = format!("{provider:?}");
    assert!(debug.contains("myresource.openai.azure.com"));
}

#[test]
fn azure_url_does_not_contain_api_key() {
    let provider = opi_ai::azure_openai::AzureOpenAIProvider::new(
        Some("https://myresource.openai.azure.com".into()),
        "my-deploy".into(),
        None,
    )
    .unwrap();
    let url = provider.build_azure_url("my-deploy");
    assert!(!url.contains("super-secret-key"));
    assert!(url.contains("my-deploy"));
}
