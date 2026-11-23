//! TypeSafe's builtin catalog selection and System One image wire behavior.

use super::{
    Arc, BuiltinOverrides, Catalog, CatalogOperation, Models, map_context, register_builtin,
};
use opi_ai::classifier::{
    ClassifierAnswer, ClassifierCall, ClassifierContext, ClassifierImageInput,
    ClassifierInputModality, ClassifierQuestion, ClassifierStopReason,
};
use opi_ai::models::ModelsError;
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

const PROVIDER: &str = "typesafe";
const FIXTURE_KEY: &str = "fixture-typesafe-key";

fn builtin_models() -> Models {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[("TYPESAFE_API_KEY", FIXTURE_KEY)]),
        Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    models
}

fn loopback_base(server: &MockServer, published_base: &str) -> String {
    let published = url::Url::parse(published_base).unwrap();
    assert_eq!(published.host_str(), Some("api.typesafe.ai"));
    format!("{}{}", server.uri(), published.path().trim_end_matches('/'))
}

fn context() -> ClassifierContext {
    ClassifierContext {
        state: json!({"text": "hello"}),
        questions: [(
            "ok".into(),
            ClassifierQuestion::Bool {
                instructions: "Is it a greeting?".into(),
                true_criteria: "Greeting".into(),
                false_criteria: "Other".into(),
            },
        )]
        .into_iter()
        .collect(),
        images: None,
    }
}

#[tokio::test]
async fn builtin_typesafe_lists_the_published_classifier_record() {
    let catalog = Catalog::builtin().unwrap();
    let models = builtin_models();
    let raw_bundle: Value = serde_json::from_slice(Catalog::builtin_bundle_bytes().0).unwrap();
    let author_source: Value = serde_json::from_slice(include_bytes!(
        "../../resources/catalog/providers/typesafe.json"
    ))
    .unwrap();
    assert_eq!(raw_bundle[PROVIDER], author_source);

    let entries: Vec<_> = catalog
        .models(PROVIDER, CatalogOperation::Classifier)
        .unwrap()
        .collect();
    assert_eq!(entries.len(), 1);
    let entry = entries[0];
    assert_eq!(entry.id, "jev-latest");
    assert_eq!(
        entry.input,
        [
            opi_ai::catalog::CatalogInputModality::Text,
            opi_ai::catalog::CatalogInputModality::Image
        ]
    );
    assert_eq!(entry.raw["compat"]["imageInput"], "assets");
    assert_eq!(entry.raw["compat"]["sendImageOptInHeader"], true);

    let record = models.classifier_model(PROVIDER, &entry.id).unwrap();
    assert_eq!(
        record.input,
        [
            ClassifierInputModality::Text,
            ClassifierInputModality::Image
        ]
    );
    assert_eq!(
        record.compat.as_ref().unwrap().image_input,
        Some(ClassifierImageInput::Assets)
    );
    assert_eq!(
        record.compat.as_ref().unwrap().send_image_opt_in_header,
        Some(true)
    );
    assert_eq!(
        models.get_models_of_type(CatalogOperation::Classifier, Some(PROVIDER)),
        [opi_ai::catalog::CatalogRecord::Classifier(record.clone())]
    );
    assert_eq!(
        models
            .get_model_of_type(CatalogOperation::Classifier, PROVIDER, &entry.id)
            .as_ref(),
        Some(&opi_ai::catalog::CatalogRecord::Classifier(record.clone()))
    );
    assert_eq!(
        models
            .get_available_of_type(CatalogOperation::Classifier, Some(PROVIDER))
            .await
            .unwrap(),
        [opi_ai::catalog::CatalogRecord::Classifier(record)]
    );
}

#[tokio::test]
async fn builtin_typesafe_sends_assets_and_image_opt_in_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "answers": {"ok": {"type": "noul", "noul": 0.91}},
            "usage": {"input_tokens": 12, "output_tokens": 1}
        })))
        .expect(2)
        .mount(&server)
        .await;

    let models = builtin_models();
    let mut image_record = models.classifier_model(PROVIDER, "jev-latest").unwrap();
    image_record.base_url = loopback_base(&server, &image_record.base_url);
    let image_result = models
        .classify(
            image_record.clone(),
            ClassifierContext {
                images: Some(vec![opi_ai::classifier::ClassifierImage {
                    mime_type: "image/png".into(),
                    data: "cG5n".into(),
                }]),
                ..context()
            },
            ClassifierCall::default(),
        )
        .await;
    assert_eq!(image_result.stop_reason, ClassifierStopReason::Stop);
    assert_eq!(
        image_result.answers["ok"],
        ClassifierAnswer::Bool { probability: 0.91 }
    );

    let plain_result = models
        .classify(image_record, context(), ClassifierCall::default())
        .await;
    assert_eq!(plain_result.stop_reason, ClassifierStopReason::Stop);

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 2);
    let image_body: Value = serde_json::from_slice(&sent[0].body).unwrap();
    assert_eq!(
        sent[0].headers["authorization"],
        format!("Bearer {FIXTURE_KEY}")
    );
    assert_eq!(sent[0].headers["typesafe-danger-zone-images"], "true");
    assert_eq!(image_body["state"], json!(["<image-1>", {"text": "hello"}]));
    assert_eq!(
        image_body["assets"]["<image-1>"],
        json!({
            "type": "image", "format": "png", "data_base64": "cG5n"
        })
    );
    assert_eq!(image_body["questions"]["ok"]["type"], "noul");
    let plain_body: Value = serde_json::from_slice(&sent[1].body).unwrap();
    assert_eq!(plain_body["state"], json!({"text": "hello"}));
    assert!(plain_body.get("assets").is_none());
    assert!(!sent[1].headers.contains_key("typesafe-danger-zone-images"));
}

#[tokio::test]
async fn builtin_typesafe_unknown_models_are_removed_before_dispatch() {
    let server = MockServer::start().await;
    let models = builtin_models();
    assert!(
        Catalog::builtin()
            .unwrap()
            .models(PROVIDER, CatalogOperation::Classifier)
            .unwrap()
            .all(|entry| entry.id == "jev-latest")
    );
    assert!(matches!(
        models.classifier_model(PROVIDER, "jev-retired"),
        Err(ModelsError::NotAClassifierModel { .. })
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}
