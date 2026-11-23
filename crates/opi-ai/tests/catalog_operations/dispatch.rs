//! Registered operation selection, availability and actual adapter dispatch.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use opi_ai::api::openrouter_images::OpenRouterImagesApi;
use opi_ai::api::system_one::TypesafeSystemOneApi;
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::catalog::{HttpCatalogSource, ModelsStore};
use opi_ai::classifier::{ClassifierContext, ClassifierQuestion, ClassifierStopReason};
use opi_ai::http::HttpClient;
use opi_ai::images::{ImageOutputModality, ImagesContext, ImagesStopReason};
use opi_ai::models::Models;
use opi_ai::provider::ProviderError;
use opi_ai::types::ApiId;
use opi_ai::{AuthProvenanceSource, ChatModelRecord};
use serde_json::{Value, json};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::fixtures::{FixtureAuth, document, registered_collection, request};

fn collection(store: ModelsStore, endpoint: &str, auth: Arc<FixtureAuth>) -> Models {
    let mut models = registered_collection(store, endpoint, auth);
    models.register_catalog_source(Arc::new(HttpCatalogSource::new(
        format!("{endpoint}/catalog"),
        ["fixture".into()],
    )));
    models
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
    let report = models.refresh(Default::default()).await.unwrap();
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
        opi_ai::catalog::decode_document(&document(&server.uri())).unwrap()
    );
    let tiers = persisted["providers"]["fixture"]["models"][1]["cost"]["tiers"]
        .as_array()
        .unwrap();
    assert_eq!(tiers.len(), 2);
    for (tier, rate) in tiers.iter().zip([99.0, 88.0]) {
        assert_eq!(tier["inputTokensAbove"].as_f64(), Some(0.0));
        assert_eq!(tier["input"].as_f64(), Some(rate));
        assert_eq!(tier["output"].as_f64(), Some(rate));
    }
    drop(models);

    let mut models = collection(
        ModelsStore::open(store.path()).unwrap(),
        &server.uri(),
        auth.clone(),
    );
    models
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();
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
        if sent.url.path() == "/chat/completions" {
            assert_eq!(body["priority"], 0.125);
            assert_eq!(
                body["provider"],
                document(&server.uri())["providers"]["fixture"]["models"][0]["compat"]["openRouterRouting"]
            );
            assert_eq!(body["providerOptions"]["gateway"]["only"], json!([]));
            assert!(body.get("store").is_none());
            assert!(body.get("stream_options").is_none());
        }
    }
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
        .commit(opi_ai::catalog::decode_document(&value).unwrap())
        .await
        .unwrap();
    let auth = Arc::new(FixtureAuth::default());
    let mut models = registered_collection(store, &server.uri(), auth.clone());
    models
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();
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
async fn management_lists_actual_operation_owners_and_preserves_shared_catalog_bytes() {
    use opi_ai::catalog::{CatalogOperation, CatalogRecord};

    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("management.json")).unwrap();
    store
        .begin()
        .commit(opi_ai::catalog::decode_document(&document("http://127.0.0.1:1")).unwrap())
        .await
        .unwrap();
    let before = std::fs::read(store.path()).unwrap();
    let auth = Arc::new(FixtureAuth::default());
    let mut models = registered_collection(store.clone(), "http://127.0.0.1:1", auth.clone());
    models
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();

    let mut image = opi_ai::api::openrouter_images::default_openrouter_image_record();
    image.provider_id = "image-only".into();
    image.id = "shared".into();
    models
        .register_image_api(
            Arc::new(OpenRouterImagesApi::new(HttpClient::new(), image)),
            auth.clone(),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let mut classifier = opi_ai::api::system_one::default_typesafe_classifier_record();
    classifier.provider_id = "classifier-only".into();
    classifier.id = "shared".into();
    models
        .register_classifier_api(
            Arc::new(TypesafeSystemOneApi::new(HttpClient::new(), classifier)),
            auth.clone(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    assert_eq!(
        models.provider_ids(),
        ["classifier-only", "fixture", "image-only"]
    );
    assert_eq!(models.get_models(None).len(), 1);
    assert_eq!(models.get_models(Some("fixture"))[0].id, "shared");
    assert!(models.get_models(Some("image-only")).is_empty());
    assert_eq!(models.get_all_models(None).len(), 5);
    assert_eq!(models.get_all_models(Some("fixture")).len(), 3);
    for operation in [
        CatalogOperation::Chat,
        CatalogOperation::Image,
        CatalogOperation::Classifier,
    ] {
        let selected = models
            .get_model_of_type(operation, "fixture", "shared")
            .unwrap();
        assert_eq!(selected.operation(), operation);
        assert_eq!(selected.id(), "shared");
        assert_eq!(
            models.get_models_of_type(operation, Some("fixture")).len(),
            1
        );
        assert!(
            models
                .get_model_of_type(operation, "fixture", "missing")
                .is_none()
        );
        assert!(
            models
                .get_models_of_type(operation, Some("missing"))
                .is_empty()
        );
        assert!(
            models
                .get_available_of_type(operation, Some("missing"))
                .await
                .unwrap()
                .is_empty()
        );
    }
    assert!(matches!(
        models.get_model_of_type(CatalogOperation::Image, "image-only", "shared"),
        Some(CatalogRecord::Image(_))
    ));
    assert_eq!(
        models
            .get_all_available(Some("fixture"))
            .await
            .unwrap()
            .len(),
        3
    );
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
    assert_eq!(std::fs::read(store.path()).unwrap(), before);

    assert!(!models.delete_provider("missing"));
    assert_eq!(models.get_all_models(None).len(), 5);
    assert!(models.delete_provider("fixture"));
    assert_eq!(models.provider_ids(), ["classifier-only", "image-only"]);
    assert_eq!(models.get_all_models(None).len(), 2);
    models.clear_providers();
    models.clear_providers();
    models
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(models.provider_ids().is_empty());
    assert!(models.get_all_models(None).is_empty());
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
}

#[tokio::test]
async fn restoring_a_cached_image_api_does_not_suppress_a_replacement_api_fallback() {
    use opi_ai::catalog::{CatalogRecord, ModelsStoreEntry};
    use std::collections::HashMap;

    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("api-replacement.json")).unwrap();
    let empty_store = ModelsStore::open(temp.path().join("authoritative-empty.json")).unwrap();
    let mut cached_a = opi_ai::api::openrouter_images::default_openrouter_image_record();
    cached_a.provider_id = "switched".into();
    cached_a.id = "cached-a".into();
    cached_a.api = ApiId::new("api-a");
    store
        .begin()
        .commit(HashMap::from([(
            "switched".into(),
            ModelsStoreEntry {
                models: vec![CatalogRecord::Image(cached_a.clone())],
                ..Default::default()
            },
        )]))
        .await
        .unwrap();
    empty_store
        .begin()
        .commit(HashMap::from([(
            "switched".into(),
            ModelsStoreEntry::default(),
        )]))
        .await
        .unwrap();
    let before = std::fs::read(store.path()).unwrap();
    let empty_before = std::fs::read(empty_store.path()).unwrap();
    let auth = Arc::new(FixtureAuth::default());
    let mut static_b = cached_a.clone();
    static_b.id = "static-b".into();
    static_b.api = ApiId::new("api-b");

    let mut empty_models = Models::new();
    empty_models
        .register_image_api(
            Arc::new(OpenRouterImagesApi::new(
                HttpClient::new(),
                static_b.clone(),
            )),
            auth.clone(),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    assert!(empty_models.image_model("switched", "static-b").is_ok());
    empty_models.set_models_store(empty_store.clone());
    empty_models
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(empty_models.image_model("switched", "static-b").is_err());
    assert!(empty_models.get_all_models(Some("switched")).is_empty());
    assert_eq!(std::fs::read(empty_store.path()).unwrap(), empty_before);

    let mut models = Models::new();
    let mut static_a = cached_a.clone();
    static_a.id = "static-a".into();
    models
        .register_image_api(
            Arc::new(OpenRouterImagesApi::new(HttpClient::new(), static_a)),
            auth.clone(),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    models.set_models_store(store.clone());
    models
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(models.image_model("switched", "cached-a").is_ok());
    assert!(models.image_model("switched", "static-a").is_err());
    assert!(models.delete_provider("switched"));
    models
        .register_image_api(
            Arc::new(OpenRouterImagesApi::new(HttpClient::new(), static_b)),
            auth.clone(),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    assert!(models.image_model("switched", "static-b").is_ok());
    models
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(models.image_model("switched", "cached-a").is_err());
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert!(
        models.image_model("switched", "static-b").is_ok(),
        "a nonempty cache for retired API A must not fabricate an authoritative empty publication for replacement API B"
    );
}

#[tokio::test]
async fn image_availability_uses_each_provider_api_configuration_without_resolving_auth() {
    use opi_ai::catalog::{CatalogOperation, CatalogRecord};

    for first_available in [false, true] {
        let mut models = Models::new();
        let checks = [Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0))];
        for (index, counter) in checks.iter().enumerate() {
            let mut record = opi_ai::api::openrouter_images::default_openrouter_image_record();
            record.provider_id = "images".into();
            record.id = format!("image-{index}");
            record.api = ApiId::new(format!("image-api-{index}"));
            models
                .register_image_api(
                    Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record)),
                    Arc::new(ConfiguredAuth {
                        configured: (index == 0) == first_available,
                        checks: counter.clone(),
                    }),
                    AuthProvenanceSource::Static,
                )
                .unwrap();
        }
        assert_eq!(
            models
                .get_models_of_type(CatalogOperation::Image, None)
                .len(),
            2
        );
        let available = models
            .get_available_of_type(CatalogOperation::Image, Some("images"))
            .await
            .unwrap();
        assert_eq!(available.len(), 1);
        let expected = if first_available {
            "image-0"
        } else {
            "image-1"
        };
        assert!(matches!(&available[0], CatalogRecord::Image(record) if record.id == expected));
        let all_available = models.get_all_available(None).await.unwrap();
        assert_eq!(all_available, available);
        for counter in &checks {
            assert_eq!(counter.load(Ordering::SeqCst), 2);
        }
        assert!(
            models
                .get_all_available(Some("missing"))
                .await
                .unwrap()
                .is_empty()
        );
        for counter in &checks {
            assert_eq!(counter.load(Ordering::SeqCst), 2);
        }
    }
}

#[tokio::test]
async fn removal_preserves_captured_image_and_classifier_calls_and_other_instances() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/image/chat/completions"))
        .and(header("authorization", "Bearer catalog-fixture-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"choices": []})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/classifier/systemone"))
        .and(header("authorization", "Bearer catalog-fixture-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "answers": {"approved": {"type": "noul", "noul": 0.9}}
        })))
        .expect(1)
        .mount(&server)
        .await;
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("frozen-management.json")).unwrap();
    store
        .begin()
        .commit(opi_ai::catalog::decode_document(&document(&server.uri())).unwrap())
        .await
        .unwrap();
    let before = std::fs::read(store.path()).unwrap();
    let auth = Arc::new(FixtureAuth::default());
    let mut models = registered_collection(store.clone(), &server.uri(), auth.clone());
    let mut other = registered_collection(store.clone(), &server.uri(), auth.clone());
    for collection in [&mut models, &mut other] {
        collection
            .refresh(opi_ai::ModelsRefreshOptions {
                allow_network: false,
                ..Default::default()
            })
            .await
            .unwrap();
    }
    let image = models.image_model("fixture", "shared").unwrap();
    let classifier = models.classifier_model("fixture", "shared").unwrap();
    let classify_context = ClassifierContext {
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
    let captured_image =
        models.generate_images(image.clone(), Default::default(), Default::default());
    let captured_classifier = models.classify(
        classifier.clone(),
        classify_context.clone(),
        Default::default(),
    );
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
    assert!(models.delete_provider("fixture"));
    assert!(!models.delete_provider("fixture"));
    assert!(models.get_all_models(None).is_empty());
    assert!(models.get_all_available(None).await.unwrap().is_empty());
    assert!(models.image_model("fixture", "shared").is_err());
    assert!(models.classifier_model("fixture", "shared").is_err());
    assert_eq!(other.get_all_models(Some("fixture")).len(), 3);
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
    assert_eq!(
        models
            .generate_images(image, Default::default(), Default::default())
            .await
            .stop_reason,
        ImagesStopReason::Error
    );
    assert_eq!(
        models
            .classify(classifier, classify_context, Default::default())
            .await
            .stop_reason,
        ClassifierStopReason::Error
    );
    assert_eq!(auth.0.load(Ordering::SeqCst), 0);
    assert!(server.received_requests().await.unwrap().is_empty());

    assert_eq!(captured_image.await.stop_reason, ImagesStopReason::Stop);
    assert_eq!(
        captured_classifier.await.stop_reason,
        ClassifierStopReason::Stop
    );
    assert_eq!(auth.0.load(Ordering::SeqCst), 2);
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
    other.clear_providers();
    assert!(other.provider_ids().is_empty());
    assert!(other.get_all_models(None).is_empty());
    assert_eq!(std::fs::read(store.path()).unwrap(), before);
}

#[tokio::test]
async fn custom_classifier_availability_is_scoped_to_each_provider_api() {
    for decisions_configured in [false, true] {
        for reverse_registration in [false, true] {
            let mut models = Models::new();
            let decisions_checks = Arc::new(AtomicUsize::new(0));
            let system_checks = Arc::new(AtomicUsize::new(0));
            let mut decisions =
                opi_ai::api::openai_decisions::default_decisions_classifier_record();
            decisions.provider_id = "fixture".into();
            decisions.id = "decisions".into();
            let mut system = opi_ai::api::system_one::default_typesafe_classifier_record();
            system.provider_id = "fixture".into();
            system.id = "system".into();
            for first in [!reverse_registration, reverse_registration] {
                if first {
                    models
                        .register_classifier_api(
                            Arc::new(opi_ai::api::openai_decisions::OpenAiDecisionsApi::new(
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
