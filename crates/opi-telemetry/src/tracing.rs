//! Optional tracing adapter. The host supplies a span and owns its subscriber.
//! The span's creating dispatcher is frozen, never replaced by poll-time ambient
//! state. Neutral spans use the static tracing name `opi.telemetry.span` and
//! carry their dynamic name in `telemetry.name`. Events/metrics carry neutral
//! metadata as debug fields; this is not an OpenTelemetry exporter.
//!
//! `telemetry.operation = "finish"` marks logical completion. Native tracing
//! close may be later while an already-entered synchronous scope is still alive.
//! Updates admitted before finish may be delivered concurrently; final fields
//! include all admitted updates. No subscriber callback runs under a state lock.

use crate::backend::{ContextBackend, InstrumentBackend, MetricsBackend, ScopeGuard, SpanBackend};
use crate::{
    Attributes, ErrorStatus, InstrumentDefinition, Result, SpanOptions, SpanStatus,
    TelemetryContext,
};
use ::tracing::Span;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use tracing_core::{Dispatch, dispatcher};

impl TelemetryContext {
    /// Freeze a span with its creating dispatch. A disabled span becomes neutral
    /// no-op, preserving ambient host tracing. No global subscriber is installed.
    pub fn from_span(parent: Span) -> Self {
        let Some(dispatch) = parent.with_subscriber(|(_, dispatch)| dispatch.clone()) else {
            return Self::noop();
        };
        Self::new(Arc::new(TracingContext {
            dispatch,
            scope: Arc::new(()),
            id: None,
            state: Mutex::new(State {
                span: Some(Arc::new(parent)),
                attributes: Attributes::new(),
                status: SpanStatus::Ok,
                explicit_status: false,
            }),
        }))
    }
}

#[derive(Debug)]
struct State {
    span: Option<Arc<Span>>,
    attributes: Attributes,
    status: SpanStatus,
    explicit_status: bool,
}

#[derive(Debug)]
struct TracingContext {
    dispatch: Dispatch,
    scope: Arc<()>,
    id: Option<u64>,
    state: Mutex<State>,
}

// Native tracing IDs can be recycled after close. Logical IDs stay unique for
// late histogram correlation without retaining the native span.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> u64 {
    let mut current = NEXT_ID.load(Ordering::Relaxed);
    loop {
        let next = current
            .checked_add(1)
            .expect("logical tracing identity exhausted");
        match NEXT_ID.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return current,
            Err(observed) => current = observed,
        }
    }
}

struct Guard {
    // Fields drop in declaration order: exit parent, then restore dispatch.
    _entered: ::tracing::span::EnteredSpan,
    _default: dispatcher::DefaultGuard,
}
impl ScopeGuard for Guard {}

impl TracingContext {
    fn span(&self) -> Option<Arc<Span>> {
        self.state
            .lock()
            .expect("tracing state poisoned")
            .span
            .clone()
    }
}

impl ContextBackend for TracingContext {
    fn start_span(&self, options: &SpanOptions) -> Result<Arc<dyn SpanBackend>> {
        // Cloning the active parent admits this child atomically with finish.
        let Some(parent) = self.span() else {
            return Ok(crate::noop::Noop::with_metrics(self.metrics()));
        };
        let id = next_id();
        let span = dispatcher::with_default(&self.dispatch, || {
            ::tracing::info_span!(parent: parent.as_ref(), "opi.telemetry.span", telemetry.name = %options.name,
                telemetry.id = id, telemetry.attributes = ?options.attributes, telemetry.status = ::tracing::field::Empty)
        });
        Ok(Arc::new(Self {
            dispatch: self.dispatch.clone(),
            scope: self.scope.clone(),
            id: Some(id),
            state: Mutex::new(State {
                span: Some(Arc::new(span)),
                attributes: options.attributes.clone(),
                status: SpanStatus::Ok,
                explicit_status: false,
            }),
        }))
    }
    fn metrics(&self) -> Arc<dyn MetricsBackend> {
        Arc::new(TracingMetrics {
            dispatch: self.dispatch.clone(),
            scope: self.scope.clone(),
        })
    }
    fn enter(&self) -> Box<dyn ScopeGuard + '_> {
        match self.span() {
            Some(span) => {
                let default = dispatcher::set_default(&self.dispatch);
                Box::new(Guard {
                    _entered: span.as_ref().clone().entered(),
                    _default: default,
                })
            }
            None => Box::new(()),
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl SpanBackend for TracingContext {
    fn set_attributes(&self, attributes: &Attributes) -> Result<()> {
        let update = {
            let mut state = self.state.lock().expect("tracing state poisoned");
            state.span.clone().map(|span| {
                state.attributes.extend(attributes.clone());
                (span, state.attributes.clone())
            })
        };
        if let Some((span, attributes)) = update {
            dispatcher::with_default(&self.dispatch, || {
                ::tracing::info!(parent: span.as_ref(), telemetry.operation = "attributes", telemetry.attributes = ?attributes);
            });
        }
        Ok(())
    }
    fn add_event(&self, name: &str, attributes: &Attributes) -> Result<()> {
        if let Some(span) = self.span() {
            dispatcher::with_default(&self.dispatch, || {
                ::tracing::info!(parent: span.as_ref(), telemetry.operation = "event", telemetry.name = name, telemetry.attributes = ?attributes);
            });
        }
        Ok(())
    }
    fn set_status(&self, status: SpanStatus) -> Result<()> {
        let span = {
            let mut state = self.state.lock().expect("tracing state poisoned");
            state.span.clone().inspect(|_| {
                state.status = status.clone();
                state.explicit_status = true;
            })
        };
        if let Some(span) = span {
            dispatcher::with_default(&self.dispatch, || {
                ::tracing::info!(parent: span.as_ref(), telemetry.operation = "status", telemetry.status = ?status);
            });
        }
        Ok(())
    }
    fn finish(&self, error: Option<ErrorStatus>) -> Result<()> {
        let finished = {
            let mut state = self.state.lock().expect("tracing state poisoned");
            if !state.explicit_status
                && let Some(error) = error
            {
                state.status = SpanStatus::Error { error: Some(error) };
            }
            state
                .span
                .take()
                .map(|span| (span, state.attributes.clone(), state.status.clone()))
        };
        if let Some((span, attributes, status)) = finished {
            span.record("telemetry.attributes", ::tracing::field::debug(&attributes));
            span.record("telemetry.status", ::tracing::field::debug(&status));
            dispatcher::with_default(&self.dispatch, || {
                ::tracing::info!(parent: span.as_ref(), telemetry.operation = "finish", telemetry.status = ?status, telemetry.attributes = ?attributes);
            });
        }
        Ok(())
    }
}

#[derive(Debug)]
struct TracingMetrics {
    dispatch: Dispatch,
    scope: Arc<()>,
}
impl MetricsBackend for TracingMetrics {
    fn instrument(&self, definition: &InstrumentDefinition) -> Result<Arc<dyn InstrumentBackend>> {
        Ok(Arc::new(TracingInstrument {
            dispatch: self.dispatch.clone(),
            scope: self.scope.clone(),
            definition: definition.clone(),
        }))
    }
}

#[derive(Debug)]
struct TracingInstrument {
    dispatch: Dispatch,
    scope: Arc<()>,
    definition: InstrumentDefinition,
}
impl InstrumentBackend for TracingInstrument {
    fn record(
        &self,
        value: f64,
        attributes: &Attributes,
        context: Option<&TelemetryContext>,
    ) -> Result<()> {
        // Correlation never changes the metric owner's frozen dispatcher.
        let context = context
            .and_then(|ctx| ctx.backend().as_any().downcast_ref::<TracingContext>())
            .filter(|ctx| Arc::ptr_eq(&ctx.scope, &self.scope))
            .and_then(|ctx| ctx.id);
        dispatcher::with_default(&self.dispatch, || {
            ::tracing::info!(parent: None, telemetry.operation = "measurement", telemetry.definition = ?self.definition,
                telemetry.value = value, telemetry.attributes = ?attributes, telemetry.context = ?context);
        });
        Ok(())
    }
}
