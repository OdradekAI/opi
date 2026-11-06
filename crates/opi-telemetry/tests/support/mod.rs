use opi_telemetry::{
    AttributeValue, Attributes, ErrorStatus, InstrumentDefinition, MetricKind, SpanOptions,
    SpanStatus, TelemetryContext,
};

pub fn attrs(value: &str) -> Attributes {
    [("value".into(), AttributeValue::from(value))].into()
}

/// The same public operations exercise no-op, memory and optional tracing.
/// The returned live context is already settled, but still owns metrics.
pub fn lifecycle(context: &TelemetryContext) -> TelemetryContext {
    let mut options = SpanOptions::new("request");
    options.attributes = [("initial".into(), "retained".into())].into();
    let owner = context.start_span(options).unwrap();
    let span = owner.span();
    let parent = owner.context();
    drop(owner.span());
    drop(parent.instrument(std::future::pending::<()>()));
    let child = parent.start_span(SpanOptions::new("child")).unwrap();
    child.finish(None).unwrap();
    span.set_attributes(&attrs("updated")).unwrap();
    span.add_event("first", &attrs("first")).unwrap();
    span.add_event("second", &Attributes::new()).unwrap();
    span.set_status(SpanStatus::Error { error: None }).unwrap();
    span.set_status(SpanStatus::Ok).unwrap();
    owner
        .finish(Some(ErrorStatus {
            name: "business".into(),
            message: "explicit ok wins".into(),
        }))
        .unwrap();
    span.set_attributes(&attrs("late")).unwrap();
    span.add_event("late", &Attributes::new()).unwrap();
    span.set_status(SpanStatus::Error { error: None }).unwrap();
    let inert = parent.start_span(SpanOptions::new("inert")).unwrap();
    let mut calls = 0;
    let output = inert.context().in_scope(|| {
        calls += 1;
        7
    });
    assert_eq!((calls, output), (1, 7));
    let metrics = inert.context().metrics();
    let counter = metrics
        .counter(InstrumentDefinition::new("count", MetricKind::Counter))
        .unwrap();
    counter.add(3.0, &attrs("counter")).unwrap();
    let mut definition = InstrumentDefinition::new("duration", MetricKind::Histogram);
    definition.description = Some("elapsed".into());
    definition.unit = Some("ms".into());
    definition.boundaries = Some(vec![1.0, 10.0]);
    metrics
        .histogram(definition)
        .unwrap()
        .record(2.0, &attrs("histogram"), Some(&parent))
        .unwrap();
    metrics
        .up_down_counter(InstrumentDefinition::new(
            "balance",
            MetricKind::UpDownCounter,
        ))
        .unwrap()
        .add(-1.0, &Attributes::new())
        .unwrap();
    inert.finish(None).unwrap();
    parent
}
