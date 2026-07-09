//! Config tests: TOML loading, precedence, proxy parsing, and keybindings.
//!
//! Merged from config_loading + config_precedence + proxy_config +
//! keybindings_config to cut integration-binary count (Candidate 4).

use std::fs;
use std::path::{Path, PathBuf};

use opi_coding_agent::config::{ConfigSource, OpiConfig, load_config_file, resolve_config};
use tempfile::NamedTempFile;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn write_temp_config(dir: &Path, contents: &str) -> std::path::PathBuf {
    let path = dir.join("config.toml");
    fs::write(&path, contents).unwrap();
    path
}

// ---------------------------------------------------------------------------
// Missing config → defaults (silent fallback)
// ---------------------------------------------------------------------------

#[test]
fn missing_file_returns_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nonexistent.toml");
    let config = load_config_file(&missing).unwrap();
    let defaults = OpiConfig::default();
    assert_eq!(
        config.defaults.model, defaults.defaults.model,
        "missing file should fall back to default model"
    );
}

#[test]
fn missing_file_does_not_error() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nonexistent.toml");
    let result = load_config_file(&missing);
    assert!(
        result.is_ok(),
        "missing optional config file should not error, got {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Valid TOML → correct parsed values
// ---------------------------------------------------------------------------

#[test]
fn valid_config_parses_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[defaults]
model = "anthropic:claude-sonnet-4"
max_iterations = 100
tool_timeout_ms = 60000
theme = "dark"
"#,
    );
    let config = load_config_file(&path).unwrap();
    assert_eq!(config.defaults.model, "anthropic:claude-sonnet-4");
    assert_eq!(config.defaults.max_iterations, 100);
    assert_eq!(config.defaults.tool_timeout_ms, 60000);
    assert_eq!(config.defaults.theme, "dark");
}

#[test]
fn valid_config_parses_thinking() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[thinking]
enabled = true
budget_tokens = 20000
"#,
    );
    let config = load_config_file(&path).unwrap();
    assert!(config.thinking.enabled);
    assert_eq!(config.thinking.budget_tokens, 20000);
}

#[test]
fn valid_config_parses_providers() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.anthropic]
api_key_env = "MY_ANTHROPIC_KEY"
"#,
    );
    let config = load_config_file(&path).unwrap();
    assert_eq!(config.providers.anthropic.api_key_env, "MY_ANTHROPIC_KEY");
}

#[test]
fn valid_config_parses_extension_and_package_paths() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[extensions]
paths = ["vendor/ext-a", "vendor/ext-b"]

[packages]
paths = ["vendor/pkg-a"]
"#,
    );
    let config = load_config_file(&path).unwrap();
    assert_eq!(
        config.extensions.paths,
        vec![PathBuf::from("vendor/ext-a"), PathBuf::from("vendor/ext-b")]
    );
    assert_eq!(config.packages.paths, vec![PathBuf::from("vendor/pkg-a")]);
}

#[test]
fn resolve_config_appends_resource_paths_in_layer_order() {
    let dir = tempfile::tempdir().unwrap();

    let user_config = write_temp_config(
        dir.path(),
        r#"
[extensions]
paths = ["user-ext"]

[packages]
paths = ["user-pkg"]
"#,
    );

    let project_dir = dir.path().join("project");
    let project_opi = project_dir.join(".opi");
    fs::create_dir_all(&project_opi).unwrap();
    fs::write(
        project_opi.join("config.toml"),
        r#"
[extensions]
paths = ["project-ext"]

[packages]
paths = ["project-pkg"]
"#,
    )
    .unwrap();

    let cli_config = dir.path().join("explicit.toml");
    fs::write(
        &cli_config,
        r#"
[extensions]
paths = ["cli-ext"]

[packages]
paths = ["cli-pkg"]
"#,
    )
    .unwrap();

    let config = resolve_config(ConfigSource {
        cli_model: None,
        config_path: Some(cli_config),
        env_model: None,
        project_dir: Some(project_dir),
        user_config_path: Some(user_config),
    })
    .unwrap();

    assert_eq!(
        config.extensions.paths,
        vec![
            PathBuf::from("user-ext"),
            PathBuf::from("project-ext"),
            PathBuf::from("cli-ext")
        ]
    );
    assert_eq!(
        config.packages.paths,
        vec![
            PathBuf::from("user-pkg"),
            PathBuf::from("project-pkg"),
            PathBuf::from("cli-pkg")
        ]
    );
}

// ---------------------------------------------------------------------------
// Malformed TOML → clear error
// ---------------------------------------------------------------------------

#[test]
fn malformed_toml_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
this is not valid toml [[[

[defaults
model = broken
"#,
    );
    let result = load_config_file(&path);
    assert!(result.is_err(), "malformed TOML should return error");
}

#[test]
fn malformed_error_message_is_clear() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[invalid toml !!
"#,
    );
    let result = load_config_file(&path);
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("config") || msg.contains("parse") || msg.contains("TOML"),
        "error message should mention config/parse/TOML, got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Partial config → defaults for missing fields
// ---------------------------------------------------------------------------

#[test]
fn partial_config_fills_missing_with_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[defaults]
model = "anthropic:claude-sonnet-4"
"#,
    );
    let config = load_config_file(&path).unwrap();
    assert_eq!(config.defaults.model, "anthropic:claude-sonnet-4");
    let defaults = OpiConfig::default();
    assert_eq!(
        config.defaults.max_iterations, defaults.defaults.max_iterations,
        "missing field should use default"
    );
    assert_eq!(
        config.defaults.tool_timeout_ms, defaults.defaults.tool_timeout_ms,
        "missing field should use default"
    );
}

#[test]
fn empty_config_uses_all_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(dir.path(), "");
    let config = load_config_file(&path).unwrap();
    let defaults = OpiConfig::default();
    assert_eq!(config.defaults.model, defaults.defaults.model);
    assert_eq!(
        config.defaults.max_iterations,
        defaults.defaults.max_iterations
    );
}

// ---------------------------------------------------------------------------
// resolve_config: defaults when no sources
// ---------------------------------------------------------------------------

#[test]
fn resolve_with_no_sources_returns_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let config = resolve_config(ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: Some(dir.path().to_path_buf()),
        user_config_path: None,
    })
    .unwrap();
    let defaults = OpiConfig::default();
    assert_eq!(config.defaults.model, defaults.defaults.model);
}

// ---------------------------------------------------------------------------
// Unknown fields ignored gracefully
// ---------------------------------------------------------------------------

#[test]
fn unknown_fields_are_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[defaults]
model = "anthropic:claude-sonnet-4"

[future_feature]
some_new_option = true
"#,
    );
    let result = load_config_file(&path);
    assert!(
        result.is_ok(),
        "unknown fields should be ignored, got {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Proxy config ([providers.*.proxy] parsing) — merged from proxy_config.rs
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// No proxy config → None
// ---------------------------------------------------------------------------

#[test]
fn no_proxy_config_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"
"#,
    );
    let config = load_config_file(&path).unwrap();
    assert!(
        config.providers.anthropic.proxy.is_none(),
        "no proxy config should be None"
    );
}

#[test]
fn empty_config_has_no_proxy() {
    let config = OpiConfig::default();
    assert!(config.providers.anthropic.proxy.is_none());
    assert!(config.providers.openai.proxy.is_none());
    assert!(config.providers.openrouter.proxy.is_none());
    assert!(config.providers.gemini.proxy.is_none());
}

// ---------------------------------------------------------------------------
// Parse proxy config for Anthropic provider
// ---------------------------------------------------------------------------

#[test]
fn parse_anthropic_proxy_url() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"

[providers.anthropic.proxy]
url = "http://proxy.example.com:8080"
"#,
    );
    let config = load_config_file(&path).unwrap();
    let proxy = config
        .providers
        .anthropic
        .proxy
        .as_ref()
        .expect("proxy should be set");
    assert_eq!(proxy.url, "http://proxy.example.com:8080");
    assert!(proxy.no_proxy.is_none());
}

#[test]
fn parse_anthropic_proxy_with_no_proxy() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"

[providers.anthropic.proxy]
url = "http://proxy.example.com:8080"
no_proxy = "localhost,*.internal"
"#,
    );
    let config = load_config_file(&path).unwrap();
    let proxy = config
        .providers
        .anthropic
        .proxy
        .as_ref()
        .expect("proxy should be set");
    assert_eq!(proxy.url, "http://proxy.example.com:8080");
    assert_eq!(proxy.no_proxy.as_deref(), Some("localhost,*.internal"));
}

// ---------------------------------------------------------------------------
// Parse proxy config for OpenAI provider
// ---------------------------------------------------------------------------

#[test]
fn parse_openai_proxy_url() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.openai]
api_key_env = "OPENAI_API_KEY"

[providers.openai.proxy]
url = "http://proxy.example.com:8080"
"#,
    );
    let config = load_config_file(&path).unwrap();
    let proxy = config
        .providers
        .openai
        .proxy
        .as_ref()
        .expect("proxy should be set");
    assert_eq!(proxy.url, "http://proxy.example.com:8080");
}

// ---------------------------------------------------------------------------
// Parse proxy config for Gemini provider
// ---------------------------------------------------------------------------

#[test]
fn parse_gemini_proxy_url() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.gemini]
api_key_env = "GEMINI_API_KEY"

[providers.gemini.proxy]
url = "http://proxy.example.com:8080"
no_proxy = "localhost"
"#,
    );
    let config = load_config_file(&path).unwrap();
    let proxy = config
        .providers
        .gemini
        .proxy
        .as_ref()
        .expect("proxy should be set");
    assert_eq!(proxy.url, "http://proxy.example.com:8080");
    assert_eq!(proxy.no_proxy.as_deref(), Some("localhost"));
}

// ---------------------------------------------------------------------------
// Parse proxy config for OpenRouter provider
// ---------------------------------------------------------------------------

#[test]
fn parse_openrouter_proxy_url() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.openrouter]
api_key_env = "OPENROUTER_API_KEY"

[providers.openrouter.proxy]
url = "http://proxy.example.com:8080"
"#,
    );
    let config = load_config_file(&path).unwrap();
    let proxy = config
        .providers
        .openrouter
        .proxy
        .as_ref()
        .expect("proxy should be set");
    assert_eq!(proxy.url, "http://proxy.example.com:8080");
}

// ---------------------------------------------------------------------------
// Parse proxy config for Mistral provider
// ---------------------------------------------------------------------------

#[test]
fn parse_mistral_proxy_url() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.mistral]
api_key_env = "MISTRAL_API_KEY"

[providers.mistral.proxy]
url = "http://proxy.example.com:8080"
"#,
    );
    let config = load_config_file(&path).unwrap();
    let proxy = config
        .providers
        .mistral
        .proxy
        .as_ref()
        .expect("proxy should be set");
    assert_eq!(proxy.url, "http://proxy.example.com:8080");
}

// ---------------------------------------------------------------------------
// Parse proxy config for OpenAI Responses provider
// ---------------------------------------------------------------------------

#[test]
fn parse_openai_responses_proxy_url() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.openai_responses]
api_key_env = "OPENAI_API_KEY"

[providers.openai_responses.proxy]
url = "http://proxy.example.com:8080"
"#,
    );
    let config = load_config_file(&path).unwrap();
    let proxy = config
        .providers
        .openai_responses
        .proxy
        .as_ref()
        .expect("proxy should be set");
    assert_eq!(proxy.url, "http://proxy.example.com:8080");
}

// ---------------------------------------------------------------------------
// Multiple providers with different proxy configs
// ---------------------------------------------------------------------------

#[test]
fn different_proxy_per_provider() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"

[providers.anthropic.proxy]
url = "http://anthropic-proxy:8080"

[providers.openai]
api_key_env = "OPENAI_API_KEY"

[providers.openai.proxy]
url = "http://openai-proxy:9090"
no_proxy = "localhost"
"#,
    );
    let config = load_config_file(&path).unwrap();
    let ap = config
        .providers
        .anthropic
        .proxy
        .as_ref()
        .expect("anthropic proxy");
    assert_eq!(ap.url, "http://anthropic-proxy:8080");
    assert!(ap.no_proxy.is_none());

    let op = config
        .providers
        .openai
        .proxy
        .as_ref()
        .expect("openai proxy");
    assert_eq!(op.url, "http://openai-proxy:9090");
    assert_eq!(op.no_proxy.as_deref(), Some("localhost"));

    // Gemini has no proxy configured
    assert!(config.providers.gemini.proxy.is_none());
}

// ---------------------------------------------------------------------------
// Empty proxy section without url is ignored
// ---------------------------------------------------------------------------

#[test]
fn empty_proxy_section_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(
        dir.path(),
        r#"
[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"

[providers.anthropic.proxy]
no_proxy = "localhost"
"#,
    );
    let config = load_config_file(&path).unwrap();
    assert!(
        config.providers.anthropic.proxy.is_none(),
        "proxy section without url should be ignored"
    );
}

// ---------------------------------------------------------------------------
// build_http_client tests
// ---------------------------------------------------------------------------

#[test]
fn build_http_client_with_explicit_proxy() {
    use opi_coding_agent::config::{ProviderProxyConfig, build_http_client};
    let proxy = ProviderProxyConfig {
        url: "http://proxy.example.com:8080".into(),
        no_proxy: Some("localhost".into()),
    };
    let client = build_http_client(Some(&proxy)).expect("valid proxy should succeed");
    let config = client.proxy_config();
    assert_eq!(
        config.url.as_deref(),
        Some("http://proxy.example.com:8080"),
        "proxy URL should be set"
    );
    assert_eq!(
        config.no_proxy.as_deref(),
        Some("localhost"),
        "no_proxy should be set"
    );
}

#[test]
fn build_http_client_with_no_proxy_falls_back_to_env() {
    use opi_coding_agent::config::build_http_client;
    // Without env proxy vars set, this should still produce a valid client.
    let client = build_http_client(None).expect("no-proxy should succeed");
    // Just verify it does not panic and returns a usable client.
    let _ = client.proxy_config();
}

#[test]
fn build_http_client_with_proxy_and_no_proxy_list() {
    use opi_coding_agent::config::{ProviderProxyConfig, build_http_client};
    let proxy = ProviderProxyConfig {
        url: "http://corporate-proxy.internal:3128".into(),
        no_proxy: Some("localhost,*.internal,10.0.0.0/8".into()),
    };
    let client = build_http_client(Some(&proxy)).expect("valid proxy should succeed");
    let config = client.proxy_config();
    assert!(config.url.is_some());
    assert_eq!(
        config.no_proxy.as_deref(),
        Some("localhost,*.internal,10.0.0.0/8")
    );
}

#[test]
fn build_http_client_with_proxy_no_no_proxy() {
    use opi_coding_agent::config::{ProviderProxyConfig, build_http_client};
    let proxy = ProviderProxyConfig {
        url: "http://proxy.example.com:9999".into(),
        no_proxy: None,
    };
    let client = build_http_client(Some(&proxy)).expect("valid proxy should succeed");
    let config = client.proxy_config();
    assert_eq!(config.url.as_deref(), Some("http://proxy.example.com:9999"));
    assert!(config.no_proxy.is_none());
}

#[test]
fn build_http_client_rejects_invalid_proxy_url() {
    use opi_coding_agent::config::{ProviderProxyConfig, build_http_client};
    let proxy = ProviderProxyConfig {
        url: "not a proxy url".into(),
        no_proxy: None,
    };
    let result = build_http_client(Some(&proxy));
    assert!(result.is_err(), "invalid proxy URL should return Err");
}

#[test]
fn build_http_client_without_proxy_succeeds() {
    use opi_coding_agent::config::build_http_client;
    let client = build_http_client(None).expect("no proxy should succeed");
    assert!(client.proxy_config().url.is_none());
}

// ---------------------------------------------------------------------------
// Keybindings ([keybindings] TOML) — merged from keybindings_config.rs
// ---------------------------------------------------------------------------

fn write_toml(contents: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut f, contents.as_bytes()).unwrap();
    f
}

// ---------------------------------------------------------------------------
// [keybindings] TOML section parsing
// ---------------------------------------------------------------------------

#[test]
fn keybindings_section_parsed_into_config() {
    let toml = r#"
[keybindings]
submit = "ctrl+j"
abort = "ctrl+c"
new_line = "shift+enter"
"#;
    let f = write_toml(toml);
    let config = load_config_file(f.path()).unwrap();
    assert_eq!(config.keybindings.submit, "ctrl+j");
    assert_eq!(config.keybindings.abort, "ctrl+c");
    assert_eq!(config.keybindings.new_line, "shift+enter");
}

#[test]
fn keybindings_missing_section_uses_defaults() {
    let toml = r#"
[defaults]
model = "anthropic:claude-sonnet-4"
"#;
    let f = write_toml(toml);
    let config = load_config_file(f.path()).unwrap();
    assert_eq!(config.keybindings.submit, "enter");
    assert_eq!(config.keybindings.abort, "escape");
    assert_eq!(config.keybindings.new_line, "alt+enter");
}

#[test]
fn keybindings_partial_override() {
    let toml = r#"
[keybindings]
submit = "ctrl+s"
"#;
    let f = write_toml(toml);
    let config = load_config_file(f.path()).unwrap();
    assert_eq!(config.keybindings.submit, "ctrl+s");
    // Non-overridden fields keep defaults
    assert_eq!(config.keybindings.abort, "escape");
    assert_eq!(config.keybindings.new_line, "alt+enter");
}

#[test]
fn keybindings_case_insensitive() {
    let toml = r#"
[keybindings]
submit = "Enter"
abort = "ESCAPE"
"#;
    let f = write_toml(toml);
    let config = load_config_file(f.path()).unwrap();
    assert_eq!(config.keybindings.submit, "Enter");
    assert_eq!(config.keybindings.abort, "ESCAPE");
}

#[test]
fn nonexistent_file_gives_default_keybindings() {
    let config = load_config_file(std::path::Path::new("/nonexistent/opi/config.toml")).unwrap();
    assert_eq!(config.keybindings.submit, "enter");
    assert_eq!(config.keybindings.abort, "escape");
    assert_eq!(config.keybindings.new_line, "alt+enter");
}

// ---------------------------------------------------------------------------
// Config precedence (CLI > env > project > user > defaults) — merged from config_precedence.rs
// ---------------------------------------------------------------------------

fn write_config(dir: &std::path::Path, subpath: &str, contents: &str) -> std::path::PathBuf {
    if let Some(parent) = std::path::Path::new(subpath).parent() {
        let parent_dir = dir.join(parent);
        fs::create_dir_all(&parent_dir).unwrap();
    }
    let path = dir.join(subpath);
    fs::write(&path, contents).unwrap();
    path
}

fn user_config_path(temp: &std::path::Path) -> std::path::PathBuf {
    temp.join("user_config").join("config.toml")
}

fn project_dir(temp: &std::path::Path) -> std::path::PathBuf {
    temp.join("project")
}

// ---------------------------------------------------------------------------
// CLI overrides everything
// ---------------------------------------------------------------------------

#[test]
fn cli_model_overrides_user_config() {
    let temp = tempfile::tempdir().unwrap();

    write_config(
        temp.path(),
        "user_config/config.toml",
        r#"
[defaults]
model = "anthropic:claude-opus-4"
"#,
    );

    write_config(
        temp.path(),
        "project/.opi/config.toml",
        r#"
[defaults]
model = "anthropic:claude-haiku-4"
"#,
    );

    let config = resolve_config(ConfigSource {
        cli_model: Some("anthropic:claude-sonnet-4".into()),
        config_path: None,
        env_model: None,
        project_dir: Some(project_dir(temp.path())),
        user_config_path: Some(user_config_path(temp.path())),
    })
    .unwrap();

    assert_eq!(config.defaults.model, "anthropic:claude-sonnet-4");
}

// ---------------------------------------------------------------------------
// Env overrides user and project config
// ---------------------------------------------------------------------------

#[test]
fn env_model_overrides_user_config() {
    let temp = tempfile::tempdir().unwrap();

    write_config(
        temp.path(),
        "user_config/config.toml",
        r#"
[defaults]
model = "anthropic:claude-opus-4"
"#,
    );

    let config = resolve_config(ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: Some("anthropic:claude-haiku-4".into()),
        project_dir: None,
        user_config_path: Some(user_config_path(temp.path())),
    })
    .unwrap();

    assert_eq!(config.defaults.model, "anthropic:claude-haiku-4");
}

// ---------------------------------------------------------------------------
// Project config overrides user config
// ---------------------------------------------------------------------------

#[test]
fn project_config_overrides_user_config() {
    let temp = tempfile::tempdir().unwrap();

    write_config(
        temp.path(),
        "user_config/config.toml",
        r#"
[defaults]
model = "anthropic:claude-opus-4"
max_iterations = 200
"#,
    );

    write_config(
        temp.path(),
        "project/.opi/config.toml",
        r#"
[defaults]
model = "anthropic:claude-sonnet-4"
"#,
    );

    let config = resolve_config(ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: Some(project_dir(temp.path())),
        user_config_path: Some(user_config_path(temp.path())),
    })
    .unwrap();

    // Project model wins
    assert_eq!(config.defaults.model, "anthropic:claude-sonnet-4");
    // User's max_iterations still applies (project didn't override it)
    assert_eq!(config.defaults.max_iterations, 200);
}

// ---------------------------------------------------------------------------
// User config overrides defaults
// ---------------------------------------------------------------------------

#[test]
fn user_config_overrides_defaults() {
    let temp = tempfile::tempdir().unwrap();

    write_config(
        temp.path(),
        "user_config/config.toml",
        r#"
[defaults]
model = "anthropic:claude-opus-4"
max_iterations = 100
"#,
    );

    let config = resolve_config(ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: None,
        user_config_path: Some(user_config_path(temp.path())),
    })
    .unwrap();

    assert_eq!(config.defaults.model, "anthropic:claude-opus-4");
    assert_eq!(config.defaults.max_iterations, 100);
}

// ---------------------------------------------------------------------------
// Built-in defaults when nothing is configured
// ---------------------------------------------------------------------------

#[test]
fn defaults_when_nothing_configured() {
    let temp = tempfile::tempdir().unwrap();

    let config = resolve_config(ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: Some(temp.path().join("no_project")),
        user_config_path: Some(temp.path().join("no_user").join("config.toml")),
    })
    .unwrap();

    let defaults = OpiConfig::default();
    assert_eq!(config.defaults.model, defaults.defaults.model);
    assert_eq!(
        config.defaults.max_iterations,
        defaults.defaults.max_iterations
    );
    assert_eq!(
        config.defaults.tool_timeout_ms,
        defaults.defaults.tool_timeout_ms
    );
}

// ---------------------------------------------------------------------------
// Full precedence chain: CLI > env > project > user > defaults
// ---------------------------------------------------------------------------

#[test]
fn full_precedence_chain() {
    let temp = tempfile::tempdir().unwrap();

    write_config(
        temp.path(),
        "user_config/config.toml",
        r#"
[defaults]
model = "user-model"
"#,
    );

    write_config(
        temp.path(),
        "project/.opi/config.toml",
        r#"
[defaults]
model = "project-model"
"#,
    );

    // CLI wins over env, project, user
    let config = resolve_config(ConfigSource {
        cli_model: Some("cli-model".into()),
        config_path: None,
        env_model: Some("env-model".into()),
        project_dir: Some(project_dir(temp.path())),
        user_config_path: Some(user_config_path(temp.path())),
    })
    .unwrap();
    assert_eq!(config.defaults.model, "cli-model");

    // Env wins over project, user (no CLI)
    let config = resolve_config(ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: Some("env-model".into()),
        project_dir: Some(project_dir(temp.path())),
        user_config_path: Some(user_config_path(temp.path())),
    })
    .unwrap();
    assert_eq!(config.defaults.model, "env-model");

    // Project wins over user (no CLI, no env)
    let config = resolve_config(ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: Some(project_dir(temp.path())),
        user_config_path: Some(user_config_path(temp.path())),
    })
    .unwrap();
    assert_eq!(config.defaults.model, "project-model");

    // User wins over defaults (no CLI, no env, no project)
    let config = resolve_config(ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: None,
        user_config_path: Some(user_config_path(temp.path())),
    })
    .unwrap();
    assert_eq!(config.defaults.model, "user-model");
}

// ---------------------------------------------------------------------------
// Malformed user config errors out
// ---------------------------------------------------------------------------

#[test]
fn malformed_user_config_is_error() {
    let temp = tempfile::tempdir().unwrap();

    write_config(
        temp.path(),
        "user_config/config.toml",
        r#"
[invalid toml !!!
"#,
    );

    let result = resolve_config(ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: None,
        user_config_path: Some(user_config_path(temp.path())),
    });

    assert!(result.is_err(), "malformed user config should be an error");
}

// ---------------------------------------------------------------------------
// Malformed project config errors out
// ---------------------------------------------------------------------------

#[test]
fn malformed_project_config_is_error() {
    let temp = tempfile::tempdir().unwrap();

    write_config(
        temp.path(),
        "project/.opi/config.toml",
        r#"
[broken [[[
"#,
    );

    let result = load_config_file(&temp.path().join("project").join(".opi").join("config.toml"));
    assert!(
        result.is_err(),
        "malformed project config should be an error"
    );
}

// ---------------------------------------------------------------------------
// --config with non-existent file is an error
// ---------------------------------------------------------------------------

#[test]
fn explicit_config_path_nonexistent_is_error() {
    let temp = tempfile::tempdir().unwrap();

    let result = resolve_config(ConfigSource {
        cli_model: None,
        config_path: Some(temp.path().join("does_not_exist.toml")),
        env_model: None,
        project_dir: None,
        user_config_path: None,
    });

    assert!(
        result.is_err(),
        "explicit --config with non-existent file should be an error"
    );
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("not found") || err.to_string().contains("config"),
        "error message should indicate file not found, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// --config file loads and takes effect
// ---------------------------------------------------------------------------

#[test]
fn explicit_config_path_overrides_project() {
    let temp = tempfile::tempdir().unwrap();

    write_config(
        temp.path(),
        "project/.opi/config.toml",
        r#"
[defaults]
model = "project-model"
"#,
    );

    let config_path = write_config(
        temp.path(),
        "cli_config.toml",
        r#"
[defaults]
model = "cli-config-model"
"#,
    );

    let config = resolve_config(ConfigSource {
        cli_model: None,
        config_path: Some(config_path),
        env_model: None,
        project_dir: Some(project_dir(temp.path())),
        user_config_path: None,
    })
    .unwrap();

    assert_eq!(config.defaults.model, "cli-config-model");
}

// ---------------------------------------------------------------------------
// --config model is NOT overridden by OPI_MODEL
// ---------------------------------------------------------------------------

#[test]
fn explicit_config_model_not_overridden_by_env() {
    let temp = tempfile::tempdir().unwrap();

    let config_path = write_config(
        temp.path(),
        "cli_config.toml",
        r#"
[defaults]
model = "config-model"
"#,
    );

    let config = resolve_config(ConfigSource {
        cli_model: None,
        config_path: Some(config_path),
        env_model: Some("env-model".into()),
        project_dir: None,
        user_config_path: None,
    })
    .unwrap();

    assert_eq!(
        config.defaults.model, "config-model",
        "--config model should not be overridden by OPI_MODEL"
    );
}

// ---------------------------------------------------------------------------
// --model still overrides --config model
// ---------------------------------------------------------------------------

#[test]
fn cli_model_overrides_explicit_config() {
    let temp = tempfile::tempdir().unwrap();

    let config_path = write_config(
        temp.path(),
        "cli_config.toml",
        r#"
[defaults]
model = "config-model"
"#,
    );

    let config = resolve_config(ConfigSource {
        cli_model: Some("cli-model".into()),
        config_path: Some(config_path),
        env_model: Some("env-model".into()),
        project_dir: None,
        user_config_path: None,
    })
    .unwrap();

    assert_eq!(
        config.defaults.model, "cli-model",
        "--model should override --config and env"
    );
}
