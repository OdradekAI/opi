//! NVIDIA's fixed Pi 1.5 catalog and OpenAI Completions behavior.

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

const PROVIDER: &str = "nvidia";
const FIXTURE_KEY: &str = "fixture-nvidia-key";
const PUBLISHED_IDS: [&str; 21] = [
    "deepseek-ai/deepseek-v4.1-flash",
    "google/diffusiongemma-26b-a4b-it",
    "google/gemma-3-12b-it",
    "google/gemma-3-4b-it",
    "meta/llama-3.2-11b-vision-instruct",
    "meta/llama-3.2-90b-vision-instruct",
    "meta/muse-glimmer-30b",
    "minimaxai/minimax-m3.1-flash",
    "mistralai/mistral-7b-instruct-v0.3",
    "moonshotai/kimi-k2.6",
    "nvidia/cosmos-reason2-8b",
    "nvidia/llama-3.1-nemotron-70b-instruct",
    "nvidia/llama-3.1-nemotron-ultra-253b-v1",
    "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning",
    "nvidia/nemotron-3-ultra-550b-a55b",
    "nvidia/nemotron-3.5-lightning-30b-a3b",
    "nvidia/nemotron-3.5-super-vl-120b-a12b",
    "openai/gpt-oss-20b",
    "poolside/laguna-xs-2.1",
    "stepfun-ai/step-5-preview",
    "z-ai/glm-5.3-flash",
];

fn builtin_models(overrides: &BuiltinOverrides) -> Models {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[("NVIDIA_API_KEY", FIXTURE_KEY)]),
        Arc::new(opi_ai::http::HttpClient::new()),
        overrides,
    )
    .unwrap();
    models
}

#[tokio::test]
async fn builtin_nvidia_lists_the_published_records_and_metadata() {
    let catalog = Catalog::builtin().unwrap();
    let models = builtin_models(&BuiltinOverrides::default());
    let raw_bundle: Value = serde_json::from_slice(Catalog::builtin_bundle_bytes().0).unwrap();
    let author_source: Value = serde_json::from_slice(include_bytes!(
        "../../resources/catalog/providers/nvidia.json"
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
    for id in [
        "minimaxai/minimax-m3.1-flash",
        "nvidia/nemotron-3.5-super-vl-120b-a12b",
        "stepfun-ai/step-5-preview",
    ] {
        assert!(entries.iter().any(|entry| entry.id == id));
    }
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.id == "nvidia/nemotron-3-ultra-550b-a55b")
            .unwrap()
            .raw["cost"],
        serde_json::json!({
            "input": 0,
            "output": 0,
            "cacheRead": 0,
            "cacheWrite": 0
        })
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
async fn builtin_nvidia_sends_bearer_requests_and_catalog_reasoning_parameters() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(super::node5_reply(WireApi::OpenAiCompletions)),
        )
        .expect(2)
        .mount(&server)
        .await;

    let mut overrides = BuiltinOverrides::default();
    overrides
        .base_urls
        .insert(PROVIDER.into(), format!("{}/v1", server.uri()));
    let models = builtin_models(&overrides);
    for id in [
        "deepseek-ai/deepseek-v4.1-flash",
        "nvidia/nemotron-3.5-super-vl-120b-a12b",
    ] {
        let info = models.get_model(PROVIDER, id).unwrap();
        let spec = format!("{PROVIDER}:{id}");
        let mut request = super::minimal_request(&spec);
        request.max_tokens = Some(8_192);
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
    assert_eq!(sent.len(), 2);
    for (request, id) in sent.iter().zip([
        "deepseek-ai/deepseek-v4.1-flash",
        "nvidia/nemotron-3.5-super-vl-120b-a12b",
    ]) {
        assert_eq!(request.url.path(), "/v1/chat/completions");
        assert_eq!(
            request.headers["authorization"],
            format!("Bearer {FIXTURE_KEY}")
        );
        assert_eq!(request.headers["nvcf-poll-seconds"], "3600");
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["model"], id);
        assert_eq!(body["max_tokens"], 8_192);
        assert!(body.get("reasoning_effort").is_none());
        assert!(body.get("max_completion_tokens").is_none());
        if id == "deepseek-ai/deepseek-v4.1-flash" {
            assert_eq!(body["thinking"]["type"], "enabled");
        } else {
            assert!(body.get("thinking").is_none());
        }
    }
}

#[tokio::test]
async fn builtin_nvidia_removes_retired_models_before_dispatch() {
    let server = MockServer::start().await;
    let mut overrides = BuiltinOverrides::default();
    overrides
        .base_urls
        .insert(PROVIDER.into(), format!("{}/v1", server.uri()));
    let models = builtin_models(&overrides);
    let listed = models.get_models_of_type(CatalogOperation::Chat, Some(PROVIDER));
    let available = models
        .get_available_of_type(CatalogOperation::Chat, Some(PROVIDER))
        .await
        .unwrap();
    assert_eq!(listed.len(), PUBLISHED_IDS.len());
    assert_eq!(available.len(), PUBLISHED_IDS.len());
    assert!(listed.iter().all(|record| record.id() != "z-ai/glm-5.3"));
    assert!(available.iter().all(|record| record.id() != "z-ai/glm-5.3"));
    assert!(models.get_model(PROVIDER, "z-ai/glm-5.3").is_none());
    assert!(matches!(
        models.prepare_call(
            &format!("{PROVIDER}:z-ai/glm-5.3"),
            super::minimal_request(&format!("{PROVIDER}:z-ai/glm-5.3"))
        )
        .await,
        Err(ModelsError::Registry(RegistryError::UnknownModel { provider, model }))
            if provider == PROVIDER && model == "z-ai/glm-5.3"
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}
