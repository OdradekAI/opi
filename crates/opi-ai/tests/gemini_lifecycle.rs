//! Contract tests for GeminiProvider::stream() -- real HTTP-level verification.
//!
//! Covers: SSE streaming, HTTP error mapping, no-terminal-event detection.
//!
//! Uses wiremock to simulate the Gemini streamGenerateContent API endpoint
//! without live API calls.

use futures_util::StreamExt;
use opi_ai::gemini::GeminiProvider;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::stream::AssistantStreamEvent;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn make_request(cancel: CancellationToken) -> Request {
    Request {
        model: "gemini:gemini-2.5-flash".into(),
        system: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "hello".into(),
            }],
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
        session_id: None,
    }
}

fn text_sse_fixture() -> &'static str {
    "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hello\"}]},\"index\":0}]}\n\n\
     data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":10,\"candidatesTokenCount\":5,\"totalTokenCount\":15}}\n\n"
}

fn incomplete_sse_fixture() -> &'static str {
    "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Partial\"}]},\"index\":0}]}\n\n"
}

// ---------------------------------------------------------------------------
// Happy path: SSE streaming
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_success() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1beta/models/gemini-2.5-flash:streamGenerateContent",
        ))
        .and(header("x-goog-api-key", "test-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = GeminiProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_auth(),
    );

    let mut events = Vec::new();
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                let is_terminal = event.is_terminal();
                events.push(event);
                if is_terminal {
                    break;
                }
            }
            Err(e) => panic!("unexpected error: {e}"),
        }
    }

    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Start { .. })),
        "should have Start event"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::TextDelta { .. })),
        "should have TextDelta event"
    );

    let done = events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("should have Done event");
    if let AssistantStreamEvent::Done { reason, message } = done {
        assert_eq!(*reason, opi_ai::stream::StopReason::Stop);
        let text: String = message
            .content
            .iter()
            .filter_map(|c| match c {
                opi_ai::message::AssistantContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(text, "Hello");
    }
}

// ---------------------------------------------------------------------------
// HTTP error mapping: auth error -> AuthFailed
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_auth_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1beta/models/gemini-2.5-flash:streamGenerateContent",
        ))
        .respond_with(ResponseTemplate::new(400).set_body_string(
            r#"{"error":{"code":401,"message":"API key not valid","status":"INVALID_ARGUMENT"}}"#,
        ))
        .mount(&server)
        .await;

    let provider = GeminiProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_auth(),
    );

    let result = stream.next().await.expect("should have event");
    match result {
        Err(ProviderError::AuthFailed(msg)) => {
            assert!(
                msg.contains("authentication failed"),
                "should mention auth failure: {msg}"
            );
        }
        other => panic!("expected AuthFailed, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// HTTP error mapping: 429 -> RateLimited
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_rate_limited() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1beta/models/gemini-2.5-flash:streamGenerateContent",
        ))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_string(r#"{"error":{"code":429,"message":"Resource exhausted"}}"#)
                .insert_header("retry-after", "5"),
        )
        .mount(&server)
        .await;

    let provider = GeminiProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_auth(),
    );

    let result = stream.next().await.expect("should have event");
    match result {
        Err(ProviderError::RateLimited { retry_after_ms }) => {
            assert!(retry_after_ms.is_some(), "should parse retry-after header");
            // 5 seconds -> 5000 ms
            assert_eq!(retry_after_ms.unwrap(), 5000);
        }
        other => panic!("expected RateLimited, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// No terminal event -> StreamError
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_no_terminal_event() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1beta/models/gemini-2.5-flash:streamGenerateContent",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(incomplete_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = GeminiProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_auth(),
    );

    let mut saw_stream_error = false;
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                if event.is_terminal() {
                    break;
                }
            }
            Err(ProviderError::StreamError(msg)) if msg.contains("terminal event") => {
                saw_stream_error = true;
                break;
            }
            Err(_) => break,
        }
    }
    assert!(
        saw_stream_error,
        "should produce StreamError about missing terminal event"
    );
}
