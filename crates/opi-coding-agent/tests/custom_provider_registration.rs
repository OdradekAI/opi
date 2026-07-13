//! Coding-agent custom provider registration integration tests (task 4.6).
//!
//! Tests verify that extensions can declare custom providers and model
//! overrides, that ExtensionRegistry collects them, and that the full
//! registration chain from extension -> registry -> resolve works.
//! All tests use MockProvider -- no live provider calls.

use opi_agent::extension::{Extension, ExtensionRegistry};
use opi_ai::provider::{CacheRetention, ModelInfo, Provider};
use opi_ai::registry::ModelCapabilities;
use opi_ai::registry::ProviderRegistry;
use opi_ai::test_support::{MockProvider, text_response};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn custom_model(id: &str, display: &str) -> ModelInfo {
    ModelInfo {
        id: id.into(),
        display_name: display.into(),
        capabilities: ModelCapabilities::new(50_000, 2_048).with_streaming(true),
    }
}

/// A test extension that provides custom providers and model overrides.
struct ProviderExtension {
    ext_name: String,
    provider_configs: Vec<(String, Vec<ModelInfo>)>,
    ext_model_overrides: Vec<(String, ModelInfo)>,
}

impl ProviderExtension {
    fn new(name: &str) -> Self {
        Self {
            ext_name: name.into(),
            provider_configs: Vec::new(),
            ext_model_overrides: Vec::new(),
        }
    }

    fn with_provider(mut self, id: &str, models: Vec<ModelInfo>) -> Self {
        self.provider_configs.push((id.into(), models));
        self
    }

    fn with_model_override(mut self, provider_id: &str, model: ModelInfo) -> Self {
        self.ext_model_overrides.push((provider_id.into(), model));
        self
    }
}

impl Extension for ProviderExtension {
    fn name(&self) -> &str {
        &self.ext_name
    }

    fn providers(&self) -> Vec<Box<dyn Provider>> {
        self.provider_configs
            .iter()
            .map(|(id, models)| {
                let responses = vec![text_response("ext-provider-response")];
                Box::new(MockProvider::new_with_models(id, models.clone(), responses))
                    as Box<dyn Provider>
            })
            .collect()
    }

    fn model_overrides(&self) -> Vec<(String, ModelInfo)> {
        self.ext_model_overrides.clone()
    }
}

// ---------------------------------------------------------------------------
// 1. Extension declares providers
// ---------------------------------------------------------------------------

#[test]
fn extension_provides_custom_provider() {
    let ext = ProviderExtension::new("test-ext")
        .with_provider("ext-prov", vec![custom_model("ext-model", "Ext Model")]);

    let providers = ext.providers();
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].id(), "ext-prov");
    assert_eq!(providers[0].models().len(), 1);
    assert_eq!(providers[0].models()[0].id, "ext-model");
}

#[test]
fn extension_provides_model_overrides() {
    let ext = ProviderExtension::new("test-ext")
        .with_model_override("anthropic", custom_model("claude-custom", "Custom Claude"));

    let overrides = ext.model_overrides();
    assert_eq!(overrides.len(), 1);
    assert_eq!(overrides[0].0, "anthropic");
    assert_eq!(overrides[0].1.id, "claude-custom");
}

// ---------------------------------------------------------------------------
// 2. ExtensionRegistry collects providers and model overrides
// ---------------------------------------------------------------------------

#[test]
fn extension_registry_collects_providers() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(
            ProviderExtension::new("ext-a").with_provider("prov-a", vec![custom_model("ma", "MA")]),
        ))
        .unwrap();
    registry
        .register(Box::new(
            ProviderExtension::new("ext-b").with_provider("prov-b", vec![custom_model("mb", "MB")]),
        ))
        .unwrap();

    let providers = registry.collect_providers();
    assert_eq!(providers.len(), 2);

    let ids: Vec<&str> = providers.iter().map(|p| p.id()).collect();
    assert!(ids.contains(&"prov-a"));
    assert!(ids.contains(&"prov-b"));
}

#[test]
fn extension_registry_collects_model_overrides() {
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(
            ProviderExtension::new("ext-1")
                .with_model_override("anthropic", custom_model("c1", "C1"))
                .with_model_override("openai", custom_model("g1", "G1")),
        ))
        .unwrap();

    let overrides = registry.collect_model_overrides();
    assert_eq!(overrides.len(), 2);

    let prov_ids: Vec<&str> = overrides.iter().map(|(pid, _)| pid.as_str()).collect();
    assert!(prov_ids.contains(&"anthropic"));
    assert!(prov_ids.contains(&"openai"));
}

// ---------------------------------------------------------------------------
// 3. Full registration chain: extension -> ProviderRegistry -> resolve
// ---------------------------------------------------------------------------

#[test]
fn extension_providers_register_and_resolve() {
    let mut ext_registry = ExtensionRegistry::new();
    ext_registry
        .register(Box::new(ProviderExtension::new("chain-ext").with_provider(
            "chain-prov",
            vec![custom_model("chain-model", "Chain Model")],
        )))
        .unwrap();

    let mut provider_registry = ProviderRegistry::new();
    for provider in ext_registry.collect_providers() {
        provider_registry.register_provider(provider).unwrap();
    }

    let (resolved, model) = provider_registry.resolve("chain-prov:chain-model").unwrap();
    assert_eq!(resolved.id(), "chain-prov");
    assert_eq!(model.id, "chain-model");
}

#[test]
fn extension_model_overrides_register_and_resolve() {
    let mut ext_registry = ExtensionRegistry::new();
    ext_registry
        .register(Box::new(
            ProviderExtension::new("override-ext")
                .with_model_override("base-prov", custom_model("extra-model", "Extra")),
        ))
        .unwrap();

    let mut provider_registry = ProviderRegistry::new();
    // Need the base provider registered first.
    provider_registry
        .register_provider(Box::new(MockProvider::new_with_models(
            "base-prov",
            vec![custom_model("base-model", "Base")],
            vec![text_response("base")],
        )))
        .unwrap();

    // Register model overrides from extensions.
    for (provider_id, model) in ext_registry.collect_model_overrides() {
        provider_registry
            .register_model(&provider_id, model)
            .unwrap();
    }

    // The override model resolves.
    let (_, model) = provider_registry.resolve("base-prov:extra-model").unwrap();
    assert_eq!(model.id, "extra-model");

    // The base model still resolves.
    let (_, model) = provider_registry.resolve("base-prov:base-model").unwrap();
    assert_eq!(model.id, "base-model");
}

// ---------------------------------------------------------------------------
// 4. No live provider calls
// ---------------------------------------------------------------------------

#[tokio::test]
async fn extension_provider_streams_without_network() {
    use futures_util::StreamExt;
    use opi_ai::provider::Request;
    use tokio_util::sync::CancellationToken;

    let mut ext_registry = ExtensionRegistry::new();
    ext_registry
        .register(Box::new(
            ProviderExtension::new("stream-ext")
                .with_provider("mock-net", vec![custom_model("local-m", "Local Model")]),
        ))
        .unwrap();

    let mut provider_registry = ProviderRegistry::new();
    for provider in ext_registry.collect_providers() {
        provider_registry.register_provider(provider).unwrap();
    }

    let (provider, _) = provider_registry.resolve("mock-net:local-m").unwrap();

    let request = Request {
        model: "mock-net:local-m".into(),
        system: None,
        messages: vec![],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: opi_ai::provider::ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };

    let stream = provider.stream(request);
    let events: Vec<_> = stream.collect::<Vec<_>>().await;
    assert!(!events.is_empty());
}

// ---------------------------------------------------------------------------
// 5. Duplicate registration across extensions
// ---------------------------------------------------------------------------

#[test]
fn duplicate_provider_from_different_extensions_replaces() {
    let mut ext_registry = ExtensionRegistry::new();
    ext_registry
        .register(Box::new(ProviderExtension::new("ext-1").with_provider(
            "shared-prov",
            vec![custom_model("v1-model", "V1")],
        )))
        .unwrap();
    ext_registry
        .register(Box::new(ProviderExtension::new("ext-2").with_provider(
            "shared-prov",
            vec![custom_model("v2-model", "V2")],
        )))
        .unwrap();

    let mut provider_registry = ProviderRegistry::new();
    for provider in ext_registry.collect_providers() {
        provider_registry.register_provider(provider).unwrap();
    }

    // Second extension's provider replaced the first.
    let ids = provider_registry.provider_ids();
    assert_eq!(ids, vec!["shared-prov"]);

    // Only v2-model should resolve (v1 was replaced).
    let (_, model) = provider_registry.resolve("shared-prov:v2-model").unwrap();
    assert_eq!(model.id, "v2-model");
}
