//! Explicit execution owns progress; its handle owns only event consumption.

use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll};
use std::time::Duration;

use futures_core::Stream;
use futures_util::StreamExt;
use opi_ai::provider::{EventStream, ProviderError, Request};
use opi_ai::stream::{AssistantStreamEvent, ChatStream, StopReason};
use opi_ai::test_support::{MockProvider, base_assistant, single_route_collection};
use opi_ai::{ChatModelRecord, ModelInfo, Provider};
use opi_telemetry::memory::Recorder;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

type Item = Result<AssistantStreamEvent, ProviderError>;

struct ChannelSource {
    events: mpsc::Receiver<Item>,
    dropped: Option<oneshot::Sender<()>>,
}

impl Stream for ChannelSource {
    type Item = Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Item>> {
        self.get_mut().events.poll_recv(cx)
    }
}

impl Drop for ChannelSource {
    fn drop(&mut self) {
        let _ = self.dropped.take().unwrap().send(());
    }
}

fn channel_source() -> (EventStream, mpsc::Sender<Item>, oneshot::Receiver<()>) {
    let (tx, events) = mpsc::channel(1);
    let (dropped, released) = oneshot::channel();
    (
        Box::pin(ChannelSource {
            events,
            dropped: Some(dropped),
        }),
        tx,
        released,
    )
}

async fn bounded<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(5), future)
        .await
        .expect("controlled operation must make progress")
}

fn delta(index: usize) -> AssistantStreamEvent {
    AssistantStreamEvent::TextDelta {
        content_index: 0,
        delta: index.to_string(),
        partial: base_assistant(),
    }
}

fn done() -> AssistantStreamEvent {
    AssistantStreamEvent::Done {
        reason: StopReason::Stop,
        message: base_assistant(),
    }
}

#[tokio::test]
async fn start_runs_without_consumption_and_replays_the_complete_ordered_buffer() {
    let (source, tx, released) = channel_source();
    let mut started = ChatStream::new(source).start();
    // A bounded output channel would deadlock this producer before completion.
    for n in 0..257 {
        bounded(tx.send(Ok(delta(n)))).await.unwrap();
    }
    let terminal = done();
    bounded(tx.send(Ok(terminal.clone()))).await.unwrap();
    bounded(released).await.unwrap();
    for n in 0..257 {
        let event = started.next().await.unwrap().unwrap();
        assert!(
            matches!(event, AssistantStreamEvent::TextDelta { delta, .. } if delta == n.to_string())
        );
    }
    assert_eq!(started.next().await.unwrap().unwrap(), terminal);
    assert!(started.next().await.is_none());
    assert!(started.next().await.is_none());
    let AssistantStreamEvent::Done { message, .. } = terminal else {
        unreachable!()
    };
    assert_eq!(started.result().await.unwrap(), message);
}

#[tokio::test]
async fn dropping_started_consumer_before_or_after_first_event_keeps_source_alive() {
    for consume_first in [false, true] {
        let (source, tx, mut released) = channel_source();
        let mut started = ChatStream::new(source).start();
        if consume_first {
            tx.send(Ok(delta(0))).await.unwrap();
            assert!(started.next().await.unwrap().is_ok());
        }
        drop(started);
        assert!(matches!(
            released.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ));
        for n in 1..129 {
            bounded(tx.send(Ok(delta(n)))).await.unwrap();
        }
        bounded(tx.send(Ok(done()))).await.unwrap();
        bounded(released).await.unwrap();
    }
}

#[tokio::test]
async fn abandoning_result_wait_does_not_abandon_execution() {
    let (source, tx, released) = channel_source();
    let mut result = Box::pin(ChatStream::new(source).start().result());
    assert!(futures_util::poll!(&mut result).is_pending());
    drop(result);
    for n in 0..129 {
        bounded(tx.send(Ok(delta(n)))).await.unwrap();
    }
    bounded(tx.send(Ok(done()))).await.unwrap();
    bounded(released).await.unwrap();
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
async fn abort_retains_unread_partial_and_does_not_hide_source_panic() {
    use futures_util::FutureExt;
    for panic in [false, true] {
        let mut partial = base_assistant();
        partial.usage.output_tokens = 29;
        let (reached, ready) = oneshot::channel();
        let source = futures_util::stream::iter([Ok(AssistantStreamEvent::Start {
            partial: partial.clone(),
        })])
        .chain(futures_util::stream::once(async move {
            reached.send(()).unwrap();
            if panic {
                panic!("abort-source-panic");
            }
            std::future::pending::<Item>().await
        }));
        let mut started = ChatStream::new(Box::pin(source)).start();
        bounded(ready).await.unwrap();
        let outcome = std::panic::AssertUnwindSafe(started.abort())
            .catch_unwind()
            .await;
        if panic {
            assert_eq!(
                outcome.unwrap_err().downcast_ref::<&str>(),
                Some(&"abort-source-panic")
            );
        } else {
            let failure = outcome.unwrap().unwrap_err();
            assert_eq!(failure.message().usage, partial.usage);
            assert!(matches!(failure.cause(), ProviderError::Cancelled));
            assert!(started.next().await.is_none());
            assert_eq!(started.result().await, Err(failure));
        }
    }
}

#[tokio::test]
async fn explicit_token_cancellation_preserves_partial_and_aborted_outcome() {
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let mut partial = base_assistant();
    partial.usage.input_tokens = 42;
    let source = futures_util::stream::iter([Ok(AssistantStreamEvent::Start {
        partial: partial.clone(),
    })])
    .chain(futures_util::stream::once(async move {
        token.cancelled().await;
        Err(ProviderError::Cancelled)
    }));
    let mut started = ChatStream::new(Box::pin(source)).start();
    assert!(started.next().await.unwrap().is_ok());
    assert!(!cancel.is_cancelled());
    cancel.cancel();
    let failure = bounded(started.result()).await.unwrap_err();
    assert!(matches!(failure.cause(), ProviderError::Cancelled));
    assert_eq!(failure.message().stop_reason, StopReason::Aborted);
    assert_eq!(failure.message().usage, partial.usage);
}

#[tokio::test]
async fn cancellation_preserves_already_forwarded_events_before_its_failure() {
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let (waiting, ready) = oneshot::channel();
    let source = futures_util::stream::iter([Ok(delta(1)), Ok(delta(2))]).chain(
        futures_util::stream::once(async move {
            let _ = waiting.send(());
            token.cancelled().await;
            Err(ProviderError::Cancelled)
        }),
    );
    let mut started = ChatStream::new(Box::pin(source)).start();
    bounded(ready).await.unwrap();
    cancel.cancel();
    for expected in ["1", "2"] {
        assert!(
            matches!(started.next().await.unwrap().unwrap(), AssistantStreamEvent::TextDelta { delta, .. } if delta == expected)
        );
    }
    let failure = bounded(started.next()).await.unwrap().unwrap_err();
    assert!(matches!(failure.cause(), ProviderError::Cancelled));
    assert!(started.next().await.is_none());
    assert_eq!(started.result().await, Err(failure));
}

#[test]
fn starting_a_partially_consumed_source_keeps_its_snapshot_on_runtime_shutdown() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let mut partial = base_assistant();
    partial.usage.output_tokens = 17;
    let source = futures_util::stream::iter([Ok(AssistantStreamEvent::Start {
        partial: partial.clone(),
    })])
    .chain(futures_util::stream::pending());
    let mut stream = ChatStream::new(Box::pin(source));
    let started = runtime.block_on(async {
        assert!(stream.next().await.unwrap().is_ok());
        stream.start() // task is not given a first poll before this runtime stops
    });
    drop(runtime);
    let waiter = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let failure = waiter.block_on(started.result()).unwrap_err();
    assert_eq!(failure.message().usage, partial.usage);
}

#[tokio::test]
async fn start_preserves_partial_iteration_and_an_already_delivered_result() {
    for already_terminal in [false, true] {
        let terminal = done();
        let mut stream = ChatStream::new(Box::pin(futures_util::stream::iter([
            Ok(delta(0)),
            Ok(terminal.clone()),
            Ok(delta(999)),
        ])));
        assert!(stream.next().await.unwrap().is_ok());
        if already_terminal {
            assert_eq!(stream.next().await.unwrap().unwrap(), terminal);
        }
        let mut started = stream.start();
        if !already_terminal {
            assert_eq!(started.next().await.unwrap().unwrap(), terminal);
        }
        assert!(started.next().await.is_none());
        let AssistantStreamEvent::Done { message, .. } = terminal else {
            unreachable!()
        };
        assert_eq!(started.result().await.unwrap(), message);
    }
}

#[tokio::test]
async fn started_failure_has_one_terminal_and_cached_result() {
    let (source, tx, released) = channel_source();
    let mut started = ChatStream::new(source).start();
    tx.send(Ok(delta(0))).await.unwrap();
    drop(tx); // premature EOF must still become the existing protocol failure
    bounded(released).await.unwrap();
    assert!(started.next().await.unwrap().is_ok());
    let failure = started.next().await.unwrap().unwrap_err();
    assert!(matches!(failure.cause(), ProviderError::StreamError(_)));
    assert!(started.next().await.is_none());
    assert_eq!(started.result().await, Err(failure));
}

#[tokio::test]
async fn execution_panic_is_resumed_by_both_waiting_interfaces() {
    use futures_util::FutureExt;
    for iterate in [false, true] {
        let source = futures_util::stream::poll_fn(|_| -> Poll<Option<Item>> {
            panic!("source-panic-marker")
        });
        let mut started = ChatStream::new(Box::pin(source)).start();
        let result = std::panic::AssertUnwindSafe(async move {
            if iterate {
                let _ = started.next().await;
            } else {
                let _ = started.result().await;
            }
        })
        .catch_unwind()
        .await;
        let panic = result.expect_err("source panic must remain observable");
        assert_eq!(panic.downcast_ref::<&str>(), Some(&"source-panic-marker"));
    }
}

#[test]
fn start_requires_a_runtime_but_lazy_construction_does_not() {
    let lazy = ChatStream::new(Box::pin(futures_util::stream::pending()));
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lazy.start())).is_err());
}

#[test]
fn runtime_shutdown_is_a_failure_for_an_unfinished_execution() {
    for iterate in [false, true] {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let mut started = {
            let _guard = runtime.enter();
            ChatStream::new(Box::pin(futures_util::stream::pending())).start()
        };
        drop(runtime);
        let waiter = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        waiter.block_on(async {
            if iterate {
                let failure = started.next().await.unwrap().unwrap_err();
                assert!(matches!(failure.cause(), ProviderError::StreamError(_)));
                assert!(started.next().await.is_none());
                assert_eq!(started.result().await, Err(failure));
            } else {
                let failure = started.result().await.unwrap_err();
                assert!(matches!(failure.cause(), ProviderError::StreamError(_)));
            }
        });
    }
}

#[test]
fn runtime_shutdown_retains_produced_partial_even_when_it_was_not_consumed() {
    for consume in [false, true] {
        for iterate in [false, true] {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap();
            let mut partial = base_assistant();
            partial.usage.input_tokens = 31;
            partial
                .content
                .push(opi_ai::message::AssistantContent::Text {
                    text: "retained".into(),
                    signature: None,
                });
            let (entered, produced) = oneshot::channel();
            let event = AssistantStreamEvent::Start {
                partial: partial.clone(),
            };
            let source = futures_util::stream::once(async move {
                let _ = entered.send(());
                Ok(event)
            })
            .chain(futures_util::stream::pending());
            let mut started = runtime.block_on(async {
                let mut started = ChatStream::new(Box::pin(source)).start();
                produced.await.unwrap();
                if consume {
                    assert!(started.next().await.unwrap().is_ok());
                }
                started
            });
            drop(runtime);
            let waiter = tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap();
            waiter.block_on(async {
                let failure = if iterate {
                    if !consume {
                        assert!(started.next().await.unwrap().is_ok());
                    }
                    started.next().await.unwrap().unwrap_err()
                } else {
                    started.result().await.unwrap_err()
                };
                assert_eq!(failure.message().content, partial.content);
                assert_eq!(failure.message().usage, partial.usage);
                assert_eq!(failure.message().provider, partial.provider);
                assert_eq!(failure.message().stop_reason, StopReason::Error);
            });
        }
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
async fn started_models_setup_failure_is_one_typed_outcome() {
    let models = opi_ai::Models::new();
    let model = MockProvider::new("missing", vec![]).models()[0].clone();
    let recorder = Recorder::default();
    let mut request = request(CancellationToken::new());
    request.telemetry_context = Some(recorder.context());
    let mut started = models
        .stream_simple(
            ChatModelRecord::new("missing", model),
            request,
            Default::default(),
        )
        .start();
    let failure = bounded(started.next()).await.unwrap().unwrap_err();
    assert!(matches!(failure.cause(), ProviderError::Config(_)));
    assert!(started.next().await.is_none());
    assert_eq!(started.result().await, Err(failure));
    let spans = recorder.spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "pi.ai.request");
    assert_eq!(
        spans[0].attributes.get("stop"),
        Some(&opi_telemetry::AttributeValue::String("error".into()))
    );
    assert!(spans[0].end_sequence.is_some());
}

fn request(cancel: CancellationToken) -> Request {
    Request {
        model: "execution:mock-model".into(),
        context: opi_ai::transcript::normalize_context(Default::default()).unwrap(),
        projection: Default::default(),
        transport: Default::default(),
        max_tokens: None,
        temperature: None,
        sampling_params: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

struct ControlledProvider {
    models: Vec<ModelInfo>,
    source: Mutex<Option<EventStream>>,
    entered: Mutex<Option<oneshot::Sender<()>>>,
}

#[tokio::test]
async fn ordinary_entries_execute_after_unpolled_call_or_result_is_dropped() {
    for complete in [false, true] {
        let (source, tx, released) = channel_source();
        let (entered, dispatched) = oneshot::channel();
        let model = MockProvider::new("execution", vec![]).models()[0].clone();
        let models = single_route_collection(Box::new(ControlledProvider {
            models: vec![model.clone()],
            source: Mutex::new(Some(source)),
            entered: Mutex::new(Some(entered)),
        }));
        let cancel = CancellationToken::new();
        let record = ChatModelRecord::new("execution", model);
        if complete {
            drop(models.complete_simple(record, request(cancel.clone()), Default::default()));
        } else {
            drop(models.stream_simple(record, request(cancel.clone()), Default::default()));
        }
        drop(models);
        bounded(dispatched)
            .await
            .expect("setup runs without a consumer");
        assert!(!cancel.is_cancelled());
        for n in 0..129 {
            bounded(tx.send(Ok(delta(n)))).await.unwrap();
        }
        bounded(tx.send(Ok(done()))).await.unwrap();
        bounded(released).await.unwrap();
    }
}

#[test]
fn ordinary_stream_and_completion_require_runtime_at_construction() {
    let models = opi_ai::Models::new();
    let model = MockProvider::new("missing", vec![]).models()[0].clone();
    for complete in [false, true] {
        let record = ChatModelRecord::new("missing", model.clone());
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if complete {
                    drop(models.complete_simple(
                        record,
                        request(CancellationToken::new()),
                        Default::default(),
                    ));
                } else {
                    drop(models.stream_simple(
                        record,
                        request(CancellationToken::new()),
                        Default::default(),
                    ));
                }
            }))
            .is_err()
        );
    }
}

#[test]
fn ordinary_call_observes_shutdown_before_setup_and_can_be_consumed_on_another_thread() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let models = opi_ai::Models::new();
    let model = MockProvider::new("missing", vec![]).models()[0].clone();
    let call = {
        let _guard = runtime.enter();
        models.stream_simple(
            ChatModelRecord::new("missing", model),
            request(CancellationToken::new()),
            Default::default(),
        )
    };
    drop(runtime);
    // start is now a handle conversion, not a second spawn on this thread.
    let failure = std::thread::spawn(move || {
        let mut started = call.start();
        let waker = futures_util::task::noop_waker();
        let mut cx = Context::from_waker(&waker);
        let Poll::Ready(Some(Err(failure))) = Pin::new(&mut started).poll_next(&mut cx) else {
            panic!("completed runtime shutdown must be observable without another runtime");
        };
        assert!(matches!(
            Pin::new(&mut started).poll_next(&mut cx),
            Poll::Ready(None)
        ));
        failure
    })
    .join()
    .unwrap();
    assert!(matches!(failure.cause(), ProviderError::StreamError(_)));
}

#[tokio::test]
async fn ordinary_abort_after_partial_consumption_retains_one_execution_and_outcome() {
    let (source, tx, mut released) = channel_source();
    let (entered, dispatched) = oneshot::channel();
    let model = MockProvider::new("execution", vec![]).models()[0].clone();
    let models = single_route_collection(Box::new(ControlledProvider {
        models: vec![model.clone()],
        source: Mutex::new(Some(source)),
        entered: Mutex::new(Some(entered)),
    }));
    let token = CancellationToken::new();
    let mut call = models.stream_simple(
        ChatModelRecord::new("execution", model),
        request(token.clone()),
        Default::default(),
    );
    bounded(dispatched).await.unwrap();
    let mut partial = base_assistant();
    partial.content = vec![opi_ai::message::AssistantContent::Text {
        text: "retained ordinary partial".into(),
        signature: None,
    }];
    partial.usage.output_tokens = 7;
    let expected = partial.content.clone();
    let event = AssistantStreamEvent::TextDelta {
        content_index: 0,
        delta: "retained ordinary partial".into(),
        partial,
    };
    tx.send(Ok(event)).await.unwrap();
    bounded(call.next()).await.unwrap().unwrap();
    let failure = bounded(call.abort()).await.unwrap_err();
    assert!(matches!(failure.cause(), ProviderError::Cancelled));
    assert_eq!(failure.message().content, expected);
    assert_eq!(failure.message().usage.output_tokens, 7);
    assert_eq!(released.try_recv(), Ok(()));
    assert!(!token.is_cancelled());
    let mut started = call.start();
    assert!(started.next().await.is_none());
    assert_eq!(started.result().await, Err(failure));
}

impl Provider for ControlledProvider {
    fn id(&self) -> &str {
        "execution"
    }
    fn models(&self) -> &[ModelInfo] {
        &self.models
    }
    fn stream_prepared(&self, _: Request, _: opi_ai::ResolvedAuth) -> EventStream {
        let _ = self.entered.lock().unwrap().take().unwrap().send(());
        self.source.lock().unwrap().take().unwrap()
    }
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
