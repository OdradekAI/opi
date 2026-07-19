use std::str::FromStr;
use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::ProviderHeaders;
use opi_ai::anthropic;
use opi_ai::auth::{AuthScheme, StaticAuthResolver};
use opi_ai::azure_openai::AzureOpenAIProvider;
use opi_ai::bedrock;
use opi_ai::gemini;
use opi_ai::http::HttpClient;
use opi_ai::mistral;
use opi_ai::model_info::{
    AnthropicMessagesCompat, ModelInfo, ModelPricing, PricingTier, ThinkingLevel, ThinkingLevelMap,
    ThinkingLevelMapping, WireApi, WireCompat,
};
use opi_ai::openai_chat;
use opi_ai::openai_responses;
use opi_ai::openrouter;
use opi_ai::provider::{
    Provider, ProviderError, Request, ThinkingConfig, validate_request_capabilities,
};
use opi_ai::registry::ModelCapabilities;
use opi_ai::stream::Pricing;
use opi_ai::vertex;
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn pricing(input: f64) -> Pricing {
    Pricing {
        input_cost_per_mtok: input,
        output_cost_per_mtok: input * 2.0,
        cache_read_cost_per_mtok: input / 10.0,
        cache_write_cost_per_mtok: input * 1.25,
    }
}

#[test]
fn wire_api_serializes_exact_reviewed_names() {
    let cases = [
        (WireApi::AnthropicMessages, "anthropic-messages"),
        (WireApi::OpenAiCompletions, "openai-completions"),
        (WireApi::OpenAiResponses, "openai-responses"),
        (WireApi::OpenAiCodexResponses, "openai-codex-responses"),
        (WireApi::GoogleGenerativeAi, "google-generative-ai"),
        (WireApi::GoogleVertex, "google-vertex"),
        (WireApi::BedrockConverseStream, "bedrock-converse-stream"),
        (WireApi::AzureOpenAiCompletions, "azure-openai-completions"),
    ];

    for (wire, name) in cases {
        assert_eq!(wire.to_string(), name);
        assert_eq!(serde_json::to_value(wire).unwrap(), name);
        assert_eq!(WireApi::from_str(name).unwrap(), wire);
    }
}

#[test]
fn model_info_public_constructor_sets_required_wire() {
    let model = ModelInfo::new(
        "external-model",
        "External Model",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(128_000, 16_384).with_streaming(true),
    );

    assert_eq!(model.id, "external-model");
    assert_eq!(model.wire_api, WireApi::OpenAiCompletions);
    assert_eq!(model.compat.wire_api(), WireApi::OpenAiCompletions);
    model.validate().unwrap();
}

#[test]
fn model_info_rejects_wire_compat_mismatch() {
    let error = ModelInfo::new(
        "mismatch",
        "Mismatch",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(8_192, 1_024),
    )
    .with_compat(WireCompat::AnthropicMessages(
        AnthropicMessagesCompat::default(),
    ))
    .unwrap_err();

    assert!(error.to_string().contains("wire"));
    assert!(error.to_string().contains("mismatch"));
}

#[test]
fn pricing_tier_uses_strict_greater_than_boundary() {
    let model_pricing = ModelPricing::try_new(
        pricing(1.0),
        vec![PricingTier {
            input_tokens_above: 200_000,
            pricing: pricing(2.0),
        }],
    )
    .unwrap();

    assert_eq!(model_pricing.effective(200_000), pricing(1.0));
    assert_eq!(model_pricing.effective(200_001), pricing(2.0));
}

#[test]
fn pricing_rejects_non_finite_negative_duplicate_and_unsorted_tiers() {
    assert!(ModelPricing::try_new(pricing(f64::NAN), Vec::new()).is_err());
    assert!(ModelPricing::try_new(pricing(-1.0), Vec::new()).is_err());
    assert!(
        ModelPricing::try_new(
            pricing(1.0),
            vec![
                PricingTier {
                    input_tokens_above: 10,
                    pricing: pricing(2.0),
                },
                PricingTier {
                    input_tokens_above: 10,
                    pricing: pricing(3.0),
                },
            ],
        )
        .is_err()
    );
    assert!(
        ModelPricing::try_new(
            pricing(1.0),
            vec![
                PricingTier {
                    input_tokens_above: 20,
                    pricing: pricing(2.0),
                },
                PricingTier {
                    input_tokens_above: 10,
                    pricing: pricing(3.0),
                },
            ],
        )
        .is_err()
    );
    assert!(
        ModelPricing::try_new(
            pricing(1.0),
            vec![PricingTier {
                input_tokens_above: 0,
                pricing: pricing(2.0),
            }],
        )
        .is_err()
    );
}

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
    Request {
        model: format!("{provider_id}:{model_id}"),
        system: None,
        messages: Vec::new(),
        tools: Vec::new(),
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig {
            enabled: true,
            budget_tokens: None,
            level: ThinkingLevel::Max,
        },
        stop_sequences: Vec::new(),
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: Vec::new(),
        cache_retention: Default::default(),
        session_id: None,
    }
}

fn test_auth() -> Arc<dyn opi_ai::AuthResolver> {
    Arc::new(StaticAuthResolver::new(
        AuthScheme::Bearer,
        SecretString::from("test-token"),
    ))
}

async fn run_production_request(
    provider: &dyn Provider,
    request: Request,
) -> Result<(), ProviderError> {
    validate_request_capabilities(provider, &request)?;
    let mut stream = provider.stream(request);
    while let Some(event) = stream.next().await {
        event?;
    }
    Ok(())
}

#[tokio::test]
async fn chat_unsupported_thinking_level_reaches_http_without_reasoning() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(
                    "data: {\"id\":\"chatcmpl-1\",\"model\":\"model\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}\n\n\
                     data: {\"id\":\"chatcmpl-1\",\"model\":\"model\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n\
                     data: [DONE]\n\n",
                ),
        )
        .expect(1)
        .mount(&server)
        .await;
    let provider = openai_chat::OpenAiChatProvider::for_route(
        test_auth(),
        Some(server.uri()),
        "chat".into(),
        ProviderHeaders::default(),
        vec![unsupported_thinking_model(
            "model",
            WireApi::OpenAiCompletions,
        )],
        Arc::new(HttpClient::new()),
    );

    run_production_request(&provider, unsupported_thinking_request("chat", "model"))
        .await
        .expect("unsupported Chat reasoning is omitted");
    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert!(body.get("reasoning_effort").is_none());
}

#[tokio::test]
async fn responses_unsupported_thinking_level_reaches_http_without_reasoning() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(
                    "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{\"id\":\"r\",\"model\":\"model\"}}\n\n\
                     event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"r\",\"model\":\"model\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n",
                ),
        )
        .expect(1)
        .mount(&server)
        .await;
    let provider = openai_responses::OpenAiResponsesProvider::for_route(
        test_auth(),
        Some(server.uri()),
        "responses".into(),
        ProviderHeaders::default(),
        vec![unsupported_thinking_model(
            "model",
            WireApi::OpenAiResponses,
        )],
        Arc::new(HttpClient::new()),
    );

    run_production_request(
        &provider,
        unsupported_thinking_request("responses", "model"),
    )
    .await
    .expect("unsupported Responses reasoning is omitted");
    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert!(body.get("reasoning").is_none());
}

#[tokio::test]
async fn strict_wire_unsupported_thinking_level_is_rejected_before_http() {
    let server = MockServer::start().await;
    let provider = anthropic::AnthropicProvider::for_route(
        test_auth(),
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
    assert!(matches!(error, ProviderError::UnsupportedCapability(_)));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[test]
fn every_builtin_model_declares_exact_wire() {
    let catalogs = [
        (anthropic::model_catalog(), WireApi::AnthropicMessages),
        (openai_chat::model_catalog(), WireApi::OpenAiCompletions),
        (openai_responses::model_catalog(), WireApi::OpenAiResponses),
        (openrouter::model_catalog(), WireApi::OpenAiCompletions),
        (mistral::model_catalog(), WireApi::OpenAiCompletions),
        (gemini::model_catalog(), WireApi::GoogleGenerativeAi),
        (vertex::model_catalog(), WireApi::GoogleVertex),
        (bedrock::model_catalog(), WireApi::BedrockConverseStream),
    ];

    for (catalog, wire) in catalogs {
        assert!(!catalog.is_empty());
        for model in catalog {
            assert_eq!(model.wire_api, wire, "{}", model.id);
            model.validate().unwrap();
        }
    }

    let azure = AzureOpenAIProvider::from_config(
        "key".into(),
        Some("https://example.openai.azure.com".into()),
        vec!["deployment".into()],
        Some("2024-10-21".into()),
    )
    .unwrap();
    assert_eq!(azure.models()[0].wire_api, WireApi::AzureOpenAiCompletions);
}
