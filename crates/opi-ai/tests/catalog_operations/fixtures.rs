//! Synthetic catalog records and provider setup shared within this target.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use opi_ai::api::openrouter_images::{OPENROUTER_IMAGES_API_ID, OpenRouterImagesApi};
use opi_ai::api::system_one::{TYPESAFE_SYSTEM_ONE_API_ID, TypesafeSystemOneApi};
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::catalog::ModelsStore;
use opi_ai::classifier::{ClassifierInputModality, ClassifierModelRecord};
use opi_ai::http::HttpClient;
use opi_ai::images::{ImageCostRates, ImageModelRecord, ImageOutputModality};
use opi_ai::models::Models;
use opi_ai::provider::{ProviderError, Request};
use opi_ai::types::ApiId;
use opi_ai::{AuthProvenanceSource, ModelCapabilities, ModelInfo, ModelPricing, WireApi};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub(super) struct FixtureAuth(pub(super) AtomicUsize);

impl AuthResolver for FixtureAuth {
    fn resolve<'a>(&'a self) -> opi_ai::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            Ok(ResolvedAuth {
                scheme: AuthScheme::Bearer,
                secret: secrecy::SecretString::from("catalog-fixture-key"),
                base_url: None,
                account_id: None,
                env: Default::default(),
                provenance: Default::default(),
            })
        })
    }
}

pub(super) fn registered_collection(
    store: ModelsStore,
    endpoint: &str,
    auth: Arc<FixtureAuth>,
) -> Models {
    let mut models = Models::new();
    let chat = ModelInfo::new(
        "baseline",
        "Baseline",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(8192, 1024).with_streaming(true),
    );
    models
        .register_route(
            Box::new(opi_ai::providers::openai::OpenAiChatProvider::for_route(
                Some(endpoint.into()),
                "fixture".into(),
                Default::default(),
                vec![chat],
                Arc::new(HttpClient::new()),
            )),
            auth.clone(),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    models
        .register_image_api(
            Arc::new(OpenRouterImagesApi::new(
                HttpClient::new(),
                ImageModelRecord {
                    cost_tiers: Vec::new(),
                    headers: Vec::new(),
                    input_limits: None,
                    input: vec![
                        opi_ai::catalog::CatalogInputModality::Text,
                        opi_ai::catalog::CatalogInputModality::Image,
                    ],
                    provider_id: "fixture".into(),
                    id: "baseline".into(),
                    name: "Baseline image".into(),
                    api: ApiId::new(OPENROUTER_IMAGES_API_ID),
                    base_url: format!("{endpoint}/old-image"),
                    output: vec![ImageOutputModality::Image],
                    cost: Some(ImageCostRates {
                        input: 0.0,
                        output: 0.0,
                        cache_read: 0.0,
                        cache_write: 0.0,
                    }),
                },
            )),
            auth.clone(),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    models
        .register_classifier_api(
            Arc::new(TypesafeSystemOneApi::new(
                HttpClient::new(),
                ClassifierModelRecord {
                    headers: Vec::new(),
                    input_limits: None,
                    compat: None,
                    provider_id: "fixture".into(),
                    id: "baseline".into(),
                    name: "Baseline classifier".into(),
                    api: ApiId::new(TYPESAFE_SYSTEM_ONE_API_ID),
                    base_url: format!("{endpoint}/old-classifier"),
                    input: vec![ClassifierInputModality::Text],
                    context_window: 8192,
                    cost: Some(ModelPricing::try_new(Default::default(), vec![]).unwrap()),
                },
            )),
            auth,
            AuthProvenanceSource::Static,
        )
        .unwrap();
    models.set_models_store(store);
    models
}

pub(super) fn document(endpoint: &str) -> Value {
    let cost = json!({"input": 1, "output": 2, "cacheRead": 0.25, "cacheWrite": 0.5,
    "tiers": [
        {"inputTokensAbove": 0, "input": 99, "output": 99, "cacheRead": 0, "cacheWrite": 0},
        {"inputTokensAbove": 0, "input": 88, "output": 88, "cacheRead": 0, "cacheWrite": 0}
    ]});
    json!({"providers": {"fixture": {
        "lastModified": 1700000000123_i64, "checkedAt": 1700000000456_i64, "etag": "W/\"catalog-v1\"",
        "models": [
            {"type": "chat", "id": "shared", "name": "Remote chat", "api": "openai-completions",
                "baseUrl": endpoint, "contextWindow": 8192, "maxTokens": 512,
                "input": ["text"], "reasoning": false, "cost": cost,
                "compat": {"supportsStore":false, "supportsUsageInStreaming":false,
                    "vllmPriority":0.125, "openRouterRouting":{"allow_fallbacks":false,
                        "only":[],"max_price":{"prompt":0,"completion":"0.5"},
                        "sort":{"partition":null}},
                    "vercelGatewayRouting":{"only":[]}},
                "x-opi": {"streaming": true}},
            {"type": "image", "id": "shared", "name": "Remote image", "api": "openrouter-images",
                "baseUrl": format!("{endpoint}/image"), "output": ["image", "text"], "cost": cost,
                "input": ["text", "image"], "headers": {"x-catalog": "image", "x-remove": null},
                "x-opi": {"headers": [["x-catalog", "image"], ["x-override", "model"],
                    ["x-remove", "on"], ["X-Remove", "off"], ["x-remove", null]]},
                "inputLimits": {"images": {"maxPerRequest": 7}}},
            {"type": "classifier", "id": "shared", "name": "Remote classifier", "api": "typesafe-system-one",
                "baseUrl": format!("{endpoint}/classifier"), "input": ["text"], "contextWindow": 4096, "cost": cost,
                "headers": {"x-catalog": "classifier", "x-remove": null},
                "x-opi": {"headers": [["x-catalog", "classifier"], ["x-override", "model"],
                    ["x-remove", "on"], ["X-Remove", "off"], ["x-remove", null]]},
                "inputLimits": {"images": {"maxPerRequest": 3}}}
        ]
    }}})
}

pub(super) fn request() -> Request {
    Request {
        projection: Default::default(),
        model: "fixture:shared".into(),
        context: opi_ai::transcript::normalize_context(Default::default()).unwrap(),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        callbacks: Default::default(),
        telemetry_context: None,
        transport: Default::default(),
    }
}

pub(super) mod anthropic {
    use opi_ai::ModelInfo;
    use opi_ai::catalog::{CatalogRecord, ModelsStore, ModelsStoreEntry};
    use opi_ai::model_info::{AnthropicMessagesCompat, WireCompat};
    use serde_json::{Value, json};

    pub(crate) fn entry(compat: Option<Value>) -> Value {
        let mut entry = json!({
            "id": "source-model", "name": "Source model", "api": "anthropic-messages",
            "contextWindow": 8192, "maxTokens": 1024,
            "x-opi": {"streaming": true}
        });
        if let Some(compat) = compat {
            entry["compat"] = compat;
        }
        entry
    }

    pub(crate) fn decode(entry: Value) -> Result<ModelInfo, String> {
        let mut entries = opi_ai::catalog::decode_document(&json!({
            "providers": {"anthropic": {"models": [entry]}}
        }))
        .map_err(|error| error.to_string())?;
        let record = entries.remove("anthropic").unwrap().models.remove(0);
        let CatalogRecord::Chat(model) = record else {
            panic!("Anthropic metadata is a Chat record")
        };
        Ok(*model)
    }

    pub(crate) fn compat(model: &ModelInfo) -> &AnthropicMessagesCompat {
        let WireCompat::AnthropicMessages(compat) = &model.compat else {
            panic!("matching Anthropic compatibility")
        };
        compat
    }

    pub(crate) fn old_owned_compat() -> Value {
        json!({
            "api": "anthropic-messages",
            "supportsEagerToolInputStreaming": false,
            "forceAdaptiveThinking": false,
            "supportsTemperature": false
        })
    }

    pub(crate) fn fallback() -> Value {
        json!({
            "provider": "anthropic", "model": "fallback-model",
            "cost": {"input": 5, "output": 25, "cacheRead": 0.5, "cacheWrite": 6.25}
        })
    }

    pub(crate) async fn persist(model: ModelInfo) -> (Value, ModelInfo) {
        let temp = tempfile::tempdir().unwrap();
        let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
        store
            .begin()
            .commit(
                [(
                    "anthropic".into(),
                    ModelsStoreEntry {
                        models: vec![CatalogRecord::from(model.clone())],
                        ..Default::default()
                    },
                )]
                .into_iter()
                .collect(),
            )
            .await
            .unwrap();
        let document = serde_json::from_slice(&std::fs::read(store.path()).unwrap()).unwrap();
        let reopened = ModelsStore::open(store.path()).unwrap().snapshot();
        let CatalogRecord::Chat(restored) = &reopened.catalogs["anthropic"].models[0] else {
            panic!("restored Chat metadata")
        };
        assert_eq!(restored.as_ref(), &model);
        (document, restored.as_ref().clone())
    }
}
