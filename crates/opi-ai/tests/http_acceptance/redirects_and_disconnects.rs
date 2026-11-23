//! Redirect send rules and failures before or after HTTP response acquisition.

use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::http::{HttpClient, HttpClientBuilder, RedirectPolicy};
use opi_ai::provider::{EventStream, ProviderError};
use tokio::net::TcpListener;

use super::fixtures::{
    RecordedRequest, Script, accepted_failure_cause, bearer_of, chat_call, chat_error,
    chat_request, chunk, is_post_acceptance_stream_failure, plain_client, registered_models,
    sse_chunked_response, sse_head, start_raw,
};

/// One SSE data event without the terminal DONE (for partial-body cases).
const SSE_PARTIAL: &str = concat!(
    "data: {\"id\":\"chatcmpl-acc\",\"object\":\"chat.completion.chunk\",\"created\":1,",
    "\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"partial\"},",
    "\"finish_reason\":null}]}\n\n",
);

fn redirect_response(status: u16, location: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status} Moved\r\nlocation: {location}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
    )
    .into_bytes()
}

#[test]
fn redirect_policy_defaults_to_limited_twenty_and_is_configurable() {
    assert_eq!(
        HttpClient::new().redirect_policy(),
        RedirectPolicy::Limited(20)
    );
    assert_eq!(
        RedirectPolicy::default(),
        RedirectPolicy::Limited(20),
        "the source-qualified default must not silently track the backend"
    );
    let client = HttpClientBuilder::new()
        .redirects(RedirectPolicy::None)
        .build()
        .unwrap();
    assert_eq!(client.redirect_policy(), RedirectPolicy::None);
    let client = HttpClientBuilder::new()
        .redirects(RedirectPolicy::Limited(2))
        .build()
        .unwrap();
    assert_eq!(client.redirect_policy(), RedirectPolicy::Limited(2));
}

#[tokio::test]
async fn source_qualified_default_redirect_budget_has_exact_twenty_hop_boundary() {
    // The explicit source budget controls distinguish peer/fixture failures
    // from the default client selecting a different redirect policy.
    for explicit in [true, false] {
        for hops in [20, 21] {
            let server = start_raw(Arc::new(move |index: usize, _: &RecordedRequest| {
                if index < hops {
                    Script::coalesced(redirect_response(307, &format!("/hop{}", index + 1)))
                } else {
                    Script::coalesced(sse_chunked_response())
                }
            }))
            .await;
            let client = if explicit {
                plain_client(RedirectPolicy::Limited(20))
            } else {
                Arc::new(HttpClient::new())
            };
            if hops == 20 {
                let message = tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    chat_call(&server.url, client),
                )
                .await
                .expect("finite redirect chain must settle")
                .unwrap_or_else(|error| {
                    panic!("20 redirects must succeed; explicit={explicit}: {error:?}")
                });
                assert_eq!(message.response_id.as_deref(), Some("chatcmpl-acc"));
            } else {
                let error = tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    chat_error(&server.url, client),
                )
                .await
                .expect("redirect budget must terminate the chain");
                let ProviderError::Network(summary) =
                    accepted_failure_cause(&error, &chat_request())
                else {
                    panic!("redirect refusal must preserve its typed cause: {error:?}")
                };
                assert_eq!(summary.as_str(), "too many redirects");
            }
            let requests = server.stats.requests();
            assert_eq!(requests.len(), 21, "hops={hops}, explicit={explicit}");
            let body = &requests[0].body;
            assert!(!body.is_empty());
            let parsed: serde_json::Value = serde_json::from_slice(body).unwrap();
            assert_eq!(parsed["model"], "gpt-4o");
            for (index, request) in requests.iter().enumerate() {
                assert_eq!(request.method, "POST");
                assert_eq!(request.content_length, body.len());
                assert_eq!(&request.body, body, "307 must retain exact body bytes");
                assert_eq!(bearer_of(request), Some("Bearer acceptance-contract-key"));
                let target = if index == 0 {
                    "/v1/chat/completions".into()
                } else {
                    format!("/hop{index}")
                };
                assert_eq!(request.target, target);
            }
            server.stop().await;
        }
    }
}

#[tokio::test]
async fn same_origin_redirect_keeps_authorization_with_exact_sends() {
    let server = start_raw(Arc::new(|index: usize, request: &RecordedRequest| {
        if index == 0 {
            assert_eq!(request.method, "POST", "original request is the POST");
            Script::coalesced(redirect_response(307, "/final"))
        } else {
            Script::coalesced(sse_chunked_response())
        }
    }))
    .await;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_call(&server.url, plain_client(RedirectPolicy::default())),
    )
    .await
    .expect("same-origin redirect completes")
    .expect("same-origin redirect round trip");
    assert_eq!(result.response_id.as_deref(), Some("chatcmpl-acc"));
    let requests = server.stats.requests();
    assert_eq!(
        requests.len(),
        2,
        "exactly the original plus one hop: {requests:?}"
    );
    assert_eq!(requests[0].target, "/v1/chat/completions");
    assert_eq!(
        requests[1].target, "/final",
        "the hop follows the Location target"
    );
    for request in &requests {
        assert_eq!(request.referer, None, "redirects do not invent a Referer");
        assert_eq!(
            bearer_of(request),
            Some("Bearer acceptance-contract-key"),
            "same-origin redirect must preserve the protected authorization header"
        );
        assert_eq!(
            request.content_length, 130,
            "307 preserves the POST method and body across the hop"
        );
    }
    server.stop().await;
}

#[tokio::test]
async fn cross_origin_redirect_strips_protected_authorization() {
    let origin = start_raw(Arc::new(|_: usize, _: &RecordedRequest| {
        Script::coalesced(redirect_response(307, "@@TARGET@@/v1/chat/completions"))
    }))
    .await;
    let target = start_raw(Arc::new(|_: usize, _: &RecordedRequest| {
        Script::coalesced(sse_chunked_response())
    }))
    .await;
    let redirect = redirect_response(307, &format!("{}/v1/chat/completions", target.url));
    let origin_server = origin;
    // Re-point the origin responder at the real target port by serving a
    // second scripted server whose bytes embed the target URL.
    origin_server.stop().await;
    let origin = start_raw(Arc::new(move |_: usize, _: &RecordedRequest| {
        Script::coalesced(redirect.clone())
    }))
    .await;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_call(&origin.url, plain_client(RedirectPolicy::default())),
    )
    .await
    .expect("cross-origin redirect completes")
    .expect("cross-origin redirect round trip");
    assert_eq!(result.response_id.as_deref(), Some("chatcmpl-acc"));
    let origin_requests = origin.stats.requests();
    let target_requests = target.stats.requests();
    assert_eq!(origin_requests.len(), 1);
    assert_eq!(target_requests.len(), 1);
    assert_eq!(origin_requests[0].referer, None);
    assert_eq!(target_requests[0].referer, None);
    assert_eq!(
        bearer_of(&origin_requests[0]),
        Some("Bearer acceptance-contract-key"),
        "the original request carries the authorization header"
    );
    assert_eq!(
        bearer_of(&target_requests[0]),
        None,
        "cross-origin redirect must strip the protected authorization header"
    );
    origin.stop().await;
    target.stop().await;
}

#[tokio::test]
async fn redirect_limit_enforced_with_exact_lawful_sends_and_typed_cause() {
    let server = start_raw(Arc::new(|_: usize, _: &RecordedRequest| {
        Script::coalesced(redirect_response(307, "/loop"))
    }))
    .await;
    let error = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_error(&server.url, plain_client(RedirectPolicy::Limited(2))),
    )
    .await
    .expect("redirect loop terminates");
    match accepted_failure_cause(&error, &chat_request()) {
        ProviderError::Network(summary) => {
            assert_eq!(
                summary.as_str(),
                "too many redirects",
                "typed redacted cause, no redirect target or URL leaks"
            );
        }
        other => panic!("expected Network, got {other:?}"),
    }
    let requests = server.stats.requests();
    assert_eq!(
        requests.len(),
        3,
        "1 original + 2 lawful hops, then refusal: {requests:?}"
    );
    server.stop().await;
}

#[tokio::test]
async fn redirect_none_returns_status_to_caller_with_single_send() {
    let server = start_raw(Arc::new(|_: usize, _: &RecordedRequest| {
        Script::coalesced(redirect_response(307, "/elsewhere"))
    }))
    .await;
    let error = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_error(&server.url, plain_client(RedirectPolicy::None)),
    )
    .await
    .expect("no-redirect policy returns the status");
    match accepted_failure_cause(&error, &chat_request()) {
        ProviderError::ProviderSide(summary) => {
            let text = summary.as_str();
            assert!(
                text.contains("307") || text.contains("3"),
                "status surfaces: {text}"
            );
        }
        other => panic!("expected ProviderSide, got {other:?}"),
    }
    assert_eq!(server.stats.count(), 1, "exactly one send, no follow");
    server.stop().await;
}

// ---------------------------------------------------------------------------
// Acquisition failure versus failure after accepted partial usage
// ---------------------------------------------------------------------------

#[tokio::test]
async fn acquisition_failure_before_any_request_is_typed_network() {
    // Reserve a port and close the listener so the target is unreachable.
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("reserve port");
    let addr = listener.local_addr().expect("reserved address");
    drop(listener);
    let error = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_error(
            &format!("http://{addr}"),
            plain_client(RedirectPolicy::default()),
        ),
    )
    .await
    .expect("acquisition failure terminates");
    assert!(
        matches!(
            accepted_failure_cause(&error, &chat_request()),
            ProviderError::Network(_)
        ),
        "acquisition-stage failure must stay typed Network: {error:?}"
    );
}

#[tokio::test]
async fn server_close_after_accepted_head_is_single_terminal_without_retry() {
    // Complete chunked body without the terminal SSE DONE: the response was
    // accepted (200 head + a content event), so the failure is a
    // post-acquisition stream error with one terminal and no retry.
    let body = format!(
        "{}{}\r\n0\r\n\r\n",
        chunk(SSE_PARTIAL.as_bytes()),
        SSE_PARTIAL
    );
    let mut script = sse_head(&[]);
    script.extend_from_slice(body.as_bytes());
    let server = start_raw(Arc::new(move |_: usize, _: &RecordedRequest| {
        Script::coalesced(script.clone())
    }))
    .await;
    let models = registered_models(plain_client(RedirectPolicy::default()), &server.url);
    let prepared = models
        .prepare_call("openai:gpt-4o", chat_request())
        .await
        .expect("prepare");
    let mut stream: EventStream = prepared.start_attempt().expect("attempt");
    let mut terminals = 0usize;
    let mut saw_error = false;
    let drained = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(item) = stream.next().await {
            match item {
                Ok(event) => assert!(!event.is_terminal(), "no success terminal expected"),
                Err(error) => {
                    saw_error = true;
                    assert!(
                        is_post_acceptance_stream_failure(&error),
                        "post-acceptance EOF must be typed StreamError: {error:?}"
                    );
                    terminals += 1;
                }
            }
        }
    })
    .await;
    assert!(drained.is_ok(), "stream reaches EOF");
    assert!(
        saw_error,
        "exactly one error terminal for the permanent EOF"
    );
    assert_eq!(terminals, 1);
    assert_eq!(
        server.stats.count(),
        1,
        "no hidden retry or extra send after EOF"
    );
    server.stop().await;
}

#[tokio::test]
async fn mid_body_abort_after_partial_events_is_stage_provable() {
    // Write the head plus half a chunk frame, then hard-close: the response
    // was accepted, partial content was delivered, and the truncated body
    // fails as a stream error rather than an acquisition failure.
    let body = format!("{}{}", chunk(SSE_PARTIAL.as_bytes()), SSE_PARTIAL);
    let mut script = sse_head(&[]);
    script.extend_from_slice(body.as_bytes());
    script.extend_from_slice(b"5\r\npar"); // truncated follow-up chunk
    let server = start_raw(Arc::new(move |_: usize, _: &RecordedRequest| {
        Script::coalesced(script.clone())
    }))
    .await;
    let error = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        chat_error(&server.url, plain_client(RedirectPolicy::default())),
    )
    .await
    .expect("mid-body abort terminates");
    assert!(
        is_post_acceptance_stream_failure(&error),
        "post-acceptance truncation must be typed StreamError: {error:?}"
    );
    assert_eq!(server.stats.count(), 1, "no retry after the aborted stream");
    server.stop().await;
}
