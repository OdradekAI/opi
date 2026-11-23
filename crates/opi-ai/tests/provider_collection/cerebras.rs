//! Cerebras' fixed Pi 1.5 catalog and OpenAI Completions dispatch.

use super::{
    Arc, BuiltinOverrides, Catalog, CatalogOperation, Models, WireApi, map_context,
    register_builtin,
};
use opi_ai::ThinkingLevel;
use opi_ai::WireCompat;
use opi_ai::catalog::CatalogRecord;
use opi_ai::models::{ChatModelRecord, ModelsError};
use opi_ai::provider::ThinkingConfig;
use opi_ai::registry::RegistryError;
use serde_json::Value;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};

const PROVIDER: &str = "cerebras";
const FIXTURE_KEY: &str = "fixture-cerebras-key";
const PUBLISHED_IDS: [&str; 2] = ["gpt-oss-120b", "qwen-3.8-27b"];

fn builtin_models(overrides: &BuiltinOverrides) -> Models {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[("CEREBRAS_API_KEY", FIXTURE_KEY)]),
        Arc::new(opi_ai::http::HttpClient::new()),
        overrides,
    )
    .unwrap();
    models
}

fn loopback_base(server: &MockServer) -> String {
    format!("{}/v1", server.uri())
}

#[tokio::test]
async fn builtin_cerebras_lists_the_published_catalog() {
    let catalog = Catalog::builtin().unwrap();
    let models = builtin_models(&BuiltinOverrides::default());
    let raw_bundle: Value = serde_json::from_slice(Catalog::builtin_bundle_bytes().0).unwrap();
    let author_source: Value = serde_json::from_slice(include_bytes!(
        "../../resources/catalog/providers/cerebras.json"
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
    let expected: Vec<_> = entries
        .iter()
        .map(|entry| {
            assert_eq!(entry.api, "openai-completions");
            let info = entry.to_chat_model_info().unwrap();
            assert_eq!(models.get_model(PROVIDER, &entry.id).as_ref(), Some(&info));
            CatalogRecord::from(info)
        })
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
    for id in PUBLISHED_IDS {
        assert_eq!(
            models
                .get_model_of_type(CatalogOperation::Chat, PROVIDER, id)
                .as_ref(),
            Some(&CatalogRecord::Chat(Box::new(
                models.get_model(PROVIDER, id).unwrap(),
            )))
        );
    }

    let gpt = entries
        .iter()
        .find(|entry| entry.id == "gpt-oss-120b")
        .unwrap();
    assert_eq!(gpt.raw["cost"]["cacheRead"], 0.35);
    assert_eq!(gpt.raw["compat"]["supportsStore"], false);
    assert_eq!(gpt.raw["compat"]["supportsDeveloperRole"], false);

    let info = models.get_model(PROVIDER, "gpt-oss-120b").unwrap();
    assert!(matches!(&info.compat, WireCompat::OpenAiCompletions(_)));
    let WireCompat::OpenAiCompletions(compat) = &info.compat else {
        unreachable!();
    };
    assert!(compat.max_tokens_field.is_none());
    assert!(compat.supports_reasoning_effort.is_none());
}

#[tokio::test]
async fn builtin_cerebras_sends_bearer_completions_with_published_parameters() {
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
        .insert(PROVIDER.into(), loopback_base(&server));
    let models = builtin_models(&overrides);
    let model_id = "gpt-oss-120b";
    let mut request = super::minimal_request(&format!("{PROVIDER}:{model_id}"));
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
    assert_eq!(super::assistant_text(&output), "hi");

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 1);
    let request = &sent[0];
    assert_eq!(request.url.path(), "/v1/chat/completions");
    assert_eq!(
        request.headers["authorization"],
        format!("Bearer {FIXTURE_KEY}")
    );
    assert!(!request.headers.contains_key("x-api-key"));
    let body: Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["model"], model_id);
    assert_eq!(body["max_completion_tokens"], 4_096);
    assert!(body.get("max_tokens").is_none());
    assert_eq!(body["reasoning_effort"], "high");
}

#[tokio::test]
async fn builtin_cerebras_rejects_unknown_models_before_dispatch() {
    let server = MockServer::start().await;
    let mut overrides = BuiltinOverrides::default();
    overrides
        .base_urls
        .insert(PROVIDER.into(), loopback_base(&server));
    let models = builtin_models(&overrides);
    let catalog = Catalog::builtin().unwrap();
    let listed = models.get_models_of_type(CatalogOperation::Chat, Some(PROVIDER));
    let available = models
        .get_available_of_type(CatalogOperation::Chat, Some(PROVIDER))
        .await
        .unwrap();
    assert_eq!(listed.len(), PUBLISHED_IDS.len());
    assert_eq!(available.len(), PUBLISHED_IDS.len());

    let retired = "gpt-oss-120b-retired";
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
    assert!(server.received_requests().await.unwrap().is_empty());
}
