//! Reopening a saved job with a caller-owned clock and polling schedule.

use opi_ai::api::contract::DeferredFetchCall;
use opi_ai::auth::{AuthScheme, StaticAuthResolver};
use opi_ai::stream::StopReason;
use opi_ai::test_support::{self, ScriptedDeferredBackend};
use opi_ai::types::deferred::DeferredHandle;
use opi_ai::{AuthProvenanceSource, Models};
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;

use super::fixtures::{chat_record, chat_request, registered_collection};

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
                as std::sync::Arc<dyn opi_ai::api::contract::DeferredApi<()>>,
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
            opi_ai::api::contract::DeferredCancelCall {
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
