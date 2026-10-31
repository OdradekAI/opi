use std::str::FromStr;
use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::ProviderHeaders;
use opi_ai::anthropic;
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
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

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
            input_tokens_above: 200_000.into(),
            pricing: pricing(2.0),
        }],
    )
    .unwrap();

    assert_eq!(model_pricing.effective(200_000), pricing(1.0));
    assert_eq!(model_pricing.effective(200_001), pricing(2.0));
}

#[test]
fn pricing_validates_rates_without_narrowing_source_tier_order() {
    assert!(ModelPricing::try_new(pricing(f64::NAN), Vec::new()).is_err());
    assert!(ModelPricing::try_new(pricing(-1.0), Vec::new()).is_err());
    let tiers = ModelPricing::try_new(
        pricing(1.0),
        vec![
            PricingTier {
                input_tokens_above: 20.into(),
                pricing: pricing(2.0),
            },
            PricingTier {
                input_tokens_above: 10.into(),
                pricing: pricing(3.0),
            },
            PricingTier {
                input_tokens_above: 10.into(),
                pricing: pricing(4.0),
            },
            PricingTier {
                input_tokens_above: 0.into(),
                pricing: pricing(5.0),
            },
        ],
    )
    .expect("zero, duplicate and unordered thresholds are source-valid");
    assert_eq!(tiers.effective(1), pricing(5.0));
    assert_eq!(tiers.effective(11), pricing(3.0));
    assert_eq!(tiers.effective(21), pricing(2.0));
    assert!(
        ModelPricing::try_new(
            pricing(1.0),
            vec![PricingTier {
                input_tokens_above: 0.into(),
                pricing: pricing(-1.0),
            }]
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
        transport: Default::default(),
        model: format!("{provider_id}:{model_id}"),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: Vec::new(),
            tools: Vec::new(),
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
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
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

async fn run_production_request(
    provider: &dyn Provider,
    request: Request,
) -> Result<(), ProviderError> {
    validate_request_capabilities(provider, &request)?;
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    while let Some(event) = stream.next().await {
        event?;
    }
    Ok(())
}

#[tokio::test]
async fn chat_unsupported_thinking_level_is_rejected_before_http() {
    let server = MockServer::start().await;
    let provider = openai_chat::OpenAiChatProvider::for_route(
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
    assert!(
        matches!(error, ProviderError::UnsupportedCapability(_)),
        "got {error:?}"
    );
    assert!(
        server.received_requests().await.unwrap().is_empty(),
        "no HTTP request should be issued for an unsupported thinking level"
    );
}

#[tokio::test]
async fn responses_unsupported_thinking_level_is_rejected_before_http() {
    let server = MockServer::start().await;
    let provider = openai_responses::OpenAiResponsesProvider::for_route(
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
    assert!(
        matches!(error, ProviderError::UnsupportedCapability(_)),
        "got {error:?}"
    );
    assert!(
        server.received_requests().await.unwrap().is_empty(),
        "no HTTP request should be issued for an unsupported thinking level"
    );
}

#[tokio::test]
async fn strict_wire_unsupported_thinking_level_is_rejected_before_http() {
    let server = MockServer::start().await;
    let provider = anthropic::AnthropicProvider::for_route(
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
        Some("https://example.openai.azure.com".into()),
        vec!["deployment".into()],
        Some("2024-10-21".into()),
    )
    .unwrap();
    assert_eq!(azure.models()[0].wire_api, WireApi::AzureOpenAiCompletions);
}

#[tokio::test]
async fn nonreasoning_metadata_preserves_map_without_enabling_thinking() {
    let catalog = opi_ai::catalog::Catalog::builtin().unwrap();
    let entry = catalog
        .model(
            "openai",
            opi_ai::catalog::CatalogOperation::Chat,
            "gpt-5.3-chat-latest",
        )
        .unwrap();
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
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"gpt-5.3-chat-latest\",\"output\":[]}}\n\n",
        )).expect(1).mount(&server).await;
    model.base_url = Some(server.uri());
    let provider = openai_responses::OpenAiResponsesProvider::for_route(
        Some(server.uri()),
        "openai".into(),
        ProviderHeaders::default(),
        vec![model],
        Arc::new(HttpClient::new()),
    );
    let mut request = unsupported_thinking_request("openai", "gpt-5.3-chat-latest");
    request.thinking = Default::default();
    run_production_request(&provider, request.clone())
        .await
        .unwrap();
    request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::XHigh,
        budget_tokens: None,
    };
    assert!(matches!(
        run_production_request(&provider, request).await,
        Err(ProviderError::UnsupportedCapability(_))
    ));
    assert_eq!(provider.models()[0].thinking_level_map, original_map);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[test]
fn model_metadata_rejects_long_cache_without_cache_control() {
    let model = ModelInfo::new(
        "bad-cache",
        "Bad Cache",
        WireApi::AnthropicMessages,
        ModelCapabilities::new(1000, 100).with_long_cache_retention(true),
    );

    assert!(model.validate().is_err());
}

// ---------------------------------------------------------------------------
// Task 20.35: sampling metadata precedence and adapter wiring (AIR-062).
// ---------------------------------------------------------------------------

use opi_ai::model_info::resolve_sampling_params;
use opi_ai::stream::ServiceTier;
use serde_json::json;

fn sampling_json(value: serde_json::Value) -> opi_ai::model_info::SamplingParams {
    value.as_object().cloned().expect("object sampling params")
}

#[test]
fn sampling_metadata_defaults_empty_and_builders_attach() {
    let model = ModelInfo::new(
        "sampling-model",
        "Sampling Model",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(100_000, 10_000).with_streaming(true),
    );
    assert!(model.sampling_params.is_empty());
    assert!(model.sampling_params_by_thinking_level.is_empty());

    let model = model
        .with_sampling_params(sampling_json(json!({"top_k": 40})))
        .with_sampling_level_params(
            ThinkingLevel::High,
            sampling_json(json!({"temperature": 0.5})),
        );
    assert_eq!(model.sampling_params["top_k"], json!(40));
    assert_eq!(
        model.sampling_params_by_thinking_level[&ThinkingLevel::High]["temperature"],
        json!(0.5)
    );
    model.validate().unwrap();
}

#[test]
fn resolve_sampling_params_applies_model_level_request_precedence() {
    let model = ModelInfo::new(
        "sampling-model",
        "Sampling Model",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(100_000, 10_000).with_streaming(true),
    )
    .with_sampling_params(sampling_json(json!({"a": 1, "b": 1})))
    .with_sampling_level_params(ThinkingLevel::High, sampling_json(json!({"b": 2})));

    let resolved =
        resolve_sampling_params(&model, ThinkingLevel::High, &sampling_json(json!({"c": 3})))
            .expect("non-empty sources resolve");
    assert_eq!(resolved["a"], json!(1));
    assert_eq!(resolved["b"], json!(2));
    assert_eq!(resolved["c"], json!(3));

    let request_wins = resolve_sampling_params(
        &model,
        ThinkingLevel::High,
        &sampling_json(json!({"a": 9, "b": 9})),
    )
    .expect("non-empty sources resolve");
    assert_eq!(request_wins["a"], json!(9));
    assert_eq!(request_wins["b"], json!(9));

    let empty = ModelInfo::new(
        "empty",
        "Empty",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(100_000, 10_000).with_streaming(true),
    );
    assert!(resolve_sampling_params(&empty, ThinkingLevel::High, &Default::default()).is_none());
}

fn sampling_model(wire: WireApi) -> ModelInfo {
    ModelInfo::new(
        "model",
        "Model",
        wire,
        ModelCapabilities::new(100_000, 10_000)
            .with_streaming(true)
            .with_thinking(true),
    )
    .with_sampling_params(sampling_json(json!({"temperature": 0.9, "top_k": 40})))
    .with_sampling_level_params(
        ThinkingLevel::High,
        sampling_json(json!({"temperature": 0.5})),
    )
}

fn sampling_request(provider_id: &str) -> Request {
    let mut request = unsupported_thinking_request(provider_id, "model");
    request.thinking.level = ThinkingLevel::High;
    request.temperature = Some(0.7);
    request.sampling_params = sampling_json(json!({"top_p": 0.9}));
    request
}

async fn capture_request_body(
    server: &MockServer,
    provider: &dyn Provider,
    request: Request,
) -> serde_json::Value {
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    while let Some(event) = stream.next().await {
        if let Err(error) = event {
            panic!("unexpected provider error: {error}");
        }
    }
    let captured = server.received_requests().await.unwrap().remove(0);
    serde_json::from_slice(&captured.body).expect("captured body should be JSON")
}

#[tokio::test]
async fn chat_merges_sampling_metadata_over_named_fields() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{\"content\":\"hi\"},\"index\":0,\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
        ))
        .expect(1)
        .mount(&server)
        .await;
    let provider = openai_chat::OpenAiChatProvider::for_route(
        Some(server.uri()),
        "chat".into(),
        ProviderHeaders::default(),
        vec![sampling_model(WireApi::OpenAiCompletions)],
        Arc::new(HttpClient::new()),
    );

    let body = capture_request_body(&server, &provider, sampling_request("chat")).await;

    assert_eq!(
        body["temperature"],
        json!(0.5),
        "level overrides named field"
    );
    assert_eq!(body["top_k"], json!(40), "model base key survives");
    assert_eq!(body["top_p"], json!(0.9), "request key merged last");
}

#[tokio::test]
async fn responses_merges_sampling_after_named_fields() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"status\":\"completed\",\"model\":\"model\",\"output\":[],\"usage\":{\"input_tokens\":10,\"output_tokens\":5}}}\n\n",
        ))
        .expect(1)
        .mount(&server)
        .await;
    let provider = openai_responses::OpenAiResponsesProvider::for_route(
        Some(server.uri()),
        "custom-responses".into(),
        ProviderHeaders::default(),
        vec![sampling_model(WireApi::OpenAiResponses)],
        Arc::new(HttpClient::new()),
    );

    let body = capture_request_body(&server, &provider, sampling_request("custom-responses")).await;

    assert_eq!(
        body["temperature"],
        json!(0.5),
        "level overrides named field"
    );
    assert_eq!(body["top_k"], json!(40), "model base key survives");
    assert_eq!(body["top_p"], json!(0.9), "request key merged last");
}

#[test]
fn service_tier_usage_field_round_trips() {
    let mut usage = opi_ai::stream::Usage::reported(10, 5, 0, 0, None, None);
    usage.service_tier = Some(ServiceTier::Flex);
    let json = serde_json::to_string(&usage).unwrap();
    assert!(json.contains("flex"));
    let round_trip: opi_ai::stream::Usage = serde_json::from_str(&json).unwrap();
    assert_eq!(round_trip.service_tier, Some(ServiceTier::Flex));
}
