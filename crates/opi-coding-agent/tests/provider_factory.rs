//! Tests for provider factory construction across all 6 providers.
//!
//! Each test constructs a provider with a dummy API key and verifies the
//! provider reports the correct ID. Config integration tests verify that
//! TOML-deserialized provider configs resolve to the right env var names.

use std::sync::Mutex;

use opi_ai::provider::{Provider, Request, ThinkingConfig};
use opi_ai::test_support::MockProvider;
use opi_coding_agent::config::{
    GenericProviderConfig, OpenRouterProviderConfig, OpiConfig, load_config_file,
};
use tokio_util::sync::CancellationToken;

static ENV_MUTEX: Mutex<()> = Mutex::new(());

struct EnvVarGuard {
    key: String,
    original: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    fn set(key: &str, value: &str) -> Self {
        let guard = Self {
            key: key.to_owned(),
            original: std::env::var_os(key),
        };
        unsafe { std::env::set_var(key, value) };
        guard
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => unsafe { std::env::set_var(&self.key, value) },
            None => unsafe { std::env::remove_var(&self.key) },
        }
    }
}

fn with_env_vars<F, R>(vars: &[(&str, &str)], f: F) -> R
where
    F: FnOnce() -> R,
{
    let _lock = ENV_MUTEX.lock().unwrap();
    let _guards: Vec<_> = vars
        .iter()
        .map(|(key, value)| EnvVarGuard::set(key, value))
        .collect();
    f()
}

fn with_env_var<F, R>(key: &str, value: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    with_env_vars(&[(key, value)], f)
}

fn minimal_request(model: &str) -> Request {
    Request {
        model: model.into(),
        system: None,
        messages: vec![],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
    }
}

// ---------------------------------------------------------------------------
// Provider construction: correct id() per provider
// ---------------------------------------------------------------------------

#[test]
fn anthropic_provider_construction() {
    let provider = opi_ai::anthropic::AnthropicProvider::new("test-key".into(), None);
    assert_eq!(provider.id(), "anthropic");
}

#[test]
fn openai_provider_construction() {
    let provider = opi_ai::openai_chat::OpenAiChatProvider::new("test-key".into(), None);
    assert_eq!(provider.id(), "openai");
}

#[test]
fn openrouter_provider_construction() {
    let provider = opi_ai::openrouter::openrouter_provider("test-key".into(), None);
    assert_eq!(provider.id(), "openrouter");
}

#[test]
fn mistral_provider_construction() {
    let provider = opi_ai::mistral::mistral_provider("test-key".into(), None);
    assert_eq!(provider.id(), "mistral");
}

#[test]
fn openai_responses_provider_construction() {
    let provider = opi_ai::openai_responses::OpenAiResponsesProvider::new("test-key".into(), None);
    assert_eq!(provider.id(), "openai-responses");
}

#[test]
fn gemini_provider_construction() {
    let provider = opi_ai::gemini::GeminiProvider::new("test-key".into(), None);
    assert_eq!(provider.id(), "gemini");
}

// ---------------------------------------------------------------------------
// OpenRouter with custom referer header
// ---------------------------------------------------------------------------

#[test]
fn openrouter_with_custom_referer() {
    let compat = opi_ai::openai_chat::CompatConfig::default();
    // Get the default model list from the convenience function.
    let temp = opi_ai::openrouter::openrouter_provider(String::new(), None);
    let models = temp.models().to_vec();
    let provider = opi_ai::openai_chat::OpenAiChatProvider::new_for_profile(
        "test-key".into(),
        "https://openrouter.ai/api".into(),
        "openrouter".into(),
        compat,
        vec![
            ("HTTP-Referer".into(), "https://custom.example.com".into()),
            ("X-Title".into(), "opi".into()),
        ],
        models,
    );
    assert_eq!(provider.id(), "openrouter");
}

// ---------------------------------------------------------------------------
// Defaults config: provider structs
// ---------------------------------------------------------------------------

#[test]
fn generic_provider_default_has_empty_env() {
    let cfg = GenericProviderConfig::default();
    assert!(cfg.api_key_env.is_empty());
    assert!(cfg.base_url.is_none());
}

#[test]
fn openrouter_provider_default_has_empty_env() {
    let cfg = OpenRouterProviderConfig::default();
    assert!(cfg.api_key_env.is_empty());
    assert!(cfg.base_url.is_none());
    assert!(cfg.referer.is_none());
}

#[test]
fn opi_config_default_anthropic_env() {
    let config = OpiConfig::default();
    assert_eq!(config.providers.anthropic.api_key_env, "ANTHROPIC_API_KEY");
}

// ---------------------------------------------------------------------------
// TOML deserialization: all provider sections
// ---------------------------------------------------------------------------

#[test]
fn toml_parses_openai_provider() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[providers.openai]
api_key_env = "MY_OPENAI_KEY"
base_url = "https://custom.openai.example.com"
"#,
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();
    assert_eq!(config.providers.openai.api_key_env, "MY_OPENAI_KEY");
    assert_eq!(
        config.providers.openai.base_url.as_deref(),
        Some("https://custom.openai.example.com")
    );
}

#[test]
fn toml_parses_openai_compatible_profile_with_models_and_flags() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[providers.openai_compatible.localai]
api_key_env = "LOCALAI_API_KEY"
base_url = "https://localai.example.com"
system_role_override = "developer"
max_tokens_field = "max_completion_tokens"
tool_result_name_field = true
usage_in_stream = true

[providers.openai_compatible.localai.proxy]
url = "http://proxy.example.com:8080"

[[providers.openai_compatible.localai.models]]
id = "local-model"
display_name = "Local Model"
context_window = 128000
max_output_tokens = 4096
supports_images = true
supports_streaming = true
supports_thinking = true
"#,
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();

    let profile = config
        .providers
        .openai_compatible
        .get("localai")
        .expect("profile should be parsed");
    assert_eq!(profile.id, "localai");
    assert_eq!(profile.api_key_env, "LOCALAI_API_KEY");
    assert_eq!(profile.base_url, "https://localai.example.com");
    assert_eq!(profile.system_role_override.as_deref(), Some("developer"));
    assert_eq!(
        profile.max_tokens_field.as_deref(),
        Some("max_completion_tokens")
    );
    assert!(profile.tool_result_name_field);
    assert!(profile.usage_in_stream);
    assert_eq!(
        profile.proxy.as_ref().map(|proxy| proxy.url.as_str()),
        Some("http://proxy.example.com:8080")
    );

    let model = profile.models.first().expect("model should be parsed");
    assert_eq!(model.id, "local-model");
    assert_eq!(model.display_name, "Local Model");
    assert_eq!(model.context_window, 128000);
    assert_eq!(model.max_output_tokens, 4096);
    assert!(model.supports_images);
    assert!(model.supports_streaming);
    assert!(model.supports_thinking);
}

#[test]
fn toml_parses_openrouter_provider() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[providers.openrouter]
api_key_env = "MY_OPENROUTER_KEY"
referer = "https://myapp.example.com"
"#,
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();
    assert_eq!(config.providers.openrouter.api_key_env, "MY_OPENROUTER_KEY");
    assert_eq!(
        config.providers.openrouter.referer.as_deref(),
        Some("https://myapp.example.com")
    );
}

#[test]
fn toml_parses_mistral_provider() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[providers.mistral]
api_key_env = "MY_MISTRAL_KEY"
"#,
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();
    assert_eq!(config.providers.mistral.api_key_env, "MY_MISTRAL_KEY");
}

#[test]
fn toml_parses_openai_responses_provider() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[providers.openai_responses]
api_key_env = "MY_OPENAI_KEY"
"#,
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();
    assert_eq!(
        config.providers.openai_responses.api_key_env,
        "MY_OPENAI_KEY"
    );
}

#[test]
fn toml_parses_gemini_provider() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[providers.gemini]
api_key_env = "MY_GEMINI_KEY"
base_url = "https://custom-gemini.example.com"
"#,
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();
    assert_eq!(config.providers.gemini.api_key_env, "MY_GEMINI_KEY");
    assert_eq!(
        config.providers.gemini.base_url.as_deref(),
        Some("https://custom-gemini.example.com")
    );
}

#[test]
fn toml_multiple_providers_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[providers.anthropic]
api_key_env = "KEY_A"

[providers.openai]
api_key_env = "KEY_O"

[providers.gemini]
api_key_env = "KEY_G"

[providers.mistral]
api_key_env = "KEY_M"

[providers.openrouter]
api_key_env = "KEY_OR"

[providers.openai_responses]
api_key_env = "KEY_OAR"
"#,
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();
    assert_eq!(config.providers.anthropic.api_key_env, "KEY_A");
    assert_eq!(config.providers.openai.api_key_env, "KEY_O");
    assert_eq!(config.providers.gemini.api_key_env, "KEY_G");
    assert_eq!(config.providers.mistral.api_key_env, "KEY_M");
    assert_eq!(config.providers.openrouter.api_key_env, "KEY_OR");
    assert_eq!(config.providers.openai_responses.api_key_env, "KEY_OAR");
}

// ---------------------------------------------------------------------------
// Phase 10.2: provider construction routes through the collection/auth seam
// ---------------------------------------------------------------------------

#[test]
fn provider_factory_routes_through_collection() {
    use opi_ai::{AuthDescriptor, AuthStatus};
    use opi_coding_agent::config::load_config_file;
    use opi_coding_agent::provider_factory::{auth_descriptor_for, build_collection_for_listing};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let env_var = "OPI_TEST_FACTORY_ROUTE_9F2A7C11";
    std::fs::write(
        &path,
        format!(
            r#"
[providers.openai_compatible.testprof]
api_key_env = "{env_var}"
base_url = "https://testprof.example.com"

[[providers.openai_compatible.testprof.models]]
id = "test-model"
display_name = "Test Model"
context_window = 128000
max_output_tokens = 4096
supports_images = false
supports_streaming = true
supports_thinking = false
"#
        ),
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();

    // Auth-policy mapping is centralized in the factory and deterministic
    // (no environment variable needs to be set).
    match auth_descriptor_for(&config, "openai") {
        Some(AuthDescriptor::EnvApiKey { env_var }) => assert_eq!(env_var, "OPENAI_API_KEY"),
        other => panic!("expected openai EnvApiKey, got {other:?}"),
    }
    assert!(auth_descriptor_for(&config, "not-a-real-provider").is_none());

    with_env_var(env_var, "test-key", || {
        let collection = build_collection_for_listing(&config)
            .expect("listing collection builds through the factory");

        // The factory returns the ProviderCollection/auth-seam type and the profile
        // model is resolvable through it.
        let (_provider, model) = collection
            .resolve("testprof:test-model")
            .expect("profile model resolves through the collection");
        assert_eq!(model.id, "test-model");

        // Auth + compat metadata for the config-sourced profile live on the collection.
        assert_eq!(
            collection.auth_status("testprof"),
            Some(AuthStatus::Configured)
        );
        match collection.auth_descriptor("testprof") {
            Some(AuthDescriptor::Resolved { source }) => {
                assert_eq!(source, &format!("env {env_var}"))
            }
            other => panic!("expected profile Resolved auth, got {other:?}"),
        }
        let compat = collection
            .compat("testprof")
            .expect("profile compat metadata attached");
        assert!(compat.openai_compatible);
        assert_eq!(compat.profile.as_deref(), Some("testprof"));
    });
}

// ---------------------------------------------------------------------------
// Phase 12 task 12.3 — OpenAI-compatible profile flags + override precedence
// ---------------------------------------------------------------------------

#[test]
fn openai_compatible_profile_overrides() {
    use opi_coding_agent::provider_factory::build_provider;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let env_var = "OPI_TEST_COMPAT_OVERRIDES_4F2A91";
    std::fs::write(
        &path,
        format!(
            r#"
[defaults]
model = "myprof:reasoning-model"

[providers.openai_compatible.myprof]
api_key_env = "{env_var}"
base_url = "https://myprof.example.com"
system_role_override = "system"
max_tokens_field = "max_tokens"
strict_tool_schema = true
reasoning_effort = "medium"
cache_key = "profile-cache"
require_assistant_after_tool_result = true

[providers.openai_compatible.myprof.extra_headers]
X-Session-Id = "sess-1"

[[providers.openai_compatible.myprof.models]]
id = "reasoning-model"
display_name = "Reasoning"
context_window = 128000
max_output_tokens = 4096
system_role_override = "developer"
max_tokens_field = "max_completion_tokens"

[[providers.openai_compatible.myprof.models]]
id = "plain-model"
display_name = "Plain"
context_window = 128000
max_output_tokens = 4096
"#
        ),
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();

    let profile = config
        .providers
        .openai_compatible
        .get("myprof")
        .expect("profile should be parsed");
    assert!(profile.strict_tool_schema);
    assert_eq!(profile.reasoning_effort.as_deref(), Some("medium"));
    assert_eq!(profile.cache_key.as_deref(), Some("profile-cache"));
    assert!(profile.require_assistant_after_tool_result);
    assert_eq!(
        profile.extra_headers,
        vec![("X-Session-Id".to_string(), "sess-1".to_string())]
    );
    let reasoning = profile
        .models
        .iter()
        .find(|m| m.id == "reasoning-model")
        .expect("reasoning model");
    assert_eq!(reasoning.system_role_override.as_deref(), Some("developer"));
    assert_eq!(
        reasoning.max_tokens_field.as_deref(),
        Some("max_completion_tokens")
    );
    let plain = profile
        .models
        .iter()
        .find(|m| m.id == "plain-model")
        .expect("plain model");
    assert!(plain.system_role_override.is_none());
    assert!(plain.max_tokens_field.is_none());

    // The config-driven profile routes through the shared factory path.
    with_env_var(env_var, "test-key", || {
        build_provider(&config).expect("config-driven profile builds through the factory");
    });
}

#[test]
fn config_driven_compatible_profiles_are_preferred() {
    use opi_coding_agent::provider_factory::{ProviderBuildError, build_provider};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let env_var = "OPI_TEST_COMPAT_PREFERRED_7B1C04";
    std::fs::write(
        &path,
        format!(
            r#"
[defaults]
model = "acmeprof:acme-model"

[providers.openai_compatible.acmeprof]
api_key_env = "{env_var}"
base_url = "https://acme.example.com"

[[providers.openai_compatible.acmeprof.models]]
id = "acme-model"
display_name = "Acme"
context_window = 128000
max_output_tokens = 4096
"#
        ),
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();

    // A compatible provider is representable through profile metadata + config
    // overrides and routes through build_provider — the config-driven path is
    // preferred for OpenAI-compatible breadth rather than a new first-class
    // adapter (Phase 12 non-goal guard).
    with_env_var(env_var, "test-key", || {
        let provider = build_provider(&config).expect("config-driven profile is preferred");
        assert_eq!(provider.id(), "acmeprof");
    });

    // A provider id that is neither built-in nor a declared profile does NOT
    // silently become a first-class adapter — it fails explicitly. This guards
    // against broad first-class provider expansion as the default path.
    let mut cfg2 = config.clone();
    cfg2.defaults.model = "brand-new-vendor:some-model".into();
    match build_provider(&cfg2) {
        Ok(_) => panic!("unknown provider must not become first-class"),
        Err(ProviderBuildError::Config(_)) => {}
        Err(other) => panic!("expected Config build error, got {other:?}"),
    }
}

#[test]
fn listing_collection_skips_whitespace_only_credentials() {
    use opi_coding_agent::config::load_config_file;
    use opi_coding_agent::provider_factory::build_collection_for_listing;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let openai_env = "OPI_TEST_FACTORY_OPENAI_WS_ONLY_3E7B91";
    let profile_env = "OPI_TEST_FACTORY_PROFILE_WS_ONLY_3E7B91";
    std::fs::write(
        &path,
        format!(
            r#"
[providers.openai]
api_key_env = "{openai_env}"

[providers.openai_compatible.testprof]
api_key_env = "{profile_env}"
base_url = "https://testprof.example.com"

[[providers.openai_compatible.testprof.models]]
id = "test-model"
display_name = "Test Model"
context_window = 128000
max_output_tokens = 4096
supports_images = false
supports_streaming = true
supports_thinking = false
"#
        ),
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();

    with_env_vars(&[(openai_env, "   "), (profile_env, "\t ")], || {
        let collection =
            build_collection_for_listing(&config).expect("whitespace credentials are skipped");
        let provider_ids = collection.provider_ids();
        assert!(!provider_ids.contains(&"openai"));
        assert!(!provider_ids.contains(&"testprof"));
    });
}

#[test]
fn listing_collection_uses_resolved_auth_for_constructed_builtin() {
    use opi_ai::{AuthDescriptor, AuthStatus};
    use opi_coding_agent::config::load_config_file;
    use opi_coding_agent::provider_factory::build_collection_for_listing;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let env_var = "OPI_TEST_FACTORY_OPENAI_RESOLVED_25E1AC";
    std::fs::write(
        &path,
        format!(
            r#"
[providers.openai]
api_key_env = "{env_var}"
"#
        ),
    )
    .unwrap();
    let config = load_config_file(&path).unwrap();

    with_env_var(env_var, "test-key", || {
        let collection = build_collection_for_listing(&config).expect("listing collection builds");
        assert_eq!(
            collection.auth_status("openai"),
            Some(AuthStatus::Configured)
        );
        match collection.auth_descriptor("openai") {
            Some(AuthDescriptor::Resolved { source }) => {
                assert_eq!(source, &format!("env {env_var}"))
            }
            other => panic!("expected builtin Resolved auth, got {other:?}"),
        }
    });
}

#[tokio::test]
async fn metadata_only_provider_dispatch_returns_explicit_error() {
    use opi_coding_agent::provider_factory::assemble_harness_collection;

    let provider = MockProvider::new("metadata-provider", vec![]);
    let (collection, diagnostics) = assemble_harness_collection(&provider, None);
    assert!(diagnostics.is_empty());

    let error = collection
        .dispatch_complete(
            "metadata-provider:mock-model",
            minimal_request("metadata-provider:mock-model"),
        )
        .await
        .expect_err("metadata provider should not dispatch");
    let message = error.to_string();
    assert!(
        message.contains("metadata-only provider"),
        "expected metadata-only dispatch error, got {message:?}"
    );
    assert!(
        message.contains("'metadata-provider'"),
        "expected active provider id in dispatch error, got {message:?}"
    );
}

#[test]
fn built_in_openai_compatible_metadata_is_set() {
    use opi_coding_agent::provider_factory::compat_metadata_for;

    for provider in ["openai", "openrouter", "mistral"] {
        assert!(
            compat_metadata_for(provider).openai_compatible,
            "{provider} should advertise OpenAI-compatible chat metadata"
        );
    }
    assert!(!compat_metadata_for("anthropic").openai_compatible);
}

/// Strip Rust line and nested block comments while preserving string/char
/// literal contents, so documentation comments do not trip source scans.
fn strip_rust_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'/' && i + 1 < bytes.len() {
            if bytes[i + 1] == b'/' {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            } else if bytes[i + 1] == b'*' {
                let mut depth = 1;
                i += 2;
                while i < bytes.len() && depth > 0 {
                    if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                        depth += 1;
                        i += 2;
                    } else if bytes[i] == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                        depth -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                continue;
            }
        }
        if c == b'"' || c == b'\'' {
            let quote = c;
            out.push(c as char);
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    out.push(bytes[i] as char);
                    out.push(bytes[i + 1] as char);
                    i += 2;
                    continue;
                }
                out.push(bytes[i] as char);
                if bytes[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        out.push(c as char);
        i += 1;
    }
    out
}

/// Phase 10.2 centralization contract: every provider/model/auth
/// construction-policy symbol in `src/` lives only in `provider_factory.rs`.
/// Includes a vacuous-allowlist guard so the test cannot pass with an empty or
/// stale token set.
#[test]
fn provider_policy_is_centralized() {
    use std::fs;
    use std::path::{Path, PathBuf};

    fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).expect("read src dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                collect_rs(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }

    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let allow = "provider_factory.rs";

    let tokens = [
        "ProviderRegistry::new",
        "ProviderCollection::from_registry",
        "ProviderCollection::new",
        "fn parse_model_spec",
        "fn build_provider",
        "fn build_runtime_provider",
        "fn build_list_models_provider",
        "fn build_anthropic",
        "fn build_openai",
        "fn build_openrouter",
        "fn build_mistral",
        "fn build_openai_responses",
        "fn build_gemini",
        "fn build_bedrock",
        "fn build_azure",
        "fn build_vertex",
        "fn build_runtime_openai_compatible_profile",
        "fn build_list_models_openai_compatible_profile",
        "fn build_openai_compatible_profile",
        "fn build_collection_for_listing",
        "fn assemble_harness_collection",
        "fn require_api_key",
        "fn require_list_models_api_key",
        "fn non_empty_env_var",
        "fn resolve_env_name",
        "fn resolve_bedrock_env_credentials",
        "fn aws_credentials_path",
        "fn aws_config_path",
        "fn aws_home_dir",
        "fn profile_api_key_env_default",
        "fn auth_descriptor_for",
        "fn resolved_auth_descriptor_for",
        "fn resolved_auth_descriptor_for_profile",
        "fn auth_descriptor_for_profile",
        "fn compat_metadata_for",
        "struct MetadataProvider",
        "BUILT_IN_PROVIDER_IDS",
    ];

    let mut files = Vec::new();
    collect_rs(&src_dir, &mut files);

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        let rel = file
            .strip_prefix(&src_dir)
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");
        let text = strip_rust_comments(&fs::read_to_string(file).unwrap_or_default());
        for token in tokens {
            if text.contains(token) && rel != allow {
                violations.push(format!("token `{token}` appears in `{rel}`"));
            }
        }
    }

    // Vacuous-allowlist guard: provider_factory.rs must contain every token,
    // otherwise the centralization test would pass trivially.
    let factory_text = strip_rust_comments(
        &fs::read_to_string(src_dir.join(allow)).expect("provider_factory.rs exists"),
    );
    let missing: Vec<&str> = tokens
        .iter()
        .filter(|t| !factory_text.contains(*t))
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "provider_factory.rs is missing centralized tokens {missing:?} (allowlist is vacuous)"
    );

    assert!(
        violations.is_empty(),
        "provider construction policy is not centralized:\n{}",
        violations.join("\n")
    );
}
