//! Explicit whole-call retry helper (AIR-065/V-57, task 20.52).
//!
//! The helper is a standalone policy loop: it consumes message-shaped
//! results from a caller-owned `produce` closure (each invocation a NEW
//! logical call), classifies failures against the pinned source pattern
//! lists (quota/billing/usage-limit vetoes first), and sleeps a
//! cancellable exponential backoff between calls. It never wires product
//! recovery, mutates transcripts, re-executes tools or resets prepared
//! call budgets; omission/disability means exactly one call.

use opi_ai::ApiKind;
use opi_ai::message::{AssistantContent, AssistantMessage};
use opi_ai::retry::{
    DEFAULT_WHOLE_CALL_RETRY_MAX_DELAY_MS, RetryCallbackFailure, WholeCallRetryCallbacks,
    WholeCallRetryClassification, WholeCallRetryPolicy,
};
use opi_ai::retry_assistant_call;
use opi_ai::stream::{StopReason, Usage};

fn message(stop: StopReason, error: Option<&str>) -> AssistantMessage {
    AssistantMessage {
        content: vec![AssistantContent::Text {
            text: "partial".into(),
        }],
        api: ApiKind::Anthropic,
        provider: "anthropic".into(),
        model: "claude-sonnet-4-5-20250514".into(),
        response_model: None,
        wire: None,
        response_id: None,
        usage: Usage {
            input_tokens: 3,
            output_tokens: 5,
            ..Usage::default()
        },
        stop_reason: stop,
        error_message: error.map(str::to_string),
        timestamp_ms: 1_000,
        diagnostics: Vec::new(),
        duration_ms: None,
    }
}

fn transient_policy(max_retries: u32, base_delay_ms: u64) -> WholeCallRetryPolicy {
    WholeCallRetryPolicy {
        enabled: true,
        max_retries,
        base_delay_ms,
        max_delay_ms: None,
    }
}

/// Shared sink for finished-callback assertions.
type Finals = std::sync::Arc<std::sync::Mutex<Vec<(bool, u32, Option<String>)>>>;

fn error_text(stop: StopReason) -> Option<&'static str> {
    match stop {
        StopReason::Error => Some("overloaded_error"),
        _ => None,
    }
}

#[test]
fn classification_vetoes_quota_billing_and_usage_limits_first() {
    let vetoed = [
        "GoUsageLimitError: weekly cap",
        "FreeUsageLimitError hit",
        "Monthly usage limit reached, enable spending",
        "insufficient_quota: you exceeded your quota",
        "out of budget",
        "quota exceeded for this key",
        "billing hard limit reached",
        "subscription_sharing_usage_limit_exceeded",
        // A transient-looking token inside a vetoed message stays vetoed.
        "insufficient_quota despite the 429",
    ];
    for text in vetoed {
        let classified =
            opi_ai::retry::classify_whole_call_retry(&message(StopReason::Error, Some(text)));
        assert_eq!(
            classified,
            WholeCallRetryClassification::Terminal,
            "must veto first: {text}"
        );
    }
}

#[test]
fn classification_flags_source_transient_families() {
    let transient = [
        "The server is overloaded",
        "server_busy",
        "servers are currently busy",
        "currently experiencing high demand",
        "model is at capacity",
        "rate limit exceeded",
        "Too Many Requests",
        "HTTP 429",
        "HTTP 500",
        "502 Bad Gateway",
        "503 Service Unavailable",
        "504 Gateway Timeout",
        "520",
        "524",
        "The service is temporarily unavailable",
        "service unavailable",
        "server error",
        "internal error",
        "Provider returned error",
        "The upstream provider returned an error",
        "unable to forward upstream",
        "exceeded request buffer limit while retrying upstream",
        "network error",
        "connection error",
        "connection refused",
        "connection lost",
        "other side closed",
        "fetch failed",
        "getaddrinfo ENOTFOUND",
        "EAI_AGAIN",
        "upstream connect error",
        "reset before headers",
        "socket hang up",
        "socket connection was closed",
        "request timed out",
        "timeout while waiting",
        "terminated",
        "websocket closed",
        "websocket error",
        "stream ended without a message_stop",
        "stream ended before message_stop",
        "stream ended before a terminal response event",
        "http2 request did not get a response",
        "pending stream has been canceled",
        "retry delay exceeded the maximum",
        "You can retry your request",
        "Please try your request again after 30s",
        "ResourceExhausted",
        "subscription_sharing_usage_unavailable",
        "subscription_sharing_user_unavailable",
    ];
    for text in transient {
        let classified =
            opi_ai::retry::classify_whole_call_retry(&message(StopReason::Error, Some(text)));
        assert_eq!(
            classified,
            WholeCallRetryClassification::Transient,
            "must be transient: {text}"
        );
    }
}

#[test]
fn classification_marks_valid_stops_aborted_and_bare_errors() {
    for stop in [StopReason::Stop, StopReason::Length, StopReason::ToolUse] {
        assert_eq!(
            opi_ai::retry::classify_whole_call_retry(&message(stop, None)),
            WholeCallRetryClassification::Valid
        );
    }
    assert_eq!(
        opi_ai::retry::classify_whole_call_retry(&message(StopReason::Aborted, None)),
        WholeCallRetryClassification::Aborted
    );
    assert_eq!(
        opi_ai::retry::classify_whole_call_retry(&message(StopReason::Error, None)),
        WholeCallRetryClassification::Terminal
    );
    // Deterministic errors fail fast.
    assert_eq!(
        opi_ai::retry::classify_whole_call_retry(&message(
            StopReason::Error,
            Some("no such model: unknown-model-id")
        )),
        WholeCallRetryClassification::Terminal
    );
}

#[test]
fn delay_grows_exponentially_and_caps_at_the_default_60s() {
    let policy = transient_policy(8, 2_000);
    assert_eq!(opi_ai::retry::whole_call_retry_delay_ms(&policy, 1), 2_000);
    assert_eq!(opi_ai::retry::whole_call_retry_delay_ms(&policy, 2), 4_000);
    assert_eq!(opi_ai::retry::whole_call_retry_delay_ms(&policy, 3), 8_000);
    assert_eq!(opi_ai::retry::whole_call_retry_delay_ms(&policy, 4), 16_000);
    // Explicit caller caps replace the default.
    let capped = WholeCallRetryPolicy {
        max_delay_ms: Some(5_000),
        ..transient_policy(8, 30_000)
    };
    assert_eq!(opi_ai::retry::whole_call_retry_delay_ms(&capped, 2), 5_000);
    // Huge exponents saturate at the cap, never overflowing.
    let huge = transient_policy(8, u64::MAX / 2);
    assert_eq!(
        opi_ai::retry::whole_call_retry_delay_ms(&huge, 5),
        DEFAULT_WHOLE_CALL_RETRY_MAX_DELAY_MS
    );
}

#[tokio::test]
async fn omitted_or_disabled_policy_makes_exactly_one_call() {
    let mut calls = 0u32;
    let failing = || {
        calls += 1;
        std::future::ready(message(StopReason::Error, Some("HTTP 503")))
    };
    let first = retry_assistant_call(
        failing,
        None,
        tokio_util::sync::CancellationToken::new(),
        None,
    )
    .await
    .expect("helper never fails without callback failures");
    assert_eq!(calls, 1);
    assert_eq!(first.stop_reason, StopReason::Error);

    let disabled = WholeCallRetryPolicy {
        enabled: false,
        ..transient_policy(9, 1)
    };
    let second = retry_assistant_call(
        || {
            calls += 1;
            std::future::ready(message(StopReason::Error, Some("HTTP 503")))
        },
        Some(&disabled),
        tokio_util::sync::CancellationToken::new(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(calls, 2, "disabled must not add calls");
    assert_eq!(second.stop_reason, StopReason::Error);
}

#[tokio::test]
async fn transient_retry_succeeds_with_exact_callback_order_and_counts() {
    let policy = transient_policy(2, 1);
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let events: std::sync::Arc<std::sync::Mutex<Vec<String>>> = Default::default();

    let calls_for_produce = calls.clone();
    let events_for_scheduled = events.clone();
    let events_for_start = events.clone();
    let events_for_finished = events.clone();

    let mut callbacks = WholeCallRetryCallbacks::new();
    callbacks.on_retry_scheduled = Some(Box::new(
        move |attempt: u32, max: u32, delay: u64, error: &str| {
            events_for_scheduled
                .lock()
                .unwrap()
                .push(format!("scheduled({attempt},{max},{delay},{error})"));
            Ok(())
        },
    ));
    callbacks.on_retry_attempt_start = Some(Box::new(move || {
        events_for_start
            .lock()
            .unwrap()
            .push("attempt_start".into());
        Ok(())
    }));
    callbacks.on_retry_finished = Some(Box::new(move |success: bool, attempt: u32, error| {
        events_for_finished.lock().unwrap().push(format!(
            "finished({success},{attempt},{})",
            error.unwrap_or("-")
        ));
        Ok(())
    }));

    let result = retry_assistant_call(
        move || {
            let n = calls_for_produce.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let stop = if n == 0 {
                StopReason::Error
            } else {
                StopReason::Stop
            };
            std::future::ready(message(stop, error_text(stop)))
        },
        Some(&policy),
        tokio_util::sync::CancellationToken::new(),
        Some(callbacks),
    )
    .await
    .unwrap();

    assert_eq!(result.stop_reason, StopReason::Stop);
    assert_eq!(
        calls.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "one extra whole call only"
    );
    assert_eq!(
        *events.lock().unwrap(),
        vec![
            "scheduled(1,2,1,overloaded_error)".to_string(),
            "attempt_start".to_string(),
            "finished(true,1,-)".to_string(),
        ],
        "scheduled -> sleep -> attemptStart -> produce, finished exactly once"
    );
}

#[tokio::test]
async fn quota_veto_is_terminal_without_any_retry_scheduling() {
    let policy = transient_policy(3, 1);
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let finished = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let calls_for_produce = calls.clone();
    let finished_sink = finished.clone();

    let mut callbacks = WholeCallRetryCallbacks::new();
    callbacks.on_retry_scheduled = Some(Box::new(|_, _, _, _| {
        panic!("quota vetoes must not schedule a retry")
    }));
    callbacks.on_retry_finished = Some(Box::new(move |_, _, _| {
        finished_sink.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }));

    let result = retry_assistant_call(
        move || {
            calls_for_produce.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            std::future::ready(message(
                StopReason::Error,
                Some("insufficient_quota: billing limit"),
            ))
        },
        Some(&policy),
        tokio_util::sync::CancellationToken::new(),
        Some(callbacks),
    )
    .await
    .unwrap();

    assert_eq!(result.stop_reason, StopReason::Error);
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(
        finished.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "finished fires only when a retry was scheduled"
    );
}

#[tokio::test]
async fn exhausted_budget_returns_final_error_message() {
    let policy = transient_policy(2, 1);
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let finals: Finals = Default::default();
    let calls_for_produce = calls.clone();
    let finals_sink = finals.clone();

    let mut callbacks = WholeCallRetryCallbacks::new();
    callbacks.on_retry_finished = Some(Box::new(move |success, attempt, error| {
        finals_sink
            .lock()
            .unwrap()
            .push((success, attempt, error.map(str::to_string)));
        Ok(())
    }));

    let result = retry_assistant_call(
        move || {
            let n = calls_for_produce.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let text = if n < 2 {
                "HTTP 503"
            } else {
                "insufficient_quota after retries"
            };
            std::future::ready(message(StopReason::Error, Some(text)))
        },
        Some(&policy),
        tokio_util::sync::CancellationToken::new(),
        Some(callbacks),
    )
    .await
    .unwrap();

    assert_eq!(
        calls.load(std::sync::atomic::Ordering::SeqCst),
        3,
        "initial call + max_retries extra calls"
    );
    assert_eq!(
        result.error_message.as_deref(),
        Some("insufficient_quota after retries")
    );
    assert_eq!(
        *finals.lock().unwrap(),
        vec![(false, 2, Some("insufficient_quota after retries".into()))]
    );
}

#[tokio::test]
async fn aborted_after_a_scheduled_retry_returns_without_further_calls() {
    let policy = transient_policy(3, 1);
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let finals: Finals = Default::default();
    let calls_for_produce = calls.clone();
    let finals_sink = finals.clone();

    let mut callbacks = WholeCallRetryCallbacks::new();
    callbacks.on_retry_finished = Some(Box::new(move |success, attempt, error| {
        finals_sink
            .lock()
            .unwrap()
            .push((success, attempt, error.map(str::to_string)));
        Ok(())
    }));

    let result = retry_assistant_call(
        move || {
            let n = calls_for_produce.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let stop = if n == 0 {
                StopReason::Error
            } else {
                StopReason::Aborted
            };
            std::future::ready(message(stop, error_text(stop)))
        },
        Some(&policy),
        tokio_util::sync::CancellationToken::new(),
        Some(callbacks),
    )
    .await
    .unwrap();

    assert_eq!(result.stop_reason, StopReason::Aborted);
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(*finals.lock().unwrap(), vec![(false, 1, None)]);
}

#[tokio::test]
async fn cancelled_backoff_normalizes_to_aborted_preserving_partial_and_usage() {
    let policy = transient_policy(3, 120);
    let cancel = tokio_util::sync::CancellationToken::new();
    let cancel_for_loop = cancel.clone();
    let finished: Finals = Default::default();
    let finished_sink = finished.clone();

    let mut callbacks = WholeCallRetryCallbacks::new();
    callbacks.on_retry_scheduled = Some(Box::new(move |_, _, _, _| {
        cancel_for_loop.cancel();
        Ok(())
    }));
    callbacks.on_retry_finished = Some(Box::new(move |success, attempt, error| {
        finished_sink
            .lock()
            .unwrap()
            .push((success, attempt, error.map(str::to_string)));
        Ok(())
    }));

    let calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let calls_for_produce = calls.clone();
    let result = retry_assistant_call(
        move || {
            calls_for_produce.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            std::future::ready(message(
                StopReason::Error,
                Some("connection refused while streaming"),
            ))
        },
        Some(&policy),
        cancel,
        Some(callbacks),
    )
    .await
    .unwrap();

    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(result.stop_reason, StopReason::Aborted);
    assert_eq!(result.error_message, None, "aborted drops the error text");
    assert_eq!(result.usage.input_tokens, 3, "usage is preserved");
    assert_eq!(
        result.content,
        vec![AssistantContent::Text {
            text: "partial".into()
        }],
        "partial output is preserved"
    );
    assert_eq!(
        *finished.lock().unwrap(),
        vec![(false, 1, Some("connection refused while streaming".into()))]
    );
}

#[tokio::test]
async fn callback_failure_returns_explicitly_and_stops_the_loop() {
    let policy = transient_policy(3, 1);
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let calls_for_produce = calls.clone();

    let mut callbacks = WholeCallRetryCallbacks::new();
    callbacks.on_retry_scheduled = Some(Box::new(|_, _, _, _| {
        Err(RetryCallbackFailure {
            detail: "observer channel closed".into(),
        })
    }));

    let error = retry_assistant_call(
        move || {
            calls_for_produce.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            std::future::ready(message(StopReason::Error, Some("HTTP 503")))
        },
        Some(&policy),
        tokio_util::sync::CancellationToken::new(),
        Some(callbacks),
    )
    .await
    .expect_err("callback failure must surface");

    assert!(
        matches!(error, opi_ai::retry::WholeCallRetryError::Callback(ref failure)
            if failure.detail == "observer channel closed"),
        "typed callback failure, got {error:?}"
    );
    assert_eq!(
        calls.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "no further calls after a callback failure"
    );
}

// --- controlled frozen-target factory over a loopback wire (AIR-065) ---

mod frozen_target_wire {
    use super::*;
    use opi_ai::message::{Message, UserMessage};
    use opi_ai::provider::{CacheRetention, Provider, Request};
    use opi_ai::test_support::resolved_auth;
    use opi_ai::transcript;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers};

    const OK_SSE: &str = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg-ok\",\"model\":\"claude-sonnet-4-5-20250514\",\"usage\":{\"input_tokens\":3,\"output_tokens\":0}}}\n\nevent: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\"}}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"recovered\"}}\n\nevent: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\nevent: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":4}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";

    pub(super) fn logical_request() -> Request {
        Request {
            transport: Default::default(),
            model: "anthropic:claude-sonnet-4-5-20250514".into(),
            context: transcript::normalize_context(transcript::Context {
                system_prompt: None,
                messages: vec![Message::User(UserMessage {
                    content: vec![opi_ai::message::InputContent::Text {
                        text: "Hello".into(),
                    }],
                    timestamp_ms: 0,
                })],
                tools: vec![],
            })
            .expect("valid context declarations"),
            max_tokens: Some(64),
            temperature: None,
            sampling_params: Default::default(),
            thinking: opi_ai::provider::ThinkingConfig::default(),
            stop_sequences: vec![],
            metadata: None,
            cancel: tokio_util::sync::CancellationToken::new(),
            timeout: None,
            extra_headers: vec![],
            cache_retention: CacheRetention::None,
            session_id: None,
            telemetry_context: None,
            callbacks: Default::default(),
        }
    }

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

        let provider = std::sync::Arc::new(opi_ai::anthropic::AnthropicProvider::new(Some(
            server.uri(),
        )));
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let calls_for_factory = calls.clone();

        let result = super::retry_assistant_call(
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
                            if let opi_ai::stream::AssistantStreamEvent::Done { message, .. } =
                                event
                            {
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
                    let mut failed = super::message(StopReason::Error, None);
                    failed.error_message = Some(detail);
                    failed
                }
            },
            Some(&super::transient_policy(2, 1)),
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

        let provider = std::sync::Arc::new(opi_ai::anthropic::AnthropicProvider::new(Some(
            server.uri(),
        )));

        let result = super::retry_assistant_call(
            move || {
                let provider = provider.clone();
                async move {
                    let events: Vec<_> = futures_util::stream::StreamExt::collect(
                        provider.stream_prepared(logical_request(), resolved_auth()),
                    )
                    .await;
                    let done = events.iter().rev().find_map(|item| {
                        item.as_ref().ok().and_then(|event| {
                            if let opi_ai::stream::AssistantStreamEvent::Done { message, .. } =
                                event
                            {
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
                    let mut failed = super::message(StopReason::Error, None);
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
}

mod quota_transport {
    use super::*;
    use opi_ai::auth::{AuthProvenanceSource, AuthScheme, StaticAuthResolver};
    use opi_ai::models::{CompatMetadata, Models};
    use opi_ai::provider::{Provider, ProviderError};
    use opi_ai::stream::ChatStream;
    use secrecy::SecretString;
    use std::sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    };
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers};

    async fn exercise(responses: bool, via_models: bool, body: serde_json::Value, expected: u32) {
        let server = MockServer::start().await;
        let endpoint = if responses {
            "/v1/responses"
        } else {
            "/v1/chat/completions"
        };
        Mock::given(matchers::method("POST"))
            .and(matchers::path(endpoint))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("retry-after", "1")
                    .set_body_json(body),
            )
            .mount(&server)
            .await;
        let provider: Box<dyn Provider> = if responses {
            Box::new(opi_ai::openai_responses::OpenAiResponsesProvider::new(
                Some(server.uri()),
            ))
        } else {
            Box::new(opi_ai::openai_chat::OpenAiChatProvider::new(Some(
                server.uri(),
            )))
        };
        let record =
            opi_ai::ChatModelRecord::new(provider.id(), provider.models().first().unwrap().clone());
        let requested_model = provider.models().first().unwrap().id.clone();
        let expected_model = requested_model.clone();
        let mut models = Models::new();
        // Direct and Models consumers use the same frozen local endpoint.
        let direct: Option<Arc<dyn Provider>> = if via_models {
            models
                .register_route(
                    provider,
                    Arc::new(StaticAuthResolver::new(
                        AuthScheme::Bearer,
                        SecretString::from("quota-test-key"),
                    )),
                    AuthProvenanceSource::Static,
                    CompatMetadata {
                        openai_compatible: true,
                        profile: None,
                    },
                )
                .unwrap();
            None
        } else {
            Some(Arc::from(provider))
        };
        let models = Arc::new(models);
        let calls = Arc::new(AtomicU32::new(0));
        let counter = calls.clone();
        let result = retry_assistant_call(
            move || {
                let models = models.clone();
                let direct = direct.clone();
                let record = record.clone();
                let requested_model = requested_model.clone();
                counter.fetch_add(1, Ordering::SeqCst);
                async move {
                    let mut request = super::frozen_target_wire::logical_request();
                    request.model = requested_model;
                    let failure =
                        if let Some(provider) = direct {
                            ChatStream::new(provider.stream_prepared(
                                request,
                                opi_ai::test_support::resolved_bearer_auth(),
                            ))
                            .result()
                            .await
                            .unwrap_err()
                        } else {
                            models
                                .complete_simple(record, request, Default::default())
                                .await
                                .unwrap_err()
                        };
                    assert!(
                        matches!(
                            failure.cause(),
                            ProviderError::RateLimited {
                                retry_after_ms: Some(1000)
                            }
                        ),
                        "typed acquisition cause is preserved: {failure:?}"
                    );
                    failure.into_parts().0
                }
            },
            Some(&transient_policy(1, 0)),
            tokio_util::sync::CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(
            calls.load(Ordering::SeqCst),
            expected,
            "responses={responses} Models={via_models}"
        );
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), expected as usize);
        for request in sent {
            let payload: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            assert_eq!(payload["model"], expected_model);
            assert!(request.body.windows(5).any(|bytes| bytes == b"Hello"));
        }
        assert_eq!(result.stop_reason, StopReason::Error);
        assert!(result.content.is_empty());
        assert!(!format!("{result:?}").contains("PRIVATE_CANARY"));
    }

    #[tokio::test]
    async fn quota_429_veto_reaches_whole_call_retry_through_real_adapters() {
        for responses in [false, true] {
            for via_models in [false, true] {
                for error in [
                    serde_json::json!({"code":"insufficient_quota", "message":"PRIVATE_CANARY"}),
                    serde_json::json!({"type":"FreeUsageLimitError", "message":"PRIVATE_CANARY"}),
                    serde_json::json!({"message":"billing limit reached PRIVATE_CANARY"}),
                ] {
                    exercise(responses, via_models, serde_json::json!({"error":error}), 1).await;
                }
            }
        }
    }

    #[tokio::test]
    async fn transient_429_keeps_explicit_whole_call_retry() {
        for responses in [false, true] {
            for via_models in [false, true] {
                exercise(responses, via_models, serde_json::json!({"error":{"code":"rate_limit_exceeded","message":"PRIVATE_CANARY transient"}}), 2).await;
            }
        }
    }

    #[tokio::test]
    async fn unrelated_quota_text_does_not_override_status_classification() {
        for responses in [false, true] {
            exercise(responses, true, serde_json::json!({"metadata":{"message":"billing PRIVATE_CANARY"},"error":{"code":429}}), 2).await;
            exercise(responses, true, serde_json::Value::Null, 2).await;
        }
    }

    #[tokio::test]
    async fn quota_detail_does_not_change_acquisition_budget_or_payload_freeze() {
        for responses in [false, true] {
            for max_retries in [0, 1] {
                let server = MockServer::start().await;
                Mock::given(matchers::method("POST"))
                    .respond_with(
                        ResponseTemplate::new(429)
                            .insert_header("retry-after", "0")
                            .set_body_json(
                                serde_json::json!({"error":{"code":"insufficient_quota"}}),
                            ),
                    )
                    .mount(&server)
                    .await;
                let provider: Box<dyn Provider> = if responses {
                    Box::new(opi_ai::openai_responses::OpenAiResponsesProvider::new(
                        Some(server.uri()),
                    ))
                } else {
                    Box::new(opi_ai::openai_chat::OpenAiChatProvider::new(Some(
                        server.uri(),
                    )))
                };
                let target = format!("{}:{}", provider.id(), provider.models()[0].id);
                let mut models = Models::new();
                models
                    .register_route(
                        provider,
                        Arc::new(StaticAuthResolver::new(
                            AuthScheme::Bearer,
                            SecretString::from("quota-test-key"),
                        )),
                        AuthProvenanceSource::Static,
                        CompatMetadata {
                            openai_compatible: true,
                            profile: None,
                        },
                    )
                    .unwrap();
                let payload_calls = Arc::new(AtomicU32::new(0));
                let response_calls = Arc::new(AtomicU32::new(0));
                let payload_sink = payload_calls.clone();
                let response_sink = response_calls.clone();
                let mut request = super::frozen_target_wire::logical_request();
                request.model = target.clone();
                request.callbacks = opi_ai::callbacks::RequestCallbacks::new()
                    .with_on_payload(Arc::new(move |_| {
                        payload_sink.fetch_add(1, Ordering::SeqCst);
                        Ok(None)
                    }))
                    .with_on_response(Arc::new(move |_, _| {
                        response_sink.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    }));
                let call = models
                    .prepare_call_with_policy(
                        &target,
                        request,
                        opi_ai::retry::AcquisitionPolicy::with_max_retries(max_retries),
                    )
                    .await
                    .unwrap();
                for attempt in 0..=max_retries {
                    let failure = ChatStream::new(call.start_attempt().unwrap())
                        .result()
                        .await
                        .unwrap_err();
                    assert!(matches!(
                        failure.cause(),
                        ProviderError::RateLimited {
                            retry_after_ms: Some(0)
                        }
                    ));
                    assert!(
                        failure
                            .message()
                            .error_message
                            .as_ref()
                            .unwrap()
                            .contains("quota exceeded")
                    );
                    let admission = call.retry_admission();
                    if attempt < max_retries {
                        let admission = admission.unwrap();
                        tokio::time::sleep(std::time::Duration::from_millis(
                            admission.delay_ms.max(1),
                        ))
                        .await;
                    } else {
                        assert!(admission.is_none());
                    }
                }
                assert_eq!(payload_calls.load(Ordering::SeqCst), 1);
                assert_eq!(response_calls.load(Ordering::SeqCst), 0);
                let sent = server.received_requests().await.unwrap();
                assert_eq!(sent.len(), (max_retries + 1) as usize);
                assert!(sent.iter().all(|request| request.body == sent[0].body));
            }
        }
    }

    #[tokio::test]
    async fn cancelling_pending_429_body_aborts_without_retry_or_raw_text() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        for responses in [false, true] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let endpoint = format!("http://{}", listener.local_addr().unwrap());
            let (headers_sent, received) = tokio::sync::oneshot::channel();
            let peer = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut buffer = [0; 8192];
                assert!(socket.read(&mut buffer).await.unwrap() > 0);
                socket.write_all(b"HTTP/1.1 429 Too Many Requests\r\nContent-Length: 4096\r\nConnection: close\r\n\r\n{").await.unwrap();
                headers_sent.send(()).unwrap();
                loop {
                    if socket.read(&mut buffer).await.unwrap_or(0) == 0 {
                        break;
                    }
                }
            });
            let provider: Box<dyn Provider> = if responses {
                Box::new(opi_ai::openai_responses::OpenAiResponsesProvider::new(
                    Some(endpoint),
                ))
            } else {
                Box::new(opi_ai::openai_chat::OpenAiChatProvider::new(Some(endpoint)))
            };
            let request = super::frozen_target_wire::logical_request();
            let cancel = request.cancel.clone();
            let result = tokio::spawn(
                ChatStream::new(
                    provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()),
                )
                .result(),
            );
            received.await.unwrap();
            cancel.cancel();
            let failure = tokio::time::timeout(std::time::Duration::from_secs(2), result)
                .await
                .unwrap()
                .unwrap()
                .unwrap_err();
            assert!(matches!(failure.cause(), ProviderError::Cancelled));
            assert_eq!(failure.message().stop_reason, StopReason::Aborted);
            tokio::time::timeout(std::time::Duration::from_secs(2), peer)
                .await
                .unwrap()
                .unwrap();
        }
    }
}
