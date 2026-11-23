//! Provider replacement, lookup and streaming through the registry.

use opi_ai::RegistrationError;
use opi_ai::provider::CacheRetention;
use opi_ai::registry::ProviderRegistry;

use super::fixtures::{custom_model, custom_provider};

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
        projection: Default::default(),
        transport: Default::default(),
        model: "stream-prov:s-model".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: opi_ai::provider::ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };

    let stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let events: Vec<_> = stream.collect::<Vec<_>>().await;
    assert!(!events.is_empty());
    // Should have at least Start, TextDelta, Done.
    assert!(events.len() >= 2);
}

#[test]
fn empty_registry_provider_ids_is_empty() {
    let registry = ProviderRegistry::new();
    assert!(registry.provider_ids().is_empty());
}

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
