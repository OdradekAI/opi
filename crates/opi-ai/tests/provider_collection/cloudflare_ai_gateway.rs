//! Cloudflare AI Gateway's fixed Pi 1.5 catalog and three wire routes.

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

const PROVIDER: &str = "cloudflare-ai-gateway";
const FIXTURE_KEY: &str = "fixture-cloudflare-gateway-key";
const PUBLISHED_IDS: [&str; 59] = [
    "claude-fable-5",
    "claude-fable-5-1",
    "claude-haiku-4-5",
    "claude-haiku-5-5",
    "claude-opus-4-5",
    "claude-opus-4-6",
    "claude-opus-4-7",
    "claude-opus-4-8",
    "claude-opus-5",
    "claude-opus-5-5",
    "claude-sonnet-4-5",
    "claude-sonnet-4-6",
    "claude-sonnet-5",
    "claude-sonnet-5-5",
    "gpt-4.1",
    "gpt-4.1-mini",
    "gpt-4.1-nano",
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-5",
    "gpt-5-mini",
    "gpt-5-nano",
    "gpt-5.1",
    "gpt-5.4",
    "gpt-5.4-mini",
    "gpt-5.4-nano",
    "gpt-5.4-pro",
    "gpt-5.5",
    "gpt-5.5-pro",
    "gpt-5.6-luna",
    "gpt-5.6-sol",
    "gpt-5.6-terra",
    "gpt-6-astra",
    "gpt-6-luna",
    "gpt-6-sol",
    "gpt-6.1-sol",
    "gpt-6.1-terra",
    "o3",
    "o3-mini",
    "o4-mini",
    "workers-ai/@cf/deepseek-ai/deepseek-v4-flash-0731",
    "workers-ai/@cf/deepseek-ai/deepseek-v4-pro-0813",
    "workers-ai/@cf/deepseek-ai/deepseek-v4.1-flash-1017",
    "workers-ai/@cf/google/gemma-4-26b-a4b-it",
    "workers-ai/@cf/ibm-granite/granite-4.0-h-micro",
    "workers-ai/@cf/meta/llama-3.3-70b-instruct-fp8-fast",
    "workers-ai/@cf/meta/llama-4-scout-17b-16e-instruct",
    "workers-ai/@cf/mistralai/mistral-small-3.1-24b-instruct",
    "workers-ai/@cf/moonshotai/kimi-k2.6",
    "workers-ai/@cf/moonshotai/kimi-k2.7-code",
    "workers-ai/@cf/nvidia/nemotron-3-120b-a12b",
    "workers-ai/@cf/openai/gpt-oss-120b",
    "workers-ai/@cf/openai/gpt-oss-20b",
    "workers-ai/@cf/qwen/qwen3-30b-a3b-fp8",
    "workers-ai/@cf/qwen/qwen3.8-27b",
    "workers-ai/@cf/zai-org/glm-4.7-flash",
    "workers-ai/@cf/zai-org/glm-5.2",
    "workers-ai/@cf/zai-org/glm-5.3",
    "workers-ai/@cf/zai-org/glm-5.3-flash",
];

fn builtin_models() -> Models {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[
            ("CLOUDFLARE_API_KEY", FIXTURE_KEY),
            ("CLOUDFLARE_ACCOUNT_ID", "fixture-account"),
            ("CLOUDFLARE_GATEWAY_ID", "fixture-gateway"),
        ]),
        Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    models
}

#[tokio::test]
async fn builtin_cloudflare_ai_gateway_lists_the_published_records() {
    let catalog = Catalog::builtin().unwrap();
    let models = builtin_models();
    let raw_bundle: Value = serde_json::from_slice(Catalog::builtin_bundle_bytes().0).unwrap();
    let author_source: Value = serde_json::from_slice(include_bytes!(
        "../../resources/catalog/providers/cloudflare-ai-gateway.json"
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
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry.api == "anthropic-messages")
            .count(),
        14
    );
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry.api == "openai-responses")
            .count(),
        26
    );
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry.api == "openai-completions")
            .count(),
        19
    );
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.id == "claude-sonnet-5-5")
            .unwrap()
            .raw["cost"]["cacheRead"],
        0.1
    );
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.id == "gpt-6.1-terra")
            .unwrap()
            .raw["thinkingLevelMap"]["xhigh"],
        "xhigh"
    );
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.id == "workers-ai/@cf/deepseek-ai/deepseek-v4.1-flash-1017")
            .unwrap()
            .raw["compat"]["maxTokensField"],
        "max_tokens"
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
async fn builtin_cloudflare_ai_gateway_sends_all_three_wire_requests() {
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
    Mock::given(method("POST"))
        .and(path("/compat/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(super::node5_reply(WireApi::OpenAiCompletions)),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/openai/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(super::node5_reply(WireApi::OpenAiResponses)),
        )
        .expect(1)
        .mount(&server)
        .await;

    let models = builtin_models();

    let claude_id = "claude-sonnet-5-5";
    let mut claude = models.get_model(PROVIDER, claude_id).unwrap();
    claude.base_url = Some(format!("{}/anthropic", server.uri()));
    let claude_spec = format!("{PROVIDER}:{claude_id}");
    let mut claude_request = super::minimal_request(&claude_spec);
    claude_request.max_tokens = Some(12_345);
    claude_request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::XHigh,
        budget_tokens: None,
    };
    models
        .complete_simple(
            ChatModelRecord::new(PROVIDER, claude),
            claude_request,
            Default::default(),
        )
        .await
        .unwrap();

    let deepseek_id = "workers-ai/@cf/deepseek-ai/deepseek-v4.1-flash-1017";
    let mut deepseek = models.get_model(PROVIDER, deepseek_id).unwrap();
    deepseek.base_url = Some(format!("{}/compat", server.uri()));
    let deepseek_spec = format!("{PROVIDER}:{deepseek_id}");
    let mut deepseek_request = super::minimal_request(&deepseek_spec);
    deepseek_request.max_tokens = Some(54_321);
    deepseek_request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::High,
        budget_tokens: None,
    };
    models
        .complete_simple(
            ChatModelRecord::new(PROVIDER, deepseek),
            deepseek_request,
            Default::default(),
        )
        .await
        .unwrap();

    let terra_id = "gpt-6.1-terra";
    let mut terra = models.get_model(PROVIDER, terra_id).unwrap();
    terra.base_url = Some(format!("{}/openai", server.uri()));
    let terra_spec = format!("{PROVIDER}:{terra_id}");
    let mut terra_request = super::minimal_request(&terra_spec);
    terra_request.max_tokens = Some(1_024);
    terra_request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::XHigh,
        budget_tokens: None,
    };
    models
        .complete_simple(
            ChatModelRecord::new(PROVIDER, terra),
            terra_request,
            Default::default(),
        )
        .await
        .unwrap();

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 3);
    for request in &sent {
        assert_eq!(
            request.headers["cf-aig-authorization"],
            format!("Bearer {FIXTURE_KEY}")
        );
        assert!(!request.headers.contains_key("authorization"));
        assert!(!request.headers.contains_key("x-api-key"));
    }
    let anthropic = sent
        .iter()
        .find(|request| request.url.path() == "/anthropic/v1/messages")
        .unwrap();
    let anthropic_body: Value = serde_json::from_slice(&anthropic.body).unwrap();
    assert_eq!(anthropic_body["model"], claude_id);
    assert_eq!(anthropic_body["thinking"]["type"], "adaptive");
    assert_eq!(anthropic_body["output_config"]["effort"], "xhigh");

    let deepseek = sent
        .iter()
        .find(|request| request.url.path() == "/compat/chat/completions")
        .unwrap();
    let deepseek_body: Value = serde_json::from_slice(&deepseek.body).unwrap();
    assert_eq!(deepseek_body["model"], deepseek_id);
    assert_eq!(deepseek_body["max_tokens"], 54_321);
    assert!(deepseek_body.get("max_completion_tokens").is_none());
    assert!(deepseek_body.get("reasoning_effort").is_none());

    let terra = sent
        .iter()
        .find(|request| request.url.path() == "/openai/responses")
        .unwrap();
    let terra_body: Value = serde_json::from_slice(&terra.body).unwrap();
    assert_eq!(terra_body["model"], terra_id);
    assert_eq!(terra_body["reasoning"]["effort"], "xhigh");
}

#[tokio::test]
async fn builtin_cloudflare_ai_gateway_rejects_unknown_models_before_dispatch() {
    let server = MockServer::start().await;
    let models = builtin_models();
    let listed = models.get_models_of_type(CatalogOperation::Chat, Some(PROVIDER));
    let available = models
        .get_available_of_type(CatalogOperation::Chat, Some(PROVIDER))
        .await
        .unwrap();
    assert_eq!(listed.len(), PUBLISHED_IDS.len());
    assert_eq!(available.len(), PUBLISHED_IDS.len());
    assert!(
        models
            .get_model(PROVIDER, "gpt-6.1-terra-retired")
            .is_none()
    );
    let spec = format!("{PROVIDER}:gpt-6.1-terra-retired");
    assert!(matches!(
        models.prepare_call(&spec, super::minimal_request(&spec)).await,
        Err(ModelsError::Registry(RegistryError::UnknownModel { provider, model }))
            if provider == PROVIDER && model == "gpt-6.1-terra-retired"
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}
