//! Process-local telemetry carrier: a frozen parent [`Span`] plus the
//! source [`Dispatch`] captured at [`TelemetryContext::from_span`] time
//! (task 20.54, DEL-12).
//!
//! This is the pinned *carrier substrate* for explicit host telemetry, not an
//! SDK propagation feature: SDK operations (the later P20-TELEMETRY consumers)
//! accept an optional owned context and observe through it. The context is
//! process-local, never serialized, and never placed on the wire.
//!
//! ## Owned tracing difference
//!
//! The carrier obtains the Span's creating dispatcher through
//! `Span::with_subscriber` and freezes the pair `(parent, Dispatch)` at
//! [`from_span`](TelemetryContext::from_span) time. Observation
//! never adopts poll-time ambient dispatch - the frozen pair, not whatever is
//! default when the wrapped work actually polls, routes the observation.
//!
//! ## Scoped synchronous sections
//!
//! [`in_scope`](TelemetryContext::in_scope) and every
//! [`instrument`](TelemetryContext::instrument) poll or drop run one short
//! synchronous section under the frozen pair - the dispatcher is installed as
//! the section default and the parent is entered for the section only. Enter
//! guards are created and dropped inside each section, so no guard crosses an
//! await, each outer poll polls the wrapped future exactly once, and `Drop`
//! sections scope the wrapped future's drop even when it was never polled.
//! The disabled ([`Noop`](TelemetryContext::noop)) carrier runs the same
//! sections under a no-op dispatcher: business work still executes exactly
//! once with its output preserved, while observation inside the section is
//! explicitly silenced instead of leaking to ambient dispatch.
//!
//! ## Explicit non-goals
//!
//! No global subscriber initialization, exporter, sampling or flush (host
//! owned); no domain span names, status or settlement vocabulary (domain
//! owners interpret their own results); no automatic SDK call/attempt spans;
//! no claim of equivalence to a generic span callback/schema DSL.

use std::future::Future;
use std::pin::Pin;

use tracing::Span;
use tracing_core::Dispatch;
use tracing_core::dispatcher;

/// Frozen telemetry carrier: an owned parent span plus the source dispatch
/// captured at [`from_span`](Self::from_span) time, or an explicit disabled
/// state.
///
/// `Clone` retains the same parent and dispatch; the context is `Send + Sync`
/// so producer tasks can carry it. Omitting a context never captures ambient
/// state - a host may explicitly pass `Span::current()` to opt in.
#[derive(Clone, Debug)]
pub struct TelemetryContext {
    state: CarrierState,
}

#[derive(Clone, Debug)]
enum CarrierState {
    /// Explicit disabled carrier: sections run under a no-op dispatcher.
    Noop,
    /// Frozen parent span and source dispatch captured at creation time.
    Active { parent: Span, dispatch: Dispatch },
}

impl TelemetryContext {
    /// Freeze the given parent together with its creating dispatch. Neither
    /// capture-time nor poll-time ambient dispatch can replace that source.
    ///
    /// A disabled span (`Span::none()` or subscriber-disabled) degrades to
    /// the explicit [`noop`](Self::noop) state.
    #[must_use]
    pub fn from_span(parent: Span) -> Self {
        let Some(dispatch) = parent.with_subscriber(|(_, dispatch)| dispatch.clone()) else {
            return Self::noop();
        };
        Self {
            state: CarrierState::Active { parent, dispatch },
        }
    }

    /// The explicit disabled carrier: business work still runs exactly once,
    /// observation inside its sections is explicitly silenced.
    #[must_use]
    pub fn noop() -> Self {
        Self {
            state: CarrierState::Noop,
        }
    }

    /// Run the synchronous body exactly once under the frozen parent and
    /// dispatch, restoring the caller context before returning.
    pub fn in_scope<T>(&self, f: impl FnOnce() -> T) -> T {
        self.scoped(f)
    }

    /// Wrap a future so each of its polls and its drop run inside the same
    /// scoped section as [`in_scope`](Self::in_scope). The wrapper is lazy
    /// (constructing it polls nothing), polls the wrapped future exactly once
    /// per outer poll, preserves [`std::task::Poll::Pending`] and the output type, and scopes
    /// the wrapped future's drop even if it is never polled.
    pub fn instrument<F: Future>(&self, future: F) -> Instrumented<F> {
        Instrumented {
            context: self.clone(),
            inner: Some(Box::pin(future)),
        }
    }

    /// One short synchronous section: install the carrier's dispatch as the
    /// section default, enter the parent span for the section only, run the
    /// body, then restore the caller context. The disabled carrier runs the
    /// body under a no-op dispatcher instead.
    fn scoped<T>(&self, f: impl FnOnce() -> T) -> T {
        match &self.state {
            CarrierState::Noop => dispatcher::with_default(&Dispatch::none(), f),
            CarrierState::Active { parent, dispatch } => dispatcher::with_default(dispatch, || {
                let _guard = parent.enter();
                f()
            }),
        }
    }
}

/// Future wrapper returned by [`TelemetryContext::instrument`].
///
/// Each poll and the drop of the wrapped future run inside the carrier's
/// scoped section; the boxed inner future keeps this wrapper `Unpin` without
/// `unsafe`. Business output, [`std::task::Poll::Pending`] propagation and laziness are
/// preserved; observation routes to the frozen pair (or is silenced for the
/// disabled carrier).
pub struct Instrumented<F> {
    context: TelemetryContext,
    inner: Option<Pin<Box<F>>>,
}

impl<F: Future> Future for Instrumented<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<F::Output> {
        let this = self.get_mut();
        let inner = this
            .inner
            .as_mut()
            .expect("Instrumented polled after completion");
        this.context.scoped(|| inner.as_mut().poll(cx))
    }
}

impl<F> Drop for Instrumented<F> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            self.context.scoped(|| drop(inner));
        }
    }
}
