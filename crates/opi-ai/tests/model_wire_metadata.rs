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
use opi_ai::openai_chat;
use opi_ai::openai_responses;
use opi_ai::openrouter;
use opi_ai::provider::{
    Provider, ProviderError, Request, ThinkingConfig, validate_request_capabilities,
};
use opi_ai::registry::ModelCapabilities;
use opi_ai::stream::Pricing;
use opi_ai::types::model::{
    AnthropicMessagesCompat, ModelInfo, ModelPricing, PricingTier, ThinkingLevel, ThinkingLevelMap,
    ThinkingLevelMapping, WireApi, WireCompat,
};
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
fn routing_types_retain_explicit_empty_and_nested_partition_null() {
    use opi_ai::types::model::{OpenRouterRouting, VercelGatewayRouting};
    for routing in [
        serde_json::json!({}),
        serde_json::json!({"sort":"price"}),
        serde_json::json!({"sort":{}}),
        serde_json::json!({"sort":{"by":"price"}}),
        serde_json::json!({"sort":{"partition":null}}),
        serde_json::json!({"sort":{"partition":"model"}}),
        serde_json::json!({"data_collection":"allow","preferred_min_throughput":0,
            "preferred_max_latency":{"p50":0,"p75":1.25,"p90":2,"p99":3}}),
    ] {
        let typed: OpenRouterRouting = serde_json::from_value(routing.clone()).unwrap();
        assert_eq!(serde_json::to_value(typed).unwrap(), routing);
    }
    for routing in [
        serde_json::json!({}),
        serde_json::json!({"only":[]}),
        serde_json::json!({"order":[]}),
        serde_json::json!({"only":["chosen"],"order":[]}),
    ] {
        let typed: VercelGatewayRouting = serde_json::from_value(routing.clone()).unwrap();
        assert_eq!(serde_json::to_value(typed).unwrap(), routing);
    }
    for invalid in [
        serde_json::json!([]),
        serde_json::json!({"sort":[]}),
        serde_json::json!({"sort":{"partition":false}}),
        serde_json::json!({"max_price":[]}),
        serde_json::json!({"max_price":{"prompt":false}}),
        serde_json::json!({"preferred_min_throughput":[]}),
        serde_json::json!({"preferred_min_throughput":{"p99":"1"}}),
        serde_json::json!({"preferred_max_latency":[]}),
        serde_json::json!({"quantizations":[0]}),
    ] {
        assert!(
            serde_json::from_value::<OpenRouterRouting>(invalid.clone()).is_err(),
            "accepted invalid routing: {invalid}"
        );
    }
    assert!(serde_json::from_value::<VercelGatewayRouting>(serde_json::json!([])).is_err());
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
        projection: Default::default(),
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

use opi_ai::stream::ServiceTier;
use opi_ai::types::model::resolve_sampling_params;
use serde_json::json;

fn sampling_json(value: serde_json::Value) -> opi_ai::types::model::SamplingParams {
    value.as_object().cloned().expect("object sampling params")
}

#[test]
fn sampling_metadata_defaults_absent_and_builders_preserve_presence() {
    let model = ModelInfo::new(
        "sampling-model",
        "Sampling Model",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(100_000, 10_000).with_streaming(true),
    );
    assert!(model.sampling_params.is_none());
    assert!(model.sampling_params_by_thinking_level.is_empty());

    let explicit_empty = model.clone().with_sampling_params(Default::default());
    assert_eq!(explicit_empty.sampling_params, Some(Default::default()));

    let model = model
        .with_sampling_params(sampling_json(json!({"top_k": 40})))
        .with_sampling_level_params(
            ThinkingLevel::High,
            sampling_json(json!({"temperature": 0.5})),
        );
    assert_eq!(model.sampling_params.as_ref().unwrap()["top_k"], json!(40));
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

    let resolved = resolve_sampling_params(
        &model,
        ThinkingLevel::High,
        Some(&sampling_json(json!({"c": 3}))),
    )
    .expect("non-empty sources resolve");
    assert_eq!(resolved["a"], json!(1));
    assert_eq!(resolved["b"], json!(2));
    assert_eq!(resolved["c"], json!(3));

    let request_wins = resolve_sampling_params(
        &model,
        ThinkingLevel::High,
        Some(&sampling_json(json!({"a": 9, "b": 9}))),
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
    assert!(resolve_sampling_params(&empty, ThinkingLevel::High, None).is_none());
}

#[test]
fn sampling_presence_shallow_merge_matrix() {
    let empty = sampling_json(json!({}));
    for model_present in [false, true] {
        for level_present in [false, true] {
            for request_present in [false, true] {
                let mut model = ModelInfo::new(
                    "presence",
                    "Presence",
                    WireApi::OpenAiCompletions,
                    ModelCapabilities::new(8192, 1024),
                );
                if model_present {
                    model = model.with_sampling_params(empty.clone());
                }
                if level_present {
                    model = model.with_sampling_level_params(ThinkingLevel::High, empty.clone());
                }
                let result = resolve_sampling_params(
                    &model,
                    ThinkingLevel::High,
                    request_present.then_some(&empty),
                );
                assert_eq!(
                    result,
                    (model_present || level_present || request_present).then(|| empty.clone()),
                    "model={model_present}, level={level_present}, request={request_present}"
                );
                assert_eq!(
                    resolve_sampling_params(&model, ThinkingLevel::Low, None).is_some(),
                    model_present
                );
            }
        }
    }
    let model = ModelInfo::new(
        "precedence",
        "Precedence",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(8192, 1024),
    )
    .with_sampling_params(sampling_json(json!({"a":1,"b":1,"nested":{"base":1}})))
    .with_sampling_level_params(
        ThinkingLevel::High,
        sampling_json(json!({"b":2,"nested":{"x":2}})),
    );
    let request = sampling_json(json!({"c":3,"nested":{"y":3}}));
    assert_eq!(
        resolve_sampling_params(&model, ThinkingLevel::High, Some(&request)),
        Some(sampling_json(json!({"a":1,"b":2,"c":3,"nested":{"y":3}})))
    );
    assert_eq!(
        resolve_sampling_params(&model, ThinkingLevel::High, Some(&empty)),
        Some(sampling_json(json!({"a":1,"b":2,"nested":{"x":2}})))
    );
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
    request.sampling_params = Some(sampling_json(json!({"top_p": 0.9})));
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

#[tokio::test]
async fn sampling_presence_reaches_supported_wires_once_and_other_wires_keep_metadata_only() {
    use opi_ai::auth::{AuthResolver, AuthScheme, StaticAuthResolver};
    use opi_ai::message::{InputContent, Message, UserMessage};
    use std::sync::Mutex;

    for wire in [
        WireApi::OpenAiCompletions,
        WireApi::OpenAiResponses,
        WireApi::AzureOpenAiResponses,
        WireApi::AnthropicMessages,
    ] {
        for request_params in [
            None,
            Some(sampling_json(json!({}))),
            Some(sampling_json(
                json!({"temperature":0.3,"top_p":0.9,"nested":{"y":3}}),
            )),
        ] {
            let server = MockServer::start().await;
            let (provider_id, endpoint, response) = match wire {
                WireApi::OpenAiCompletions => (
                    "openai",
                    "/chat/completions",
                    "data: {\"id\":\"chat-1\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
                ),
                WireApi::OpenAiResponses | WireApi::AzureOpenAiResponses => (
                    if wire == WireApi::AzureOpenAiResponses {
                        "azure"
                    } else {
                        "openai"
                    },
                    "/responses",
                    "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"response-1\",\"status\":\"completed\",\"model\":\"sampling-model\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n",
                ),
                WireApi::AnthropicMessages => (
                    "anthropic",
                    "/v1/messages",
                    "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"message-1\",\"model\":\"sampling-model\",\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\nevent: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":1}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
                ),
                _ => unreachable!(),
            };
            Mock::given(method("POST"))
                .and(path(endpoint))
                .respond_with(
                    wiremock::ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(response),
                )
                .expect(1)
                .mount(&server)
                .await;
            let base = sampling_json(json!({"temperature":0.1,"top_k":40,"nested":{"base":1}}));
            let model = ModelInfo::new(
                "sampling-model",
                "Sampling model",
                wire,
                ModelCapabilities::new(8192, 1024).with_streaming(true),
            )
            .with_base_url(server.uri())
            .with_sampling_params(base.clone())
            .with_sampling_level_params(
                ThinkingLevel::None,
                sampling_json(json!({"temperature":0.2,"nested":{"x":2}})),
            );
            assert_eq!(model.sampling_params.as_ref(), Some(&base));
            let auth: Arc<dyn AuthResolver> = Arc::new(StaticAuthResolver::new(
                if matches!(
                    wire,
                    WireApi::AnthropicMessages | WireApi::AzureOpenAiResponses
                ) {
                    AuthScheme::ApiKey
                } else {
                    AuthScheme::Bearer
                },
                "offline-sampling-key".into(),
            ));
            let mut models = opi_ai::Models::new();
            match wire {
                WireApi::OpenAiCompletions => {
                    let provider = Arc::new(openai_chat::OpenAiChatProvider::for_route(
                        Some(server.uri()),
                        provider_id.into(),
                        ProviderHeaders::default(),
                        vec![model.clone()],
                        Arc::new(HttpClient::new()),
                    ));
                    models
                        .register_chat_api(
                            Arc::new(
                                openai_chat::OpenAiCompletionsChatApi::new(provider, model.clone())
                                    .unwrap(),
                            ),
                            auth,
                            opi_ai::AuthProvenanceSource::Static,
                        )
                        .unwrap();
                }
                WireApi::OpenAiResponses => {
                    let provider = Arc::new(openai_responses::OpenAiResponsesProvider::for_route(
                        Some(server.uri()),
                        provider_id.into(),
                        ProviderHeaders::default(),
                        vec![model.clone()],
                        Arc::new(HttpClient::new()),
                    ));
                    models
                        .register_chat_api(
                            Arc::new(
                                openai_responses::OpenAiResponsesChatApi::new(
                                    provider,
                                    model.clone(),
                                )
                                .unwrap(),
                            ),
                            auth,
                            opi_ai::AuthProvenanceSource::Static,
                        )
                        .unwrap();
                }
                WireApi::AzureOpenAiResponses => {
                    let provider = Arc::new(
                        AzureOpenAIProvider::for_records(
                            Some(server.uri()),
                            vec![model.clone()],
                            None,
                        )
                        .unwrap(),
                    );
                    models
                        .register_chat_api(
                            Arc::new(
                                opi_ai::azure_openai::AzureOpenAIResponsesChatApi::new(
                                    provider,
                                    model.clone(),
                                )
                                .unwrap(),
                            ),
                            auth,
                            opi_ai::AuthProvenanceSource::Static,
                        )
                        .unwrap();
                }
                WireApi::AnthropicMessages => {
                    let provider = Arc::new(anthropic::AnthropicProvider::for_route(
                        provider_id.into(),
                        vec![model.clone()],
                        Some(server.uri()),
                        Default::default(),
                        Arc::new(HttpClient::new()),
                        false,
                    ));
                    models
                        .register_chat_api(
                            Arc::new(
                                anthropic::AnthropicMessagesChatApi::new(provider, model.clone())
                                    .unwrap(),
                            ),
                            auth,
                            opi_ai::AuthProvenanceSource::Static,
                        )
                        .unwrap();
                }
                _ => unreachable!(),
            }
            let observed = Arc::new(Mutex::new(Vec::new()));
            let sink = observed.clone();
            let mut request = unsupported_thinking_request(provider_id, &model.id);
            request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                system_prompt: None,
                messages: vec![Message::User(UserMessage {
                    content: vec![InputContent::Text { text: "Hi".into() }],
                    timestamp_ms: 0,
                })],
                tools: vec![],
            })
            .unwrap();
            request.thinking = Default::default();
            request.temperature = Some(0.7);
            request.sampling_params = request_params.clone();
            request.callbacks =
                opi_ai::RequestCallbacks::new().with_on_payload(Arc::new(move |body| {
                    sink.lock().unwrap().push(body.clone());
                    Ok(None)
                }));
            assert_eq!(request.sampling_params, request_params);
            models
                .complete_simple(
                    opi_ai::ChatModelRecord::new(provider_id, model),
                    request,
                    Default::default(),
                )
                .await
                .unwrap();
            let requests = server.received_requests().await.unwrap();
            assert_eq!(requests.len(), 1);
            server.verify().await;
            let body: serde_json::Value = requests[0].body_json().unwrap();
            let observed = observed.lock().unwrap();
            assert_eq!(
                observed.as_slice(),
                std::slice::from_ref(&body),
                "one final-body callback for {wire}"
            );
            if wire == WireApi::AnthropicMessages {
                assert_eq!(body["temperature"], json!(0.7));
                for key in ["top_k", "top_p", "nested"] {
                    assert!(body.get(key).is_none(), "unsupported sampling key {key}");
                }
            } else {
                let mut expected =
                    sampling_json(json!({"temperature":0.2,"top_k":40,"nested":{"x":2}}));
                if let Some(params) = request_params {
                    expected.extend(params);
                }
                for (key, value) in expected {
                    assert_eq!(body[&key], value, "{wire} sampling key {key}");
                }
            }
        }
    }
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

mod anthropic_complete_metadata {
    #[test]
    fn absent_source_true_defaults_remain_true() {
        let catalog = opi_ai::catalog::Catalog::builtin().unwrap();
        let source = catalog
            .model(
                "anthropic",
                opi_ai::catalog::CatalogOperation::Chat,
                "claude-fable-5",
            )
            .unwrap();
        let model = source.to_chat_model_info().unwrap();
        let opi_ai::WireCompat::AnthropicMessages(compat) = model.compat else {
            panic!("Anthropic source uses its matching wire")
        };
        assert!(compat.supports_eager_tool_input_streaming);
        assert!(compat.supports_temperature);
    }
}

mod anthropic_response_cost {
    use super::*;
    use opi_ai::message::AssistantMessage;
    use opi_ai::stream::Usage;
    use opi_ai::types::model::AnthropicAllowedFallbackModel;

    fn rates(scale: f64) -> Pricing {
        Pricing {
            input_cost_per_mtok: scale,
            output_cost_per_mtok: scale * 2.0,
            cache_read_cost_per_mtok: scale * 3.0,
            cache_write_cost_per_mtok: scale * 4.0,
        }
    }

    fn prices(scale: f64) -> ModelPricing {
        ModelPricing {
            base: rates(scale),
            tiers: vec![],
        }
    }

    fn model() -> ModelInfo {
        let mut model = ModelInfo::new(
            "requested",
            "Requested",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(200_000, 8192),
        );
        model.pricing = Some(prices(1.0));
        let WireCompat::AnthropicMessages(compat) = &mut model.compat else {
            unreachable!()
        };
        compat.allowed_fallback_models = Some(vec![
            AnthropicAllowedFallbackModel {
                provider: "anthropic".into(),
                model: "fallback".into(),
                cost: prices(10.0),
            },
            AnthropicAllowedFallbackModel {
                provider: "anthropic".into(),
                model: "requested".into(),
                cost: prices(999.0),
            },
            AnthropicAllowedFallbackModel {
                provider: "different".into(),
                model: "other-target".into(),
                cost: prices(20.0),
            },
        ]);
        model
    }

    fn fallback(model: &mut ModelInfo) -> &mut AnthropicAllowedFallbackModel {
        let WireCompat::AnthropicMessages(compat) = &mut model.compat else {
            unreachable!()
        };
        &mut compat.allowed_fallback_models.as_mut().unwrap()[0]
    }

    fn response(reported: Option<&str>, provider: &str) -> AssistantMessage {
        let mut response = opi_ai::test_support::base_assistant();
        response.provider = provider.into();
        response.model = "requested".into();
        response.response_model = reported.map(str::to_owned);
        response.wire = Some(WireApi::AnthropicMessages);
        response.usage = Usage::reported(
            1_000_000,
            1_000_000,
            1_000_000,
            1_000_000,
            Some(500_000),
            None,
        );
        response
    }

    #[test]
    fn cost_selects_only_a_different_reported_model_with_matching_declared_provider() {
        let model = model();
        for (reported, provider, expected) in [
            (Some("fallback"), "anthropic", 90.0),
            (Some("fallback"), "different", 9.0),
            (Some("requested"), "anthropic", 9.0),
            (Some("unlisted"), "anthropic", 9.0),
            (Some("other-target"), "anthropic", 9.0),
            (Some(""), "anthropic", 9.0),
            (None, "anthropic", 9.0),
        ] {
            let response = response(reported, provider);
            let before = serde_json::to_vec(&response).unwrap();
            let cost = model.calculate_response_cost(&response).unwrap();
            assert_eq!(cost.total_cost(), expected);
            assert_eq!(serde_json::to_vec(&response).unwrap(), before);
        }
        let mut other_wire = ModelInfo::new(
            "requested",
            "Requested",
            WireApi::OpenAiResponses,
            ModelCapabilities::new(200_000, 8192),
        );
        other_wire.pricing = model.pricing.clone();
        assert_eq!(
            other_wire
                .calculate_response_cost(&response(Some("fallback"), "anthropic"))
                .unwrap()
                .total_cost(),
            9.0
        );
    }

    #[test]
    fn fallback_pricing_uses_prompt_tiers_and_prices_the_one_hour_subset_once() {
        let mut model = model();
        fallback(&mut model).cost.tiers.push(PricingTier {
            input_tokens_above: 3_000_000.into(),
            pricing: rates(20.0),
        });
        let mut reply = response(Some("fallback"), "anthropic");
        let cost = model.calculate_response_cost(&reply).unwrap();
        assert_eq!(
            (
                cost.input_cost,
                cost.output_cost,
                cost.cache_read_cost,
                cost.cache_write_cost
            ),
            (10.0, 20.0, 30.0, 30.0)
        );
        assert_eq!(cost.total_cost(), 90.0);
        reply.usage.input_tokens += 1;
        let cost = model.calculate_response_cost(&reply).unwrap();
        assert!((cost.input_cost - 20.00002).abs() < 1e-10);
        assert_eq!(
            (
                cost.output_cost,
                cost.cache_read_cost,
                cost.cache_write_cost
            ),
            (40.0, 60.0, 60.0)
        );
        assert!((cost.total_cost() - 180.00002).abs() < 1e-10);
        fallback(&mut model).cost.base.input_cost_per_mtok = -1.0;
        assert!(
            (model.calculate_response_cost(&reply).unwrap().total_cost() - 180.00002).abs() < 1e-10
        );
        reply.usage.input_tokens -= 1;
        assert!(model.calculate_response_cost(&reply).is_none());
    }

    #[test]
    fn unknown_usage_or_selected_prices_remain_unknown_and_known_free_remains_zero() {
        let mut model = model();
        let mut reply = response(Some("fallback"), "anthropic");
        model.pricing = None;
        assert_eq!(
            model.calculate_response_cost(&reply).unwrap().total_cost(),
            90.0
        );
        assert!(
            model
                .calculate_response_cost(&response(None, "anthropic"))
                .is_none()
        );
        reply.usage = Usage::unknown();
        assert!(model.calculate_response_cost(&reply).is_none());
        reply.usage = Usage::reported(0, 0, 0, 0, None, None);
        assert_eq!(
            model.calculate_response_cost(&reply).unwrap().total_cost(),
            0.0
        );
        for unknown in [-1.0, f64::NAN, f64::INFINITY] {
            fallback(&mut model).cost.base.input_cost_per_mtok = unknown;
            assert!(model.calculate_response_cost(&reply).is_none());
        }
        fallback(&mut model).cost = prices(0.0);
        reply.usage = response(Some("fallback"), "anthropic").usage;
        assert_eq!(
            model.calculate_response_cost(&reply).unwrap().total_cost(),
            0.0
        );
    }

    #[test]
    fn fallback_declarations_reject_missing_identity_cost_and_excess_targets() {
        use opi_ai::catalog::{Catalog, CatalogOperation};
        let record = Catalog::builtin()
            .unwrap()
            .model("anthropic", CatalogOperation::Chat, "claude-fable-5")
            .unwrap()
            .clone();
        for missing in ["provider", "model", "cost"] {
            let mut record = record.clone();
            let mut target = record.raw["compat"]["allowedFallbackModels"][0].clone();
            target.as_object_mut().unwrap().remove(missing);
            record.raw["compat"]["allowedFallbackModels"] = serde_json::json!([target]);
            assert!(record.to_chat_model_info().is_err(), "missing {missing}");
        }
        let mut invalid = model();
        let WireCompat::AnthropicMessages(compat) = &mut invalid.compat else {
            unreachable!()
        };
        let target = compat.allowed_fallback_models.as_ref().unwrap()[0].clone();
        compat.allowed_fallback_models = Some(vec![target; 4]);
        assert!(invalid.validate().is_err());
        for missing in ["provider", "model"] {
            let mut invalid = model();
            if missing == "provider" {
                fallback(&mut invalid).provider.clear();
            } else {
                fallback(&mut invalid).model.clear();
            }
            assert!(invalid.validate().is_err());
        }
    }
}
