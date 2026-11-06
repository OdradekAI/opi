#![cfg(feature = "tracing")]
mod support;

use opi_telemetry::{Attributes, InstrumentDefinition, MetricKind, SpanOptions, TelemetryContext};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tracing_core::{
    Dispatch, Event, Subscriber, dispatcher,
    field::{Field, Visit},
    span::{Attributes as SpanAttributes, Id, Record},
};

#[derive(Default, Debug)]
struct Fields(BTreeMap<String, String>);
impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name().into(), format!("{value:?}"));
    }
}
#[derive(Default)]
struct State {
    spans: Vec<(Option<u64>, Fields)>,
    events: Vec<(Option<u64>, Fields)>,
    references: BTreeMap<u64, usize>,
    indices: BTreeMap<u64, usize>,
    closed: Vec<u64>,
}
struct Recording(Arc<Mutex<State>>);
impl Subscriber for Recording {
    fn enabled(&self, _: &tracing_core::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, attributes: &SpanAttributes<'_>) -> Id {
        let mut fields = Fields::default();
        attributes.record(&mut fields);
        let mut state = self.0.lock().unwrap();
        state
            .spans
            .push((attributes.parent().map(Id::into_u64), fields));
        let index = state.spans.len() - 1;
        let id = state
            .references
            .iter()
            .find(|(_, refs)| **refs == 0)
            .map(|(id, _)| *id)
            .unwrap_or(state.references.len() as u64 + 1);
        state.indices.insert(id, index);
        state.references.insert(id, 1);
        Id::from_u64(id)
    }
    fn record(&self, id: &Id, values: &Record<'_>) {
        let mut state = self.0.lock().unwrap();
        let index = state.indices[&id.into_u64()];
        values.record(&mut state.spans[index].1);
    }
    fn record_follows_from(&self, _: &Id, _: &Id) {}
    fn event(&self, event: &Event<'_>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        self.0
            .lock()
            .unwrap()
            .events
            .push((event.parent().map(Id::into_u64), fields));
    }
    fn enter(&self, _: &Id) {}
    fn exit(&self, _: &Id) {}
    fn clone_span(&self, id: &Id) -> Id {
        *self
            .0
            .lock()
            .unwrap()
            .references
            .get_mut(&id.into_u64())
            .unwrap() += 1;
        id.clone()
    }
    fn try_close(&self, id: Id) -> bool {
        let mut state = self.0.lock().unwrap();
        let count = state.references.get_mut(&id.into_u64()).unwrap();
        *count -= 1;
        if *count == 0 {
            state.closed.push(id.into_u64());
            true
        } else {
            false
        }
    }
}

fn fixture() -> (Arc<Mutex<State>>, TelemetryContext) {
    let state = Arc::new(Mutex::new(State::default()));
    let dispatch = Dispatch::new(Recording(state.clone()));
    let context = dispatcher::with_default(&dispatch, || {
        TelemetryContext::from_span(tracing::info_span!("host"))
    });
    (state, context)
}

#[test]
fn shared_lifecycle_routes_metadata_and_finishes_despite_retained_contexts() {
    let (state, context) = fixture();
    let kept = support::lifecycle(&context);
    let state = state.lock().unwrap();
    assert_eq!(state.spans.len(), 3, "host, request, child; no inert span");
    assert_eq!(state.spans[1].0, Some(1));
    assert_eq!(state.spans[2].0, Some(2));
    assert_eq!(state.spans[1].1.0["telemetry.name"], "request");
    assert_eq!(state.spans[1].1.0["telemetry.status"], "Ok");
    assert!(state.spans[1].1.0["telemetry.attributes"].contains("updated"));
    assert!(!state.spans[1].1.0["telemetry.attributes"].contains("late"));
    let named: Vec<_> = state
        .events
        .iter()
        .filter(|(_, f)| f.0["telemetry.operation"] == "\"event\"")
        .map(|(_, f)| f.0["telemetry.name"].as_str())
        .collect();
    assert_eq!(named, ["\"first\"", "\"second\""]);
    let finish: Vec<_> = state
        .events
        .iter()
        .filter(|(_, f)| f.0["telemetry.operation"] == "\"finish\"")
        .map(|(parent, _)| *parent)
        .collect();
    assert_eq!(finish, [Some(3), Some(2)]);
    let metrics: Vec<_> = state
        .events
        .iter()
        .filter(|(_, f)| f.0["telemetry.operation"] == "\"measurement\"")
        .collect();
    assert_eq!(metrics.len(), 3);
    assert_eq!(
        metrics[1].1.0["telemetry.context"],
        format!("Some({})", state.spans[1].1.0["telemetry.id"])
    );
    assert!(metrics[1].1.0["telemetry.definition"].contains("[1.0, 10.0]"));
    assert_eq!(
        state.closed,
        [3, 2],
        "observation handles must not retain native child spans after finish"
    );
    drop(kept);
}

#[test]
fn late_histogram_identity_survives_native_span_id_reuse() {
    let (state, root) = fixture();
    let first = root.start_span(SpanOptions::new("first")).unwrap();
    let kept = first.context();
    first.finish(None).unwrap();
    let second = root.start_span(SpanOptions::new("second")).unwrap();
    root.metrics()
        .histogram(InstrumentDefinition::new("duration", MetricKind::Histogram))
        .unwrap()
        .record(1.0, &Attributes::new(), Some(&kept))
        .unwrap();
    second.finish(None).unwrap();
    let state = state.lock().unwrap();
    assert_eq!(
        state.closed,
        [2, 2],
        "fixture must actually recycle native IDs"
    );
    let original = &state.spans[1].1.0["telemetry.id"];
    let later = &state.spans[2].1.0["telemetry.id"];
    assert_ne!(original, later);
    let metric = state
        .events
        .iter()
        .find(|(_, f)| f.0["telemetry.operation"] == "\"measurement\"")
        .unwrap();
    assert_eq!(metric.1.0["telemetry.context"], format!("Some({original})"));
}

#[test]
fn histogram_does_not_misattribute_a_foreign_dispatch_with_the_same_local_id() {
    let (a, a_context) = fixture();
    let (b, b_context) = fixture();
    let metric = a_context
        .metrics()
        .histogram(InstrumentDefinition::new("duration", MetricKind::Histogram))
        .unwrap();
    metric
        .record(1.0, &Attributes::new(), Some(&b_context))
        .unwrap();
    assert_eq!(a.lock().unwrap().events[0].1.0["telemetry.context"], "None");
    assert!(b.lock().unwrap().events.is_empty());
}

#[test]
fn scope_after_finish_does_not_reenter_a_closed_span() {
    let (state, context) = fixture();
    let owner = context.start_span(SpanOptions::new("request")).unwrap();
    let child_context = owner.context();
    owner.finish(None).unwrap();
    child_context.in_scope(|| tracing::info!(host_marker = "ambient only"));
    assert_eq!(
        state.lock().unwrap().events.len(),
        1,
        "only finish routes to frozen recorder"
    );
}
