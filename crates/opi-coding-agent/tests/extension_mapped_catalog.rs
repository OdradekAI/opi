use std::sync::{Arc, Mutex, atomic::AtomicUsize};

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

struct EmptyLaterRouteExtension;

impl Extension for EmptyLaterRouteExtension {
    fn name(&self) -> &str {
        "mapped-empty-later-route-extension"
    }

    fn model_overrides(&self) -> Vec<(String, ModelInfo)> {
        vec![(
            "mapped".into(),
            ModelInfo::new(
                "chat",
                "Chat moved to Anthropic",
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

struct ObservedRoute {
    models: Vec<ModelInfo>,
    observed_ids: Arc<Mutex<Vec<String>>>,
}

impl Provider for ObservedRoute {
    fn id(&self) -> &str {
        "mapped"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream_prepared(&self, _request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        Box::pin(futures_util::stream::empty())
    }

    fn replace_model_catalog(&mut self, models: Vec<ModelInfo>) -> Result<(), ProviderError> {
        *self.observed_ids.lock().unwrap() = models.iter().map(|model| model.id.clone()).collect();
        self.models = models;
        Ok(())
    }
}

impl Provider for Route {
    fn id(&self) -> &str {
        "mapped"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream_prepared(&self, _request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
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

    drop(provider.stream_prepared(request("added"), opi_ai::test_support::resolved_auth()));

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

    drop(provider.stream_prepared(request("original"), opi_ai::test_support::resolved_auth()));
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

#[test]
fn harness_collection_keeps_all_mapped_routes_unchanged_when_a_later_route_would_be_empty() {
    let anthropic = ModelInfo::new(
        "anthropic",
        "Anthropic",
        WireApi::AnthropicMessages,
        ModelCapabilities::new(1000, 100),
    );
    let chat = ModelInfo::new(
        "chat",
        "Chat",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(1000, 100),
    );
    let anthropic_ids = Arc::new(Mutex::new(vec!["anthropic".to_owned()]));
    let chat_ids = Arc::new(Mutex::new(vec!["chat".to_owned()]));
    let mut provider = ApiMappedProvider::try_new(
        "mapped",
        vec![anthropic.clone(), chat.clone()],
        [
            (
                WireApi::AnthropicMessages,
                Box::new(ObservedRoute {
                    models: vec![anthropic],
                    observed_ids: Arc::clone(&anthropic_ids),
                }) as Box<dyn Provider>,
            ),
            (
                WireApi::OpenAiCompletions,
                Box::new(ObservedRoute {
                    models: vec![chat],
                    observed_ids: Arc::clone(&chat_ids),
                }) as Box<dyn Provider>,
            ),
        ],
    )
    .unwrap();
    let mut extensions = ExtensionRegistry::new();
    extensions
        .register(Box::new(EmptyLaterRouteExtension))
        .unwrap();

    let (collection, diagnostics) = opi_coding_agent::provider_factory::assemble_harness_collection(
        &mut provider,
        Some(&extensions),
    );

    assert_eq!(diagnostics.len(), 1);
    assert!(collection.resolve("mapped:anthropic").is_ok());
    assert!(collection.resolve("mapped:chat").is_ok());
    assert_eq!(anthropic_ids.lock().unwrap().as_slice(), &["anthropic"]);
    assert_eq!(chat_ids.lock().unwrap().as_slice(), &["chat"]);
    assert_eq!(
        provider
            .models()
            .iter()
            .map(|model| (model.id.as_str(), model.wire_api))
            .collect::<Vec<_>>(),
        [
            ("anthropic", WireApi::AnthropicMessages),
            ("chat", WireApi::OpenAiCompletions),
        ]
    );
}
