//! Thinking-level admission through the provider entry point before HTTP dispatch.

use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::api::shared::headers::ProviderHeaders;
use opi_ai::http::HttpClient;
use opi_ai::provider::{Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::providers::anthropic as anthropic_provider;
use opi_ai::providers::openai as openai_profile;
use opi_ai::providers::openai_responses as openai_responses_profile;
use opi_ai::registry::ModelCapabilities;
use opi_ai::types::model::{
    ModelInfo, ThinkingLevel, ThinkingLevelMap, ThinkingLevelMapping, WireApi,
};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer};

use super::fixtures::model_request;

fn unsupported_thinking_model(id: &str, wire_api: WireApi) -> ModelInfo {
    ModelInfo::new(
        id,
        id,
        wire_api,
        ModelCapabilities::new(100_000, 10_000)
            .with_streaming(true)
            .with_thinking(true),
    )
    .with_thinking_level_map(
        ThinkingLevelMap::reasoning_default()
            .with_mapping(ThinkingLevel::Max, ThinkingLevelMapping::Unsupported),
    )
}

fn unsupported_thinking_request(provider_id: &str, model_id: &str) -> Request {
    let mut request = model_request(provider_id, model_id);
    request.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: None,
        level: ThinkingLevel::Max,
    };
    request
}

async fn run_production_request(
    provider: &dyn Provider,
    request: Request,
) -> Result<(), ProviderError> {
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    while let Some(event) = stream.next().await {
        event?;
    }
    Ok(())
}

fn assert_unsupported_thinking(error: ProviderError) {
    assert!(
        matches!(&error, ProviderError::StreamFailure(failure)
            if matches!(failure.cause(), ProviderError::UnsupportedCapability(_))),
        "expected an unsupported capability from the provider stream, got {error:?}"
    );
}

#[tokio::test]
async fn chat_unsupported_thinking_level_is_rejected_before_http() {
    let server = MockServer::start().await;
    let provider = openai_profile::OpenAiChatProvider::for_route(
        Some(server.uri()),
        "chat".into(),
        ProviderHeaders::default(),
        vec![unsupported_thinking_model(
            "model",
            WireApi::OpenAiCompletions,
        )],
        Arc::new(HttpClient::new()),
    );

    let error = run_production_request(&provider, unsupported_thinking_request("chat", "model"))
        .await
        .expect_err("unsupported Chat thinking is rejected before HTTP");
    assert_unsupported_thinking(error);
    assert!(
        server.received_requests().await.unwrap().is_empty(),
        "no HTTP request should be issued for an unsupported thinking level"
    );
}

#[tokio::test]
async fn responses_unsupported_thinking_level_is_rejected_before_http() {
    let server = MockServer::start().await;
    let provider = openai_responses_profile::OpenAiResponsesProvider::for_route(
        Some(server.uri()),
        "responses".into(),
        ProviderHeaders::default(),
        vec![unsupported_thinking_model(
            "model",
            WireApi::OpenAiResponses,
        )],
        Arc::new(HttpClient::new()),
    );

    let error = run_production_request(
        &provider,
        unsupported_thinking_request("responses", "model"),
    )
    .await
    .expect_err("unsupported Responses thinking is rejected before HTTP");
    assert_unsupported_thinking(error);
    assert!(
        server.received_requests().await.unwrap().is_empty(),
        "no HTTP request should be issued for an unsupported thinking level"
    );
}

#[tokio::test]
async fn strict_wire_unsupported_thinking_level_is_rejected_before_http() {
    let server = MockServer::start().await;
    let provider = anthropic_provider::AnthropicProvider::for_route(
        "strict".into(),
        vec![unsupported_thinking_model(
            "model",
            WireApi::AnthropicMessages,
        )],
        Some(server.uri()),
        ProviderHeaders::default(),
        Arc::new(HttpClient::new()),
        false,
    );

    let error = run_production_request(&provider, unsupported_thinking_request("strict", "model"))
        .await
        .expect_err("strict wire rejection");
    assert_unsupported_thinking(error);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn nonreasoning_metadata_preserves_map_without_enabling_thinking() {
    let catalog = opi_ai::catalog::Catalog::builtin().unwrap();
    let mut entry = catalog
        .model("openai", opi_ai::catalog::CatalogOperation::Chat, "gpt-4o")
        .unwrap()
        .clone();
    entry.raw["thinkingLevelMap"] = serde_json::json!({"xhigh": "xhigh"});
    let mut model = entry.to_chat_model_info().unwrap();
    assert!(!model.capabilities.supports_thinking);
    assert_eq!(
        model
            .thinking_level_map
            .resolve(ThinkingLevel::XHigh)
            .unwrap(),
        Some("xhigh".into())
    );
    model.validate().unwrap();
    let original_map = model.thinking_level_map.clone();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"gpt-4o\",\"output\":[]}}\n\n",
        )).expect(1).mount(&server).await;
    model.base_url = Some(server.uri());
    let provider = openai_responses_profile::OpenAiResponsesProvider::for_route(
        Some(server.uri()),
        "openai".into(),
        ProviderHeaders::default(),
        vec![model],
        Arc::new(HttpClient::new()),
    );
    let mut request = model_request("openai", "gpt-4o");
    run_production_request(&provider, request.clone())
        .await
        .unwrap();
    request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::XHigh,
        budget_tokens: None,
    };
    let error = run_production_request(&provider, request)
        .await
        .expect_err("stored thinking metadata does not enable thinking");
    assert_unsupported_thinking(error);
    assert_eq!(provider.models()[0].thinking_level_map, original_map);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}
