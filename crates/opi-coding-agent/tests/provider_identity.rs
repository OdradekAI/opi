use std::collections::HashMap;

use opi_ai::credential::CredentialSource;
use opi_ai::provider::{ProviderError, ProviderErrorCategory};
use opi_coding_agent::config::{ConfigError, OpiConfig, load_config_file};
use opi_coding_agent::oauth::OAuthProviderRegistry;
use opi_coding_agent::provider_factory::{
    ProviderBuildError, build_collection_for_listing, build_provider, built_in_provider_ids,
};

#[test]
fn canonical_oauth_provider_ids_are_exact() {
    let registry = OAuthProviderRegistry::registry_with_builtins();
    assert_eq!(
        registry.ids(),
        vec!["anthropic", "github-copilot", "openai-codex"]
    );
    assert!(built_in_provider_ids().contains(&"github-copilot"));
    assert!(built_in_provider_ids().contains(&"openai-codex"));

    let store_probe = HashMap::from([
        (
            "github-copilot".to_owned(),
            CredentialSource::Present {
                label: "keychain opi:github-copilot".into(),
            },
        ),
        (
            "openai-codex".to_owned(),
            CredentialSource::Present {
                label: "keychain opi:openai-codex".into(),
            },
        ),
    ]);
    let collection = build_collection_for_listing(&OpiConfig::default(), &store_probe).unwrap();
    assert_eq!(
        collection.provider_ids(),
        vec!["github-copilot", "openai-codex"]
    );
}

#[test]
fn development_provider_ids_are_rejected_without_alias_or_migration() {
    let registry = OAuthProviderRegistry::registry_with_builtins();
    assert!(registry.lookup("copilot").is_none());
    assert!(registry.lookup("codex").is_none());
    assert!(!built_in_provider_ids().contains(&"copilot"));
    assert!(!built_in_provider_ids().contains(&"codex"));

    // The deprecated dev-only ids must surface a rename hint rather than the
    // generic "unknown provider" fallthrough.
    for (deprecated, canonical) in [("copilot", "github-copilot"), ("codex", "openai-codex")] {
        let mut config = OpiConfig::default();
        config.defaults.model = format!("{deprecated}:gpt-4o");
        // Match instead of `.expect_err(..)`: the Ok variant is a
        // `Box<dyn Provider>`, which is not `Debug`, so `expect_err` would not
        // compile.
        let error = match build_provider(&config) {
            Err(e) => e,
            Ok(_) => panic!("deprecated id '{deprecated}' must error, but the provider built"),
        };
        match error {
            ProviderBuildError::Config(message) => {
                assert!(
                    message.contains(canonical),
                    "expected '{canonical}' rename hint for '{deprecated}', got: {message}"
                );
                assert!(
                    !message.contains("unknown provider"),
                    "rename hint must win over the generic fallthrough for '{deprecated}'"
                );
            }
            other => {
                panic!("expected ProviderBuildError::Config for '{deprecated}', got {other:?}")
            }
        }
    }
}

#[test]
fn credential_needed_remediation_uses_canonical_provider_id() {
    for provider_id in ["github-copilot", "openai-codex"] {
        let error = ProviderError::CredentialNeeded {
            provider_id: provider_id.into(),
        };
        assert_eq!(error.category(), ProviderErrorCategory::Auth);
        assert_eq!(
            error.to_string(),
            format!("credential needed for provider '{provider_id}'; run `/login {provider_id}`")
        );
    }
}

fn load(contents: &str) -> Result<OpiConfig, ConfigError> {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.toml");
    std::fs::write(&path, contents).unwrap();
    load_config_file(&path)
}

fn custom(provider: &str) -> String {
    format!(
        r#"
[providers.custom."{provider}"]
base_url = "https://custom.example"
api = "openai-completions"

[[providers.custom."{provider}".models]]
id = "model"
context_window = 1000
max_output_tokens = 100
"#
    )
}

#[test]
fn custom_provider_ids_cannot_shadow_builtins_or_deprecated_aliases() {
    for provider in ["anthropic", "github-copilot", "copilot", "codex"] {
        let error = load(&custom(provider)).expect_err("reserved id must fail");
        assert!(matches!(
            error,
            ConfigError::InvalidProviderNamespace { provider: ref id, .. } if id == provider
        ));
    }
}

#[test]
fn configured_provider_ids_cannot_contain_model_separator() {
    let error = load(&custom("bad:id")).expect_err("colon id must fail");
    assert!(matches!(
        error,
        ConfigError::InvalidProviderNamespace { ref provider, .. } if provider == "bad:id"
    ));
}

#[test]
fn custom_and_compatible_provider_namespaces_cannot_overlap() {
    let mut contents = custom("same");
    contents.push_str(
        r#"
[providers.openai_compatible.same]
api_key_env = "SAME_API_KEY"
base_url = "https://compatible.example"

[[providers.openai_compatible.same.models]]
id = "model"
context_window = 1000
max_output_tokens = 100
"#,
    );

    let error = load(&contents).expect_err("cross-table duplicate must fail");
    assert!(matches!(
        error,
        ConfigError::InvalidProviderNamespace { ref provider, .. } if provider == "same"
    ));
}

#[test]
fn custom_header_failures_reach_the_header_validation_gate() {
    for header in [
        r#"Authorization = "secret""#,
        r#""bad:name" = "value""#,
        r#""x-test" = "bad\nvalue""#,
    ] {
        let contents = format!(
            r#"
[providers.custom.bad]
base_url = "https://custom.example"
api = "openai-completions"

[providers.custom.bad.headers]
{header}

[[providers.custom.bad.models]]
id = "model"
context_window = 1000
max_output_tokens = 100
"#
        );
        let error = load(&contents).expect_err("invalid header must fail");
        assert!(matches!(
            error,
            ConfigError::InvalidCustomProvider {
                ref provider,
                field: "headers",
                ..
            } if provider == "bad"
        ));
    }
}
