//! Detached, process-local snapshots for tests and small diagnostic sessions.
//! This recorder retains observations until it is dropped; it is not an exporter,
//! bounded production store, query service or transport propagation adapter.

use crate::backend::{ContextBackend, InstrumentBackend, MetricsBackend, SpanBackend};
use crate::{
    Attributes, ErrorStatus, InstrumentDefinition, Result, SpanOptions, SpanStatus,
    TelemetryContext,
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
pub struct RecordedEvent {
    pub name: String,
    pub attributes: Attributes,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecordedSpan {
    /// Local to one recorder, in start order.
    pub id: usize,
    pub parent_id: Option<usize>,
    pub name: String,
    pub attributes: Attributes,
    pub events: Vec<RecordedEvent>,
    pub status: SpanStatus,
    /// Completion order, independent of start order. None means unfinished.
    pub end_sequence: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecordedMeasurement {
    pub definition: InstrumentDefinition,
    pub value: f64,
    pub attributes: Attributes,
    /// A histogram's explicit context, only when owned by this recorder.
    pub span_id: Option<usize>,
}

#[derive(Debug)]
struct Entry {
    record: RecordedSpan,
    explicit_status: bool,
}

#[derive(Debug, Default)]
struct State {
    spans: Vec<Entry>,
    measurements: Vec<RecordedMeasurement>,
    next_end: usize,
}

/// Cloneable recorder handle. All getters copy data; editing a returned snapshot
/// cannot change a live observation. Independent instances never share state.
#[derive(Clone, Debug, Default)]
pub struct Recorder {
    state: Arc<Mutex<State>>,
}

impl Recorder {
    pub fn context(&self) -> TelemetryContext {
        TelemetryContext::new(Arc::new(MemoryContext {
            recorder: self.clone(),
            span_id: None,
        }))
    }
    pub fn spans(&self) -> Vec<RecordedSpan> {
        self.state
            .lock()
            .expect("recorder poisoned")
            .spans
            .iter()
            .map(|s| s.record.clone())
            .collect()
    }
    pub fn measurements(&self) -> Vec<RecordedMeasurement> {
        self.state
            .lock()
            .expect("recorder poisoned")
            .measurements
            .clone()
    }
}

#[derive(Debug)]
struct MemoryContext {
    recorder: Recorder,
    span_id: Option<usize>,
}

impl ContextBackend for MemoryContext {
    fn start_span(&self, options: &SpanOptions) -> Result<Arc<dyn SpanBackend>> {
        let mut state = self.recorder.state.lock().expect("recorder poisoned");
        if self
            .span_id
            .is_some_and(|id| state.spans[id].record.end_sequence.is_some())
        {
            return Ok(crate::noop::Noop::with_metrics(self.metrics()));
        }
        let id = state.spans.len();
        state.spans.push(Entry {
            record: RecordedSpan {
                id,
                parent_id: self.span_id,
                name: options.name.clone(),
                attributes: options.attributes.clone(),
                events: Vec::new(),
                status: SpanStatus::Ok,
                end_sequence: None,
            },
            explicit_status: false,
        });
        Ok(Arc::new(Self {
            recorder: self.recorder.clone(),
            span_id: Some(id),
        }))
    }
    fn metrics(&self) -> Arc<dyn MetricsBackend> {
        Arc::new(self.recorder.clone())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl MemoryContext {
    fn update(&self, f: impl FnOnce(&mut Entry)) {
        let mut state = self.recorder.state.lock().expect("recorder poisoned");
        if let Some(id) = self.span_id {
            let entry = &mut state.spans[id];
            if entry.record.end_sequence.is_none() {
                f(entry);
            }
        }
    }
}

impl SpanBackend for MemoryContext {
    fn set_attributes(&self, attributes: &Attributes) -> Result<()> {
        self.update(|entry| entry.record.attributes.extend(attributes.clone()));
        Ok(())
    }
    fn add_event(&self, name: &str, attributes: &Attributes) -> Result<()> {
        self.update(|entry| {
            entry.record.events.push(RecordedEvent {
                name: name.into(),
                attributes: attributes.clone(),
            })
        });
        Ok(())
    }
    fn set_status(&self, status: SpanStatus) -> Result<()> {
        self.update(|entry| {
            entry.record.status = status;
            entry.explicit_status = true;
        });
        Ok(())
    }
    fn finish(&self, error: Option<ErrorStatus>) -> Result<()> {
        let mut state = self.recorder.state.lock().expect("recorder poisoned");
        if let Some(id) = self.span_id {
            let sequence = state.next_end;
            let entry = &mut state.spans[id];
            if entry.record.end_sequence.is_none() {
                if !entry.explicit_status
                    && let Some(error) = error
                {
                    entry.record.status = SpanStatus::Error { error: Some(error) };
                }
                entry.record.end_sequence = Some(sequence);
                state.next_end += 1;
            }
        }
        Ok(())
    }
}

impl MetricsBackend for Recorder {
    fn instrument(&self, definition: &InstrumentDefinition) -> Result<Arc<dyn InstrumentBackend>> {
        Ok(Arc::new(MemoryInstrument {
            recorder: self.clone(),
            definition: definition.clone(),
        }))
    }
}

#[derive(Debug)]
struct MemoryInstrument {
    recorder: Recorder,
    definition: InstrumentDefinition,
}
impl InstrumentBackend for MemoryInstrument {
    fn record(
        &self,
        value: f64,
        attributes: &Attributes,
        context: Option<&TelemetryContext>,
    ) -> Result<()> {
        let span_id = context
            .and_then(|ctx| ctx.backend().as_any().downcast_ref::<MemoryContext>())
            .filter(|ctx| Arc::ptr_eq(&ctx.recorder.state, &self.recorder.state))
            .and_then(|ctx| ctx.span_id);
        self.recorder
            .state
            .lock()
            .expect("recorder poisoned")
            .measurements
            .push(RecordedMeasurement {
                definition: self.definition.clone(),
                value,
                attributes: attributes.clone(),
                span_id,
            });
        Ok(())
    }
}
