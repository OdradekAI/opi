//! Provider image serialization fixture tests for task 3.4.
//!
//! Validates that each provider serializes InputContent::Image correctly
//! in its HTTP request body. Uses wiremock to capture the serialized body.

use futures_util::StreamExt;
use opi_ai::message::{ImageSource, InputContent, MediaType, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Provider, Request, ThinkingConfig};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn image_url_msg() -> Message {
    Message::User(UserMessage {
        content: vec![InputContent::Image {
            source: ImageSource::Url {
                url: "https://example.com/photo.png".into(),
            },
            media_type: MediaType::Png,
        }],
        timestamp_ms: 1000,
    })
}

fn image_base64_msg() -> Message {
    Message::User(UserMessage {
        content: vec![InputContent::Image {
            source: ImageSource::Base64 {
                data: "iVBORw0KGgo=".into(),
            },
            media_type: MediaType::Png,
        }],
        timestamp_ms: 1000,
    })
}

fn image_bytes_msg() -> Message {
    Message::User(UserMessage {
        content: vec![InputContent::Image {
            source: ImageSource::Bytes {
                data: vec![0x89, 0x50, 0x4E, 0x47],
            },
            media_type: MediaType::Png,
        }],
        timestamp_ms: 1000,
    })
}

fn mixed_text_image_msg() -> Message {
    Message::User(UserMessage {
        content: vec![
            InputContent::Text {
                text: "Describe this".into(),
            },
            InputContent::Image {
                source: ImageSource::Url {
                    url: "https://example.com/photo.png".into(),
                },
                media_type: MediaType::Png,
            },
        ],
        timestamp_ms: 1000,
    })
}

fn done_sse() -> &'static str {
    "data: {\"id\":\"done\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n"
}

fn anthropic_done_sse() -> &'static str {
    "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[],\"model\":\"claude-sonnet-4-5-20250514\",\"stop_reason\":null,\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\nevent: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"stop\"},\"usage\":{\"output_tokens\":1}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n"
}

fn gemini_done_sse() -> &'static str {
    "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"ok\"}]},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":1,\"candidatesTokenCount\":1,\"totalTokenCount\":2}}\n\n"
}

fn make_request(messages: Vec<Message>) -> Request {
    Request {
        transport: Default::default(),
        model: "test-model".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages,
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

async fn drain_stream(mut stream: opi_ai::provider::EventStream) {
    while let Some(result) = stream.next().await {
        let event = result.expect("image serialization reaches the real adapter");
        if event.is_terminal() {
            break;
        }
    }
}

async fn get_request_body(server: &MockServer) -> serde_json::Value {
    let requests = server.received_requests().await;
    let requests = requests.expect("no requests received");
    let body = &requests[0].body;
    serde_json::from_slice(body).expect("body is not valid JSON")
}

// --- Anthropic image serialization ---

#[tokio::test]
async fn anthropic_image_url_in_request_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(anthropic_done_sse())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = opi_ai::anthropic::AnthropicProvider::new(Some(server.uri()));
    drain_stream(provider.stream_prepared(
        Request {
            max_tokens: Some(64),
            ..make_request(vec![image_url_msg()])
        },
        opi_ai::test_support::resolved_auth(),
    ))
    .await;

    let body = get_request_body(&server).await;
    let content = &body["messages"][0]["content"][0];
    assert_eq!(content["type"], "image");
    assert_eq!(content["source"]["type"], "url");
    assert_eq!(content["source"]["url"], "https://example.com/photo.png");
}

#[tokio::test]
async fn anthropic_image_base64_in_request_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(anthropic_done_sse())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = opi_ai::anthropic::AnthropicProvider::new(Some(server.uri()));
    drain_stream(provider.stream_prepared(
        Request {
            max_tokens: Some(64),
            ..make_request(vec![image_base64_msg()])
        },
        opi_ai::test_support::resolved_auth(),
    ))
    .await;

    let body = get_request_body(&server).await;
    let content = &body["messages"][0]["content"][0];
    assert_eq!(content["type"], "image");
    assert_eq!(content["source"]["type"], "base64");
    assert_eq!(content["source"]["data"], "iVBORw0KGgo=");
}

#[tokio::test]
async fn anthropic_image_bytes_encoded_as_base64() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(anthropic_done_sse())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = opi_ai::anthropic::AnthropicProvider::new(Some(server.uri()));
    drain_stream(provider.stream_prepared(
        Request {
            max_tokens: Some(64),
            ..make_request(vec![image_bytes_msg()])
        },
        opi_ai::test_support::resolved_auth(),
    ))
    .await;

    let body = get_request_body(&server).await;
    let content = &body["messages"][0]["content"][0];
    assert_eq!(content["source"]["type"], "base64");
    assert_eq!(content["source"]["data"], "iVBORw==");
}

// --- OpenAI Chat image serialization ---

#[tokio::test]
async fn openai_chat_image_url_in_request_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(done_sse())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = opi_ai::openai_chat::OpenAiChatProvider::new(Some(server.uri()));
    drain_stream(provider.stream_prepared(
        make_request(vec![image_url_msg()]),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;

    let body = get_request_body(&server).await;
    let content = &body["messages"][0]["content"];
    assert!(content.is_array());
    assert_eq!(content[0]["type"], "image_url");
    assert_eq!(
        content[0]["image_url"]["url"],
        "https://example.com/photo.png"
    );
}

#[tokio::test]
async fn openai_chat_image_base64_as_data_uri() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(done_sse())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = opi_ai::openai_chat::OpenAiChatProvider::new(Some(server.uri()));
    drain_stream(provider.stream_prepared(
        make_request(vec![image_base64_msg()]),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;

    let body = get_request_body(&server).await;
    let content = &body["messages"][0]["content"][0];
    assert_eq!(
        content["image_url"]["url"],
        "data:image/png;base64,iVBORw0KGgo="
    );
}

#[tokio::test]
async fn openai_chat_mixed_text_image_not_flattened() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(done_sse())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = opi_ai::openai_chat::OpenAiChatProvider::new(Some(server.uri()));
    drain_stream(provider.stream_prepared(
        make_request(vec![mixed_text_image_msg()]),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;

    let body = get_request_body(&server).await;
    let content = &body["messages"][0]["content"];
    assert!(content.is_array());
    assert_eq!(content[0]["type"], "text");
    assert_eq!(content[1]["type"], "image_url");
}

// --- OpenAI Responses image serialization ---

#[tokio::test]
async fn openai_responses_image_url_in_request_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("data: {\"type\":\"response.completed\",\"response\":{\"id\":\"r1\",\"object\":\"response\",\"status\":\"completed\"}}\n\n")
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = opi_ai::openai_responses::OpenAiResponsesProvider::new(Some(server.uri()));
    drain_stream(provider.stream_prepared(
        make_request(vec![image_url_msg()]),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;

    let body = get_request_body(&server).await;
    let input = &body["input"];
    assert!(input.is_array());
    let item = &input[0];
    assert_eq!(item["role"], "user");
    assert!(item["content"].is_array());
    assert_eq!(item["content"][0]["type"], "input_image");
    assert_eq!(
        item["content"][0]["image_url"],
        "https://example.com/photo.png"
    );
}

#[tokio::test]
async fn openai_responses_image_base64_as_data_uri() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("data: {\"type\":\"response.completed\",\"response\":{\"id\":\"r1\",\"object\":\"response\",\"status\":\"completed\"}}\n\n")
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = opi_ai::openai_responses::OpenAiResponsesProvider::new(Some(server.uri()));
    drain_stream(provider.stream_prepared(
        make_request(vec![image_base64_msg()]),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;

    let body = get_request_body(&server).await;
    let content = &body["input"][0]["content"][0];
    assert_eq!(content["image_url"], "data:image/png;base64,iVBORw0KGgo=");
}

// --- Gemini image serialization ---

#[tokio::test]
async fn gemini_image_url_in_request_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/test-model:streamGenerateContent"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(gemini_done_sse())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = opi_ai::gemini::GeminiProvider::new(Some(server.uri()));
    drain_stream(provider.stream_prepared(
        make_request(vec![image_url_msg()]),
        opi_ai::test_support::resolved_auth(),
    ))
    .await;

    let body = get_request_body(&server).await;
    let parts = &body["contents"][0]["parts"][0];
    assert_eq!(
        parts["file_data"]["file_uri"],
        "https://example.com/photo.png"
    );
    assert_eq!(parts["file_data"]["mime_type"], "image/png");
}

#[tokio::test]
async fn gemini_image_base64_as_inline_data() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(gemini_done_sse())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = opi_ai::gemini::GeminiProvider::new(Some(server.uri()));
    drain_stream(provider.stream_prepared(
        make_request(vec![image_base64_msg()]),
        opi_ai::test_support::resolved_auth(),
    ))
    .await;

    let body = get_request_body(&server).await;
    let parts = &body["contents"][0]["parts"][0];
    assert_eq!(parts["inline_data"]["mime_type"], "image/png");
    assert_eq!(parts["inline_data"]["data"], "iVBORw0KGgo=");
}

// --- OpenRouter (delegates to OpenAI Chat) ---

#[tokio::test]
async fn openrouter_image_in_request_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(done_sse())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = opi_ai::openrouter::openrouter_provider(Some(server.uri()));
    drain_stream(provider.stream_prepared(
        make_request(vec![mixed_text_image_msg()]),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;

    let body = get_request_body(&server).await;
    let content = &body["messages"][0]["content"];
    assert!(content.is_array());
    assert_eq!(content[0]["type"], "text");
    assert_eq!(content[1]["type"], "image_url");
}

// --- Mistral (delegates to OpenAI Chat) ---

#[tokio::test]
async fn mistral_image_in_request_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(done_sse())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = opi_ai::mistral::mistral_provider(Some(server.uri()));
    drain_stream(provider.stream_prepared(
        make_request(vec![mixed_text_image_msg()]),
        opi_ai::test_support::resolved_bearer_auth(),
    ))
    .await;

    let body = get_request_body(&server).await;
    let content = &body["messages"][0]["content"];
    assert!(content.is_array());
    assert_eq!(content[0]["type"], "text");
    assert_eq!(content[1]["type"], "image_url");
}

// ---------------------------------------------------------------------------
// Task 20.45: OpenRouter image adapter wire serialization (direct adapter
// level, no Models). Pins the nonstream Chat-wire body: modalities from the
// record's output, context projection with data-URL images, and the
// Bearer-only pre-dispatch scheme rejection.
// ---------------------------------------------------------------------------

use std::time::Duration;

use opi_ai::api::ImageApi;
use opi_ai::api::openrouter_images::{OpenRouterImagesApi, OpenRouterImagesOptions};
use opi_ai::auth::{AuthScheme, ResolvedAuth};
use opi_ai::http::HttpClient;
use opi_ai::images::{
    ImageCostRates, ImageModelRecord, ImageOutputModality, ImagesCall, ImagesContext, ImagesInput,
    ImagesStopReason,
};
use opi_ai::models::ApiId;
use secrecy::SecretString;

fn adapter_record(base_url: String, output: Vec<ImageOutputModality>) -> ImageModelRecord {
    ImageModelRecord {
        cost_tiers: Vec::new(),
        headers: Vec::new(),
        input_limits: None,
        input: vec![
            opi_ai::catalog::CatalogInputModality::Text,
            opi_ai::catalog::CatalogInputModality::Image,
        ],
        provider_id: "openrouter".into(),
        id: "google/gemini-2.5-flash-image".into(),
        name: "Gemini 2.5 Flash Image (via OpenRouter)".into(),
        api: ApiId::new("openrouter-images"),
        base_url,
        output,
        cost: Some(ImageCostRates {
            input: 3.0,
            output: 15.0,
            cache_read: 0.3,
            cache_write: 3.75,
        }),
    }
}

fn adapter_bearer_auth() -> ResolvedAuth {
    ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::Bearer,
        secret: SecretString::from("test-key"),
        base_url: None,
        account_id: None,
        provenance: Default::default(),
    }
}

fn adapter_call() -> ImagesCall {
    ImagesCall {
        timeout: Some(Duration::from_secs(30)),
        ..Default::default()
    }
}

async fn adapter_generate(
    base_url: String,
    output: Vec<ImageOutputModality>,
    context: ImagesContext,
    auth: ResolvedAuth,
) -> opi_ai::images::ImagesResult {
    let api = OpenRouterImagesApi::new(
        HttpClient::new(),
        adapter_record(base_url.clone(), output.clone()),
    );
    api.generate_images_prepared(
        adapter_record(base_url, output),
        context,
        OpenRouterImagesOptions::default(),
        auth,
        adapter_call(),
    )
    .await
}

#[tokio::test]
async fn openrouter_images_serializes_context_and_image_only_modalities() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-ser",
            "choices": []
        })))
        .mount(&server)
        .await;

    let context = ImagesContext {
        input: vec![
            ImagesInput::Text {
                text: "edit this 🐈 photo".into(),
            },
            ImagesInput::Image {
                mime_type: "image/png".into(),
                data: "iVBORw0KGgo=".into(),
            },
        ],
    };
    let result = adapter_generate(
        server.uri(),
        vec![ImageOutputModality::Image],
        context,
        adapter_bearer_auth(),
    )
    .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 1);
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "model": "google/gemini-2.5-flash-image",
            "messages": [
                {
                    "role": "user",
                    "content": [
                        { "type": "text", "text": "edit this 🐈 photo" },
                        {
                            "type": "image_url",
                            "image_url": { "url": "data:image/png;base64,iVBORw0KGgo=" }
                        }
                    ]
                }
            ],
            "stream": false,
            "modalities": ["image"]
        })
    );
}

#[tokio::test]
async fn openrouter_images_text_output_modalities() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-mod",
            "choices": []
        })))
        .mount(&server)
        .await;

    let result = adapter_generate(
        server.uri(),
        vec![ImageOutputModality::Image, ImageOutputModality::Text],
        ImagesContext {
            input: vec![ImagesInput::Text { text: "hi".into() }],
        },
        adapter_bearer_auth(),
    )
    .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Stop);
    let sent = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
    assert_eq!(body["modalities"], serde_json::json!(["image", "text"]));
}

#[tokio::test]
async fn openrouter_images_rejects_non_bearer_scheme_before_dispatch() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let auth = ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::ApiKey,
        secret: SecretString::from("test-key"),
        base_url: None,
        account_id: None,
        provenance: Default::default(),
    };
    let result = adapter_generate(
        server.uri(),
        vec![ImageOutputModality::Image],
        ImagesContext {
            input: vec![ImagesInput::Text { text: "hi".into() }],
        },
        auth,
    )
    .await;

    assert_eq!(result.stop_reason, ImagesStopReason::Error);
    assert!(matches!(
        result.error.as_ref().expect("typed error"),
        opi_ai::provider::ProviderError::Config(_)
    ));
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains("bearer"),
        "message: {:?}",
        result.error_message
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}
