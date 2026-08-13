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
        model: "test-model".into(),
        system: None,
        messages,
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

async fn drain_stream(mut stream: opi_ai::provider::EventStream) {
    while let Some(result) = stream.next().await {
        if let Ok(event) = result
            && event.is_terminal()
        {
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
        make_request(vec![image_url_msg()]),
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
        make_request(vec![image_base64_msg()]),
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
        make_request(vec![image_bytes_msg()]),
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
        opi_ai::test_support::resolved_auth(),
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
        opi_ai::test_support::resolved_auth(),
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
        opi_ai::test_support::resolved_auth(),
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
        opi_ai::test_support::resolved_auth(),
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
        opi_ai::test_support::resolved_auth(),
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
        opi_ai::test_support::resolved_auth(),
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
        opi_ai::test_support::resolved_auth(),
    ))
    .await;

    let body = get_request_body(&server).await;
    let content = &body["messages"][0]["content"];
    assert!(content.is_array());
    assert_eq!(content[0]["type"], "text");
    assert_eq!(content[1]["type"], "image_url");
}
