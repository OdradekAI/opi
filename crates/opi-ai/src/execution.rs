//! Private ownership of independently executing chat calls.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use futures_util::StreamExt;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use tokio::task::{JoinError, JoinHandle};

use crate::message::AssistantMessage;
use crate::provider::{ProviderError, ProviderErrorSummary};
use crate::stream::{AssistantStreamEvent, CallFailure, ProducerSnapshot};

type Event = Result<AssistantStreamEvent, CallFailure>;
type Outcome = Result<AssistantMessage, CallFailure>;

/// The consumer of a chat call scheduled on a Tokio runtime.
///
/// Ordinary [`ChatCall`](crate::ChatCall) wraps this consumer from construction;
/// [`ChatCall::start`](crate::ChatCall::start) extracts it without spawning again.
/// [`ChatStream::start`](crate::ChatStream::start) schedules a raw collector.
/// The execution task owns
/// setup, source polling and source release. It progresses without polling
/// this handle. Dropping the handle, stopping iteration or abandoning
/// [`result`](Self::result) does not cancel that task. Retain and cancel the
/// original request's cancellation token to request cancellation; the source
/// must cooperate with that token. [`abort`](Self::abort) explicitly stops and
/// joins this execution task. Runtime shutdown can still stop execution.
///
/// Events retain their order and event-time snapshots. Unread events use an
/// **unbounded queue** so that a retained, unpolled handle cannot block request
/// completion. Memory grows with unread events; consuming them, calling
/// `result`, or dropping this handle releases the backlog. After consumer
/// Drop the task continues polling but discards further events.
/// Request-token cancellation does not retract events already forwarded into this
/// queue: iteration delivers them before the source's cancellation failure.
/// `result` discards those events and waits directly for the outcome. The
/// source's own cancellation priority still applies to its internal queue.
///
/// The underlying call's first-terminal absorption and single final result
/// are preserved, including for calls partially consumed before `start`.
/// Source resources are released before a terminal is forwarded. This does
/// not join independent tasks spawned internally by a custom provider, or
/// add a provider-specific cleanup or timeout policy.
///
/// Provider panics use Tokio's panic hook and are resumed when this handle is
/// polled or awaited; they are not converted to a successful business result.
/// If the runtime cancels the task, a waiting consumer receives a typed stream
/// failure retaining the latest produced partial. No AI span or exporter is
/// installed by this execution owner.
pub struct StartedChatStream {
    events: Option<UnboundedReceiver<Event>>,
    execution: Option<JoinHandle<Outcome>>,
    terminal: Option<Outcome>,
    progress: ProducerSnapshot,
    abort_requested: bool,
}

impl StartedChatStream {
    pub(crate) fn finished(outcome: Outcome) -> Self {
        Self {
            events: None,
            execution: None,
            terminal: Some(outcome),
            progress: ProducerSnapshot::default(),
            abort_requested: false,
        }
    }

    pub(crate) fn spawn<S, F, R>(mut source: S, finish: F, progress: ProducerSnapshot) -> Self
    where
        S: Stream<Item = Event> + Send + Unpin + 'static,
        F: FnOnce(S) -> R + Send + 'static,
        R: Future<Output = Outcome> + Send,
    {
        let (tx, events) = unbounded_channel();
        let produced = progress.clone();
        let execution = tokio::spawn(async move {
            let mut terminal = None;
            while let Some(event) = source.next().await {
                match &event {
                    Ok(event) => {
                        if let Some(message) = event.latest_partial() {
                            produced.record(message);
                        }
                    }
                    Err(failure) => produced.record(failure.message()),
                }
                if event
                    .as_ref()
                    .map_or(true, AssistantStreamEvent::is_terminal)
                {
                    terminal = Some(event);
                    break;
                }
                // A closed consumer is not a cancellation request.
                let _ = tx.send(event);
                // Also cooperate when a custom source is continuously ready.
                tokio::task::consume_budget().await;
            }
            // Reuse the source's terminal cache, including when it was
            // consumed before start(). This consumes/releases the source
            // before either terminal delivery or task completion is observed.
            let outcome = finish(source).await;
            if let Some(event) = terminal {
                let _ = tx.send(event);
            }
            outcome
        });
        Self {
            events: Some(events),
            execution: Some(execution),
            terminal: None,
            progress,
            abort_requested: false,
        }
    }

    /// Stop retaining events and await the execution's one final outcome.
    ///
    /// This consumes the event handle, immediately frees queued events on
    /// first poll, and does not resubmit the request. Dropping this future
    /// detaches the waiter; the execution continues until its own terminal.
    pub async fn result(mut self) -> Outcome {
        self.events.take();
        match self.execution.take() {
            Some(task) => completed(task.await, &self.progress, self.abort_requested),
            None => self.terminal.take().expect("observed execution outcome"),
        }
    }

    /// Explicitly abort the execution task and wait for its source to be dropped.
    ///
    /// On first poll, releases buffered events and requests task abortion.
    /// If execution has already finished (or finishes before abortion takes
    /// effect), returns its original outcome. Otherwise returns cancellation
    /// with the latest partial observed by this execution, including unread
    /// events. Provider state that emitted no event is not available here.
    /// Source panics are resumed, just as with [`result`](Self::result).
    ///
    /// This does not cancel the original request token. Cancel that token too
    /// when other work shares the request. Awaiting this method joins this
    /// execution task only; independently spawned provider tasks and session
    /// resources retain their own cleanup contracts. A source that blocks
    /// inside a poll must yield before Tokio can abort it.
    ///
    /// The handle retains the task while waiting. Dropping this future after
    /// its first poll does not undo the abort request; `abort`, `result` or
    /// iteration can still observe completion through the same handle.
    /// Dropping this future before first poll requests no abortion.
    pub async fn abort(&mut self) -> Outcome {
        self.events.take();
        if let Some(task) = &mut self.execution {
            if !task.is_finished() {
                self.abort_requested = true;
                task.abort();
            }
            let joined = task.await;
            self.execution.take();
            self.terminal = Some(completed(joined, &self.progress, self.abort_requested));
        }
        self.terminal.clone().expect("observed execution outcome")
    }
}

impl Stream for StartedChatStream {
    type Item = Event;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Event>> {
        let this = self.get_mut();
        if this.terminal.is_some() {
            return Poll::Ready(None);
        }
        if let Some(events) = &mut this.events {
            match events.poll_recv(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Some(event)) => {
                    match &event {
                        Ok(AssistantStreamEvent::Done { message, .. }) => {
                            this.terminal = Some(Ok(message.clone()));
                        }
                        Err(failure) => this.terminal = Some(Err(failure.clone())),
                        _ => {}
                    }
                    if this.terminal.is_some() {
                        this.events.take();
                    }
                    return Poll::Ready(Some(event));
                }
                Poll::Ready(None) => {
                    this.events.take();
                }
            }
        }
        let task = this.execution.as_mut().expect("active execution");
        let Poll::Ready(joined) = Pin::new(task).poll(cx) else {
            return Poll::Pending;
        };
        this.execution.take();
        let interrupted = joined.is_err();
        let outcome = completed(joined, &this.progress, this.abort_requested);
        let event = if interrupted {
            Some(Err(outcome.clone().expect_err("interrupted task failed")))
        } else {
            // A source whose terminal was consumed before start has no new
            // events. Cache its outcome without manufacturing a second one.
            None
        };
        this.terminal = Some(outcome);
        Poll::Ready(event)
    }
}

fn completed(
    joined: Result<Outcome, JoinError>,
    progress: &ProducerSnapshot,
    abort_requested: bool,
) -> Outcome {
    match joined {
        Ok(outcome) => outcome,
        Err(error) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
        Err(_) if abort_requested => Err(CallFailure::from_partial(
            None,
            progress.failure(ProviderError::Cancelled),
        )),
        Err(_) => Err(CallFailure::from_partial(
            None,
            progress.failure(ProviderError::StreamError(
                ProviderErrorSummary::attested_static(
                    "chat execution task stopped before completion",
                ),
            )),
        )),
    }
}
