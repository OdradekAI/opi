//! Controlled event sources and chat setup shared by lifecycle tests.

use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll};
use std::time::Duration;

use futures_core::Stream;
use futures_util::StreamExt;
use opi_ai::provider::{EventStream, ProviderError, Request};
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use opi_ai::test_support::base_assistant;
use opi_ai::{ModelInfo, Provider};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

pub(super) type Item = Result<AssistantStreamEvent, ProviderError>;

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

pub(super) fn channel_source() -> (EventStream, mpsc::Sender<Item>, oneshot::Receiver<()>) {
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

pub(super) async fn bounded<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(5), future)
        .await
        .expect("controlled operation must make progress")
}

pub(super) fn delta(index: usize) -> AssistantStreamEvent {
    AssistantStreamEvent::TextDelta {
        content_index: 0,
        delta: index.to_string(),
        partial: base_assistant(),
    }
}

pub(super) fn done() -> AssistantStreamEvent {
    AssistantStreamEvent::Done {
        reason: StopReason::Stop,
        message: base_assistant(),
    }
}

pub(super) fn request(cancel: CancellationToken) -> Request {
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

pub(super) struct ControlledProvider {
    pub(super) models: Vec<ModelInfo>,
    pub(super) source: Mutex<Option<EventStream>>,
    pub(super) entered: Mutex<Option<oneshot::Sender<()>>>,
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

pub(super) struct GatedTail {
    pub(super) source: EventStream,
    pub(super) entered: oneshot::Receiver<()>,
    pub(super) release: oneshot::Sender<()>,
    pub(super) eof: oneshot::Receiver<()>,
    pub(super) dropped: oneshot::Receiver<()>,
}

pub(super) fn gated_tail(terminal: Item, cancel: CancellationToken) -> GatedTail {
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
