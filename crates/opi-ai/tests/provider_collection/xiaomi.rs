//! Xiaomi's fixed Pi 1.5 catalog and OpenAI Completions behavior.

use super::{
    Arc, BuiltinOverrides, Catalog, CatalogOperation, Models, WireApi, map_context,
    register_builtin,
};
use opi_ai::ThinkingLevel;
use opi_ai::catalog::CatalogRecord;
use opi_ai::models::{ChatModelRecord, ModelsError};
use opi_ai::provider::ThinkingConfig;
use opi_ai::registry::RegistryError;
use serde_json::Value;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};

const PROVIDER: &str = "xiaomi";
const FIXTURE_KEY: &str = "fixture-xiaomi-key";
const PUBLISHED_IDS: [&str; 3] = [
    "mimo-v2.6-flash",
    "mimo-v2.6-pro",
    "mimo-v2.6-pro-ultraspeed",
];

fn builtin_models(overrides: &BuiltinOverrides) -> Models {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[("XIAOMI_API_KEY", FIXTURE_KEY)]),
        Arc::new(opi_ai::http::HttpClient::new()),
        overrides,
    )
    .unwrap();
    models
}

#[tokio::test]
async fn builtin_xiaomi_lists_the_published_catalog() {
    let catalog = Catalog::builtin().unwrap();
    let models = builtin_models(&BuiltinOverrides::default());
    let raw_bundle: Value = serde_json::from_slice(Catalog::builtin_bundle_bytes().0).unwrap();
    let author_source: Value = serde_json::from_slice(include_bytes!(
        "../../resources/catalog/providers/xiaomi.json"
    ))
    .unwrap();
    assert_eq!(raw_bundle[PROVIDER], author_source);

    let entries: Vec<_> = catalog
        .models(PROVIDER, CatalogOperation::Chat)
        .unwrap()
        .collect();
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        PUBLISHED_IDS
    );
    assert!(
        entries
            .iter()
            .all(|entry| entry.api == "openai-completions")
    );

    for entry in &entries {
        let info = entry.to_chat_model_info().unwrap();
        assert_eq!(models.get_model(PROVIDER, &entry.id).as_ref(), Some(&info));
        assert!(matches!(
            models.get_model_of_type(CatalogOperation::Chat, PROVIDER, &entry.id),
            Some(CatalogRecord::Chat(_))
        ));
        assert_eq!(entry.raw["contextWindow"], 1_048_576);
    }

    let flash = entries
        .iter()
        .find(|entry| entry.id == "mimo-v2.6-flash")
        .unwrap();
    assert_eq!(flash.raw["cost"]["input"], 0.14);
    assert_eq!(flash.raw["cost"]["output"], 0.28);
    assert_eq!(flash.raw["cost"]["cacheRead"], 0.0028);
    assert_eq!(flash.raw["maxTokens"], 131_072);
    assert_eq!(flash.raw["compat"]["thinkingFormat"], "deepseek");
    assert_eq!(flash.raw["compat"]["supportsReasoningEffort"], false);
}

#[tokio::test]
async fn builtin_xiaomi_sends_bearer_deepseek_thinking_to_chat_completions() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", format!("Bearer {FIXTURE_KEY}")))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(super::node5_reply(WireApi::OpenAiCompletions)),
        )
        .expect(1)
        .mount(&server)
        .await;

    let mut overrides = BuiltinOverrides::default();
    overrides
        .base_urls
        .insert(PROVIDER.into(), format!("{}/v1", server.uri()));
    let models = builtin_models(&overrides);
    let model_id = "mimo-v2.6-pro";
    let spec = format!("{PROVIDER}:{model_id}");
    let mut request = super::minimal_request(&spec);
    request.max_tokens = Some(4_096);
    request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::High,
        budget_tokens: None,
    };
    let info = models.get_model(PROVIDER, model_id).unwrap();
    let output = models
        .complete_simple(
            ChatModelRecord::new(PROVIDER, info),
            request,
            Default::default(),
        )
        .await
        .unwrap_or_else(|error| panic!("{model_id}: {error:?}"));
    assert_eq!(output.provider, PROVIDER);
    assert_eq!(output.model, model_id);

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 1);
    let request = &sent[0];
    assert_eq!(request.url.path(), "/v1/chat/completions");
    assert_eq!(
        request.headers["authorization"],
        format!("Bearer {FIXTURE_KEY}")
    );
    let body: Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["model"], model_id);
    assert_eq!(body["max_completion_tokens"], 4_096);
    assert_eq!(body["thinking"]["type"], "enabled");
    assert!(body.get("reasoning_effort").is_none());
    assert!(body.get("max_tokens").is_none());
}

#[tokio::test]
async fn builtin_xiaomi_removes_retired_models_before_dispatch() {
    let server = MockServer::start().await;
    let mut overrides = BuiltinOverrides::default();
    overrides
        .base_urls
        .insert(PROVIDER.into(), format!("{}/v1", server.uri()));
    let models = builtin_models(&overrides);
    let catalog = Catalog::builtin().unwrap();
    let listed = models.get_models_of_type(CatalogOperation::Chat, Some(PROVIDER));
    let available = models
        .get_available_of_type(CatalogOperation::Chat, Some(PROVIDER))
        .await
        .unwrap();
    assert_eq!(listed.len(), PUBLISHED_IDS.len());
    assert_eq!(available.len(), PUBLISHED_IDS.len());
    for retired in ["mimo-v2.5", "mimo-v2.5-pro", "mimo-v2.5-pro-ultraspeed"] {
        assert!(
            catalog
                .model(PROVIDER, CatalogOperation::Chat, retired)
                .is_none()
        );
        assert!(listed.iter().all(|record| record.id() != retired));
        assert!(available.iter().all(|record| record.id() != retired));
        assert!(models.get_model(PROVIDER, retired).is_none());

        let spec = format!("{PROVIDER}:{retired}");
        assert!(matches!(
            models.prepare_call(&spec, super::minimal_request(&spec)).await,
            Err(ModelsError::Registry(RegistryError::UnknownModel { provider, model }))
                if provider == PROVIDER && model == retired
        ));
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}
