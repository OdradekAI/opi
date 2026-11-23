//! Anthropic prepared request serialization and typed HTTP failures.

use crate::fixtures::{make_request, text_sse_fixture};
use futures_util::StreamExt;
use opi_ai::provider::{Provider, ProviderError, Request};
use opi_ai::providers::anthropic::AnthropicProvider;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn stream_sends_correct_headers_and_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key-123"))
        .and(header("anthropic-version", "2023-06-01"))
        .and(header("content-type", "application/json"))
        .and(body_partial_json(serde_json::json!({
            "model": "claude-sonnet-4",
            "max_tokens": 1024,
            "stream": true,
            "messages": [{"role": "user", "content": [{"type": "text", "text": "hello"}]}],
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new(Some(server.uri()));
    // The mock matches x-api-key "test-key-123", so build a ResolvedAuth whose
    // secret is exactly that (resolved_auth()'s placeholder is "test-key").
    let auth = opi_ai::auth::ResolvedAuth {
        env: Default::default(),
        scheme: opi_ai::auth::AuthScheme::ApiKey,
        secret: secrecy::SecretString::from("test-key-123"),
        base_url: None,
        account_id: None,
        provenance: opi_ai::auth::AuthProvenance::default(),
    };
    let mut stream = provider.stream_prepared(make_request(CancellationToken::new()), auth);

    // Consume stream to trigger the request
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) if event.is_terminal() => break,
            Err(_) => break,
            _ => {}
        }
    }

    // wiremock verifies that all matchers were satisfied
    server.verify().await;
}

fn accepted_failure_cause<'a>(error: &'a ProviderError, request: &Request) -> &'a ProviderError {
    use sha2::Digest as _;
    let ProviderError::StreamFailure(failure) = error else {
        panic!("accepted projection must retain its failure carrier: {error:?}");
    };
    let facts = failure
        .message()
        .projection
        .as_ref()
        .expect("accepted projection is known");
    let bytes = serde_json::to_vec(request.context.messages()).unwrap();
    assert_eq!(
        facts.source_prefix.message_count,
        request.context.messages().len()
    );
    assert_eq!(
        facts.source_prefix.sha256,
        format!("{:x}", sha2::Sha256::digest(bytes))
    );
    assert_eq!(
        facts.temporal.mode,
        opi_ai::transcript::ProjectionMode::Lossless
    );
    assert!(facts.temporal.affected_ranges.is_empty());
    assert!(facts.replay.losses.is_empty());
    failure.cause()
}

#[tokio::test]
async fn stream_http_401_maps_to_auth_failed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_string(
            r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#,
        ))
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new(Some(server.uri()));
    let request = make_request(CancellationToken::new());
    let mut stream =
        provider.stream_prepared(request.clone(), opi_ai::test_support::resolved_auth());

    let result = stream.next().await.expect("should have event");
    match result
        .as_ref()
        .map_err(|error| accepted_failure_cause(error, &request))
    {
        Err(ProviderError::AuthFailed(msg)) => {
            assert!(
                msg.contains("provider rejected credentials"),
                "should mention auth failure: {msg}"
            );
        }
        other => panic!("expected AuthFailed, got: {other:?}"),
    }
    assert!(stream.next().await.is_none(), "failure must be terminal");
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn stream_http_429_maps_to_rate_limited() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(429).set_body_string(
            r#"{"type":"error","error":{"type":"rate_limit_error","message":"too many requests"}}"#,
        ))
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new(Some(server.uri()));
    let request = make_request(CancellationToken::new());
    let mut stream =
        provider.stream_prepared(request.clone(), opi_ai::test_support::resolved_auth());

    let result = stream.next().await.expect("should have event");
    assert!(
        matches!(
            result
                .as_ref()
                .map_err(|error| accepted_failure_cause(error, &request)),
            Err(ProviderError::RateLimited {
                retry_after_ms: None
            })
        ),
        "expected RateLimited, got: {result:?}"
    );
    assert!(stream.next().await.is_none(), "failure must be terminal");
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn stream_http_500_maps_to_provider_side() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(500).set_body_string(
            r#"{"type":"error","error":{"type":"api_error","message":"internal"}}"#,
        ))
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new(Some(server.uri()));
    let request = make_request(CancellationToken::new());
    let mut stream =
        provider.stream_prepared(request.clone(), opi_ai::test_support::resolved_auth());

    let result = stream.next().await.expect("should have event");
    match result
        .as_ref()
        .map_err(|error| accepted_failure_cause(error, &request))
    {
        Err(ProviderError::ProviderSide(msg)) => {
            assert!(msg.contains("HTTP 500"), "should mention status: {msg}");
        }
        other => panic!("expected ProviderSide, got: {other:?}"),
    }
    assert!(stream.next().await.is_none(), "failure must be terminal");
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}
