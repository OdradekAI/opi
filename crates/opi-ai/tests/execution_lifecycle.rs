//! Explicit execution owns progress; its handle owns only event consumption.

use std::pin::Pin;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use futures_core::Stream;
use futures_util::StreamExt;
use opi_ai::deferred::{DeferredApi, DeferredCancelCall, DeferredFetchCall, DeferredHandle};
use opi_ai::message::AssistantMessage;
use opi_ai::models::ApiId;
use opi_ai::provider::{EventStream, ProviderError, ProviderErrorSummary, Request};
use opi_ai::stream::{
    AssistantStreamEvent, CallFailure, ChatStream, StartedChatStream, StopReason,
};
use opi_ai::test_support::{
    CountingAuthResolver, MockProvider, base_assistant, single_route_collection,
};
use opi_ai::{AuthProvenanceSource, BoxAuthFuture, ChatModelRecord, ModelInfo, Models, Provider};
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
    request.model = "request-provider:request-model".into();
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
    let message = failure.message();
    assert_eq!(message.provider, "missing");
    assert_eq!(message.model, "mock-model");
    assert_eq!(message.stop_reason, StopReason::Error);
    assert!(message.content.is_empty());
    assert!(!message.usage.reported);
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
        drop(tx); // Ordinary execution also waits for source EOF after Done.
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

struct GatedTail {
    source: EventStream,
    entered: oneshot::Receiver<()>,
    release: oneshot::Sender<()>,
    eof: oneshot::Receiver<()>,
    dropped: oneshot::Receiver<()>,
}

fn gated_tail(terminal: Item, cancel: CancellationToken) -> GatedTail {
    let (entered, waiting) = oneshot::channel();
    let (release, proceed) = oneshot::channel();
    let (eof, reached_eof) = oneshot::channel();
    let mut eof = Some(eof);
    let (resource, tx, dropped) = channel_source();
    drop(tx);
    let tail = futures_util::stream::once(async move {
        let _ = entered.send(());
        tokio::select! {
            _ = proceed => {},
            _ = cancel.cancelled() => {},
        }
        Ok(delta(999))
    });
    let source = futures_util::stream::iter([Ok(delta(0)), terminal])
        .chain(tail)
        .chain(futures_util::stream::iter([
            Err(ProviderError::Timeout),
            Ok(done()),
        ]))
        .chain(resource)
        .chain(futures_util::stream::poll_fn(move |_| {
            if let Some(eof) = eof.take() {
                let _ = eof.send(());
            }
            Poll::Ready(None)
        }));
    GatedTail {
        source: Box::pin(source),
        entered: waiting,
        release,
        eof: reached_eof,
        dropped,
    }
}

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
