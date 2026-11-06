use opi_telemetry::memory::Recorder;
use opi_telemetry::{Attributes, SpanOptions};

fn main() -> opi_telemetry::Result<()> {
    let recorder = Recorder::default();
    let context = recorder.context();
    let owner = context.start_span(SpanOptions::new("example.operation"))?;
    let observed = owner.span();
    observed.add_event("work.started", &Attributes::new())?;
    let answer = owner.context().in_scope(|| 42);
    observed.set_attributes(&[("answer".into(), answer.to_string().into())].into())?;
    owner.finish(None)?;
    assert_eq!(recorder.spans()[0].end_sequence, Some(0));
    Ok(())
}
