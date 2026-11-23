//! Sampling projection on actual Chat, Responses, Azure and Anthropic requests.

use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::api::shared::headers::ProviderHeaders;
use opi_ai::api::{anthropic, openai_completions, openai_responses};
use opi_ai::http::HttpClient;
use opi_ai::provider::{Provider, Request, ThinkingConfig};
use opi_ai::providers::anthropic as anthropic_provider;
use opi_ai::providers::azure_openai::AzureOpenAIProvider;
use opi_ai::providers::openai as openai_profile;
use opi_ai::providers::openai_responses as openai_responses_profile;
use opi_ai::registry::ModelCapabilities;
use opi_ai::types::model::{ModelInfo, ThinkingLevel, WireApi};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

use super::fixtures::{model_request, sampling_json};

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
    let mut request = model_request(provider_id, "model");
    request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::High,
        budget_tokens: None,
    };
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
    let provider = openai_profile::OpenAiChatProvider::for_route(
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
    let provider = openai_responses_profile::OpenAiResponsesProvider::for_route(
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
                    let provider = Arc::new(openai_profile::OpenAiChatProvider::for_route(
                        Some(server.uri()),
                        provider_id.into(),
                        ProviderHeaders::default(),
                        vec![model.clone()],
                        Arc::new(HttpClient::new()),
                    ));
                    models
                        .register_chat_api(
                            Arc::new(
                                openai_completions::OpenAiCompletionsChatApi::new(
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
                WireApi::OpenAiResponses => {
                    let provider = Arc::new(
                        openai_responses_profile::OpenAiResponsesProvider::for_route(
                            Some(server.uri()),
                            provider_id.into(),
                            ProviderHeaders::default(),
                            vec![model.clone()],
                            Arc::new(HttpClient::new()),
                        ),
                    );
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
                                opi_ai::api::azure_openai::AzureOpenAIResponsesChatApi::new(
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
                    let provider = Arc::new(anthropic_provider::AnthropicProvider::for_route(
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
            let mut request = model_request(provider_id, &model.id);
            request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                system_prompt: None,
                messages: vec![Message::User(UserMessage {
                    content: vec![InputContent::Text { text: "Hi".into() }],
                    timestamp_ms: 0,
                })],
                tools: vec![],
            })
            .unwrap();
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
