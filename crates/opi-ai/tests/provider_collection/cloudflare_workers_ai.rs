//! Cloudflare Workers AI's fixed Pi 1.5 catalog and wire behavior.

use super::{
    Arc, BuiltinOverrides, Catalog, CatalogOperation, Models, WireApi, map_context,
    register_builtin,
};
use opi_ai::catalog::CatalogRecord;
use opi_ai::classifier::{
    ClassifierCall, ClassifierContext, ClassifierImage, ClassifierInputModality,
    ClassifierQuestion, ClassifierStopReason,
};
use opi_ai::models::{ChatModelRecord, ModelsError};
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

const PROVIDER: &str = "cloudflare-workers-ai";
const FIXTURE_KEY: &str = "fixture-cloudflare-key";
const ACCOUNT_ID: &str = "account-id";
const CHAT_IDS: [&str; 19] = [
    "@cf/deepseek-ai/deepseek-v4-flash-0731",
    "@cf/deepseek-ai/deepseek-v4-pro-0813",
    "@cf/deepseek-ai/deepseek-v4.1-flash-1017",
    "@cf/google/gemma-4-26b-a4b-it",
    "@cf/ibm-granite/granite-4.0-h-micro",
    "@cf/meta/llama-3.3-70b-instruct-fp8-fast",
    "@cf/meta/llama-4-scout-17b-16e-instruct",
    "@cf/mistralai/mistral-small-3.1-24b-instruct",
    "@cf/moonshotai/kimi-k2.6",
    "@cf/moonshotai/kimi-k2.7-code",
    "@cf/nvidia/nemotron-3-120b-a12b",
    "@cf/openai/gpt-oss-120b",
    "@cf/openai/gpt-oss-20b",
    "@cf/qwen/qwen3-30b-a3b-fp8",
    "@cf/qwen/qwen3.8-27b",
    "@cf/zai-org/glm-4.7-flash",
    "@cf/zai-org/glm-5.2",
    "@cf/zai-org/glm-5.3",
    "@cf/zai-org/glm-5.3-flash",
];
const CLASSIFIER_IDS: [&str; 5] = [
    "@cf/cloudflare/clef",
    "@cf/cloudflare/clef-flash",
    "@cf/cloudflare/clef-omni",
    "typesafe/jev",
    "typesafe/jev-1.14.0",
];

fn builtin_models(overrides: &BuiltinOverrides) -> Models {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[
            ("CLOUDFLARE_API_KEY", FIXTURE_KEY),
            ("CLOUDFLARE_ACCOUNT_ID", ACCOUNT_ID),
        ]),
        Arc::new(opi_ai::http::HttpClient::new()),
        overrides,
    )
    .unwrap();
    models
}

#[tokio::test]
async fn builtin_cloudflare_lists_the_published_chat_and_classifier_records() {
    let catalog = Catalog::builtin().unwrap();
    let models = builtin_models(&BuiltinOverrides::default());
    let raw_bundle: Value = serde_json::from_slice(Catalog::builtin_bundle_bytes().0).unwrap();
    let author_source: Value = serde_json::from_slice(include_bytes!(
        "../../resources/catalog/providers/cloudflare-workers-ai.json"
    ))
    .unwrap();
    assert_eq!(raw_bundle[PROVIDER], author_source);

    let chats: Vec<_> = catalog
        .models(PROVIDER, CatalogOperation::Chat)
        .unwrap()
        .collect();
    assert_eq!(chats.len(), CHAT_IDS.len());
    assert_eq!(
        chats
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        CHAT_IDS
    );
    assert!(chats.iter().all(|entry| entry.api == "openai-completions"));
    assert!(chats.iter().all(|entry| {
        entry
            .input
            .contains(&opi_ai::catalog::CatalogInputModality::Text)
    }));
    assert!(
        chats
            .iter()
            .any(|entry| entry.id == "@cf/deepseek-ai/deepseek-v4.1-flash-1017")
    );

    let classifiers: Vec<_> = catalog
        .models(PROVIDER, CatalogOperation::Classifier)
        .unwrap()
        .collect();
    assert_eq!(classifiers.len(), CLASSIFIER_IDS.len());
    assert_eq!(
        classifiers
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        CLASSIFIER_IDS
    );
    assert!(
        classifiers
            .iter()
            .all(|entry| entry.api == "cloudflare-workers-ai-system-one")
    );
    for entry in &classifiers {
        assert!(
            entry
                .input
                .contains(&opi_ai::catalog::CatalogInputModality::Image)
        );
        let expected = if entry.id.starts_with("@cf/cloudflare/clef") {
            "images"
        } else {
            "assets"
        };
        assert_eq!(entry.raw["compat"]["imageInput"], expected);
        let record = models.classifier_model(PROVIDER, &entry.id).unwrap();
        assert_eq!(
            record.input,
            [
                ClassifierInputModality::Text,
                ClassifierInputModality::Image
            ]
        );
    }

    let gemma = chats
        .iter()
        .find(|entry| entry.id == "@cf/google/gemma-4-26b-a4b-it")
        .unwrap();
    assert!(gemma.raw.get("thinkingLevelMap").is_none());
    let nemotron = chats
        .iter()
        .find(|entry| entry.id == "@cf/nvidia/nemotron-3-120b-a12b")
        .unwrap();
    assert_eq!(nemotron.raw["thinkingLevelMap"]["medium"], Value::Null);

    let listed_chats = models.get_models_of_type(CatalogOperation::Chat, Some(PROVIDER));
    assert_eq!(listed_chats.len(), CHAT_IDS.len());
    assert_eq!(
        models
            .get_available_of_type(CatalogOperation::Chat, Some(PROVIDER))
            .await
            .unwrap()
            .len(),
        CHAT_IDS.len()
    );
    assert_eq!(
        models
            .get_available_classifiers(Some(PROVIDER))
            .await
            .unwrap()
            .len(),
        CLASSIFIER_IDS.len()
    );
    for id in CHAT_IDS {
        assert!(matches!(
            models.get_model_of_type(CatalogOperation::Chat, PROVIDER, id),
            Some(CatalogRecord::Chat(_))
        ));
    }
}

#[tokio::test]
async fn builtin_cloudflare_sends_bearer_chat_requests_to_the_published_route() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(super::node5_reply(WireApi::OpenAiCompletions)),
        )
        .mount(&server)
        .await;

    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert(
        PROVIDER.into(),
        format!("{}/accounts/{ACCOUNT_ID}/ai/v1", server.uri()),
    );
    let models = builtin_models(&overrides);
    let info = models
        .get_model(PROVIDER, "@cf/deepseek-ai/deepseek-v4.1-flash-1017")
        .unwrap();
    let mut request =
        super::minimal_request("cloudflare-workers-ai:@cf/deepseek-ai/deepseek-v4.1-flash-1017");
    request.max_tokens = Some(321);
    let output = models
        .complete_simple(
            ChatModelRecord::new(PROVIDER, info),
            request,
            Default::default(),
        )
        .await
        .unwrap();
    assert_eq!(output.model, "@cf/deepseek-ai/deepseek-v4.1-flash-1017");

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(
        sent[0].url.path(),
        "/accounts/account-id/ai/v1/chat/completions"
    );
    assert_eq!(
        sent[0].headers["authorization"],
        format!("Bearer {FIXTURE_KEY}")
    );
    let body: Value = serde_json::from_slice(&sent[0].body).unwrap();
    assert_eq!(body["model"], "@cf/deepseek-ai/deepseek-v4.1-flash-1017");
    assert_eq!(body["max_completion_tokens"], 321);
    assert_eq!(body["stream"], true);
}

#[tokio::test]
async fn builtin_cloudflare_sends_classifier_images_as_data_urls() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "result": {
                "answers": {"approved": {"type": "noul", "noul": 0.9}},
                "usage": {"input_tokens": 1, "output_tokens": 0}
            }
        })))
        .expect(2)
        .mount(&server)
        .await;

    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert(
        PROVIDER.into(),
        format!("{}/accounts/{ACCOUNT_ID}/ai", server.uri()),
    );
    let models = builtin_models(&overrides);
    let record = models
        .classifier_model(PROVIDER, "@cf/cloudflare/clef")
        .unwrap();
    let result = models
        .classify(
            record,
            ClassifierContext {
                state: json!({"text": "hello"}),
                questions: [(
                    "approved".into(),
                    ClassifierQuestion::Bool {
                        instructions: "Approve?".into(),
                        true_criteria: "Yes".into(),
                        false_criteria: "No".into(),
                    },
                )]
                .into_iter()
                .collect(),
                images: Some(vec![ClassifierImage {
                    mime_type: "image/png".into(),
                    data: "aGk=".into(),
                }]),
            },
            ClassifierCall::default(),
        )
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].url.path(), "/accounts/account-id/ai/run");
    assert_eq!(
        sent[0].headers["authorization"],
        format!("Bearer {FIXTURE_KEY}")
    );
    assert!(!sent[0].headers.contains_key("typesafe-danger-zone-images"));
    let body: Value = serde_json::from_slice(&sent[0].body).unwrap();
    assert_eq!(body["model"], "@cf/cloudflare/clef");
    assert_eq!(
        body["input"]["images"],
        json!(["data:image/png;base64,aGk="])
    );
    assert_eq!(body["input"]["state"], json!({"text": "hello"}));
    assert_eq!(body["input"]["questions"]["approved"]["type"], "noul");

    let jev = models.classifier_model(PROVIDER, "typesafe/jev").unwrap();
    let jev_result = models
        .classify(
            jev,
            ClassifierContext {
                images: Some(vec![ClassifierImage {
                    mime_type: "image/png".into(),
                    data: "aGk=".into(),
                }]),
                ..ClassifierContext {
                    state: json!({"text": "hello"}),
                    questions: [(
                        "approved".into(),
                        ClassifierQuestion::Bool {
                            instructions: "Approve?".into(),
                            true_criteria: "Yes".into(),
                            false_criteria: "No".into(),
                        },
                    )]
                    .into_iter()
                    .collect(),
                    images: None,
                }
            },
            ClassifierCall::default(),
        )
        .await;
    assert_eq!(jev_result.stop_reason, ClassifierStopReason::Stop);

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 2);
    let jev_body: Value = serde_json::from_slice(&sent[1].body).unwrap();
    assert_eq!(jev_body["model"], "typesafe/jev");
    assert_eq!(
        jev_body["input"]["state"],
        json!(["<image-1>", {"text": "hello"}])
    );
    assert_eq!(jev_body["input"]["assets"]["<image-1>"]["format"], "png");
}

#[tokio::test]
async fn builtin_cloudflare_has_no_retired_models_in_any_entrypoint() {
    let server = MockServer::start().await;
    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert(PROVIDER.into(), server.uri());
    let models = builtin_models(&overrides);
    assert!(
        models
            .get_model(PROVIDER, "@cf/cloudflare/clef-retired")
            .is_none()
    );
    assert!(matches!(
        models.classifier_model(PROVIDER, "@cf/cloudflare/clef-retired"),
        Err(ModelsError::NotAClassifierModel { .. })
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}
