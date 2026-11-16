//! Private ownership of independently executing chat calls.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};
use std::task::{Context, Poll};

use futures_core::Stream;
use futures_util::StreamExt;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use tokio::task::{JoinError, JoinHandle};

use crate::message::AssistantMessage;
use crate::provider::{ProviderError, ProviderErrorSummary};
use crate::stream::{AssistantStreamEvent, CallFailure, ProducerSnapshot};
use crate::telemetry::SharedAiRequestObserver;

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
/// Terminals ordinarily follow source release. A deferred fetch caches and
/// forwards a successful `Done` before draining its source. Public EOF and
/// `result` wait for source release and poll settlement. Provider-owned tasks
/// retain their own cleanup and timeout policies.
///
/// Provider panics use Tokio's panic hook and are resumed when this handle
/// joins the task. Runtime cancellation yields a typed stream failure with
/// the latest produced partial, or the cached successful `Done` of a
/// deferred fetch. Joining that cancelled tail settles the existing poll
/// observer with the cached outcome after source release.
pub struct StartedChatStream {
    events: Option<UnboundedReceiver<Event>>,
    execution: Option<JoinHandle<Outcome>>,
    terminal: Option<Outcome>,
    progress: ProducerSnapshot,
    abort_requested: bool,
    observer: Option<SharedAiRequestObserver>,
    committed_success: Option<Arc<OnceLock<AssistantMessage>>>,
}

impl StartedChatStream {
    pub(crate) fn finished(outcome: Outcome) -> Self {
        Self {
            events: None,
            execution: None,
            terminal: Some(outcome),
            progress: ProducerSnapshot::default(),
            abort_requested: false,
            observer: None,
            committed_success: None,
        }
    }

    pub(crate) fn spawn<S, F, R>(
        mut source: S,
        finish: F,
        progress: ProducerSnapshot,
        observer: Option<SharedAiRequestObserver>,
        committed_success: Option<Arc<OnceLock<AssistantMessage>>>,
    ) -> Self
    where
        S: Stream<Item = Event> + Send + Unpin + 'static,
        F: FnOnce(S) -> R + Send + 'static,
        R: Future<Output = Outcome> + Send,
    {
        let (tx, events) = unbounded_channel();
        let produced = progress.clone();
        let deferred_success = committed_success.clone();
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
                    if let (Some(committed), Ok(AssistantStreamEvent::Done { message, .. })) =
                        (&deferred_success, &event)
                    {
                        // Commit before publication, including when the queue is closed.
                        committed.get_or_init(|| message.clone());
                        let _ = tx.send(event);
                    } else {
                        terminal = Some(event);
                    }
                    break;
                }
                // A closed consumer is not a cancellation request.
                let _ = tx.send(event);
                // Also cooperate when a custom source is continuously ready.
                tokio::task::consume_budget().await;
            }
            // Reuse the source's terminal cache, including after a terminal
            // was consumed before start(). Completion waits for source release.
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
            observer,
            committed_success,
        }
    }

    /// Stop retaining events and await the execution's one final outcome.
    ///
    /// This consumes the event handle, immediately frees queued events on
    /// first poll, and does not resubmit the request. Dropping this future
    /// detaches the waiter and the execution continues through source cleanup.
    pub async fn result(mut self) -> Outcome {
        self.events.take();
        match self.execution.take() {
            Some(task) => self.finish_join(task.await, false),
            None => self.terminal.take().expect("observed execution outcome"),
        }
    }

    /// Explicitly abort the execution task and wait for its source to be dropped.
    ///
    /// On first poll, releases buffered events and requests task abortion.
    /// If execution has already finished (or finishes before abortion takes
    /// effect), returns its original outcome. A deferred fetch also retains
    /// its cached successful `Done` when abortion stops the tail, even if
    /// the event was unread. Otherwise returns cancellation with the latest
    /// partial observed by this execution, including unread events.
    /// Source panics are resumed, just as with [`result`](Self::result).
    ///
    /// This does not cancel the original request token. Cancel that token too
    /// when other work shares the request. Awaiting this method joins this
    /// execution task only; independently spawned provider tasks and session
    /// resources retain their own cleanup contracts. A source that blocks
    /// inside a poll must yield before Tokio can abort it. When Tokio confirms
    /// cancellation, the join settles the existing observer with the returned
    /// outcome.
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
            self.terminal = Some(self.finish_join(joined, true));
        }
        self.terminal.clone().expect("observed execution outcome")
    }

    fn finish_join(&mut self, joined: Result<Outcome, JoinError>, aborting: bool) -> Outcome {
        let committed = self.committed_success.take();
        let observer = self.observer.take();
        let outcome = match joined {
            Ok(outcome) => return outcome,
            Err(error) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
            Err(_) => match committed.as_deref().and_then(OnceLock::get) {
                Some(message) => Ok(message.clone()),
                None => {
                    let cause = if self.abort_requested {
                        ProviderError::Cancelled
                    } else {
                        ProviderError::StreamError(ProviderErrorSummary::attested_static(
                            "chat execution task stopped before completion",
                        ))
                    };
                    Err(CallFailure::from_partial(
                        None,
                        self.progress.failure(cause),
                    ))
                }
            },
        };
        if let Some(observer) = observer
            && (aborting || self.abort_requested || outcome.is_ok())
        {
            let message = outcome.as_ref().unwrap_or_else(|failure| failure.message());
            observer.finish(Some(message));
        }
        outcome
    }

    pub(crate) async fn await_ready(
        mut self,
        ready: tokio::sync::oneshot::Receiver<Outcome>,
    ) -> Outcome {
        self.events.take();
        match ready.await {
            Ok(outcome) => outcome,
            // Join to recover interruption or a panic before the notification.
            Err(_) => self.result().await,
        }
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
                            this.events.take();
                            // A deferred Done still needs to join its source drain.
                            if this.committed_success.is_none() {
                                this.terminal = Some(Ok(message.clone()));
                            }
                        }
                        Err(failure) => {
                            this.events.take();
                            this.terminal = Some(Err(failure.clone()));
                        }
                        _ => {}
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
        let outcome = this.finish_join(joined, false);
        let event = match (&outcome, interrupted) {
            (Err(failure), true) => Some(Err(failure.clone())),
            _ => None,
        };
        this.terminal = Some(outcome);
        Poll::Ready(event)
    }
}
