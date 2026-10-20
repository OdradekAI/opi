//! Agent retry integration tests (task 2.15).
//!
//! Tests that agent_loop retries on retryable errors (RateLimited, Timeout),
//! emits AutoRetryStart/End events, respects max_attempts, and does not
//! retry non-retryable errors (AuthFailed).

// opi-runtime-acceptance

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
use opi_ai::provider::{ProviderError, ProviderErrorCategory, ProviderErrorSummary};
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
        session_id: None,
        evidence_sink: None,
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
        session_id: None,
        evidence_sink: None,
    }
}

fn make_config(retry: Option<RetryConfig>) -> AgentLoopConfig {
    AgentLoopConfig { retry }
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
    .await
    .into_execution_result();

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
            ProviderErrorSummary::authentication_rejected(),
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
    .await
    .into_execution_result();

    assert!(result.is_err());
    match result.unwrap_err() {
        AgentError::Provider(failure) => match failure.provider_error() {
            ProviderError::AuthFailed(summary) => {
                assert_eq!(summary.as_str(), "provider rejected credentials")
            }
            other => panic!("expected retained AuthFailed provider error, got {other:?}"),
        },
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
    .await
    .into_execution_result();

    assert!(result.is_err());
    match result.unwrap_err() {
        AgentError::Provider(failure) => {
            assert_eq!(failure.category(), ProviderErrorCategory::RateLimit);
            assert!(failure.summary().is_none());
            assert!(matches!(
                failure.provider_error(),
                ProviderError::RateLimited {
                    retry_after_ms: Some(40)
                }
            ));
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
    .await
    .into_execution_result();

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
            MockResponse::Error(ProviderError::Network(ProviderErrorSummary::redacted())),
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
    .await
    .into_execution_result();

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
    .await
    .into_execution_result();

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
    .await
    .into_execution_result();

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
    .await
    .into_execution_result();

    // The partial-output error must surface as a failure, not recover.
    assert!(
        result.is_err(),
        "partial-output stream error should surface, not retry"
    );
    match result.unwrap_err() {
        AgentError::Provider(failure) => assert!(
            failure.category() == ProviderErrorCategory::RateLimit
                && matches!(
                    failure.provider_error(),
                    ProviderError::RateLimited {
                        retry_after_ms: Some(10)
                    }
                ),
            "expected the mid-stream rate-limit error to surface, got: {failure}"
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
    .await
    .into_execution_result();

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
        // The SDK computes the no-hint family backoff (~375-500ms with
        // jitter for the first retry); the long window still guarantees the
        // cancel lands mid-sleep rather than racing setup.
        initial_delay_ms: 2000,
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
        .into_execution_result()
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
async fn provider_cancelled_remains_a_typed_provider_failure() {
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
    .await
    .into_execution_result();

    assert!(
        matches!(
            result,
            Err(AgentError::Provider(ref failure))
                if matches!(failure.provider_error(), ProviderError::Cancelled)
        ),
        "provider cancellation should retain ProviderError::Cancelled, got {result:?}"
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

mod prepared_call_auth {
    use super::common;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    use opi_agent::agent::Agent;
    use opi_agent::hooks::{
        AgentHooks, BeforeToolCallContext, BeforeToolCallResult, ShouldStopAfterTurnContext,
    };
    use opi_agent::loop_types::{AgentError, AgentLoopConfig, InferenceConfig};
    use opi_agent::message::AgentMessage;
    use opi_ai::provider::ProviderError;
    use opi_ai::test_support::{CountingAuthResolver, MockProvider, MockResponse, text_response};
    use opi_ai::{AuthProvenanceSource, CompatMetadata, Models};

    struct StopImmediatelyHooks;
    impl AgentHooks for StopImmediatelyHooks {
        fn convert_to_llm(
            &self,
            messages: &[AgentMessage],
        ) -> Result<Vec<opi_ai::message::Message>, AgentError> {
            Ok(messages
                .iter()
                .filter_map(|m| match m {
                    AgentMessage::Llm(m) => Some(m.clone()),
                    _ => None,
                })
                .collect())
        }
        fn should_stop_after_turn(
            &self,
            _ctx: ShouldStopAfterTurnContext,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>> {
            Box::pin(async { true })
        }
        fn before_tool_call(
            &self,
            _ctx: BeforeToolCallContext,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = BeforeToolCallResult> + Send>>
        {
            Box::pin(async { BeforeToolCallResult::Continue })
        }
    }
    #[tokio::test]
    async fn prepare_call_resolves_auth_once_across_retries() {
        // First attempt: retriable rate-limit (no content delivered). Second
        // attempt: a successful text response. The retry reuses the same prepared
        // call, so the resolver must be invoked exactly once.
        let provider = MockProvider::new_with_errors(
            "mock",
            vec![
                MockResponse::Error(ProviderError::RateLimited {
                    retry_after_ms: None,
                }),
                MockResponse::Events(text_response("recovered")),
            ],
        );
        let call_log = provider.call_log_handle();

        let resolve_count = Arc::new(AtomicU32::new(0));
        let resolver: Arc<dyn opi_ai::auth::AuthResolver> =
            Arc::new(CountingAuthResolver::new(resolve_count.clone()));

        let mut collection = Models::new();
        collection
            .register_route(
                Box::new(provider),
                resolver,
                AuthProvenanceSource::Static,
                CompatMetadata::default(),
            )
            .expect("register route");

        let mut agent = Agent::new(
            Arc::new(collection),
            common::registrations_from(Vec::new()),
            Some(common::permissive_authorizer()),
            "mock:mock-model".to_string(),
            None,
            InferenceConfig::default(),
            AgentLoopConfig {
                retry: Some(opi_ai::retry::RetryConfig {
                    max_attempts: 3,
                    ..Default::default()
                }),
            },
            Box::new(StopImmediatelyHooks),
        )
        .expect("agent");

        let result = agent.prompt("hi").await;
        assert!(
            result.error().is_none(),
            "prompt should succeed after retry: {:?}",
            result.error()
        );

        // Auth resolved exactly once across the two attempts.
        assert_eq!(
            resolve_count.load(Ordering::SeqCst),
            1,
            "resolver must be invoked once per logical call, not per attempt"
        );
        // Two provider attempts: the failed attempt plus the recovered one.
        assert_eq!(
            call_log.lock().unwrap().len(),
            2,
            "provider stream must be invoked once per attempt"
        );
    }
}

// ---------------------------------------------------------------------------
// SDK acquisition owner cutover (task 20.4)
// ---------------------------------------------------------------------------

/// The SDK acquisition episode performs exactly 1+N sends: the loop schedules
/// only admitted attempts, so nothing stacks on top of the SDK owner.
#[tokio::test]
async fn sdk_acquisition_episode_totals_exact_sends() {
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Error(ProviderError::RateLimited {
                retry_after_ms: Some(1),
            }),
            MockResponse::Error(ProviderError::RateLimited {
                retry_after_ms: Some(1),
            }),
            MockResponse::Events(test_support::text_response("recovered")),
        ],
    );
    let call_log = provider.call_log_handle();

    let (log, sink) = collect_events();
    let result = agent_loop(
        make_context(provider),
        make_config(Some(RetryConfig {
            max_attempts: 3,
            ..fast_retry_config()
        })),
        &NoopHooks,
        sink,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .into_execution_result();

    assert!(result.is_ok(), "two rate limits then success: {result:?}");
    assert_eq!(
        call_log.lock().unwrap().len(),
        3,
        "exactly 1+N sends (initial + two admitted retries)"
    );
    let events = log.lock().unwrap().clone();
    let starts: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::AutoRetryStart { .. }))
        .collect();
    assert_eq!(starts.len(), 2, "one AutoRetryStart per admitted retry");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AgentEvent::AutoRetryEnd { success: true, .. }))
    );
}

/// A policy of one extra send stops the episode at 2 sends even though the
/// failure stays retryable: the product loop cannot stack sends beyond the
/// frozen per-call budget.
#[tokio::test]
async fn product_loop_cannot_stack_sends_beyond_the_frozen_budget() {
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Error(ProviderError::RateLimited {
                retry_after_ms: Some(1),
            }),
            MockResponse::Error(ProviderError::RateLimited {
                retry_after_ms: Some(1),
            }),
            // A third response exists only to prove the loop never consumes
            // it: exceeding it would panic the mock.
            MockResponse::Events(test_support::text_response("never sent")),
        ],
    );
    let call_log = provider.call_log_handle();

    let (log, sink) = collect_events();
    let result = agent_loop(
        make_context(provider),
        make_config(Some(RetryConfig {
            max_attempts: 1,
            initial_delay_ms: 1,
            max_delay_ms: 10,
        })),
        &NoopHooks,
        sink,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .into_execution_result();

    assert!(result.is_err(), "the exhausted episode must fail");
    match result.unwrap_err() {
        AgentError::Provider(failure) => assert!(
            matches!(failure.provider_error(), ProviderError::RateLimited { .. }),
            "the second rate limit surfaces, got {failure}"
        ),
        other => panic!("expected Provider error, got {other:?}"),
    }
    assert_eq!(
        call_log.lock().unwrap().len(),
        2,
        "exactly 1+N sends with max_attempts=1"
    );
    let events = log.lock().unwrap().clone();
    let starts: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, AgentEvent::AutoRetryStart { .. }))
        .collect();
    assert_eq!(starts.len(), 1, "only the first failure was admitted");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AgentEvent::AutoRetryEnd { success: false, .. }))
    );
}
