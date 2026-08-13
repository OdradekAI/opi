//! Provider/model registration tests (task 4.6).
//!
//! Tests verify that custom providers and model overrides can be registered
//! into the ProviderRegistry and that model resolution, capability queries,
//! streaming, and model listing all work through the existing contracts.
//! All tests use MockProvider — no live provider calls.

use opi_ai::provider::{CacheRetention, ModelInfo, Provider};
use opi_ai::registry::ModelCapabilities;
use opi_ai::registry::ProviderRegistry;
use opi_ai::test_support::{MockProvider, text_response};
use opi_ai::{ModelInfoError, RegistrationError, RegistryError, WireApi};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn custom_model(id: &str, display: &str) -> ModelInfo {
    ModelInfo::new(
        id,
        display,
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(50_000, 2_048).with_streaming(true),
    )
}

fn custom_provider(id: &str, models: Vec<ModelInfo>) -> Box<dyn Provider> {
    let responses = vec![text_response("custom response")];
    let provider = MockProvider::new_with_models(id, models, responses);
    Box::new(provider) as Box<dyn Provider>
}

// ---------------------------------------------------------------------------
// 1. Provider registration
// ---------------------------------------------------------------------------

#[test]
fn register_custom_provider_and_resolve() {
    let mut registry = ProviderRegistry::new();
    let provider = custom_provider("my-custom", vec![custom_model("model-a", "Model A")]);
    registry.register_provider(provider).unwrap();

    let (resolved_provider, model) = registry.resolve("my-custom:model-a").unwrap();
    assert_eq!(resolved_provider.id(), "my-custom");
    assert_eq!(model.id, "model-a");
}

#[test]
fn register_provider_appears_in_provider_ids() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(custom_provider("zebra", vec![]))
        .unwrap();
    registry
        .register_provider(custom_provider("alpha", vec![]))
        .unwrap();

    let ids = registry.provider_ids();
    assert_eq!(ids, vec!["alpha", "zebra"]);
}

#[test]
fn register_provider_empty_id_rejected() {
    let mut registry = ProviderRegistry::new();
    let provider = custom_provider("", vec![]);
    let err = registry.register_provider(provider).unwrap_err();
    assert!(matches!(err, RegistrationError::EmptyProviderId));
}

#[test]
fn register_provider_replaces_existing() {
    let mut registry = ProviderRegistry::new();

    // First registration with model-a.
    registry
        .register_provider(custom_provider(
            "custom",
            vec![custom_model("model-a", "Model A")],
        ))
        .unwrap();

    // Second registration with model-b replaces the first.
    registry
        .register_provider(custom_provider(
            "custom",
            vec![custom_model("model-b", "Model B")],
        ))
        .unwrap();

    // model-a is gone; model-b resolves.
    assert!(registry.resolve("custom:model-a").is_err());
    let (_, model) = registry.resolve("custom:model-b").unwrap();
    assert_eq!(model.id, "model-b");
}

// ---------------------------------------------------------------------------
// 2. Model override registration
// ---------------------------------------------------------------------------

#[test]
fn register_model_and_resolve() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(custom_provider(
            "base",
            vec![custom_model("existing", "Existing")],
        ))
        .unwrap();

    // Add an additional model to the provider.
    registry
        .register_model("base", custom_model("extra", "Extra Model"))
        .unwrap();

    let (_, model) = registry.resolve("base:extra").unwrap();
    assert_eq!(model.id, "extra");
    assert_eq!(model.display_name, "Extra Model");
}

#[test]
fn register_model_duplicate_rejected() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(custom_provider("prov", vec![custom_model("m1", "M1")]))
        .unwrap();

    // Register same model override twice.
    registry
        .register_model("prov", custom_model("m1", "M1 Override"))
        .unwrap();

    // Second registration should fail because it conflicts with the existing override.
    let err = registry
        .register_model("prov", custom_model("m1", "M1 Override 2"))
        .unwrap_err();
    assert!(matches!(err, RegistrationError::DuplicateModel { .. }));
}

#[test]
fn register_model_empty_id_rejected() {
    let mut registry = ProviderRegistry::new();
    let model = ModelInfo::new(
        "",
        "Empty",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(0, 0),
    );
    let err = registry.register_model("prov", model).unwrap_err();
    assert!(matches!(err, RegistrationError::EmptyModelId { .. }));
}

#[test]
fn register_model_rejects_zero_token_limits() {
    let cases = [
        ("context_window", ModelCapabilities::new(0, 2_048)),
        ("max_output_tokens", ModelCapabilities::new(50_000, 0)),
    ];

    for (field, capabilities) in cases {
        let mut registry = ProviderRegistry::new();
        let model = ModelInfo::new(
            format!("zero-{field}"),
            "Invalid",
            WireApi::OpenAiCompletions,
            capabilities,
        );
        let error = registry.register_model("prov", model).unwrap_err();
        assert!(matches!(
            error,
            RegistrationError::InvalidModel {
                provider,
                model,
                source: ModelInfoError::InvalidCapabilities {
                    field: invalid_field,
                    ..
                },
            } if provider == "prov"
                && model == format!("zero-{field}")
                && invalid_field == field
        ));
    }
}

#[test]
fn register_model_override_duplicate_override_rejected() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(custom_provider("prov", vec![]))
        .unwrap();

    registry
        .register_model("prov", custom_model("extra", "Extra"))
        .unwrap();

    // Same override again should fail.
    let err = registry
        .register_model("prov", custom_model("extra", "Extra V2"))
        .unwrap_err();
    assert!(matches!(err, RegistrationError::DuplicateModel { .. }));
}

// ---------------------------------------------------------------------------
// 3. Capabilities
// ---------------------------------------------------------------------------

#[test]
fn capabilities_for_custom_provider() {
    let mut registry = ProviderRegistry::new();
    let model = ModelInfo::new(
        "cap-model",
        "Cap Model",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 8_192)
            .with_images(true)
            .with_streaming(true)
            .with_thinking(true),
    );
    registry
        .register_provider(custom_provider("cap-prov", vec![model]))
        .unwrap();

    let caps = registry.capabilities("cap-prov:cap-model").unwrap();
    assert_eq!(caps.context_window, 200_000);
    assert_eq!(caps.max_output_tokens, 8_192);
    assert!(caps.supports_images);
    assert!(caps.supports_streaming);
    assert!(caps.supports_thinking);
}

#[test]
fn capabilities_for_model_override() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(custom_provider("prov", vec![]))
        .unwrap();

    let model = ModelInfo::new(
        "override-model",
        "Override",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(300_000, 16_384).with_streaming(true),
    );
    registry.register_model("prov", model).unwrap();

    let caps = registry.capabilities("prov:override-model").unwrap();
    assert_eq!(caps.context_window, 300_000);
    assert_eq!(caps.max_output_tokens, 16_384);
}

// ---------------------------------------------------------------------------
// 4. Streaming through registered provider
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_from_custom_provider() {
    use futures_util::StreamExt;
    use opi_ai::provider::Request;
    use tokio_util::sync::CancellationToken;

    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(custom_provider(
            "stream-prov",
            vec![custom_model("s-model", "Stream Model")],
        ))
        .unwrap();

    let (provider, _) = registry.resolve("stream-prov:s-model").unwrap();
    let request = Request {
        model: "stream-prov:s-model".into(),
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

    let stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let events: Vec<_> = stream.collect::<Vec<_>>().await;
    assert!(!events.is_empty());
    // Should have at least Start, TextDelta, Done.
    assert!(events.len() >= 2);
}

// ---------------------------------------------------------------------------
// 5. all_models listing
// ---------------------------------------------------------------------------

#[test]
fn all_models_includes_custom_providers() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(custom_provider(
            "prov-a",
            vec![custom_model("m1", "M1"), custom_model("m2", "M2")],
        ))
        .unwrap();

    let models = registry.all_models();
    let ids: Vec<&str> = models.iter().map(|(_, m)| m.id.as_str()).collect();
    assert!(ids.contains(&"m1"));
    assert!(ids.contains(&"m2"));
}

#[test]
fn all_models_includes_model_overrides() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(custom_provider("prov", vec![custom_model("base", "Base")]))
        .unwrap();
    registry
        .register_model("prov", custom_model("extra", "Extra"))
        .unwrap();

    let models = registry.all_models();
    let ids: Vec<&str> = models.iter().map(|(_, m)| m.id.as_str()).collect();
    assert!(ids.contains(&"base"));
    assert!(ids.contains(&"extra"));
}

#[test]
fn all_models_lists_overrides_in_deterministic_order() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(custom_provider("prov", vec![custom_model("base", "Base")]))
        .unwrap();
    registry
        .register_model("z-provider", custom_model("z-model", "Z"))
        .unwrap();
    registry
        .register_model("a-provider", custom_model("b-model", "B"))
        .unwrap();
    registry
        .register_model("a-provider", custom_model("a-model", "A"))
        .unwrap();

    let models = registry.all_models();
    let override_entries = models
        .iter()
        .filter(|(_, model)| model.id != "base")
        .map(|(provider, model)| (*provider, model.id.as_str()))
        .collect::<Vec<_>>();

    assert_eq!(
        override_entries,
        vec![
            ("a-provider", "a-model"),
            ("a-provider", "b-model"),
            ("z-provider", "z-model")
        ]
    );
}

// ---------------------------------------------------------------------------
// 6. all_models deduplication
// ---------------------------------------------------------------------------

#[test]
fn all_models_deduplicates_when_override_shadows_built_in() {
    let mut registry = ProviderRegistry::new();

    // Provider has "shared" model.
    let base_model = ModelInfo::new(
        "shared",
        "Base Shared",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(100_000, 4_096).with_streaming(true),
    );
    registry
        .register_provider(custom_provider("prov", vec![base_model]))
        .unwrap();

    // Override shadows "shared".
    let override_model = ModelInfo::new(
        "shared",
        "Override Shared",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 8_192)
            .with_images(true)
            .with_streaming(true),
    );
    registry.register_model("prov", override_model).unwrap();

    let models = registry.all_models();
    // Only one entry for "shared" — the override.
    let shared_entries: Vec<_> = models.iter().filter(|(_, m)| m.id == "shared").collect();
    assert_eq!(shared_entries.len(), 1);
    assert_eq!(shared_entries[0].1.display_name, "Override Shared");
}

// ---------------------------------------------------------------------------
// 7. Resolution precedence
// ---------------------------------------------------------------------------

#[test]
fn resolve_override_takes_precedence_over_provider_model() {
    let mut registry = ProviderRegistry::new();

    // Provider declares model "shared" with context_window 100_000.
    let base_model = ModelInfo::new(
        "shared",
        "Base Shared",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(100_000, 4_096).with_streaming(true),
    );
    registry
        .register_provider(custom_provider("prov", vec![base_model]))
        .unwrap();

    // Override with different context_window.
    let override_model = ModelInfo::new(
        "shared",
        "Override Shared",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 8_192)
            .with_images(true)
            .with_streaming(true),
    );
    registry.register_model("prov", override_model).unwrap();

    // Override should win.
    let caps = registry.capabilities("prov:shared").unwrap();
    assert_eq!(caps.context_window, 200_000);
    assert_eq!(caps.max_output_tokens, 8_192);
    assert!(caps.supports_images);
}

#[test]
fn resolve_unknown_provider_returns_error() {
    let registry = ProviderRegistry::new();
    match registry.resolve("nonexistent:model") {
        Err(RegistryError::UnknownProvider(_)) => {}
        Err(other) => panic!("expected UnknownProvider, got: {other}"),
        Ok(_) => panic!("expected error, got success"),
    }
}

#[test]
fn resolve_unknown_model_returns_error() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(custom_provider(
            "prov",
            vec![custom_model("exists", "Exists")],
        ))
        .unwrap();

    match registry.resolve("prov:nonexistent") {
        Err(RegistryError::UnknownModel { .. }) => {}
        Err(other) => panic!("expected UnknownModel, got: {other}"),
        Ok(_) => panic!("expected error, got success"),
    }
}

// ---------------------------------------------------------------------------
// 7. Empty registry
// ---------------------------------------------------------------------------

#[test]
fn empty_registry_provider_ids_is_empty() {
    let registry = ProviderRegistry::new();
    assert!(registry.provider_ids().is_empty());
}

#[test]
fn empty_registry_all_models_is_empty() {
    let registry = ProviderRegistry::new();
    assert!(registry.all_models().is_empty());
}

// ---------------------------------------------------------------------------
// 8. get_provider for registered provider
// ---------------------------------------------------------------------------

#[test]
fn get_provider_returns_registered() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(custom_provider("my-prov", vec![custom_model("m1", "M1")]))
        .unwrap();

    let provider = registry.get_provider("my-prov").unwrap();
    assert_eq!(provider.id(), "my-prov");
    assert_eq!(provider.models().len(), 1);
}
