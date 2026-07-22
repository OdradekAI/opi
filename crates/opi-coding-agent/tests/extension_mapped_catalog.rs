use std::sync::{Arc, atomic::AtomicUsize};

use opi_agent::extension::{Extension, ExtensionRegistry};
use opi_ai::provider::{
    CacheRetention, EventStream, Provider, ProviderError, Request, ThinkingConfig,
};
use opi_ai::{ApiMappedProvider, ModelCapabilities, ModelInfo, WireApi};
use tokio_util::sync::CancellationToken;

struct ModelExtension;

impl Extension for ModelExtension {
    fn name(&self) -> &str {
        "mapped-model-extension"
    }

    fn model_overrides(&self) -> Vec<(String, ModelInfo)> {
        vec![(
            "mapped".into(),
            ModelInfo::new(
                "added",
                "Added",
                WireApi::OpenAiCompletions,
                ModelCapabilities::new(2000, 200),
            ),
        )]
    }
}

struct WireOverrideExtension;

impl Extension for WireOverrideExtension {
    fn name(&self) -> &str {
        "mapped-wire-override-extension"
    }

    fn model_overrides(&self) -> Vec<(String, ModelInfo)> {
        vec![(
            "mapped".into(),
            ModelInfo::new(
                "original",
                "Original override",
                WireApi::AnthropicMessages,
                ModelCapabilities::new(3000, 300),
            ),
        )]
    }
}

struct Route {
    models: Vec<ModelInfo>,
    calls: Arc<AtomicUsize>,
}

impl Provider for Route {
    fn id(&self) -> &str {
        "mapped"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream(&self, _request: Request) -> EventStream {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::pin(futures_util::stream::empty())
    }

    fn replace_model_catalog(&mut self, models: Vec<ModelInfo>) -> Result<(), ProviderError> {
        self.models = models;
        Ok(())
    }
}

fn request(model: &str) -> Request {
    Request {
        model: format!("mapped:{model}"),
        system: None,
        messages: Vec::new(),
        tools: Vec::new(),
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: Vec::new(),
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: Vec::new(),
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

#[test]
fn harness_collection_materializes_active_mapped_provider_overrides() {
    let original = ModelInfo::new(
        "original",
        "Original",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(1000, 100),
    );
    let openai_calls = Arc::new(AtomicUsize::new(0));
    let mut provider = ApiMappedProvider::try_new(
        "mapped",
        vec![original.clone()],
        [(
            WireApi::OpenAiCompletions,
            Box::new(Route {
                models: vec![original],
                calls: openai_calls.clone(),
            }) as Box<dyn Provider>,
        )],
    )
    .unwrap();
    let mut extensions = ExtensionRegistry::new();
    extensions.register(Box::new(ModelExtension)).unwrap();

    let (collection, diagnostics) = opi_coding_agent::provider_factory::assemble_harness_collection(
        &mut provider,
        Some(&extensions),
    );
    assert!(diagnostics.is_empty());
    assert!(collection.resolve("mapped:added").is_ok());

    drop(provider.stream(request("added")));

    assert_eq!(openai_calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(provider.models().iter().any(|model| model.id == "added"));
}

#[test]
fn harness_collection_rejects_an_override_whose_wire_has_no_concrete_route() {
    let original = ModelInfo::new(
        "original",
        "Original",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(1000, 100),
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let mut provider = ApiMappedProvider::try_new(
        "mapped",
        vec![original.clone()],
        [(
            WireApi::OpenAiCompletions,
            Box::new(Route {
                models: vec![original],
                calls: calls.clone(),
            }) as Box<dyn Provider>,
        )],
    )
    .unwrap();
    let mut extensions = ExtensionRegistry::new();
    extensions
        .register(Box::new(WireOverrideExtension))
        .unwrap();

    let (collection, diagnostics) = opi_coding_agent::provider_factory::assemble_harness_collection(
        &mut provider,
        Some(&extensions),
    );
    assert_eq!(diagnostics.len(), 1);
    assert!(collection.resolve("mapped:original").is_ok());

    drop(provider.stream(request("original")));
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(
        provider
            .models()
            .iter()
            .find(|model| model.id == "original")
            .unwrap()
            .wire_api,
        WireApi::OpenAiCompletions
    );
}
