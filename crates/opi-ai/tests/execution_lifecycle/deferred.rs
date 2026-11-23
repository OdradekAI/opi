//! Deferred fetch readiness, drain, and poll settlement.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Poll;

use futures_core::Stream;
use futures_util::StreamExt;
use opi_ai::api::contract::{DeferredApi, DeferredCancelCall, DeferredFetchCall};
use opi_ai::message::AssistantMessage;
use opi_ai::provider::{EventStream, ProviderError, ProviderErrorSummary};
use opi_ai::stream::{
    AssistantStreamEvent, CallFailure, ChatStream, StartedChatStream, StopReason,
};
use opi_ai::test_support::{CountingAuthResolver, base_assistant};
use opi_ai::types::ApiId;
use opi_ai::types::deferred::DeferredHandle;
use opi_ai::{AuthProvenanceSource, BoxAuthFuture, Models};
use opi_telemetry::memory::Recorder;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use super::fixtures::{GatedTail, Item, bounded, channel_source, delta, gated_tail};

struct ControlledDeferredBackend {
    api: ApiId,
    handle: DeferredHandle,
    wait_ms: u64,
    source: Mutex<Option<EventStream>>,
}

impl DeferredApi<()> for ControlledDeferredBackend {
    fn api_id(&self) -> &ApiId {
        &self.api
    }

    fn provider_id(&self) -> &str {
        &self.handle.provider
    }

    fn model_id(&self) -> &str {
        &self.handle.model_id
    }

    fn default_options(&self) {}

    fn fetch_prepared(
        &self,
        handle: &DeferredHandle,
        _: &(),
        _: opi_ai::ResolvedAuth,
        call: DeferredFetchCall,
    ) -> EventStream {
        assert_eq!(handle, &self.handle);
        assert_eq!(call.wait_ms, self.wait_ms);
        self.source.lock().unwrap().take().expect("one fetch")
    }

    fn cancel_prepared<'a>(
        &'a self,
        _: &'a DeferredHandle,
        _: &'a (),
        _: opi_ai::ResolvedAuth,
        _: DeferredCancelCall,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        panic!("local execution must leave the remote job alone")
    }
}

fn deferred_handle() -> DeferredHandle {
    DeferredHandle {
        provider: "test".into(),
        model_id: "mock-model".into(),
        api: "execution-deferred".into(),
        id: "one-fetch".into(),
        expires_at_ms: None,
        poll_after_ms: None,
        data: None,
    }
}

fn deferred_models(
    source: EventStream,
    handle: &DeferredHandle,
    wait_ms: u64,
) -> (Models, Arc<AtomicU32>) {
    let auth_calls = Arc::new(AtomicU32::new(0));
    let mut models = Models::new();
    models
        .register_deferred_api(
            Arc::new(ControlledDeferredBackend {
                api: ApiId::new(handle.api.clone()),
                handle: handle.clone(),
                wait_ms,
                source: Mutex::new(Some(source)),
            }),
            Arc::new(CountingAuthResolver::new(auth_calls.clone())),
            AuthProvenanceSource::Static,
        )
        .expect("register deferred backend");
    (models, auth_calls)
}

fn deferred_call(
    source: EventStream,
    cancel: CancellationToken,
    recorder: &Recorder,
) -> (ChatStream, Arc<AtomicU32>) {
    let handle = deferred_handle();
    let (models, auth_calls) = deferred_models(source, &handle, 0);
    let stream = models.stream_deferred(
        &handle,
        DeferredFetchCall {
            cancel,
            telemetry_context: Some(recorder.context()),
            ..Default::default()
        },
    );
    (stream, auth_calls)
}

#[derive(Clone, Copy)]
enum DeferredDoneRead {
    Unread,
    Started,
    Direct,
}

fn deferred_ready_terminal() -> (AssistantMessage, Item) {
    let mut message = base_assistant();
    message.usage.output_tokens = 47;
    let terminal = Ok(AssistantStreamEvent::Done {
        reason: StopReason::Stop,
        message: message.clone(),
    });
    (message, terminal)
}

async fn assert_deferred_done(
    stream: &mut (impl Stream<Item = Result<AssistantStreamEvent, CallFailure>> + Unpin),
    expected: &AssistantMessage,
) {
    assert!(matches!(
        bounded(stream.next()).await,
        Some(Ok(AssistantStreamEvent::TextDelta { delta, .. })) if delta == "0"
    ));
    assert_eq!(
        bounded(stream.next()).await,
        Some(Ok(AssistantStreamEvent::Done {
            reason: StopReason::Stop,
            message: expected.clone(),
        }))
    );
}

async fn start_deferred_at_held_tail(
    mut stream: ChatStream,
    entered: oneshot::Receiver<()>,
    read: DeferredDoneRead,
    expected: &AssistantMessage,
) -> StartedChatStream {
    if matches!(read, DeferredDoneRead::Direct) {
        assert_deferred_done(&mut stream, expected).await;
    }
    let mut started = stream.start();
    bounded(entered).await.expect("source reaches the tail");
    if matches!(read, DeferredDoneRead::Started) {
        assert_deferred_done(&mut started, expected).await;
    }
    if !matches!(read, DeferredDoneRead::Unread) {
        assert!(futures_util::poll!(started.next()).is_pending());
    }
    started
}

fn assert_deferred_poll_settled(recorder: &Recorder, stop: StopReason) {
    let spans = recorder.spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "pi.ai.poll");
    assert!(spans[0].end_sequence.is_some());
    assert_eq!(
        spans[0].attributes.get("stop"),
        Some(&opi_telemetry::AttributeValue::String(stop.as_str().into()))
    );
    assert_eq!(
        recorder
            .measurements()
            .iter()
            .filter(|record| record.definition.name == "pi.ai.request.duration")
            .count(),
        1
    );
}

#[tokio::test]
async fn direct_deferred_drop_settles_cached_success_after_source_release() {
    for poll_tail in [false, true] {
        let cancel = CancellationToken::new();
        let (expected, terminal) = deferred_ready_terminal();
        let GatedTail {
            source,
            mut entered,
            release,
            mut eof,
            mut dropped,
        } = gated_tail(terminal, cancel.clone());
        let recorder = Recorder::default();
        let (mut stream, auth_calls) = deferred_call(source, cancel.clone(), &recorder);
        assert_deferred_done(&mut stream, &expected).await;
        if poll_tail {
            assert!(futures_util::poll!(stream.next()).is_pending());
            assert_eq!(entered.try_recv(), Ok(()));
        } else {
            assert_eq!(entered.try_recv(), Err(oneshot::error::TryRecvError::Empty));
        }
        assert_eq!(dropped.try_recv(), Err(oneshot::error::TryRecvError::Empty));
        assert!(recorder.spans()[0].end_sequence.is_none());

        drop(stream);

        assert_eq!(dropped.try_recv(), Ok(()));
        assert_eq!(eof.try_recv(), Err(oneshot::error::TryRecvError::Closed));
        assert!(release.send(()).is_err());
        assert_eq!(auth_calls.load(Ordering::SeqCst), 1);
        assert!(!cancel.is_cancelled());
        assert!(
            recorder.spans()[0].end_sequence.is_some(),
            "direct Drop must settle the cached result (tail polled: {poll_tail})"
        );
        assert_deferred_poll_settled(&recorder, StopReason::Stop);
        assert_eq!(recorder.measurements().len(), 7);
    }
}

#[tokio::test]
async fn deferred_hard_abort_preserves_success_while_the_source_tail_is_held() {
    for read in [
        DeferredDoneRead::Unread,
        DeferredDoneRead::Started,
        DeferredDoneRead::Direct,
    ] {
        let cancel = CancellationToken::new();
        let (expected, terminal) = deferred_ready_terminal();
        let GatedTail {
            source,
            entered,
            release,
            mut eof,
            mut dropped,
        } = gated_tail(terminal, cancel.clone());
        let recorder = Recorder::default();
        let (stream, auth_calls) = deferred_call(source, cancel.clone(), &recorder);
        let mut started = start_deferred_at_held_tail(stream, entered, read, &expected).await;
        assert!(recorder.spans()[0].end_sequence.is_none());

        assert_eq!(bounded(started.abort()).await, Ok(expected.clone()));
        assert_eq!(dropped.try_recv(), Ok(()));
        assert!(matches!(
            eof.try_recv(),
            Err(oneshot::error::TryRecvError::Closed)
        ));
        assert!(release.send(()).is_err());
        assert!(!cancel.is_cancelled());
        assert!(bounded(started.next()).await.is_none());
        assert_eq!(bounded(started.result()).await, Ok(expected));
        assert_eq!(auth_calls.load(Ordering::SeqCst), 1);
        assert_deferred_poll_settled(&recorder, StopReason::Stop);
    }
}

#[tokio::test]
async fn deferred_abandoned_abort_wait_settles_when_iteration_or_result_joins() {
    for committed in [false, true] {
        for iterate in [false, true] {
            let (expected, terminal) = deferred_ready_terminal();
            let (source, tx, mut dropped) = channel_source();
            let cancel = CancellationToken::new();
            let recorder = Recorder::default();
            let (stream, auth_calls) = deferred_call(source, cancel.clone(), &recorder);
            let mut started = stream.start();
            tx.send(Ok(delta(0))).await.unwrap();
            if committed {
                bounded(tx.send(terminal)).await.unwrap();
                assert_deferred_done(&mut started, &expected).await;
            } else {
                assert!(matches!(
                    bounded(started.next()).await,
                    Some(Ok(AssistantStreamEvent::TextDelta { delta, .. })) if delta == "0"
                ));
            }
            assert!(futures_util::poll!(started.next()).is_pending());
            {
                let mut waiting = std::pin::pin!(started.abort());
                assert!(futures_util::poll!(waiting.as_mut()).is_pending());
            }

            let iterated_failure = if iterate && !committed {
                Some(bounded(started.next()).await.unwrap().unwrap_err())
            } else {
                None
            };
            if iterate {
                assert!(bounded(started.next()).await.is_none());
                assert_eq!(dropped.try_recv(), Ok(()));
            }
            let outcome = bounded(started.result()).await;
            if committed {
                assert_eq!(outcome, Ok(expected));
                assert_deferred_poll_settled(&recorder, StopReason::Stop);
            } else {
                let failure = outcome.unwrap_err();
                assert!(matches!(failure.cause(), ProviderError::Cancelled));
                assert_eq!(failure.message().stop_reason, StopReason::Aborted);
                if let Some(iterated) = iterated_failure {
                    assert_eq!(iterated, failure);
                }
                assert_deferred_poll_settled(&recorder, StopReason::Aborted);
            }
            if !iterate {
                assert_eq!(dropped.try_recv(), Ok(()));
            }
            assert!(tx.is_closed());
            assert!(!cancel.is_cancelled());
            assert_eq!(auth_calls.load(Ordering::SeqCst), 1);
        }
    }
}

#[test]
fn deferred_runtime_shutdown_preserves_committed_success_for_both_waiters() {
    let waiter = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    for read in [
        DeferredDoneRead::Unread,
        DeferredDoneRead::Started,
        DeferredDoneRead::Direct,
    ] {
        for iterate in [false, true] {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .unwrap();
            let cancel = CancellationToken::new();
            let (expected, terminal) = deferred_ready_terminal();
            let GatedTail {
                source,
                entered,
                release,
                mut eof,
                mut dropped,
            } = gated_tail(terminal, cancel.clone());
            let recorder = Recorder::default();
            let (stream, auth_calls) = deferred_call(source, cancel.clone(), &recorder);
            let mut started = runtime.block_on(start_deferred_at_held_tail(
                stream, entered, read, &expected,
            ));
            assert!(recorder.spans()[0].end_sequence.is_none());
            drop(runtime);

            assert_eq!(dropped.try_recv(), Ok(()));
            assert!(matches!(
                eof.try_recv(),
                Err(oneshot::error::TryRecvError::Closed)
            ));
            assert!(release.send(()).is_err());
            waiter.block_on(async {
                if iterate {
                    if matches!(read, DeferredDoneRead::Unread) {
                        assert_deferred_done(&mut started, &expected).await;
                    }
                    assert!(bounded(started.next()).await.is_none());
                    assert_deferred_poll_settled(&recorder, StopReason::Stop);
                }
                assert_eq!(bounded(started.result()).await, Ok(expected));
            });
            assert!(!cancel.is_cancelled());
            assert_eq!(auth_calls.load(Ordering::SeqCst), 1);
            assert_deferred_poll_settled(&recorder, StopReason::Stop);
        }
    }
}

#[test]
fn deferred_runtime_shutdown_before_done_keeps_typed_interruption() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let (source, tx, mut dropped) = channel_source();
    let cancel = CancellationToken::new();
    let recorder = Recorder::default();
    let (stream, auth_calls) = deferred_call(source, cancel.clone(), &recorder);
    let mut started = runtime.block_on(async {
        let mut started = stream.start();
        tx.send(Ok(delta(0))).await.unwrap();
        assert!(matches!(
            bounded(started.next()).await,
            Some(Ok(AssistantStreamEvent::TextDelta { delta, .. })) if delta == "0"
        ));
        started
    });
    drop(runtime);
    assert_eq!(dropped.try_recv(), Ok(()));
    assert!(tx.is_closed());
    assert!(!cancel.is_cancelled());
    assert_eq!(auth_calls.load(Ordering::SeqCst), 1);
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async {
            let failure = bounded(started.next()).await.unwrap().unwrap_err();
            assert!(matches!(failure.cause(), ProviderError::StreamError(_)));
            assert_eq!(failure.message().stop_reason, StopReason::Error);
            assert!(bounded(started.next()).await.is_none());
            assert_eq!(bounded(started.result()).await, Err(failure));
        });
    let spans = recorder.spans();
    assert_eq!(spans.len(), 1);
    assert!(spans[0].end_sequence.is_none());
}

#[tokio::test]
async fn deferred_tail_panic_is_resumed_after_done() {
    use futures_util::FutureExt;

    for join in ["eof", "result", "abort"] {
        let (expected, terminal) = deferred_ready_terminal();
        let source = futures_util::stream::iter([Ok(delta(0)), terminal]).chain(
            futures_util::stream::poll_fn(|_| -> Poll<Option<Item>> {
                panic!("deferred tail panic")
            }),
        );
        let (stream, _) = deferred_call(
            Box::pin(source),
            CancellationToken::new(),
            &Recorder::default(),
        );
        let mut started = stream.start();
        assert_deferred_done(&mut started, &expected).await;
        let joined = std::panic::AssertUnwindSafe(async {
            match join {
                "eof" => drop(started.next().await),
                "result" => drop(started.result().await),
                "abort" => drop(started.abort().await),
                _ => unreachable!(),
            }
        })
        .catch_unwind()
        .await;
        let panic = joined.expect_err("tail panic must remain observable");
        assert_eq!(panic.downcast_ref::<&str>(), Some(&"deferred tail panic"));
    }
}

#[tokio::test]
async fn deferred_consumer_can_return_at_done_while_result_waits_for_tail() {
    for read_done in [true, false] {
        let cancel = CancellationToken::new();
        let (expected, terminal) = deferred_ready_terminal();
        let GatedTail {
            source,
            entered,
            release,
            mut eof,
            mut dropped,
        } = gated_tail(terminal, cancel.clone());
        let recorder = Recorder::default();
        let (stream, auth_calls) = deferred_call(source, cancel.clone(), &recorder);
        let mut started = stream.start();
        bounded(entered)
            .await
            .expect("source reaches the held tail");
        let mut consumer = std::pin::pin!(async move {
            if read_done {
                while let Some(event) = started.next().await {
                    if let AssistantStreamEvent::Done { message, .. } = event? {
                        return Ok(message);
                    }
                }
            }
            started.result().await
        });

        let held = futures_util::poll!(consumer.as_mut());
        assert_eq!(held.is_ready(), read_done);
        if let Poll::Ready(outcome) = &held {
            assert_eq!(outcome.as_ref(), Ok(&expected));
        }
        assert_eq!(eof.try_recv(), Err(oneshot::error::TryRecvError::Empty));
        assert_eq!(dropped.try_recv(), Err(oneshot::error::TryRecvError::Empty));
        assert!(recorder.spans()[0].end_sequence.is_none());

        release.send(()).expect("release the source tail");
        let outcome = match held {
            Poll::Ready(outcome) => outcome,
            Poll::Pending => bounded(consumer).await,
        };
        assert_eq!(outcome, Ok(expected));
        bounded(dropped).await.expect("worker releases the source");
        assert_eq!(eof.try_recv(), Ok(()));
        assert_eq!(auth_calls.load(Ordering::SeqCst), 1);
        assert!(!cancel.is_cancelled());
        assert_deferred_poll_settled(&recorder, StopReason::Stop);
    }
}

#[tokio::test]
async fn deferred_ready_returns_the_normalized_outcome_at_the_held_tail() {
    for (stop, cancel_tail) in [
        (StopReason::Deferred, false),
        (StopReason::Stop, false),
        (StopReason::Error, false),
        (StopReason::Aborted, false),
        (StopReason::Stop, true),
    ] {
        let handle = deferred_handle();
        let mut expected = base_assistant();
        expected.stop_reason = stop;
        expected.usage.output_tokens = 47;
        expected.deferred = (stop == StopReason::Deferred).then(|| Box::new(handle.clone()));
        let expected_cause = match stop {
            StopReason::Error => Some(ProviderError::StreamError(ProviderErrorSummary::redacted())),
            StopReason::Aborted => Some(ProviderError::Cancelled),
            _ => None,
        };
        let terminal = if expected_cause.is_some() {
            expected.error_message = Some("legacy failure".into());
            AssistantStreamEvent::Error {
                reason: stop,
                message: expected.clone(),
            }
        } else {
            AssistantStreamEvent::Done {
                reason: stop,
                message: expected.clone(),
            }
        };
        let GatedTail {
            source,
            entered,
            release,
            mut eof,
            mut dropped,
        } = gated_tail(Ok(terminal), CancellationToken::new());
        let recorder = Recorder::default();
        let cancel = CancellationToken::new();
        let (models, auth_calls) = deferred_models(source, &handle, 37);
        let mut ready = std::pin::pin!(models.fetch_deferred_ready(
            &handle,
            DeferredFetchCall {
                wait_ms: 37,
                cancel: cancel.clone(),
                telemetry_context: Some(recorder.context()),
                ..Default::default()
            },
        ));
        assert!(futures_util::poll!(ready.as_mut()).is_pending());
        bounded(entered)
            .await
            .expect("worker reaches the held tail");
        let Poll::Ready(outcome) = futures_util::poll!(ready) else {
            panic!("the normalized {stop:?} outcome must be available at the held tail");
        };
        if let Some(cause) = expected_cause {
            let failure = outcome.unwrap_err();
            assert_eq!(failure.message(), &expected);
            assert_eq!(failure.cause(), &cause);
        } else {
            assert_eq!(outcome, Ok(expected));
        }
        assert_eq!(eof.try_recv(), Err(oneshot::error::TryRecvError::Empty));
        assert_eq!(dropped.try_recv(), Err(oneshot::error::TryRecvError::Empty));
        assert!(recorder.spans()[0].end_sequence.is_none());
        assert!(!cancel.is_cancelled());

        if cancel_tail {
            cancel.cancel();
            bounded(dropped)
                .await
                .expect("local cancellation releases the tail");
            assert_eq!(eof.try_recv(), Err(oneshot::error::TryRecvError::Closed));
        } else {
            release.send(()).expect("release the source tail");
            bounded(dropped).await.expect("worker releases the source");
            assert_eq!(eof.try_recv(), Ok(()));
        }
        assert_eq!(auth_calls.load(Ordering::SeqCst), 1);
        assert_deferred_poll_settled(&recorder, stop);
        if stop == StopReason::Aborted {
            assert_eq!(recorder.spans()[0].status, opi_telemetry::SpanStatus::Ok);
        }
        if stop == StopReason::Deferred {
            assert!(!recorder.measurements().iter().any(|record| matches!(
                record.definition.name.as_str(),
                "pi.ai.tokens" | "pi.ai.cost"
            )));
        }
    }
}

#[tokio::test]
async fn deferred_ready_preserves_typed_cancellation_before_a_terminal() {
    let (resource, _, dropped) = channel_source();
    let (waiting, entered) = oneshot::channel();
    let source = futures_util::stream::once(async move {
        let _ = waiting.send(());
        std::future::pending::<Item>().await
    })
    .chain(resource);
    let handle = deferred_handle();
    let (models, auth_calls) = deferred_models(Box::pin(source), &handle, 0);
    let recorder = Recorder::default();
    let cancel = CancellationToken::new();
    let mut ready = std::pin::pin!(models.fetch_deferred_ready(
        &handle,
        DeferredFetchCall {
            cancel: cancel.clone(),
            telemetry_context: Some(recorder.context()),
            ..Default::default()
        },
    ));
    assert!(futures_util::poll!(ready.as_mut()).is_pending());
    bounded(entered)
        .await
        .expect("fetch waits before its first item");
    assert!(futures_util::poll!(ready.as_mut()).is_pending());

    cancel.cancel();
    let failure = bounded(ready).await.unwrap_err();
    assert!(cancel.is_cancelled());
    assert_eq!(failure.cause(), &ProviderError::Cancelled);
    assert_eq!(failure.message().stop_reason, StopReason::Aborted);
    assert_eq!(failure.message().provider, handle.provider);
    assert_eq!(failure.message().model, handle.model_id);
    bounded(dropped)
        .await
        .expect("worker releases the cancelled source");
    assert_eq!(auth_calls.load(Ordering::SeqCst), 1);
    assert_deferred_poll_settled(&recorder, StopReason::Aborted);
}

#[tokio::test]
async fn deferred_ready_starts_on_poll_and_drains_after_the_waiter_is_dropped() {
    let (_, terminal) = deferred_ready_terminal();
    let tail = gated_tail(terminal, CancellationToken::new());
    let (waiting, entered) = oneshot::channel();
    let (allow_dispatch, dispatch_gate) = oneshot::channel();
    let source = futures_util::stream::once(async move {
        let _ = waiting.send(());
        dispatch_gate.await.expect("release the first item");
        Ok(delta(0))
    })
    .chain(tail.source);
    let handle = deferred_handle();
    let (models, auth_calls) = deferred_models(Box::pin(source), &handle, 0);
    let recorder = Recorder::default();
    let call = DeferredFetchCall {
        telemetry_context: Some(recorder.context()),
        ..Default::default()
    };
    drop(models.fetch_deferred_ready(&handle, call.clone()));
    assert_eq!(auth_calls.load(Ordering::SeqCst), 0);
    assert!(recorder.spans().is_empty());
    {
        let mut waiter = std::pin::pin!(models.fetch_deferred_ready(&handle, call));
        assert!(futures_util::poll!(waiter.as_mut()).is_pending());
        bounded(entered)
            .await
            .expect("the first item is still held");
        assert_eq!(auth_calls.load(Ordering::SeqCst), 1);
    }

    allow_dispatch.send(()).expect("release the first item");
    bounded(tail.entered)
        .await
        .expect("the worker reaches the tail without a waiter");
    tail.release.send(()).expect("release the source tail");
    bounded(tail.dropped)
        .await
        .expect("the worker releases the source");
    bounded(tail.eof).await.expect("the worker drains to EOF");
    assert_eq!(auth_calls.load(Ordering::SeqCst), 1);
    assert_deferred_poll_settled(&recorder, StopReason::Stop);
}

#[tokio::test]
#[should_panic(expected = "panic before deferred terminal")]
async fn deferred_ready_resumes_a_source_panic_before_the_terminal() {
    let source = futures_util::stream::poll_fn(|_| panic!("panic before deferred terminal"));
    let handle = deferred_handle();
    let (models, _) = deferred_models(Box::pin(source), &handle, 0);
    let _ = bounded(models.fetch_deferred_ready(&handle, Default::default())).await;
}

#[tokio::test]
async fn deferred_ready_exposes_failure_inputs_for_durable_consumers() {
    use opi_ai::message::{AssistantContent, is_context_overflow};
    use opi_ai::retry::{WholeCallRetryClassification, classify_whole_call_retry};

    let quota = "429 GoUsageLimitError: Monthly usage limit reached";
    for (name, stop, diagnostic, cause) in [
        (
            "transient",
            StopReason::Error,
            Some("503 Service Unavailable"),
            None,
        ),
        ("quota", StopReason::Error, Some(quota), None),
        (
            "workspace-quota",
            StopReason::Error,
            Some(
                r#"OpenAI API error (429): {"code":"subscription_sharing_workplace_individual_usage_limit_exceeded"}"#,
            ),
            None,
        ),
        (
            "zhipu-quota",
            StopReason::Error,
            Some(r#"429 {"code":"1308","message":"Usage limit reached"}"#),
            Some(ProviderError::RateLimited {
                retry_after_ms: None,
            }),
        ),
        (
            "gateway",
            StopReason::Error,
            Some("upstream_stream_read_error"),
            None,
        ),
        (
            "context-window",
            StopReason::Error,
            Some("Context Window exceeded for given call."),
            None,
        ),
        (
            "request-size",
            StopReason::Error,
            Some("github-copilot API error (413): failed to parse request"),
            None,
        ),
        (
            "known-overflow",
            StopReason::Error,
            Some("prompt is too long: 250000 tokens > 200000 maximum"),
            None,
        ),
        (
            "provider-abort",
            StopReason::Aborted,
            Some("503 Service Unavailable"),
            None,
        ),
        (
            "typed-provider-abort",
            StopReason::Aborted,
            Some("503 Service Unavailable"),
            Some(ProviderError::Cancelled),
        ),
        (
            "typed-quota",
            StopReason::Error,
            Some(quota),
            Some(ProviderError::RateLimited {
                retry_after_ms: None,
            }),
        ),
        (
            "typed-timeout",
            StopReason::Error,
            None,
            Some(ProviderError::Timeout),
        ),
    ] {
        let handle = deferred_handle();
        let mut expected = base_assistant();
        expected.provider.clone_from(&handle.provider);
        expected.model.clone_from(&handle.model_id);
        expected.stop_reason = stop;
        expected.error_message = diagnostic.map(str::to_owned);
        expected.content.push(AssistantContent::Text {
            text: "partial".into(),
            signature: None,
        });
        let mut items = Vec::new();
        if let Some(cause) = cause.clone() {
            let mut partial = expected.clone();
            partial.stop_reason = StopReason::Pending;
            items.push(Ok(AssistantStreamEvent::Start { partial }));
            items.push(Err(cause));
        } else {
            items.push(Ok(AssistantStreamEvent::Error {
                reason: stop,
                message: expected.clone(),
            }));
        }
        let source = Box::pin(futures_util::stream::iter(items));
        let (models, auth_calls) = deferred_models(source, &handle, 0);
        let cancel = CancellationToken::new();
        let failure = bounded(models.fetch_deferred_ready(
            &handle,
            DeferredFetchCall {
                cancel: cancel.clone(),
                ..Default::default()
            },
        ))
        .await
        .unwrap_err();
        let (message, typed_cause) = failure.into_parts();
        assert_eq!(message.content, expected.content);
        assert_eq!(message.provider, handle.provider);
        assert_eq!(message.model, handle.model_id);
        assert_eq!(message.stop_reason, stop, "stop reason for {name}");
        let expected_cause = cause.unwrap_or_else(|| match stop {
            StopReason::Aborted => ProviderError::Cancelled,
            _ => ProviderError::StreamError(ProviderErrorSummary::redacted()),
        });
        assert_eq!(typed_cause, expected_cause, "cause for {name}");
        let retry = classify_whole_call_retry(&message);
        let expected_retry = match name {
            "transient" | "typed-timeout" | "gateway" => WholeCallRetryClassification::Transient,
            "provider-abort" | "typed-provider-abort" => WholeCallRetryClassification::Aborted,
            _ => WholeCallRetryClassification::Terminal,
        };
        assert_eq!(retry, expected_retry, "message classification for {name}");
        if let Some(diagnostic) = diagnostic {
            assert_eq!(message.error_message.as_deref(), Some(diagnostic));
        } else {
            assert_eq!(message.error_message.as_deref(), Some("request timed out"));
        }
        assert_eq!(auth_calls.load(Ordering::SeqCst), 1);
        assert!(!cancel.is_cancelled());
        let context_overflow = is_context_overflow(&message, None);
        assert_eq!(
            context_overflow,
            matches!(name, "context-window" | "request-size" | "known-overflow"),
            "overflow classification for {name}"
        );
        let cause = match &typed_cause {
            ProviderError::StreamError(_) => "StreamError",
            ProviderError::Cancelled => "Cancelled",
            ProviderError::RateLimited { .. } => "RateLimited",
            ProviderError::Timeout => "Timeout",
            _ => panic!("unexpected cause for {name}: {typed_cause:?}"),
        };
        println!(
            "deferred failure observation {}",
            serde_json::json!({
                "name": name,
                "inputStop": stop.as_str(),
                "stop": message.stop_reason.as_str(),
                "diagnostic": message.error_message,
                "cause": cause,
                "typedRetryable": typed_cause.is_retryable(),
                "messageRetry": format!("{retry:?}"),
                "contextOverflow": context_overflow,
                "tokenCancelled": cancel.is_cancelled(),
            })
        );
    }
}
