//! Deferred-response contracts (task 20.50, AIR-043/AIR-044, V-37/V-38).
//!
//! Covers the pi-shaped deferred surface: submission through normal
//! generation returns `Pending(handle)` or a final message, `fetch` checks
//! once at wait0 and may stay pending, explicit best-effort cancellation is
//! independent of local abort, and handles validate identity/expiry/serde
//! with typed errors. The reopenable consumer checkpoint proves fetch works
//! without resubmission under a fake monotonic clock; the SDK never polls.

use opi_ai::AuthProvenanceSource;
use opi_ai::Models;
use opi_ai::auth::ResolvedAuth;
use opi_ai::auth::{AuthScheme, StaticAuthResolver};
use opi_ai::deferred::{DeferredError, DeferredFetchCall, DeferredHandle};
use opi_ai::stream::StopReason;
use opi_ai::telemetry::TelemetryContext;
use opi_ai::test_support::{self, ScriptedDeferredBackend};
use secrecy::SecretString;
use serde_json::json;
use tokio_util::sync::CancellationToken;

fn pending_handle() -> DeferredHandle {
    DeferredHandle {
        provider: "test".into(),
        model_id: "mock-model".into(),
        api: "scripted-deferred".into(),
        id: "deferred-1".into(),
        expires_at_ms: None,
        poll_after_ms: None,
        data: None,
    }
}

// ---------------------------------------------------------------------------
// Handle identity, expiry, and serialization (AIR-043)
// ---------------------------------------------------------------------------

#[test]
fn deferred_handle_serializes_pi_camel_case() {
    let handle = DeferredHandle {
        provider: "p".into(),
        model_id: "m".into(),
        api: "anthropic-messages".into(),
        id: "resp_123".into(),
        expires_at_ms: Some(1_700_000_000_000),
        poll_after_ms: Some(5_000),
        data: Some(json!({ "row": 2 })),
    };
    let value = serde_json::to_value(&handle).expect("serialize");
    assert_eq!(
        value,
        json!({
            "provider": "p",
            "modelId": "m",
            "api": "anthropic-messages",
            "id": "resp_123",
            "expiresAt": 1_700_000_000_000u64,
            "pollAfterMs": 5_000u64,
            "data": { "row": 2 },
        })
    );
    let round: DeferredHandle = serde_json::from_value(value).expect("deserialize");
    assert_eq!(round, handle);
}

#[test]
fn deferred_handle_minimal_roundtrip_omits_optional_fields() {
    let handle = pending_handle();
    let value = serde_json::to_value(&handle).expect("serialize");
    assert!(value.get("expiresAt").is_none());
    assert!(value.get("pollAfterMs").is_none());
    assert!(value.get("data").is_none());
    let round: DeferredHandle = serde_json::from_value(value).expect("deserialize");
    assert_eq!(round, handle);
}

#[test]
fn deferred_handle_identity_mismatch_is_typed() {
    let handle = pending_handle();
    let wrong = handle
        .validate_identity("other-provider", "mock-model", "scripted-deferred")
        .unwrap_err();
    assert!(matches!(wrong, DeferredError::RouteMismatch { .. }));
    let wrong = handle
        .validate_identity("test", "other-model", "scripted-deferred")
        .unwrap_err();
    assert!(matches!(wrong, DeferredError::RouteMismatch { .. }));
    let wrong = handle
        .validate_identity("test", "mock-model", "other-api")
        .unwrap_err();
    assert!(matches!(wrong, DeferredError::RouteMismatch { .. }));
    handle
        .validate_identity("test", "mock-model", "scripted-deferred")
        .expect("matching identity validates");
}

#[test]
fn deferred_handle_expiry_is_typed_against_injected_clock() {
    let mut handle = pending_handle();
    handle.expires_at_ms = Some(1_000);
    handle.validate_expiry(999).expect("not yet expired");
    let expired = handle.validate_expiry(1_000).unwrap_err();
    assert!(matches!(expired, DeferredError::Expired { .. }));
}

// ---------------------------------------------------------------------------
// Submission through normal generation (AIR-043)
// ---------------------------------------------------------------------------

fn chat_request() -> opi_ai::provider::Request {
    opi_ai::provider::Request {
        transport: Default::default(),
        model: "mock-model".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![opi_ai::message::Message::User(
                opi_ai::message::UserMessage {
                    content: vec![opi_ai::message::InputContent::Text { text: "Hi".into() }],
                    timestamp_ms: 0,
                },
            )],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: opi_ai::provider::CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

fn chat_record(models: &Models) -> opi_ai::models::ChatModelRecord {
    let (_provider, model) = models
        .resolve("test:mock-model")
        .expect("mock route resolves");
    opi_ai::models::ChatModelRecord::new("test", model.clone())
}

#[tokio::test]
async fn normal_generation_returns_pending_handle_or_final() {
    let backend = std::sync::Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let mut models = test_support::single_route_collection(Box::new(
        test_support::MockProvider::new("test", vec![test_support::text_response("final answer")])
            .with_deferred(std::sync::Arc::clone(&backend), 1, Some(5_000)),
    ));
    models
        .register_deferred_api(
            std::sync::Arc::clone(&backend)
                as std::sync::Arc<dyn opi_ai::deferred::DeferredApi<()>>,
            std::sync::Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register deferred api");

    // Deferred submission: the same ordinary simple entry returns a pending
    // terminal with the handle instead of final content.
    let pending = models
        .complete_simple(chat_record(&models), chat_request(), Default::default())
        .await
        .expect("pending submission is an Ok outcome");
    assert_eq!(pending.stop_reason, StopReason::Deferred);
    assert!(pending.content.is_empty());
    let handle = *pending.deferred.clone().expect("handle rides the message");
    assert_eq!(handle.provider, "test");
    assert_eq!(handle.model_id, "mock-model");
    assert_eq!(handle.api, "scripted-deferred");
    assert_eq!(handle.poll_after_ms, Some(5_000));
    assert_eq!(backend.fetch_count(), 0);

    // Without deferred scripting the same normal generation stays final.
    let plain = test_support::single_route_collection(Box::new(test_support::MockProvider::new(
        "test",
        vec![test_support::text_response("done")],
    )));
    let final_message = plain
        .complete_simple(chat_record(&plain), chat_request(), Default::default())
        .await
        .expect("final submission");
    assert_eq!(final_message.stop_reason, StopReason::Stop);
    assert!(final_message.deferred.is_none());
}

// ---------------------------------------------------------------------------
// Fetch: wait0 checks once, pending or ready (AIR-043)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn wait0_fetch_checks_once_and_may_stay_pending() {
    let backend = std::sync::Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let mut collection = test_support::single_route_collection(Box::new(
        test_support::MockProvider::new("test", vec![test_support::text_response("ready")]),
    ));
    collection
        .register_deferred_api(
            std::sync::Arc::clone(&backend)
                as std::sync::Arc<dyn opi_ai::deferred::DeferredApi<()>>,
            std::sync::Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register");

    // Script one submission with one pending fetch before readiness.
    let mut submission_events = test_support::text_response("ready");
    for event in &mut submission_events {
        if let opi_ai::stream::AssistantStreamEvent::Done { message, .. } = event {
            message.provider = "test".into();
            message.model = "mock-model".into();
        }
    }
    let handle = backend.submit(submission_events, 1, None);

    // wait0: exactly one status check; the outcome stays pending with the
    // same handle.
    let pending = collection
        .fetch_deferred(&handle, DeferredFetchCall::default())
        .await
        .expect("pending is Ok");
    assert_eq!(pending.stop_reason, StopReason::Deferred);
    assert_eq!(pending.deferred.as_deref(), Some(&handle));
    assert_eq!(backend.fetch_count(), 1);

    // A second explicit fetch is ready: the final assistant message alone.
    let ready = collection
        .fetch_deferred(&handle, DeferredFetchCall::default())
        .await
        .expect("ready");
    assert_eq!(ready.stop_reason, StopReason::Stop);
    assert!(ready.deferred.is_none());
    let text = match ready.content.first() {
        Some(opi_ai::message::AssistantContent::Text { text, .. }) => text.clone(),
        other => panic!("expected final text, got {other:?}"),
    };
    assert_eq!(text, "ready");
    assert_eq!(backend.fetch_count(), 2);
}

#[tokio::test]
async fn unsupported_provider_and_unknown_handle_fail_typed() {
    let backend = std::sync::Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let mut collection = test_support::single_route_collection(Box::new(
        test_support::MockProvider::new("test", vec![test_support::text_response("x")]),
    ));
    // No deferred registration: fetch fails typed before auth.
    let mut stranger = pending_handle();
    stranger.provider = "other".into();
    let error = collection
        .validate_deferred_handle(&stranger, None)
        .unwrap_err();
    assert!(matches!(error, DeferredError::Unsupported { .. }));
    let failure = collection
        .fetch_deferred(&stranger, DeferredFetchCall::default())
        .await
        .unwrap_err();
    assert_eq!(failure.message().stop_reason, StopReason::Error);

    // Registered backend, but the handle token is unknown: the scripted
    // backend fails the single fetch attempt.
    collection
        .register_deferred_api(
            std::sync::Arc::clone(&backend)
                as std::sync::Arc<dyn opi_ai::deferred::DeferredApi<()>>,
            std::sync::Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register");
    let mut unknown = pending_handle();
    unknown.id = "deferred-999".into();
    let failure = collection
        .fetch_deferred(&unknown, DeferredFetchCall::default())
        .await
        .unwrap_err();
    assert!(
        failure
            .message()
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("Unknown scripted deferred response")
    );
}

#[tokio::test]
async fn expired_handle_fails_typed_before_auth_or_dispatch() {
    let backend = std::sync::Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let mut collection = test_support::single_route_collection(Box::new(
        test_support::MockProvider::new("test", vec![test_support::text_response("x")]),
    ));
    collection
        .register_deferred_api(
            std::sync::Arc::clone(&backend)
                as std::sync::Arc<dyn opi_ai::deferred::DeferredApi<()>>,
            std::sync::Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register");
    let handle =
        backend.submit_with_expiry(test_support::text_response("late"), 0, None, Some(1_000));
    // Typed validation rejects at the SDK boundary with the injected clock.
    let error = collection
        .validate_deferred_handle(&handle, Some(1_000))
        .unwrap_err();
    assert!(matches!(error, DeferredError::Expired { .. }));
    // Fetch with the same clock reading never dispatches.
    let call = DeferredFetchCall {
        now_ms: Some(1_000),
        ..Default::default()
    };
    let failure = collection.fetch_deferred(&handle, call).await.unwrap_err();
    assert!(
        failure
            .message()
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("expired")
    );
    assert_eq!(backend.fetch_count(), 0);
}

#[tokio::test]
async fn missing_credential_fails_typed_with_remediation_message() {
    let backend = std::sync::Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let mut collection = test_support::single_route_collection(Box::new(
        test_support::MockProvider::new("test", vec![test_support::text_response("x")]),
    ));
    // A resolver that reports CredentialNeeded, like classify/images.
    struct MissingKey;
    impl opi_ai::auth::AuthResolver for MissingKey {
        fn resolve<'a>(
            &'a self,
        ) -> opi_ai::credential::BoxAuthFuture<
            'a,
            Result<ResolvedAuth, opi_ai::provider::ProviderError>,
        > {
            Box::pin(async {
                Err(opi_ai::provider::ProviderError::CredentialNeeded {
                    provider_id: "test".into(),
                })
            })
        }
    }
    collection
        .register_deferred_api(
            std::sync::Arc::clone(&backend)
                as std::sync::Arc<dyn opi_ai::deferred::DeferredApi<()>>,
            std::sync::Arc::new(MissingKey),
            AuthProvenanceSource::Static,
        )
        .expect("register");
    let handle = backend.submit(test_support::text_response("never"), 5, None);
    let failure = collection
        .fetch_deferred(&handle, DeferredFetchCall::default())
        .await
        .unwrap_err();
    // The typed cause stays CredentialNeeded (remediation-classified) and
    // the canonical message carries the provider name; nothing dispatched.
    assert!(matches!(
        failure.cause(),
        opi_ai::provider::ProviderError::CredentialNeeded { .. }
    ));
    assert!(
        failure
            .message()
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("test")
    );
    assert_eq!(backend.fetch_count(), 0);
}

// ---------------------------------------------------------------------------
// Cancellation: local abort vs explicit best-effort remote cancel
// (AIR-043/AIR-044)
// ---------------------------------------------------------------------------

fn registered_collection(
    backend: &std::sync::Arc<ScriptedDeferredBackend>,
) -> (Models, opi_ai::models::ChatModelRecord) {
    let mut collection = test_support::single_route_collection(Box::new(
        test_support::MockProvider::new("test", vec![test_support::text_response("x")]),
    ));
    collection
        .register_deferred_api(
            std::sync::Arc::clone(backend) as std::sync::Arc<dyn opi_ai::deferred::DeferredApi<()>>,
            std::sync::Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register");
    let record = chat_record(&collection);
    (collection, record)
}

#[tokio::test]
async fn local_abort_never_cancels_the_remote_job() {
    let backend = std::sync::Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let handle = backend.submit(test_support::text_response("late"), 3, None);
    let (collection, _record) = registered_collection(&backend);

    // Cancel the local fetch token mid-flight: the attempt aborts locally
    // and the backend never received a cancel.
    let cancel = CancellationToken::new();
    cancel.cancel();
    let call = DeferredFetchCall {
        cancel,
        ..Default::default()
    };
    let failure = collection.fetch_deferred(&handle, call).await.unwrap_err();
    assert!(matches!(
        failure.cause(),
        opi_ai::provider::ProviderError::Cancelled
    ));
    assert!(backend.cancelled_handles().is_empty());
    // Cancellation before the first poll dispatches zero backend work
    // (AIR-004: drop before first poll performs zero source work).
    assert_eq!(backend.fetch_count(), 0);

    // The remote job is untouched: the next explicit fetch still works.
    let pending = collection
        .fetch_deferred(&handle, DeferredFetchCall::default())
        .await
        .expect("still fetchable");
    assert_eq!(pending.stop_reason, StopReason::Deferred);
    assert!(backend.cancelled_handles().is_empty());
}

#[tokio::test]
async fn explicit_cancel_is_recorded_and_best_effort() {
    let backend = std::sync::Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let handle = backend.submit(test_support::text_response("late"), 3, None);
    let (collection, _record) = registered_collection(&backend);

    // A cancelled deferred response fails subsequent fetches typed.
    collection
        .cancel_deferred(&handle, opi_ai::deferred::DeferredCancelCall::default())
        .await
        .expect("best-effort cancel succeeds");
    assert_eq!(backend.cancelled_handles(), vec![handle.clone()]);
    let failure = collection
        .fetch_deferred(&handle, DeferredFetchCall::default())
        .await
        .unwrap_err();
    assert!(
        failure
            .message()
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("cancelled")
    );

    // Best-effort failure is reported, never swallowed.
    let other = backend.submit(test_support::text_response("late"), 3, None);
    backend.fail_cancels();
    let error = collection
        .cancel_deferred(&other, opi_ai::deferred::DeferredCancelCall::default())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("best-effort"));

    // Cancelling an unsupported provider fails typed without dispatch.
    let mut stranger = handle.clone();
    stranger.provider = "other".into();
    let error = collection
        .cancel_deferred(&stranger, opi_ai::deferred::DeferredCancelCall::default())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("does not support deferred"));
}

// ---------------------------------------------------------------------------
// Reopenable consumer checkpoint (AIR-044)
// ---------------------------------------------------------------------------

/// The offline reopenable consumer fixture: stores handle/model/next-poll
/// and reopens against a rebuilt collection without resubmitting. Time is
/// a caller-owned fake monotonic counter; the fixture's 5s poll advice is
/// test input only.
#[derive(Clone)]
struct ConsumerCheckpoint {
    handle: DeferredHandle,
    model: String,
    next_poll_ms: u64,
}

impl ConsumerCheckpoint {
    fn new(handle: DeferredHandle) -> Self {
        let model = format!("{}:{}", handle.provider, handle.model_id);
        Self {
            handle,
            model,
            next_poll_ms: 0,
        }
    }

    /// Serialize like a durable checkpoint would.
    fn serialize(&self) -> serde_json::Value {
        serde_json::json!({
            "handle": serde_json::to_value(&self.handle).expect("handle serializes"),
            "model": self.model,
            "nextPollMs": self.next_poll_ms,
        })
    }

    fn deserialize(value: serde_json::Value) -> Self {
        let handle: DeferredHandle =
            serde_json::from_value(value["handle"].clone()).expect("handle deserializes");
        Self {
            handle,
            model: value["model"].as_str().expect("model").to_owned(),
            next_poll_ms: value["nextPollMs"].as_u64().expect("next poll"),
        }
    }
}

#[derive(Clone)]
struct ConsumerState {
    checkpoint: ConsumerCheckpoint,
    committed: Vec<opi_ai::message::AssistantMessage>,
    errors: Vec<String>,
    scheduled: bool,
}

/// A caller-owned scheduler driven solely by explicit fake-clock ticks.
/// It has no submit path: reopening can only fetch the saved route.
struct ReopenedConsumer {
    clock: tokio::sync::mpsc::UnboundedSender<(u64, tokio::sync::oneshot::Sender<ConsumerState>)>,
    local_cancel: CancellationToken,
    scheduler: Option<tokio::task::JoinHandle<ConsumerState>>,
}

impl ReopenedConsumer {
    fn open(models: std::sync::Arc<Models>, checkpoint: ConsumerCheckpoint) -> Self {
        let local_cancel = CancellationToken::new();
        let stop = local_cancel.clone();
        let (clock, mut ticks) = tokio::sync::mpsc::unbounded_channel::<(
            u64,
            tokio::sync::oneshot::Sender<ConsumerState>,
        )>();
        let scheduler = tokio::spawn(async move {
            let mut state = ConsumerState {
                checkpoint,
                committed: Vec::new(),
                errors: Vec::new(),
                scheduled: true,
            };
            loop {
                let tick = tokio::select! {
                    biased;
                    _ = stop.cancelled() => break,
                    tick = ticks.recv() => tick,
                };
                let Some((now_ms, observed)) = tick else {
                    break;
                };
                if state.scheduled && now_ms >= state.checkpoint.next_poll_ms {
                    if models.resolve(&state.checkpoint.model).is_err() {
                        state.errors.push(format!(
                            "saved model is unavailable: {}",
                            state.checkpoint.model
                        ));
                        state.scheduled = false;
                    } else {
                        let result = models
                            .fetch_deferred(
                                &state.checkpoint.handle,
                                DeferredFetchCall {
                                    now_ms: Some(now_ms),
                                    cancel: stop.child_token(),
                                    ..Default::default()
                                },
                            )
                            .await;
                        match result {
                            Ok(message) if message.stop_reason == StopReason::Deferred => {
                                let handle = *message.deferred.expect("pending carries a handle");
                                // This is caller policy, not an SDK polling default.
                                state.checkpoint.next_poll_ms =
                                    now_ms + handle.poll_after_ms.unwrap_or(5_000);
                                state.checkpoint.handle = handle;
                            }
                            Ok(message) => {
                                state.committed.push(message);
                                state.scheduled = false;
                            }
                            Err(failure) => {
                                state.errors.push(failure.cause().to_string());
                                state.scheduled = false;
                            }
                        }
                    }
                }
                let _ = observed.send(state.clone());
            }
            state.scheduled = false;
            state
        });
        Self {
            clock,
            local_cancel,
            scheduler: Some(scheduler),
        }
    }

    async fn advance_to(&self, now_ms: u64) -> ConsumerState {
        let (observed, state) = tokio::sync::oneshot::channel();
        assert!(self.clock.send((now_ms, observed)).is_ok());
        state.await.expect("scheduler observed the clock tick")
    }

    async fn close(mut self) -> ConsumerState {
        self.local_cancel.cancel();
        self.scheduler
            .take()
            .expect("open scheduler")
            .await
            .expect("scheduler stopped")
    }
}

impl Drop for ReopenedConsumer {
    fn drop(&mut self) {
        self.local_cancel.cancel();
    }
}

#[tokio::test]
async fn reopened_consumer_fetches_without_resubmit_and_never_polls() {
    let backend = std::sync::Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let submitter =
        test_support::MockProvider::new("test", vec![test_support::text_response("the answer")])
            .with_deferred(std::sync::Arc::clone(&backend), 2, Some(7_000));
    let submitted = submitter.call_log_handle();
    let mut first = test_support::single_route_collection(Box::new(submitter));
    first
        .register_deferred_api(
            std::sync::Arc::clone(&backend)
                as std::sync::Arc<dyn opi_ai::deferred::DeferredApi<()>>,
            std::sync::Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register");

    // Submit through normal generation: one pending handle.
    let pending = first
        .complete_simple(chat_record(&first), chat_request(), Default::default())
        .await
        .expect("pending");
    let handle = pending.deferred.clone().expect("handle");
    let consumer = ReopenedConsumer::open(
        std::sync::Arc::new(first),
        ConsumerCheckpoint::new(*handle.clone()),
    );
    let first_pending = consumer.advance_to(1_000).await;
    assert_eq!(first_pending.checkpoint.next_poll_ms, 8_000);
    assert!(first_pending.committed.is_empty());
    assert_eq!(backend.fetch_count(), 1);
    consumer.advance_to(7_999).await;
    assert_eq!(backend.fetch_count(), 1, "before due time no fetch is sent");
    let clock = consumer.clock.clone();
    let saved = consumer.close().await;
    assert!(clock.is_closed(), "close joins and releases the scheduler");
    assert!(!saved.scheduled);
    assert!(
        backend.cancelled_handles().is_empty(),
        "closing never cancels the remote job"
    );
    let reopened = ConsumerCheckpoint::deserialize(saved.checkpoint.serialize());
    assert_eq!(reopened.handle, *handle);
    let reopened_provider = test_support::MockProvider::new("test", vec![]);
    let resubmitted = reopened_provider.call_log_handle();
    let mut second = test_support::single_route_collection(Box::new(reopened_provider));
    second
        .register_deferred_api(
            backend.clone(),
            std::sync::Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let reopened = ReopenedConsumer::open(std::sync::Arc::new(second), reopened);
    reopened.advance_to(7_999).await;
    assert_eq!(backend.fetch_count(), 1);
    let still_pending = reopened.advance_to(8_000).await;
    assert_eq!(still_pending.checkpoint.next_poll_ms, 15_000);
    assert!(still_pending.committed.is_empty());
    reopened.advance_to(8_000).await;
    reopened.advance_to(14_999).await;
    assert_eq!(backend.fetch_count(), 2);
    let ready_state = reopened.advance_to(15_000).await;
    assert_eq!(ready_state.committed.len(), 1);
    let ready = &ready_state.committed[0];
    assert_eq!(ready.stop_reason, StopReason::Stop);
    let text = match ready.content.first() {
        Some(opi_ai::message::AssistantContent::Text { text, .. }) => text.clone(),
        other => panic!("expected final text, got {other:?}"),
    };
    assert_eq!(text, "the answer");
    assert_eq!(backend.fetch_count(), 3);
    reopened.advance_to(30_000).await;
    let completed = reopened.close().await;
    assert_eq!(completed.committed.len(), 1);
    assert_eq!(backend.fetch_count(), 3);
    assert_eq!(submitted.lock().unwrap().len(), 1);
    assert!(resubmitted.lock().unwrap().is_empty());
}

#[tokio::test]
async fn consumer_fallback_missing_model_and_cancel_failures_are_observable() {
    let backend = std::sync::Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let handle = backend.submit(test_support::text_response("ready"), 3, None);
    let (models, _) = registered_collection(&backend);
    let models = std::sync::Arc::new(models);
    let consumer = ReopenedConsumer::open(models.clone(), ConsumerCheckpoint::new(handle.clone()));
    let pending = consumer.advance_to(1_000).await;
    assert_eq!(
        pending.checkpoint.next_poll_ms, 6_000,
        "caller fixture chooses the 5s fallback"
    );
    assert_eq!(
        pending.checkpoint.handle.poll_after_ms, None,
        "the SDK did not invent poll advice"
    );
    assert_eq!(
        pending.checkpoint.handle.expires_at_ms, None,
        "the SDK did not invent expiry"
    );
    consumer.advance_to(5_999).await;
    assert_eq!(backend.fetch_count(), 1);
    let aborted_wait = consumer.local_cancel.clone();
    let saved = consumer.close().await;
    assert!(aborted_wait.is_cancelled());
    assert!(backend.cancelled_handles().is_empty());
    backend.fail_cancels();
    let cancel_error = models
        .cancel_deferred(
            &handle,
            opi_ai::deferred::DeferredCancelCall {
                timeout: Some(std::time::Duration::from_secs(1)),
                cancel: CancellationToken::new(),
                ..Default::default()
            },
        )
        .await
        .expect_err("best-effort failure is observable");
    assert!(cancel_error.to_string().contains("best-effort"));
    assert_eq!(
        backend.cancel_telemetry_present().len(),
        1,
        "a fresh cancel token actually reaches the backend after local abort"
    );

    let missing = ReopenedConsumer::open(
        std::sync::Arc::new(Models::new()),
        ConsumerCheckpoint::deserialize(saved.checkpoint.serialize()),
    );
    let unavailable = missing.advance_to(6_000).await;
    assert_eq!(unavailable.errors.len(), 1);
    assert!(unavailable.errors[0].contains("saved model is unavailable: test:mock-model"));
    assert!(unavailable.committed.is_empty());
    assert!(!unavailable.scheduled);
    assert_eq!(unavailable.checkpoint.handle, handle);
    assert_eq!(backend.fetch_count(), 1);
    missing.close().await;
}

// ---------------------------------------------------------------------------
// Telemetry carrier participation (AIR-054)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn fetch_and_cancel_carry_the_frozen_telemetry_carrier() {
    let backend = std::sync::Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let handle = backend.submit(test_support::text_response("ready"), 1, None);
    let (collection, _record) = registered_collection(&backend);

    let fetch_call = DeferredFetchCall {
        telemetry_context: Some(TelemetryContext::noop()),
        ..Default::default()
    };
    collection
        .fetch_deferred(&handle, fetch_call)
        .await
        .expect("pending");
    assert_eq!(backend.fetch_telemetry_present(), vec![true]);

    // Default calls carry no carrier: no automatic capture.
    collection
        .fetch_deferred(&handle, DeferredFetchCall::default())
        .await
        .expect("ready");
    assert_eq!(backend.fetch_telemetry_present(), vec![true, false]);

    let cancel_call = opi_ai::deferred::DeferredCancelCall {
        telemetry_context: Some(TelemetryContext::noop()),
        ..Default::default()
    };
    collection
        .cancel_deferred(&handle, cancel_call)
        .await
        .expect("cancel");
    assert_eq!(backend.cancel_telemetry_present(), vec![true]);
}
