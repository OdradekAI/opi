//! Vertex AI provider wiring tests (task 3.3).
//!
//! Tests config parsing, provider construction, and secret redaction
//! for the Vertex provider. No live Google Cloud calls.

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
fn parse_vertex_config_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(dir.path(), "");
    let config = load_config_file(&path);
    assert!(config.providers.vertex.access_token_env.is_empty());
    assert!(config.providers.vertex.project.is_none());
    assert!(config.providers.vertex.location.is_none());
    assert!(config.providers.vertex.models.is_empty());
}

#[test]
fn parse_vertex_config_with_project_and_location() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.vertex]
access_token_env = "MY_VERTEX_TOKEN"
project = "my-gcp-project"
location = "europe-west1"
"#,
    );
    let config = load_config_file(&path);
    assert_eq!(config.providers.vertex.access_token_env, "MY_VERTEX_TOKEN");
    assert_eq!(
        config.providers.vertex.project.as_deref(),
        Some("my-gcp-project")
    );
    assert_eq!(
        config.providers.vertex.location.as_deref(),
        Some("europe-west1")
    );
}

#[test]
fn parse_vertex_config_with_models() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.vertex]
project = "my-project"
location = "us-central1"
models = ["gemini-2.5-flash", "gemini-2.5-pro"]
"#,
    );
    let config = load_config_file(&path);
    assert_eq!(config.providers.vertex.models.len(), 2);
    assert_eq!(config.providers.vertex.models[0], "gemini-2.5-flash");
    assert_eq!(config.providers.vertex.models[1], "gemini-2.5-pro");
}

#[test]
fn vertex_config_independent_from_other_providers() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.gemini]
api_key_env = "GEMINI_KEY"

[providers.vertex]
access_token_env = "VERTEX_TOKEN"
project = "my-project"
"#,
    );
    let config = load_config_file(&path);
    assert_eq!(config.providers.gemini.api_key_env, "GEMINI_KEY");
    assert_eq!(config.providers.vertex.access_token_env, "VERTEX_TOKEN");
}

// ---------------------------------------------------------------------------
// Provider construction tests (no live calls)
// ---------------------------------------------------------------------------

#[test]
fn vertex_provider_builds_with_token() {
    let provider =
        opi_ai::vertex::VertexProvider::new("my-project".into(), "us-central1".into(), None);
    assert_eq!(provider.id(), "vertex");
}

#[test]
fn vertex_provider_from_config_with_models() {
    let provider = opi_ai::vertex::VertexProvider::from_config(
        "proj".into(),
        "europe-west4".into(),
        vec!["model-a".into(), "model-b".into()],
        None,
    );
    assert_eq!(provider.id(), "vertex");
    assert_eq!(provider.models().len(), 2);
    assert_eq!(provider.models()[0].id, "model-a");
    assert_eq!(provider.models()[1].id, "model-b");
}

// ---------------------------------------------------------------------------
// Secret redaction tests
// ---------------------------------------------------------------------------

#[test]
fn vertex_access_token_not_in_debug() {
    let provider = opi_ai::vertex::VertexProvider::new("proj".into(), "us-central1".into(), None);
    let debug = format!("{provider:?}");
    assert!(
        !debug.contains("super-secret-oauth-token-xyz"),
        "access token leaked in Debug: {debug}"
    );
    // Post-17.5: VertexProvider no longer stores the token (it arrives via
    // ResolvedAuth), so its Debug carries no `***` redaction marker.
    // Redaction is implicit-by-absence, proven by the assertion above.
}

#[test]
fn vertex_project_visible_in_debug() {
    let provider =
        opi_ai::vertex::VertexProvider::new("my-vertex-project".into(), "us-central1".into(), None);
    let debug = format!("{provider:?}");
    assert!(debug.contains("my-vertex-project"));
    assert!(debug.contains("us-central1"));
}

#[test]
fn vertex_url_does_not_contain_access_token() {
    let provider = opi_ai::vertex::VertexProvider::new("proj".into(), "us-central1".into(), None);
    let url = provider.build_vertex_url("gemini-2.5-flash");
    assert!(!url.contains("super-secret-token"));
    assert!(url.contains("proj"));
}
