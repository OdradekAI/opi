//! Baseten's builtin catalog selection and OpenAI-compatible wire dispatch.

use super::{
    Arc, BuiltinOverrides, Catalog, CatalogOperation, Models, map_context, register_builtin,
};
use opi_ai::catalog::CatalogRecord;
use opi_ai::models::ModelsError;
use opi_ai::provider::ThinkingConfig;
use opi_ai::registry::RegistryError;
use opi_ai::stream::StopReason;
use opi_ai::{ChatModelRecord, ThinkingLevel, WireApi};
use serde_json::Value;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

const PROVIDER: &str = "baseten";
const FIXTURE_KEY: &str = "fixture-baseten-key";

fn builtin_models(overrides: &BuiltinOverrides) -> Models {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[("BASETEN_API_KEY", FIXTURE_KEY)]),
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
async fn builtin_baseten_lists_the_published_chat_records_across_sdk_entries() {
    let catalog = Catalog::builtin().unwrap();
    let models = builtin_models(&BuiltinOverrides::default());
    let raw_bundle: Value = serde_json::from_slice(Catalog::builtin_bundle_bytes().0).unwrap();
    let author_source: Value = serde_json::from_slice(include_bytes!(
        "../../resources/catalog/providers/baseten.json"
    ))
    .unwrap();
    assert_eq!(raw_bundle[PROVIDER], author_source);

    let expected: Vec<_> = catalog
        .models(PROVIDER, CatalogOperation::Chat)
        .unwrap()
        .map(|entry| {
            assert_eq!(entry.api, "openai-completions");
            let info = entry.to_chat_model_info().unwrap();
            assert_eq!(models.get_model(PROVIDER, &entry.id).as_ref(), Some(&info));
            assert_eq!(
                models
                    .resolve(&format!("{PROVIDER}:{}", entry.id))
                    .unwrap()
                    .1,
                &info
            );
            CatalogRecord::from(info)
        })
        .collect();
    assert_eq!(expected.len(), 14);
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
    for record in &expected {
        assert_eq!(
            models
                .get_model_of_type(CatalogOperation::Chat, PROVIDER, record.id())
                .as_ref(),
            Some(record)
        );
    }
    assert_eq!(models.get_all_models(Some(PROVIDER)), expected);
}

#[tokio::test]
async fn builtin_baseten_uses_chat_completions_auth_and_published_reasoning_fields() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(super::node5_reply(WireApi::OpenAiCompletions)),
        )
        .expect(4)
        .mount(&server)
        .await;
    let models = builtin_models(&BuiltinOverrides::default());
    let cases = [
        (
            "deepseek-ai/DeepSeek-V4-Flash-0731",
            ThinkingLevel::XHigh,
            Some("max"),
            None,
        ),
        (
            "nvidia/NVIDIA-Nemotron-3-Ultra-550B-A55B",
            ThinkingLevel::High,
            Some("high"),
            None,
        ),
        ("zai-org/GLM-5.3-Fast", ThinkingLevel::None, None, None),
        (
            "zai-org/GLM-5.2",
            ThinkingLevel::High,
            Some("high"),
            Some(true),
        ),
    ];

    for (id, level, effort, template_enabled) in cases {
        let mut info = models.get_model(PROVIDER, id).unwrap();
        info.base_url = Some(loopback_base(&server));
        let mut request = super::minimal_request(&format!("{PROVIDER}:{id}"));
        request.max_tokens = Some(4_096);
        request.thinking = ThinkingConfig {
            enabled: level != ThinkingLevel::None,
            level,
            budget_tokens: None,
        };
        let output = models
            .complete_simple(
                ChatModelRecord::new(PROVIDER, info),
                request,
                Default::default(),
            )
            .await
            .unwrap_or_else(|error| panic!("{id}, {level:?}: {error:?}"));
        assert_eq!(output.provider, PROVIDER);
        assert_eq!(output.model, id);
        assert_eq!(output.stop_reason, StopReason::Stop);
        assert_eq!(super::assistant_text(&output), "hi");

        let sent = server.received_requests().await.unwrap();
        let request = sent.last().unwrap();
        assert_eq!(request.url.path(), "/v1/chat/completions");
        assert_eq!(request.url.query(), None);
        assert_eq!(
            request.headers["authorization"],
            format!("Bearer {FIXTURE_KEY}")
        );
        assert!(!request.headers.contains_key("x-api-key"));
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["model"], id);
        assert_eq!(body["stream"], true);
        assert_eq!(body["stream_options"]["include_usage"], true);
        assert_eq!(body["max_tokens"], 4_096);
        match effort {
            Some(value) => assert_eq!(body["reasoning_effort"], value),
            None => assert!(body.get("reasoning_effort").is_none(), "{id}: {body}"),
        }
        match template_enabled {
            Some(enabled) => assert_eq!(body["chat_template_args"]["enable_thinking"], enabled),
            None => assert!(body.get("chat_template_args").is_none(), "{id}: {body}"),
        }
    }
}

#[tokio::test]
async fn builtin_baseten_retired_models_fail_selection_without_sending_requests() {
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
    for id in [
        "deepseek-ai/DeepSeek-V4-Pro",
        "moonshotai/Kimi-K2.5",
        "moonshotai/Kimi-K2.6",
        "moonshotai/Kimi-K2.7-Code",
        "nvidia/Nemotron-120B-A12B",
        "thinkingmachines/inkling",
        "thinkingmachines/inkling-small",
        "zai-org/GLM-4.7",
        "zai-org/GLM-5",
        "zai-org/GLM-5.1",
    ] {
        assert!(
            catalog
                .model(PROVIDER, CatalogOperation::Chat, id)
                .is_none(),
            "{id}"
        );
        assert!(listed.iter().all(|record| record.id() != id), "{id}");
        assert!(available.iter().all(|record| record.id() != id), "{id}");
        assert!(models.get_model(PROVIDER, id).is_none(), "{id}");
        let spec = format!("{PROVIDER}:{id}");
        assert!(
            matches!(
                models.prepare_call(&spec, super::minimal_request(&spec)).await,
                Err(ModelsError::Registry(RegistryError::UnknownModel { provider, model }))
                    if provider == PROVIDER && model == id
            ),
            "{id}"
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}
