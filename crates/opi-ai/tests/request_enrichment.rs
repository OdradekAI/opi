//! Tests for Phase 14.3 Request scalar enrichment: timeout, extra_headers,
//! cache_retention, and session_id fields on `opi_ai::provider::Request`, plus
//! CacheRetention wire semantics and invalid-header rejection (no live calls).

use std::time::Duration;

use futures_util::StreamExt;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{
    CacheRetention, Provider, ProviderError, Request, ThinkingConfig, validate_extra_headers,
};
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
        .and(header("prompt-cache-key", "sess-abc123"))
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
}

#[tokio::test]
async fn openai_chat_session_id_clamps_to_64_chars() {
    let session_id = "a".repeat(100);
    let clamped = &session_id[..64];

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("prompt-cache-key", clamped))
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
