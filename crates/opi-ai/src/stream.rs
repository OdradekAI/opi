//! Streaming response events (S7.3).

#[cfg(any(feature = "builtins", feature = "provider-protocol-common", test))]
use std::future::Future;
use std::future::poll_fn;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};
use std::task::{Context, Poll, ready};

use futures_core::Stream;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

pub use crate::execution::StartedChatStream;
pub use crate::types::usage::*;
#[cfg(any(feature = "builtins", feature = "provider-protocol-common", test))]
use tokio_util::sync::{CancellationToken, WaitForCancellationFutureOwned};

use crate::provider::{ProviderError, ProviderErrorSummary};

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AssistantStreamEvent {
    #[serde(rename = "start")]
    Start {
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "text_start")]
    TextStart {
        content_index: usize,
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "text_delta")]
    TextDelta {
        content_index: usize,
        delta: String,
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "text_end")]
    TextEnd {
        content_index: usize,
        content: String,
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "thinking_start")]
    ThinkingStart {
        content_index: usize,
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "thinking_delta")]
    ThinkingDelta {
        content_index: usize,
        delta: String,
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "thinking_end")]
    ThinkingEnd {
        content_index: usize,
        content: String,
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "tool_call_start")]
    ToolCallStart {
        content_index: usize,
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "tool_call_delta")]
    ToolCallDelta {
        content_index: usize,
        delta: String,
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "tool_call_end")]
    ToolCallEnd {
        content_index: usize,
        tool_call: crate::message::ToolCall,
        partial: crate::message::AssistantMessage,
    },
    #[serde(rename = "done")]
    Done {
        reason: StopReason,
        message: crate::message::AssistantMessage,
    },
    #[serde(rename = "error")]
    /// Legacy in-band error terminal (v0 stream format).
    ///
    /// Read compatibility for saved events and legacy adapters. `ChatStream`
    /// converts it to `Err(CallFailure)`, preserving an abort when `reason`
    /// and `message.stop_reason` are both `Aborted`.
    Error {
        reason: StopReason,
        message: crate::message::AssistantMessage,
    },
}

impl AssistantStreamEvent {
    pub(crate) fn set_projection(&mut self, facts: &crate::transcript::ProjectionFacts) {
        match self {
            Self::Start { partial }
            | Self::TextStart { partial, .. }
            | Self::TextDelta { partial, .. }
            | Self::TextEnd { partial, .. }
            | Self::ThinkingStart { partial, .. }
            | Self::ThinkingDelta { partial, .. }
            | Self::ThinkingEnd { partial, .. }
            | Self::ToolCallStart { partial, .. }
            | Self::ToolCallDelta { partial, .. }
            | Self::ToolCallEnd { partial, .. } => partial.projection = Some(facts.clone()),
            Self::Done { message, .. } | Self::Error { message, .. } => {
                message.projection = Some(facts.clone())
            }
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Done { .. } | Self::Error { .. })
    }

    /// The latest partial-message snapshot carried by this event, if any.
    ///
    /// Every streaming event clones the producer's partial message at event
    /// time, so consumers observe immutable event-time snapshots regardless of
    /// later stream progress. Terminal events carry their complete payload in
    /// `message`.
    pub(crate) fn latest_partial(&self) -> Option<&crate::message::AssistantMessage> {
        match self {
            Self::Start { partial }
            | Self::TextStart { partial, .. }
            | Self::TextDelta { partial, .. }
            | Self::TextEnd { partial, .. }
            | Self::ThinkingStart { partial, .. }
            | Self::ThinkingDelta { partial, .. }
            | Self::ThinkingEnd { partial, .. }
            | Self::ToolCallStart { partial, .. }
            | Self::ToolCallDelta { partial, .. }
            | Self::ToolCallEnd { partial, .. } => Some(partial),
            Self::Done { message, .. } | Self::Error { message, .. } => Some(message),
        }
    }
}

// ---------------------------------------------------------------------------
// Call failure (task 20.2, AIR-047 / P20-FINAL)
// ---------------------------------------------------------------------------

/// The single final failure of one logical provider call.
///
/// `message` always contains the complete canonical partial or terminal
/// [`AssistantMessage`](crate::message::AssistantMessage): generated content,
/// provenance, stop reason (`Error`, or `Aborted` when the cause is a
/// cancellation), diagnostics in `error_message` and known usage. A synthetic
/// failure has no generated content and unknown usage. The ordinary
/// [`Models`](crate::Models) chat setup and deferred fetch paths include the
/// requested provider and model IDs in synthesized failure messages.
///
/// `cause` supplies the typed, redacted classification. An aborted provider
/// terminal carries [`ProviderError::Cancelled`]. The caller's cancellation
/// token records local cancellation independently. [`ChatStream`] converts
/// legacy [`AssistantStreamEvent::Error`] terminals into this failure type.
#[derive(Debug, Clone, PartialEq)]
pub struct CallFailure {
    message: Box<crate::message::AssistantMessage>,
    cause: ProviderError,
}

impl CallFailure {
    pub(crate) fn with_projection(mut self, facts: &crate::transcript::ProjectionFacts) -> Self {
        self.message.projection = Some(facts.clone());
        self
    }

    /// The canonical partial/terminal assistant message for the failed call.
    pub fn message(&self) -> &crate::message::AssistantMessage {
        &self.message
    }

    /// The typed, redacted cause classification.
    pub fn cause(&self) -> &ProviderError {
        &self.cause
    }

    /// Split into the canonical message and the typed cause.
    pub fn into_parts(self) -> (crate::message::AssistantMessage, ProviderError) {
        (*self.message, self.cause)
    }

    /// Use the frozen record for the fallback message. A producer-supplied
    /// partial still takes precedence.
    pub(crate) fn from_setup(record: &crate::ChatModelRecord, cause: ProviderError) -> Self {
        Self::from_requested_route(record.provider_id(), record.model_id(), cause)
    }

    fn from_requested_route(provider: &str, model: &str, cause: ProviderError) -> Self {
        let mut message = setup_failure_message(&cause);
        message.provider = provider.to_owned();
        message.model = model.to_owned();
        Self::from_partial(Some(message), cause)
    }

    /// Build a failure from the latest event-time partial (if any) and a typed
    /// cause. Normalizes the stop reason per the delivery contract: a
    /// cancellation cause maps to [`StopReason::Aborted`], every other cause to
    /// [`StopReason::Error`]. Diagnostics from the cause are preserved on the
    /// message when the producer did not supply one.
    pub(crate) fn from_partial(
        partial: Option<crate::message::AssistantMessage>,
        cause: ProviderError,
    ) -> Self {
        let (producer_partial, cause) = cause.into_stream_parts();
        let partial = producer_partial.or(partial);
        let mut message = partial.unwrap_or_else(|| setup_failure_message(&cause));
        message.stop_reason = if matches!(
            cause,
            ProviderError::Cancelled | ProviderError::LoginCancelled { .. }
        ) {
            StopReason::Aborted
        } else {
            StopReason::Error
        };
        if message.error_message.is_none() {
            message.error_message = Some(cause.to_string());
        }
        Self {
            message: Box::new(message),
            cause,
        }
    }

    /// Convert a legacy in-band error terminal, preserving its payload. Both
    /// stop fields must agree to classify a provider abort.
    pub(crate) fn from_legacy_error_event(
        reason: StopReason,
        mut message: crate::message::AssistantMessage,
    ) -> Self {
        let cause = if reason == StopReason::Aborted && message.stop_reason == StopReason::Aborted {
            ProviderError::Cancelled
        } else {
            message.stop_reason = StopReason::Error;
            ProviderError::StreamError(ProviderErrorSummary::redacted())
        };
        Self {
            message: Box::new(message),
            cause,
        }
    }

    /// Classify a delivered `Done` terminal whose stop reason is itself a
    /// failure classification (`Error`/`Aborted`) instead of a completion.
    pub(crate) fn from_error_stop_reason(
        reason: StopReason,
        message: crate::message::AssistantMessage,
    ) -> Self {
        match reason {
            StopReason::Aborted => Self::from_partial(Some(message), ProviderError::Cancelled),
            _ => Self::from_partial(
                Some(message),
                ProviderError::StreamError(ProviderErrorSummary::attested_static(
                    "provider terminal carried an error stop reason",
                )),
            ),
        }
    }
}

/// An empty fallback for failures where the request identity is unavailable.
pub(crate) fn setup_failure_message(cause: &ProviderError) -> crate::message::AssistantMessage {
    crate::message::AssistantMessage {
        projection: None,
        content: Vec::new(),
        api: crate::ApiKind::Anthropic,
        provider: String::new(),
        deferred: None,
        model: String::new(),
        response_model: None,
        wire: None,
        response_id: None,
        usage: crate::stream::Usage::default(),
        stop_reason: StopReason::Error,
        error_message: Some(cause.to_string()),
        timestamp_ms: crate::time::now_ms(),
        diagnostics: Vec::new(),
        duration_ms: None,
        raw_stop_reason: None,
        provider_thinking_level: None,
    }
}

/// Retain an attested classification fact without replacing the acquisition
/// cause or admitting provider-controlled text into the canonical message.
#[cfg(any(
    feature = "builtins",
    feature = "provider-protocol-common",
    feature = "http-transport"
))]
#[cfg_attr(
    not(any(feature = "builtins", feature = "provider-protocol-common")),
    allow(dead_code)
)]
pub(crate) fn setup_failure_with_detail(
    cause: ProviderError,
    detail: &'static str,
) -> ProviderError {
    let mut message = setup_failure_message(&cause);
    message.error_message = Some(detail.to_owned());
    cause.with_partial(message)
}

// ---------------------------------------------------------------------------
// ChatStream (task 20.2, AIR-003/AIR-047 / P20-FINAL)
// ---------------------------------------------------------------------------

/// The public owner of one logical provider call's event stream.
///
/// Iteration is `Stream<Item = Result<AssistantStreamEvent, CallFailure>>`:
/// events are delivered with their immutable event-time partials, and the
/// first delivered terminal fixes the outcome and absorbs later source items.
/// [`ChatStream::result`] drains the retained source after that terminal.
/// Streams from [`Models::stream_deferred`](crate::Models::stream_deferred) also
/// drive that drain before returning public EOF. Once iteration reaches EOF,
/// it stays exhausted. Failures surface as exactly one `Err(CallFailure)` carrying
/// the canonical partial and a typed, redacted cause; a stream that ends
/// without a terminal yields one typed protocol failure. Cancellation before
/// any delivery is a setup failure with an `Aborted` stop reason.
///
/// [`ChatStream::result`] consumes the handle and actively
/// drives the remaining bounded stream to the same
/// `Result<AssistantMessage, CallFailure>` outcome that complete dispatch
/// produces (`Stop`/`Length`/`ToolUse` are `Ok`; `Error`/`Aborted` are `Err`),
/// reusing a delivered terminal cache without resubmission or self-cancel.
/// Dropping the handle without polling performs no inner work; dropping it
/// mid-stream ends the attempt and releases the producer.
/// A direct deferred collector dropped after a successful `Done` releases
/// the source before settling its enabled poll observer with the cached result.
pub struct ChatStream {
    inner: Option<crate::provider::EventStream>,
    partial: Option<crate::message::AssistantMessage>,
    requested_route: Option<(String, String)>,
    terminal: Option<Result<crate::message::AssistantMessage, CallFailure>>,
    ai_telemetry: Option<crate::telemetry::SharedAiRequestObserver>,
    drain_tail_on_completion: bool,
    execution_owns_completion: bool,
}

fn poll_source_tail(
    source: &mut Option<crate::provider::EventStream>,
    cx: &mut Context<'_>,
) -> Poll<()> {
    if let Some(inner) = source.as_mut() {
        while ready!(inner.as_mut().poll_next(cx)).is_some() {}
        *source = None;
    }
    Poll::Ready(())
}

struct ProjectionStream {
    inner: Option<crate::provider::EventStream>,
    facts: crate::transcript::ProjectionFacts,
    terminal: bool,
    partial: Option<crate::message::AssistantMessage>,
}
impl Stream for ProjectionStream {
    type Item = Result<AssistantStreamEvent, ProviderError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.terminal {
            ready!(poll_source_tail(&mut this.inner, cx));
            return Poll::Ready(None);
        }
        match this
            .inner
            .as_mut()
            .expect("active projection stream")
            .as_mut()
            .poll_next(cx)
        {
            Poll::Ready(Some(Ok(mut event))) => {
                event.set_projection(&this.facts);
                this.partial = event.latest_partial().cloned();
                if event.is_terminal() {
                    this.terminal = true;
                }
                Poll::Ready(Some(Ok(event)))
            }
            Poll::Ready(Some(Err(cause))) => {
                this.terminal = true;
                this.inner.take();
                let cause = match &this.partial {
                    Some(partial) => cause.with_partial(partial.clone()),
                    None => cause,
                };
                Poll::Ready(Some(Err(cause.with_projection(&this.facts))))
            }
            Poll::Ready(None) => {
                this.terminal = true;
                this.inner.take();
                let cause = ProviderError::StreamError(
                    crate::provider::ProviderErrorSummary::attested_static(
                        "stream ended without a terminal event",
                    ),
                );
                let cause = match &this.partial {
                    Some(partial) => cause.with_partial(partial.clone()),
                    None => cause,
                };
                Poll::Ready(Some(Err(cause.with_projection(&this.facts))))
            }
            other => other,
        }
    }
}

#[cfg(any(feature = "builtins", feature = "provider-protocol-common"))]
pub(crate) fn project_call(
    facts: crate::transcript::ProjectionFacts,
    prepare: impl FnOnce() -> crate::provider::EventStream,
) -> crate::provider::EventStream {
    with_projection(prepare(), facts)
}

pub(crate) fn with_projection(
    stream: crate::provider::EventStream,
    facts: crate::transcript::ProjectionFacts,
) -> crate::provider::EventStream {
    Box::pin(ProjectionStream {
        inner: Some(stream),
        facts,
        terminal: false,
        partial: None,
    })
}

impl ChatStream {
    /// Schedule this source independently of event consumption.
    ///
    /// The returned handle does not have to be polled for execution to
    /// progress. Its Drop does not cancel the source. Existing partial
    /// consumption and a delivered terminal are preserved. See
    /// [`StartedChatStream`] for buffering, cancellation and panic behavior.
    /// Streams from [`Models::stream_deferred`](crate::Models::stream_deferred)
    /// cache and forward a successful `Done` before draining the source.
    /// The task uses [`Self::result`] to release the retained source and
    /// settle the poll before public EOF or the started result. This also
    /// drives the tail of a terminal consumed before `start`.
    ///
    /// # Panics
    ///
    /// Panics outside a Tokio runtime when starting an unfinished stream or
    /// a deferred stream with a retained tail. The source may first be polled
    /// after this method returns.
    pub fn start(mut self) -> StartedChatStream {
        let terminal_delivered = self.terminal.is_some();
        if terminal_delivered && (!self.drain_tail_on_completion || self.inner.is_none()) {
            self.finish_ai_telemetry();
            let outcome = self
                .terminal
                .take()
                .expect("terminal presence checked before conversion");
            return StartedChatStream::finished(outcome);
        }
        // The worker and its join own settlement after the source is handed off.
        self.execution_owns_completion = true;
        let progress = self.progress_snapshot();
        let observer = self.ai_telemetry.clone();
        if !self.drain_tail_on_completion {
            return StartedChatStream::spawn(
                self,
                Self::result_without_tail,
                progress,
                observer,
                None,
            );
        }
        let committed_success = Some(Arc::new(
            self.terminal
                .as_ref()
                .and_then(|outcome| outcome.as_ref().ok())
                .cloned()
                .map_or_else(OnceLock::new, OnceLock::from),
        ));
        if terminal_delivered {
            // Skip event iteration so result owns both tail release and poll settlement.
            return StartedChatStream::spawn(
                futures_util::stream::empty(),
                move |_| self.result(),
                progress,
                observer,
                committed_success,
            );
        }
        StartedChatStream::spawn(self, Self::result, progress, observer, committed_success)
    }

    pub(crate) fn progress_snapshot(&self) -> ProducerSnapshot {
        let progress = ProducerSnapshot::default();
        if let Some(partial) = &self.partial {
            progress.record(partial);
        }
        progress
    }

    /// Wrap one provider event stream (for example
    /// [`PreparedProviderCall::start_attempt`](crate::models::PreparedProviderCall::start_attempt)).
    ///
    /// Construction is lazy: the inner stream is not polled until the first
    /// `poll_next`/`next().await` or [`ChatStream::result`].
    pub fn new(stream: crate::provider::EventStream) -> Self {
        Self {
            inner: Some(stream),
            partial: None,
            requested_route: None,
            terminal: None,
            ai_telemetry: None,
            drain_tail_on_completion: false,
            execution_owns_completion: false,
        }
    }

    /// Wrap one provider event stream with the standard Pi-shaped AI
    /// telemetry observer.
    ///
    /// The first normalized delivery, including a typed failure, counts as
    /// the first chunk.
    ///
    /// The observer is enabled only when `context` is `Some`; omission keeps
    /// this collector business-only. Backend failures are isolated in the
    /// same way as ordinary [`Models`](crate::models::Models) chat calls:
    /// they are reported through the host tracing channel and never replace
    /// the provider's typed stream result.
    pub fn new_with_telemetry(
        stream: crate::provider::EventStream,
        context: Option<&crate::telemetry::TelemetryContext>,
        attributes: crate::telemetry::AiRequestAttributes,
    ) -> Self {
        Self::with_shared_telemetry(
            stream,
            crate::telemetry::SharedAiRequestObserver::start(context, attributes),
        )
    }

    pub(crate) fn with_shared_telemetry(
        stream: crate::provider::EventStream,
        observer: crate::telemetry::SharedAiRequestObserver,
    ) -> Self {
        let mut stream = Self::new(stream);
        stream.ai_telemetry = Some(observer);
        stream
    }

    pub(crate) fn with_requested_route(mut self, provider: String, model: String) -> Self {
        self.requested_route = Some((provider, model));
        self
    }

    pub(crate) fn with_tail_drain_on_completion(mut self) -> Self {
        self.drain_tail_on_completion = true;
        self
    }

    fn source_failure(&self, cause: ProviderError) -> CallFailure {
        if let (None, Some((provider, model))) = (&self.partial, &self.requested_route) {
            CallFailure::from_requested_route(provider, model, cause)
        } else {
            CallFailure::from_partial(self.partial.clone(), cause)
        }
    }

    /// Consume the handle and actively drive the remaining bounded stream to
    /// the single final outcome.
    ///
    /// Equivalent to full iteration for the same event sequence: a terminal
    /// already delivered to an iterator is reused from cache, partial
    /// iteration does not change the outcome, and a stream that ends without
    /// a terminal produces the one typed protocol failure (AIR-003). After a
    /// business terminal is cached, this method also drains the retained
    /// source until EOF so provider-owned settlement work is complete before
    /// the handle is dropped.
    pub async fn result(self) -> Result<crate::message::AssistantMessage, CallFailure> {
        let mut stream = self;
        while stream.terminal.is_none() {
            if stream.next().await.is_none() {
                // poll_next synthesizes the premature-EOF protocol failure, so
                // exhaustion always leaves a terminal set.
                break;
            }
        }
        poll_fn(|cx| poll_source_tail(&mut stream.inner, cx)).await;
        stream.finish_ai_telemetry();
        stream
            .terminal
            .take()
            .expect("chat stream exhaustion always records a terminal")
    }

    pub(crate) async fn result_without_tail(
        mut self,
    ) -> Result<crate::message::AssistantMessage, CallFailure> {
        self.finish_ai_telemetry();
        self.terminal
            .take()
            .expect("started execution observes a terminal before finalization")
    }

    pub(crate) async fn first_outcome(
        mut self,
    ) -> Result<crate::message::AssistantMessage, CallFailure> {
        self.execution_owns_completion = true;
        let (send, ready) = tokio::sync::oneshot::channel();
        let progress = self.progress_snapshot();
        let observer = self.ai_telemetry.clone();
        StartedChatStream::spawn(
            self,
            move |source| async move {
                // The worker has polled through the first normalized terminal.
                if let Some(outcome) = &source.terminal {
                    let _ = send.send(outcome.clone());
                }
                source.result().await
            },
            progress,
            observer,
            None,
        )
        .await_ready(ready)
        .await
    }

    fn finish_ai_telemetry(&mut self) {
        let Some(observer) = self.ai_telemetry.take() else {
            return;
        };
        match self.terminal.as_ref() {
            Some(Ok(message)) => observer.finish(Some(message)),
            Some(Err(failure)) => observer.finish(Some(failure.message())),
            None => {}
        }
    }
}

impl Drop for ChatStream {
    fn drop(&mut self) {
        if self.drain_tail_on_completion
            && !self.execution_owns_completion
            && matches!(&self.terminal, Some(Ok(_)))
            && self.ai_telemetry.is_some()
            && !std::thread::panicking()
        {
            drop(self.inner.take());
            self.finish_ai_telemetry();
        }
    }
}

impl Stream for ChatStream {
    type Item = Result<AssistantStreamEvent, CallFailure>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.terminal.is_some() {
            if this.drain_tail_on_completion {
                ready!(poll_source_tail(&mut this.inner, cx));
            }
            this.finish_ai_telemetry();
            return Poll::Ready(None);
        }
        let Some(inner) = this.inner.as_mut() else {
            // Absorbed without a recorded terminal: the protocol failure was
            // delivered and EOF is permanent.
            return Poll::Ready(None);
        };
        let item = inner.as_mut().poll_next(cx);
        let delivery = match item {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => {
                // Premature EOF: one typed protocol failure, then permanent
                // EOF. The failure preserves the latest event-time partial.
                this.inner = None;
                let failure = this.source_failure(ProviderError::StreamError(
                    ProviderErrorSummary::attested_static("stream ended without a terminal event"),
                ));
                this.terminal = Some(Err(failure.clone()));
                Poll::Ready(Some(Err(failure)))
            }
            Poll::Ready(Some(Ok(event))) => {
                if let Some(partial) = event.latest_partial() {
                    this.partial = Some(partial.clone());
                }
                match &event {
                    AssistantStreamEvent::Done { reason, message } => {
                        let outcome = match reason {
                            _ if *reason != message.stop_reason => Err(CallFailure::from_partial(
                                Some(message.clone()),
                                ProviderError::StreamError(ProviderErrorSummary::attested_static(
                                    "provider terminal stop reasons disagree",
                                )),
                            )),
                            StopReason::Stop
                            | StopReason::Length
                            | StopReason::ToolUse
                            | StopReason::Deferred => Ok(message.clone()),
                            StopReason::Pending => Err(CallFailure::from_partial(
                                Some(message.clone()),
                                ProviderError::StreamError(ProviderErrorSummary::attested_static(
                                    "provider terminal carried a pending stop reason",
                                )),
                            )),
                            StopReason::Error | StopReason::Aborted => Err(
                                CallFailure::from_error_stop_reason(*reason, message.clone()),
                            ),
                        };
                        // Cache the terminal and retain the source for tail draining.
                        this.terminal = Some(outcome.clone());
                        Poll::Ready(Some(outcome.map(|_| event)))
                    }
                    AssistantStreamEvent::Error { reason, message } => {
                        let failure =
                            CallFailure::from_legacy_error_event(*reason, message.clone());
                        // Retain the source for the drain after terminal delivery.
                        this.terminal = Some(Err(failure.clone()));
                        Poll::Ready(Some(Err(failure)))
                    }
                    _ => Poll::Ready(Some(Ok(event))),
                }
            }
            Poll::Ready(Some(Err(cause))) => {
                let failure = this.source_failure(cause);
                this.inner = None;
                this.terminal = Some(Err(failure.clone()));
                Poll::Ready(Some(Err(failure)))
            }
        };
        if let (Poll::Ready(Some(item)), Some(observer)) = (&delivery, &this.ai_telemetry) {
            observer.on_delivery(item.as_ref());
        }
        delivery
    }
}

/// Call-local canonical state, including facts that emitted no public event.
/// Cancellation reads this snapshot before discarding buffered delivery.
#[derive(Clone, Default)]
pub(crate) struct ProducerSnapshot(std::sync::Arc<std::sync::Mutex<ProducerState>>);

#[derive(Default)]
struct ProducerState {
    projection: Option<crate::transcript::ProjectionFacts>,
    message: Option<crate::message::AssistantMessage>,
    response_received: bool,
    #[cfg(any(feature = "builtins", feature = "provider-protocol-common", test))]
    terminal_delivered: bool,
}

impl ProducerSnapshot {
    #[cfg(any(feature = "builtins", feature = "provider-protocol-common", test))]
    pub(crate) fn with_projection(facts: crate::transcript::ProjectionFacts) -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(ProducerState {
            projection: Some(facts),
            ..Default::default()
        })))
    }
    #[cfg(any(feature = "builtins", feature = "provider-protocol-common", test))]
    fn project_event(&self, event: &mut AssistantStreamEvent) {
        if let Some(facts) = &self.0.lock().expect("producer snapshot lock").projection {
            event.set_projection(facts);
        }
    }

    #[cfg(any(feature = "builtins", feature = "provider-protocol-common", test))]
    pub(crate) fn terminal_delivered(&self) -> bool {
        self.0
            .lock()
            .expect("producer snapshot lock")
            .terminal_delivered
    }
    pub(crate) fn record(&self, message: &crate::message::AssistantMessage) {
        let mut state = self.0.lock().expect("producer snapshot lock");
        let mut message = message.clone();
        if let Some(facts) = &state.projection {
            message.projection = Some(facts.clone());
        }
        state.message = Some(message);
    }

    /// Freeze successful transport acquisition before reading response data.
    #[cfg(any(feature = "builtins", feature = "provider-protocol-common"))]
    pub(crate) fn received_response(&self) {
        self.0
            .lock()
            .expect("producer snapshot lock")
            .response_received = true;
    }

    pub(crate) fn failure(&self, cause: ProviderError) -> ProviderError {
        let state = self.0.lock().expect("producer snapshot lock");
        let cause = match &state.message {
            Some(message) => cause.with_partial(message.clone()),
            None => cause,
        };
        let cause = if let Some(facts) = &state.projection {
            cause.with_projection(facts)
        } else {
            cause
        };
        if state.response_received {
            cause.after_response()
        } else {
            cause
        }
    }
}

/// Gives cancellation priority over buffered events while retaining the
/// producer's canonical partial, then closes delivery permanently.
#[cfg(any(feature = "builtins", feature = "provider-protocol-common", test))]
pub(crate) struct CancelAwareReceiverStream {
    rx: Option<tokio::sync::mpsc::Receiver<Result<AssistantStreamEvent, ProviderError>>>,
    cancelled: Pin<Box<WaitForCancellationFutureOwned>>,
    snapshot: ProducerSnapshot,
}

#[cfg(any(feature = "builtins", feature = "provider-protocol-common", test))]
impl CancelAwareReceiverStream {
    #[cfg(test)]
    pub(crate) fn new(
        rx: tokio::sync::mpsc::Receiver<Result<AssistantStreamEvent, ProviderError>>,
        cancel: CancellationToken,
    ) -> Self {
        Self::with_snapshot(rx, cancel, ProducerSnapshot::default())
    }

    pub(crate) fn with_snapshot(
        rx: tokio::sync::mpsc::Receiver<Result<AssistantStreamEvent, ProviderError>>,
        cancel: CancellationToken,
        snapshot: ProducerSnapshot,
    ) -> Self {
        Self {
            rx: Some(rx),
            cancelled: Box::pin(cancel.cancelled_owned()),
            snapshot,
        }
    }
}

#[cfg(any(feature = "builtins", feature = "provider-protocol-common", test))]
impl Stream for CancelAwareReceiverStream {
    type Item = Result<AssistantStreamEvent, ProviderError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.rx.is_none() {
            return Poll::Ready(None);
        }
        if this.snapshot.terminal_delivered() {
            this.rx.take();
            return Poll::Ready(None);
        }
        if this.cancelled.as_mut().poll(cx).is_ready() {
            this.snapshot
                .0
                .lock()
                .expect("producer snapshot lock")
                .terminal_delivered = true;
            this.rx.take();
            return Poll::Ready(Some(Err(this.snapshot.failure(ProviderError::Cancelled))));
        }

        let mut item = this
            .rx
            .as_mut()
            .expect("active receiver stream")
            .poll_recv(cx);
        if let Poll::Ready(Some(Ok(event))) = &mut item {
            this.snapshot.project_event(event);
        }
        if let Poll::Ready(Some(Err(error))) = &mut item {
            *error = this
                .snapshot
                .failure(std::mem::replace(error, ProviderError::Cancelled));
        }
        if matches!(item, Poll::Ready(None)) {
            let mut state = this.snapshot.0.lock().expect("producer snapshot lock");
            let premature = state.projection.is_some() && !state.terminal_delivered;
            if premature {
                state.terminal_delivered = true;
            }
            drop(state);
            this.rx.take();
            return if premature {
                Poll::Ready(Some(Err(this.snapshot.failure(
                    ProviderError::StreamError(
                        crate::provider::ProviderErrorSummary::attested_static(
                            "stream ended without a terminal event",
                        ),
                    ),
                ))))
            } else {
                Poll::Ready(None)
            };
        }
        if matches!(&item, Poll::Ready(Some(Ok(event))) if event.is_terminal())
            || matches!(&item, Poll::Ready(Some(Err(_))))
        {
            // Resource owners observe delivery while the receiver closes.
            this.snapshot
                .0
                .lock()
                .expect("producer snapshot lock")
                .terminal_delivered = true;
            this.rx.take();
        }
        item
    }
}

#[cfg(any(feature = "builtins", feature = "provider-protocol-common", test))]
pub(crate) async fn send_or_cancel(
    cancel: &CancellationToken,
    tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
    item: Result<AssistantStreamEvent, ProviderError>,
) -> Result<bool, ProviderError> {
    tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(ProviderError::Cancelled),
        sent = tx.send(item) => Ok(sent.is_ok()),
    }
}

#[cfg(test)]
mod cancellation_tests {
    use std::future::Future as _;

    use futures_util::StreamExt;
    use tokio_util::sync::CancellationToken;

    use super::{CancelAwareReceiverStream, send_or_cancel};
    use crate::provider::{ProviderError, ProviderErrorSummary};

    #[cfg(feature = "provider-anthropic")]
    #[tokio::test]
    async fn cancellation_retains_silent_producer_state_at_the_barrier() {
        let cancel = CancellationToken::new();
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let snapshot = super::ProducerSnapshot::default();
        let mut mapper = crate::api::anthropic::AnthropicMapper::new();
        mapper.set_snapshot(snapshot.clone());
        // Silent facts belong to the producer even when no public snapshot
        // was queued. A task barrier ensures they exist before cancellation.
        let (published_tx, published_rx) = tokio::sync::oneshot::channel();
        let producer_cancel = cancel.clone();
        let producer = tokio::spawn(async move {
            let _keep_sender = tx;
            assert!(
                mapper
                    .process(crate::api::anthropic::AnthropicEvent::MessageDelta {
                        stop_reason: None,
                        usage: crate::api::anthropic::AnthropicUsageDelta {
                            input_tokens: Some(10),
                            output_tokens: Some(7),
                            ..Default::default()
                        },
                    })
                    .is_empty()
            );
            assert!(
                mapper
                    .process(crate::api::anthropic::AnthropicEvent::ContentBlockStart {
                        index: 0,
                        block_type: crate::api::anthropic::ContentBlockType::RedactedThinking {
                            data: "opaque-artificial".into(),
                        },
                    })
                    .is_empty()
            );
            published_tx.send(()).unwrap();
            producer_cancel.cancelled().await;
        });
        let stream = super::ChatStream::new(Box::pin(CancelAwareReceiverStream::with_snapshot(
            rx,
            cancel.clone(),
            snapshot,
        )));
        published_rx.await.unwrap();
        cancel.cancel();
        let failure = stream.result().await.expect_err("one cancellation");
        assert_eq!(failure.message().usage.output_tokens, 7);
        assert_eq!(failure.message().content.len(), 1);
        assert_eq!(failure.message().stop_reason, super::StopReason::Aborted);
        assert_eq!(failure.cause(), &ProviderError::Cancelled);
        producer.await.unwrap();
    }

    #[tokio::test]
    async fn backpressured_producer_cancels_and_buffered_events_are_discarded() {
        let cancel = CancellationToken::new();
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        tx.send(Err(ProviderError::StreamError(
            ProviderErrorSummary::redacted(),
        )))
        .await
        .expect("prefill event channel");
        let mut stream = CancelAwareReceiverStream::new(rx, cancel.clone());
        let producer_cancel = cancel.clone();
        let producer_tx = tx.clone();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let producer = tokio::spawn(async move {
            let mut started_tx = Some(started_tx);
            let mut blocked_send = Box::pin(send_or_cancel(
                &producer_cancel,
                &producer_tx,
                Err(ProviderError::StreamError(ProviderErrorSummary::redacted())),
            ));
            std::future::poll_fn(move |cx| {
                let result = blocked_send.as_mut().poll(cx);
                if result.is_pending()
                    && let Some(started_tx) = started_tx.take()
                {
                    started_tx.send(()).expect("signal producer backpressure");
                }
                result
            })
            .await
        });

        started_rx.await.expect("producer reached backpressure");
        cancel.cancel();

        let producer_result = tokio::time::timeout(std::time::Duration::from_secs(1), producer)
            .await
            .expect("backpressured producer must terminate promptly")
            .expect("producer task did not panic");
        assert!(matches!(producer_result, Err(ProviderError::Cancelled)));
        assert!(matches!(
            stream.next().await,
            Some(Err(ProviderError::Cancelled))
        ));
        assert!(
            stream.next().await.is_none(),
            "buffered pre-cancel events must be discarded"
        );
    }

    #[tokio::test]
    async fn backpressured_producer_exits_at_receiver_drop_barrier() {
        let cancel = CancellationToken::new();
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        tx.send(Err(ProviderError::Cancelled)).await.unwrap();
        let (blocked_tx, blocked_rx) = tokio::sync::oneshot::channel();
        let producer = tokio::spawn(async move {
            let mut blocked_tx = Some(blocked_tx);
            let mut send = Box::pin(send_or_cancel(&cancel, &tx, Err(ProviderError::Cancelled)));
            std::future::poll_fn(move |cx| {
                let outcome = send.as_mut().poll(cx);
                if outcome.is_pending()
                    && let Some(signal) = blocked_tx.take()
                {
                    signal.send(()).unwrap();
                }
                outcome
            })
            .await
        });
        blocked_rx.await.unwrap();
        drop(rx);
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(1), producer)
                .await
                .unwrap()
                .unwrap(),
            Ok(false)
        );
    }

    #[tokio::test]
    async fn idle_live_sender_does_not_hide_cancellation_from_pending_consumer() {
        let cancel = CancellationToken::new();
        let (_idle_tx, rx) = tokio::sync::mpsc::channel(1);
        let mut stream = CancelAwareReceiverStream::new(rx, cancel.clone());
        let (pending_tx, pending_rx) = tokio::sync::oneshot::channel();
        let consumer = tokio::spawn(async move {
            let mut pending_tx = Some(pending_tx);
            let first = {
                let mut next = Box::pin(stream.next());
                std::future::poll_fn(move |cx| {
                    let result = next.as_mut().poll(cx);
                    if result.is_pending()
                        && let Some(pending_tx) = pending_tx.take()
                    {
                        pending_tx.send(()).expect("signal pending consumer");
                    }
                    result
                })
                .await
            };
            (first, stream.next().await)
        });

        pending_rx.await.expect("consumer reached Pending");
        cancel.cancel();

        let (first, second) = tokio::time::timeout(std::time::Duration::from_secs(1), consumer)
            .await
            .expect("pending consumer must wake promptly on cancellation")
            .expect("consumer task did not panic");
        assert!(matches!(first, Some(Err(ProviderError::Cancelled))));
        assert!(second.is_none());
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;

    #[test]
    fn setup_fallback_preserves_an_attached_partial() {
        let record = crate::ChatModelRecord::new(
            "requested-provider",
            crate::ModelInfo::new(
                "requested-model",
                "Requested model",
                crate::WireApi::OpenAiCompletions,
                crate::registry::ModelCapabilities::new(4096, 512),
            ),
        );
        let mut partial = crate::test_support::base_assistant();
        partial.provider.clear();
        partial.model = "producer-model".into();
        partial.stop_reason = StopReason::Error;
        partial.error_message = Some("producer diagnostic".into());
        partial.usage.output_tokens = 9;
        partial.projection = Some(terminal_facts());

        let cause = ProviderError::Timeout;
        let failure = CallFailure::from_setup(&record, cause.clone().with_partial(partial.clone()));
        assert_eq!(failure.into_parts(), (partial, cause));
    }

    #[tokio::test]
    async fn richer_producer_projection_envelope_retains_usage_and_handoff() {
        let facts = crate::transcript::ProjectionFacts {
            source_prefix: crate::transcript::SourcePrefix {
                message_count: 0,
                sha256: "4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945".into(),
            },
            temporal: crate::transcript::TemporalProjectionFacts {
                mode: crate::transcript::ProjectionMode::Lossless,
                affected_ranges: vec![],
            },
            replay: Default::default(),
        };
        let delivered = crate::test_support::base_assistant();
        let mut richer = delivered.clone();
        richer.usage.output_tokens = 9;
        richer.duration_ms = Some(23);
        richer.content = vec![crate::message::AssistantContent::Text {
            text: "producer-only tail".into(),
            signature: None,
        }];
        let cause = ProviderError::StreamError(crate::provider::ProviderErrorSummary::sanitized(
            "synthetic",
        ))
        .with_partial(richer.clone())
        .after_response();
        let mut stream = with_projection(
            Box::pin(futures_util::stream::iter(vec![
                Ok(AssistantStreamEvent::Start { partial: delivered }),
                Err(cause),
            ])),
            facts.clone(),
        );
        assert!(stream.next().await.unwrap().is_ok());
        let cause = stream.next().await.unwrap().unwrap_err();
        assert!(cause.response_received());
        let (partial, cause) = cause.into_stream_parts();
        let partial = partial.unwrap();
        assert_eq!(partial.content, richer.content);
        assert_eq!(partial.usage, richer.usage);
        assert_eq!(partial.duration_ms, richer.duration_ms);
        assert_eq!(partial.projection, Some(facts));
        assert!(matches!(cause, ProviderError::StreamError(_)));
    }

    fn terminal_facts() -> crate::transcript::ProjectionFacts {
        crate::transcript::ProjectionFacts {
            source_prefix: crate::transcript::SourcePrefix {
                message_count: 0,
                sha256: "original-prefix".into(),
            },
            temporal: crate::transcript::TemporalProjectionFacts {
                mode: crate::transcript::ProjectionMode::Lossless,
                affected_ranges: vec![],
            },
            replay: Default::default(),
        }
    }

    async fn terminal_receiver(
        queued_tail: bool,
    ) -> (
        CancelAwareReceiverStream,
        tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
    ) {
        let (tx, rx) = tokio::sync::mpsc::channel(2);
        tx.send(Err(ProviderError::StreamError(
            crate::provider::ProviderErrorSummary::sanitized("first failure"),
        )))
        .await
        .unwrap();
        if queued_tail {
            tx.send(Ok(AssistantStreamEvent::Start {
                partial: crate::test_support::base_assistant(),
            }))
            .await
            .unwrap();
        }
        (
            CancelAwareReceiverStream::with_snapshot(
                rx,
                tokio_util::sync::CancellationToken::new(),
                ProducerSnapshot::with_projection(terminal_facts()),
            ),
            tx,
        )
    }

    #[tokio::test]
    async fn receiver_error_closes_a_live_sender_and_becomes_permanent_eof() {
        let (mut stream, tx) = terminal_receiver(false).await;
        let error = stream.next().await.unwrap().unwrap_err();
        let (partial, cause) = error.into_stream_parts();
        assert!(matches!(cause, ProviderError::StreamError(_)));
        assert_eq!(partial.unwrap().projection, Some(terminal_facts()));
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(50), stream.next())
                .await
                .expect("terminal is immediate EOF")
                .is_none()
        );
        assert!(tx.is_closed());
    }

    #[tokio::test]
    async fn receiver_error_discards_queued_tail_before_closing_delivery() {
        let (mut stream, tx) = terminal_receiver(true).await;
        assert!(stream.next().await.unwrap().is_err());
        assert!(
            stream.next().await.is_none(),
            "queued tail must not survive terminal failure"
        );
        assert!(tx.is_closed());
    }

    #[tokio::test]
    async fn receiver_cancellation_cannot_replace_a_delivered_error_eof_or_done() {
        for kind in 0..3 {
            let (tx, rx) = tokio::sync::mpsc::channel(2);
            let cancel = tokio_util::sync::CancellationToken::new();
            if kind == 0 {
                tx.send(Err(ProviderError::StreamError(
                    crate::provider::ProviderErrorSummary::sanitized("first failure"),
                )))
                .await
                .unwrap();
            }
            if kind == 2 {
                tx.send(Ok(AssistantStreamEvent::Done {
                    reason: StopReason::Stop,
                    message: crate::test_support::base_assistant(),
                }))
                .await
                .unwrap();
            }
            if kind == 1 {
                drop(tx);
            }
            let snapshot = ProducerSnapshot::with_projection(terminal_facts());
            let mut stream =
                CancelAwareReceiverStream::with_snapshot(rx, cancel.clone(), snapshot.clone());
            let first = stream.next().await.unwrap();
            if kind == 2 {
                assert!(first.unwrap().is_terminal());
            } else {
                let error = first.unwrap_err();
                let (partial, cause) = error.into_stream_parts();
                assert!(matches!(cause, ProviderError::StreamError(_)));
                assert_eq!(partial.unwrap().projection, Some(terminal_facts()));
            }
            assert!(snapshot.terminal_delivered());
            cancel.cancel();
            assert!(
                stream.next().await.is_none(),
                "no cancellation after terminal kind {kind}"
            );
            assert!(stream.next().await.is_none());
        }
    }

    struct CountedProjectionStream {
        items: std::collections::VecDeque<Result<AssistantStreamEvent, ProviderError>>,
        polls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        drops: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }
    impl Stream for CountedProjectionStream {
        type Item = Result<AssistantStreamEvent, ProviderError>;
        fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let this = self.get_mut();
            this.polls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Poll::Ready(this.items.pop_front())
        }
    }
    impl Drop for CountedProjectionStream {
        fn drop(&mut self) {
            self.drops.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }
    #[tokio::test]
    async fn projection_terminal_drains_source_tail_before_eof() {
        for kind in 0..3 {
            let polls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let drops = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let mut message = crate::test_support::base_assistant();
            message.usage.output_tokens = 7;
            let mut items = std::collections::VecDeque::new();
            if kind == 0 {
                items.push_back(Err(ProviderError::StreamError(
                    crate::provider::ProviderErrorSummary::sanitized("first failure"),
                )
                .with_partial(message.clone())));
            }
            if kind == 2 {
                items.push_back(Ok(AssistantStreamEvent::Done {
                    reason: StopReason::Stop,
                    message: message.clone(),
                }));
            }
            if kind != 1 {
                items.push_back(Ok(AssistantStreamEvent::Start {
                    partial: crate::test_support::base_assistant(),
                }));
            }
            let inner = CountedProjectionStream {
                items,
                polls: polls.clone(),
                drops: drops.clone(),
            };
            let mut stream = with_projection(Box::pin(inner), terminal_facts());
            let first = stream.next().await.unwrap();
            if kind == 2 {
                let event = first.unwrap();
                assert_eq!(event.latest_partial().unwrap().usage.output_tokens, 7);
                assert_eq!(
                    event.latest_partial().unwrap().projection,
                    Some(terminal_facts())
                );
            } else {
                let (partial, cause) = first.unwrap_err().into_stream_parts();
                assert!(matches!(cause, ProviderError::StreamError(_)));
                let partial = partial.unwrap();
                assert_eq!(partial.projection, Some(terminal_facts()));
                if kind == 0 {
                    assert_eq!(partial.usage.output_tokens, 7);
                }
            }
            if kind == 2 {
                assert_eq!(
                    drops.load(std::sync::atomic::Ordering::SeqCst),
                    0,
                    "business terminal retains the source for result draining"
                );
            } else {
                assert_eq!(
                    drops.load(std::sync::atomic::Ordering::SeqCst),
                    1,
                    "transport/EOF terminals release the source"
                );
            }
            assert!(stream.next().await.is_none());
            assert!(stream.next().await.is_none());
            assert_eq!(
                polls.load(std::sync::atomic::Ordering::SeqCst),
                if kind == 2 { 3 } else { 1 },
                "business terminal drains the repeated tail and source EOF"
            );
            assert_eq!(drops.load(std::sync::atomic::Ordering::SeqCst), 1);
        }
    }
}

impl ProviderError {
    pub(crate) fn with_projection(mut self, facts: &crate::transcript::ProjectionFacts) -> Self {
        if let Self::StreamFailure(failure) = &mut self {
            failure.set_projection(facts);
            return self;
        }
        let mut message = crate::stream::setup_failure_message(&self);
        message.projection = Some(facts.clone());
        self.with_partial(message)
    }
}
