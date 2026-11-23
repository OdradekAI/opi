//! Radius' fixed Pi 1.5 catalog, dynamic refresh, and Pi Messages route.

use super::{
    Arc, BuiltinOverrides, Catalog, CatalogOperation, Models, WireApi, map_context,
    register_builtin,
};
use opi_ai::catalog::CatalogRecord;
use opi_ai::models::{ChatModelRecord, ModelsError};
use opi_ai::provider::ThinkingConfig;
use opi_ai::registry::RegistryError;
use opi_ai::{ThinkingLevel, WireCompat};
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};

const PROVIDER: &str = "radius";
const FIXTURE_KEY: &str = "radius-fixture-key";
const PUBLISHED_IDS: [&str; 28] = [
    "balanced",
    "cheap",
    "claude-fable-5",
    "claude-fable-5-1",
    "claude-haiku-4-5",
    "claude-haiku-5-5",
    "claude-opus-4-5",
    "claude-opus-4-8",
    "claude-opus-5",
    "claude-opus-5-5",
    "claude-sonnet-4-5",
    "claude-sonnet-5",
    "claude-sonnet-5-5",
    "deepseek-v4.1-flash",
    "glm-5.3",
    "glm-5.3-flash",
    "gpt-5.6-luna",
    "gpt-5.6-sol",
    "gpt-5.6-terra",
    "gpt-6-astra",
    "gpt-6-luna",
    "gpt-6-sol",
    "gpt-6.1-sol",
    "grok-4.7",
    "kimi-k3",
    "minimax-m3.1-flash",
    "precise",
    "recursive/flint",
];

fn builtin_models(overrides: &BuiltinOverrides) -> Models {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[("RADIUS_API_KEY", FIXTURE_KEY)]),
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
async fn builtin_radius_lists_the_published_catalog() {
    let catalog = Catalog::builtin().unwrap();
    let models = builtin_models(&BuiltinOverrides::default());
    let raw_bundle: Value = serde_json::from_slice(Catalog::builtin_bundle_bytes().0).unwrap();
    let author_source: Value = serde_json::from_slice(include_bytes!(
        "../../resources/catalog/providers/radius.json"
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
            assert_eq!(entry.api, "pi-messages");
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

    let flint = catalog
        .model(PROVIDER, CatalogOperation::Chat, "recursive/flint")
        .unwrap();
    assert_eq!(flint.raw["free"], true);
    let glm = models.get_model(PROVIDER, "glm-5.3").unwrap();
    assert_eq!(
        glm.thinking_level_map.resolve(ThinkingLevel::Max),
        Ok(Some("max".to_owned()))
    );
    assert!(
        glm.thinking_level_map
            .resolve(ThinkingLevel::Medium)
            .is_err()
    );
    assert!(
        glm.thinking_level_map
            .resolve(ThinkingLevel::Minimal)
            .is_err()
    );
    assert!(
        glm.thinking_level_map
            .resolve(ThinkingLevel::XHigh)
            .is_err()
    );
    assert!(matches!(glm.compat, WireCompat::PiMessages));
}

#[tokio::test]
async fn builtin_radius_refreshes_and_sends_pi_messages_with_bearer() {
    let server = MockServer::start().await;
    let catalog = Catalog::builtin().unwrap();
    let model_id = "gpt-6.1-sol";
    let mut model = catalog
        .model(PROVIDER, CatalogOperation::Chat, model_id)
        .unwrap()
        .raw
        .clone();
    model["baseUrl"] = json!(loopback_base(&server));

    Mock::given(method("GET"))
        .and(path("/v1/config"))
        .and(header("authorization", format!("Bearer {FIXTURE_KEY}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "baseUrl": loopback_base(&server),
            "models": [model],
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("authorization", format!("Bearer {FIXTURE_KEY}")))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(super::node5_reply(WireApi::PiMessages)),
        )
        .expect(1)
        .mount(&server)
        .await;

    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert(PROVIDER.into(), server.uri());
    let mut models = builtin_models(&overrides);
    let refresh = models
        .refresh(opi_ai::ModelsRefreshOptions {
            providers: Some([PROVIDER.to_owned()].into_iter().collect()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        refresh.find(PROVIDER).unwrap().outcome,
        opi_ai::RefreshOutcome::Published { models: 1 }
    );

    let info = models.get_model(PROVIDER, model_id).unwrap();
    assert_eq!(
        info.base_url.as_deref(),
        Some(loopback_base(&server).as_str())
    );
    let mut request = super::minimal_request(&format!("{PROVIDER}:{model_id}"));
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
        .unwrap_or_else(|error| panic!("{model_id}: {error:?}"));
    assert_eq!(output.provider, PROVIDER);
    assert_eq!(output.model, model_id);
    assert_eq!(super::assistant_text(&output), "hi");

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 2);
    let config = sent
        .iter()
        .find(|request| request.url.path() == "/v1/config")
        .unwrap();
    assert_eq!(
        config.headers["authorization"],
        format!("Bearer {FIXTURE_KEY}")
    );
    let message = sent
        .iter()
        .find(|request| request.url.path() == "/v1/messages")
        .unwrap();
    assert_eq!(
        message.headers["authorization"],
        format!("Bearer {FIXTURE_KEY}")
    );
    let body: Value = serde_json::from_slice(&message.body).unwrap();
    assert_eq!(body["model"], model_id);
    assert!(body["context"]["messages"].is_array());
    assert_eq!(body["options"]["reasoning"], "high");
}

#[tokio::test]
async fn builtin_radius_retires_models_before_dispatch() {
    let server = MockServer::start().await;
    let models = builtin_models(&BuiltinOverrides::default());
    let catalog = Catalog::builtin().unwrap();
    let listed = models.get_models_of_type(CatalogOperation::Chat, Some(PROVIDER));
    let available = models
        .get_available_of_type(CatalogOperation::Chat, Some(PROVIDER))
        .await
        .unwrap();
    assert_eq!(listed.len(), PUBLISHED_IDS.len());
    assert_eq!(available.len(), PUBLISHED_IDS.len());

    for retired in ["gpt-5.3-codex", "gpt-5.4", "gpt-5.4-mini", "gpt-5.5"] {
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
