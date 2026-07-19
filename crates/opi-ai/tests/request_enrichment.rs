//! Tests for Phase 14.3 Request scalar enrichment: timeout, extra_headers,
//! cache_retention, and session_id fields on `opi_ai::provider::Request`, plus
//! CacheRetention wire semantics and invalid-header rejection (no live calls).

use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use opi_ai::auth::{AuthScheme, StaticAuthResolver};
use opi_ai::http::HttpClient;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{
    CacheRetention, ModelInfo, Provider, ProviderError, Request, ThinkingConfig,
    validate_extra_headers,
};
use opi_ai::registry::ModelCapabilities;
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// CacheRetention type
// ---------------------------------------------------------------------------

#[test]
fn cache_retention_default_is_none() {
    let cr = CacheRetention::default();
    assert_eq!(cr, CacheRetention::None);
}

#[test]
fn cache_retention_variants_exist() {
    let none = CacheRetention::None;
    let disabled = CacheRetention::Disabled;
    let short = CacheRetention::Short;
    let long = CacheRetention::Long;

    assert_ne!(none, disabled);
    assert_ne!(none, short);
    assert_ne!(none, long);
    assert_ne!(disabled, short);
    assert_ne!(disabled, long);
    assert_ne!(short, long);
}

// ---------------------------------------------------------------------------
// Request new fields
// ---------------------------------------------------------------------------

#[test]
fn request_new_scalars_default_to_none_empty() {
    let cancel = CancellationToken::new();
    let req = Request {
        model: "test".into(),
        system: None,
        messages: vec![],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::default(),
        session_id: None,
    };
    assert!(req.timeout.is_none());
    assert!(req.extra_headers.is_empty());
    assert_eq!(req.cache_retention, CacheRetention::None);
    assert!(req.session_id.is_none());
}

#[test]
fn request_scalars_carry_explicit_values() {
    let cancel = CancellationToken::new();
    let timeout = Duration::from_secs(30);
    let headers = vec![("X-Test".to_string(), "value".to_string())];
    let req = Request {
        model: "test".into(),
        system: None,
        messages: vec![],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout: Some(timeout),
        extra_headers: headers.clone(),
        cache_retention: CacheRetention::Long,
        session_id: Some("sess-abc".into()),
    };
    assert_eq!(req.timeout, Some(Duration::from_secs(30)));
    assert_eq!(req.extra_headers, vec![("X-Test".into(), "value".into())]);
    assert_eq!(req.cache_retention, CacheRetention::Long);
    assert_eq!(req.session_id.as_deref(), Some("sess-abc"));
}

// ---------------------------------------------------------------------------
// Invalid extra-header rejection (no network)
// ---------------------------------------------------------------------------

#[test]
fn extra_headers_reject_auth_header_names() {
    let auth_names = [
        "authorization",
        "Authorization",
        "AUTHORIZATION",
        "x-api-key",
        "X-Api-Key",
    ];
    for name in &auth_names {
        let headers = vec![(name.to_string(), "secret".into())];
        let err = validate_extra_headers(&headers).unwrap_err();
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("auth") || msg.contains("reserved") || msg.contains("forbidden"),
            "expected auth rejection for header '{name}', got: {err}"
        );
    }
}

#[test]
fn extra_headers_reject_control_characters_in_name() {
    for bad in &["X-\nHeader", "X-\rHeader", "X:\tHeader"] {
        let headers = vec![(bad.to_string(), "value".into())];
        let err = validate_extra_headers(&headers).unwrap_err();
        assert!(
            matches!(err, ProviderError::RequestFailed(_)),
            "expected RequestFailed for header {bad:?}, got: {err}"
        );
    }
}

#[test]
fn extra_headers_reject_empty_name() {
    let headers = vec![("".to_string(), "value".into())];
    let err = validate_extra_headers(&headers).unwrap_err();
    assert!(matches!(err, ProviderError::RequestFailed(_)));
}

#[test]
fn extra_headers_accept_valid_names() {
    let headers = vec![
        ("X-Custom".to_string(), "val".into()),
        ("x-request-id".to_string(), "abc123".into()),
        ("OpenAI-Beta".to_string(), "v1".into()),
    ];
    validate_extra_headers(&headers).unwrap();
}

// ---------------------------------------------------------------------------
// Timeout wire
// ---------------------------------------------------------------------------

#[tokio::test]
async fn provider_timeout_produces_timeout_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::anthropic::AnthropicProvider::new("test-key".into(), Some(server.uri()));
    let request = make_anthropic_request(cancel, Some(Duration::from_millis(100)));

    let mut stream = provider.stream(request);
    let item = stream.next().await.unwrap();
    match item {
        Err(ProviderError::Timeout) => {}
        other => panic!("expected Timeout, got {other:?}"),
    }
}

#[tokio::test]
async fn provider_no_timeout_completes_normally() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                "event: message_stop\r\ndata: {\"type\":\"message_stop\"}\r\n\r\n",
            ),
        )
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::anthropic::AnthropicProvider::new("test-key".into(), Some(server.uri()));
    let request = make_anthropic_request(cancel, None);

    let mut stream = provider.stream(request);
    // Drain until we get a Done event (or error).
    let mut saw_done = false;
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => saw_done = true,
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
    assert!(saw_done, "stream should produce at least one event");
}

// ---------------------------------------------------------------------------
// Extra-headers wire
// ---------------------------------------------------------------------------

#[tokio::test]
async fn extra_headers_reach_anthropic_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("X-Custom", "my-value"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                "event: message_stop\r\ndata: {\"type\":\"message_stop\"}\r\n\r\n",
            ),
        )
        .expect(1)
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::anthropic::AnthropicProvider::new("test-key".into(), Some(server.uri()));
    let mut request = make_anthropic_request(cancel, None);
    request.extra_headers = vec![("X-Custom".to_string(), "my-value".to_string())];

    let mut stream = provider.stream(request);
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => {}
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Session-affinity wire mappings (OpenAI Chat)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn openai_chat_session_id_becomes_prompt_cache_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{\"content\":\"hi\"},\"index\":0,\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider =
        opi_ai::openai_chat::OpenAiChatProvider::new("test-key".into(), Some(server.uri()));
    let request = make_openai_chat_request(cancel, Some("sess-abc123".into()));

    let mut stream = provider.stream(request);
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => {}
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
    let request = server.received_requests().await.unwrap().remove(0);
    assert_eq!(
        request_body_json(&request)["prompt_cache_key"],
        "sess-abc123"
    );
    assert_eq!(header_value(&request, "prompt-cache-key"), None);
}

#[tokio::test]
async fn openai_chat_session_id_clamps_to_64_chars() {
    let session_id = "a".repeat(100);
    let clamped = session_id[..64].to_owned();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{\"content\":\"hi\"},\"index\":0,\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider =
        opi_ai::openai_chat::OpenAiChatProvider::new("test-key".into(), Some(server.uri()));
    let request = make_openai_chat_request(cancel, Some(session_id));

    let mut stream = provider.stream(request);
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => {}
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
    let request = server.received_requests().await.unwrap().remove(0);
    assert_eq!(request_body_json(&request)["prompt_cache_key"], clamped);
}

#[tokio::test]
async fn openai_chat_no_session_id_no_cache_key_header() {
    let server = MockServer::start().await;
    // The mock accepts any POST without requiring a prompt-cache-key header.
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"delta\":{\"content\":\"hi\"},\"index\":0,\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider =
        opi_ai::openai_chat::OpenAiChatProvider::new("test-key".into(), Some(server.uri()));
    let request = make_openai_chat_request(cancel, None);

    let mut stream = provider.stream(request);
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => {}
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
    let request = server.received_requests().await.unwrap().remove(0);
    assert!(
        request_body_json(&request)
            .get("prompt_cache_key")
            .is_none()
    );
    assert_eq!(header_value(&request, "prompt-cache-key"), None);
}

#[tokio::test]
async fn session_affinity_wire_mappings() {
    let chat_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string("data: [DONE]\n\n"))
        .expect(4)
        .mount(&chat_server)
        .await;
    let models = vec![ModelInfo::new(
        "model",
        "Model",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(8_192, 1_024).with_streaming(true),
    )];
    let direct =
        opi_ai::openai_chat::OpenAiChatProvider::new("test-key".into(), Some(chat_server.uri()));
    drain(direct.stream(make_openai_chat_request(
        CancellationToken::new(),
        Some("session-direct".into()),
    )))
    .await;
    let compatible = opi_ai::openai_chat::OpenAiChatProvider::new_for_profile(
        "test-key".into(),
        chat_server.uri(),
        "compatible".into(),
        opi_ai::openai_chat::CompatConfig {
            send_session_affinity_headers: true,
            ..Default::default()
        },
        vec![],
        models.clone(),
    );
    drain(compatible.stream(make_openai_chat_request(
        CancellationToken::new(),
        Some("session-compatible".into()),
    )))
    .await;
    let compatible_default = opi_ai::openai_chat::OpenAiChatProvider::new_for_profile(
        "test-key".into(),
        chat_server.uri(),
        "compatible-default".into(),
        Default::default(),
        vec![],
        models,
    );
    drain(compatible_default.stream(make_openai_chat_request(
        CancellationToken::new(),
        Some("session-default".into()),
    )))
    .await;
    let mut disabled =
        make_openai_chat_request(CancellationToken::new(), Some("session-disabled".into()));
    disabled.cache_retention = CacheRetention::Disabled;
    drain(compatible.stream(disabled)).await;

    let chat_requests = chat_server.received_requests().await.unwrap();
    assert_eq!(chat_requests.len(), 4);
    assert_eq!(
        request_body_json(&chat_requests[0])["prompt_cache_key"],
        "session-direct"
    );
    for name in ["session_id", "x-client-request-id", "x-session-affinity"] {
        assert_eq!(header_value(&chat_requests[0], name), None);
        assert_eq!(
            header_value(&chat_requests[1], name),
            Some("session-compatible")
        );
        assert_eq!(header_value(&chat_requests[2], name), None);
        assert_eq!(header_value(&chat_requests[3], name), None);
    }
    for request in &chat_requests[1..] {
        assert!(request_body_json(request).get("prompt_cache_key").is_none());
    }

    let responses_server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string("data: [DONE]\n\n"))
        .expect(5)
        .mount(&responses_server)
        .await;
    let standard = opi_ai::openai_responses::OpenAiResponsesProvider::new(
        "test-key".into(),
        Some(responses_server.uri()),
    );
    drain(standard.stream(make_openai_responses_request(
        "openai-responses:model",
        "session-standard",
    )))
    .await;
    let direct_with_session_header =
        opi_ai::openai_responses::OpenAiResponsesProvider::new_with_config(
            "test-key".into(),
            Some(responses_server.uri()),
            opi_ai::openai_responses::ResponsesConfig {
                send_session_id_header: false,
                ..Default::default()
            },
        );
    drain(
        direct_with_session_header.stream(make_openai_responses_request(
            "openai-responses:model",
            "session-direct-header",
        )),
    )
    .await;
    let mut disabled = make_openai_responses_request("openai-responses:model", "session-disabled");
    disabled.cache_retention = CacheRetention::Disabled;
    drain(standard.stream(disabled)).await;
    let custom_model = ModelInfo::new(
        "model",
        "Model",
        opi_ai::WireApi::OpenAiResponses,
        ModelCapabilities::new(128_000, 16_384).with_streaming(true),
    );
    let custom_default = opi_ai::openai_responses::OpenAiResponsesProvider::for_route(
        Arc::new(StaticAuthResolver::new(
            AuthScheme::Bearer,
            SecretString::from("test-token"),
        )),
        Some(responses_server.uri()),
        "custom-responses".into(),
        opi_ai::ProviderHeaders::default(),
        vec![custom_model.clone()],
        Arc::new(HttpClient::new()),
    );
    drain(custom_default.stream(make_openai_responses_request(
        "custom-responses:model",
        "session-custom-default",
    )))
    .await;
    let custom_opt_in_model = custom_model
        .with_compat(opi_ai::WireCompat::OpenAiResponses(
            opi_ai::model_info::OpenAiResponsesCompat {
                send_session_id_header: true,
                ..Default::default()
            },
        ))
        .unwrap();
    let custom_opt_in = opi_ai::openai_responses::OpenAiResponsesProvider::for_route(
        Arc::new(StaticAuthResolver::new(
            AuthScheme::Bearer,
            SecretString::from("test-token"),
        )),
        Some(responses_server.uri()),
        "custom-responses".into(),
        opi_ai::ProviderHeaders::default(),
        vec![custom_opt_in_model],
        Arc::new(HttpClient::new()),
    );
    drain(custom_opt_in.stream(make_openai_responses_request(
        "custom-responses:model",
        "session-custom-opt-in",
    )))
    .await;

    let response_requests = responses_server.received_requests().await.unwrap();
    assert_eq!(response_requests.len(), 5);
    assert_eq!(
        request_body_json(&response_requests[0])["prompt_cache_key"],
        "session-standard"
    );
    let standard_request_id = header_value(&response_requests[0], "x-client-request-id")
        .expect("direct Responses request id");
    assert_eq!(
        uuid::Uuid::parse_str(standard_request_id)
            .unwrap()
            .get_version_num(),
        7
    );
    assert_ne!(standard_request_id, "session-standard");
    assert_eq!(
        header_value(&response_requests[0], "session_id"),
        Some("session-standard")
    );

    assert_eq!(
        request_body_json(&response_requests[1])["prompt_cache_key"],
        "session-direct-header"
    );
    let direct_request_id = header_value(&response_requests[1], "x-client-request-id")
        .expect("direct Responses request id");
    assert_eq!(
        uuid::Uuid::parse_str(direct_request_id)
            .unwrap()
            .get_version_num(),
        7
    );
    assert_ne!(direct_request_id, standard_request_id);
    assert_eq!(header_value(&response_requests[1], "session_id"), None);

    for request in &response_requests[2..4] {
        assert!(request_body_json(request).get("prompt_cache_key").is_none());
        for name in ["session_id", "session-id", "x-client-request-id"] {
            assert_eq!(header_value(request, name), None);
        }
    }
    assert_eq!(
        request_body_json(&response_requests[4])["prompt_cache_key"],
        "session-custom-opt-in"
    );
    assert_eq!(
        header_value(&response_requests[4], "session_id"),
        Some("session-custom-opt-in")
    );
    let custom_request_id = header_value(&response_requests[4], "x-client-request-id")
        .expect("custom opt-in request id");
    assert_eq!(
        uuid::Uuid::parse_str(custom_request_id)
            .unwrap()
            .get_version_num(),
        7
    );
    assert_ne!(custom_request_id, "session-custom-opt-in");

    let anthropic_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .expect(1)
        .mount(&anthropic_server)
        .await;
    let anthropic =
        opi_ai::anthropic::AnthropicProvider::new("test-key".into(), Some(anthropic_server.uri()));
    let mut request = make_anthropic_request(CancellationToken::new(), None);
    request.session_id = Some("session-anthropic".into());
    drain(anthropic.stream(request)).await;
    let request = anthropic_server
        .received_requests()
        .await
        .unwrap()
        .remove(0);
    for name in [
        "prompt-cache-key",
        "session-id",
        "session_id",
        "x-client-request-id",
        "x-session-affinity",
    ] {
        assert_eq!(header_value(&request, name), None);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_anthropic_request(cancel: CancellationToken, timeout: Option<Duration>) -> Request {
    Request {
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
        system: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text { text: "hi".into() }],
            timestamp_ms: 0,
        })],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

fn make_openai_chat_request(cancel: CancellationToken, session_id: Option<String>) -> Request {
    Request {
        model: "openai:gpt-4o".into(),
        system: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text { text: "hi".into() }],
            timestamp_ms: 0,
        })],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id,
    }
}

fn make_openai_responses_request(model: &str, session_id: &str) -> Request {
    let mut request = make_openai_chat_request(CancellationToken::new(), Some(session_id.into()));
    request.model = model.into();
    request
}

async fn drain(mut stream: opi_ai::provider::EventStream) {
    while stream.next().await.is_some() {}
}

fn header_value<'a>(request: &'a wiremock::Request, name: &str) -> Option<&'a str> {
    request
        .headers
        .get(name)
        .and_then(|value| value.to_str().ok())
}

fn request_body_json(request: &wiremock::Request) -> serde_json::Value {
    serde_json::from_slice(&request.body).expect("captured request body should be JSON")
}
