//! Provider/model registration tests (task 4.6).
//!
//! Tests verify that custom providers and model overrides can be registered
//! into the ProviderRegistry and that model resolution, capability queries,
//! streaming, and model listing all work through the existing contracts.
//! All tests use MockProvider — no live provider calls.

use opi_ai::provider::ProviderError;
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
        callbacks: Default::default(),
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

// ---------------------------------------------------------------------------
// Task 20.3: single Models, frozen binding, typed custom chat calls
// (AIR-045 / P20-S03 / P20-ACTOR-02)
// ---------------------------------------------------------------------------

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use opi_ai::Models;
use opi_ai::auth::{AuthProvenanceSource, AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::credential::BoxAuthFuture;
use opi_ai::message::AssistantContent;
use opi_ai::models::{ApiId, ChatApi, ChatModelRecord, SimpleOptions};
use opi_ai::provider::{Request, ThinkingConfig};
use secrecy::{ExposeSecret, SecretString};
use tokio_util::sync::CancellationToken;

fn assistant_text(message: &opi_ai::message::AssistantMessage) -> String {
    message
        .content
        .iter()
        .filter_map(|content| match content {
            AssistantContent::Text { text } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Typed native options for the echo adapter: the label must reach the
/// adapter unchanged.
#[derive(Clone, Debug, PartialEq)]
struct EchoOptions {
    label: String,
}

/// A second, incompatible options type for the same ApiId.
#[derive(Clone, Debug)]
struct OtherOptions {
    #[expect(dead_code)]
    value: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct ObservedCall {
    request_model: String,
    max_tokens: Option<u64>,
    temperature: Option<f64>,
    options_label: String,
    auth_secret: String,
}

type Observed = Arc<Mutex<Vec<ObservedCall>>>;

/// One custom chat adapter: records the effective request, options, and
/// resolved-auth secret, then completes with a fixed text.
struct EchoChatApi {
    api: ApiId,
    provider_id: String,
    record: ModelInfo,
    text: String,
    observed: Observed,
}

impl ChatApi<EchoOptions> for EchoChatApi {
    fn api_id(&self) -> &ApiId {
        &self.api
    }

    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn complete_record(&self) -> ModelInfo {
        self.record.clone()
    }

    fn simple_options(&self) -> EchoOptions {
        EchoOptions {
            label: "simple".into(),
        }
    }

    fn stream_prepared(
        &self,
        request: Request,
        options: EchoOptions,
        auth: ResolvedAuth,
    ) -> opi_ai::provider::EventStream {
        self.observed.lock().unwrap().push(ObservedCall {
            request_model: request.model,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            options_label: options.label,
            auth_secret: auth.secret.expose_secret().to_owned(),
        });
        let events = text_response(&self.text);
        Box::pin(futures_util::stream::iter(events.into_iter().map(Ok)))
    }
}

/// An adapter with an incompatible options type, registered under the same
/// ApiId to prove changed-type failures.
struct TypedChatApi {
    api: ApiId,
    provider_id: String,
    record: ModelInfo,
}

impl ChatApi<OtherOptions> for TypedChatApi {
    fn api_id(&self) -> &ApiId {
        &self.api
    }

    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn complete_record(&self) -> ModelInfo {
        self.record.clone()
    }

    fn simple_options(&self) -> OtherOptions {
        OtherOptions { value: 0 }
    }

    fn stream_prepared(
        &self,
        _request: Request,
        _options: OtherOptions,
        _auth: ResolvedAuth,
    ) -> opi_ai::provider::EventStream {
        let events = text_response("typed");
        Box::pin(futures_util::stream::iter(events.into_iter().map(Ok)))
    }
}

struct CountingResolver {
    calls: AtomicUsize,
}

impl AuthResolver for CountingResolver {
    fn resolve(&self) -> BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            Ok(ResolvedAuth {
                scheme: AuthScheme::ApiKey,
                secret: SecretString::from("resolver-secret"),
                base_url: None,
                account_id: None,
                provenance: opi_ai::AuthProvenance::default(),
            })
        })
    }
}

fn echo_record_info(max_output: u64) -> ModelInfo {
    ModelInfo::new(
        "echo-model",
        "Echo model",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(100_000, max_output).with_streaming(true),
    )
}

fn models_request() -> Request {
    Request {
        transport: Default::default(),
        model: String::new(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("be terse".into()),
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: opi_ai::provider::CacheRetention::None,
        session_id: None,
        callbacks: Default::default(),
    }
}

fn observed_call(observed: &Observed) -> ObservedCall {
    observed.lock().unwrap().last().unwrap().clone()
}

fn counting_resolver() -> Arc<CountingResolver> {
    Arc::new(CountingResolver {
        calls: AtomicUsize::new(0),
    })
}

#[tokio::test]
async fn models_register_chat_api_keeps_record_out_of_catalog() {
    let mut models = Models::new();
    let observed: Observed = Arc::default();
    let record_info = echo_record_info(2_048);
    let adapter = EchoChatApi {
        api: ApiId::new("custom-chat-v1"),
        provider_id: "custom".into(),
        record: record_info.clone(),
        text: "api response".into(),
        observed: observed.clone(),
    };
    let handle = models
        .register_chat_api(
            Arc::new(adapter),
            counting_resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // Exact ApiId identity is preserved on the registration handle.
    assert_eq!(handle.api_id().as_str(), "custom-chat-v1");
    // The custom complete record stays absent from every catalog read.
    assert_eq!(models.get_model("custom", "echo-model"), None);
    assert!(
        models
            .registry()
            .all_models()
            .iter()
            .all(|(provider, _)| *provider != "custom")
    );

    // ... yet it is callable on the registered provider/wire.
    let record = ChatModelRecord::new("custom", record_info);
    let message = models
        .complete(
            record,
            models_request(),
            handle.options(EchoOptions {
                label: "native".into(),
            }),
        )
        .await
        .unwrap();
    assert_eq!(assistant_text(&message), "api response");
    let call = observed_call(&observed);
    assert_eq!(call.request_model, "custom:echo-model");
    assert_eq!(call.options_label, "native");
    assert_eq!(call.auth_secret, "resolver-secret");
}

#[tokio::test]
async fn models_get_model_returns_owned_record_with_exact_wire() {
    let mut models = Models::new();
    let provider = custom_provider("owned", vec![custom_model("owned-model", "Owned Model")]);
    models
        .register_route(
            provider,
            counting_resolver(),
            AuthProvenanceSource::Static,
            opi_ai::CompatMetadata::default(),
        )
        .unwrap();

    let owned = models
        .get_model("owned", "owned-model")
        .expect("catalog record");
    assert_eq!(owned.id, "owned-model");
    assert_eq!(owned.wire_api, WireApi::OpenAiCompletions);
    assert_eq!(models.get_model("owned", "missing"), None);
    assert_eq!(models.get_model("missing", "owned-model"), None);
}

#[tokio::test]
async fn models_frozen_record_is_not_replaced_by_same_id_catalog_data() {
    let mut models = Models::new();
    let observed: Observed = Arc::default();
    let record_info = echo_record_info(1_024);
    let adapter = EchoChatApi {
        api: ApiId::new("custom-chat-v1"),
        provider_id: "custom".into(),
        record: record_info.clone(),
        text: "frozen".into(),
        observed: observed.clone(),
    };
    let handle = models
        .register_chat_api(
            Arc::new(adapter),
            counting_resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // A catalog model with the SAME provider+model id but a larger output
    // limit exists; it must not replace the caller's complete record. The
    // route registration replaces the adapter's catalog-invisible route
    // provider while the typed ApiId registration keeps serving the native
    // entry.
    let catalog_model = ModelInfo::new(
        "echo-model",
        "Catalog Echo",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 8_192).with_streaming(true),
    );
    models
        .register_route(
            custom_provider("custom", vec![catalog_model]),
            counting_resolver(),
            AuthProvenanceSource::Static,
            opi_ai::CompatMetadata::default(),
        )
        .unwrap();
    let catalog = models
        .get_model("custom", "echo-model")
        .expect("catalog record present");
    assert_eq!(catalog.capabilities.max_output_tokens, 8_192);

    let record = ChatModelRecord::new("custom", record_info);
    models
        .complete(
            record,
            models_request(),
            handle.options(EchoOptions {
                label: "native".into(),
            }),
        )
        .await
        .unwrap();
    let call = observed_call(&observed);
    // The frozen caller record's limit (1,024) reached the adapter; the
    // catalog record's larger default did not replace it.
    assert_eq!(call.max_tokens, Some(1_024));
}

#[tokio::test]
async fn models_simple_entry_uses_neutral_options_and_same_binding() {
    let mut models = Models::new();
    let observed: Observed = Arc::default();
    let record_info = echo_record_info(2_048);
    let adapter = EchoChatApi {
        api: ApiId::new("custom-chat-v1"),
        provider_id: "custom".into(),
        record: record_info.clone(),
        text: "simple response".into(),
        observed: observed.clone(),
    };
    models
        .register_chat_api(
            Arc::new(adapter),
            counting_resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    let record = ChatModelRecord::new("custom", record_info);
    let message = models
        .complete_simple(
            record,
            models_request(),
            SimpleOptions {
                temperature: Some(0.25),
                ..SimpleOptions::default()
            },
        )
        .await
        .unwrap();
    let call = observed_call(&observed);
    assert_eq!(call.options_label, "simple");
    assert_eq!(call.temperature, Some(0.25));
    assert_eq!(call.request_model, "custom:echo-model");
    assert_eq!(assistant_text(&message), "simple response");
    assert_eq!(call.auth_secret, "resolver-secret");
}

#[tokio::test]
async fn models_direct_entry_uses_injected_auth_without_models_auth() {
    let mut models = Models::new();
    let observed: Observed = Arc::default();
    let record_info = echo_record_info(2_048);
    let adapter = EchoChatApi {
        api: ApiId::new("custom-chat-v1"),
        provider_id: "custom".into(),
        record: record_info.clone(),
        text: "direct response".into(),
        observed: observed.clone(),
    };
    let reads = counting_resolver();
    let handle = models
        .register_chat_api(
            Arc::new(adapter),
            reads.clone() as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
        )
        .unwrap();

    let auth = ResolvedAuth {
        scheme: AuthScheme::Bearer,
        secret: SecretString::from("direct-secret"),
        base_url: None,
        account_id: None,
        provenance: opi_ai::AuthProvenance::default(),
    };
    let record = ChatModelRecord::new("custom", record_info);
    models
        .complete_direct(
            record,
            models_request(),
            handle.options(EchoOptions {
                label: "direct".into(),
            }),
            auth,
        )
        .await
        .unwrap();

    let call = observed_call(&observed);
    assert_eq!(call.auth_secret, "direct-secret");
    assert_eq!(reads.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn models_native_options_portable_across_instances() {
    let record_info = echo_record_info(2_048);
    let mut first = Models::new();
    let observed_a: Observed = Arc::default();
    let handle = first
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info.clone(),
                text: "instance-a".into(),
                observed: observed_a.clone(),
            }),
            counting_resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    let mut second = Models::new();
    let observed_b: Observed = Arc::default();
    second
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info.clone(),
                text: "instance-b".into(),
                observed: observed_b.clone(),
            }),
            counting_resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // The same option value (created from the FIRST instance's handle)
    // crosses to the second instance with a compatible registration.
    let options = handle.options(EchoOptions {
        label: "portable".into(),
    });
    let record = ChatModelRecord::new("custom", record_info.clone());
    let a = first
        .complete(record.clone(), models_request(), options.clone())
        .await
        .unwrap();
    let b = second
        .complete(record, models_request(), options)
        .await
        .unwrap();
    assert_eq!(assistant_text(&a), "instance-a");
    assert_eq!(assistant_text(&b), "instance-b");
    assert_eq!(observed_call(&observed_a).options_label, "portable");
    assert_eq!(observed_call(&observed_b).options_label, "portable");
}

#[tokio::test]
async fn models_old_call_survives_provider_mutation() {
    let mut models = Models::new();
    let observed: Observed = Arc::default();
    let record_info = echo_record_info(2_048);
    let handle = models
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info.clone(),
                text: "before-mutation".into(),
                observed: observed.clone(),
            }),
            counting_resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // Create (but do not poll) a call; then replace the registration.
    let record = ChatModelRecord::new("custom", record_info.clone());
    let call = models.stream(
        record.clone(),
        models_request(),
        handle.options(EchoOptions {
            label: "frozen".into(),
        }),
    );

    models
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info,
                text: "after-mutation".into(),
                observed: Arc::default(),
            }),
            counting_resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // The already-created call keeps its captured binding.
    let message = call.result().await.unwrap();
    assert_eq!(assistant_text(&message), "before-mutation");

    // A NEW call on the same handle observes the replacement.
    let message = models
        .complete(
            record,
            models_request(),
            handle.options(EchoOptions {
                label: "fresh".into(),
            }),
        )
        .await
        .unwrap();
    assert_eq!(assistant_text(&message), "after-mutation");
}

#[tokio::test]
async fn models_completion_entries_capture_owned_binding_before_first_poll() {
    fn owned<F: std::future::Future + Send + 'static>(future: F) -> F {
        future
    }

    let mut models = Models::new();
    let observed: Observed = Arc::default();
    let record_info = echo_record_info(2_048);
    let reads = counting_resolver();
    let handle = models
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info.clone(),
                text: "before-mutation".into(),
                observed: observed.clone(),
            }),
            reads.clone() as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let record = ChatModelRecord::new("custom", record_info.clone());
    let native = owned(models.complete(
        record.clone(),
        models_request(),
        handle.options(EchoOptions {
            label: "native".into(),
        }),
    ));
    let simple =
        owned(models.complete_simple(record.clone(), models_request(), SimpleOptions::default()));
    let direct = owned(models.complete_direct(
        record.clone(),
        models_request(),
        handle.options(EchoOptions {
            label: "direct".into(),
        }),
        ResolvedAuth {
            scheme: AuthScheme::Bearer,
            secret: SecretString::from("direct-secret"),
            base_url: None,
            account_id: None,
            provenance: opi_ai::AuthProvenance::default(),
        },
    ));
    assert_eq!(reads.calls.load(Ordering::SeqCst), 0);
    assert!(observed.lock().unwrap().is_empty());

    let replacement_reads = counting_resolver();
    models
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info,
                text: "after-mutation".into(),
                observed: Arc::default(),
            }),
            replacement_reads.clone() as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let fresh = owned(models.complete_simple(record, models_request(), SimpleOptions::default()));
    drop(models);

    for future in [
        tokio::spawn(native),
        tokio::spawn(simple),
        tokio::spawn(direct),
    ] {
        assert_eq!(
            assistant_text(&future.await.unwrap().unwrap()),
            "before-mutation"
        );
    }
    assert_eq!(
        assistant_text(&tokio::spawn(fresh).await.unwrap().unwrap()),
        "after-mutation"
    );
    assert_eq!(reads.calls.load(Ordering::SeqCst), 2);
    assert_eq!(replacement_reads.calls.load(Ordering::SeqCst), 1);
    let calls = observed.lock().unwrap();
    assert_eq!(calls.len(), 3);
    let direct = calls
        .iter()
        .find(|call| call.options_label == "direct")
        .unwrap();
    assert_eq!(direct.auth_secret, "direct-secret");
}

#[tokio::test]
async fn models_type_mismatch_and_unregistered_api_fail_before_auth() {
    let mut models = Models::new();
    let observed: Observed = Arc::default();
    let record_info = echo_record_info(2_048);
    let reads = counting_resolver();
    let handle = models
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info.clone(),
                text: "unreachable".into(),
                observed: observed.clone(),
            }),
            reads.clone() as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let record = ChatModelRecord::new("custom", record_info.clone());

    // Upsert the same ApiId with an incompatible options type.
    models
        .register_chat_api(
            Arc::new(TypedChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info.clone(),
            }),
            counting_resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // Options created before the upsert now fail the type check pre-auth.
    let failure = models
        .complete(
            record.clone(),
            models_request(),
            handle.options(EchoOptions {
                label: "stale".into(),
            }),
        )
        .await
        .unwrap_err();
    assert!(
        failure.cause().to_string().contains("does not match"),
        "{}",
        failure.cause()
    );
    // The failed preflight never reached the replaced adapter or auth.
    assert!(observed.lock().unwrap().is_empty());
    assert_eq!(reads.calls.load(Ordering::SeqCst), 0);

    // Options from another instance's handle (different ApiId) fail as
    // unregistered on this instance, also before auth.
    let mut fresh = Models::new();
    let orphan = fresh
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("orphan-chat"),
                provider_id: "orphan".into(),
                record: record_info.clone(),
                text: "orphan".into(),
                observed: Arc::default(),
            }),
            counting_resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let failure = models
        .complete(
            record.clone(),
            models_request(),
            orphan.options(EchoOptions {
                label: "orphan".into(),
            }),
        )
        .await
        .unwrap_err();
    assert!(
        failure.cause().to_string().contains("not registered"),
        "{}",
        failure.cause()
    );

    // A record whose provider does not own the API fails pre-auth as well.
    let failure = models
        .complete(
            ChatModelRecord::new("elsewhere", record_info),
            models_request(),
            handle.options(EchoOptions {
                label: "mismatched".into(),
            }),
        )
        .await
        .unwrap_err();
    assert!(
        failure.cause().to_string().contains("does not match"),
        "{}",
        failure.cause()
    );
    assert_eq!(reads.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn models_auth_starts_on_first_poll_and_call_is_send_static() {
    let mut models = Models::new();
    let observed: Observed = Arc::default();
    let record_info = echo_record_info(2_048);
    let reads = counting_resolver();
    let handle = models
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info.clone(),
                text: "lazy auth".into(),
                observed,
            }),
            reads.clone() as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
        )
        .unwrap();

    let record = ChatModelRecord::new("custom", record_info);
    let call = models.stream(
        record,
        models_request(),
        handle.options(EchoOptions {
            label: "lazy".into(),
        }),
    );
    // Construction captured the binding without auth IO.
    assert_eq!(reads.calls.load(Ordering::SeqCst), 0);

    // The call is Send + 'static: it can cross into a fresh runtime task.
    let first_poll_happened = tokio::spawn(async move {
        let mut call = call;
        let first = call.next().await;
        let saw_event = first.is_some();
        let message = call.result().await.unwrap();
        (saw_event, assistant_text(&message))
    })
    .await
    .unwrap();
    assert!(first_poll_happened.0);
    assert_eq!(first_poll_happened.1, "lazy auth");
    assert_eq!(reads.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn models_cancel_before_first_poll_fails_typed() {
    let mut models = Models::new();
    let record_info = echo_record_info(2_048);
    let reads = counting_resolver();
    let handle = models
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info.clone(),
                text: "cancelled".into(),
                observed: Arc::default(),
            }),
            reads.clone() as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
        )
        .unwrap();

    let request = models_request();
    request.cancel.cancel();
    let record = ChatModelRecord::new("custom", record_info);
    let failure = models
        .complete(
            record,
            request,
            handle.options(EchoOptions {
                label: "never".into(),
            }),
        )
        .await
        .unwrap_err();
    // A cancelled logical call fails typed with the cancellation cause
    // (mapped to Aborted by CallFailure), before any auth resolution.
    assert!(matches!(
        failure.cause(),
        ProviderError::Cancelled | ProviderError::LoginCancelled { .. }
    ));
    assert_eq!(failure.message().stop_reason.as_str(), "aborted");
    assert_eq!(reads.calls.load(Ordering::SeqCst), 0);
}
