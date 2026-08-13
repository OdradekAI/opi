//! Agent retry integration tests (task 2.15).
//!
//! Tests that agent_loop retries on retryable errors (RateLimited, Timeout),
//! emits AutoRetryStart/End events, respects max_attempts, and does not
//! retry non-retryable errors (AuthFailed).

mod common;

use std::sync::{Arc, Mutex};

use opi_agent::agent_loop;
use opi_agent::diagnostic::code::{
    CODE_PROVIDER_RETRY_EXHAUSTED, CODE_PROVIDER_RETRY_SUPPRESSED_AFTER_PARTIAL_OUTPUT,
};
use opi_agent::event::{AgentEvent, AgentEventSink};
use opi_agent::hooks::AgentHooks;
use opi_agent::loop_types::{
    AgentError, AgentLoopConfig, AgentLoopContext, InferenceConfig, ModelSelection, NextTurnState,
};
use opi_agent::message::AgentMessage;
use opi_agent::{DiagnosticSink, RecordingSink};
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::ProviderError;
use opi_ai::retry::RetryConfig;
use opi_ai::test_support::{self, MockProvider, MockResponse, single_route_collection};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

struct NoopHooks;

impl AgentHooks for NoopHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        let mut result = Vec::new();
        for msg in messages {
            if let AgentMessage::Llm(m) = msg {
                result.push(m.clone());
            }
        }
        Ok(result)
    }
}

fn user_msg(text: &str) -> AgentMessage {
    AgentMessage::Llm(Message::User(UserMessage {
        content: vec![InputContent::Text { text: text.into() }],
        timestamp_ms: 0,
    }))
}

fn make_context(provider: MockProvider) -> AgentLoopContext {
    AgentLoopContext {
        collection: Arc::new(single_route_collection(Box::new(provider))),
        registry: common::test_registry(vec![]),
        authorizer: Some(common::permissive_authorizer()),
        evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
        state: NextTurnState::new(
            vec![user_msg("hello")],
            ModelSelection::parse_spec("mock:mock-model").unwrap(),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: None,
        trace: None,
        session_id: None,
    }
}

fn make_context_with_sink(provider: MockProvider, sink: Arc<RecordingSink>) -> AgentLoopContext {
    AgentLoopContext {
        collection: Arc::new(single_route_collection(Box::new(provider))),
        registry: common::test_registry(vec![]),
        authorizer: Some(common::permissive_authorizer()),
        evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
        state: NextTurnState::new(
            vec![user_msg("hello")],
            ModelSelection::parse_spec("mock:mock-model").unwrap(),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: Some(sink as Arc<dyn DiagnosticSink>),
        trace: None,
        session_id: None,
    }
}

fn make_config(retry: Option<RetryConfig>) -> AgentLoopConfig {
    AgentLoopConfig {
        max_turns: 10,
        retry,
    }
}

fn collect_events() -> (Arc<Mutex<Vec<AgentEvent>>>, AgentEventSink) {
    let log = Arc::new(Mutex::new(Vec::<AgentEvent>::new()));
    let l = log.clone();
    let sink = Box::new(move |e: AgentEvent| {
        l.lock().unwrap().push(e);
    }) as AgentEventSink;
    (log, sink)
}

fn fast_retry_config() -> RetryConfig {
    RetryConfig {
        max_attempts: 3,
        initial_delay_ms: 10,
        max_delay_ms: 100,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn retry_on_rate_limited_then_succeed() {
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Error(ProviderError::RateLimited {
                retry_after_ms: Some(100),
            }),
            MockResponse::Events(test_support::text_response("success after retry")),
        ],
    );

    let (log, sink) = collect_events();
    let result = agent_loop(
        make_context(provider),
        make_config(Some(fast_retry_config())),
        &NoopHooks,
        sink,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(
        result.is_ok(),
        "should succeed after retry: {:?}",
        result.err()
    );
    let events = log.lock().unwrap().clone();

    let starts: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::AutoRetryStart { .. }))
        .collect();
    assert_eq!(starts.len(), 1, "should have one AutoRetryStart");

    let ends: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::AutoRetryEnd { success: true, .. }))
        .collect();
    assert_eq!(ends.len(), 1, "should have one AutoRetryEnd(success=true)");
}

#[tokio::test]
async fn no_retry_on_auth_error() {
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![MockResponse::Error(ProviderError::AuthFailed(
            "bad key".into(),
        ))],
    );

    let (log, sink) = collect_events();
    let result = agent_loop(
        make_context(provider),
        make_config(Some(fast_retry_config())),
        &NoopHooks,
        sink,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AgentError::AuthFailed(msg) => assert!(msg.contains("bad key")),
        other => panic!("expected AuthFailed, got {other:?}"),
    }

    let events = log.lock().unwrap().clone();
    let retry_events: Vec<_> = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                AgentEvent::AutoRetryStart { .. } | AgentEvent::AutoRetryEnd { .. }
            )
        })
        .collect();
    assert!(
        retry_events.is_empty(),
        "auth error should not trigger retry"
    );
}

#[tokio::test]
async fn retry_exhausted_returns_error() {
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Error(ProviderError::RateLimited {
                retry_after_ms: Some(10),
            }),
            MockResponse::Error(ProviderError::RateLimited {
                retry_after_ms: Some(20),
            }),
            MockResponse::Error(ProviderError::RateLimited {
                retry_after_ms: Some(40),
            }),
        ],
    );

    let config = RetryConfig {
        max_attempts: 2,
        initial_delay_ms: 10,
        max_delay_ms: 100,
    };

    let (log, sink) = collect_events();
    let result = agent_loop(
        make_context(provider),
        make_config(Some(config)),
        &NoopHooks,
        sink,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AgentError::Provider(msg) => {
            assert!(msg.contains("rate limited"), "got: {msg}");
        }
        other => panic!("expected Provider error, got {other:?}"),
    }

    let events = log.lock().unwrap().clone();
    let starts: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::AutoRetryStart { .. }))
        .collect();
    assert_eq!(starts.len(), 2, "should have 2 AutoRetryStart events");

    let fails: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::AutoRetryEnd { success: false, .. }))
        .collect();
    assert_eq!(fails.len(), 1, "should have 1 AutoRetryEnd(success=false)");
}

#[tokio::test]
async fn retry_on_timeout_then_succeed() {
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Error(ProviderError::Timeout),
            MockResponse::Events(test_support::text_response("after timeout")),
        ],
    );

    let (log, sink) = collect_events();
    let result = agent_loop(
        make_context(provider),
        make_config(Some(fast_retry_config())),
        &NoopHooks,
        sink,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(result.is_ok());
    let events = log.lock().unwrap().clone();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AgentEvent::AutoRetryStart { .. }))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AgentEvent::AutoRetryEnd { success: true, .. }))
    );
}

#[tokio::test]
async fn retry_on_network_error_then_succeed() {
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Error(ProviderError::Network("connection reset".into())),
            MockResponse::Events(test_support::text_response("success after network retry")),
        ],
    );

    let (log, sink) = collect_events();
    let result = agent_loop(
        make_context(provider),
        make_config(Some(fast_retry_config())),
        &NoopHooks,
        sink,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(
        result.is_ok(),
        "network error should retry and then succeed: {result:?}"
    );
    let events = log.lock().unwrap().clone();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AgentEvent::AutoRetryStart { .. })),
        "network retry should emit AutoRetryStart"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AgentEvent::AutoRetryEnd { success: true, .. })),
        "network retry should emit AutoRetryEnd(success=true)"
    );
}

#[tokio::test]
async fn no_retry_when_config_is_none() {
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![MockResponse::Error(ProviderError::RateLimited {
            retry_after_ms: None,
        })],
    );

    let (log, sink) = collect_events();
    let result = agent_loop(
        make_context(provider),
        make_config(None),
        &NoopHooks,
        sink,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(result.is_err());
    let events = log.lock().unwrap().clone();
    let retry_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::AutoRetryStart { .. }))
        .collect();
    assert!(retry_events.is_empty(), "no retry when config is None");
}

#[tokio::test]
async fn retry_auto_retry_start_fields() {
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Error(ProviderError::RateLimited {
                retry_after_ms: Some(5000),
            }),
            MockResponse::Events(test_support::text_response("ok")),
        ],
    );

    let (log, sink) = collect_events();
    let result = agent_loop(
        make_context(provider),
        make_config(Some(fast_retry_config())),
        &NoopHooks,
        sink,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(result.is_ok());
    let events = log.lock().unwrap().clone();
    let start = events
        .iter()
        .find_map(|e| match e {
            AgentEvent::AutoRetryStart {
                attempt,
                max_attempts,
                delay_ms,
                error_message,
            } => Some((*attempt, *max_attempts, *delay_ms, error_message.clone())),
            _ => None,
        })
        .expect("should have AutoRetryStart");

    assert_eq!(start.0, 1, "attempt should be 1 (first retry)");
    assert_eq!(start.1, 3, "max_attempts should be 3");
    assert!(start.2 > 0, "delay_ms should be positive");
    assert!(
        start.3.contains("rate limited"),
        "error_message should describe the error"
    );
}

// ---------------------------------------------------------------------------
// No retry after partial streamed content (Phase 12 task 12.7 DoD clauses 4/5)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn no_retry_after_partial_streamed_content() {
    // Once the provider has streamed content (Start + TextDelta), a subsequent
    // mid-stream retryable error must NOT trigger a retry: the caller has
    // already observed partial output, and retrying would emit a second Start
    // plus duplicated content. The error must surface through the runtime
    // instead of panicking or silently retrying. (DoD clause 4: partial-output
    // stream errors map into provider/runtime diagnostics; clause 5: no retry
    // after partial streamed content.)
    let mut partial_events = test_support::text_response("partial content");
    partial_events.pop(); // drop Done, leaving Start + TextDelta

    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::EventsThenError(
                partial_events,
                ProviderError::RateLimited {
                    retry_after_ms: Some(10),
                },
            ),
            // A second response would only be consumed if the loop wrongly retried.
            MockResponse::Events(test_support::text_response("after retry")),
        ],
    );

    let (log, sink) = collect_events();
    let result = agent_loop(
        make_context(provider),
        make_config(Some(fast_retry_config())),
        &NoopHooks,
        sink,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    // The partial-output error must surface as a failure, not recover.
    assert!(
        result.is_err(),
        "partial-output stream error should surface, not retry"
    );
    match result.unwrap_err() {
        AgentError::Provider(msg) => assert!(
            msg.contains("rate limited"),
            "expected the mid-stream rate-limit error to surface, got: {msg}"
        ),
        other => panic!("expected AgentError::Provider from partial-output stream, got {other:?}"),
    }

    let events = log.lock().unwrap().clone();
    let retry_starts: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::AutoRetryStart { .. }))
        .collect();
    assert!(
        retry_starts.is_empty(),
        "must NOT retry after partial streamed content (saw AutoRetryStart)"
    );

    // A retry would have consumed the second ("after retry") response and
    // returned Ok; since the run surfaced Err with no AutoRetryStart, the
    // second response was never popped.
}

#[tokio::test]
async fn retry_after_prior_attempt_then_partial_stream_error_is_not_exhausted() {
    let mut partial_events = test_support::text_response("partial after retry");
    partial_events.pop();

    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Error(ProviderError::RateLimited {
                retry_after_ms: Some(1),
            }),
            MockResponse::EventsThenError(
                partial_events,
                ProviderError::RateLimited {
                    retry_after_ms: Some(1),
                },
            ),
        ],
    );
    let sink = Arc::new(RecordingSink::new());

    let result = agent_loop(
        make_context_with_sink(provider, sink.clone()),
        make_config(Some(fast_retry_config())),
        &NoopHooks,
        Box::new(|_: AgentEvent| {}),
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(
        result.is_err(),
        "partial stream error after retry should fail"
    );
    let codes: Vec<_> = sink.snapshot().iter().map(|d| d.code).collect();
    assert!(
        codes.contains(&CODE_PROVIDER_RETRY_SUPPRESSED_AFTER_PARTIAL_OUTPUT),
        "missing partial-output suppression diagnostic: {codes:?}"
    );
    assert!(
        !codes.contains(&CODE_PROVIDER_RETRY_EXHAUSTED),
        "partial-output suppression must not report retry exhaustion: {codes:?}"
    );
}

// ---------------------------------------------------------------------------
// Cancellation during retry backoff (Phase 12 task 12.7 DoD clause 8)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cancellation_during_retry_backoff_aborts() {
    // A cancellation signal arriving during the retry backoff sleep must abort
    // the run promptly instead of waiting for the backoff to elapse. This
    // exercises the agent_loop tokio::select! arm tagged "during_retry_sleep".
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Error(ProviderError::RateLimited {
                retry_after_ms: None,
            }),
            MockResponse::Events(test_support::text_response("after retry")),
        ],
    );

    let retry = RetryConfig {
        max_attempts: 3,
        initial_delay_ms: 2000, // long backoff so the cancel lands mid-sleep
        max_delay_ms: 5000,
    };
    let cancel = tokio_util::sync::CancellationToken::new();

    let (log, sink) = collect_events();
    let cancel_for_task = cancel.clone();
    let handle = tokio::spawn(async move {
        agent_loop(
            make_context(provider),
            make_config(Some(retry)),
            &NoopHooks,
            sink,
            cancel_for_task,
        )
        .await
    });

    // Wait until the retry has actually started (AutoRetryStart emitted) so the
    // cancel lands during the backoff sleep rather than racing setup. The 2s
    // poll window is generous insurance against CI scheduler stalls even though
    // AutoRetryStart is emitted synchronously well before the 2s backoff sleep.
    let mut started = false;
    for _ in 0..200 {
        if log
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e, AgentEvent::AutoRetryStart { .. }))
        {
            started = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(started, "retry should have started within the poll window");
    cancel.cancel();

    let result = handle.await.expect("task should join");
    assert!(result.is_err(), "cancelled run should return Err");
    assert!(
        matches!(result.unwrap_err(), AgentError::Cancelled),
        "cancelled-during-backoff run should return AgentError::Cancelled"
    );

    let events = log.lock().unwrap().clone();
    // No successful retry: the run was aborted during backoff, not recovered.
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, AgentEvent::AutoRetryEnd { success: true, .. })),
        "cancelled-during-backoff run must not report retry success"
    );
}

#[tokio::test]
async fn provider_cancelled_routes_to_agent_cancelled() {
    let provider =
        MockProvider::new_with_errors("mock", vec![MockResponse::Error(ProviderError::Cancelled)]);

    let (log, sink) = collect_events();
    let result = agent_loop(
        make_context(provider),
        make_config(Some(fast_retry_config())),
        &NoopHooks,
        sink,
        tokio_util::sync::CancellationToken::new(),
    )
    .await;

    assert!(
        matches!(result, Err(AgentError::Cancelled)),
        "provider cancellation should surface as AgentError::Cancelled, got {result:?}"
    );

    let events = log.lock().unwrap().clone();
    assert!(
        !events.iter().any(|e| {
            matches!(
                e,
                AgentEvent::AutoRetryStart { .. } | AgentEvent::AutoRetryEnd { .. }
            )
        }),
        "provider cancellation must not emit retry events"
    );
}
