mod support;

use opi_telemetry::memory::Recorder;
use opi_telemetry::{
    AttributeValue, Attributes, ErrorStatus, InstrumentDefinition, MetricKind, SpanOptions,
    SpanStatus, TelemetryContext,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Barrier};
use std::task::{Context, Poll};
use support::{attrs, lifecycle};

#[test]
fn shared_lifecycle_merges_in_order_ignores_late_records_and_retains_metrics() {
    lifecycle(&TelemetryContext::noop());
    let recorder = Recorder::default();
    lifecycle(&recorder.context());
    let spans = recorder.spans();
    assert_eq!(spans.len(), 2);
    assert_eq!(
        (spans[0].id, spans[0].parent_id, spans[0].end_sequence),
        (0, None, Some(1))
    );
    assert_eq!(
        (spans[1].parent_id, spans[1].end_sequence),
        (Some(0), Some(0))
    );
    assert_eq!(spans[0].attributes["initial"], "retained".into());
    assert_eq!(spans[0].attributes["value"], "updated".into());
    assert_eq!(spans[0].status, SpanStatus::Ok);
    assert_eq!(
        spans[0]
            .events
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>(),
        ["first", "second"]
    );
    let measurements = recorder.measurements();
    assert_eq!(
        measurements
            .iter()
            .map(|m| (m.definition.kind, m.value))
            .collect::<Vec<_>>(),
        [
            (MetricKind::Counter, 3.0),
            (MetricKind::Histogram, 2.0),
            (MetricKind::UpDownCounter, -1.0)
        ]
    );
    assert_eq!(measurements[1].span_id, Some(0));
    assert_eq!(measurements[1].definition.unit.as_deref(), Some("ms"));
    assert_eq!(
        measurements[1].definition.description.as_deref(),
        Some("elapsed")
    );
    assert_eq!(measurements[1].definition.boundaries, Some(vec![1.0, 10.0]));
    assert_eq!(measurements[0].span_id, None);
}

#[test]
fn start_is_synchronous_and_only_explicit_finish_settles() {
    let recorder = Recorder::default();
    let owner = recorder
        .context()
        .start_span(SpanOptions::new("request"))
        .unwrap();
    assert_eq!(recorder.spans().len(), 1, "no future poll needed");
    let kept = owner.span();
    drop(kept.clone());
    drop(owner.context().instrument(std::future::pending::<()>()));
    assert_eq!(recorder.spans()[0].end_sequence, None);
    owner.finish(None).unwrap();
    assert_eq!(
        recorder.spans()[0].end_sequence,
        Some(0),
        "kept observer does not delay finish"
    );
    drop(kept);
    let abandoned = recorder
        .context()
        .start_span(SpanOptions::new("unfinished"))
        .unwrap();
    drop(abandoned);
    assert_eq!(
        recorder.spans()[1].end_sequence,
        None,
        "Drop is not success"
    );
}

#[test]
fn automatic_business_error_is_used_only_without_explicit_status() {
    let recorder = Recorder::default();
    let error = ErrorStatus {
        name: "original".into(),
        message: "failure".into(),
    };
    let automatic = recorder
        .context()
        .start_span(SpanOptions::new("automatic"))
        .unwrap();
    automatic.finish(Some(error.clone())).unwrap();
    let explicit = recorder
        .context()
        .start_span(SpanOptions::new("explicit"))
        .unwrap();
    explicit
        .span()
        .set_status(SpanStatus::Error { error: None })
        .unwrap();
    explicit.finish(Some(error.clone())).unwrap();
    assert_eq!(
        recorder.spans()[0].status,
        SpanStatus::Error { error: Some(error) }
    );
    assert_eq!(
        recorder.spans()[1].status,
        SpanStatus::Error { error: None }
    );
}

#[test]
fn input_and_snapshot_mutation_cannot_rewrite_records_and_recorders_are_isolated() {
    let a = Recorder::default();
    let b = Recorder::default();
    let mut values = Attributes::from([(
        "array".into(),
        AttributeValue::Strings(vec!["original".into()]),
    )]);
    let owner = a
        .context()
        .start_span(SpanOptions {
            name: "snapshot".into(),
            attributes: values.clone(),
        })
        .unwrap();
    owner.span().add_event("event", &values).unwrap();
    let histogram = a
        .context()
        .metrics()
        .histogram(InstrumentDefinition::new("samples", MetricKind::Histogram))
        .unwrap();
    let foreign = b.context().start_span(SpanOptions::new("foreign")).unwrap();
    histogram
        .record(1.0, &values, Some(&foreign.context()))
        .unwrap();
    values.clear();
    let original = a.spans();
    let mut detached = a.spans();
    detached[0].attributes.clear();
    detached[0].events[0].attributes.clear();
    a.measurements()[0].attributes.clear();
    assert_eq!(a.spans(), original);
    assert_eq!(
        a.measurements()[0].span_id,
        None,
        "foreign recorder must not collide on local id"
    );
    assert_eq!(a.measurements()[0].attributes.len(), 1);
    assert!(b.measurements().is_empty());
    owner.finish(None).unwrap();
    foreign.finish(None).unwrap();
}

#[test]
fn child_completion_order_is_independent_of_parent_clones_across_threads() {
    let recorder = Recorder::default();
    let owner = recorder
        .context()
        .start_span(SpanOptions::new("parent"))
        .unwrap();
    let child = owner
        .context()
        .start_span(SpanOptions::new("child"))
        .unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let gate = barrier.clone();
    let child_span = child.span();
    let worker = std::thread::spawn(move || {
        gate.wait();
        child.finish(None).unwrap();
        child_span.add_event("late", &Attributes::new()).unwrap();
    });
    owner.finish(None).unwrap();
    barrier.wait();
    worker.join().unwrap();
    assert_eq!(
        recorder
            .spans()
            .iter()
            .map(|s| s.end_sequence)
            .collect::<Vec<_>>(),
        [Some(0), Some(1)]
    );
    assert!(recorder.spans()[1].events.is_empty());
}

#[test]
fn noop_and_memory_preserve_pending_output_error_identity_and_unwind() {
    struct PendingOnce(bool);
    impl Future for PendingOnce {
        type Output = Arc<String>;
        fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
            if self.0 {
                Poll::Ready(Arc::new("business".into()))
            } else {
                self.0 = true;
                Poll::Pending
            }
        }
    }
    for context in [TelemetryContext::noop(), Recorder::default().context()] {
        let error = Arc::new("original error");
        let returned: std::result::Result<(), _> = context.in_scope(|| Err(error.clone()));
        assert!(Arc::ptr_eq(&error, &returned.unwrap_err()));
        let mut future = context.instrument(PendingOnce(false));
        let waker = futures_util::task::noop_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(Pin::new(&mut future).poll(&mut cx).is_pending());
        assert_eq!(
            Pin::new(&mut future).poll(&mut cx),
            Poll::Ready(Arc::new("business".into()))
        );
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            context.in_scope(|| panic!("business panic"))
        }));
        assert!(panic.is_err(), "no blanket panic swallowing");
    }
}

#[test]
fn schema_retains_metadata_without_runtime_validation_and_forwards_metric_options() {
    use opi_telemetry::schema::TelemetrySchema;
    let value = serde_json::json!({
        "version": 1,
        "spans": { "request": {
            "description": "request", "parents": { "kind": "any" },
            "startAttributes": { "provider": { "type": "string", "description": "owner", "required": true, "values": ["a"], "sensitive": true } },
            "endAttributes": { "size": { "type": "number", "description": "size", "cardinality": "high" } },
            "events": { "block": { "description": "block", "attributes": { "chunks": { "type": "string[]", "description": "chunks", "required": false, "elementValues": ["one"], "examples": [["one"]] } } } },
            "status": { "default": "ok", "errorWhen": "domain failure" }
        } },
        "metrics": { "duration": { "kind": "histogram", "description": "elapsed", "unit": "ms", "boundaries": [1,10], "attributes": {} } }
    });
    let schema: TelemetrySchema = serde_json::from_value(value).unwrap();
    let again: TelemetrySchema =
        serde_json::from_value(serde_json::to_value(&schema).unwrap()).unwrap();
    assert_eq!(schema, again);
    let definition = schema.metrics.unwrap()["duration"].instrument("duration");
    let recorder = Recorder::default();
    let instrument = recorder
        .context()
        .metrics()
        .histogram(definition.clone())
        .unwrap();
    instrument
        .record(2.0, &attrs("extra attributes accepted"), None)
        .unwrap();
    assert_eq!(recorder.measurements()[0].definition, definition);
    // The schema is descriptive; generic runtime start has no new validator.
    recorder
        .context()
        .start_span(SpanOptions::new("unregistered name"))
        .unwrap()
        .finish(None)
        .unwrap();
}
