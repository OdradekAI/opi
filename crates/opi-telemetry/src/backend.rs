//! Synchronous adapter contracts. Methods may fail; callers receive that failure.
//! Adapters must not initialize global services, swallow failures or infer a
//! logical operation's end from handle Drop. Input values are trusted; schema
//! descriptions are metadata, not a new runtime validation policy.

use crate::{
    Attributes, ErrorStatus, InstrumentDefinition, Result, SpanOptions, SpanStatus,
    TelemetryContext,
};
use std::fmt::Debug;
use std::sync::Arc;

/// A short synchronous scope. Guards never cross a future poll or await.
pub trait ScopeGuard {}
impl ScopeGuard for () {}

/// Explicit parent context. Implementations must ignore child recording after
/// settlement while retaining the same metrics capability on inert children.
pub trait ContextBackend: Debug + Send + Sync + 'static {
    fn start_span(&self, options: &SpanOptions) -> Result<Arc<dyn SpanBackend>>;
    fn metrics(&self) -> Arc<dyn MetricsBackend>;
    fn enter(&self) -> Box<dyn ScopeGuard + '_> {
        Box::new(())
    }
    /// Adapter-private identity for local correlation. It is not a wire carrier.
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Observation backend. Mutations racing with finish must serialize: once
/// finish wins, late mutations have no effect. Explicit status overrides the
/// optional automatic error supplied to finish. Retained contexts stay usable
/// for metrics, but cannot reopen the completed span or record new children.
pub trait SpanBackend: ContextBackend {
    fn set_attributes(&self, attributes: &Attributes) -> Result<()>;
    fn add_event(&self, name: &str, attributes: &Attributes) -> Result<()>;
    fn set_status(&self, status: SpanStatus) -> Result<()>;
    fn finish(&self, error: Option<ErrorStatus>) -> Result<()>;
}

pub trait MetricsBackend: Debug + Send + Sync + 'static {
    fn instrument(&self, definition: &InstrumentDefinition) -> Result<Arc<dyn InstrumentBackend>>;
}

pub trait InstrumentBackend: Debug + Send + Sync + 'static {
    /// Finite measurements and nonnegative counter increments are caller
    /// preconditions. Malformed input behavior is unspecified, as upstream.
    fn record(
        &self,
        value: f64,
        attributes: &Attributes,
        context: Option<&TelemetryContext>,
    ) -> Result<()>;
}
