use crate::backend::{InstrumentBackend, MetricsBackend};
use crate::{Attributes, Result, TelemetryContext};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MetricKind {
    Counter,
    UpDownCounter,
    Histogram,
}

/// Owned factory metadata. Histogram boundaries are hints, finite and strictly
/// increasing by caller contract. No runtime schema validation is inserted.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InstrumentDefinition {
    pub name: String,
    pub kind: MetricKind,
    pub description: Option<String>,
    pub unit: Option<String>,
    pub boundaries: Option<Vec<f64>>,
}

impl InstrumentDefinition {
    pub fn new(name: impl Into<String>, kind: MetricKind) -> Self {
        Self {
            name: name.into(),
            kind,
            description: None,
            unit: None,
            boundaries: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Metrics {
    backend: Arc<dyn MetricsBackend>,
}

impl Metrics {
    pub fn new(backend: Arc<dyn MetricsBackend>) -> Self {
        Self { backend }
    }
    pub fn counter(&self, mut definition: InstrumentDefinition) -> Result<Counter> {
        definition.kind = MetricKind::Counter;
        Ok(Counter(self.backend.instrument(&definition)?))
    }
    pub fn up_down_counter(&self, mut definition: InstrumentDefinition) -> Result<UpDownCounter> {
        definition.kind = MetricKind::UpDownCounter;
        Ok(UpDownCounter(self.backend.instrument(&definition)?))
    }
    pub fn histogram(&self, mut definition: InstrumentDefinition) -> Result<Histogram> {
        definition.kind = MetricKind::Histogram;
        Ok(Histogram(self.backend.instrument(&definition)?))
    }
}

#[derive(Clone, Debug)]
pub struct Counter(Arc<dyn InstrumentBackend>);
impl Counter {
    pub fn add(&self, value: f64, attributes: &Attributes) -> Result<()> {
        self.0.record(value, attributes, None)
    }
}

#[derive(Clone, Debug)]
pub struct UpDownCounter(Arc<dyn InstrumentBackend>);
impl UpDownCounter {
    pub fn add(&self, value: f64, attributes: &Attributes) -> Result<()> {
        self.0.record(value, attributes, None)
    }
}

#[derive(Clone, Debug)]
pub struct Histogram(Arc<dyn InstrumentBackend>);
impl Histogram {
    pub fn record(
        &self,
        value: f64,
        attributes: &Attributes,
        context: Option<&TelemetryContext>,
    ) -> Result<()> {
        self.0.record(value, attributes, context)
    }
}
