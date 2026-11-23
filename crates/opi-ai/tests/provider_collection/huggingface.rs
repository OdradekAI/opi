//! Hugging Face's fixed Pi 1.5 catalog and OpenAI Completions dispatch.

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

const PROVIDER: &str = "huggingface";
const FIXTURE_KEY: &str = "fixture-huggingface-token";
const PUBLISHED_COUNT: usize = 78;

fn builtin_models(overrides: &BuiltinOverrides) -> Models {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[("HF_TOKEN", FIXTURE_KEY)]),
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
async fn builtin_huggingface_lists_the_published_catalog() {
    let catalog = Catalog::builtin().unwrap();
    let models = builtin_models(&BuiltinOverrides::default());
    let raw_bundle: Value = serde_json::from_slice(Catalog::builtin_bundle_bytes().0).unwrap();
    let author_source: Value = serde_json::from_slice(include_bytes!(
        "../../resources/catalog/providers/huggingface.json"
    ))
    .unwrap();
    assert_eq!(raw_bundle[PROVIDER], author_source);

    let entries: Vec<_> = catalog
        .models(PROVIDER, CatalogOperation::Chat)
        .unwrap()
        .collect();
    assert_eq!(entries.len(), PUBLISHED_COUNT);
    assert!(
        entries
            .iter()
            .all(|entry| entry.api == "openai-completions")
    );

    let expected: Vec<_> = entries
        .iter()
        .map(|entry| {
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

    for id in [
        "MiniMaxAI/MiniMax-M3.1-Flash",
        "Qwen/Qwen3-8B",
        "deepseek-ai/DeepSeek-V4.1-Flash-1017",
    ] {
        assert!(models.get_model(PROVIDER, id).is_some(), "missing {id}");
    }

    assert_eq!(
        models
            .get_model(PROVIDER, "deepseek-ai/DeepSeek-R1-0528")
            .unwrap()
            .capabilities
            .max_output_tokens,
        32_768
    );
    assert_eq!(
        models
            .get_model(PROVIDER, "Qwen/Qwen3-235B-A22B-Thinking-2507")
            .unwrap()
            .capabilities
            .max_output_tokens,
        81_920
    );
    assert_eq!(
        models
            .get_model(PROVIDER, "Qwen/Qwen3-Coder-480B-A35B-Instruct")
            .unwrap()
            .capabilities
            .max_output_tokens,
        65_536
    );
    assert_eq!(
        models
            .get_model(PROVIDER, "Qwen/Qwen3-Next-80B-A3B-Instruct")
            .unwrap()
            .capabilities
            .max_output_tokens,
        32_768
    );
}

#[tokio::test]
async fn builtin_huggingface_sends_bearer_chat_with_updated_max_tokens() {
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
    let model_id = "deepseek-ai/DeepSeek-R1-0528";
    let spec = format!("{PROVIDER}:{model_id}");
    let mut request = super::minimal_request(&spec);
    request.max_tokens = Some(32_768);
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
    assert_eq!(body["max_completion_tokens"], 32_768);
    assert_eq!(body["reasoning_effort"], "high");
}

#[tokio::test]
async fn builtin_huggingface_retires_models_before_dispatch() {
    let server = MockServer::start().await;
    let models = builtin_models(&BuiltinOverrides::default());
    let catalog = Catalog::builtin().unwrap();
    let listed = models.get_models_of_type(CatalogOperation::Chat, Some(PROVIDER));
    let available = models
        .get_available_of_type(CatalogOperation::Chat, Some(PROVIDER))
        .await
        .unwrap();
    assert_eq!(listed.len(), PUBLISHED_COUNT);
    assert_eq!(available.len(), PUBLISHED_COUNT);

    for retired in [
        "Qwen/Qwen3-Next-80B-A3B-Thinking",
        "XiaomiMiMo/MiMo-V2-Flash",
        "zai-org/GLM-4.5",
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
