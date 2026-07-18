//! Behavioral tests for the provider registry (task 1.4).
//!
//! DoD: "resolves anthropic:model and capabilities"

use opi_ai::WireApi;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::provider::{EventStream, ModelInfo, Provider, Request};
use opi_ai::registry::ModelCapabilities;
use opi_ai::registry::{ProviderRegistry, RegistryError};

/// Minimal stub provider for multi-provider tests.
struct StubProvider {
    id: String,
    models: Vec<ModelInfo>,
}

impl StubProvider {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_owned(),
            models: vec![ModelInfo::new(
                format!("{id}-model-1"),
                format!("{id} Model 1"),
                WireApi::OpenAiCompletions,
                ModelCapabilities::new(128000, 4096)
                    .with_images(true)
                    .with_streaming(true),
            )],
        }
    }
}

impl Provider for StubProvider {
    fn id(&self) -> &str {
        &self.id
    }
    fn models(&self) -> &[ModelInfo] {
        &self.models
    }
    fn stream(&self, _request: Request) -> EventStream {
        Box::pin(futures_util::stream::empty())
    }
}

/// Helper: create a registry with the Anthropic provider registered.
fn anthropic_registry() -> ProviderRegistry {
    let mut reg = ProviderRegistry::new();
    reg.register(Box::new(AnthropicProvider::new("test-key".into(), None)));
    reg
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

#[test]
fn register_and_list_providers() {
    let reg = anthropic_registry();
    let ids = reg.provider_ids();
    assert_eq!(ids, vec!["anthropic"]);
}

#[test]
fn register_multiple_providers() {
    let mut reg = ProviderRegistry::new();
    reg.register(Box::new(StubProvider::new("alpha")));
    reg.register(Box::new(StubProvider::new("beta")));
    let mut ids = reg.provider_ids();
    ids.sort();
    assert_eq!(ids, vec!["alpha", "beta"]);
}

#[test]
fn duplicate_registration_replaces() {
    let mut reg = ProviderRegistry::new();
    reg.register(Box::new(StubProvider::new("p")));
    reg.register(Box::new(StubProvider::new("p")));
    assert_eq!(reg.provider_ids().len(), 1);
}

// ---------------------------------------------------------------------------
// Resolve: provider:model ->(Provider, ModelInfo)
// ---------------------------------------------------------------------------

#[test]
fn resolve_anthropic_model() {
    let reg = anthropic_registry();
    let (provider, model) = reg.resolve("anthropic:claude-sonnet-4-5-20250514").unwrap();
    assert_eq!(provider.id(), "anthropic");
    assert_eq!(model.id, "claude-sonnet-4-5-20250514");
    assert!(model.capabilities.supports_streaming);
}

#[test]
fn resolve_returns_capabilities() {
    let reg = anthropic_registry();
    let caps = reg
        .capabilities("anthropic:claude-sonnet-4-5-20250514")
        .unwrap();
    assert!(caps.supports_streaming);
    assert!(caps.supports_thinking);
}

#[test]
fn resolve_unknown_provider() {
    let reg = anthropic_registry();
    let result = reg.resolve("openai:gpt-4o");
    assert!(matches!(result, Err(RegistryError::UnknownProvider(ref s)) if s == "openai"));
}

#[test]
fn resolve_unknown_model() {
    let reg = anthropic_registry();
    let result = reg.resolve("anthropic:nonexistent-model");
    assert!(
        matches!(result, Err(RegistryError::UnknownModel { ref provider, .. }) if provider == "anthropic")
    );
}

#[test]
fn resolve_missing_colon() {
    let reg = anthropic_registry();
    let result = reg.resolve("just-a-string");
    assert!(matches!(result, Err(RegistryError::InvalidSpec(_))));
}

#[test]
fn resolve_empty_spec() {
    let reg = anthropic_registry();
    let result = reg.resolve("");
    assert!(matches!(result, Err(RegistryError::InvalidSpec(_))));
}

// ---------------------------------------------------------------------------
// Capabilities
// ---------------------------------------------------------------------------

#[test]
fn capabilities_context_window() {
    let reg = anthropic_registry();
    let caps = reg
        .capabilities("anthropic:claude-sonnet-4-5-20250514")
        .unwrap();
    assert_eq!(caps.context_window, 200000);
    assert_eq!(caps.max_output_tokens, 8192);
}

#[test]
fn capabilities_unknown_provider() {
    let reg = anthropic_registry();
    let result = reg.capabilities("google:gemini-pro");
    assert!(matches!(result, Err(RegistryError::UnknownProvider(_))));
}

// ---------------------------------------------------------------------------
// Resolve partial model name (exact match required)
// ---------------------------------------------------------------------------

#[test]
fn resolve_requires_exact_model_id() {
    let reg = anthropic_registry();
    let result = reg.resolve("anthropic:claude-sonnet-4");
    assert!(matches!(result, Err(RegistryError::UnknownModel { .. })));
}

// ---------------------------------------------------------------------------
// Provider access
// ---------------------------------------------------------------------------

#[test]
fn get_provider_by_id() {
    let reg = anthropic_registry();
    let provider = reg.get_provider("anthropic").unwrap();
    assert_eq!(provider.id(), "anthropic");
    assert!(!provider.models().is_empty());
}

#[test]
fn get_provider_unknown() {
    let reg = anthropic_registry();
    assert!(reg.get_provider("nonexistent").is_none());
}

// ---------------------------------------------------------------------------
// Phase 10.1 regression: every built-in provider still resolves (SC1).
// ---------------------------------------------------------------------------

#[test]
fn registry_resolves_all_builtin_providers() {
    use std::sync::Arc;

    use opi_ai::anthropic::AnthropicProvider;
    use opi_ai::azure_openai::AzureOpenAIProvider;
    use opi_ai::bedrock::BedrockProvider;
    use opi_ai::bedrock::sigv4::AwsCredentials;
    use opi_ai::gemini::GeminiProvider;
    use opi_ai::http::HttpClient;
    use opi_ai::mistral::mistral_provider;
    use opi_ai::openai_chat::OpenAiChatProvider;
    use opi_ai::openai_responses::OpenAiResponsesProvider;
    use opi_ai::openrouter::openrouter_provider;
    use opi_ai::vertex::VertexProvider;

    let dummy_key = "test-key".to_string();
    let bedrock_creds = AwsCredentials {
        access_key_id: "AKIATEST".into(),
        secret_access_key: "secret".into(),
        session_token: None,
        region: "us-east-1".into(),
    };
    let azure = AzureOpenAIProvider::from_config(
        dummy_key.clone(),
        Some("https://example.openai.azure.com".into()),
        vec!["gpt-4o".into()],
        None,
    )
    .unwrap();

    let mut reg = ProviderRegistry::new();
    reg.register_provider(Box::new(AnthropicProvider::new(dummy_key.clone(), None)))
        .unwrap();
    reg.register_provider(Box::new(OpenAiChatProvider::new(dummy_key.clone(), None)))
        .unwrap();
    reg.register_provider(Box::new(openrouter_provider(dummy_key.clone(), None)))
        .unwrap();
    reg.register_provider(Box::new(mistral_provider(dummy_key.clone(), None)))
        .unwrap();
    reg.register_provider(Box::new(OpenAiResponsesProvider::new(
        dummy_key.clone(),
        None,
    )))
    .unwrap();
    reg.register_provider(Box::new(GeminiProvider::new(dummy_key.clone(), None)))
        .unwrap();
    reg.register_provider(Box::new(BedrockProvider::new(
        bedrock_creds,
        None,
        Arc::new(HttpClient::new()),
    )))
    .unwrap();
    reg.register_provider(Box::new(azure)).unwrap();
    reg.register_provider(Box::new(VertexProvider::new(
        "test-token".into(),
        "test-project".into(),
        "us-central1".into(),
        None,
    )))
    .unwrap();

    // The full built-in provider set is present.
    assert_eq!(
        reg.provider_ids(),
        vec![
            "anthropic",
            "azure",
            "bedrock",
            "gemini",
            "mistral",
            "openai",
            "openai-responses",
            "openrouter",
            "vertex"
        ]
    );

    // Each built-in provider resolves its first advertised model spec.
    for provider_id in reg.provider_ids() {
        let provider = reg.get_provider(provider_id).unwrap();
        let first_model = provider
            .models()
            .first()
            .unwrap_or_else(|| panic!("provider '{provider_id}' advertises no models"));
        let spec = format!("{}:{}", provider_id, first_model.id);
        let (resolved_provider, resolved_model) = reg
            .resolve(&spec)
            .unwrap_or_else(|e| panic!("resolve {spec} failed: {e}"));
        assert_eq!(resolved_provider.id(), provider_id);
        assert_eq!(resolved_model.id, first_model.id);
    }
}
