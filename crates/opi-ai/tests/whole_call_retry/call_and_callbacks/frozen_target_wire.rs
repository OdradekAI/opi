//! Anthropic wire sends for opt-in retry against the same logical target.

use crate::fixtures::{logical_request, message, transient_policy};
use opi_ai::ApiKind;
use opi_ai::provider::Provider;
use opi_ai::retry_assistant_call;
use opi_ai::stream::StopReason;
use opi_ai::test_support::resolved_auth;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};

const OK_SSE: &str = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg-ok\",\"model\":\"claude-sonnet-4-5-20250514\",\"usage\":{\"input_tokens\":3,\"output_tokens\":0}}}\n\nevent: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\"}}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"recovered\"}}\n\nevent: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\nevent: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":4}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";

async fn mount(server: &MockServer) {
    // First logical call fails 503 (transient), second succeeds.
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/v1/messages"))
        .respond_with(ResponseTemplate::new(503).set_body_string(
            "{\"type\":\"error\",\"error\":{\"type\":\"api_error\",\"message\":\"overloaded\"}}",
        ))
        .up_to_n_times(1)
        .mount(server)
        .await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(OK_SSE)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(server)
        .await;
}

#[tokio::test]
async fn helper_retries_with_new_logical_calls_against_a_frozen_target() {
    let server = MockServer::start().await;
    mount(&server).await;

    let provider = std::sync::Arc::new(opi_ai::providers::anthropic::AnthropicProvider::new(Some(
        server.uri(),
    )));
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let calls_for_factory = calls.clone();

    let result = retry_assistant_call(
        move || {
            let provider = provider.clone();
            calls_for_factory.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async move {
                // Every invocation prepares a NEW logical call against the
                // same frozen target (route/model/auth never change).
                let events: Vec<_> = futures_util::stream::StreamExt::collect(
                    provider.stream_prepared(logical_request(), resolved_auth()),
                )
                .await;
                let done = events.iter().rev().find_map(|item| {
                    item.as_ref().ok().and_then(|event| {
                        if let opi_ai::stream::AssistantStreamEvent::Done { message, .. } = event {
                            Some(message.clone())
                        } else {
                            None
                        }
                    })
                });
                if let Some(message) = done {
                    return message;
                }
                let detail = events
                    .iter()
                    .find_map(|item| item.as_ref().err().map(|error| error.to_string()))
                    .unwrap_or_else(|| "unknown failure".into());
                let mut failed = message(StopReason::Error, None);
                failed.error_message = Some(detail);
                failed
            }
        },
        Some(&transient_policy(2, 1)),
        tokio_util::sync::CancellationToken::new(),
        None,
    )
    .await
    .unwrap();

    assert_eq!(result.stop_reason, StopReason::Stop);
    assert_eq!(
        calls.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "the retry is a second logical call, not an in-call attempt"
    );
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2, "exactly two dispatches");
    assert_eq!(
        requests[0].url, requests[1].url,
        "the frozen target never moves"
    );

    let _ = ApiKind::Anthropic;
}

#[tokio::test]
async fn without_a_policy_no_recovery_happens_on_the_wire() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/v1/messages"))
        .respond_with(ResponseTemplate::new(503).set_body_string(
            "{\"type\":\"error\",\"error\":{\"type\":\"api_error\",\"message\":\"overloaded\"}}",
        ))
        .mount(&server)
        .await;

    let provider = std::sync::Arc::new(opi_ai::providers::anthropic::AnthropicProvider::new(Some(
        server.uri(),
    )));

    let result = retry_assistant_call(
        move || {
            let provider = provider.clone();
            async move {
                let events: Vec<_> = futures_util::stream::StreamExt::collect(
                    provider.stream_prepared(logical_request(), resolved_auth()),
                )
                .await;
                let done = events.iter().rev().find_map(|item| {
                    item.as_ref().ok().and_then(|event| {
                        if let opi_ai::stream::AssistantStreamEvent::Done { message, .. } = event {
                            Some(message.clone())
                        } else {
                            None
                        }
                    })
                });
                if let Some(message) = done {
                    return message;
                }
                let detail = events
                    .iter()
                    .find_map(|item| item.as_ref().err().map(|error| error.to_string()))
                    .unwrap_or_else(|| "unknown failure".into());
                let mut failed = message(StopReason::Error, None);
                failed.error_message = Some(detail);
                failed
            }
        },
        None,
        tokio_util::sync::CancellationToken::new(),
        None,
    )
    .await
    .unwrap();

    assert_eq!(result.stop_reason, StopReason::Error);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "no automatic recovery without opt-in");
}
