//! Behavioral tests for --list-models (task 2.1).
//!
//! Tests that `opi --list-models` exits with code 0 when at least one
//! provider has credentials, outputs model IDs, and supports --json.
//! Also tests graceful failure when no credentials are available.
//!
//! All tests run from a temp dir to avoid .env file loading.

use std::process::Command;

use opi_ai::provider::{EventStream, ModelInfo, Provider, ProviderError, Request};
use opi_ai::stream::AssistantStreamEvent;
use opi_coding_agent::model_listing::model_entries_from_registry;

struct TestProvider {
    id: String,
    models: Vec<ModelInfo>,
}

impl Provider for TestProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream(&self, _request: Request) -> EventStream {
        let stream: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();
        Box::pin(futures_util::stream::iter(stream))
    }
}

fn model(id: &str, display_name: &str) -> ModelInfo {
    ModelInfo {
        id: id.into(),
        display_name: display_name.into(),
        context_window: 100_000,
        max_output_tokens: 4_096,
        supports_images: false,
        supports_streaming: true,
        supports_thinking: false,
    }
}

#[test]
fn model_entries_from_registry_include_overrides() {
    let mut registry = opi_ai::ProviderRegistry::new();
    registry
        .register_provider(Box::new(TestProvider {
            id: "provider-a".into(),
            models: vec![model("base", "Base")],
        }))
        .unwrap();
    registry
        .register_model("provider-a", model("extra", "Extra"))
        .unwrap();

    let entries = model_entries_from_registry(&registry);

    assert!(entries.iter().any(|entry| entry.provider_id == "provider-a"
        && entry.model_id == "base"
        && entry.display_name == "Base"));
    assert!(entries.iter().any(|entry| entry.provider_id == "provider-a"
        && entry.model_id == "extra"
        && entry.display_name == "Extra"));
}

fn opi_bin() -> String {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest.parent().unwrap().parent().unwrap();

    // Prefer debug (always fresh from `cargo test`), fall back to release.
    for profile in &["debug", "release"] {
        let mut path = workspace_root.join("target").join(profile).join("opi");
        if cfg!(windows) {
            path.set_extension("exe");
        }
        if path.exists() {
            return path.to_string_lossy().into_owned();
        }
    }

    // Fall back to debug path even if it doesn't exist yet (will fail clearly).
    let mut path = workspace_root.join("target/debug/opi");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    path.to_string_lossy().into_owned()
}

fn run_opi(args: &[&str], envs: &[(&str, &str)]) -> std::process::Output {
    let bin = opi_bin();
    let tmp = tempfile::tempdir().unwrap();
    let mut cmd = Command::new(&bin);
    cmd.args(args).current_dir(tmp.path()).env_clear();
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output()
        .unwrap_or_else(|e| panic!("failed to run {bin}: {e}"))
}

/// Run opi with a temp config file. The caller provides TOML content for the
/// config file; `envs` are set in addition to a clean environment.
fn run_opi_with_config(
    config_toml: &str,
    extra_args: &[&str],
    envs: &[(&str, &str)],
) -> std::process::Output {
    let bin = opi_bin();
    let tmp = tempfile::tempdir().unwrap();
    let config_path = tmp.path().join("test-config.toml");
    std::fs::write(&config_path, config_toml).unwrap();

    let mut args = vec![
        "--config".to_string(),
        config_path.to_string_lossy().into_owned(),
    ];
    for a in extra_args {
        args.push((*a).to_string());
    }

    let mut cmd = Command::new(&bin);
    cmd.args(&args).current_dir(tmp.path()).env_clear();
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output()
        .unwrap_or_else(|e| panic!("failed to run {bin}: {e}"))
}

#[test]
fn list_models_without_credentials_exits_nonzero() {
    let output = run_opi(&["--list-models"], &[]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "expected non-zero exit without credentials, got {:?}\nstderr: {stderr}",
        output.status.code()
    );
    assert!(
        stderr.contains("no models available"),
        "stderr should mention no models available, got: {stderr}"
    );
}

#[test]
fn list_models_with_anthropic_key_outputs_models() {
    let output = run_opi(
        &["--list-models"],
        &[("ANTHROPIC_API_KEY", "test-key-for-listing")],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "expected exit 0 with ANTHROPIC_API_KEY, got {:?}\nstdout: {stdout}\nstderr: {stderr}",
        output.status.code()
    );
    assert!(
        stdout.contains("anthropic"),
        "output should mention 'anthropic' provider, got: {stdout}"
    );
    assert!(
        stdout.contains("claude"),
        "output should contain claude model IDs, got: {stdout}"
    );
}

#[test]
fn list_models_json_outputs_ndjson() {
    let output = run_opi(
        &["--list-models", "--json"],
        &[("ANTHROPIC_API_KEY", "test-key-for-listing")],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "expected exit 0, got {:?}\nstdout: {stdout}\nstderr: {stderr}",
        output.status.code()
    );

    let mut found_anthropic = false;
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line is not valid JSON: {line}\nerror: {e}"));
        assert!(
            v.get("model").is_some(),
            "JSON line missing 'model' field: {line}"
        );
        assert!(
            v.get("provider").is_some(),
            "JSON line missing 'provider' field: {line}"
        );
        assert!(
            v.get("display_name").is_some(),
            "JSON line missing 'display_name' field: {line}"
        );
        if v["provider"].as_str() == Some("anthropic") {
            found_anthropic = true;
        }
    }
    assert!(
        found_anthropic,
        "expected at least one anthropic model in JSON output"
    );
}

#[test]
fn list_models_includes_provider_column() {
    let output = run_opi(
        &["--list-models"],
        &[("ANTHROPIC_API_KEY", "test-key-for-listing")],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "expected exit 0, got {:?}",
        output.status.code()
    );

    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.len() >= 3,
        "expected at least header + separator + 1 model, got {} lines",
        lines.len()
    );
    assert!(
        lines[0].contains("PROVIDER"),
        "header should contain PROVIDER, got: {}",
        lines[0]
    );
    assert!(
        lines[0].contains("MODEL ID"),
        "header should contain MODEL ID, got: {}",
        lines[0]
    );
}

// ---------------------------------------------------------------------------
// Invalid proxy config must cause --list-models to exit with config error
// ---------------------------------------------------------------------------

#[test]
fn list_models_invalid_proxy_exits_config_error() {
    let output = run_opi_with_config(
        r#"
[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"

[providers.anthropic.proxy]
url = "not a proxy url"
"#,
        &["--list-models"],
        &[("ANTHROPIC_API_KEY", "test-key-for-listing")],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let code = output.status.code();

    assert!(
        code == Some(2),
        "expected exit code 2 for config error, got {code:?}\nstdout: {stdout}\nstderr: {stderr}",
    );
    assert!(
        stderr.contains("config error"),
        "stderr should mention config error, got: {stderr}",
    );
    assert!(
        stderr.contains("failed to build HTTP client with proxy config"),
        "stderr should mention proxy config failure, got: {stderr}",
    );
    assert!(
        stdout.is_empty(),
        "stdout should be empty on config error, got: {stdout}",
    );
}

#[test]
fn list_models_valid_proxy_with_credentials_succeeds() {
    // A well-formed proxy URL (nothing is listening, but config parsing succeeds).
    // This verifies that valid proxy config does not block --list-models.
    let output = run_opi_with_config(
        r#"
[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"

[providers.anthropic.proxy]
url = "http://proxy.example.com:8080"
"#,
        &["--list-models"],
        &[("ANTHROPIC_API_KEY", "test-key-for-listing")],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "expected exit 0 with valid proxy config, got {:?}\nstdout: {stdout}\nstderr: {stderr}",
        output.status.code()
    );
    assert!(
        stdout.contains("claude"),
        "output should contain claude model IDs, got: {stdout}",
    );
}

#[test]
fn list_models_missing_credentials_skips_provider_silently() {
    // No API key set for anthropic -- should skip silently, not error.
    // Another provider (openai) has a key, so we still get output.
    let output = run_opi_with_config(
        r#"
[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"
"#,
        &["--list-models"],
        // No ANTHROPIC_API_KEY, but set OPENAI_API_KEY
        &[("OPENAI_API_KEY", "test-key-for-listing")],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should succeed since OpenAI has credentials
    assert!(
        output.status.success(),
        "expected exit 0, got {:?}",
        output.status.code()
    );
    // Should have openai models but not anthropic
    assert!(
        stdout.contains("openai"),
        "output should contain openai models, got: {stdout}",
    );
}

#[test]
fn list_models_includes_configured_openai_compatible_profile() {
    let output = run_opi_with_config(
        r#"
[providers.openai_compatible.localai]
api_key_env = "LOCALAI_API_KEY"
base_url = "https://localai.example.com"
system_role_override = "developer"
max_tokens_field = "max_completion_tokens"
tool_result_name_field = true
usage_in_stream = true

[[providers.openai_compatible.localai.models]]
id = "local-model"
display_name = "Local Model"
context_window = 128000
max_output_tokens = 4096
supports_images = true
supports_streaming = true
supports_thinking = true
"#,
        &["--list-models", "--json"],
        &[("LOCALAI_API_KEY", "test-key-for-listing")],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "expected exit 0, got {:?}\nstdout: {stdout}\nstderr: {stderr}",
        output.status.code()
    );

    let found = stdout.lines().any(|line| {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            return false;
        };
        value["provider"].as_str() == Some("localai")
            && value["model"].as_str() == Some("local-model")
            && value["display_name"].as_str() == Some("Local Model")
    });

    assert!(
        found,
        "expected configured profile model in --list-models output, got: {stdout}"
    );
}

#[test]
fn list_models_does_not_leak_api_key_value() {
    // DoD: a configured API key value must never reach --list-models stdout or
    // stderr in either table or JSON form. AuthDescriptor/SecretKey keep the
    // value internal; this asserts that boundary through the CLI surface (the
    // only credential-bearing list-models test sets a key and asserts positive
    // content but never that the secret is redacted from output).
    let sentinel = "opi-sentinel-key-do-not-leak-9F3A2B";

    let table = run_opi(&["--list-models"], &[("ANTHROPIC_API_KEY", sentinel)]);
    let table_stdout = String::from_utf8_lossy(&table.stdout);
    let table_stderr = String::from_utf8_lossy(&table.stderr);
    assert!(
        !table_stdout.contains(sentinel),
        "API key sentinel leaked into --list-models table stdout:\n{table_stdout}"
    );
    assert!(
        !table_stderr.contains(sentinel),
        "API key sentinel leaked into --list-models table stderr:\n{table_stderr}"
    );

    let json = run_opi(
        &["--list-models", "--json"],
        &[("ANTHROPIC_API_KEY", sentinel)],
    );
    let json_stdout = String::from_utf8_lossy(&json.stdout);
    let json_stderr = String::from_utf8_lossy(&json.stderr);
    assert!(
        !json_stdout.contains(sentinel),
        "API key sentinel leaked into --list-models --json stdout:\n{json_stdout}"
    );
    assert!(
        !json_stderr.contains(sentinel),
        "API key sentinel leaked into --list-models --json stderr:\n{json_stderr}"
    );
}

// ---------------------------------------------------------------------------
// --list-models --json output validity (merged from list_models_json.rs)
// ---------------------------------------------------------------------------

#[test]
fn list_models_json_output_is_valid_json() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "opi-coding-agent",
            "--",
            "--list-models",
            "--json",
        ])
        .output()
        .expect("failed to run opi");
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        assert!(
            serde_json::from_str::<serde_json::Value>(line).is_ok(),
            "invalid JSON in --list-models --json output: {line:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Phase 14.1: stored-credential metadata is redacted (acceptance scenario)
// ---------------------------------------------------------------------------

/// Acceptance scenario `phase14-store-probe-surfaces` (list-models half):
/// with a keychain-backend config, `build_collection_for_listing` carries the
/// redacted, precomputed probe for the StoreCredential provider and exposes no
/// credential value through the listing entries. The env key is set only so the
/// provider constructs for model enumeration; it never reaches the probe or the
/// listing rows.
#[test]
fn stored_credential_metadata_is_redacted() {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use opi_ai::{AuthDescriptor, CredentialSource};
    use opi_coding_agent::config::CredentialBackendSource;
    use opi_coding_agent::provider_factory::build_collection_for_listing;

    static ENV_LOCK: Mutex<()> = Mutex::new(());
    let _guard = ENV_LOCK.lock().unwrap();
    let secret = "sk-listmodels-DO-NOT-LEAK";
    let original = std::env::var_os("ANTHROPIC_API_KEY");
    // SAFETY: serialized by ENV_LOCK; restored below.
    unsafe { std::env::set_var("ANTHROPIC_API_KEY", secret) };

    let outcome = build_collection_for_listing(
        &{
            let mut config = opi_coding_agent::config::OpiConfig::default();
            config.defaults.model = "anthropic:claude-listmodels".into();
            config.defaults.credential_backend = Some(CredentialBackendSource::Keychain);
            config
        },
        &{
            let mut probe_map = HashMap::new();
            probe_map.insert(
                "anthropic".to_string(),
                CredentialSource::Present {
                    label: "keychain opi:anthropic".to_owned(),
                },
            );
            probe_map
        },
    );

    // Restore env regardless of outcome.
    match original {
        Some(v) => unsafe { std::env::set_var("ANTHROPIC_API_KEY", v) },
        None => unsafe { std::env::remove_var("ANTHROPIC_API_KEY") },
    }

    let collection = outcome.expect("listing collection builds with env key");
    // Redacted probe carried, no secret.
    let probe = collection.probe("anthropic").expect("probe injected");
    assert!(
        matches!(probe, CredentialSource::Present { .. }),
        "expected Present probe, got {probe:?}"
    );
    assert!(!format!("{probe:?}").contains(secret));
    // StoreCredential descriptor, no secret.
    let desc = collection
        .auth_descriptor("anthropic")
        .expect("descriptor present");
    assert!(
        matches!(desc, AuthDescriptor::StoreCredential { .. }),
        "expected StoreCredential, got {desc:?}"
    );
    assert!(!format!("{desc:?}").contains(secret));
    // Model rows enumerate and carry no secret.
    let entries = model_entries_from_registry(collection.registry());
    assert!(entries.iter().any(|e| e.provider_id == "anthropic"));
    for e in &entries {
        assert!(!e.provider_id.contains(secret));
        assert!(!e.model_id.contains(secret));
        assert!(!e.display_name.contains(secret));
    }
}
