use std::collections::HashMap;

use opi_ai::credential::CredentialSource;
use opi_ai::provider::{ProviderError, ProviderErrorCategory};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::oauth::OAuthProviderRegistry;
use opi_coding_agent::provider_factory::{build_collection_for_listing, built_in_provider_ids};

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
