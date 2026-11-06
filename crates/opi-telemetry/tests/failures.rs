use opi_telemetry::backend::{ContextBackend, InstrumentBackend, MetricsBackend, SpanBackend};
use opi_telemetry::{
    Attributes, ErrorStatus, InstrumentDefinition, MetricKind, Result, SpanOptions, SpanStatus,
    TelemetryContext, TelemetryError,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Debug)]
struct Failure(&'static str);
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Failure {}

#[derive(Clone, Debug)]
struct Failing {
    phase: &'static str,
    failures: Arc<AtomicUsize>,
}
impl Failing {
    fn check(&self, phase: &'static str) -> Result<()> {
        if self.phase == phase {
            self.failures.fetch_add(1, Ordering::SeqCst);
            Err(TelemetryError::new(Failure(phase)))
        } else {
            Ok(())
        }
    }
}
impl ContextBackend for Failing {
    fn start_span(&self, _: &SpanOptions) -> Result<Arc<dyn SpanBackend>> {
        self.check("start")?;
        Ok(Arc::new(self.clone()))
    }
    fn metrics(&self) -> Arc<dyn MetricsBackend> {
        Arc::new(self.clone())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl SpanBackend for Failing {
    fn set_attributes(&self, _: &Attributes) -> Result<()> {
        self.check("attributes")
    }
    fn add_event(&self, _: &str, _: &Attributes) -> Result<()> {
        self.check("event")
    }
    fn set_status(&self, _: SpanStatus) -> Result<()> {
        self.check("status")
    }
    fn finish(&self, _: Option<ErrorStatus>) -> Result<()> {
        self.check("finish")
    }
}
impl MetricsBackend for Failing {
    fn instrument(&self, _: &InstrumentDefinition) -> Result<Arc<dyn InstrumentBackend>> {
        self.check("factory")?;
        Ok(Arc::new(self.clone()))
    }
}
impl InstrumentBackend for Failing {
    fn record(&self, _: f64, _: &Attributes, _: Option<&TelemetryContext>) -> Result<()> {
        self.check("measurement")
    }
}

#[test]
fn backend_failures_remain_distinct_synchronous_and_are_not_retried_or_swallowed() {
    for phase in [
        "start",
        "attributes",
        "event",
        "status",
        "finish",
        "factory",
        "measurement",
    ] {
        let failures = Arc::new(AtomicUsize::new(0));
        let context = TelemetryContext::new(Arc::new(Failing {
            phase,
            failures: failures.clone(),
        }));
        let result = match phase {
            "start" => context.start_span(SpanOptions::new("failure")).map(|_| ()),
            "factory" | "measurement" => context
                .metrics()
                .counter(InstrumentDefinition::new("failure", MetricKind::Counter))
                .and_then(|instrument| instrument.add(1.0, &Attributes::new())),
            _ => {
                let owner = context.start_span(SpanOptions::new("failure")).unwrap();
                match phase {
                    "attributes" => owner.span().set_attributes(&Attributes::new()),
                    "event" => owner.span().add_event("failure", &Attributes::new()),
                    "status" => owner.span().set_status(SpanStatus::Ok),
                    "finish" => owner.finish(None),
                    _ => unreachable!(),
                }
            }
        };
        let error = result.unwrap_err();
        let source = std::error::Error::source(&error)
            .unwrap()
            .downcast_ref::<Failure>()
            .unwrap();
        assert_eq!(source.0, phase);
        assert_eq!(
            failures.load(Ordering::SeqCst),
            1,
            "failure must be synchronous with no Drop retry"
        );
    }
}
