use crate::backend::{ContextBackend, InstrumentBackend, MetricsBackend, SpanBackend};
use crate::{
    Attributes, ErrorStatus, InstrumentDefinition, Result, SpanOptions, SpanStatus,
    TelemetryContext,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct Noop {
    metrics: Arc<dyn MetricsBackend>,
}
impl Default for Noop {
    fn default() -> Self {
        Self {
            metrics: Arc::new(NoopMetrics),
        }
    }
}
impl Noop {
    pub(crate) fn with_metrics(metrics: Arc<dyn MetricsBackend>) -> Arc<dyn SpanBackend> {
        Arc::new(Self { metrics })
    }
}
impl ContextBackend for Noop {
    fn start_span(&self, _: &SpanOptions) -> Result<Arc<dyn SpanBackend>> {
        Ok(Self::with_metrics(self.metrics.clone()))
    }
    fn metrics(&self) -> Arc<dyn MetricsBackend> {
        self.metrics.clone()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl SpanBackend for Noop {
    fn set_attributes(&self, _: &Attributes) -> Result<()> {
        Ok(())
    }
    fn add_event(&self, _: &str, _: &Attributes) -> Result<()> {
        Ok(())
    }
    fn set_status(&self, _: SpanStatus) -> Result<()> {
        Ok(())
    }
    fn finish(&self, _: Option<ErrorStatus>) -> Result<()> {
        Ok(())
    }
}
#[derive(Debug)]
struct NoopMetrics;
impl MetricsBackend for NoopMetrics {
    fn instrument(&self, _: &InstrumentDefinition) -> Result<Arc<dyn InstrumentBackend>> {
        Ok(Arc::new(NoopInstrument))
    }
}
#[derive(Debug)]
struct NoopInstrument;
impl InstrumentBackend for NoopInstrument {
    fn record(&self, _: f64, _: &Attributes, _: Option<&TelemetryContext>) -> Result<()> {
        Ok(())
    }
}
