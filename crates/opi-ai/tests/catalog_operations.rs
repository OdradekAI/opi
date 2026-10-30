//! Operation-qualified catalog publication, persistence and real adapter calls.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use opi_ai::api::openrouter_images::{OPENROUTER_IMAGES_API_ID, OpenRouterImagesApi};
use opi_ai::api::system_one::{TYPESAFE_SYSTEM_ONE_API_ID, TypesafeSystemOneApi};
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::catalog::{HttpCatalogSource, ModelsStore};
use opi_ai::classifier::{
    ClassifierContext, ClassifierInputModality, ClassifierModelRecord, ClassifierQuestion,
    ClassifierStopReason,
};
use opi_ai::http::HttpClient;
use opi_ai::images::{
    ImageCostRates, ImageModelRecord, ImageOutputModality, ImagesContext, ImagesStopReason,
};
use opi_ai::models::{ApiId, Models};
use opi_ai::provider::{ProviderError, Request};
use opi_ai::{
    AuthProvenanceSource, ChatModelRecord, ModelCapabilities, ModelInfo, ModelPricing, WireApi,
};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[derive(Default)]
struct FixtureAuth(AtomicUsize);

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

fn collection(store: ModelsStore, endpoint: &str, auth: Arc<FixtureAuth>) -> Models {
    let mut models = registered_collection(store, endpoint, auth);
    models.register_catalog_source(Arc::new(HttpCatalogSource::new(format!(
        "{endpoint}/catalog"
    ))));
    models
}

fn registered_collection(store: ModelsStore, endpoint: &str, auth: Arc<FixtureAuth>) -> Models {
    let mut models = Models::new();
    let chat = ModelInfo::new(
        "baseline",
        "Baseline",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(8192, 1024).with_streaming(true),
    );
    models
        .register_route(
            Box::new(opi_ai::openai_chat::OpenAiChatProvider::for_route(
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

fn document(endpoint: &str) -> Value {
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

fn request() -> Request {
    Request {
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

#[test]
fn catalog_document_rejects_unknown_operation() {
    let mut document = document("http://127.0.0.1:1");
    document["providers"]["fixture"]["models"] = json!([{
        "type": "audio", "id": "unsupported", "api": "openai-completions",
        "contextWindow": 8192, "maxTokens": 512,
    }]);
    assert!(HttpCatalogSource::decode_document(&document).is_err());
}

#[tokio::test]
async fn provider_freshness_is_preserved_without_fabricating_missing_values() {
    let mut document = document("http://127.0.0.1:1");
    let chat = document["providers"]["fixture"]["models"][0].clone();
    document["providers"]["fixture"]["models"] = json!([chat]);
    document["providers"]["fixture"]["lastModified"] = json!(0);
    document["providers"]["fixture"]
        .as_object_mut()
        .unwrap()
        .remove("checkedAt");
    document["providers"]["fixture"]["etag"] = json!("\"opaque-validator\"");
    let decoded = HttpCatalogSource::decode_document(&document).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    store.begin().commit(decoded).await.unwrap();
    let reopened = ModelsStore::open(store.path()).unwrap();
    let persisted: Value =
        serde_json::from_slice(&std::fs::read(reopened.path()).unwrap()).unwrap();
    let entry = &persisted["providers"]["fixture"];
    assert_eq!(entry["lastModified"], 0);
    assert!(entry.get("checkedAt").is_none());
    assert_eq!(entry["etag"], "\"opaque-validator\"");
}

#[tokio::test]
async fn mixed_operation_catalog_reopens_into_three_real_models_calls() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/catalog"))
        .respond_with(ResponseTemplate::new(200).set_body_json(document(&server.uri())))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST")).and(path("/chat/completions"))
        .and(header("authorization", "Bearer catalog-fixture-key"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-type", "text/event-stream")
            .set_body_string("data: {\"id\":\"chat\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"chat ready\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n"))
        .expect(1).mount(&server).await;
    Mock::given(method("POST"))
        .and(path("/image/chat/completions"))
        .and(header("x-catalog", "image"))
        .and(header("x-override", "call"))
        .and(header("authorization", "Bearer catalog-fixture-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"choices": [{"message": {"content": "image ready"}}],
                    "usage": {"prompt_tokens": 4, "completion_tokens": 2, "total_tokens": 6}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/classifier/systemone"))
        .and(header("x-catalog", "classifier"))
        .and(header("x-override", "call"))
        .and(header("authorization", "Bearer catalog-fixture-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"answers": {"approved": {"type": "noul", "noul": 0.9}}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    let auth = Arc::new(FixtureAuth::default());
    let mut models = collection(store.clone(), &server.uri(), auth.clone());
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
    let report = models.refresh(true).await.unwrap();
    assert!(
        report
            .entries()
            .iter()
            .all(|entry| !entry.outcome.is_failure()),
        "{report:?}"
    );
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
    let persisted: Value = serde_json::from_slice(&std::fs::read(store.path()).unwrap()).unwrap();
    assert_eq!(
        persisted["providers"]["fixture"]["lastModified"],
        1700000000123_i64
    );
    assert_eq!(
        persisted["providers"]["fixture"]["checkedAt"],
        1700000000456_i64
    );
    assert_eq!(
        persisted["providers"]["fixture"]["etag"],
        "W/\"catalog-v1\""
    );
    for index in [1, 2] {
        for key in ["input", "inputLimits"] {
            assert_eq!(
                persisted["providers"]["fixture"]["models"][index][key],
                document(&server.uri())["providers"]["fixture"]["models"][index][key]
            );
        }
    }
    assert_eq!(
        store.snapshot().catalogs,
        HttpCatalogSource::decode_document(&document(&server.uri())).unwrap()
    );
    let tiers = persisted["providers"]["fixture"]["models"][1]["cost"]["tiers"]
        .as_array()
        .unwrap();
    assert_eq!(tiers.len(), 2);
    for (tier, rate) in tiers.iter().zip([99.0, 88.0]) {
        assert_eq!(tier["inputTokensAbove"].as_u64(), Some(0));
        assert_eq!(tier["input"].as_f64(), Some(rate));
        assert_eq!(tier["output"].as_f64(), Some(rate));
    }
    drop(models);

    let mut models = collection(
        ModelsStore::open(store.path()).unwrap(),
        &server.uri(),
        auth.clone(),
    );
    models.refresh(false).await.unwrap();
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
    let chat = models.get_model("fixture", "shared").unwrap().clone();
    assert_eq!(chat.capabilities.max_output_tokens, 512);
    models
        .complete_simple(
            ChatModelRecord::new("fixture", chat),
            request(),
            Default::default(),
        )
        .await
        .unwrap();
    let image = models.image_model("fixture", "shared").unwrap();
    assert_eq!(
        image.output,
        vec![ImageOutputModality::Image, ImageOutputModality::Text]
    );
    assert_eq!(image.cost.unwrap().input, 1.0);
    let image_result = models
        .generate_images(
            image,
            ImagesContext::default(),
            opi_ai::images::ImagesCall {
                extra_headers: vec![("X-Override".into(), "call".into())],
                ..Default::default()
            },
        )
        .await;
    assert_eq!(image_result.stop_reason, ImagesStopReason::Stop);
    assert!((image_result.cost.unwrap().total - 0.000_008).abs() < f64::EPSILON);
    let classifier = models.classifier_model("fixture", "shared").unwrap();
    assert_eq!(classifier.context_window, 4096);
    let context = ClassifierContext {
        state: json!({"ready": true}),
        questions: [(
            "approved".into(),
            ClassifierQuestion::Bool {
                instructions: "Approve?".into(),
                true_criteria: "Ready".into(),
                false_criteria: "Not ready".into(),
            },
        )]
        .into(),
        images: None,
    };
    assert_eq!(
        models
            .classify(
                classifier,
                context,
                opi_ai::classifier::ClassifierCall {
                    extra_headers: vec![("X-Override".into(), "call".into())],
                    ..Default::default()
                }
            )
            .await
            .stop_reason,
        ClassifierStopReason::Stop
    );
    assert_eq!(auth.0.load(Ordering::SeqCst), 3);
    let sent = server.received_requests().await.unwrap();
    for sent in sent.iter().filter(|request| request.method == "POST") {
        assert!(!sent.headers.contains_key("x-remove"));
        let body: Value = serde_json::from_slice(&sent.body).unwrap();
        assert_eq!(body["model"], "shared");
    }
}

#[derive(Clone)]
enum CatalogAction {
    Publish(Value),
    Fail,
    Cancel(Value),
    Pending,
    InvalidateThenPublish(Value, ModelsStore),
}

struct ControlledSource(std::sync::Mutex<CatalogAction>);

impl ControlledSource {
    fn set(&self, action: CatalogAction) {
        *self.0.lock().unwrap() = action;
    }
}

impl opi_ai::catalog::DynamicCatalogSource for ControlledSource {
    fn id(&self) -> &str {
        "controlled"
    }

    fn fetch(
        &self,
        token: CancellationToken,
    ) -> futures_util::future::BoxFuture<
        'static,
        Result<std::collections::HashMap<String, opi_ai::catalog::ModelsStoreEntry>, ProviderError>,
    > {
        let action = self.0.lock().unwrap().clone();
        Box::pin(async move {
            let value = match action {
                CatalogAction::Publish(value) => value,
                CatalogAction::Fail => {
                    return Err(ProviderError::Network(
                        opi_ai::provider::ProviderErrorSummary::sanitized("fixture source failed"),
                    ));
                }
                CatalogAction::Cancel(value) => {
                    token.cancel();
                    value
                }
                CatalogAction::Pending => return std::future::pending().await,
                CatalogAction::InvalidateThenPublish(value, store) => {
                    store.invalidate_source("controlled").await.unwrap();
                    value
                }
            };
            HttpCatalogSource::decode_document(&value).map_err(|error| {
                ProviderError::Config(opi_ai::provider::ProviderErrorSummary::sanitized(
                    error.to_string(),
                ))
            })
        })
    }
}

fn assert_selected(models: &Models, id: &str, expected: bool) {
    assert_eq!(models.get_model("fixture", id).is_some(), expected);
    assert_eq!(models.image_model("fixture", id).is_ok(), expected);
    assert_eq!(models.classifier_model("fixture", id).is_ok(), expected);
}

#[tokio::test]
async fn mixed_catalog_failure_cancel_drop_and_empty_preserve_publication_rules() {
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    let auth = Arc::new(FixtureAuth::default());
    let endpoint = "http://127.0.0.1:1";
    let original = document(endpoint);
    let source = Arc::new(ControlledSource(std::sync::Mutex::new(
        CatalogAction::Publish(original.clone()),
    )));
    let mut models = registered_collection(store.clone(), endpoint, auth.clone());
    models.register_catalog_source(source.clone());
    models.refresh(true).await.unwrap();
    assert_selected(&models, "shared", true);
    assert_selected(&models, "baseline", false);
    assert_eq!(
        models.get_available_classifiers(None).await.unwrap().len(),
        1
    );
    let confirmed = store.snapshot().catalogs;

    source.set(CatalogAction::Fail);
    assert!(
        models
            .refresh(true)
            .await
            .unwrap()
            .find("controlled")
            .unwrap()
            .outcome
            .is_failure()
    );
    assert_eq!(store.snapshot().catalogs, confirmed);
    assert_selected(&models, "shared", true);

    let mut empty = original;
    empty["providers"]["fixture"]["models"] = json!([]);
    source.set(CatalogAction::Cancel(empty.clone()));
    let before = std::fs::read(store.path()).unwrap();
    let token = CancellationToken::new();
    assert_eq!(
        models.refresh_with(true, &token).await.unwrap().status(),
        opi_ai::RefreshStatus::Cancelled
    );
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert_selected(&models, "shared", true);

    source.set(CatalogAction::Pending);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(10), models.refresh(true))
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert_selected(&models, "shared", true);

    source.set(CatalogAction::Publish(empty));
    models.refresh(true).await.unwrap();
    assert_selected(&models, "shared", false);
    assert_selected(&models, "baseline", false);
    assert!(
        models
            .get_available_classifiers(None)
            .await
            .unwrap()
            .is_empty()
    );
    let mut reopened = registered_collection(
        ModelsStore::open(store.path()).unwrap(),
        endpoint,
        auth.clone(),
    );
    reopened.refresh(false).await.unwrap();
    assert_selected(&reopened, "shared", false);
    assert_selected(&reopened, "baseline", false);
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn mixed_catalog_late_source_cannot_undo_invalidation_or_reopen_stale_records() {
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    let auth = Arc::new(FixtureAuth::default());
    let endpoint = "http://127.0.0.1:1";
    let original = document(endpoint);
    let source = Arc::new(ControlledSource(std::sync::Mutex::new(
        CatalogAction::Publish(original.clone()),
    )));
    let mut models = registered_collection(store.clone(), endpoint, auth.clone());
    models.register_catalog_source(source.clone());
    models.refresh(true).await.unwrap();
    source.set(CatalogAction::InvalidateThenPublish(
        original,
        store.clone(),
    ));
    assert_eq!(
        models.refresh(true).await.unwrap().status(),
        opi_ai::RefreshStatus::Superseded
    );
    assert!(store.snapshot().catalogs.is_empty());
    models.refresh(false).await.unwrap();
    assert_selected(&models, "shared", false);
    let reopened_store = ModelsStore::open(store.path()).unwrap();
    assert!(
        reopened_store
            .snapshot()
            .invalidated_sources
            .contains("controlled")
    );
    let mut reopened = registered_collection(reopened_store, endpoint, auth.clone());
    reopened.refresh(false).await.unwrap();
    assert_selected(&reopened, "shared", false);
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn malformed_operation_route_does_not_replace_any_record_in_provider_entry() {
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    let auth = Arc::new(FixtureAuth::default());
    let endpoint = "http://127.0.0.1:1";
    let mut value = document(endpoint);
    let source = Arc::new(ControlledSource(std::sync::Mutex::new(
        CatalogAction::Publish(value.clone()),
    )));
    let mut models = registered_collection(store.clone(), endpoint, auth.clone());
    models.register_catalog_source(source.clone());
    models.refresh(true).await.unwrap();
    let confirmed = store.snapshot().catalogs;
    value["providers"]["fixture"]["models"][1]["api"] = json!("unregistered-image-api");
    value["providers"]["fixture"]["models"][0]["maxTokens"] = json!(17);
    source.set(CatalogAction::Publish(value));
    assert!(
        models
            .refresh(true)
            .await
            .unwrap()
            .find("controlled")
            .unwrap()
            .outcome
            .is_failure()
    );
    assert_eq!(store.snapshot().catalogs, confirmed);
    assert_eq!(
        models
            .get_model("fixture", "shared")
            .unwrap()
            .capabilities
            .max_output_tokens,
        512
    );
    assert_selected(&models, "shared", true);
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn mixed_store_preserves_unknown_image_cost_and_classifier_tier_order() {
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    let mut value = document("http://127.0.0.1:1");
    value["providers"]["fixture"]["models"][1]
        .as_object_mut()
        .unwrap()
        .remove("cost");
    value["providers"]["fixture"]["models"][2]["cost"]["tiers"] = json!([
        {"inputTokensAbove": 0, "input": 3, "output": 4, "cacheRead": 0, "cacheWrite": 0},
        {"inputTokensAbove": 0, "input": 5, "output": 6, "cacheRead": 0, "cacheWrite": 0}
    ]);
    let entries = HttpCatalogSource::decode_document(&value).unwrap();
    store.begin().commit(entries.clone()).await.unwrap();
    let restored = ModelsStore::open(store.path()).unwrap().snapshot();
    assert_eq!(restored.catalogs, entries);
    let opi_ai::catalog::CatalogRecord::Image(image) = &restored.catalogs["fixture"].models[1]
    else {
        panic!("image");
    };
    assert!(image.cost.is_none());
}

#[tokio::test]
async fn complete_nonchat_record_protected_header_is_rejected_before_auth() {
    let temp = tempfile::tempdir().unwrap();
    let auth = Arc::new(FixtureAuth::default());
    let models = registered_collection(
        ModelsStore::open(temp.path().join("models.json")).unwrap(),
        "http://127.0.0.1:1",
        auth.clone(),
    );
    let mut image = models.image_model("fixture", "baseline").unwrap();
    image.headers = vec![("Authorization".into(), None)];
    let result = models
        .generate_images(image, Default::default(), Default::default())
        .await;
    assert_eq!(result.stop_reason, ImagesStopReason::Error);
    let mut classifier = models.classifier_model("fixture", "baseline").unwrap();
    classifier.headers = vec![("x-api-key".into(), Some("must-not-send".into()))];
    let result = models
        .classify(classifier, Default::default(), Default::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn image_adapter_replacement_keeps_authoritative_other_operations_and_frozen_calls() {
    let server = MockServer::start().await;
    for key in ["catalog-fixture-key", "replacement-key"] {
        Mock::given(method("POST"))
            .and(path("/image/chat/completions"))
            .and(header("authorization", format!("Bearer {key}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"choices": []})))
            .expect(1)
            .mount(&server)
            .await;
    }
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    let mut value = document(&server.uri());
    let image = value["providers"]["fixture"]["models"][1].clone();
    value["providers"]["fixture"]["models"] = json!([image]);
    store
        .begin()
        .commit(HttpCatalogSource::decode_document(&value).unwrap())
        .await
        .unwrap();
    let auth = Arc::new(FixtureAuth::default());
    let mut models = registered_collection(store, &server.uri(), auth.clone());
    models.refresh(false).await.unwrap();
    let image = models.image_model("fixture", "shared").unwrap();
    let old = models.generate_images(image.clone(), Default::default(), Default::default());
    let mut replacement_record = image.clone();
    replacement_record.id = "replacement-baseline".into();
    models
        .register_image_api(
            Arc::new(OpenRouterImagesApi::new(
                HttpClient::new(),
                replacement_record,
            )),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from("replacement-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    assert!(models.get_model("fixture", "baseline").is_none());
    assert!(models.classifier_model("fixture", "baseline").is_err());
    // Source publication owns selection; replacing its API changes dispatch,
    // without reviving baseline models outside that publication.
    assert!(
        models
            .image_model("fixture", "replacement-baseline")
            .is_err()
    );
    let current = models.generate_images(
        models.image_model("fixture", "shared").unwrap(),
        Default::default(),
        Default::default(),
    );
    assert_eq!(old.await.stop_reason, ImagesStopReason::Stop);
    assert_eq!(current.await.stop_reason, ImagesStopReason::Stop);
    assert_eq!(auth.0.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn mixed_catalog_persist_failure_keeps_all_three_visible_records() {
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    let auth = Arc::new(FixtureAuth::default());
    let endpoint = "http://127.0.0.1:1";
    let original = document(endpoint);
    let source = Arc::new(ControlledSource(std::sync::Mutex::new(
        CatalogAction::Publish(original.clone()),
    )));
    let mut models = registered_collection(store.clone(), endpoint, auth.clone());
    models.register_catalog_source(source.clone());
    models.refresh(true).await.unwrap();
    let confirmed = store.snapshot().catalogs;
    let before = std::fs::read(store.path()).unwrap();
    // Isolated filesystem failure at the store's write-before-rename step.
    std::fs::create_dir(store.path().with_extension("json.committing")).unwrap();
    let mut empty = original;
    empty["providers"]["fixture"]["models"] = json!([]);
    source.set(CatalogAction::Publish(empty));
    assert!(models.refresh(true).await.is_err());
    assert_eq!(store.snapshot().catalogs, confirmed);
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert_selected(&models, "shared", true);
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
}

struct ConfiguredAuth {
    configured: bool,
    checks: Arc<AtomicUsize>,
}

impl AuthResolver for ConfiguredAuth {
    fn resolve(&self) -> opi_ai::BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
        panic!("availability must not resolve credentials");
    }

    fn check_configuration(
        &self,
    ) -> Option<opi_ai::BoxAuthFuture<'_, Result<bool, ProviderError>>> {
        self.checks.fetch_add(1, Ordering::SeqCst);
        let configured = self.configured;
        Some(Box::pin(async move { Ok(configured) }))
    }
}

#[tokio::test]
async fn custom_classifier_availability_is_scoped_to_each_provider_api() {
    for decisions_configured in [false, true] {
        for reverse_registration in [false, true] {
            let mut models = Models::new();
            let decisions_checks = Arc::new(AtomicUsize::new(0));
            let system_checks = Arc::new(AtomicUsize::new(0));
            let mut decisions = opi_ai::api::decisions::default_decisions_classifier_record();
            decisions.provider_id = "fixture".into();
            decisions.id = "decisions".into();
            let mut system = opi_ai::api::system_one::default_typesafe_classifier_record();
            system.provider_id = "fixture".into();
            system.id = "system".into();
            for first in [!reverse_registration, reverse_registration] {
                if first {
                    models
                        .register_classifier_api(
                            Arc::new(opi_ai::api::decisions::OpenAiDecisionsApi::new(
                                HttpClient::new(),
                                decisions.clone(),
                            )),
                            Arc::new(ConfiguredAuth {
                                configured: decisions_configured,
                                checks: decisions_checks.clone(),
                            }),
                            AuthProvenanceSource::Static,
                        )
                        .unwrap();
                } else {
                    models
                        .register_classifier_api(
                            Arc::new(TypesafeSystemOneApi::new(HttpClient::new(), system.clone())),
                            Arc::new(ConfiguredAuth {
                                configured: !decisions_configured,
                                checks: system_checks.clone(),
                            }),
                            AuthProvenanceSource::Static,
                        )
                        .unwrap();
                }
            }
            let available = models
                .get_available_classifiers(Some("fixture"))
                .await
                .unwrap();
            assert_eq!(
                available.len(),
                1,
                "decisions={decisions_configured}, reverse={reverse_registration}"
            );
            assert_eq!(
                available[0].id,
                if decisions_configured {
                    "decisions"
                } else {
                    "system"
                }
            );
            assert_eq!(decisions_checks.load(Ordering::SeqCst), 1);
            assert_eq!(system_checks.load(Ordering::SeqCst), 1);
        }
    }
}

#[tokio::test]
async fn dynamic_unknown_prices_preserve_models_without_reporting_free_results() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/image/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"choices": [],
            "usage": {"prompt_tokens": 4, "completion_tokens": 2, "total_tokens": 6}})),
        )
        .expect(3)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/classifier/systemone"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"answers": {},
            "usage": {"input_tokens": 4, "output_tokens": 2}})),
        )
        .expect(3)
        .mount(&server)
        .await;
    Mock::given(method("POST")).and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-type", "text/event-stream")
            .set_body_string("data: {\"id\":\"prices\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":2,\"total_tokens\":6}}\n\ndata: [DONE]\n\n"))
        .expect(3).mount(&server).await;
    let auth = Arc::new(FixtureAuth::default());
    for price in [Some(-1.0), None, Some(0.0)] {
        let mut value = document(&server.uri());
        for model in value["providers"]["fixture"]["models"]
            .as_array_mut()
            .unwrap()
        {
            model["cost"] = price.map_or(Value::Null, |rate| {
                json!({
                "input": rate, "output": rate, "cacheRead": rate, "cacheWrite": rate})
            });
        }
        let entry =
            HttpCatalogSource::decode_document(&value).expect("unknown prices keep usable models");
        let temp = tempfile::tempdir().unwrap();
        let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
        store.begin().commit(entry).await.unwrap();
        let mut models = registered_collection(
            ModelsStore::open(store.path()).unwrap(),
            &server.uri(),
            auth.clone(),
        );
        models.refresh(false).await.unwrap();
        let image = models
            .generate_images(
                models.image_model("fixture", "shared").unwrap(),
                Default::default(),
                Default::default(),
            )
            .await;
        assert_eq!(image.stop_reason, ImagesStopReason::Stop);
        assert_eq!(
            image.cost.map(|cost| cost.total),
            (price == Some(0.0)).then_some(0.0)
        );
        let classifier = models
            .classify(
                models.classifier_model("fixture", "shared").unwrap(),
                ClassifierContext {
                    state: json!({}),
                    ..Default::default()
                },
                Default::default(),
            )
            .await;
        assert_eq!(classifier.stop_reason, ClassifierStopReason::Stop);
        assert_eq!(
            classifier.cost.map(|cost| cost.total_cost()),
            (price == Some(0.0)).then_some(0.0)
        );
        let chat = models.get_model("fixture", "shared").unwrap();
        let pricing = chat.pricing.clone();
        let result = models
            .complete_simple(
                ChatModelRecord::new("fixture", chat),
                request(),
                Default::default(),
            )
            .await
            .unwrap();
        let cost = pricing.and_then(|pricing| {
            opi_ai::model_info::calculate_cost_tiered(
                &result.usage,
                &pricing,
                "shared",
                WireApi::OpenAiCompletions,
            )
        });
        assert_eq!(
            cost.map(|cost| cost.total_cost()),
            (price == Some(0.0)).then_some(0.0)
        );
    }
    assert_eq!(auth.0.load(Ordering::SeqCst), 9);
}

#[tokio::test]
async fn source_unknown_base_retains_known_tiers_and_prices_only_above_the_boundary() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/image/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"choices": [],
            "usage": {"prompt_tokens": 4, "completion_tokens": 2, "total_tokens": 6}})),
        )
        .expect(3)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/classifier/systemone"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"answers": {},
            "usage": {"input_tokens": 4, "output_tokens": 2}})),
        )
        .expect(3)
        .mount(&server)
        .await;
    Mock::given(method("POST")).and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-type", "text/event-stream")
            .set_body_string("data: {\"id\":\"prices\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":2,\"total_tokens\":6}}\n\ndata: [DONE]\n\n"))
        .expect(3).mount(&server).await;
    for (threshold, unknown_tier) in [(4, false), (3, false), (3, true)] {
        let mut value = document(&server.uri());
        for model in value["providers"]["fixture"]["models"]
            .as_array_mut()
            .unwrap()
        {
            let base = if unknown_tier { 1.0 } else { -1.0 };
            let tier_input = if unknown_tier { -1.0 } else { 5.0 };
            model["cost"] = json!({"input": base, "output": base, "cacheRead": base, "cacheWrite": base,
            "tiers": [
                {"inputTokensAbove": threshold, "input": tier_input, "output": 7, "cacheRead": 0, "cacheWrite": 0},
                {"inputTokensAbove": threshold, "input": 99, "output": 99, "cacheRead": 0, "cacheWrite": 0}
            ]});
        }
        let temp = tempfile::tempdir().unwrap();
        let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
        store
            .begin()
            .commit(HttpCatalogSource::decode_document(&value).unwrap())
            .await
            .unwrap();
        let mut models = registered_collection(
            ModelsStore::open(store.path()).unwrap(),
            &server.uri(),
            Arc::new(FixtureAuth::default()),
        );
        models.refresh(false).await.unwrap();
        let classifier_record = models.classifier_model("fixture", "shared").unwrap();
        let pricing = classifier_record.cost.as_ref().unwrap();
        assert_eq!(
            pricing.base.input_cost_per_mtok,
            if unknown_tier { 1.0 } else { -1.0 }
        );
        assert_eq!(pricing.tiers.len(), 2);
        assert_eq!(
            pricing.tiers[0].pricing.input_cost_per_mtok,
            if unknown_tier { -1.0 } else { 5.0 }
        );
        let image_record = models.image_model("fixture", "shared").unwrap();
        assert_eq!(image_record.cost_tiers, pricing.tiers);
        assert_eq!(
            models
                .get_model("fixture", "shared")
                .unwrap()
                .pricing
                .as_ref(),
            Some(pricing)
        );
        let image = models
            .generate_images(
                models.image_model("fixture", "shared").unwrap(),
                Default::default(),
                Default::default(),
            )
            .await;
        let classifier = models
            .classify(
                models.classifier_model("fixture", "shared").unwrap(),
                ClassifierContext {
                    state: json!({}),
                    ..Default::default()
                },
                Default::default(),
            )
            .await;
        let chat = models.get_model("fixture", "shared").unwrap();
        let pricing = chat.pricing.clone();
        let result = models
            .complete_simple(
                ChatModelRecord::new("fixture", chat),
                request(),
                Default::default(),
            )
            .await
            .unwrap();
        let chat_cost = pricing.and_then(|pricing| {
            opi_ai::model_info::calculate_cost_tiered(
                &result.usage,
                &pricing,
                "shared",
                WireApi::OpenAiCompletions,
            )
        });
        assert_eq!(image.stop_reason, ImagesStopReason::Stop);
        assert_eq!(classifier.stop_reason, ClassifierStopReason::Stop);
        assert_eq!(
            image.cost.map(|cost| cost.total),
            unknown_tier.then_some(0.000_006),
            "image always uses its base price, including when the selected metadata tier is unknown"
        );
        for cost in [chat_cost, classifier.cost] {
            if threshold == 4 || unknown_tier {
                assert!(cost.is_none());
            } else {
                assert!(
                    (cost.expect("known selected tier").total_cost() - 0.000_034).abs() < 1e-12
                );
            }
        }
    }
}
