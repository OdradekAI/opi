//! Source release and attempt sealing for started, ordinary, and prepared calls.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use opi_ai::provider::{EventStream, ProviderError, Request};
use opi_ai::stream::{AssistantStreamEvent, ChatStream, StopReason};
use opi_ai::test_support::{MockProvider, base_assistant, single_route_collection};
use opi_ai::{ChatModelRecord, ModelInfo, Provider};
use opi_telemetry::memory::Recorder;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use super::fixtures::{
    ControlledProvider, GatedTail, bounded, channel_source, done, gated_tail, request,
};

fn ordinary_call(source: EventStream, request: Request) -> opi_ai::ChatCall {
    let (entered, _dispatched) = oneshot::channel();
    let model = MockProvider::new("execution", vec![]).models()[0].clone();
    let models = single_route_collection(Box::new(ControlledProvider {
        models: vec![model.clone()],
        source: Mutex::new(Some(source)),
        entered: Mutex::new(Some(entered)),
    }));
    models.stream_simple(
        ChatModelRecord::new("execution", model),
        request,
        Default::default(),
    )
}

struct RetryThenTailProvider {
    inner: ControlledProvider,
    setups: Arc<AtomicUsize>,
}

impl Provider for RetryThenTailProvider {
    fn id(&self) -> &str {
        self.inner.id()
    }

    fn models(&self) -> &[ModelInfo] {
        self.inner.models()
    }

    fn acquisition_policy_default(&self) -> opi_ai::retry::AcquisitionPolicy {
        opi_ai::retry::AcquisitionPolicy::with_max_retries(1)
    }

    fn stream_prepared(&self, request: Request, auth: opi_ai::ResolvedAuth) -> EventStream {
        let attempt = self.setups.fetch_add(1, Ordering::SeqCst) + 1;
        if attempt == 1 {
            // No preceding event: response handoff must not seal the retry yet.
            return Box::pin(futures_util::stream::iter([Err(
                ProviderError::RateLimited {
                    retry_after_ms: Some(0),
                },
            )]));
        }
        assert_eq!(attempt, 2, "tail work must not dispatch a third attempt");
        self.inner.stream_prepared(request, auth)
    }
}

#[tokio::test]
async fn abort_releases_an_uncooperative_source_and_preserves_partial() {
    let (source, tx, mut released) = channel_source();
    let mut started = ChatStream::new(source).start();
    let mut partial = base_assistant();
    partial.usage.output_tokens = 17;
    tx.send(Ok(AssistantStreamEvent::Start {
        partial: partial.clone(),
    }))
    .await
    .unwrap();
    started.next().await.unwrap().unwrap();
    let failure = bounded(started.abort()).await.unwrap_err();
    assert!(matches!(failure.cause(), ProviderError::Cancelled));
    assert_eq!(failure.message().stop_reason, StopReason::Aborted);
    assert_eq!(failure.message().usage, partial.usage);
    assert_eq!(released.try_recv(), Ok(()), "abort joins source release");
    assert!(tx.is_closed());
}

#[tokio::test]
async fn abort_preserves_completed_outcomes_before_and_after_terminal_consumption() {
    for consume_terminal in [false, true] {
        for fail in [false, true] {
            let (source, tx, released) = channel_source();
            let mut started = ChatStream::new(source).start();
            let expected = base_assistant();
            let event = if fail {
                Err(ProviderError::Timeout)
            } else {
                Ok(AssistantStreamEvent::Done {
                    reason: StopReason::Stop,
                    message: expected.clone(),
                })
            };
            tx.send(event).await.unwrap();
            bounded(released).await.unwrap();
            // Source release is before forwarding/join completion; observing
            // the next item also covers the cached consumer outcome.
            if consume_terminal {
                assert_eq!(started.next().await.unwrap().is_err(), fail);
            }
            let outcome = bounded(started.abort()).await;
            assert_eq!(outcome.is_err(), fail);
            match outcome {
                Ok(message) => assert_eq!(message, expected),
                Err(failure) => assert!(matches!(failure.cause(), ProviderError::Timeout)),
            }
        }
    }
}

#[tokio::test]
async fn abandoning_abort_wait_retains_the_join_and_cancellation_outcome() {
    for abort_again in [false, true] {
        let (source, _tx, mut released) = channel_source();
        let mut started = ChatStream::new(source).start();
        let mut waiting = Box::pin(started.abort());
        assert!(futures_util::poll!(&mut waiting).is_pending());
        drop(waiting);
        let outcome = if abort_again {
            bounded(started.abort()).await
        } else {
            bounded(started.result()).await
        };
        let failure = outcome.unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::Cancelled));
        assert_eq!(released.try_recv(), Ok(()));
    }
}

#[tokio::test]
async fn cancelling_after_consumer_drop_still_releases_the_source() {
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let (source, _tx, mut released) = channel_source();
    let cancellation = futures_util::stream::once(async move {
        // Keep the resource across the explicit cancellation wait.
        token.cancelled().await;
        drop(source);
        Err(ProviderError::Cancelled)
    });
    drop(ChatStream::new(Box::pin(cancellation)).start());
    assert!(!cancel.is_cancelled());
    assert!(matches!(
        released.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
    cancel.cancel();
    bounded(released).await.unwrap();
}

#[tokio::test]
async fn ordinary_business_terminal_waits_for_source_tail_and_settles_once() {
    for consume in [false, true] {
        for (reason, legacy) in [
            (StopReason::Stop, false),
            (StopReason::Error, false),
            (StopReason::Aborted, false),
            (StopReason::Error, true),
        ] {
            let mut expected = base_assistant();
            expected.stop_reason = reason;
            expected.usage.output_tokens = 17;
            let terminal = if legacy {
                AssistantStreamEvent::Error {
                    reason,
                    message: expected.clone(),
                }
            } else {
                AssistantStreamEvent::Done {
                    reason,
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
            let mut request = request(CancellationToken::new());
            request.telemetry_context = Some(recorder.context());
            let mut call = ordinary_call(source, request);
            if consume {
                assert!(matches!(
                    bounded(call.next()).await.unwrap().unwrap(),
                    AssistantStreamEvent::TextDelta { delta, .. } if delta == "0"
                ));
            }
            bounded(entered)
                .await
                .expect("business terminal must not truncate the source tail");
            assert!(matches!(
                eof.try_recv(),
                Err(oneshot::error::TryRecvError::Empty)
            ));
            assert!(matches!(
                dropped.try_recv(),
                Err(oneshot::error::TryRecvError::Empty)
            ));
            assert_eq!(recorder.spans().len(), 1);
            assert!(recorder.spans()[0].end_sequence.is_none());
            let outcome = if consume {
                assert!(futures_util::poll!(call.next()).is_pending());
                release.send(()).unwrap();
                let terminal = bounded(call.next()).await.unwrap();
                assert_eq!(terminal.is_ok(), reason == StopReason::Stop);
                assert!(bounded(call.next()).await.is_none());
                assert!(bounded(call.next()).await.is_none());
                let outcome = bounded(call.result()).await;
                match terminal {
                    Ok(AssistantStreamEvent::Done { message, .. }) => {
                        assert_eq!(outcome, Ok(message));
                    }
                    Err(failure) => assert_eq!(outcome, Err(failure)),
                    other => panic!("unexpected terminal: {other:?}"),
                }
                outcome
            } else {
                let mut result = Box::pin(call.result());
                assert!(futures_util::poll!(&mut result).is_pending());
                release.send(()).unwrap();
                bounded(result).await
            };
            let actual = match &outcome {
                Ok(message) => message,
                Err(failure) => failure.message(),
            };
            assert_eq!(outcome.is_ok(), reason == StopReason::Stop);
            assert_eq!(actual.stop_reason, expected.stop_reason);
            assert_eq!(actual.content, expected.content);
            assert_eq!(actual.usage, expected.usage);
            assert_eq!(eof.try_recv(), Ok(()));
            assert_eq!(dropped.try_recv(), Ok(()));
            let spans = recorder.spans();
            assert_eq!(spans.len(), 1);
            assert!(spans[0].end_sequence.is_some());
            assert_eq!(
                spans[0].attributes.get("stop"),
                Some(&opi_telemetry::AttributeValue::String(
                    reason.as_str().into()
                ))
            );
        }
    }
}

#[tokio::test]
async fn ordinary_admitted_retry_drains_only_the_successful_attempt_tail() {
    let mut message = base_assistant();
    message.stop_reason = StopReason::Length;
    message.usage.output_tokens = 23;
    let GatedTail {
        source,
        entered,
        release,
        mut eof,
        mut dropped,
    } = gated_tail(
        Ok(AssistantStreamEvent::Done {
            reason: StopReason::Length,
            message,
        }),
        CancellationToken::new(),
    );
    let setups = Arc::new(AtomicUsize::new(0));
    let (dispatched, _dispatch) = oneshot::channel();
    let model = MockProvider::new("execution", vec![]).models()[0].clone();
    let models = single_route_collection(Box::new(RetryThenTailProvider {
        inner: ControlledProvider {
            models: vec![model.clone()],
            source: Mutex::new(Some(source)),
            entered: Mutex::new(Some(dispatched)),
        },
        setups: setups.clone(),
    }));
    let mut call = models.stream_simple(
        ChatModelRecord::new("execution", model),
        request(CancellationToken::new()),
        Default::default(),
    );
    bounded(entered)
        .await
        .expect("admitted retry reaches its own source tail");
    assert_eq!(setups.load(Ordering::SeqCst), 2);
    assert!(matches!(
        bounded(call.next()).await.unwrap().unwrap(),
        AssistantStreamEvent::TextDelta { delta, .. } if delta == "0"
    ));
    assert!(futures_util::poll!(call.next()).is_pending());
    assert!(matches!(
        eof.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
    assert!(matches!(
        dropped.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
    release.send(()).unwrap();
    let AssistantStreamEvent::Done { reason, message } =
        bounded(call.next()).await.unwrap().unwrap()
    else {
        panic!("expected second attempt's business terminal");
    };
    assert_eq!(reason, StopReason::Length);
    assert_eq!(message.usage.output_tokens, 23);
    assert_eq!(eof.try_recv(), Ok(()));
    assert_eq!(dropped.try_recv(), Ok(()));
    assert!(call.next().await.is_none());
    assert_eq!(call.result().await, Ok(message));
    assert_eq!(setups.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn ordinary_abort_during_terminal_tail_joins_release_without_fabricating_eof() {
    let cancel = CancellationToken::new();
    let GatedTail {
        source,
        entered,
        release,
        mut eof,
        mut dropped,
    } = gated_tail(Ok(done()), cancel.clone());
    let recorder = Recorder::default();
    let mut request = request(cancel.clone());
    request.telemetry_context = Some(recorder.context());
    let mut call = ordinary_call(source, request);
    bounded(entered)
        .await
        .expect("source reaches terminal tail");
    let failure = bounded(call.abort()).await.unwrap_err();
    assert!(matches!(failure.cause(), ProviderError::Cancelled));
    assert_eq!(failure.message().stop_reason, StopReason::Aborted);
    assert_eq!(dropped.try_recv(), Ok(()));
    assert!(matches!(
        eof.try_recv(),
        Err(oneshot::error::TryRecvError::Closed)
    ));
    assert!(release.send(()).is_err());
    assert!(!cancel.is_cancelled());
    assert!(call.next().await.is_none());
    assert_eq!(call.result().await, Err(failure));
    let spans = recorder.spans();
    assert_eq!(spans.len(), 1);
    assert!(spans[0].end_sequence.is_some());
    assert_eq!(
        spans[0].attributes.get("stop"),
        Some(&opi_telemetry::AttributeValue::String("aborted".into()))
    );
}

#[tokio::test]
async fn ordinary_cooperative_cancellation_in_tail_preserves_first_terminal() {
    let cancel = CancellationToken::new();
    let GatedTail {
        source,
        entered,
        release: _release,
        mut eof,
        mut dropped,
    } = gated_tail(Ok(done()), cancel.clone());
    let call = ordinary_call(source, request(cancel.clone()));
    bounded(entered)
        .await
        .expect("source reaches terminal tail");
    cancel.cancel();
    assert_eq!(
        bounded(call.result()).await.unwrap().stop_reason,
        StopReason::Stop
    );
    assert_eq!(eof.try_recv(), Ok(()));
    assert_eq!(dropped.try_recv(), Ok(()));
}

#[tokio::test]
async fn ordinary_transport_failure_releases_without_polling_a_tail() {
    let GatedTail {
        source,
        entered,
        release,
        eof,
        dropped,
    } = gated_tail(Err(ProviderError::Timeout), CancellationToken::new());
    let failure = bounded(ordinary_call(source, request(CancellationToken::new())).result())
        .await
        .unwrap_err();
    assert!(matches!(failure.cause(), ProviderError::Timeout));
    assert!(entered.await.is_err());
    assert!(eof.await.is_err());
    assert!(release.send(()).is_err());
    bounded(dropped).await.unwrap();
}

#[tokio::test]
async fn ordinary_and_prepared_calls_retain_execution_after_consumer_drop() {
    for advanced in [false, true] {
        let (source, tx, released) = channel_source();
        let (entered, dispatched) = oneshot::channel();
        let model = MockProvider::new("execution", vec![]).models()[0].clone();
        let models = single_route_collection(Box::new(ControlledProvider {
            models: vec![model.clone()],
            source: Mutex::new(Some(source)),
            entered: Mutex::new(Some(entered)),
        }));
        let cancel = CancellationToken::new();
        let prepared;
        let started = if advanced {
            prepared = Some(
                models
                    .prepare_call("execution:mock-model", request(cancel.clone()))
                    .await
                    .unwrap(),
            );
            ChatStream::new(prepared.as_ref().unwrap().start_attempt().unwrap()).start()
        } else {
            prepared = None;
            models
                .stream_simple(
                    ChatModelRecord::new("execution", model),
                    request(cancel.clone()),
                    Default::default(),
                )
                .start()
        };
        bounded(dispatched).await.unwrap(); // no event consumer has been polled
        drop(started);
        assert!(!cancel.is_cancelled());
        if let Some(prepared) = &prepared {
            assert!(
                matches!(
                    prepared.start_attempt(),
                    Err(opi_ai::ModelsError::AttemptAlreadyActive)
                ),
                "attempt still owned"
            );
        }
        bounded(tx.send(Ok(done()))).await.unwrap();
        drop(tx); // Ordinary calls drain; raw started attempts still release at Done.
        bounded(released).await.unwrap();
        if let Some(prepared) = &prepared {
            // Terminal delivery releases the slot but seals acquisition: it
            // must not allow a duplicate request or report unknown dispatch.
            assert_eq!(
                prepared.acquisition_seal(),
                Some(opi_ai::retry::AcquisitionSeal::PostHandoff)
            );
            assert!(matches!(
                prepared.start_attempt(),
                Err(opi_ai::ModelsError::CallSealed {
                    seal: opi_ai::retry::AcquisitionSeal::PostHandoff
                })
            ));
        }
    }
}

#[tokio::test]
async fn abort_does_not_cancel_the_request_token_or_reopen_unknown_dispatch() {
    let (source, _tx, mut released) = channel_source();
    let (entered, dispatched) = oneshot::channel();
    let model = MockProvider::new("execution", vec![]).models()[0].clone();
    let models = single_route_collection(Box::new(ControlledProvider {
        models: vec![model],
        source: Mutex::new(Some(source)),
        entered: Mutex::new(Some(entered)),
    }));
    let cancel = CancellationToken::new();
    let prepared = models
        .prepare_call("execution:mock-model", request(cancel.clone()))
        .await
        .unwrap();
    let mut started = ChatStream::new(prepared.start_attempt().unwrap()).start();
    bounded(dispatched).await.unwrap();
    assert!(matches!(
        started.abort().await.unwrap_err().cause(),
        ProviderError::Cancelled
    ));
    assert_eq!(released.try_recv(), Ok(()));
    assert!(
        !cancel.is_cancelled(),
        "abort does not revoke other token users"
    );
    assert_eq!(
        prepared.acquisition_seal(),
        Some(opi_ai::retry::AcquisitionSeal::UnknownDispatch)
    );
    assert!(matches!(
        prepared.start_attempt(),
        Err(opi_ai::ModelsError::CallSealed {
            seal: opi_ai::retry::AcquisitionSeal::UnknownDispatch
        })
    ));
    cancel.cancel();
    assert!(matches!(
        prepared.start_attempt(),
        Err(opi_ai::ModelsError::CallCancelled)
    ));
}
