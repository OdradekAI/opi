//! MiniMax's fixed Pi 1.5 catalog and Anthropic Messages behavior.

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
    matchers::{method, path},
};

const PROVIDER: &str = "minimax";
const FIXTURE_KEY: &str = "fixture-minimax-key";
const PUBLISHED_IDS: [&str; 4] = [
    "MiniMax-M2.7",
    "MiniMax-M2.7-highspeed",
    "MiniMax-M3",
    "MiniMax-M3.1-Flash",
];

fn builtin_models(overrides: &BuiltinOverrides) -> Models {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[("MINIMAX_API_KEY", FIXTURE_KEY)]),
        Arc::new(opi_ai::http::HttpClient::new()),
        overrides,
    )
    .unwrap();
    models
}

#[tokio::test]
async fn builtin_minimax_lists_the_published_records_and_adaptive_metadata() {
    let catalog = Catalog::builtin().unwrap();
    let models = builtin_models(&BuiltinOverrides::default());
    let raw_bundle: Value = serde_json::from_slice(Catalog::builtin_bundle_bytes().0).unwrap();
    let author_source: Value = serde_json::from_slice(include_bytes!(
        "../../resources/catalog/providers/minimax.json"
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
            .all(|entry| entry.api == "anthropic-messages")
    );
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.id == "MiniMax-M3.1-Flash")
            .unwrap()
            .raw["compat"]["forceAdaptiveThinking"],
        true
    );
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.id == "MiniMax-M3.1-Flash")
            .unwrap()
            .raw["thinkingLevelMap"]["xhigh"],
        "xhigh"
    );
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.id == "MiniMax-M3")
            .unwrap()
            .raw["maxTokens"],
        524_288
    );
    let flash = entries
        .iter()
        .find(|entry| entry.id == "MiniMax-M3.1-Flash")
        .unwrap();
    assert_eq!(flash.raw["cost"]["input"], 0.15);
    assert_eq!(flash.raw["cost"]["output"], 0.6);
    assert_eq!(flash.raw["cost"]["cacheRead"], 0.015);
    assert_eq!(flash.raw["contextWindow"], 1_000_000);
    assert_eq!(flash.raw["maxTokens"], 524_288);
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.id == "MiniMax-M2.7")
            .unwrap()
            .raw["cost"]["input"],
        0.3
    );

    for entry in &entries {
        let info = entry.to_chat_model_info().unwrap();
        assert_eq!(models.get_model(PROVIDER, &entry.id).as_ref(), Some(&info));
        assert!(matches!(
            models.get_model_of_type(CatalogOperation::Chat, PROVIDER, &entry.id),
            Some(CatalogRecord::Chat(_))
        ));
    }
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
async fn builtin_minimax_sends_api_key_and_adaptive_xhigh_requests() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/anthropic/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(super::node5_reply(WireApi::AnthropicMessages)),
        )
        .expect(1)
        .mount(&server)
        .await;

    let mut overrides = BuiltinOverrides::default();
    overrides
        .base_urls
        .insert(PROVIDER.into(), format!("{}/anthropic", server.uri()));
    let models = builtin_models(&overrides);
    let id = "MiniMax-M3.1-Flash";
    let info = models.get_model(PROVIDER, id).unwrap();
    assert!(matches!(
        info.compat,
        WireCompat::AnthropicMessages(ref compat) if compat.force_adaptive_thinking
    ));
    assert_eq!(
        info.thinking_level_map.resolve(ThinkingLevel::XHigh),
        Ok(Some("xhigh".to_owned()))
    );
    let spec = format!("{PROVIDER}:{id}");
    let mut request = super::minimal_request(&spec);
    request.max_tokens = Some(12_345);
    request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::XHigh,
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

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 1);
    let request = &sent[0];
    assert_eq!(request.url.path(), "/anthropic/v1/messages");
    assert_eq!(request.headers["x-api-key"], FIXTURE_KEY);
    assert!(!request.headers.contains_key("authorization"));
    assert_eq!(request.headers["anthropic-version"], "2023-06-01");
    let body: Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["model"], id);
    assert_eq!(body["max_tokens"], 12_345);
    assert_eq!(body["thinking"]["type"], "adaptive");
    assert_eq!(body["output_config"]["effort"], "xhigh");
    assert!(body.get("reasoning_effort").is_none());
}

#[tokio::test]
async fn builtin_minimax_rejects_unknown_models_before_dispatch() {
    let server = MockServer::start().await;
    let mut overrides = BuiltinOverrides::default();
    overrides
        .base_urls
        .insert(PROVIDER.into(), format!("{}/anthropic", server.uri()));
    let models = builtin_models(&overrides);
    let listed = models.get_models_of_type(CatalogOperation::Chat, Some(PROVIDER));
    let available = models
        .get_available_of_type(CatalogOperation::Chat, Some(PROVIDER))
        .await
        .unwrap();
    assert_eq!(listed.len(), PUBLISHED_IDS.len());
    assert_eq!(available.len(), PUBLISHED_IDS.len());
    assert!(listed.iter().all(|record| record.id() != "MiniMax-M2.5"));
    assert!(available.iter().all(|record| record.id() != "MiniMax-M2.5"));
    assert!(models.get_model(PROVIDER, "MiniMax-M2.5").is_none());

    let spec = format!("{PROVIDER}:MiniMax-M2.5");
    assert!(matches!(
        models.prepare_call(&spec, super::minimal_request(&spec)).await,
        Err(ModelsError::Registry(RegistryError::UnknownModel { provider, model }))
            if provider == PROVIDER && model == "MiniMax-M2.5"
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}
