//! Xiaomi Token Plan CN's fixed Pi 1.5 catalog and OpenAI Completions behavior.

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

const PROVIDER: &str = "xiaomi-token-plan-cn";
const FIXTURE_KEY: &str = "fixture-xiaomi-token-plan-cn-key";
const PUBLISHED_IDS: [&str; 2] = ["mimo-v2.6-flash", "mimo-v2.6-pro"];

fn builtin_models(overrides: &BuiltinOverrides) -> Models {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[("XIAOMI_TOKEN_PLAN_CN_API_KEY", FIXTURE_KEY)]),
        Arc::new(opi_ai::http::HttpClient::new()),
        overrides,
    )
    .unwrap();
    models
}

#[tokio::test]
async fn builtin_xiaomi_token_plan_cn_lists_the_published_catalog() {
    let catalog = Catalog::builtin().unwrap();
    let models = builtin_models(&BuiltinOverrides::default());
    let raw_bundle: Value = serde_json::from_slice(Catalog::builtin_bundle_bytes().0).unwrap();
    let author_source: Value = serde_json::from_slice(include_bytes!(
        "../../resources/catalog/providers/xiaomi-token-plan-cn.json"
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
    for entry in &entries {
        assert_eq!(entry.api, "openai-completions");
        assert_eq!(entry.raw["contextWindow"], 1_048_576);
        let info = entry.to_chat_model_info().unwrap();
        assert_eq!(models.get_model(PROVIDER, &entry.id).as_ref(), Some(&info));
        assert!(matches!(
            models.get_model_of_type(CatalogOperation::Chat, PROVIDER, &entry.id),
            Some(CatalogRecord::Chat(_))
        ));
    }
    assert_eq!(entries[0].raw["cost"]["input"], 0);
    assert_eq!(entries[0].raw["cost"]["output"], 0);
    assert_eq!(entries[0].raw["cost"]["cacheRead"], 0);
}

#[tokio::test]
async fn builtin_xiaomi_token_plan_cn_sends_bearer_deepseek_thinking() {
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
    let output = models
        .complete_simple(
            ChatModelRecord::new(PROVIDER, models.get_model(PROVIDER, model_id).unwrap()),
            request,
            Default::default(),
        )
        .await
        .unwrap();
    assert_eq!(output.provider, PROVIDER);
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
    assert!(body.get("max_tokens").is_none());
    assert!(body.get("reasoning_effort").is_none());
}

#[tokio::test]
async fn builtin_xiaomi_token_plan_cn_removes_retired_models() {
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
    assert_eq!(listed.len(), 2);
    assert_eq!(available.len(), 2);
    for retired in ["mimo-v2.5", "mimo-v2.5-pro"] {
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
