//! Qwen Token Plan CN's fixed Pi 1.5 catalog and OpenAI Completions behavior.

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
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

const PROVIDER: &str = "qwen-token-plan-cn";
const FIXTURE_KEY: &str = "fixture-qwen-token-plan-cn-key";
const PUBLISHED_IDS: [&str; 21] = [
    "MiniMax-M2.5",
    "deepseek-v3.2",
    "deepseek-v4-flash",
    "deepseek-v4-flash-0731",
    "deepseek-v4-pro",
    "deepseek-v4-pro-0813",
    "deepseek-v4.1-flash",
    "deepseek-v4.1-flash-1017",
    "glm-5",
    "glm-5.1",
    "glm-5.2",
    "glm-5.3",
    "kimi-k2.5",
    "kimi-k2.6",
    "kimi-k2.7-code",
    "qwen3.6-flash",
    "qwen3.6-plus",
    "qwen3.7-max",
    "qwen3.7-plus",
    "qwen3.8-flash",
    "qwen3.8-max",
];

fn builtin_models(overrides: &BuiltinOverrides) -> Models {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[("QWEN_TOKEN_PLAN_CN_API_KEY", FIXTURE_KEY)]),
        Arc::new(opi_ai::http::HttpClient::new()),
        overrides,
    )
    .unwrap();
    models
}

#[tokio::test]
async fn builtin_qwen_token_plan_cn_lists_the_published_records_and_reasoning_metadata() {
    let catalog = Catalog::builtin().unwrap();
    let models = builtin_models(&BuiltinOverrides::default());
    let raw_bundle: Value = serde_json::from_slice(Catalog::builtin_bundle_bytes().0).unwrap();
    let author_source: Value = serde_json::from_slice(include_bytes!(
        "../../resources/catalog/providers/qwen-token-plan-cn.json"
    ))
    .unwrap();
    assert_eq!(raw_bundle[PROVIDER], author_source);

    let entries: Vec<_> = catalog
        .models(PROVIDER, CatalogOperation::Chat)
        .unwrap()
        .collect();
    assert_eq!(entries.len(), PUBLISHED_IDS.len());
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
    }

    let entry = entries
        .iter()
        .find(|entry| entry.id == "deepseek-v4.1-flash-1017")
        .unwrap();
    assert_eq!(entry.raw["compat"]["supportsReasoningEffort"], true);
    assert_eq!(entry.raw["thinkingLevelMap"]["high"], "high");
    assert_eq!(entry.raw["thinkingLevelMap"]["max"], "max");
    assert_eq!(entry.raw["cost"]["input"], 0);
    assert_eq!(entry.raw["contextWindow"], 1_000_000);
    assert_eq!(
        models
            .get_model(PROVIDER, "deepseek-v4.1-flash-1017")
            .unwrap()
            .capabilities
            .context_window,
        1_000_000
    );

    let expected: Vec<_> = entries
        .iter()
        .map(|entry| CatalogRecord::Chat(Box::new(entry.to_chat_model_info().unwrap())))
        .collect();
    assert_eq!(
        models.get_models_of_type(CatalogOperation::Chat, Some(PROVIDER)),
        expected
    );
    assert_eq!(
        models
            .get_available_of_type(CatalogOperation::Chat, Some(PROVIDER))
            .await
            .unwrap(),
        expected
    );
}

#[tokio::test]
async fn builtin_qwen_token_plan_cn_sends_bearer_high_reasoning_to_chat_completions() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(super::node5_reply(WireApi::OpenAiCompletions)),
        )
        .expect(1)
        .mount(&server)
        .await;

    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert(
        PROVIDER.into(),
        format!("{}/compatible-mode/v1", server.uri()),
    );
    let models = builtin_models(&overrides);
    for id in ["deepseek-v4.1-flash-1017"] {
        let info = models.get_model(PROVIDER, id).unwrap();
        let spec = format!("{PROVIDER}:{id}");
        let mut request = super::minimal_request(&spec);
        request.max_tokens = Some(12_345);
        request.thinking = ThinkingConfig {
            enabled: true,
            level: ThinkingLevel::High,
            budget_tokens: None,
        };
        let output = models
            .complete_simple(
                ChatModelRecord::new(PROVIDER, info),
                request,
                Default::default(),
            )
            .await
            .unwrap_or_else(|error| panic!("{id}: {error:?}"));
        assert_eq!(output.provider, PROVIDER);
        assert_eq!(output.model, id);
    }

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 1);
    for (request, id) in sent.iter().zip(["deepseek-v4.1-flash-1017"]) {
        assert_eq!(request.url.path(), "/compatible-mode/v1/chat/completions");
        assert_eq!(
            request.headers["authorization"],
            format!("Bearer {FIXTURE_KEY}")
        );
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["model"], id);
        assert_eq!(body["max_completion_tokens"], 12_345, "body={body}");
        assert!(body.get("max_tokens").is_none());
        assert_eq!(body["enable_thinking"], true);
        assert_eq!(body["reasoning_effort"], "high");
    }
}

#[tokio::test]
async fn builtin_qwen_token_plan_cn_has_no_retired_models_in_any_entrypoint() {
    let server = MockServer::start().await;
    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert(
        PROVIDER.into(),
        format!("{}/compatible-mode/v1", server.uri()),
    );
    let models = builtin_models(&overrides);
    let listed = models.get_models_of_type(CatalogOperation::Chat, Some(PROVIDER));
    let available = models
        .get_available_of_type(CatalogOperation::Chat, Some(PROVIDER))
        .await
        .unwrap();
    assert_eq!(listed.len(), PUBLISHED_IDS.len());
    assert_eq!(available.len(), PUBLISHED_IDS.len());
    assert!(
        listed
            .iter()
            .all(|record| record.id() != "deepseek-v4.1-flash-1017-retired")
    );
    assert!(
        available
            .iter()
            .all(|record| record.id() != "deepseek-v4.1-flash-1017-retired")
    );
    assert!(
        models
            .get_model(PROVIDER, "deepseek-v4.1-flash-1017-retired")
            .is_none()
    );

    let spec = format!("{PROVIDER}:deepseek-v4.1-flash-1017-retired");
    assert!(matches!(
        models.prepare_call(&spec, super::minimal_request(&spec)).await,
        Err(ModelsError::Registry(RegistryError::UnknownModel { provider, model }))
            if provider == PROVIDER && model == "deepseek-v4.1-flash-1017-retired"
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}
