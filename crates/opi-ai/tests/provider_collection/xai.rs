//! xAI's fixed Pi 1.5 catalog and OpenAI Responses dispatch.

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
    matchers::{method, path},
};

const PROVIDER: &str = "xai";
const FIXTURE_KEY: &str = "fixture-xai-key";
const PUBLISHED_IDS: [&str; 4] = ["grok-4.3", "grok-4.5", "grok-4.6", "grok-4.7"];

fn builtin_models(overrides: &BuiltinOverrides) -> Models {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[("XAI_API_KEY", FIXTURE_KEY)]),
        Arc::new(opi_ai::http::HttpClient::new()),
        overrides,
    )
    .unwrap();
    models
}

fn loopback_base(server: &MockServer, published_base: &str) -> String {
    let published = url::Url::parse(published_base).unwrap();
    assert_eq!(published.host_str(), Some("api.x.ai"));
    format!("{}{}", server.uri(), published.path().trim_end_matches('/'))
}

#[tokio::test]
async fn builtin_xai_lists_the_published_chat_records_and_xhigh_mapping() {
    let catalog = Catalog::builtin().unwrap();
    let models = builtin_models(&BuiltinOverrides::default());
    let raw_bundle: Value = serde_json::from_slice(Catalog::builtin_bundle_bytes().0).unwrap();
    let author_source: Value =
        serde_json::from_slice(include_bytes!("../../resources/catalog/providers/xai.json"))
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
            assert_eq!(entry.api, "openai-responses");
            let info = entry.to_chat_model_info().unwrap();
            assert_eq!(models.get_model(PROVIDER, &entry.id).as_ref(), Some(&info));
            if matches!(entry.id.as_str(), "grok-4.3" | "grok-4.5") {
                assert_eq!(
                    info.thinking_level_map.resolve(ThinkingLevel::XHigh),
                    Ok(Some("xhigh".to_owned()))
                );
            }
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
                models.get_model(PROVIDER, id).unwrap()
            )))
        );
    }
}

#[tokio::test]
async fn builtin_xai_sends_xhigh_reasoning_to_responses() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(super::node5_reply(WireApi::OpenAiResponses)),
        )
        .expect(2)
        .mount(&server)
        .await;

    let models = builtin_models(&BuiltinOverrides::default());
    for id in ["grok-4.3", "grok-4.5"] {
        let mut info = models.get_model(PROVIDER, id).unwrap();
        info.base_url = Some(loopback_base(&server, info.base_url.as_deref().unwrap()));
        let spec = format!("{PROVIDER}:{id}");
        let mut request = super::minimal_request(&spec);
        request.max_tokens = Some(1_024);
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
        assert_eq!(super::assistant_text(&output), "hi");
    }

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 2);
    for (request, id) in sent.iter().zip(["grok-4.3", "grok-4.5"]) {
        assert_eq!(request.url.path(), "/v1/responses");
        assert_eq!(
            request.headers["authorization"],
            format!("Bearer {FIXTURE_KEY}")
        );
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["model"], id);
        assert_eq!(body["stream"], true);
        assert_eq!(body["reasoning"]["effort"], "xhigh");
        assert_eq!(
            body["include"],
            serde_json::json!(["reasoning.encrypted_content"])
        );
    }
}

#[tokio::test]
async fn builtin_xai_unknown_models_are_rejected_before_dispatch() {
    let server = MockServer::start().await;
    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert(PROVIDER.into(), server.uri());
    let models = builtin_models(&overrides);
    let catalog = Catalog::builtin().unwrap();
    let listed = models.get_models_of_type(CatalogOperation::Chat, Some(PROVIDER));
    let available = models
        .get_available_of_type(CatalogOperation::Chat, Some(PROVIDER))
        .await
        .unwrap();
    assert_eq!(listed.len(), PUBLISHED_IDS.len());
    assert_eq!(available.len(), PUBLISHED_IDS.len());
    assert!(
        catalog
            .model(PROVIDER, CatalogOperation::Chat, "grok-retired")
            .is_none()
    );
    assert!(listed.iter().all(|record| record.id() != "grok-retired"));
    assert!(available.iter().all(|record| record.id() != "grok-retired"));
    assert!(models.get_model(PROVIDER, "grok-retired").is_none());

    let spec = format!("{PROVIDER}:grok-retired");
    assert!(matches!(
        models.prepare_call(&spec, super::minimal_request(&spec)).await,
        Err(ModelsError::Registry(RegistryError::UnknownModel { provider, model }))
            if provider == PROVIDER && model == "grok-retired"
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}
