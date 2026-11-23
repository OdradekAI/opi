//! Shared request construction, captured wire values, and accepted failure facts.

use std::time::Duration;

use futures_util::StreamExt;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Request, ThinkingConfig};
use tokio_util::sync::CancellationToken;

pub(super) fn make_anthropic_request(
    cancel: CancellationToken,
    timeout: Option<Duration>,
) -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text { text: "hi".into() }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

pub(super) fn make_openai_chat_request(
    cancel: CancellationToken,
    session_id: Option<String>,
) -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text { text: "hi".into() }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

pub(super) async fn drain(mut stream: opi_ai::provider::EventStream) {
    while stream.next().await.is_some() {}
}

pub(super) fn header_value<'a>(request: &'a wiremock::Request, name: &str) -> Option<&'a str> {
    request
        .headers
        .get(name)
        .and_then(|value| value.to_str().ok())
}

pub(super) fn request_body_json(request: &wiremock::Request) -> serde_json::Value {
    serde_json::from_slice(&request.body).expect("captured request body should be JSON")
}

pub(super) fn accepted_failure_cause<'a>(
    error: &'a opi_ai::provider::ProviderError,
    request: &opi_ai::provider::Request,
) -> &'a opi_ai::provider::ProviderError {
    use opi_ai::provider::ProviderError;
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
