use crate::backend::{ContextBackend, SpanBackend};
use crate::{Attributes, ErrorStatus, Metrics, Result, SpanOptions, SpanStatus};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Explicit, cloneable parent and observation capability. Cloning or dropping a
/// context does not finish a logical span or cancel any business work.
#[derive(Clone, Debug)]
pub struct TelemetryContext {
    pub(crate) backend: Arc<dyn ContextBackend>,
}

impl Default for TelemetryContext {
    fn default() -> Self {
        Self::noop()
    }
}

impl TelemetryContext {
    pub fn new(backend: Arc<dyn ContextBackend>) -> Self {
        Self { backend }
    }

    /// Disable this interface's recording without changing host ambient tracing.
    pub fn noop() -> Self {
        Self::new(Arc::new(crate::noop::Noop::default()))
    }

    /// Begin immediately, before returning. No future must be polled to admit
    /// this span. The caller must retain the owner until its execution completes.
    pub fn start_span(&self, options: SpanOptions) -> Result<SpanOwner> {
        let backend = self.backend.start_span(&options)?;
        Ok(SpanOwner {
            span: TelemetrySpan { backend },
        })
    }

    pub fn metrics(&self) -> Metrics {
        Metrics::new(self.backend.metrics())
    }

    /// Execute exactly once and restore the caller's scope, also during unwind.
    pub fn in_scope<T>(&self, f: impl FnOnce() -> T) -> T {
        let _scope = self.backend.enter();
        f()
    }

    /// Lazy wrapper: one inner poll per outer poll, with the same scope during
    /// inner Drop even when unpolled. This wrapper never settles a logical span.
    /// Dropping it drops the inner future; cancellation follows that future's
    /// own contract. This adapter does not detach or spawn business execution.
    pub fn instrument<F: Future>(&self, future: F) -> Instrumented<F> {
        Instrumented {
            context: self.clone(),
            inner: Some(Box::pin(future)),
        }
    }

    /// Adapter identity, for local histogram correlation without global state.
    pub fn backend(&self) -> &dyn ContextBackend {
        self.backend.as_ref()
    }
}

/// Observation only; completion authority is held separately by [`SpanOwner`].
#[derive(Clone, Debug)]
pub struct TelemetrySpan {
    backend: Arc<dyn SpanBackend>,
}

impl TelemetrySpan {
    pub fn context(&self) -> TelemetryContext {
        TelemetryContext::new(self.backend.clone())
    }
    pub fn set_attributes(&self, attributes: &Attributes) -> Result<()> {
        self.backend.set_attributes(attributes)
    }
    pub fn add_event(&self, name: &str, attributes: &Attributes) -> Result<()> {
        self.backend.add_event(name, attributes)
    }
    pub fn set_status(&self, status: SpanStatus) -> Result<()> {
        self.backend.set_status(status)
    }
}

/// Unique execution-side authority. Drop deliberately does not report completion.
/// Losing this owner leaves an unfinished observation; applications must retain
/// it in the execution task, independently of user-side stream consumption.
#[derive(Debug)]
#[must_use = "retain the owner and explicitly finish when execution ends"]
pub struct SpanOwner {
    span: TelemetrySpan,
}

impl SpanOwner {
    pub fn span(&self) -> TelemetrySpan {
        self.span.clone()
    }
    pub fn context(&self) -> TelemetryContext {
        self.span.context()
    }

    /// Make one synchronous completion attempt. `error` describes an automatic
    /// business failure only if no explicit status was supplied. Backend errors
    /// propagate; there is no retry, hidden success or automatic panic mapping.
    pub fn finish(self, error: Option<ErrorStatus>) -> Result<()> {
        self.span.backend.finish(error)
    }
}

/// Scoped future adapter; no guard survives a poll and no Drop settles a span.
pub struct Instrumented<F> {
    context: TelemetryContext,
    inner: Option<Pin<Box<F>>>,
}

impl<F: Future> Future for Instrumented<F> {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<F::Output> {
        let this = self.get_mut();
        let inner = this.inner.as_mut().expect("instrumented future missing");
        this.context.in_scope(|| inner.as_mut().poll(cx))
    }
}

impl<F> Drop for Instrumented<F> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            self.context.in_scope(|| drop(inner));
        }
    }
}
