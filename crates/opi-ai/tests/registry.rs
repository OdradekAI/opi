//! Behavioral tests for the provider registry (task 1.4).
//!
//! DoD: "resolves anthropic:model and capabilities"

use opi_ai::WireApi;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::provider::{EventStream, ModelInfo, Provider, Request};
use opi_ai::registry::ModelCapabilities;
use opi_ai::registry::{ProviderRegistry, RegistrationError, RegistryError};

#[test]
fn unchecked_registration_alias_is_not_public() {
    let source = include_str!("../src/registry.rs");
    assert!(!source.contains("pub fn register(&mut self"));
}

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
    fn stream_prepared(&self, _request: Request, _auth: opi_ai::auth::ResolvedAuth) -> EventStream {
        Box::pin(futures_util::stream::empty())
    }
}

/// Helper: create a registry with the Anthropic provider registered.
fn anthropic_registry() -> ProviderRegistry {
    let mut reg = ProviderRegistry::new();
    reg.register_provider(Box::new(AnthropicProvider::new(None)))
        .unwrap();
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
    reg.register_provider(Box::new(StubProvider::new("alpha")))
        .unwrap();
    reg.register_provider(Box::new(StubProvider::new("beta")))
        .unwrap();
    let mut ids = reg.provider_ids();
    ids.sort();
    assert_eq!(ids, vec!["alpha", "beta"]);
}

#[test]
fn duplicate_registration_replaces() {
    let mut reg = ProviderRegistry::new();
    reg.register_provider(Box::new(StubProvider::new("p")))
        .unwrap();
    reg.register_provider(Box::new(StubProvider::new("p")))
        .unwrap();
    assert_eq!(reg.provider_ids().len(), 1);
}

#[test]
fn validated_registration_rejects_whitespace_only_provider_id() {
    let mut reg = ProviderRegistry::new();
    let error = reg
        .register_provider(Box::new(StubProvider::new("   ")))
        .expect_err("whitespace-only provider id must be rejected");
    assert!(matches!(error, RegistrationError::EmptyProviderId));
}

#[test]
fn validated_provider_replacement_clears_dynamic_catalog() {
    let mut reg = ProviderRegistry::new();
    reg.register_provider(Box::new(StubProvider::new("p")))
        .unwrap();
    reg.set_dynamic_catalog(
        "p",
        vec![ModelInfo::new(
            "dynamic",
            "Dynamic",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(128_000, 4_096),
        )],
    );
    assert!(reg.resolve("p:dynamic").is_ok());
    assert!(reg.resolve("p:p-model-1").is_err());

    reg.register_provider(Box::new(StubProvider::new("p")))
        .unwrap();

    assert!(reg.resolve("p:dynamic").is_err());
    assert!(reg.resolve("p:p-model-1").is_ok());
}

#[test]
fn compatibility_provider_replacement_clears_dynamic_catalog() {
    let mut reg = ProviderRegistry::new();
    reg.register_provider(Box::new(StubProvider::new("p")))
        .unwrap();
    reg.set_dynamic_catalog(
        "p",
        vec![ModelInfo::new(
            "dynamic",
            "Dynamic",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(128_000, 4_096),
        )],
    );
    assert!(reg.resolve("p:dynamic").is_ok());
    assert!(reg.resolve("p:p-model-1").is_err());

    reg.register_provider(Box::new(StubProvider::new("p")))
        .unwrap();

    assert!(reg.resolve("p:dynamic").is_err());
    assert!(reg.resolve("p:p-model-1").is_ok());
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

#[test]
fn resolve_uses_one_trimmed_canonical_provider_model_parser() {
    let mut reg = ProviderRegistry::new();
    reg.register_provider(Box::new(StubProvider::new("alpha")))
        .unwrap();

    let (provider, model) = reg
        .resolve("  alpha : alpha-model-1  ")
        .expect("canonical parser trims both identity components");
    assert_eq!(provider.id(), "alpha");
    assert_eq!(model.id, "alpha-model-1");

    for invalid in ["alpha:   ", "   :alpha-model-1", "   :   "] {
        assert!(
            matches!(reg.resolve(invalid), Err(RegistryError::InvalidSpec(_))),
            "invalid padded identity accepted: {invalid:?}"
        );
    }
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
    use opi_ai::gemini::GeminiProvider;
    use opi_ai::http::HttpClient;
    use opi_ai::mistral::mistral_provider;
    use opi_ai::openai_chat::OpenAiChatProvider;
    use opi_ai::openai_responses::OpenAiResponsesProvider;
    use opi_ai::openrouter::openrouter_provider;
    use opi_ai::vertex::VertexProvider;

    let azure = AzureOpenAIProvider::from_config(
        Some("https://example.openai.azure.com".into()),
        vec!["gpt-4o".into()],
        None,
    )
    .unwrap();

    let mut reg = ProviderRegistry::new();
    reg.register_provider(Box::new(AnthropicProvider::new(None)))
        .unwrap();
    reg.register_provider(Box::new(OpenAiChatProvider::new(None)))
        .unwrap();
    reg.register_provider(Box::new(openrouter_provider(None)))
        .unwrap();
    reg.register_provider(Box::new(mistral_provider(None)))
        .unwrap();
    reg.register_provider(Box::new(OpenAiResponsesProvider::new(None)))
        .unwrap();
    reg.register_provider(Box::new(GeminiProvider::new(None)))
        .unwrap();
    reg.register_provider(Box::new(BedrockProvider::new(
        None,
        Arc::new(HttpClient::new()),
    )))
    .unwrap();
    reg.register_provider(Box::new(azure)).unwrap();
    reg.register_provider(Box::new(VertexProvider::new(
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
// ---------------------------------------------------------------------------
// ModelsStore revision state machine and distribution-schema-v1 codec (20.33)
// ---------------------------------------------------------------------------

use std::collections::HashMap;

use opi_ai::catalog::{HttpCatalogSource, ModelsStore, StoreCommitError};
use opi_ai::stream::Pricing;
use opi_ai::{ModelPricing, PricingTier, ThinkingLevel, ThinkingLevelMapping};

fn store_catalogs() -> HashMap<String, Vec<ModelInfo>> {
    HashMap::from([(
        "prov-a".to_owned(),
        vec![ModelInfo::new(
            "m1",
            "Model One",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(128_000, 8_192),
        )],
    )])
}

#[tokio::test]
async fn models_store_commit_publishes_and_reopens_last_known() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("models-store.json");
    let store = ModelsStore::open(&path).unwrap();
    assert_eq!(store.revision(), 0, "fresh store starts at revision 0");

    let commit = store.begin().commit(store_catalogs()).await.unwrap();
    assert_eq!(commit.revision, 1);
    assert_eq!(store.revision(), 1);
    assert!(store.path().ends_with("models-store.json"));

    // Reopen: restore-before-network reads the last-known revision verbatim.
    let reopened = ModelsStore::open(&path).unwrap();
    assert_eq!(reopened.revision(), 1);
    let snapshot = reopened.snapshot();
    assert_eq!(snapshot.revision, 1);
    let restored = snapshot.catalogs.get("prov-a").unwrap();
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].id, "m1");
    assert_eq!(restored[0].display_name, "Model One");
    assert_eq!(restored[0].wire_api, WireApi::OpenAiCompletions);
    assert_eq!(restored[0].capabilities.context_window, 128_000);
    assert_eq!(restored[0].capabilities.max_output_tokens, 8_192);
}

#[tokio::test]
async fn models_store_file_uses_distribution_schema_v1_not_hydration_v6() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("models-store.json");
    let store = ModelsStore::open(&path).unwrap();
    store.begin().commit(store_catalogs()).await.unwrap();

    let raw: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        raw["$schema"], "https://pi.dev/schemas/models.schema.json",
        "store speaks the distribution schema identity"
    );
    assert!(
        raw.get("schemaVersion").is_none(),
        "store must not reuse the hydration data v6 envelope"
    );
    assert!(raw["revision"].is_u64(), "store carries its revision");
    assert_eq!(raw["providers"]["prov-a"]["models"][0]["id"], "m1");
    assert_eq!(
        raw["providers"]["prov-a"]["models"][0]["api"],
        "openai-completions"
    );
    assert_eq!(
        raw["providers"]["prov-a"]["models"][0]["contextWindow"],
        128_000
    );
    assert_eq!(raw["providers"]["prov-a"]["models"][0]["maxTokens"], 8_192);
}

#[tokio::test]
async fn models_store_rejects_superseded_late_commit_and_keeps_revision() {
    let dir = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(dir.path().join("s.json")).unwrap();

    // Guard A begins at revision 0, then B commits revision 1 first.
    let guard_a = store.begin();
    let guard_b = store.begin();
    guard_b.commit(store_catalogs()).await.unwrap();

    // A arrives late: its write predates the new revision and must be dropped
    // without rolling back the current publication.
    let late = HashMap::from([(
        "stale".to_owned(),
        vec![ModelInfo::new(
            "old",
            "Old",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(1_000, 100),
        )],
    )]);
    let err = guard_a.commit(late).await.unwrap_err();
    match err {
        StoreCommitError::Superseded {
            guard_revision: 0,
            store_revision: 1,
        } => {}
        other => panic!("expected superseded rejection, got {other:?}"),
    }
    assert_eq!(store.revision(), 1);
    assert!(
        !store.snapshot().catalogs.contains_key("stale"),
        "superseded write must not enter the published state"
    );

    // A guard created after the new revision is current and commits legally.
    store
        .begin()
        .commit(HashMap::from([(
            "prov-a".to_owned(),
            vec![ModelInfo::new(
                "m2",
                "Model Two",
                WireApi::OpenAiCompletions,
                ModelCapabilities::new(1_000, 100),
            )],
        )]))
        .await
        .unwrap();
    assert_eq!(store.revision(), 2);
}

#[tokio::test]
async fn models_store_rejects_cancelled_commit_without_touching_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.json");
    let store = ModelsStore::open(&path).unwrap();
    let token = tokio_util::sync::CancellationToken::new();
    let guard = store.begin_with(token.clone());
    token.cancel();
    let err = guard.commit(store_catalogs()).await.unwrap_err();
    assert!(matches!(err, StoreCommitError::Cancelled));
    assert_eq!(store.revision(), 0);
    assert!(!path.exists(), "cancelled commit must not persist anything");
}

#[test]
fn models_store_open_corrupt_file_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.json");
    std::fs::write(&path, b"{ not json").unwrap();
    let err = ModelsStore::open(&path).unwrap_err();
    let text = err.to_string();
    assert!(
        text.to_lowercase().contains("corrupt"),
        "corrupt store must surface a typed corrupt error, got: {text}"
    );
}

#[tokio::test]
async fn store_codec_round_trips_full_model_fidelity() {
    let full = ModelInfo::new(
        "f1",
        "Full Fidelity",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 16_384)
            .with_images(true)
            .with_streaming(true)
            .with_thinking(true)
            .with_cache_control(true)
            .with_long_cache_retention(true),
    )
    .with_base_url("https://example.test/v1")
    .with_thinking_level_map(opi_ai::ThinkingLevelMap::reasoning_default().with_mapping(
        ThinkingLevel::High,
        ThinkingLevelMapping::Mapped("think-hard".to_owned()),
    ));

    let mut priced = full;
    priced.pricing = Some(ModelPricing {
        base: Pricing {
            input_cost_per_mtok: 1.0,
            output_cost_per_mtok: 2.0,
            cache_read_cost_per_mtok: 0.5,
            cache_write_cost_per_mtok: 1.25,
        },
        tiers: vec![PricingTier {
            input_tokens_above: 100_000,
            pricing: Pricing {
                input_cost_per_mtok: 0.9,
                output_cost_per_mtok: 1.8,
                cache_read_cost_per_mtok: 0.45,
                cache_write_cost_per_mtok: 1.1,
            },
        }],
    });

    let dir = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(dir.path().join("s.json")).unwrap();
    store
        .begin()
        .commit(HashMap::from([("full".to_owned(), vec![priced.clone()])]))
        .await
        .unwrap();
    let restored = ModelsStore::open(store.path()).unwrap();
    let back = restored.snapshot().catalogs["full"][0].clone();

    assert_eq!(back.id, "f1");
    assert_eq!(back.display_name, "Full Fidelity");
    assert_eq!(back.wire_api, WireApi::OpenAiCompletions);
    assert_eq!(back.base_url.as_deref(), Some("https://example.test/v1"));
    assert_eq!(back.capabilities, priced.capabilities);
    assert_eq!(back.pricing, priced.pricing);
    for level in ThinkingLevel::ALL {
        assert_eq!(
            back.thinking_level_map.resolve(level).ok(),
            priced.thinking_level_map.resolve(level).ok(),
            "thinking map fidelity must survive the store round trip for {level:?}"
        );
    }
}

#[test]
fn v1_entry_without_extension_decodes_with_defaults() {
    // A plain distribution-schema-v1 document (no opi extension) still decodes
    // into usable ModelInfo: v1 fields win, opi-only capabilities default.
    let document = serde_json::json!({
        "providers": {
            "plain": {
                "models": [
                    {
                        "id": "p1",
                        "name": "Plain One",
                        "api": "anthropic-messages",
                        "contextWindow": 64_000,
                        "maxTokens": 4_096,
                        "input": ["text", "image"],
                        "cost": {
                            "input": 3.0, "output": 15.0,
                            "cacheRead": 0.3, "cacheWrite": 3.75
                        }
                    }
                ]
            }
        }
    });
    let catalogs = HttpCatalogSource::decode_document(&document).unwrap();
    let model = &catalogs["plain"][0];
    assert_eq!(model.id, "p1");
    assert_eq!(model.display_name, "Plain One");
    assert_eq!(model.wire_api, WireApi::AnthropicMessages);
    assert_eq!(model.capabilities.context_window, 64_000);
    assert_eq!(model.capabilities.max_output_tokens, 4_096);
    assert!(model.capabilities.supports_images);
    assert!(
        !model.capabilities.supports_streaming,
        "v1 carries no streaming flag; opi default is closed"
    );
    let pricing = model.pricing.as_ref().unwrap();
    assert_eq!(pricing.base.input_cost_per_mtok, 3.0);
    assert_eq!(pricing.base.output_cost_per_mtok, 15.0);
    assert!(pricing.tiers.is_empty());
}
