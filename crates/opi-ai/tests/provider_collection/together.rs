//! Together's fixed Pi 1.5 catalog and OpenAI Completions dispatch.

use super::{
    Arc, BuiltinOverrides, Catalog, CatalogOperation, Models, WireApi, map_context,
    register_builtin,
};
use opi_ai::catalog::CatalogRecord;
use opi_ai::models::{ChatModelRecord, ModelsError};
use opi_ai::provider::ThinkingConfig;
use opi_ai::registry::RegistryError;
use opi_ai::{ThinkingLevel, WireCompat};
use serde_json::Value;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};

const PROVIDER: &str = "together";
const FIXTURE_KEY: &str = "fixture-together-key";
const PUBLISHED_IDS: [&str; 15] = [
    "MiniMaxAI/MiniMax-M3",
    "Qwen/Qwen3.5-9B",
    "Qwen/Qwen3.6-Plus",
    "Qwen/Qwen3.7-Max",
    "deepseek-ai/DeepSeek-V4-Pro-0813",
    "deepseek-ai/DeepSeek-V4.1-Flash",
    "meta-models/Muse-Spark-Open-816B-A42B",
    "moonshotai/Kimi-K3",
    "nvidia/NVIDIA-Nemotron-3.5-Super-VL",
    "openai/gpt-oss-120b",
    "reflection/Beam-1.0-501B-A23B",
    "thinkingmachines/Inkling",
    "zai-org/GLM-5.2",
    "zai-org/GLM-5.3",
    "zai-org/GLM-5.3-Flash",
];

fn builtin_models(overrides: &BuiltinOverrides) -> Models {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[("TOGETHER_API_KEY", FIXTURE_KEY)]),
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
async fn builtin_together_lists_the_published_catalog() {
    let catalog = Catalog::builtin().unwrap();
    let models = builtin_models(&BuiltinOverrides::default());
    let raw_bundle: Value = serde_json::from_slice(Catalog::builtin_bundle_bytes().0).unwrap();
    let author_source: Value = serde_json::from_slice(include_bytes!(
        "../../resources/catalog/providers/together.json"
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

    let minimax = models.get_model(PROVIDER, "MiniMaxAI/MiniMax-M3").unwrap();
    assert_eq!(minimax.capabilities.max_output_tokens, 524_288);
    let qwen = models.get_model(PROVIDER, "Qwen/Qwen3.7-Max").unwrap();
    assert!(qwen.capabilities.supports_thinking);
    assert_eq!(qwen.capabilities.max_output_tokens, 131_072);
    assert!(matches!(qwen.compat, WireCompat::OpenAiCompletions(_)));
    let glm = models.get_model(PROVIDER, "zai-org/GLM-5.3").unwrap();
    assert_eq!(glm.capabilities.context_window, 1_048_575);
}

#[tokio::test]
async fn builtin_together_sends_bearer_chat_with_qwen_reasoning() {
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
    let model_id = "Qwen/Qwen3.7-Max";
    let mut request = super::minimal_request(&format!("{PROVIDER}:{model_id}"));
    request.max_tokens = Some(131_072);
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
    assert_eq!(
        request.headers["authorization"],
        format!("Bearer {FIXTURE_KEY}")
    );
    let body: Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["model"], model_id);
    assert_eq!(body["max_tokens"], 131_072);
    assert_eq!(body["reasoning"]["enabled"], true);
    assert!(body.get("reasoning_effort").is_none());
}

#[tokio::test]
async fn builtin_together_retires_models_before_dispatch() {
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

    for retired in [
        "MiniMaxAI/MiniMax-M2.7",
        "Qwen/Qwen2.5-7B-Instruct-Turbo",
        "deepseek-ai/DeepSeek-V4-Flash-0731",
        "meta-llama/Llama-3.3-70B-Instruct-Turbo",
        "nvidia/nemotron-3-ultra-550b-a55b",
    ] {
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
